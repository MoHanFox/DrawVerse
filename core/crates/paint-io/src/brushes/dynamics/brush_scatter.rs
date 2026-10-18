use paint_core::{ScatterControl, ScatterControlSource, ScatterDynamics};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ControlData {
    source: String,
    minimum: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    fade_steps: u32,
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Data {
    enabled: bool,
    amount: f32,
    both_axes: bool,
    count: u32,
    count_jitter: f32,
    seed: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    amount_control: Option<ControlData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    count_control: Option<ControlData>,
}
#[derive(Clone, Copy, Debug)]
pub struct ScatterParameters {
    scatter: ScatterDynamics,
}
impl ScatterParameters {
    pub fn from_core(scatter: ScatterDynamics) -> Self {
        Self { scatter }
    }
    pub fn core(self) -> ScatterDynamics {
        self.scatter
    }
    pub fn has_controls(self) -> bool {
        self.scatter.has_controls()
    }
}
pub fn present<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Option<ScatterParameters>, D::Error> {
    ScatterParameters::deserialize(d).map(Some)
}
impl Serialize for ScatterParameters {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let c = self.scatter;
        Data {
            enabled: c.enabled(),
            amount: c.amount(),
            both_axes: c.both_axes(),
            count: c.count(),
            count_jitter: c.count_jitter(),
            seed: format!("{:016x}", c.seed()),
            amount_control: control_data(c.amount_control()),
            count_control: control_data(c.count_control()),
        }
        .serialize(s)
    }
}
fn control_data(control: ScatterControl) -> Option<ControlData> {
    if control.is_default() {
        return None;
    }
    let source = match control.source() {
        ScatterControlSource::Off => "off",
        ScatterControlSource::Fade => "fade",
        ScatterControlSource::Pressure => "pressure",
        ScatterControlSource::Tilt => "tilt",
        ScatterControlSource::Direction => "direction",
        ScatterControlSource::Rotation => "rotation",
    };
    Some(ControlData {
        source: source.into(),
        minimum: control.minimum(),
        fade_steps: control.fade_steps(),
    })
}
fn parse_control(data: Option<ControlData>) -> std::result::Result<ScatterControl, String> {
    let Some(data) = data else {
        return Ok(ScatterControl::off());
    };
    let source = match data.source.as_str() {
        "off" => ScatterControlSource::Off,
        "fade" => ScatterControlSource::Fade,
        "pressure" => ScatterControlSource::Pressure,
        "tilt" => ScatterControlSource::Tilt,
        "direction" => ScatterControlSource::Direction,
        "rotation" => ScatterControlSource::Rotation,
        _ => return Err("scatter control source".into()),
    };
    ScatterControl::new(source, data.minimum, data.fade_steps).map_err(|e| e.to_string())
}
impl<'de> Deserialize<'de> for ScatterParameters {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let data = Data::deserialize(d)?;
        if data.seed.len() != 16
            || !data
                .seed
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(serde::de::Error::custom(
                "scatter seed must be 16 lowercase hex digits",
            ));
        }
        let seed = u64::from_str_radix(&data.seed, 16).map_err(serde::de::Error::custom)?;
        let amount_control =
            parse_control(data.amount_control).map_err(serde::de::Error::custom)?;
        let count_control = parse_control(data.count_control).map_err(serde::de::Error::custom)?;
        ScatterDynamics::new(
            data.enabled,
            data.amount,
            data.both_axes,
            data.count,
            data.count_jitter,
            seed,
        )
        .and_then(|scatter| scatter.with_controls(amount_control, count_control))
        .map(Self::from_core)
        .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_json_omits_controls_and_round_trips_unchanged() {
        let bytes = br#"{"enabled":true,"amount":2,"bothAxes":true,"count":4,"countJitter":0.25,"seed":"0000000000000007"}"#;
        let parameters: ScatterParameters = serde_json::from_slice(bytes).unwrap();
        assert!(!parameters.has_controls());
        let encoded = serde_json::to_vec(&parameters).unwrap();
        assert!(!String::from_utf8_lossy(&encoded).contains("Control"));
        let again: ScatterParameters = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(again.core(), parameters.core());
    }

    #[test]
    fn controls_serialize_without_off_fields_and_restore_core_values() {
        let scatter = ScatterDynamics::new(true, 2., true, 4, 0.25, 7)
            .unwrap()
            .with_controls(
                ScatterControl::new(ScatterControlSource::Pressure, 0.25, 0).unwrap(),
                ScatterControl::new(ScatterControlSource::Fade, 0.5, 12).unwrap(),
            )
            .unwrap();
        let parameters = ScatterParameters::from_core(scatter);
        let encoded = serde_json::to_vec(&parameters).unwrap();
        let text = String::from_utf8_lossy(&encoded);
        assert!(text.contains("\"amountControl\""));
        assert!(text.contains("\"countControl\""));
        assert!(text.contains("\"fadeSteps\":12"));
        let restored: ScatterParameters = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored.core(), scatter);
        assert_eq!(
            restored.core().amount_control().source(),
            ScatterControlSource::Pressure
        );
        assert_eq!(restored.core().count_control().fade_steps(), 12);
    }

    #[test]
    fn invalid_control_json_is_rejected() {
        for control in [
            r#"{"source":"pressure","minimum":-0.1}"#,
            r#"{"source":"fade","minimum":0,"fadeSteps":0}"#,
            r#"{"source":"pressure","minimum":0,"fadeSteps":2}"#,
            r#"{"source":"unknown","minimum":0}"#,
        ] {
            let text = format!(
                r#"{{"enabled":true,"amount":1,"bothAxes":false,"count":2,"countJitter":0,"seed":"0000000000000000","amountControl":{control}}}"#
            );
            assert!(
                serde_json::from_str::<ScatterParameters>(&text).is_err(),
                "{text}"
            );
        }
    }
}
