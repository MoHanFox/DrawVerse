//! Whole-stroke outline settings and bounded raster geometry, independent of UI and IO.
use crate::{BrushTexture, Error, Result, SecondaryTip};
use std::sync::Arc;

pub const STROKE_OUTLINE_MODEL: &str = "stroke-contour-v1";
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
    Existing(Arc<SecondaryTip>),
    Texture(BrushTexture),
}

#[derive(Clone, Debug, Default)]
pub struct StrokeOutline {
    enabled: bool,
    placement: OutlinePlacement,
    width: OutlineWidth,
    source: OutlineSource,
    secondary_color: bool,
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
            enabled,
            placement,
            width,
            source,
            secondary_color,
        })
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
}

fn allocated<T: Clone>(count: usize, value: T) -> Result<Vec<T>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(count)
        .map_err(|_| Error::ResourceLimit("stroke outline scratch allocation"))?;
    output.resize(count, value);
    Ok(output)
}

/// Squared distance to the closest finite parabola, in one linear pass.
fn distance_line(input: &[f64], output: &mut [f64], sites: &mut [usize], edges: &mut [f64]) {
    let mut count = 0;
    for (q, &value) in input.iter().enumerate().filter(|(_, v)| v.is_finite()) {
        let mut crossing = f64::NEG_INFINITY;
        while count > 0 {
            let p = sites[count - 1];
            crossing = ((value + (q as f64).powi(2)) - (input[p] + (p as f64).powi(2)))
                / (2. * (q - p) as f64);
            if crossing > edges[count - 1] {
                break;
            }
            count -= 1;
        }
        sites[count] = q;
        edges[count] = if count == 0 {
            f64::NEG_INFINITY
        } else {
            crossing
        };
        count += 1;
    }
    if count == 0 {
        output.fill(f64::INFINITY);
        return;
    }
    let mut site = 0;
    for (q, result) in output.iter_mut().enumerate() {
        while site + 1 < count && edges[site + 1] < q as f64 {
            site += 1;
        }
        let p = sites[site];
        *result = (q as f64 - p as f64).powi(2) + input[p];
    }
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
    let longest = width.max(height);
    let mut distance = allocated(count, f64::INFINITY)?;
    let mut line = allocated(longest, 0_f64)?;
    let mut transformed = allocated(longest, 0_f64)?;
    let mut sites = allocated(longest, 0_usize)?;
    let mut edges = allocated(longest, 0_f64)?;
    let (outside, inside) = placement.extents(band_width);
    for target_inside in [true, false] {
        for (d, value) in distance.iter_mut().zip(coverage) {
            *d = if (*value >= 128) == target_inside {
                0.
            } else {
                f64::INFINITY
            };
        }
        for x in 0..width {
            for y in 0..height {
                line[y] = distance[y * width + x];
            }
            distance_line(
                &line[..height],
                &mut transformed[..height],
                &mut sites,
                &mut edges,
            );
            for y in 0..height {
                distance[y * width + x] = transformed[y];
            }
        }
        for y in 0..height {
            distance_line(
                &distance[y * width..(y + 1) * width],
                &mut transformed[..width],
                &mut sites,
                &mut edges,
            );
            for (x, &squared) in transformed[..width].iter().enumerate() {
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
