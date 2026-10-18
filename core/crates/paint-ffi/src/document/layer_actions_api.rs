use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
    session::Operation,
};

/// ABI 1.20: atomic layer operation. Paste alone uses bytes and target; other actions use IDs.
/// OK means queued. Copy returns bytes via read_layer_clipboard after its sequence completes.
/// # Safety
/// IDs/bytes must be readable for the declared lengths, with writable nonoverlapping sequence.
/// No caller pointers are retained. Follow core/session ownership and callback restrictions.
#[no_mangle]
pub unsafe extern "C" fn paint_session_layer_action(
    core: *mut PaintCore,
    session: *mut PaintSession,
    kind: u32,
    ids: *const u64,
    count: u32,
    target: u64,
    bytes: *const u8,
    length: u64,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let operation = if kind == PAINT_LAYER_ACTION_PASTE {
            if count != 0 || !ids.is_null() {
                return Err(ApiError::invalid("paste fields"));
            }
            Operation::PasteLayers(pointers::layer_clipboard_bytes(bytes, length)?, target)
        } else {
            if target != 0 || length != 0 || !bytes.is_null() {
                return Err(ApiError::invalid("layer action fields"));
            }
            let ids = pointers::layer_ids(ids, count)?;
            match kind {
                PAINT_LAYER_ACTION_COPY => Operation::CopyLayers(ids),
                PAINT_LAYER_ACTION_DUPLICATE => Operation::DuplicateLayers(ids),
                PAINT_LAYER_ACTION_MERGE => Operation::MergeLayers(ids),
                _ => return Err(ApiError::invalid("layer action kind")),
            }
        };
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(operation)?;
        pointers::write(out_sequence, sequence)
    })
}
/// Latest successful copy result remains owned by the session. Query with null buffer/zero capacity.
/// # Safety
/// Output required must be writable, buffer writable for capacity, outputs nonoverlapping.
#[no_mangle]
pub unsafe extern "C" fn paint_session_read_layer_clipboard(
    core: *mut PaintCore,
    session: *mut PaintSession,
    sequence: u64,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(required, 0)?;
        let bytes = runtime::registry()?
            .session(core, session)?
            .layer_clipboard(sequence)?;
        pointers::copy_buffer(&bytes, buffer, capacity, required)
    })
}
