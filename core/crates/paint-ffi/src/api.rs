//! Stable ABI DTOs. Keep internal Rust models out of this module.
use std::ffi::c_void;

pub type PaintStatus = i32;
pub const PAINT_OK: PaintStatus = 0;
pub const PAINT_INVALID_ARGUMENT: PaintStatus = 1;
pub const PAINT_INVALID_HANDLE: PaintStatus = 2;
pub const PAINT_BUSY: PaintStatus = 3;
pub const PAINT_NOT_FOUND: PaintStatus = 4;
pub const PAINT_LIMIT_EXCEEDED: PaintStatus = 5;
pub const PAINT_UNSUPPORTED: PaintStatus = 6;
pub const PAINT_CANCELLED: PaintStatus = 7;
pub const PAINT_IO_ERROR: PaintStatus = 8;
pub const PAINT_INTERNAL_ERROR: PaintStatus = 9;
pub const PAINT_BUFFER_TOO_SMALL: PaintStatus = 10;

pub const PAINT_ABI_MAJOR: u32 = 1;
pub const PAINT_ABI_MINOR: u32 = 11;

/// ABI 1.10: metadata for one retained history boundary/operation.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintHistoryEntry {
    pub struct_size: u32,
    pub kind: u32,
    pub depth: u32,
    pub reserved: u32,
}
pub const PAINT_HISTORY_INITIAL: u32 = 0;
pub const PAINT_HISTORY_BRUSH: u32 = 1;
pub const PAINT_HISTORY_ERASER: u32 = 2;
pub const PAINT_HISTORY_SELECT_ALL: u32 = 3;
pub const PAINT_HISTORY_SELECTION: u32 = 4;
pub const PAINT_HISTORY_DESELECT: u32 = 5;
pub const PAINT_HISTORY_INVERT_SELECTION: u32 = 6;
pub const PAINT_HISTORY_ADD_LAYER: u32 = 7;
pub const PAINT_HISTORY_REMOVE_LAYER: u32 = 8;
pub const PAINT_HISTORY_LAYER_PROPERTIES: u32 = 9;
pub const PAINT_HISTORY_LAYER_BLEND: u32 = 10;
pub const PAINT_HISTORY_LAYER_FILL: u32 = 11;
pub const PAINT_HISTORY_LAYER_LOCKS: u32 = 12;
pub const PAINT_HISTORY_MOVE_LAYER: u32 = 13;
pub const PAINT_HISTORY_GROUP: u32 = 14;
pub const PAINT_HISTORY_UNGROUP: u32 = 15;
pub const PAINT_HISTORY_REPARENT: u32 = 16;
pub const PAINT_HISTORY_MASK: u32 = 17;
pub const PAINT_HISTORY_CLIPPING: u32 = 18;
pub const PAINT_HISTORY_TRUNCATED: u32 = 19;
pub const PAINT_HISTORY_ELLIPSE_SELECTION: u32 = 20;
/// ABI 1.12: freehand lasso path and magic-wand region.
pub const PAINT_HISTORY_LASSO_SELECTION: u32 = 21;
pub const PAINT_HISTORY_MAGIC_SELECTION: u32 = 22;
/// Automatic release of clipped layers whose base went away.
pub const PAINT_HISTORY_RELEASE_CLIPPING: u32 = 23;
pub const PAINT_ABI_PATCH: u32 = 0;
pub const PAINT_WORKING_LINEAR_SRGB: u32 = 1;
pub const PAINT_STORAGE_RGBA32F_PREMULTIPLIED: u32 = 1;
pub const PAINT_TILE_RGBA32F_LINEAR_PREMULTIPLIED: u32 = 2;
pub const PAINT_MODE_PAINT: u32 = 1;
pub const PAINT_MODE_ERASE: u32 = 2;
pub const PAINT_TOOL_PEN: u32 = 1;
pub const PAINT_TOOL_ERASER: u32 = 2;
pub const PAINT_TOOL_MOUSE: u32 = 3;
pub const PAINT_INPUT_PRESSURE: u32 = 1;
pub const PAINT_INPUT_TILT: u32 = 2;
pub const PAINT_INPUT_ROTATION: u32 = 4;
pub const PAINT_INPUT_TANGENTIAL_PRESSURE: u32 = 8;
pub const PAINT_INPUT_BUTTONS: u32 = 16;
pub const PAINT_FEATURE_DOCUMENT: u64 = 1;
pub const PAINT_FEATURE_HISTORY: u64 = 2;
pub const PAINT_FEATURE_LINEAR_TILE_READ: u64 = 4;
pub const PAINT_FEATURE_EVENTS: u64 = 8;
pub const PAINT_EVENT_DOCUMENT_CHANGED: u32 = 1;
pub const PAINT_EVENT_DOCUMENT_STATE_CHANGED: u32 = 2;
pub const PAINT_FEATURE_ASYNC_SESSION: u64 = 16;
pub const PAINT_FEATURE_CPU_VIEWPORT: u64 = 32;
pub const PAINT_FEATURE_FILE_IO: u64 = 64;
pub const PAINT_FILE_OPEN: u32 = 1;
pub const PAINT_FILE_SAVE: u32 = 2;
pub const PAINT_FILE_AUTO: u32 = 0;
pub const PAINT_FILE_PNG: u32 = 1;
pub const PAINT_FILE_JPEG: u32 = 2;
pub const PAINT_FILE_WEBP: u32 = 3;
pub const PAINT_FILE_OPENRASTER: u32 = 4;
pub const PAINT_FILE_QUEUED: u32 = 1;
pub const PAINT_FILE_RUNNING: u32 = 2;
pub const PAINT_FILE_SUCCEEDED: u32 = 3;
pub const PAINT_FILE_FAILED: u32 = 4;
pub const PAINT_FILE_CANCELLED: u32 = 5;

/// ABI 1.2: UTF-8 OS path, copied before returning; not a file URL.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PaintFileRequest {
    pub struct_size: u32,
    pub kind: u32,
    pub format: u32,
    pub quality: u32,
    pub path: *const u8,
    pub path_length: u64,
    pub document_generation: u64,
    pub linear_background: [f32; 4],
    pub reserved: [u32; 2],
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintFileJobInfo {
    pub struct_size: u32,
    pub state: u32,
    pub job_id: u64,
    pub source_generation: u64,
    pub source_revision: u64,
    pub result_generation: u64,
    pub result_revision: u64,
    pub kind: u32,
    pub format: u32,
    pub status: PaintStatus,
    pub reserved: u32,
}
pub const PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED: u32 = 3;
pub const PAINT_SESSION_RUNNING: u32 = 1;
pub const PAINT_SESSION_MODIFIED: u32 = 2;
pub const PAINT_SESSION_FAILED: u32 = 4;
pub const PAINT_COMMAND_NEW_DOCUMENT: u32 = 1;
pub const PAINT_COMMAND_BEGIN_STROKE: u32 = 2;
pub const PAINT_COMMAND_STROKE_TO: u32 = 3;
pub const PAINT_COMMAND_END_STROKE: u32 = 4;
pub const PAINT_COMMAND_CANCEL_STROKE: u32 = 5;
pub const PAINT_COMMAND_UNDO: u32 = 6;
pub const PAINT_COMMAND_REDO: u32 = 7;
pub const PAINT_COMMAND_ADD_LAYER: u32 = 8;
pub const PAINT_COMMAND_REMOVE_LAYER: u32 = 9;
pub const PAINT_COMMAND_SELECT_LAYER: u32 = 10;
pub const PAINT_COMMAND_LAYER_PROPERTIES: u32 = 11;

pub struct PaintSession {
    _private: u8,
}

/// ABI 1.1 asynchronous command. Only fields used by kind are read; reserved is zero.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PaintCommand {
    pub struct_size: u32,
    pub kind: u32,
    pub stroke: PaintStrokeDesc,
    pub point: PaintPoint,
    pub layer_id: u64,
    pub text: *const u8,
    pub text_length: u64,
    pub width: u32,
    pub height: u32,
    pub visible: u32,
    pub opacity: f32,
    pub reserved: [u32; 2],
}
impl Default for PaintCommand {
    fn default() -> Self {
        Self {
            struct_size: 0,
            kind: 0,
            stroke: PaintStrokeDesc::default(),
            point: PaintPoint::default(),
            layer_id: 0,
            text: std::ptr::null(),
            text_length: 0,
            width: 0,
            height: 0,
            visible: 0,
            opacity: 0.,
            reserved: [0; 2],
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintSessionInfo {
    pub struct_size: u32,
    pub flags: u32,
    pub document_generation: u64,
    pub publication: u64,
    pub completed_sequence: u64,
    pub revision: u64,
    pub active_layer_id: u64,
    pub last_error_sequence: u64,
    pub width: u32,
    pub height: u32,
    pub layer_count: u32,
    pub undo_depth: u32,
    pub redo_depth: u32,
    pub stroke_active: u32,
    pub queue_length: u32,
    pub last_error_status: PaintStatus,
    pub reserved: [u64; 2],
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintViewport {
    pub struct_size: u32,
    pub view_id: u32,
    pub enabled: u32,
    pub reserved: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub document_generation: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintFrameInfo {
    pub struct_size: u32,
    pub format: u32,
    pub document_generation: u64,
    pub revision: u64,
    pub request_id: u64,
    pub frame_id: u64,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub required_bytes: u64,
    pub reserved: u64,
}

// No repr(C): cbindgen emits opaque forward declarations, never these fields.
pub struct PaintCore {
    _private: u8,
}
pub struct PaintDocument {
    _private: u8,
}
pub struct PaintSubscription {
    _private: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintVersion {
    pub struct_size: u32,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintCapabilities {
    pub struct_size: u32,
    pub abi_major: u32,
    pub features: u64,
    pub max_dimension: u32,
    pub tile_size: u32,
    pub max_read_tile_edge: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintDocumentDesc {
    pub struct_size: u32,
    pub width: u32,
    pub height: u32,
    pub working_space: u32,
    pub pixel_format: u32,
    pub reserved: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintDocumentInfo {
    pub struct_size: u32,
    pub width: u32,
    pub height: u32,
    pub layer_count: u32,
    pub document_id: u64,
    pub active_layer_id: u64,
    pub revision: u64,
    pub allocated_tiles: u64,
    pub undo_depth: u32,
    pub redo_depth: u32,
    pub stroke_active: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintLayerInfo {
    pub struct_size: u32,
    pub visible: u32,
    pub layer_id: u64,
    pub opacity: f32,
    pub reserved: u32,
    pub name_length: u64,
    pub allocated_tiles: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintPoint {
    pub struct_size: u32,
    pub capabilities: u32,
    pub x: f64,
    pub y: f64,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub rotation: f32,
    pub tangential_pressure: f32,
    pub buttons: u32,
    pub tool: u32,
    pub timestamp_ns: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintStrokeDesc {
    pub struct_size: u32,
    pub mode: u32,
    pub layer_id: u64,
    pub radius: f32,
    pub opacity: f32,
    pub spacing: f32,
    pub linear_rgba: [f32; 4],
    pub reserved: [u32; 3],
}

/// data/capacity/stride/format are inputs. Other fields are filled after complete copying.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PaintTile {
    pub struct_size: u32,
    pub format: u32,
    pub data: *mut u8,
    pub capacity: u64,
    pub stride: u64,
    pub required: u64,
    pub width: u32,
    pub height: u32,
    pub revision: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintEvent {
    pub struct_size: u32,
    pub kind: u32,
    pub document_id: u64,
    pub revision: u64,
    pub task_id: u64,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub status: PaintStatus,
    pub reserved: u32,
}

/// Borrowed event, valid only for the callback. Copy before queuing to a UI thread.
pub type PaintEventCallback = Option<unsafe extern "C" fn(*const PaintEvent, *mut c_void)>;

/// ABI 1.3: zero path_length selects the system temp directory. Other fields are explicit.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PaintStorageOptions {
    pub struct_size: u32,
    pub reserved: u32,
    pub path: *const u8,
    pub path_length: u64,
    pub resident_bytes: u64,
    pub scratch_bytes: u64,
    pub min_free_bytes: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintStorageInfo {
    pub struct_size: u32,
    pub removed_runs: u32,
    pub available_bytes: u64,
    pub removed_bytes: u64,
    pub skipped_runs: u32,
    pub reserved: u32,
}
pub const PAINT_FEATURE_STORAGE_SETTINGS: u64 = 128;

pub const PAINT_LAYER_LOCKED: PaintStatus = 11;

pub const PAINT_FEATURE_LAYER_APPEARANCE: u64 = 256;
pub const PAINT_FEATURE_LAYER_GROUPS: u64 = 512;
pub const PAINT_LAYER_PIXEL: u32 = 0;
pub const PAINT_LAYER_GROUP: u32 = 1;
pub const PAINT_GROUP_WRAP: u32 = 1;
pub const PAINT_GROUP_UNGROUP: u32 = 2;
pub const PAINT_GROUP_REPARENT: u32 = 3;

/// ABI 1.5: independent hierarchy metadata; no existing DTO changes.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintLayerHierarchy {
    pub struct_size: u32,
    pub kind: u32,
    pub parent_id: u64,
    pub depth: u32,
    pub effective_locks: u32,
}
/// WRAP copies a UTF-8 name; REPARENT uses parent_id (0=root); UNGROUP uses neither.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PaintGroupRequest {
    pub struct_size: u32,
    pub kind: u32,
    pub layer_id: u64,
    pub parent_id: u64,
    pub name: *const u8,
    pub name_length: u64,
}
pub const PAINT_LOCK_TRANSPARENCY: u32 = 1;
pub const PAINT_LOCK_POSITION: u32 = 2;
pub const PAINT_LOCK_ALL: u32 = 4;
/// ABI 1.4; replaces no prior DTO. fill is independent of opacity; offsets are document pixels.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintLayerAppearance {
    pub struct_size: u32,
    pub locks: u32,
    pub blend_mode: u32,
    pub reserved: u32,
    pub fill: f32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub dissolve_seed: u32,
}

/// ABI 1.6 additive command kinds; old NEW_DOCUMENT remains transparent.
pub const PAINT_COMMAND_NEW_WHITE_DOCUMENT: u32 = 12;
pub const PAINT_COMMAND_ADD_MASK: u32 = 13;
pub const PAINT_LAYER_MASK: u32 = 2;
pub const PAINT_FEATURE_MASKS: u64 = 1024;
pub const PAINT_FEATURE_CLIPPING: u64 = 2048;
pub const PAINT_FEATURE_SELECTION: u64 = 4096;
pub const PAINT_SELECTION_SHAPE: u32 = 0;
pub const PAINT_SELECTION_CLEAR: u32 = 1;
pub const PAINT_SELECTION_ALL: u32 = 2;
pub const PAINT_SELECTION_INVERT: u32 = 3;
pub const PAINT_SELECTION_REPLACE: u32 = 0;
pub const PAINT_SELECTION_ADD: u32 = 1;
pub const PAINT_SELECTION_SUBTRACT: u32 = 2;
pub const PAINT_SELECTION_INTERSECT: u32 = 3;
pub const PAINT_SELECTION_STEP_INVERT: u32 = 4;
pub const PAINT_SELECTION_RECTANGLE: u32 = 0;
pub const PAINT_SELECTION_ELLIPSE: u32 = 1;
/// ABI 1.12: rasterized shapes. The geometry fields stay the bounding box for old callers.
pub const PAINT_SELECTION_POLYGON: u32 = 2;
pub const PAINT_SELECTION_MASK: u32 = 3;
/// ABI 1.12: free-path and content-derived selection edits.
pub const PAINT_SELECTION_PATH_POLYGON: u32 = 0;
pub const PAINT_SELECTION_PATH_MAGIC: u32 = 1;
/// ABI 1.12: one point of a selection path, in document pixels.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintSelectionPoint {
    pub x: f64,
    pub y: f64,
}
/// ABI 1.12: a freehand path (lasso) or a seed point (magic wand) with tolerance.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintSelectionPath {
    pub struct_size: u32,
    pub edit_kind: u32,
    pub operation: u32,
    pub antialias: u32,
    pub point_count: u32,
    pub tolerance: u32,
    pub reserved: [u32; 2],
    pub points: *const PaintSelectionPoint,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintSelectionEdit {
    pub struct_size: u32,
    pub action: u32,
    pub operation: u32,
    pub shape: u32,
    pub antialias: u32,
    pub reserved: [u32; 3],
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintSelectionInfo {
    pub struct_size: u32,
    pub enabled: u32,
    pub step_count: u32,
    pub reserved: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintSelectionStep {
    pub struct_size: u32,
    pub operation: u32,
    pub shape: u32,
    pub antialias: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
/// ABI 1.7 additive clipping query/edit. base_layer_id is query-only (0 if orphaned).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintLayerClipping {
    pub struct_size: u32,
    pub enabled: u32,
    pub base_layer_id: u64,
}
/// placement: 0=inside group/root, 1=above sibling, 2=below sibling.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PaintLayerDrop {
    pub struct_size: u32,
    pub placement: u32,
    pub layer_id: u64,
    pub target_id: u64,
}
