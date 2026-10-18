use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
    session::Operation,
};
use paint_core::{SelectionOperation, WandSelection};
/// Copy and enqueue a magic-wand selection.
/// # Safety
/// Live handles, aligned readable sized request and nonoverlapping writable sequence are required.
#[no_mangle]
pub unsafe extern "C" fn paint_session_select_wand(
    core: *mut PaintCore,
    session: *mut PaintSession,
    request: *const PaintWandSelection,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let r: PaintWandSelection = pointers::read_sized(request)?;
        if r.reserved != [0; 3] || r.antialias > 1 || r.contiguous > 1 || r.all_layers > 1 {
            return Err(ApiError::invalid("wand flags"));
        }
        let operation = match r.operation {
            0 => SelectionOperation::Replace,
            1 => SelectionOperation::Add,
            2 => SelectionOperation::Subtract,
            3 => SelectionOperation::Intersect,
            _ => return Err(ApiError::invalid("wand operation")),
        };
        let options = WandSelection {
            x: r.x,
            y: r.y,
            sample_size: r.sample_size,
            tolerance: r.tolerance,
            antialias: r.antialias == 1,
            contiguous: r.contiguous == 1,
            all_layers: r.all_layers == 1,
            operation,
        };
        options.validate()?;
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Wand(options))?;
        pointers::write(out_sequence, sequence)
    })
}
/// Read a selection step's alpha mask at one publication. Query required length with capacity zero.
/// # Safety
/// Writable nonoverlapping length and capacity bytes are required; bytes may be NULL for zero capacity.
#[no_mangle]
pub unsafe extern "C" fn paint_session_selection_mask(
    core: *mut PaintCore,
    session: *mut PaintSession,
    publication: u64,
    index: u32,
    bytes: *mut u8,
    capacity: u32,
    out_length: *mut u32,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_length, 0)?;
        runtime::outside_callback()?;
        let state = runtime::registry()?.session(core, session)?.publication();
        if publication != state.info.publication {
            return Err(ApiError::new(PAINT_BUSY, "publication changed"));
        }
        let shape = state
            .selection
            .steps()
            .get(index as usize)
            .and_then(|s| s.shape.as_ref())
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "selection mask index"))?;
        let mask = shape
            .mask
            .as_ref()
            .ok_or_else(|| ApiError::invalid("selection is not a mask"))?;
        pointers::write(out_length, mask.bytes() as u32)?;
        if (capacity as usize) < mask.bytes() {
            return Err(ApiError::new(
                PAINT_BUFFER_TOO_SMALL,
                "selection mask capacity",
            ));
        }
        pointers::write_slice(bytes, &mask.alpha_bytes())
    })
}
