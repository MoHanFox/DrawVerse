use crate::{Error, Result};
use std::sync::{Arc, OnceLock};

/// Immutable, bounded pressure response. X is input pressure; Y is output ratio.
#[derive(Clone, Debug, PartialEq)]
pub struct PressureCurve {
    points: Arc<[(f32, f32)]>,
    smooth: bool,
}
impl Default for PressureCurve {
    fn default() -> Self {
        static LINEAR: OnceLock<Arc<[(f32, f32)]>> = OnceLock::new();
        Self {
            points: LINEAR
                .get_or_init(|| Arc::from([(0., 0.), (1., 1.)]))
                .clone(),
            smooth: false,
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
            smooth: false,
        })
    }
    /// Shape-preserving C1 cubic interpolation; legacy `new` remains piecewise linear.
    pub fn new_smooth(points: Vec<(f32, f32)>) -> Result<Self> {
        let mut curve = Self::new(points)?;
        curve.smooth = true;
        Ok(curve)
    }
    pub fn is_smooth(&self) -> bool {
        self.smooth
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
    pub fn samples(&self, count: usize) -> Result<Vec<f32>> {
        if !(2..=1025).contains(&count) {
            return Err(Error::InvalidArgument("pressure sample count"));
        }
        let compiled = self.compile();
        Ok((0..count)
            .map(|i| {
                let p = i as f32 / (count - 1) as f32;
                compiled.as_ref().map_or(p, |c| c.response(p))
            })
            .collect())
    }
    pub(crate) fn compile(&self) -> Option<CompiledPressureControl> {
        if self.is_identity() {
            return None;
        }
        let points = self.curve.points();
        let h: Vec<_> = points
            .windows(2)
            .map(|p| f64::from(p[1].0) - f64::from(p[0].0))
            .collect();
        let slopes: Vec<_> = points
            .windows(2)
            .zip(&h)
            .map(|(p, h)| (f64::from(p[1].1) - f64::from(p[0].1)) / h)
            .collect();
        let mut tangents = vec![0.; points.len()];
        if self.curve.smooth {
            if points.len() == 2 {
                tangents.fill(slopes[0]);
            } else {
                fn endpoint(h0: f64, h1: f64, d0: f64, d1: f64) -> f64 {
                    let m = ((2. * h0 + h1) * d0 - h0 * d1) / (h0 + h1);
                    if m.signum() != d0.signum() {
                        0.
                    } else if d0.signum() != d1.signum() && m.abs() > 3. * d0.abs() {
                        3. * d0
                    } else {
                        m
                    }
                }
                tangents[0] = endpoint(h[0], h[1], slopes[0], slopes[1]);
                let n = h.len();
                tangents[n] = endpoint(h[n - 1], h[n - 2], slopes[n - 1], slopes[n - 2]);
                for i in 1..n {
                    let (a, b) = (slopes[i - 1], slopes[i]);
                    if a != 0. && b != 0. && a.signum() == b.signum() {
                        let w1 = 2. * h[i] + h[i - 1];
                        let w2 = h[i] + 2. * h[i - 1];
                        tangents[i] = (w1 + w2) / (w1 / a + w2 / b);
                    }
                }
            }
        }
        let segments = points
            .windows(2)
            .enumerate()
            .map(|(i, p)| {
                let (y0, y1) = (f64::from(p[0].1), f64::from(p[1].1));
                let delta = y1 - y0;
                let (b, c, d) = if self.curve.smooth {
                    let b = tangents[i] * h[i];
                    let end = tangents[i + 1] * h[i];
                    (b, 3. * delta - 2. * b - end, b + end - 2. * delta)
                } else {
                    (delta, 0., 0.)
                };
                Segment {
                    x0: f64::from(p[0].0),
                    x1: f64::from(p[1].0),
                    y0,
                    y1,
                    b,
                    c,
                    d,
                    linear: !self.curve.smooth,
                    slope: slopes[i],
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
    pub shape_jitter: crate::ShapeJitter,
    pub color: crate::ColorDynamics,
    pub speed: SpeedDynamics,
}
impl BrushDynamics {
    pub fn is_identity(&self) -> bool {
        self.size.is_identity()
            && self.flow.is_identity()
            && !self.pressure_opacity
            && self.tip == TipDynamics::default()
            && self.scatter == crate::ScatterDynamics::default()
            && self.shape_jitter == crate::ShapeJitter::default()
            && self.color == crate::ColorDynamics::default()
            && self.speed.is_default()
    }
}

/// Optional speed-driven size, flow and opacity response. Input is normalized
/// against `max_speed`; identity curves leave that channel unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct SpeedDynamics {
    enabled: bool,
    max_speed: f32,
    size: PressureControl,
    flow: PressureControl,
    opacity: PressureControl,
}
impl Default for SpeedDynamics {
    fn default() -> Self {
        Self {
            enabled: false,
            max_speed: 2000.,
            size: PressureControl::default(),
            flow: PressureControl::default(),
            opacity: PressureControl::default(),
        }
    }
}
impl SpeedDynamics {
    pub fn new(
        enabled: bool,
        max_speed: f32,
        size: PressureControl,
        flow: PressureControl,
        opacity: PressureControl,
    ) -> Result<Self> {
        if !max_speed.is_finite() || !(1. ..=100_000.).contains(&max_speed) {
            return Err(Error::InvalidArgument("speed maximum"));
        }
        Ok(Self {
            enabled,
            max_speed,
            size,
            flow,
            opacity,
        })
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn max_speed(&self) -> f32 {
        self.max_speed
    }
    pub fn size(&self) -> &PressureControl {
        &self.size
    }
    pub fn flow(&self) -> &PressureControl {
        &self.flow
    }
    pub fn opacity(&self) -> &PressureControl {
        &self.opacity
    }
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
    pub(crate) fn compile(&self) -> Option<CompiledSpeedDynamics> {
        if !self.enabled {
            return None;
        }
        Some(CompiledSpeedDynamics {
            max_speed: self.max_speed,
            size: self.size.compile(),
            flow: self.flow.compile(),
            opacity: self.opacity.compile(),
        })
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
    y1: f64,
    linear: bool,
    slope: f64,
    b: f64,
    c: f64,
    d: f64,
}
#[derive(Clone, Debug)]
pub(crate) struct CompiledPressureControl {
    segments: Arc<[Segment]>,
    minimum: f32,
}
#[derive(Clone, Debug)]
pub(crate) struct CompiledSpeedDynamics {
    max_speed: f32,
    size: Option<CompiledPressureControl>,
    flow: Option<CompiledPressureControl>,
    opacity: Option<CompiledPressureControl>,
}
impl CompiledSpeedDynamics {
    pub(crate) fn factors(&self, speed: f32) -> (f32, f32, f32) {
        if !speed.is_finite() || speed <= 0. {
            return (1., 1., 1.);
        }
        let input = (speed / self.max_speed).clamp(0., 1.);
        let response = |control: &Option<CompiledPressureControl>| {
            control.as_ref().map_or(1., |c| c.response(input))
        };
        (
            response(&self.size),
            response(&self.flow),
            response(&self.opacity),
        )
    }
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
        let t = ((input - segment.x0) / (segment.x1 - segment.x0)).clamp(0., 1.);
        let output = if segment.linear {
            (segment.y0 + (input - segment.x0) * segment.slope).clamp(0., 1.)
        } else if t == 0. {
            segment.y0
        } else if t == 1. {
            segment.y1
        } else {
            (((segment.d * t + segment.c) * t + segment.b) * t + segment.y0)
                .clamp(segment.y0.min(segment.y1), segment.y0.max(segment.y1))
        } as f32;
        (self.minimum + (1. - self.minimum) * output).clamp(0., 1.)
    }
}
