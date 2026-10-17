//! Complete bounded native handoff, distinct from the legacy flat Package JSON.
use crate::{brush_organization::OrganizationParameters, vbr, Error, Result};
use paint_core::{BrushCatalogItem, BrushOrganization};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageHandoff {
    model: String,
    package: vbr::Package,
    organization: OrganizationParameters,
}
impl PackageHandoff {
    pub fn new(package: vbr::Package, organization: BrushOrganization) -> Result<Self> {
        let value = Self {
            model: "ordered-package-v1".into(),
            package,
            organization: OrganizationParameters::from_core(organization),
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        if self.model != "ordered-package-v1" {
            return Err(Error::Invalid("VBR handoff model"));
        }
        self.package.validate()?;
        if self
            .organization
            .core()
            .preset_order()
            .iter()
            .map(String::as_str)
            .ne(self.package.brushes.iter().map(|p| p.id.as_str()))
        {
            return Err(Error::Invalid("VBR handoff projection"));
        }
        Ok(())
    }
    pub fn from_json(bytes: &[u8]) -> Result<Self> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Input {
            model: String,
            package: vbr::Package,
            organization: OrganizationParameters,
        }
        let input: Input = vbr::json(bytes)?;
        let value = Self {
            model: input.model,
            package: input.package,
            organization: input.organization,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn to_json(&self) -> Result<Vec<u8>> {
        self.validate()?;
        vbr::encode_json(self)
    }
    pub fn package(&self) -> &vbr::Package {
        &self.package
    }
    pub fn organization(&self) -> &BrushOrganization {
        self.organization.core()
    }
    pub fn into_parts(self) -> (vbr::Package, BrushOrganization) {
        (self.package, self.organization.core().clone())
    }
    pub fn from_bundle(bundle: &crate::vbr_image::ImageBundle) -> Result<Self> {
        bundle.validate()?;
        let organization = match bundle.organization() {
            Some(value) => value.clone(),
            None => BrushOrganization::new(
                bundle
                    .package
                    .brushes
                    .iter()
                    .map(|p| BrushCatalogItem::Preset(p.id.clone()))
                    .collect(),
                vec![],
            )?,
        };
        Self::new(bundle.package.clone(), organization)
    }
}
