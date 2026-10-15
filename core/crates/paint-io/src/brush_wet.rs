//! Complete, strictly bounded wet preset policy. Reservoir pixels are tool state.
use paint_core::{WetMix, WetSettings, WET_CARRIER_MODEL};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    model: String,
    enabled: bool,
    all_layers: bool,
    auto_clean: bool,
    auto_load: bool,
    wetness: f64,
    pickup: f64,
    mix: f64,
    load: f64,
    deposit: f64,
    consumption: f64,
}
#[derive(Clone, Copy, Debug)]
pub struct WetParameters {
    settings: WetSettings,
}
impl WetParameters {
    pub fn from_core(settings: WetSettings) -> Self {
        Self { settings }
    }
    pub fn core(self) -> WetSettings {
        self.settings
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<WetParameters>, D::Error> {
    WetParameters::deserialize(d).map(Some)
}
impl Serialize for WetParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let p = self.settings;
        let m = p.mix();
        Data {
            model: WET_CARRIER_MODEL.into(),
            enabled: p.enabled(),
            all_layers: p.all_layers(),
            auto_clean: p.auto_clean(),
            auto_load: p.auto_load(),
            wetness: f64::from(m.wetness()),
            pickup: f64::from(m.pickup()),
            mix: f64::from(m.mix()),
            load: f64::from(m.load()),
            deposit: f64::from(m.deposit()),
            consumption: f64::from(m.consumption()),
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for WetParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let p = Data::deserialize(d)?;
        if p.model != WET_CARRIER_MODEL
            || [p.wetness, p.pickup, p.mix, p.load, p.deposit, p.consumption]
                .iter()
                .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
        {
            return Err(serde::de::Error::custom("wet mix model or ratio"));
        }
        let mix = WetMix::new(
            p.wetness as f32,
            p.pickup as f32,
            p.mix as f32,
            p.load as f32,
            p.deposit as f32,
            p.consumption as f32,
        )
        .map_err(serde::de::Error::custom)?;
        Ok(Self::from_core(WetSettings::new(
            p.enabled,
            p.all_layers,
            p.auto_clean,
            p.auto_load,
            mix,
        )))
    }
}
