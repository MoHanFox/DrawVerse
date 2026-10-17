//! Field-level basic conversion. A compilable baseline is not full Photoshop equivalence.
use crate::{
    abr_association::LinkedPreset,
    abr_patterns::{PatternResource, TextureSource},
    check,
    ps_descriptor::{Descriptor, Value},
    Error, Result,
};
use paint_core::{
    AngleControl, BrushProgram, BrushSettings, BrushTexture, BrushTip, ColorDynamics,
    DirectionDynamics, DynamicsSource, DynamicsSourceMatrix, PressureControl, PressureCurve,
    RotationDynamics, ScatterDynamics, ShapeJitter, TextureMode, TextureResource, TextureSettings,
    TextureSpace, TiltDynamics, TipDynamics,
};
use paint_task::CancellationToken;
type TextureLookup<'a> = Option<(
    &'a std::collections::BTreeMap<usize, TextureSource>,
    &'a std::collections::BTreeMap<String, PatternResource>,
)>;
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
    convert_inner(linked, token, 7, None)
}
/// Import ABR texture parameters and their embedded pattern resources.
pub fn convert_textured<'a>(
    linked: &[LinkedPreset<'a>],
    textures: &std::collections::BTreeMap<usize, TextureSource>,
    patterns: &std::collections::BTreeMap<String, PatternResource>,
    token: &CancellationToken,
) -> Result<Vec<ConvertedPreset<'a>>> {
    convert_inner(linked, token, 8, Some((textures, patterns)))
}
/// Preserve converter v7 behavior without ABR texture conversion.
pub fn convert_v7<'a>(
    linked: &[LinkedPreset<'a>],
    token: &CancellationToken,
) -> Result<Vec<ConvertedPreset<'a>>> {
    convert_inner(linked, token, 7, None)
}
/// Preserve converter v6 shape jitter behavior and reports without device conversion.
pub fn convert_v6<'a>(
    linked: &[LinkedPreset<'a>],
    token: &CancellationToken,
) -> Result<Vec<ConvertedPreset<'a>>> {
    convert_inner(linked, token, 6, None)
}
/// Preserve converter v5 color behavior and reports without shape jitter conversion.
pub fn convert_v5<'a>(
    linked: &[LinkedPreset<'a>],
    token: &CancellationToken,
) -> Result<Vec<ConvertedPreset<'a>>> {
    convert_inner(linked, token, 5, None)
}
/// Preserve converter v4 scatter behavior and reports without color conversion.
pub fn convert_v4<'a>(
    linked: &[LinkedPreset<'a>],
    token: &CancellationToken,
) -> Result<Vec<ConvertedPreset<'a>>> {
    convert_inner(linked, token, 4, None)
}
/// Preserve the immutable baseline and report semantics of converter v1.
pub fn convert_legacy<'a>(
    linked: &[LinkedPreset<'a>],
    token: &CancellationToken,
) -> Result<Vec<ConvertedPreset<'a>>> {
    convert_inner(linked, token, 1, None)
}
/// Preserve converter v2 opacity semantics without interpreting newer dynamics.
pub fn convert_v2<'a>(
    linked: &[LinkedPreset<'a>],
    token: &CancellationToken,
) -> Result<Vec<ConvertedPreset<'a>>> {
    convert_inner(linked, token, 2, None)
}
/// Preserve converter v3 pressure behavior and reports without scatter conversion.
pub fn convert_v3<'a>(
    linked: &[LinkedPreset<'a>],
    token: &CancellationToken,
) -> Result<Vec<ConvertedPreset<'a>>> {
    convert_inner(linked, token, 3, None)
}
pub(crate) fn pressure_parameters(
    settings: &BrushSettings,
    version: u32,
) -> Option<crate::brush_pressure::PressureParameters> {
    let mut pressure = settings.dynamics.clone();
    pressure.scatter = Default::default();
    pressure.color = Default::default();
    pressure.shape_jitter = Default::default();
    pressure.tip = Default::default();
    (version >= 3
        && (version == 3
            || pressure != Default::default()
            || settings.tip.pressure_size
            || settings.tip.pressure_flow))
        .then(|| crate::brush_pressure::PressureParameters::from_core(&pressure))
}
pub(crate) fn scatter_parameters(
    settings: &BrushSettings,
    version: u32,
) -> Option<crate::brush_scatter::ScatterParameters> {
    (version >= 4 && (version == 4 || settings.dynamics.scatter != Default::default()))
        .then(|| crate::brush_scatter::ScatterParameters::from_core(settings.dynamics.scatter))
}
pub(crate) fn color_parameters(
    settings: &BrushSettings,
    version: u32,
) -> Option<crate::brush_color_dynamics::ColorParameters> {
    (version >= 5 && (version == 5 || settings.dynamics.color != Default::default()))
        .then(|| crate::brush_color_dynamics::ColorParameters::from_core(settings.dynamics.color))
}
pub(crate) fn shape_parameters(
    settings: &BrushSettings,
    version: u32,
) -> Option<crate::brush_shape_jitter::ShapeParameters> {
    (version >= 6 && (version == 6 || settings.dynamics.shape_jitter != Default::default())).then(
        || crate::brush_shape_jitter::ShapeParameters::from_core(settings.dynamics.shape_jitter),
    )
}
pub(crate) fn device_parameters(
    settings: &BrushSettings,
    version: u32,
) -> Option<crate::brush_tip_dynamics::TipParameters> {
    (version >= 7)
        .then(|| crate::brush_tip_dynamics::TipParameters::from_core(settings.dynamics.tip))
}
fn device_controls(
    preset: &Descriptor,
    tool: Option<&Descriptor>,
    settings: &mut BrushSettings,
    issues: &mut Vec<Issue>,
) -> Result<Vec<String>> {
    if flag(preset, b"useTipDynamics")? != Some(true) {
        return Ok(Vec::new());
    }
    let pose = flag(preset, b"useBrushPose")? == Some(true);
    let mut angle = AngleControl::Fixed;
    let mut tilt_roundness = false;
    let mut mapped = Vec::new();
    for (key, is_angle) in [
        (b"angleDynamics".as_slice(), true),
        (b"roundnessDynamics".as_slice(), false),
    ] {
        let data = match preset.get(key) {
            None => continue,
            Some(Value::Object { descriptor, .. }) => descriptor,
            _ => return Err(Error::Invalid("ABR device dynamics object")),
        };
        let control = match data.get(b"bVTy") {
            Some(Value::Integer(value)) => *value,
            _ => return Err(Error::Invalid("ABR device control type")),
        };
        unit(data, b"Mnm ", *b"#Prc", 0., 100.)?;
        unit(data, b"jitter", *b"#Prc", 0., 100.)?;
        if !matches!(data.get(b"fStp"), None | Some(Value::Integer(0..=i32::MAX))) {
            return Err(Error::Invalid("ABR device fade steps"));
        }
        if pose || tool.is_some_and(|t| t.get(key).is_some()) {
            continue;
        }
        if is_angle {
            angle = match control {
                6 => AngleControl::Direction,
                8 => AngleControl::PenRotation,
                3 => AngleControl::Tilt,
                _ => continue,
            };
        } else if control == 3 {
            tilt_roundness = true;
        } else {
            continue;
        }
        mapped.push((path_key(key), data.as_ref()));
    }
    if mapped.is_empty() {
        return Ok(Vec::new());
    }
    let minimum = if tilt_roundness && preset.get(b"minimumRoundness").is_some() {
        (unit(preset, b"minimumRoundness", *b"#Prc", 0., 100.)? / 100. / settings.tip.roundness)
            .clamp(0.01, 1.)
    } else {
        0.01
    };
    settings.dynamics.tip = TipDynamics::new(angle, tilt_roundness, minimum)?;
    let mut handled = Vec::new();
    let mut report = |path: String, status, reason| {
        if let Some(old) = issues.iter_mut().find(|i| i.path == path) {
            old.status = status;
            old.reason = reason;
        } else {
            issue(issues, &path, status, reason);
        }
        handled.push(path);
    };
    report(
        "useTipDynamics".into(),
        Status::Mapped,
        "形状动态开启；设备、压力与随机通道独立转换",
    );
    for (name, data) in mapped {
        report(
            format!("{name}.bVTy"),
            Status::Approximate,
            "原生方向/笔旋转/倾斜几何及设备缺省回退；PS设备公式未逐像素核对",
        );
        for field in ["Mnm ", "fStp"] {
            if data.get(field.as_bytes()).is_some() {
                report(
                    format!("{name}.{field}"),
                    Status::NotExecuted,
                    "该设备通道的最小/渐隐字段尚未解释，原值保留在来源",
                );
            }
        }
    }
    if tilt_roundness {
        report(
            "minimumRoundness".into(),
            Status::Approximate,
            "原生余弦倾斜下限，按基础圆度换算并裁剪；缺省使用1%，PS公式未核对",
        );
    }
    Ok(handled)
}
fn shape_controls(
    preset: &Descriptor,
    tool: Option<&Descriptor>,
    settings: &mut BrushSettings,
    issues: &mut Vec<Issue>,
) -> Result<Vec<String>> {
    let enabled = match flag(preset, b"useTipDynamics")? {
        None => return Ok(Vec::new()),
        Some(value) => value,
    };
    let keys = [
        b"szVr".as_slice(),
        b"angleDynamics",
        b"roundnessDynamics",
        b"minimumDiameter",
        b"minimumRoundness",
    ];
    if !enabled && keys.iter().any(|key| preset.get(key).is_none()) {
        return Ok(Vec::new());
    }
    let mut amounts = [0.; 3];
    let mut controls = Vec::new();
    for (i, key) in [b"szVr".as_slice(), b"angleDynamics", b"roundnessDynamics"]
        .into_iter()
        .enumerate()
    {
        let dynamic = match preset.get(key) {
            None => continue,
            Some(Value::Object { descriptor, .. }) => descriptor,
            _ => return Err(Error::Invalid("ABR shape dynamics object")),
        };
        let control = match dynamic.get(b"bVTy") {
            Some(Value::Integer(value)) => *value,
            _ => return Err(Error::Invalid("ABR shape control type")),
        };
        unit(dynamic, b"Mnm ", *b"#Prc", 0., 100.)?;
        if !matches!(
            dynamic.get(b"fStp"),
            None | Some(Value::Integer(0..=i32::MAX))
        ) {
            return Err(Error::Invalid("ABR shape fade steps"));
        }
        let amount = unit(dynamic, b"jitter", *b"#Prc", 0., 100.)? / 100.;
        // Tool overrides and saved poses need their own conversion semantics.
        let blocked = i == 0
            && (tool.is_some_and(|t| t.get(b"szVr").is_some())
                || tool
                    .and_then(|t| t.get(b"usePressureOverridesSize"))
                    .is_some_and(|v| !matches!(v, Value::Boolean(false)))
                || flag(preset, b"useBrushPose")? == Some(true));
        if !blocked {
            amounts[i] = amount;
        }
        controls.push((path_key(key), dynamic.as_ref(), control, amount, blocked));
    }
    let size_min = if preset.get(b"minimumDiameter").is_some() {
        unit(preset, b"minimumDiameter", *b"#Prc", 0., 100.)? / 100.
    } else {
        0.
    };
    let round_min = if preset.get(b"minimumRoundness").is_some() {
        unit(preset, b"minimumRoundness", *b"#Prc", 0., 100.)? / 100. / settings.tip.roundness
    } else {
        0.01
    };
    // No newly executable behavior: keep the old report and source version exactly.
    if enabled && amounts == [0.; 3] {
        return Ok(Vec::new());
    }
    let shape = ShapeJitter::new(
        enabled,
        amounts[0],
        size_min,
        amounts[1],
        amounts[2],
        round_min.clamp(0.01, 1.),
        0,
    )?;
    if shape == ShapeJitter::default() {
        return Ok(Vec::new());
    }
    let mut handled = Vec::new();
    let mut report = |path: String, status, reason| {
        if let Some(old) = issues.iter_mut().find(|i| i.path == path) {
            old.status = status;
            old.reason = reason;
        } else {
            issue(issues, &path, status, reason);
        }
        handled.push(path);
    };
    report(
        "useTipDynamics".into(),
        Status::Mapped,
        "形状动态开关；各设备通道独立报告",
    );
    for (key, native, fallback) in [
        ("minimumDiameter", size_min, 0.),
        ("minimumRoundness", round_min.clamp(0.01, 1.), 0.01),
    ] {
        if preset.get(key.as_bytes()).is_some() {
            report(
                key.into(),
                if enabled || (key == "minimumRoundness" && round_min != native) {
                    Status::Approximate
                } else {
                    Status::Mapped
                },
                if enabled {
                    "原生压感后相对随机下限；圆度按基础圆度换算且限制在内核范围，PS叠加公式尚未核对"
                } else if key == "minimumRoundness" && round_min != native {
                    "关闭配置的最低圆度按内核范围裁剪，未裁剪原值保留在来源"
                } else {
                    "完整关闭配置保留最低比例与原始来源"
                },
            );
        } else if native == fallback {
            report(
                key.into(),
                Status::Approximate,
                "源未保存随机最低比例，采用原生默认下限",
            );
        }
    }
    for (name, data, control, amount, blocked) in controls {
        report(
            format!("{name}.jitter"),
            if blocked {
                Status::NotExecuted
            } else if enabled && amount != 0. {
                Status::Approximate
            } else {
                Status::Mapped
            },
            if blocked {
                "工具覆盖或笔姿下的尺寸随机规则尚未执行"
            } else if enabled && amount != 0. {
                "原生确定性尺寸/角度/圆度随机缩小；PS随机分布及压感叠加尚未逐像素核对"
            } else {
                "零幅度或完整关闭配置不执行随机变化"
            },
        );
        // The pressure conversion already owns szVr control/minimum/fade when supported.
        if name == "szVr" && settings.tip.pressure_size {
            continue;
        }
        report(
            format!("{name}.bVTy"),
            if control == 0 {
                Status::Mapped
            } else {
                Status::NotExecuted
            },
            if control == 0 {
                "无设备调制，独立随机量仍由jitter决定"
            } else {
                "该形状设备或渐隐调制尚未执行，原字段保留"
            },
        );
        if control == 0 {
            for field in ["Mnm ", "fStp"] {
                if data.get(field.as_bytes()).is_some() {
                    report(
                        format!("{name}.{field}"),
                        Status::Mapped,
                        "无设备控制时不参与执行，原始字段保留在来源",
                    );
                }
            }
        }
    }
    settings.dynamics.shape_jitter = shape;
    Ok(handled)
}
fn color_controls(
    preset: &Descriptor,
    settings: &mut BrushSettings,
    issues: &mut Vec<Issue>,
) -> Result<Vec<String>> {
    let enabled = match flag(preset, b"useColorDynamics")? {
        None => return Ok(Vec::new()),
        Some(value) => value,
    };
    let keys = [
        b"clVr".as_slice(),
        b"H   ",
        b"Strt",
        b"Brgh",
        b"colorDynamicsPerTip",
    ];
    if !enabled && keys.iter().any(|key| preset.get(key).is_none()) {
        return Ok(Vec::new());
    }
    let dynamic = match preset.get(b"clVr") {
        Some(Value::Object { descriptor, .. }) => descriptor,
        _ => return Err(Error::Invalid("ABR color dynamics object")),
    };
    let control = match dynamic.get(b"bVTy") {
        Some(Value::Integer(value)) => *value,
        _ => return Err(Error::Invalid("ABR color control type")),
    };
    unit(dynamic, b"Mnm ", *b"#Prc", 0., 100.)?;
    if !matches!(
        dynamic.get(b"fStp"),
        None | Some(Value::Integer(0..=i32::MAX))
    ) {
        return Err(Error::Invalid("ABR color fade steps"));
    }
    let per_tip = flag(preset, b"colorDynamicsPerTip")?;
    let values = [
        unit(dynamic, b"jitter", *b"#Prc", 0., 100.)? / 100.,
        unit(preset, b"H   ", *b"#Prc", 0., 100.)? / 100.,
        unit(preset, b"Strt", *b"#Prc", 0., 100.)? / 100.,
        unit(preset, b"Brgh", *b"#Prc", 0., 100.)? / 100.,
    ];
    let color = ColorDynamics::new(
        enabled,
        values[0],
        values[1],
        values[2],
        values[3],
        per_tip.unwrap_or(false),
        0,
    )?;
    if color == ColorDynamics::default() {
        return Ok(Vec::new());
    }
    let mut handled = vec![
        "useColorDynamics".into(),
        "colorDynamicsPerTip".into(),
        "clVr.bVTy".into(),
    ];
    issue(
        issues,
        "useColorDynamics",
        Status::Mapped,
        "保存颜色动态启用状态；关闭配置不参与绘画",
    );
    issue(
        issues,
        "colorDynamicsPerTip",
        if per_tip.is_some() {
            Status::Mapped
        } else {
            Status::Approximate
        },
        if per_tip.is_some() {
            "逐笔或逐实际印章采用原生颜色动态频率"
        } else {
            "源未保存颜色频率，使用逐笔；PS缺省行为未核对"
        },
    );
    issue(
        issues,
        "clVr.bVTy",
        if control == 0 {
            Status::Mapped
        } else {
            Status::NotExecuted
        },
        if control == 0 {
            "无设备调制，前景背景混色幅度由jitter控制"
        } else {
            "设备或渐隐颜色调制尚未执行；源控制保留"
        },
    );
    if control == 0 {
        for field in ["Mnm ", "fStp"] {
            if dynamic.get(field.as_bytes()).is_some() {
                let path = format!("clVr.{field}");
                issue(
                    issues,
                    &path,
                    Status::Mapped,
                    "无设备控制时不参与执行，原值保留在来源",
                );
                handled.push(path);
            }
        }
    }
    for (path, value) in ["clVr.jitter", "H   ", "Strt", "Brgh"]
        .into_iter()
        .zip(values)
    {
        issue(
            issues,
            path,
            if !enabled || value == 0. {
                Status::Mapped
            } else {
                Status::Approximate
            },
            if !enabled {
                "完整关闭颜色配置保存，不执行颜色变化"
            } else if value == 0. {
                "颜色变化幅度为零"
            } else {
                "原生确定性随机与线性RGB/HSV；尚未与PS颜色空间及随机序列逐像素核对"
            },
        );
        handled.push(path.into());
    }
    settings.dynamics.color = color;
    Ok(handled)
}
fn scatter_controls(
    preset: &Descriptor,
    settings: &mut BrushSettings,
    issues: &mut Vec<Issue>,
) -> Result<Vec<String>> {
    let enabled = match flag(preset, b"useScatter")? {
        None => return Ok(Vec::new()),
        Some(value) => value,
    };
    let keys = [
        b"bothAxes".as_slice(),
        b"Cnt ",
        b"scatterDynamics",
        b"countDynamics",
    ];
    if !enabled && keys.iter().any(|key| preset.get(key).is_none()) {
        return Ok(Vec::new());
    }
    let both_axes = flag(preset, b"bothAxes")?.ok_or(Error::Invalid("ABR scatter axes missing"))?;
    let count = match preset.get(b"Cnt ") {
        Some(Value::Integer(n)) if (1..=16).contains(n) => *n as u32,
        Some(Value::Double(n)) if n.is_finite() && (1. ..=16.).contains(n) && n.fract() == 0. => {
            *n as u32
        }
        _ => return Err(Error::Invalid("ABR scatter count outside executable range")),
    };
    let dynamic = |key: &[u8]| -> Result<&Descriptor> {
        match preset.get(key) {
            Some(Value::Object { descriptor, .. }) => Ok(descriptor),
            _ => Err(Error::Invalid("ABR scatter dynamics object")),
        }
    };
    let amount = dynamic(b"scatterDynamics")?;
    let count_dynamic = dynamic(b"countDynamics")?;
    let scatter = ScatterDynamics::new(
        enabled,
        unit(amount, b"jitter", *b"#Prc", 0., 1000.)? / 100.,
        both_axes,
        count,
        unit(count_dynamic, b"jitter", *b"#Prc", 0., 100.)? / 100.,
        0,
    )?;
    // A complete disabled default contributes no new configuration or provenance.
    if scatter == ScatterDynamics::default() {
        return Ok(Vec::new());
    }
    let mut handled = Vec::new();
    for (path, reason) in [
        ("useScatter", "保存散布启用状态；关闭配置不参与绘画"),
        ("bothAxes", "原生单轴法线或文档双轴散布"),
        ("Cnt ", "每逻辑印章的子印章数量"),
    ] {
        issue(issues, path, Status::Mapped, reason);
        handled.push(path.into());
    }
    for (name, data, max) in [
        ("scatterDynamics", amount, 1000.),
        ("countDynamics", count_dynamic, 100.),
    ] {
        let control = match data.get(b"bVTy") {
            Some(Value::Integer(n)) => *n,
            _ => return Err(Error::Invalid("ABR scatter control type")),
        };
        unit(data, b"Mnm ", *b"#Prc", 0., 100.)?;
        if !matches!(data.get(b"fStp"), None | Some(Value::Integer(0..=i32::MAX))) {
            return Err(Error::Invalid("ABR scatter fade steps"));
        }
        let jitter = unit(data, b"jitter", *b"#Prc", 0., max)?;
        let path = format!("{name}.jitter");
        issue(
            issues,
            &path,
            if jitter == 0. || !enabled {
                Status::Mapped
            } else {
                Status::Approximate
            },
            if !enabled {
                "完整关闭配置保存，不执行随机分布"
            } else if jitter == 0. {
                "随机幅度为零"
            } else {
                "使用原生确定性随机分布；PS位移/数量随机序列尚未逐像素核对"
            },
        );
        handled.push(path);
        let path = format!("{name}.bVTy");
        issue(
            issues,
            &path,
            if control == 0 {
                Status::Mapped
            } else {
                Status::NotExecuted
            },
            if control == 0 {
                "无设备调制，随机量仍由jitter控制"
            } else {
                "设备/渐隐幅度或数量调制尚未执行；源控制保留"
            },
        );
        handled.push(path);
        if control == 0 {
            for field in ["Mnm ", "fStp"] {
                if data.get(field.as_bytes()).is_some() {
                    let path = format!("{name}.{field}");
                    issue(
                        issues,
                        &path,
                        Status::Mapped,
                        "无设备控制时不参与执行，原值保留在来源",
                    );
                    handled.push(path);
                }
            }
        }
    }
    settings.dynamics.scatter = scatter;
    Ok(handled)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DynamicsControl {
    Pressure,
    Tilt,
    Direction,
    Rotation,
}
impl DynamicsControl {
    fn from_wire(value: i32) -> Option<Self> {
        match value {
            2 => Some(Self::Pressure),
            3 => Some(Self::Tilt),
            6 => Some(Self::Direction),
            8 => Some(Self::Rotation),
            _ => None,
        }
    }
}
fn route_source(
    matrix: DynamicsSourceMatrix,
    target: &[u8],
    source: DynamicsSource,
) -> DynamicsSourceMatrix {
    match target {
        b"szVr" => DynamicsSourceMatrix::new(source, matrix.flow(), matrix.opacity()),
        b"prVr" => DynamicsSourceMatrix::new(matrix.size(), source, matrix.opacity()),
        b"opVr" => DynamicsSourceMatrix::new(matrix.size(), matrix.flow(), source),
        _ => matrix,
    }
}
fn tilt_channel(d: &TiltDynamics, target: &[u8], response: PressureControl) -> TiltDynamics {
    TiltDynamics::new(
        true,
        if target == b"szVr" {
            response.clone()
        } else {
            d.size().clone()
        },
        if target == b"prVr" {
            response.clone()
        } else {
            d.flow().clone()
        },
        if target == b"opVr" {
            response
        } else {
            d.opacity().clone()
        },
    )
}
fn rotation_channel(
    d: &RotationDynamics,
    target: &[u8],
    response: PressureControl,
) -> RotationDynamics {
    RotationDynamics::new(
        true,
        if target == b"szVr" {
            response.clone()
        } else {
            d.size().clone()
        },
        if target == b"prVr" {
            response.clone()
        } else {
            d.flow().clone()
        },
        if target == b"opVr" {
            response
        } else {
            d.opacity().clone()
        },
    )
}
fn direction_channel(
    d: &DirectionDynamics,
    target: &[u8],
    response: PressureControl,
) -> DirectionDynamics {
    DirectionDynamics::new(
        true,
        if target == b"szVr" {
            response.clone()
        } else {
            d.size().clone()
        },
        if target == b"prVr" {
            response.clone()
        } else {
            d.flow().clone()
        },
        if target == b"opVr" {
            response
        } else {
            d.opacity().clone()
        },
    )
}
fn pressure_controls(
    preset: &Descriptor,
    tool: Option<&Descriptor>,
    settings: &mut BrushSettings,
    issues: &mut Vec<Issue>,
) -> Result<(Vec<String>, bool)> {
    let mut handled = Vec::new();
    let mut source_changed = false;
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
            Some(Value::Integer(value)) => DynamicsControl::from_wire(*value),
            _ => return Err(Error::Invalid("ABR pressure control type")),
        };
        let Some(control) = control else {
            continue;
        };
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
        match (key, control) {
            (b"szVr", DynamicsControl::Pressure) => {
                settings.tip.pressure_size = true;
                settings.dynamics.size = response;
            }
            (b"prVr", DynamicsControl::Pressure) => {
                settings.tip.pressure_flow = true;
                settings.dynamics.flow = response;
            }
            (b"opVr", DynamicsControl::Pressure) => {
                settings.dynamics.pressure_opacity = true;
                settings.dynamics.opacity = response;
            }
            (b"szVr", DynamicsControl::Tilt) => {
                settings.tip.pressure_size = false;
                settings.dynamics.tilt = tilt_channel(&settings.dynamics.tilt, key, response);
                settings.dynamics.matrix =
                    route_source(settings.dynamics.matrix, key, DynamicsSource::Tilt);
                source_changed = true;
            }
            (b"prVr", DynamicsControl::Tilt) => {
                settings.tip.pressure_flow = false;
                settings.dynamics.tilt = tilt_channel(&settings.dynamics.tilt, key, response);
                settings.dynamics.matrix =
                    route_source(settings.dynamics.matrix, key, DynamicsSource::Tilt);
                source_changed = true;
            }
            (b"opVr", DynamicsControl::Tilt) => {
                settings.dynamics.pressure_opacity = false;
                settings.dynamics.tilt = tilt_channel(&settings.dynamics.tilt, key, response);
                settings.dynamics.matrix =
                    route_source(settings.dynamics.matrix, key, DynamicsSource::Tilt);
                source_changed = true;
            }
            (b"szVr", DynamicsControl::Rotation) => {
                settings.tip.pressure_size = false;
                settings.dynamics.rotation =
                    rotation_channel(&settings.dynamics.rotation, key, response);
                settings.dynamics.matrix =
                    route_source(settings.dynamics.matrix, key, DynamicsSource::Rotation);
                source_changed = true;
            }
            (b"prVr", DynamicsControl::Rotation) => {
                settings.tip.pressure_flow = false;
                settings.dynamics.rotation =
                    rotation_channel(&settings.dynamics.rotation, key, response);
                settings.dynamics.matrix =
                    route_source(settings.dynamics.matrix, key, DynamicsSource::Rotation);
                source_changed = true;
            }
            (b"opVr", DynamicsControl::Rotation) => {
                settings.dynamics.pressure_opacity = false;
                settings.dynamics.rotation =
                    rotation_channel(&settings.dynamics.rotation, key, response);
                settings.dynamics.matrix =
                    route_source(settings.dynamics.matrix, key, DynamicsSource::Rotation);
                source_changed = true;
            }
            (b"szVr", DynamicsControl::Direction) => {
                settings.tip.pressure_size = false;
                settings.dynamics.direction =
                    direction_channel(&settings.dynamics.direction, key, response);
                settings.dynamics.matrix =
                    route_source(settings.dynamics.matrix, key, DynamicsSource::Direction);
                source_changed = true;
            }
            (b"prVr", DynamicsControl::Direction) => {
                settings.tip.pressure_flow = false;
                settings.dynamics.direction =
                    direction_channel(&settings.dynamics.direction, key, response);
                settings.dynamics.matrix =
                    route_source(settings.dynamics.matrix, key, DynamicsSource::Direction);
                source_changed = true;
            }
            (b"opVr", DynamicsControl::Direction) => {
                settings.dynamics.pressure_opacity = false;
                settings.dynamics.direction =
                    direction_channel(&settings.dynamics.direction, key, response);
                settings.dynamics.matrix =
                    route_source(settings.dynamics.matrix, key, DynamicsSource::Direction);
                source_changed = true;
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
                match control {
                    DynamicsControl::Pressure => "笔压控制；使用独立线性响应与最低比例",
                    DynamicsControl::Tilt => "设备倾斜控制；使用独立线性响应与最低比例",
                    DynamicsControl::Direction => "笔触方向控制；使用独立线性响应与最低比例",
                    DynamicsControl::Rotation => "笔旋转控制；使用独立线性响应与最低比例",
                },
            ),
            (
                "Mnm ",
                if minimum == effective {
                    Status::Mapped
                } else {
                    Status::NotExecuted
                },
                if minimum == effective {
                    "动态最低比例"
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
                    "随机抖动尚未转换；独立动态分量已执行"
                },
            ),
        ] {
            let path = format!("{name}.{field}");
            issue(issues, &path, status, reason);
            handled.push(path);
        }
        if steps {
            let path = format!("{name}.fStp");
            issue(issues, &path, Status::Mapped, "动态控制不使用渐隐步数");
            handled.push(path);
        }
    }
    Ok((handled, source_changed))
}
fn texture_mode(id: &str) -> Option<TextureMode> {
    match id {
        "normal" => Some(TextureMode::Normal),
        "dissolve" => Some(TextureMode::Dissolve),
        "multiply" => Some(TextureMode::Multiply),
        "darken" => Some(TextureMode::Darken),
        "lighten" => Some(TextureMode::Lighten),
        "subtract" => Some(TextureMode::Subtract),
        "difference" => Some(TextureMode::Difference),
        "screen" => Some(TextureMode::Screen),
        "colorBurn" => Some(TextureMode::ColorBurn),
        "linearBurn" => Some(TextureMode::LinearBurn),
        "colorDodge" => Some(TextureMode::ColorDodge),
        "linearDodge" => Some(TextureMode::LinearDodge),
        "overlay" => Some(TextureMode::Overlay),
        "softLight" => Some(TextureMode::SoftLight),
        "hardLight" => Some(TextureMode::HardLight),
        "exclusion" => Some(TextureMode::Exclusion),
        "hardMix" => Some(TextureMode::HardMix),
        "vividLight" => Some(TextureMode::VividLight),
        "linearLight" => Some(TextureMode::LinearLight),
        "pinLight" => Some(TextureMode::PinLight),
        "divide" => Some(TextureMode::Divide),
        _ => None,
    }
}

fn texture_controls(
    index: usize,
    textures: TextureLookup<'_>,
    preset: &Descriptor,
    settings: &mut BrushSettings,
    issues: &mut Vec<Issue>,
) -> Result<(bool, Vec<String>)> {
    let Some((sources, patterns)) = textures else {
        return Ok((false, Vec::new()));
    };
    let Some(source) = sources.get(&index) else {
        return Ok((false, Vec::new()));
    };
    let pattern = patterns
        .get(&source.pattern_id)
        .ok_or(Error::Invalid("ABR texture pattern reference not found"))?;
    let mode = source.mode.as_deref().and_then(texture_mode);
    let enabled = mode.is_some();
    let depth_pressure = match preset.get(b"textureDepthDynamics") {
        Some(Value::Object { descriptor, .. }) => match descriptor.get(b"bVTy") {
            Some(Value::Integer(2)) => true,
            Some(Value::Integer(0)) | None => false,
            _ => false,
        },
        _ => false,
    };
    let minimum_depth = source.minimum_depth.min(source.depth);
    let texture_settings = TextureSettings::new(
        if source.each_tip {
            TextureSpace::Tip
        } else {
            TextureSpace::Document
        },
        source.scale,
        0.,
        source.depth,
        source.invert,
        0.,
        0.,
    )?
    .with_mode(mode.unwrap_or(TextureMode::Normal))
    .with_adjustments(source.brightness, source.contrast)?
    .with_depth_dynamics(minimum_depth, source.depth_jitter, depth_pressure)?;
    settings.texture = BrushTexture::new(
        enabled,
        texture_settings,
        Some(std::sync::Arc::new(TextureResource::new(
            pattern.mask.clone(),
        ))),
    )?;
    let handled = vec![
        "useTexture".into(),
        "textureDepth".into(),
        "textureScale".into(),
        "Txtr".into(),
        "Txtr.Ptrn.Idnt".into(),
        "Txtr.Ptrn.Nm  ".into(),
        "TxtC".into(),
        "InvT".into(),
        "textureBlendMode".into(),
        "textureBrightness".into(),
        "textureContrast".into(),
        "minimumDepth".into(),
        "textureDepthDynamics".into(),
    ];
    for (path, status, reason) in [
        (
            "useTexture",
            if enabled {
                Status::Mapped
            } else {
                Status::NotExecuted
            },
            if enabled {
                "纹理图案与参数已绑定"
            } else {
                "纹理模式尚未映射，保留图案与参数但不启用"
            },
        ),
        ("textureDepth", Status::Mapped, "纹理深度百分比"),
        ("textureScale", Status::Mapped, "纹理缩放百分比"),
        ("textureBrightness", Status::Mapped, "纹理亮度调整"),
        ("textureContrast", Status::Mapped, "纹理对比度调整"),
        ("minimumDepth", Status::Mapped, "纹理深度动态的最低深度"),
        ("Txtr.Ptrn.Idnt", Status::Mapped, "按图案 ID 绑定嵌入资源"),
        (
            "Txtr.Ptrn.Nm  ",
            Status::Approximate,
            "图案名称仅用于来源诊断",
        ),
        (
            "TxtC",
            Status::Mapped,
            "按 Texture Each Tip 选择笔尖或文档空间",
        ),
        ("InvT", Status::Mapped, "反相纹理"),
        (
            "textureBlendMode",
            if enabled {
                Status::Mapped
            } else {
                Status::NotExecuted
            },
            if enabled {
                "映射到既有纹理覆盖模式"
            } else {
                "该 Photoshop 纹理模式尚无等价执行公式"
            },
        ),
        (
            "textureDepthDynamics.bVTy",
            if depth_pressure {
                Status::Mapped
            } else {
                Status::NotExecuted
            },
            if depth_pressure {
                "纹理深度使用笔压控制"
            } else {
                "纹理深度控制未映射；仅保留随机深度抖动"
            },
        ),
        (
            "textureDepthDynamics.jitter",
            Status::Mapped,
            "纹理深度逐印章随机抖动",
        ),
        (
            "textureDepthDynamics.fStp",
            Status::Approximate,
            "纹理深度不使用渐隐步数",
        ),
    ] {
        issue(issues, path, status, reason);
    }
    Ok((enabled, handled))
}

fn convert_inner<'a>(
    linked: &[LinkedPreset<'a>],
    token: &CancellationToken,
    generation: u32,
    textures: TextureLookup<'_>,
) -> Result<Vec<ConvertedPreset<'a>>> {
    check(token)?;
    if linked.is_empty() || linked.len() > 1024 {
        return Err(Error::Limit("ABR converted preset count"));
    }
    let mut output = Vec::new();
    let mut report_bytes = 0usize;
    for (index, item) in linked.iter().enumerate() {
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
            taper: Default::default(),
            wet: Default::default(),
            smoothing: Default::default(),
            texture: Default::default(),
            dual: Default::default(),
            airbrush: Default::default(),
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
        let (mut handled, source_changed) = if generation >= 3 {
            pressure_controls(item.preset, tool, &mut settings, &mut issues)?
        } else {
            (Vec::new(), false)
        };
        let scatter_changed = if generation >= 4 {
            let extra = scatter_controls(item.preset, &mut settings, &mut issues)?;
            let changed = !extra.is_empty();
            handled.extend(extra);
            changed
        } else {
            false
        };
        let color_changed = if generation >= 5 {
            let extra = color_controls(item.preset, &mut settings, &mut issues)?;
            let changed = !extra.is_empty();
            handled.extend(extra);
            changed
        } else {
            false
        };
        let shape_changed = if generation >= 6 {
            let extra = shape_controls(item.preset, tool, &mut settings, &mut issues)?;
            let changed = !extra.is_empty();
            handled.extend(extra);
            changed
        } else {
            false
        };
        let device_changed = if generation >= 7 {
            let extra = device_controls(item.preset, tool, &mut settings, &mut issues)?;
            let changed = !extra.is_empty();
            handled.extend(extra);
            changed
        } else {
            false
        };
        let texture_changed = if generation >= 8 {
            let (changed, extra) =
                texture_controls(index, textures, item.preset, &mut settings, &mut issues)?;
            handled.extend(extra);
            changed
        } else {
            false
        };
        for (key, value) in &item.preset.entries {
            if key != b"Nm  " && key != b"Brsh" && key != b"toolOptions" {
                if [
                    b"useTipDynamics".as_slice(),
                    b"usePaintDynamics",
                    b"minimumDiameter",
                    b"minimumRoundness",
                    b"angleDynamics",
                    b"roundnessDynamics",
                    b"szVr",
                    b"prVr",
                    b"opVr",
                    b"useScatter",
                    b"bothAxes",
                    b"Cnt ",
                    b"scatterDynamics",
                    b"countDynamics",
                    b"useColorDynamics",
                    b"clVr",
                    b"H   ",
                    b"Strt",
                    b"Brgh",
                    b"colorDynamicsPerTip",
                    b"useTexture",
                    b"textureDepth",
                    b"textureScale",
                    b"Txtr",
                    b"TxtC",
                    b"InvT",
                    b"textureBlendMode",
                    b"minimumDepth",
                    b"textureDepthDynamics",
                    b"textureBrightness",
                    b"textureContrast",
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
        if source_changed {
            converter_version = 9;
        } else if texture_changed {
            converter_version = 8;
        } else if device_changed {
            converter_version = 7;
        } else if shape_changed {
            converter_version = 6;
        } else if color_changed {
            converter_version = 5;
        } else if scatter_changed {
            converter_version = 4;
        } else if !handled.is_empty() {
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
