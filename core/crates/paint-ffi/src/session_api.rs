use crate::{
    api::*,
    error::{boundary, ApiError, ApiResult},
    pointers, runtime,
    session::{Operation, Publication, Session},
};
use paint_core::LayerProperties;
use std::{mem::size_of, ptr, sync::Arc};

fn session(core: *mut PaintCore, handle: *mut PaintSession) -> ApiResult<Arc<Session>> {
    runtime::registry()?.session(core, handle)
}
fn publication(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    expected: u64,
) -> ApiResult<Arc<Publication>> {
    let publication = session(core, handle)?.publication();
    if publication.info.publication != expected {
        return Err(ApiError::new(
            PAINT_BUSY,
            "metadata publication changed; retry enumeration",
        ));
    }
    Ok(publication)
}
/// ABI 1.10: read retained operation metadata from the exact publication.
/// Depth zero is INITIAL or TRUNCATED; remaining indices are chronological operations.
/// # Safety
/// Initialize output size and follow session/publication lifetime contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_session_history_entry(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    expected_publication: u64,
    index: u32,
    out_entry: *mut PaintHistoryEntry,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_entry,
            PaintHistoryEntry {
                struct_size: size_of::<PaintHistoryEntry>() as u32,
                ..Default::default()
            },
        )?;
        let state = publication(core, handle, expected_publication)?;
        let entry = state
            .history
            .get(index as usize)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "history entry not found"))?;
        pointers::write(out_entry, *entry)
    })
}
/// ABI 1.1: create an owning async document session. Creation/destruction run off the UI thread.
/// # Safety
/// Follow writable output, initialized DTO, and core lifetime contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_session_create(
    core: *mut PaintCore,
    desc: *const PaintDocumentDesc,
    out_session: *mut *mut PaintSession,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_session, ptr::null_mut())?;
        runtime::outside_callback()?;
        let desc = pointers::read_sized(desc)?;
        if desc.reserved != [0; 3] {
            return Err(ApiError::invalid("document reserved fields must be zero"));
        }
        if desc.working_space != PAINT_WORKING_LINEAR_SRGB
            || desc.pixel_format != PAINT_STORAGE_RGBA32F_PREMULTIPLIED
        {
            return Err(ApiError::new(
                PAINT_UNSUPPORTED,
                "unsupported document storage",
            ));
        }
        let storage = { runtime::registry()?.storage(core)? };
        let state = Arc::new(Session::with_storage(desc.width, desc.height, storage)?);
        let handle = { runtime::registry()?.create_session(core, Arc::clone(&state))? };
        pointers::write(out_session, handle)
    })
}
/// Stop/cancel/join workers; clear handle after success. Core stays BUSY until sessions are released.
/// # Safety
/// Caller must stop concurrent calls and serialize lifecycle; background thread only.
#[no_mangle]
pub unsafe extern "C" fn paint_session_destroy(
    core: *mut PaintCore,
    handle: *mut *mut PaintSession,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let value = pointers::read(handle)?;
        runtime::registry()?.core(core)?;
        if !value.is_null() {
            let removed = { runtime::registry()?.destroy_session(core, value)? };
            removed.stop();
            drop(removed);
        }
        pointers::write(handle, ptr::null_mut())
    })
}
/// Copy a small command into Rust FIFO. OK means accepted, not completed; poll session_info.
/// Layer ID zero in Begin means the active layer at execution time. Overload rolls a stroke back.
/// # Safety
/// Initialize outer/nested DTO sizes, keep text valid for this call, serialize stroke input order.
#[no_mangle]
pub unsafe extern "C" fn paint_session_submit(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    command: *const PaintCommand,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let command = pointers::read_sized(command)?;
        if command.reserved != [0; 2] {
            return Err(ApiError::invalid("command reserved fields must be zero"));
        }
        let operation = match command.kind {
            PAINT_COMMAND_NEW_DOCUMENT | PAINT_COMMAND_NEW_WHITE_DOCUMENT => {
                if command.width == 0
                    || command.height == 0
                    || command.width > paint_core::MAX_DIMENSION
                    || command.height > paint_core::MAX_DIMENSION
                {
                    return Err(ApiError::invalid("invalid document dimensions"));
                }
                if command.kind == PAINT_COMMAND_NEW_WHITE_DOCUMENT {
                    Operation::NewWhite(command.width, command.height)
                } else {
                    Operation::New(command.width, command.height)
                }
            }
            PAINT_COMMAND_BEGIN_STROKE => {
                let desc = pointers::read_sized(&command.stroke)?;
                let brush = crate::brush(desc)?;
                brush.validate()?;
                let point = crate::point(pointers::read_sized(&command.point)?)?;
                point.validate()?;
                Operation::Begin(brush, point, desc.layer_id)
            }
            PAINT_COMMAND_STROKE_TO => {
                let point = crate::point(pointers::read_sized(&command.point)?)?;
                point.validate()?;
                Operation::Move(point)
            }
            PAINT_COMMAND_END_STROKE => Operation::End,
            PAINT_COMMAND_CANCEL_STROKE => Operation::Cancel(false),
            PAINT_COMMAND_UNDO => Operation::Undo,
            PAINT_COMMAND_REDO => Operation::Redo,
            PAINT_COMMAND_ADD_LAYER => {
                Operation::Add(pointers::utf8(command.text, command.text_length)?)
            }
            PAINT_COMMAND_ADD_MASK => {
                if command.layer_id == 0 {
                    return Err(ApiError::invalid("mask requires explicit owner"));
                }
                Operation::Mask(command.layer_id)
            }
            PAINT_COMMAND_REMOVE_LAYER => Operation::Remove(command.layer_id),
            PAINT_COMMAND_SELECT_LAYER => Operation::Select(command.layer_id),
            PAINT_COMMAND_LAYER_PROPERTIES => {
                if command.visible > 1
                    || !command.opacity.is_finite()
                    || !(0.0..=1.0).contains(&command.opacity)
                {
                    return Err(ApiError::invalid("invalid layer properties"));
                }
                Operation::Properties(
                    command.layer_id,
                    LayerProperties {
                        visible: command.visible == 1,
                        opacity: command.opacity,
                    },
                )
            }
            _ => return Err(ApiError::new(PAINT_UNSUPPORTED, "unknown command kind")),
        };
        let sequence = session(core, handle)?.submit(operation)?;
        pointers::write(out_sequence, sequence)
    })
}
/// ABI 1.11: retarget the per-document history command limit. Accepted like a command and applied
/// in order on the session worker; a smaller limit evicts the oldest undo records and the history
/// boundary becomes unreachable (no fake initial state). Returns the accepted sequence.
/// # Safety
/// Follow session lifetime contracts; serialize with other submit calls for this session.
#[no_mangle]
pub unsafe extern "C" fn paint_session_set_history_limit(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    max_commands: u32,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        if max_commands == 0 {
            return Err(ApiError::invalid("history limit must be positive"));
        }
        let sequence =
            session(core, handle)?.submit(Operation::HistoryLimit(max_commands as usize))?;
        pointers::write(out_sequence, sequence)
    })
}
/// Read published immutable metadata; never waits for the document computation lock.
/// # Safety
/// Follow initialized output DTO and session lifetime contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_session_info(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    out_info: *mut PaintSessionInfo,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_info,
            PaintSessionInfo {
                struct_size: size_of::<PaintSessionInfo>() as u32,
                ..Default::default()
            },
        )?;
        pointers::write(out_info, session(core, handle)?.info())
    })
}
/// Enumerate bottom-to-top from one exact publication; BUSY means retry the complete enumeration.
/// # Safety
/// Follow initialized output DTO, publication, and session lifetime contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_session_layer_info(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    expected_publication: u64,
    index: u32,
    out_info: *mut PaintLayerInfo,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_info,
            PaintLayerInfo {
                struct_size: size_of::<PaintLayerInfo>() as u32,
                ..Default::default()
            },
        )?;
        let publication = publication(core, handle, expected_publication)?;
        let layer = publication
            .layers
            .get(index as usize)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "layer index not found"))?;
        pointers::write(out_info, layer.info)
    })
}
/// Copy UTF-8 without NUL. NULL/0 capacity query returns required bytes.
/// # Safety
/// Follow nonoverlapping caller buffers and exact publication contract.
#[no_mangle]
pub unsafe extern "C" fn paint_session_layer_name(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    expected_publication: u64,
    id: u64,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(required, 0)?;
        let publication = publication(core, handle, expected_publication)?;
        let layer = publication
            .layers
            .iter()
            .find(|layer| layer.info.layer_id == id)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "layer not found"))?;
        pointers::copy_buffer(layer.name.as_bytes(), buffer, capacity, required)
    })
}
/// Read the latest asynchronous execution error. Synchronous submission errors use paint_error_message.
/// # Safety
/// Follow UTF-8 buffer contract; error sequence must match session_info.
#[no_mangle]
pub unsafe extern "C" fn paint_session_error_message(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    error_sequence: u64,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(required, 0)?;
        let message = session(core, handle)?.execution_error(error_sequence)?;
        pointers::copy_buffer(message.as_bytes(), buffer, capacity, required)
    })
}
/// Update one of four output slots without model computation. enabled=0 cancels/releases that view.
/// # Safety
/// Follow initialized viewport, document generation, and nonoverlapping output contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_session_set_viewport(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    viewport: *const PaintViewport,
    out_request: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_request, 0)?;
        runtime::outside_callback()?;
        let viewport = pointers::read_sized(viewport)?;
        let request = session(core, handle)?.viewport(viewport)?;
        pointers::write(out_request, request)
    })
}
/// Return the latest complete frame. BUSY while pending/disabled. No partial frame is exposed.
/// # Safety
/// Follow initialized output and session lifetime contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_session_frame_info(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    view_id: u32,
    out_info: *mut PaintFrameInfo,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_info,
            PaintFrameInfo {
                struct_size: size_of::<PaintFrameInfo>() as u32,
                ..Default::default()
            },
        )?;
        pointers::write(out_info, session(core, handle)?.frame(view_id)?.info())
    })
}
/// Worker-only RGBA8 frame copy. Exact request/frame IDs are required. Padding is not touched.
/// # Safety
/// Follow capacity/stride/nonoverlap contract. No pixels are written until all inputs validate.
#[no_mangle]
pub unsafe extern "C" fn paint_session_read_frame(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    view_id: u32,
    request_id: u64,
    frame_id: u64,
    out_tile: *mut PaintTile,
) -> PaintStatus {
    boundary(|| unsafe {
        let mut tile: PaintTile = pointers::read_sized(out_tile)?;
        tile.struct_size = size_of::<PaintTile>() as u32;
        tile.required = 0;
        tile.width = 0;
        tile.height = 0;
        tile.revision = 0;
        pointers::write(out_tile, tile)?;
        if tile.format != PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED {
            return Err(ApiError::new(
                PAINT_UNSUPPORTED,
                "frame copy requires premultiplied sRGB RGBA8",
            ));
        }
        let frame = session(core, handle)?.frame(view_id)?;
        if frame.request_id != request_id || frame.id != frame_id {
            return Err(ApiError::new(
                PAINT_BUSY,
                "frame replaced; retry frame_info",
            ));
        }
        let width = frame.frame.width;
        let height = frame.frame.height;
        let row_bytes = u64::from(width) * 4;
        if tile.stride < row_bytes {
            return Err(ApiError::invalid("frame stride smaller than row"));
        }
        let required = tile
            .stride
            .checked_mul(u64::from(height - 1))
            .and_then(|n| n.checked_add(row_bytes))
            .ok_or_else(|| ApiError::invalid("frame stride overflow"))?;
        pointers::length(required)?;
        pointers::length(tile.capacity)?;
        tile.required = required;
        pointers::write(out_tile, tile)?;
        if tile.capacity < required {
            return Err(ApiError::new(
                PAINT_BUFFER_TOO_SMALL,
                "frame buffer too small",
            ));
        }
        pointers::valid_pointer(tile.data)?;
        let bytes = frame.bytes();
        for row in 0..height as usize {
            let start = row * row_bytes as usize;
            pointers::copy_row(
                &bytes[start..start + row_bytes as usize],
                tile.data,
                row * tile.stride as usize,
            );
        }
        tile.width = width;
        tile.height = height;
        tile.revision = frame.frame.revision;
        pointers::write(out_tile, tile)
    })
}
