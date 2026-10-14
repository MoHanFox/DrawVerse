use paint_core::*;
use paint_io::{
    brush_smoothing::SmoothingParameters,
    brush_state,
    vbr::{Package, Parameters, Preset},
    vbr_image,
};
use paint_task::CancellationToken;
use std::collections::BTreeMap;
fn package(smoothing: BrushSmoothing) -> Package {
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "Smooth".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 11,
            required_capabilities: vec!["procedural-tip".into(), "path-smoothing".into()],
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
            smoothing: Some(SmoothingParameters::from_core(smoothing)),
            texture: None,
            dual_brush: None,
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
fn state_and_vbr_smoothing_preserve_real_two_strokes_and_closed_values() {
    for enabled in [false, true] {
        let smoothing = BrushSmoothing::new(enabled, 24., 12., 0.7, 3.).unwrap();
        let mut p = package(smoothing);
        let dynamics = BrushDynamics {
            size: PressureControl::new(
                PressureCurve::new_smooth(vec![(0., 0.), (0.5, 0.25), (1., 1.)]).unwrap(),
                0.1,
            )
            .unwrap(),
            opacity: PressureControl::new(PressureCurve::default(), 0.2).unwrap(),
            pressure_opacity: true,
            ..Default::default()
        };
        let dual = DualBrush::new(
            true,
            DualMode::Multiply,
            SecondaryTip::new(4., BrushTip::default(), None, BrushDynamics::default())
                .unwrap()
                .with_spacing(0.04)
                .unwrap(),
        );
        p.brushes[0].parameters.pressure_dynamics = Some(
            paint_io::brush_pressure::PressureParameters::from_core(&dynamics),
        );
        p.brushes[0].dual_brush =
            Some(paint_io::brush_dual::DualParameters::from_core(&dual).unwrap());
        p.brushes[0].required_capabilities.extend([
            "pressure-curves".into(),
            "opacity-pressure".into(),
            "pressure-smooth".into(),
            "dual-cover".into(),
        ]);
        let token = CancellationToken::default();
        let bundle = vbr_image::ImageBundle::new(p.clone(), BTreeMap::new(), &token).unwrap();
        let bytes = vbr_image::encode(&bundle, &token).unwrap();
        let restored = vbr_image::decode(&bytes, &token).unwrap();
        assert_eq!(p.to_json().unwrap(), restored.package.to_json().unwrap());
        let mut catalog = BrushCatalog::default();
        let settings = BrushSettings {
            smoothing,
            dynamics,
            dual,
            diameter: 12.,
            spacing: 0.04,
            opacity: 0.4,
            ..Default::default()
        };
        catalog.edit("round-pressure", settings).unwrap();
        let state = brush_state::encode(&catalog).unwrap();
        let restored_state = brush_state::decode(&state, MAX_CATALOG_MASK_BYTES).unwrap();
        let original = catalog.selected().settings().definition();
        let mut a = Document::new(64, 64).unwrap();
        let mut b = Document::new(64, 64).unwrap();
        let mut c = Document::new(64, 64).unwrap();
        draw(&mut a, original.clone());
        draw(&mut b, restored.definition(0).unwrap());
        draw(&mut c, restored_state.selected().settings().definition());
        assert_eq!(pixels(&a), pixels(&b));
        assert_eq!(pixels(&a), pixels(&c));
        assert_eq!(a.history_depth(), (2, 0));
        let expected = pixels(&b);
        b.undo().unwrap();
        b.redo().unwrap();
        assert_eq!(pixels(&b), expected);
        let mut unsmoothed = original;
        unsmoothed.smoothing = BrushSmoothing::default();
        let mut d = Document::new(64, 64).unwrap();
        draw(&mut d, unsmoothed);
        if enabled {
            assert_ne!(pixels(&a), pixels(&d));
        } else {
            assert_eq!(pixels(&a), pixels(&d));
        }
        let mut bad = p.clone();
        bad.brushes[0].behavior_version = 10;
        assert!(bad.validate().is_err());
        bad = p.clone();
        bad.brushes[0]
            .required_capabilities
            .retain(|c| c != "path-smoothing");
        assert!(bad.validate().is_err());
        assert!(paint_io::vbr::encode(&p, &token).is_err());
    }
}
#[test]
fn smoothing_dto_rejects_unknown_null_and_pre_rounding_ranges_atomically() {
    let p = package(BrushSmoothing::new(false, 24., 12., 0.7, 3.).unwrap());
    let good = serde_json::to_value(&p).unwrap();
    for (field, value) in [
        ("model", serde_json::json!("unknown")),
        ("positionMs", serde_json::json!(250.0000000001)),
        ("pressureMs", serde_json::json!(-0.0000000001)),
        ("speedResponse", serde_json::json!(1.0000000001)),
        ("stabilizerPx", serde_json::json!(128.0000000001)),
        ("enabled", serde_json::json!(1)),
    ] {
        let mut bad = good.clone();
        bad["brushes"][0]["smoothing"][field] = value;
        assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let mut bad = good.clone();
    bad["brushes"][0]["smoothing"] = serde_json::Value::Null;
    assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    bad = good.clone();
    bad["brushes"][0]["smoothing"]
        .as_object_mut()
        .unwrap()
        .remove("positionMs");
    assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    let mut old = good;
    old["brushes"][0]
        .as_object_mut()
        .unwrap()
        .remove("smoothing");
    old["brushes"][0]["behaviorVersion"] = 1.into();
    old["brushes"][0]["requiredCapabilities"] = serde_json::json!(["procedural-tip"]);
    let old = Package::from_json(&serde_json::to_vec(&old).unwrap()).unwrap();
    assert!(old.brushes[0].smoothing.is_none());
}

#[test]
fn smoothing_contract_and_manifest_downgrade_are_strict() {
    use std::io::{Cursor, Read, Write};
    use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};
    let token = CancellationToken::default();
    let p = package(BrushSmoothing::new(false, 24., 12., 0.7, 3.).unwrap());
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
            assert_eq!(json["version"]["minor"], 12);
            json["version"]["minor"] = 11.into();
            data = serde_json::to_vec(&json).unwrap();
        }
        writer
            .start_file(file.name(), SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&data).unwrap();
    }
    assert!(vbr_image::decode(&writer.finish().unwrap().into_inner(), &token).is_err());
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-smoothing-v1.12.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["smoothing"]["properties"]["model"]["const"],
        "time-grid-v1"
    );
    assert_eq!(
        schema["$defs"]["smoothing"]["properties"]["positionMs"]["maximum"],
        250
    );
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        12
    );
}
