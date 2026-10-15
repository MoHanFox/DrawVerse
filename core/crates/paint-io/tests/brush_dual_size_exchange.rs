use paint_core::*;
use paint_io::{
    brush_state,
    vbr::{Package, Parameters, Preset},
    vbr_image,
};
use paint_task::CancellationToken;
use std::collections::BTreeMap;
fn package(sizing: DualSizing, enabled: bool) -> Package {
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            wet_mix: None,
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "Smooth".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 12,
            required_capabilities: vec![
                "procedural-tip".into(),
                "dual-cover".into(),
                "dual-relative-size".into(),
            ],
            parameters: Parameters {
                diameter: 12.,
                spacing: 0.04,
                dab_opacity: 0.4,
                hardness: 1.,
                roundness: 1.,
                angle: 0.,
                flow: 1.,
                pressure_size: true,
                pressure_flow: true,
                pressure_dynamics: None,
                tip_dynamics: None,
                scatter_dynamics: None,
                shape_jitter: None,
                color_dynamics: None,
            },
            smoothing: None,
            texture: None,
            dual_brush: Some(
                paint_io::brush_dual::DualParameters::from_core(
                    &DualBrush::new(
                        enabled,
                        DualMode::Multiply,
                        SecondaryTip::new(4., BrushTip::default(), None, Default::default())
                            .unwrap()
                            .with_spacing(0.01)
                            .unwrap(),
                    )
                    .with_sizing(sizing),
                )
                .unwrap(),
            ),
            tip_resource: None,
            source_resource: None,
        }],
    }
}
fn draw(doc: &mut Document, definition: BrushDefinition) {
    for shift in [0., 6.] {
        let mut p = InputPoint::new(8.5, 24.5 + shift, 0.3);
        p.timestamp_ns = 1_000_000;
        doc.begin_brush_stroke(definition.clone(), p).unwrap();
        for (x, y, pressure, time) in [
            (16.5, 12.5, 0.9, 25_000_000),
            (32.5, 38.5, 0.4, 41_000_000),
            (54.5, 24.5, 1., 81_000_000),
        ] {
            let mut p = InputPoint::new(x, y + shift, pressure);
            p.timestamp_ns = time;
            doc.stroke_to(p).unwrap();
        }
        doc.end_stroke().unwrap();
    }
}
fn pixels(doc: &Document) -> Vec<Pixel> {
    (0..64)
        .flat_map(|y| (0..64).map(move |x| doc.pixel(x, y)))
        .collect()
}

#[test]
fn relative_sizing_state_and_vbr_preserve_two_real_strokes_and_closed_values() {
    let token = CancellationToken::default();
    for enabled in [false, true] {
        for mode in [DualSizeMode::Independent, DualSizeMode::Relative] {
            for rule in [DualWidthRule::FollowPressure, DualWidthRule::FixedWidth] {
                let sizing = DualSizing::new(mode, -3.5, rule).unwrap();
                let mut p = package(sizing, enabled);
                p.brushes[0].smoothing =
                    Some(paint_io::brush_smoothing::SmoothingParameters::from_core(
                        BrushSmoothing::new(true, 24., 12., 0.7, 2.).unwrap(),
                    ));
                p.brushes[0]
                    .required_capabilities
                    .push("path-smoothing".into());
                let bundle =
                    vbr_image::ImageBundle::new(p.clone(), BTreeMap::new(), &token).unwrap();
                let bytes = vbr_image::encode(&bundle, &token).unwrap();
                let restored = vbr_image::decode(&bytes, &token).unwrap();
                assert_eq!(p.to_json().unwrap(), restored.package.to_json().unwrap());
                let original = bundle.definition(0).unwrap();
                let mut settings = p.brushes[0].parameters.settings();
                settings.dual = original.dual.clone();
                settings.smoothing = original.smoothing;
                let mut catalog = BrushCatalog::default();
                catalog.edit("round-pressure", settings).unwrap();
                let state = brush_state::encode(&catalog).unwrap();
                let state_value: serde_json::Value = serde_json::from_slice(&state).unwrap();
                assert_eq!(state_value["version"], 15);
                let captured = brush_state::decode(&state, MAX_CATALOG_MASK_BYTES).unwrap();
                assert_eq!(captured.selected().settings().dual.sizing(), sizing);
                let mut a = Document::new(64, 64).unwrap();
                let mut b = Document::new(64, 64).unwrap();
                let mut c = Document::new(64, 64).unwrap();
                draw(&mut a, original);
                draw(&mut b, restored.definition(0).unwrap());
                draw(&mut c, captured.selected().settings().definition());
                assert_eq!(pixels(&a), pixels(&b));
                assert_eq!(pixels(&a), pixels(&c));
                assert_eq!(a.history_depth(), (2, 0));
                let expected = pixels(&c);
                c.undo().unwrap();
                c.redo().unwrap();
                assert_eq!(pixels(&c), expected);
                let mut old = state_value;
                old["version"] = serde_json::json!(14);
                assert!(brush_state::decode(
                    &serde_json::to_vec(&old).unwrap(),
                    MAX_CATALOG_MASK_BYTES
                )
                .is_err());
                let mut wrong = p.clone();
                wrong.brushes[0].behavior_version = 11;
                assert!(wrong.validate().is_err());
                wrong = p.clone();
                wrong.brushes[0]
                    .required_capabilities
                    .retain(|c| c != "dual-relative-size");
                assert!(wrong.validate().is_err());
            }
        }
    }
}
#[test]
fn sizing_dto_rejects_unknown_null_missing_and_pre_rounding_range() {
    let p = package(
        DualSizing::new(DualSizeMode::Relative, 8., DualWidthRule::FixedWidth).unwrap(),
        false,
    );
    let good = serde_json::to_value(&p).unwrap();
    for (field, value) in [
        ("model", serde_json::json!("unknown")),
        ("mode", serde_json::json!("bad")),
        ("widthRule", serde_json::json!("bad")),
        ("diameterOffset", serde_json::json!(511.000000001)),
        ("diameterOffset", serde_json::json!(-511.000000001)),
        ("diameterOffset", serde_json::json!("8")),
        ("extra", serde_json::json!(1)),
    ] {
        let mut bad = good.clone();
        bad["brushes"][0]["dualBrush"]["sizing"][field] = value;
        assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    for field in ["model", "mode", "diameterOffset", "widthRule"] {
        let mut bad = good.clone();
        bad["brushes"][0]["dualBrush"]["sizing"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let mut bad = good;
    bad["brushes"][0]["dualBrush"]["sizing"] = serde_json::Value::Null;
    assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    let default = BrushCatalog::default();
    let first = brush_state::encode(&default).unwrap();
    assert_eq!(
        first,
        brush_state::encode(&brush_state::decode(&first, MAX_CATALOG_MASK_BYTES).unwrap()).unwrap()
    );
    assert!(
        serde_json::from_slice::<serde_json::Value>(&first).unwrap()["version"]
            .as_u64()
            .unwrap()
            < 15
    );
}

#[test]
fn sizing_contract_and_manifest_downgrade_are_strict() {
    use std::io::{Cursor, Read, Write};
    use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};
    let token = CancellationToken::default();
    let p = package(
        DualSizing::new(DualSizeMode::Relative, 8., DualWidthRule::FixedWidth).unwrap(),
        false,
    );
    let bundle = vbr_image::ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let mut archive = ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        if file.name() == "manifest.json" {
            let mut json: serde_json::Value = serde_json::from_slice(&data).unwrap();
            assert_eq!(json["version"]["minor"], 13);
            json["version"]["minor"] = 12.into();
            data = serde_json::to_vec(&json).unwrap();
        }
        writer
            .start_file(file.name(), SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&data).unwrap();
    }
    assert!(vbr_image::decode(&writer.finish().unwrap().into_inner(), &token).is_err());
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-dual-sizing-v1.13.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["dualSizing"]["properties"]["model"]["const"],
        "relative-size-v1"
    );
    assert_eq!(
        schema["$defs"]["dualSizing"]["properties"]["diameterOffset"]["maximum"],
        511
    );
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        13
    );
}
