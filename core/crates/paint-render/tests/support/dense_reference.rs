use paint_core::{DocumentSnapshot, TileCoord, TILE_SIZE};
use paint_render::*;
use paint_task::CancellationToken;
use std::collections::{BTreeSet, HashMap};
const MAX_SOURCE_TILES: usize = 8192;
const MAX_WORK_BYTES: usize = 96 * 1024 * 1024;
pub struct DenseReference;
fn checkpoint(cancel: &CancellationToken) -> Result<(), RenderError> {
    if cancel.is_cancelled() {
        Err(RenderError::Cancelled)
    } else {
        Ok(())
    }
}
/// Standard sRGB encoding; returned channels are explicitly premultiplied for display.
pub fn display_pixel(linear: [f32; 4]) -> [u8; 4] {
    let alpha = linear[3].clamp(0., 1.);
    if alpha <= 0. {
        return [0; 4];
    }
    let channel = |v: f32| {
        let v = (v / alpha).clamp(0., 1.);
        let encoded = if v <= 0.0031308 {
            12.92 * v
        } else {
            1.055 * v.powf(1. / 2.4) - 0.055
        };
        (encoded * alpha * 255.).round() as u8
    };
    [
        channel(linear[0]),
        channel(linear[1]),
        channel(linear[2]),
        (alpha * 255.).round() as u8,
    ]
}
impl Renderer for DenseReference {
    fn render(
        &self,
        snapshot: &DocumentSnapshot,
        request: RenderRequest,
        cancel: &CancellationToken,
    ) -> Result<RenderFrame, RenderError> {
        request.validate()?;
        checkpoint(cancel)?;
        let r = request.region;
        let scale = (r.width / f64::from(request.width)).min(r.height / f64::from(request.height));
        let lod = if scale >= 1. {
            scale.log2().ceil().clamp(0., 20.) as u32
        } else {
            0
        };
        let block = 1_u32 << lod;
        // Expand to complete globally aligned LOD blocks: averaging must include pixels
        // outside a cropped viewport that belong to the same sampled block.
        let x0 = (r.x.max(0.) as u32 / block) * block;
        let y0 = (r.y.max(0.) as u32 / block) * block;
        let x1 =
            (((r.x + r.width).max(0.).ceil() as u32).div_ceil(block) * block).min(snapshot.width);
        let y1 =
            (((r.y + r.height).max(0.).ceil() as u32).div_ceil(block) * block).min(snapshot.height);
        let mut coordinates = BTreeSet::new();
        for layer in snapshot
            .layers()
            .iter()
            .filter(|l| l.properties().visible && l.properties().opacity > 0.)
        {
            checkpoint(cancel)?;
            for (coord, _) in layer.tiles() {
                let x = coord.x * TILE_SIZE;
                let y = coord.y * TILE_SIZE;
                if x < x1 && y < y1 && x + TILE_SIZE > x0 && y + TILE_SIZE > y0 {
                    coordinates.insert(coord);
                    if coordinates.len() > MAX_SOURCE_TILES {
                        return Err(RenderError::ResourceLimit);
                    }
                }
            }
        }
        let local_lod = lod.min(6);
        let edge = TILE_SIZE >> local_lod;
        let work = coordinates
            .len()
            .checked_mul((edge * edge * 16) as usize)
            .ok_or(RenderError::ResourceLimit)?;
        if work > MAX_WORK_BYTES {
            return Err(RenderError::ResourceLimit);
        }
        let mut tiles: HashMap<TileCoord, Vec<[f32; 4]>> = HashMap::new();
        let mut coarse: HashMap<(u32, u32), [f32; 4]> = HashMap::new();
        for coord in coordinates {
            checkpoint(cancel)?;
            let contributors: Vec<_> = snapshot
                .layers()
                .iter()
                .filter(|layer| layer.properties().visible && layer.properties().opacity > 0.)
                .filter_map(|layer| {
                    layer
                        .tile(coord)
                        .map(|tile| (tile, layer.properties().opacity))
                })
                .collect();
            let mut mip = vec![[0.; 4]; (edge * edge) as usize];
            for y in 0..TILE_SIZE {
                checkpoint(cancel)?;
                for x in 0..TILE_SIZE {
                    if coord.x * TILE_SIZE + x >= snapshot.width
                        || coord.y * TILE_SIZE + y >= snapshot.height
                    {
                        continue;
                    }
                    let mut composite = [0.; 4];
                    for (tile, opacity) in &contributors {
                        let source = tile.pixel(x, y).components().map(|v| v * opacity);
                        let remaining = 1. - source[3];
                        for i in 0..4 {
                            composite[i] = (source[i] + composite[i] * remaining).min(1.);
                        }
                    }
                    let output = &mut mip[((y >> local_lod) * edge + (x >> local_lod)) as usize];
                    // A LOD block intersecting the canvas ends at the document
                    // boundary; padding outside the document is not image alpha.
                    let bx = (coord.x * TILE_SIZE + x) / (1 << local_lod) * (1 << local_lod);
                    let by = (coord.y * TILE_SIZE + y) / (1 << local_lod) * (1 << local_lod);
                    let area = (snapshot.width - bx).min(1 << local_lod) as u64
                        * u64::from((snapshot.height - by).min(1 << local_lod));
                    let weight = 1. / area as f32;
                    for i in 0..4 {
                        output[i] += composite[i] * weight;
                    }
                }
            }
            if lod <= 6 {
                tiles.insert(coord, mip);
            } else {
                let output = coarse
                    .entry((coord.x >> (lod - 6), coord.y >> (lod - 6)))
                    .or_insert([0.; 4]);
                let tile_area = u64::from((snapshot.width - coord.x * TILE_SIZE).min(TILE_SIZE))
                    * u64::from((snapshot.height - coord.y * TILE_SIZE).min(TILE_SIZE));
                let bx = coord.x * TILE_SIZE / block * block;
                let by = coord.y * TILE_SIZE / block * block;
                let area = u64::from((snapshot.width - bx).min(block))
                    * u64::from((snapshot.height - by).min(block));
                let weight = tile_area as f32 / area as f32;
                for i in 0..4 {
                    output[i] += mip[0][i] * weight;
                }
            }
        }
        let count = (request.width * request.height) as usize;
        let mut pixels = match request.format {
            PixelFormat::LinearRgba32F => FramePixels::Linear(Vec::with_capacity(count)),
            PixelFormat::SrgbRgba8Premultiplied => FramePixels::Srgb(Vec::with_capacity(count * 4)),
        };
        for y in 0..request.height {
            checkpoint(cancel)?;
            for x in 0..request.width {
                let sx = r.x + (f64::from(x) + 0.5) * r.width / f64::from(request.width);
                let sy = r.y + (f64::from(y) + 0.5) * r.height / f64::from(request.height);
                let mut pixel = [0.; 4];
                if sx >= 0.
                    && sy >= 0.
                    && sx < f64::from(snapshot.width)
                    && sy < f64::from(snapshot.height)
                {
                    let px = sx.floor() as u32;
                    let py = sy.floor() as u32;
                    if lod > 6 {
                        pixel = coarse
                            .get(&(px >> lod, py >> lod))
                            .copied()
                            .unwrap_or([0.; 4]);
                    } else if let Some(mip) = tiles.get(&TileCoord {
                        x: px / TILE_SIZE,
                        y: py / TILE_SIZE,
                    }) {
                        pixel = mip[(((py % TILE_SIZE) >> lod) * edge + ((px % TILE_SIZE) >> lod))
                            as usize];
                    }
                }
                match &mut pixels {
                    FramePixels::Linear(values) => values.push(pixel),
                    FramePixels::Srgb(bytes) => bytes.extend_from_slice(&display_pixel(pixel)),
                }
            }
        }
        Ok(RenderFrame {
            region: r,
            width: request.width,
            height: request.height,
            revision: snapshot.revision,
            lod,
            pixels,
        })
    }
}
