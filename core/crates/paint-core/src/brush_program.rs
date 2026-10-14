use crate::{
    brush_dynamics::CompiledPressureControl, AngleControl, Brush, BrushDynamics, BrushMask, Error,
    InputPoint, Result, TipDynamics,
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
    pub texture: crate::BrushTexture,
    pub dual: crate::DualBrush,
    pub smoothing: crate::BrushSmoothing,
}

impl From<Brush> for BrushDefinition {
    fn from(brush: Brush) -> Self {
        Self {
            brush,
            tip: BrushTip::default(),
            mask: None,
            dynamics: BrushDynamics::default(),
            texture: Default::default(),
            dual: Default::default(),
            smoothing: Default::default(),
        }
    }
}

/// A validated immutable execution plan shared by all stroke operators.
#[derive(Clone, Debug)]
pub struct BrushProgram {
    pub(crate) smoothing: crate::BrushSmoothing,
    pub(crate) tip: BrushTip,
    sine: f64,
    cosine: f64,
    mask: Option<Arc<BrushMask>>,
    size_response: Option<CompiledPressureControl>,
    flow_response: Option<CompiledPressureControl>,
    opacity_response: Option<CompiledPressureControl>,
    pressure_opacity: bool,
    tip_dynamics: TipDynamics,
    pub(crate) scatter: crate::ScatterDynamics,
    pub(crate) shape_jitter: crate::ShapeJitter,
    pub(crate) color: crate::ColorDynamics,
    pub(crate) texture: Option<crate::brush_texture::CompiledTexture>,
    pub(crate) dual: Option<crate::brush_dual::CompiledDual>,
}
#[derive(Clone, Copy, Debug)]
pub struct BrushPose {
    pub radius: f32,
    pub angle: f32,
    pub roundness: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct TipGeometry {
    sine: f64,
    cosine: f64,
    roundness: f32,
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
    pub fn cursor_pose(
        &self,
        radius: f32,
        point: InputPoint,
        pressure_known: bool,
        direction: Option<(f64, f64)>,
    ) -> Result<BrushPose> {
        if !radius.is_finite() || !(0.5..=256.).contains(&radius) {
            return Err(Error::InvalidArgument("pose radius"));
        }
        point.validate()?;
        let direction = match direction {
            Some((x, y)) if x.is_finite() && y.is_finite() && x.hypot(y) > 1e-9 => Some(y.atan2(x)),
            Some(_) => return Err(Error::InvalidArgument("pose direction")),
            None => None,
        };
        let geometry = self.geometry(point, direction);
        Ok(BrushPose {
            radius: radius
                * if pressure_known && self.tip.pressure_size {
                    self.size_pressure(point.pressure)
                } else {
                    1.
                },
            angle: geometry.sine.atan2(geometry.cosine).to_degrees() as f32,
            roundness: geometry.roundness,
        })
    }
    pub fn compile(definition: BrushDefinition) -> Result<Self> {
        definition.brush.validate()?;
        definition.tip.validate()?;
        let (sine, cosine) = f64::from(definition.tip.angle).to_radians().sin_cos();
        Ok(Self {
            smoothing: definition.smoothing,
            tip: definition.tip,
            sine,
            cosine,
            mask: definition.mask,
            size_response: definition.dynamics.size.compile(),
            flow_response: definition.dynamics.flow.compile(),
            opacity_response: definition.dynamics.opacity.compile(),
            pressure_opacity: definition.dynamics.pressure_opacity,
            tip_dynamics: definition.dynamics.tip,
            scatter: definition.dynamics.scatter,
            shape_jitter: definition.dynamics.shape_jitter,
            color: definition.dynamics.color,
            texture: definition.texture.compile(),
            dual: definition.dual.compile()?,
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
    /// Base tip coverage; the document executor binds dual/texture dab operators.
    pub fn coverage(&self, dx: f64, dy: f64, radius: f64) -> f32 {
        if self.has_mask() {
            self.mask_dab(radius).map_or(0., |dab| dab.coverage(dx, dy))
        } else {
            self.procedural_coverage(dx, dy, radius)
        }
    }
    pub(crate) fn has_texture(&self) -> bool {
        self.texture.is_some()
    }
    pub(crate) fn texture_dab(
        &self,
        point: InputPoint,
        geometry: TipGeometry,
    ) -> Option<crate::brush_texture::TextureDab<'_>> {
        self.texture
            .as_ref()
            .map(|t| t.dab(point.x, point.y, geometry.sine, geometry.cosine))
    }
    pub fn has_mask(&self) -> bool {
        self.mask.is_some()
    }
    fn base_geometry(&self) -> TipGeometry {
        TipGeometry {
            sine: self.sine,
            cosine: self.cosine,
            roundness: self.tip.roundness,
        }
    }
    pub(crate) fn geometry(&self, point: InputPoint, direction: Option<f64>) -> TipGeometry {
        if self.tip_dynamics == TipDynamics::default() {
            return self.base_geometry();
        }
        let tilt = point.capabilities & 2 != 0;
        let tilt_length = f64::from(point.tilt_x).hypot(f64::from(point.tilt_y));
        let angle = match self.tip_dynamics.angle() {
            AngleControl::Fixed => None,
            AngleControl::Direction => direction,
            AngleControl::PenRotation => {
                (point.capabilities & 4 != 0).then(|| f64::from(point.rotation).to_radians())
            }
            AngleControl::Tilt => (tilt && tilt_length > 1e-9)
                .then(|| f64::from(point.tilt_y).atan2(f64::from(point.tilt_x))),
        };
        let (sine, cosine) = angle.map_or((self.sine, self.cosine), |angle| {
            (angle + f64::from(self.tip.angle).to_radians()).sin_cos()
        });
        let roundness = if tilt && self.tip_dynamics.tilt_roundness() {
            (self.tip.roundness
                * (tilt_length.min(90.).to_radians().cos() as f32)
                    .max(self.tip_dynamics.minimum_roundness()))
            .max(0.01)
        } else {
            self.tip.roundness
        };
        TipGeometry {
            sine,
            cosine,
            roundness,
        }
    }
    pub(crate) fn dab_geometry(
        &self,
        point: InputPoint,
        direction: Option<f64>,
        sample: crate::brush_shape_jitter::ShapeSample,
    ) -> TipGeometry {
        let mut geometry = self.geometry(point, direction);
        if sample.angle != 0. {
            let (s, c) = sample.angle.sin_cos();
            let sine = geometry.sine * c + geometry.cosine * s;
            geometry.cosine = geometry.cosine * c - geometry.sine * s;
            geometry.sine = sine;
        }
        if sample.roundness != 1. {
            geometry.roundness = (geometry.roundness * sample.roundness).max(0.01);
        }
        geometry
    }
    fn mask_dab(&self, radius: f64) -> Option<MaskDab<'_>> {
        self.mask_dab_with_geometry(radius, self.base_geometry())
    }
    fn procedural_coverage(&self, dx: f64, dy: f64, radius: f64) -> f32 {
        self.procedural_coverage_with_geometry(dx, dy, radius, self.base_geometry())
    }
    pub(crate) fn extent(&self, radius: f64, geometry: TipGeometry) -> (f64, f64) {
        if let Some(mask) = &self.mask {
            let (w, h) = mask.dimensions();
            let scale = radius / f64::from(w.max(h));
            let (x, y) = (
                f64::from(w) * scale,
                f64::from(h) * scale * f64::from(geometry.roundness),
            );
            (
                geometry.cosine.abs() * x + geometry.sine.abs() * y,
                geometry.sine.abs() * x + geometry.cosine.abs() * y,
            )
        } else {
            (radius, radius)
        }
    }
    pub(crate) fn mask_dab_with_geometry(
        &self,
        radius: f64,
        geometry: TipGeometry,
    ) -> Option<MaskDab<'_>> {
        if radius <= 0. {
            return None;
        }
        let mask = self.mask.as_ref().expect("image path");
        let (w, h) = mask.dimensions();
        let scale = radius / f64::from(w.max(h));
        let (rx, ry) = (
            f64::from(w) * scale,
            f64::from(h) * scale * f64::from(geometry.roundness),
        );
        Some(MaskDab {
            mask,
            sine: geometry.sine,
            cosine: geometry.cosine,
            rx,
            ry,
            lod: mask.lod(radius * 2. * f64::from(geometry.roundness)),
        })
    }
    #[inline]
    pub(crate) fn procedural_coverage_with_geometry(
        &self,
        dx: f64,
        dy: f64,
        radius: f64,
        geometry: TipGeometry,
    ) -> f32 {
        if radius <= 0. {
            return 0.;
        }
        let (distance, gradient) = if geometry.roundness == 1. {
            (dx.hypot(dy), 1.)
        } else {
            let u = dx * geometry.cosine + dy * geometry.sine;
            let v = -dx * geometry.sine + dy * geometry.cosine;
            let aspect = f64::from(geometry.roundness);
            let distance = u.hypot(v / aspect);
            let gradient = if distance > 1e-12 {
                u.hypot(v / (aspect * aspect)) / distance
            } else {
                1.
            };
            (distance, gradient)
        };
        // Keep the exact legacy arithmetic for hard round presets.
        if self.tip.hardness == 1. && geometry.roundness == 1. {
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
