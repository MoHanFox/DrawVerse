//! Migration/normalization of parameter state; excludes tool/global preferences.
use crate::{brush_resource::ResourceStore, Error, Result};
use paint_core::{BrushCatalog, BrushSettings, BrushTip};
use paint_task::CancellationToken;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    version: u32,
    selected: String,
    #[serde(deserialize_with = "rows")]
    presets: Vec<Row>,
}
fn rows<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Vec<Row>, D::Error> {
    struct Rows;
    impl<'de> serde::de::Visitor<'de> for Rows {
        type Value = Vec<Row>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("at most 1024 brush presets")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut rows = Vec::new();
            while rows.len() < paint_core::MAX_BRUSH_PRESETS {
                match sequence.next_element()? {
                    Some(row) => rows.push(row),
                    None => return Ok(rows),
                }
            }
            if sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
                return Err(serde::de::Error::custom("brush preset count exceeds 1024"));
            }
            Ok(rows)
        }
    }
    deserializer.deserialize_seq(Rows)
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Row {
    id: String,
    name: String,
    radius: f32,
    opacity: f32,
    spacing: f32,
    #[serde(default)]
    hardness: Option<f32>,
    #[serde(default)]
    roundness: Option<f32>,
    #[serde(default)]
    angle: Option<f32>,
    #[serde(default)]
    flow: Option<f32>,
    #[serde(default)]
    pressure_size: Option<bool>,
    #[serde(default)]
    pressure_flow: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mask_resource: Option<String>,
}
pub fn decode(bytes: &[u8], mask_budget: usize) -> Result<BrushCatalog> {
    decode_inner(bytes, mask_budget, None, &CancellationToken::default())
}
pub fn decode_with_resources(
    bytes: &[u8],
    mask_budget: usize,
    resources: &ResourceStore,
    token: &CancellationToken,
) -> Result<BrushCatalog> {
    decode_inner(bytes, mask_budget, Some(resources), token)
}
fn decode_inner(
    bytes: &[u8],
    mask_budget: usize,
    resources: Option<&ResourceStore>,
    token: &CancellationToken,
) -> Result<BrushCatalog> {
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(Error::Limit("brush state bytes"));
    }
    let state: State =
        serde_json::from_slice(bytes).map_err(|e| Error::Codec(format!("brush state: {e}")))?;
    if !matches!(state.version, 1..=3) {
        return Err(Error::Unsupported("brush state version".into()));
    }
    let mut entries = Vec::new();
    for row in state.presets {
        crate::check(token)?;
        let mask = if let Some(id) = row.mask_resource {
            if state.version != 3 {
                return Err(Error::Unsupported("brush images require state v3".into()));
            }
            Some(
                resources
                    .ok_or(Error::Unsupported(
                        "brush resource store is required".into(),
                    ))?
                    .load(&id, token)?
                    .mask,
            )
        } else {
            None
        };
        let tip = if state.version == 1 {
            BrushTip::default()
        } else {
            BrushTip {
                hardness: row.hardness.ok_or(Error::Invalid("brush hardness"))?,
                roundness: row.roundness.ok_or(Error::Invalid("brush roundness"))?,
                angle: row.angle.ok_or(Error::Invalid("brush angle"))?,
                flow: row.flow.ok_or(Error::Invalid("brush flow"))?,
                pressure_size: row
                    .pressure_size
                    .ok_or(Error::Invalid("brush pressure size"))?,
                pressure_flow: row
                    .pressure_flow
                    .ok_or(Error::Invalid("brush pressure flow"))?,
            }
        };
        entries.push((
            row.id,
            row.name,
            BrushSettings {
                diameter: row.radius * 2.,
                opacity: row.opacity,
                spacing: row.spacing / 2.,
                tip,
                mask,
            },
        ));
    }
    BrushCatalog::from_entries(entries, state.selected, mask_budget).map_err(Into::into)
}
pub fn encode(catalog: &BrushCatalog) -> Result<Vec<u8>> {
    let mut presets = Vec::new();
    let mut version = 2;
    for id in catalog.ids() {
        let p = catalog
            .preset(id)
            .ok_or(Error::Invalid("brush catalog ID"))?;
        let s = p.settings();
        let mask_resource = if let Some(mask) = &s.mask {
            version = 3;
            Some(
                mask.resource_id()
                    .ok_or(Error::Unsupported(
                        "brush image has no persistent identity".into(),
                    ))?
                    .to_owned(),
            )
        } else {
            None
        };
        presets.push(Row {
            id: id.clone(),
            name: p.name().into(),
            radius: s.diameter / 2.,
            opacity: s.opacity,
            spacing: s.spacing * 2.,
            hardness: Some(s.tip.hardness),
            roundness: Some(s.tip.roundness),
            angle: Some(s.tip.angle),
            flow: Some(s.tip.flow),
            pressure_size: Some(s.tip.pressure_size),
            pressure_flow: Some(s.tip.pressure_flow),
            mask_resource,
        });
    }
    serde_json::to_vec(&State {
        version,
        selected: catalog.selected().id().into(),
        presets,
    })
    .map_err(|e| Error::Codec(e.to_string()))
}
