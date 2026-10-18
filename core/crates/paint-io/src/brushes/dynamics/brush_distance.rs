use paint_core::{DistanceDynamics, PressureControl, PressureCurve};
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
        deserialize_with = "interpolation"
    )]
    interpolation: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    model: String,
    enabled: bool,
    max_distance: f32,
    size: Control,
    flow: Control,
    opacity: Control,
}
fn interpolation<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}
fn points<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Vec<(f32, f32)>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Point {
        Pair((f32, f32)),
        Array([f32; 2]),
    }
    let values = Vec::<Point>::deserialize(d)?;
    if !(2..=17).contains(&values.len()) {
        return Err(serde::de::Error::custom("distance curve point count"));
    }
    Ok(values
        .into_iter()
        .map(|p| match p {
            Point::Pair(p) => p,
            Point::Array(p) => (p[0], p[1]),
        })
        .collect())
}
#[derive(Clone, Debug)]
pub struct DistanceParameters {
    core: DistanceDynamics,
}
impl DistanceParameters {
    pub fn from_core(core: DistanceDynamics) -> Self {
        Self { core }
    }
    pub fn core(&self) -> DistanceDynamics {
        self.core.clone()
    }
    pub fn has_smooth(&self) -> bool {
        self.core.size().curve().is_smooth()
            || self.core.flow().curve().is_smooth()
            || self.core.opacity().curve().is_smooth()
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<DistanceParameters>, D::Error> {
    DistanceParameters::deserialize(d).map(Some)
}
impl Serialize for DistanceParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let control = |c: &PressureControl| Control {
            curve: c.curve().points().to_vec(),
            minimum: c.minimum(),
            interpolation: c.curve().is_smooth().then(|| "smooth-pchip-v1".into()),
        };
        Data {
            model: "distance-v1".into(),
            enabled: self.core.enabled(),
            max_distance: self.core.max_distance(),
            size: control(self.core.size()),
            flow: control(self.core.flow()),
            opacity: control(self.core.opacity()),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for DistanceParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let data = Data::deserialize(d)?;
        if data.model != "distance-v1" {
            return Err(serde::de::Error::custom("distance model"));
        }
        let control = |c: Control| {
            let curve = match c.interpolation.as_deref() {
                None => PressureCurve::new(c.curve),
                Some("smooth-pchip-v1") => PressureCurve::new_smooth(c.curve),
                _ => return Err(paint_core::Error::InvalidArgument("distance interpolation")),
            }?;
            PressureControl::new(curve, c.minimum)
        };
        let size = control(data.size).map_err(serde::de::Error::custom)?;
        let flow = control(data.flow).map_err(serde::de::Error::custom)?;
        let opacity = control(data.opacity).map_err(serde::de::Error::custom)?;
        let core = DistanceDynamics::new(data.enabled, data.max_distance, size, flow, opacity)
            .map_err(serde::de::Error::custom)?;
        Ok(Self { core })
    }
}
