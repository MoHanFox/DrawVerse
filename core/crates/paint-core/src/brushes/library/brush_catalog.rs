use crate::{Brush, BrushDefinition, BrushMask, BrushMode, BrushProgram, BrushTip, Error, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
#[path = "brush_groups.rs"]
mod groups;
pub use groups::{
    BrushCatalogItem, BrushGroup, BrushOrganization, MAX_BRUSH_GROUPS, MAX_BRUSH_GROUP_DEPTH,
};
pub const MAX_BRUSH_PRESETS: usize = 1024;
pub const MAX_CATALOG_MASK_BYTES: usize = 128 * 1024 * 1024;
const BUILTINS: [&str; 4] = [
    "round-pressure",
    "round-fine",
    "round-translucent",
    "round-wide",
];

#[derive(Clone, Debug)]
pub struct BrushSettings {
    pub diameter: f32,
    pub opacity: f32,
    pub spacing: f32,
    pub tip: BrushTip,
    pub mask: Option<Arc<BrushMask>>,
    pub dynamics: crate::BrushDynamics,
    pub texture: crate::BrushTexture,
    pub dual: crate::DualBrush,
    pub smoothing: crate::BrushSmoothing,
    pub wet: crate::WetSettings,
    pub taper: crate::StrokeTaper,
    pub outline: crate::StrokeOutline,
    pub airbrush: crate::Airbrush,
}
impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            diameter: 24.,
            opacity: 1.,
            spacing: 0.075,
            tip: BrushTip::default(),
            mask: None,
            dynamics: crate::BrushDynamics::default(),
            texture: Default::default(),
            dual: Default::default(),
            smoothing: Default::default(),
            wet: Default::default(),
            taper: Default::default(),
            outline: Default::default(),
            airbrush: Default::default(),
        }
    }
}
impl BrushSettings {
    /// Tool colour/mode and temporary size are applied by the stroke caller.
    pub fn definition(&self) -> BrushDefinition {
        BrushDefinition {
            brush: Brush {
                radius: self.diameter / 2.,
                opacity: self.opacity,
                spacing: self.spacing * 2.,
                mode: BrushMode::Paint,
                ..Brush::default()
            },
            tip: self.tip,
            mask: self.mask.clone(),
            dynamics: self.dynamics.clone(),
            texture: self.texture.clone(),
            dual: self.dual.clone(),
            smoothing: self.smoothing,
            wet: self.wet,
            taper: self.taper.clone(),
            outline: self.outline.clone(),
            airbrush: self.airbrush,
            write_blend: Default::default(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct BrushPreset {
    id: String,
    name: String,
    builtin: bool,
    settings: BrushSettings,
    program: BrushProgram,
    source_resource: Option<String>,
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
impl BrushPreset {
    pub fn new(id: String, name: String, settings: BrushSettings) -> Result<Self> {
        if !uuid(&id) {
            return Err(Error::InvalidArgument("brush preset UUID"));
        }
        Self::build(id, name, settings, false)
    }
    fn build(id: String, name: String, settings: BrushSettings, builtin: bool) -> Result<Self> {
        if name.trim().is_empty() || name.chars().count() > 80 || name.contains('\0') {
            return Err(Error::InvalidArgument("brush preset name"));
        }
        let program = BrushProgram::compile(settings.definition())?;
        Ok(Self {
            id,
            name: name.trim().to_owned(),
            builtin,
            settings,
            program,
            source_resource: None,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn builtin(&self) -> bool {
        self.builtin
    }
    pub fn settings(&self) -> &BrushSettings {
        &self.settings
    }
    pub fn program(&self) -> &BrushProgram {
        &self.program
    }
    pub fn source_resource(&self) -> Option<&str> {
        self.source_resource.as_deref()
    }
    pub fn with_source(mut self, source: Option<String>) -> Result<Self> {
        if source.as_ref().is_some_and(|id| {
            id.len() != 64
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }) {
            return Err(Error::InvalidArgument("brush source identity"));
        }
        self.source_resource = source;
        Ok(self)
    }
}
#[derive(Clone, Debug)]
pub struct BrushCatalog {
    presets: BTreeMap<String, Arc<BrushPreset>>,
    order: Vec<String>,
    selected: String,
    revision: u64,
    mask_budget: usize,
    organization: Arc<BrushOrganization>,
}
impl Default for BrushCatalog {
    fn default() -> Self {
        let mut presets = BTreeMap::new();
        for (i, name, diameter, opacity, spacing) in [
            (0, "压感圆笔", 24., 1., 0.075),
            (1, "细线圆笔", 6., 1., 0.075),
            (2, "透明圆笔", 36., 0.25, 0.075),
            (3, "宽幅圆笔", 96., 1., 0.125),
        ] {
            let id = BUILTINS[i].to_owned();
            let settings = BrushSettings {
                diameter,
                opacity,
                spacing,
                ..BrushSettings::default()
            };
            presets.insert(
                id.clone(),
                Arc::new(
                    BrushPreset::build(id, name.into(), settings, true).expect("valid built-in"),
                ),
            );
        }
        Self {
            presets,
            order: BUILTINS.iter().map(|s| s.to_string()).collect(),
            selected: BUILTINS[0].into(),
            revision: 1,
            mask_budget: MAX_CATALOG_MASK_BYTES,
            organization: Arc::new(BrushOrganization::flat(
                &BUILTINS.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            )),
        }
    }
}
impl BrushCatalog {
    pub fn from_entries(
        entries: Vec<(String, String, BrushSettings)>,
        selected: String,
        mask_budget: usize,
    ) -> Result<Self> {
        Self::from_entries_with_sources(
            entries
                .into_iter()
                .map(|(id, name, settings)| (id, name, settings, None))
                .collect(),
            selected,
            mask_budget,
        )
    }
    pub fn from_entries_with_sources(
        entries: Vec<(String, String, BrushSettings, Option<String>)>,
        selected: String,
        mask_budget: usize,
    ) -> Result<Self> {
        let mut restored = Self::with_mask_budget(mask_budget)?;
        if entries.len() < 4 || entries.len() > MAX_BRUSH_PRESETS {
            return Err(Error::ResourceLimit("brush catalog count"));
        }
        let mut seen = BTreeSet::new();
        let mut order = Vec::new();
        let mut presets = BTreeMap::new();
        for (id, name, settings, source) in entries {
            if !seen.insert(id.clone()) {
                return Err(Error::InvalidArgument("duplicate brush preset ID"));
            }
            let builtin = BUILTINS.contains(&id.as_str());
            let preset = if builtin {
                BrushPreset::build(id.clone(), name, settings, true)?
            } else {
                BrushPreset::new(id.clone(), name, settings)?
            };
            order.push(id.clone());
            presets.insert(id, Arc::new(preset.with_source(source)?));
        }
        if !BUILTINS.iter().all(|id| seen.contains(*id)) || !seen.contains(&selected) {
            return Err(Error::InvalidArgument("missing built-in or selected brush"));
        }
        restored.organization = Arc::new(BrushOrganization::flat(&order));
        restored.order = order;
        restored.presets = presets;
        restored.selected = selected;
        restored.resource_budget(std::iter::empty(), None)?;
        Ok(restored)
    }
    pub fn replace_validated(&mut self, mut catalog: Self) {
        catalog.revision = self.revision + 1;
        *self = catalog;
    }
    fn resource_budget<'a>(
        &'a self,
        additional: impl Iterator<Item = &'a BrushSettings>,
        replace: Option<&str>,
    ) -> Result<()> {
        let mut ids = BTreeSet::new();
        let mut texture_ids = BTreeSet::new();
        let mut total = 0usize;
        for settings in self
            .presets
            .values()
            .filter(|p| replace != Some(p.id()))
            .map(|p| &p.settings)
            .chain(additional)
        {
            for mask in settings
                .mask
                .iter()
                .chain(settings.texture.resource().map(|r| r.source()))
                .chain(settings.dual.secondary().mask())
                .chain(
                    settings
                        .dual
                        .secondary()
                        .texture()
                        .resource()
                        .map(|r| r.source()),
                )
            {
                if ids.insert(Arc::as_ptr(mask) as usize) {
                    total = total.saturating_add(mask.resident_bytes());
                }
            }
            for texture in settings
                .texture
                .resource()
                .into_iter()
                .chain(settings.dual.secondary().texture().resource())
            {
                if texture_ids.insert(Arc::as_ptr(texture) as usize) {
                    total = total.saturating_add(texture.mip_bytes());
                }
            }
        }
        if total > self.mask_budget {
            return Err(Error::ResourceLimit("brush catalog image resources"));
        }
        Ok(())
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn mask_budget(&self) -> usize {
        self.mask_budget
    }
    pub fn with_mask_budget(bytes: usize) -> Result<Self> {
        if bytes == 0 || bytes > MAX_CATALOG_MASK_BYTES {
            return Err(Error::InvalidArgument("brush catalog resource budget"));
        }
        Ok(Self {
            mask_budget: bytes,
            ..Self::default()
        })
    }
    pub fn ids(&self) -> &[String] {
        &self.order
    }
    pub fn preset(&self, id: &str) -> Option<Arc<BrushPreset>> {
        self.presets.get(id).cloned()
    }
    pub fn selected(&self) -> Arc<BrushPreset> {
        self.presets[&self.selected].clone()
    }
    pub fn select(&mut self, id: &str) -> Result<()> {
        if !self.presets.contains_key(id) {
            return Err(Error::InvalidArgument("unknown brush preset"));
        }
        if self.selected != id {
            self.selected = id.into();
            self.revision += 1;
        }
        Ok(())
    }
    pub fn edit(&mut self, id: &str, settings: BrushSettings) -> Result<()> {
        let before = self
            .presets
            .get(id)
            .ok_or(Error::InvalidArgument("unknown brush preset"))?;
        let after = BrushPreset::build(id.into(), before.name.clone(), settings, before.builtin)?
            .with_source(before.source_resource.clone())?;
        self.resource_budget(std::iter::once(&after.settings), Some(id))?;
        self.presets.insert(id.into(), Arc::new(after));
        self.revision += 1;
        Ok(())
    }
    pub fn rename(&mut self, id: &str, name: String) -> Result<()> {
        let before = self
            .presets
            .get(id)
            .ok_or(Error::InvalidArgument("unknown brush preset"))?;
        let after = BrushPreset::build(id.into(), name, before.settings.clone(), before.builtin)?
            .with_source(before.source_resource.clone())?;
        self.presets.insert(id.into(), Arc::new(after));
        self.revision += 1;
        Ok(())
    }
    pub fn save_copy(&mut self, id: String, name: String) -> Result<()> {
        let selected = self.selected();
        let preset = BrushPreset::new(id, name, selected.settings.clone())?
            .with_source(selected.source_resource.clone())?;
        let parent = self.selected_group();
        self.import_into_group(vec![preset], parent.as_deref())
    }
    pub fn set_source(&mut self, id: &str, source: Option<String>) -> Result<()> {
        let before = self
            .presets
            .get(id)
            .ok_or(Error::InvalidArgument("unknown brush preset"))?;
        if before.source_resource == source {
            return Ok(());
        }
        let after = (**before).clone().with_source(source)?;
        self.presets.insert(id.into(), Arc::new(after));
        self.revision += 1;
        Ok(())
    }
    pub fn remove(&mut self, id: &str) -> Result<()> {
        let before = self
            .presets
            .get(id)
            .ok_or(Error::InvalidArgument("unknown brush preset"))?;
        if before.builtin {
            return Err(Error::InvalidArgument("built-in brush cannot be removed"));
        }
        let layout = self.organization.without_preset(id)?;
        let mut candidate = self.clone();
        candidate.presets.remove(id);
        if candidate.selected == id {
            candidate.selected = BUILTINS[0].into();
        }
        candidate.set_organization(layout)?;
        *self = candidate;
        Ok(())
    }

    /// Presets are already compiled; batch identity/capacity validation precedes publication.
    pub fn import(&mut self, presets: Vec<BrushPreset>) -> Result<()> {
        self.import_into_group(presets, None)
    }
}
