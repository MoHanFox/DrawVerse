//! Immutable paired secondary tips. No nested operators or per-pixel allocation.
use crate::brush_program::{MaskDab, TipGeometry};
use crate::{
    BrushDefinition, BrushDynamics, BrushMask, BrushProgram, BrushTip, Error, InputPoint, Result,
};
use std::sync::{Arc, OnceLock};

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
}
impl Default for DualBrush {
    fn default() -> Self {
        static DEFAULT: OnceLock<Arc<SecondaryTip>> = OnceLock::new();
        Self {
            enabled: false,
            mode: DualMode::default(),
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
    pub fn new(enabled: bool, mode: DualMode, secondary: SecondaryTip) -> Self {
        Self {
            enabled,
            mode,
            secondary: Arc::new(secondary),
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn mode(&self) -> DualMode {
        self.mode
    }
    pub fn secondary(&self) -> &SecondaryTip {
        &self.secondary
    }
    pub fn is_default(&self) -> bool {
        !self.enabled && self.mode == DualMode::default() && self.secondary.is_default()
    }
    pub(crate) fn compile(&self) -> Result<Option<CompiledDual>> {
        if !self.enabled {
            return Ok(None);
        }
        let secondary = &self.secondary;
        let definition = BrushDefinition {
            brush: crate::Brush {
                radius: secondary.radius,
                ..Default::default()
            },
            tip: secondary.tip,
            mask: secondary.mask.clone(),
            dynamics: secondary.dynamics.clone(),
            ..Default::default()
        };
        Ok(Some(CompiledDual {
            radius: secondary.radius,
            mode: self.mode,
            program: Box::new(BrushProgram::compile(definition)?),
            step: secondary
                .spacing
                .map(|spacing| f64::from(secondary.radius * 2. * spacing)),
        }))
    }
}
#[derive(Clone, Debug)]
pub(crate) struct CompiledDual {
    radius: f32,
    mode: DualMode,
    program: Box<BrushProgram>,
    step: Option<f64>,
}
impl CompiledDual {
    pub fn step(&self) -> Option<f64> {
        self.step
    }
    pub fn combine(&self, primary: f32, secondary: f32) -> f32 {
        self.mode.combine(primary, secondary)
    }
    pub fn path_work(&self, count: usize) -> (usize, usize) {
        if self.step.is_none() {
            return (0, 0);
        }
        let diameter = (f64::from(self.radius)
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
        (self.program.tip.pressure_size
            && self.program.size_pressure(before) != self.program.size_pressure(after))
            || (self.program.tip.pressure_flow
                && self.program.flow_pressure(before) != self.program.flow_pressure(after))
    }
    pub fn random_active(&self) -> bool {
        self.program.scatter.enabled() || self.program.shape_jitter.active()
    }
    pub fn sample_cost(&self) -> usize {
        self.program.scatter.maximum_count() * if self.program.has_mask() { 8 } else { 1 }
    }
    pub fn maximum_count(&self) -> usize {
        self.program.scatter.maximum_count()
    }
    pub fn dab(
        &self,
        point: InputPoint,
        direction: Option<f64>,
        stroke: u64,
        logical: u64,
        primary: u32,
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
            let radius = f64::from(
                self.radius
                    * shape.size
                    * if self.program.tip.pressure_size {
                        self.program.size_pressure(point.pressure)
                    } else {
                        1.
                    },
            );
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
