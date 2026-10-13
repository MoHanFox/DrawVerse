//! Validated portable pressure data; core types never depend on serialization.
use paint_core::{BrushDynamics, PressureControl, PressureCurve};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    #[serde(deserialize_with = "points")]
    curve: Vec<(f32, f32)>,
    minimum: f32,
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
    enabled: bool,
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
        let control = |c: &PressureControl| Control {
            curve: c.curve().points().to_vec(),
            minimum: c.minimum(),
        };
        Data {
            size: control(&self.dynamics.size),
            flow: control(&self.dynamics.flow),
            opacity: self.has_opacity.then(|| OpacityControl {
                curve: self.dynamics.opacity.curve().points().to_vec(),
                minimum: self.dynamics.opacity.minimum(),
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
        let control = |c: Control| PressureControl::new(PressureCurve::new(c.curve)?, c.minimum);
        let size = control(data.size).map_err(serde::de::Error::custom)?;
        let flow = control(data.flow).map_err(serde::de::Error::custom)?;
        let has_opacity = data.opacity.is_some();
        let (opacity, pressure_opacity) = match data.opacity {
            Some(c) => (
                control(Control {
                    curve: c.curve,
                    minimum: c.minimum,
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
            },
            has_opacity,
        })
    }
}
