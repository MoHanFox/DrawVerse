use crate::{
    brush_random::{mix, unit},
    Error, Pixel, Result,
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorDynamics {
    enabled: bool,
    foreground_background: f32,
    hue: f32,
    saturation: f32,
    value: f32,
    per_tip: bool,
    seed: u64,
}
impl Default for ColorDynamics {
    fn default() -> Self {
        Self {
            enabled: false,
            foreground_background: 0.,
            hue: 0.,
            saturation: 0.,
            value: 0.,
            per_tip: true,
            seed: 0,
        }
    }
}
impl ColorDynamics {
    pub fn new(
        enabled: bool,
        foreground_background: f32,
        hue: f32,
        saturation: f32,
        value: f32,
        per_tip: bool,
        seed: u64,
    ) -> Result<Self> {
        if [foreground_background, hue, saturation, value]
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
        {
            return Err(Error::InvalidArgument("color dynamics bounds"));
        }
        Ok(Self {
            enabled,
            foreground_background,
            hue,
            saturation,
            value,
            per_tip,
            seed,
        })
    }
    pub fn enabled(self) -> bool {
        self.enabled
    }
    pub fn foreground_background(self) -> f32 {
        self.foreground_background
    }
    pub fn hue(self) -> f32 {
        self.hue
    }
    pub fn saturation(self) -> f32 {
        self.saturation
    }
    pub fn value(self) -> f32 {
        self.value
    }
    pub fn per_tip(self) -> bool {
        self.per_tip
    }
    pub fn seed(self) -> u64 {
        self.seed
    }
    pub(crate) fn active(self) -> bool {
        self.enabled
            && (self.foreground_background != 0.
                || self.hue != 0.
                || self.saturation != 0.
                || self.value != 0.)
    }
    pub(crate) fn sample(
        self,
        foreground: Pixel,
        background: Pixel,
        stroke: u64,
        index: u64,
        child: u32,
    ) -> Pixel {
        if !self.active() || foreground.alpha() == 0. {
            return foreground;
        }
        let (index, child) = if self.per_tip { (index, child) } else { (0, 0) };
        let key = mix(self.seed
            ^ mix(stroke)
            ^ mix(index.wrapping_add(0x9e3779b97f4a7c15))
            ^ 0x636f6c6f722d7631);
        let straight = |p: Pixel| {
            let c = p.components();
            [c[0] / c[3], c[1] / c[3], c[2] / c[3]]
        };
        let f = straight(foreground);
        let b = if background.alpha() > 0. {
            straight(background)
        } else {
            f
        };
        let blend = unit(key, child, 0) as f32 * self.foreground_background;
        let rgb = std::array::from_fn::<_, 3, _>(|i| f[i] + (b[i] - f[i]) * blend);
        let max = rgb.iter().copied().fold(0., f32::max);
        let min = rgb.iter().copied().fold(1., f32::min);
        let delta = max - min;
        let mut h = if delta == 0. {
            0.
        } else if max == rgb[0] {
            ((rgb[1] - rgb[2]) / delta).rem_euclid(6.) / 6.
        } else if max == rgb[1] {
            ((rgb[2] - rgb[0]) / delta + 2.) / 6.
        } else {
            ((rgb[0] - rgb[1]) / delta + 4.) / 6.
        };
        let mut s = if max == 0. { 0. } else { delta / max };
        let mut v = max;
        h = (h + (unit(key, child, 1) as f32 - 0.5) * self.hue).rem_euclid(1.);
        s = (s + (unit(key, child, 2) as f32 * 2. - 1.) * self.saturation).clamp(0., 1.);
        v = (v + (unit(key, child, 3) as f32 * 2. - 1.) * self.value).clamp(0., 1.);
        let c = v * s;
        let x = c * (1. - ((h * 6.).rem_euclid(2.) - 1.).abs());
        let m = v - c;
        let channels = match (h * 6.).floor() as u32 {
            0 => [c, x, 0.],
            1 => [x, c, 0.],
            2 => [0., c, x],
            3 => [0., x, c],
            4 => [x, 0., c],
            _ => [c, 0., x],
        };
        Pixel::from_straight([
            (channels[0] + m).clamp(0., 1.),
            (channels[1] + m).clamp(0., 1.),
            (channels[2] + m).clamp(0., 1.),
            foreground.alpha(),
        ])
        .expect("bounded color dynamics")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_settings_and_color_frequency_are_bounded() {
        for bad in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            for values in [
                [bad, 0., 0., 0.],
                [0., bad, 0., 0.],
                [0., 0., bad, 0.],
                [0., 0., 0., bad],
            ] {
                assert!(ColorDynamics::new(
                    true, values[0], values[1], values[2], values[3], true, 0
                )
                .is_err());
            }
        }
        let red = Pixel::from_straight([1., 0., 0., 0.5]).unwrap();
        let blue = Pixel::from_straight([0., 0., 1., 0.8]).unwrap();
        let d = ColorDynamics::new(true, 1., 1., 1., 1., false, u64::MAX).unwrap();
        let first = d.sample(red, blue, 1, 0, 0);
        for i in 0..1000 {
            assert_eq!(d.sample(red, blue, 1, i, 15), first);
            let p = ColorDynamics::new(true, 1., 1., 1., 1., true, u64::MAX)
                .unwrap()
                .sample(red, blue, 1, i, 3);
            assert_eq!(p.alpha(), 0.5);
            assert!(p
                .components()
                .iter()
                .all(|v| v.is_finite() && (0. ..=0.5).contains(v)));
        }
        assert_ne!(d.sample(red, blue, 2, 0, 0), first);
        let mix = ColorDynamics::new(true, 1., 0., 0., 0., true, 12).unwrap();
        assert_eq!(mix.sample(red, Pixel::TRANSPARENT, 1, 2, 3), red);
        assert_eq!(
            mix.sample(Pixel::TRANSPARENT, blue, 1, 2, 3),
            Pixel::TRANSPARENT
        );
        let p = mix.sample(red, blue, 1, 0, 0).components();
        assert!(p[0] > 0. && p[2] > 0. && p[1] == 0.);
    }
}
