use paint_core::*;
use paint_io::{
    brush_color_dynamics::ColorParameters,
    brush_pressure::PressureParameters,
    brush_scatter::ScatterParameters,
    brush_shape_jitter::ShapeParameters,
    brush_state,
    brush_tip_dynamics::TipParameters,
    vbr::{Package, Parameters, Preset},
    vbr_image,
};
use paint_task::CancellationToken;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};
fn settings() -> BrushSettings {
    BrushSettings {
        diameter: 4.,
        spacing: 0.25,
        opacity: 0.5,
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        dynamics: BrushDynamics {
            scatter: ScatterDynamics::new(true, 2., true, 4, 0.5, u64::MAX).unwrap(),
            shape_jitter: ShapeJitter::new(true, 0.8, 0.2, 0.7, 0.6, 0.1, u64::MAX).unwrap(),
            color: ColorDynamics::new(true, 1., 0.5, 0.3, 0.2, true, u64::MAX).unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn package(s: &BrushSettings) -> Package {
    let mut pressure = s.dynamics.clone();
    pressure.tip = Default::default();
    pressure.scatter = Default::default();
    pressure.shape_jitter = Default::default();
    pressure.color = Default::default();
    let mut caps = vec![
        "procedural-tip".into(),
        "scatter-stamps".into(),
        "shape-jitter".into(),
        "color-dynamics".into(),
        "pressure-curves".into(),
    ];
    if pressure.pressure_opacity || pressure.opacity != PressureControl::default() {
        caps.push("opacity-pressure".into());
    }
    if s.dynamics.tip != TipDynamics::default() {
        caps.push("tip-dynamics".into());
    }
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            stroke_taper: None,
            wet_mix: None,
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "scatter".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 7,
            required_capabilities: caps,
            tip_resource: None,
            texture: None,
            smoothing: None,
            dual_brush: None,
            source_resource: None,
            parameters: Parameters {
                diameter: s.diameter,
                spacing: s.spacing,
                dab_opacity: s.opacity,
                hardness: s.tip.hardness,
                roundness: s.tip.roundness,
                angle: s.tip.angle,
                flow: s.tip.flow,
                pressure_size: s.tip.pressure_size,
                pressure_flow: s.tip.pressure_flow,
                pressure_dynamics: Some(PressureParameters::from_core(&pressure)),
                tip_dynamics: (s.dynamics.tip != TipDynamics::default())
                    .then(|| TipParameters::from_core(s.dynamics.tip)),
                scatter_dynamics: Some(ScatterParameters::from_core(s.dynamics.scatter)),
                shape_jitter: Some(ShapeParameters::from_core(s.dynamics.shape_jitter)),
                color_dynamics: Some(ColorParameters::from_core(s.dynamics.color)),
            },
        }],
    }
}
fn paint(mut s: BrushDefinition) -> Vec<Pixel> {
    s.brush.color = Pixel::from_straight([1., 0.1, 0., 0.6]).unwrap();
    s.brush.background_color = Pixel::from_straight([0., 0., 1., 0.8]).unwrap();
    let mut doc = Document::new(96, 96).unwrap();
    for _ in 0..2 {
        doc.begin_brush_stroke(
            s.clone(),
            InputPoint {
                capabilities: 7,
                rotation: 45.,
                tilt_x: 30.,
                tilt_y: 20.,
                ..InputPoint::new(16.5, 48.5, 0.5)
            },
        )
        .unwrap();
        doc.stroke_to(InputPoint {
            capabilities: 7,
            rotation: 80.,
            tilt_x: 60.,
            tilt_y: 20.,
            ..InputPoint::new(80.5, 48.5, 0.8)
        })
        .unwrap();
        doc.end_stroke().unwrap();
    }
    (0..96)
        .flat_map(|y| (0..96).map(move |x| (x, y)))
        .map(|(x, y)| doc.pixel(x, y))
        .collect()
}
fn minor(bytes: &[u8], v: u32) -> Vec<u8> {
    let mut input = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..input.len() {
        let mut entry = input.by_index(i).unwrap();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).unwrap();
        if entry.name() == "manifest.json" {
            let mut m: serde_json::Value = serde_json::from_slice(&data).unwrap();
            m["version"]["minor"] = v.into();
            data = serde_json::to_vec(&m).unwrap();
        }
        out.start_file(entry.name(), SimpleFileOptions::default())
            .unwrap();
        out.write_all(&data).unwrap();
    }
    out.finish().unwrap().into_inner()
}
#[test]
fn state10_vbr18_keep_combined_parameters_tool_colors_and_actual_pixels() {
    let token = CancellationToken::default();
    let mut s = settings();
    s.dynamics.tip = TipDynamics::new(AngleControl::Direction, true, 0.3).unwrap();
    s.dynamics.pressure_opacity = true;
    s.dynamics.opacity = PressureControl::new(PressureCurve::default(), 0.2).unwrap();
    let expected = paint(s.definition());
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", s.clone()).unwrap();
    let encoded = brush_state::encode(&catalog).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(json["version"], 10);
    assert_eq!(
        paint(
            brush_state::decode(&encoded, MAX_CATALOG_MASK_BYTES)
                .unwrap()
                .selected()
                .settings()
                .definition()
        ),
        expected
    );
    let bundle = vbr_image::ImageBundle::new(package(&s), BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let restored = vbr_image::decode(&bytes, &token).unwrap();
    assert_eq!(
        restored.definition(0).unwrap().dynamics.color.seed(),
        u64::MAX
    );
    assert_eq!(restored.definition(0).unwrap().dynamics, s.dynamics);
    assert_eq!(paint(restored.definition(0).unwrap()), expected);
    assert!(vbr_image::decode(&minor(&bytes, 7), &token).is_err());
    s.dynamics.color = ColorDynamics::new(false, 1., 0.5, 0.3, 0.2, true, u64::MAX).unwrap();
    let closed = vbr_image::ImageBundle::new(package(&s), BTreeMap::new(), &token).unwrap();
    let closed = vbr_image::decode(&vbr_image::encode(&closed, &token).unwrap(), &token).unwrap();
    assert_eq!(closed.definition(0).unwrap().dynamics, s.dynamics);
    assert_eq!(paint(closed.definition(0).unwrap()), paint(s.definition()));
    catalog.edit("round-pressure", s.clone()).unwrap();
    assert_eq!(
        brush_state::decode(
            &brush_state::encode(&catalog).unwrap(),
            MAX_CATALOG_MASK_BYTES
        )
        .unwrap()
        .selected()
        .settings()
        .dynamics
        .color,
        s.dynamics.color
    );
}
#[test]
fn bad_seed_types_ranges_versions_and_capabilities_reject() {
    let value =
        serde_json::to_value(ColorParameters::from_core(settings().dynamics.color)).unwrap();
    assert_eq!(value["seed"], "ffffffffffffffff");
    for (field, bad) in [
        ("seed", serde_json::json!(u64::MAX)),
        ("seed", serde_json::json!("FFFFFFFFFFFFFFFF")),
        ("seed", serde_json::json!("1")),
        ("model", serde_json::json!("srgb-hsv")),
        ("foregroundBackground", serde_json::json!(1.1)),
        ("hue", serde_json::json!(1.00000001)),
        ("model", serde_json::json!(7)),
        ("model", serde_json::json!({"linear-hsv-v1":null})),
        ("value", serde_json::json!(-1e-50)),
        ("perTip", serde_json::json!(1)),
        ("hue", serde_json::json!(1.1)),
        ("enabled", serde_json::json!(1)),
        ("saturation", serde_json::json!(1.1)),
        ("extra", serde_json::json!(0)),
    ] {
        let mut v = value.clone();
        v[field] = bad;
        assert!(serde_json::from_value::<ColorParameters>(v).is_err());
    }
    assert!(serde_json::from_str::<ColorParameters>("null").is_err());
    let mut p = package(&settings());
    p.brushes[0]
        .required_capabilities
        .retain(|c| c != "color-dynamics");
    assert!(p.validate().is_err());
    let mut p = package(&settings());
    p.brushes[0].behavior_version = 6;
    assert!(p.validate().is_err());
    let mut json: serde_json::Value =
        serde_json::from_slice(&package(&settings()).to_json().unwrap()).unwrap();
    json["brushes"][0]["parameters"]["colorDynamics"] = serde_json::Value::Null;
    assert!(Package::from_json(&serde_json::to_vec(&json).unwrap()).is_err());
    let mut c = BrushCatalog::default();
    c.edit("round-pressure", settings()).unwrap();
    let mut v: serde_json::Value =
        serde_json::from_slice(&brush_state::encode(&c).unwrap()).unwrap();
    v["version"] = 9.into();
    assert!(brush_state::decode(&serde_json::to_vec(&v).unwrap(), MAX_CATALOG_MASK_BYTES).is_err());
}
#[test]
fn color_shape_and_scatter_without_pressure_keep_old_default_bytes() {
    let mut c = BrushCatalog::default();
    let old = brush_state::encode(&c).unwrap();
    assert_eq!(
        brush_state::encode(&brush_state::decode(&old, MAX_CATALOG_MASK_BYTES).unwrap()).unwrap(),
        old
    );
    let s = settings();
    c.edit("round-pressure", s.clone()).unwrap();
    let json: serde_json::Value =
        serde_json::from_slice(&brush_state::encode(&c).unwrap()).unwrap();
    let row = json["presets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "round-pressure")
        .unwrap();
    assert!(row.get("pressureDynamics").is_none());
    let mut p = package(&s);
    p.brushes[0].parameters.pressure_dynamics = None;
    p.brushes[0].required_capabilities = vec![
        "procedural-tip".into(),
        "scatter-stamps".into(),
        "shape-jitter".into(),
        "color-dynamics".into(),
    ];
    let token = CancellationToken::default();
    let b = vbr_image::ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
    let restored = vbr_image::decode(&vbr_image::encode(&b, &token).unwrap(), &token).unwrap();
    assert_eq!(
        paint(restored.definition(0).unwrap()),
        paint(s.definition())
    );
}

#[test]
fn color_alone_and_whole_stroke_frequency_restore_real_pixels() {
    let token = CancellationToken::default();
    for per_tip in [true, false] {
        let mut s = settings();
        s.dynamics.scatter = Default::default();
        s.dynamics.shape_jitter = Default::default();
        s.dynamics.color = ColorDynamics::new(true, 1., 0.5, 0.3, 0.2, per_tip, u64::MAX).unwrap();
        let mut p = package(&s);
        p.brushes[0].parameters.pressure_dynamics = None;
        p.brushes[0].parameters.scatter_dynamics = None;
        p.brushes[0].parameters.shape_jitter = None;
        p.brushes[0].required_capabilities = vec!["procedural-tip".into(), "color-dynamics".into()];
        let b = vbr_image::ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
        let bytes = vbr_image::encode(&b, &token).unwrap();
        let restored = vbr_image::decode(&bytes, &token).unwrap();
        assert_eq!(
            restored.definition(0).unwrap().dynamics.color,
            s.dynamics.color
        );
        assert_eq!(
            paint(restored.definition(0).unwrap()),
            paint(s.definition())
        );
        assert!(vbr_image::decode(&minor(&bytes, 7), &token).is_err());
    }
}
#[test]
fn published_color_contract_names_the_actual_model_and_frozen_baseline() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-color-dynamics-v1.8.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["colorDynamics"]["properties"]["model"]["const"],
        "linear-hsv-v1"
    );
    assert_eq!(
        schema["$defs"]["colorDynamics"]["properties"]["seed"]["pattern"],
        "^[0-9a-f]{16}$"
    );
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        8
    );
    assert_eq!(
        schema["$defs"]["sourceRecord"]["properties"]["baseline"]["$ref"],
        "#/$defs/legacyParameters"
    );
}
