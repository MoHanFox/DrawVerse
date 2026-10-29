//! Read-only sparse presentation for a live move.
use super::*;

/// Native texels of a single ordinary raster, for immutable display
/// assets. Return None when general composition or resampling is required.
/// Tile-local encoding avoids repeatedly composing offset guard pixels through
/// the general renderer. Materialisation/export keep their existing paths.
pub fn render_native_preview(
    snapshot: &DocumentSnapshot,
    request: RenderRequest,
    cancel: &CancellationToken,
) -> Result<Option<RenderFrame>, RenderError> {
    request.validate()?;
    checkpoint(cancel)?;
    let r = request.region;
    if request.format != PixelFormat::SrgbRgba8Premultiplied
        || r.width != f64::from(request.width)
        || r.height != f64::from(request.height)
        || r.x.fract() != 0.
        || r.y.fract() != 0.
        || r.x < 0.
        || r.y < 0.
        || r.x + r.width > f64::from(snapshot.width)
        || r.y + r.height > f64::from(snapshot.height)
        || snapshot.layers().len() != 1
    {
        return Ok(None);
    }
    let layer = &snapshot.layers()[0];
    let a = layer.appearance();
    if layer.is_group()
        || layer.is_mask()
        || layer.is_clipped()
        || layer.is_image()
        || layer.parent_id() != 0
        || a.blend != BlendMode::Normal
        || !layer.properties().visible
        || layer.properties().opacity != 1.
        || a.fill != 1.
    {
        return Ok(None);
    }
    let length = request.width as usize * request.height as usize * 4;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| RenderError::ResourceLimit)?;
    bytes.resize(length, 0);
    let left = r.x as i64 - i64::from(a.offset_x);
    let top = r.y as i64 - i64::from(a.offset_y);
    let right = left + i64::from(request.width);
    let bottom = top + i64::from(request.height);
    let (white_w, white_h) = layer.white_extent();
    for y in top.max(0)..bottom.min(i64::from(white_h)) {
        checkpoint(cancel)?;
        let x0 = left.max(0);
        let x1 = right.min(i64::from(white_w));
        if x0 < x1 {
            let start = ((y - top) as usize * request.width as usize + (x0 - left) as usize) * 4;
            bytes[start..start + (x1 - x0) as usize * 4].fill(255);
        }
    }
    for (coord, tile) in layer.tiles() {
        checkpoint(cancel)?;
        let tx = i64::from(coord.signed_x()) * 64;
        let ty = i64::from(coord.signed_y()) * 64;
        let x0 = left.max(tx);
        let x1 = right.min(tx + 64);
        let y0 = top.max(ty);
        let y1 = bottom.min(ty + 64);
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        {
            let pixels = tile
                .try_pixels()
                .map_err(|e| RenderError::Storage(e.to_string()))?;
            for y in y0..y1 {
                checkpoint(cancel)?;
                let from = ((y - ty) * 64 + x0 - tx) as usize;
                let target =
                    ((y - top) as usize * request.width as usize + (x0 - left) as usize) * 4;
                for (dest, pixel) in bytes[target..target + (x1 - x0) as usize * 4]
                    .chunks_exact_mut(4)
                    .zip(&pixels[from..from + (x1 - x0) as usize])
                {
                    dest.copy_from_slice(&display(pixel.components()));
                }
            }
        }
        snapshot
            .trim_storage()
            .map_err(|e| RenderError::Storage(e.to_string()))?;
    }
    Ok(Some(RenderFrame {
        region: r,
        width: request.width,
        height: request.height,
        revision: snapshot.revision,
        lod: 0,
        pixels: FramePixels::Srgb(bytes),
    }))
}

// The linear quantization error maps to less than half an 8-bit sRGB step.
// Materialisation and export retain the ordinary exact transfer curve.
fn display(value: [f32; 4]) -> [u8; 4] {
    static TABLE: std::sync::OnceLock<Vec<f32>> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        (0..=4096)
            .map(|i| {
                let linear = i as f32 / 4096.;
                if linear <= 0.0031308 {
                    12.92 * linear
                } else {
                    1.055 * linear.powf(1. / 2.4) - 0.055
                }
            })
            .collect()
    });
    let alpha = value[3].clamp(0., 1.);
    if alpha <= 0. {
        return [0; 4];
    }
    let channel = |v: f32| {
        // All operands are nonnegative and bounded. Truncation after adding
        // half avoids scalar roundf calls for every channel on baseline x64.
        let index = ((v / alpha).clamp(0., 1.) * 4096. + 0.5) as usize;
        (table[index] * alpha * 255. + 0.5) as u8
    };
    [
        channel(value[0]),
        channel(value[1]),
        channel(value[2]),
        (alpha * 255. + 0.5) as u8,
    ]
}

/// Sparse nearest-neighbour display only. Never use for materialisation or export.
pub fn render_interactive(
    snapshot: &DocumentSnapshot,
    mut request: RenderRequest,
    cancel: &CancellationToken,
) -> Result<RenderFrame, RenderError> {
    request.validate()?;
    checkpoint(cancel)?;
    let ratio =
        (f64::from(MOVE_PREVIEW_MAX_EDGE) / f64::from(request.width.max(request.height))).min(1.);
    request.width = (f64::from(request.width) * ratio).ceil().max(1.) as u32;
    request.height = (f64::from(request.height) * ratio).ceil().max(1.) as u32;
    render_sampled_preview(snapshot, request, cancel)
}

/// Display-only point sampling at the requested proxy resolution. Unlike the
/// ordinary LOD renderer this never composites pixels between output samples.
/// Callers choose their proxy bound; materialisation/export must not use this.
pub fn render_sampled_preview(
    snapshot: &DocumentSnapshot,
    request: RenderRequest,
    cancel: &CancellationToken,
) -> Result<RenderFrame, RenderError> {
    request.validate()?;
    checkpoint(cancel)?;
    let r = request.region;
    let xs = axis(r.x, r.width, request.width, snapshot.width, 1).pixels;
    let ys = axis(r.y, r.height, request.height, snapshot.height, 1).pixels;
    // Canvas clipping occupies the two ends of this monotone sample axis.
    let first = xs.iter().position(|x| *x != u32::MAX).unwrap_or(xs.len());
    let end = xs
        .iter()
        .rposition(|x| *x != u32::MAX)
        .map_or(first, |x| x + 1);
    let count = (request.width * request.height) as usize;
    let mut pixels = match request.format {
        PixelFormat::LinearRgba32F => {
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|_| RenderError::ResourceLimit)?;
            values.resize(count, [0.; 4]);
            FramePixels::Linear(values)
        }
        PixelFormat::SrgbRgba8Premultiplied => {
            let mut values = Vec::new();
            values
                .try_reserve_exact(count * 4)
                .map_err(|_| RenderError::ResourceLimit)?;
            values.resize(count * 4, 0);
            FramePixels::Srgb(values)
        }
    };
    for (y, sy) in ys.into_iter().enumerate() {
        checkpoint(cancel)?;
        if sy == u32::MAX || first == end {
            continue;
        }
        let row = snapshot
            .read_sampled_row(&xs[first..end], sy)
            .map_err(|e| RenderError::Storage(e.to_string()))?;
        for (x, source) in row.into_iter().enumerate() {
            let value = source.components();
            let target = y * request.width as usize + first + x;
            match &mut pixels {
                FramePixels::Linear(values) => values[target] = value,
                FramePixels::Srgb(bytes) => {
                    bytes[target * 4..target * 4 + 4].copy_from_slice(&display(value))
                }
            }
        }
        snapshot
            .trim_storage()
            .map_err(|e| RenderError::Storage(e.to_string()))?;
    }
    Ok(RenderFrame {
        region: r,
        width: request.width,
        height: request.height,
        revision: snapshot.revision,
        lod: 0,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coarse_display_quantization_stays_within_one_output_level() {
        for alpha in [0., 0.01, 0.2, 0.7, 1.] {
            for i in 0..65536 {
                let v = i as f32 / 65535.;
                let value = [v * alpha, (1. - v) * alpha, v * v * alpha, alpha];
                for (actual, reference) in display(value).into_iter().zip(display_pixel(value)) {
                    assert!(actual.abs_diff(reference) <= 1);
                }
            }
        }
    }
}
