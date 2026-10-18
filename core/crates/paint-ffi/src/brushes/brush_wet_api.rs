use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
    session::Operation,
};
use std::mem::size_of;
/// Create an independent reservoir. At most 64 external tools may remain alive.
/// # Safety
/// out must be aligned and writable; handles must never be dereferenced.
#[no_mangle]
pub unsafe extern "C" fn paint_wet_tool_create(out: *mut *mut PaintWetTool) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out, std::ptr::null_mut())?;
        runtime::outside_callback()?;
        let handle = runtime::registry()?.create_wet_tool()?;
        pointers::write(out, handle)
    })
}
/// Release the token. Bound sessions retain ownership until unbound or destroyed.
/// # Safety
/// tool must point to a live writable handle; no caller memory is retained.
#[no_mangle]
pub unsafe extern "C" fn paint_wet_tool_destroy(tool: *mut *mut PaintWetTool) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let handle = pointers::read(tool)?;
        runtime::registry()?.destroy_wet_tool(handle)?;
        pointers::write(tool, std::ptr::null_mut())
    })
}
/// Manual idle cleaning. Active painting returns BUSY and preserves ink.
/// # Safety
/// A live typed tool token is required.
#[no_mangle]
pub unsafe extern "C" fn paint_wet_tool_clean(tool: *mut PaintWetTool) -> PaintStatus {
    boundary(|| {
        runtime::outside_callback()?;
        let tool = runtime::registry()?.wet_tool(tool)?;
        tool.clean()?;
        Ok(())
    })
}
/// Read idle reservoir geometry/revision and center premultiplied linear RGBA.
/// BUSY/error clears output. The center sample is not a grid average.
/// # Safety
/// out must be aligned and writable for a complete PaintWetToolInfo.
#[no_mangle]
pub unsafe extern "C" fn paint_wet_tool_info(
    tool: *mut PaintWetTool,
    out: *mut PaintWetToolInfo,
) -> PaintStatus {
    boundary(|| unsafe {
        let mut info = PaintWetToolInfo {
            struct_size: size_of::<PaintWetToolInfo>() as u32,
            ..Default::default()
        };
        pointers::write(out, info)?;
        runtime::outside_callback()?;
        let tool = runtime::registry()?.wet_tool(tool)?;
        if let Some(carrier) = tool.snapshot()? {
            info.flags = PAINT_WET_TOOL_INITIALIZED;
            info.carrier_revision = carrier.revision();
            info.edge = carrier.edge() as u32;
            info.center_linear_premul_rgba = carrier.sample(0.5, 0.5).components();
        }
        pointers::write(out, info)
    })
}
/// Load using one exact prepared wet preset. BUSY never waits for compilation.
/// # Safety
/// Live handles, readable ID/sized descriptor and valid caller memory required.
#[no_mangle]
pub unsafe extern "C" fn paint_wet_tool_reload_prepared(
    tool: *mut PaintWetTool,
    library: *mut PaintBrushLibrary,
    preparation: u64,
    id: *const u8,
    id_length: u64,
    desc: *const PaintWetLoadDesc,
) -> PaintStatus {
    boundary(|| unsafe {
        runtime::outside_callback()?;
        let d: PaintWetLoadDesc = pointers::read_sized(desc)?;
        if d.reserved != 0
            || d.reserved_tail != 0
            || !d.radius.is_finite()
            || !(0.5..=256.).contains(&d.radius)
        {
            return Err(ApiError::invalid("wet load descriptor"));
        }
        let color = paint_core::Pixel::from_straight(d.linear_rgba)?;
        let id = pointers::utf8(id, id_length)?;
        let (tool, ticket) = {
            let registry = runtime::registry()?;
            (
                registry.wet_tool(tool)?,
                registry.brush_library(library)?.ticket(preparation)?,
            )
        };
        let catalog = ticket.try_result()?;
        let preset = catalog
            .preset(&id)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "wet load preset"))?;
        if preset.settings().wet.dual_color() {
            return Err(ApiError::new(
                PAINT_UNSUPPORTED,
                "wet dual load requires updated native entry",
            ));
        }
        if !preset.settings().wet.enabled() {
            return Err(ApiError::invalid("wet load preset disabled"));
        }
        tool.reload(preset.settings().wet.mix(), d.radius * 2., color)?;
        Ok(())
    })
}
/// Enqueue reservoir binding. Active session painting rejects binding asynchronously.
/// # Safety
/// Live matching core/session and tool tokens plus writable sequence required.
#[no_mangle]
pub unsafe extern "C" fn paint_session_bind_wet_tool(
    core: *mut PaintCore,
    session: *mut PaintSession,
    tool: *mut PaintWetTool,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let (session, tool) = {
            let registry = runtime::registry()?;
            (registry.session(core, session)?, registry.wet_tool(tool)?)
        };
        let sequence = session.submit(Operation::BindWetTool(tool))?;
        pointers::write(out_sequence, sequence)
    })
}
/// Opt in to complete state17 preparation, including closed wet parameters.
/// # Safety
/// Live library/readable bounded JSON and nonoverlapping writable sequence required.
#[no_mangle]
pub unsafe extern "C" fn paint_brush_library_prepare_state_wet(
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
        let sequence = library.submit_wet(json)?;
        pointers::write(out_sequence, sequence)
    })
}
