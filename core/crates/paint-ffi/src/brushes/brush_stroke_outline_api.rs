use crate::{api::*, error::boundary, pointers, runtime};

/// Opt in to complete state41 preparation, including closed stroke outline settings.
/// # Safety
/// Live library/readable bounded JSON and nonoverlapping writable sequence are required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_prepare_state_outline(
    library: *mut PaintBrushLibrary,
    json: *const u8,
    length: u64,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let json = pointers::brush_json(json, length)?;
        let library = runtime::registry()?.brush_library(library)?;
        let sequence = library.submit_stroke_outline(json)?;
        pointers::write(out_sequence, sequence)
    })
}
