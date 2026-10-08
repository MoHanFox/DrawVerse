//! Bounded codecs and atomic files. File operations run on background workers only.
mod ora;
mod raster;
use paint_core::{Document, DocumentSnapshot};
use paint_task::CancellationToken;
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};
pub const MAX_PIXELS: u64 = 134_217_728;
pub const MAX_EDGE: u32 = 16_384;
pub const MAX_INPUT: u64 = 256 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Jpeg,
    WebP,
    OpenRaster,
}
#[derive(Clone, Copy, Debug)]
pub struct ExportOptions {
    pub format: Format,
    pub quality: u8,
    pub background: [f32; 3],
}
impl ExportOptions {
    pub fn validate(self) -> Result<()> {
        if !(1..=100).contains(&self.quality)
            || self
                .background
                .iter()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(Error::Invalid("invalid JPEG quality or linear background"));
        }
        Ok(())
    }
}
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Codec(String),
    Invalid(&'static str),
    Unsupported(String),
    Limit(&'static str),
    Cancelled,
    Core(paint_core::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "file IO: {e}"),
            Self::Codec(e) => write!(f, "invalid image: {e}"),
            Self::Invalid(e) => write!(f, "invalid input: {e}"),
            Self::Unsupported(e) => write!(f, "unsupported: {e}"),
            Self::Limit(e) => write!(f, "file resource limit: {e}"),
            Self::Cancelled => write!(f, "file operation cancelled"),
            Self::Core(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<image::ImageError> for Error {
    fn from(e: image::ImageError) -> Self {
        Self::Codec(e.to_string())
    }
}
impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Self::Codec(e.to_string())
    }
}
impl From<paint_core::Error> for Error {
    fn from(e: paint_core::Error) -> Self {
        Self::Core(e)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn check(token: &CancellationToken) -> Result<()> {
    if token.is_cancelled() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
pub(crate) fn dimensions(w: u32, h: u32) -> Result<()> {
    if w == 0 || h == 0 || w > MAX_EDGE || h > MAX_EDGE || u64::from(w) * u64::from(h) > MAX_PIXELS
    {
        return Err(Error::Limit("raster dimensions/pixels"));
    }
    Ok(())
}
pub fn load(path: &Path, token: &CancellationToken) -> Result<Document> {
    load_with_format(path, token).map(|(doc, _)| doc)
}
/// The detected signature, rather than a suffix, determines editable save behavior.
pub fn load_with_format(path: &Path, token: &CancellationToken) -> Result<(Document, Format)> {
    load_with_storage(
        path,
        token,
        paint_core::DocumentOptions::default(),
        paint_storage::ScratchSpace::system(),
    )
}
pub fn load_with_storage(
    path: &Path,
    token: &CancellationToken,
    options: paint_core::DocumentOptions,
    storage: std::sync::Arc<paint_storage::ScratchSpace>,
) -> Result<(Document, Format)> {
    check(token)?;
    let mut file = File::open(path)?;
    if file.metadata()?.len() > MAX_INPUT {
        return Err(Error::Limit("input exceeds 256 MiB"));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_INPUT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_INPUT {
        return Err(Error::Limit("input exceeds 256 MiB"));
    }
    check(token)?;
    if bytes.starts_with(b"PK\x03\x04") {
        ora::decode(&bytes, token, options, storage).map(|doc| (doc, Format::OpenRaster))
    } else {
        let format = match image::guess_format(&bytes)? {
            image::ImageFormat::Png => Format::Png,
            image::ImageFormat::Jpeg => Format::Jpeg,
            image::ImageFormat::WebP => Format::WebP,
            _ => return Err(Error::Unsupported("image format".into())),
        };
        raster::decode_document(&bytes, token, options, storage).map(|doc| (doc, format))
    }
}
pub fn save(
    path: &Path,
    snapshot: &DocumentSnapshot,
    active: u64,
    options: ExportOptions,
    token: &CancellationToken,
) -> Result<()> {
    options.validate()?;
    dimensions(snapshot.width, snapshot.height)?;
    check(token)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::Builder::new()
        .prefix(".drawverse-")
        .tempfile_in(parent)?;
    if options.format == Format::OpenRaster {
        ora::encode(&mut temporary, snapshot, active, token)?;
    } else {
        raster::encode_snapshot(&mut temporary, snapshot, options, token)?;
    }
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    check(token)?;
    // Persist is the commit point. A subsequent cancellation cannot undo the file.
    temporary.persist(path).map_err(|e| Error::Io(e.error))?;
    Ok(())
}
