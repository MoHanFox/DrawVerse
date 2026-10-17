//! One validated organization DTO, shared by state and future grouped packages.
use paint_core::{
    BrushCatalogItem, BrushGroup, BrushOrganization, MAX_BRUSH_GROUPS, MAX_BRUSH_PRESETS,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum Item {
    Preset { id: String },
    Group { id: String },
}
impl Item {
    fn core(self) -> BrushCatalogItem {
        match self {
            Self::Preset { id } => BrushCatalogItem::Preset(id),
            Self::Group { id } => BrushCatalogItem::Group(id),
        }
    }
    fn from_core(item: &BrushCatalogItem) -> Self {
        match item {
            BrushCatalogItem::Preset(id) => Self::Preset { id: id.clone() },
            BrushCatalogItem::Group(id) => Self::Group { id: id.clone() },
        }
    }
}
fn bounded<'de, D, T: Deserialize<'de>, const N: usize>(d: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor<T, const N: usize>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>, const N: usize> serde::de::Visitor<'de> for Visitor<T, N> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "at most {N} organization entries")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
            let mut rows = Vec::new();
            while rows.len() < N {
                match seq.next_element()? {
                    Some(row) => rows.push(row),
                    None => return Ok(rows),
                }
            }
            if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                return Err(serde::de::Error::custom("organization entry count"));
            }
            Ok(rows)
        }
    }
    d.deserialize_seq(Visitor::<T, N>(std::marker::PhantomData))
}
fn items<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Item>, D::Error> {
    bounded::<D, Item, { MAX_BRUSH_GROUPS + MAX_BRUSH_PRESETS }>(d)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Group {
    id: String,
    name: String,
    #[serde(deserialize_with = "items")]
    items: Vec<Item>,
}
fn groups<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Group>, D::Error> {
    bounded::<D, Group, MAX_BRUSH_GROUPS>(d)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Data {
    model: String,
    #[serde(deserialize_with = "items")]
    roots: Vec<Item>,
    #[serde(deserialize_with = "groups")]
    groups: Vec<Group>,
}
#[derive(Clone, Debug)]
pub struct OrganizationParameters(BrushOrganization);
impl OrganizationParameters {
    pub fn from_core(value: BrushOrganization) -> Self {
        Self(value)
    }
    pub fn core(&self) -> &BrushOrganization {
        &self.0
    }
}
impl Serialize for OrganizationParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Data {
            model: "ordered-tree-v1".into(),
            roots: self.0.roots().iter().map(Item::from_core).collect(),
            groups: self
                .0
                .groups()
                .values()
                .map(|g| Group {
                    id: g.id().into(),
                    name: g.name().into(),
                    items: g.items().iter().map(Item::from_core).collect(),
                })
                .collect(),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for OrganizationParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let data = Data::deserialize(d)?;
        if data.model != "ordered-tree-v1" {
            return Err(serde::de::Error::custom("brush organization model"));
        }
        let groups = data
            .groups
            .into_iter()
            .map(|g| BrushGroup::new(g.id, g.name, g.items.into_iter().map(Item::core).collect()))
            .collect::<paint_core::Result<Vec<_>>>()
            .map_err(serde::de::Error::custom)?;
        BrushOrganization::new(data.roots.into_iter().map(Item::core).collect(), groups)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<OrganizationParameters>, D::Error> {
    OrganizationParameters::deserialize(d).map(Some)
}
