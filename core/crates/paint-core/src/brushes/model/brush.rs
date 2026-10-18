pub const MAX_BRUSH_RADIUS: f32 = 1000.;
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
    Smudge {
        all_layers: bool,
        finger_painting: bool,
        pressure_strength: bool,
    },
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

    /// Normalized tilt magnitude (0..1); devices without tilt report 0.
    pub fn tilt_magnitude(self) -> f32 {
        if self.capabilities & 2 == 0 {
            return 0.;
        }
        (self.tilt_x.hypot(self.tilt_y) / 90.).clamp(0., 1.)
    }

    /// Normalized pen rotation (0..1); devices without rotation report 0.
    pub fn rotation_unit(self) -> f32 {
        if self.capabilities & 4 == 0 {
            return 0.;
        }
        (self.rotation.rem_euclid(360.) / 360.).clamp(0., 1.)
    }

    pub(crate) fn interpolate(self, other: Self, t: f64) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * t as f32;
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            pressure: mix(self.pressure, other.pressure),
            tilt_x: if self.capabilities & other.capabilities & 2 != 0 {
                mix(self.tilt_x, other.tilt_x)
            } else {
                other.tilt_x
            },
            tilt_y: if self.capabilities & other.capabilities & 2 != 0 {
                mix(self.tilt_y, other.tilt_y)
            } else {
                other.tilt_y
            },
            rotation: if self.capabilities & other.capabilities & 4 != 0 {
                let delta = (other.rotation - self.rotation + 180.).rem_euclid(360.) - 180.;
                (self.rotation + delta * t as f32 + 180.).rem_euclid(360.) - 180.
            } else {
                other.rotation
            },
            tangential_pressure: mix(self.tangential_pressure, other.tangential_pressure),
            ..other
        }
    }
}

#[cfg(test)]
mod device_interpolation_tests {
    use super::*;
    #[test]
    fn shortest_rotation_and_missing_channels_have_explicit_fallbacks() {
        let mut a = InputPoint::new(0., 0., 1.);
        let mut b = InputPoint::new(8., 0., 1.);
        a.capabilities = 4;
        b.capabilities = 4;
        a.rotation = 350.;
        b.rotation = 10.;
        assert!(a.interpolate(b, 0.5).rotation.abs() < 1e-6);
        a.capabilities = 0;
        b.capabilities = 6;
        b.rotation = 90.;
        b.tilt_x = 60.;
        b.tilt_y = 30.;
        let mid = a.interpolate(b, 0.5);
        assert_eq!(mid.rotation, 90.);
        assert_eq!(mid.tilt_x, 60.);
        assert_eq!(mid.tilt_y, 30.);
        a.capabilities = 2;
        a.tilt_x = 0.;
        a.tilt_y = 0.;
        let mid = a.interpolate(b, 0.5);
        assert_eq!(mid.tilt_x, 30.);
        assert_eq!(mid.tilt_y, 15.);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Brush {
    pub radius: f32,
    pub opacity: f32,
    /// Dab distance as a fraction of unpressured radius, not event frequency.
    pub spacing: f32,
    pub color: Pixel,
    pub background_color: Pixel,
    pub mode: BrushMode,
}

impl Default for Brush {
    fn default() -> Self {
        Self {
            radius: 8.0,
            opacity: 1.0,
            spacing: 0.25,
            color: Pixel::from_straight([0.0, 0.0, 0.0, 1.0]).expect("constant color"),
            background_color: Pixel::WHITE,
            mode: BrushMode::Paint,
        }
    }
}

impl Brush {
    pub fn validate(self) -> Result<()> {
        for (value, min, max) in [
            (self.radius, 0.5, MAX_BRUSH_RADIUS),
            (self.opacity, 0.0, 1.0),
            (self.spacing, 0.02, 1.0),
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
