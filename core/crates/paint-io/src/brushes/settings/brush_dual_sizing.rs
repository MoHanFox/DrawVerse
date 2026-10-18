//! Strict, versioned secondary diameter sizing parameters.
use paint_core::{DualSizeMode, DualSizing, DualWidthRule};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    model: String,
    mode: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "read_number"
    )]
    diameter_offset: Option<f64>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "read_number"
    )]
    diameter_ratio: Option<f64>,
    width_rule: String,
}
fn read_number<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<f64>, D::Error> {
    f64::deserialize(d).map(Some)
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
            model: if self.settings.diameter_ratio().is_some() {
                "relative-ratio-v1"
            } else {
                "relative-size-v1"
            }
            .into(),
            mode: match self.settings.mode() {
                DualSizeMode::Independent => "independent",
                DualSizeMode::Relative => "relative",
            }
            .into(),
            diameter_offset: self
                .settings
                .diameter_ratio()
                .is_none()
                .then(|| f64::from(self.settings.diameter_offset())),
            diameter_ratio: self
                .settings
                .diameter_ratio()
                .map(|v| f64::from(v).max(0.0005)),
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
        let settings = match (
            data.model.as_str(),
            data.diameter_offset,
            data.diameter_ratio,
        ) {
            ("relative-size-v1", Some(offset), None)
                if offset.is_finite() && (-511. ..=511.).contains(&offset) =>
            {
                DualSizing::new(mode, offset as f32, rule)
            }
            ("relative-ratio-v1", None, Some(ratio))
                if ratio.is_finite() && (0.0005..=2000.).contains(&ratio) =>
            {
                DualSizing::new_ratio(mode, ratio as f32, rule)
            }
            _ => return Err(serde::de::Error::custom("secondary sizing model/range")),
        }
        .map_err(serde::de::Error::custom)?;
        Ok(Self { settings })
    }
}
