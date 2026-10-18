//! Complete whole-stroke outline settings with strict pre-rounding validation.
use crate::{brush_dual::DualParameters, brush_texture::TextureParameters, Error, Result};
use paint_core::{
    BrushMask, DualBrush, DualMode, OutlinePlacement, OutlineSource, OutlineWidth, StrokeOutline,
    TextureResource, STROKE_OUTLINE_MODEL,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Width {
    relative: bool,
    pixels: f64,
    percent: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "model", rename_all = "kebab-case", deny_unknown_fields)]
enum Source {
    SelfBrush,
    ExistingBrush { brush: Box<DualParameters> },
    Texture { texture: TextureParameters },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    model: String,
    enabled: bool,
    placement: String,
    width: Width,
    source: Source,
    secondary_color: bool,
}

#[derive(Clone, Debug)]
pub struct OutlineParameters {
    data: Data,
}

impl OutlineParameters {
    pub fn from_core(outline: &StrokeOutline) -> Result<Self> {
        outline.validate()?;
        let source = match outline.source() {
            OutlineSource::SelfBrush => Source::SelfBrush,
            OutlineSource::Existing(tip) => Source::ExistingBrush {
                brush: Box::new(DualParameters::from_core(&DualBrush::new(
                    false,
                    DualMode::Multiply,
                    tip.as_ref().clone(),
                ))?),
            },
            OutlineSource::Texture(texture) => Source::Texture {
                texture: TextureParameters::from_core(texture)?,
            },
        };
        let data = Data {
            model: STROKE_OUTLINE_MODEL.into(),
            enabled: outline.enabled(),
            placement: match outline.placement() {
                OutlinePlacement::Outside => "outside",
                OutlinePlacement::Center => "center",
                OutlinePlacement::Inside => "inside",
            }
            .into(),
            width: Width {
                relative: outline.width().relative(),
                pixels: f64::from(outline.width().pixels()),
                percent: f64::from(outline.width().percent()),
            },
            source,
            secondary_color: outline.secondary_color(),
        };
        let parameters = Self { data };
        parameters.validate()?;
        Ok(parameters)
    }
    pub fn enabled(&self) -> bool {
        self.data.enabled
    }
    pub fn existing_resource_id(&self) -> Option<&str> {
        match &self.data.source {
            Source::ExistingBrush { brush } => brush.resource_id(),
            _ => None,
        }
    }
    pub fn texture_resource_id(&self) -> Option<&str> {
        match &self.data.source {
            Source::Texture { texture } => texture.resource_id(),
            _ => None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.data.model != STROKE_OUTLINE_MODEL
            || !matches!(
                self.data.placement.as_str(),
                "outside" | "center" | "inside"
            )
            || !(0. ..=f64::from(paint_core::MAX_STROKE_OUTLINE_WIDTH))
                .contains(&self.data.width.pixels)
            || !(0. ..=10_000.).contains(&self.data.width.percent)
            || [self.data.width.pixels, self.data.width.percent]
                .iter()
                .any(|value| !value.is_finite())
        {
            return Err(Error::Invalid("stroke outline model/placement/width"));
        }
        OutlineWidth::new(
            self.data.width.relative,
            self.data.width.pixels as f32,
            self.data.width.percent as f32,
        )?;
        match &self.data.source {
            Source::SelfBrush => {}
            Source::ExistingBrush { brush } => brush.validate()?,
            Source::Texture { texture } => {
                texture.validate()?;
                if self.data.enabled && !texture.enabled() {
                    return Err(Error::Invalid("enabled outline requires texture"));
                }
            }
        }
        Ok(())
    }
    pub fn bind(
        self,
        existing: Option<Arc<BrushMask>>,
        texture: Option<Arc<TextureResource>>,
    ) -> Result<StrokeOutline> {
        self.validate()?;
        let source = match self.data.source {
            Source::SelfBrush => {
                if existing.is_some() || texture.is_some() {
                    return Err(Error::Invalid("self outline resource binding"));
                }
                OutlineSource::SelfBrush
            }
            Source::ExistingBrush { brush } => {
                if texture.is_some() {
                    return Err(Error::Invalid("existing outline resource binding"));
                }
                let bound = (*brush).bind(existing)?;
                OutlineSource::Existing(Arc::new(bound.secondary().clone()))
            }
            Source::Texture {
                texture: parameters,
            } => {
                if existing.is_some() {
                    return Err(Error::Invalid("texture outline resource binding"));
                }
                OutlineSource::Texture(parameters.bind(texture)?)
            }
        };
        let placement = match self.data.placement.as_str() {
            "outside" => OutlinePlacement::Outside,
            "center" => OutlinePlacement::Center,
            "inside" => OutlinePlacement::Inside,
            _ => return Err(Error::Invalid("stroke outline placement")),
        };
        StrokeOutline::new(
            self.data.enabled,
            placement,
            OutlineWidth::new(
                self.data.width.relative,
                self.data.width.pixels as f32,
                self.data.width.percent as f32,
            )?,
            source,
            self.data.secondary_color,
        )
        .map_err(Error::Core)
    }
}

impl Serialize for OutlineParameters {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.data.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OutlineParameters {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let parameters = Self {
            data: Data::deserialize(deserializer)?,
        };
        parameters.validate().map_err(serde::de::Error::custom)?;
        Ok(parameters)
    }
}

pub fn present<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<OutlineParameters>, D::Error> {
    OutlineParameters::deserialize(deserializer).map(Some)
}
