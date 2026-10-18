use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
    session::Operation,
};
use paint_core::{BucketFill, Pixel};

/// Copy, validate and enqueue an atomic fill; no pixel work on the submitting thread.
/// # Safety
/// Caller must provide aligned, valid, nonoverlapping sized request and writable sequence.
/// Session/core must remain live; caller-memory access follows contracts/ffi/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_session_bucket_fill(
    core: *mut PaintCore,
    session: *mut PaintSession,
    request: *const PaintBucketFill,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let request = pointers::read_sized(request)?;
        if request.reserved != 0
            || request.reserved_tail != [0; 3]
            || request.antialias > 1
            || request.contiguous > 1
            || request.all_layers > 1
        {
            return Err(ApiError::invalid("bucket flags/reserved"));
        }
        let options = BucketFill {
            x: request.x,
            y: request.y,
            color: Pixel::from_straight(request.linear_rgba)?,
            opacity: request.opacity,
            tolerance: request.tolerance,
            antialias: request.antialias == 1,
            contiguous: request.contiguous == 1,
            all_layers: request.all_layers == 1,
        };
        options.validate()?;
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Bucket(options))?;
        pointers::write(out_sequence, sequence)
    })
}
