use crate::brush_random::{mix, unit};
use crate::{Error, InputPoint, Result};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScatterControlSource {
    #[default]
    Off,
    Fade,
    Pressure,
    Tilt,
    Direction,
    Rotation,
}

/// Linear device response for one scatter channel. `minimum` is the value at
/// zero input; the response reaches one at full input. Fade uses stamp count
/// instead of a device channel.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScatterControl {
    source: ScatterControlSource,
    minimum: f32,
    fade_steps: u32,
}

impl ScatterControl {
    pub fn off() -> Self {
        Self::default()
    }
    pub fn new(source: ScatterControlSource, minimum: f32, fade_steps: u32) -> Result<Self> {
        if !minimum.is_finite() || !(0. ..=1.).contains(&minimum) {
            return Err(Error::InvalidArgument("scatter control minimum"));
        }
        if source == ScatterControlSource::Off {
            if minimum != 0. || fade_steps != 0 {
                return Err(Error::InvalidArgument("scatter control off values"));
            }
        } else if source == ScatterControlSource::Fade {
            if fade_steps == 0 || fade_steps > 1_000_000 {
                return Err(Error::InvalidArgument("scatter fade steps"));
            }
        } else if fade_steps != 0 {
            return Err(Error::InvalidArgument("scatter control fade steps"));
        }
        Ok(Self {
            source,
            minimum,
            fade_steps,
        })
    }
    pub fn source(self) -> ScatterControlSource {
        self.source
    }
    pub fn minimum(self) -> f32 {
        self.minimum
    }
    pub fn fade_steps(self) -> u32 {
        self.fade_steps
    }
    pub fn is_default(self) -> bool {
        self == Self::default()
    }
    fn factor(self, point: InputPoint, fade_index: u32, direction: Option<f64>) -> f32 {
        let input = match self.source {
            ScatterControlSource::Off => return 1.,
            ScatterControlSource::Fade => {
                if self.fade_steps == 0 {
                    return 1.;
                }
                return 1.
                    - (1. - self.minimum)
                        * (fade_index as f32 / self.fade_steps as f32).clamp(0., 1.);
            }
            ScatterControlSource::Pressure => point.pressure,
            ScatterControlSource::Tilt => point.tilt_magnitude(),
            ScatterControlSource::Direction => direction.map_or(0., |angle| {
                (angle.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU) as f32
            }),
            ScatterControlSource::Rotation => point.rotation_unit(),
        };
        (self.minimum + (1. - self.minimum) * input.clamp(0., 1.)).clamp(0., 1.)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScatterDynamics {
    enabled: bool,
    amount: f32,
    both_axes: bool,
    count: u32,
    count_jitter: f32,
    seed: u64,
    amount_control: ScatterControl,
    count_control: ScatterControl,
}
impl Default for ScatterDynamics {
    fn default() -> Self {
        Self {
            enabled: false,
            amount: 0.,
            both_axes: false,
            count: 1,
            count_jitter: 0.,
            seed: 0,
            amount_control: ScatterControl::off(),
            count_control: ScatterControl::off(),
        }
    }
}
impl ScatterDynamics {
    pub fn new(
        enabled: bool,
        amount: f32,
        both_axes: bool,
        count: u32,
        count_jitter: f32,
        seed: u64,
    ) -> Result<Self> {
        if !amount.is_finite()
            || !(0. ..=10.).contains(&amount)
            || !(1..=16).contains(&count)
            || !count_jitter.is_finite()
            || !(0. ..=1.).contains(&count_jitter)
        {
            return Err(Error::InvalidArgument("scatter bounds"));
        }
        Ok(Self {
            enabled,
            amount,
            both_axes,
            count,
            count_jitter,
            seed,
            amount_control: ScatterControl::off(),
            count_control: ScatterControl::off(),
        })
    }
    pub fn with_controls(
        mut self,
        amount_control: ScatterControl,
        count_control: ScatterControl,
    ) -> Result<Self> {
        self.amount_control = amount_control;
        self.count_control = count_control;
        Ok(self)
    }
    pub fn enabled(self) -> bool {
        self.enabled
    }
    pub fn amount(self) -> f32 {
        self.amount
    }
    pub fn both_axes(self) -> bool {
        self.both_axes
    }
    pub fn count(self) -> u32 {
        self.count
    }
    pub fn count_jitter(self) -> f32 {
        self.count_jitter
    }
    pub fn seed(self) -> u64 {
        self.seed
    }
    pub fn amount_control(self) -> ScatterControl {
        self.amount_control
    }
    pub fn count_control(self) -> ScatterControl {
        self.count_control
    }
    pub fn has_controls(self) -> bool {
        !self.amount_control.is_default() || !self.count_control.is_default()
    }
    pub(crate) fn maximum_count(self) -> usize {
        if self.enabled {
            self.count as usize
        } else {
            1
        }
    }
    pub(crate) fn plan(
        self,
        radius: f32,
        stroke: u64,
        index: u64,
        direction: Option<f64>,
        point: InputPoint,
        fade_index: u32,
    ) -> Option<ScatterPlan> {
        if !self.enabled {
            return None;
        }
        let key = mix(self.seed ^ mix(stroke) ^ mix(index.wrapping_add(0x9e3779b97f4a7c15)));
        let controlled_count = ((self.count as f32
            * self.count_control.factor(point, fade_index, direction))
        .round() as u32)
            .clamp(1, self.count);
        let floor = ((controlled_count as f32 * (1. - self.count_jitter)).ceil() as u32).max(1);
        let count =
            floor + (unit(key, 0, 0) * f64::from(controlled_count - floor + 1)).floor() as u32;
        let (sine, cosine) = direction.unwrap_or(0.).sin_cos();
        Some(ScatterPlan {
            key,
            count,
            extent: f64::from(radius)
                * 2.
                * f64::from(self.amount)
                * f64::from(self.amount_control.factor(point, fade_index, direction)),
            both_axes: self.both_axes,
            sine,
            cosine,
        })
    }
}
pub(crate) struct ScatterPlan {
    key: u64,
    pub count: u32,
    extent: f64,
    both_axes: bool,
    sine: f64,
    cosine: f64,
}
impl ScatterPlan {
    pub fn point(&self, index: u32, point: InputPoint) -> InputPoint {
        let a = (unit(self.key, index, 1) * 2. - 1.) * self.extent;
        let (dx, dy) = if self.both_axes {
            (a, (unit(self.key, index, 2) * 2. - 1.) * self.extent)
        } else {
            (-self.sine * a, self.cosine * a)
        };
        InputPoint {
            x: point.x + dx,
            y: point.y + dy,
            ..point
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(pressure: f32) -> InputPoint {
        InputPoint::new(12., 18., pressure)
    }

    #[test]
    fn controls_are_off_by_default_and_preserve_legacy_plans() {
        let scatter = ScatterDynamics::new(true, 2., true, 4, 0.25, 7).unwrap();
        assert!(!scatter.has_controls());
        let plan = scatter.plan(8., 11, 3, Some(0.5), point(0.37), 0).unwrap();
        assert!((1..=4).contains(&plan.count));
        assert!(scatter.amount_control().is_default());
        assert!(scatter.count_control().is_default());
    }

    #[test]
    fn pressure_scales_amount_and_count_with_a_minimum() {
        let scatter = ScatterDynamics::new(true, 2., true, 8, 0., 7)
            .unwrap()
            .with_controls(
                ScatterControl::new(ScatterControlSource::Pressure, 0.25, 0).unwrap(),
                ScatterControl::new(ScatterControlSource::Pressure, 0., 0).unwrap(),
            )
            .unwrap();
        let low = scatter.plan(10., 11, 3, None, point(0.), 0).unwrap();
        let high = scatter.plan(10., 11, 3, None, point(1.), 0).unwrap();
        assert!(low.extent < high.extent);
        assert_eq!(low.count, 1);
        assert_eq!(high.count, 8);
    }

    #[test]
    fn fade_reaches_the_minimum_and_clamps() {
        let scatter = ScatterDynamics::new(true, 2., false, 4, 0., 7)
            .unwrap()
            .with_controls(
                ScatterControl::new(ScatterControlSource::Fade, 0.2, 4).unwrap(),
                ScatterControl::off(),
            )
            .unwrap();
        let first = scatter.plan(10., 11, 3, None, point(1.), 0).unwrap();
        let middle = scatter.plan(10., 11, 3, None, point(1.), 2).unwrap();
        let end = scatter.plan(10., 11, 3, None, point(1.), 4).unwrap();
        let after = scatter.plan(10., 11, 3, None, point(1.), 99).unwrap();
        assert!(first.extent > middle.extent);
        assert!(middle.extent > end.extent);
        assert_eq!(end.extent.to_bits(), after.extent.to_bits());
        assert!((end.extent - first.extent * 0.2).abs() < 1e-5);
    }

    #[test]
    fn tilt_direction_and_rotation_use_device_channels() {
        for (source, sample) in [
            (
                ScatterControlSource::Tilt,
                InputPoint {
                    tilt_x: 45.,
                    tilt_y: 0.,
                    capabilities: 2,
                    ..point(1.)
                },
            ),
            (ScatterControlSource::Direction, point(1.)),
            (
                ScatterControlSource::Rotation,
                InputPoint {
                    rotation: 180.,
                    capabilities: 4,
                    ..point(1.)
                },
            ),
        ] {
            let scatter = ScatterDynamics::new(true, 1., false, 4, 0., 7)
                .unwrap()
                .with_controls(
                    ScatterControl::new(source, 0., 0).unwrap(),
                    ScatterControl::off(),
                )
                .unwrap();
            let plan = scatter.plan(8., 11, 3, Some(0.), sample, 0).unwrap();
            assert!(plan.count >= 1 && plan.count <= 4);
            assert!(plan.extent.is_finite());
        }
    }

    #[test]
    fn invalid_controls_reject() {
        for (source, minimum, steps) in [
            (ScatterControlSource::Pressure, -0.1, 0),
            (ScatterControlSource::Pressure, 1.1, 0),
            (ScatterControlSource::Pressure, 0., 1),
            (ScatterControlSource::Fade, 0., 0),
            (ScatterControlSource::Off, 0.1, 0),
        ] {
            assert!(ScatterControl::new(source, minimum, steps).is_err());
        }
        let disabled = ScatterDynamics::new(false, 1., false, 1, 0., 0)
            .unwrap()
            .with_controls(
                ScatterControl::new(ScatterControlSource::Pressure, 0., 0).unwrap(),
                ScatterControl::off(),
            )
            .unwrap();
        assert!(!disabled.enabled());
        assert!(disabled.has_controls());
    }
}
