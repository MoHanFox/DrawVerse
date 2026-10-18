use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
    session::Operation,
};
/// ABI 1.21: begin/preview/reset/cancel/commit a single atomic liquify transaction.
/// Only DAB requires a descriptor. All other actions require null. OK means queued.
/// # Safety
/// Provide initialized readable descriptor/points, writable nonoverlapping sequence and valid
/// serialized core/session ownership. Input is copied, never retained. No callbacks may reenter.
#[no_mangle]
pub unsafe extern "C" fn paint_session_liquify(
    core: *mut PaintCore,
    session: *mut PaintSession,
    kind: u32,
    dab: *const PaintLiquifyDab,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        if kind > PAINT_LIQUIFY_REDO {
            return Err(ApiError::invalid("liquify action"));
        }
        let options = if kind == PAINT_LIQUIFY_DAB {
            let value = pointers::read_sized(dab)?;
            if value.pressure_enabled > 1 || value.reserved != 0 || value.reserved_tail != [0; 3] {
                return Err(ApiError::invalid("liquify flags/reserved"));
            }
            pointers::sized_pointer(&value.from)?;
            pointers::sized_pointer(&value.to)?;
            let model = paint_core::LiquifyDab {
                tool: value.tool,
                radius: value.radius,
                strength: value.strength,
                density: value.density,
                speed: value.speed,
                pressure_enabled: value.pressure_enabled != 0,
                from: crate::point(value.from)?,
                to: crate::point(value.to)?,
                elapsed_ms: value.elapsed_ms,
            };
            model.validate()?;
            Some(model)
        } else {
            if !dab.is_null() {
                return Err(ApiError::invalid("unexpected liquify descriptor"));
            }
            None
        };
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Liquify(kind, options))?;
        pointers::write(out_sequence, sequence)
    })
}
