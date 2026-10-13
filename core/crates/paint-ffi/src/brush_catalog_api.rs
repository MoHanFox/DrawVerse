use crate::{
    api::*,
    error::{boundary, ApiError, ApiResult},
    pointers, runtime,
    session::Operation,
};
use paint_core::{BrushCatalog, BrushMode, BrushPreset, BrushSettings, BrushTip, Pixel};
use std::{mem::size_of, sync::Arc};
fn catalog<T>(
    library: *mut PaintBrushLibrary,
    revision: u64,
    op: impl FnOnce(&mut BrushCatalog) -> ApiResult<T>,
) -> ApiResult<T> {
    let state = runtime::registry()?.brush_library(library)?;
    let mut library = state.current()?;
    if revision != 0 && revision != library.revision() {
        return Err(ApiError::new(PAINT_BUSY, "brush library revision expired"));
    }
    op(&mut library)
}
fn preset(catalog: &BrushCatalog, index: u32) -> ApiResult<Arc<BrushPreset>> {
    let id = catalog
        .ids()
        .get(index as usize)
        .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "brush preset index"))?;
    catalog.preset(id).ok_or_else(ApiError::internal)
}
fn catalog_mut<T>(
    library: *mut PaintBrushLibrary,
    op: impl FnOnce(&mut BrushCatalog) -> ApiResult<T>,
) -> ApiResult<T> {
    runtime::registry()?.brush_library(library)?.mutate(op)
}
pub(crate) fn validate_tool(t: PaintBrushToolDesc) -> ApiResult<()> {
    if t.reserved != 0
        || t.all_layers > 1
        || t.finger_painting > 1
        || t.pressure_strength > 1
        || !t.strength.is_finite()
        || !(0. ..=1.).contains(&t.strength)
        || t.operation > PAINT_BRUSH_SMUDGE
    {
        return Err(ApiError::invalid("brush tool parameters"));
    }
    paint_core::Brush {
        radius: t.radius,
        color: Pixel::from_straight(t.linear_rgba)?,
        ..paint_core::Brush::default()
    }
    .validate()?;
    Ok(())
}
pub(crate) fn tool_definition(
    preset: &BrushPreset,
    t: PaintBrushToolDesc,
) -> ApiResult<paint_core::BrushDefinition> {
    validate_tool(t)?;
    let mut definition = preset.settings().definition();
    definition.brush.radius = t.radius;
    definition.brush.color = Pixel::from_straight(t.linear_rgba)?;
    definition.brush.mode = match t.operation {
        PAINT_BRUSH_PAINT => BrushMode::Paint,
        PAINT_BRUSH_ERASE => BrushMode::Erase,
        _ => {
            definition.brush.opacity *= t.strength;
            BrushMode::Smudge {
                all_layers: t.all_layers == 1,
                finger_painting: t.finger_painting == 1,
                pressure_strength: t.pressure_strength == 1,
            }
        }
    };
    Ok(definition)
}
fn tip(t: BrushTip) -> PaintBrushTip {
    PaintBrushTip {
        struct_size: size_of::<PaintBrushTip>() as u32,
        hardness: t.hardness,
        roundness: t.roundness,
        angle: t.angle,
        flow: t.flow,
        pressure_size: u32::from(t.pressure_size),
        pressure_flow: u32::from(t.pressure_flow),
        reserved: [0; 3],
    }
}
fn revision_valid(revision: u64) -> ApiResult<()> {
    if revision == 0 {
        Err(ApiError::invalid("brush library revision is required"))
    } else {
        Ok(())
    }
}
/// Create a document-independent library; zero budget selects the default.
/// # Safety
/// out must be a writable, aligned pointer slot.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_create(
    mask_budget_bytes: u64,
    out: *mut *mut PaintBrushLibrary,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out, std::ptr::null_mut())?;
        runtime::outside_callback()?;
        crate::brush_preparation::warm_pool()?;
        let library = if mask_budget_bytes == 0 {
            BrushCatalog::default()
        } else {
            BrushCatalog::with_mask_budget(pointers::length(mask_budget_bytes)?)?
        };
        let handle = runtime::registry()?.create_brush_library(library)?;
        pointers::write(out, handle)
    })
}
/// Destroy a library; already captured stroke definitions remain alive.
/// # Safety
/// library must be a readable/writable aligned pointer slot with a live handle.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_destroy(
    library: *mut *mut PaintBrushLibrary,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let handle = pointers::read(library)?;
        runtime::registry()?.destroy_brush_library(handle)?;
        pointers::write(library, std::ptr::null_mut())
    })
}
/// Read one consistent small library publication.
/// # Safety
/// Live library and writable initialized sized out are required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_info(
    library: *mut PaintBrushLibrary,
    out: *mut PaintBrushLibraryInfo,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out,
            PaintBrushLibraryInfo {
                struct_size: size_of::<PaintBrushLibraryInfo>() as u32,
                ..Default::default()
            },
        )?;
        let info = catalog(library, 0, |c| {
            Ok(PaintBrushLibraryInfo {
                struct_size: size_of::<PaintBrushLibraryInfo>() as u32,
                preset_count: c.ids().len() as u32,
                selected_index: c
                    .ids()
                    .iter()
                    .position(|id| id == c.selected().id())
                    .ok_or_else(ApiError::internal)? as u32,
                revision: c.revision(),
                mask_budget_bytes: c.mask_budget() as u64,
                ..Default::default()
            })
        })?;
        pointers::write(out, info)
    })
}
/// Read preset parameters at an exact library revision; stale reads return BUSY.
/// # Safety
/// Live library and writable initialized sized out are required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_preset_info(
    library: *mut PaintBrushLibrary,
    revision: u64,
    index: u32,
    out: *mut PaintBrushPresetInfo,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out,
            PaintBrushPresetInfo {
                struct_size: size_of::<PaintBrushPresetInfo>() as u32,
                ..Default::default()
            },
        )?;
        revision_valid(revision)?;
        let info = catalog(library, revision, |c| {
            let p = preset(c, index)?;
            let s = p.settings();
            Ok(PaintBrushPresetInfo {
                struct_size: size_of::<PaintBrushPresetInfo>() as u32,
                builtin: u32::from(p.builtin()),
                diameter: s.diameter,
                opacity: s.opacity,
                spacing: s.spacing,
                has_image: u32::from(s.mask.is_some()),
                tip: tip(s.tip),
            })
        })?;
        pointers::write(out, info)
    })
}
/// Read UTF-8 identity (field=0) or name (field=1) with caller-buffer semantics.
/// # Safety
/// Live library, writable required and buffer of capacity bytes are required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_text(
    library: *mut PaintBrushLibrary,
    revision: u64,
    index: u32,
    field: u32,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(required, 0)?;
        revision_valid(revision)?;
        let text = catalog(library, revision, |c| {
            let p = preset(c, index)?;
            match field {
                0 => Ok(p.id().to_owned()),
                1 => Ok(p.name().to_owned()),
                _ => Err(ApiError::invalid("brush text field")),
            }
        })?;
        pointers::copy_buffer(text.as_bytes(), buffer, capacity, required)
    })
}
/// Select an existing preset without changing its parameters.
/// # Safety
/// Live library and readable id_length UTF-8 bytes are required for this call.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_select(
    library: *mut PaintBrushLibrary,
    id: *const u8,
    id_length: u64,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let id = pointers::utf8(id, id_length)?;
        catalog_mut(library, |c| c.select(&id).map_err(Into::into))
    })
}
/// Atomically edit parameters, retaining an existing immutable image resource.
/// # Safety
/// Live library, readable UTF-8 ID and aligned sized settings are required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_edit(
    library: *mut PaintBrushLibrary,
    id: *const u8,
    id_length: u64,
    settings: *const PaintBrushSettings,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let id = pointers::utf8(id, id_length)?;
        let s: PaintBrushSettings = pointers::read_sized(settings)?;
        let t: PaintBrushTip = pointers::read_sized(&s.tip)?;
        if s.reserved != 0
            || s.reserved_tail != 0
            || t.reserved != [0; 3]
            || t.pressure_size > 1
            || t.pressure_flow > 1
        {
            return Err(ApiError::invalid("brush settings reserved/flags"));
        }
        catalog_mut(library, |c| {
            let before = c
                .preset(&id)
                .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "brush preset"))?;
            let settings = BrushSettings {
                diameter: s.diameter,
                opacity: s.opacity,
                spacing: s.spacing,
                tip: BrushTip {
                    hardness: t.hardness,
                    roundness: t.roundness,
                    angle: t.angle,
                    flow: t.flow,
                    pressure_size: t.pressure_size == 1,
                    pressure_flow: t.pressure_flow == 1,
                },
                mask: before.settings().mask.clone(),
            };
            c.edit(&id, settings).map_err(Into::into)
        })
    })
}
/// Atomically rename a preset.
/// # Safety
/// Live library and readable UTF-8 ID/name bytes are required for this call.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_rename(
    library: *mut PaintBrushLibrary,
    id: *const u8,
    id_length: u64,
    name: *const u8,
    name_length: u64,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let id = pointers::utf8(id, id_length)?;
        let name = pointers::utf8(name, name_length)?;
        catalog_mut(library, |c| c.rename(&id, name).map_err(Into::into))
    })
}
/// Copy the active preset to a supplied fresh UUID and name, then select it.
/// # Safety
/// Live library and readable UTF-8 ID/name bytes are required for this call.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_copy(
    library: *mut PaintBrushLibrary,
    id: *const u8,
    id_length: u64,
    name: *const u8,
    name_length: u64,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let id = pointers::utf8(id, id_length)?;
        let name = pointers::utf8(name, name_length)?;
        catalog_mut(library, |c| c.save_copy(id, name).map_err(Into::into))
    })
}
/// Remove a user preset; built-ins are protected.
/// # Safety
/// Live library and readable UTF-8 ID bytes are required for this call.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_remove(
    library: *mut PaintBrushLibrary,
    id: *const u8,
    id_length: u64,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let id = pointers::utf8(id, id_length)?;
        catalog_mut(library, |c| c.remove(&id).map_err(Into::into))
    })
}
/// Capture a preset for one asynchronous paint, erase or smudge stroke.
/// # Safety
/// Live handles, readable ID and sized aligned tool/point, and nonoverlapping
/// writable sequence are required. No caller memory is retained.
#[no_mangle]
pub unsafe extern "C" fn paint_session_begin_preset(
    core: *mut PaintCore,
    session: *mut PaintSession,
    library: *mut PaintBrushLibrary,
    id: *const u8,
    id_length: u64,
    tool: *const PaintBrushToolDesc,
    point: *const PaintPoint,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let id = pointers::utf8(id, id_length)?;
        let t: PaintBrushToolDesc = pointers::read_sized(tool)?;
        if t.reserved != 0
            || t.all_layers > 1
            || t.finger_painting > 1
            || t.pressure_strength > 1
            || !t.strength.is_finite()
            || !(0. ..=1.).contains(&t.strength)
        {
            return Err(ApiError::invalid("brush tool parameters"));
        }
        let preset = catalog_mut(library, |c| {
            c.preset(&id)
                .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "brush preset"))
        })?;
        let mut definition = preset.settings().definition();
        definition.brush.radius = t.radius;
        definition.brush.color = Pixel::from_straight(t.linear_rgba)?;
        definition.brush.mode = match t.operation {
            PAINT_BRUSH_PAINT => BrushMode::Paint,
            PAINT_BRUSH_ERASE => BrushMode::Erase,
            PAINT_BRUSH_SMUDGE => {
                definition.brush.opacity *= t.strength;
                BrushMode::Smudge {
                    all_layers: t.all_layers == 1,
                    finger_painting: t.finger_painting == 1,
                    pressure_strength: t.pressure_strength == 1,
                }
            }
            _ => return Err(ApiError::invalid("brush operation")),
        };
        definition.brush.validate()?;
        let point = crate::point(pointers::read_sized(point)?)?;
        point.validate()?;
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Begin(definition, point, t.layer_id))?;
        pointers::write(out_sequence, sequence)
    })
}

/// Enqueue a bounded parameter-state copy for background migration/compilation.
/// # Safety
/// Live library, readable JSON bytes and nonoverlapping writable sequence required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_prepare_state(
    library: *mut PaintBrushLibrary,
    json: *const u8,
    json_length: u64,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let json = pointers::brush_json(json, json_length)?;
        let library = runtime::registry()?.brush_library(library)?;
        let sequence = library.submit(json)?;
        pointers::write(out_sequence, sequence)
    })
}
/// Read canonical state of one exact prepared record. BUSY means still compiling.
/// # Safety
/// Live library, writable required and buffer of capacity bytes required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_prepared_state(
    library: *mut PaintBrushLibrary,
    sequence: u64,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(required, 0)?;
        let library = runtime::registry()?.brush_library(library)?;
        let snapshot = library.ticket(sequence)?.try_result()?;
        let bytes = paint_io::brush_state::encode(&snapshot).map_err(crate::file_job::io_error)?;
        drop(library.current()?);
        pointers::copy_buffer(&bytes, buffer, capacity, required)
    })
}
/// Read last valid canonical parameter state; pending/failed drafts do not erase it.
/// # Safety
/// Live library, writable required and buffer of capacity bytes required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_current_state(
    library: *mut PaintBrushLibrary,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(required, 0)?;
        let library = runtime::registry()?.brush_library(library)?;
        let state = library.current()?;
        let bytes = paint_io::brush_state::encode(&state).map_err(crate::file_job::io_error)?;
        pointers::copy_buffer(&bytes, buffer, capacity, required)
    })
}
/// Begin after an exact prepared revision, with waiting confined to Rust workers.
/// # Safety
/// Live handles, readable ID, sized aligned tool/point and writable sequence required.
#[no_mangle]
pub unsafe extern "C" fn paint_session_begin_prepared_preset(
    core: *mut PaintCore,
    session: *mut PaintSession,
    library: *mut PaintBrushLibrary,
    preparation: u64,
    id: *const u8,
    id_length: u64,
    tool: *const PaintBrushToolDesc,
    point: *const PaintPoint,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let id = pointers::utf8(id, id_length)?;
        let tool = pointers::read_sized(tool)?;
        validate_tool(tool)?;
        let point = crate::point(pointers::read_sized(point)?)?;
        point.validate()?;
        let ticket = runtime::registry()?
            .brush_library(library)?
            .ticket(preparation)?;
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::BeginPrepared(ticket, id, tool, point))?;
        pointers::write(out_sequence, sequence)
    })
}
