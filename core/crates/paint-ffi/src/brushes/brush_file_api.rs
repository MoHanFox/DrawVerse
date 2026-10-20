use crate::{
    api::*,
    brush_preparation::BrushSupport,
    error::{boundary, ApiError},
    file_job::io_error,
    pointers, runtime,
};
use paint_task::CancellationToken;
use std::path::Path;

/// Read a complete ordered-package-v1 handoff, including validated organization.
/// Insufficient capacity does not install resources or publish a catalog.
/// # Safety
/// Live library/readable path and nonoverlapping writable buffer/required required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_package_read_groups(
    library: *mut PaintBrushLibrary,
    path: *const u8,
    path_length: u64,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    unsafe {
        paint_brush_library_package_read_impl(
            library,
            path,
            path_length,
            buffer,
            capacity,
            required,
            BrushSupport::Groups,
        )
    }
}

/// Write a complete ordered-package-v1 handoff with captured resources and sources.
/// # Safety
/// Live library and readable bounded path/JSON required for this call only.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_package_write_groups(
    library: *mut PaintBrushLibrary,
    path: *const u8,
    path_length: u64,
    json: *const u8,
    json_length: u64,
) -> PaintStatus {
    unsafe {
        paint_brush_library_package_write_impl(
            library,
            path,
            path_length,
            json,
            json_length,
            BrushSupport::Groups,
        )
    }
}

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
        reject_wet_package(&package)?;
        reject_taper_package(&package)?;
        reject_outline_package(&package)?;

        if package.brushes.iter().any(|b| b.dual_brush.is_some()) {
            return Err(ApiError::new(
                PAINT_UNSUPPORTED,
                "dual brush requires a library resource context",
            ));
        }
        if package.brushes.iter().any(|b| b.texture.is_some()) {
            return Err(ApiError::new(
                PAINT_UNSUPPORTED,
                "texture requires a library resource context",
            ));
        }
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
    unsafe {
        paint_brush_library_package_read_impl(
            library,
            path,
            path_length,
            buffer,
            capacity,
            required,
            BrushSupport::Legacy,
        )
    }
}
unsafe fn paint_brush_library_package_read_impl(
    library: *mut PaintBrushLibrary,
    path: *const u8,
    path_length: u64,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
    support: BrushSupport,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(required, 0)?;
        runtime::outside_callback()?;
        let path = pointers::path_utf8(path, path_length)?;
        let library = runtime::registry()?.brush_library(library)?;
        let bundle = paint_io::vbr_image::load(Path::new(&path), &CancellationToken::default())
            .map_err(io_error)?;
        if !support.groups() && bundle.organization().is_some() {
            return Err(ApiError::new(
                PAINT_UNSUPPORTED,
                "grouped VBR requires updated native entry",
            ));
        }
        if !support.wet_dual() {
            reject_wet_dual_package(&bundle.package)?;
        }
        if !support.taper() {
            reject_taper_package(&bundle.package)?;
        }
        if !support.stroke_outline() {
            reject_outline_package(&bundle.package)?;
        }
        if !support.wet() {
            reject_wet_package(&bundle.package)?;
        }
        let bytes = if support.groups() {
            paint_io::vbr_handoff::PackageHandoff::from_bundle(&bundle)
                .and_then(|value| value.to_json())
                .map_err(io_error)?
        } else {
            bundle.package.to_json().map_err(io_error)?
        };
        if pointers::length(capacity)? < bytes.len() {
            pointers::write(required, bytes.len() as u64)?;
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
/// Complete wet-preset library package entry (state17/VBR1.15).
/// # Safety
/// Same live handle, readable path/JSON and writable buffer contracts as the legacy entry.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_package_read_wet(
    library: *mut PaintBrushLibrary,
    path: *const u8,
    path_length: u64,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    unsafe {
        paint_brush_library_package_read_impl(
            library,
            path,
            path_length,
            buffer,
            capacity,
            required,
            BrushSupport::Wet,
        )
    }
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
    unsafe {
        paint_brush_library_package_write_impl(
            library,
            path,
            path_length,
            json,
            json_length,
            BrushSupport::Legacy,
        )
    }
}
unsafe fn paint_brush_library_package_write_impl(
    library: *mut PaintBrushLibrary,
    path: *const u8,
    path_length: u64,
    json: *const u8,
    json_length: u64,
    support: BrushSupport,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let path = pointers::path_utf8(path, path_length)?;
        let json = pointers::brush_json(json, json_length)?;
        let library = runtime::registry()?.brush_library(library)?;
        let (package, organization) = if support.groups() {
            let (package, organization) =
                paint_io::vbr_handoff::PackageHandoff::from_json(json.as_bytes())
                    .map_err(io_error)?
                    .into_parts();
            (package, Some(organization))
        } else {
            (
                paint_io::vbr::Package::from_json(json.as_bytes()).map_err(io_error)?,
                None,
            )
        };
        if !support.wet_dual() {
            reject_wet_dual_package(&package)?;
        }
        if !support.taper() {
            reject_taper_package(&package)?;
        }
        if !support.stroke_outline() {
            reject_outline_package(&package)?;
        }
        if !support.wet() {
            reject_wet_package(&package)?;
        }

        let mut assets = std::collections::BTreeMap::new();
        let token = CancellationToken::default();
        let mut sources = Vec::new();
        let source_ids = package
            .brushes
            .iter()
            .filter_map(|b| b.source_resource.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let texture_ids = package
            .brushes
            .iter()
            .filter_map(|b| {
                b.texture
                    .as_ref()
                    .and_then(|t| t.resource_id())
                    .map(str::to_owned)
            })
            .collect::<std::collections::BTreeSet<_>>();
        let secondary_ids = package
            .brushes
            .iter()
            .flat_map(|b| {
                b.dual_brush
                    .as_ref()
                    .into_iter()
                    .flat_map(|d| d.resource_id().into_iter().chain(d.texture_resource_id()))
            })
            .map(str::to_owned)
            .collect::<std::collections::BTreeSet<_>>();
        let outline_ids = package
            .brushes
            .iter()
            .flat_map(|b| {
                b.stroke_outline.as_ref().into_iter().flat_map(|o| {
                    o.existing_resource_id()
                        .into_iter()
                        .chain(o.texture_resource_id())
                })
            })
            .map(str::to_owned)
            .collect::<std::collections::BTreeSet<_>>();
        if package.brushes.iter().any(|b| b.tip_resource.is_some())
            || !source_ids.is_empty()
            || !texture_ids.is_empty()
            || !secondary_ids.is_empty()
            || !outline_ids.is_empty()
        {
            let store = library.resource_store()?;
            if !source_ids.is_empty() {
                sources = store
                    .source_store()
                    .map_err(io_error)?
                    .export_batches(&source_ids, &token)
                    .map_err(io_error)?;
            }
            let mut tips = package
                .brushes
                .iter()
                .filter_map(|b| b.tip_resource.clone())
                .collect::<std::collections::BTreeSet<_>>();
            tips.extend(texture_ids);
            tips.extend(secondary_ids);
            tips.extend(outline_ids);
            for source in &sources {
                tips.extend(source.tip_ids().map(str::to_owned));
            }
            for id in tips {
                assets.insert(id.clone(), store.encoded(&id).map_err(io_error)?);
            }
        }
        let bundle =
            paint_io::vbr_image::ImageBundle::with_sources(package, assets, sources, &token)
                .map_err(io_error)?;
        let bundle = match organization {
            Some(organization) => bundle.with_organization(organization).map_err(io_error)?,
            None => bundle,
        };
        paint_io::vbr_image::save(Path::new(&path), &bundle, &token).map_err(io_error)
    })
}
/// Complete wet-preset library package entry (state17/VBR1.15).
/// # Safety
/// Same live handle, readable path/JSON and writable buffer contracts as the legacy entry.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_package_write_wet(
    library: *mut PaintBrushLibrary,
    path: *const u8,
    path_length: u64,
    json: *const u8,
    json_length: u64,
) -> PaintStatus {
    unsafe {
        paint_brush_library_package_write_impl(
            library,
            path,
            path_length,
            json,
            json_length,
            BrushSupport::Wet,
        )
    }
}

fn reject_wet_dual_package(package: &paint_io::vbr::Package) -> crate::error::ApiResult<()> {
    if package.brushes.iter().any(|b| b.has_wet_dual()) {
        return Err(ApiError::new(
            PAINT_UNSUPPORTED,
            "wet dual color requires updated native entry",
        ));
    }
    Ok(())
}
fn reject_wet_package(package: &paint_io::vbr::Package) -> crate::error::ApiResult<()> {
    if package.brushes.iter().any(|b| b.wet_mix.is_some()) {
        return Err(ApiError::new(
            PAINT_UNSUPPORTED,
            "wet mix requires updated native entry",
        ));
    }
    Ok(())
}
fn reject_taper_package(package: &paint_io::vbr::Package) -> crate::error::ApiResult<()> {
    if package.brushes.iter().any(|b| b.stroke_taper.is_some()) {
        return Err(ApiError::new(
            PAINT_UNSUPPORTED,
            "stroke taper requires updated native entry",
        ));
    }
    Ok(())
}
fn reject_outline_package(package: &paint_io::vbr::Package) -> crate::error::ApiResult<()> {
    if package.brushes.iter().any(|b| b.stroke_outline.is_some()) {
        return Err(ApiError::new(
            PAINT_UNSUPPORTED,
            "stroke outline requires updated native entry",
        ));
    }
    Ok(())
}
/// Complete state41/VBR1.0 resource package read, including closed outline settings.
/// # Safety
/// Live library, readable path and nonoverlapping writable buffer/required are required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_package_read_outline(
    library: *mut PaintBrushLibrary,
    path: *const u8,
    path_length: u64,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    unsafe {
        paint_brush_library_package_read_impl(
            library,
            path,
            path_length,
            buffer,
            capacity,
            required,
            BrushSupport::StrokeOutline,
        )
    }
}
/// Complete state41/VBR1.0 resource package save, including closed outline settings.
/// # Safety
/// Live library and readable bounded path/JSON are required for this call only.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_package_write_outline(
    library: *mut PaintBrushLibrary,
    path: *const u8,
    path_length: u64,
    json: *const u8,
    json_length: u64,
) -> PaintStatus {
    unsafe {
        paint_brush_library_package_write_impl(
            library,
            path,
            path_length,
            json,
            json_length,
            BrushSupport::StrokeOutline,
        )
    }
}
/// Complete state18/VBR1.22 resource package read, including closed taper and wet settings.
/// # Safety
/// Live library, readable path and nonoverlapping writable buffer/required are required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_package_read_taper(
    library: *mut PaintBrushLibrary,
    path: *const u8,
    path_length: u64,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    unsafe {
        paint_brush_library_package_read_impl(
            library,
            path,
            path_length,
            buffer,
            capacity,
            required,
            BrushSupport::Taper,
        )
    }
}
/// Complete state18/VBR1.22 resource package save, including closed taper and wet settings.
/// # Safety
/// Live library and readable bounded path/JSON are required for this call only.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_package_write_taper(
    library: *mut PaintBrushLibrary,
    path: *const u8,
    path_length: u64,
    json: *const u8,
    json_length: u64,
) -> PaintStatus {
    unsafe {
        paint_brush_library_package_write_impl(
            library,
            path,
            path_length,
            json,
            json_length,
            BrushSupport::Taper,
        )
    }
}
/// Complete state19/VBR1.23 resource package read, including closed taper, wet and dual-color settings.
/// # Safety
/// Live library, readable path and nonoverlapping writable buffer/required are required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_package_read_wet_dual(
    library: *mut PaintBrushLibrary,
    path: *const u8,
    path_length: u64,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    unsafe {
        paint_brush_library_package_read_impl(
            library,
            path,
            path_length,
            buffer,
            capacity,
            required,
            BrushSupport::WetDual,
        )
    }
}
/// Complete state19/VBR1.23 resource package save, including closed taper, wet and dual-color settings.
/// # Safety
/// Live library and readable bounded path/JSON are required for this call only.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_package_write_wet_dual(
    library: *mut PaintBrushLibrary,
    path: *const u8,
    path_length: u64,
    json: *const u8,
    json_length: u64,
) -> PaintStatus {
    unsafe {
        paint_brush_library_package_write_impl(
            library,
            path,
            path_length,
            json,
            json_length,
            BrushSupport::WetDual,
        )
    }
}
