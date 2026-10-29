use crate::{
    api::*,
    error::{boundary, ApiError},
    pointers, runtime,
    session::Operation,
};
/// Compute bounded shared display geometry from absolute input, without a session,
/// pixel reads, IO, history or actor waits. Query with capacity_floats=0/NULL.
/// BUFFER_TOO_SMALL publishes the descriptor and leaves xyuv untouched.
/// # Safety
/// Readable initialized geometry, writable initialized out_info, and writable
/// nonoverlapping capacity_floats f32s. No pointer is retained after return.
#[no_mangle]
pub unsafe extern "C" fn paint_transform_display_geometry(
    geometry: *const PaintTransform,
    out_info: *mut PaintTransformDisplayGeometry,
    xyuv: *mut f32,
    capacity_floats: u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_info,
            PaintTransformDisplayGeometry {
                struct_size: std::mem::size_of::<PaintTransformDisplayGeometry>() as u32,
                version: 1,
                ..Default::default()
            },
        )?;
        if capacity_floats > 145 * 145 * 4 {
            return Err(ApiError::invalid("display geometry buffer limit"));
        }
        let v = pointers::read_sized(geometry)?;
        if v.warp > 1 || v.reserved != 0 {
            return Err(ApiError::invalid("transform flags"));
        }
        let value = paint_core::TransformGeometry {
            points: std::array::from_fn(|i| [v.points[i * 2], v.points[i * 2 + 1]]),
            warp: v.warp != 0,
            interpolation: v.interpolation,
            skew_h: v.skew_h,
            skew_v: v.skew_v,
        }
        .display_geometry()?;
        pointers::write(
            out_info,
            PaintTransformDisplayGeometry {
                struct_size: std::mem::size_of::<PaintTransformDisplayGeometry>() as u32,
                version: 1,
                columns: value.columns,
                rows: value.rows,
                vertex_bytes: (value.vertices.len() * 16) as u64,
                inverse: value.inverse,
                reserved: 0,
            },
        )?;
        if capacity_floats < (value.vertices.len() * 4) as u64 {
            return Err(ApiError::new(
                PAINT_BUFFER_TOO_SMALL,
                "display geometry buffer too small",
            ));
        }
        pointers::write_slice(xyuv, value.vertices.as_flattened())
    })
}
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
        if kind > PAINT_TRANSFORM_FLIP_IMAGE_VERTICAL {
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

/// Resize canvas in one queued authoritative history transaction.
/// # Safety
/// Owned handles, initialized readable DTO and nonoverlapping writable sequence.
#[no_mangle]
pub unsafe extern "C" fn paint_session_resize_canvas(
    core: *mut PaintCore,
    session: *mut PaintSession,
    request: *const PaintCanvasResize,
    out_sequence: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_sequence, 0)?;
        runtime::outside_callback()?;
        let v = pointers::read_sized(request)?;
        if v.flags > 1
            || v.reserved != 0
            || v.width == 0
            || v.height == 0
            || v.width > 1_000_000
            || v.height > 1_000_000
            || !(1..=12000).contains(&v.dpi)
            || v.x.unsigned_abs() > 1_000_000
            || v.y.unsigned_abs() > 1_000_000
        {
            return Err(ApiError::invalid("canvas bounds/dpi/flags"));
        }
        let fill = paint_core::Pixel::from_straight(v.fill)?;
        let sequence =
            runtime::registry()?
                .session(core, session)?
                .submit(Operation::ResizeCanvas(
                    v.x,
                    v.y,
                    v.width,
                    v.height,
                    v.dpi,
                    v.flags != 0,
                    fill,
                ))?;
        pointers::write(out_sequence, sequence)
    })
}
/// Read DPI from the same immutable publication as session dimensions.
/// # Safety
/// Valid owned handles and writable output.
#[no_mangle]
pub unsafe extern "C" fn paint_session_canvas_dpi(
    core: *mut PaintCore,
    session: *mut PaintSession,
    publication: u64,
    out_dpi: *mut u32,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_dpi, 0)?;
        let state = runtime::registry()?.session(core, session)?.publication();
        if state.info.publication != publication {
            return Err(ApiError::new(PAINT_BUSY, "publication changed"));
        }
        pointers::write(out_dpi, state.dpi)
    })
}
