//! Migration/normalization of parameter state; excludes tool/global preferences.
use crate::{brush_resource::ResourceStore, brush_source::SourceStore, Error, Result};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_resource: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_pressure::present"
    )]
    pressure_dynamics: Option<crate::brush_pressure::PressureParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_tip_dynamics::present"
    )]
    tip_dynamics: Option<crate::brush_tip_dynamics::TipParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_scatter::present"
    )]
    scatter_dynamics: Option<crate::brush_scatter::ScatterParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_shape_jitter::present"
    )]
    shape_jitter: Option<crate::brush_shape_jitter::ShapeParameters>,
}
pub fn decode(bytes: &[u8], mask_budget: usize) -> Result<BrushCatalog> {
    decode_inner(
        false,
        bytes,
        mask_budget,
        None,
        None,
        &CancellationToken::default(),
    )
    .map(|(catalog, _)| catalog)
}
pub fn decode_with_resources(
    bytes: &[u8],
    mask_budget: usize,
    resources: &ResourceStore,
    token: &CancellationToken,
) -> Result<BrushCatalog> {
    decode_inner(false, bytes, mask_budget, Some(resources), None, token)
        .map(|(catalog, _)| catalog)
}
pub fn decode_with_sources(
    bytes: &[u8],
    mask_budget: usize,
    resources: Option<&ResourceStore>,
    sources: &SourceStore,
    token: &CancellationToken,
) -> Result<BrushCatalog> {
    decode_inner(false, bytes, mask_budget, resources, Some(sources), token)
        .map(|(catalog, _)| catalog)
}
/// Capture verified immutable source identities for prepared revision lifetimes.
pub fn decode_captured(
    bytes: &[u8],
    mask_budget: usize,
    resources: &ResourceStore,
    sources: &SourceStore,
    token: &CancellationToken,
) -> Result<(
    BrushCatalog,
    Vec<std::sync::Arc<crate::brush_source::ValidatedSource>>,
)> {
    decode_inner(
        true,
        bytes,
        mask_budget,
        Some(resources),
        Some(sources),
        token,
    )
}
fn decode_inner(
    captured: bool,
    bytes: &[u8],
    mask_budget: usize,
    resources: Option<&ResourceStore>,
    sources: Option<&SourceStore>,
    token: &CancellationToken,
) -> Result<(
    BrushCatalog,
    Vec<std::sync::Arc<crate::brush_source::ValidatedSource>>,
)> {
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(Error::Limit("brush state bytes"));
    }
    let state: State =
        serde_json::from_slice(bytes).map_err(|e| Error::Codec(format!("brush state: {e}")))?;
    if !matches!(state.version, 1..=9) {
        return Err(Error::Unsupported("brush state version".into()));
    }
    let mut entries = Vec::new();
    let mut references = std::collections::BTreeSet::new();
    for row in state.presets {
        crate::check(token)?;
        let mask = if let Some(id) = row.mask_resource {
            if state.version < 3 {
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
        if let Some(id) = &row.source_resource {
            if state.version < 4 {
                return Err(Error::Unsupported("brush sources require state v4".into()));
            }
            references.insert(id.clone());
        }
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
        if row
            .pressure_dynamics
            .as_ref()
            .is_some_and(|p| state.version < if p.has_opacity() { 6 } else { 5 })
        {
            return Err(Error::Unsupported(
                "pressure dynamics require state v5 or opacity state v6".into(),
            ));
        }
        if row.tip_dynamics.is_some() && state.version < 7 {
            return Err(Error::Unsupported("tip dynamics require state v7".into()));
        }
        let mut dynamics = row
            .pressure_dynamics
            .as_ref()
            .map(|p| p.core().clone())
            .unwrap_or_default();
        dynamics.tip = row.tip_dynamics.map(|p| p.core()).unwrap_or_default();
        if row.scatter_dynamics.is_some() && state.version < 8 {
            return Err(Error::Unsupported("scatter require state8".into()));
        }
        dynamics.scatter = row.scatter_dynamics.map(|p| p.core()).unwrap_or_default();
        if row.shape_jitter.is_some() && state.version < 9 {
            return Err(Error::Unsupported("shape jitter require state9".into()));
        }
        dynamics.shape_jitter = row.shape_jitter.map(|p| p.core()).unwrap_or_default();
        entries.push((
            row.id,
            row.name,
            BrushSettings {
                dynamics,
                diameter: row.radius * 2.,
                opacity: row.opacity,
                spacing: row.spacing / 2.,
                tip,
                mask,
            },
            row.source_resource,
        ));
    }
    let catalog = BrushCatalog::from_entries_with_sources(entries, state.selected, mask_budget)?;
    let mut receipts = Vec::new();
    if !references.is_empty() {
        let sources = sources.ok_or(Error::Unsupported("brush source store is required".into()))?;
        if captured {
            receipts = sources.resolve_references(&references, token)?;
        } else {
            sources.validate_references(&references, token)?;
        }
    }
    Ok((catalog, receipts))
}
pub fn encode(catalog: &BrushCatalog) -> Result<Vec<u8>> {
    let mut presets = Vec::new();
    let mut version = 2;
    for id in catalog.ids() {
        let p = catalog
            .preset(id)
            .ok_or(Error::Invalid("brush catalog ID"))?;
        let s = p.settings();
        let shape_jitter = if s.dynamics.shape_jitter != paint_core::ShapeJitter::default() {
            version = version.max(9);
            Some(crate::brush_shape_jitter::ShapeParameters::from_core(
                s.dynamics.shape_jitter,
            ))
        } else {
            None
        };
        let scatter_dynamics = if s.dynamics.scatter != paint_core::ScatterDynamics::default() {
            version = version.max(8);
            Some(crate::brush_scatter::ScatterParameters::from_core(
                s.dynamics.scatter,
            ))
        } else {
            None
        };
        let tip_dynamics = if s.dynamics.tip != paint_core::TipDynamics::default() {
            version = version.max(7);
            Some(crate::brush_tip_dynamics::TipParameters::from_core(
                s.dynamics.tip,
            ))
        } else {
            None
        };
        let mut pressure = s.dynamics.clone();
        pressure.tip = Default::default();
        pressure.scatter = Default::default();
        pressure.shape_jitter = Default::default();
        let pressure_dynamics = if pressure != paint_core::BrushDynamics::default() {
            version = version.max(
                if s.dynamics.pressure_opacity
                    || s.dynamics.opacity != paint_core::PressureControl::default()
                {
                    6
                } else {
                    5
                },
            );
            Some(crate::brush_pressure::PressureParameters::from_core(
                &pressure,
            ))
        } else {
            None
        };
        let mask_resource = if let Some(mask) = &s.mask {
            version = version.max(3);
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
        let source_resource = p.source_resource().map(str::to_owned);
        if source_resource.is_some() {
            version = version.max(4);
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
            mask_resource,
            source_resource,
            pressure_dynamics,
            tip_dynamics,
            scatter_dynamics,
            shape_jitter,
        });
    }
    serde_json::to_vec(&State {
        version,
        selected: catalog.selected().id().into(),
        presets,
    })
    .map_err(|e| Error::Codec(e.to_string()))
}
