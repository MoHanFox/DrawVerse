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
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_organization::present"
    )]
    organization: Option<crate::brush_organization::OrganizationParameters>,
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
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_color_dynamics::present"
    )]
    color_dynamics: Option<crate::brush_color_dynamics::ColorParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_texture::present"
    )]
    texture: Option<crate::brush_texture::TextureParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_dual::present"
    )]
    dual_brush: Option<crate::brush_dual::DualParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_smoothing::present"
    )]
    smoothing: Option<crate::brush_smoothing::SmoothingParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_wet::present"
    )]
    wet_mix: Option<crate::brush_wet::WetParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_taper::present"
    )]
    stroke_taper: Option<crate::brush_taper::TaperParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_airbrush::present"
    )]
    airbrush: Option<crate::brush_airbrush::AirbrushParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_speed::present"
    )]
    speed_dynamics: Option<crate::brush_speed::SpeedParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_tangent::present"
    )]
    tangent_dynamics: Option<crate::brush_tangent::TangentParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_distance::present"
    )]
    distance_dynamics: Option<crate::brush_distance::DistanceParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_tilt::present"
    )]
    tilt_dynamics: Option<crate::brush_tilt::TiltParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_rotation::present"
    )]
    rotation_dynamics: Option<crate::brush_rotation::RotationParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_direction::present"
    )]
    direction_dynamics: Option<crate::brush_direction::DirectionParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_source_matrix::present"
    )]
    source_matrix: Option<crate::brush_source_matrix::SourceMatrixParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_fade::present"
    )]
    fade_dynamics: Option<crate::brush_fade::FadeParameters>,
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
    if !matches!(state.version, 1..=36) {
        return Err(Error::Unsupported("brush state version".into()));
    }
    if let Some(organization) = &state.organization {
        if state.version < 20 {
            return Err(Error::Unsupported(
                "brush organization requires state20".into(),
            ));
        }
        if organization
            .core()
            .preset_order()
            .iter()
            .map(String::as_str)
            .ne(state.presets.iter().map(|row| row.id.as_str()))
        {
            return Err(Error::Invalid(
                "organization projection differs from brush rows",
            ));
        }
    }
    let mut entries = Vec::new();
    let mut textures = std::collections::BTreeMap::new();
    let mut resident_masks = std::collections::BTreeSet::new();
    let mut resident = 0usize;
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
        if let Some(m) = &mask {
            if resident_masks.insert(std::sync::Arc::as_ptr(m) as usize) {
                resident = resident.saturating_add(m.resident_bytes());
                if resident > mask_budget {
                    return Err(Error::Limit("brush state resource budget"));
                }
            }
        }
        let texture = if let Some(t) = row.texture {
            if state.version < 11 {
                return Err(Error::Unsupported("texture require state11".into()));
            }
            if t.requires_vbr_26() && state.version < 21 {
                return Err(Error::Unsupported(
                    "texture brightness/contrast requires state21".into(),
                ));
            }
            if t.requires_vbr_27() && state.version < 21 {
                return Err(Error::Unsupported(
                    "texture depth dynamics requires state21".into(),
                ));
            }
            if t.requires_vbr_30() && state.version < 30 {
                return Err(Error::Unsupported(
                    "advanced texture mode requires state30".into(),
                ));
            }
            if t.requires_vbr_31() && state.version < 31 {
                return Err(Error::Unsupported(
                    "latest texture mode requires state31".into(),
                ));
            }
            if t.requires_vbr_39() && state.version < 33 {
                return Err(Error::Unsupported(
                    "dissolve texture mode requires state33".into(),
                ));
            }
            let resource = if let Some(id) = t.resource_id() {
                if !textures.contains_key(id) {
                    let source = resources
                        .ok_or(Error::Unsupported("texture resource store required".into()))?
                        .load(id, token)?
                        .mask;
                    if resident_masks.insert(std::sync::Arc::as_ptr(&source) as usize) {
                        resident = resident.saturating_add(source.resident_bytes());
                    }
                    resident =
                        resident.saturating_add(source.resident_bytes() - source.coverage().len());
                    if resident > mask_budget {
                        return Err(Error::Limit("texture mip resource budget"));
                    }
                    textures.insert(
                        id.to_owned(),
                        std::sync::Arc::new(paint_core::TextureResource::new(source)),
                    );
                }
                Some(textures[id].clone())
            } else {
                None
            };
            t.bind(resource)?
        } else {
            Default::default()
        };
        if state.version < 13
            && (row
                .pressure_dynamics
                .as_ref()
                .is_some_and(|p| p.has_smooth())
                || row.dual_brush.as_ref().is_some_and(|d| d.has_smooth()))
        {
            return Err(Error::Unsupported(
                "smooth pressure requires state13".into(),
            ));
        }
        if row.wet_mix.is_some_and(|p| p.core().dual_color()) && state.version < 19 {
            return Err(Error::Unsupported("wet dual color requires state19".into()));
        }
        if row.wet_mix.is_some() && state.version < 17 {
            return Err(Error::Unsupported("wet mix requires state17".into()));
        }
        if row.stroke_taper.is_some() && state.version < 18 {
            return Err(Error::Unsupported("stroke taper requires state18".into()));
        }
        if row.smoothing.is_some() && state.version < 14 {
            return Err(Error::Unsupported("smoothing requires state14".into()));
        }
        if row.airbrush.is_some() && state.version < 23 {
            return Err(Error::Unsupported("airbrush requires state23".into()));
        }
        if row.speed_dynamics.is_some() && state.version < 24 {
            return Err(Error::Unsupported("speed dynamics requires state24".into()));
        }
        if row.tangent_dynamics.is_some() && state.version < 25 {
            return Err(Error::Unsupported(
                "tangent dynamics requires state25".into(),
            ));
        }
        if row.distance_dynamics.is_some() && state.version < 26 {
            return Err(Error::Unsupported(
                "distance dynamics requires state26".into(),
            ));
        }
        if row.tilt_dynamics.is_some() && state.version < 27 {
            return Err(Error::Unsupported("tilt dynamics requires state27".into()));
        }
        if row.rotation_dynamics.is_some() && state.version < 28 {
            return Err(Error::Unsupported(
                "rotation dynamics requires state28".into(),
            ));
        }
        if row.direction_dynamics.is_some() && state.version < 29 {
            return Err(Error::Unsupported(
                "direction dynamics requires state29".into(),
            ));
        }
        if row.source_matrix.is_some() && state.version < 32 {
            return Err(Error::Unsupported(
                "dynamics source matrix requires state32".into(),
            ));
        }
        if row.fade_dynamics.is_some() && state.version < 34 {
            return Err(Error::Unsupported("fade dynamics requires state34".into()));
        }
        let dual = if let Some(d) = row.dual_brush {
            if d.has_color() && state.version < 16 {
                return Err(Error::Unsupported("dual-color requires state16".into()));
            }
            if d.has_interleaved_color() && state.version < 22 {
                return Err(Error::Unsupported(
                    "interleaved dual-color requires state22".into(),
                ));
            }
            if d.has_sizing() && state.version < 15 {
                return Err(Error::Unsupported(
                    "secondary sizing requires state15".into(),
                ));
            }
            if d.has_scatter_controls() && state.version < 35 {
                return Err(Error::Unsupported(
                    "secondary scatter controls require state35".into(),
                ));
            }
            if d.has_extended_mode() && state.version < 36 {
                return Err(Error::Unsupported(
                    "extended dual blend modes require state36".into(),
                ));
            }
            if state.version < 12 {
                return Err(Error::Unsupported("dual brush require state12".into()));
            }
            let mask = if let Some(id) = d.resource_id() {
                let mask = resources
                    .ok_or(Error::Unsupported(
                        "secondary resource store is required".into(),
                    ))?
                    .load(id, token)?
                    .mask;
                if resident_masks.insert(std::sync::Arc::as_ptr(&mask) as usize) {
                    resident = resident.saturating_add(mask.resident_bytes());
                    if resident > mask_budget {
                        return Err(Error::Limit("secondary resource budget"));
                    }
                }
                Some(mask)
            } else {
                None
            };
            d.bind(mask)?
        } else {
            Default::default()
        };
        if dual.color().model() == paint_core::DualColorModel::InterleavedStamp
            && dual.color().enabled()
            && (dual.secondary().spacing().is_some()
                || row.wet_mix.is_some_and(|p| p.core().enabled()))
        {
            return Err(Error::Invalid(
                "interleaved dual color requires paired dry painting",
            ));
        }
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
        if row.scatter_dynamics.is_some_and(|p| p.has_controls()) && state.version < 35 {
            return Err(Error::Unsupported(
                "scatter device controls require state35".into(),
            ));
        }
        dynamics.scatter = row.scatter_dynamics.map(|p| p.core()).unwrap_or_default();
        if row.shape_jitter.is_some() && state.version < 9 {
            return Err(Error::Unsupported("shape jitter require state9".into()));
        }
        dynamics.shape_jitter = row.shape_jitter.map(|p| p.core()).unwrap_or_default();
        if row.color_dynamics.is_some() && state.version < 10 {
            return Err(Error::Unsupported("color dynamics require state10".into()));
        }
        dynamics.color = row.color_dynamics.map(|p| p.core()).unwrap_or_default();
        dynamics.speed = row.speed_dynamics.map(|p| p.core()).unwrap_or_default();
        dynamics.tangent = row.tangent_dynamics.map(|p| p.core()).unwrap_or_default();
        dynamics.distance = row.distance_dynamics.map(|p| p.core()).unwrap_or_default();
        dynamics.tilt = row.tilt_dynamics.map(|p| p.core()).unwrap_or_default();
        dynamics.rotation = row.rotation_dynamics.map(|p| p.core()).unwrap_or_default();
        dynamics.direction = row.direction_dynamics.map(|p| p.core()).unwrap_or_default();
        dynamics.matrix = row.source_matrix.map(|p| p.core()).unwrap_or_default();
        dynamics.fade = row.fade_dynamics.map(|p| p.core()).unwrap_or_default();
        entries.push((
            row.id,
            row.name,
            BrushSettings {
                taper: row
                    .stroke_taper
                    .map(|p| p.core().clone())
                    .unwrap_or_default(),
                wet: row.wet_mix.map(|p| p.core()).unwrap_or_default(),
                smoothing: row.smoothing.map(|p| p.core()).unwrap_or_default(),
                airbrush: row.airbrush.map(|p| p.core()).unwrap_or_default(),
                texture,
                dual,
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
    let mut catalog =
        BrushCatalog::from_entries_with_sources(entries, state.selected, mask_budget)?;
    if let Some(organization) = state.organization {
        catalog.set_organization(organization.core().clone())?;
    }
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
    let organization = (!catalog.organization().groups().is_empty()).then(|| {
        crate::brush_organization::OrganizationParameters::from_core(
            (*catalog.organization()).clone(),
        )
    });
    let mut presets = Vec::new();
    let mut version = 2;
    for id in catalog.ids() {
        let p = catalog
            .preset(id)
            .ok_or(Error::Invalid("brush catalog ID"))?;
        let s = p.settings();
        let stroke_taper = if s.taper != Default::default() {
            version = version.max(18);
            Some(crate::brush_taper::TaperParameters::from_core(&s.taper))
        } else {
            None
        };
        let wet_mix = if !s.wet.is_default() {
            version = version.max(17);
            if s.wet.dual_color() {
                version = version.max(19);
            }
            Some(crate::brush_wet::WetParameters::from_core(s.wet))
        } else {
            None
        };
        let airbrush = if !s.airbrush.is_default() {
            version = version.max(23);
            Some(crate::brush_airbrush::AirbrushParameters::from_core(
                s.airbrush,
            ))
        } else {
            None
        };
        let speed_dynamics = if !s.dynamics.speed.is_default() {
            version = version.max(24);
            Some(crate::brush_speed::SpeedParameters::from_core(
                s.dynamics.speed.clone(),
            ))
        } else {
            None
        };
        let tangent_dynamics = if !s.dynamics.tangent.is_default() {
            version = version.max(25);
            Some(crate::brush_tangent::TangentParameters::from_core(
                s.dynamics.tangent.clone(),
            ))
        } else {
            None
        };
        let distance_dynamics = if !s.dynamics.distance.is_default() {
            version = version.max(26);
            Some(crate::brush_distance::DistanceParameters::from_core(
                s.dynamics.distance.clone(),
            ))
        } else {
            None
        };
        let tilt_dynamics = if !s.dynamics.tilt.is_default() {
            version = version.max(27);
            Some(crate::brush_tilt::TiltParameters::from_core(
                s.dynamics.tilt.clone(),
            ))
        } else {
            None
        };
        let rotation_dynamics = if !s.dynamics.rotation.is_default() {
            version = version.max(28);
            Some(crate::brush_rotation::RotationParameters::from_core(
                s.dynamics.rotation.clone(),
            ))
        } else {
            None
        };
        let direction_dynamics = if !s.dynamics.direction.is_default() {
            version = version.max(29);
            Some(crate::brush_direction::DirectionParameters::from_core(
                s.dynamics.direction.clone(),
            ))
        } else {
            None
        };
        let source_matrix = if !s.dynamics.matrix.is_default() {
            version = version.max(32);
            Some(crate::brush_source_matrix::SourceMatrixParameters::from_core(s.dynamics.matrix))
        } else {
            None
        };
        let fade_dynamics = if !s.dynamics.fade.is_default() {
            version = version.max(34);
            Some(crate::brush_fade::FadeParameters::from_core(
                s.dynamics.fade.clone(),
            ))
        } else {
            None
        };
        if [&s.dynamics, s.dual.secondary().dynamics()]
            .iter()
            .any(|d| {
                d.size.curve().is_smooth()
                    || d.flow.curve().is_smooth()
                    || d.opacity.curve().is_smooth()
            })
        {
            version = version.max(13);
        }
        let smoothing = if !s.smoothing.is_default() {
            version = version.max(14);
            Some(crate::brush_smoothing::SmoothingParameters::from_core(
                s.smoothing,
            ))
        } else {
            None
        };
        let dual_brush = if !s.dual.is_default() {
            version = version.max(12);
            if s.dual.color() != Default::default() {
                version = version.max(16);
                if s.dual.color().model() == paint_core::DualColorModel::InterleavedStamp {
                    version = version.max(22);
                }
            }
            if s.dual.sizing() != Default::default() {
                version = version.max(15);
            }
            if (s.dual.secondary().dynamics().scatter).has_controls() {
                version = version.max(35);
            }
            let parameters = crate::brush_dual::DualParameters::from_core(&s.dual)?;
            if parameters.has_extended_mode() {
                version = version.max(36);
            }
            Some(parameters)
        } else {
            None
        };
        let texture = if !s.texture.is_default() {
            version = version.max(11);
            let texture = crate::brush_texture::TextureParameters::from_core(&s.texture)?;
            if texture.requires_vbr_39() {
                version = version.max(33);
            } else if texture.requires_vbr_31() {
                version = version.max(31);
            } else if texture.requires_vbr_30() {
                version = version.max(30);
            } else if texture.requires_vbr_26() || texture.requires_vbr_27() {
                version = version.max(21);
            }
            Some(texture)
        } else {
            None
        };
        let color_dynamics = if s.dynamics.color != paint_core::ColorDynamics::default() {
            version = version.max(10);
            Some(crate::brush_color_dynamics::ColorParameters::from_core(
                s.dynamics.color,
            ))
        } else {
            None
        };
        let shape_jitter = if s.dynamics.shape_jitter != paint_core::ShapeJitter::default() {
            version = version.max(9);
            Some(crate::brush_shape_jitter::ShapeParameters::from_core(
                s.dynamics.shape_jitter,
            ))
        } else {
            None
        };
        let scatter_dynamics = if s.dynamics.scatter != paint_core::ScatterDynamics::default() {
            version = version.max(if s.dynamics.scatter.has_controls() {
                35
            } else {
                8
            });
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
        pressure.color = Default::default();
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
            color_dynamics,
            texture,
            dual_brush,
            smoothing,
            wet_mix,
            stroke_taper,
            airbrush,
            speed_dynamics,
            tangent_dynamics,
            distance_dynamics,
            tilt_dynamics,
            rotation_dynamics,
            direction_dynamics,
            source_matrix,
            fade_dynamics,
        });
    }
    if organization.is_some() {
        version = version.max(20);
    }
    let bytes = serde_json::to_vec(&State {
        version,
        selected: catalog.selected().id().into(),
        presets,
        organization,
    })
    .map_err(|e| Error::Codec(e.to_string()))?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(Error::Limit("brush state bytes"));
    }
    Ok(bytes)
}
