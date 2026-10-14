use crate::{Error, InputPoint, Result};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScatterDynamics {
    enabled: bool,
    amount: f32,
    both_axes: bool,
    count: u32,
    count_jitter: f32,
    seed: u64,
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
        })
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
    ) -> Option<ScatterPlan> {
        if !self.enabled {
            return None;
        }
        let key = mix(self.seed ^ mix(stroke) ^ mix(index.wrapping_add(0x9e3779b97f4a7c15)));
        let floor = ((self.count as f32 * (1. - self.count_jitter)).ceil() as u32).max(1);
        let count = floor + (unit(key, 0, 0) * f64::from(self.count - floor + 1)).floor() as u32;
        let (sine, cosine) = direction.unwrap_or(0.).sin_cos();
        Some(ScatterPlan {
            key,
            count,
            extent: f64::from(radius) * 2. * f64::from(self.amount),
            both_axes: self.both_axes,
            sine,
            cosine,
        })
    }
}
fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}
fn unit(key: u64, index: u32, lane: u64) -> f64 {
    let value = mix(key ^ mix(u64::from(index)) ^ mix(lane.wrapping_add(0xd1b54a32d192ed03)));
    ((value >> 40) as u32) as f64 / 16777216.
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
