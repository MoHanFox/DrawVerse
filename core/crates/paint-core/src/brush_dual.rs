//! Immutable paired secondary tips. No nested operators or per-pixel allocation.
use crate::brush_program::{MaskDab, TipGeometry};
use crate::{
    BrushDefinition, BrushDynamics, BrushMask, BrushProgram, BrushTip, Error, InputPoint, Result,
};
use std::sync::{Arc, OnceLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DualSizeMode {
    #[default]
    Independent,
    Relative,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DualWidthRule {
    #[default]
    FollowPressure,
    FixedWidth,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DualSizing {
    mode: DualSizeMode,
    diameter_offset: f32,
    width_rule: DualWidthRule,
}
impl DualSizing {
    pub fn new(
        mode: DualSizeMode,
        diameter_offset: f32,
        width_rule: DualWidthRule,
    ) -> Result<Self> {
        if !diameter_offset.is_finite() || !(-511. ..=511.).contains(&diameter_offset) {
            return Err(Error::InvalidArgument("secondary diameter offset"));
        }
        Ok(Self {
            mode,
            diameter_offset,
            width_rule,
        })
    }
    pub fn mode(self) -> DualSizeMode {
        self.mode
    }
    pub fn diameter_offset(self) -> f32 {
        self.diameter_offset
    }
    pub fn width_rule(self) -> DualWidthRule {
        self.width_rule
    }
    fn fixed(self) -> bool {
        self.mode == DualSizeMode::Relative && self.width_rule == DualWidthRule::FixedWidth
    }
    pub(crate) fn nominal_radius(self, primary: f32, secondary: f32) -> f32 {
        match self.mode {
            DualSizeMode::Independent => secondary,
            DualSizeMode::Relative => (primary + self.diameter_offset / 2.).clamp(0.5, 256.),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DualMode {
    #[default]
    Multiply,
    Darken,
    Lighten,
    Subtract,
    Difference,
    Screen,
}
impl DualMode {
    pub fn combine(self, a: f32, b: f32) -> f32 {
        if a == 0. {
            return 0.;
        }
        match self {
            Self::Multiply => a * b,
            Self::Darken => a.min(b),
            Self::Lighten => a.max(b),
            Self::Subtract => a * (1. - b),
            Self::Difference => (a - b).abs(),
            Self::Screen => a + b - a * b,
        }
        .clamp(0., 1.)
    }
}
#[derive(Clone, Debug)]
pub struct SecondaryTip {
    radius: f32,
    tip: BrushTip,
    mask: Option<Arc<BrushMask>>,
    dynamics: BrushDynamics,
    spacing: Option<f32>,
}
impl Default for SecondaryTip {
    fn default() -> Self {
        Self {
            radius: 12.,
            tip: BrushTip::default(),
            mask: None,
            dynamics: BrushDynamics::default(),
            spacing: None,
        }
    }
}
impl SecondaryTip {
    pub fn new(
        radius: f32,
        tip: BrushTip,
        mask: Option<Arc<BrushMask>>,
        dynamics: BrushDynamics,
    ) -> Result<Self> {
        if !radius.is_finite()
            || !(0.5..=256.).contains(&radius)
            || dynamics.pressure_opacity
            || dynamics.opacity != Default::default()
            || dynamics.color != Default::default()
        {
            return Err(Error::InvalidArgument("secondary tip parameters"));
        }
        tip.validate()?;
        Ok(Self {
            radius,
            tip,
            mask,
            dynamics,
            spacing: None,
        })
    }
    pub fn radius(&self) -> f32 {
        self.radius
    }
    /// Independent path distance as a fraction of the secondary nominal diameter.
    pub fn with_spacing(mut self, spacing: f32) -> Result<Self> {
        if !spacing.is_finite() || !(0.01..=1.).contains(&spacing) {
            return Err(Error::InvalidArgument("secondary spacing"));
        }
        self.spacing = Some(spacing);
        Ok(self)
    }
    pub fn spacing(&self) -> Option<f32> {
        self.spacing
    }
    pub fn tip(&self) -> BrushTip {
        self.tip
    }
    pub fn mask(&self) -> Option<&Arc<BrushMask>> {
        self.mask.as_ref()
    }
    pub fn dynamics(&self) -> &BrushDynamics {
        &self.dynamics
    }
    fn is_default(&self) -> bool {
        self.radius == 12.
            && self.tip == BrushTip::default()
            && self.mask.is_none()
            && self.dynamics == BrushDynamics::default()
            && self.spacing == Some(0.25)
    }
}
#[derive(Clone, Debug)]
pub struct DualBrush {
    enabled: bool,
    mode: DualMode,
    secondary: Arc<SecondaryTip>,
    sizing: DualSizing,
    color: crate::DualColor,
}
impl Default for DualBrush {
    fn default() -> Self {
        static DEFAULT: OnceLock<Arc<SecondaryTip>> = OnceLock::new();
        Self {
            enabled: false,
            mode: DualMode::default(),
            sizing: DualSizing::default(),
            color: crate::DualColor::default(),
            secondary: DEFAULT
                .get_or_init(|| {
                    Arc::new(SecondaryTip {
                        spacing: Some(0.25),
                        ..SecondaryTip::default()
                    })
                })
                .clone(),
        }
    }
}
impl DualBrush {
    /// Nominal carrier size, shared by automatic and manual wet loading.
    pub fn nominal_diameter(&self, primary_radius: f32) -> Result<f32> {
        if !primary_radius.is_finite() || !(0.5..=256.).contains(&primary_radius) {
            return Err(Error::InvalidArgument("primary nominal radius"));
        }
        Ok(self
            .sizing
            .nominal_radius(primary_radius, self.secondary.radius)
            * 2.)
    }
    pub fn new(enabled: bool, mode: DualMode, secondary: SecondaryTip) -> Self {
        Self {
            enabled,
            mode,
            secondary: Arc::new(secondary),
            sizing: DualSizing::default(),
            color: crate::DualColor::default(),
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn with_sizing(mut self, sizing: DualSizing) -> Self {
        self.sizing = sizing;
        self
    }
    pub fn sizing(&self) -> DualSizing {
        self.sizing
    }
    pub fn with_color(mut self, color: crate::DualColor) -> Self {
        self.color = color;
        self
    }
    pub fn color(&self) -> crate::DualColor {
        self.color
    }
    pub fn mode(&self) -> DualMode {
        self.mode
    }
    pub fn secondary(&self) -> &SecondaryTip {
        &self.secondary
    }
    pub fn is_default(&self) -> bool {
        !self.enabled
            && self.mode == DualMode::default()
            && self.secondary.is_default()
            && self.sizing == DualSizing::default()
            && self.color == crate::DualColor::default()
    }
    pub(crate) fn compile(
        &self,
        primary_radius: f32,
        primary_pressure_size: bool,
        primary_size: &crate::PressureControl,
    ) -> Result<Option<CompiledDual>> {
        if !self.enabled {
            return Ok(None);
        }
        let secondary = &self.secondary;
        let radius = self.sizing.nominal_radius(primary_radius, secondary.radius);
        let definition = BrushDefinition {
            brush: crate::Brush {
                radius,
                ..Default::default()
            },
            tip: secondary.tip,
            mask: secondary.mask.clone(),
            dynamics: secondary.dynamics.clone(),
            ..Default::default()
        };
        Ok(Some(CompiledDual {
            radius,
            maximum_radius: if self.sizing.fixed() {
                (f64::from(primary_radius) + f64::from(self.sizing.diameter_offset) / 2.)
                    .clamp(0.5, 256.)
            } else {
                f64::from(radius)
            },
            sizing: self.sizing,
            color: self.color,
            primary_pressure_size,
            primary_size: primary_size.compile(),
            mode: self.mode,
            program: Box::new(BrushProgram::compile(definition)?),
            step: secondary
                .spacing
                .map(|spacing| f64::from(radius * 2. * spacing)),
        }))
    }
}
#[derive(Clone, Debug)]
pub(crate) struct CompiledDual {
    radius: f32,
    maximum_radius: f64,
    mode: DualMode,
    program: Box<BrushProgram>,
    step: Option<f64>,
    sizing: DualSizing,
    pub(crate) color: crate::DualColor,
    primary_pressure_size: bool,
    primary_size: Option<crate::brush_dynamics::CompiledPressureControl>,
}
impl CompiledDual {
    pub fn has_mask(&self) -> bool {
        self.program.has_mask()
    }
    pub fn cursor_pose(
        &self,
        primary_nominal: f32,
        primary_actual: f32,
        point: InputPoint,
        pressure_known: bool,
        direction: Option<(f64, f64)>,
    ) -> Result<crate::BrushPose> {
        let nominal = self.sizing.nominal_radius(primary_nominal, self.radius);
        let mut pose = self
            .program
            .cursor_pose(nominal, point, pressure_known, direction)?;
        if self.sizing.fixed() {
            pose.radius = (primary_actual + self.sizing.diameter_offset / 2.).clamp(0.5, 256.);
        }
        Ok(pose)
    }
    pub fn step(&self) -> Option<f64> {
        self.step
    }
    pub fn colored_work(&self, count: usize, tool_blend: crate::BlendMode) -> usize {
        let diameter = (self.maximum_radius
            * 2.
            * if self.program.has_mask() {
                std::f64::consts::SQRT_2
            } else {
                1.
            }
            + 3.)
            .ceil() as usize;
        let cost = self.colored_cost(tool_blend);
        count
            .saturating_mul(self.maximum_count())
            .saturating_mul(diameter * diameter)
            .saturating_mul(cost)
    }
    pub fn colored_cost(&self, tool_blend: crate::BlendMode) -> usize {
        4 + if self.color.primary_blend() != crate::BlendMode::Normal {
            8
        } else {
            0
        } + if self.color.secondary_blend() != crate::BlendMode::Normal {
            8
        } else {
            0
        } + if tool_blend != crate::BlendMode::Normal {
            8
        } else {
            0
        }
    }
    pub fn combine(&self, primary: f32, secondary: f32) -> f32 {
        self.mode.combine(primary, secondary)
    }
    pub fn path_work(&self, count: usize) -> (usize, usize) {
        if self.step.is_none() {
            return (0, 0);
        }
        let diameter = (self.maximum_radius
            * 2.
            * if self.program.has_mask() {
                std::f64::consts::SQRT_2
            } else {
                1.
            }
            + 2.)
            .ceil() as usize;
        (
            count.saturating_mul(self.maximum_count()),
            count
                .saturating_mul(diameter * diameter)
                .saturating_mul(self.sample_cost()),
        )
    }
    pub fn pressure_changed(&self, before: f32, after: f32) -> bool {
        let size_changed = if self.sizing.fixed() {
            self.primary_pressure_size
                && self
                    .primary_size
                    .as_ref()
                    .map_or(before, |p| p.response(before))
                    != self
                        .primary_size
                        .as_ref()
                        .map_or(after, |p| p.response(after))
        } else {
            self.program.tip.pressure_size
                && self.program.size_pressure(before) != self.program.size_pressure(after)
        };
        size_changed
            || (self.program.tip.pressure_flow
                && self.program.flow_pressure(before) != self.program.flow_pressure(after))
    }
    pub fn random_active(&self) -> bool {
        self.program.scatter.enabled()
            || self.program.shape_jitter.active()
            || self.color.random_active()
    }
    pub fn sample_cost(&self) -> usize {
        self.program.scatter.maximum_count() * if self.program.has_mask() { 8 } else { 1 }
    }
    pub fn maximum_count(&self) -> usize {
        self.program.scatter.maximum_count()
    }
    #[allow(clippy::too_many_arguments)]
    pub fn dab(
        &self,
        point: InputPoint,
        direction: Option<f64>,
        stroke: u64,
        logical: u64,
        primary: u32,
        primary_radius: f64,
        size_scale: f64,
    ) -> DualDab<'_> {
        let stroke = stroke ^ crate::brush_random::mix(0xa4093822299f31d0 ^ u64::from(primary));
        let plan = self
            .program
            .scatter
            .plan(self.radius, stroke, logical, direction);
        let count = plan.as_ref().map_or(1, |p| p.count);
        let flow = self.program.tip.flow
            * if self.program.tip.pressure_flow {
                self.program.flow_pressure(point.pressure)
            } else {
                1.
            };
        let mut dabs = Vec::with_capacity(count as usize);
        for child in 0..count {
            let center = plan.as_ref().map_or(point, |p| p.point(child, point));
            let shape = self.program.shape_jitter.sample(stroke, logical, child);
            let radius = if self.sizing.fixed() {
                (primary_radius + f64::from(self.sizing.diameter_offset) / 2.).clamp(0.5, 256.)
            } else {
                f64::from(
                    self.radius
                        * shape.size
                        * if self.program.tip.pressure_size {
                            self.program.size_pressure(point.pressure)
                        } else {
                            1.
                        },
                ) * size_scale
            };
            let geometry = self.program.dab_geometry(center, direction, shape);
            let mask = if radius > 0. && self.program.has_mask() {
                self.program.mask_dab_with_geometry(radius, geometry)
            } else {
                None
            };
            dabs.push(SecondaryDab {
                x: center.x,
                y: center.y,
                radius,
                geometry,
                mask,
            });
        }
        DualDab {
            program: &self.program,
            mode: self.mode,
            flow,
            dabs,
        }
    }
}
struct SecondaryDab<'a> {
    x: f64,
    y: f64,
    radius: f64,
    geometry: TipGeometry,
    mask: Option<MaskDab<'a>>,
}

pub(crate) struct DualDab<'a> {
    program: &'a BrushProgram,
    mode: DualMode,
    flow: f32,
    dabs: Vec<SecondaryDab<'a>>,
}
impl DualDab<'_> {
    pub fn scaled(mut self, factor: f32) -> Self {
        if factor != 1. {
            for dab in &mut self.dabs {
                dab.radius *= f64::from(factor);
                dab.mask = if dab.radius > 0. && self.program.has_mask() {
                    self.program
                        .mask_dab_with_geometry(dab.radius, dab.geometry)
                } else {
                    None
                };
            }
        }
        self
    }
    pub fn count(&self) -> usize {
        self.dabs.len()
    }
    pub fn bounds(&self, index: usize) -> (f64, f64, f64, f64) {
        let dab = &self.dabs[index];
        let (x, y) = self.program.extent(dab.radius, dab.geometry);
        (
            dab.x - x - 1.,
            dab.y - y - 1.,
            dab.x + x + 1.,
            dab.y + y + 1.,
        )
    }
    pub(crate) fn wet_geometry(&self, index: usize) -> Option<crate::WetDabGeometry> {
        let dab = &self.dabs[index];
        (dab.radius > 0.).then(|| {
            dab.geometry
                .wet_geometry(InputPoint::new(dab.x, dab.y, 1.), dab.radius)
        })
    }
    pub(crate) fn wet_uv(&self, index: usize, x: f64, y: f64) -> (f64, f64) {
        let dab = &self.dabs[index];
        dab.geometry.normalized(x - dab.x, y - dab.y, dab.radius)
    }
    pub(crate) fn shape_coverage(&self, index: usize, x: f64, y: f64) -> f32 {
        let dab = &self.dabs[index];
        if dab.radius <= 0. {
            0.
        } else if let Some(mask) = &dab.mask {
            mask.coverage(x - dab.x, y - dab.y)
        } else {
            self.program.procedural_coverage_with_geometry(
                x - dab.x,
                y - dab.y,
                dab.radius,
                dab.geometry,
            )
        }
    }
    pub fn coverage(&self, index: usize, x: f64, y: f64) -> f32 {
        let dab = &self.dabs[index];
        if dab.radius == 0. {
            0.
        } else if let Some(mask) = &dab.mask {
            mask.coverage(x - dab.x, y - dab.y) * self.flow
        } else {
            self.program.procedural_coverage_with_geometry(
                x - dab.x,
                y - dab.y,
                dab.radius,
                dab.geometry,
            ) * self.flow
        }
    }
    pub fn combine(&self, primary: f32, x: f64, y: f64) -> f32 {
        if primary == 0. {
            return 0.;
        }
        let mut coverage = 0.;
        for dab in &self.dabs {
            let amount = if dab.radius == 0. {
                0.
            } else if let Some(mask) = &dab.mask {
                mask.coverage(x - dab.x, y - dab.y)
            } else {
                self.program.procedural_coverage_with_geometry(
                    x - dab.x,
                    y - dab.y,
                    dab.radius,
                    dab.geometry,
                )
            } * self.flow;
            coverage += amount * (1. - coverage);
        }
        self.mode.combine(primary, coverage)
    }
}

#[cfg(test)]
mod sizing_tests {
    use super::*;
    #[test]
    fn composite_cursor_radius_matches_actual_base_dabs() {
        for mode in [DualSizeMode::Independent, DualSizeMode::Relative] {
            for rule in [DualWidthRule::FollowPressure, DualWidthRule::FixedWidth] {
                for offset in [-511., -8., 0., 8., 511.] {
                    for nominal in [0.5, 12., 256.] {
                        let definition =
                            DualBrush::new(true, DualMode::Multiply, SecondaryTip::default())
                                .with_sizing(DualSizing::new(mode, offset, rule).unwrap());
                        let compiled = definition
                            .compile(nominal, true, &Default::default())
                            .unwrap()
                            .unwrap();
                        for pressure in [0., 0.1, 0.5, 1.] {
                            let point = InputPoint::new(32.5, 32.5, pressure);
                            let actual = nominal * pressure;
                            let pose = compiled
                                .cursor_pose(nominal, actual, point, true, None)
                                .unwrap();
                            let dab = compiled.dab(point, None, 17, 0, 0, f64::from(actual), 1.);
                            assert_eq!(
                                pose.radius, dab.dabs[0].radius as f32,
                                "{mode:?}/{rule:?}/{offset}/{nominal}/{pressure}"
                            );
                            let hover = compiled
                                .cursor_pose(nominal, nominal, point, false, None)
                                .unwrap();
                            let dab = compiled.dab(
                                InputPoint::new(32.5, 32.5, 1.),
                                None,
                                17,
                                0,
                                0,
                                f64::from(nominal),
                                1.,
                            );
                            assert_eq!(hover.radius, dab.dabs[0].radius as f32);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn fixed_geometry_step_and_pressure_checks_use_primary_rules() {
        let tip = BrushTip {
            pressure_size: true,
            pressure_flow: false,
            ..Default::default()
        };
        let secondary = SecondaryTip::new(
            32.,
            tip,
            None,
            BrushDynamics {
                shape_jitter: crate::ShapeJitter::new(true, 0.9, 0., 0., 0., 0.01, 73).unwrap(),
                ..Default::default()
            },
        )
        .unwrap()
        .with_spacing(0.01)
        .unwrap();
        let sizing =
            DualSizing::new(DualSizeMode::Relative, 8., DualWidthRule::FixedWidth).unwrap();
        let compiled = DualBrush::new(true, DualMode::Multiply, secondary)
            .with_sizing(sizing)
            .compile(8., true, &crate::PressureControl::default())
            .unwrap()
            .unwrap();
        assert_eq!(compiled.step(), Some(f64::from(24_f32 * 0.01)));
        assert!(compiled.pressure_changed(0.25, 1.));
        for actual in [0., 2., 6.25, 8.] {
            let dab = compiled.dab(
                InputPoint::new(32.5, 32.5, 0.01),
                None,
                19,
                5,
                0,
                actual,
                1.,
            );
            assert_eq!(dab.dabs[0].radius, actual + 4.);
        }
        let lower = DualBrush::new(
            true,
            DualMode::Multiply,
            SecondaryTip::new(12., tip, None, Default::default()).unwrap(),
        )
        .with_sizing(
            DualSizing::new(DualSizeMode::Relative, -511., DualWidthRule::FixedWidth).unwrap(),
        )
        .compile(8., false, &crate::PressureControl::default())
        .unwrap()
        .unwrap();
        assert_eq!(
            lower
                .dab(InputPoint::new(1., 1., 0.), None, 1, 0, 0, 0., 1.)
                .dabs[0]
                .radius,
            0.5
        );
        assert!(!lower.pressure_changed(0., 1.));
    }
}
