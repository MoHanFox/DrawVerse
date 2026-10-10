//! UI-independent document model. All mutation requires exclusive access.
//! Pixels are linear premultiplied RGBA32F; display conversion is external.
mod appearance;
mod brush;
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
pub use document::{DirtyTiles, Document, DocumentOptions, DocumentSnapshot, MAX_DIMENSION};
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
