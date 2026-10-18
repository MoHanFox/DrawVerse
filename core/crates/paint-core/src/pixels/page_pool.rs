//! Paging changes storage, never immutable pixel identity.
use crate::{history_storage, Error, Result, Tile};
use paint_storage::{ManagedFile, ScratchSpace};
use std::{
    collections::{BTreeSet, HashMap},
    io::{Read, Seek, SeekFrom, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, Weak,
    },
};
const BUCKETS: [usize; 8] = [512, 1024, 2048, 4096, 8192, 16384, 32768, 65552];
#[derive(Clone, Copy, Debug, Default)]
pub struct StorageStats {
    pub resident_tiles: usize,
    pub registered_tiles: usize,
    pub scratch_allocated_bytes: u64,
    pub scratch_live_bytes: u64,
    pub page_reads: u64,
    pub page_writes: u64,
}
#[derive(Debug)]
struct Node {
    tile: Weak<Tile>,
    stamp: u64,
    resident: bool,
}
#[derive(Debug, Default)]
struct Cache {
    nodes: HashMap<u64, Node>,
    order: BTreeSet<(u64, u64)>,
    clock: u64,
}
#[derive(Debug, Default)]
struct Disk {
    file: Option<ManagedFile>,
    end: u64,
    live: u64,
    free: [Vec<u64>; 8],
    reads: u64,
    writes: u64,
}
#[derive(Debug)]
pub(crate) struct PagePool {
    storage: Arc<ScratchSpace>,
    limit: usize,
    disk_limit: u64,
    resident: AtomicUsize,
    cache: Mutex<Cache>,
    disk: Mutex<Disk>,
    trim_lock: Mutex<()>,
}
#[derive(Debug)]
pub(crate) struct Page {
    pool: Arc<PagePool>,
    offset: u64,
    bucket: usize,
    length: usize,
    checksum: u64,
}
fn error(e: impl std::fmt::Display) -> Error {
    Error::Storage(e.to_string())
}
impl PagePool {
    #[cfg(test)]
    pub fn new(limit: usize, disk_limit: u64) -> Arc<Self> {
        Self::with_storage(limit, disk_limit, ScratchSpace::system())
    }
    pub fn with_storage(limit: usize, disk_limit: u64, storage: Arc<ScratchSpace>) -> Arc<Self> {
        Arc::new(Self {
            storage,
            limit,
            disk_limit,
            resident: AtomicUsize::new(0),
            cache: Mutex::new(Cache::default()),
            disk: Mutex::new(Disk::default()),
            trim_lock: Mutex::new(()),
        })
    }
    pub fn attach(self: &Arc<Self>, tile: &Arc<Tile>) -> Result<()> {
        let resident = {
            let mut state = tile.state.lock().unwrap_or_else(|e| e.into_inner());
            if state
                .pool
                .upgrade()
                .is_some_and(|old| !Arc::ptr_eq(&old, self))
            {
                return Err(Error::InvalidArgument(
                    "tile already belongs to another document cache",
                ));
            }
            if state
                .pool
                .upgrade()
                .is_some_and(|old| Arc::ptr_eq(&old, self))
                && self
                    .cache
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .nodes
                    .contains_key(&tile.id)
            {
                return Ok(());
            }
            // An owned import clone can retain a cold page from its source.
            // Detach that backing after loading, so this pool's disk quota and
            // lifetime account for all of its pixel storage.
            if state
                .backing
                .as_ref()
                .is_some_and(|page| !Arc::ptr_eq(&page.pool, self))
            {
                if state.resident.is_none() {
                    state.resident = Some(state.backing.as_ref().expect("foreign backing").read()?);
                }
                state.backing = None;
            }
            state.pool = Arc::downgrade(self);
            state.resident.is_some()
        };
        {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            cache.clock += 1;
            let stamp = cache.clock;
            cache.nodes.insert(
                tile.id,
                Node {
                    tile: Arc::downgrade(tile),
                    stamp,
                    resident,
                },
            );
            if resident {
                cache.order.insert((stamp, tile.id));
                self.resident.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.trim()
    }
    pub fn untrack(&self, id: u64) {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(node) = cache.nodes.remove(&id) {
            cache.order.remove(&(node.stamp, id));
            if node.resident {
                self.resident.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }
    pub fn touch(self: &Arc<Self>, id: u64, loaded: bool) -> Result<()> {
        if !loaded && self.resident.load(Ordering::Relaxed) < self.limit {
            return Ok(());
        }
        {
            let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            cache.clock += 1;
            let stamp = cache.clock;
            let Some(node) = cache.nodes.get_mut(&id) else {
                return Ok(());
            };
            let old = node.stamp;
            let was = node.resident;
            node.stamp = stamp;
            if !was && loaded {
                node.resident = true;
                self.resident.fetch_add(1, Ordering::Relaxed);
            }
            let now = node.resident;
            cache.order.remove(&(old, id));
            if now {
                cache.order.insert((stamp, id));
            }
        }
        if loaded {
            self.trim()?;
        }
        Ok(())
    }
    fn cold(&self, id: u64) {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(node) = cache.nodes.get_mut(&id) {
            let stamp = node.stamp;
            if node.resident {
                node.resident = false;
                self.resident.fetch_sub(1, Ordering::Relaxed);
            }
            cache.order.remove(&(stamp, id));
        }
    }
    pub fn discard_loaded(&self, tile: &Tile, pixels: &Arc<Box<[crate::Pixel]>>) {
        let mut state = tile.state.lock().unwrap_or_else(|e| e.into_inner());
        if state
            .resident
            .as_ref()
            .is_some_and(|value| Arc::ptr_eq(value, pixels))
        {
            state.resident = None;
            self.cold(tile.id);
        }
    }
    pub fn trim(self: &Arc<Self>) -> Result<()> {
        if self.resident.load(Ordering::Relaxed) <= self.limit {
            return Ok(());
        }
        let _guard = self.trim_lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut attempts = self.resident.load(Ordering::Relaxed);
        while self.resident.load(Ordering::Relaxed) > self.limit && attempts > 0 {
            attempts -= 1;
            let (id, tile) = {
                let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
                let Some((_, id)) = cache.order.first().copied() else {
                    break;
                };
                cache.clock += 1;
                let stamp = cache.clock;
                let node = cache.nodes.get_mut(&id).expect("LRU node");
                let old = node.stamp;
                node.stamp = stamp;
                let tile = node.tile.upgrade();
                cache.order.remove(&(old, id));
                cache.order.insert((stamp, id));
                (id, tile)
            };
            let Some(tile) = tile else {
                self.untrack(id);
                continue;
            };
            {
                let mut state = tile.state.lock().unwrap_or_else(|e| e.into_inner());
                match &state.resident {
                    Some(pixels) if Arc::strong_count(pixels) == 1 => {
                        if state.backing.is_none() {
                            state.backing = Some(self.write(pixels)?);
                        }
                        state.resident = None;
                        // Tile state -> registry is the only nested lock order.
                        // Publish eviction before unlocking state, so a concurrent
                        // reload cannot lose its residency registration.
                        self.cold(id);
                    }
                    None => {
                        self.cold(id);
                    }
                    _ => {}
                }
            };
            drop(tile);
        }
        Ok(())
    }
    fn write(self: &Arc<Self>, pixels: &[crate::Pixel]) -> Result<Arc<Page>> {
        #[cfg(test)]
        if FAIL_WRITES.with(|value| value.get()) {
            return Err(error("injected scratch write failure"));
        }
        let mut bytes = Vec::new();
        history_storage::encode_pixels(&mut bytes, pixels).map_err(error)?;
        let bucket = BUCKETS
            .iter()
            .position(|size| *size >= bytes.len())
            .ok_or_else(|| error("invalid page length"))?;
        let mut disk = self.disk.lock().unwrap_or_else(|e| e.into_inner());
        let recycled = disk.free[bucket].pop();
        let offset = match recycled {
            Some(offset) => offset,
            None => {
                if disk.end.saturating_add(BUCKETS[bucket] as u64) > self.disk_limit {
                    return Err(error("document scratch capacity exhausted"));
                }
                disk.end
            }
        };
        let result = (|| -> std::io::Result<()> {
            if disk.file.is_none() {
                disk.file = Some(self.storage.tempfile("pages")?);
            }
            let file = disk.file.as_mut().expect("scratch opened");
            file.seek(SeekFrom::Start(offset))?;
            file.write_all(&bytes)?;
            Ok(())
        })();
        if let Err(e) = result {
            if recycled.is_some() {
                disk.free[bucket].push(offset);
            }
            return Err(error(e));
        }
        if recycled.is_none() {
            disk.end += BUCKETS[bucket] as u64;
        }
        disk.live += BUCKETS[bucket] as u64;
        disk.writes += 1;
        Ok(Arc::new(Page {
            pool: Arc::clone(self),
            offset,
            bucket,
            length: bytes.len(),
            checksum: history_storage::hash(history_storage::HASH_START, &bytes),
        }))
    }
    pub fn storage(&self) -> Arc<ScratchSpace> {
        Arc::clone(&self.storage)
    }
    pub fn stats(&self) -> StorageStats {
        let (resident_tiles, registered_tiles) = {
            let cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
            (self.resident.load(Ordering::Relaxed), cache.nodes.len())
        };
        let disk = self.disk.lock().unwrap_or_else(|e| e.into_inner());
        StorageStats {
            resident_tiles,
            registered_tiles,
            scratch_allocated_bytes: disk.end,
            scratch_live_bytes: disk.live,
            page_reads: disk.reads,
            page_writes: disk.writes,
        }
    }
    #[cfg(test)]
    pub(crate) fn corrupt_for_test(&self) {
        let mut disk = self.disk.lock().unwrap();
        disk.file
            .as_mut()
            .unwrap()
            .seek(SeekFrom::Start(0))
            .unwrap();
        disk.file.as_mut().unwrap().write_all(&[255]).unwrap();
    }
}
#[cfg(test)]
thread_local! {static FAIL_WRITES:std::cell::Cell<bool>=const {std::cell::Cell::new(false)};}
#[cfg(test)]
pub(crate) fn fail_writes_for_test(value: bool) {
    FAIL_WRITES.with(|flag| flag.set(value));
}
impl Page {
    pub fn read(&self) -> Result<Arc<Box<[crate::Pixel]>>> {
        let mut bytes = vec![0; self.length];
        {
            let mut disk = self.pool.disk.lock().unwrap_or_else(|e| e.into_inner());
            let file = disk
                .file
                .as_mut()
                .ok_or_else(|| error("missing document scratch"))?;
            file.seek(SeekFrom::Start(self.offset)).map_err(error)?;
            file.read_exact(&mut bytes).map_err(error)?;
            disk.reads += 1;
        }
        if history_storage::hash(history_storage::HASH_START, &bytes) != self.checksum {
            return Err(error("document scratch checksum mismatch"));
        }
        Ok(Arc::new(
            history_storage::decode_page(&bytes)
                .map_err(error)?
                .into_boxed_slice(),
        ))
    }
}
impl Drop for Page {
    fn drop(&mut self) {
        let mut disk = self.pool.disk.lock().unwrap_or_else(|e| e.into_inner());
        disk.live -= BUCKETS[self.bucket] as u64;
        disk.free[self.bucket].push(self.offset);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Pixel;
    fn tile(value: f32) -> Arc<Tile> {
        Arc::new(
            Tile::from_pixels(vec![
                Pixel::from_straight([value, 0.2, 0.4, 1.]).unwrap();
                4096
            ])
            .unwrap(),
        )
    }
    #[test]
    fn pins_are_not_evicted_and_scratch_lives_until_last_snapshot_tile() {
        let pool = PagePool::new(1, 65552);
        let a = tile(0.3);
        pool.attach(&a).unwrap();
        let pin = a.try_pixels().unwrap();
        let b = tile(0.4);
        pool.attach(&b).unwrap();
        assert_eq!(pin[0].components()[0], 0.3);
        assert!(a.state.lock().unwrap().resident.is_some());
        drop(pin);
        // Reload b while pinned; a must now acquire its own backing page.
        let pin = b.try_pixels().unwrap();
        pool.trim().unwrap();
        drop(pin);
        let path = pool
            .disk
            .lock()
            .unwrap()
            .file
            .as_ref()
            .unwrap()
            .path()
            .to_owned();
        drop(b);
        drop(pool);
        assert!(path.exists());
        assert_eq!(a.try_pixels().unwrap()[0].components()[0], 0.3);
        drop(a);
        assert!(!path.exists());
    }
    #[test]
    fn freed_blocks_are_reused_without_aliasing_retained_pages() {
        let pool = PagePool::new(1, 65552);
        let a = tile(0.3);
        pool.attach(&a).unwrap();
        let b = tile(0.4);
        pool.attach(&b).unwrap();
        let original = pool.stats().scratch_allocated_bytes;
        assert_eq!(a.try_pixels().unwrap()[0].components()[0], 0.3);
        pool.trim().unwrap();
        drop(a);
        drop(b);
        let allocated = pool.stats().scratch_allocated_bytes;
        assert!(allocated >= original);
        for i in 0..200 {
            let a = tile(i as f32 / 200.);
            pool.attach(&a).unwrap();
            let b = tile(0.8);
            pool.attach(&b).unwrap();
            assert_eq!(a.try_pixels().unwrap()[0].components()[0], i as f32 / 200.);
            drop(a);
            drop(b);
        }
        assert_eq!(pool.stats().scratch_live_bytes, 0);
        assert!(pool.stats().scratch_allocated_bytes <= allocated + 512);
    }
    #[test]
    fn checksum_failure_is_a_storage_error() {
        let pool = PagePool::new(1, 65552);
        let a = tile(0.3);
        pool.attach(&a).unwrap();
        let b = tile(0.4);
        pool.attach(&b).unwrap();
        pool.corrupt_for_test();
        assert!(matches!(a.try_pixels(), Err(Error::Storage(_))));
        assert_eq!(b.try_pixels().unwrap()[0].components()[0], 0.4);
    }
    #[test]
    fn owned_cold_clone_uses_destination_budget_and_releases_source_file() {
        let source = PagePool::new(1, 65552);
        let a = tile(0.3);
        source.attach(&a).unwrap();
        let b = tile(0.4);
        source.attach(&b).unwrap();
        let path = source
            .disk
            .lock()
            .unwrap()
            .file
            .as_ref()
            .unwrap()
            .path()
            .to_owned();
        let copied = Arc::new(a.as_ref().clone());
        let destination = PagePool::new(1, 65552);
        destination.attach(&copied).unwrap();
        let c = tile(0.8);
        destination.attach(&c).unwrap();
        assert!(destination.stats().scratch_live_bytes > 0);
        drop(a);
        drop(b);
        drop(source);
        assert!(!path.exists());
        assert_eq!(copied.try_pixels().unwrap()[0].components()[0], 0.3);
        destination.trim().unwrap();
        assert!(destination.stats().resident_tiles <= 1);
    }
    #[test]
    fn private_edit_re_registers_an_original_acquired_by_a_weak_reader() {
        let pool = PagePool::new(1, 65552);
        let mut current = tile(0.3);
        pool.attach(&current).unwrap();
        let pin = current.try_pixels().unwrap();
        assert!(Tile::can_edit_private(&current));
        // Simulate a reader winning the gap between the eligibility check
        // and make_mut. Both versions must remain accounted and exact.
        let original = Arc::downgrade(&current).upgrade().unwrap();
        drop(pin);
        Tile::edit_private(
            &mut current,
            &pool,
            &[(0, crate::Pixel::from_straight([0.7, 0.2, 0.4, 1.]).unwrap())],
        )
        .unwrap();
        assert_eq!(original.try_pixels().unwrap()[0].components()[0], 0.3);
        assert_eq!(current.try_pixels().unwrap()[0].components()[0], 0.7);
        pool.trim().unwrap();
        assert_eq!(pool.stats().registered_tiles, 2);
        assert!(pool.stats().resident_tiles <= 1);
    }
    #[test]
    fn parallel_readers_and_eviction_preserve_exact_pixels_and_resident_accounting() {
        let pool = PagePool::new(2, 1024 * 1024);
        let tiles: Vec<_> = (0..16)
            .map(|i| {
                let tile = tile(i as f32 / 16.);
                pool.attach(&tile).unwrap();
                tile
            })
            .collect();
        std::thread::scope(|scope| {
            for offset in 0..4 {
                let tiles = &tiles;
                let pool = &pool;
                scope.spawn(move || {
                    for i in 0..100 {
                        let n = (i + offset) % 16;
                        assert_eq!(
                            tiles[n].try_pixels().unwrap()[0].components()[0],
                            n as f32 / 16.
                        );
                        pool.trim().unwrap();
                    }
                });
            }
        });
        pool.trim().unwrap();
        let actual = tiles
            .iter()
            .filter(|tile| tile.state.lock().unwrap().resident.is_some())
            .count();
        assert_eq!(pool.stats().resident_tiles, actual);
        assert!(actual <= 2);
    }
}
