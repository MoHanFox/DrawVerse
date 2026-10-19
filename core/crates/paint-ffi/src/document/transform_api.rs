use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
    session::Operation,
};
/// Queue a transform operation. UPDATE alone requires geometry; input is copied.
/// # Safety
/// Valid owned handles, readable initialized DTO and writable nonoverlapping sequence.
#[no_mangle]
pub unsafe extern "C" fn paint_session_transform(
    core: *mut PaintCore,
    session: *mut PaintSession,
    kind: u32,
    geometry: *const PaintTransform,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        if kind > PAINT_TRANSFORM_ENABLE_RESIDENT {
            return Err(ApiError::invalid("transform kind"));
        }
        let g = if kind == PAINT_TRANSFORM_UPDATE {
            let v = pointers::read_sized(geometry)?;
            if v.warp > 1 || v.reserved != 0 {
                return Err(ApiError::invalid("transform flags"));
            }
            let g = paint_core::TransformGeometry {
                points: std::array::from_fn(|i| [v.points[i * 2], v.points[i * 2 + 1]]),
                warp: v.warp != 0,
                interpolation: v.interpolation,
                skew_h: v.skew_h,
                skew_v: v.skew_v,
            };
            g.validate()?;
            Some(g)
        } else {
            if !geometry.is_null() {
                return Err(ApiError::invalid("unexpected transform descriptor"));
            }
            None
        };
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::Transform(kind, g))?;
        pointers::write(out_sequence, sequence)
    })
}
/// Read the published transform geometry without waiting for computation.
/// # Safety
/// Valid owned handles and writable initialized DTO.
#[no_mangle]
pub unsafe extern "C" fn paint_session_transform_info(
    core: *mut PaintCore,
    session: *mut PaintSession,
    publication: u64,
    out: *mut PaintTransform,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out,
            PaintTransform {
                struct_size: std::mem::size_of::<PaintTransform>() as u32,
                ..Default::default()
            },
        )?;
        let state = runtime::registry()?.session(core, session)?.publication();
        if state.info.publication != publication {
            return Err(ApiError::new(PAINT_BUSY, "publication changed"));
        }
        if let Some(g) = &state.transform {
            pointers::write(
                out,
                PaintTransform {
                    struct_size: std::mem::size_of::<PaintTransform>() as u32,
                    warp: u32::from(g.warp),
                    interpolation: g.interpolation,
                    skew_h: g.skew_h,
                    skew_v: g.skew_v,
                    reserved: 0,
                    points: std::array::from_fn(|i| g.points[i / 2][i % 2]),
                },
            )?;
        }
        Ok(())
    })
}
/// Read an image-layer flag from the same publication as layer metadata.
/// # Safety
/// Valid owned handles and writable output scalar.
#[no_mangle]
pub unsafe extern "C" fn paint_session_layer_is_image(
    core: *mut PaintCore,
    session: *mut PaintSession,
    publication: u64,
    id: u64,
    out: *mut u32,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out, 0)?;
        let state = runtime::registry()?.session(core, session)?.publication();
        if state.info.publication != publication {
            return Err(ApiError::new(PAINT_BUSY, "publication changed"));
        }
        let layer = state
            .layers
            .iter()
            .find(|l| l.info.layer_id == id)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "layer"))?;
        pointers::write(out, u32::from(layer.image))
    })
}

/// Begin one atomic transform for layers or complete group subtrees.
/// # Safety
/// Valid owned handles, readable count-element ID array and writable sequence.
#[no_mangle]
pub unsafe extern "C" fn paint_session_begin_transform_layers(
    core: *mut PaintCore,
    session: *mut PaintSession,
    ids: *const u64,
    count: u64,
    out_sequence: *mut u64,
) -> PaintStatus {
    unsafe { begin_layers(core, session, ids, count, out_sequence, false) }
}

/// ABI 1.64: opt into resident geometry-only transform preview when the stack is compatible.
/// Unsupported stacks keep the pixel preview. Read the common resident preview publication.
/// # Safety
/// Same ownership, readable IDs and writable sequence contract as the ordinary begin entry.
#[no_mangle]
pub unsafe extern "C" fn paint_session_begin_transform_layers_deferred(
    core: *mut PaintCore,
    session: *mut PaintSession,
    ids: *const u64,
    count: u64,
    out_sequence: *mut u64,
) -> PaintStatus {
    unsafe { begin_layers(core, session, ids, count, out_sequence, true) }
}

unsafe fn begin_layers(
    core: *mut PaintCore,
    session: *mut PaintSession,
    ids: *const u64,
    count: u64,
    out_sequence: *mut u64,
    deferred: bool,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        if count == 0 || count > 4096 {
            return Err(ApiError::invalid("transform target count"));
        }
        let ids = pointers::layer_ids(ids, count as u32)?;
        if ids.contains(&0)
            || ids
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != ids.len()
        {
            return Err(ApiError::invalid("transform target IDs"));
        }
        let sequence = runtime::registry()?
            .session(core, session)?
            .submit(Operation::TransformLayers(ids, deferred))?;
        pointers::write(out_sequence, sequence)
    })
}
