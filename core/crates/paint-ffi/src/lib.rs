//! Versioned C ABI. Caller-memory access is isolated in pointers.rs.
//! All synchronous exports are worker-thread-only; no Qt or plugin runtime dependencies.
#[cfg(not(target_pointer_width = "64"))]
compile_error!("DrawVerse ABI v1 supports 64-bit targets only");

mod api;
mod layer_api;
pub use layer_api::*;
mod error;
mod file_api;
mod file_job;
mod pointers;
mod runtime;
mod session;
mod session_api;
mod storage_api;
pub use api::*;
use error::{boundary, query_boundary, ApiError, ApiResult};
pub use file_api::*;
use paint_core::{Brush, BrushMode, Document, InputPoint, LayerProperties, Pixel, Tool};
pub use session_api::*;
use std::{ffi::c_void, mem::size_of, ptr};
pub use storage_api::*;

fn point(value: PaintPoint) -> ApiResult<InputPoint> {
    let tool = match value.tool {
        PAINT_TOOL_PEN => Tool::Pen,
        PAINT_TOOL_ERASER => Tool::Eraser,
        PAINT_TOOL_MOUSE => Tool::Mouse,
        _ => return Err(ApiError::new(PAINT_UNSUPPORTED, "unsupported input tool")),
    };
    Ok(InputPoint {
        x: value.x,
        y: value.y,
        pressure: value.pressure,
        tilt_x: value.tilt_x,
        tilt_y: value.tilt_y,
        rotation: value.rotation,
        tangential_pressure: value.tangential_pressure,
        buttons: value.buttons,
        capabilities: value.capabilities,
        tool,
        timestamp_ns: value.timestamp_ns,
    })
}

fn brush(value: PaintStrokeDesc) -> ApiResult<Brush> {
    if value.reserved != [0; 3] {
        return Err(ApiError::invalid("stroke reserved fields must be zero"));
    }
    let mode = match value.mode {
        PAINT_MODE_PAINT => BrushMode::Paint,
        PAINT_MODE_ERASE => BrushMode::Erase,
        _ => return Err(ApiError::new(PAINT_UNSUPPORTED, "unsupported brush mode")),
    };
    Ok(Brush {
        radius: value.radius,
        opacity: value.opacity,
        spacing: value.spacing,
        color: Pixel::from_straight(value.linear_rgba)?,
        mode,
    })
}

/// Query ABI version; out_version.struct_size must be initialized.
/// # Safety
/// Follow the valid, aligned, nonoverlapping caller-memory contract in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_version(out_version: *mut PaintVersion) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_version,
            PaintVersion {
                struct_size: size_of::<PaintVersion>() as u32,
                major: PAINT_ABI_MAJOR,
                minor: PAINT_ABI_MINOR,
                patch: PAINT_ABI_PATCH,
            },
        )
    })
}

/// Only reports implemented features. No GPU capability is advertised.
/// # Safety
/// Follow the caller-memory and handle contract in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_capabilities(
    core: *mut PaintCore,
    out_caps: *mut PaintCapabilities,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_caps,
            PaintCapabilities {
                struct_size: size_of::<PaintCapabilities>() as u32,
                ..Default::default()
            },
        )?;
        runtime::registry()?.core(core)?;
        pointers::write(
            out_caps,
            PaintCapabilities {
                struct_size: size_of::<PaintCapabilities>() as u32,
                abi_major: PAINT_ABI_MAJOR,
                features: PAINT_FEATURE_DOCUMENT
                    | PAINT_FEATURE_CLIPPING
                    | PAINT_FEATURE_HISTORY
                    | PAINT_FEATURE_LINEAR_TILE_READ
                    | PAINT_FEATURE_EVENTS
                    | PAINT_FEATURE_ASYNC_SESSION
                    | PAINT_FEATURE_CPU_VIEWPORT
                    | PAINT_FEATURE_FILE_IO
                    | PAINT_FEATURE_STORAGE_SETTINGS
                    | PAINT_FEATURE_LAYER_APPEARANCE
                    | PAINT_FEATURE_LAYER_GROUPS
                    | PAINT_FEATURE_MASKS,
                max_dimension: paint_core::MAX_DIMENSION,
                tile_size: paint_core::TILE_SIZE,
                max_read_tile_edge: 256,
                reserved: 0,
            },
        )
    })
}

/// Create an owning core handle; failure writes NULL.
/// # Safety
/// out_core must be writable nonoverlapping pointer storage; see contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_create(out_core: *mut *mut PaintCore) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_core, ptr::null_mut())?;
        runtime::outside_callback()?;
        let core = runtime::registry()?.create_core(storage_api::CoreStorage::system())?;
        pointers::write(out_core, core)
    })
}

/// BUSY while any document/subscription is alive; success writes NULL; NULL is idempotent.
/// # Safety
/// Valid writable pointer-to-handle; caller must serialize destruction; see contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_destroy(core: *mut *mut PaintCore) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let value = pointers::read(core)?;
        if !value.is_null() {
            let removed = { runtime::registry()?.destroy_core(value)? };
            drop(removed);
        }
        pointers::write(core, ptr::null_mut())
    })
}

/// Create a transparent document with one layer. Failure writes NULL.
/// # Safety
/// Follow caller-memory/struct_size/handle contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_new_document(
    core: *mut PaintCore,
    desc: *const PaintDocumentDesc,
    out_doc: *mut *mut PaintDocument,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_doc, ptr::null_mut())?;
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
                "only linear sRGB RGBA32F documents are supported",
            ));
        }
        let storage = { runtime::registry()?.storage(core)? };
        let model = Document::with_storage(
            desc.width,
            desc.height,
            storage.options,
            storage.space.clone(),
        )?;
        let doc = runtime::registry()?.create_document(core, model)?;
        pointers::write(out_doc, doc)
    })
}

/// Release a document (including any uncommitted stroke); success writes NULL.
/// # Safety
/// Follow writable pointer-to-handle and serialized destruction contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_document_destroy(
    core: *mut PaintCore,
    doc: *mut *mut PaintDocument,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let value = pointers::read(doc)?;
        let mut registry = runtime::registry()?;
        registry.core(core)?;
        let removed = if value.is_null() {
            None
        } else {
            Some(registry.destroy_document(core, value)?)
        };
        drop(registry);
        // Free potentially large tile/history allocations outside the registry lock.
        drop(removed);
        pointers::write(doc, ptr::null_mut())
    })
}

/// Read metadata without exposing the internal document layout.
/// # Safety
/// Follow output struct_size and pointer/handle contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_document_info(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
    out_info: *mut PaintDocumentInfo,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_info,
            PaintDocumentInfo {
                struct_size: size_of::<PaintDocumentInfo>() as u32,
                ..Default::default()
            },
        )?;
        let info = runtime::read_document(core, doc, |state, model| {
            let (width, height) = model.dimensions();
            let (undo, redo) = model.history_depth();
            Ok(PaintDocumentInfo {
                struct_size: size_of::<PaintDocumentInfo>() as u32,
                width,
                height,
                layer_count: model.layers().len() as u32,
                document_id: state.id,
                active_layer_id: model.active_layer(),
                revision: model.revision(),
                allocated_tiles: model.tile_count() as u64,
                undo_depth: undo as u32,
                redo_depth: redo as u32,
                stroke_active: u32::from(model.stroke_active()),
                reserved: 0,
            })
        })?;
        pointers::write(out_info, info)
    })
}

/// Enumerate layers bottom-to-top by zero-based index. Invalid index returns NOT_FOUND.
/// # Safety
/// Follow output struct_size and pointer/handle contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_layer_info(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
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
        let info = runtime::read_document(core, doc, |_, model| {
            let layer = model
                .layers()
                .get(index as usize)
                .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "layer index out of range"))?;
            Ok(PaintLayerInfo {
                struct_size: size_of::<PaintLayerInfo>() as u32,
                visible: u32::from(layer.properties().visible),
                layer_id: layer.id(),
                opacity: layer.properties().opacity,
                name_length: layer.name().len() as u64,
                allocated_tiles: layer.tile_count() as u64,
                reserved: 0,
            })
        })?;
        pointers::write(out_info, info)
    })
}

/// UTF-8 bytes without NUL. Query with NULL/0; BUFFER_TOO_SMALL returns required bytes.
/// # Safety
/// buffer/required must be valid nonoverlapping writable memory; see contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_layer_name(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
    layer_id: u64,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(required, 0)?;
        let name = runtime::read_document(core, doc, |_, model| {
            model
                .layer(layer_id)
                .map(|layer| layer.name().as_bytes().to_vec())
                .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "layer does not exist"))
        })?;
        pointers::copy_buffer(&name, buffer, capacity, required)
    })
}

/// Add a UTF-8-named layer above all existing layers; makes it active. Failure writes ID 0.
/// # Safety
/// Follow UTF-8 length, pointer and handle contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_layer_add(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
    name: *const u8,
    name_length: u64,
    out_id: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_id, 0)?;
        let name = pointers::utf8(name, name_length)?;
        let id = runtime::mutate_document(core, doc, |model| Ok(model.add_layer(name)?))?;
        pointers::write(out_id, id)
    })
}

/// Remove a layer with undo history; removing the last layer is invalid.
/// # Safety
/// Follow handle and ordered-command contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_layer_remove(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
    layer_id: u64,
) -> PaintStatus {
    boundary(|| {
        runtime::mutate_document(core, doc, |model| {
            model.remove_layer(layer_id)?;
            Ok(())
        })
    })
}

/// Selection state only; emits a state event, but no undo command or pixel revision.
/// # Safety
/// Follow handle and ordered-command contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_layer_set_active(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
    layer_id: u64,
) -> PaintStatus {
    boundary(|| {
        runtime::mutate_document(core, doc, |model| {
            model.set_active_layer(layer_id)?;
            Ok(())
        })
    })
}

/// visible is exactly 0 or 1; opacity is finite in [0,1]. Undoable.
/// # Safety
/// Follow handle and ordered-command contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_layer_set_properties(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
    layer_id: u64,
    visible: u32,
    opacity: f32,
) -> PaintStatus {
    boundary(|| {
        if visible > 1 {
            return Err(ApiError::invalid("visible must be 0 or 1"));
        }
        runtime::mutate_document(core, doc, |model| {
            model.set_layer_properties(
                layer_id,
                LayerProperties {
                    visible: visible != 0,
                    opacity,
                },
            )?;
            Ok(())
        })
    })
}

/// Begin one undoable stroke on desc.layer_id; invalid input preserves active-layer selection.
/// # Safety
/// Follow input struct_size, handle and ordered-command contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_begin_stroke(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
    desc: *const PaintStrokeDesc,
    first: *const PaintPoint,
) -> PaintStatus {
    boundary(|| unsafe {
        let desc = pointers::read_sized(desc)?;
        let brush = brush(desc)?;
        let first = point(pointers::read_sized(first)?)?;
        runtime::mutate_document(core, doc, |model| {
            let previous = model.active_layer();
            model.set_active_layer(desc.layer_id)?;
            let result = model.begin_stroke(brush, first).map_err(ApiError::from);
            if result.is_err() {
                model.set_active_layer(previous)?;
            }
            result
        })
    })
}

/// Add an ordered tablet sample. Resource-budget failure may roll back the entire stroke.
/// # Safety
/// Follow input struct_size, handle and ordered-command contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_stroke_to(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
    sample: *const PaintPoint,
) -> PaintStatus {
    boundary(|| unsafe {
        let sample = point(pointers::read_sized(sample)?)?;
        runtime::mutate_document(core, doc, |model| {
            model.stroke_to(sample)?;
            Ok(())
        })
    })
}

/// Commit an active stroke. A no-op stroke succeeds without creating history.
/// # Safety
/// Follow handle and ordered-command contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_end_stroke(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
) -> PaintStatus {
    boundary(|| {
        runtime::mutate_document(core, doc, |model| {
            model.end_stroke()?;
            Ok(())
        })
    })
}

/// Roll back the active stroke; no active stroke returns INVALID_ARGUMENT.
/// # Safety
/// Follow handle and ordered-command contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_cancel_stroke(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
) -> PaintStatus {
    boundary(|| {
        runtime::mutate_document(core, doc, |model| {
            model.cancel_stroke()?;
            Ok(())
        })
    })
}

/// Undo one command; empty history succeeds. BUSY during an active stroke.
/// # Safety
/// Follow handle and ordered-command contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_undo(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
) -> PaintStatus {
    boundary(|| {
        runtime::mutate_document(core, doc, |model| {
            model.undo()?;
            Ok(())
        })
    })
}

/// Redo one command; empty history succeeds. BUSY during an active stroke.
/// # Safety
/// Follow handle and ordered-command contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_redo(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
) -> PaintStatus {
    boundary(|| {
        runtime::mutate_document(core, doc, |model| {
            model.redo()?;
            Ok(())
        })
    })
}

/// Read normal-composited linear premultiplied RGBA32F, native endian, at most 256x256.
/// Query capacity with data=NULL/capacity=0; initialize format and stride (bytes).
/// # Safety
/// Follow buffer capacities, nonoverlap, struct_size and handle contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_read_tile(
    core: *mut PaintCore,
    doc: *mut PaintDocument,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
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
        if tile.format != PAINT_TILE_RGBA32F_LINEAR_PREMULTIPLIED {
            return Err(ApiError::new(
                PAINT_UNSUPPORTED,
                "only linear premultiplied RGBA32F tile reads are supported",
            ));
        }
        if width == 0 || height == 0 || width > 256 || height > 256 {
            return Err(ApiError::invalid("read tile dimensions must be 1..=256"));
        }
        let row_bytes = u64::from(width) * 16;
        if tile.stride < row_bytes {
            return Err(ApiError::invalid("tile stride smaller than pixel row"));
        }
        let required = tile
            .stride
            .checked_mul(u64::from(height - 1))
            .and_then(|value| value.checked_add(row_bytes))
            .ok_or_else(|| ApiError::invalid("tile stride multiplication overflow"))?;
        pointers::length(required)?;
        pointers::length(tile.capacity)?;
        let snapshot = runtime::read_document(core, doc, |_, model| {
            let (w, h) = model.dimensions();
            if x.checked_add(width).is_none_or(|end| end > w)
                || y.checked_add(height).is_none_or(|end| end > h)
            {
                return Err(ApiError::invalid("tile ROI is outside the document"));
            }
            Ok(model.snapshot())
        })?;
        tile.required = required;
        pointers::write(out_tile, tile)?;
        if tile.capacity < required {
            return Err(ApiError::new(
                PAINT_BUFFER_TOO_SMALL,
                "tile buffer too small",
            ));
        }
        pointers::valid_pointer(tile.data)?;
        // Stage the bounded ROI before touching caller memory. A cold-page
        // failure must not publish a half-written tile.
        let mut bytes = Vec::with_capacity((row_bytes * u64::from(height)) as usize);
        for row_index in 0..height {
            for pixel in snapshot.read_row(x, y + row_index, width)? {
                for component in pixel.components() {
                    bytes.extend_from_slice(&component.to_ne_bytes());
                }
            }
        }
        snapshot.trim_storage()?;
        for (index, row) in bytes.chunks_exact(row_bytes as usize).enumerate() {
            pointers::copy_row(row, tile.data, index * tile.stride as usize);
        }
        tile.width = width;
        tile.height = height;
        tile.revision = snapshot.revision;
        pointers::write(out_tile, tile)
    })
}

/// Subscribe to pixel/structural and metadata state changes; no initial event is emitted.
/// # Safety
/// Callback and user_data must remain valid until unsubscribe returns; cannot throw/unwind.
#[no_mangle]
pub unsafe extern "C" fn paint_core_subscribe(
    core: *mut PaintCore,
    callback: PaintEventCallback,
    user_data: *mut c_void,
    out_subscription: *mut *mut PaintSubscription,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_subscription, ptr::null_mut())?;
        runtime::outside_callback()?;
        let subscription = runtime::registry()?.subscribe(core, callback, user_data)?;
        pointers::write(out_subscription, subscription)
    })
}

/// Disable and drain callbacks; success writes NULL. Forbidden inside a callback (BUSY).
/// # Safety
/// Follow writable pointer-to-handle and serialized destruction contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_core_unsubscribe(
    core: *mut PaintCore,
    subscription: *mut *mut PaintSubscription,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let value = pointers::read(subscription)?;
        runtime::registry()?.core(core)?;
        if !value.is_null() {
            runtime::unsubscribe(core, value)?;
        }
        pointers::write(subscription, ptr::null_mut())
    })
}

/// Copy the calling thread's last error, without clearing/replacing it. No NUL terminator.
/// # Safety
/// Follow nonoverlapping writable buffer/required and capacity contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_error_message(
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    query_boundary(|| unsafe {
        pointers::copy_buffer(&error::last_error(), buffer, capacity, required)
    })
}

#[cfg(test)]
mod tests;
