use crate::{Error, Pixel, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Pen,
    Eraser,
    Mouse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushMode {
    Paint,
    Erase,
}

/// Coordinates in document pixels; time is a device-independent monotonic nanosecond count.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputPoint {
    pub x: f64,
    pub y: f64,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub rotation: f32,
    pub tangential_pressure: f32,
    pub buttons: u32,
    pub capabilities: u32,
    pub tool: Tool,
    pub timestamp_ns: u64,
}

impl InputPoint {
    pub fn new(x: f64, y: f64, pressure: f32) -> Self {
        Self {
            x,
            y,
            pressure,
            tilt_x: 0.0,
            tilt_y: 0.0,
            rotation: 0.0,
            tangential_pressure: 0.0,
            buttons: 0,
            capabilities: 0,
            tool: Tool::Pen,
            timestamp_ns: 0,
        }
    }

    pub fn validate(self) -> Result<()> {
        if !self.x.is_finite()
            || !self.y.is_finite()
            || self.x.abs() > 2_000_000.0
            || self.y.abs() > 2_000_000.0
        {
            return Err(Error::InvalidArgument(
                "coordinates must be finite and within +/-2,000,000",
            ));
        }
        for (value, min, max) in [
            (self.pressure, 0.0, 1.0),
            (self.tilt_x, -90.0, 90.0),
            (self.tilt_y, -90.0, 90.0),
            (self.rotation, -360.0, 360.0),
            (self.tangential_pressure, -1.0, 1.0),
        ] {
            if !value.is_finite() || !(min..=max).contains(&value) {
                return Err(Error::InvalidArgument("invalid tablet channel"));
            }
        }
        Ok(())
    }

    pub(crate) fn interpolate(self, other: Self, t: f64) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * t as f32;
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            pressure: mix(self.pressure, other.pressure),
            tilt_x: mix(self.tilt_x, other.tilt_x),
            tilt_y: mix(self.tilt_y, other.tilt_y),
            rotation: mix(self.rotation, other.rotation),
            tangential_pressure: mix(self.tangential_pressure, other.tangential_pressure),
            ..other
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Brush {
    pub radius: f32,
    pub opacity: f32,
    /// Dab distance as a fraction of unpressured radius, not event frequency.
    pub spacing: f32,
    pub color: Pixel,
    pub mode: BrushMode,
}

impl Default for Brush {
    fn default() -> Self {
        Self {
            radius: 8.0,
            opacity: 1.0,
            spacing: 0.25,
            color: Pixel::from_straight([0.0, 0.0, 0.0, 1.0]).expect("constant color"),
            mode: BrushMode::Paint,
        }
    }
}

impl Brush {
    pub fn validate(self) -> Result<()> {
        for (value, min, max) in [
            (self.radius, 0.5, 256.0),
            (self.opacity, 0.0, 1.0),
            (self.spacing, 0.05, 1.0),
        ] {
            if !value.is_finite() || !(min..=max).contains(&value) {
                return Err(Error::InvalidArgument(
                    "brush radius/opacity/spacing outside supported range",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn step(self) -> f64 {
        f64::from(self.radius * self.spacing)
    }
}
