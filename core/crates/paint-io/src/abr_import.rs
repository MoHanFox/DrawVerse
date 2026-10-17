//! Whole-package baseline import. No catalog publication or invented PS dynamics.
use crate::{
    abr_association, abr_conversion, abr_sampled,
    brush_resource::{encode_mask, hash, ResourceStore},
    brush_source::SourceBatch,
    check, Error, Result,
};
use paint_core::{BrushProgram, BrushSettings, TextureResource, MAX_CATALOG_MASK_BYTES};
use paint_task::CancellationToken;
use std::{collections::BTreeMap, fs::File, io::Read, path::Path, sync::Arc};

pub struct ImportedPreset {
    converter_version: u32,
    name: String,
    settings: BrushSettings,
    source: String,
    partial: bool,
    tip: Option<String>,
    texture: Option<String>,
}
impl ImportedPreset {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn settings(&self) -> &BrushSettings {
        &self.settings
    }
    pub fn source_id(&self) -> &str {
        &self.source
    }
    pub fn is_partial(&self) -> bool {
        self.partial
    }
}
pub struct ImportBatch {
    presets: Vec<ImportedPreset>,
    assets: BTreeMap<String, Vec<u8>>,
    sources: SourceBatch,
}
impl ImportBatch {
    /// ABR-only handoff; deliberately not a VBR package or a catalog state.
    pub fn to_json(&self) -> Result<Vec<u8>> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Row<'a> {
            name: &'a str,
            parameters: crate::vbr::Parameters,
            tip_resource: Option<&'a str>,
            texture: Option<crate::brush_texture::TextureParameters>,
            source_resource: &'a str,
            partial: bool,
        }
        #[derive(serde::Serialize)]
        struct Handoff<'a> {
            version: u32,
            format: &'a str,
            presets: Vec<Row<'a>>,
        }
        let presets = self
            .presets
            .iter()
            .map(|p| -> Result<Row<'_>> {
                let s = &p.settings;
                Ok(Row {
                    name: &p.name,
                    parameters: crate::vbr::Parameters {
                        scatter_dynamics: abr_conversion::scatter_parameters(
                            s,
                            p.converter_version,
                        ),
                        shape_jitter: abr_conversion::shape_parameters(s, p.converter_version),
                        color_dynamics: abr_conversion::color_parameters(s, p.converter_version),
                        tip_dynamics: abr_conversion::device_parameters(s, p.converter_version),
                        pressure_dynamics: abr_conversion::pressure_parameters(
                            s,
                            p.converter_version,
                        ),
                        airbrush: None,
                        speed_dynamics: None,
                        tangent_dynamics: None,
                        distance_dynamics: None,
                        tilt_dynamics: (!s.dynamics.tilt.is_default()).then(|| {
                            crate::brush_tilt::TiltParameters::from_core(s.dynamics.tilt.clone())
                        }),
                        rotation_dynamics: (!s.dynamics.rotation.is_default()).then(|| {
                            crate::brush_rotation::RotationParameters::from_core(
                                s.dynamics.rotation.clone(),
                            )
                        }),
                        direction_dynamics: (!s.dynamics.direction.is_default()).then(|| {
                            crate::brush_direction::DirectionParameters::from_core(
                                s.dynamics.direction.clone(),
                            )
                        }),
                        source_matrix: (!s.dynamics.matrix.is_default()).then(|| {
                            crate::brush_source_matrix::SourceMatrixParameters::from_core(
                                s.dynamics.matrix,
                            )
                        }),
                        diameter: s.diameter,
                        spacing: s.spacing,
                        dab_opacity: s.opacity,
                        hardness: s.tip.hardness,
                        roundness: s.tip.roundness,
                        angle: s.tip.angle,
                        flow: s.tip.flow,
                        pressure_size: s.tip.pressure_size,
                        pressure_flow: s.tip.pressure_flow,
                    },
                    tip_resource: p.tip.as_deref(),
                    texture: (!s.texture.is_default())
                        .then(|| crate::brush_texture::TextureParameters::from_core(&s.texture))
                        .transpose()?,
                    source_resource: &p.source,
                    partial: p.partial,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let bytes = serde_json::to_vec(&Handoff {
            version: if self.presets.iter().any(|p| p.converter_version >= 9) {
                8
            } else if self.presets.iter().any(|p| p.converter_version >= 8) {
                7
            } else if self.presets.iter().any(|p| p.converter_version >= 7) {
                6
            } else if self.presets.iter().any(|p| p.converter_version >= 6) {
                5
            } else if self.presets.iter().any(|p| p.converter_version >= 5) {
                4
            } else if self.presets.iter().any(|p| p.converter_version >= 4) {
                3
            } else if self.presets.iter().any(|p| p.converter_version >= 3) {
                2
            } else {
                1
            },
            format: "abr",
            presets,
        })
        .map_err(|e| Error::Codec(e.to_string()))?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err(Error::Limit("ABR handoff JSON bytes"));
        }
        Ok(bytes)
    }
    pub fn presets(&self) -> &[ImportedPreset] {
        &self.presets
    }
    pub fn asset_count(&self) -> usize {
        self.assets.len()
    }
    pub fn read(path: &Path, mask_budget: usize, token: &CancellationToken) -> Result<Self> {
        check(token)?;
        let mut file = File::open(path)?;
        const LIMIT: u64 = 128 * 1024 * 1024;
        if file.metadata()?.len() > LIMIT {
            return Err(Error::Limit("ABR import file bytes"));
        }
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(LIMIT + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > LIMIT {
            return Err(Error::Limit("ABR import file bytes"));
        }
        Self::decode(Arc::new(bytes), mask_budget, token)
    }
    pub fn decode(
        bytes: Arc<Vec<u8>>,
        mask_budget: usize,
        token: &CancellationToken,
    ) -> Result<Self> {
        check(token)?;
        if mask_budget == 0 || mask_budget > MAX_CATALOG_MASK_BYTES {
            return Err(Error::Invalid("ABR import mask budget"));
        }
        let parsed = abr_sampled::decode_sectioned(&bytes, token)?;
        let descriptors = parsed.descriptors(token)?;
        let linked = abr_association::associate(&parsed, &descriptors, token)?;
        let converted = abr_conversion::convert_textured(
            &linked,
            &parsed.texture_presets,
            &parsed.patterns,
            token,
        )?;
        let mut assets = BTreeMap::new();
        let mut identities: BTreeMap<usize, String> = BTreeMap::new();
        let mut resident = 0usize;
        let mut encoded = 0usize;
        let mut tips = Vec::new();
        let mut textures = Vec::new();
        for preset in &converted {
            check(token)?;
            BrushProgram::compile(preset.settings.definition())?;
            let tip = if let Some(mask) = &preset.settings.mask {
                let key = Arc::as_ptr(mask) as usize;
                if let Some(identity) = identities.get(&key) {
                    Some(identity.clone())
                } else {
                    resident = resident.saturating_add(mask.resident_bytes());
                    if resident > mask_budget {
                        return Err(Error::Limit("ABR import image resources"));
                    }
                    let png = encode_mask(mask)?;
                    let identity = hash(&png);
                    if !assets.contains_key(&identity) {
                        encoded = encoded.saturating_add(png.len());
                        if encoded > 128 * 1024 * 1024 {
                            return Err(Error::Limit("ABR import asset bytes"));
                        }
                        assets.insert(identity.clone(), png);
                    }
                    identities.insert(key, identity.clone());
                    Some(identity)
                }
            } else {
                None
            };
            tips.push(tip);
            let texture = if let Some(resource) = preset.settings.texture.resource() {
                let source = resource.source();
                let identity = source
                    .resource_id()
                    .ok_or(Error::Invalid("ABR imported texture identity"))?
                    .to_owned();
                if !assets.contains_key(&identity) {
                    let png = encode_mask(source)?;
                    if hash(&png) != identity {
                        return Err(Error::Invalid("ABR imported texture hash"));
                    }
                    encoded = encoded.saturating_add(png.len());
                    if encoded > 128 * 1024 * 1024 {
                        return Err(Error::Limit("ABR import asset bytes"));
                    }
                    assets.insert(identity.clone(), png);
                }
                Some(identity)
            } else {
                None
            };
            textures.push(texture);
        }
        let sources = SourceBatch::capture(bytes, &converted, token)?;
        let presets = converted
            .iter()
            .zip(sources.ids())
            .zip(tips)
            .zip(textures)
            .map(|(((preset, source), tip), texture)| ImportedPreset {
                converter_version: preset.converter_version,
                name: preset.name.into(),
                settings: preset.settings.clone(),
                source: source.into(),
                partial: preset.is_partial(),
                tip,
                texture,
            })
            .collect();
        check(token)?;
        Ok(Self {
            presets,
            assets,
            sources,
        })
    }
    /// Store only a fully decoded batch; failure returns no partially usable list.
    /// Content-addressed files may remain unreferenced after a later IO failure.
    pub fn persist(
        self,
        resources: &ResourceStore,
        token: &CancellationToken,
    ) -> Result<Vec<ImportedPreset>> {
        check(token)?;
        let mut masks = BTreeMap::new();
        for (id, png) in &self.assets {
            check(token)?;
            masks.insert(id.clone(), resources.put_verified(id, png, token)?.mask);
        }
        resources.source_store()?.save_batch(&self.sources, token)?;
        let mut presets = self.presets;
        for preset in &mut presets {
            check(token)?;
            if let Some(id) = &preset.tip {
                preset.settings.mask = Some(
                    masks
                        .get(id)
                        .ok_or(Error::Invalid("ABR imported resource identity"))?
                        .clone(),
                );
            }
            if let Some(id) = &preset.texture {
                let source = masks
                    .get(id)
                    .ok_or(Error::Invalid("ABR imported texture resource identity"))?;
                let parameters =
                    crate::brush_texture::TextureParameters::from_core(&preset.settings.texture)?;
                preset.settings.texture = parameters.bind(Some(std::sync::Arc::new(
                    TextureResource::new(source.clone()),
                )))?;
            }
            BrushProgram::compile(preset.settings.definition())?;
        }
        check(token)?;
        Ok(presets)
    }
}
