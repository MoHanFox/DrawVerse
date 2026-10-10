use crate::{
    api::*,
    error::{boundary, ApiError, ApiResult},
    pointers, runtime,
    session::Operation,
};
use paint_core::{BlendMode, LayerAppearance};
use std::mem::size_of;

/// ABI 1.5: hierarchy metadata from the same immutable publication as layer_info.
/// # Safety
/// Initialize the output DTO and follow session/publication ownership contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_session_layer_hierarchy(
    core: *mut PaintCore,
    session: *mut PaintSession,
    publication: u64,
    layer_id: u64,
    out: *mut PaintLayerHierarchy,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out,
            PaintLayerHierarchy {
                struct_size: size_of::<PaintLayerHierarchy>() as u32,
                ..Default::default()
            },
        )?;
        let state = runtime::registry()?.session(core, session)?.publication();
        if state.info.publication != publication {
            return Err(ApiError::new(PAINT_BUSY, "publication changed"));
        }
        let layer = state
            .layers
            .iter()
            .find(|l| l.info.layer_id == layer_id)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "layer not found"))?;
        pointers::write(out, layer.hierarchy)
    })
}
/// ABI 1.5: FIFO group operation. Name bytes are copied before return.
/// # Safety
/// Initialize request size, keep name alive for this call and provide writable sequence.
#[no_mangle]
pub unsafe extern "C" fn paint_session_group(
    core: *mut PaintCore,
    session: *mut PaintSession,
    request: *const PaintGroupRequest,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let value = pointers::read_sized(request)?;
        if value.layer_id == 0 {
            return Err(ApiError::invalid("group operation needs explicit layer"));
        }
        let operation = match value.kind {
            PAINT_GROUP_WRAP if value.parent_id == 0 => Operation::Group(
                value.layer_id,
                pointers::utf8(value.name, value.name_length)?,
            ),
            PAINT_GROUP_UNGROUP
                if value.parent_id == 0 && value.name.is_null() && value.name_length == 0 =>
            {
                Operation::Ungroup(value.layer_id)
            }
            PAINT_GROUP_REPARENT if value.name.is_null() && value.name_length == 0 => {
                Operation::Reparent(value.layer_id, value.parent_id)
            }
            _ => return Err(ApiError::invalid("invalid group request fields")),
        };
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(operation)?;
        pointers::write(out_sequence, sequence)
    })
}

/// ABI 1.4: background raw layer preview using reserved view slot 2 or 3.
/// Read via the existing frame_info/read_frame functions; disable via set_viewport.
/// # Safety
/// Follow initialized input DTO, output sequence and handle ownership contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_session_set_layer_preview(
    core: *mut PaintCore,
    session: *mut PaintSession,
    layer_id: u64,
    viewport: *const PaintViewport,
    out_request: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_request, 0)?;
        runtime::outside_callback()?;
        if layer_id == 0 {
            return Err(ApiError::invalid("preview requires explicit layer"));
        }
        let view = pointers::read_sized(viewport)?;
        let request = runtime::registry()?
            .session(core, session)?
            .viewport_for_layer(view, layer_id)?;
        pointers::write(out_request, request)
    })
}
/// ABI 1.9: non-mutating full composite blend preview in view slot 0/1.
/// A regular set_viewport replaces/clears the override; requests retain normal generation semantics.
/// # Safety
/// Follow initialized viewport, writable output and session lifetime contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_session_set_blend_preview(
    core: *mut PaintCore,
    session: *mut PaintSession,
    layer_id: u64,
    blend_mode: u32,
    viewport: *const PaintViewport,
    out_request: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_request, 0)?;
        runtime::outside_callback()?;
        let blend = BlendMode::from_id(blend_mode)?;
        let view = pointers::read_sized(viewport)?;
        let request = runtime::registry()?
            .session(core, session)?
            .blend_preview(view, layer_id, blend)?;
        pointers::write(out_request, request)
    })
}
pub(crate) fn dto(a: LayerAppearance) -> PaintLayerAppearance {
    PaintLayerAppearance {
        struct_size: size_of::<PaintLayerAppearance>() as u32,
        locks: a.locks,
        blend_mode: a.blend as u32,
        fill: a.fill,
        dissolve_seed: a.dissolve_seed,
        offset_x: a.offset_x,
        offset_y: a.offset_y,
        reserved: 0,
    }
}
fn model(a: PaintLayerAppearance) -> ApiResult<LayerAppearance> {
    if a.reserved != 0 {
        return Err(ApiError::invalid("layer appearance reserved fields"));
    }
    let value = LayerAppearance {
        fill: a.fill,
        dissolve_seed: a.dissolve_seed,
        locks: a.locks,
        blend: BlendMode::from_id(a.blend_mode)?,
        offset_x: a.offset_x,
        offset_y: a.offset_y,
    };
    value.validate()?;
    Ok(value)
}
/// ABI 1.4: small immutable layer configuration query, tied to exact publication ID.
/// # Safety
/// Follow initialized output DTO, publication and ownership contracts in contracts/abi.md.
#[no_mangle]
pub unsafe extern "C" fn paint_session_layer_appearance(
    core: *mut PaintCore,
    session: *mut PaintSession,
    publication: u64,
    layer_id: u64,
    out: *mut PaintLayerAppearance,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out,
            PaintLayerAppearance {
                struct_size: size_of::<PaintLayerAppearance>() as u32,
                ..Default::default()
            },
        )?;
        let state = runtime::registry()?.session(core, session)?.publication();
        if state.info.publication != publication {
            return Err(ApiError::new(PAINT_BUSY, "publication changed"));
        }
        let layer = state
            .layers
            .iter()
            .find(|l| l.info.layer_id == layer_id)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "layer not found"))?;
        pointers::write(out, dto(layer.appearance))
    })
}
/// ABI 1.4: FIFO appearance edit; validate/copy DTO before return, actor enforces locks.
/// # Safety
/// Follow valid initialized input, writable sequence and session ownership contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_session_set_layer_appearance(
    core: *mut PaintCore,
    session: *mut PaintSession,
    layer_id: u64,
    options: *const PaintLayerAppearance,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let value = model(pointers::read_sized(options)?)?;
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Appearance(layer_id, value))?;
        pointers::write(out_sequence, sequence)
    })
}
/// ABI 1.4: integer relative non-destructive content translation, checked by the actor.
/// # Safety
/// Follow writable sequence, session ownership and serialized destruction contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_session_move_layer(
    core: *mut PaintCore,
    session: *mut PaintSession,
    layer_id: u64,
    dx: i32,
    dy: i32,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        if dx.unsigned_abs() > paint_core::MAX_DIMENSION
            || dy.unsigned_abs() > paint_core::MAX_DIMENSION
        {
            return Err(ApiError::invalid("invalid layer translation"));
        }
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Translate(layer_id, dx, dy))?;
        pointers::write(out_sequence, sequence)
    })
}

/// ABI 1.13: rotate or flip the whole canvas. Quarter turns swap the canvas dimensions; one undo
/// restores both the pixels and the size.
/// # Safety
/// Provide a writable sequence pointer; follow session ownership contract.
#[no_mangle]
pub unsafe extern "C" fn paint_session_transform_canvas(
    core: *mut PaintCore,
    session: *mut PaintSession,
    kind: u32,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        // Validate before touching the document so an unknown kind cannot submit half a queue.
        paint_core::CanvasTransform::from_raw(kind)
            .map_err(|_| ApiError::invalid("unknown canvas transform"))?;
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Transform(kind))?;
        pointers::write(out_sequence, sequence)
    })
}

/// ABI 1.6: atomic subtree reorder/reparent; IDs and placement are copied immediately.
/// # Safety
/// Initialize input DTO and provide writable sequence; follow session ownership contract.
#[no_mangle]
pub unsafe extern "C" fn paint_session_drop_layer(
    core: *mut PaintCore,
    session: *mut PaintSession,
    request: *const PaintLayerDrop,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let r = pointers::read_sized(request)?;
        if r.layer_id == 0 || r.placement > 2 || (r.target_id == 0 && r.placement != 0) {
            return Err(ApiError::invalid("invalid layer drop request"));
        }
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Drop(r.layer_id, r.target_id, r.placement))?;
        pointers::write(out_sequence, sequence)
    })
}

/// ABI 1.7: publication-consistent clipping flag and resolved same-parent base.
/// # Safety
/// Provide initialized writable DTO; follow publication and handle ownership contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_session_layer_clipping(
    core: *mut PaintCore,
    session: *mut PaintSession,
    publication: u64,
    layer_id: u64,
    out: *mut PaintLayerClipping,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out,
            PaintLayerClipping {
                struct_size: size_of::<PaintLayerClipping>() as u32,
                ..Default::default()
            },
        )?;
        let state = runtime::registry()?.session(core, session)?.publication();
        if state.info.publication != publication {
            return Err(ApiError::new(PAINT_BUSY, "publication changed"));
        }
        let layer = state
            .layers
            .iter()
            .find(|l| l.info.layer_id == layer_id)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "layer not found"))?;
        pointers::write(out, layer.clipping)
    })
}
/// ABI 1.7: FIFO clipping edit. base_layer_id must be 0; actor resolves adjacency.
/// # Safety
/// Initialize input DTO and writable sequence; follow session ownership contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_session_set_layer_clipping(
    core: *mut PaintCore,
    session: *mut PaintSession,
    layer_id: u64,
    options: *const PaintLayerClipping,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let value = pointers::read_sized(options)?;
        if layer_id == 0 || value.enabled > 1 || value.base_layer_id != 0 {
            return Err(ApiError::invalid("invalid clipping edit"));
        }
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Clipping(layer_id, value.enabled == 1))?;
        pointers::write(out_sequence, sequence)
    })
}
