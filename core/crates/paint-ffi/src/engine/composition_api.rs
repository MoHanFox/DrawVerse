//! Versioned immutable display-plan transport. All tile reads are worker-only.
use super::composition::{self, Charge, Request};
use crate::{
    api::*,
    error::{boundary, ApiError, ApiResult},
    pointers, runtime,
    session::Session,
};
use paint_render::composition::{
    NodeKind, MAX_METADATA_BYTES, MAX_NODES, MAX_PIN_BYTES, MAX_TILES,
};
use std::{mem::size_of, sync::Arc};

fn session(core: *mut PaintCore, handle: *mut PaintSession) -> ApiResult<Arc<Session>> {
    runtime::registry()?.session(core, handle)
}

/// Process-wide current/peak accounting. Plan charges survive cancellation
/// while a worker owns immutable data; peaks are not reset by this query.
/// # Safety
/// Initialize out_usage.struct_size and follow the nonoverlapping DTO contract.
#[no_mangle]
pub unsafe extern "C" fn paint_core_composition_usage(
    core: *mut PaintCore,
    out_usage: *mut PaintCompositionUsage,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_usage,
            PaintCompositionUsage {
                struct_size: size_of::<PaintCompositionUsage>() as u32,
                ..Default::default()
            },
        )?;
        runtime::registry()?.core(core)?;
        pointers::write(out_usage, composition::usage())
    })
}
unsafe fn key(value: *const PaintCompositionKey) -> ApiResult<PaintCompositionKey> {
    let mut key = unsafe { pointers::read_sized(value)? };
    key.struct_size = size_of::<PaintCompositionKey>() as u32;
    Ok(key)
}

/// ABI1.81: query display-plan protocol independently of numeric GPU support.
/// Version 1 transports linear premultiplied little-endian RGBA32F tiles only.
/// # Safety
/// Initialize out_caps.struct_size; follow aligned nonoverlapping DTO contracts.
#[no_mangle]
pub unsafe extern "C" fn paint_core_composition_capabilities(
    core: *mut PaintCore,
    version: u32,
    out_caps: *mut PaintCompositionCapabilities,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_caps,
            PaintCompositionCapabilities {
                struct_size: size_of::<PaintCompositionCapabilities>() as u32,
                ..Default::default()
            },
        )?;
        runtime::registry()?.core(core)?;
        if version != PAINT_COMPOSITION_VERSION {
            return Err(ApiError::new(PAINT_UNSUPPORTED, "composition version"));
        }
        pointers::write(
            out_caps,
            PaintCompositionCapabilities {
                struct_size: size_of::<PaintCompositionCapabilities>() as u32,
                version,
                features: paint_render::composition::FEATURES,
                max_nodes: MAX_NODES as u32,
                max_tiles: MAX_TILES as u32,
                max_leases_per_session: 2,
                tile_edge: paint_core::TILE_SIZE,
                max_pinned_bytes: MAX_PIN_BYTES as u64,
                max_metadata_bytes: MAX_METADATA_BYTES as u64,
                max_read_bytes: paint_core::TILE_BYTES as u64,
                process_plan_bytes: composition::PROCESS_PLAN_BYTES as u64,
                process_read_bytes: composition::PROCESS_READ_BYTES as u64,
                reserved: [0; 2],
            },
        )
    })
}

/// Copy into the actor's separate bounded display mailbox, outside the input
/// command FIFO. OK means accepted, not ready.
/// Request IDs and device epochs are monotonic per view, independent of revision.
/// Up to two ready/pending leases are retained; superseded pending work coalesces.
/// Unsupported content is reported as a FAILED lease without changing document,
/// history or last document error. Poll info, then release even failed leases.
/// # Safety
/// Initialize outer/nested sizes. No caller memory survives this call.
#[no_mangle]
pub unsafe extern "C" fn paint_session_request_composition(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    desc: *const PaintCompositionRequest,
    out_lease: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(out_lease, 0)?;
        runtime::outside_callback()?;
        let request = Request::parse(pointers::read_sized(desc)?)?;
        let id = session(core, handle)?.request_composition(request)?;
        pointers::write(out_lease, id)
    })
}

/// Poll bounded metadata. A ready key must be passed unchanged to all reads.
/// Cancelled/released/old-generation leases return CANCELLED, never stale data.
/// # Safety
/// Initialize out_info.struct_size; lifecycle calls must be serialized.
#[no_mangle]
pub unsafe extern "C" fn paint_session_composition_info(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    lease: u64,
    out_info: *mut PaintCompositionInfo,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_info,
            PaintCompositionInfo {
                struct_size: size_of::<PaintCompositionInfo>() as u32,
                ..Default::default()
            },
        )?;
        pointers::write(out_info, session(core, handle)?.composition().info(lease)?)
    })
}

/// Read one immutable node; mask_index=UINT32_MAX means no mask. Node/tile
/// indices belong only to this exact lease; explicit transparent tiles override
/// the white_extent default. Optional capabilities define group/blend metadata.
/// # Safety
/// Initialize key/out_node sizes; outputs must not overlap inputs.
#[no_mangle]
pub unsafe extern "C" fn paint_session_composition_node(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    expected: *const PaintCompositionKey,
    index: u32,
    out_node: *mut PaintCompositionNode,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_node,
            PaintCompositionNode {
                struct_size: size_of::<PaintCompositionNode>() as u32,
                ..Default::default()
            },
        )?;
        let key = key(expected)?;
        let session = session(core, handle)?;
        let service = session.composition();
        let (lease, _) = service.get(key)?;
        let n = lease
            .plan
            .nodes
            .get(index as usize)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "composition node index"))?;
        let node = PaintCompositionNode {
            struct_size: size_of::<PaintCompositionNode>() as u32,
            kind: match n.kind {
                NodeKind::Raster => PAINT_COMPOSITION_RASTER,
                NodeKind::Mask => PAINT_COMPOSITION_MASK,
                NodeKind::Group => PAINT_COMPOSITION_GROUP,
            },
            layer_id: n.layer_id,
            mask_index: n.mask.map_or(u32::MAX, |i| i as u32),
            visible: u32::from(n.visible),
            opacity: n.opacity,
            fill: n.fill,
            offset_x: n.offset.0,
            offset_y: n.offset.1,
            white_width: n.white_extent.0,
            white_height: n.white_extent.1,
            first_tile: n.first_tile as u32,
            tile_count: n.tile_count as u32,
            reserved: n.blend as u64
                | if n.fixed_mask {
                    PAINT_COMPOSITION_NODE_FIXED_MASK
                } else {
                    0
                }
                | if lease.plan.required_features & PAINT_COMPOSITION_GROUPS != 0 {
                    n.parent.map_or(0, |i| ((i + 1) as u64) << 33)
                } else {
                    0
                },
        };
        service.publish_read(key, || pointers::write(out_node, node))
    })
}

/// ABI1.85: read one immutable ordered group operation with the exact lease key.
/// Only plans requiring GROUPS contain operations. No IO or native handles.
/// # Safety
/// Initialize key/out_operation sizes; outputs must not overlap inputs.
#[no_mangle]
pub unsafe extern "C" fn paint_session_composition_operation(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    expected: *const PaintCompositionKey,
    index: u32,
    out_operation: *mut PaintCompositionOperation,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_operation,
            PaintCompositionOperation {
                struct_size: size_of::<PaintCompositionOperation>() as u32,
                ..Default::default()
            },
        )?;
        let key = key(expected)?;
        let session = session(core, handle)?;
        let service = session.composition();
        let (lease, _) = service.get(key)?;
        let op = lease
            .plan
            .operations
            .get(index as usize)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "composition operation index"))?;
        let value = PaintCompositionOperation {
            struct_size: size_of::<PaintCompositionOperation>() as u32,
            kind: op.kind as u32,
            node_index: op.node,
            output: op.output,
            source: op.source,
            backdrop: op.backdrop,
            reserved: [0; 2],
        };
        service.publish_read(key, || pointers::write(out_operation, value))
    })
}

/// Read a stable tile identity and local signed 64px coordinates, without IO.
/// # Safety
/// Initialize key/out_tile sizes; outputs must not overlap inputs.
#[no_mangle]
pub unsafe extern "C" fn paint_session_composition_tile(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    expected: *const PaintCompositionKey,
    index: u32,
    out_tile: *mut PaintCompositionTile,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::reset_sized(
            out_tile,
            PaintCompositionTile {
                struct_size: size_of::<PaintCompositionTile>() as u32,
                ..Default::default()
            },
        )?;
        let key = key(expected)?;
        let session = session(core, handle)?;
        let service = session.composition();
        let (lease, _) = service.get(key)?;
        let tile = lease
            .plan
            .tiles
            .get(index as usize)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "composition tile index"))?;
        service.publish_read(key, || {
            pointers::write(
                out_tile,
                PaintCompositionTile {
                    struct_size: size_of::<PaintCompositionTile>() as u32,
                    format: PAINT_COMPOSITION_LINEAR_RGBA32F as u32,
                    content_id: tile.content_id,
                    tile_x: tile.coord.signed_x(),
                    tile_y: tile.coord.signed_y(),
                    required_bytes: paint_core::TILE_BYTES as u64,
                    reserved: 0,
                },
            )
        })
    })
}

/// Worker-only: may perform cold storage IO. Query null/zero returns required
/// bytes/BUFFER_TOO_SMALL without IO. No partial writes, no compressed format,
/// no native GPU pointer. Capacity is bytes, at most 64KiB for version 1.
/// The process staging budget remains charged through the final copy.
/// # Safety
/// Required must be writable; buffer must cover capacity, nonoverlapping with
/// key/required. Do not call from GUI/render threads or inside callbacks.
#[no_mangle]
pub unsafe extern "C" fn paint_session_read_composition_tile(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    expected: *const PaintCompositionKey,
    index: u32,
    buffer: *mut u8,
    capacity: u64,
    required: *mut u64,
) -> PaintStatus {
    boundary(|| unsafe {
        pointers::write(required, 0)?;
        runtime::outside_callback()?;
        if capacity > paint_core::TILE_BYTES as u64 {
            return Err(ApiError::invalid(
                "composition tile capacity exceeds version limit",
            ));
        }
        let key = key(expected)?;
        let session = session(core, handle)?;
        let service = session.composition();
        let (lease, cancel) = service.get(key)?;
        let tile = lease
            .plan
            .tiles
            .get(index as usize)
            .ok_or_else(|| ApiError::new(PAINT_NOT_FOUND, "composition tile index"))?;
        pointers::write(required, paint_core::TILE_BYTES as u64)?;
        if capacity < paint_core::TILE_BYTES as u64 {
            return Err(ApiError::new(
                PAINT_BUFFER_TOO_SMALL,
                "composition tile buffer too small",
            ));
        }
        pointers::valid_pointer(buffer)?;
        let _staging = Charge::read()?;
        let bytes = tile.read(&cancel).map_err(composition::error)?;
        service.publish_read(key, || {
            pointers::copy_buffer(&bytes, buffer, capacity, required)
        })
    })
}

/// Cancel pending work or release a ready/failed lease. Nonblocking: a worker
/// already reading retains its immutable data and budget until it finishes,
/// then rejects publication. New documents/devices and session close cancel too.
/// # Safety
/// Follow session lifetime contract; no callbacks or caller pointers retained.
#[no_mangle]
pub unsafe extern "C" fn paint_session_release_composition(
    core: *mut PaintCore,
    handle: *mut PaintSession,
    lease: u64,
) -> PaintStatus {
    boundary(|| {
        runtime::outside_callback()?;
        session(core, handle)?.composition().release(lease)
    })
}
