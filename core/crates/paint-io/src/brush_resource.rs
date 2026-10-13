//! Content-addressed immutable tip PNGs. This module performs no document mutation.
use crate::{check, Error, Result};
use image::{
    codecs::png::{CompressionType, FilterType, PngDecoder, PngEncoder},
    ColorType, ExtendedColorType, ImageDecoder, ImageEncoder, Limits,
};
use paint_core::{BrushMask, MAX_BRUSH_MASK_EDGE};
use paint_task::CancellationToken;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
};
pub const MAX_RESOURCE_PNG_BYTES: u64 = 128 * 1024 * 1024;
pub const DEFAULT_RESOURCE_BUDGET: usize = 128 * 1024 * 1024;
pub const MAX_RESOURCE_FILES: usize = 4096;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interpretation {
    Coverage,
    Alpha,
    InverseLuminance,
}
#[derive(Clone, Debug)]
pub struct StoredMask {
    pub id: String,
    pub mask: Arc<BrushMask>,
    pub source_depth: u8,
}
impl StoredMask {
    pub fn quantized_from_16(&self) -> bool {
        self.source_depth == 16
    }
}
struct Cached {
    mask: Weak<BrushMask>,
    source_depth: u8,
}
pub struct ResourceStore {
    directory: PathBuf,
    budget: usize,
    cache: Mutex<BTreeMap<String, Cached>>,
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn valid_id(id: &str) -> bool {
    id.len() == 64
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn estimate(width: u32, height: u32) -> usize {
    let (mut width, mut height) = (width, height);
    let mut bytes = 0;
    loop {
        bytes += width as usize * height as usize;
        if width == 1 && height == 1 {
            return bytes;
        }
        width = width.div_ceil(2);
        height = height.div_ceil(2);
    }
}
fn decoder(bytes: &[u8]) -> Result<PngDecoder<Cursor<&[u8]>>> {
    if bytes.len() as u64 > MAX_RESOURCE_PNG_BYTES {
        return Err(Error::Limit("brush PNG bytes"));
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_BRUSH_MASK_EDGE);
    limits.max_image_height = Some(MAX_BRUSH_MASK_EDGE);
    limits.max_alloc = Some(128 * 1024 * 1024);
    let decoder = PngDecoder::with_limits(Cursor::new(bytes), limits)?;
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 {
        return Err(Error::Invalid("brush PNG dimensions"));
    }
    if decoder.is_apng()? {
        return Err(Error::Unsupported("animated brush PNG".into()));
    }
    Ok(decoder)
}
fn coverage(
    decoder: PngDecoder<Cursor<&[u8]>>,
    interpretation: Interpretation,
    token: &CancellationToken,
) -> Result<(u32, u32, Vec<u8>)> {
    let (w, h) = decoder.dimensions();
    let color = decoder.color_type();
    let channels = color.channel_count() as usize;
    let depth = match color {
        ColorType::L8 | ColorType::La8 | ColorType::Rgb8 | ColorType::Rgba8 => 1,
        ColorType::L16 | ColorType::La16 | ColorType::Rgb16 | ColorType::Rgba16 => 2,
        _ => return Err(Error::Unsupported("brush PNG channels".into())),
    };
    let mut raw = vec![0; decoder.total_bytes() as usize];
    decoder.read_image(&mut raw)?;
    check(token)?;
    let maximum = if depth == 1 { 255u64 } else { 65535 };
    let mut mask = Vec::with_capacity(w as usize * h as usize);
    for (i, pixel) in raw.chunks_exact(channels * depth).enumerate() {
        if i % 65536 == 0 {
            check(token)?;
        }
        let value = |channel: usize| -> u64 {
            if depth == 1 {
                u64::from(pixel[channel])
            } else {
                u64::from(u16::from_ne_bytes([
                    pixel[channel * 2],
                    pixel[channel * 2 + 1],
                ]))
            }
        };
        let alpha = if channels == 2 || channels == 4 {
            value(channels - 1)
        } else {
            maximum
        };
        let gray = if channels < 3 {
            value(0)
        } else {
            if interpretation == Interpretation::Coverage
                && (value(0) != value(1) || value(0) != value(2))
            {
                return Err(Error::Unsupported(
                    "tip-mask RGB pixels must be grayscale".into(),
                ));
            }
            (value(0) * 2126 + value(1) * 7152 + value(2) * 722 + 5000) / 10000
        };
        let numerator = match interpretation {
            Interpretation::Alpha => alpha * maximum,
            Interpretation::Coverage => gray * alpha,
            Interpretation::InverseLuminance => (maximum - gray) * alpha,
        };
        mask.push(((numerator * 255 + maximum * maximum / 2) / (maximum * maximum)) as u8);
    }
    Ok((w, h, mask))
}
pub fn encode_mask(mask: &BrushMask) -> Result<Vec<u8>> {
    let (w, h) = mask.dimensions();
    encode_pixels(w, h, mask.coverage())
}
fn encode_pixels(w: u32, h: u32, coverage: &[u8]) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    PngEncoder::new_with_quality(&mut bytes, CompressionType::Best, FilterType::Adaptive)
        .write_image(coverage, w, h, ExtendedColorType::L8)?;
    Ok(bytes)
}
impl ResourceStore {
    pub fn open(directory: PathBuf, budget: usize) -> Result<Self> {
        if budget == 0 || budget > DEFAULT_RESOURCE_BUDGET {
            return Err(Error::Invalid("brush resource budget"));
        }
        std::fs::create_dir_all(&directory)?;
        Ok(Self {
            directory,
            budget,
            cache: Mutex::new(BTreeMap::new()),
        })
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn resident_bytes(&self) -> usize {
        self.cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .filter_map(|v| v.mask.upgrade())
            .map(|m| m.resident_bytes())
            .sum()
    }
    fn reserve(&self, cache: &mut BTreeMap<String, Cached>, bytes: usize) -> Result<()> {
        cache.retain(|_, v| v.mask.strong_count() > 0);
        let resident: usize = cache
            .values()
            .filter_map(|v| v.mask.upgrade())
            .map(|m| m.resident_bytes())
            .sum();
        if resident.saturating_add(bytes) > self.budget {
            return Err(Error::Limit("live brush resource mip budget"));
        }
        Ok(())
    }
    fn write(&self, id: &str, bytes: &[u8], token: &CancellationToken) -> Result<()> {
        check(token)?;
        let path = self.directory.join(format!("{id}.png"));
        if path.exists() {
            let existing = self.read(id)?;
            if hash(&existing) != id {
                return Err(Error::Invalid("existing brush resource hash mismatch"));
            }
            check(token)?;
            return Ok(());
        }
        let mut count = 0;
        for entry in std::fs::read_dir(&self.directory)? {
            if entry?.path().extension().is_some_and(|v| v == "png") {
                count += 1;
            }
        }
        if count >= MAX_RESOURCE_FILES {
            return Err(Error::Limit("brush resource file count"));
        }
        check(token)?;
        let mut temporary = tempfile::Builder::new()
            .prefix(".brush-")
            .tempfile_in(&self.directory)?;
        temporary.write_all(bytes)?;
        temporary.flush()?;
        temporary.as_file().sync_all()?;
        check(token)?;
        match temporary.persist_noclobber(&path) {
            Ok(_) => Ok(()),
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                if hash(&self.read(id)?) != id {
                    return Err(Error::Invalid("brush resource hash mismatch"));
                }
                Ok(())
            }
            Err(e) => Err(Error::Io(e.error)),
        }
    }
    fn read(&self, id: &str) -> Result<Vec<u8>> {
        if !valid_id(id) {
            return Err(Error::Invalid("brush resource ID"));
        }
        let path = self.directory.join(format!("{id}.png"));
        if std::fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err(Error::Invalid("brush resource symlink"));
        }
        let mut file = File::open(path)?;
        if file.metadata()?.len() > MAX_RESOURCE_PNG_BYTES {
            return Err(Error::Limit("brush PNG bytes"));
        }
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(MAX_RESOURCE_PNG_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_RESOURCE_PNG_BYTES {
            return Err(Error::Limit("brush PNG bytes"));
        }
        Ok(bytes)
    }
    pub fn load(&self, id: &str, token: &CancellationToken) -> Result<StoredMask> {
        check(token)?;
        if !valid_id(id) {
            return Err(Error::Invalid("brush resource ID"));
        }
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| Error::Invalid("brush resource cache poisoned"))?;
        if let Some(cached) = cache.get(id) {
            if let Some(mask) = cached.mask.upgrade() {
                return Ok(StoredMask {
                    id: id.into(),
                    mask,
                    source_depth: cached.source_depth,
                });
            }
        }
        let bytes = self.read(id)?;
        if hash(&bytes) != id {
            return Err(Error::Invalid("brush resource hash mismatch"));
        }
        let decoder = decoder(&bytes)?;
        let (w, h) = decoder.dimensions();
        self.reserve(&mut cache, estimate(w, h))?;
        let (w, h, pixels) = coverage(decoder, Interpretation::Coverage, token)?;
        let mask = Arc::new(BrushMask::new(w, h, pixels)?);
        check(token)?;
        let source_depth = bytes[24];
        cache.insert(
            id.into(),
            Cached {
                mask: Arc::downgrade(&mask),
                source_depth,
            },
        );
        Ok(StoredMask {
            id: id.into(),
            mask,
            source_depth,
        })
    }
    /// Import a declared standard VBR resource, retaining its full-byte identity.
    pub fn put_verified(
        &self,
        id: &str,
        bytes: &[u8],
        token: &CancellationToken,
    ) -> Result<StoredMask> {
        check(token)?;
        if !valid_id(id) || hash(bytes) != id {
            return Err(Error::Invalid("brush PNG hash"));
        }
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| Error::Invalid("brush resource cache poisoned"))?;
        if let Some(cached) = cache.get(id) {
            if let Some(mask) = cached.mask.upgrade() {
                self.write(id, bytes, token)?;
                return Ok(StoredMask {
                    id: id.into(),
                    mask,
                    source_depth: cached.source_depth,
                });
            }
        }
        let decoder = decoder(bytes)?;
        let (w, h) = decoder.dimensions();
        self.reserve(&mut cache, estimate(w, h))?;
        let (w, h, pixels) = coverage(decoder, Interpretation::Coverage, token)?;
        let mask = Arc::new(BrushMask::new(w, h, pixels)?);
        self.write(id, bytes, token)?;
        let source_depth = bytes[24];
        cache.insert(
            id.into(),
            Cached {
                mask: Arc::downgrade(&mask),
                source_depth,
            },
        );
        Ok(StoredMask {
            id: id.into(),
            mask,
            source_depth,
        })
    }
    /// Normalize explicitly interpreted user images into portable grayscale coverage.
    pub fn import_png(
        &self,
        bytes: &[u8],
        interpretation: Interpretation,
        token: &CancellationToken,
    ) -> Result<StoredMask> {
        check(token)?;
        let decoder = decoder(bytes)?;
        let (w, h, pixels) = coverage(decoder, interpretation, token)?;
        let normalized = encode_pixels(w, h, &pixels)?;
        check(token)?;
        let id = hash(&normalized);
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| Error::Invalid("brush resource cache poisoned"))?;
        if let Some(cached) = cache.get(&id) {
            if let Some(existing) = cached.mask.upgrade() {
                self.write(&id, &normalized, token)?;
                return Ok(StoredMask {
                    id,
                    mask: existing,
                    source_depth: bytes[24],
                });
            }
        }
        self.reserve(&mut cache, estimate(w, h))?;
        let mask = Arc::new(BrushMask::new(w, h, pixels)?);
        self.write(&id, &normalized, token)?;
        cache.insert(
            id.clone(),
            Cached {
                mask: Arc::downgrade(&mask),
                source_depth: 8,
            },
        );
        Ok(StoredMask {
            id,
            mask,
            source_depth: bytes[24],
        })
    }
    pub fn encoded(&self, id: &str) -> Result<Vec<u8>> {
        let bytes = self.read(id)?;
        if hash(&bytes) != id {
            return Err(Error::Invalid("brush resource hash mismatch"));
        }
        Ok(bytes)
    }
}
