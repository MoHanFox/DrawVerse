//! Whole-stroke outline settings and bounded raster geometry, independent of UI and IO.
use crate::{BrushTexture, Error, Result, SecondaryTip};
use std::sync::Arc;

pub const STROKE_REPLAY_OUTLINE_MODEL: &str = "stroke-replay-v1";

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
            enabled,
            placement,
            width,
            source,
            secondary_color,
            allow_overflow: false,
        })
    }
    pub fn with_allow_overflow(mut self, allow: bool) -> Self {
        self.allow_overflow = allow;
        self
    }
    pub fn allow_overflow(&self) -> bool {
        self.allow_overflow
    }
    pub fn model(&self) -> &'static str {
        STROKE_REPLAY_OUTLINE_MODEL
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
        !self.enabled
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
