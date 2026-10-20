//! Bounded CPU reference operators for alpha masks and distance-field outlines.
//! Pixel centres are `(x + .5, y + .5)`; no document or device is owned here.
mod contours;
mod distance;
mod mask;

pub use contours::{Isolines, Stamp};
pub use distance::{squared_distance_transform, DistanceField};
pub use mask::{combine_masks, compose_outline, disk_coverage, rasterize_max, MaskOp};

pub const MAX_PIXELS: usize = 8 * 1024 * 1024;
pub const MAX_SEGMENTS: usize = 262_144;
pub const MAX_STAMPS: usize = 262_144;
pub const MAX_STAMP_VISITS: usize = 64_000_000;
pub const MAX_OUTLINE_WIDTH: f32 = 2000.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid(&'static str),
    Limit(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(s) => write!(f, "invalid SDF {s}"),
            Self::Limit(s) => write!(f, "SDF resource limit: {s}"),
        }
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    Outside,
    Inside,
    Center,
}
impl Placement {
    pub fn level(self, width: f32) -> f32 {
        match self {
            Self::Outside => width * 0.5,
            Self::Inside => -width * 0.5,
            Self::Center => 0.,
        }
    }
}

fn count(width: u32, height: u32) -> Result<usize> {
    if width == 0 || height == 0 {
        return Err(Error::Invalid("dimensions"));
    }
    let n = u64::from(width) * u64::from(height);
    if n > MAX_PIXELS as u64 {
        return Err(Error::Limit("pixels"));
    }
    Ok(n as usize)
}
fn allocated<T: Clone>(n: usize, value: T) -> Result<Vec<T>> {
    let mut v = Vec::new();
    v.try_reserve_exact(n)
        .map_err(|_| Error::Limit("allocation"))?;
    v.resize(n, value);
    Ok(v)
}
fn push<T>(v: &mut Vec<T>, value: T, max: usize) -> Result<()> {
    if v.len() == max {
        return Err(Error::Limit("geometry"));
    }
    v.try_reserve(1).map_err(|_| Error::Limit("allocation"))?;
    v.push(value);
    Ok(())
}
fn valid_width(width: f32) -> Result<()> {
    if !width.is_finite() || !(0. ..=MAX_OUTLINE_WIDTH).contains(&width) {
        return Err(Error::Invalid("outline width"));
    }
    Ok(())
}
fn smooth(v: f32) -> f32 {
    let t = v.clamp(0., 1.);
    t * t * (3. - 2. * t)
}
