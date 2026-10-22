//! Read-only sparse presentation for a live move.
use super::*;

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
        (table[((v / alpha).clamp(0., 1.) * 4096.).round() as usize] * alpha * 255.).round() as u8
    };
    [
        channel(value[0]),
        channel(value[1]),
        channel(value[2]),
        (alpha * 255.).round() as u8,
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
