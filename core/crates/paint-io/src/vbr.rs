//! VBR 1.0 executable procedural subset. No arbitrary archive extraction.
use crate::{check, Error, Result};
use paint_core::{BrushDefinition, BrushProgram, BrushSettings, BrushTip};
use paint_task::CancellationToken;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Cursor, Read, Write},
    path::Path,
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

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
        BrushSettings {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_resource: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Package {
    pub package_id: String,
    pub brushes: Vec<Preset>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Version {
    major: u32,
    minor: u32,
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
    groups: Vec<serde_json::Value>,
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
impl Package {
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        let package: Self = json(bytes)?;
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
                    || p.core().color != paint_core::ColorDynamics::default()
            }) {
                return Err(Error::Unsupported(
                    "non-pressure controls cannot be encoded in pressure payload".into(),
                ));
            }
            let pressure = b.parameters.pressure_dynamics.is_some();
            let tip_dynamic = b.parameters.tip_dynamics.is_some();
            let scatter = b.parameters.scatter_dynamics.is_some();
            let shape = b.parameters.shape_jitter.is_some();
            let color = b.parameters.color_dynamics.is_some();
            let airbrush = b.parameters.airbrush.is_some();
            let speed = b.parameters.speed_dynamics.is_some();
            let tangent = b.parameters.tangent_dynamics.is_some();
            let textured = b.texture.is_some();
            let dual = b.dual_brush.is_some();
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
            let opacity = b
                .parameters
                .pressure_dynamics
                .as_ref()
                .is_some_and(|p| p.has_opacity());
            if b.behavior_version
                != if tangent {
                    20
                } else if speed {
                    19
                } else if airbrush {
                    18
                } else if b.has_interleaved_color() {
                    17
                } else if b.has_wet_dual() {
                    16
                } else if b.stroke_taper.is_some() {
                    15
                } else if b.wet_mix.is_some() {
                    14
                } else if b.has_dual_color() {
                    13
                } else if b.has_dual_sizing() {
                    12
                } else if b.smoothing.is_some() {
                    11
                } else if b.has_smooth() {
                    10
                } else if dual {
                    9
                } else if textured {
                    8
                } else if color {
                    7
                } else if shape {
                    6
                } else if scatter {
                    5
                } else if tip_dynamic {
                    4
                } else if opacity {
                    3
                } else if pressure {
                    2
                } else {
                    1
                }
                || !matches!(
                    b.engine_profile.as_str(),
                    "legacy.round" | "drawverse.brush"
                )
            {
                return Err(Error::Unsupported("VBR behavior profile/version".into()));
            }
            if b.required_capabilities.len() > 23
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
                            | "dual-color-stroke"
                            | "dual-color-stamp"
                            | "airbrush"
                            | "speed-dynamics"
                            | "tangent-dynamics"
                            | "wet-mix"
                            | "stroke-taper"
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
            if b.has_wet_dual()
                != b.required_capabilities
                    .iter()
                    .any(|c| c == "wet-dual-color")
                || (b.has_wet_dual() && b.engine_profile != "drawverse.brush")
                || b.stroke_taper.is_some()
                    != b.required_capabilities.iter().any(|c| c == "stroke-taper")
                || (b.stroke_taper.is_some() && b.engine_profile != "drawverse.brush")
                || b.wet_mix.is_some() != b.required_capabilities.iter().any(|c| c == "wet-mix")
                || (b.wet_mix.is_some() && b.engine_profile != "drawverse.brush")
                || b.has_dual_color()
                    != b.required_capabilities
                        .iter()
                        .any(|c| c == "dual-color-stroke")
                || (b.has_dual_color() && b.engine_profile != "drawverse.brush")
                || b.has_interleaved_color()
                    != b.required_capabilities
                        .iter()
                        .any(|c| c == "dual-color-stamp")
                || (b.has_interleaved_color() && b.engine_profile != "drawverse.brush")
                || airbrush != b.required_capabilities.iter().any(|c| c == "airbrush")
                || (airbrush && b.engine_profile != "drawverse.brush")
                || speed
                    != b.required_capabilities
                        .iter()
                        .any(|c| c == "speed-dynamics")
                || (speed && b.engine_profile != "drawverse.brush")
                || tangent
                    != b.required_capabilities
                        .iter()
                        .any(|c| c == "tangent-dynamics")
                || (tangent && b.engine_profile != "drawverse.brush")
                || (b.has_dual_sizing()
                    && (b.engine_profile != "drawverse.brush"
                        || !b
                            .required_capabilities
                            .iter()
                            .any(|c| c == "dual-relative-size")))
                || b.smoothing.is_some()
                    != b.required_capabilities
                        .iter()
                        .any(|c| c == "path-smoothing")
                || (b.smoothing.is_some() && b.engine_profile != "drawverse.brush")
                || b.has_smooth()
                    != b.required_capabilities
                        .iter()
                        .any(|c| c == "pressure-smooth")
                || (b.has_smooth() && b.engine_profile != "drawverse.brush")
                || pressure
                    != b.required_capabilities
                        .iter()
                        .any(|c| c == "pressure-curves")
                || (pressure && b.engine_profile != "drawverse.brush")
                || opacity
                    != b.required_capabilities
                        .iter()
                        .any(|c| c == "opacity-pressure")
                || tip_dynamic != b.required_capabilities.iter().any(|c| c == "tip-dynamics")
                || (tip_dynamic && b.engine_profile != "drawverse.brush")
                || scatter
                    != b.required_capabilities
                        .iter()
                        .any(|c| c == "scatter-stamps")
                || (scatter && b.engine_profile != "drawverse.brush")
                || shape != b.required_capabilities.iter().any(|c| c == "shape-jitter")
                || (shape && b.engine_profile != "drawverse.brush")
                || color
                    != b.required_capabilities
                        .iter()
                        .any(|c| c == "color-dynamics")
                || (color && b.engine_profile != "drawverse.brush")
                || textured != b.required_capabilities.iter().any(|c| c == "texture-cover")
                || (textured && b.engine_profile != "drawverse.brush")
                || dual != b.required_capabilities.iter().any(|c| c == "dual-cover")
                || (dual && b.engine_profile != "drawverse.brush")
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
                if !crate::brush_resource::valid_id(id)
                    || b.engine_profile != "drawverse.brush"
                    || !b.required_capabilities.iter().any(|c| c == "image-tip")
                {
                    return Err(Error::Invalid("VBR image tip identity/profile/capability"));
                }
            } else if b.required_capabilities.iter().any(|c| c == "image-tip") {
                return Err(Error::Invalid("VBR missing image tip reference"));
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
    // Manifest + 1024 presets + 4096 tip/texture/secondary/baseline images + 1024 records + 1024 originals.
    if zip.len() > 7169 {
        return Err(Error::Limit("VBR entry count"));
    }
    let mut paths = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut expanded = 0u64;
    for i in 0..zip.len() {
        check(token)?;
        let entry = zip.by_index(i)?;
        let name = entry.name();
        if !path_valid(name)
            || !paths.insert(name.to_owned())
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
    if manifest.format != "DrawVerseBrush"
        || manifest.version.major != 1
        || manifest.version.minor != 0
    {
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
        let preset: Preset = json(&read_entry(&mut zip, &reference.path, MAX_JSON)?)?;
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
    Ok(package)
}
pub fn encode(package: &Package, token: &CancellationToken) -> Result<Vec<u8>> {
    package.validate()?;
    if package.brushes.iter().any(|b| {
        b.stroke_taper.is_some()
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
        version: Version { major: 1, minor: 0 },
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
