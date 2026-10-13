//! VBR 1.1 portable picture tips. No file extraction or catalogue mutation.
use crate::{
    brush_resource::{decode_verified, valid_id},
    check,
    vbr::{self, Package, Preset, MAX_JSON, MAX_PACKAGE},
    Error, Result,
};
use paint_core::{BrushDefinition, BrushMask, BrushProgram, MAX_CATALOG_MASK_BYTES};
use paint_task::CancellationToken;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{Cursor, Read, Write},
    path::Path,
    sync::Arc,
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

pub struct ImageAsset {
    bytes: Vec<u8>,
    mask: Arc<BrushMask>,
}
impl ImageAsset {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn mask(&self) -> Arc<BrushMask> {
        self.mask.clone()
    }
}
pub struct ImageBundle {
    pub package: Package,
    assets: BTreeMap<String, ImageAsset>,
}
impl ImageBundle {
    pub fn new(
        package: Package,
        encoded: BTreeMap<String, Vec<u8>>,
        token: &CancellationToken,
    ) -> Result<Self> {
        package.validate()?;
        let referenced: BTreeSet<_> = package
            .brushes
            .iter()
            .filter_map(|b| b.tip_resource.as_ref())
            .cloned()
            .collect();
        if referenced != encoded.keys().cloned().collect() {
            return Err(Error::Invalid("VBR missing/unreferenced resource"));
        }
        if encoded.values().map(|b| b.len() as u64).sum::<u64>() > 256 * 1024 * 1024 {
            return Err(Error::Limit("VBR resource bytes"));
        }
        let mut assets = BTreeMap::new();
        let mut resident = 0;
        for (id, bytes) in encoded {
            check(token)?;
            let resource = decode_verified(&id, &bytes, token)?;
            resident += resource.mask.resident_bytes();
            if resident > MAX_CATALOG_MASK_BYTES {
                return Err(Error::Limit("VBR decoded resource budget"));
            }
            assets.insert(
                id,
                ImageAsset {
                    bytes,
                    mask: resource.mask,
                },
            );
        }
        let bundle = Self { package, assets };
        bundle.validate()?;
        Ok(bundle)
    }
    pub fn assets(&self) -> &BTreeMap<String, ImageAsset> {
        &self.assets
    }
    pub fn definition(&self, index: usize) -> Result<BrushDefinition> {
        let preset = self
            .package
            .brushes
            .get(index)
            .ok_or(Error::Invalid("VBR brush index"))?;
        let mut definition = preset.parameters.definition();
        if let Some(id) = &preset.tip_resource {
            definition.mask = Some(
                self.assets
                    .get(id)
                    .ok_or(Error::Invalid("VBR missing tip resource"))?
                    .mask(),
            );
        }
        Ok(definition)
    }
    pub fn validate(&self) -> Result<()> {
        self.package.validate()?;
        let referenced: BTreeSet<_> = self
            .package
            .brushes
            .iter()
            .filter_map(|b| b.tip_resource.as_ref())
            .collect();
        if referenced != self.assets.keys().collect() {
            return Err(Error::Invalid("VBR missing/unreferenced resource"));
        }
        for i in 0..self.package.brushes.len() {
            BrushProgram::compile(self.definition(i)?)?;
        }
        Ok(())
    }
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
#[serde(deny_unknown_fields)]
struct AssetReference {
    id: String,
    path: String,
    sha256: String,
    kind: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    format: String,
    version: Version,
    package_id: String,
    brushes: Vec<Reference>,
    groups: Vec<serde_json::Value>,
    assets: Vec<AssetReference>,
}

pub fn decode(bytes: &[u8], token: &CancellationToken) -> Result<ImageBundle> {
    let (mut zip, paths) = vbr::checked_archive(bytes, token)?;
    let manifest: Manifest = vbr::json(&vbr::read_entry(&mut zip, "manifest.json", MAX_JSON)?)?;
    if manifest.format != "DrawVerseBrush"
        || manifest.version.major != 1
        || manifest.version.minor > 1
    {
        return Err(Error::Unsupported("VBR format/version".into()));
    }
    if manifest.version.minor == 0 {
        return ImageBundle::new(vbr::decode(bytes, token)?, BTreeMap::new(), token);
    }
    if !manifest.groups.is_empty() {
        return Err(Error::Unsupported("VBR groups".into()));
    }
    if manifest.brushes.is_empty() || manifest.brushes.len() > 1024 || manifest.assets.len() > 1024
    {
        return Err(Error::Limit("VBR resource/preset count"));
    }
    let mut expected = BTreeSet::from(["manifest.json".into()]);
    let mut brushes = Vec::new();
    for reference in manifest.brushes {
        check(token)?;
        if reference.path != format!("brushes/{}.json", reference.id)
            || !expected.insert(reference.path.clone())
        {
            return Err(Error::Invalid("VBR preset reference"));
        }
        let preset: Preset = vbr::json(&vbr::read_entry(&mut zip, &reference.path, MAX_JSON)?)?;
        if preset.id != reference.id {
            return Err(Error::Invalid("VBR preset identity mismatch"));
        }
        brushes.push(preset);
    }
    let mut assets = BTreeMap::new();
    for reference in manifest.assets {
        check(token)?;
        if reference.kind != "tip-mask" {
            return Err(Error::Unsupported("VBR resource kind".into()));
        }
        if !valid_id(&reference.id)
            || reference.sha256 != reference.id
            || reference.path != format!("assets/{}.png", reference.id)
            || !expected.insert(reference.path.clone())
        {
            return Err(Error::Invalid("VBR asset reference"));
        }
        assets.insert(
            reference.id,
            vbr::read_entry(&mut zip, &reference.path, MAX_PACKAGE)?,
        );
    }
    if paths != expected {
        return Err(Error::Invalid("VBR unreferenced archive entries"));
    }
    ImageBundle::new(
        Package {
            package_id: manifest.package_id,
            brushes,
        },
        assets,
        token,
    )
}
pub fn encode(bundle: &ImageBundle, token: &CancellationToken) -> Result<Vec<u8>> {
    bundle.validate()?;
    check(token)?;
    if bundle.assets.is_empty() {
        return vbr::encode(&bundle.package, token);
    }
    let manifest = Manifest {
        format: "DrawVerseBrush".into(),
        version: Version { major: 1, minor: 1 },
        package_id: bundle.package.package_id.clone(),
        groups: Vec::new(),
        brushes: bundle
            .package
            .brushes
            .iter()
            .map(|b| Reference {
                id: b.id.clone(),
                path: format!("brushes/{}.json", b.id),
            })
            .collect(),
        assets: bundle
            .assets
            .keys()
            .map(|id| AssetReference {
                id: id.clone(),
                path: format!("assets/{id}.png"),
                sha256: id.clone(),
                kind: "tip-mask".into(),
            })
            .collect(),
    };
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file("manifest.json", options)?;
    zip.write_all(&vbr::encode_json(&manifest)?)?;
    for preset in &bundle.package.brushes {
        check(token)?;
        zip.start_file(format!("brushes/{}.json", preset.id), options)?;
        zip.write_all(&vbr::encode_json(preset)?)?;
    }
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (id, asset) in &bundle.assets {
        check(token)?;
        zip.start_file(format!("assets/{id}.png"), options)?;
        zip.write_all(asset.bytes())?;
    }
    let bytes = zip.finish()?.into_inner();
    if bytes.len() as u64 > MAX_PACKAGE {
        return Err(Error::Limit("VBR compressed bytes"));
    }
    Ok(bytes)
}
pub fn load(path: &Path, token: &CancellationToken) -> Result<ImageBundle> {
    check(token)?;
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
pub fn save(path: &Path, bundle: &ImageBundle, token: &CancellationToken) -> Result<()> {
    let bytes = encode(bundle, token)?;
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
