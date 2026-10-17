//! Strict separated-stroke dual-color configuration. Tool colors are never serialized.
use paint_core::{BlendMode, DualColor, DualColorModel, DualColorOrder};
use serde::{Deserialize, Serialize};
pub const BLEND_NAMES: [&str; 27] = [
    "normal",
    "dissolve",
    "darken",
    "multiply",
    "color-burn",
    "linear-burn",
    "darker-color",
    "lighten",
    "screen",
    "color-dodge",
    "linear-dodge",
    "lighter-color",
    "overlay",
    "soft-light",
    "hard-light",
    "vivid-light",
    "linear-light",
    "pin-light",
    "hard-mix",
    "difference",
    "exclusion",
    "subtract",
    "divide",
    "hue",
    "saturation",
    "color",
    "luminosity",
];
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    model: String,
    enabled: bool,
    use_background: bool,
    overflow: bool,
    order: String,
    primary_blend: String,
    secondary_blend: String,
}
#[derive(Clone, Copy, Debug)]
pub struct ColorParameters {
    settings: DualColor,
}
impl ColorParameters {
    pub fn from_core(settings: DualColor) -> Self {
        Self { settings }
    }
    pub fn core(self) -> DualColor {
        self.settings
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<ColorParameters>, D::Error> {
    ColorParameters::deserialize(d).map(Some)
}
impl Serialize for ColorParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Data {
            model: match self.settings.model() {
                DualColorModel::SeparatedStroke => "separated-stroke-v1",
                DualColorModel::InterleavedStamp => "interleaved-stamp-v1",
            }
            .into(),
            enabled: self.settings.enabled(),
            use_background: self.settings.use_background(),
            overflow: self.settings.overflow(),
            order: match self.settings.order() {
                DualColorOrder::SecondBehind => "second-behind",
                DualColorOrder::SecondInFront => "second-in-front",
            }
            .into(),
            primary_blend: BLEND_NAMES[self.settings.primary_blend() as usize].into(),
            secondary_blend: BLEND_NAMES[self.settings.secondary_blend() as usize].into(),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for ColorParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let data = Data::deserialize(d)?;
        let model = match data.model.as_str() {
            "separated-stroke-v1" => DualColorModel::SeparatedStroke,
            "interleaved-stamp-v1" => DualColorModel::InterleavedStamp,
            _ => return Err(serde::de::Error::custom("dual-color rendering model")),
        };
        let order = match data.order.as_str() {
            "second-behind" => DualColorOrder::SecondBehind,
            "second-in-front" => DualColorOrder::SecondInFront,
            _ => return Err(serde::de::Error::custom("dual-color order")),
        };
        let mode = |name: &str| -> Result<BlendMode, D::Error> {
            BLEND_NAMES
                .iter()
                .position(|v| *v == name)
                .and_then(|i| BlendMode::from_id(i as u32).ok())
                .ok_or_else(|| serde::de::Error::custom("dual-color blend mode"))
        };
        let settings = DualColor::new_with_model(
            model,
            data.enabled,
            data.use_background,
            data.overflow,
            order,
            mode(&data.primary_blend)?,
            mode(&data.secondary_blend)?,
        )
        .map_err(serde::de::Error::custom)?;
        Ok(Self { settings })
    }
}
