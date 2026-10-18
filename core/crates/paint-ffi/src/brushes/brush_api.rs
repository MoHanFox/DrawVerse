use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
    session::Operation,
};
use paint_core::{BrushDefinition, BrushMode, BrushProgram, BrushTip};

/// Begin paint, erase or smudge with a copied, validated procedural tip.
/// # Safety
/// Live handles and aligned readable sized descriptors/point and a nonoverlapping
/// writable sequence are required. Caller memory is never retained.
#[no_mangle]
pub unsafe extern "C" fn paint_session_begin_brush(
    core: *mut PaintCore,
    session: *mut PaintSession,
    request: *const PaintBrushStrokeDesc,
    point: *const PaintPoint,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let r: PaintBrushStrokeDesc = pointers::read_sized(request)?;
        let t: PaintBrushTip = pointers::read_sized(&r.tip)?;
        if t.reserved != [0; 3]
            || t.pressure_size > 1
            || t.pressure_flow > 1
            || r.all_layers > 1
            || r.finger_painting > 1
            || r.pressure_strength > 1
            || !r.strength.is_finite()
            || !(0. ..=1.).contains(&r.strength)
        {
            return Err(ApiError::invalid("brush parameters"));
        }
        let mut brush = crate::brush(pointers::read_sized(&r.brush)?)?;
        brush.validate()?;
        brush.mode = match r.operation {
            PAINT_BRUSH_PAINT if r.brush.mode == PAINT_MODE_PAINT => BrushMode::Paint,
            PAINT_BRUSH_ERASE => BrushMode::Erase,
            PAINT_BRUSH_SMUDGE if r.brush.mode == PAINT_MODE_PAINT => {
                brush.opacity *= r.strength;
                BrushMode::Smudge {
                    all_layers: r.all_layers == 1,
                    finger_painting: r.finger_painting == 1,
                    pressure_strength: r.pressure_strength == 1,
                }
            }
            _ => return Err(ApiError::invalid("brush operation")),
        };
        let definition = BrushDefinition {
            taper: Default::default(),
            outline: Default::default(),
            airbrush: Default::default(),
            wet: Default::default(),
            smoothing: Default::default(),
            write_blend: Default::default(),
            texture: Default::default(),
            dual: Default::default(),
            dynamics: Default::default(),
            brush,
            mask: None,
            tip: BrushTip {
                hardness: t.hardness,
                roundness: t.roundness,
                angle: t.angle,
                flow: t.flow,
                pressure_size: t.pressure_size == 1,
                pressure_flow: t.pressure_flow == 1,
            },
        };
        BrushProgram::compile(definition.clone())?;
        let point = crate::point(pointers::read_sized(point)?)?;
        point.validate()?;
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Begin(
                Box::new(definition),
                point,
                r.brush.layer_id,
            ))?;
        pointers::write(out_sequence, sequence)
    })
}
