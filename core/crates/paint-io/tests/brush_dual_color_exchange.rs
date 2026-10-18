fn second(path: bool) -> paint_core::SecondaryTip {
    let s = paint_core::SecondaryTip::new(
        4.,
        paint_core::BrushTip::default(),
        None,
        Default::default(),
    )
    .unwrap();
    if path {
        s.with_spacing(0.25).unwrap()
    } else {
        s
    }
}
use paint_core::*;
use paint_io::{
    brush_state,
    vbr::{Package, Parameters, Preset},
    vbr_image,
};
use paint_task::CancellationToken;
use std::collections::BTreeMap;
fn package(color: DualColor, enabled: bool, path: bool) -> Package {
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            stroke_taper: None,
            wet_mix: None,
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "Smooth".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 13,
            required_capabilities: vec![
                "procedural-tip".into(),
                "dual-cover".into(),
                "dual-relative-size".into(),
                "dual-color-stroke".into(),
            ],
            parameters: Parameters {
                transfer_jitter: None,
                airbrush: None,
                speed_dynamics: None,
                tangent_dynamics: None,
                distance_dynamics: None,
                tilt_dynamics: None,
                rotation_dynamics: None,
                direction_dynamics: None,
                source_matrix: None,
                fade_dynamics: None,
                diameter: 12.,
                spacing: 0.25,
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
                    &DualBrush::new(enabled, DualMode::Multiply, second(path))
                        .with_sizing(
                            DualSizing::new(DualSizeMode::Relative, 8., DualWidthRule::FixedWidth)
                                .unwrap(),
                        )
                        .with_color(color),
                )
                .unwrap(),
            ),
            tip_resource: None,
            source_resource: None,
        }],
    }
}
fn draw(doc: &mut Document, mut definition: BrushDefinition) {
    doc.begin_stroke(
        Brush {
            radius: 60.,
            color: Pixel::from_straight([0.4, 0.5, 0.6, 0.6]).unwrap(),
            ..Default::default()
        },
        InputPoint::new(32.5, 32.5, 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    definition.write_blend = BlendMode::Multiply;

    for shift in [0., 6.] {
        definition.brush.color = Pixel::from_straight(if shift == 0. {
            [0.8, 0.2, 0.1, 0.7]
        } else {
            [0.1, 0.6, 0.3, 0.8]
        })
        .unwrap();
        definition.brush.background_color = Pixel::from_straight(if shift == 0. {
            [0.1, 0.3, 0.8, 0.8]
        } else {
            [0.8, 0.2, 0.1, 0.7]
        })
        .unwrap();
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
fn state_and_vbr_color_restore_two_real_strokes_with_closed_configuration() {
    let token = CancellationToken::default();
    for dual_enabled in [false, true] {
        for enabled in [false, true] {
            for path in [false, true] {
                for front in [false, true] {
                    let color = DualColor::new(
                        enabled,
                        true,
                        front,
                        if front {
                            DualColorOrder::SecondInFront
                        } else {
                            DualColorOrder::SecondBehind
                        },
                        BlendMode::Normal,
                        BlendMode::Screen,
                    )
                    .unwrap();
                    let mut p = package(color, dual_enabled, path);
                    p.brushes[0].smoothing =
                        Some(paint_io::brush_smoothing::SmoothingParameters::from_core(
                            BrushSmoothing::new(true, 12., 8., 0.5, 0.).unwrap(),
                        ));
                    p.brushes[0]
                        .required_capabilities
                        .push("path-smoothing".into());
                    let dynamics = BrushDynamics {
                        size: PressureControl::new(
                            PressureCurve::new_smooth(vec![(0., 0.), (0.5, 0.3), (1., 1.)])
                                .unwrap(),
                            0.1,
                        )
                        .unwrap(),
                        pressure_opacity: true,
                        ..Default::default()
                    };
                    p.brushes[0].parameters.pressure_dynamics = Some(
                        paint_io::brush_pressure::PressureParameters::from_core(&dynamics),
                    );
                    p.brushes[0].required_capabilities.extend([
                        "pressure-curves".into(),
                        "opacity-pressure".into(),
                        "pressure-smooth".into(),
                    ]);
                    let bundle =
                        vbr_image::ImageBundle::new(p.clone(), BTreeMap::new(), &token).unwrap();
                    let bytes = vbr_image::encode(&bundle, &token).unwrap();
                    let reopened = vbr_image::decode(&bytes, &token).unwrap();
                    assert_eq!(p.to_json().unwrap(), reopened.package.to_json().unwrap());
                    let original = bundle.definition(0).unwrap();
                    let mut settings = p.brushes[0].parameters.settings();
                    settings.dual = original.dual.clone();
                    settings.smoothing = original.smoothing;
                    let mut catalog = BrushCatalog::default();
                    catalog.edit("round-pressure", settings).unwrap();
                    let state = brush_state::encode(&catalog).unwrap();
                    let mut value: serde_json::Value = serde_json::from_slice(&state).unwrap();
                    assert_eq!(value["version"], 16);
                    let restored = brush_state::decode(&state, MAX_CATALOG_MASK_BYTES).unwrap();
                    assert_eq!(restored.selected().settings().dual.color(), color);
                    let mut a = Document::new(64, 64).unwrap();
                    let mut b = Document::new(64, 64).unwrap();
                    let mut c = Document::new(64, 64).unwrap();
                    draw(&mut a, original);
                    draw(&mut b, reopened.definition(0).unwrap());
                    draw(&mut c, restored.selected().settings().definition());
                    assert_eq!(pixels(&a), pixels(&b));
                    assert_eq!(pixels(&a), pixels(&c));
                    assert_eq!(a.history_depth(), (3, 0));
                    let expected = pixels(&c);
                    c.undo().unwrap();
                    c.redo().unwrap();
                    assert_eq!(pixels(&c), expected);
                    value["version"] = serde_json::json!(15);
                    assert!(brush_state::decode(
                        &serde_json::to_vec(&value).unwrap(),
                        MAX_CATALOG_MASK_BYTES
                    )
                    .is_err());
                    let mut bad = p.clone();
                    bad.brushes[0].behavior_version = 12;
                    assert!(bad.validate().is_err());
                    bad = p.clone();
                    bad.brushes[0]
                        .required_capabilities
                        .retain(|v| v != "dual-color-stroke");
                    assert!(bad.validate().is_err());
                }
            }
        }
    }
}
#[test]
fn color_dto_rejects_missing_null_unknown_models_and_nonpainting_modes() {
    let color = DualColor::new(
        false,
        true,
        true,
        DualColorOrder::SecondBehind,
        BlendMode::Normal,
        BlendMode::Multiply,
    )
    .unwrap();
    let p = package(color, false, true);
    let good = serde_json::to_value(&p).unwrap();
    for (field, value) in [
        ("model", serde_json::json!("per-dab-v1")),
        ("order", serde_json::json!("bad")),
        ("primaryBlend", serde_json::json!("pass-through")),
        ("secondaryBlend", serde_json::json!("unknown")),
        ("enabled", serde_json::json!(1)),
        ("useBackground", serde_json::json!("true")),
        ("overflow", serde_json::json!("true")),
        ("extra", serde_json::json!(false)),
    ] {
        let mut bad = good.clone();
        bad["brushes"][0]["dualBrush"]["color"][field] = value;
        assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    for field in [
        "model",
        "enabled",
        "useBackground",
        "overflow",
        "order",
        "primaryBlend",
        "secondaryBlend",
    ] {
        let mut bad = good.clone();
        bad["brushes"][0]["dualBrush"]["color"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_ok());
    }
    let mut bad = good;
    bad["brushes"][0]["dualBrush"]["color"] = serde_json::Value::Null;
    assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap())
        .unwrap()
        .brushes[0]
        .dual_brush
        .as_ref()
        .unwrap()
        .has_color()
        .eq(&false));
}

#[test]
fn color_contract_and_manifest_downgrade_are_strict() {
    use std::io::{Cursor, Read, Write};
    use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};
    let token = CancellationToken::default();
    let p = package(
        DualColor::new(
            false,
            true,
            true,
            DualColorOrder::SecondBehind,
            BlendMode::Normal,
            BlendMode::Multiply,
        )
        .unwrap(),
        false,
        true,
    );
    assert!(paint_io::vbr::encode(&p, &token).is_err());
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
            assert_eq!(json["version"]["minor"], 0);
            json["version"]["minor"] = 13.into();
            data = serde_json::to_vec(&json).unwrap();
        }
        writer
            .start_file(file.name(), SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&data).unwrap();
    }
    assert!(vbr_image::decode(&writer.finish().unwrap().into_inner(), &token).is_ok());
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/schema-history/dual/vbr-dual-color-v1.14.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["dualColor"]["properties"]["model"]["const"],
        "separated-stroke-v1"
    );
    assert_eq!(
        schema["$defs"]["dualColor"]["properties"]["primaryBlend"]["enum"]
            .as_array()
            .unwrap()
            .len(),
        27
    );
    for field in ["primaryBlend", "secondaryBlend"] {
        let names = schema["$defs"]["dualColor"]["properties"][field]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(names, paint_io::brush_dual_color::BLEND_NAMES);
    }
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        14
    );
}
