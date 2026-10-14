use paint_core::ShapeJitter;
use serde::{Deserialize, Serialize};
#[derive(Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    enabled: bool,
    size: f32,
    minimum_size: f32,
    angle: f32,
    roundness: f32,
    minimum_roundness: f32,
    seed: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input {
    enabled: bool,
    size: f64,
    minimum_size: f64,
    angle: f64,
    roundness: f64,
    minimum_roundness: f64,
    seed: String,
}
#[derive(Clone, Copy, Debug)]
pub struct ShapeParameters {
    shape: ShapeJitter,
}
impl ShapeParameters {
    pub fn from_core(shape: ShapeJitter) -> Self {
        Self { shape }
    }
    pub fn core(self) -> ShapeJitter {
        self.shape
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<ShapeParameters>, D::Error> {
    ShapeParameters::deserialize(d).map(Some)
}
impl Serialize for ShapeParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let c = self.shape;
        Data {
            enabled: c.enabled(),
            size: c.size(),
            minimum_size: c.minimum_size(),
            angle: c.angle(),
            roundness: c.roundness(),
            minimum_roundness: c.minimum_roundness(),
            seed: format!("{:016x}", c.seed()),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for ShapeParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let data = Input::deserialize(d)?;
        if [data.size, data.minimum_size, data.angle, data.roundness]
            .iter()
            .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
            || !data.minimum_roundness.is_finite()
            || !(0.01..=1.).contains(&data.minimum_roundness)
        {
            return Err(serde::de::Error::custom("shape jitter bounds"));
        }
        if data.seed.len() != 16
            || !data
                .seed
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(serde::de::Error::custom(
                "shape seed must be 16 lowercase hex digits",
            ));
        }
        let seed = u64::from_str_radix(&data.seed, 16).map_err(serde::de::Error::custom)?;
        ShapeJitter::new(
            data.enabled,
            data.size as f32,
            data.minimum_size as f32,
            data.angle as f32,
            data.roundness as f32,
            data.minimum_roundness as f32,
            seed,
        )
        .map(Self::from_core)
        .map_err(serde::de::Error::custom)
    }
}
