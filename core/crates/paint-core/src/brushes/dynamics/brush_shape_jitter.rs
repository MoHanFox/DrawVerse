use crate::{
    brush_random::{mix, unit},
    Error, Result,
};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeJitter {
    enabled: bool,
    size: f32,
    minimum_size: f32,
    angle: f32,
    roundness: f32,
    minimum_roundness: f32,
    seed: u64,
}
impl Default for ShapeJitter {
    fn default() -> Self {
        Self {
            enabled: false,
            size: 0.,
            minimum_size: 0.,
            angle: 0.,
            roundness: 0.,
            minimum_roundness: 0.01,
            seed: 0,
        }
    }
}
impl ShapeJitter {
    pub fn new(
        enabled: bool,
        size: f32,
        minimum_size: f32,
        angle: f32,
        roundness: f32,
        minimum_roundness: f32,
        seed: u64,
    ) -> Result<Self> {
        if [size, minimum_size, angle, roundness]
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
            || !minimum_roundness.is_finite()
            || !(0.01..=1.).contains(&minimum_roundness)
        {
            return Err(Error::InvalidArgument("shape jitter bounds"));
        }
        Ok(Self {
            enabled,
            size,
            minimum_size,
            angle,
            roundness,
            minimum_roundness,
            seed,
        })
    }
    pub fn enabled(self) -> bool {
        self.enabled
    }
    pub fn size(self) -> f32 {
        self.size
    }
    pub fn minimum_size(self) -> f32 {
        self.minimum_size
    }
    pub fn angle(self) -> f32 {
        self.angle
    }
    pub fn roundness(self) -> f32 {
        self.roundness
    }
    pub fn minimum_roundness(self) -> f32 {
        self.minimum_roundness
    }
    pub fn seed(self) -> u64 {
        self.seed
    }
    pub(crate) fn active(self) -> bool {
        self.enabled && (self.size != 0. || self.angle != 0. || self.roundness != 0.)
    }
    pub(crate) fn sample(self, stroke: u64, index: u64, child: u32) -> ShapeSample {
        if !self.active() {
            return ShapeSample::default();
        }
        let key = mix(self.seed
            ^ mix(stroke)
            ^ mix(index.wrapping_add(0x9e3779b97f4a7c15))
            ^ 0x73686170652d7631);
        let factor = |jitter: f32, minimum: f32, lane| {
            let low = (1. - jitter).max(minimum);
            low + (1. - low) * unit(key, child, lane) as f32
        };
        ShapeSample {
            size: factor(self.size, self.minimum_size, 0),
            angle: (unit(key, child, 1) * 2. - 1.) * f64::from(self.angle) * std::f64::consts::PI,
            roundness: factor(self.roundness, self.minimum_roundness, 2),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShapeSample {
    pub size: f32,
    pub angle: f64,
    pub roundness: f32,
}
impl Default for ShapeSample {
    fn default() -> Self {
        Self {
            size: 1.,
            angle: 0.,
            roundness: 1.,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shape_samples_obey_minima_and_independent_child_channels() {
        let a = ShapeJitter::new(true, 0.9, 0.4, 0.6, 0.95, 0.3, u64::MAX).unwrap();
        let b = ShapeJitter::new(true, 0.9, 0.4, 0., 0.95, 0.3, u64::MAX).unwrap();
        let first = a.sample(1, 0, 0);
        assert_ne!(first.size, a.sample(1, 0, 1).size);
        let mut changed_child = false;
        for index in 0..1000 {
            for child in 0..16 {
                let p = a.sample(1, index, child);
                let q = b.sample(1, index, child);
                assert!((0.4..=1.).contains(&p.size));
                assert!((0.3..=1.).contains(&p.roundness));
                assert!(p.angle.abs() <= std::f64::consts::PI * 0.600001);
                assert_eq!(p.size, q.size);
                assert_eq!(p.roundness, q.roundness);
                assert_eq!(q.angle, 0.);
                if child > 0 && p.size != first.size {
                    changed_child = true;
                }
            }
        }
        assert!(changed_child);
        assert_ne!(a.sample(1, 0, 0).size, a.sample(2, 0, 0).size);
        let zero = ShapeJitter::new(true, 0., 1., 0., 0., 1., 123).unwrap();
        assert!(!zero.active());
        let p = zero.sample(1, 2, 3);
        assert_eq!((p.size, p.angle, p.roundness), (1., 0., 1.));
    }
    #[test]
    fn non_finite_and_out_of_range_settings_reject() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.1, 1.1] {
            assert!(ShapeJitter::new(true, bad, 0., 0., 0., 0.01, 0).is_err());
            assert!(ShapeJitter::new(true, 0., bad, 0., 0., 0.01, 0).is_err());
            assert!(ShapeJitter::new(true, 0., 0., bad, 0., 0.01, 0).is_err());
            assert!(ShapeJitter::new(true, 0., 0., 0., bad, 0.01, 0).is_err());
            assert!(ShapeJitter::new(true, 0., 0., 0., 0., bad, 0).is_err());
        }
        assert!(ShapeJitter::new(true, 0., 0., 0., 0., 0., 0).is_err());
    }
}
