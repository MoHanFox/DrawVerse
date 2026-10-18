use crate::{brush_resource::valid_id, Error, Result};
use paint_core::{BrushTexture, TextureMode, TextureResource, TextureSettings, TextureSpace};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
fn default_mode() -> String {
    "normal".into()
}
fn is_default_adjustment(value: &f64) -> bool {
    *value == 0.
}
fn is_false(value: &bool) -> bool {
    !*value
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextureParameters {
    model: String,
    enabled: bool,
    space: String,
    #[serde(default = "default_mode")]
    mode: String,
    scale: f64,
    angle: f64,
    depth: f64,
    #[serde(default, skip_serializing_if = "is_default_adjustment")]
    brightness: f64,
    #[serde(default, skip_serializing_if = "is_default_adjustment")]
    contrast: f64,
    #[serde(default, skip_serializing_if = "is_default_adjustment")]
    minimum_depth: f64,
    #[serde(default, skip_serializing_if = "is_default_adjustment")]
    depth_jitter: f64,
    #[serde(default, skip_serializing_if = "is_false")]
    depth_pressure: bool,
    invert: bool,
    offset_x: f64,
    offset_y: f64,
    #[serde(deserialize_with = "read_resource")]
    resource: Option<String>,
}
impl TextureParameters {
    pub fn from_core(t: &BrushTexture) -> Result<Self> {
        let s = t.settings();
        let resource = t
            .resource()
            .map(|r| {
                r.source()
                    .resource_id()
                    .map(str::to_owned)
                    .ok_or(Error::Unsupported(
                        "texture has no persistent identity".into(),
                    ))
            })
            .transpose()?;
        Ok(Self {
            model: "periodic-coverage-v1".into(),
            enabled: t.enabled(),
            space: match s.space() {
                TextureSpace::Document => "document",
                TextureSpace::Tip => "tip",
            }
            .into(),
            mode: match s.mode() {
                TextureMode::Normal => "normal",
                TextureMode::Dissolve => "dissolve",
                TextureMode::Multiply => "multiply",
                TextureMode::Darken => "darken",
                TextureMode::Lighten => "lighten",
                TextureMode::Subtract => "subtract",
                TextureMode::Difference => "difference",
                TextureMode::Screen => "screen",
                TextureMode::ColorBurn => "colorBurn",
                TextureMode::LinearBurn => "linearBurn",
                TextureMode::ColorDodge => "colorDodge",
                TextureMode::LinearDodge => "linearDodge",
                TextureMode::Overlay => "overlay",
                TextureMode::SoftLight => "softLight",
                TextureMode::HardLight => "hardLight",
                TextureMode::Exclusion => "exclusion",
                TextureMode::HardMix => "hardMix",
                TextureMode::VividLight => "vividLight",
                TextureMode::LinearLight => "linearLight",
                TextureMode::PinLight => "pinLight",
                TextureMode::Divide => "divide",
                TextureMode::Height => "height",
                TextureMode::LinearHeight => "linearHeight",
            }
            .into(),
            scale: f64::from(s.scale()).max(0.01),
            angle: f64::from(s.angle()),
            depth: f64::from(s.depth()),
            brightness: f64::from(s.brightness()),
            contrast: f64::from(s.contrast()),
            minimum_depth: f64::from(s.minimum_depth()),
            depth_jitter: f64::from(s.depth_jitter()),
            depth_pressure: s.depth_pressure(),
            invert: s.invert(),
            offset_x: s.offset().0,
            offset_y: s.offset().1,
            resource,
        })
    }
    pub fn resource_id(&self) -> Option<&str> {
        self.resource.as_deref()
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn mode(&self) -> &str {
        &self.mode
    }
    pub fn requires_vbr_26(&self) -> bool {
        self.brightness != 0. || self.contrast != 0.
    }
    pub fn requires_vbr_27(&self) -> bool {
        self.minimum_depth != 0. || self.depth_jitter != 0. || self.depth_pressure
    }
    pub fn requires_vbr_30(&self) -> bool {
        matches!(
            self.mode.as_str(),
            "linearBurn"
                | "colorDodge"
                | "linearDodge"
                | "overlay"
                | "softLight"
                | "hardLight"
                | "exclusion"
                | "hardMix"
        )
    }
    pub fn requires_vbr_31(&self) -> bool {
        matches!(
            self.mode.as_str(),
            "vividLight" | "linearLight" | "pinLight" | "divide" | "height" | "linearHeight"
        )
    }
    pub fn requires_vbr_39(&self) -> bool {
        self.mode == "dissolve"
    }
    pub fn validate(&self) -> Result<()> {
        if self.model != "periodic-coverage-v1"
            || !matches!(self.space.as_str(), "document" | "tip")
            || !matches!(
                self.mode.as_str(),
                "normal"
                    | "dissolve"
                    | "multiply"
                    | "darken"
                    | "lighten"
                    | "subtract"
                    | "difference"
                    | "screen"
                    | "colorBurn"
                    | "linearBurn"
                    | "colorDodge"
                    | "linearDodge"
                    | "overlay"
                    | "softLight"
                    | "hardLight"
                    | "exclusion"
                    | "hardMix"
                    | "vividLight"
                    | "linearLight"
                    | "pinLight"
                    | "divide"
                    | "height"
                    | "linearHeight"
            )
            || !self.scale.is_finite()
            || !(0.01..=100.).contains(&self.scale)
            || !self.angle.is_finite()
            || !(-180. ..=180.).contains(&self.angle)
            || !self.depth.is_finite()
            || !(0. ..=1.).contains(&self.depth)
            || !self.brightness.is_finite()
            || !(-1. ..=1.).contains(&self.brightness)
            || !self.contrast.is_finite()
            || !(-1. ..=1.).contains(&self.contrast)
            || !self.minimum_depth.is_finite()
            || !(0. ..=1.).contains(&self.minimum_depth)
            || !self.depth_jitter.is_finite()
            || !(0. ..=1.).contains(&self.depth_jitter)
            || [self.offset_x, self.offset_y]
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
            || self.resource.as_ref().is_some_and(|id| !valid_id(id))
            || (self.enabled && self.resource.is_none())
        {
            return Err(Error::Invalid("texture parameters"));
        }
        Ok(())
    }
    pub fn bind(&self, resource: Option<Arc<TextureResource>>) -> Result<BrushTexture> {
        self.validate()?;
        if self.resource.is_some() != resource.is_some() {
            return Err(Error::Invalid("texture resource binding"));
        }
        if let Some(r) = &resource {
            if r.source().resource_id() != self.resource_id() {
                return Err(Error::Invalid("texture resource identity"));
            }
        }
        let mode = match self.mode.as_str() {
            "dissolve" => TextureMode::Dissolve,
            "multiply" => TextureMode::Multiply,
            "darken" => TextureMode::Darken,
            "lighten" => TextureMode::Lighten,
            "subtract" => TextureMode::Subtract,
            "difference" => TextureMode::Difference,
            "screen" => TextureMode::Screen,
            "colorBurn" => TextureMode::ColorBurn,
            "linearBurn" => TextureMode::LinearBurn,
            "colorDodge" => TextureMode::ColorDodge,
            "linearDodge" => TextureMode::LinearDodge,
            "overlay" => TextureMode::Overlay,
            "softLight" => TextureMode::SoftLight,
            "hardLight" => TextureMode::HardLight,
            "exclusion" => TextureMode::Exclusion,
            "hardMix" => TextureMode::HardMix,
            "vividLight" => TextureMode::VividLight,
            "linearLight" => TextureMode::LinearLight,
            "pinLight" => TextureMode::PinLight,
            "divide" => TextureMode::Divide,
            "height" => TextureMode::Height,
            "linearHeight" => TextureMode::LinearHeight,
            _ => TextureMode::Normal,
        };
        let settings = TextureSettings::new(
            if self.space == "document" {
                TextureSpace::Document
            } else {
                TextureSpace::Tip
            },
            self.scale as f32,
            self.angle as f32,
            self.depth as f32,
            self.invert,
            self.offset_x,
            self.offset_y,
        )?
        .with_mode(mode)
        .with_adjustments(self.brightness as f32, self.contrast as f32)?
        .with_depth_dynamics(
            self.minimum_depth as f32,
            self.depth_jitter as f32,
            self.depth_pressure,
        )?;
        Ok(BrushTexture::new(self.enabled, settings, resource)?)
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<TextureParameters>, D::Error> {
    let t = TextureParameters::deserialize(d)?;
    t.validate().map_err(serde::de::Error::custom)?;
    Ok(Some(t))
}

fn read_resource<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)
}
