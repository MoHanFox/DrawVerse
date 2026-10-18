use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
};

/// Opt in to full state20 preparation, including ordered groups and all prior preset fields.
/// This capability does not enable grouped package read/write through legacy entries.
/// # Safety
/// Live library/readable bounded JSON and nonoverlapping writable sequence are required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_prepare_state_groups(
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
        let sequence = library.submit_groups(json)?;
        pointers::write(out_sequence, sequence)
    })
}

/// Opt in to complete state19 preparation, including closed wet dual, taper and wet parameters.
/// # Safety
/// Live library/readable bounded JSON and nonoverlapping writable sequence are required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_prepare_state_wet_dual(
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
        let sequence = library.submit_wet_dual(json)?;
        pointers::write(out_sequence, sequence)
    })
}

/// Read both idle reservoirs in one snapshot. BUSY/error clears the complete output.
/// # Safety
/// out must be aligned and writable for a complete PaintWetToolInfoDual.
#[no_mangle]
pub unsafe extern "C" fn paint_wet_tool_info_dual(
    tool: *mut PaintWetTool,
    out: *mut PaintWetToolInfoDual,
) -> PaintStatus {
    boundary(|| unsafe {
        let mut info = PaintWetToolInfoDual {
            struct_size: std::mem::size_of::<PaintWetToolInfoDual>() as u32,
            ..Default::default()
        };
        pointers::write(out, info)?;
        runtime::outside_callback()?;
        let tool = runtime::registry()?.wet_tool(tool)?;
        if let Some(ink) = tool.snapshot_ink()? {
            let primary = ink.primary();
            info.flags = PAINT_WET_TOOL_INITIALIZED;
            info.primary_revision = primary.revision();
            info.primary_edge = primary.edge() as u32;
            info.primary_center_linear_premul_rgba = primary.sample(0.5, 0.5).components();
            if let Some(secondary) = ink.secondary() {
                info.flags |= PAINT_WET_TOOL_SECONDARY_INITIALIZED;
                info.secondary_revision = secondary.revision();
                info.secondary_edge = secondary.edge() as u32;
                info.secondary_center_linear_premul_rgba = secondary.sample(0.5, 0.5).components();
            }
        }
        pointers::write(out, info)
    })
}
/// Atomically load an exact prepared wet preset, including independent dual roles.
/// BUSY never waits for compilation. Ordinary wet presets preserve inactive secondary ink.
/// # Safety
/// Live handles, readable bounded ID and both complete sized descriptors are required.
#[no_mangle]
pub unsafe extern "C" fn paint_wet_tool_reload_prepared_dual(
    tool: *mut PaintWetTool,
    library: *mut PaintBrushLibrary,
    preparation: u64,
    id: *const u8,
    id_length: u64,
    desc: *const PaintWetLoadDesc,
    background: *const PaintBrushBackgroundDesc,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let d: PaintWetLoadDesc = pointers::read_sized(desc)?;
        let bg: PaintBrushBackgroundDesc = pointers::read_sized(background)?;
        if d.reserved != 0
            || d.reserved_tail != 0
            || !d.radius.is_finite()
            || !(0.5..=paint_core::MAX_BRUSH_RADIUS).contains(&d.radius)
            || bg.reserved != 0
        {
            return Err(ApiError::invalid("wet dual load descriptor"));
        }
        let color = paint_core::Pixel::from_straight(d.linear_rgba)?;
        let background = paint_core::Pixel::from_straight(bg.linear_rgba)?;
        let id = pointers::utf8(id, id_length)?;
        let (tool, ticket) = {
            let registry = runtime::registry()?;
            (
                registry.wet_tool(tool)?,
                registry.brush_library(library)?.ticket(preparation)?,
            )
        };
        if tool.is_busy() {
            return Err(ApiError::new(PAINT_BUSY, "wet tool is painting"));
        }
        let catalog = ticket.try_result()?;
        let preset = catalog
            .preset(&id)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "wet dual load preset"))?;
        let s = preset.settings();
        if !s.wet.enabled() {
            return Err(ApiError::invalid("wet load preset disabled"));
        }
        if s.wet.dual_color() && s.dual.enabled() && s.dual.color().enabled() {
            let secondary = if s.dual.color().use_background() {
                background
            } else {
                color
            };
            tool.reload_dual(
                s.wet.mix(),
                [d.radius * 2., s.dual.nominal_diameter(d.radius)?],
                [color, secondary],
            )?;
        } else {
            tool.reload(s.wet.mix(), d.radius * 2., color)?;
        }
        Ok(())
    })
}
