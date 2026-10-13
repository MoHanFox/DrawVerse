//! Portable validated geometry controls, separate from pressure curves.
use paint_core::{AngleControl, TipDynamics};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Angle {
    Fixed,
    Direction,
    PenRotation,
    Tilt,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    angle_control: Angle,
    tilt_roundness: bool,
    minimum_roundness: f32,
}
#[derive(Clone, Copy, Debug)]
pub struct TipParameters {
    tip: TipDynamics,
}
impl TipParameters {
    pub fn from_core(tip: TipDynamics) -> Self {
        Self { tip }
    }
    pub fn core(self) -> TipDynamics {
        self.tip
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<TipParameters>, D::Error> {
    TipParameters::deserialize(d).map(Some)
}
impl Serialize for TipParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        Data {
            angle_control: match self.tip.angle() {
                AngleControl::Fixed => Angle::Fixed,
                AngleControl::Direction => Angle::Direction,
                AngleControl::PenRotation => Angle::PenRotation,
                AngleControl::Tilt => Angle::Tilt,
            },
            tilt_roundness: self.tip.tilt_roundness(),
            minimum_roundness: self.tip.minimum_roundness(),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for TipParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let data = Data::deserialize(d)?;
        let angle = match data.angle_control {
            Angle::Fixed => AngleControl::Fixed,
            Angle::Direction => AngleControl::Direction,
            Angle::PenRotation => AngleControl::PenRotation,
            Angle::Tilt => AngleControl::Tilt,
        };
        TipDynamics::new(angle, data.tilt_roundness, data.minimum_roundness)
            .map(Self::from_core)
            .map_err(serde::de::Error::custom)
    }
}
