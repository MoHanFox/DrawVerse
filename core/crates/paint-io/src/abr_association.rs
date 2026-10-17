//! Link validated typed source presets to immutable sampled resources, without defaults.
use crate::{
    abr_sampled::{SectionedAbr, SectionedTip},
    check,
    ps_descriptor::{Descriptor, Value},
    Error, Result,
};
use paint_task::CancellationToken;
use std::collections::BTreeMap;
pub struct LinkedPreset<'a> {
    pub name: &'a str,
    pub preset: &'a Descriptor,
    pub shape: &'a Descriptor,
    pub sampled_tip: Option<&'a SectionedTip>,
    pub dual_sampled_tip: Option<&'a SectionedTip>,
}
pub fn source_text(value: &Value) -> Result<&str> {
    let Value::Text(text) = value else {
        return Err(Error::Invalid("ABR expected descriptor text"));
    };
    let text = text.strip_suffix('\0').unwrap_or(text);
    if text.is_empty() || text.contains('\0') {
        return Err(Error::Invalid("ABR descriptor text identity/name"));
    }
    Ok(text)
}
fn object(value: &Value) -> Result<&Descriptor> {
    let Value::Object { descriptor, .. } = value else {
        return Err(Error::Invalid("ABR expected descriptor object"));
    };
    Ok(descriptor)
}
pub fn associate<'a>(
    source: &'a SectionedAbr,
    descriptors: &'a [Descriptor],
    token: &CancellationToken,
) -> Result<Vec<LinkedPreset<'a>>> {
    check(token)?;
    let mut samples = BTreeMap::new();
    for tip in &source.tips {
        if samples.insert(tip.id.as_str(), tip).is_some() {
            return Err(Error::Invalid("duplicate ABR sample identity"));
        }
    }
    let mut presets = Vec::new();
    for root in descriptors {
        let Some(Value::List(items)) = root.get(b"Brsh") else {
            return Err(Error::Invalid("ABR root brush list missing"));
        };
        if items.len() > 1024 - presets.len() {
            return Err(Error::Limit("ABR preset count"));
        }
        for item in items {
            check(token)?;
            let preset = object(item)?;
            let name = source_text(
                preset
                    .get(b"Nm  ")
                    .ok_or(Error::Invalid("ABR preset name missing"))?,
            )?;
            let shape = object(
                preset
                    .get(b"Brsh")
                    .ok_or(Error::Invalid("ABR shape missing"))?,
            )?;
            let sampled_tip = match shape.class.id.as_slice() {
                b"computedBrush" => None,
                b"sampledBrush" => {
                    let id = source_text(
                        shape
                            .get(b"sampledData")
                            .ok_or(Error::Invalid("ABR sample reference missing"))?,
                    )?;
                    Some(
                        *samples
                            .get(id)
                            .ok_or(Error::Invalid("ABR sample reference not found"))?,
                    )
                }
                _ => {
                    return Err(Error::Unsupported(format!(
                        "ABR brush shape {:?}",
                        String::from_utf8_lossy(&shape.class.id)
                    )))
                }
            };
            let dual_sampled_tip = match preset.get(b"dualBrush") {
                Some(Value::Object { descriptor, .. })
                    if matches!(descriptor.get(b"useDualBrush"), Some(Value::Boolean(true))) =>
                {
                    let dual_shape = match descriptor.get(b"Brsh") {
                        Some(Value::Object { descriptor, .. }) => descriptor,
                        _ => return Err(Error::Invalid("ABR dual shape missing")),
                    };
                    if dual_shape.class.id == b"sampledBrush" {
                        let id = source_text(
                            dual_shape
                                .get(b"sampledData")
                                .ok_or(Error::Invalid("ABR dual sample reference missing"))?,
                        )?;
                        Some(
                            *samples
                                .get(id)
                                .ok_or(Error::Invalid("ABR dual sample reference not found"))?,
                        )
                    } else {
                        None
                    }
                }
                _ => None,
            };
            presets.push(LinkedPreset {
                name,
                preset,
                shape,
                sampled_tip,
                dual_sampled_tip,
            });
        }
    }
    if presets.is_empty() {
        return Err(Error::Invalid("ABR preset list empty"));
    }
    Ok(presets)
}
