//! VBR 1.0 executable procedural subset. No arbitrary archive extraction.
use crate::{check, Error, Result};
use paint_core::{BrushDefinition, BrushProgram, BrushSettings, BrushTip};
use paint_task::CancellationToken;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Cursor, Read, Write},
    path::Path,
    sync::OnceLock,
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

pub const VBR_MAJOR: u32 = 1;
pub const VBR_MINOR: u32 = 0;
pub const MAX_JSON: u64 = 2 * 1024 * 1024;
pub const MAX_PACKAGE: u64 = 128 * 1024 * 1024;
const MAX_EXPANDED: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Parameters {
    pub diameter: f32,
    pub spacing: f32,
    pub dab_opacity: f32,
    pub hardness: f32,
    pub roundness: f32,
    pub angle: f32,
    pub flow: f32,
    pub pressure_size: bool,
    pub pressure_flow: bool,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_pressure::present"
    )]
    pub pressure_dynamics: Option<crate::brush_pressure::PressureParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_tip_dynamics::present"
    )]
    pub tip_dynamics: Option<crate::brush_tip_dynamics::TipParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_scatter::present"
    )]
    pub scatter_dynamics: Option<crate::brush_scatter::ScatterParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_shape_jitter::present"
    )]
    pub shape_jitter: Option<crate::brush_shape_jitter::ShapeParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_transfer_jitter::present"
    )]
    pub transfer_jitter: Option<crate::brush_transfer_jitter::TransferParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_color_dynamics::present"
    )]
    pub color_dynamics: Option<crate::brush_color_dynamics::ColorParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_airbrush::present"
    )]
    pub airbrush: Option<crate::brush_airbrush::AirbrushParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_speed::present"
    )]
    pub speed_dynamics: Option<crate::brush_speed::SpeedParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_tangent::present"
    )]
    pub tangent_dynamics: Option<crate::brush_tangent::TangentParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_distance::present"
    )]
    pub distance_dynamics: Option<crate::brush_distance::DistanceParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_tilt::present"
    )]
    pub tilt_dynamics: Option<crate::brush_tilt::TiltParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_rotation::present"
    )]
    pub rotation_dynamics: Option<crate::brush_rotation::RotationParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_direction::present"
    )]
    pub direction_dynamics: Option<crate::brush_direction::DirectionParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_source_matrix::present"
    )]
    pub source_matrix: Option<crate::brush_source_matrix::SourceMatrixParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_fade::present"
    )]
    pub fade_dynamics: Option<crate::brush_fade::FadeParameters>,
}
impl Parameters {
    pub fn definition(&self) -> BrushDefinition {
        self.settings().definition()
    }
    pub fn settings(&self) -> BrushSettings {
        let mut dynamics = self
            .pressure_dynamics
            .as_ref()
            .map(|p| p.core().clone())
            .unwrap_or_default();
        if let Some(tip) = self.tip_dynamics {
            dynamics.tip = tip.core();
        }
        if let Some(scatter) = self.scatter_dynamics {
            dynamics.scatter = scatter.core();
        }
        if let Some(shape) = self.shape_jitter {
            dynamics.shape_jitter = shape.core();
        }
        if let Some(jitter) = &self.transfer_jitter {
            dynamics.transfer_jitter = jitter.core();
        }
        if let Some(color) = self.color_dynamics {
            dynamics.color = color.core();
        }
        dynamics.speed = self
            .speed_dynamics
            .as_ref()
            .map(|p| p.core())
            .unwrap_or_default();
        dynamics.tangent = self
            .tangent_dynamics
            .as_ref()
            .map(|p| p.core())
            .unwrap_or_default();
        dynamics.distance = self
            .distance_dynamics
            .as_ref()
            .map(|p| p.core())
            .unwrap_or_default();
        dynamics.tilt = self
            .tilt_dynamics
            .as_ref()
            .map(|p| p.core())
            .unwrap_or_default();
        dynamics.rotation = self
            .rotation_dynamics
            .as_ref()
            .map(|p| p.core())
            .unwrap_or_default();
        dynamics.direction = self
            .direction_dynamics
            .as_ref()
            .map(|p| p.core())
            .unwrap_or_default();
        dynamics.matrix = self
            .source_matrix
            .as_ref()
            .map(|p| p.core())
            .unwrap_or_default();
        dynamics.fade = self
            .fade_dynamics
            .as_ref()
            .map(|p| p.core())
            .unwrap_or_default();
        BrushSettings {
            outline: Default::default(),
            taper: Default::default(),
            wet: Default::default(),
            smoothing: Default::default(),
            texture: Default::default(),
            dual: Default::default(),
            dynamics,
            airbrush: self.airbrush.map(|p| p.core()).unwrap_or_default(),
            mask: None,
            diameter: self.diameter,
            spacing: self.spacing,
            opacity: self.dab_opacity,
            tip: BrushTip {
                hardness: self.hardness,
                roundness: self.roundness,
                angle: self.angle,
                flow: self.flow,
                pressure_size: self.pressure_size,
                pressure_flow: self.pressure_flow,
            },
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub engine_profile: String,
    pub behavior_version: u32,
    pub required_capabilities: Vec<String>,
    pub parameters: Parameters,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tip_resource: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_texture::present"
    )]
    pub texture: Option<crate::brush_texture::TextureParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_dual::present"
    )]
    pub dual_brush: Option<crate::brush_dual::DualParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_smoothing::present"
    )]
    pub smoothing: Option<crate::brush_smoothing::SmoothingParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_wet::present"
    )]
    pub wet_mix: Option<crate::brush_wet::WetParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_taper::present"
    )]
    pub stroke_taper: Option<crate::brush_taper::TaperParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_stroke_outline::present"
    )]
    pub stroke_outline: Option<crate::brush_stroke_outline::OutlineParameters>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_resource: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Package {
    pub package_id: String,
    pub brushes: Vec<Preset>,
}
fn merge_missing(value: &mut Value, defaults: &Value) {
    let (Value::Object(value), Value::Object(defaults)) = (value, defaults) else {
        return;
    };
    let mut remove = Vec::new();
    for (key, default) in defaults {
        match value.get_mut(key) {
            Some(current) if current.is_null() => {
                if key == "parameters" || !default.is_object() {
                    *current = default.clone();
                } else {
                    // Optional feature objects written as null are absent, not
                    // an instruction to enable a canonical feature section.
                    remove.push(key.clone());
                }
            }
            Some(current) => merge_missing(current, default),
            None => {
                // Optional feature sections stay absent when the source omitted
                // them. Scalar/array configuration and the required parameters
                // object are filled so old records remain executable.
                if key == "parameters" || !default.is_object() {
                    value.insert(key.clone(), default.clone());
                }
            }
        }
    }
    for key in remove {
        value.remove(&key);
    }
}
fn canonical_preset_value() -> &'static Value {
    static CANONICAL: OnceLock<Value> = OnceLock::new();
    CANONICAL.get_or_init(|| {
        let settings = BrushSettings::default();
        let dynamics = &settings.dynamics;
        let parameters = Parameters {
            diameter: settings.diameter,
            spacing: settings.spacing,
            dab_opacity: settings.opacity,
            hardness: settings.tip.hardness,
            roundness: settings.tip.roundness,
            angle: settings.tip.angle,
            flow: settings.tip.flow,
            pressure_size: settings.tip.pressure_size,
            pressure_flow: settings.tip.pressure_flow,
            pressure_dynamics: Some(crate::brush_pressure::PressureParameters::from_core(
                dynamics,
            )),
            tip_dynamics: Some(crate::brush_tip_dynamics::TipParameters::from_core(
                dynamics.tip,
            )),
            scatter_dynamics: Some(crate::brush_scatter::ScatterParameters::from_core(
                dynamics.scatter,
            )),
            shape_jitter: Some(crate::brush_shape_jitter::ShapeParameters::from_core(
                dynamics.shape_jitter,
            )),
            transfer_jitter: Some(crate::brush_transfer_jitter::TransferParameters::from_core(
                dynamics.transfer_jitter,
            )),
            color_dynamics: Some(crate::brush_color_dynamics::ColorParameters::from_core(
                dynamics.color,
            )),
            airbrush: Some(crate::brush_airbrush::AirbrushParameters::from_core(
                paint_core::Airbrush::default(),
            )),
            speed_dynamics: Some(crate::brush_speed::SpeedParameters::from_core(
                dynamics.speed.clone(),
            )),
            tangent_dynamics: Some(crate::brush_tangent::TangentParameters::from_core(
                dynamics.tangent.clone(),
            )),
            distance_dynamics: Some(crate::brush_distance::DistanceParameters::from_core(
                dynamics.distance.clone(),
            )),
            tilt_dynamics: Some(crate::brush_tilt::TiltParameters::from_core(
                dynamics.tilt.clone(),
            )),
            rotation_dynamics: Some(crate::brush_rotation::RotationParameters::from_core(
                dynamics.rotation.clone(),
            )),
            direction_dynamics: Some(crate::brush_direction::DirectionParameters::from_core(
                dynamics.direction.clone(),
            )),
            source_matrix: Some(
                crate::brush_source_matrix::SourceMatrixParameters::from_core(dynamics.matrix),
            ),
            fade_dynamics: Some(crate::brush_fade::FadeParameters::from_core(
                dynamics.fade.clone(),
            )),
        };
        let preset = Preset {
            id: "00000000-0000-4000-8000-000000000000".into(),
            name: String::new(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 1,
            required_capabilities: Vec::new(),
            parameters,
            tip_resource: None,
            texture: Some(
                crate::brush_texture::TextureParameters::from_core(
                    &paint_core::BrushTexture::default(),
                )
                .expect("default texture"),
            ),
            dual_brush: Some(
                crate::brush_dual::DualParameters::from_core(&paint_core::DualBrush::default())
                    .expect("default dual brush"),
            ),
            smoothing: Some(crate::brush_smoothing::SmoothingParameters::from_core(
                paint_core::BrushSmoothing::default(),
            )),
            wet_mix: Some(crate::brush_wet::WetParameters::from_core(
                paint_core::WetSettings::default(),
            )),
            stroke_taper: Some(crate::brush_taper::TaperParameters::from_core(
                &paint_core::StrokeTaper::default(),
            )),
            stroke_outline: Some(
                crate::brush_stroke_outline::OutlineParameters::from_core(
                    &paint_core::StrokeOutline::default(),
                )
                .expect("default stroke outline"),
            ),
            source_resource: None,
        };
        let mut value = serde_json::to_value(preset).expect("canonical VBR preset");
        // Missing legacy fields use the original wire defaults. Never inject a
        // replay-only field into a legacy record (or invent a missing required
        // allowOverflow for an explicitly selected replay model).
        if let Some(outline) = value
            .get_mut("strokeOutline")
            .and_then(Value::as_object_mut)
        {
            outline.insert("model".into(), Value::String("stroke-contour-v1".into()));
            outline.remove("allowOverflow");
        }
        if let Some(dual) = value.get_mut("dualBrush").and_then(Value::as_object_mut) {
            for (key, entry) in [
                (
                    "pressureDynamics",
                    serde_json::to_value(Some(
                        crate::brush_pressure::PressureParameters::from_core(
                            &paint_core::BrushDynamics::default(),
                        ),
                    ))
                    .expect("default dual pressure"),
                ),
                (
                    "tipDynamics",
                    serde_json::to_value(Some(
                        crate::brush_tip_dynamics::TipParameters::from_core(
                            paint_core::TipDynamics::default(),
                        ),
                    ))
                    .expect("default dual tip dynamics"),
                ),
                (
                    "scatterDynamics",
                    serde_json::to_value(Some(crate::brush_scatter::ScatterParameters::from_core(
                        paint_core::ScatterDynamics::default(),
                    )))
                    .expect("default dual scatter"),
                ),
                (
                    "shapeJitter",
                    serde_json::to_value(Some(
                        crate::brush_shape_jitter::ShapeParameters::from_core(
                            paint_core::ShapeJitter::default(),
                        ),
                    ))
                    .expect("default dual shape"),
                ),
                (
                    "sizing",
                    serde_json::to_value(Some(
                        crate::brush_dual_sizing::SizingParameters::from_core(
                            paint_core::DualSizing::default(),
                        ),
                    ))
                    .expect("default dual sizing"),
                ),
                (
                    "color",
                    serde_json::to_value(Some(
                        crate::brush_dual_color::ColorParameters::from_core(
                            paint_core::DualColor::default(),
                        ),
                    ))
                    .expect("default dual color"),
                ),
            ] {
                dual.entry(key).or_insert(entry);
            }
        }
        normalize_canonical(&mut value);
        if let Value::Object(object) = &mut value {
            object.remove("id");
            object.remove("name");
        }
        value
    })
}
fn complete_preset(value: &mut Value) {
    let ratio = value
        .pointer("/dualBrush/sizing/model")
        .and_then(Value::as_str)
        == Some("relative-ratio-v1");
    if ratio {
        let mut defaults = canonical_preset_value().clone();
        defaults["dualBrush"]["sizing"] = serde_json::json!({"model":"relative-ratio-v1","mode":"relative","diameterRatio":1.,"widthRule":"follow-pressure"});
        merge_missing(value, &defaults);
    } else {
        merge_missing(value, canonical_preset_value());
    }
    if let Some(dual) = value.get_mut("dualBrush").and_then(Value::as_object_mut) {
        match dual.get("model").and_then(Value::as_str) {
            Some("paired-dab-v1") => {
                dual.insert("spacing".into(), Value::Null);
            }
            Some("path-coverage-v1")
                if !dual
                    .get("spacing")
                    .and_then(Value::as_f64)
                    .is_some_and(|spacing| (0.01..=1.).contains(&spacing)) =>
            {
                dual.insert("spacing".into(), 0.25.into());
            }
            _ => {}
        }
    }
}
fn normalize_canonical(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if key == "minimumRoundness" && value.as_f64().is_some_and(|number| number < 0.01) {
                    *value = 0.01.into();
                } else {
                    normalize_canonical(value);
                }
            }
        }
        Value::Array(values) => values.iter_mut().for_each(normalize_canonical),
        _ => {}
    }
}
pub(crate) fn decode_preset(bytes: &[u8]) -> Result<Preset> {
    let mut value: Value =
        serde_json::from_slice(bytes).map_err(|e| Error::Codec(format!("VBR JSON: {e}")))?;
    complete_preset(&mut value);
    let mut preset: Preset =
        serde_json::from_value(value).map_err(|e| Error::Codec(format!("VBR JSON: {e}")))?;
    preset.normalize_compatibility();
    Ok(preset)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Version {
    #[serde(default = "one")]
    major: u32,
    #[serde(default)]
    minor: u32,
}
fn one() -> u32 {
    1
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    id: String,
    path: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    format: String,
    version: Version,
    package_id: String,
    brushes: Vec<Reference>,
    #[serde(default)]
    groups: Vec<serde_json::Value>,
    #[serde(default)]
    assets: Vec<serde_json::Value>,
}
fn uuid(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == b'-'
            } else {
                c.is_ascii_digit() || (b'a'..=b'f').contains(&c)
            }
        })
        && id != "00000000-0000-0000-0000-000000000000"
}
pub(crate) fn json<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    if bytes.len() as u64 > MAX_JSON {
        return Err(Error::Limit("brush JSON bytes"));
    }
    serde_json::from_slice(bytes).map_err(|e| Error::Codec(format!("VBR JSON: {e}")))
}
pub(crate) fn encode_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(value).map_err(|e| Error::Codec(e.to_string()))?;
    if bytes.len() as u64 > MAX_JSON {
        return Err(Error::Limit("brush JSON bytes"));
    }
    Ok(bytes)
}
impl Preset {
    fn normalize_compatibility(&mut self) {
        let pressure = self.parameters.pressure_dynamics.as_ref();
        let pressure_enabled = pressure.is_some();
        let opacity = pressure.is_some_and(|p| p.has_opacity());
        let scatter = self.parameters.scatter_dynamics.is_some();
        let smooth = self.has_smooth();
        let dual_extended = self
            .dual_brush
            .as_ref()
            .is_some_and(|d| d.has_extended_mode());
        let scatter_controls = self
            .parameters
            .scatter_dynamics
            .is_some_and(|p| p.has_controls())
            || self
                .dual_brush
                .as_ref()
                .is_some_and(|d| d.has_scatter_controls());
        let mut required = Vec::new();
        if self.tip_resource.is_some() {
            required.push("image-tip");
        }
        for (active, capability) in [
            (
                self.dual_brush.as_ref().is_some_and(|d| d.has_texture()),
                "dual-tip-texture",
            ),
            (self.has_wet_dual(), "wet-dual-color"),
            (self.stroke_taper.is_some(), "stroke-taper"),
            (self.stroke_outline.is_some(), "stroke-outline"),
            (
                self.stroke_outline.as_ref().is_some_and(|p| p.is_replay()),
                "stroke-outline-replay",
            ),
            (
                self.stroke_outline.as_ref().is_some_and(|p| p.is_sdf()),
                "stroke-outline-sdf",
            ),
            (
                self.stroke_outline
                    .as_ref()
                    .is_some_and(|p| p.has_self_texture()),
                "outline-self-texture",
            ),
            (self.wet_mix.is_some(), "wet-mix"),
            (self.has_dual_color(), "dual-color-stroke"),
            (self.has_interleaved_color(), "dual-color-stamp"),
            (self.parameters.airbrush.is_some(), "airbrush"),
            (self.parameters.speed_dynamics.is_some(), "speed-dynamics"),
            (
                self.parameters.tangent_dynamics.is_some(),
                "tangent-dynamics",
            ),
            (
                self.parameters.distance_dynamics.is_some(),
                "distance-dynamics",
            ),
            (self.parameters.tilt_dynamics.is_some(), "tilt-dynamics"),
            (
                self.parameters.rotation_dynamics.is_some(),
                "rotation-dynamics",
            ),
            (
                self.parameters.direction_dynamics.is_some(),
                "direction-dynamics",
            ),
            (
                self.parameters.source_matrix.is_some(),
                "dynamics-source-matrix",
            ),
            (self.parameters.fade_dynamics.is_some(), "fade-dynamics"),
            (scatter_controls, "scatter-controls"),
            (dual_extended, "dual-cover-extended"),
            (self.parameters.transfer_jitter.is_some(), "transfer-jitter"),
            (self.has_dual_sizing(), "dual-relative-size"),
            (self.has_dual_ratio(), "dual-size-ratio"),
            (self.has_relative_taper(), "taper-relative-length"),
            (self.has_node_modes(), "curve-node-modes"),
            (self.smoothing.is_some(), "path-smoothing"),
            (smooth, "pressure-smooth"),
            (pressure_enabled, "pressure-curves"),
            (opacity, "opacity-pressure"),
            (self.parameters.tip_dynamics.is_some(), "tip-dynamics"),
            (scatter, "scatter-stamps"),
            (self.parameters.shape_jitter.is_some(), "shape-jitter"),
            (self.parameters.color_dynamics.is_some(), "color-dynamics"),
            (self.texture.is_some(), "texture-cover"),
            (self.dual_brush.is_some(), "dual-cover"),
        ] {
            if active {
                required.push(capability);
            }
        }
        let mut capabilities =
            Vec::with_capacity(self.required_capabilities.len() + required.len());
        for capability in &self.required_capabilities {
            if (capability == "image-tip" && self.tip_resource.is_none())
                || (is_feature_capability(capability)
                    && !required.iter().any(|required| required == capability))
                || capabilities.iter().any(|existing| existing == capability)
            {
                continue;
            }
            capabilities.push(capability.clone());
        }
        for capability in required {
            if !capabilities.iter().any(|existing| existing == capability) {
                capabilities.push(capability.to_owned());
            }
        }
        self.required_capabilities = capabilities;
        self.behavior_version = if self.stroke_outline.as_ref().is_some_and(|p| p.is_replay()) {
            37
        } else if self.dual_brush.as_ref().is_some_and(|d| d.has_texture()) {
            36
        } else if self.stroke_outline.as_ref().is_some_and(|p| p.is_sdf()) {
            35
        } else if self
            .stroke_outline
            .as_ref()
            .is_some_and(|p| p.has_self_texture())
        {
            34
        } else if self.stroke_outline.is_some() {
            33
        } else if self.has_node_modes() {
            32
        } else if self.has_relative_taper() {
            31
        } else if self.has_dual_ratio() {
            30
        } else if self.parameters.transfer_jitter.is_some() {
            29
        } else if dual_extended {
            28
        } else if scatter_controls {
            27
        } else if self.parameters.fade_dynamics.is_some() {
            26
        } else if self.parameters.source_matrix.is_some() {
            25
        } else if self.parameters.direction_dynamics.is_some() {
            24
        } else if self.parameters.rotation_dynamics.is_some() {
            23
        } else if self.parameters.tilt_dynamics.is_some() {
            22
        } else if self.parameters.distance_dynamics.is_some() {
            21
        } else if self.parameters.tangent_dynamics.is_some() {
            20
        } else if self.parameters.speed_dynamics.is_some() {
            19
        } else if self.parameters.airbrush.is_some() {
            18
        } else if self.has_interleaved_color() {
            17
        } else if self.has_wet_dual() {
            16
        } else if self.stroke_taper.is_some() {
            15
        } else if self.wet_mix.is_some() {
            14
        } else if self.has_dual_color() {
            13
        } else if self.has_dual_sizing() {
            12
        } else if self.smoothing.is_some() {
            11
        } else if smooth {
            10
        } else if self.dual_brush.is_some() {
            9
        } else if self.texture.is_some() {
            8
        } else if self.parameters.color_dynamics.is_some() {
            7
        } else if self.parameters.shape_jitter.is_some() {
            6
        } else if scatter {
            5
        } else if self.parameters.tip_dynamics.is_some() {
            4
        } else if opacity {
            3
        } else if pressure_enabled {
            2
        } else {
            1
        };
        if self.engine_profile == "legacy.round"
            && (self.tip_resource.is_some() || self.behavior_version > 1)
        {
            self.engine_profile = "drawverse.brush".into();
        }
    }
    pub fn has_wet_dual(&self) -> bool {
        self.wet_mix.is_some_and(|p| p.core().dual_color())
    }
    pub fn has_smooth(&self) -> bool {
        self.parameters
            .pressure_dynamics
            .as_ref()
            .is_some_and(|p| p.has_smooth())
            || self.dual_brush.as_ref().is_some_and(|d| d.has_smooth())
    }
    pub fn has_node_modes(&self) -> bool {
        self.parameters.settings().dynamics.has_node_modes()
            || self.dual_brush.as_ref().is_some_and(|p| p.has_node_modes())
            || self
                .stroke_taper
                .as_ref()
                .is_some_and(|p| p.core().has_node_modes())
    }
    pub fn has_relative_taper(&self) -> bool {
        self.stroke_taper
            .as_ref()
            .is_some_and(|p| p.core().has_relative_length())
    }
    pub fn has_dual_ratio(&self) -> bool {
        self.dual_brush
            .as_ref()
            .is_some_and(|d| d.has_ratio_sizing())
    }
    pub fn has_dual_sizing(&self) -> bool {
        self.dual_brush.as_ref().is_some_and(|d| d.has_sizing())
    }
    pub fn has_dual_color(&self) -> bool {
        self.dual_brush.as_ref().is_some_and(|d| d.has_color())
    }
    pub fn has_interleaved_color(&self) -> bool {
        self.dual_brush
            .as_ref()
            .is_some_and(|d| d.has_interleaved_color())
    }
}
fn is_feature_capability(capability: &str) -> bool {
    matches!(
        capability,
        "pressure-curves"
            | "opacity-pressure"
            | "tip-dynamics"
            | "scatter-stamps"
            | "shape-jitter"
            | "color-dynamics"
            | "texture-cover"
            | "dual-cover"
            | "pressure-smooth"
            | "path-smoothing"
            | "dual-relative-size"
            | "dual-size-ratio"
            | "taper-relative-length"
            | "curve-node-modes"
            | "dual-color-stroke"
            | "dual-color-stamp"
            | "airbrush"
            | "speed-dynamics"
            | "tangent-dynamics"
            | "distance-dynamics"
            | "tilt-dynamics"
            | "rotation-dynamics"
            | "direction-dynamics"
            | "dynamics-source-matrix"
            | "fade-dynamics"
            | "scatter-controls"
            | "dual-cover-extended"
            | "transfer-jitter"
            | "wet-mix"
            | "stroke-taper"
            | "stroke-outline"
            | "dual-tip-texture"
            | "stroke-outline-sdf"
            | "stroke-outline-replay"
            | "outline-self-texture"
            | "wet-dual-color"
    )
}
impl Package {
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        let mut value: Value =
            serde_json::from_slice(bytes).map_err(|e| Error::Codec(format!("VBR JSON: {e}")))?;
        if let Some(brushes) = value.get_mut("brushes").and_then(Value::as_array_mut) {
            for brush in brushes {
                complete_preset(brush);
            }
        }
        let mut package: Self =
            serde_json::from_value(value).map_err(|e| Error::Codec(format!("VBR JSON: {e}")))?;
        for brush in &mut package.brushes {
            brush.normalize_compatibility();
        }
        package.validate()?;
        Ok(package)
    }
    pub fn to_json(&self) -> Result<Vec<u8>> {
        self.validate()?;
        encode_json(self)
    }
    pub fn validate(&self) -> Result<()> {
        if !uuid(&self.package_id) {
            return Err(Error::Invalid("VBR package UUID"));
        }
        if self.brushes.is_empty() || self.brushes.len() > 1024 {
            return Err(Error::Limit("VBR brush count"));
        }
        let mut ids = BTreeSet::new();
        for b in &self.brushes {
            if !uuid(&b.id)
                || !ids.insert(&b.id)
                || b.name.trim().is_empty()
                || b.name.chars().count() > 80
                || b.name.contains('\0')
            {
                return Err(Error::Invalid("VBR preset identity/name"));
            }
            if b.parameters.pressure_dynamics.as_ref().is_some_and(|p| {
                p.core().tip != paint_core::TipDynamics::default()
                    || p.core().scatter != paint_core::ScatterDynamics::default()
                    || p.core().shape_jitter != paint_core::ShapeJitter::default()
                    || p.core().transfer_jitter != paint_core::TransferJitter::default()
                    || p.core().color != paint_core::ColorDynamics::default()
            }) {
                return Err(Error::Unsupported(
                    "non-pressure controls cannot be encoded in pressure payload".into(),
                ));
            }
            if let Some(d) = &b.dual_brush {
                d.validate()?;
                if d.has_interleaved_color()
                    && d.active_color()
                    && (!d.is_paired() || b.wet_mix.is_some_and(|p| p.core().enabled()))
                {
                    return Err(Error::Unsupported(
                        "interleaved dual color requires paired dry painting".into(),
                    ));
                }
                if b.wet_mix
                    .is_some_and(|p| p.core().enabled() && !p.core().dual_color())
                    && d.active_color()
                {
                    return Err(Error::Unsupported("wet dual color is not supported".into()));
                }
            }
            if let Some(t) = &b.texture {
                t.validate()?;
            }
            if !matches!(
                b.engine_profile.as_str(),
                "legacy.round" | "drawverse.brush"
            ) {
                return Err(Error::Unsupported("VBR behavior profile/version".into()));
            }
            if b.required_capabilities.len() > 32
                || b.required_capabilities.iter().any(|c| {
                    !matches!(
                        c.as_str(),
                        "procedural-tip"
                            | "image-tip"
                            | "pressure"
                            | "dab-flow"
                            | "pressure-curves"
                            | "opacity-pressure"
                            | "tip-dynamics"
                            | "scatter-stamps"
                            | "shape-jitter"
                            | "color-dynamics"
                            | "texture-cover"
                            | "dual-cover"
                            | "pressure-smooth"
                            | "path-smoothing"
                            | "dual-relative-size"
                            | "dual-size-ratio"
                            | "taper-relative-length"
                            | "curve-node-modes"
                            | "dual-color-stroke"
                            | "dual-color-stamp"
                            | "airbrush"
                            | "speed-dynamics"
                            | "tangent-dynamics"
                            | "distance-dynamics"
                            | "tilt-dynamics"
                            | "rotation-dynamics"
                            | "direction-dynamics"
                            | "dynamics-source-matrix"
                            | "fade-dynamics"
                            | "scatter-controls"
                            | "dual-cover-extended"
                            | "transfer-jitter"
                            | "wet-mix"
                            | "stroke-taper"
                            | "stroke-outline"
                            | "dual-tip-texture"
                            | "stroke-outline-sdf"
                            | "stroke-outline-replay"
                            | "outline-self-texture"
                            | "wet-dual-color"
                    )
                })
            {
                return Err(Error::Unsupported("VBR required capabilities".into()));
            }
            if b.source_resource
                .as_ref()
                .is_some_and(|id| !crate::brush_resource::valid_id(id))
            {
                return Err(Error::Invalid("VBR source identity"));
            }
            let mut normalized = b.clone();
            normalized.normalize_compatibility();
            let actual_capabilities: BTreeSet<_> = b.required_capabilities.iter().collect();
            let expected_capabilities: BTreeSet<_> =
                normalized.required_capabilities.iter().collect();
            if b.behavior_version != normalized.behavior_version
                || b.engine_profile != normalized.engine_profile
                || actual_capabilities != expected_capabilities
            {
                return Err(Error::Invalid("VBR pressure behavior/capability"));
            }
            let mut definition = b.parameters.definition();
            if let Some(p) = &b.stroke_taper {
                definition.taper = p.core().clone();
            }
            if let Some(p) = b.wet_mix {
                definition.wet = p.core();
            }
            if let Some(p) = b.smoothing {
                definition.smoothing = p.core();
            }
            if let Some(id) = &b.tip_resource {
                if !crate::brush_resource::valid_id(id) || b.engine_profile != "drawverse.brush" {
                    return Err(Error::Invalid("VBR image tip identity/profile/capability"));
                }
            }
            BrushProgram::compile(definition.clone())?;
            if b.engine_profile == "legacy.round" && definition.tip != BrushTip::default() {
                return Err(Error::Unsupported(
                    "legacy.round cannot execute advanced tip parameters".into(),
                ));
            }
        }
        Ok(())
    }
}
fn path_valid(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.contains(['\\', ':', '\0'])
        && name
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
}
type Archive<'a> = (ZipArchive<Cursor<&'a [u8]>>, BTreeSet<String>);
pub(crate) fn checked_archive<'a>(
    bytes: &'a [u8],
    token: &CancellationToken,
) -> Result<Archive<'a>> {
    check(token)?;
    if bytes.len() as u64 > MAX_PACKAGE {
        return Err(Error::Limit("VBR compressed bytes"));
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes))?;
    // Manifest + 1024 presets + 6144 images + 1024 source records + 1024 originals.
    if zip.len() > 10217 {
        return Err(Error::Limit("VBR entry count"));
    }
    let mut paths = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut expanded = 0u64;
    for i in 0..zip.len() {
        check(token)?;
        let entry = zip.by_index(i)?;
        let name = entry.name()?;
        if !path_valid(&name)
            || !paths.insert(name.to_string())
            || !names.insert(name.to_ascii_lowercase())
            || entry.is_symlink()
            || entry.encrypted()
        {
            return Err(Error::Invalid("VBR archive path/type/duplicate"));
        }
        if !matches!(
            entry.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(Error::Unsupported("VBR compression".into()));
        }
        expanded = expanded
            .checked_add(entry.size())
            .ok_or(Error::Limit("VBR expanded bytes"))?;
        if expanded > MAX_EXPANDED || entry.size() > MAX_PACKAGE {
            return Err(Error::Limit("VBR expanded entry bytes"));
        }
    }
    Ok((zip, paths))
}
pub(crate) fn read_entry(
    zip: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    limit: u64,
) -> Result<Vec<u8>> {
    let mut entry = zip.by_name(name)?;
    let expected_size = entry.size();
    let mut bytes = Vec::new();
    Read::by_ref(&mut entry)
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Error::Limit("VBR entry bytes"));
    }
    if bytes.len() as u64 != expected_size {
        return Err(Error::Invalid("VBR entry size mismatch"));
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8], token: &CancellationToken) -> Result<Package> {
    let (mut zip, paths) = checked_archive(bytes, token)?;
    let manifest: Manifest = json(&read_entry(&mut zip, "manifest.json", MAX_JSON)?)?;
    if manifest.format != "DrawVerseBrush" || manifest.version.major != 1 {
        return Err(Error::Unsupported("VBR format/version".into()));
    }
    if !manifest.assets.is_empty() || !manifest.groups.is_empty() {
        return Err(Error::Unsupported(
            "VBR assets/groups not implemented".into(),
        ));
    }
    if manifest.brushes.is_empty() || manifest.brushes.len() > 1024 {
        return Err(Error::Limit("VBR brush count"));
    }
    let mut expected = BTreeSet::from(["manifest.json".to_owned()]);
    let mut brushes = Vec::new();
    for reference in manifest.brushes {
        check(token)?;
        if !uuid(&reference.id)
            || reference.path != format!("brushes/{}.json", reference.id)
            || !expected.insert(reference.path.clone())
        {
            return Err(Error::Invalid("VBR reference"));
        }
        let preset = decode_preset(&read_entry(&mut zip, &reference.path, MAX_JSON)?)?;
        if preset.id != reference.id {
            return Err(Error::Invalid("VBR reference ID mismatch"));
        }
        brushes.push(preset);
    }
    if paths != expected {
        return Err(Error::Invalid("VBR unreferenced archive entries"));
    }
    let package = Package {
        package_id: manifest.package_id,
        brushes,
    };
    package.validate()?;
    if package.brushes.iter().any(|b| {
        b.stroke_taper.is_some()
            || b.stroke_outline.is_some()
            || b.wet_mix.is_some()
            || b.smoothing.is_some()
            || b.tip_resource.is_some()
            || b.texture.is_some()
            || b.dual_brush.is_some()
            || b.source_resource.is_some()
            || b.parameters.pressure_dynamics.is_some()
            || b.parameters.tip_dynamics.is_some()
            || b.parameters.scatter_dynamics.is_some()
            || b.parameters.shape_jitter.is_some()
            || b.parameters.color_dynamics.is_some()
            || b.parameters.airbrush.is_some()
            || b.parameters.speed_dynamics.is_some()
            || b.parameters.tangent_dynamics.is_some()
            || b.parameters.distance_dynamics.is_some()
            || b.parameters.tilt_dynamics.is_some()
            || b.parameters.rotation_dynamics.is_some()
            || b.parameters.direction_dynamics.is_some()
            || b.parameters.source_matrix.is_some()
            || b.parameters.fade_dynamics.is_some()
    }) {
        return Err(Error::Unsupported(
            "image VBR requires a resource bundle".into(),
        ));
    }
    Ok(package)
}
pub fn encode(package: &Package, token: &CancellationToken) -> Result<Vec<u8>> {
    package.validate()?;
    if package.brushes.iter().any(|b| {
        b.stroke_taper.is_some()
            || b.stroke_outline.is_some()
            || b.wet_mix.is_some()
            || b.smoothing.is_some()
            || b.tip_resource.is_some()
            || b.texture.is_some()
            || b.dual_brush.is_some()
            || b.source_resource.is_some()
            || b.parameters.pressure_dynamics.is_some()
            || b.parameters.tip_dynamics.is_some()
            || b.parameters.scatter_dynamics.is_some()
            || b.parameters.shape_jitter.is_some()
            || b.parameters.color_dynamics.is_some()
    }) {
        return Err(Error::Unsupported(
            "image VBR requires a resource bundle".into(),
        ));
    }
    check(token)?;
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let manifest = Manifest {
        format: "DrawVerseBrush".into(),
        version: Version {
            major: VBR_MAJOR,
            minor: VBR_MINOR,
        },
        package_id: package.package_id.clone(),
        brushes: package
            .brushes
            .iter()
            .map(|b| Reference {
                id: b.id.clone(),
                path: format!("brushes/{}.json", b.id),
            })
            .collect(),
        groups: Vec::new(),
        assets: Vec::new(),
    };
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("manifest.json", options)?;
    zip.write_all(&encode_json(&manifest)?)?;
    for b in &package.brushes {
        check(token)?;
        zip.start_file(format!("brushes/{}.json", b.id), options)?;
        zip.write_all(&encode_json(b)?)?;
    }
    Ok(zip.finish()?.into_inner())
}
pub fn load(path: &Path, token: &CancellationToken) -> Result<Package> {
    let mut file = File::open(path)?;
    if file.metadata()?.len() > MAX_PACKAGE {
        return Err(Error::Limit("VBR compressed bytes"));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_PACKAGE + 1)
        .read_to_end(&mut bytes)?;
    decode(&bytes, token)
}
pub fn save(path: &Path, package: &Package, token: &CancellationToken) -> Result<()> {
    let bytes = encode(package, token)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::Builder::new()
        .prefix(".drawverse-brush-")
        .tempfile_in(parent)?;
    temporary.write_all(&bytes)?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    check(token)?;
    temporary.persist(path).map_err(|e| Error::Io(e.error))?;
    Ok(())
}
