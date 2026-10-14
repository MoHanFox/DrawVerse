use paint_core::*;
use paint_io::{
    brush_pressure::PressureParameters,
    brush_state,
    vbr::{Package, Parameters, Preset},
    vbr_image,
};
use paint_task::CancellationToken;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};
fn dynamics() -> BrushDynamics {
    BrushDynamics {
        size: PressureControl::new(
            PressureCurve::new(vec![(0., 0.), (0.5, 0.25), (1., 1.)]).unwrap(),
            0.1,
        )
        .unwrap(),
        flow: PressureControl::new(
            PressureCurve::new(vec![(0., 0.75), (1., 0.75)]).unwrap(),
            0.,
        )
        .unwrap(),
        ..Default::default()
    }
}
fn package() -> Package {
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "Pressure".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 2,
            required_capabilities: vec!["procedural-tip".into(), "pressure-curves".into()],
            parameters: Parameters {
                scatter_dynamics: None,
                shape_jitter: None,
                color_dynamics: None,
                tip_dynamics: None,
                diameter: 16.,
                spacing: 0.01,
                dab_opacity: 1.,
                hardness: 1.,
                roundness: 1.,
                angle: 0.,
                flow: 1.,
                pressure_size: true,
                pressure_flow: true,
                pressure_dynamics: Some(PressureParameters::from_core(&dynamics())),
            },
            tip_resource: None,
            source_resource: None,
        }],
    }
}

#[test]
fn pressure_payload_rejects_scatter_while_state8_preserves_it() {
    let mut catalog = BrushCatalog::default();
    let mut settings = catalog.selected().settings().clone();
    settings.dynamics.scatter = ScatterDynamics::new(false, 2., true, 4, 0.5, 1).unwrap();
    catalog.edit("round-pressure", settings.clone()).unwrap();
    let state = brush_state::encode(&catalog).unwrap();
    assert_eq!(
        brush_state::decode(&state, MAX_CATALOG_MASK_BYTES)
            .unwrap()
            .selected()
            .settings()
            .dynamics
            .scatter,
        settings.dynamics.scatter
    );
    let data = PressureParameters::from_core(&settings.dynamics);
    assert!(serde_json::to_vec(&data).is_err());
    let mut p = package();
    p.brushes[0].parameters.pressure_dynamics = Some(data);
    assert!(p.validate().is_err());
}

#[test]
fn pressure_only_payload_rejects_device_controls_while_state_saves_separate_fields() {
    let mut d = dynamics();
    d.tip = TipDynamics::new(AngleControl::Direction, false, 0.01).unwrap();
    let parameters = PressureParameters::from_core(&d);
    assert!(serde_json::to_vec(&parameters).is_err());
    let mut p = package();
    p.brushes[0].parameters.pressure_dynamics = Some(parameters);
    assert_eq!(p.brushes[0].parameters.definition().dynamics.tip, d.tip);
    assert!(p.validate().is_err());
    let mut catalog = BrushCatalog::default();
    let mut settings = catalog.selected().settings().clone();
    settings.dynamics = d;
    catalog.edit("round-pressure", settings).unwrap();
    let state = brush_state::encode(&catalog).unwrap();
    assert_eq!(
        brush_state::decode(&state, MAX_CATALOG_MASK_BYTES)
            .unwrap()
            .selected()
            .settings()
            .dynamics
            .tip,
        TipDynamics::new(AngleControl::Direction, false, 0.01).unwrap()
    );
}

#[test]
fn opacity_state6_vbr14_versions_flags_and_disabled_data_are_preserved() {
    let mut d = dynamics();
    d.pressure_opacity = true;
    d.opacity = PressureControl::new(PressureCurve::default(), 0.2).unwrap();
    let mut catalog = BrushCatalog::default();
    let mut settings = catalog.selected().settings().clone();
    settings.dynamics = d.clone();
    catalog.edit("round-pressure", settings).unwrap();
    let encoded = brush_state::encode(&catalog).unwrap();
    let mut state: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(state["version"], 6);
    assert_eq!(
        brush_state::decode(&encoded, MAX_CATALOG_MASK_BYTES)
            .unwrap()
            .selected()
            .settings()
            .dynamics,
        d
    );
    state["version"] = 5.into();
    assert!(
        brush_state::decode(&serde_json::to_vec(&state).unwrap(), MAX_CATALOG_MASK_BYTES).is_err()
    );
    let mut p = package();
    p.brushes[0].behavior_version = 3;
    p.brushes[0]
        .required_capabilities
        .push("opacity-pressure".into());
    p.brushes[0].parameters.pressure_dynamics = Some(PressureParameters::from_core(&d));
    let token = CancellationToken::default();
    let bundle = vbr_image::ImageBundle::new(p.clone(), BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    assert_eq!(
        vbr_image::decode(&bytes, &token)
            .unwrap()
            .definition(0)
            .unwrap()
            .dynamics,
        d
    );
    assert!(vbr_image::decode(&rewrite(&bytes, 3), &token).is_err());
    p.brushes[0]
        .required_capabilities
        .retain(|c| c != "opacity-pressure");
    assert!(p.validate().is_err());
    let mut bad: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    let preset = bad["presets"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["id"] == "round-pressure")
        .unwrap();
    preset["pressureDynamics"]["opacity"]["enabled"] = 1.into();
    assert!(
        brush_state::decode(&serde_json::to_vec(&bad).unwrap(), MAX_CATALOG_MASK_BYTES).is_err()
    );
    let mut bad: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    let preset = bad["presets"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|p| p["id"] == "round-pressure")
        .unwrap();
    preset["pressureDynamics"]["opacity"] = serde_json::Value::Null;
    assert!(
        brush_state::decode(&serde_json::to_vec(&bad).unwrap(), MAX_CATALOG_MASK_BYTES).is_err()
    );
    d.pressure_opacity = false;
    let parameters = PressureParameters::from_core(&d);
    assert!(parameters.has_opacity());
    let roundtrip: PressureParameters =
        serde_json::from_slice(&serde_json::to_vec(&parameters).unwrap()).unwrap();
    assert_eq!(roundtrip.core(), &d);
}
fn rewrite(bytes: &[u8], minor: u32) -> Vec<u8> {
    let mut input = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut output = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..input.len() {
        let mut entry = input.by_index(i).unwrap();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).unwrap();
        if entry.name() == "manifest.json" {
            let mut json: serde_json::Value = serde_json::from_slice(&data).unwrap();
            json["version"]["minor"] = minor.into();
            data = serde_json::to_vec(&json).unwrap();
        }
        output
            .start_file(entry.name(), SimpleFileOptions::default())
            .unwrap();
        output.write_all(&data).unwrap();
    }
    output.finish().unwrap().into_inner()
}
#[test]
fn state5_and_vbr13_compile_to_same_pressure_pixels_and_never_downgrade() {
    let mut catalog = BrushCatalog::default();
    let mut settings = catalog.selected().settings().clone();
    settings.diameter = 16.;
    settings.dynamics = dynamics();
    catalog.edit("round-pressure", settings).unwrap();
    let bytes = brush_state::encode(&catalog).unwrap();
    let restored = brush_state::decode(&bytes, MAX_CATALOG_MASK_BYTES).unwrap();
    assert_eq!(restored.selected().settings().dynamics, dynamics());
    let token = CancellationToken::default();
    let package = package();
    let bundle = vbr_image::ImageBundle::new(package.clone(), BTreeMap::new(), &token).unwrap();
    let packed = vbr_image::encode(&bundle, &token).unwrap();
    let unpacked = vbr_image::decode(&packed, &token).unwrap();
    assert_eq!(
        unpacked.package.to_json().unwrap(),
        package.to_json().unwrap()
    );
    let mut state = Document::new(32, 32).unwrap();
    let mut vbr = Document::new(32, 32).unwrap();
    for (doc, definition) in [
        (&mut state, restored.selected().settings().definition()),
        (&mut vbr, unpacked.definition(0).unwrap()),
    ] {
        doc.begin_brush_stroke(definition, InputPoint::new(16.5, 16.5, 0.5))
            .unwrap();
        doc.end_stroke().unwrap();
    }
    for y in 0..32 {
        for x in 0..32 {
            assert_eq!(state.pixel(x, y), vbr.pixel(x, y));
        }
    }
    assert_eq!(state.pixel(16, 16).alpha(), 0.75);
    assert_eq!(state.pixel(21, 16).alpha(), 0.);
    for version in [0, 1, 2, vbr_image::MAX_EXECUTABLE_MINOR + 1] {
        assert!(vbr_image::decode(&rewrite(&packed, version), &token).is_err());
    }
    assert!(paint_io::vbr::encode(&package, &token).is_err());
}
#[test]
fn invalid_pressure_shapes_ranges_sizes_and_old_state_versions_are_rejected() {
    let mut catalog = BrushCatalog::default();
    let mut settings = catalog.selected().settings().clone();
    settings.dynamics = dynamics();
    catalog.edit("round-pressure", settings).unwrap();
    let root: serde_json::Value =
        serde_json::from_slice(&brush_state::encode(&catalog).unwrap()).unwrap();
    for mutation in 0..9 {
        let mut bad = root.clone();
        match mutation {
            0 => bad["version"] = 4.into(),
            1 => {
                bad["presets"][0]["pressureDynamics"]["size"]["curve"] =
                    serde_json::json!([[0., 0.], [0.5, 0.2], [0.5, 0.8], [1., 1.]])
            }
            2 => bad["presets"][0]["pressureDynamics"]["flow"]["minimum"] = (-1.).into(),
            3 => {
                bad["presets"][0]["pressureDynamics"]["size"]["curve"] =
                    serde_json::json!(vec![[0., 0.]; 18])
            }
            4 => {
                bad["presets"][0]["pressureDynamics"]["size"]["curve"] =
                    serde_json::json!([[0.1, 0.], [1., 1.]])
            }
            5 => {
                bad["presets"][0]["pressureDynamics"]["size"]["curve"] =
                    serde_json::json!([[0., 0.], [1., 2.]])
            }
            6 => bad["presets"][0]["pressureDynamics"]["unexpected"] = true.into(),
            7 => bad["presets"][0]["pressureDynamics"]["size"]["minimum"] = serde_json::Value::Null,
            _ => bad["presets"][0]["pressureDynamics"] = serde_json::Value::Null,
        }
        assert!(
            brush_state::decode(&serde_json::to_vec(&bad).unwrap(), MAX_CATALOG_MASK_BYTES)
                .is_err(),
            "mutation {mutation}"
        );
    }
    let mut bad = package();
    bad.brushes[0].behavior_version = 1;
    assert!(bad.validate().is_err());
    bad.brushes[0].behavior_version = 2;
    bad.brushes[0].required_capabilities.pop();
    assert!(bad.validate().is_err());
    bad.brushes[0]
        .required_capabilities
        .push("pressure-curves".into());
    bad.brushes[0].engine_profile = "legacy.round".into();
    assert!(bad.validate().is_err());
}

#[test]
fn pressure_contract_bounds_and_original_converter_baseline_remain_separate() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-pressure-v1.3.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["pressureControl"]["properties"]["curve"]["maxItems"],
        17
    );
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        3
    );
    assert_eq!(
        schema["$defs"]["sourceRecord"]["properties"]["baseline"]["$ref"],
        "#/$defs/legacyParameters"
    );
    assert!(schema["$defs"]["legacyParameters"]["properties"]
        .get("pressureDynamics")
        .is_none());
    let token = CancellationToken::default();
    let bundle = vbr_image::ImageBundle::new(package(), BTreeMap::new(), &token).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("old.vbr");
    std::fs::write(&target, b"old").unwrap();
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(vbr_image::save(&target, &bundle, &cancelled).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"old");
}
