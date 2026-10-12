use crate::{Brush, Error, Result};

/// Procedural tip parameters. Defaults preserve the legacy round brush.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushTip {
    pub hardness: f32,
    pub roundness: f32,
    pub angle: f32,
    pub flow: f32,
    pub pressure_size: bool,
    pub pressure_flow: bool,
}

impl Default for BrushTip {
    fn default() -> Self {
        Self {
            hardness: 1.,
            roundness: 1.,
            angle: 0.,
            flow: 1.,
            pressure_size: true,
            pressure_flow: true,
        }
    }
}

impl BrushTip {
    pub fn validate(self) -> Result<()> {
        for (value, minimum, maximum) in [
            (self.hardness, 0., 1.),
            (self.roundness, 0.01, 1.),
            (self.angle, -180., 180.),
            (self.flow, 0., 1.),
        ] {
            if !value.is_finite() || !(minimum..=maximum).contains(&value) {
                return Err(Error::InvalidArgument("brush tip parameter out of range"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BrushDefinition {
    pub brush: Brush,
    pub tip: BrushTip,
}

impl From<Brush> for BrushDefinition {
    fn from(brush: Brush) -> Self {
        Self {
            brush,
            tip: BrushTip::default(),
        }
    }
}

/// A validated immutable execution plan shared by all stroke operators.
#[derive(Clone, Copy, Debug)]
pub struct BrushProgram {
    pub(crate) tip: BrushTip,
    sine: f64,
    cosine: f64,
}

impl BrushProgram {
    pub fn compile(definition: BrushDefinition) -> Result<Self> {
        definition.brush.validate()?;
        definition.tip.validate()?;
        let (sine, cosine) = f64::from(definition.tip.angle).to_radians().sin_cos();
        Ok(Self {
            tip: definition.tip,
            sine,
            cosine,
        })
    }

    #[inline]
    pub fn coverage(self, dx: f64, dy: f64, radius: f64) -> f32 {
        if radius <= 0. {
            return 0.;
        }
        let (distance, gradient) = if self.tip.roundness == 1. {
            (dx.hypot(dy), 1.)
        } else {
            let u = dx * self.cosine + dy * self.sine;
            let v = -dx * self.sine + dy * self.cosine;
            let aspect = f64::from(self.tip.roundness);
            let distance = u.hypot(v / aspect);
            let gradient = if distance > 1e-12 {
                u.hypot(v / (aspect * aspect)) / distance
            } else {
                1.
            };
            (distance, gradient)
        };
        // Keep the exact legacy arithmetic for hard round presets.
        if self.tip.hardness == 1. && self.tip.roundness == 1. {
            return (radius + 0.5 - distance).clamp(0., 1.) as f32;
        }
        let edge = ((radius - distance) / gradient + 0.5).clamp(0., 1.);
        if self.tip.hardness == 1. {
            return edge as f32;
        }
        let soft = ((radius + 0.5 - distance)
            / (radius * (1. - f64::from(self.tip.hardness))).max(0.5))
        .clamp(0., 1.);
        edge.min(soft) as f32
    }
}
