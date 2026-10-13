//! Migration/normalization of parameter state; excludes tool/global preferences.
use crate::{Error, Result};
use paint_core::{BrushCatalog, BrushSettings, BrushTip};
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
}
pub fn decode(bytes: &[u8], mask_budget: usize) -> Result<BrushCatalog> {
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(Error::Limit("brush state bytes"));
    }
    let state: State =
        serde_json::from_slice(bytes).map_err(|e| Error::Codec(format!("brush state: {e}")))?;
    if !matches!(state.version, 1 | 2) {
        return Err(Error::Unsupported("brush state version".into()));
    }
    let mut entries = Vec::new();
    for row in state.presets {
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
                mask: None,
            },
        ));
    }
    BrushCatalog::from_entries(entries, state.selected, mask_budget).map_err(Into::into)
}
pub fn encode(catalog: &BrushCatalog) -> Result<Vec<u8>> {
    let mut presets = Vec::new();
    for id in catalog.ids() {
        let p = catalog
            .preset(id)
            .ok_or(Error::Invalid("brush catalog ID"))?;
        let s = p.settings();
        if s.mask.is_some() {
            return Err(Error::Unsupported(
                "brush state image resources require resource codec".into(),
            ));
        }
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
        });
    }
    serde_json::to_vec(&State {
        version: 2,
        selected: catalog.selected().id().into(),
        presets,
    })
    .map_err(|e| Error::Codec(e.to_string()))
}
