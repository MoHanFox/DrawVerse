use crate::{brush_resource::valid_id, Error, Result};
use paint_core::{BrushDynamics, BrushMask, BrushTip, DualBrush, DualMode, SecondaryTip};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DualParameters {
    model: String,
    enabled: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    flip: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    flip_x: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    flip_y: bool,
    mode: String,
    diameter: f64,
    #[serde(deserialize_with = "read_spacing")]
    spacing: Option<f64>,
    hardness: f64,
    roundness: f64,
    angle: f64,
    flow: f64,
    pressure_size: bool,
    pressure_flow: bool,
    #[serde(deserialize_with = "read_resource")]
    resource: Option<String>,
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
        deserialize_with = "crate::brush_dual_sizing::present"
    )]
    sizing: Option<crate::brush_dual_sizing::SizingParameters>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::brush_dual_color::present"
    )]
    color: Option<crate::brush_dual_color::ColorParameters>,
}
impl DualParameters {
    pub fn from_core(dual: &DualBrush) -> Result<Self> {
        let s = dual.secondary();
        let t = s.tip();
        let d = s.dynamics();
        let mut pressure = d.clone();
        pressure.tip = Default::default();
        pressure.scatter = Default::default();
        pressure.shape_jitter = Default::default();
        let resource = s
            .mask()
            .map(|m| {
                m.resource_id().map(str::to_owned).ok_or(Error::Unsupported(
                    "secondary image has no persistent identity".into(),
                ))
            })
            .transpose()?;
        Ok(Self {
            color: (dual.color() != Default::default())
                .then(|| crate::brush_dual_color::ColorParameters::from_core(dual.color())),
            sizing: (dual.sizing() != Default::default())
                .then(|| crate::brush_dual_sizing::SizingParameters::from_core(dual.sizing())),
            model: if s.spacing().is_some() {
                "path-coverage-v1"
            } else {
                "paired-dab-v1"
            }
            .into(),
            enabled: dual.enabled(),
            flip: s.flip(),
            flip_x: s.shape_flip().0,
            flip_y: s.shape_flip().1,
            mode: match dual.mode() {
                DualMode::Multiply => "multiply",
                DualMode::Darken => "darken",
                DualMode::Lighten => "lighten",
                DualMode::Subtract => "subtract",
                DualMode::Difference => "difference",
                DualMode::Screen => "screen",
            }
            .into(),
            diameter: f64::from(s.radius()) * 2.,
            spacing: s.spacing().map(|v| f64::from(v).max(0.01)),
            hardness: f64::from(t.hardness),
            roundness: f64::from(t.roundness).max(0.01),
            angle: f64::from(t.angle),
            flow: f64::from(t.flow),
            pressure_size: t.pressure_size,
            pressure_flow: t.pressure_flow,
            resource,
            pressure_dynamics: (pressure != BrushDynamics::default())
                .then(|| crate::brush_pressure::PressureParameters::from_core(&pressure)),
            tip_dynamics: (d.tip != Default::default())
                .then(|| crate::brush_tip_dynamics::TipParameters::from_core(d.tip)),
            scatter_dynamics: (d.scatter != Default::default())
                .then(|| crate::brush_scatter::ScatterParameters::from_core(d.scatter)),
            shape_jitter: (d.shape_jitter != Default::default())
                .then(|| crate::brush_shape_jitter::ShapeParameters::from_core(d.shape_jitter)),
        })
    }
    pub fn has_smooth(&self) -> bool {
        self.pressure_dynamics
            .as_ref()
            .is_some_and(|p| p.has_smooth())
    }
    pub fn has_sizing(&self) -> bool {
        self.sizing.is_some()
    }
    pub(crate) fn active_color(&self) -> bool {
        self.enabled && self.color.as_ref().is_some_and(|p| p.core().enabled())
    }
    pub fn has_color(&self) -> bool {
        self.color.is_some()
    }
    pub fn has_interleaved_color(&self) -> bool {
        self.color
            .as_ref()
            .is_some_and(|p| p.core().model() == paint_core::DualColorModel::InterleavedStamp)
    }
    pub fn is_paired(&self) -> bool {
        self.model == "paired-dab-v1" && self.spacing.is_none()
    }
    pub fn resource_id(&self) -> Option<&str> {
        self.resource.as_deref()
    }
    pub fn validate(&self) -> Result<()> {
        if !matches!(
            self.mode.as_str(),
            "multiply" | "darken" | "lighten" | "subtract" | "difference" | "screen"
        ) || !(self.model == "paired-dab-v1" && self.spacing.is_none()
            || self.model == "path-coverage-v1"
                && self
                    .spacing
                    .is_some_and(|s| s.is_finite() && (0.01..=1.).contains(&s)))
            || self.resource.as_ref().is_some_and(|id| !valid_id(id))
            || self
                .pressure_dynamics
                .as_ref()
                .is_some_and(|p| p.has_opacity())
            || self.scatter_dynamics.is_some_and(|p| p.has_controls())
        {
            return Err(Error::Invalid("dual brush model/parameters"));
        }
        for (v, lo, hi) in [
            (self.diameter, 1., 512.),
            (self.hardness, 0., 1.),
            (self.roundness, 0.01, 1.),
            (self.angle, -180., 180.),
            (self.flow, 0., 1.),
        ] {
            if !v.is_finite() || !(lo..=hi).contains(&v) {
                return Err(Error::Invalid("dual brush range"));
            }
        }
        Ok(())
    }
    pub fn bind(&self, mask: Option<Arc<BrushMask>>) -> Result<DualBrush> {
        self.validate()?;
        if self.resource.is_some() != mask.is_some()
            || mask
                .as_ref()
                .is_some_and(|m| m.resource_id() != self.resource_id())
        {
            return Err(Error::Invalid("secondary resource binding"));
        }
        let mut dynamics = self
            .pressure_dynamics
            .as_ref()
            .map(|p| p.core().clone())
            .unwrap_or_default();
        dynamics.tip = self.tip_dynamics.map(|p| p.core()).unwrap_or_default();
        dynamics.scatter = self.scatter_dynamics.map(|p| p.core()).unwrap_or_default();
        dynamics.shape_jitter = self.shape_jitter.map(|p| p.core()).unwrap_or_default();
        let tip = BrushTip {
            hardness: self.hardness as f32,
            roundness: self.roundness as f32,
            angle: self.angle as f32,
            flow: self.flow as f32,
            pressure_size: self.pressure_size,
            pressure_flow: self.pressure_flow,
        };
        let mut secondary = SecondaryTip::new(self.diameter as f32 / 2., tip, mask, dynamics)?;
        if let Some(spacing) = self.spacing {
            secondary = secondary.with_spacing(spacing as f32)?;
        }
        secondary = secondary.with_flip(self.flip);
        secondary = secondary.with_shape_flip(self.flip_x, self.flip_y);
        let mode = match self.mode.as_str() {
            "multiply" => DualMode::Multiply,
            "darken" => DualMode::Darken,
            "lighten" => DualMode::Lighten,
            "subtract" => DualMode::Subtract,
            "difference" => DualMode::Difference,
            "screen" => DualMode::Screen,
            _ => return Err(Error::Invalid("dual mode")),
        };
        Ok(DualBrush::new(self.enabled, mode, secondary)
            .with_sizing(self.sizing.map(|p| p.core()).unwrap_or_default())
            .with_color(self.color.map(|p| p.core()).unwrap_or_default()))
    }
}
fn is_false(value: &bool) -> bool {
    !*value
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<DualParameters>, D::Error> {
    let p = DualParameters::deserialize(d)?;
    p.validate().map_err(serde::de::Error::custom)?;
    Ok(Some(p))
}
fn read_resource<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)
}
fn read_spacing<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<f64>, D::Error> {
    Option::<f64>::deserialize(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_parameters_preserve_random_flip() {
        let dual = DualBrush::new(
            true,
            DualMode::Multiply,
            SecondaryTip::default()
                .with_flip(true)
                .with_shape_flip(true, false),
        );
        let parameters = DualParameters::from_core(&dual).unwrap();
        let restored = parameters.bind(None).unwrap();
        assert!(restored.secondary().flip());
        assert_eq!(restored.secondary().shape_flip(), (true, false));
    }
}
