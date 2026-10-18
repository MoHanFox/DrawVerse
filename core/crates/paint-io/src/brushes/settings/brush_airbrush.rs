use paint_core::Airbrush;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    model: String,
    enabled: bool,
    interval_ms: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct AirbrushParameters {
    core: Airbrush,
}

impl AirbrushParameters {
    pub fn from_core(core: Airbrush) -> Self {
        Self { core }
    }
    pub fn core(self) -> Airbrush {
        self.core
    }
}

pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<AirbrushParameters>, D::Error> {
    AirbrushParameters::deserialize(d).map(Some)
}

impl Serialize for AirbrushParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Data {
            model: "time-interval-v1".into(),
            enabled: self.core.enabled(),
            interval_ms: f64::from(self.core.interval_ms()),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for AirbrushParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let data = Data::deserialize(d)?;
        if data.model != "time-interval-v1" {
            return Err(serde::de::Error::custom("airbrush model"));
        }
        let core = Airbrush::new(data.enabled, data.interval_ms as f32)
            .map_err(serde::de::Error::custom)?;
        Ok(Self { core })
    }
}
