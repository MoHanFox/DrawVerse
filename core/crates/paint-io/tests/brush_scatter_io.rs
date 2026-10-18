use paint_core::*;
use paint_io::{
    brush_pressure::PressureParameters,
    brush_scatter::ScatterParameters,
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
            ..Default::default()
        },
        ..Default::default()
    }
}
fn package(s: &BrushSettings) -> Package {
    let mut pressure = s.dynamics.clone();
    pressure.tip = Default::default();
    pressure.scatter = Default::default();
    let mut caps = vec![
        "procedural-tip".into(),
        "scatter-stamps".into(),
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
            behavior_version: 5,
            required_capabilities: caps,
            tip_resource: None,
            texture: None,
            smoothing: None,
            dual_brush: None,
            source_resource: None,
            parameters: Parameters {
                airbrush: None,
                speed_dynamics: None,
                tangent_dynamics: None,
                distance_dynamics: None,
                tilt_dynamics: None,
                rotation_dynamics: None,
                direction_dynamics: None,
                source_matrix: None,
                fade_dynamics: None,
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
                shape_jitter: None,
                color_dynamics: None,
            },
        }],
    }
}
fn paint(s: BrushDefinition) -> Vec<Pixel> {
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
fn state8_vbr16_keep_seed_closed_configuration_and_actual_pixels() {
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
    assert_eq!(json["version"], 8);
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
        restored.definition(0).unwrap().dynamics.scatter.seed(),
        u64::MAX
    );
    assert_eq!(restored.definition(0).unwrap().dynamics, s.dynamics);
    assert_eq!(paint(restored.definition(0).unwrap()), expected);
    assert!(vbr_image::decode(&minor(&bytes, 5), &token).is_err());
    s.dynamics.scatter = ScatterDynamics::new(false, 2., true, 4, 0.5, u64::MAX).unwrap();
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
        .scatter,
        s.dynamics.scatter
    );
}
#[test]
fn bad_seed_types_ranges_versions_and_capabilities_reject() {
    let value =
        serde_json::to_value(ScatterParameters::from_core(settings().dynamics.scatter)).unwrap();
    assert_eq!(value["seed"], "ffffffffffffffff");
    for (field, bad) in [
        ("seed", serde_json::json!(u64::MAX)),
        ("seed", serde_json::json!("FFFFFFFFFFFFFFFF")),
        ("seed", serde_json::json!("1")),
        ("count", serde_json::json!(0)),
        ("count", serde_json::json!(17)),
        ("count", serde_json::json!(2.5)),
        ("amount", serde_json::json!(10.1)),
        ("enabled", serde_json::json!(1)),
        ("countJitter", serde_json::json!(1.1)),
        ("extra", serde_json::json!(0)),
    ] {
        let mut v = value.clone();
        v[field] = bad;
        assert!(serde_json::from_value::<ScatterParameters>(v).is_err());
    }
    assert!(serde_json::from_str::<ScatterParameters>("null").is_err());
    let mut p = package(&settings());
    p.brushes[0]
        .required_capabilities
        .retain(|c| c != "scatter-stamps");
    assert!(p.validate().is_err());
    let mut p = package(&settings());
    p.brushes[0].behavior_version = 4;
    assert!(p.validate().is_err());
    let mut json: serde_json::Value =
        serde_json::from_slice(&package(&settings()).to_json().unwrap()).unwrap();
    json["brushes"][0]["parameters"]["scatterDynamics"] = serde_json::Value::Null;
    assert!(Package::from_json(&serde_json::to_vec(&json).unwrap()).is_err());
    let mut c = BrushCatalog::default();
    c.edit("round-pressure", settings()).unwrap();
    let mut v: serde_json::Value =
        serde_json::from_slice(&brush_state::encode(&c).unwrap()).unwrap();
    v["version"] = 7.into();
    assert!(brush_state::decode(&serde_json::to_vec(&v).unwrap(), MAX_CATALOG_MASK_BYTES).is_err());
}
#[test]
fn scatter_only_has_no_pressure_payload_and_old_state_remains_old() {
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
    p.brushes[0].required_capabilities = vec!["procedural-tip".into(), "scatter-stamps".into()];
    let token = CancellationToken::default();
    let b = vbr_image::ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
    let restored = vbr_image::decode(&vbr_image::encode(&b, &token).unwrap(), &token).unwrap();
    assert_eq!(
        paint(restored.definition(0).unwrap()),
        paint(s.definition())
    );
}

#[test]
fn published_scatter_contract_matches_seed_and_count_units() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-scatter-v1.6.schema.json"
    ))
    .unwrap();
    let fields = &schema["$defs"]["scatterDynamics"]["properties"];
    assert_eq!(fields["count"]["maximum"], 16);
    assert_eq!(fields["amount"]["maximum"], 10);
    assert_eq!(fields["seed"]["pattern"], "^[0-9a-f]{16}$");
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        6
    );
    assert_eq!(
        schema["$defs"]["sourceRecord"]["properties"]["baseline"]["$ref"],
        "#/$defs/legacyParameters"
    );
}

#[test]
fn published_scatter_controls_contract_matches_optional_fields() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-scatter-controls-v1.42.schema.json"
    ))
    .unwrap();
    assert_eq!(schema["$id"], "urn:drawverse:brush:scatter-controls:1.42");
    assert_eq!(
        schema["$defs"]["control"]["properties"]["source"]["enum"][1],
        "fade"
    );
    assert_eq!(
        schema["$defs"]["scatter"]["properties"]["amountControl"]["$ref"],
        "#/$defs/control"
    );
}

#[test]
fn published_dual_coverage_modes_contract_matches_core_names() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-dual-modes-v1.43.schema.json"
    ))
    .unwrap();
    assert_eq!(schema["$id"], "urn:drawverse:brush:dual-modes:1.43");
    let modes = schema["$defs"]["dual"]["properties"]["mode"]["enum"]
        .as_array()
        .unwrap();
    for mode in ["overlay", "softLight", "hardLight", "divide"] {
        assert!(modes.iter().any(|value| value == mode));
    }
}

#[test]
fn state35_vbr42_preserve_scatter_device_controls_and_real_pixels() {
    let mut s = settings();
    s.dynamics.scatter = s
        .dynamics
        .scatter
        .with_controls(
            ScatterControl::new(ScatterControlSource::Pressure, 0.25, 0).unwrap(),
            ScatterControl::new(ScatterControlSource::Fade, 0.5, 6).unwrap(),
        )
        .unwrap();
    let expected = paint(s.definition());
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", s.clone()).unwrap();
    let encoded = brush_state::encode(&catalog).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(json["version"], 35);
    let restored = brush_state::decode(&encoded, MAX_CATALOG_MASK_BYTES).unwrap();
    let restored_settings = restored.selected().settings().clone();
    assert_eq!(restored_settings.dynamics.scatter, s.dynamics.scatter);
    assert_eq!(paint(restored_settings.definition()), expected);

    let mut p = package(&s);
    p.brushes[0].behavior_version = 27;
    p.brushes[0]
        .required_capabilities
        .push("scatter-controls".into());
    let token = CancellationToken::default();
    let bundle = vbr_image::ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let manifest: serde_json::Value = {
        let mut zip = ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
        let mut data = Vec::new();
        zip.by_name("manifest.json")
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        serde_json::from_slice(&data).unwrap()
    };
    assert_eq!(manifest["version"]["minor"], 42);
    let restored = vbr_image::decode(&bytes, &token).unwrap();
    let definition = restored.definition(0).unwrap();
    assert_eq!(definition.dynamics.scatter, s.dynamics.scatter);
    assert_eq!(paint(definition), expected);
    assert!(vbr_image::decode(&minor(&bytes, 41), &token).is_err());
}

#[test]
fn state35_vbr42_preserve_secondary_scatter_device_controls() {
    let mut s = settings();
    let secondary = SecondaryTip::new(
        6.,
        BrushTip::default(),
        None,
        BrushDynamics {
            scatter: ScatterDynamics::new(true, 0.5, true, 3, 0.25, 17)
                .unwrap()
                .with_controls(
                    ScatterControl::new(ScatterControlSource::Pressure, 0.2, 0).unwrap(),
                    ScatterControl::new(ScatterControlSource::Fade, 0.4, 5).unwrap(),
                )
                .unwrap(),
            ..Default::default()
        },
    )
    .unwrap();
    s.dual = DualBrush::new(true, DualMode::Multiply, secondary);
    let expected = paint(s.definition());
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", s.clone()).unwrap();
    let encoded = brush_state::encode(&catalog).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(json["version"], 35);
    let restored = brush_state::decode(&encoded, MAX_CATALOG_MASK_BYTES).unwrap();
    let restored_settings = restored.selected().settings().clone();
    assert_eq!(
        restored_settings.dual.secondary().dynamics().scatter,
        s.dual.secondary().dynamics().scatter
    );
    assert_eq!(paint(restored_settings.definition()), expected);

    let mut p = package(&s);
    p.brushes[0].dual_brush =
        Some(paint_io::brush_dual::DualParameters::from_core(&s.dual).unwrap());
    p.brushes[0].behavior_version = 27;
    p.brushes[0].required_capabilities.push("dual-cover".into());
    p.brushes[0]
        .required_capabilities
        .push("scatter-controls".into());
    let token = CancellationToken::default();
    let bundle = vbr_image::ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let manifest: serde_json::Value = {
        let mut zip = ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
        let mut data = Vec::new();
        zip.by_name("manifest.json")
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        serde_json::from_slice(&data).unwrap()
    };
    assert_eq!(manifest["version"]["minor"], 42);
    let restored = vbr_image::decode(&bytes, &token).unwrap();
    let definition = restored.definition(0).unwrap();
    assert_eq!(
        definition.dual.secondary().dynamics().scatter,
        s.dual.secondary().dynamics().scatter
    );
    assert_eq!(paint(definition), expected);
    assert!(vbr_image::decode(&minor(&bytes, 41), &token).is_err());
}

#[test]
fn state36_vbr43_preserve_extended_dual_coverage_modes() {
    let mut s = settings();
    s.dual = DualBrush::new(
        true,
        DualMode::Overlay,
        SecondaryTip::new(6., BrushTip::default(), None, BrushDynamics::default()).unwrap(),
    );
    let expected = paint(s.definition());
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", s.clone()).unwrap();
    let encoded = brush_state::encode(&catalog).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(json["version"], 36);
    let restored = brush_state::decode(&encoded, MAX_CATALOG_MASK_BYTES).unwrap();
    let restored_settings = restored.selected().settings().clone();
    assert_eq!(restored_settings.dual.mode(), DualMode::Overlay);
    assert_eq!(paint(restored_settings.definition()), expected);

    let mut p = package(&s);
    p.brushes[0].dual_brush =
        Some(paint_io::brush_dual::DualParameters::from_core(&s.dual).unwrap());
    p.brushes[0].behavior_version = 28;
    p.brushes[0].required_capabilities.push("dual-cover".into());
    p.brushes[0]
        .required_capabilities
        .push("dual-cover-extended".into());
    let token = CancellationToken::default();
    let bundle = vbr_image::ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let manifest: serde_json::Value = {
        let mut zip = ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
        let mut data = Vec::new();
        zip.by_name("manifest.json")
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        serde_json::from_slice(&data).unwrap()
    };
    assert_eq!(manifest["version"]["minor"], 43);
    let restored = vbr_image::decode(&bytes, &token).unwrap();
    let definition = restored.definition(0).unwrap();
    assert_eq!(definition.dual.mode(), DualMode::Overlay);
    assert_eq!(paint(definition), expected);
    assert!(vbr_image::decode(&minor(&bytes, 42), &token).is_err());
}
