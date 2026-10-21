//! Whole-stroke outline settings and bounded raster geometry, independent of UI and IO.
use crate::{BrushTexture, Error, Result, SecondaryTip};
use std::sync::Arc;

pub const STROKE_OUTLINE_MODEL: &str = "stroke-contour-v1";
pub const STROKE_SDF_OUTLINE_MODEL: &str = "stroke-sdf-v1";
pub const STROKE_REPLAY_OUTLINE_MODEL: &str = "stroke-replay-v1";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutlineAlgorithm {
    #[default]
    Path,
    Sdf,
    Replay,
}

pub const MAX_STROKE_OUTLINE_PIXELS: usize = 8 * 1024 * 1024;
pub const MAX_STROKE_OUTLINE_WIDTH: f32 = 2000.;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutlinePlacement {
    #[default]
    Outside,
    Center,
    Inside,
}
impl OutlinePlacement {
    /// Width is the complete band width, including both halves of a centred band.
    pub fn extents(self, width: f32) -> (f32, f32) {
        match self {
            Self::Outside => (width, 0.),
            Self::Center => (width / 2., width / 2.),
            Self::Inside => (0., width),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OutlineWidth {
    relative: bool,
    pixels: f32,
    percent: f32,
}
impl Default for OutlineWidth {
    fn default() -> Self {
        Self {
            relative: false,
            pixels: 2.,
            percent: 10.,
        }
    }
}
impl OutlineWidth {
    pub fn new(relative: bool, pixels: f32, percent: f32) -> Result<Self> {
        if !pixels.is_finite()
            || !(0. ..=MAX_STROKE_OUTLINE_WIDTH).contains(&pixels)
            || !percent.is_finite()
            || !(0. ..=10000.).contains(&percent)
        {
            return Err(Error::InvalidArgument("stroke outline width"));
        }
        Ok(Self {
            relative,
            pixels,
            percent,
        })
    }
    pub fn relative(self) -> bool {
        self.relative
    }
    pub fn pixels(self) -> f32 {
        self.pixels
    }
    pub fn percent(self) -> f32 {
        self.percent
    }
    /// The caller captures the nominal diameter once when beginning the stroke.
    pub fn effective(self, diameter: f32) -> Result<f32> {
        if !diameter.is_finite() || !(1. ..=crate::MAX_BRUSH_RADIUS * 2.).contains(&diameter) {
            return Err(Error::InvalidArgument("stroke outline brush diameter"));
        }
        Ok(if self.relative {
            (diameter * self.percent / 100.).min(MAX_STROKE_OUTLINE_WIDTH)
        } else {
            self.pixels
        })
    }
}

/// Existing brushes are immutable snapshots; a source preset cannot form a cycle.
#[derive(Clone, Debug, Default)]
pub enum OutlineSource {
    #[default]
    SelfBrush,
    /// Main stroke geometry, with a separately captured outline-only texture.
    SelfTextured(BrushTexture),
    Existing(Arc<SecondaryTip>),
    Texture(BrushTexture),
}

#[derive(Clone, Debug, Default)]
pub struct StrokeOutline {
    algorithm: OutlineAlgorithm,
    enabled: bool,
    placement: OutlinePlacement,
    width: OutlineWidth,
    source: OutlineSource,
    secondary_color: bool,
    allow_overflow: bool,
}
impl StrokeOutline {
    pub fn new(
        enabled: bool,
        placement: OutlinePlacement,
        width: OutlineWidth,
        source: OutlineSource,
        secondary_color: bool,
    ) -> Result<Self> {
        if enabled && matches!(&source, OutlineSource::Texture(t) if !t.enabled()) {
            return Err(Error::InvalidArgument("stroke outline texture is disabled"));
        }
        Ok(Self {
            algorithm: OutlineAlgorithm::Path,
            enabled,
            placement,
            width,
            source,
            secondary_color,
            allow_overflow: false,
        })
    }
    pub fn with_algorithm(mut self, algorithm: OutlineAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }
    pub fn algorithm(&self) -> OutlineAlgorithm {
        self.algorithm
    }
    pub fn with_allow_overflow(mut self, allow: bool) -> Self {
        self.allow_overflow = allow;
        self
    }
    pub fn allow_overflow(&self) -> bool {
        self.allow_overflow
    }
    pub fn model(&self) -> &'static str {
        match self.algorithm {
            OutlineAlgorithm::Path => STROKE_OUTLINE_MODEL,
            OutlineAlgorithm::Sdf => STROKE_SDF_OUTLINE_MODEL,
            OutlineAlgorithm::Replay => STROKE_REPLAY_OUTLINE_MODEL,
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn placement(&self) -> OutlinePlacement {
        self.placement
    }
    pub fn width(&self) -> OutlineWidth {
        self.width
    }
    pub fn source(&self) -> &OutlineSource {
        &self.source
    }
    pub fn secondary_color(&self) -> bool {
        self.secondary_color
    }
    pub fn is_default(&self) -> bool {
        self.algorithm == OutlineAlgorithm::Path
            && !self.enabled
            && self.placement == OutlinePlacement::default()
            && self.width == OutlineWidth::default()
            && matches!(self.source, OutlineSource::SelfBrush)
            && !self.secondary_color
            && !self.allow_overflow
    }
    pub fn validate(&self) -> Result<()> {
        if self.enabled && matches!(&self.source, OutlineSource::Texture(t) if !t.enabled()) {
            return Err(Error::InvalidArgument("stroke outline texture is disabled"));
        }
        Ok(())
    }
}

fn allocated<T: Clone>(count: usize, value: T) -> Result<Vec<T>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| Error::ResourceLimit("stroke outline scratch allocation"))?;
    output.resize(count, value);
    Ok(output)
}

/// Antialiased band around the UNION of the supplied stroke coverage, at 50%.
/// The caller supplies transparent padding for an outside band. Beyond this
/// raster is transparent, including when every supplied pixel is covered.
/// No document pixels are read: drawing the same colour over itself keeps its contour.
pub fn stroke_outline_band(
    width: u32,
    height: u32,
    coverage: &[u8],
    band_width: f32,
    placement: OutlinePlacement,
) -> Result<Vec<f32>> {
    if width == 0 || height == 0 {
        return Err(Error::InvalidArgument("stroke outline raster dimensions"));
    }
    let count = (width as usize)
        .checked_mul(height as usize)
        .ok_or(Error::ResourceLimit("stroke outline raster size"))?;
    if count > MAX_STROKE_OUTLINE_PIXELS {
        return Err(Error::ResourceLimit("stroke outline raster pixel limit"));
    }
    if coverage.len() != count
        || !band_width.is_finite()
        || !(0. ..=MAX_STROKE_OUTLINE_WIDTH).contains(&band_width)
    {
        return Err(Error::InvalidArgument("stroke outline raster parameters"));
    }
    let mut output = allocated(count, 0_f32)?;
    if band_width == 0. || !coverage.iter().any(|v| *v >= 128) {
        return Ok(output);
    }
    let (width, height) = (width as usize, height as usize);
    let mut distance = allocated(count, f64::INFINITY)?;
    let (outside, inside) = placement.extents(band_width);
    for target_inside in [true, false] {
        for (d, value) in distance.iter_mut().zip(coverage) {
            *d = if (*value >= 128) == target_inside {
                0.
            } else {
                f64::INFINITY
            };
        }
        paint_ops::sdf::squared_distance_transform(width as u32, height as u32, &mut distance)
            .map_err(|error| match error {
                paint_ops::sdf::Error::Invalid(_) => {
                    Error::InvalidArgument("stroke outline distance")
                }
                paint_ops::sdf::Error::Limit(_) => Error::ResourceLimit("stroke outline distance"),
            })?;
        for y in 0..height {
            for (x, &squared) in distance[y * width..(y + 1) * width].iter().enumerate() {
                let i = y * width + x;
                if (coverage[i] >= 128) == target_inside {
                    continue;
                }
                let mut squared = squared;
                if !target_inside {
                    let border = (x + 1).min(width - x).min(y + 1).min(height - y) as f64;
                    squared = squared.min(border * border);
                }
                let distance = squared.sqrt() - 0.5;
                let signed = if target_inside { -distance } else { distance };
                output[i] = ((signed + f64::from(outside) + 0.5).clamp(0., 1.)
                    * (f64::from(inside) - signed + 0.5).clamp(0., 1.))
                    as f32;
            }
        }
    }
    Ok(output)
}
