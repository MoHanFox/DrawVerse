//! Validated portable pressure data; core types never depend on serialization.
use paint_core::{BrushDynamics, PressureControl, PressureCurve};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    #[serde(deserialize_with = "points")]
    curve: Vec<(f32, f32)>,
    minimum: f32,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "interpolation_present"
    )]
    interpolation: Option<String>,
    #[serde(
        default,
        rename = "nodeModes",
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_pressure::present_node_modes"
    )]
    node_modes: Option<Vec<bool>>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Data {
    size: Control,
    flow: Control,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "opacity_present"
    )]
    opacity: Option<OpacityControl>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OpacityControl {
    #[serde(deserialize_with = "points")]
    curve: Vec<(f32, f32)>,
    minimum: f32,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "interpolation_present"
    )]
    interpolation: Option<String>,
    #[serde(
        default,
        rename = "nodeModes",
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_pressure::present_node_modes"
    )]
    node_modes: Option<Vec<bool>>,
    enabled: bool,
}
fn interpolation_present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}
fn opacity_present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<OpacityControl>, D::Error> {
    OpacityControl::deserialize(d).map(Some)
}
fn points<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Vec<(f32, f32)>, D::Error> {
    struct Points;
    impl<'de> serde::de::Visitor<'de> for Points {
        type Value = Vec<(f32, f32)>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("2 to 17 pressure points")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut points = Vec::new();
            while points.len() < 17 {
                match sequence.next_element()? {
                    Some(point) => points.push(point),
                    None => return Ok(points),
                }
            }
            if sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
                return Err(serde::de::Error::custom(
                    "pressure curve has more than 17 points",
                ));
            }
            Ok(points)
        }
    }
    deserializer.deserialize_seq(Points)
}
#[derive(Clone, Debug)]
pub struct PressureParameters {
    dynamics: BrushDynamics,
    has_opacity: bool,
}
/// Absent is default; explicitly present null is not a pressure object.
pub fn present<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<PressureParameters>, D::Error> {
    PressureParameters::deserialize(deserializer).map(Some)
}
impl PressureParameters {
    pub fn from_core(dynamics: &BrushDynamics) -> Self {
        Self {
            dynamics: dynamics.clone(),
            has_opacity: dynamics.pressure_opacity
                || dynamics.opacity != PressureControl::default(),
        }
    }
    pub fn has_smooth(&self) -> bool {
        self.dynamics.size.curve().is_smooth()
            || self.dynamics.flow.curve().is_smooth()
            || self.dynamics.opacity.curve().is_smooth()
    }
    pub fn has_opacity(&self) -> bool {
        self.has_opacity
    }
    pub fn core(&self) -> &BrushDynamics {
        &self.dynamics
    }
}
impl Serialize for PressureParameters {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        if self.dynamics.tip != paint_core::TipDynamics::default()
            || self.dynamics.scatter != paint_core::ScatterDynamics::default()
            || self.dynamics.shape_jitter != paint_core::ShapeJitter::default()
            || self.dynamics.transfer_jitter != paint_core::TransferJitter::default()
            || self.dynamics.color != paint_core::ColorDynamics::default()
        {
            return Err(serde::ser::Error::custom(
                "non-pressure dynamics are not supported by pressure payload",
            ));
        }
        let control = |c: &PressureControl| Control {
            curve: c.curve().points().to_vec(),
            minimum: c.minimum(),
            interpolation: c.curve().is_smooth().then(|| "smooth-pchip-v1".into()),
            node_modes: c.curve().node_modes().map(<[bool]>::to_vec),
        };
        Data {
            size: control(&self.dynamics.size),
            flow: control(&self.dynamics.flow),
            opacity: self.has_opacity.then(|| OpacityControl {
                curve: self.dynamics.opacity.curve().points().to_vec(),
                minimum: self.dynamics.opacity.minimum(),
                interpolation: self
                    .dynamics
                    .opacity
                    .curve()
                    .is_smooth()
                    .then(|| "smooth-pchip-v1".into()),
                node_modes: self
                    .dynamics
                    .opacity
                    .curve()
                    .node_modes()
                    .map(<[bool]>::to_vec),
                enabled: self.dynamics.pressure_opacity,
            }),
        }
        .serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for PressureParameters {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let data = Data::deserialize(deserializer)?;
        let control = |c: Control| {
            let curve = match c.interpolation.as_deref() {
                None => PressureCurve::new(c.curve)?,
                Some("smooth-pchip-v1") => PressureCurve::new_smooth(c.curve)?,
                _ => return Err(paint_core::Error::InvalidArgument("pressure interpolation")),
            };
            let curve = match c.node_modes {
                Some(modes) if curve.is_smooth() => curve.with_node_modes(modes)?,
                Some(_) => {
                    return Err(paint_core::Error::InvalidArgument(
                        "node modes require smooth interpolation",
                    ))
                }
                None => curve,
            };
            PressureControl::new(curve, c.minimum)
        };
        let size = control(data.size).map_err(serde::de::Error::custom)?;
        let flow = control(data.flow).map_err(serde::de::Error::custom)?;
        let has_opacity = data.opacity.is_some();
        let (opacity, pressure_opacity) = match data.opacity {
            Some(c) => (
                control(Control {
                    curve: c.curve,
                    minimum: c.minimum,
                    interpolation: c.interpolation,
                    node_modes: c.node_modes,
                })
                .map_err(serde::de::Error::custom)?,
                c.enabled,
            ),
            None => (PressureControl::default(), false),
        };
        Ok(Self {
            dynamics: BrushDynamics {
                size,
                flow,
                opacity,
                pressure_opacity,
                tip: Default::default(),
                scatter: Default::default(),
                shape_jitter: Default::default(),
                transfer_jitter: Default::default(),
                color: Default::default(),
                speed: Default::default(),
                tangent: Default::default(),
                distance: Default::default(),
                tilt: Default::default(),
                rotation: Default::default(),
                direction: Default::default(),
                matrix: Default::default(),
                fade: Default::default(),
            },
            has_opacity,
        })
    }
}

/// Bounded pure curve query used by the native editor; execution evaluates the same compiled curve.
pub fn curve_samples(bytes: &[u8], smooth: bool) -> crate::Result<Vec<f32>> {
    if bytes.is_empty() || bytes.len() > 4096 {
        return Err(crate::Error::Limit("pressure points JSON bytes"));
    }
    let points: Vec<(f32, f32)> =
        serde_json::from_slice(bytes).map_err(|e| crate::Error::Codec(e.to_string()))?;
    let curve = if smooth {
        PressureCurve::new_smooth(points)?
    } else {
        PressureCurve::new(points)?
    };
    Ok(PressureControl::new(curve, 0.)?.samples(257)?)
}

/// Node-aware editor query; shares strict control parsing and the execution compiler.
pub fn curve_node_samples(bytes: &[u8]) -> crate::Result<Vec<f32>> {
    if bytes.is_empty() || bytes.len() > 4096 {
        return Err(crate::Error::Limit("pressure node JSON bytes"));
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Query {
        #[serde(deserialize_with = "points")]
        curve: Vec<(f32, f32)>,
        #[serde(deserialize_with = "present_node_modes")]
        node_modes: Option<Vec<bool>>,
    }
    let query: Query =
        serde_json::from_slice(bytes).map_err(|e| crate::Error::Codec(e.to_string()))?;
    let curve = PressureCurve::new_smooth(query.curve)?.with_node_modes(
        query
            .node_modes
            .ok_or(crate::Error::Invalid("node modes required"))?,
    )?;
    Ok(PressureControl::new(curve, 0.)?.samples(257)?)
}

/// Presence is strict: null, non-booleans and unbounded lists are invalid.
pub(crate) fn present_node_modes<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<Vec<bool>>, D::Error> {
    struct Modes;
    impl<'de> serde::de::Visitor<'de> for Modes {
        type Value = Vec<bool>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("2 to 17 boolean node modes")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut modes = Vec::new();
            while let Some(mode) = seq.next_element::<bool>()? {
                if modes.len() == 17 {
                    return Err(serde::de::Error::custom("node mode count"));
                }
                modes.push(mode);
            }
            if modes.len() < 2 {
                return Err(serde::de::Error::custom("node mode count"));
            }
            Ok(modes)
        }
    }
    d.deserialize_seq(Modes).map(Some)
}
