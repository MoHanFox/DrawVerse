//! ABI1.81: independent display-plan protocol; no native GPU pointers.
pub const PAINT_COMPOSITION_VERSION: u32 = 1;
pub const PAINT_COMPOSITION_NORMAL: u64 = 1;
pub const PAINT_COMPOSITION_SELF_MASK: u64 = 2;
pub const PAINT_COMPOSITION_LINEAR_RGBA32F: u64 = 4;
/// ABI1.83 optional version1 extensions; older feature sets retain old semantics.
pub const PAINT_COMPOSITION_FIXED_MASK: u64 = 8;
pub const PAINT_COMPOSITION_BLEND_MODES: u64 = 16;
/// ABI1.85: ordered group operations and plan-local intermediate surfaces.
pub const PAINT_COMPOSITION_GROUPS: u64 = 32;
pub const PAINT_COMPOSITION_DISSOLVE: u64 = 64;
pub const PAINT_COMPOSITION_CLIPPING: u64 = 128;
/// ABI1.89: request.revision is a lower bound; READY key identifies the captured snapshot.
pub const PAINT_COMPOSITION_LATEST_SNAPSHOT: u64 = 256;
pub const PAINT_COMPOSITION_NODE_CLIPPED: u64 = 1 << 49;
/// Node.reserved low32 = blend ID when BLEND_MODES is required.
/// Bit32 marks an unlinked mask when FIXED_MASK is required. With GROUPS,
/// bits33..48 hold parent node index+1, or zero for a root. Bit49 marks clipping
/// only with CLIPPING; all other bits are zero.
pub const PAINT_COMPOSITION_NODE_FIXED_MASK: u64 = 0x1_0000_0000;
pub const PAINT_COMPOSITION_PENDING: u32 = 0;
pub const PAINT_COMPOSITION_READY: u32 = 1;
pub const PAINT_COMPOSITION_FAILED: u32 = 2;
pub const PAINT_COMPOSITION_RASTER: u32 = 0;
pub const PAINT_COMPOSITION_MASK: u32 = 1;
pub const PAINT_COMPOSITION_GROUP: u32 = 2;
pub const PAINT_COMPOSITION_CLEAR: u32 = 0;
pub const PAINT_COMPOSITION_COPY: u32 = 1;
pub const PAINT_COMPOSITION_DRAW_RASTER: u32 = 2;
pub const PAINT_COMPOSITION_BLEND_SURFACE: u32 = 3;
pub const PAINT_COMPOSITION_INFLUENCE: u32 = 4;
pub const PAINT_COMPOSITION_MASKED_RASTER: u32 = 5;
pub const PAINT_COMPOSITION_MASK_SURFACE: u32 = 6;
pub const PAINT_COMPOSITION_CLIP_RASTER: u32 = 7;
pub const PAINT_COMPOSITION_CLIP_SURFACE: u32 = 8;
pub const PAINT_COMPOSITION_CLIP_INFLUENCE: u32 = 9;
pub const PAINT_COMPOSITION_APPLY_PREPARED: u32 = 10;

/// Fixed-size immutable group/clipping work. For CLEAR, source/backdrop are unused;
/// COPY reads backdrop, DRAW_RASTER reads node/backdrop, BLEND_SURFACE and
/// INFLUENCE read source/backdrop/node. Output must not alias a read surface.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintCompositionOperation {
    pub struct_size: u32,
    pub kind: u32,
    pub node_index: u32,
    pub output: u32,
    pub source: u32,
    pub backdrop: u32,
    /// With DISSOLVE: seed u32; otherwise zero. With CLIPPING: coverage
    /// surface+1 for CLIP operations, zero means full coverage.
    pub reserved: [u32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintCompositionCapabilities {
    pub struct_size: u32,
    pub version: u32,
    pub features: u64,
    pub max_nodes: u32,
    pub max_tiles: u32,
    pub max_leases_per_session: u32,
    pub tile_edge: u32,
    pub max_pinned_bytes: u64,
    pub max_metadata_bytes: u64,
    pub max_read_bytes: u64,
    pub process_plan_bytes: u64,
    pub process_read_bytes: u64,
    pub reserved: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintCompositionRequest {
    pub struct_size: u32,
    pub version: u32,
    pub features: u64,
    pub viewport: super::PaintViewport,
    pub revision: u64,
    pub request_id: u64,
    pub device_epoch: u64,
    pub max_pinned_bytes: u64,
    pub max_metadata_bytes: u64,
    pub max_nodes: u32,
    pub max_tiles: u32,
    pub reserved: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaintCompositionKey {
    pub struct_size: u32,
    pub view_id: u32,
    pub lease_id: u64,
    pub request_id: u64,
    pub device_epoch: u64,
    pub document_generation: u64,
    pub revision: u64,
    pub publication: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintCompositionInfo {
    pub struct_size: u32,
    pub state: u32,
    pub key: PaintCompositionKey,
    pub status: super::PaintStatus,
    pub version: u32,
    pub required_features: u64,
    pub document_width: u32,
    pub document_height: u32,
    pub node_count: u32,
    pub tile_count: u32,
    pub pinned_bytes: u64,
    pub metadata_bytes: u64,
    /// Conservative process budget charge, including referenced tile upper bound.
    pub budget_bytes: u64,
    /// With GROUPS/DISSOLVE/CLIPPING: [operation_count, surface_count<<32 | output].
    /// Otherwise both are zero and the original flat raster order applies.
    pub reserved: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintCompositionUsage {
    pub struct_size: u32,
    pub reserved: u32,
    pub plan_bytes: u64,
    pub read_bytes: u64,
    pub peak_plan_bytes: u64,
    pub peak_read_bytes: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintCompositionNode {
    pub struct_size: u32,
    pub kind: u32,
    pub layer_id: u64,
    /// u32::MAX means no mask. Otherwise an exact node index in this lease.
    pub mask_index: u32,
    pub visible: u32,
    pub opacity: f32,
    pub fill: f32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub white_width: u32,
    pub white_height: u32,
    pub first_tile: u32,
    pub tile_count: u32,
    pub reserved: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintCompositionTile {
    pub struct_size: u32,
    pub format: u32,
    pub content_id: u64,
    pub tile_x: i32,
    pub tile_y: i32,
    pub required_bytes: u64,
    pub reserved: u64,
}
