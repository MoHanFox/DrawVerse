use crate::{Error, Result};

/// Linear premultiplied components. Construction preserves 0 <= RGB <= alpha <= 1.
/// HDR/unbounded scene color will need a separate validated color policy.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pixel([f32; 4]);

impl Pixel {
    pub const TRANSPARENT: Self = Self([0.0; 4]);
    pub const WHITE: Self = Self([1.0; 4]);

    pub fn from_straight(rgba: [f32; 4]) -> Result<Self> {
        if rgba
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(Error::InvalidArgument(
                "color must be finite normalized linear RGBA",
            ));
        }
        let a = rgba[3];
        Ok(Self([rgba[0] * a, rgba[1] * a, rgba[2] * a, a]))
    }

    pub fn components(self) -> [f32; 4] {
        self.0
    }

    pub(crate) fn from_premultiplied(rgba: [f32; 4]) -> Result<Self> {
        if rgba
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            || rgba[..3].iter().any(|v| *v > rgba[3])
        {
            return Err(Error::HistoryStorage("invalid premultiplied pixel".into()));
        }
        Ok(Self(rgba))
    }

    pub fn alpha(self) -> f32 {
        self.0[3]
    }

    pub fn scaled(self, amount: f32) -> Self {
        let amount = if amount.is_nan() {
            0.
        } else {
            amount.clamp(0.0, 1.0)
        };
        Self(self.0.map(|v| v * amount))
    }

    pub(crate) fn paint_preserving_alpha(self, color: Self, amount: f32) -> Self {
        let remaining = 1. - color.alpha() * amount;
        Self([
            (self.0[0] * remaining + color.0[0] * amount * self.alpha()).min(self.alpha()),
            (self.0[1] * remaining + color.0[1] * amount * self.alpha()).min(self.alpha()),
            (self.0[2] * remaining + color.0[2] * amount * self.alpha()).min(self.alpha()),
            self.alpha(),
        ])
    }
    /// Porter-Duff source-over in linear space. No gamma-encoded compositing.
    pub fn over(self, destination: Self) -> Self {
        let remaining = 1.0 - self.alpha();
        Self(std::array::from_fn(|i| {
            (self.0[i] + destination.0[i] * remaining).min(1.0)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premultiplication_and_source_over() {
        let red = Pixel::from_straight([1.0, 0.0, 0.0, 0.5]).unwrap();
        let blue = Pixel::from_straight([0.0, 0.0, 1.0, 1.0]).unwrap();
        assert_eq!(red.components(), [0.5, 0.0, 0.0, 0.5]);
        assert_eq!(red.over(blue).components(), [0.5, 0.0, 0.5, 1.0]);
    }

    #[test]
    fn invalid_colors_are_rejected() {
        for value in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            assert!(Pixel::from_straight([value, 0.0, 0.0, 1.0]).is_err());
        }
    }
    #[test]
    fn locked_alpha_blending_keeps_premultiplied_bounds_and_exact_alpha_bits() {
        for i in 1..1000 {
            let alpha = i as f32 / 1000.;
            let pixel = Pixel::from_straight([1., 0.3, 0., alpha]).unwrap();
            for flow in [0., 0.1, 0.43, 0.99, 1.] {
                for color in [[1.; 4], [0.7, 1., 0.2, 0.37]] {
                    let changed =
                        pixel.paint_preserving_alpha(Pixel::from_straight(color).unwrap(), flow);
                    assert_eq!(changed.alpha().to_bits(), alpha.to_bits());
                    assert!(Pixel::from_premultiplied(changed.components()).is_ok());
                }
            }
        }
        assert_eq!(Pixel::TRANSPARENT.scaled(f32::NAN), Pixel::TRANSPARENT);
    }
}
