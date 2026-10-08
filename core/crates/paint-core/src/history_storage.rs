//! Private, volatile history backing. Encoding is lossless and never touches Qt.
use crate::{
    history::{Command, TileChange, MAX_DISK_HISTORY_BYTES},
    Error, Layer, LayerProperties, Pixel, Result, Tile, TileCoord, TILE_BYTES, TILE_SIZE,
};
use paint_storage::{ManagedFile, ScratchSpace};
use std::{
    io::{self, BufReader, BufWriter, Cursor, Read, Write},
    sync::Arc,
};

pub(crate) const HASH_START: u64 = 0xcbf29ce484222325;
#[cfg(test)]
thread_local! { static FAIL_WRITES: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
#[cfg(test)]
pub(crate) fn fail_writes_for_test(value: bool) {
    FAIL_WRITES.with(|flag| flag.set(value));
}
pub(crate) fn hash(mut value: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        value = (value ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
    }
    value
}
fn storage_error(error: impl std::fmt::Display) -> Error {
    Error::HistoryStorage(error.to_string())
}
fn invalid() -> Error {
    storage_error("invalid or corrupted internal history")
}

#[derive(Debug)]
enum Backing {
    Memory(Box<[u8]>),
    Disk(ManagedFile),
}
#[derive(Debug)]
pub(crate) struct Payload {
    backing: Backing,
    checksum: u64,
    length: u64,
    tile_limit: usize,
}
struct SpillWriter {
    storage: Arc<ScratchSpace>,
    memory: Vec<u8>,
    file: Option<ManagedFile>,
    memory_limit: usize,
    length: u64,
    checksum: u64,
}
impl Write for SpillWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        #[cfg(test)]
        if FAIL_WRITES.with(|flag| flag.get()) {
            return Err(io::Error::other("simulated scratch write failure"));
        }
        if self.length.saturating_add(bytes.len() as u64) > MAX_DISK_HISTORY_BYTES {
            return Err(io::Error::other(
                "single history command exceeds 4GiB scratch budget",
            ));
        }
        if self.file.is_none() && self.memory.len().saturating_add(bytes.len()) > self.memory_limit
        {
            // tempfile creates an exclusive, private file; no predictable-name truncation.
            let mut file = self.storage.tempfile("history")?;
            file.write_all(&self.memory)?;
            self.file = Some(file);
            self.memory = Vec::new();
        }
        if let Some(file) = &mut self.file {
            file.write_all(bytes)?;
        } else {
            let required = self.memory.len() + bytes.len();
            if required > self.memory.capacity() {
                let capacity = required
                    .max(self.memory.capacity().saturating_mul(2))
                    .min(self.memory_limit);
                self.memory.reserve_exact(capacity - self.memory.len());
            }
            self.memory.extend_from_slice(bytes);
        }
        self.checksum = hash(self.checksum, bytes);
        self.length += bytes.len() as u64;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if let Some(file) = &mut self.file {
            file.flush()?;
        }
        Ok(())
    }
}
struct CheckedReader<R> {
    source: R,
    checksum: u64,
    length: u64,
    tile_limit: usize,
    pool: Option<Arc<crate::page_pool::PagePool>>,
}
impl<R: Read> Read for CheckedReader<R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let count = self.source.read(bytes)?;
        self.checksum = hash(self.checksum, &bytes[..count]);
        self.length += count as u64;
        Ok(count)
    }
}
impl Payload {
    #[cfg(test)]
    pub(crate) fn disk_path_for_test(&self) -> std::path::PathBuf {
        match &self.backing {
            Backing::Disk(file) => file.path().to_owned(),
            _ => panic!("disk history expected"),
        }
    }
    #[cfg(test)]
    pub(crate) fn truncate_for_test(&mut self) {
        match &mut self.backing {
            Backing::Memory(bytes) => *bytes = bytes[..3].into(),
            Backing::Disk(file) => file.as_file_mut().set_len(3).unwrap(),
        }
    }
    #[cfg(test)]
    pub fn encode(command: &Command, memory_limit: usize) -> Result<Self> {
        Self::encode_in(command, memory_limit, ScratchSpace::system())
    }
    pub fn encode_in(
        command: &Command,
        memory_limit: usize,
        storage: Arc<ScratchSpace>,
    ) -> Result<Self> {
        let mut writer = BufWriter::new(SpillWriter {
            storage,
            memory: Vec::new(),
            file: None,
            memory_limit,
            length: 0,
            checksum: HASH_START,
        });
        encode_command(&mut writer, command).map_err(storage_error)?;
        writer.flush().map_err(storage_error)?;
        let writer = writer.into_inner().map_err(storage_error)?;
        let backing = match writer.file {
            Some(file) => Backing::Disk(file),
            None => Backing::Memory(writer.memory.into_boxed_slice()),
        };
        Ok(Self {
            backing,
            checksum: writer.checksum,
            length: writer.length,
            tile_limit: command.bytes() / TILE_BYTES + 2,
        })
    }
    pub fn memory_bytes(&self) -> usize {
        match &self.backing {
            Backing::Memory(bytes) => bytes.len(),
            Backing::Disk(_) => 0,
        }
    }
    pub fn disk_bytes(&self) -> u64 {
        match self.backing {
            Backing::Disk(_) => self.length,
            Backing::Memory(_) => 0,
        }
    }
    #[cfg(test)]
    pub fn decode(&self) -> Result<Command> {
        self.decode_in(None)
    }
    pub fn decode_in(&self, pool: Option<&Arc<crate::page_pool::PagePool>>) -> Result<Command> {
        match &self.backing {
            Backing::Memory(bytes) => self.read_checked(Cursor::new(bytes), pool),
            Backing::Disk(file) => {
                let file = file.reopen().map_err(storage_error)?;
                self.read_checked(BufReader::new(file), pool)
            }
        }
    }
    fn read_checked(
        &self,
        source: impl Read,
        pool: Option<&Arc<crate::page_pool::PagePool>>,
    ) -> Result<Command> {
        let mut reader = CheckedReader {
            source,
            checksum: HASH_START,
            length: 0,
            tile_limit: self.tile_limit,
            pool: pool.cloned(),
        };
        let command = decode_command(&mut reader)?;
        let mut extra = [0];
        if reader.read(&mut extra).map_err(storage_error)? != 0
            || reader.length != self.length
            || reader.checksum != self.checksum
        {
            return Err(invalid());
        }
        Ok(command)
    }
}
fn put_u8(w: &mut impl Write, value: u8) -> io::Result<()> {
    w.write_all(&[value])
}
fn put_u32(w: &mut impl Write, value: u32) -> io::Result<()> {
    w.write_all(&value.to_le_bytes())
}
fn put_u64(w: &mut impl Write, value: u64) -> io::Result<()> {
    w.write_all(&value.to_le_bytes())
}
fn get<const N: usize>(r: &mut impl Read) -> Result<[u8; N]> {
    let mut bytes = [0; N];
    r.read_exact(&mut bytes).map_err(storage_error)?;
    Ok(bytes)
}
fn u8_value(r: &mut impl Read) -> Result<u8> {
    Ok(get::<1>(r)?[0])
}
fn u32_value(r: &mut impl Read) -> Result<u32> {
    Ok(u32::from_le_bytes(get(r)?))
}
fn u64_value(r: &mut impl Read) -> Result<u64> {
    Ok(u64::from_le_bytes(get(r)?))
}
fn boolean(r: &mut impl Read) -> Result<bool> {
    match u8_value(r)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(invalid()),
    }
}
fn properties(w: &mut impl Write, p: LayerProperties) -> io::Result<()> {
    put_u8(w, u8::from(p.visible))?;
    put_u32(w, p.opacity.to_bits())
}
fn read_properties(r: &mut impl Read) -> Result<LayerProperties> {
    let visible = boolean(r)?;
    let opacity = f32::from_bits(u32_value(r)?);
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return Err(invalid());
    }
    Ok(LayerProperties { visible, opacity })
}
fn coordinate(w: &mut impl Write, c: TileCoord) -> io::Result<()> {
    put_u32(w, c.x)?;
    put_u32(w, c.y)
}
fn read_coordinate(r: &mut impl Read) -> Result<TileCoord> {
    let coord = TileCoord {
        x: u32_value(r)?,
        y: u32_value(r)?,
    };
    if coord.signed_x().unsigned_abs() > crate::MAX_DIMENSION.div_ceil(TILE_SIZE) * 2
        || coord.signed_y().unsigned_abs() > crate::MAX_DIMENSION.div_ceil(TILE_SIZE) * 2
    {
        return Err(invalid());
    }
    Ok(coord)
}
fn bits(p: Pixel) -> [u32; 4] {
    p.components().map(f32::to_bits)
}
fn encode_tile(w: &mut impl Write, tile: Option<&Arc<Tile>>) -> io::Result<()> {
    let Some(tile) = tile else {
        return put_u8(w, 0);
    };
    let pixels = tile.try_pixels().map_err(io::Error::other)?;
    encode_pixels(w, &pixels)
}
pub(crate) fn encode_pixels(w: &mut impl Write, pixels: &[Pixel]) -> io::Result<()> {
    let mut runs = Vec::new();
    let mut index = 0;
    while index < pixels.len() {
        let pixel = bits(pixels[index]);
        let start = index;
        index += 1;
        while index < pixels.len() && bits(pixels[index]) == pixel {
            index += 1;
        }
        runs.extend_from_slice(&((index - start) as u16).to_le_bytes());
        for channel in pixel {
            runs.extend_from_slice(&channel.to_le_bytes());
        }
        if runs.len() >= TILE_BYTES {
            break;
        }
    }
    if runs.len() < TILE_BYTES {
        put_u8(w, 2)?;
        put_u32(w, runs.len() as u32)?;
        w.write_all(&runs)
    } else {
        put_u8(w, 1)?;
        for pixel in pixels {
            for channel in bits(*pixel) {
                put_u32(w, channel)?;
            }
        }
        Ok(())
    }
}
fn read_pixel(r: &mut impl Read) -> Result<Pixel> {
    Pixel::from_premultiplied([
        f32::from_bits(u32_value(r)?),
        f32::from_bits(u32_value(r)?),
        f32::from_bits(u32_value(r)?),
        f32::from_bits(u32_value(r)?),
    ])
}
fn decode_tile<R: Read>(r: &mut CheckedReader<R>) -> Result<Option<Arc<Tile>>> {
    let kind = u8_value(r)?;
    if kind == 0 {
        return Ok(None);
    }
    if r.tile_limit == 0 {
        return Err(invalid());
    }
    r.tile_limit -= 1;
    let pixels = decode_pixels(r, kind)?;
    let tile = Arc::new(Tile::from_pixels(pixels)?);
    if let Some(pool) = &r.pool {
        pool.attach(&tile)?;
    }
    Ok(Some(tile))
}
fn decode_pixels(r: &mut impl Read, kind: u8) -> Result<Vec<Pixel>> {
    let count = (TILE_SIZE * TILE_SIZE) as usize;
    let mut pixels = Vec::with_capacity(count);
    match kind {
        1 => {
            for _ in 0..count {
                pixels.push(read_pixel(r)?);
            }
        }
        2 => {
            let length = u32_value(r)? as usize;
            if length == 0 || length >= TILE_BYTES || length % 18 != 0 {
                return Err(invalid());
            }
            for _ in 0..length / 18 {
                let repeats = u16::from_le_bytes(get(r)?) as usize;
                let pixel = read_pixel(r)?;
                if repeats == 0 || pixels.len() + repeats > count {
                    return Err(invalid());
                }
                pixels.resize(pixels.len() + repeats, pixel);
            }
            if pixels.len() != count {
                return Err(invalid());
            }
        }
        _ => return Err(invalid()),
    }
    Ok(pixels)
}
fn encode_layer(w: &mut impl Write, index: usize, layer: &Layer) -> io::Result<()> {
    put_u32(w, index as u32)?;
    put_u64(w, layer.id)?;
    put_u32(w, layer.name.len() as u32)?;
    w.write_all(layer.name.as_bytes())?;
    properties(w, layer.properties)?;
    appearance(w, layer.appearance)?;
    put_u64(w, layer.parent)?;
    put_u8(w, u8::from(layer.group))?;
    put_u8(w, u8::from(layer.mask))?;
    put_u8(w, u8::from(layer.clipped))?;
    put_u32(w, layer.white.0)?;
    put_u32(w, layer.white.1)?;
    put_u32(w, layer.tiles.len() as u32)?;
    for (coord, tile) in &layer.tiles {
        coordinate(w, *coord)?;
        encode_tile(w, Some(tile))?;
    }
    Ok(())
}
fn decode_layer<R: Read>(r: &mut CheckedReader<R>) -> Result<(usize, Layer)> {
    let index = u32_value(r)? as usize;
    let id = u64_value(r)?;
    let length = u32_value(r)? as usize;
    if index >= 4096 || id == 0 || length == 0 || length > 1024 {
        return Err(invalid());
    }
    let mut name = vec![0; length];
    r.read_exact(&mut name).map_err(storage_error)?;
    let name = String::from_utf8(name).map_err(storage_error)?;
    if name.contains('\0') {
        return Err(invalid());
    }
    let mut layer = Layer::new(id, name);
    layer.properties = read_properties(r)?;
    layer.appearance = read_appearance(r)?;
    layer.parent = u64_value(r)?;
    layer.group = boolean(r)?;
    layer.mask = boolean(r)?;
    layer.clipped = boolean(r)?;
    layer.white = (u32_value(r)?, u32_value(r)?);
    if (layer.white.0 == 0) != (layer.white.1 == 0)
        || layer.white.0 > crate::MAX_DIMENSION
        || layer.white.1 > crate::MAX_DIMENSION
        || (layer.mask && (layer.white != (0, 0) || layer.clipped))
    {
        return Err(invalid());
    }
    let count = u32_value(r)? as usize;
    if count > r.tile_limit {
        return Err(invalid());
    }
    for _ in 0..count {
        let coord = read_coordinate(r)?;
        let tile = decode_tile(r)?.ok_or_else(invalid)?;
        if layer.tiles.insert(coord, tile).is_some() {
            return Err(invalid());
        }
    }
    Ok((index, layer))
}
fn appearance(w: &mut impl Write, a: crate::LayerAppearance) -> io::Result<()> {
    put_u32(w, a.fill.to_bits())?;
    put_u32(w, a.dissolve_seed)?;
    put_u32(w, a.blend as u32)?;
    put_u32(w, a.locks)?;
    put_u32(w, a.offset_x as u32)?;
    put_u32(w, a.offset_y as u32)
}
fn read_appearance(r: &mut impl Read) -> Result<crate::LayerAppearance> {
    let a = crate::LayerAppearance {
        fill: f32::from_bits(u32_value(r)?),
        dissolve_seed: u32_value(r)?,
        blend: crate::BlendMode::from_id(u32_value(r)?).map_err(|_| invalid())?,
        locks: u32_value(r)?,
        offset_x: u32_value(r)? as i32,
        offset_y: u32_value(r)? as i32,
    };
    a.validate().map_err(|_| invalid())?;
    Ok(a)
}
fn encode_command(w: &mut impl Write, command: &Command) -> io::Result<()> {
    w.write_all(b"DVH6")?;
    match command {
        Command::Selection { before, after } => {
            put_u8(w, 8)?;
            for selection in [before, after] {
                let text = selection.to_text();
                put_u32(w, text.len() as u32)?;
                w.write_all(text.as_bytes())?;
            }
            Ok(())
        }
        Command::Structure { before, after } => {
            put_u8(w, 6)?;
            for nodes in [before, after] {
                put_u32(w, nodes.len() as u32)?;
                for (index, node) in nodes.iter().enumerate() {
                    encode_layer(w, index, node)?;
                }
            }
            Ok(())
        }
        Command::RemoveTree { index, layers } => {
            put_u8(w, 7)?;
            put_u32(w, *index as u32)?;
            put_u32(w, layers.len() as u32)?;
            for (i, layer) in layers.iter().enumerate() {
                encode_layer(w, i, layer)?;
            }
            Ok(())
        }
        Command::Stroke { layer, changes } => {
            put_u8(w, 1)?;
            put_u64(w, *layer)?;
            put_u32(w, changes.len() as u32)?;
            for change in changes {
                coordinate(w, change.coord)?;
                encode_tile(w, change.before.as_ref())?;
                encode_tile(w, change.after.as_ref())?;
            }
            Ok(())
        }
        Command::AddLayer { index, layer } => {
            put_u8(w, 2)?;
            encode_layer(w, *index, layer)
        }
        Command::RemoveLayer { index, layer } => {
            put_u8(w, 3)?;
            encode_layer(w, *index, layer)
        }
        Command::Appearance {
            layer,
            before,
            after,
        } => {
            put_u8(w, 5)?;
            put_u64(w, *layer)?;
            appearance(w, *before)?;
            appearance(w, *after)
        }
        Command::Properties {
            layer,
            before,
            after,
        } => {
            put_u8(w, 4)?;
            put_u64(w, *layer)?;
            properties(w, *before)?;
            properties(w, *after)
        }
    }
}
fn decode_command<R: Read>(r: &mut CheckedReader<R>) -> Result<Command> {
    if &get::<4>(r)? != b"DVH6" {
        return Err(invalid());
    }
    match u8_value(r)? {
        8 => {
            let mut selections = Vec::new();
            for _ in 0..2 {
                let count = u32_value(r)? as usize;
                if count > 32768 {
                    return Err(invalid());
                }
                let mut bytes = vec![0; count];
                r.read_exact(&mut bytes).map_err(storage_error)?;
                let text = std::str::from_utf8(&bytes).map_err(|_| invalid())?;
                selections.push(crate::Selection::from_text(text).map_err(|_| invalid())?);
            }
            let after = selections.pop().expect("two selections");
            let before = selections.pop().expect("two selections");
            Ok(Command::Selection { before, after })
        }
        6 => {
            let mut sets = Vec::new();
            for _ in 0..2 {
                let count = u32_value(r)? as usize;
                if count == 0 || count > 4096 {
                    return Err(invalid());
                }
                let mut nodes = Vec::with_capacity(count);
                for i in 0..count {
                    let (index, node) = decode_layer(r)?;
                    if index != i || !node.tiles.is_empty() {
                        return Err(invalid());
                    }
                    nodes.push(node);
                }
                crate::document::groups::validate(&nodes).map_err(|_| invalid())?;
                sets.push(nodes);
            }
            let after = sets.pop().expect("two sets");
            let before = sets.pop().expect("two sets");
            // Structure may create/delete groups only; raster IDs must match.
            let ids = |nodes: &[Layer]| {
                nodes
                    .iter()
                    .filter(|l| !l.group)
                    .map(|l| l.id)
                    .collect::<std::collections::BTreeSet<_>>()
            };
            if ids(&before) != ids(&after) {
                return Err(invalid());
            }
            Ok(Command::Structure { before, after })
        }
        7 => {
            let index = u32_value(r)? as usize;
            let count = u32_value(r)? as usize;
            if count == 0 || index + count > 4096 {
                return Err(invalid());
            }
            let mut layers = Vec::with_capacity(count);
            for i in 0..count {
                let (j, layer) = decode_layer(r)?;
                if i != j {
                    return Err(invalid());
                }
                layers.push(layer);
            }
            Ok(Command::RemoveTree { index, layers })
        }
        5 => {
            let layer = u64_value(r)?;
            if layer == 0 {
                return Err(invalid());
            }
            let before = read_appearance(r)?;
            let after = read_appearance(r)?;
            Ok(Command::Appearance {
                layer,
                before,
                after,
            })
        }
        1 => {
            let layer = u64_value(r)?;
            let count = u32_value(r)? as usize;
            if layer == 0 || count == 0 || count > r.tile_limit {
                return Err(invalid());
            }
            let mut changes = Vec::with_capacity(count);
            let mut seen = std::collections::BTreeSet::new();
            for _ in 0..count {
                let coord = read_coordinate(r)?;
                if !seen.insert(coord) {
                    return Err(invalid());
                }
                changes.push(TileChange {
                    coord,
                    before: decode_tile(r)?,
                    after: decode_tile(r)?,
                });
            }
            Ok(Command::Stroke { layer, changes })
        }
        2 => {
            let (index, layer) = decode_layer(r)?;
            Ok(Command::AddLayer { index, layer })
        }
        3 => {
            let (index, layer) = decode_layer(r)?;
            Ok(Command::RemoveLayer { index, layer })
        }
        4 => {
            let layer = u64_value(r)?;
            if layer == 0 {
                return Err(invalid());
            }
            Ok(Command::Properties {
                layer,
                before: read_properties(r)?,
                after: read_properties(r)?,
            })
        }
        _ => Err(invalid()),
    }
}

pub(crate) fn decode_page(bytes: &[u8]) -> Result<Vec<Pixel>> {
    let mut r = Cursor::new(bytes);
    let kind = u8_value(&mut r)?;
    let pixels = decode_pixels(&mut r, kind)?;
    if r.position() != bytes.len() as u64 {
        return Err(invalid());
    }
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_disk_roundtrip_and_corruption_are_checked() {
        let shape = crate::SelectionShape {
            kind: crate::SelectionKind::Ellipse,
            x: 1.25,
            y: 2.5,
            width: 50.,
            height: 30.,
            antialias: true,
        };
        let before = crate::Selection::default();
        let after = before
            .apply(shape, crate::SelectionOperation::Replace, 100, 100)
            .unwrap()
            .inverted(100, 100)
            .unwrap();
        let command = Command::Selection {
            before: before.clone(),
            after: after.clone(),
        };
        let mut payload = Payload::encode(&command, 0).unwrap();
        match payload.decode().unwrap() {
            Command::Selection {
                before: b,
                after: a,
            } => {
                assert_eq!(b, before);
                assert_eq!(a, after);
            }
            _ => panic!("selection tag"),
        }
        payload.truncate_for_test();
        assert!(payload.decode().is_err());
    }
    fn command(noisy: bool) -> Command {
        let pixels = (0..4096)
            .map(|i| {
                Pixel::from_straight(if noisy {
                    [i as f32 / 4096., ((i * 17) % 4096) as f32 / 4096., 0.25, 1.]
                } else {
                    [0.5, 0.25, 0.125, 0.75]
                })
                .unwrap()
            })
            .collect();
        let tile = Arc::new(Tile::from_pixels(pixels).unwrap());
        Command::Stroke {
            layer: 1,
            changes: vec![TileChange {
                coord: TileCoord { x: 0, y: 0 },
                before: Some(Arc::clone(&tile)),
                after: Some(tile),
            }],
        }
    }
    #[test]
    fn compressed_and_raw_tiles_are_bit_exact_in_memory_and_on_disk() {
        for noisy in [false, true] {
            for budget in [0, 1024 * 1024] {
                let command = command(noisy);
                let payload = Payload::encode(&command, budget).unwrap();
                let restored = payload.decode().unwrap();
                let Command::Stroke {
                    changes: expected, ..
                } = command
                else {
                    unreachable!()
                };
                let Command::Stroke {
                    changes: actual, ..
                } = restored
                else {
                    unreachable!()
                };
                for (a, b) in [
                    (actual[0].before.as_ref(), expected[0].before.as_ref()),
                    (actual[0].after.as_ref(), expected[0].after.as_ref()),
                ] {
                    for (a, b) in a.unwrap().pixels().iter().zip(b.unwrap().pixels().iter()) {
                        assert_eq!(bits(*a), bits(*b));
                    }
                }
                if budget == 0 {
                    assert!(payload.disk_bytes() > 0);
                    assert_eq!(payload.memory_bytes(), 0);
                } else {
                    assert_eq!(payload.disk_bytes(), 0);
                    if !noisy {
                        assert!(payload.memory_bytes() < 128);
                    }
                }
            }
        }
    }
    #[test]
    fn corruption_and_truncation_are_rejected_before_use() {
        let mut payload = Payload::encode(&command(true), 1024 * 1024).unwrap();
        let Backing::Memory(bytes) = &mut payload.backing else {
            unreachable!()
        };
        // Change a raw normalized channel to another valid value: checksum must catch it.
        bytes[26..30].copy_from_slice(&0.125f32.to_bits().to_le_bytes());
        assert!(payload.decode().is_err());
        for budget in [0, 1024 * 1024] {
            let mut payload = Payload::encode(&command(false), budget).unwrap();
            payload.truncate_for_test();
            assert!(payload.decode().is_err());
        }
    }
    #[test]
    fn temporary_history_is_removed_when_payload_is_released() {
        let payload = Payload::encode(&command(false), 0).unwrap();
        let Backing::Disk(file) = &payload.backing else {
            unreachable!()
        };
        let path = file.path().to_owned();
        assert!(path.exists());
        drop(payload);
        assert!(!path.exists());
    }
    #[test]
    fn writer_cap_fails_before_unbounded_disk_growth() {
        let mut writer = SpillWriter {
            storage: ScratchSpace::system(),
            memory: Vec::new(),
            file: None,
            memory_limit: 0,
            length: MAX_DISK_HISTORY_BYTES,
            checksum: HASH_START,
        };
        assert!(writer.write_all(b"more").is_err());
        assert!(writer.file.is_none());
    }
}
