use crate::{Error, Result};

/// Time-based airbrush deposition. The interval is independent of input event
/// frequency; the UI or FFI caller supplies stationary ticks at this cadence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Airbrush {
    enabled: bool,
    interval_ms: f32,
}

impl Default for Airbrush {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_ms: 50.,
        }
    }
}

impl Airbrush {
    pub fn new(enabled: bool, interval_ms: f32) -> Result<Self> {
        if !interval_ms.is_finite() || !(1. ..=1000.).contains(&interval_ms) {
            return Err(Error::InvalidArgument("airbrush interval"));
        }
        Ok(Self {
            enabled,
            interval_ms,
        })
    }
    pub fn enabled(self) -> bool {
        self.enabled
    }
    pub fn interval_ms(self) -> f32 {
        self.interval_ms
    }
    pub fn interval_ns(self) -> u64 {
        (self.interval_ms as f64 * 1_000_000.).round() as u64
    }
    pub fn is_default(self) -> bool {
        self == Self::default()
    }
}
