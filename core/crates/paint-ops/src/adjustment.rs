//! Display RGB adjustment of linear premultiplied pixels. Shared CPU/GPU wire contract.
use crate::Rgba;

#[derive(Clone, Debug)]
pub struct AdjustmentOp {
    pub pixels: Vec<Rgba>,
    /// 0: RGB lookup table, 1: HSL (hue turns, saturation/lightness -1..1).
    pub kind: u32,
    pub parameters: Rgba,
    pub lut: Vec<Rgba>,
}
pub fn encode(v: f32) -> f32 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }
}
pub fn decode(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

impl AdjustmentOp {
    pub fn validate(&self) -> Result<(), String> {
        if (self.parameters[3] != 0. && (self.kind != 1 || self.parameters[3] != 1.))
            || self.lut.iter().any(|p| p[3] != 1.)
            || self.kind > 1
            || self
                .parameters
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 1.)
            || self.lut.len() != 4097
            || self
                .lut
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
            || self.pixels.iter().any(|p| {
                p.iter().any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
                    || p[..3].iter().any(|v| *v > p[3])
            })
        {
            return Err("invalid adjustment".into());
        }
        Ok(())
    }
    pub fn pixel(&self, p: Rgba) -> Rgba {
        if p[3] == 0. {
            return p;
        }
        let mut c = [0, 1, 2].map(|i| encode(p[i] / p[3]).clamp(0., 1.));
        if self.kind == 0 {
            c = std::array::from_fn(|i| {
                let x = c[i] * 4096.;
                let a = (x.floor() as usize).min(4095);
                let t = x - a as f32;
                self.lut[a][i] * (1. - t) + self.lut[a + 1][i] * t
            });
        } else {
            let original_lightness =
                (c.into_iter().fold(0., f32::max) + c.into_iter().fold(1., f32::min)) * 0.5;
            c = c.map(|v| {
                if self.parameters[2] < 0. {
                    v * (1. + self.parameters[2])
                } else {
                    v + (1. - v) * self.parameters[2]
                }
            });
            let max = c.into_iter().fold(0., f32::max);
            let min = c.into_iter().fold(1., f32::min);
            let d = max - min;
            let mut l = (max + min) * 0.5;
            let mut h = if d == 0. {
                0.
            } else if max == c[0] {
                ((c[1] - c[2]) / d).rem_euclid(6.) / 6.
            } else if max == c[1] {
                ((c[2] - c[0]) / d + 2.) / 6.
            } else {
                ((c[0] - c[1]) / d + 4.) / 6.
            };
            let s = if d == 0. {
                0.
            } else {
                d / (1. - (2. * l - 1.).abs()).max(1e-10)
            };
            h = (h + self.parameters[0]).rem_euclid(1.);
            let mut s = if self.parameters[1] > 0. {
                s / (1. - self.parameters[1]).max(1. / 512.)
            } else {
                s * (1. + self.parameters[1])
            }
            .clamp(0., 1.);
            if self.parameters[3] == 1. {
                h = self.parameters[0].rem_euclid(1.);
                s = self.parameters[1].abs();
                l = if self.parameters[2] < 0. {
                    original_lightness * (1. + self.parameters[2])
                } else {
                    original_lightness + (1. - original_lightness) * self.parameters[2]
                };
            }
            let chroma = (1. - (2. * l - 1.).abs()) * s;
            let h = h * 6.;
            let x = chroma * (1. - (h.rem_euclid(2.) - 1.).abs());
            c = match h as u32 {
                0 => [chroma, x, 0.],
                1 => [x, chroma, 0.],
                2 => [0., chroma, x],
                3 => [0., x, chroma],
                4 => [x, 0., chroma],
                _ => [chroma, 0., x],
            }
            .map(|v| v + l - chroma * 0.5);
        }
        [
            decode(c[0].clamp(0., 1.)) * p[3],
            decode(c[1].clamp(0., 1.)) * p[3],
            decode(c[2].clamp(0., 1.)) * p[3],
            p[3],
        ]
    }
    pub fn pack(&self) -> Vec<Rgba> {
        let mut out = vec![
            [
                f32::from_bits(3),
                f32::from_bits(self.pixels.len() as u32),
                f32::from_bits(self.kind),
                0.,
            ],
            self.parameters,
        ];
        out.extend_from_slice(&self.lut);
        out.extend_from_slice(&self.pixels);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_hue_gray_and_alpha_results() {
        let mut op = AdjustmentOp {
            pixels: vec![],
            kind: 1,
            parameters: [1. / 3., 0., 0., 0.],
            lut: vec![[0.; 4]; 4097],
        };
        let green = op.pixel([0.5, 0., 0., 0.5]);
        assert!(
            green[0] < 1e-6 && (green[1] - 0.5).abs() < 1e-6 && green[2] < 1e-6 && green[3] == 0.5
        );
        op.parameters = [0., -1., 0., 0.];
        let gray = op.pixel([1., 0., 0., 1.]);
        assert!((gray[0] - decode(0.5)).abs() < 1e-6);
        assert_eq!(gray[0], gray[1]);
        assert_eq!(gray[1], gray[2]);
        op.parameters = [1. / 3., 1., 0., 1.];
        let tinted = op.pixel([0.5, 0., 0., 0.5]);
        assert!(tinted[0] < 1e-6 && (tinted[1] - 0.5).abs() < 1e-6 && tinted[2] < 1e-6);
        op.parameters = [0.7, 0., 0., 1.];
        let gray = op.pixel([0.5, 0., 0., 0.5]);
        assert_eq!(gray[0], gray[1]);
        assert_eq!(gray[1], gray[2]);
        op.parameters = [0., 1., 1., 0.];
        assert_eq!(op.pixel([0., 0., 0., 0.]), [0.; 4]);
        let white = op.pixel([0.1, 0.2, 0.3, 0.5]);
        assert_eq!(white, [0.5; 4]);
    }
}
