//! Validated portable time-grid smoothing. No Qt or serialization in the kernel.
use paint_core::BrushSmoothing;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    model: String,
    enabled: bool,
    position_ms: f64,
    pressure_ms: f64,
    speed_response: f64,
    stabilizer_px: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct SmoothingParameters {
    settings: BrushSmoothing,
}
impl SmoothingParameters {
    pub fn from_core(settings: BrushSmoothing) -> Self {
        Self { settings }
    }
    pub fn core(self) -> BrushSmoothing {
        self.settings
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<SmoothingParameters>, D::Error> {
    SmoothingParameters::deserialize(d).map(Some)
}
impl Serialize for SmoothingParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        Data {
            model: "time-grid-v1".into(),
            enabled: self.settings.enabled(),
            position_ms: self.settings.position_ms(),
            pressure_ms: self.settings.pressure_ms(),
            speed_response: self.settings.speed_response(),
            stabilizer_px: self.settings.stabilizer_px(),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for SmoothingParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let data = Data::deserialize(d)?;
        if data.model != "time-grid-v1" {
            return Err(serde::de::Error::custom("smoothing model"));
        }
        let settings = BrushSmoothing::new(
            data.enabled,
            data.position_ms,
            data.pressure_ms,
            data.speed_response,
            data.stabilizer_px,
        )
        .map_err(serde::de::Error::custom)?;
        Ok(Self { settings })
    }
}
