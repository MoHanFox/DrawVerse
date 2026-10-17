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
                color: Default::default(),
                speed: Default::default(),
                tangent: Default::default(),
                distance: Default::default(),
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
