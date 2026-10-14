//! UI-independent document model. All mutation requires exclusive access.
//! Pixels are linear premultiplied RGBA32F; display conversion is external.
mod appearance;
mod brush;
mod brush_catalog;
mod brush_dynamics;
mod brush_texture;
pub use brush_texture::{BrushTexture, TextureResource, TextureSettings, TextureSpace};
mod brush_mask;
mod brush_outline;
mod brush_program;
mod clipping;
mod document;
mod error;
mod history;
mod history_storage;
mod layer;
mod page_pool;
mod pixel;
mod selection;
mod tile;

pub use appearance::{
    blend_pixel, BlendMode, LayerAppearance, LOCK_ALL, LOCK_POSITION, LOCK_TRANSPARENCY,
};
pub use brush::{Brush, BrushMode, InputPoint, Tool};
pub use brush_catalog::{
    BrushCatalog, BrushPreset, BrushSettings, MAX_BRUSH_PRESETS, MAX_CATALOG_MASK_BYTES,
};
pub use brush_dynamics::{
    AngleControl, BrushDynamics, PressureControl, PressureCurve, TipDynamics,
};
pub use brush_mask::{BrushMask, MAX_BRUSH_MASK_BYTES, MAX_BRUSH_MASK_EDGE};
pub use brush_outline::{BrushOutlineSegment, MAX_BRUSH_OUTLINE_SEGMENTS};
pub use brush_program::{BrushDefinition, BrushPose, BrushProgram, BrushTip};
mod brush_color_dynamics;
pub use brush_color_dynamics::ColorDynamics;
mod brush_random;
mod brush_shape_jitter;
pub use brush_shape_jitter::ShapeJitter;
mod brush_scatter;
pub use brush_scatter::ScatterDynamics;
pub use document::{
    BucketFill, DirtyTiles, Document, DocumentOptions, DocumentSnapshot, LiquifyDab, PlacedImage,
    TransformGeometry, WandSelection, MAX_DIMENSION,
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
