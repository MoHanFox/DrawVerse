use crate::{Pixel, Tile, TileCoord, TILE_SIZE};
use std::{collections::BTreeMap, sync::Arc};

pub type LayerId = u64;

/// Owned import staging, validated as a whole before becoming a document.
#[derive(Debug)]
pub struct ImportedLayer {
    pub name: String,
    pub properties: LayerProperties,
    pub tiles: BTreeMap<TileCoord, Tile>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerProperties {
    pub visible: bool,
    pub opacity: f32,
}

impl Default for LayerProperties {
    fn default() -> Self {
        Self {
            visible: true,
            opacity: 1.0,
        }
    }
}

/// Read-only public surface; mutations go through Document and its history.
#[derive(Clone, Debug)]
pub struct Layer {
    pub(crate) id: LayerId,
    pub(crate) name: String,
    pub(crate) properties: LayerProperties,
    pub(crate) appearance: crate::LayerAppearance,
    pub(crate) parent: LayerId,
    pub(crate) group: bool,
    pub(crate) mask: bool,
    pub(crate) clipped: bool,
    pub(crate) white: (u32, u32),
    pub(crate) tiles: BTreeMap<TileCoord, Arc<Tile>>,
}

impl Layer {
    pub(crate) fn new(id: LayerId, name: String) -> Self {
        Self {
            id,
            name,
            properties: LayerProperties::default(),
            appearance: crate::LayerAppearance {
                dissolve_seed: id as u32,
                ..Default::default()
            },
            tiles: BTreeMap::new(),
            parent: 0,
            group: false,
            mask: false,
            clipped: false,
            white: (0, 0),
        }
    }

    pub fn id(&self) -> LayerId {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn parent_id(&self) -> LayerId {
        self.parent
    }
    pub fn is_group(&self) -> bool {
        self.group
    }
    pub fn is_mask(&self) -> bool {
        self.mask
    }
    pub fn is_clipped(&self) -> bool {
        self.clipped
    }
    pub fn white_extent(&self) -> (u32, u32) {
        self.white
    }
    pub fn default_pixel(&self, x: i64, y: i64) -> Pixel {
        if x >= 0 && y >= 0 && x < i64::from(self.white.0) && y < i64::from(self.white.1) {
            Pixel::WHITE
        } else {
            Pixel::TRANSPARENT
        }
    }
    /// Structure history must not clone tile maps or retain pixel pages.
    pub(crate) fn metadata(&self) -> Self {
        let mut node = Self::new(self.id, self.name.clone());
        node.properties = self.properties;
        node.appearance = self.appearance;
        node.parent = self.parent;
        node.group = self.group;
        node.mask = self.mask;
        node.clipped = self.clipped;
        node.white = self.white;
        node
    }
    pub fn properties(&self) -> LayerProperties {
        self.properties
    }
    pub fn appearance(&self) -> crate::LayerAppearance {
        self.appearance
    }
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    pub fn tiles(&self) -> impl Iterator<Item = (TileCoord, &Arc<Tile>)> {
        self.tiles.iter().map(|(coord, tile)| (*coord, tile))
    }

    /// Read-only tile access for snapshot renderers; does not expose mutable storage.
    pub fn tile(&self, coord: TileCoord) -> Option<&Tile> {
        self.tiles.get(&coord).map(Arc::as_ref)
    }
    /// Immutable identity for renderer caches. Weak references do not retain pixel buffers.
    pub fn tile_ref(&self, coord: TileCoord) -> Option<&Arc<Tile>> {
        self.tiles.get(&coord)
    }

    pub fn try_pixel(&self, x: u32, y: u32) -> crate::Result<Pixel> {
        let coord = TileCoord {
            x: x / TILE_SIZE,
            y: y / TILE_SIZE,
        };
        self.tiles
            .get(&coord)
            .map_or(Ok(self.default_pixel(i64::from(x), i64::from(y))), |t| {
                t.try_pixel(x % TILE_SIZE, y % TILE_SIZE)
            })
    }
    pub fn pixel(&self, x: u32, y: u32) -> Pixel {
        self.try_pixel(x, y)
            .expect("layer read failed; use try_pixel")
    }
    pub fn canvas_pixel(&self, x: u32, y: u32) -> crate::Result<Pixel> {
        let x = i64::from(x) - i64::from(self.appearance.offset_x);
        let y = i64::from(y) - i64::from(self.appearance.offset_y);
        let bound = i64::from(crate::MAX_DIMENSION) * 2;
        if x < -bound || x >= bound || y < -bound || y >= bound {
            return Err(crate::Error::InvalidArgument(
                "sample outside coordinate budget",
            ));
        }
        // Sampling must not allocate a row for every pixel of a tile read.
        let coord = TileCoord::from_signed(x.div_euclid(64) as i32, y.div_euclid(64) as i32);
        self.tiles
            .get(&coord)
            .map_or(Ok(self.default_pixel(x, y)), |tile| {
                tile.try_pixel(x.rem_euclid(64) as u32, y.rem_euclid(64) as u32)
            })
    }
    pub fn read_canvas_row(&self, x: u32, y: u32, count: u32) -> crate::Result<Vec<Pixel>> {
        self.read_local_row(
            i64::from(x) - i64::from(self.appearance.offset_x),
            i64::from(y) - i64::from(self.appearance.offset_y),
            count,
        )
    }
    pub fn canvas_tile_sources(&self, coord: TileCoord) -> Vec<&Arc<Tile>> {
        let x = i64::from(coord.x) * 64 - i64::from(self.appearance.offset_x);
        let y = i64::from(coord.y) * 64 - i64::from(self.appearance.offset_y);
        let mut out = Vec::new();
        for ty in y.div_euclid(64)..=(y + 63).div_euclid(64) {
            for tx in x.div_euclid(64)..=(x + 63).div_euclid(64) {
                if let Some(t) = self
                    .tiles
                    .get(&TileCoord::from_signed(tx as i32, ty as i32))
                {
                    out.push(t);
                }
            }
        }
        out
    }
    /// Raw local pixels (including retained off-canvas content), with signed tile-boundary pins.
    pub fn read_local_row(&self, x: i64, y: i64, count: u32) -> crate::Result<Vec<Pixel>> {
        let end = x
            .checked_add(i64::from(count))
            .ok_or(crate::Error::InvalidArgument("row overflow"))?;
        let bound = i64::from(crate::MAX_DIMENSION) * 2;
        if count > crate::MAX_DIMENSION || x < -bound || end > bound || y < -bound || y >= bound {
            return Err(crate::Error::InvalidArgument(
                "local row outside coordinate budget",
            ));
        }
        let mut row = vec![Pixel::TRANSPARENT; count as usize];
        if y >= 0 && y < i64::from(self.white.1) {
            let a = x.max(0);
            let b = end.min(i64::from(self.white.0));
            if a < b {
                row[(a - x) as usize..(b - x) as usize].fill(Pixel::WHITE);
            }
        }
        let mut col = x;
        while col < end {
            let stop = end.min((col.div_euclid(64) + 1) * 64);
            if let Some(tile) = self.tiles.get(&TileCoord::from_signed(
                col.div_euclid(64) as i32,
                y.div_euclid(64) as i32,
            )) {
                let pixels = tile.try_pixels()?;
                let start = (y.rem_euclid(64) * 64 + col.rem_euclid(64)) as usize;
                row[(col - x) as usize..(stop - x) as usize]
                    .copy_from_slice(&pixels[start..start + (stop - col) as usize]);
            }
            col = stop;
        }
        Ok(row)
    }
    pub fn read_row(&self, x: u32, y: u32, count: u32) -> crate::Result<Vec<Pixel>> {
        self.read_local_row(i64::from(x), i64::from(y), count)
    }
}
