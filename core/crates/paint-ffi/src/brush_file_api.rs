use crate::{
    api::*,
    error::{boundary, ApiError},
    file_job::io_error,
    pointers, runtime,
};
use paint_task::CancellationToken;
use std::path::Path;

/// Background ABR baseline read. Bounded ABR-only JSON; no catalog publication.
/// Size-only calls parse afresh; insufficient output capacity writes no resources.
/// # Safety
/// Live library/readable path and nonoverlapping writable buffer/required required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_abr_read(
    library: *mut PaintBrushLibrary,
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
        let library = runtime::registry()?.brush_library(library)?;
        let budget = library.current()?.mask_budget();
        let batch = paint_io::abr_import::ImportBatch::read(
            Path::new(&path),
            budget,
            &CancellationToken::default(),
        )
        .map_err(io_error)?;
        let bytes = batch.to_json().map_err(io_error)?;
        if pointers::length(capacity)? < bytes.len() {
            pointers::write(required, bytes.len() as u64)?;
            return Err(ApiError::new(
                PAINT_BUFFER_TOO_SMALL,
                "ABR JSON buffer too small",
            ));
        }
        pointers::valid_pointer(buffer)?;
        library.import_abr(batch)?;
        pointers::copy_buffer(&bytes, buffer, capacity, required)
    })
}

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

/// Background VBR 1.0/1.1 read, storing verified picture resources in the library.
/// Does not modify the catalogue; caller publishes returned JSON through preparation.
/// # Safety
/// Live library, readable path, writable nonoverlapping buffer/required required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_package_read(
    library: *mut PaintBrushLibrary,
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
        let library = runtime::registry()?.brush_library(library)?;
        let bundle = paint_io::vbr_image::load(Path::new(&path), &CancellationToken::default())
            .map_err(io_error)?;
        if !bundle.sources().is_empty() {
            return Err(ApiError::new(
                PAINT_UNSUPPORTED,
                "VBR source resources are not supported by this interface",
            ));
        }
        let bytes = bundle.package.to_json().map_err(io_error)?;
        pointers::write(required, bytes.len() as u64)?;
        if pointers::length(capacity)? < bytes.len() {
            return Err(ApiError::new(
                PAINT_BUFFER_TOO_SMALL,
                "VBR JSON buffer too small",
            ));
        }
        pointers::valid_pointer(buffer)?;
        library.import_bundle(&bundle)?;
        pointers::copy_buffer(&bytes, buffer, capacity, required)
    })
}
/// Background VBR save using immutable resource identities from the library store.
/// # Safety
/// Live library and readable bounded path/JSON bytes required for this call only.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_package_write(
    library: *mut PaintBrushLibrary,
    path: *const u8,
    path_length: u64,
    json: *const u8,
    json_length: u64,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let path = pointers::path_utf8(path, path_length)?;
        let json = pointers::brush_json(json, json_length)?;
        let library = runtime::registry()?.brush_library(library)?;
        let package = paint_io::vbr::Package::from_json(json.as_bytes()).map_err(io_error)?;
        let mut assets = std::collections::BTreeMap::new();
        let token = CancellationToken::default();
        if package.brushes.iter().any(|b| b.tip_resource.is_some()) {
            let store = library.resource_store()?;
            for preset in &package.brushes {
                if let Some(id) = &preset.tip_resource {
                    if !assets.contains_key(id) {
                        assets.insert(id.clone(), store.encoded(id).map_err(io_error)?);
                    }
                }
            }
        }
        let bundle =
            paint_io::vbr_image::ImageBundle::new(package, assets, &token).map_err(io_error)?;
        paint_io::vbr_image::save(Path::new(&path), &bundle, &token).map_err(io_error)
    })
}
