use crate::{
    brush_dynamics::{
        CompiledDirectionDynamics, CompiledDistanceDynamics, CompiledFadeDynamics,
        CompiledPressureControl, CompiledRotationDynamics, CompiledSpeedDynamics,
        CompiledTangentDynamics, CompiledTiltDynamics,
    },
    AngleControl, Brush, BrushDynamics, BrushMask, DynamicsSource, Error, InputPoint, Result,
    TipDynamics,
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
    pub wet: crate::WetSettings,
    pub taper: crate::StrokeTaper,
    pub airbrush: crate::Airbrush,
    pub write_blend: crate::BlendMode,
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
            wet: Default::default(),
            taper: Default::default(),
            airbrush: Default::default(),
            write_blend: Default::default(),
        }
    }
}

/// A validated immutable execution plan shared by all stroke operators.
#[derive(Clone, Debug)]
pub struct BrushProgram {
    pub(crate) taper: crate::StrokeTaper,
    pub(crate) airbrush: crate::Airbrush,
    pub(crate) smoothing: crate::BrushSmoothing,
    wet: crate::WetSettings,
    pub(crate) write_blend: crate::BlendMode,
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
    speed: Option<CompiledSpeedDynamics>,
    tangent: Option<CompiledTangentDynamics>,
    distance: Option<CompiledDistanceDynamics>,
    tilt: Option<CompiledTiltDynamics>,
    rotation: Option<CompiledRotationDynamics>,
    direction: Option<CompiledDirectionDynamics>,
    fade: Option<CompiledFadeDynamics>,
    matrix: crate::DynamicsSourceMatrix,
}
#[derive(Clone, Copy, Debug)]
pub struct BrushPose {
    pub radius: f32,
    pub angle: f32,
    pub roundness: f32,
}

/// Base tip geometry only; randomized scatter and shape jitter are not sampled.
#[derive(Clone, Copy, Debug)]
pub struct CompositeBrushPose {
    pub primary: BrushPose,
    pub secondary: Option<BrushPose>,
    pub primary_image: bool,
    pub secondary_image: bool,
    pub secondary_overflow: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct TipGeometry {
    sine: f64,
    cosine: f64,
    roundness: f32,
}

impl TipGeometry {
    pub(crate) fn wet_geometry(self, point: InputPoint, radius: f64) -> crate::WetDabGeometry {
        crate::WetDabGeometry {
            x: point.x,
            y: point.y,
            diameter: (radius * 2.) as f32,
            angle: self.sine.atan2(self.cosine).to_degrees() as f32,
            roundness: self.roundness,
        }
    }
    pub(crate) fn normalized(self, dx: f64, dy: f64, radius: f64) -> (f64, f64) {
        (
            (dx * self.cosine + dy * self.sine) / (2. * radius) + 0.5,
            (-dx * self.sine + dy * self.cosine) / (2. * radius * f64::from(self.roundness)) + 0.5,
        )
    }
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
    pub fn cursor_poses(
        &self,
        radius: f32,
        point: InputPoint,
        pressure_known: bool,
        direction: Option<(f64, f64)>,
    ) -> Result<CompositeBrushPose> {
        let primary = self.cursor_pose(radius, point, pressure_known, direction)?;
        let secondary = self
            .dual
            .as_ref()
            .map(|dual| dual.cursor_pose(radius, primary.radius, point, pressure_known, direction))
            .transpose()?;
        Ok(CompositeBrushPose {
            primary,
            secondary,
            primary_image: self.has_mask(),
            secondary_image: self.dual.as_ref().is_some_and(|dual| dual.has_mask()),
            secondary_overflow: self
                .dual
                .as_ref()
                .is_some_and(|dual| dual.color.enabled() && dual.color.overflow()),
        })
    }
    pub fn wet_settings(&self) -> crate::WetSettings {
        self.wet
    }
    pub fn compile(definition: BrushDefinition) -> Result<Self> {
        Self::compile_inner(definition, false)
    }
    pub(crate) fn compile_wet_pair(definition: BrushDefinition) -> Result<Self> {
        Self::compile_inner(definition, true)
    }
    fn compile_inner(definition: BrushDefinition, wet_pair: bool) -> Result<Self> {
        definition.brush.validate()?;
        definition.tip.validate()?;
        if definition.write_blend == crate::BlendMode::PassThrough
            || (definition.write_blend != crate::BlendMode::Normal
                && definition.brush.mode != crate::BrushMode::Paint)
        {
            return Err(Error::InvalidArgument("brush write blend mode"));
        }
        let (sine, cosine) = f64::from(definition.tip.angle).to_radians().sin_cos();
        let mut dual = definition.dual.compile(
            definition.brush.radius,
            definition.tip.pressure_size,
            &definition.dynamics.size,
        )?;
        // Dual color is meaningful for painting and finger-painting smudge.
        // Other tools retain grayscale secondary coverage but ignore colors.
        let preserve_dual_color = match definition.brush.mode {
            crate::BrushMode::Paint => true,
            crate::BrushMode::Smudge {
                finger_painting: true,
                ..
            } => !definition.wet.enabled() && dual.as_ref().is_some_and(|d| d.step().is_none()),
            _ => false,
        };
        if !preserve_dual_color {
            if let Some(d) = dual.as_mut() {
                d.color = Default::default();
            }
        }
        if dual.as_ref().is_some_and(|d| {
            d.color.enabled()
                && d.color.model() == crate::DualColorModel::InterleavedStamp
                && (d.step().is_some() || definition.wet.enabled())
        }) {
            return Err(Error::InvalidArgument(
                "interleaved dual color requires paired paint dabs without wet mixing",
            ));
        }
        if definition.wet.enabled()
            && (definition.brush.mode == crate::BrushMode::Erase
                || (!wet_pair
                    && !definition.wet.dual_color()
                    && dual.as_ref().is_some_and(|d| d.color.enabled())))
        {
            return Err(Error::InvalidArgument(
                "wet preset requires paint/smudge and declared dual-color policy",
            ));
        }
        Ok(Self {
            wet: definition.wet,
            taper: definition.taper,
            airbrush: definition.airbrush,
            smoothing: definition.smoothing,
            write_blend: definition.write_blend,
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
            dual,
            speed: definition.dynamics.speed.compile(),
            tangent: definition.dynamics.tangent.compile(),
            distance: definition.dynamics.distance.compile(),
            tilt: definition.dynamics.tilt.compile(),
            rotation: definition.dynamics.rotation.compile(),
            direction: definition.dynamics.direction.compile(),
            fade: definition.dynamics.fade.compile(),
            matrix: definition.dynamics.matrix,
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
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn factor_all(
        &self,
        speed: f32,
        tangent: f32,
        distance: f32,
        tilt: f32,
        rotation: f32,
        direction: f32,
        fade_index: u32,
    ) -> (f32, f32, f32) {
        let factors = [
            self.speed.as_ref().map_or([None; 3], |compiled| {
                let (size, flow, opacity) = compiled.factors_optional(speed);
                [size, flow, opacity]
            }),
            self.tangent.as_ref().map_or([None; 3], |compiled| {
                let (size, flow, opacity) = compiled.factors_optional(tangent);
                [size, flow, opacity]
            }),
            self.distance.as_ref().map_or([None; 3], |compiled| {
                let (size, flow, opacity) = compiled.factors_optional(distance);
                [size, flow, opacity]
            }),
            self.tilt.as_ref().map_or([None; 3], |compiled| {
                let (size, flow, opacity) = compiled.factors_optional(tilt);
                [size, flow, opacity]
            }),
            self.rotation.as_ref().map_or([None; 3], |compiled| {
                let (size, flow, opacity) = compiled.factors_optional(rotation);
                [size, flow, opacity]
            }),
            self.direction.as_ref().map_or([None; 3], |compiled| {
                let (size, flow, opacity) = compiled.factors_optional(direction);
                [size, flow, opacity]
            }),
        ];
        let active = [
            self.speed.is_some(),
            self.tangent.is_some(),
            self.distance.is_some(),
            self.tilt.is_some(),
            self.rotation.is_some(),
            self.direction.is_some(),
        ];
        let inputs = [
            self.speed
                .as_ref()
                .map_or(0., |compiled| compiled.normalized(speed)),
            self.tangent
                .as_ref()
                .map_or(0., |compiled| compiled.normalized(tangent)),
            self.distance
                .as_ref()
                .map_or(0., |compiled| compiled.normalized(distance)),
            self.tilt
                .as_ref()
                .map_or(0., |compiled| compiled.normalized(tilt)),
            self.rotation
                .as_ref()
                .map_or(0., |compiled| compiled.normalized(rotation)),
            self.direction
                .as_ref()
                .map_or(0., |compiled| compiled.normalized(direction)),
        ];
        let pick = |source: DynamicsSource, channel: usize| match source {
            DynamicsSource::All => factors
                .iter()
                .map(|factor| factor[channel].unwrap_or(1.))
                .product(),
            DynamicsSource::Speed if active[0] => factors[0][channel].unwrap_or(inputs[0]),
            DynamicsSource::Tangent if active[1] => factors[1][channel].unwrap_or(inputs[1]),
            DynamicsSource::Distance if active[2] => factors[2][channel].unwrap_or(inputs[2]),
            DynamicsSource::Tilt if active[3] => factors[3][channel].unwrap_or(inputs[3]),
            DynamicsSource::Rotation if active[4] => factors[4][channel].unwrap_or(inputs[4]),
            DynamicsSource::Direction if active[5] => factors[5][channel].unwrap_or(inputs[5]),
            _ => 1.,
        };
        let fade = self
            .fade
            .as_ref()
            .map_or((1., 1., 1.), |compiled| compiled.factors(fade_index));
        (
            pick(self.matrix.size(), 0) * fade.0,
            pick(self.matrix.flow(), 1) * fade.1,
            pick(self.matrix.opacity(), 2) * fade.2,
        )
    }
    pub(crate) fn direction_unit(direction: Option<f64>) -> f32 {
        direction.map_or(0., |angle| {
            (angle.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU) as f32
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
        self.texture.as_ref().map(|t| {
            t.dab(
                point.x,
                point.y,
                point.pressure,
                geometry.sine,
                geometry.cosine,
            )
        })
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
        // Conservative circles stay at least half a pixel from the original AA/soft edge.
        // Keep the original hypot arithmetic throughout the transition zone.
        if geometry.roundness == 1. {
            let squared = dx * dx + dy * dy;
            let inner = (radius * f64::from(self.tip.hardness) - 1.).max(0.);
            if inner > 0. && squared <= inner * inner {
                return 1.;
            }
            let outer = radius + 1.;
            if squared >= outer * outer {
                return 0.;
            }
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

#[cfg(test)]
#[test]
fn conservative_circle_bounds_preserve_the_original_hypot_formula() {
    for radius in [0.001, 0.25, 0.5, 1., 3.7, 64., 128., 256.] {
        for hardness in [0., 0.1, 0.5, 0.99, 1.] {
            let definition = BrushDefinition {
                tip: BrushTip {
                    hardness,
                    pressure_size: false,
                    pressure_flow: false,
                    ..Default::default()
                },
                ..Default::default()
            };
            let program = BrushProgram::compile(definition).unwrap();
            for i in 0..800 {
                let angle = i as f64 * 0.231;
                let distance = (i % 137) as f64 / 136. * (radius + 2.);
                let (dx, dy) = (distance * angle.cos(), distance * angle.sin());
                let d = dx.hypot(dy);
                let expected = if hardness == 1. {
                    (radius + 0.5 - d).clamp(0., 1.) as f32
                } else {
                    let edge = (radius - d + 0.5).clamp(0., 1.);
                    let soft = ((radius + 0.5 - d)
                        / (radius * (1. - f64::from(hardness))).max(0.5))
                    .clamp(0., 1.);
                    edge.min(soft) as f32
                };
                assert_eq!(
                    program.procedural_coverage(dx, dy, radius),
                    expected,
                    "r={radius} h={hardness} d={d}"
                );
            }
        }
    }
}

#[cfg(test)]
#[test]
fn source_matrix_selects_dynamics_per_target_channel() {
    use crate::{
        DistanceDynamics, DynamicsSource, DynamicsSourceMatrix, PressureControl, PressureCurve,
        RotationDynamics, SpeedDynamics, TangentDynamics, TiltDynamics,
    };
    let control = |value| {
        PressureControl::new(PressureCurve::new(vec![(0., 1.), (1., value)]).unwrap(), 0.).unwrap()
    };
    let definition = BrushDefinition {
        dynamics: BrushDynamics {
            speed: SpeedDynamics::new(true, 1000., control(0.2), control(0.3), control(0.4))
                .unwrap(),
            tangent: TangentDynamics::new(true, control(0.5), control(0.6), control(0.7)),
            distance: DistanceDynamics::new(true, 1000., control(0.8), control(0.9), control(0.1))
                .unwrap(),
            tilt: TiltDynamics::new(true, control(0.11), control(0.12), control(0.13)),
            rotation: RotationDynamics::new(true, control(0.14), control(0.15), control(0.16)),
            direction: crate::DirectionDynamics::new(
                true,
                control(0.17),
                control(0.18),
                control(0.19),
            ),
            ..Default::default()
        },
        ..Default::default()
    };
    let program = BrushProgram::compile(definition.clone()).unwrap();
    let all = program.factor_all(1000., 1., 1000., 1., 1., 1., 0);
    let expected = (
        0.2 * 0.5 * 0.8 * 0.11 * 0.14 * 0.17,
        0.3 * 0.6 * 0.9 * 0.12 * 0.15 * 0.18,
        0.4 * 0.7 * 0.1 * 0.13 * 0.16 * 0.19,
    );
    for (actual, expected) in [
        (all.0, expected.0),
        (all.1, expected.1),
        (all.2, expected.2),
    ] {
        assert!((actual - expected).abs() < 1e-6);
    }

    let mut selected = definition;
    selected.dynamics.matrix = DynamicsSourceMatrix::new(
        DynamicsSource::Distance,
        DynamicsSource::Tilt,
        DynamicsSource::Rotation,
    );
    let factors = BrushProgram::compile(selected)
        .unwrap()
        .factor_all(1000., 1., 1000., 1., 1., 1., 0);
    assert!((factors.0 - 0.8).abs() < 1e-6);
    assert!((factors.1 - 0.12).abs() < 1e-6);
    assert!((factors.2 - 0.16).abs() < 1e-6);
}
