//! VBR 1.1–1.10 portable tips, provenance and dynamics. No file extraction or catalogue mutation.
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
pub const MAX_EXECUTABLE_MINOR: u32 = 27;
#[path = "vbr_groups.rs"]
mod groups;

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
    organization: Option<crate::brush_organization::OrganizationParameters>,
    assets: BTreeMap<String, ImageAsset>,
    sources: Vec<crate::brush_source::SourceBatch>,
    textures: std::sync::Mutex<BTreeMap<String, Arc<paint_core::TextureResource>>>,
}
impl ImageBundle {
    pub fn new(
        package: Package,
        encoded: BTreeMap<String, Vec<u8>>,
        token: &CancellationToken,
    ) -> Result<Self> {
        Self::with_sources(package, encoded, Vec::new(), token)
    }
    pub fn with_sources(
        package: Package,
        encoded: BTreeMap<String, Vec<u8>>,
        sources: Vec<crate::brush_source::SourceBatch>,
        token: &CancellationToken,
    ) -> Result<Self> {
        package.validate()?;
        let mut referenced: BTreeSet<_> = package
            .brushes
            .iter()
            .filter_map(|b| b.tip_resource.as_ref())
            .cloned()
            .collect();
        referenced.extend(package.brushes.iter().filter_map(|b| {
            b.texture
                .as_ref()
                .and_then(|t| t.resource_id())
                .map(str::to_owned)
        }));
        referenced.extend(package.brushes.iter().filter_map(|b| {
            b.dual_brush
                .as_ref()
                .and_then(|d| d.resource_id())
                .map(str::to_owned)
        }));
        for source in &sources {
            for id in source.tip_ids() {
                referenced.insert(id.to_owned());
            }
        }
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
        let bundle = Self {
            package,
            organization: None,
            assets,
            sources,
            textures: Default::default(),
        };
        bundle.validate()?;
        Ok(bundle)
    }
    pub fn organization(&self) -> Option<&paint_core::BrushOrganization> {
        self.organization.as_ref().map(|p| p.core())
    }
    fn validate_organization(&self, organization: &paint_core::BrushOrganization) -> Result<()> {
        if organization
            .preset_order()
            .iter()
            .map(String::as_str)
            .ne(self.package.brushes.iter().map(|p| p.id.as_str()))
        {
            return Err(Error::Invalid(
                "VBR organization projection differs from presets",
            ));
        }
        Ok(())
    }
    pub fn with_organization(
        mut self,
        organization: paint_core::BrushOrganization,
    ) -> Result<Self> {
        self.validate_organization(&organization)?;
        self.organization = (!organization.groups().is_empty())
            .then(|| crate::brush_organization::OrganizationParameters::from_core(organization));
        self.validate()?;
        Ok(self)
    }
    pub fn sources(&self) -> &[crate::brush_source::SourceBatch] {
        &self.sources
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
        if let Some(p) = &preset.stroke_taper {
            definition.taper = p.core().clone();
        }
        if let Some(p) = preset.wet_mix {
            definition.wet = p.core();
        }
        if let Some(p) = preset.smoothing {
            definition.smoothing = p.core();
        }
        if let Some(id) = &preset.tip_resource {
            definition.mask = Some(
                self.assets
                    .get(id)
                    .ok_or(Error::Invalid("VBR missing tip resource"))?
                    .mask(),
            );
        }
        if let Some(t) = &preset.texture {
            t.validate()?;
            let resource = if let Some(id) = t.resource_id() {
                let mut cache = self
                    .textures
                    .lock()
                    .map_err(|_| Error::Invalid("texture cache lock"))?;
                if !cache.contains_key(id) {
                    drop(cache);
                    self.prepare_textures()?;
                    cache = self
                        .textures
                        .lock()
                        .map_err(|_| Error::Invalid("texture cache lock"))?;
                }
                Some(
                    cache
                        .get(id)
                        .ok_or(Error::Invalid("texture missing resource"))?
                        .clone(),
                )
            } else {
                None
            };
            definition.texture = t.bind(resource)?;
        }
        if let Some(d) = &preset.dual_brush {
            let mask = d
                .resource_id()
                .map(|id| {
                    self.assets
                        .get(id)
                        .map(|a| a.mask())
                        .ok_or(Error::Invalid("missing secondary resource"))
                })
                .transpose()?;
            definition.dual = d.bind(mask)?;
        }
        Ok(definition)
    }
    fn prepare_textures(&self) -> Result<()> {
        let ids = self
            .package
            .brushes
            .iter()
            .filter_map(|b| b.texture.as_ref().and_then(|t| t.resource_id()))
            .collect::<BTreeSet<_>>();
        let mut resident = self
            .assets
            .values()
            .map(|a| a.mask.resident_bytes())
            .sum::<usize>();
        for id in &ids {
            let source = &self
                .assets
                .get(*id)
                .ok_or(Error::Invalid("missing texture resource"))?
                .mask;
            resident = resident.saturating_add(source.resident_bytes() - source.coverage().len());
        }
        if resident > MAX_CATALOG_MASK_BYTES {
            return Err(Error::Limit("texture mip budget"));
        }
        let mut cache = self
            .textures
            .lock()
            .map_err(|_| Error::Invalid("texture cache lock"))?;
        cache.retain(|id, _| ids.contains(id.as_str()));
        for id in ids {
            if !cache.contains_key(id) {
                let source = self.assets[id].mask.clone();
                cache.insert(
                    id.to_owned(),
                    Arc::new(paint_core::TextureResource::new(source)),
                );
            }
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        self.package.validate()?;
        if let Some(organization) = &self.organization {
            self.validate_organization(organization.core())?;
        }
        self.prepare_textures()?;
        let mut referenced: BTreeSet<_> = self
            .package
            .brushes
            .iter()
            .filter_map(|b| b.tip_resource.as_deref())
            .collect();
        referenced.extend(
            self.package
                .brushes
                .iter()
                .filter_map(|b| b.texture.as_ref().and_then(|t| t.resource_id())),
        );
        referenced.extend(
            self.package
                .brushes
                .iter()
                .filter_map(|b| b.dual_brush.as_ref().and_then(|d| d.resource_id())),
        );
        let mut source_ids = BTreeSet::new();
        let mut originals = BTreeSet::new();
        let mut records = 0usize;
        let mut original_bytes = 0usize;
        for source in &self.sources {
            if !originals.insert(source.original_id()) {
                return Err(Error::Invalid("VBR duplicate original source"));
            }
            original_bytes = original_bytes.saturating_add(source.original_bytes().len());
            for (id, bytes) in source.records() {
                if !source_ids.insert(id) {
                    return Err(Error::Invalid("VBR duplicate source record"));
                }
                records = records.saturating_add(bytes.len());
                let info: crate::brush_source::SourceInfo = vbr::json(bytes)?;
                let mut definition = info.baseline.definition();
                if let Some(id) = info.tip_resource.as_ref() {
                    definition.mask = Some(
                        self.assets
                            .get(id)
                            .ok_or(Error::Invalid("VBR missing source tip"))?
                            .mask(),
                    );
                }
                BrushProgram::compile(definition)?;
            }
            for id in source.tip_ids() {
                referenced.insert(id);
            }
        }
        if records > 32 * 1024 * 1024
            || original_bytes > 256 * 1024 * 1024
            || source_ids.len() > 1024
        {
            return Err(Error::Limit("VBR source resources"));
        }
        let wanted: BTreeSet<_> = self
            .package
            .brushes
            .iter()
            .filter_map(|b| b.source_resource.as_deref())
            .collect();
        if wanted != source_ids {
            return Err(Error::Invalid("VBR missing/unreferenced source"));
        }
        if referenced != self.assets.keys().map(String::as_str).collect() {
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
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_organization::present"
    )]
    organization: Option<crate::brush_organization::OrganizationParameters>,
}

pub fn decode(bytes: &[u8], token: &CancellationToken) -> Result<ImageBundle> {
    let (mut zip, paths) = vbr::checked_archive(bytes, token)?;
    let manifest: Manifest = vbr::json(&vbr::read_entry(&mut zip, "manifest.json", MAX_JSON)?)?;
    if manifest.format != "DrawVerseBrush"
        || manifest.version.major != 1
        || manifest.version.minor > MAX_EXECUTABLE_MINOR
    {
        return Err(Error::Unsupported("VBR format/version".into()));
    }
    if let Some(organization) = &manifest.organization {
        if manifest.version.minor < 24 {
            return Err(Error::Unsupported(
                "brush organization requires VBR1.24".into(),
            ));
        }
        if organization
            .core()
            .preset_order()
            .iter()
            .map(String::as_str)
            .ne(manifest.brushes.iter().map(|p| p.id.as_str()))
        {
            return Err(Error::Invalid(
                "VBR organization projection differs from references",
            ));
        }
    }
    if manifest.version.minor == 0 {
        return ImageBundle::new(vbr::decode(bytes, token)?, BTreeMap::new(), token);
    }
    if !manifest.groups.is_empty() {
        return Err(Error::Unsupported("VBR groups".into()));
    }
    if manifest.brushes.is_empty()
        || manifest.brushes.len() > 1024
        || manifest.assets.len()
            > if manifest.version.minor >= 10 {
                6144
            } else if manifest.version.minor >= 9 {
                5120
            } else {
                4096
            }
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
    if manifest.version.minor < 23 && brushes.iter().any(|b| b.has_wet_dual()) {
        return Err(Error::Unsupported("wet dual color requires VBR1.23".into()));
    }
    if manifest.version.minor < 22 && brushes.iter().any(|b| b.stroke_taper.is_some()) {
        return Err(Error::Unsupported("stroke taper requires VBR1.22".into()));
    }
    if manifest.version.minor < 15 && brushes.iter().any(|b| b.wet_mix.is_some()) {
        return Err(Error::Unsupported("wet mix requires VBR1.15".into()));
    }
    if manifest.version.minor < 14 && brushes.iter().any(|b| b.has_dual_color()) {
        return Err(Error::Unsupported("dual-color requires VBR1.14".into()));
    }
    if manifest.version.minor < 13 && brushes.iter().any(|b| b.has_dual_sizing()) {
        return Err(Error::Unsupported(
            "secondary sizing requires VBR1.13".into(),
        ));
    }
    if manifest.version.minor < 12 && brushes.iter().any(|b| b.smoothing.is_some()) {
        return Err(Error::Unsupported("smoothing requires VBR1.12".into()));
    }
    if manifest.version.minor < 11 && brushes.iter().any(|b| b.has_smooth()) {
        return Err(Error::Unsupported(
            "smooth pressure requires VBR1.11".into(),
        ));
    }
    if manifest.version.minor < 10 && brushes.iter().any(|b| b.dual_brush.is_some()) {
        return Err(Error::Unsupported("dual brush require VBR1.10".into()));
    }
    if manifest.version.minor < 9 && brushes.iter().any(|b| b.texture.is_some()) {
        return Err(Error::Unsupported("texture require VBR1.9".into()));
    }
    if manifest.version.minor < 25
        && brushes
            .iter()
            .any(|b| b.texture.as_ref().is_some_and(|t| t.mode() == "colorBurn"))
    {
        return Err(Error::Unsupported(
            "color-burn texture mode requires VBR1.25".into(),
        ));
    }
    if manifest.version.minor < 26
        && brushes
            .iter()
            .any(|b| b.texture.as_ref().is_some_and(|t| t.requires_vbr_26()))
    {
        return Err(Error::Unsupported(
            "texture brightness/contrast requires VBR1.26".into(),
        ));
    }
    if manifest.version.minor < 8
        && brushes
            .iter()
            .any(|b| b.parameters.color_dynamics.is_some())
    {
        return Err(Error::Unsupported("color dynamics require VBR1.8".into()));
    }
    if manifest.version.minor < 7 && brushes.iter().any(|b| b.parameters.shape_jitter.is_some()) {
        return Err(Error::Unsupported("shape jitter require VBR1.7".into()));
    }
    if manifest.version.minor < 6
        && brushes
            .iter()
            .any(|b| b.parameters.scatter_dynamics.is_some())
    {
        return Err(Error::Unsupported("scatter require VBR1.6".into()));
    }
    if manifest.version.minor < 5 && brushes.iter().any(|b| b.parameters.tip_dynamics.is_some()) {
        return Err(Error::Unsupported("tip dynamics require VBR1.5".into()));
    }
    if manifest.version.minor < 4
        && brushes.iter().any(|b| {
            b.parameters
                .pressure_dynamics
                .as_ref()
                .is_some_and(|p| p.has_opacity())
        })
    {
        return Err(Error::Unsupported(
            "VBR opacity pressure require version 1.4".into(),
        ));
    }
    if manifest.version.minor < 3
        && brushes
            .iter()
            .any(|b| b.parameters.pressure_dynamics.is_some())
    {
        return Err(Error::Unsupported(
            "VBR pressure data require version 1.3".into(),
        ));
    }
    let mut assets = BTreeMap::new();
    let mut records = BTreeMap::new();
    let mut originals = BTreeMap::new();
    for reference in manifest.assets {
        check(token)?;
        let (prefix, suffix, limit) = match reference.kind.as_str() {
            "tip-mask" => ("assets", "png", MAX_PACKAGE),
            "source-record" if manifest.version.minor >= 2 => ("sources", "json", MAX_JSON),
            "abr-original" if manifest.version.minor >= 2 => ("originals", "abr", MAX_PACKAGE),
            _ => return Err(Error::Unsupported("VBR resource kind/version".into())),
        };
        if !valid_id(&reference.id)
            || reference.sha256 != reference.id
            || reference.path != format!("{prefix}/{}.{suffix}", reference.id)
            || !expected.insert(reference.path.clone())
        {
            return Err(Error::Invalid("VBR asset reference"));
        }
        let bytes = vbr::read_entry(&mut zip, &reference.path, limit)?;
        match reference.kind.as_str() {
            "tip-mask" => {
                assets.insert(reference.id, bytes);
            }
            "source-record" => {
                records.insert(reference.id, bytes);
            }
            _ => {
                originals.insert(reference.id, Arc::new(bytes));
            }
        }
    }
    let mut grouped: BTreeMap<String, BTreeMap<String, Vec<u8>>> = BTreeMap::new();
    let mut total = 0usize;
    for (id, bytes) in records {
        check(token)?;
        total = total.saturating_add(bytes.len());
        if total > 32 * 1024 * 1024 {
            return Err(Error::Limit("VBR source records"));
        }
        let info: crate::brush_source::SourceInfo = vbr::json(&bytes)?;
        if info.version > 1 && manifest.version.minor < 27 {
            return Err(Error::Unsupported(
                "ABR texture source baseline requires VBR 1.27".into(),
            ));
        }
        if info.converter_version > 6 && manifest.version.minor < 21 {
            return Err(Error::Unsupported(
                "ABR converter v7 requires VBR 1.21".into(),
            ));
        }
        if info.converter_version > 5 && manifest.version.minor < 20 {
            return Err(Error::Unsupported(
                "VBR ABR shape sources require version 1.20".into(),
            ));
        }
        if info.converter_version > 4 && manifest.version.minor < 19 {
            return Err(Error::Unsupported(
                "VBR ABR color sources require version 1.19".into(),
            ));
        }
        if info.converter_version > 3 && manifest.version.minor < 18 {
            return Err(Error::Unsupported(
                "VBR ABR scatter sources require version 1.18".into(),
            ));
        }
        if info.converter_version > 2 && manifest.version.minor < 17 {
            return Err(Error::Unsupported(
                "VBR ABR pressure sources require version 1.17".into(),
            ));
        }
        if info.converter_version > 1 && manifest.version.minor < 16 {
            return Err(Error::Unsupported(
                "ABR opacity source requires VBR1.16".into(),
            ));
        }
        grouped
            .entry(info.original_hash)
            .or_default()
            .insert(id, bytes);
    }
    if grouped.keys().collect::<BTreeSet<_>>() != originals.keys().collect() {
        return Err(Error::Invalid("VBR missing/unreferenced original"));
    }
    let mut sources = Vec::new();
    for (id, records) in grouped {
        check(token)?;
        sources.push(crate::brush_source::SourceBatch::from_records(
            originals
                .remove(&id)
                .ok_or(Error::Invalid("VBR source original"))?,
            records,
            token,
        )?);
    }
    if paths != expected {
        return Err(Error::Invalid("VBR unreferenced archive entries"));
    }
    let bundle = ImageBundle::with_sources(
        Package {
            package_id: manifest.package_id,
            brushes,
        },
        assets,
        sources,
        token,
    )?;
    if let Some(organization) = manifest.organization {
        bundle.with_organization(organization.core().clone())
    } else {
        Ok(bundle)
    }
}
pub fn encode(bundle: &ImageBundle, token: &CancellationToken) -> Result<Vec<u8>> {
    bundle.validate()?;
    check(token)?;
    if bundle.organization.is_none()
        && bundle.assets.is_empty()
        && bundle.sources.is_empty()
        && !bundle.package.brushes.iter().any(|b| {
            b.parameters.pressure_dynamics.is_some()
                || b.parameters.tip_dynamics.is_some()
                || b.parameters.scatter_dynamics.is_some()
                || b.parameters.shape_jitter.is_some()
                || b.parameters.color_dynamics.is_some()
                || b.texture.is_some()
                || b.dual_brush.is_some()
                || b.smoothing.is_some()
                || b.wet_mix.is_some()
                || b.stroke_taper.is_some()
        })
    {
        return vbr::encode(&bundle.package, token);
    }
    let stored = bundle
        .assets
        .values()
        .map(|a| a.bytes().len() as u64)
        .sum::<u64>()
        + bundle
            .sources
            .iter()
            .map(|s| {
                s.original_bytes().len() as u64
                    + s.records()
                        .map(|(_, bytes)| bytes.len() as u64)
                        .sum::<u64>()
            })
            .sum::<u64>();
    if stored > MAX_PACKAGE {
        return Err(Error::Limit("VBR stored resource bytes"));
    }
    let manifest = Manifest {
        format: "DrawVerseBrush".into(),
        version: Version {
            major: 1,
            minor: if bundle
                .sources
                .iter()
                .any(|source| source.record_version() > 1)
            {
                27
            } else if bundle
                .package
                .brushes
                .iter()
                .any(|b| b.texture.as_ref().is_some_and(|t| t.requires_vbr_26()))
            {
                26
            } else if bundle
                .package
                .brushes
                .iter()
                .any(|b| b.texture.as_ref().is_some_and(|t| t.mode() == "colorBurn"))
            {
                25
            } else if bundle.organization.is_some() {
                24
            } else if bundle.package.brushes.iter().any(|b| b.has_wet_dual()) {
                23
            } else if bundle
                .package
                .brushes
                .iter()
                .any(|b| b.stroke_taper.is_some())
            {
                22
            } else if bundle
                .sources
                .iter()
                .any(|source| source.converter_version() > 6)
            {
                21
            } else if bundle
                .sources
                .iter()
                .any(|source| source.converter_version() > 5)
            {
                20
            } else if bundle
                .sources
                .iter()
                .any(|source| source.converter_version() > 4)
            {
                19
            } else if bundle
                .sources
                .iter()
                .any(|source| source.converter_version() > 3)
            {
                18
            } else if bundle
                .sources
                .iter()
                .any(|source| source.converter_version() > 2)
            {
                17
            } else if bundle
                .sources
                .iter()
                .any(|source| source.converter_version() > 1)
            {
                16
            } else if bundle.package.brushes.iter().any(|b| b.wet_mix.is_some()) {
                15
            } else if bundle.package.brushes.iter().any(|b| b.has_dual_color()) {
                14
            } else if bundle.package.brushes.iter().any(|b| b.has_dual_sizing()) {
                13
            } else if bundle.package.brushes.iter().any(|b| b.smoothing.is_some()) {
                12
            } else if bundle.package.brushes.iter().any(|b| b.has_smooth()) {
                11
            } else if bundle
                .package
                .brushes
                .iter()
                .any(|b| b.dual_brush.is_some())
            {
                10
            } else if bundle.package.brushes.iter().any(|b| b.texture.is_some()) {
                9
            } else if bundle
                .package
                .brushes
                .iter()
                .any(|b| b.parameters.color_dynamics.is_some())
            {
                8
            } else if bundle
                .package
                .brushes
                .iter()
                .any(|b| b.parameters.shape_jitter.is_some())
            {
                7
            } else if bundle
                .package
                .brushes
                .iter()
                .any(|b| b.parameters.scatter_dynamics.is_some())
            {
                6
            } else if bundle
                .package
                .brushes
                .iter()
                .any(|b| b.parameters.tip_dynamics.is_some())
            {
                5
            } else if bundle.package.brushes.iter().any(|b| {
                b.parameters
                    .pressure_dynamics
                    .as_ref()
                    .is_some_and(|p| p.has_opacity())
            }) {
                4
            } else if bundle
                .package
                .brushes
                .iter()
                .any(|b| b.parameters.pressure_dynamics.is_some())
            {
                3
            } else if bundle.sources.is_empty() {
                1
            } else {
                2
            },
        },
        package_id: bundle.package.package_id.clone(),
        groups: Vec::new(),
        organization: bundle.organization.clone(),
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
            .chain(bundle.sources.iter().flat_map(|source| {
                std::iter::once(AssetReference {
                    id: source.original_id().into(),
                    path: format!("originals/{}.abr", source.original_id()),
                    sha256: source.original_id().into(),
                    kind: "abr-original".into(),
                })
                .chain(source.ids().map(|id| AssetReference {
                    id: id.into(),
                    path: format!("sources/{id}.json"),
                    sha256: id.into(),
                    kind: "source-record".into(),
                }))
            }))
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
    for source in &bundle.sources {
        check(token)?;
        zip.start_file(format!("originals/{}.abr", source.original_id()), options)?;
        zip.write_all(source.original_bytes())?;
        for (id, bytes) in source.records() {
            check(token)?;
            zip.start_file(format!("sources/{id}.json"), options)?;
            zip.write_all(bytes)?;
        }
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
