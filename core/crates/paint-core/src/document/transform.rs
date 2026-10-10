//! Whole-canvas orientation changes.
//!
//! Rotation and flipping are orthogonal, so every destination pixel maps to exactly one source
//! pixel and the content needs no resampling: the transform is a lossless integer permutation. Only
//! the canvas dimensions, each layer's local extent and the tile placement change.

use crate::{Document, Error, Layer, Pixel, Result, Tile, TileCoord, TILE_SIZE};

/// A canvas orientation change requested from the 图像 menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasTransform {
    Rotate90Cw = 0,
    Rotate90Ccw = 1,
    Rotate180 = 2,
    FlipHorizontal = 3,
    FlipVertical = 4,
}

impl CanvasTransform {
    pub fn from_raw(raw: u32) -> Result<Self> {
        match raw {
            0 => Ok(Self::Rotate90Cw),
            1 => Ok(Self::Rotate90Ccw),
            2 => Ok(Self::Rotate180),
            3 => Ok(Self::FlipHorizontal),
            4 => Ok(Self::FlipVertical),
            _ => Err(Error::InvalidArgument("unknown canvas transform")),
        }
    }

    /// Canvas size after the transform: only the quarter turns swap the axes.
    pub fn dimensions(self, width: u32, height: u32) -> (u32, u32) {
        match self {
            Self::Rotate90Cw | Self::Rotate90Ccw => (height, width),
            _ => (width, height),
        }
    }

    /// Destination pixel for a source pixel in a `width` x `height` canvas.
    pub fn map(self, x: u32, y: u32, width: u32, height: u32) -> (u32, u32) {
        match self {
            Self::Rotate90Cw => (height - 1 - y, x),
            Self::Rotate90Ccw => (y, width - 1 - x),
            Self::Rotate180 => (width - 1 - x, height - 1 - y),
            Self::FlipHorizontal => (width - 1 - x, y),
            Self::FlipVertical => (x, height - 1 - y),
        }
    }
}

/// Read one pixel of a layer's untranslated document content.
fn read_content(layer: &Layer, x: i64, y: i64) -> Result<Pixel> {
    if x < 0 || y < 0 || x > i64::from(u32::MAX) || y > i64::from(u32::MAX) {
        return Ok(Pixel::TRANSPARENT);
    }
    let inside_white = x < i64::from(layer.white.0) && y < i64::from(layer.white.1);
    if layer.tiles.is_empty() {
        return Ok(if inside_white {
            Pixel::WHITE
        } else {
            Pixel::TRANSPARENT
        });
    }
    let coord = TileCoord {
        x: (x as u32) / TILE_SIZE,
        y: (y as u32) / TILE_SIZE,
    };
    let local_x = (x as u32) % TILE_SIZE;
    let local_y = (y as u32) % TILE_SIZE;
    match layer.tiles.get(&coord) {
        Some(tile) => Ok(tile.pixel(local_x, local_y)),
        None if inside_white => Ok(Pixel::WHITE),
        None => Ok(Pixel::TRANSPARENT),
    }
}

/// Rewrite one layer's content from one canvas orientation to another. Returns whether the layer
/// carried any content, so empty layers (and empty groups) stay sparse.
fn transform_layer(
    layer: &mut Layer,
    transform: CanvasTransform,
    width: u32,
    height: u32,
) -> Result<bool> {
    let (new_width, new_height) = transform.dimensions(width, height);
    let has_content = !layer.tiles.is_empty() || layer.white != (0, 0);
    if !has_content {
        return Ok(false);
    }
    let mut tiles = std::collections::BTreeMap::new();
    for (x, y) in (0..new_height).flat_map(|y| (0..new_width).map(move |x| (x, y))) {
        // Inverse map: which old pixel lands here.
        let (sx, sy) = transform.map(x, y, new_width, new_height);
        let pixel = read_content(layer, i64::from(sx), i64::from(sy))?;
        if pixel.alpha() == 0. {
            continue;
        }
        let coord = TileCoord {
            x: x / TILE_SIZE,
            y: y / TILE_SIZE,
        };
        let entry = tiles.entry(coord).or_insert_with(|| {
            vec![Pixel::TRANSPARENT; (TILE_SIZE as usize) * (TILE_SIZE as usize)]
        });
        entry[((y % TILE_SIZE) as usize) * (TILE_SIZE as usize) + ((x % TILE_SIZE) as usize)] =
            pixel;
    }
    layer.tiles = tiles
        .into_iter()
        .map(|(coord, pixels)| Ok((coord, std::sync::Arc::new(Tile::from_pixels(pixels)?))))
        .collect::<Result<_>>()?;
    layer.white = (new_width, new_height);
    layer.appearance.offset_x = 0;
    layer.appearance.offset_y = 0;
    Ok(true)
}

impl Document {
    /// Rotate or flip every layer, including masks and nested group members. One history entry, one
    /// undo. Dimensions swap for the quarter turns; masks and group relationships are preserved.
    pub fn transform_canvas(&mut self, transform: CanvasTransform) -> Result<()> {
        self.idle()?;
        let (width, height) = (self.width, self.height);
        let before = self.structure();
        let before_size = (width, height);
        let (new_width, new_height) = transform.dimensions(width, height);
        if new_width == 0
            || new_height == 0
            || new_width > crate::MAX_DIMENSION
            || new_height > crate::MAX_DIMENSION
        {
            return Err(Error::InvalidArgument(
                "transform exceeds the dimension budget",
            ));
        }
        // Work on a copy first: a failure must leave the document exactly as it was.
        let mut after = before.clone();
        for layer in &mut after {
            transform_layer(layer, transform, width, height)?;
        }
        // The command carries both dimensions so undo and redo restore the canvas size as well as
        // the pixels.
        let command = crate::history::Command::Transform {
            before,
            after: after.clone(),
            before_size,
            after_size: (new_width, new_height),
        };
        // Commit records the history entry; applying it forward is this call's job, exactly like the
        // structure commands do after their own commit.
        self.commit_with_action(command, crate::HistoryAction::TransformCanvas)?;
        self.apply_transform(&after, (new_width, new_height));
        Ok(())
    }

    /// Apply a transform command in either direction: layer stacks plus canvas dimensions.
    pub(crate) fn apply_transform(&mut self, layers: &[Layer], size: (u32, u32)) {
        self.apply_structure(layers);
        self.width = size.0;
        self.height = size.1;
        self.dirty_all();
    }
}
