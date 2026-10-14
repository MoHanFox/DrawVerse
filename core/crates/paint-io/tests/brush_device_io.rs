use paint_core::*;
use paint_io::{
    brush_pressure::PressureParameters,
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
fn package(settings: &BrushSettings) -> Package {
    let mut pressure = settings.dynamics.clone();
    pressure.tip = Default::default();
    let mut caps = vec![
        "procedural-tip".into(),
        "tip-dynamics".into(),
        "pressure-curves".into(),
    ];
    if pressure.pressure_opacity || pressure.opacity != PressureControl::default() {
        caps.push("opacity-pressure".into());
    }
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "device".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 4,
            required_capabilities: caps,
            tip_resource: None,
            texture: None,
            source_resource: None,
            parameters: Parameters {
                scatter_dynamics: None,
                shape_jitter: None,
                color_dynamics: None,
                diameter: settings.diameter,
                spacing: settings.spacing,
                dab_opacity: settings.opacity,
                hardness: settings.tip.hardness,
                roundness: settings.tip.roundness,
                angle: settings.tip.angle,
                flow: settings.tip.flow,
                pressure_size: settings.tip.pressure_size,
                pressure_flow: settings.tip.pressure_flow,
                pressure_dynamics: Some(PressureParameters::from_core(&pressure)),
                tip_dynamics: Some(TipParameters::from_core(settings.dynamics.tip)),
            },
        }],
    }
}
fn paint(definition: BrushDefinition) -> Document {
    let mut doc = Document::new(40, 40).unwrap();
    let mut point = InputPoint::new(16.5, 8.5, 0.4);
    point.capabilities = 6;
    point.rotation = 80.;
    point.tilt_x = 30.;
    point.tilt_y = 20.;
    doc.begin_brush_stroke(definition, point).unwrap();
    point.y = 24.5;
    point.pressure = 0.8;
    point.rotation = -80.;
    point.tilt_x = -30.;
    doc.stroke_to(point).unwrap();
    doc.end_stroke().unwrap();
    doc
}
fn minor(bytes: &[u8], version: u32) -> Vec<u8> {
    let mut input = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut output = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..input.len() {
        let mut entry = input.by_index(i).unwrap();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).unwrap();
        if entry.name() == "manifest.json" {
            let mut m: serde_json::Value = serde_json::from_slice(&data).unwrap();
            m["version"]["minor"] = version.into();
            data = serde_json::to_vec(&m).unwrap();
        }
        output
            .start_file(entry.name(), SimpleFileOptions::default())
            .unwrap();
        output.write_all(&data).unwrap();
    }
    output.finish().unwrap().into_inner()
}
#[test]
fn all_device_controls_restore_state7_and_vbr15_to_exact_original_pixels() {
    let token = CancellationToken::default();
    for angle in [
        AngleControl::Fixed,
        AngleControl::Direction,
        AngleControl::PenRotation,
        AngleControl::Tilt,
    ] {
        let mut catalog = BrushCatalog::default();
        let mut s = catalog.selected().settings().clone();
        s.diameter = 16.;
        s.spacing = 0.1;
        s.opacity = 0.4;
        s.tip.roundness = 0.3;
        s.tip.flow = 0.7;
        s.tip.pressure_size = false;
        s.tip.pressure_flow = false;
        s.dynamics.pressure_opacity = true;
        s.dynamics.opacity = PressureControl::new(PressureCurve::default(), 0.2).unwrap();
        s.dynamics.tip = TipDynamics::new(angle, true, 0.2).unwrap();
        let raw = paint(s.definition());
        catalog.edit("round-pressure", s.clone()).unwrap();
        let bytes = brush_state::encode(&catalog).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["version"],
            7
        );
        let restored = brush_state::decode(&bytes, MAX_CATALOG_MASK_BYTES).unwrap();
        assert_eq!(restored.selected().settings().dynamics, s.dynamics);
        let bundle = vbr_image::ImageBundle::new(package(&s), BTreeMap::new(), &token).unwrap();
        let packed = vbr_image::encode(&bundle, &token).unwrap();
        let restored_bundle = vbr_image::decode(&packed, &token).unwrap();
        assert!(vbr_image::decode(&minor(&packed, 4), &token).is_err());
        for restored in [
            paint(restored.selected().settings().definition()),
            paint(restored_bundle.definition(0).unwrap()),
        ] {
            for y in 0..40 {
                for x in 0..40 {
                    assert_eq!(restored.pixel(x, y), raw.pixel(x, y));
                }
            }
        }
    }
}
#[test]
fn tip_only_state_has_no_pressure_payload_and_legacy_state_does_not_upgrade() {
    let mut catalog = BrushCatalog::default();
    let old = brush_state::encode(&catalog).unwrap();
    let restored = brush_state::decode(&old, MAX_CATALOG_MASK_BYTES).unwrap();
    assert_eq!(brush_state::encode(&restored).unwrap(), old);
    let mut s = catalog.selected().settings().clone();
    s.dynamics.tip = TipDynamics::new(AngleControl::Direction, false, 0.01).unwrap();
    catalog.edit("round-pressure", s.clone()).unwrap();
    let state = brush_state::encode(&catalog).unwrap();
    let state: serde_json::Value = serde_json::from_slice(&state).unwrap();
    let row = state["presets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "round-pressure")
        .unwrap();
    assert!(row.get("tipDynamics").is_some());
    assert!(row.get("pressureDynamics").is_none());
    let mut p = package(&s);
    p.brushes[0].parameters.pressure_dynamics = None;
    p.brushes[0].required_capabilities = vec!["procedural-tip".into(), "tip-dynamics".into()];
    p.validate().unwrap();
    let token = CancellationToken::default();
    let bundle = vbr_image::ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    assert_eq!(
        vbr_image::decode(&bytes, &token)
            .unwrap()
            .definition(0)
            .unwrap()
            .dynamics
            .tip,
        s.dynamics.tip
    );
}
#[test]
fn bad_fields_versions_and_capabilities_are_rejected() {
    for json in [
        r#"{"angleControl":"unknown","tiltRoundness":false,"minimumRoundness":0.1}"#,
        r#"{"angleControl":"tilt","tiltRoundness":1,"minimumRoundness":0.1}"#,
        r#"{"angleControl":"tilt","tiltRoundness":false,"minimumRoundness":0}"#,
        r#"{"angleControl":"tilt","tiltRoundness":false,"minimumRoundness":1.1}"#,
        r#"{"angleControl":"tilt","tiltRoundness":false,"minimumRoundness":0.1,"extra":1}"#,
        "null",
    ] {
        assert!(serde_json::from_str::<TipParameters>(json).is_err());
    }
    let mut catalog = BrushCatalog::default();
    let mut s = catalog.selected().settings().clone();
    s.dynamics.tip = TipDynamics::new(AngleControl::Direction, false, 0.01).unwrap();
    catalog.edit("round-pressure", s.clone()).unwrap();
    let mut state: serde_json::Value =
        serde_json::from_slice(&brush_state::encode(&catalog).unwrap()).unwrap();
    state["version"] = 6.into();
    assert!(
        brush_state::decode(&serde_json::to_vec(&state).unwrap(), MAX_CATALOG_MASK_BYTES).is_err()
    );
    let mut p = package(&s);
    p.brushes[0]
        .required_capabilities
        .retain(|c| c != "tip-dynamics");
    assert!(p.validate().is_err());
    p.brushes[0]
        .required_capabilities
        .push("tip-dynamics".into());
    p.brushes[0].behavior_version = 3;
    assert!(p.validate().is_err());
    let mut json: serde_json::Value =
        serde_json::from_slice(&package(&s).to_json().unwrap()).unwrap();
    json["brushes"][0]["parameters"]["tipDynamics"] = serde_json::Value::Null;
    assert!(Package::from_json(&serde_json::to_vec(&json).unwrap()).is_err());
}

#[test]
fn published_device_contract_matches_control_units_and_source_roles() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-tip-dynamics-v1.5.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        5
    );
    let controls = &schema["$defs"]["tipDynamics"]["properties"];
    assert_eq!(controls["minimumRoundness"]["minimum"], 0.01);
    assert_eq!(controls["minimumRoundness"]["maximum"], 1);
    assert_eq!(controls["tiltRoundness"]["type"], "boolean");
    for angle in [
        AngleControl::Fixed,
        AngleControl::Direction,
        AngleControl::PenRotation,
        AngleControl::Tilt,
    ] {
        let value = serde_json::to_value(TipParameters::from_core(
            TipDynamics::new(angle, false, 0.01).unwrap(),
        ))
        .unwrap();
        assert!(controls["angleControl"]["enum"]
            .as_array()
            .unwrap()
            .contains(&value["angleControl"]));
    }
    assert_eq!(
        schema["$defs"]["sourceRecord"]["properties"]["baseline"]["$ref"],
        "#/$defs/legacyParameters"
    );
    assert!(
        schema["$defs"]["preset"]["properties"]["requiredCapabilities"]["items"]["enum"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("tip-dynamics"))
    );
}
