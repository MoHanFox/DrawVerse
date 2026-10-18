use crate::{
    page_pool::{Page, PagePool},
    Pixel, Result,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex, Weak,
};
pub const TILE_SIZE: u32 = 64;
pub const TILE_BYTES: usize = TILE_SIZE as usize * TILE_SIZE as usize * 16;
static NEXT_TILE: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileCoord {
    pub x: u32,
    pub y: u32,
}
impl TileCoord {
    /// Local coordinates use two's complement in Rust only; no C ABI representation.
    pub fn from_signed(x: i32, y: i32) -> Self {
        Self {
            x: x as u32,
            y: y as u32,
        }
    }
    pub fn signed_x(self) -> i32 {
        self.x as i32
    }
    pub fn signed_y(self) -> i32 {
        self.y as i32
    }
}
#[derive(Debug)]
pub(crate) struct State {
    // Separate Arc header from the boxed pixel allocation: Vec -> Arc<[Pixel]>
    // would allocate and copy an entire tile again on every brush dab.
    pub resident: Option<Arc<Box<[Pixel]>>>,
    pub backing: Option<Arc<Page>>,
    pub pool: Weak<PagePool>,
}
/// Pixels are immutable. Paging only changes storage representation.
#[derive(Debug)]
pub struct Tile {
    pub(crate) id: u64,
    pub(crate) state: Mutex<State>,
    empty: bool,
}
impl Clone for Tile {
    fn clone(&self) -> Self {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        Self {
            id: NEXT_TILE.fetch_add(1, Ordering::Relaxed),
            state: Mutex::new(State {
                // Owned staging copies must not permanently pin another
                // cache's resident buffer. Snapshot sharing uses Arc<Tile>.
                resident: state
                    .resident
                    .as_ref()
                    .map(|pixels| Arc::new(pixels.as_ref().clone())),
                backing: state.backing.clone(),
                pool: Weak::new(),
            }),
            empty: self.empty,
        }
    }
}
impl PartialEq for Tile {
    fn eq(&self, other: &Self) -> bool {
        self.pixels().as_ref() == other.pixels().as_ref()
    }
}
impl Default for Tile {
    fn default() -> Self {
        Self::from_pixels(vec![Pixel::TRANSPARENT; 4096]).expect("fixed tile size")
    }
}
impl Tile {
    pub(crate) fn can_edit_private(tile: &Arc<Self>) -> bool {
        if Arc::strong_count(tile) != 1 {
            return false;
        }
        let state = tile.state.lock().unwrap_or_else(|e| e.into_inner());
        // One owner plus this dab's read pin. Shared snapshots or cold backing
        // use the immutable copy path instead.
        state.backing.is_none()
            && state
                .resident
                .as_ref()
                .is_some_and(|pixels| Arc::strong_count(pixels) == 2)
    }
    pub(crate) fn edit_private(
        tile: &mut Arc<Self>,
        pool: &Arc<PagePool>,
        updates: &[(usize, Pixel)],
    ) -> Result<()> {
        let old = Arc::downgrade(tile);
        pool.untrack(tile.id);
        // make_mut invalidates renderer/cache Weak identities before changing
        // pixels. If an eviction worker acquired an owner meanwhile, clone.
        let unique = Arc::make_mut(tile);
        let state = unique.state.get_mut().unwrap_or_else(|e| e.into_inner());
        state.pool = Weak::new();
        if state.resident.is_none() {
            state.resident = Some(
                state
                    .backing
                    .as_ref()
                    .ok_or_else(|| crate::Error::Storage("missing private tile backing".into()))?
                    .read()?,
            );
        }
        let pixels = Arc::make_mut(state.resident.as_mut().expect("private pixels loaded"));
        for (offset, pixel) in updates {
            pixels[*offset] = *pixel;
        }
        unique.empty = pixels.iter().all(|pixel| pixel.alpha() == 0.);
        state.backing = None;
        // A Weak reader/eviction worker may have acquired the original after
        // can_edit_private. Keep that immutable version inside the page budget.
        if let Some(original) = old.upgrade() {
            pool.attach(&original)?;
        }
        pool.attach(tile)
    }
    pub fn from_pixels(pixels: Vec<Pixel>) -> Result<Self> {
        if pixels.len() != 4096 {
            return Err(crate::Error::InvalidArgument(
                "a tile requires exactly 4096 pixels",
            ));
        }
        let empty = pixels.iter().all(|p| p.alpha() == 0.);
        Ok(Self {
            id: NEXT_TILE.fetch_add(1, Ordering::Relaxed),
            state: Mutex::new(State {
                resident: Some(Arc::new(pixels.into_boxed_slice())),
                backing: None,
                pool: Weak::new(),
            }),
            empty,
        })
    }
    /// Pin once per tile batch. Never replace an IO failure with transparent data.
    pub fn try_pixels(&self) -> Result<Arc<Box<[Pixel]>>> {
        let (resident, backing, pool) = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            (
                state.resident.clone(),
                state.backing.clone(),
                state.pool.upgrade(),
            )
        };
        if let Some(pixels) = resident {
            if let Some(pool) = pool {
                pool.touch(self.id, false)?;
            }
            return Ok(pixels);
        }
        let pixels = backing
            .ok_or_else(|| crate::Error::Storage("tile has no backing".into()))?
            .read()?;
        let pixels = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            Arc::clone(state.resident.get_or_insert(pixels))
        };
        if let Some(pool) = pool {
            if let Err(error) = pool.touch(self.id, true) {
                // Failed eviction must not turn repeated failed reads into an
                // ever-growing resident cache. The verified backing remains.
                pool.discard_loaded(self, &pixels);
                return Err(error);
            }
        }
        Ok(pixels)
    }
    /// Convenience for tests/reference sampling. Production IO uses try_pixels.
    pub fn pixels(&self) -> Arc<Box<[Pixel]>> {
        self.try_pixels()
            .expect("tile read failed; use try_pixels to handle storage errors")
    }
    pub fn try_pixel(&self, x: u32, y: u32) -> Result<Pixel> {
        if x >= 64 || y >= 64 {
            return Ok(Pixel::TRANSPARENT);
        }
        Ok(self.try_pixels()?[(y * 64 + x) as usize])
    }
    pub fn pixel(&self, x: u32, y: u32) -> Pixel {
        self.try_pixel(x, y)
            .expect("tile read failed; use try_pixel to handle storage errors")
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.empty
    }
}
impl Drop for Tile {
    fn drop(&mut self) {
        if let Some(pool) = self
            .state
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .pool
            .upgrade()
        {
            pool.untrack(self.id);
        }
    }
}
