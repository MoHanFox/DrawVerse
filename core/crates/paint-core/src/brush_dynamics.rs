use crate::{Error, Result};
use std::sync::{Arc, OnceLock};

/// Immutable, bounded pressure response. X is input pressure; Y is output ratio.
#[derive(Clone, Debug, PartialEq)]
pub struct PressureCurve {
    points: Arc<[(f32, f32)]>,
}
impl Default for PressureCurve {
    fn default() -> Self {
        static LINEAR: OnceLock<Arc<[(f32, f32)]>> = OnceLock::new();
        Self {
            points: LINEAR
                .get_or_init(|| Arc::from([(0., 0.), (1., 1.)]))
                .clone(),
        }
    }
}
impl PressureCurve {
    pub fn new(points: Vec<(f32, f32)>) -> Result<Self> {
        if !(2..=17).contains(&points.len())
            || points.first().map(|p| p.0) != Some(0.)
            || points.last().map(|p| p.0) != Some(1.)
        {
            return Err(Error::InvalidArgument("pressure curve count/endpoints"));
        }
        if points.iter().any(|(x, y)| {
            !x.is_finite() || !y.is_finite() || !(0. ..=1.).contains(x) || !(0. ..=1.).contains(y)
        }) || points.windows(2).any(|p| p[0].0 >= p[1].0)
        {
            return Err(Error::InvalidArgument("pressure curve range/order"));
        }
        Ok(Self {
            points: points.into(),
        })
    }
    pub fn points(&self) -> &[(f32, f32)] {
        &self.points
    }
    fn is_linear(&self) -> bool {
        self.points.iter().all(|(x, y)| x == y)
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PressureControl {
    curve: PressureCurve,
    minimum: f32,
}
impl PressureControl {
    pub fn new(curve: PressureCurve, minimum: f32) -> Result<Self> {
        if !minimum.is_finite() || !(0. ..=1.).contains(&minimum) {
            return Err(Error::InvalidArgument("pressure minimum ratio"));
        }
        Ok(Self { curve, minimum })
    }
    pub fn curve(&self) -> &PressureCurve {
        &self.curve
    }
    pub fn minimum(&self) -> f32 {
        self.minimum
    }
    pub fn is_identity(&self) -> bool {
        self.minimum == 0. && self.curve.is_linear()
    }
    pub fn response(&self, pressure: f32) -> Result<f32> {
        if !pressure.is_finite() || !(0. ..=1.).contains(&pressure) {
            return Err(Error::InvalidArgument("pressure input"));
        }
        Ok(self
            .compile()
            .map_or(pressure, |curve| curve.response(pressure)))
    }
    pub(crate) fn compile(&self) -> Option<CompiledPressureControl> {
        if self.is_identity() {
            return None;
        }
        let segments = self
            .curve
            .points
            .windows(2)
            .map(|p| {
                let (x0, y0) = (f64::from(p[0].0), f64::from(p[0].1));
                let (x1, y1) = (f64::from(p[1].0), f64::from(p[1].1));
                Segment {
                    x0,
                    x1,
                    y0,
                    slope: (y1 - y0) / (x1 - x0),
                }
            })
            .collect::<Vec<_>>()
            .into();
        Some(CompiledPressureControl {
            segments,
            minimum: self.minimum,
        })
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BrushDynamics {
    pub size: PressureControl,
    pub flow: PressureControl,
    pub opacity: PressureControl,
    pub pressure_opacity: bool,
    pub tip: TipDynamics,
    pub scatter: crate::ScatterDynamics,
}
impl BrushDynamics {
    pub fn is_identity(&self) -> bool {
        self.size.is_identity()
            && self.flow.is_identity()
            && !self.pressure_opacity
            && self.tip == TipDynamics::default()
            && self.scatter == crate::ScatterDynamics::default()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AngleControl {
    #[default]
    Fixed,
    Direction,
    PenRotation,
    Tilt,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TipDynamics {
    angle: AngleControl,
    tilt_roundness: bool,
    minimum_roundness: f32,
}
impl Default for TipDynamics {
    fn default() -> Self {
        Self {
            angle: AngleControl::Fixed,
            tilt_roundness: false,
            minimum_roundness: 0.01,
        }
    }
}
impl TipDynamics {
    pub fn new(angle: AngleControl, tilt_roundness: bool, minimum_roundness: f32) -> Result<Self> {
        if !minimum_roundness.is_finite() || !(0.01..=1.).contains(&minimum_roundness) {
            return Err(Error::InvalidArgument("tip minimum roundness"));
        }
        Ok(Self {
            angle,
            tilt_roundness,
            minimum_roundness,
        })
    }
    pub fn angle(self) -> AngleControl {
        self.angle
    }
    pub fn tilt_roundness(self) -> bool {
        self.tilt_roundness
    }
    pub fn minimum_roundness(self) -> f32 {
        self.minimum_roundness
    }
}
#[derive(Clone, Debug)]
struct Segment {
    x0: f64,
    x1: f64,
    y0: f64,
    slope: f64,
}
#[derive(Clone, Debug)]
pub(crate) struct CompiledPressureControl {
    segments: Arc<[Segment]>,
    minimum: f32,
}
impl CompiledPressureControl {
    /// Input has already been validated by the document/stroke boundary.
    pub(crate) fn response(&self, pressure: f32) -> f32 {
        let input = f64::from(pressure);
        let segment = self
            .segments
            .iter()
            .find(|s| input <= s.x1)
            .unwrap_or_else(|| self.segments.last().expect("bounded nonempty curve"));
        let output = (segment.y0 + (input - segment.x0) * segment.slope).clamp(0., 1.) as f32;
        (self.minimum + (1. - self.minimum) * output).clamp(0., 1.)
    }
}
