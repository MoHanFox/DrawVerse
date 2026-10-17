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
pub const PAINT_ABI_MINOR: u32 = 56;
pub const PAINT_FEATURE_BRUSH_GROUP_PACKAGE: u64 = 1125899906842624;
pub const PAINT_FEATURE_BRUSH_GROUP_STATE: u64 = 562949953421312;
pub const PAINT_FEATURE_BRUSH_CURSOR_IMAGE: u64 = 281474976710656;
pub const PAINT_BRUSH_CURSOR_IMAGE_EDGE: u32 = 256;
pub const PAINT_FEATURE_PRESSURE_DYNAMICS: u64 = 1073741824;
pub const PAINT_FEATURE_VBR_SOURCES: u64 = 536870912;
pub const PAINT_FEATURE_ABR_IMPORT: u64 = 268435456;
pub const PAINT_FEATURE_BRUSH_SOURCES: u64 = 134217728;
pub const PAINT_FEATURE_VBR_IMAGES: u64 = 67108864;
pub const PAINT_FEATURE_BRUSH_OUTLINE: u64 = 33554432;
pub const PAINT_FEATURE_BRUSH_RESOURCES: u64 = 16777216;
pub const PAINT_FEATURE_BRUSH_PREPARATION: u64 = 8388608;
pub const PAINT_FEATURE_BRUSH_CATALOG: u64 = 4194304;
pub struct PaintBrushLibrary {
    _private: u8,
}
pub struct PaintBrushPose {
    _private: u8,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintTipPose {
    pub struct_size: u32,
    pub reserved: u32,
    pub radius: f32,
    pub angle: f32,
    pub roundness: f32,
    pub reserved_tail: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintCompositeTipPose {
    pub struct_size: u32,
    pub flags: u32,
    pub primary: PaintTipPose,
    pub secondary: PaintTipPose,
    pub reserved: u64,
}
pub const PAINT_FEATURE_COMPOSITE_BRUSH_POSE: u64 = 17592186044416;
pub const PAINT_POSE_SECONDARY: u32 = 1;
pub const PAINT_POSE_PRIMARY_IMAGE: u32 = 2;
pub const PAINT_POSE_SECONDARY_IMAGE: u32 = 4;
pub const PAINT_POSE_SECONDARY_OVERFLOW: u32 = 8;
pub const PAINT_FEATURE_BRUSH_POSE: u64 = 8589934592;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintBrushSettings {
    pub struct_size: u32,
    pub reserved: u32,
    pub diameter: f32,
    pub opacity: f32,
    pub spacing: f32,
    pub reserved_tail: u32,
    pub tip: PaintBrushTip,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintBrushLibraryInfo {
    pub struct_size: u32,
    pub preset_count: u32,
    pub selected_index: u32,
    pub reserved: u32,
    pub revision: u64,
    pub mask_budget_bytes: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintBrushPresetInfo {
    pub struct_size: u32,
    pub builtin: u32,
    pub diameter: f32,
    pub opacity: f32,
    pub spacing: f32,
    pub has_image: u32,
    pub tip: PaintBrushTip,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintBrushToolDesc {
    pub struct_size: u32,
    pub operation: u32,
    pub layer_id: u64,
    pub radius: f32,
    pub strength: f32,
    pub linear_rgba: [f32; 4],
    pub all_layers: u32,
    pub finger_painting: u32,
    pub pressure_strength: u32,
    pub reserved: u32,
}
pub const PAINT_FEATURE_VBR: u64 = 2097152;
pub const PAINT_FEATURE_BRUSH_TIP: u64 = 1048576;
pub const PAINT_BRUSH_PAINT: u32 = 0;
pub const PAINT_BRUSH_ERASE: u32 = 1;
pub const PAINT_BRUSH_SMUDGE: u32 = 2;
/// ABI 1.25 procedural tip. All percentages are normalized; angle is degrees.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintBrushTip {
    pub struct_size: u32,
    pub hardness: f32,
    pub roundness: f32,
    pub angle: f32,
    pub flow: f32,
    pub pressure_size: u32,
    pub pressure_flow: u32,
    pub reserved: [u32; 3],
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintBrushStrokeDesc {
    pub struct_size: u32,
    pub operation: u32,
    pub brush: PaintStrokeDesc,
    pub tip: PaintBrushTip,
    pub strength: f32,
    pub all_layers: u32,
    pub finger_painting: u32,
    pub pressure_strength: u32,
}
pub const PAINT_FEATURE_LIQUIFY: u64 = 262144;
pub const PAINT_FEATURE_TRANSFORM: u64 = 524288;
pub const PAINT_SESSION_LIQUIFY: u32 = 8;
pub const PAINT_HISTORY_LIQUIFY: u32 = 30;
pub const PAINT_LIQUIFY_BEGIN: u32 = 0;
pub const PAINT_LIQUIFY_DAB: u32 = 1;
pub const PAINT_LIQUIFY_RESET: u32 = 2;
pub const PAINT_LIQUIFY_CANCEL: u32 = 3;
pub const PAINT_LIQUIFY_COMMIT: u32 = 4;
pub const PAINT_LIQUIFY_FOCUS_ON: u32 = 5;
pub const PAINT_LIQUIFY_FOCUS_OFF: u32 = 6;
pub const PAINT_LIQUIFY_STROKE_BEGIN: u32 = 7;
pub const PAINT_LIQUIFY_STROKE_END: u32 = 8;
pub const PAINT_LIQUIFY_UNDO: u32 = 9;
pub const PAINT_LIQUIFY_REDO: u32 = 10;
pub const PAINT_HISTORY_LIQUIFY_PUSH: u32 = 31;
pub const PAINT_HISTORY_LIQUIFY_BLOAT: u32 = 32;
pub const PAINT_HISTORY_LIQUIFY_TWIRL: u32 = 33;
pub const PAINT_HISTORY_LIQUIFY_RECONSTRUCT: u32 = 34;
pub const PAINT_HISTORY_LIQUIFY_RESET: u32 = 35;
/// ABI 1.21: inverse-map liquify input; normalized percentages, diameter/2 radius in pixels.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintLiquifyDab {
    pub struct_size: u32,
    pub tool: u32,
    pub pressure_enabled: u32,
    pub reserved: u32,
    pub radius: f32,
    pub strength: f32,
    pub density: f32,
    pub speed: f32,
    pub elapsed_ms: f32,
    pub reserved_tail: [u32; 3],
    pub from: PaintPoint,
    pub to: PaintPoint,
}
pub const PAINT_FEATURE_LAYER_ACTIONS: u64 = 131072;
pub const PAINT_LAYER_ACTION_COPY: u32 = 0;
pub const PAINT_LAYER_ACTION_PASTE: u32 = 1;
pub const PAINT_LAYER_ACTION_DUPLICATE: u32 = 2;
pub const PAINT_LAYER_ACTION_MERGE: u32 = 3;
pub const PAINT_HISTORY_DUPLICATE_LAYERS: u32 = 27;
pub const PAINT_HISTORY_PASTE_LAYERS: u32 = 28;
pub const PAINT_HISTORY_MERGE_LAYERS: u32 = 29;
pub const PAINT_FEATURE_PASS_THROUGH: u64 = 65536;
pub const PAINT_BLEND_PASS_THROUGH: u32 = 27;
pub const PAINT_FEATURE_SMUDGE: u64 = 32768;
pub const PAINT_HISTORY_SMUDGE: u32 = 26;
/// ABI 1.18: same brush/point pipeline, independent strength and sampling options.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintSmudgeStrokeDesc {
    pub struct_size: u32,
    pub all_layers: u32,
    pub finger_painting: u32,
    pub pressure_strength: u32,
    pub strength: f32,
    pub reserved: [u32; 3],
    pub brush: PaintStrokeDesc,
}
pub const PAINT_FEATURE_WAND: u64 = 16384;
/// ABI 1.17: bounded color-region selection; flags 0/1, reserved zero.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintWandSelection {
    pub struct_size: u32,
    pub x: u32,
    pub y: u32,
    pub sample_size: u32,
    pub tolerance: u32,
    pub antialias: u32,
    pub contiguous: u32,
    pub all_layers: u32,
    pub operation: u32,
    pub reserved: [u32; 3],
}

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
pub const PAINT_HISTORY_POLYGON_SELECTION: u32 = 24;
pub const PAINT_HISTORY_BUCKET: u32 = 25;

pub const PAINT_FEATURE_BUCKET: u64 = 8192;
/// ABI 1.16: single atomic fill; booleans are exactly 0 or 1, reserved fields zero.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintBucketFill {
    pub struct_size: u32,
    pub reserved: u32,
    pub x: u32,
    pub y: u32,
    pub linear_rgba: [f32; 4],
    pub opacity: f32,
    pub tolerance: u32,
    pub antialias: u32,
    pub contiguous: u32,
    pub all_layers: u32,
    pub reserved_tail: [u32; 3],
}
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

/// ABI 1.13: delete selected subtrees, or unwrap selected groups while retaining children.
pub const PAINT_REMOVE_SUBTREES: u32 = 0;
pub const PAINT_REMOVE_KEEP_GROUP_CHILDREN: u32 = 1;

/// ABI 1.15: shared rectangle/ellipse/polygon selection settings. tool=0 geometry,
/// 1 freehand lasso, 2 polygonal lasso. Points are copied before return.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintSelectionShapeEdit {
    pub struct_size: u32,
    pub shape: u32,
    pub operation: u32,
    pub antialias: u32,
    pub tool: u32,
    pub point_count: u32,
    pub feather: f64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub points: *const PaintSelectionPoint,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintSelectionDetails {
    pub struct_size: u32,
    pub point_count: u32,
    pub feather: f64,
}

/// ABI 1.23 non-destructive image placement and free transform.
pub const PAINT_FILE_PLACE: u32 = 3;
pub const PAINT_SESSION_TRANSFORM: u32 = 16;
pub const PAINT_TRANSFORM_BEGIN: u32 = 0;
pub const PAINT_TRANSFORM_UPDATE: u32 = 1;
pub const PAINT_TRANSFORM_CHECKPOINT: u32 = 2;
pub const PAINT_TRANSFORM_CANCEL: u32 = 3;
pub const PAINT_TRANSFORM_COMMIT: u32 = 4;
pub const PAINT_TRANSFORM_UNDO: u32 = 5;
pub const PAINT_TRANSFORM_REDO: u32 = 6;
pub const PAINT_TRANSFORM_RASTERIZE: u32 = 7;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintTransform {
    pub struct_size: u32,
    pub warp: u32,
    pub interpolation: u32,
    pub reserved: u32,
    pub points: [f64; 32],
    pub skew_h: f64,
    pub skew_v: f64,
}

/// Independent per-stroke opacity pressure controls in state v6 and VBR 1.4.
pub const PAINT_FEATURE_OPACITY_PRESSURE: u64 = 2147483648;

/// Device and path controlled tip geometry, state v7 / VBR1.5.
pub const PAINT_FEATURE_DEVICE_TIP_DYNAMICS: u64 = 4294967296;

/// Deterministic scatter stamps, state v8 / VBR1.6.
pub const PAINT_FEATURE_BRUSH_SCATTER: u64 = 17179869184;

/// Per-stamp size, angle and roundness jitter, state9 / VBR1.7.
pub const PAINT_FEATURE_SHAPE_JITTER: u64 = 34359738368;

/// Background tool colour captured with the existing foreground tool DTO.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintBrushBackgroundDesc {
    pub struct_size: u32,
    pub reserved: u32,
    pub linear_rgba: [f32; 4],
}
pub const PAINT_FEATURE_COLOR_DYNAMICS: u64 = 68719476736;
/// Periodic texture coverage with resource-preserving state11 / VBR1.9 exchange.
pub const PAINT_FEATURE_BRUSH_TEXTURE: u64 = 137438953472;
/// Complete secondary brush models, state12 / VBR1.10 exchange.
pub const PAINT_FEATURE_DUAL_BRUSH: u64 = 274877906944;

/// Per-stroke drawing blend; does not change the target layer blend mode.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintBrushBlendDesc {
    pub struct_size: u32,
    pub mode: u32,
    pub reserved: u64,
}
pub const PAINT_FEATURE_BRUSH_WRITE_BLEND: u64 = 549755813888;

/// Shape-preserving pressure interpolation and bounded UI curve samples.
pub const PAINT_FEATURE_PRESSURE_SMOOTH: u64 = 1099511627776;

/// Complete time-grid smoothing state14 / VBR1.12 user entry.
pub const PAINT_FEATURE_PATH_SMOOTHING: u64 = 2199023255552;

/// Complete relative secondary sizing state15 / VBR1.13 user entry.
pub const PAINT_FEATURE_DUAL_RELATIVE_SIZE: u64 = 4398046511104;

/// Complete separated dual-color state16 / VBR1.14 user entry.
pub const PAINT_FEATURE_DUAL_COLOR_STROKE: u64 = 8796093022208;
/// Complete per-stamp interleaved dual color, state22 / VBR1.28.
pub const PAINT_FEATURE_DUAL_COLOR_INTERLEAVED: u64 = 2251799813685248;

/// Shared, tool-owned wet reservoir (ABI1.50). Never dereference this token.
pub struct PaintWetTool {
    _private: u8,
}
pub const PAINT_FEATURE_WET_BRUSH: u64 = 35184372088832;
/// Complete state18/VBR1.22 stroke taper parameter entry (ABI1.51).
pub const PAINT_FEATURE_STROKE_TAPER: u64 = 70368744177664;
pub const PAINT_WET_TOOL_INITIALIZED: u32 = 1;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintWetLoadDesc {
    pub struct_size: u32,
    pub reserved: u32,
    pub radius: f32,
    pub reserved_tail: u32,
    pub linear_rgba: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintWetToolInfo {
    pub struct_size: u32,
    pub flags: u32,
    pub carrier_revision: u64,
    pub edge: u32,
    pub reserved: u32,
    pub center_linear_premul_rgba: [f32; 4],
}

/// Complete independent wet dual carriers, state19/VBR1.23 (ABI1.52).
pub const PAINT_FEATURE_WET_DUAL_COLOR: u64 = 140737488355328;
pub const PAINT_WET_TOOL_SECONDARY_INITIALIZED: u32 = 2;
/// Atomic idle snapshot of both wet roles. Center samples are not grid averages.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PaintWetToolInfoDual {
    pub struct_size: u32,
    pub flags: u32,
    pub primary_revision: u64,
    pub secondary_revision: u64,
    pub primary_edge: u32,
    pub secondary_edge: u32,
    pub primary_center_linear_premul_rgba: [f32; 4],
    pub secondary_center_linear_premul_rgba: [f32; 4],
    pub reserved: [u32; 4],
}
