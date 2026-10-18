//! UI-independent document model. All mutation requires exclusive access.
//! Pixels are linear premultiplied RGBA32F; display conversion is external.

mod brushes;
mod document;
mod error;
mod history;
mod layers;
mod pixels;
mod selection;

// Keep existing crate paths stable while implementation folders own their modules.
use brushes::dynamics::{
    brush_color_dynamics, brush_dynamics, brush_random, brush_scatter, brush_shape_jitter,
    brush_transfer_jitter,
};
use brushes::library::brush_catalog;
use brushes::model::{
    brush, brush_cursor_image, brush_dual, brush_dual_color, brush_mask, brush_outline,
    brush_program, brush_texture,
};
use brushes::stroke::{brush_airbrush, brush_input_schedule, brush_smoothing, brush_taper};
use brushes::wet::{brush_wet_carrier, brush_wet_ink, brush_wet_settings, brush_wet_tool};
use history::history_storage;
use layers::{appearance, clipping, layer};
use pixels::{page_pool, pixel, tile};

pub use brush_airbrush::Airbrush;
pub use brush_cursor_image::BRUSH_CURSOR_IMAGE_EDGE;
pub use brush_dual::{DualBrush, DualMode, DualSizeMode, DualSizing, DualWidthRule, SecondaryTip};
pub use brush_dual_color::{DualColor, DualColorModel, DualColorOrder};
pub use brush_input_schedule::{
    BrushInputSchedule, InputScheduleBudget, InputScheduleEvent, InputSchedulePage,
    InputScheduleSegment, InputScheduleStart, InputScheduleSteps, InputScheduleStream,
    InputScheduleWork, MAX_INPUT_PAGE_DABS, MAX_INPUT_PAGE_VISITS, MAX_INPUT_SCHEDULE_DABS,
    MAX_INPUT_SCHEDULE_VISITS, MAX_INPUT_SEGMENTS,
};
pub use brush_smoothing::BrushSmoothing;
pub use brush_taper::{
    CompiledTaper, StrokeTaper, TaperBuffer, TaperEnd, TaperFactors, TaperRecord, TaperStream,
    TaperUpdate, TaperedRecord, MAX_TAPER_DISTANCE, MAX_TAPER_RECORDS, TAPER_MODEL,
};
pub use brush_texture::{
    BrushTexture, TextureMode, TextureResource, TextureSettings, TextureSpace,
};
pub use brush_wet_carrier::{
    WetCarrier, WetDab, WetDabGeometry, WetMix, WetSample, MAX_WET_CARRIER_EDGE, WET_CARRIER_MODEL,
};
pub use brush_wet_ink::WetInk;
pub use brush_wet_settings::WetSettings;
pub use brush_wet_tool::{WetLease, WetTool};

pub use appearance::{
    blend_pixel, BlendMode, LayerAppearance, LOCK_ALL, LOCK_POSITION, LOCK_TRANSPARENCY,
};
pub use brush::{Brush, BrushMode, InputPoint, Tool};
pub use brush_catalog::{
    BrushCatalog, BrushCatalogItem, BrushGroup, BrushOrganization, BrushPreset, BrushSettings,
    MAX_BRUSH_GROUPS, MAX_BRUSH_GROUP_DEPTH, MAX_BRUSH_PRESETS, MAX_CATALOG_MASK_BYTES,
};
pub use brush_color_dynamics::ColorDynamics;
pub use brush_dynamics::{
    AngleControl, BrushDynamics, DirectionDynamics, DistanceDynamics, DynamicsSource,
    DynamicsSourceMatrix, FadeDynamics, PressureControl, PressureCurve, RotationDynamics,
    SpeedDynamics, TangentDynamics, TiltDynamics, TipDynamics,
};
pub use brush_mask::{BrushMask, MAX_BRUSH_MASK_BYTES, MAX_BRUSH_MASK_EDGE};
pub use brush_outline::{BrushOutlineSegment, MAX_BRUSH_OUTLINE_SEGMENTS};
pub use brush_program::{BrushDefinition, BrushPose, BrushProgram, BrushTip, CompositeBrushPose};
pub use brush_scatter::{ScatterControl, ScatterControlSource, ScatterDynamics};
pub use brush_shape_jitter::ShapeJitter;
pub use brush_transfer_jitter::{TransferJitter, TransferSample};
pub use document::{
    BucketFill, DirtyTiles, Document, DocumentOptions, DocumentSnapshot, LiquifyDab, PlacedImage,
    StrokePageResult, TransformGeometry, WandSelection, MAX_DIMENSION,
};
pub use error::{Error, Result};
pub use history::HistoryAction;
pub use layer::{ImportedLayer, Layer, LayerId, LayerProperties};
pub use page_pool::StorageStats;
pub use pixel::Pixel;
pub use selection::{
    mask_shape, Selection, SelectionKind, SelectionMask, SelectionOperation, SelectionShape,
    SelectionStep, MAX_SELECTION_MASK_BYTES, MAX_SELECTION_STEPS,
};
pub use tile::{Tile, TileCoord, TILE_BYTES, TILE_SIZE};
