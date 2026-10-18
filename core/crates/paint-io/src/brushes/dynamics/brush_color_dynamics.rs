use paint_core::ColorDynamics;
use serde::{Deserialize, Serialize};
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    model: &'static str,
    enabled: bool,
    foreground_background: f32,
    hue: f32,
    saturation: f32,
    value: f32,
    per_tip: bool,
    seed: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input {
    model: String,
    enabled: bool,
    foreground_background: f64,
    hue: f64,
    saturation: f64,
    value: f64,
    per_tip: bool,
    seed: String,
}
#[derive(Clone, Copy, Debug)]
pub struct ColorParameters {
    color: ColorDynamics,
}
impl ColorParameters {
    pub fn from_core(color: ColorDynamics) -> Self {
        Self { color }
    }
    pub fn core(self) -> ColorDynamics {
        self.color
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<ColorParameters>, D::Error> {
    ColorParameters::deserialize(d).map(Some)
}
impl Serialize for ColorParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let c = self.color;
        Data {
            model: "linear-hsv-v1",
            enabled: c.enabled(),
            foreground_background: c.foreground_background(),
            hue: c.hue(),
            saturation: c.saturation(),
            value: c.value(),
            per_tip: c.per_tip(),
            seed: format!("{:016x}", c.seed()),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for ColorParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let data = Input::deserialize(d)?;
        if data.model != "linear-hsv-v1" {
            return Err(serde::de::Error::custom("unsupported color model"));
        }
        if [
            data.foreground_background,
            data.hue,
            data.saturation,
            data.value,
        ]
        .iter()
        .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
        {
            return Err(serde::de::Error::custom("color dynamics bounds"));
        }
        if data.seed.len() != 16
            || !data
                .seed
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(serde::de::Error::custom(
                "color seed must be 16 lowercase hex digits",
            ));
        }
        let seed = u64::from_str_radix(&data.seed, 16).map_err(serde::de::Error::custom)?;
        ColorDynamics::new(
            data.enabled,
            data.foreground_background as f32,
            data.hue as f32,
            data.saturation as f32,
            data.value as f32,
            data.per_tip,
            seed,
        )
        .map(Self::from_core)
        .map_err(serde::de::Error::custom)
    }
}
