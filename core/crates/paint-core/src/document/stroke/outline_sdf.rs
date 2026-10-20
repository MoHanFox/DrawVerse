//! CPU execution of color-independent SDF operators against immutable captured tips.
use super::*;
use paint_ops::sdf::{self, DistanceField, Placement};

#[derive(Debug)]
pub(super) struct Cache {
    // Retain the immutable inputs for this shape revision, never destination pixels.
    _alpha: Vec<u8>,
    _field: DistanceField,
    pub(super) band: Arc<[f32]>,
}
fn error(error: sdf::Error) -> Error {
    match error {
        sdf::Error::Invalid(_) => Error::InvalidArgument("SDF stroke outline operator"),
        sdf::Error::Limit(_) => Error::ResourceLimit("SDF stroke outline operator budget"),
    }
}

pub(super) fn render(
    outline: &OutlineStroke,
    primary: &BrushProgram,
    left: i64,
    top: i64,
    width: usize,
    height: usize,
    alpha: Vec<u8>,
) -> Result<Cache> {
    let placement = match outline.settings.placement() {
        crate::OutlinePlacement::Outside => Placement::Outside,
        crate::OutlinePlacement::Inside => Placement::Inside,
        crate::OutlinePlacement::Center => Placement::Center,
    };
    let field = DistanceField::from_alpha(width as u32, height as u32, &alpha).map_err(error)?;
    let stamps = field
        .outline_stamps(outline.width, placement)
        .map_err(error)?;
    let captured;
    let program = match outline.settings.source() {
        OutlineSource::Existing(tip) => {
            captured = BrushProgram::compile(BrushDefinition {
                tip: tip.tip(),
                mask: tip.mask().cloned(),
                ..Default::default()
            })?;
            &captured
        }
        OutlineSource::Texture(_) => {
            captured = BrushProgram::compile(BrushDefinition::default())?;
            &captured
        }
        _ => primary,
    };
    let geometry = program.geometry(InputPoint::new(0., 0., 1.), None);
    // Every contour stamp has the same radius. Compile image LOD/AA once.
    let radius = f64::from(outline.width) * 0.5;
    let mask = program
        .has_mask()
        .then(|| program.mask_dab_with_geometry(radius, geometry))
        .flatten();
    let texture = match outline.settings.source() {
        OutlineSource::Texture(t) | OutlineSource::SelfTextured(t) => t.compile(),
        _ => None,
    };
    let mut last_stamp = None;
    let mut texture_dab = None;
    let mut last_angle = f32::NAN;
    let mut rotation = (0_f64, 1_f64);
    let layer = sdf::rasterize_max(width as u32, height as u32, &stamps, |stamp, pixel| {
        if last_angle != stamp.texture_rotation {
            rotation = f64::from(stamp.texture_rotation).sin_cos();
            last_angle = stamp.texture_rotation;
        }
        let (s, c) = rotation;
        let (dx, dy) = (
            f64::from(pixel[0] - stamp.center[0]),
            f64::from(pixel[1] - stamp.center[1]),
        );
        let (x, y) = (dx * c + dy * s, -dx * s + dy * c);
        let coverage = mask
            .as_ref()
            .map_or_else(
                || program.procedural_coverage_with_geometry(x, y, radius, geometry),
                |mask| mask.coverage(x, y),
            )
            .min(sdf::disk_coverage(stamp, pixel));
        let key = (stamp.center, stamp.texture_rotation);
        if last_stamp != Some(key) {
            last_stamp = Some(key);
            texture_dab = texture.as_ref().map(|texture| {
                let angle =
                    f64::from(stamp.texture_rotation) + f64::from(program.tip.angle).to_radians();
                let (s, c) = angle.sin_cos();
                texture.dab(
                    f64::from(stamp.center[0]) + left as f64,
                    f64::from(stamp.center[1]) + top as f64,
                    1.,
                    s,
                    c,
                )
            });
        }
        let coverage = texture_dab
            .as_ref()
            .map_or(coverage, |texture| {
                texture.apply(
                    coverage,
                    f64::from(pixel[0]) + left as f64,
                    f64::from(pixel[1]) + top as f64,
                )
            })
            .min(sdf::disk_coverage(stamp, pixel));
        Ok(coverage)
    })
    .map_err(error)?;
    let band =
        sdf::compose_outline(&field, &alpha, &layer, outline.width, placement).map_err(error)?;
    Ok(Cache {
        _alpha: alpha,
        _field: field,
        band: Arc::from(band),
    })
}
