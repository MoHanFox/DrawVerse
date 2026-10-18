use paint_core::TransferJitter;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    model: String,
    size: f32,
    flow: f32,
    opacity: f32,
}
#[derive(Clone, Debug)]
pub struct TransferParameters {
    core: TransferJitter,
}
impl TransferParameters {
    pub fn from_core(core: TransferJitter) -> Self {
        Self { core }
    }
    pub fn core(&self) -> TransferJitter {
        self.core
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<TransferParameters>, D::Error> {
    TransferParameters::deserialize(d).map(Some)
}
impl Serialize for TransferParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Data {
            model: "transfer-jitter-v1".into(),
            size: self.core.size(),
            flow: self.core.flow(),
            opacity: self.core.opacity(),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for TransferParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let data = Data::deserialize(d)?;
        if data.model != "transfer-jitter-v1" {
            return Err(serde::de::Error::custom("transfer jitter model"));
        }
        TransferJitter::new(data.size, data.flow, data.opacity)
            .map(|core| Self { core })
            .map_err(serde::de::Error::custom)
    }
}
