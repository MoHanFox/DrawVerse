//! Atomic copy identities; asset bytes, immutable source records and brush behavior stay put.
use super::*;
use paint_core::{BrushCatalogItem, BrushGroup, BrushOrganization};
impl ImageBundle {
    pub fn remap_copy_ids(
        &mut self,
        package_id: String,
        presets: &BTreeMap<String, String>,
        groups: &BTreeMap<String, String>,
        token: &CancellationToken,
    ) -> Result<()> {
        check(token)?;
        self.validate()?;
        let preset_ids = self
            .package
            .brushes
            .iter()
            .map(|p| p.id.clone())
            .collect::<BTreeSet<_>>();
        let group_ids = self
            .organization()
            .map(|o| o.groups().keys().cloned().collect::<BTreeSet<_>>())
            .unwrap_or_default();
        if presets.keys().cloned().collect::<BTreeSet<_>>() != preset_ids
            || groups.keys().cloned().collect::<BTreeSet<_>>() != group_ids
        {
            return Err(Error::Invalid("VBR copy requires complete identity maps"));
        }
        let mut old = preset_ids;
        old.extend(group_ids);
        old.insert(self.package.package_id.clone());
        let mut new = BTreeSet::new();
        for id in presets
            .values()
            .chain(groups.values())
            .chain(std::iter::once(&package_id))
        {
            if old.contains(id) || !new.insert(id) {
                return Err(Error::Invalid("VBR copy identities overlap or repeat"));
            }
        }
        let mut package = self.package.clone();
        package.package_id = package_id;
        for preset in &mut package.brushes {
            check(token)?;
            preset.id = presets[&preset.id].clone();
        }
        package.validate()?;
        let entry = |item: &BrushCatalogItem| match item {
            BrushCatalogItem::Preset(id) => BrushCatalogItem::Preset(presets[id].clone()),
            BrushCatalogItem::Group(id) => BrushCatalogItem::Group(groups[id].clone()),
        };
        let organization = if let Some(before) = self.organization() {
            let records = before
                .groups()
                .values()
                .map(|g| {
                    BrushGroup::new(
                        groups[g.id()].clone(),
                        g.name().into(),
                        g.items().iter().map(&entry).collect(),
                    )
                })
                .collect::<paint_core::Result<Vec<_>>>()?;
            Some(
                crate::brush_organization::OrganizationParameters::from_core(
                    BrushOrganization::new(before.roots().iter().map(entry).collect(), records)?,
                ),
            )
        } else {
            None
        };
        if let Some(o) = &organization {
            if o.core()
                .preset_order()
                .iter()
                .map(String::as_str)
                .ne(package.brushes.iter().map(|p| p.id.as_str()))
            {
                return Err(Error::Invalid("VBR copy projection"));
            }
        }
        check(token)?;
        self.package = package;
        self.organization = organization;
        Ok(())
    }
}
