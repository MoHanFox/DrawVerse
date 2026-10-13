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
        }
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
        Ok(Self {
            dynamics: BrushDynamics { size, flow },
        })
    }
}
