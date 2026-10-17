//! Immutable brush library hierarchy, independent of compiled brush revisions.
use super::*;
pub const MAX_BRUSH_GROUPS: usize = 256;
pub const MAX_BRUSH_GROUP_DEPTH: usize = 8;
const MAX_ITEMS: usize = MAX_BRUSH_PRESETS + MAX_BRUSH_GROUPS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrushCatalogItem {
    Preset(String),
    Group(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrushGroup {
    id: String,
    name: String,
    items: Vec<BrushCatalogItem>,
}
impl BrushGroup {
    pub fn new(id: String, name: String, items: Vec<BrushCatalogItem>) -> Result<Self> {
        if !uuid(&id) {
            return Err(Error::InvalidArgument("brush group UUID"));
        }
        if name.trim().is_empty() || name.chars().count() > 80 || name.contains('\0') {
            return Err(Error::InvalidArgument("brush group name"));
        }
        if items.len() > MAX_ITEMS {
            return Err(Error::ResourceLimit("brush group entries"));
        }
        Ok(Self {
            id,
            name: name.trim().into(),
            items,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn items(&self) -> &[BrushCatalogItem] {
        &self.items
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrushOrganization {
    roots: Vec<BrushCatalogItem>,
    groups: BTreeMap<String, BrushGroup>,
    order: Vec<String>,
}
impl BrushOrganization {
    pub fn new(roots: Vec<BrushCatalogItem>, groups: Vec<BrushGroup>) -> Result<Self> {
        if groups.len() > MAX_BRUSH_GROUPS
            || roots
                .len()
                .saturating_add(groups.iter().map(|g| g.items.len()).sum::<usize>())
                > MAX_ITEMS
        {
            return Err(Error::ResourceLimit("brush organization entries"));
        }
        let mut records = BTreeMap::new();
        for group in groups {
            if records.insert(group.id.clone(), group).is_some() {
                return Err(Error::InvalidArgument("duplicate brush group UUID"));
            }
        }
        let mut seen_groups = BTreeSet::new();
        let mut seen_presets = BTreeSet::new();
        let mut order = Vec::new();
        let mut stack = roots.iter().rev().map(|item| (item, 0)).collect::<Vec<_>>();
        while let Some((item, depth)) = stack.pop() {
            match item {
                BrushCatalogItem::Preset(id) => {
                    if (!uuid(id) && !BUILTINS.contains(&id.as_str()))
                        || records.contains_key(id)
                        || !seen_presets.insert(id)
                    {
                        return Err(Error::InvalidArgument(
                            "invalid or repeated brush organization preset",
                        ));
                    }
                    order.push(id.clone());
                    if order.len() > MAX_BRUSH_PRESETS {
                        return Err(Error::ResourceLimit("brush organization presets"));
                    }
                }
                BrushCatalogItem::Group(id) => {
                    if depth >= MAX_BRUSH_GROUP_DEPTH {
                        return Err(Error::ResourceLimit("brush group depth"));
                    }
                    let group = records
                        .get(id)
                        .ok_or(Error::InvalidArgument("unknown brush group"))?;
                    if !seen_groups.insert(id) {
                        return Err(Error::InvalidArgument("repeated or cyclic brush group"));
                    }
                    stack.extend(group.items.iter().rev().map(|item| (item, depth + 1)));
                }
            }
        }
        if seen_groups.len() != records.len() {
            return Err(Error::InvalidArgument("unreachable brush group"));
        }
        Ok(Self {
            roots,
            groups: records,
            order,
        })
    }
    pub(super) fn flat(ids: &[String]) -> Self {
        Self {
            roots: ids.iter().cloned().map(BrushCatalogItem::Preset).collect(),
            groups: BTreeMap::new(),
            order: ids.to_vec(),
        }
    }
    pub fn roots(&self) -> &[BrushCatalogItem] {
        &self.roots
    }
    pub fn groups(&self) -> &BTreeMap<String, BrushGroup> {
        &self.groups
    }
    pub fn preset_order(&self) -> &[String] {
        &self.order
    }
    fn rebuild(self) -> Result<Self> {
        Self::new(self.roots, self.groups.into_values().collect())
    }
    fn siblings_mut(&mut self, parent: Option<&str>) -> Result<&mut Vec<BrushCatalogItem>> {
        match parent {
            None => Ok(&mut self.roots),
            Some(id) => Ok(&mut self
                .groups
                .get_mut(id)
                .ok_or(Error::InvalidArgument("unknown destination brush group"))?
                .items),
        }
    }
    fn detach(&mut self, item: &BrushCatalogItem) -> Result<(Option<String>, usize)> {
        if let Some(i) = self.roots.iter().position(|x| x == item) {
            self.roots.remove(i);
            return Ok((None, i));
        }
        for (id, group) in &mut self.groups {
            if let Some(i) = group.items.iter().position(|x| x == item) {
                group.items.remove(i);
                return Ok((Some(id.clone()), i));
            }
        }
        Err(Error::InvalidArgument("unknown brush organization item"))
    }
    fn parent_of_preset(&self, id: &str) -> Option<&str> {
        let item = BrushCatalogItem::Preset(id.into());
        self.groups
            .values()
            .find(|g| g.items.contains(&item))
            .map(|g| g.id.as_str())
    }
    pub(super) fn without_preset(&self, id: &str) -> Result<Self> {
        let mut next = self.clone();
        next.detach(&BrushCatalogItem::Preset(id.into()))?;
        next.rebuild()
    }
    pub(super) fn append_presets(
        &self,
        ids: impl Iterator<Item = String>,
        parent: Option<&str>,
    ) -> Result<Self> {
        let mut next = self.clone();
        next.siblings_mut(parent)?
            .extend(ids.map(BrushCatalogItem::Preset));
        next.rebuild()
    }
}
impl BrushCatalog {
    pub fn organization(&self) -> Arc<BrushOrganization> {
        self.organization.clone()
    }
    pub fn set_organization(&mut self, organization: BrushOrganization) -> Result<()> {
        if organization.order.len() != self.presets.len()
            || organization
                .order
                .iter()
                .any(|id| !self.presets.contains_key(id))
        {
            return Err(Error::InvalidArgument(
                "brush organization membership differs from catalog",
            ));
        }
        if *self.organization == organization {
            return Ok(());
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(Error::ResourceLimit("brush catalog revision exhausted"))?;
        self.order = organization.order.clone();
        self.organization = Arc::new(organization);
        self.revision = revision;
        Ok(())
    }
    pub fn create_group(
        &mut self,
        id: String,
        name: String,
        parent: Option<&str>,
        index: usize,
    ) -> Result<()> {
        if self.presets.contains_key(&id) || self.organization.groups.contains_key(&id) {
            return Err(Error::InvalidArgument("brush group identity collision"));
        }
        let group = BrushGroup::new(id.clone(), name, Vec::new())?;
        let mut next = (*self.organization).clone();
        let siblings = next.siblings_mut(parent)?;
        if index > siblings.len() {
            return Err(Error::InvalidArgument("brush group insertion position"));
        }
        siblings.insert(index, BrushCatalogItem::Group(id.clone()));
        next.groups.insert(id, group);
        self.set_organization(next.rebuild()?)
    }
    pub fn rename_group(&mut self, id: &str, name: String) -> Result<()> {
        let mut next = (*self.organization).clone();
        let old = next
            .groups
            .get(id)
            .ok_or(Error::InvalidArgument("unknown brush group"))?;
        let group = BrushGroup::new(id.into(), name, old.items.clone())?;
        next.groups.insert(id.into(), group);
        self.set_organization(next.rebuild()?)
    }
    /// Destination position is interpreted after detaching the source item.
    pub fn move_catalog_item(
        &mut self,
        item: BrushCatalogItem,
        parent: Option<&str>,
        index: usize,
    ) -> Result<()> {
        let mut next = (*self.organization).clone();
        next.detach(&item)?;
        let siblings = next.siblings_mut(parent)?;
        if index > siblings.len() {
            return Err(Error::InvalidArgument("brush move position"));
        }
        siblings.insert(index, item);
        self.set_organization(next.rebuild()?)
    }
    /// Ungroup lifts children in place. Cascade rejects any subtree containing built-ins.
    pub fn remove_group(&mut self, id: &str, cascade: bool) -> Result<()> {
        let mut layout = (*self.organization).clone();
        let group = layout
            .groups
            .get(id)
            .ok_or(Error::InvalidArgument("unknown brush group"))?
            .clone();
        let (parent, index) = layout.detach(&BrushCatalogItem::Group(id.into()))?;
        let mut candidate = self.clone();
        if cascade {
            let mut pending = vec![BrushCatalogItem::Group(id.into())];
            while let Some(item) = pending.pop() {
                match item {
                    BrushCatalogItem::Preset(id) => {
                        if candidate.presets[&id].builtin {
                            return Err(Error::InvalidArgument("built-in brush cannot be removed"));
                        }
                        candidate.presets.remove(&id);
                        if candidate.selected == id {
                            candidate.selected = BUILTINS[0].into();
                        }
                    }
                    BrushCatalogItem::Group(id) => {
                        pending.extend(layout.groups.remove(&id).expect("validated group").items);
                    }
                }
            }
        } else {
            layout.groups.remove(id);
            layout
                .siblings_mut(parent.as_deref())?
                .splice(index..index, group.items);
        }
        candidate.set_organization(layout.rebuild()?)?;
        *self = candidate;
        Ok(())
    }
    pub fn import_into_group(
        &mut self,
        presets: Vec<BrushPreset>,
        parent: Option<&str>,
    ) -> Result<()> {
        if presets.is_empty() || self.order.len().saturating_add(presets.len()) > MAX_BRUSH_PRESETS
        {
            return Err(Error::ResourceLimit("brush catalog capacity"));
        }
        let mut ids = BTreeSet::new();
        for p in &presets {
            if self.presets.contains_key(&p.id) || !ids.insert(&p.id) {
                return Err(Error::InvalidArgument("duplicate brush preset ID"));
            }
        }
        self.resource_budget(presets.iter().map(|p| &p.settings), None)?;
        let layout = self
            .organization
            .append_presets(presets.iter().map(|p| p.id.clone()), parent)?;
        let mut candidate = self.clone();
        candidate.selected = presets[0].id.clone();
        for preset in presets {
            candidate
                .presets
                .insert(preset.id.clone(), Arc::new(preset));
        }
        candidate.set_organization(layout)?;
        *self = candidate;
        Ok(())
    }
    pub(super) fn selected_group(&self) -> Option<String> {
        self.organization
            .parent_of_preset(&self.selected)
            .map(str::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn groups_revision_exhaustion_preserves_candidates_and_allows_true_noops() {
        let group = "12345678-1234-4234-8234-000000000001";
        let user = "12345678-1234-4234-8234-000000000002";
        let mut catalog = BrushCatalog::default();
        catalog
            .create_group(group.into(), "group".into(), None, 0)
            .unwrap();
        catalog
            .import_into_group(
                vec![BrushPreset::new(user.into(), "user".into(), Default::default()).unwrap()],
                Some(group),
            )
            .unwrap();
        catalog.revision = u64::MAX;
        let before = catalog.clone();
        let organization = catalog.organization();
        let captured = catalog.selected();
        catalog.set_organization((*organization).clone()).unwrap();
        catalog.rename_group(group, " group ".into()).unwrap();
        assert!(catalog.remove_group(group, true).is_err());
        assert!(catalog.remove(user).is_err());
        assert!(catalog
            .save_copy("12345678-1234-4234-8234-000000000003".into(), "copy".into())
            .is_err());
        assert!(catalog
            .move_catalog_item(BrushCatalogItem::Preset(user.into()), None, 0)
            .is_err());
        assert_eq!(catalog.revision, u64::MAX);
        assert_eq!(catalog.ids(), before.ids());
        assert!(Arc::ptr_eq(&catalog.organization(), &organization));
        assert!(Arc::ptr_eq(&catalog.selected(), &captured));
    }
}
