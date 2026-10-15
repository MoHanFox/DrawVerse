//! Field-level basic conversion. A compilable baseline is not full Photoshop equivalence.
use crate::{
    abr_association::LinkedPreset,
    check,
    ps_descriptor::{Descriptor, Value},
    Error, Result,
};
use paint_core::{BrushProgram, BrushSettings, BrushTip, PressureControl, PressureCurve};
use paint_task::CancellationToken;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Mapped,
    Approximate,
    NotExecuted,
}
#[derive(Debug)]
pub struct Issue {
    pub path: String,
    pub status: Status,
    pub reason: &'static str,
}
pub struct ConvertedPreset<'a> {
    pub converter_version: u32,
    pub name: &'a str,
    pub settings: BrushSettings,
    pub source: &'a Descriptor,
    pub issues: Vec<Issue>,
}
impl ConvertedPreset<'_> {
    pub fn is_partial(&self) -> bool {
        self.issues.iter().any(|i| i.status != Status::Mapped)
    }
}
fn issue(issues: &mut Vec<Issue>, path: impl Into<String>, status: Status, reason: &'static str) {
    issues.push(Issue {
        path: path.into(),
        status,
        reason,
    });
}
fn unit(source: &Descriptor, key: &[u8], expected: [u8; 4], low: f64, high: f64) -> Result<f32> {
    let (unit, value) = match source.get(key) {
        Some(Value::UnitDouble { unit, value }) => (*unit, *value),
        Some(Value::UnitFloat { unit, value }) => (*unit, f64::from(*value)),
        _ => {
            return Err(Error::Invalid(
                "ABR required unit parameter missing or wrong type",
            ))
        }
    };
    if unit != expected {
        return Err(Error::Unsupported(format!(
            "ABR parameter {:?} unit {:?}",
            String::from_utf8_lossy(key),
            unit
        )));
    }
    if !value.is_finite() || !(low..=high).contains(&value) {
        return Err(Error::Limit("ABR parameter outside executable range"));
    }
    Ok(value as f32)
}
fn flag(source: &Descriptor, key: &[u8]) -> Result<Option<bool>> {
    match source.get(key) {
        None => Ok(None),
        Some(Value::Boolean(value)) => Ok(Some(*value)),
        _ => Err(Error::Invalid("ABR expected boolean parameter")),
    }
}
fn path_key(key: &[u8]) -> String {
    if key.is_ascii() && !key.iter().any(|b| b.is_ascii_control()) {
        String::from_utf8_lossy(key).into_owned()
    } else {
        format!(
            "[hex:{}]",
            key.iter().map(|b| format!("{b:02x}")).collect::<String>()
        )
    }
}
fn unmapped(
    issues: &mut Vec<Issue>,
    path: String,
    value: &Value,
    token: &CancellationToken,
    depth: usize,
) -> Result<()> {
    unmapped_except(issues, path, value, token, depth, &[])
}
fn unmapped_except(
    issues: &mut Vec<Issue>,
    path: String,
    value: &Value,
    token: &CancellationToken,
    depth: usize,
    handled: &[String],
) -> Result<()> {
    check(token)?;
    if handled.contains(&path) {
        return Ok(());
    }
    if depth > 32 {
        return Err(Error::Limit("ABR report depth"));
    }
    match value {
        Value::Object { descriptor, .. } if !descriptor.entries.is_empty() => {
            for (key, value) in &descriptor.entries {
                unmapped_except(
                    issues,
                    format!("{path}.{}", path_key(key)),
                    value,
                    token,
                    depth + 1,
                    handled,
                )?;
            }
        }
        Value::List(values) if !values.is_empty() => {
            for (index, value) in values.iter().enumerate() {
                unmapped_except(
                    issues,
                    format!("{path}[{index}]"),
                    value,
                    token,
                    depth + 1,
                    handled,
                )?;
            }
        }
        _ => {
            if issues.len() >= 4096
                || issues.iter().map(|i| i.path.len()).sum::<usize>() + path.len() > 1024 * 1024
            {
                return Err(Error::Limit("ABR report budget"));
            }
            issue(
                issues,
                path,
                Status::NotExecuted,
                "源字段保留，尚未映射到执行参数",
            );
        }
    }
    Ok(())
}
pub fn convert<'a>(
    linked: &[LinkedPreset<'a>],
    token: &CancellationToken,
) -> Result<Vec<ConvertedPreset<'a>>> {
    convert_inner(linked, token, 3)
}
/// Preserve the immutable baseline and report semantics of converter v1.
pub fn convert_legacy<'a>(
    linked: &[LinkedPreset<'a>],
    token: &CancellationToken,
) -> Result<Vec<ConvertedPreset<'a>>> {
    convert_inner(linked, token, 1)
}
/// Preserve converter v2 opacity semantics without interpreting newer dynamics.
pub fn convert_v2<'a>(
    linked: &[LinkedPreset<'a>],
    token: &CancellationToken,
) -> Result<Vec<ConvertedPreset<'a>>> {
    convert_inner(linked, token, 2)
}
/// Only pressure fields are portable here; other dynamics stay in their own DTOs.
pub(crate) fn pressure_parameters(
    settings: &BrushSettings,
    version: u32,
) -> Option<crate::brush_pressure::PressureParameters> {
    (version >= 3).then(|| crate::brush_pressure::PressureParameters::from_core(&settings.dynamics))
}
fn pressure_controls(
    preset: &Descriptor,
    tool: Option<&Descriptor>,
    settings: &mut BrushSettings,
    issues: &mut Vec<Issue>,
) -> Result<Vec<String>> {
    let mut handled = Vec::new();
    for (flag_key, key, override_key) in [
        (
            b"useTipDynamics".as_slice(),
            b"szVr".as_slice(),
            Some(b"usePressureOverridesSize".as_slice()),
        ),
        (b"usePaintDynamics".as_slice(), b"prVr".as_slice(), None),
        (
            b"usePaintDynamics".as_slice(),
            b"opVr".as_slice(),
            Some(b"usePressureOverridesOpacity".as_slice()),
        ),
    ] {
        if flag(preset, flag_key)? != Some(true) {
            continue;
        }
        // Overrides are distinct saved tool state, not proven equivalent to preset controls.
        if tool.is_some_and(|t| t.get(key).is_some())
            || override_key.is_some_and(|k| {
                tool.and_then(|t| t.get(k))
                    .is_some_and(|v| !matches!(v, Value::Boolean(false)))
            })
        {
            continue;
        }
        // A saved brush pose can replace the actual device pressure.
        if flag(preset, b"useBrushPose")? == Some(true) {
            continue;
        }
        let dynamic = match preset.get(key) {
            None => continue,
            Some(Value::Object { descriptor, .. }) => descriptor,
            _ => return Err(Error::Invalid("ABR pressure dynamics object")),
        };
        let control = match dynamic.get(b"bVTy") {
            Some(Value::Integer(value)) => *value,
            _ => return Err(Error::Invalid("ABR pressure control type")),
        };
        if control != 2 {
            continue;
        }
        // Validate known values before narrowing. Nonzero jitter remains unexecuted.
        let minimum = unit(dynamic, b"Mnm ", *b"#Prc", 0., 100.)? / 100.;
        let jitter = unit(dynamic, b"jitter", *b"#Prc", 0., 100.)?;
        let steps = match dynamic.get(b"fStp") {
            None => false,
            Some(Value::Integer(n)) if *n >= 0 => true,
            _ => return Err(Error::Invalid("ABR fade step type/range")),
        };
        let name = path_key(key);
        let effective = if key == b"szVr" && preset.get(b"minimumDiameter").is_some() {
            let value = unit(preset, b"minimumDiameter", *b"#Prc", 0., 100.)? / 100.;
            issue(
                issues,
                "minimumDiameter",
                Status::Mapped,
                "尺寸压感最低直径比例",
            );
            handled.push("minimumDiameter".into());
            value
        } else {
            minimum
        };
        let response = PressureControl::new(PressureCurve::default(), effective)?;
        match key {
            b"szVr" => {
                settings.tip.pressure_size = true;
                settings.dynamics.size = response;
            }
            b"prVr" => {
                settings.tip.pressure_flow = true;
                settings.dynamics.flow = response;
            }
            b"opVr" => {
                settings.dynamics.pressure_opacity = true;
                settings.dynamics.opacity = response;
            }
            _ => unreachable!(),
        }
        let gate = path_key(flag_key);
        if !handled.contains(&gate) {
            issue(
                issues,
                &gate,
                Status::Mapped,
                "开启已支持的笔压通道，其他动态字段单独报告",
            );
            handled.push(gate);
        }
        for (field, status, reason) in [
            (
                "bVTy",
                Status::Mapped,
                "笔压控制；使用独立线性响应与最低比例",
            ),
            (
                "Mnm ",
                if minimum == effective {
                    Status::Mapped
                } else {
                    Status::NotExecuted
                },
                if minimum == effective {
                    "笔压最低比例"
                } else {
                    "尺寸使用minimumDiameter；不同的动态Mnm值保留但不执行"
                },
            ),
            (
                "jitter",
                if jitter == 0. {
                    Status::Mapped
                } else {
                    Status::NotExecuted
                },
                if jitter == 0. {
                    "未启用随机抖动"
                } else {
                    "随机抖动尚未转换；独立笔压分量已执行"
                },
            ),
        ] {
            let path = format!("{name}.{field}");
            issue(issues, &path, status, reason);
            handled.push(path);
        }
        if steps {
            let path = format!("{name}.fStp");
            issue(issues, &path, Status::Mapped, "笔压控制不使用渐隐步数");
            handled.push(path);
        }
    }
    Ok(handled)
}
fn convert_inner<'a>(
    linked: &[LinkedPreset<'a>],
    token: &CancellationToken,
    generation: u32,
) -> Result<Vec<ConvertedPreset<'a>>> {
    check(token)?;
    if linked.is_empty() || linked.len() > 1024 {
        return Err(Error::Limit("ABR converted preset count"));
    }
    let mut output = Vec::new();
    let mut report_bytes = 0usize;
    for item in linked {
        check(token)?;
        if item.name.trim().is_empty() || item.name.chars().count() > 80 || item.name.contains('\0')
        {
            return Err(Error::Invalid("ABR name outside native preset constraints"));
        }
        let mut issues = Vec::new();
        let shape = item.shape;
        if !matches!(
            shape.class.id.as_slice(),
            b"computedBrush" | b"sampledBrush"
        ) {
            return Err(Error::Unsupported("ABR shape conversion".into()));
        }
        if (shape.class.id == b"sampledBrush") != item.sampled_tip.is_some() {
            return Err(Error::Invalid("ABR shape/resource association"));
        }
        let diameter = unit(shape, b"Dmtr", *b"#Pxl", 1., 512.)?;
        let angle = unit(shape, b"Angl", *b"#Ang", -180., 180.)?;
        let roundness = unit(shape, b"Rndn", *b"#Prc", 1., 100.)? / 100.;
        let spacing = unit(shape, b"Spcn", *b"#Prc", 1., 50.)? / 100.;
        let hardness = if item.sampled_tip.is_none() {
            unit(shape, b"Hrdn", *b"#Prc", 0., 100.)? / 100.
        } else {
            1.
        };
        let mut settings = BrushSettings {
            wet: Default::default(),
            smoothing: Default::default(),
            texture: Default::default(),
            dual: Default::default(),
            dynamics: Default::default(),
            diameter,
            spacing,
            opacity: 1.,
            tip: BrushTip {
                hardness,
                roundness,
                angle,
                flow: 1.,
                pressure_size: false,
                pressure_flow: false,
            },
            mask: item.sampled_tip.map(|tip| tip.mask.clone()),
        };
        for key in ["Dmtr", "Rndn", "Spcn"] {
            issue(
                &mut issues,
                format!("Brsh.{key}"),
                Status::Mapped,
                "按源单位转换到真实原生参数",
            );
        }
        issue(
            &mut issues,
            "Brsh.Angl",
            if angle == 0. {
                Status::Mapped
            } else {
                Status::Approximate
            },
            if angle == 0. {
                "零角度"
            } else {
                "保留角度数值；非零方向尚未完成PS像素核对"
            },
        );
        issue(
            &mut issues,
            if settings.mask.is_some() {
                "Brsh.sampledData"
            } else {
                "Brsh.Hrdn"
            },
            Status::Mapped,
            if settings.mask.is_some() {
                "共享源覆盖资源，硬度来自图片"
            } else {
                "程序硬度比例"
            },
        );
        if let Some(value) = flag(shape, b"Intr")? {
            issue(
                &mut issues,
                "Brsh.Intr",
                if value {
                    Status::Mapped
                } else {
                    Status::NotExecuted
                },
                if value {
                    "使用源间距"
                } else {
                    "连续无间距行为尚未实现；基线使用保存的间距"
                },
            );
        } else {
            issue(
                &mut issues,
                "Brsh.Intr",
                Status::Approximate,
                "源未声明间距开关，基线使用保存的间距",
            );
        }
        for key in [b"flipX", b"flipY"] {
            if let Some(value) = flag(shape, key)? {
                issue(
                    &mut issues,
                    format!("Brsh.{}", path_key(key)),
                    if value {
                        Status::NotExecuted
                    } else {
                        Status::Mapped
                    },
                    if value {
                        "翻转尚未执行，源值保留"
                    } else {
                        "未启用翻转"
                    },
                );
            } else {
                issue(
                    &mut issues,
                    format!("Brsh.{}", path_key(key)),
                    Status::Approximate,
                    "源未声明翻转，基线不翻转",
                );
            }
        }
        if item.sampled_tip.is_some() {
            if let Some(value) = shape.get(b"Hrdn") {
                unmapped(&mut issues, "Brsh.Hrdn".into(), value, token, 0)?;
            }
        }
        if item.sampled_tip.is_none() {
            if let Some(value) = shape.get(b"sampledData") {
                unmapped(&mut issues, "Brsh.sampledData".into(), value, token, 0)?;
            }
        }
        if shape.get(b"Nm  ").is_some() {
            issue(
                &mut issues,
                "Brsh.Nm  ",
                Status::Mapped,
                "源笔尖名称保留，显示使用预设名称",
            );
        }
        for (key, value) in &shape.entries {
            if ![
                b"Dmtr".as_slice(),
                b"Angl",
                b"Rndn",
                b"Spcn",
                b"Hrdn",
                b"Intr",
                b"flipX",
                b"flipY",
                b"sampledData",
                b"Nm  ",
            ]
            .contains(&key.as_slice())
            {
                unmapped(
                    &mut issues,
                    format!("Brsh.{}", path_key(key)),
                    value,
                    token,
                    0,
                )?;
            }
        }
        let tool = match item.preset.get(b"toolOptions") {
            None => None,
            Some(Value::Object { descriptor, .. }) => Some(descriptor.as_ref()),
            _ => return Err(Error::Invalid("ABR toolOptions object")),
        };
        let handled = if generation >= 3 {
            pressure_controls(item.preset, tool, &mut settings, &mut issues)?
        } else {
            Vec::new()
        };
        for (key, value) in &item.preset.entries {
            if key != b"Nm  " && key != b"Brsh" && key != b"toolOptions" {
                if [
                    b"useTipDynamics".as_slice(),
                    b"usePaintDynamics",
                    b"minimumDiameter",
                    b"szVr",
                    b"prVr",
                    b"opVr",
                ]
                .contains(&key.as_slice())
                {
                    unmapped_except(&mut issues, path_key(key), value, token, 0, &handled)?;
                } else {
                    unmapped(&mut issues, path_key(key), value, token, 0)?;
                }
            }
        }
        let scalar = |key: &[u8]| -> Result<Option<f64>> {
            match tool.and_then(|t| t.get(key)) {
                None => Ok(None),
                Some(Value::Integer(v)) if (0..=100).contains(v) => Ok(Some(f64::from(*v) / 100.)),
                Some(Value::Double(v)) if v.is_finite() && (0. ..=100.).contains(v) => {
                    Ok(Some(*v / 100.))
                }
                _ => Err(Error::Invalid("ABR tool percentage")),
            }
        };
        if let Some(flow) = scalar(b"flow")? {
            settings.tip.flow = flow as f32;
            issue(
                &mut issues,
                "toolOptions.flow",
                Status::Mapped,
                "源流量百分比",
            );
        } else {
            issue(
                &mut issues,
                "toolOptions.flow",
                Status::Approximate,
                "源未保存工具流量，基线使用100%",
            );
        }
        let mut converter_version = 1;
        match scalar(b"Opct")? {
            Some(opacity) if generation >= 2 && opacity < 1. => {
                settings.opacity = opacity as f32;
                converter_version = 2;
                issue(
                    &mut issues,
                    "toolOptions.Opct",
                    Status::Mapped,
                    "源整笔不透明度百分比；密集重叠使用整笔上限",
                );
            }
            Some(1.) => issue(
                &mut issues,
                "toolOptions.Opct",
                Status::Mapped,
                "100%不透明度",
            ),
            Some(_) => issue(
                &mut issues,
                "toolOptions.Opct",
                Status::NotExecuted,
                "PS整笔透明度尚未实现，不使用印章透明度替代；基线100%",
            ),
            None => issue(
                &mut issues,
                "toolOptions.Opct",
                Status::Approximate,
                "源未保存工具不透明度，基线100%",
            ),
        }
        if !handled.is_empty() {
            converter_version = 3;
        }
        if let Some(tool) = tool {
            for (key, value) in &tool.entries {
                if key != b"flow" && key != b"Opct" {
                    unmapped(
                        &mut issues,
                        format!("toolOptions.{}", path_key(key)),
                        value,
                        token,
                        0,
                    )?;
                }
            }
        }
        if issues.len() > 4096 || issues.iter().map(|i| i.path.len()).sum::<usize>() > 1024 * 1024 {
            return Err(Error::Limit("ABR report budget"));
        }
        // Bound accumulation before source serialization; never return truncated reports.
        report_bytes = report_bytes.saturating_add(
            issues
                .iter()
                .map(|i| i.path.len() + i.reason.len() + 64)
                .sum::<usize>(),
        );
        if report_bytes > 32 * 1024 * 1024 {
            return Err(Error::Limit("ABR batch report budget"));
        }
        BrushProgram::compile(settings.definition())?;
        output.push(ConvertedPreset {
            converter_version,
            name: item.name,
            settings,
            source: item.preset,
            issues,
        });
    }
    check(token)?;
    Ok(output)
}
