use crate::{BrushMask, Error, Result};
use std::sync::Arc;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextureSpace {
    #[default]
    Document,
    Tip,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextureMode {
    #[default]
    Normal,
    Multiply,
    Darken,
    Lighten,
    Subtract,
    Difference,
    Screen,
    ColorBurn,
    LinearBurn,
    ColorDodge,
    LinearDodge,
    Overlay,
    SoftLight,
    HardLight,
    Exclusion,
    HardMix,
}
impl TextureMode {
    pub fn combine(self, base: f32, texture: f32) -> f32 {
        match self {
            Self::Normal | Self::Multiply => base * texture,
            Self::Darken => base.min(texture),
            Self::Lighten => base.max(texture),
            Self::Subtract => (base - texture).max(0.),
            Self::Difference => (base - texture).abs(),
            Self::Screen => base + texture - base * texture,
            Self::ColorBurn => {
                if base >= 1. {
                    1.
                } else if texture <= 0. {
                    0.
                } else {
                    1. - ((1. - base) / texture).min(1.)
                }
            }
            Self::LinearBurn => (base + texture - 1.).max(0.),
            Self::ColorDodge => {
                if base <= 0. {
                    0.
                } else if texture >= 1. {
                    1.
                } else {
                    (base / (1. - texture)).min(1.)
                }
            }
            Self::LinearDodge => (base + texture).min(1.),
            Self::Overlay => {
                if base < 0.5 {
                    2. * base * texture
                } else {
                    1. - 2. * (1. - base) * (1. - texture)
                }
            }
            Self::SoftLight => {
                if texture <= 0.5 {
                    base - (1. - 2. * texture) * base * (1. - base)
                } else {
                    let d = if base <= 0.25 {
                        ((16. * base - 12.) * base + 4.) * base
                    } else {
                        base.sqrt()
                    };
                    base + (2. * texture - 1.) * (d - base)
                }
            }
            Self::HardLight => {
                if texture < 0.5 {
                    2. * base * texture
                } else {
                    1. - 2. * (1. - base) * (1. - texture)
                }
            }
            Self::Exclusion => base + texture - 2. * base * texture,
            Self::HardMix => {
                if base + texture >= 1. {
                    1.
                } else {
                    0.
                }
            }
        }
        .clamp(0., 1.)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextureSettings {
    space: TextureSpace,
    mode: TextureMode,
    scale: f32,
    angle: f32,
    depth: f32,
    brightness: f32,
    contrast: f32,
    minimum_depth: f32,
    depth_jitter: f32,
    depth_pressure: bool,
    invert: bool,
    offset_x: f64,
    offset_y: f64,
}
impl Default for TextureSettings {
    fn default() -> Self {
        Self {
            space: TextureSpace::Document,
            mode: TextureMode::Normal,
            scale: 1.,
            angle: 0.,
            depth: 1.,
            brightness: 0.,
            contrast: 0.,
            minimum_depth: 0.,
            depth_jitter: 0.,
            depth_pressure: false,
            invert: false,
            offset_x: 0.,
            offset_y: 0.,
        }
    }
}
impl TextureSettings {
    pub fn new(
        space: TextureSpace,
        scale: f32,
        angle: f32,
        depth: f32,
        invert: bool,
        offset_x: f64,
        offset_y: f64,
    ) -> Result<Self> {
        if !scale.is_finite()
            || !(0.01..=100.).contains(&scale)
            || !angle.is_finite()
            || !(-180. ..=180.).contains(&angle)
            || !depth.is_finite()
            || !(0. ..=1.).contains(&depth)
            || [offset_x, offset_y]
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
        {
            return Err(Error::InvalidArgument("texture parameters"));
        }
        Ok(Self {
            space,
            mode: TextureMode::Normal,
            scale,
            angle,
            depth,
            brightness: 0.,
            contrast: 0.,
            minimum_depth: 0.,
            depth_jitter: 0.,
            depth_pressure: false,
            invert,
            offset_x,
            offset_y,
        })
    }
    pub fn with_mode(mut self, mode: TextureMode) -> Self {
        self.mode = mode;
        self
    }
    pub fn with_adjustments(mut self, brightness: f32, contrast: f32) -> Result<Self> {
        if !brightness.is_finite()
            || !(-1. ..=1.).contains(&brightness)
            || !contrast.is_finite()
            || !(-1. ..=1.).contains(&contrast)
        {
            return Err(Error::InvalidArgument("texture adjustments"));
        }
        self.brightness = brightness;
        self.contrast = contrast;
        Ok(self)
    }
    pub fn with_depth_dynamics(
        mut self,
        minimum_depth: f32,
        depth_jitter: f32,
        depth_pressure: bool,
    ) -> Result<Self> {
        if !minimum_depth.is_finite()
            || !(0. ..=1.).contains(&minimum_depth)
            || !depth_jitter.is_finite()
            || !(0. ..=1.).contains(&depth_jitter)
            || minimum_depth > self.depth
        {
            return Err(Error::InvalidArgument("texture depth dynamics"));
        }
        self.minimum_depth = minimum_depth;
        self.depth_jitter = depth_jitter;
        self.depth_pressure = depth_pressure;
        Ok(self)
    }
    pub fn space(self) -> TextureSpace {
        self.space
    }
    pub fn mode(self) -> TextureMode {
        self.mode
    }
    pub fn scale(self) -> f32 {
        self.scale
    }
    pub fn angle(self) -> f32 {
        self.angle
    }
    pub fn depth(self) -> f32 {
        self.depth
    }
    pub fn brightness(self) -> f32 {
        self.brightness
    }
    pub fn contrast(self) -> f32 {
        self.contrast
    }
    pub fn minimum_depth(self) -> f32 {
        self.minimum_depth
    }
    pub fn depth_jitter(self) -> f32 {
        self.depth_jitter
    }
    pub fn depth_pressure(self) -> bool {
        self.depth_pressure
    }
    pub fn invert(self) -> bool {
        self.invert
    }
    pub fn offset(self) -> (f64, f64) {
        (self.offset_x, self.offset_y)
    }
}
#[derive(Debug)]
struct Level {
    width: u32,
    height: u32,
    coverage: Box<[u8]>,
}
#[derive(Debug)]
pub struct TextureResource {
    source: Arc<BrushMask>,
    levels: Vec<Level>,
    uniform: Option<u8>,
}
impl TextureResource {
    pub fn new(source: Arc<BrushMask>) -> Self {
        let (ow, oh) = source.dimensions();
        let mut levels: Vec<Level> = Vec::new();
        let mut stride = 1u32;
        loop {
            let (w, h, bytes) = levels.last().map_or((ow, oh, source.coverage()), |l| {
                (l.width, l.height, l.coverage.as_ref())
            });
            if w == 1 && h == 1 {
                break;
            }
            let nw = w.div_ceil(2);
            let nh = h.div_ceil(2);
            let mut next = vec![0; (nw * nh) as usize];
            for y in 0..nh {
                for x in 0..nw {
                    let mut sum = 0u64;
                    let mut area = 0u64;
                    for sy in y * 2..(y * 2 + 2).min(h) {
                        for sx in x * 2..(x * 2 + 2).min(w) {
                            let weight = u64::from((ow - sx * stride).min(stride))
                                * u64::from((oh - sy * stride).min(stride));
                            sum += u64::from(bytes[(sy * w + sx) as usize]) * weight;
                            area += weight;
                        }
                    }
                    next[(y * nw + x) as usize] = ((sum + area / 2) / area) as u8;
                }
            }
            levels.push(Level {
                width: nw,
                height: nh,
                coverage: next.into_boxed_slice(),
            });
            stride *= 2;
        }
        let first = source.coverage()[0];
        let uniform = source
            .coverage()
            .iter()
            .all(|p| *p == first)
            .then_some(first);
        Self {
            source,
            levels,
            uniform,
        }
    }
    pub fn source(&self) -> &Arc<BrushMask> {
        &self.source
    }
    pub fn mip_bytes(&self) -> usize {
        self.levels.iter().map(|l| l.coverage.len()).sum()
    }
    fn level(&self, index: usize) -> View<'_> {
        if index == 0 {
            let (width, height) = self.source.dimensions();
            View {
                width,
                height,
                bytes: self.source.coverage(),
            }
        } else {
            let l = &self.levels[index - 1];
            View {
                width: l.width,
                height: l.height,
                bytes: &l.coverage,
            }
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct BrushTexture {
    enabled: bool,
    settings: TextureSettings,
    resource: Option<Arc<TextureResource>>,
}
impl BrushTexture {
    pub fn new(
        enabled: bool,
        settings: TextureSettings,
        resource: Option<Arc<TextureResource>>,
    ) -> Result<Self> {
        if enabled && resource.is_none() {
            return Err(Error::InvalidArgument(
                "enabled texture requires a resource",
            ));
        }
        Ok(Self {
            enabled,
            settings,
            resource,
        })
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn settings(&self) -> TextureSettings {
        self.settings
    }
    pub fn resource(&self) -> Option<&Arc<TextureResource>> {
        self.resource.as_ref()
    }
    pub fn is_default(&self) -> bool {
        !self.enabled && self.settings == TextureSettings::default() && self.resource.is_none()
    }
    pub(crate) fn compile(&self) -> Option<CompiledTexture> {
        if !self.enabled || self.settings.depth == 0. {
            return None;
        }
        if self
            .resource
            .as_ref()
            .is_some_and(|r| r.uniform == Some(if self.settings.invert { 0 } else { 255 }))
        {
            return None;
        }
        let (sine, cosine) = f64::from(self.settings.angle).to_radians().sin_cos();
        Some(CompiledTexture {
            resource: self.resource.as_ref().expect("validated texture").clone(),
            settings: self.settings,
            sine,
            cosine,
        })
    }
}
#[derive(Clone, Debug)]
pub(crate) struct CompiledTexture {
    resource: Arc<TextureResource>,
    settings: TextureSettings,
    sine: f64,
    cosine: f64,
}
#[derive(Clone, Copy)]
struct View<'a> {
    width: u32,
    height: u32,
    bytes: &'a [u8],
}
impl View<'_> {
    fn sample(self, u: f64, v: f64) -> f32 {
        let x = (u * f64::from(self.width) - 0.5).rem_euclid(f64::from(self.width));
        let y = (v * f64::from(self.height) - 0.5).rem_euclid(f64::from(self.height));
        let x = if x >= f64::from(self.width) { 0. } else { x };
        let y = if y >= f64::from(self.height) { 0. } else { y };
        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = (x0 + 1) % self.width;
        let y1 = (y0 + 1) % self.height;
        let at = |x, y| f64::from(self.bytes[(y * self.width + x) as usize]) / 255.;
        let a = at(x0, y0) + (at(x1, y0) - at(x0, y0)) * (x - f64::from(x0));
        let b = at(x0, y1) + (at(x1, y1) - at(x0, y1)) * (x - f64::from(x0));
        (a + (b - a) * (y - f64::from(y0))) as f32
    }
}
pub(crate) struct TextureDab<'a> {
    lower: View<'a>,
    upper: View<'a>,
    mix: f32,
    sine: f64,
    cosine: f64,
    origin_x: f64,
    origin_y: f64,
    period_x: f64,
    period_y: f64,
    depth: f32,
    brightness: f32,
    contrast: f32,
    minimum_depth: f32,
    depth_jitter: f32,
    depth_pressure: bool,
    pressure: f32,
    stamp_x: f64,
    stamp_y: f64,
    invert: bool,
    mode: TextureMode,
}
impl CompiledTexture {
    pub(crate) fn document_space(&self) -> bool {
        self.settings.space == TextureSpace::Document
    }
    pub(crate) fn dab(
        &self,
        x: f64,
        y: f64,
        pressure: f32,
        tip_sine: f64,
        tip_cosine: f64,
    ) -> TextureDab<'_> {
        let s = self.settings;
        let (sine, cosine, ox, oy) = if s.space == TextureSpace::Document {
            (self.sine, self.cosine, 0., 0.)
        } else {
            (
                self.sine * tip_cosine + self.cosine * tip_sine,
                self.cosine * tip_cosine - self.sine * tip_sine,
                x,
                y,
            )
        };
        let lod = (1. / f64::from(s.scale))
            .log2()
            .clamp(0., self.resource.levels.len() as f64);
        let lower = lod.floor() as usize;
        let (w, h) = self.resource.source.dimensions();
        TextureDab {
            lower: self.resource.level(lower),
            upper: self
                .resource
                .level((lower + 1).min(self.resource.levels.len())),
            mix: (lod - lower as f64) as f32,
            sine,
            cosine,
            origin_x: ox + s.offset_x,
            origin_y: oy + s.offset_y,
            period_x: f64::from(w) * f64::from(s.scale),
            period_y: f64::from(h) * f64::from(s.scale),
            depth: s.depth,
            brightness: s.brightness,
            contrast: s.contrast,
            minimum_depth: s.minimum_depth,
            depth_jitter: s.depth_jitter,
            depth_pressure: s.depth_pressure,
            pressure,
            stamp_x: x,
            stamp_y: y,
            invert: s.invert,
            mode: s.mode,
        }
    }
}
impl TextureDab<'_> {
    fn sample(&self, x: f64, y: f64) -> f32 {
        let x = x - self.origin_x;
        let y = y - self.origin_y;
        let u = (x * self.cosine + y * self.sine) / self.period_x;
        let v = (-x * self.sine + y * self.cosine) / self.period_y;
        let a = self.lower.sample(u, v);
        let mut value = if self.mix == 0. {
            a
        } else {
            a + (self.upper.sample(u, v) - a) * self.mix
        };
        if self.brightness != 0. {
            value = (value + self.brightness).clamp(0., 1.);
        }
        if self.contrast != 0. {
            value = ((value - 0.5) * (1. + self.contrast) + 0.5).clamp(0., 1.);
        }
        if self.invert {
            value = 1. - value;
        }
        let mut depth = self.depth;
        if self.depth_pressure {
            depth = self.minimum_depth + (depth - self.minimum_depth) * self.pressure;
        }
        if self.depth_jitter > 0. {
            let mut hash = (self.stamp_x.to_bits() ^ self.stamp_y.to_bits().rotate_left(17))
                .wrapping_mul(0x9e3779b97f4a7c15);
            hash ^= hash >> 29;
            let random = (hash as f64 / (u64::MAX as f64 + 1.)) as f32;
            depth *= 1. - self.depth_jitter * random;
            depth = depth.max(self.minimum_depth);
        }
        1. - depth * (1. - value)
    }
    pub(crate) fn apply(&self, base: f32, x: f64, y: f64) -> f32 {
        self.mode.combine(base, self.sample(x, y))
    }
    pub(crate) fn coverage(&self, x: f64, y: f64) -> f32 {
        self.sample(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn texture_modes_match_ps_style_coverage_combinations() {
        let base = 0.25;
        let texture = 0.75;
        assert_eq!(TextureMode::Normal.combine(base, texture), base * texture);
        assert_eq!(TextureMode::Multiply.combine(base, texture), base * texture);
        assert_eq!(TextureMode::Darken.combine(base, texture), base);
        assert_eq!(TextureMode::Lighten.combine(base, texture), texture);
        assert_eq!(TextureMode::Subtract.combine(base, texture), 0.);
        assert_eq!(TextureMode::Difference.combine(base, texture), 0.5);
        assert_eq!(TextureMode::Screen.combine(base, texture), 0.8125);
        assert_eq!(TextureMode::ColorBurn.combine(base, texture), 0.);
        assert_eq!(TextureMode::ColorBurn.combine(0.75, 0.25), 0.);
        assert_eq!(TextureMode::ColorBurn.combine(0.25, 1.), 0.25);
        assert_eq!(TextureMode::LinearBurn.combine(0.75, 0.75), 0.5);
        assert_eq!(TextureMode::ColorDodge.combine(0.25, 0.5), 0.5);
        assert_eq!(TextureMode::LinearDodge.combine(0.25, 0.5), 0.75);
        assert_eq!(TextureMode::Overlay.combine(0.25, 0.5), 0.25);
        assert_eq!(TextureMode::SoftLight.combine(0.5, 0.5), 0.5);
        assert_eq!(TextureMode::HardLight.combine(0.25, 0.75), 0.625);
        assert_eq!(TextureMode::Exclusion.combine(0.5, 0.5), 0.5);
        assert_eq!(TextureMode::HardMix.combine(0.5, 0.5), 1.);
        let adjusted = TextureSettings::new(TextureSpace::Document, 1., 0., 1., false, 0., 0.)
            .unwrap()
            .with_adjustments(-0.25, 0.5)
            .unwrap();
        assert_eq!(adjusted.brightness(), -0.25);
        assert_eq!(adjusted.contrast(), 0.5);
    }
    #[test]
    fn periodic_seams_negative_coordinates_and_odd_mips() {
        let resource = Arc::new(TextureResource::new(Arc::new(
            BrushMask::new(2, 1, vec![0, 255]).unwrap(),
        )));
        let texture = BrushTexture::new(true, TextureSettings::default(), Some(resource))
            .unwrap()
            .compile()
            .unwrap();
        let dab = texture.dab(0., 0., 1., 0., 1.);
        assert_eq!(dab.coverage(0.5, 0.5), 0.);
        assert_eq!(dab.coverage(1.5, 0.5), 1.);
        assert_eq!(dab.coverage(2.5, 0.5), 0.);
        assert_eq!(dab.coverage(-0.5, 0.5), 1.);
        assert!((dab.coverage(0., 0.) - 0.5).abs() < 1e-6);
        let resource = Arc::new(TextureResource::new(Arc::new(
            BrushMask::new(3, 1, vec![0, 0, 255]).unwrap(),
        )));
        let settings =
            TextureSettings::new(TextureSpace::Document, 0.01, 0., 1., false, 0., 0.).unwrap();
        let texture = BrushTexture::new(true, settings, Some(resource))
            .unwrap()
            .compile()
            .unwrap();
        assert!((texture.dab(0., 0., 1., 0., 1.).coverage(0.5, 0.5) - 1. / 3.).abs() < 1. / 255.);
    }
    #[test]
    fn brightness_and_contrast_adjust_sampled_coverage_before_invert_and_depth() {
        let resource = Arc::new(TextureResource::new(Arc::new(
            BrushMask::new(1, 1, vec![128]).unwrap(),
        )));
        for (brightness, contrast, expected) in [
            (0., 0., 128. / 255.),
            (0.25, 0., 128. / 255. + 0.25),
            (-0.25, 0., 128. / 255. - 0.25),
            (0., 0.5, (128. / 255. - 0.5) * 1.5 + 0.5),
        ] {
            let settings = TextureSettings::new(TextureSpace::Document, 1., 0., 1., false, 0., 0.)
                .unwrap()
                .with_adjustments(brightness, contrast)
                .unwrap();
            let texture = BrushTexture::new(true, settings, Some(resource.clone()))
                .unwrap()
                .compile()
                .unwrap();
            assert!((texture.dab(0., 0., 1., 0., 1.).coverage(0.5, 0.5) - expected).abs() < 1e-6);
        }
    }
    #[test]
    fn texture_depth_pressure_respects_minimum_depth() {
        let resource = Arc::new(TextureResource::new(Arc::new(
            BrushMask::new(1, 1, vec![0]).unwrap(),
        )));
        let settings = TextureSettings::new(TextureSpace::Document, 1., 0., 1., false, 0., 0.)
            .unwrap()
            .with_depth_dynamics(0.2, 0., true)
            .unwrap();
        let texture = BrushTexture::new(true, settings, Some(resource))
            .unwrap()
            .compile()
            .unwrap();
        assert!((texture.dab(0., 0., 0., 0., 1.).coverage(0.5, 0.5) - 0.8).abs() < 1e-6);
        assert!((texture.dab(0., 0., 0.5, 0., 1.).coverage(0.5, 0.5) - 0.4).abs() < 1e-6);
        assert!((texture.dab(0., 0., 1., 0., 1.).coverage(0.5, 0.5) - 0.).abs() < 1e-6);
    }
    #[test]
    fn roundoff_and_invalid_settings_do_not_escape_resource() {
        let source = Arc::new(BrushMask::new(2, 1, vec![0, 255]).unwrap());
        let resource = Arc::new(TextureResource::new(source));
        let settings = TextureSettings::new(
            TextureSpace::Document,
            1.,
            0.,
            1.,
            false,
            f64::EPSILON / 2.,
            0.,
        )
        .unwrap();
        let texture = BrushTexture::new(true, settings, Some(resource))
            .unwrap()
            .compile()
            .unwrap();
        assert!(texture.dab(0., 0., 1., 0., 1.).coverage(0.5, 0.5) < 1e-6);
        assert!(BrushTexture::new(true, TextureSettings::default(), None).is_err());
        for bad in [f32::NAN, f32::INFINITY, -1.] {
            assert!(
                TextureSettings::new(TextureSpace::Document, bad, 0., 1., false, 0., 0.).is_err()
            );
            assert!(TextureSettings::new(
                TextureSpace::Tip,
                1.,
                if bad.is_finite() { 181. } else { bad },
                1.,
                false,
                0.,
                0.
            )
            .is_err());
            assert!(TextureSettings::new(TextureSpace::Tip, 1., 0., bad, false, 0., 0.).is_err());
        }
        assert!(TextureSettings::new(TextureSpace::Tip, 1., 0., 1., false, f64::NAN, 0.).is_err());
        assert!(TextureSettings::new(TextureSpace::Tip, 1., 181., 1., false, 0., 0.).is_err());
        assert!(
            TextureSettings::new(TextureSpace::Tip, 1., 0., 1., false, 1_000_001., 0.).is_err()
        );
    }
}
