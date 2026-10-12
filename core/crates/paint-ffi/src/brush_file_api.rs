use crate::{api::*, error::boundary, file_job::io_error, pointers, runtime};
use paint_task::CancellationToken;
use std::path::Path;

/// Worker-thread synchronous bounded VBR read; returns canonical package JSON.
/// Size-only calls may return BUFFER_TOO_SMALL. File contents are read afresh per call.
/// # Safety
/// Path bytes must be readable for this call; buffer and required must be writable,
/// correctly sized and nonoverlapping. No caller memory survives the call.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_package_read(
    path: *const u8,
    path_length: u64,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(required, 0)?;
        runtime::outside_callback()?;
        let path = pointers::path_utf8(path, path_length)?;
        let package = paint_io::vbr::load(Path::new(&path), &CancellationToken::default())
            .map_err(io_error)?;
        let bytes = package.to_json().map_err(io_error)?;
        pointers::copy_buffer(&bytes, buffer, capacity, required)
    })
}
/// Worker-thread synchronous atomic VBR save of a validated package JSON.
/// # Safety
/// Readable bounded UTF-8 path and JSON bytes are required for this call only.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_package_write(
    path: *const u8,
    path_length: u64,
    json: *const u8,
    json_length: u64,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let path = pointers::path_utf8(path, path_length)?;
        let json = pointers::brush_json(json, json_length)?;
        let package = paint_io::vbr::Package::from_json(json.as_bytes()).map_err(io_error)?;
        paint_io::vbr::save(Path::new(&path), &package, &CancellationToken::default())
            .map_err(io_error)
    })
}
