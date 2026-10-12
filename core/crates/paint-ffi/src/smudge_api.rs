use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
    session::Operation,
};
use paint_core::BrushMode;
/// Begin an ordinary asynchronous brush stroke with smudge behavior.
/// # Safety
/// Live handles, aligned readable sized descriptors/point and nonoverlapping writable sequence required.
#[no_mangle]
pub unsafe extern "C" fn paint_session_begin_smudge(
    core: *mut PaintCore,
    session: *mut PaintSession,
    request: *const PaintSmudgeStrokeDesc,
    point: *const PaintPoint,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let r: PaintSmudgeStrokeDesc = pointers::read_sized(request)?;
        if r.reserved != [0; 3]
            || r.all_layers > 1
            || r.finger_painting > 1
            || r.pressure_strength > 1
            || !r.strength.is_finite()
            || !(0. ..=1.).contains(&r.strength)
            || r.brush.mode != PAINT_MODE_PAINT
        {
            return Err(ApiError::invalid("smudge parameters"));
        }
        let mut brush = crate::brush(pointers::read_sized(&r.brush)?)?;
        brush.validate()?;
        brush.opacity *= r.strength;
        brush.mode = BrushMode::Smudge {
            all_layers: r.all_layers == 1,
            finger_painting: r.finger_painting == 1,
            pressure_strength: r.pressure_strength == 1,
        };
        let point = crate::point(pointers::read_sized(point)?)?;
        point.validate()?;
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Begin(brush, point, r.brush.layer_id))?;
        pointers::write(out_sequence, sequence)
    })
}
