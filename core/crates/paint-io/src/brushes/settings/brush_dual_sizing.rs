//! Strict, versioned secondary diameter sizing parameters.
use paint_core::{DualSizeMode, DualSizing, DualWidthRule};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    model: String,
    mode: String,
    diameter_offset: f64,
    width_rule: String,
}
#[derive(Clone, Copy, Debug)]
pub struct SizingParameters {
    settings: DualSizing,
}
impl SizingParameters {
    pub fn from_core(settings: DualSizing) -> Self {
        Self { settings }
    }
    pub fn core(self) -> DualSizing {
        self.settings
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<SizingParameters>, D::Error> {
    SizingParameters::deserialize(d).map(Some)
}
impl Serialize for SizingParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Data {
            model: "relative-size-v1".into(),
            mode: match self.settings.mode() {
                DualSizeMode::Independent => "independent",
                DualSizeMode::Relative => "relative",
            }
            .into(),
            diameter_offset: f64::from(self.settings.diameter_offset()),
            width_rule: match self.settings.width_rule() {
                DualWidthRule::FollowPressure => "follow-pressure",
                DualWidthRule::FixedWidth => "fixed-width",
            }
            .into(),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for SizingParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let data = Data::deserialize(d)?;
        if data.model != "relative-size-v1"
            || !data.diameter_offset.is_finite()
            || !(-511. ..=511.).contains(&data.diameter_offset)
        {
            return Err(serde::de::Error::custom("secondary sizing model/range"));
        }
        let mode = match data.mode.as_str() {
            "independent" => DualSizeMode::Independent,
            "relative" => DualSizeMode::Relative,
            _ => return Err(serde::de::Error::custom("secondary sizing mode")),
        };
        let rule = match data.width_rule.as_str() {
            "follow-pressure" => DualWidthRule::FollowPressure,
            "fixed-width" => DualWidthRule::FixedWidth,
            _ => return Err(serde::de::Error::custom("secondary sizing pressure rule")),
        };
        Ok(Self {
            settings: DualSizing::new(mode, data.diameter_offset as f32, rule)
                .map_err(serde::de::Error::custom)?,
        })
    }
}
