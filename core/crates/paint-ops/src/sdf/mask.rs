use super::{
    allocated, count, smooth, valid_width, DistanceField, Error, Placement, Result, Stamp,
    MAX_OUTLINE_WIDTH, MAX_PIXELS, MAX_STAMPS, MAX_STAMP_VISITS,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaskOp {
    Max,
    Union,
    Intersection,
    Difference,
}

fn validate_mask(values: &[f32]) -> Result<()> {
    if values.len() > MAX_PIXELS {
        return Err(Error::Limit("mask pixels"));
    }
    if values
        .iter()
        .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
    {
        return Err(Error::Invalid("mask coverage"));
    }
    Ok(())
}
/// Color-independent coverage operations. Color is applied only after these.
pub fn combine_masks(a: &[f32], b: &[f32], op: MaskOp) -> Result<Vec<f32>> {
    if a.len() != b.len() {
        return Err(Error::Invalid("mask sizes"));
    }
    validate_mask(a)?;
    validate_mask(b)?;
    let mut output = allocated(a.len(), 0.)?;
    for ((out, &a), &b) in output.iter_mut().zip(a).zip(b) {
        *out = match op {
            MaskOp::Max => a.max(b),
            MaskOp::Union => 1. - (1. - a) * (1. - b),
            MaskOp::Intersection => a * b,
            MaskOp::Difference => a * (1. - b),
        };
    }
    Ok(output)
}

/// A disk stamp with a one-pixel smoothstep edge at the requested radius.
pub fn disk_coverage(stamp: &Stamp, pixel: [f32; 2]) -> f32 {
    smooth(stamp.radius + 0.5 - (pixel[0] - stamp.center[0]).hypot(pixel[1] - stamp.center[1]))
}

fn bounds(stamp: &Stamp, width: u32, height: u32) -> [usize; 4] {
    let r = stamp.radius + 0.5;
    [
        ((stamp.center[0] - r - 0.5).ceil().clamp(0., width as f32)) as usize,
        ((stamp.center[1] - r - 0.5).ceil().clamp(0., height as f32)) as usize,
        (((stamp.center[0] + r - 0.5).floor() + 1.).clamp(0., width as f32)) as usize,
        (((stamp.center[1] + r - 0.5).floor() + 1.).clamp(0., height as f32)) as usize,
    ]
}

/// Rasterize independent stamps using max coverage, never repeated color-over.
/// The callback samples the captured tip/texture within its circular support.
/// All visit counts are checked before the first callback. Errors discard the
/// local output; a callback must not mutate document state.
pub fn rasterize_max(
    width: u32,
    height: u32,
    stamps: &[Stamp],
    mut sample: impl FnMut(&Stamp, [f32; 2]) -> Result<f32>,
) -> Result<Vec<f32>> {
    let n = count(width, height)?;
    if stamps.len() > MAX_STAMPS {
        return Err(Error::Limit("stamps"));
    }
    let mut visits = 0_usize;
    for stamp in stamps {
        if stamp
            .center
            .iter()
            .chain(&stamp.tangent)
            .chain(&stamp.normal)
            .any(|v| !v.is_finite())
            || !stamp.radius.is_finite()
            || stamp.radius <= 0.
            || stamp.radius > MAX_OUTLINE_WIDTH * 0.5
            || !stamp.texture_rotation.is_finite()
        {
            return Err(Error::Invalid("stamp"));
        }
        let [x0, y0, x1, y1] = bounds(stamp, width, height);
        visits = visits
            .checked_add((x1 - x0) * (y1 - y0))
            .ok_or(Error::Limit("stamp visits"))?;
        if visits > MAX_STAMP_VISITS {
            return Err(Error::Limit("stamp visits"));
        }
    }
    let mut output = allocated(n, 0_f32)?;
    for stamp in stamps {
        let [x0, y0, x1, y1] = bounds(stamp, width, height);
        for y in y0..y1 {
            for x in x0..x1 {
                let coverage = sample(stamp, [x as f32 + 0.5, y as f32 + 0.5])?;
                if !coverage.is_finite() || !(0. ..=1.).contains(&coverage) {
                    return Err(Error::Invalid("stamp coverage"));
                }
                let pixel = &mut output[y * width as usize + x];
                *pixel = pixel.max(coverage);
            }
        }
    }
    Ok(output)
}

/// Boolean clipping of an independent stamp layer against immutable source A.
/// The returned alpha is ready for one premultiplied color composition.
pub fn compose_outline(
    field: &DistanceField,
    alpha: &[u8],
    stamps: &[f32],
    width: f32,
    placement: Placement,
) -> Result<Vec<f32>> {
    valid_width(width)?;
    if alpha.len() != field.values.len() || stamps.len() != alpha.len() {
        return Err(Error::Invalid("outline mask sizes"));
    }
    validate_mask(stamps)?;
    let mut output = allocated(alpha.len(), 0.)?;
    if width == 0. {
        return Ok(output);
    }
    for (((out, &a), &s), &d) in output.iter_mut().zip(alpha).zip(stamps).zip(&field.values) {
        let a = f32::from(a) / 255.;
        *out = match placement {
            Placement::Outside => s * (1. - a),
            Placement::Inside => {
                let eroded = a * smooth(-d - width + 0.5);
                s * a * (1. - eroded)
            }
            Placement::Center => s * smooth(width * 0.5 - d + 0.5) * smooth(width * 0.5 + d + 0.5),
        };
    }
    Ok(output)
}
