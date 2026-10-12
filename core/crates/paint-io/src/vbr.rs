//! VBR 1.0 executable procedural subset. No arbitrary archive extraction.
use crate::{check, Error, Result};
use paint_core::{Brush, BrushDefinition, BrushProgram, BrushTip};
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
}
impl Parameters {
    pub fn definition(&self) -> BrushDefinition {
        BrushDefinition {
            brush: Brush {
                radius: self.diameter / 2.,
                spacing: self.spacing * 2.,
                opacity: self.dab_opacity,
                ..Brush::default()
            },
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
fn json<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    if bytes.len() as u64 > MAX_JSON {
        return Err(Error::Limit("brush JSON bytes"));
    }
    serde_json::from_slice(bytes).map_err(|e| Error::Codec(format!("VBR JSON: {e}")))
}
fn encode_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(value).map_err(|e| Error::Codec(e.to_string()))?;
    if bytes.len() as u64 > MAX_JSON {
        return Err(Error::Limit("brush JSON bytes"));
    }
    Ok(bytes)
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
            if b.behavior_version != 1
                || !matches!(
                    b.engine_profile.as_str(),
                    "legacy.round" | "drawverse.brush"
                )
            {
                return Err(Error::Unsupported("VBR behavior profile/version".into()));
            }
            if b.required_capabilities.len() > 16
                || b.required_capabilities
                    .iter()
                    .any(|c| !matches!(c.as_str(), "procedural-tip" | "pressure" | "dab-flow"))
            {
                return Err(Error::Unsupported("VBR required capabilities".into()));
            }
            let definition = b.parameters.definition();
            BrushProgram::compile(definition)?;
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
pub fn decode(bytes: &[u8], token: &CancellationToken) -> Result<Package> {
    check(token)?;
    if bytes.len() as u64 > MAX_PACKAGE {
        return Err(Error::Limit("VBR compressed bytes"));
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes))?;
    if zip.len() > 4096 {
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
        if expanded > MAX_EXPANDED || entry.size() > MAX_JSON {
            return Err(Error::Limit("VBR expanded entry bytes"));
        }
    }
    fn read(zip: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<Vec<u8>> {
        let mut entry = zip.by_name(name)?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut entry)
            .take(MAX_JSON + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_JSON {
            return Err(Error::Limit("VBR JSON bytes"));
        }
        Ok(bytes)
    }
    let manifest: Manifest = json(&read(&mut zip, "manifest.json")?)?;
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
        let preset: Preset = json(&read(&mut zip, &reference.path)?)?;
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
    Ok(package)
}
pub fn encode(package: &Package, token: &CancellationToken) -> Result<Vec<u8>> {
    package.validate()?;
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
