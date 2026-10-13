use crate::{
    brush_dynamics::CompiledPressureControl, Brush, BrushDynamics, BrushMask, Error, Result,
};
use std::sync::Arc;

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

#[derive(Clone, Debug, Default)]
pub struct BrushDefinition {
    pub brush: Brush,
    pub tip: BrushTip,
    pub mask: Option<Arc<BrushMask>>,
    pub dynamics: BrushDynamics,
}

impl From<Brush> for BrushDefinition {
    fn from(brush: Brush) -> Self {
        Self {
            brush,
            tip: BrushTip::default(),
            mask: None,
            dynamics: BrushDynamics::default(),
        }
    }
}

/// A validated immutable execution plan shared by all stroke operators.
#[derive(Clone, Debug)]
pub struct BrushProgram {
    pub(crate) tip: BrushTip,
    sine: f64,
    cosine: f64,
    mask: Option<Arc<BrushMask>>,
    size_response: Option<CompiledPressureControl>,
    flow_response: Option<CompiledPressureControl>,
    opacity_response: Option<CompiledPressureControl>,
    pressure_opacity: bool,
}

/// Geometry and minification compiled once for one pressure-sized image dab.
pub(crate) struct MaskDab<'a> {
    mask: &'a BrushMask,
    sine: f64,
    cosine: f64,
    rx: f64,
    ry: f64,
    lod: (usize, usize, f32),
}
impl MaskDab<'_> {
    #[inline]
    pub(crate) fn coverage(&self, dx: f64, dy: f64) -> f32 {
        let x = dx * self.cosine + dy * self.sine;
        let y = -dx * self.sine + dy * self.cosine;
        let edge = (self.rx + 0.5 - x.abs()).clamp(0., 1.).min(self.rx * 2.)
            * (self.ry + 0.5 - y.abs()).clamp(0., 1.).min(self.ry * 2.);
        if edge == 0. {
            return 0.;
        }
        self.mask
            .sample((x / self.rx + 1.) * 0.5, (y / self.ry + 1.) * 0.5, self.lod)
            * edge as f32
    }
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
            mask: definition.mask,
            size_response: definition.dynamics.size.compile(),
            flow_response: definition.dynamics.flow.compile(),
            opacity_response: definition.dynamics.opacity.compile(),
            pressure_opacity: definition.dynamics.pressure_opacity,
        })
    }

    #[inline]
    pub(crate) fn size_pressure(&self, pressure: f32) -> f32 {
        self.size_response
            .as_ref()
            .map_or(pressure, |curve| curve.response(pressure))
    }
    #[inline]
    pub(crate) fn flow_pressure(&self, pressure: f32) -> f32 {
        self.flow_response
            .as_ref()
            .map_or(pressure, |curve| curve.response(pressure))
    }
    pub(crate) fn opacity_pressure(&self, pressure: f32) -> Option<f32> {
        self.pressure_opacity.then(|| {
            self.opacity_response
                .as_ref()
                .map_or(pressure, |curve| curve.response(pressure))
        })
    }
    #[inline]
    pub fn coverage(&self, dx: f64, dy: f64, radius: f64) -> f32 {
        if self.has_mask() {
            self.mask_dab(radius).map_or(0., |dab| dab.coverage(dx, dy))
        } else {
            self.procedural_coverage(dx, dy, radius)
        }
    }
    pub fn has_mask(&self) -> bool {
        self.mask.is_some()
    }
    pub(crate) fn extent(&self, radius: f64) -> (f64, f64) {
        if let Some(mask) = &self.mask {
            let (w, h) = mask.dimensions();
            let scale = radius / f64::from(w.max(h));
            let (x, y) = (
                f64::from(w) * scale,
                f64::from(h) * scale * f64::from(self.tip.roundness),
            );
            (
                self.cosine.abs() * x + self.sine.abs() * y,
                self.sine.abs() * x + self.cosine.abs() * y,
            )
        } else {
            (radius, radius)
        }
    }
    pub(crate) fn mask_dab(&self, radius: f64) -> Option<MaskDab<'_>> {
        if radius <= 0. {
            return None;
        }
        let mask = self.mask.as_ref().expect("image path");
        let (w, h) = mask.dimensions();
        let scale = radius / f64::from(w.max(h));
        let (rx, ry) = (
            f64::from(w) * scale,
            f64::from(h) * scale * f64::from(self.tip.roundness),
        );
        Some(MaskDab {
            mask,
            sine: self.sine,
            cosine: self.cosine,
            rx,
            ry,
            lod: mask.lod(radius * 2. * f64::from(self.tip.roundness)),
        })
    }
    #[inline]
    pub(crate) fn procedural_coverage(&self, dx: f64, dy: f64, radius: f64) -> f32 {
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
