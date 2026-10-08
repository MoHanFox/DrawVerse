//! Normal linear composition and sparse power-of-two LOD. No UI or GPU dependency.
use paint_core::{blend_pixel, BlendMode, DocumentSnapshot, Pixel, Tile, TileCoord, TILE_SIZE};
use paint_task::CancellationToken;
use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, Mutex, Weak},
};

pub const MAX_OUTPUT_PIXELS: u64 = 4_194_304;
pub const MAX_OUTPUT_EDGE: u32 = 4096;
const MAX_SOURCE_TILES: usize = 1_000_000;
const MAX_WORK_BYTES: usize = 96 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Region {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    LinearRgba32F,
    SrgbRgba8Premultiplied,
}
#[derive(Clone, Copy, Debug)]
pub struct RenderRequest {
    pub region: Region,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
}
#[derive(Debug, PartialEq, Eq)]
pub enum RenderError {
    InvalidRequest,
    ResourceLimit,
    Cancelled,
    Storage(String),
}
impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for RenderError {}
#[derive(Debug)]
pub enum FramePixels {
    Linear(Vec<[f32; 4]>),
    Srgb(Vec<u8>),
}
#[derive(Debug)]
pub struct RenderFrame {
    pub region: Region,
    pub width: u32,
    pub height: u32,
    pub revision: u64,
    pub lod: u32,
    pub pixels: FramePixels,
}
pub trait Renderer: Send + Sync {
    fn render(
        &self,
        snapshot: &DocumentSnapshot,
        request: RenderRequest,
        cancel: &CancellationToken,
    ) -> Result<RenderFrame, RenderError>;
}
#[derive(Default)]
pub struct CpuRenderer;
impl RenderRequest {
    pub fn validate(&self) -> Result<(), RenderError> {
        let r = self.region;
        if [r.x, r.y, r.width, r.height]
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 2_000_000.)
            || r.width <= 0.
            || r.height <= 0.
            || self.width == 0
            || self.height == 0
            || self.width > MAX_OUTPUT_EDGE
            || self.height > MAX_OUTPUT_EDGE
            || u64::from(self.width) * u64::from(self.height) > MAX_OUTPUT_PIXELS
        {
            return Err(RenderError::InvalidRequest);
        }
        Ok(())
    }
}
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

#[derive(Clone, Copy, Debug, Default)]
pub struct RenderCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub retained_bytes: usize,
    pub entries: usize,
}
struct Source {
    tile: Weak<Tile>,
    opacity: f32,
}
struct Mip {
    linear: Vec<[f32; 4]>,
    srgb: Option<Vec<[u8; 4]>>,
}
struct Entry {
    sources: Vec<Source>,
    mip: Arc<Mip>,
    bytes: usize,
}
type CacheKey = (TileCoord, u32, PixelFormat);
#[derive(PartialEq)]
struct LayerConfiguration {
    id: u64,
    blend: BlendMode,
    offset: (i32, i32),
    seed: u32,
    parent: u64,
    group: bool,
    mask: bool,
    clipped: bool,
    white: (u32, u32),
    visible: bool,
    opacity: u32,
    fill: u32,
}
struct Cache {
    entries: HashMap<CacheKey, Entry>,
    budget: usize,
    stats: RenderCacheStats,
    dimensions: (u32, u32),
    configuration: Vec<LayerConfiguration>,
    grouped: bool,
    visible: Vec<usize>,
}
impl Cache {
    fn new(budget: usize) -> Self {
        Self {
            entries: HashMap::new(),
            budget,
            stats: RenderCacheStats::default(),
            dimensions: (0, 0),
            configuration: Vec::new(),
            grouped: false,
            visible: Vec::new(),
        }
    }
    fn prune(
        &mut self,
        dimensions: (u32, u32),
        coordinates: &BTreeSet<TileCoord>,
        lod: u32,
        format: PixelFormat,
    ) {
        self.entries.retain(|&(coord, level, kind), _| {
            dimensions == self.dimensions
                && coordinates.contains(&coord)
                && level == lod
                && kind == format
        });
        self.stats.retained_bytes = self.entries.values().map(|entry| entry.bytes).sum();
        self.stats.entries = self.entries.len();
        self.dimensions = dimensions;
    }
    fn mip(
        &mut self,
        snapshot: &DocumentSnapshot,
        coord: TileCoord,
        lod: u32,
        format: PixelFormat,
        cancel: &CancellationToken,
    ) -> Result<Arc<Mip>, RenderError> {
        let moved = self.grouped
            || snapshot
                .layers()
                .iter()
                .filter(|l| {
                    l.properties().visible && l.properties().opacity * l.appearance().fill > 0.
                })
                .any(|l| l.appearance().offset_x != 0 || l.appearance().offset_y != 0);
        let mut sources = Vec::new();
        for &index in &self.visible {
            let layer = &snapshot.layers()[index];
            let appearance = layer.appearance();
            let opacity = layer.properties().opacity * appearance.fill;
            if moved {
                for tile in layer.canvas_tile_sources(coord) {
                    sources.push((
                        tile,
                        opacity,
                        appearance.blend,
                        u64::from(appearance.dissolve_seed),
                    ));
                }
            } else if let Some(tile) = layer.tile_ref(coord) {
                // Keep the existing normal-layer path free of an intermediate Vec per layer/tile.
                sources.push((
                    tile,
                    opacity,
                    appearance.blend,
                    u64::from(appearance.dissolve_seed),
                ));
            }
        }
        let key = (coord, lod, format);
        if let Some(entry) = self.entries.get(&key) {
            if sources.len() == entry.sources.len()
                && sources
                    .iter()
                    .zip(&entry.sources)
                    .all(|((tile, opacity, _, _), old)| {
                        Arc::as_ptr(tile) == old.tile.as_ptr() && *opacity == old.opacity
                    })
            {
                self.stats.hits = self.stats.hits.saturating_add(1);
                return Ok(Arc::clone(&entry.mip));
            }
        }
        self.stats.misses = self.stats.misses.saturating_add(1);
        if let Some(entry) = self.entries.remove(&key) {
            self.stats.retained_bytes -= entry.bytes;
        }
        let pins = sources
            .iter()
            .map(|(tile, opacity, mode, id)| {
                Ok((
                    tile.try_pixels()
                        .map_err(|e| RenderError::Storage(e.to_string()))?,
                    *opacity,
                    *mode,
                    *id,
                ))
            })
            .collect::<Result<Vec<_>, RenderError>>()?;
        let edge = TILE_SIZE >> lod;
        let normal_only = pins
            .iter()
            .all(|(_, _, mode, _)| *mode == BlendMode::Normal);
        let mut linear = vec![[0.; 4]; (edge * edge) as usize];
        let weight = 1. / ((1u64 << (2 * lod)) as f32);
        let full_tile = (coord.x + 1) * TILE_SIZE <= snapshot.width
            && (coord.y + 1) * TILE_SIZE <= snapshot.height;
        let uniform = if pins.is_empty()
            && self.grouped
            && snapshot.layers().iter().all(|l| {
                let a = l.appearance();
                let w = l.white_extent();
                a.blend != BlendMode::Dissolve
                    && (w == (0, 0) || {
                        let x = i64::from(coord.x) * 64 - i64::from(a.offset_x);
                        let y = i64::from(coord.y) * 64 - i64::from(a.offset_y);
                        (x >= 0 && y >= 0 && x + 64 <= i64::from(w.0) && y + 64 <= i64::from(w.1))
                            || x + 64 <= 0
                            || y + 64 <= 0
                            || x >= i64::from(w.0)
                            || y >= i64::from(w.1)
                    })
            })
            && (coord.x + 1) * 64 <= snapshot.width
            && (coord.y + 1) * 64 <= snapshot.height
        {
            Some(
                snapshot
                    .try_pixel(coord.x * 64, coord.y * 64)
                    .map_err(|e| RenderError::Storage(e.to_string()))?
                    .components(),
            )
        } else {
            None
        };
        if let Some(pixel) = uniform {
            linear.fill(pixel);
        }
        for y in 0..if uniform.is_some() { 0 } else { TILE_SIZE } {
            let canvas_y = coord.y * TILE_SIZE + y;
            let row = if moved && canvas_y < snapshot.height {
                Some(
                    snapshot
                        .read_row(
                            coord.x * TILE_SIZE,
                            canvas_y,
                            TILE_SIZE.min(snapshot.width - coord.x * TILE_SIZE),
                        )
                        .map_err(|e| RenderError::Storage(e.to_string()))?,
                )
            } else {
                None
            };
            checkpoint(cancel)?;
            for x in 0..TILE_SIZE {
                if coord.x * TILE_SIZE + x >= snapshot.width
                    || coord.y * TILE_SIZE + y >= snapshot.height
                {
                    continue;
                }
                let composite;
                if let Some(row) = &row {
                    composite = row[x as usize].components();
                } else if normal_only {
                    // Preserve the original vectorizable source-over fast path.
                    // Common painting should not dispatch a general blend for
                    // every source pixel of a cold 8K overview.
                    let mut channels = [0.; 4];
                    for (pixels, opacity, _, _) in &pins {
                        let source = pixels[(y * TILE_SIZE + x) as usize]
                            .components()
                            .map(|v| v * opacity);
                        let remaining = 1. - source[3];
                        for i in 0..4 {
                            channels[i] = (source[i] + channels[i] * remaining).min(1.);
                        }
                    }
                    composite = channels;
                } else {
                    let mut pixel = Pixel::TRANSPARENT;
                    for (pixels, opacity, mode, id) in &pins {
                        pixel = blend_pixel(
                            pixels[(y * TILE_SIZE + x) as usize].scaled(*opacity),
                            pixel,
                            *mode,
                            coord.x * TILE_SIZE + x,
                            canvas_y,
                            *id,
                        );
                    }
                    composite = pixel.components();
                }
                let pixel = &mut linear[((y >> lod) * edge + (x >> lod)) as usize];
                // Partial canvas-edge blocks average only valid document pixels.
                // Transparent padding belongs outside the canvas, not in its thumbnail.
                let sample_weight = if full_tile {
                    weight
                } else {
                    let bx = coord.x * TILE_SIZE + ((x >> lod) << lod);
                    let by = coord.y * TILE_SIZE + ((y >> lod) << lod);
                    let valid = (1u32 << lod).min(snapshot.width - bx)
                        * (1u32 << lod).min(snapshot.height - by);
                    1. / valid as f32
                };
                for i in 0..4 {
                    pixel[i] += composite[i] * sample_weight;
                }
            }
        }
        drop(pins);
        snapshot
            .trim_storage()
            .map_err(|e| RenderError::Storage(e.to_string()))?;
        let srgb = (format == PixelFormat::SrgbRgba8Premultiplied).then(|| {
            linear
                .iter()
                .copied()
                .map(display_pixel)
                .collect::<Vec<_>>()
        });
        checkpoint(cancel)?;
        let bytes = linear.len() * 16
            + srgb.as_ref().map_or(0, |v| v.len() * 4)
            + sources.len() * std::mem::size_of::<Source>()
            + std::mem::size_of::<Entry>()
            + std::mem::size_of::<CacheKey>();
        let mip = Arc::new(Mip { linear, srgb });
        if self.stats.retained_bytes.saturating_add(bytes) <= self.budget {
            self.entries.insert(
                key,
                Entry {
                    sources: sources
                        .into_iter()
                        .map(|(tile, opacity, _, _)| Source {
                            tile: Arc::downgrade(tile),
                            opacity,
                        })
                        .collect(),
                    mip: Arc::clone(&mip),
                    bytes,
                },
            );
            self.stats.retained_bytes += bytes;
        }
        self.stats.entries = self.entries.len();
        Ok(mip)
    }
}
/// One instance per viewport. Serialization only covers that viewport's cache;
/// no document/GUI lock is held, and obsolete jobs check cancellation per row/tile.
pub struct CachedCpuRenderer {
    cache: Mutex<Cache>,
}
impl Default for CachedCpuRenderer {
    fn default() -> Self {
        Self {
            cache: Mutex::new(Cache::new(MAX_WORK_BYTES)),
        }
    }
}
impl CachedCpuRenderer {
    pub fn with_cache_budget(bytes: usize) -> Result<Self, RenderError> {
        if bytes > MAX_WORK_BYTES {
            return Err(RenderError::ResourceLimit);
        }
        Ok(Self {
            cache: Mutex::new(Cache::new(bytes)),
        })
    }
    pub fn cache_stats(&self) -> RenderCacheStats {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).stats
    }
}
impl Renderer for CachedCpuRenderer {
    fn render(
        &self,
        snapshot: &DocumentSnapshot,
        request: RenderRequest,
        cancel: &CancellationToken,
    ) -> Result<RenderFrame, RenderError> {
        request.validate()?;
        checkpoint(cancel)?;
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        render(snapshot, request, cancel, &mut cache)
    }
}
impl Renderer for CpuRenderer {
    fn render(
        &self,
        snapshot: &DocumentSnapshot,
        request: RenderRequest,
        cancel: &CancellationToken,
    ) -> Result<RenderFrame, RenderError> {
        render(snapshot, request, cancel, &mut Cache::new(0))
    }
}
struct Axis {
    pixels: Vec<u32>,
    spans: HashMap<u32, (usize, usize)>,
}
fn axis(start: f64, size: f64, count: u32, limit: u32, block: u32) -> Axis {
    let mut pixels = Vec::with_capacity(count as usize);
    let mut spans: HashMap<u32, (usize, usize)> = HashMap::new();
    for i in 0..count {
        let source = start + (f64::from(i) + 0.5) * size / f64::from(count);
        if source < 0. || source >= f64::from(limit) {
            pixels.push(u32::MAX);
            continue;
        }
        let pixel = source.floor() as u32;
        pixels.push(pixel);
        spans
            .entry(pixel / block)
            .and_modify(|span| span.1 = i as usize + 1)
            .or_insert((i as usize, i as usize + 1));
    }
    Axis { pixels, spans }
}
/// Output starts transparent, then only allocated source rectangles are touched.
fn blit(
    output: &mut FramePixels,
    width: u32,
    axes: (&Axis, &Axis),
    coord: TileCoord,
    mip: &Mip,
    lod: u32,
    cancel: &CancellationToken,
) -> Result<(), RenderError> {
    let (xs, ys) = axes;
    let Some(&(x0, x1)) = xs.spans.get(&coord.x) else {
        return Ok(());
    };
    let Some(&(y0, y1)) = ys.spans.get(&coord.y) else {
        return Ok(());
    };
    let edge = TILE_SIZE >> lod;
    for y in y0..y1 {
        checkpoint(cancel)?;
        let row = (((ys.pixels[y] % TILE_SIZE) >> lod) * edge) as usize;
        match output {
            FramePixels::Linear(values) => {
                for x in x0..x1 {
                    values[y * width as usize + x] =
                        mip.linear[row + ((xs.pixels[x] % TILE_SIZE) >> lod) as usize];
                }
            }
            FramePixels::Srgb(bytes) => {
                let encoded = mip.srgb.as_ref().expect("display mip");
                for x in x0..x1 {
                    let value = encoded[row + ((xs.pixels[x] % TILE_SIZE) >> lod) as usize];
                    let i = (y * width as usize + x) * 4;
                    bytes[i..i + 4].copy_from_slice(&value);
                }
            }
        }
    }
    Ok(())
}
fn render(
    snapshot: &DocumentSnapshot,
    request: RenderRequest,
    cancel: &CancellationToken,
    cache: &mut Cache,
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
    if lod > 6
        && (r.width.min(f64::from(snapshot.width)) / 64.).ceil()
            * (r.height.min(f64::from(snapshot.height)) / 64.).ceil()
            > MAX_SOURCE_TILES as f64
        && snapshot.layers().iter().any(|l| l.white_extent() != (0, 0))
    {
        // Bounded nearest reference sampling for procedural canvases at extreme zoom-out.
        // Regular sparse raster overview keeps the existing exact box aggregation below.
        let mut linear = Vec::with_capacity((request.width * request.height) as usize);
        for y in 0..request.height {
            checkpoint(cancel)?;
            let py = r.y + (f64::from(y) + 0.5) * r.height / f64::from(request.height);
            for x in 0..request.width {
                let px = r.x + (f64::from(x) + 0.5) * r.width / f64::from(request.width);
                let p = if px < 0.
                    || py < 0.
                    || px >= f64::from(snapshot.width)
                    || py >= f64::from(snapshot.height)
                {
                    Pixel::TRANSPARENT
                } else {
                    snapshot
                        .try_pixel(px as u32, py as u32)
                        .map_err(|e| RenderError::Storage(e.to_string()))?
                };
                linear.push(p.components());
            }
        }
        let pixels = match request.format {
            PixelFormat::LinearRgba32F => FramePixels::Linear(linear),
            PixelFormat::SrgbRgba8Premultiplied => {
                FramePixels::Srgb(linear.into_iter().flat_map(display_pixel).collect())
            }
        };
        return Ok(RenderFrame {
            region: r,
            width: request.width,
            height: request.height,
            revision: snapshot.revision,
            lod,
            pixels,
        });
    }
    let block = 1u32 << lod;
    let x0 = (r.x.max(0.) as u32 / block) * block;
    let y0 = (r.y.max(0.) as u32 / block) * block;
    let x1 = (((r.x + r.width).max(0.).ceil() as u32).div_ceil(block) * block).min(snapshot.width);
    let y1 =
        (((r.y + r.height).max(0.).ceil() as u32).div_ceil(block) * block).min(snapshot.height);
    let mut coordinates = BTreeSet::new();
    cache.grouped = snapshot.has_groups();
    cache.visible = snapshot.visible_pixel_layers();
    for &index in &cache.visible {
        let layer = &snapshot.layers()[index];
        checkpoint(cancel)?;
        let appearance = layer.appearance();
        if layer.white_extent() != (0, 0) {
            // Only sampled viewport tiles, never the entire implicit canvas.
            if x0 < x1 && y0 < y1 {
                for ty in y0 / 64..=(y1 - 1) / 64 {
                    for tx in x0 / 64..=(x1 - 1) / 64 {
                        coordinates.insert(TileCoord { x: tx, y: ty });
                        if coordinates.len() > MAX_SOURCE_TILES {
                            return Err(RenderError::ResourceLimit);
                        }
                    }
                }
            }
        }
        for (coord, _) in layer.tiles() {
            let x = i64::from(coord.signed_x()) * 64 + i64::from(appearance.offset_x);
            let y = i64::from(coord.signed_y()) * 64 + i64::from(appearance.offset_y);
            let (left, top, right, bottom) = (
                x.max(i64::from(x0)),
                y.max(i64::from(y0)),
                (x + 64).min(i64::from(x1)),
                (y + 64).min(i64::from(y1)),
            );
            if left >= right || top >= bottom {
                continue;
            }
            for ty in top / 64..=(bottom - 1) / 64 {
                for tx in left / 64..=(right - 1) / 64 {
                    coordinates.insert(TileCoord {
                        x: tx as u32,
                        y: ty as u32,
                    });
                    if coordinates.len() > MAX_SOURCE_TILES {
                        return Err(RenderError::ResourceLimit);
                    }
                }
            }
        }
    }
    let configuration = snapshot
        .layers()
        .iter()
        .map(|l| {
            let a = l.appearance();
            LayerConfiguration {
                id: l.id(),
                blend: a.blend,
                offset: (a.offset_x, a.offset_y),
                seed: a.dissolve_seed,
                parent: l.parent_id(),
                group: l.is_group(),
                mask: l.is_mask(),
                clipped: l.is_clipped(),
                white: l.white_extent(),
                visible: l.properties().visible,
                opacity: l.properties().opacity.to_bits(),
                fill: a.fill.to_bits(),
            }
        })
        .collect::<Vec<_>>();
    if cache.configuration != configuration {
        cache.entries.clear();
        cache.configuration = configuration;
    }
    let local = lod.min(6);
    let edge = TILE_SIZE >> local;
    if coordinates
        .len()
        .checked_mul((edge * edge * 16) as usize)
        .is_none_or(|work| work > MAX_WORK_BYTES)
    {
        return Err(RenderError::ResourceLimit);
    }
    let kind = if lod > 6 {
        PixelFormat::LinearRgba32F
    } else {
        request.format
    };
    cache.prune((snapshot.width, snapshot.height), &coordinates, local, kind);
    let count = (request.width * request.height) as usize;
    let mut pixels = match request.format {
        PixelFormat::LinearRgba32F => FramePixels::Linear(vec![[0.; 4]; count]),
        PixelFormat::SrgbRgba8Premultiplied => FramePixels::Srgb(vec![0; count * 4]),
    };
    let xs = axis(
        r.x,
        r.width,
        request.width,
        snapshot.width,
        if lod > 6 { block } else { TILE_SIZE },
    );
    let ys = axis(
        r.y,
        r.height,
        request.height,
        snapshot.height,
        if lod > 6 { block } else { TILE_SIZE },
    );
    let mut coarse: HashMap<TileCoord, [f32; 4]> = HashMap::new();
    for coord in coordinates {
        checkpoint(cancel)?;
        let mip = cache.mip(snapshot, coord, local, kind, cancel)?;
        if lod <= 6 {
            blit(
                &mut pixels,
                request.width,
                (&xs, &ys),
                coord,
                &mip,
                lod,
                cancel,
            )?;
        } else {
            let output = coarse
                .entry(TileCoord {
                    x: coord.x >> (lod - 6),
                    y: coord.y >> (lod - 6),
                })
                .or_insert([0.; 4]);
            let tile_area = (64u32.min(snapshot.width - coord.x * 64)) as u64
                * u64::from(64u32.min(snapshot.height - coord.y * 64));
            let coarse_x = (coord.x >> (lod - 6)) << lod;
            let coarse_y = (coord.y >> (lod - 6)) << lod;
            let coarse_area = u64::from((1u32 << lod).min(snapshot.width - coarse_x))
                * u64::from((1u32 << lod).min(snapshot.height - coarse_y));
            let weight = tile_area as f32 / coarse_area as f32;
            for (channel, source) in output.iter_mut().zip(mip.linear[0]) {
                *channel += source * weight;
            }
        }
    }
    if lod > 6 {
        for (coord, pixel) in coarse {
            checkpoint(cancel)?;
            let Some(&(x0, x1)) = xs.spans.get(&coord.x) else {
                continue;
            };
            let Some(&(y0, y1)) = ys.spans.get(&coord.y) else {
                continue;
            };
            let encoded = display_pixel(pixel);
            for y in y0..y1 {
                checkpoint(cancel)?;
                match &mut pixels {
                    FramePixels::Linear(values) => values
                        [y * request.width as usize + x0..y * request.width as usize + x1]
                        .fill(pixel),
                    FramePixels::Srgb(bytes) => {
                        for x in x0..x1 {
                            let i = (y * request.width as usize + x) * 4;
                            bytes[i..i + 4].copy_from_slice(&encoded);
                        }
                    }
                }
            }
        }
    }
    checkpoint(cancel)?;
    Ok(RenderFrame {
        region: r,
        width: request.width,
        height: request.height,
        revision: snapshot.revision,
        lod,
        pixels,
    })
}
