//! Complete distance envelopes; strict validation precedes f32 curve conversion.
use paint_core::{PressureControl, PressureCurve, StrokeTaper, TaperEnd, TAPER_MODEL};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Control {
    #[serde(deserialize_with = "points")]
    curve: Vec<(f64, f64)>,
    minimum: f64,
    interpolation: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct End {
    length_px: f64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "mode"
    )]
    length_mode: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "percent"
    )]
    length_percent: Option<f64>,
    size: Control,
    opacity: Control,
}
fn mode<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}
fn percent<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Option<f64>, D::Error> {
    f64::deserialize(d).map(Some)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Data {
    model: String,
    enabled: bool,
    start: End,
    end: End,
}

fn points<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Vec<(f64, f64)>, D::Error> {
    struct Points;
    impl<'de> serde::de::Visitor<'de> for Points {
        type Value = Vec<(f64, f64)>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("2 to 17 taper points")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut points = Vec::new();
            while points.len() < 17 {
                match seq.next_element()? {
                    Some(p) => points.push(p),
                    None => return Ok(points),
                }
            }
            if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                return Err(serde::de::Error::custom("taper curve count"));
            }
            Ok(points)
        }
    }
    d.deserialize_seq(Points)
}
impl Control {
    fn from_core(c: &PressureControl) -> Self {
        Self {
            curve: c
                .curve()
                .points()
                .iter()
                .map(|&(x, y)| (f64::from(x), f64::from(y)))
                .collect(),
            minimum: f64::from(c.minimum()),
            interpolation: if c.curve().is_smooth() {
                "pchip"
            } else {
                "linear"
            }
            .into(),
        }
    }
    fn core(self) -> std::result::Result<PressureControl, &'static str> {
        if self.curve.first().copied() != Some((0., 0.))
            || self.curve.last().copied() != Some((1., 1.))
            || !self.minimum.is_finite()
            || !(0. ..=1.).contains(&self.minimum)
            || self.curve.iter().any(|&(x, y)| {
                !x.is_finite()
                    || !y.is_finite()
                    || !(0. ..=1.).contains(&x)
                    || !(0. ..=1.).contains(&y)
            })
            || self.curve.windows(2).any(|p| p[0].0 >= p[1].0)
        {
            return Err("taper control range/order");
        }
        let points = self
            .curve
            .into_iter()
            .map(|(x, y)| (x as f32, y as f32))
            .collect();
        let curve = match self.interpolation.as_str() {
            "linear" => PressureCurve::new(points),
            "pchip" => PressureCurve::new_smooth(points),
            _ => return Err("taper interpolation"),
        }
        .map_err(|_| "taper curve")?;
        PressureControl::new(curve, self.minimum as f32).map_err(|_| "taper minimum")
    }
}
impl End {
    fn from_core(e: &TaperEnd) -> Self {
        Self {
            length_px: e.length(),
            length_mode: e
                .relative_length()
                .map(|(relative, _)| if relative { "relative" } else { "independent" }.into()),
            length_percent: e.relative_length().map(|(_, percent)| percent),
            size: Control::from_core(e.size()),
            opacity: Control::from_core(e.opacity()),
        }
    }
    fn core(self) -> std::result::Result<TaperEnd, &'static str> {
        let end = TaperEnd::new(self.length_px, self.size.core()?, self.opacity.core()?)
            .map_err(|_| "taper length/endpoints")?;
        match (self.length_mode.as_deref(), self.length_percent) {
            (None, None) => Ok(end),
            (Some(mode @ ("relative" | "independent")), Some(percent)) => end
                .with_relative_length(mode == "relative", percent)
                .map_err(|_| "taper length percent"),
            _ => Err("taper length mode/percent"),
        }
    }
}
#[derive(Clone, Debug)]
pub struct TaperParameters {
    settings: StrokeTaper,
}
impl TaperParameters {
    pub fn from_core(settings: &StrokeTaper) -> Self {
        Self {
            settings: settings.clone(),
        }
    }
    pub fn core(&self) -> &StrokeTaper {
        &self.settings
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<TaperParameters>, D::Error> {
    TaperParameters::deserialize(d).map(Some)
}
impl Serialize for TaperParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        Data {
            model: TAPER_MODEL.into(),
            enabled: self.settings.enabled(),
            start: End::from_core(self.settings.start()),
            end: End::from_core(self.settings.end()),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for TaperParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let p = Data::deserialize(d)?;
        if p.model != TAPER_MODEL {
            return Err(serde::de::Error::custom("taper model"));
        }
        let start = p.start.core().map_err(serde::de::Error::custom)?;
        let end = p.end.core().map_err(serde::de::Error::custom)?;
        Ok(Self {
            settings: StrokeTaper::new(p.enabled, start, end),
        })
    }
}
