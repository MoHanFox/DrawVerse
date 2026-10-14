use paint_core::ScatterDynamics;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    enabled: bool,
    amount: f32,
    both_axes: bool,
    count: u32,
    count_jitter: f32,
    seed: String,
}
#[derive(Clone, Copy, Debug)]
pub struct ScatterParameters {
    scatter: ScatterDynamics,
}
impl ScatterParameters {
    pub fn from_core(scatter: ScatterDynamics) -> Self {
        Self { scatter }
    }
    pub fn core(self) -> ScatterDynamics {
        self.scatter
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<ScatterParameters>, D::Error> {
    ScatterParameters::deserialize(d).map(Some)
}
impl Serialize for ScatterParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let c = self.scatter;
        Data {
            enabled: c.enabled(),
            amount: c.amount(),
            both_axes: c.both_axes(),
            count: c.count(),
            count_jitter: c.count_jitter(),
            seed: format!("{:016x}", c.seed()),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for ScatterParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let data = Data::deserialize(d)?;
        if data.seed.len() != 16
            || !data
                .seed
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(serde::de::Error::custom(
                "scatter seed must be 16 lowercase hex digits",
            ));
        }
        let seed = u64::from_str_radix(&data.seed, 16).map_err(serde::de::Error::custom)?;
        ScatterDynamics::new(
            data.enabled,
            data.amount,
            data.both_axes,
            data.count,
            data.count_jitter,
            seed,
        )
        .map(Self::from_core)
        .map_err(serde::de::Error::custom)
    }
}
