use crate::{Error, Pixel, Result};
pub const LOCK_TRANSPARENCY: u32 = 1;
pub const LOCK_POSITION: u32 = 2;
pub const LOCK_ALL: u32 = 4;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u32)]
pub enum BlendMode {
    #[default]
    Normal = 0,
    Dissolve,
    Darken,
    Multiply,
    ColorBurn,
    LinearBurn,
    DarkerColor,
    Lighten,
    Screen,
    ColorDodge,
    LinearDodge,
    LighterColor,
    Overlay,
    SoftLight,
    HardLight,
    VividLight,
    LinearLight,
    PinLight,
    HardMix,
    Difference,
    Exclusion,
    Subtract,
    Divide,
    Hue,
    Saturation,
    Color,
    Luminosity,
}
impl BlendMode {
    pub fn from_id(id: u32) -> Result<Self> {
        use BlendMode::*;
        const MODES: [BlendMode; 27] = [
            Normal,
            Dissolve,
            Darken,
            Multiply,
            ColorBurn,
            LinearBurn,
            DarkerColor,
            Lighten,
            Screen,
            ColorDodge,
            LinearDodge,
            LighterColor,
            Overlay,
            SoftLight,
            HardLight,
            VividLight,
            LinearLight,
            PinLight,
            HardMix,
            Difference,
            Exclusion,
            Subtract,
            Divide,
            Hue,
            Saturation,
            Color,
            Luminosity,
        ];
        MODES
            .get(id as usize)
            .copied()
            .ok_or(Error::InvalidArgument("unknown blend mode"))
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerAppearance {
    pub fill: f32,
    pub dissolve_seed: u32,
    pub blend: BlendMode,
    pub locks: u32,
    pub offset_x: i32,
    pub offset_y: i32,
}
impl Default for LayerAppearance {
    fn default() -> Self {
        Self {
            fill: 1.,
            dissolve_seed: 0,
            blend: BlendMode::Normal,
            locks: 0,
            offset_x: 0,
            offset_y: 0,
        }
    }
}
impl LayerAppearance {
    pub fn validate(self) -> Result<()> {
        if !self.fill.is_finite()
            || !(0. ..=1.).contains(&self.fill)
            || self.locks & !7 != 0
            || self.offset_x.unsigned_abs() > crate::MAX_DIMENSION
            || self.offset_y.unsigned_abs() > crate::MAX_DIMENSION
        {
            return Err(Error::InvalidArgument(
                "invalid layer fill, locks or position",
            ));
        }
        Ok(())
    }
    pub fn translated(self, dx: i32, dy: i32) -> Result<Self> {
        if self.locks & (LOCK_POSITION | LOCK_ALL) != 0 {
            return Err(Error::LayerLocked);
        }
        let after = Self {
            offset_x: self
                .offset_x
                .checked_add(dx)
                .ok_or(Error::InvalidArgument("layer position overflow"))?,
            offset_y: self
                .offset_y
                .checked_add(dy)
                .ok_or(Error::InvalidArgument("layer position overflow"))?,
            ..self
        };
        after.validate()?;
        Ok(after)
    }
}
fn lum(c: [f32; 3]) -> f32 {
    c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11
}
fn sat(c: [f32; 3]) -> f32 {
    c.into_iter().fold(0., f32::max) - c.into_iter().fold(1., f32::min)
}
fn set_lum(mut c: [f32; 3], l: f32) -> [f32; 3] {
    let delta = l - lum(c);
    c = c.map(|v| v + delta);
    let light = lum(c);
    let min = c.into_iter().fold(f32::INFINITY, f32::min);
    let max = c.into_iter().fold(f32::NEG_INFINITY, f32::max);
    if min < 0. {
        c = c.map(|v| light + (v - light) * light / (light - min));
    }
    if max > 1. {
        c = c.map(|v| light + (v - light) * (1. - light) / (max - light));
    }
    c.map(|v| v.clamp(0., 1.))
}
fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    let min = c.into_iter().fold(1., f32::min);
    let max = c.into_iter().fold(0., f32::max);
    if max <= min {
        [0.; 3]
    } else {
        c.map(|v| (v - min) * s / (max - min))
    }
}
fn burn(b: f32, s: f32) -> f32 {
    if b >= 1. {
        1.
    } else if s <= 0. {
        0.
    } else {
        1. - ((1. - b) / s).min(1.)
    }
}
fn dodge(b: f32, s: f32) -> f32 {
    if b <= 0. {
        0.
    } else if s >= 1. {
        1.
    } else {
        (b / (1. - s)).min(1.)
    }
}
fn hard(b: f32, s: f32) -> f32 {
    if s <= 0.5 {
        2. * b * s
    } else {
        1. - 2. * (1. - b) * (1. - s)
    }
}
fn vivid(b: f32, s: f32) -> f32 {
    if s <= 0.5 {
        burn(b, 2. * s)
    } else {
        dodge(b, 2. * s - 1.)
    }
}
/// W3C-style blend functions in the document's linear working space, not gamma encoded RGB.
pub fn blend_pixel(
    source: Pixel,
    destination: Pixel,
    mode: BlendMode,
    x: u32,
    y: u32,
    seed: u64,
) -> Pixel {
    use BlendMode::*;
    if mode == Normal {
        return source.over(destination);
    }
    let mut a = source.alpha();
    if a == 0. {
        return destination;
    }
    let d = destination.components();
    let s = source.components();
    let da = d[3];
    let sc = std::array::from_fn::<_, 3, _>(|i| s[i] / a);
    let dc = std::array::from_fn::<_, 3, _>(|i| if da > 0. { d[i] / da } else { 0. });
    if mode == Dissolve {
        let mut h = ((u64::from(x) * 0x9e3779b1) ^ (u64::from(y) * 0x85ebca77) ^ seed)
            .wrapping_mul(0xbf58476d1ce4e5b9);
        h ^= h >> 31;
        if (h as u32 as f64 / (u32::MAX as f64 + 1.)) >= f64::from(a) {
            return destination;
        }
        a = 1.;
        return Pixel::from_straight([sc[0], sc[1], sc[2], a])
            .expect("normalized")
            .over(destination);
    }
    let mixed = match mode {
        Hue => set_lum(set_sat(sc, sat(dc)), lum(dc)),
        Saturation => set_lum(set_sat(dc, sat(sc)), lum(dc)),
        Color => set_lum(sc, lum(dc)),
        Luminosity => set_lum(dc, lum(sc)),
        DarkerColor => {
            if lum(sc) < lum(dc) {
                sc
            } else {
                dc
            }
        }
        LighterColor => {
            if lum(sc) > lum(dc) {
                sc
            } else {
                dc
            }
        }
        _ => std::array::from_fn(|i| {
            let (b, s) = (dc[i], sc[i]);
            match mode {
                Darken => b.min(s),
                Multiply => b * s,
                ColorBurn => burn(b, s),
                LinearBurn => (b + s - 1.).max(0.),
                Lighten => b.max(s),
                Screen => b + s - b * s,
                ColorDodge => dodge(b, s),
                LinearDodge => (b + s).min(1.),
                Overlay => hard(s, b),
                HardLight => hard(b, s),
                SoftLight => {
                    if s <= 0.5 {
                        b - (1. - 2. * s) * b * (1. - b)
                    } else {
                        let f = if b <= 0.25 {
                            ((16. * b - 12.) * b + 4.) * b
                        } else {
                            b.sqrt()
                        };
                        b + (2. * s - 1.) * (f - b)
                    }
                }
                VividLight => vivid(b, s),
                LinearLight => (b + 2. * s - 1.).clamp(0., 1.),
                PinLight => {
                    if s <= 0.5 {
                        b.min(2. * s)
                    } else {
                        b.max(2. * s - 1.)
                    }
                }
                HardMix => {
                    if vivid(b, s) < 0.5 {
                        0.
                    } else {
                        1.
                    }
                }
                Difference => (b - s).abs(),
                Exclusion => b + s - 2. * b * s,
                Subtract => (b - s).max(0.),
                Divide => {
                    if b == 0. {
                        0.
                    } else if s == 0. {
                        1.
                    } else {
                        (b / s).min(1.)
                    }
                }
                _ => s,
            }
        }),
    };
    let alpha = a + da * (1. - a);
    let rgb = std::array::from_fn::<_, 3, _>(|i| {
        ((1. - a) * d[i] + (1. - da) * a * sc[i] + da * a * mixed[i]).clamp(0., alpha)
    });
    Pixel::from_premultiplied([rgb[0], rgb[1], rgb[2], alpha]).expect("normalized composition")
}
