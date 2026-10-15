use paint_io::{
    abr_association::LinkedPreset,
    abr_conversion::{convert, Status},
    abr_sampled::SectionedTip,
    ps_descriptor::{Class, Descriptor, Value},
};
use paint_task::CancellationToken;
use std::sync::Arc;
fn object(id: &str, entries: Vec<(&str, Value)>) -> Descriptor {
    Descriptor {
        class: Class {
            name: String::new(),
            id: id.as_bytes().to_vec(),
        },
        entries: entries
            .into_iter()
            .map(|(key, value)| (key.as_bytes().to_vec(), value))
            .collect(),
    }
}
fn unit(unit: [u8; 4], value: f64) -> Value {
    Value::UnitDouble { unit, value }
}
fn shape(sampled: bool) -> Descriptor {
    object(
        if sampled {
            "sampledBrush"
        } else {
            "computedBrush"
        },
        vec![
            ("Dmtr", unit(*b"#Pxl", 16.)),
            ("Angl", unit(*b"#Ang", 0.)),
            ("Rndn", unit(*b"#Prc", 100.)),
            ("Spcn", unit(*b"#Prc", 2.)),
            ("Hrdn", unit(*b"#Prc", 100.)),
            ("Intr", Value::Boolean(true)),
            ("flipX", Value::Boolean(false)),
            ("flipY", Value::Boolean(false)),
        ],
    )
}
fn preset() -> Descriptor {
    object(
        "brushPreset",
        vec![(
            "toolOptions",
            Value::Object {
                global: false,
                descriptor: Box::new(object(
                    "brush",
                    vec![("flow", Value::Integer(100)), ("Opct", Value::Integer(100))],
                )),
            },
        )],
    )
}
#[test]
fn procedural_and_image_baselines_compile_and_use_actual_coverage_without_added_pressure() {
    let preset = preset();
    let procedural = shape(false);
    let mut image_shape = shape(true);
    image_shape.entries.retain(|(key, _)| key != b"Hrdn");
    let mut coverage = vec![255; 256];
    for y in 6..10 {
        for x in 6..10 {
            coverage[y * 16 + x] = 0;
        }
    }
    let tip = SectionedTip {
        id: "identity".into(),
        prefix: vec![].into(),
        bounds: [0, 0, 16, 16],
        mask: Arc::new(paint_core::BrushMask::new(16, 16, coverage).unwrap()),
        trailing_metadata: vec![].into(),
    };
    let linked = [
        LinkedPreset {
            name: "同名",
            preset: &preset,
            shape: &procedural,
            sampled_tip: None,
        },
        LinkedPreset {
            name: "同名",
            preset: &preset,
            shape: &image_shape,
            sampled_tip: Some(&tip),
        },
    ];
    let converted = convert(&linked, &CancellationToken::default()).unwrap();
    assert_eq!(converted.len(), 2);
    assert!(converted.iter().all(|p| !p.is_partial()));
    assert_eq!(converted[0].settings.spacing, 0.02);
    assert_eq!(converted[0].settings.diameter, 16.);
    assert!(!converted[0].settings.tip.pressure_size && !converted[0].settings.tip.pressure_flow);
    assert!(Arc::ptr_eq(
        converted[1].settings.mask.as_ref().unwrap(),
        &tip.mask
    ));
    for (i, preset) in converted.iter().enumerate() {
        let mut d = paint_core::Document::new(40, 40).unwrap();
        d.begin_brush_stroke(
            preset.settings.definition(),
            paint_core::InputPoint::new(20.5, 20.5, 0.1),
        )
        .unwrap();
        d.end_stroke().unwrap();
        assert!(d.pixel(25, 20).alpha() > 0.9);
        if i == 1 {
            assert_eq!(d.pixel(20, 20).alpha(), 0.);
        } else {
            assert!(d.pixel(20, 20).alpha() > 0.9);
        }
    }
}
#[test]
fn unknown_dynamic_paths_and_partial_tool_opacity_are_reported_without_imitation() {
    let mut source = preset();
    source.entries.push((
        b"szVr".to_vec(),
        Value::Object {
            global: false,
            descriptor: Box::new(object(
                "dynamics",
                vec![("bVTy", Value::Integer(2)), ("jitter", unit(*b"#Prc", 12.))],
            )),
        },
    ));
    if let Value::Object { descriptor, .. } = &mut source.entries[0].1 {
        descriptor.entries[1].1 = Value::Double(99.999999);
    }
    let mut shape = shape(false);
    shape.entries[1].1 = unit(*b"#Ang", 45.);
    shape.entries[5].1 = Value::Boolean(false);
    shape.entries[6].1 = Value::Boolean(true);
    let linked = [LinkedPreset {
        name: "部分转换",
        preset: &source,
        shape: &shape,
        sampled_tip: None,
    }];
    let converted =
        paint_io::abr_conversion::convert_legacy(&linked, &CancellationToken::default()).unwrap();
    let p = &converted[0];
    assert!(p.is_partial());
    assert_eq!(p.settings.opacity, 1.);
    assert_eq!(p.settings.tip.angle, 45.);
    for key in [
        "szVr.bVTy",
        "szVr.jitter",
        "toolOptions.Opct",
        "Brsh.Intr",
        "Brsh.flipX",
    ] {
        assert!(
            p.issues
                .iter()
                .any(|i| i.path == key && i.status == Status::NotExecuted),
            "{key}"
        );
    }
    assert!(p
        .issues
        .iter()
        .any(|i| i.path == "Brsh.Angl" && i.status == Status::Approximate));
    assert!(std::ptr::eq(p.source, &source));
}
#[test]
fn units_missing_fields_range_and_late_failure_never_return_a_partial_batch() {
    let source = preset();
    let token = CancellationToken::default();
    let original = shape(false);
    for bad_value in [
        unit(*b"#Prc", 16.),
        unit(*b"#Pxl", 513.),
        unit(*b"#Pxl", f64::NAN),
        Value::Integer(16),
    ] {
        let mut bad = shape(false);
        bad.entries[0].1 = bad_value;
        let linked = [
            LinkedPreset {
                name: "合法",
                preset: &source,
                shape: &original,
                sampled_tip: None,
            },
            LinkedPreset {
                name: "非法",
                preset: &source,
                shape: &bad,
                sampled_tip: None,
            },
        ];
        assert!(convert(&linked, &token).is_err());
    }
    let mut missing = shape(false);
    missing.entries.remove(3);
    assert!(convert(
        &[LinkedPreset {
            name: "缺间距",
            preset: &source,
            shape: &missing,
            sampled_tip: None
        }],
        &token
    )
    .is_err());
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(convert(
        &[LinkedPreset {
            name: "取消",
            preset: &source,
            shape: &original,
            sampled_tip: None
        }],
        &cancelled
    )
    .is_err());
}
#[test]
fn huge_unknown_reports_reject_with_a_bound_and_never_hide_the_extra_fields() {
    let mut source = preset();
    source.entries.push((
        b"many".to_vec(),
        Value::List((0..4096).map(|_| Value::Boolean(true)).collect()),
    ));
    let shape = shape(false);
    assert!(convert(
        &[LinkedPreset {
            name: "报告超限",
            preset: &source,
            shape: &shape,
            sampled_tip: None
        }],
        &CancellationToken::default()
    )
    .is_err());
}

#[test]
fn stroke_opacity_mapping_is_versioned_and_range_checked_before_rounding() {
    let shape = shape(false);
    let token = CancellationToken::default();
    for value in [0., 0.1, 25.5, 99.999999, 100.] {
        let mut source = preset();
        if let Value::Object { descriptor, .. } = &mut source.entries[0].1 {
            descriptor.entries[1].1 = Value::Double(value);
        }
        let linked = [LinkedPreset {
            name: "opacity",
            preset: &source,
            shape: &shape,
            sampled_tip: None,
        }];
        let current = convert(&linked, &token).unwrap();
        let legacy = paint_io::abr_conversion::convert_legacy(&linked, &token).unwrap();
        assert_eq!(current[0].settings.opacity, (value / 100.) as f32);
        assert_eq!(
            current[0].converter_version,
            if value < 100. { 2 } else { 1 }
        );
        assert!(current[0]
            .issues
            .iter()
            .any(|i| i.path == "toolOptions.Opct" && i.status == Status::Mapped));
        assert_eq!(legacy[0].converter_version, 1);
        assert_eq!(legacy[0].settings.opacity, 1.);
        assert!(legacy[0].issues.iter().any(|i| i.path == "toolOptions.Opct"
            && i.status
                == if value < 100. {
                    Status::NotExecuted
                } else {
                    Status::Mapped
                }));
    }
    for value in [
        Value::Double(-f64::EPSILON),
        Value::Double(100. + 1e-10),
        Value::Double(f64::NAN),
        Value::Boolean(true),
        unit(*b"#Prc", 50.),
    ] {
        let mut source = preset();
        if let Value::Object { descriptor, .. } = &mut source.entries[0].1 {
            descriptor.entries[1].1 = value;
        }
        assert!(convert(
            &[LinkedPreset {
                name: "invalid",
                preset: &source,
                shape: &shape,
                sampled_tip: None
            }],
            &token
        )
        .is_err());
    }
}

#[test]
fn converted_program_and_image_opacity_cap_dense_strokes_with_exact_history() {
    use paint_core::{Document, InputPoint};
    let token = CancellationToken::default();
    let mut source = preset();
    if let Value::Object { descriptor, .. } = &mut source.entries[0].1 {
        descriptor.entries[1].1 = Value::Double(25.5);
    }
    let tip = SectionedTip {
        id: "opacity".into(),
        prefix: vec![].into(),
        bounds: [0, 0, 4, 4],
        mask: Arc::new(
            paint_core::BrushMask::new(
                4,
                4,
                vec![
                    255, 255, 255, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 255, 255, 255,
                ],
            )
            .unwrap(),
        ),
        trailing_metadata: vec![].into(),
    };
    let pixels = |d: &Document| {
        (0..64)
            .flat_map(|y| (0..128).map(move |x| d.pixel(x, y)))
            .collect::<Vec<_>>()
    };
    for sampled in [false, true] {
        let shape = shape(sampled);
        let converted = convert(
            &[LinkedPreset {
                name: "stroke",
                preset: &source,
                shape: &shape,
                sampled_tip: sampled.then_some(&tip),
            }],
            &token,
        )
        .unwrap();
        let settings = &converted[0].settings;
        let mut doc = Document::new(128, 64).unwrap();
        doc.begin_brush_stroke(settings.definition(), InputPoint::new(32.5, 32.5, 1.))
            .unwrap();
        for x in [64.5, 32.5, 64.5, 32.5] {
            doc.stroke_to(InputPoint::new(x, 32.5, 1.)).unwrap();
        }
        doc.end_stroke().unwrap();
        let after = pixels(&doc);
        assert!(after.iter().all(|p| p.alpha() <= 0.255001));
        assert!(after.iter().any(|p| (p.alpha() - 0.255).abs() < 1e-6));
        assert_eq!(doc.history_depth(), (1, 0));
        doc.undo().unwrap();
        assert!(pixels(&doc).iter().all(|p| p.alpha() == 0.));
        doc.redo().unwrap();
        assert_eq!(pixels(&doc), after);
        doc.begin_brush_stroke(settings.definition(), InputPoint::new(96.5, 32.5, 1.))
            .unwrap();
        doc.cancel_stroke().unwrap();
        assert_eq!(pixels(&doc), after);
        assert_eq!(doc.history_depth(), (1, 0));
    }
}

#[test]
fn many_individually_valid_reports_are_bounded_before_entire_batch_accumulates() {
    let mut source = preset();
    for index in 0..1000 {
        source.entries.push((
            format!("{}.{index}", "x".repeat(180)).into_bytes(),
            Value::Boolean(true),
        ));
    }
    let shape = shape(false);
    let linked = (0..200)
        .map(|_| LinkedPreset {
            name: "same",
            preset: &source,
            shape: &shape,
            sampled_tip: None,
        })
        .collect::<Vec<_>>();
    assert!(matches!(
        convert(&linked, &CancellationToken::default()),
        Err(paint_io::Error::Limit("ABR batch report budget"))
    ));
}

#[test]
fn published_opacity_source_versions_match_codec_contract() {
    for bytes in [
        include_bytes!("../../../../contracts/fixtures/abr-opacity-source-v1-valid.json")
            .as_slice(),
        include_bytes!("../../../../contracts/fixtures/abr-opacity-source-v2-valid.json")
            .as_slice(),
    ] {
        let info: paint_io::brush_source::SourceInfo = serde_json::from_slice(bytes).unwrap();
        info.validate().unwrap();
    }
    let info: paint_io::brush_source::SourceInfo = serde_json::from_slice(include_bytes!(
        "../../../../contracts/fixtures/abr-opacity-source-invalid-version.json"
    ))
    .unwrap();
    assert!(info.validate().is_err());
}

fn pressure_preset() -> Descriptor {
    let mut source = preset();
    for key in ["useTipDynamics", "usePaintDynamics"] {
        source
            .entries
            .push((key.as_bytes().to_vec(), Value::Boolean(true)));
    }
    source
        .entries
        .push((b"minimumDiameter".to_vec(), unit(*b"#Prc", 20.)));
    for (key, min) in [("szVr", 20.), ("prVr", 40.), ("opVr", 30.)] {
        source.entries.push((
            key.as_bytes().to_vec(),
            Value::Object {
                global: false,
                descriptor: Box::new(object(
                    "brushDynamics",
                    vec![
                        ("bVTy", Value::Integer(2)),
                        ("Mnm ", unit(*b"#Prc", min)),
                        ("jitter", unit(*b"#Prc", 0.)),
                        ("fStp", Value::Integer(0)),
                    ],
                )),
            },
        ));
    }
    source
}
#[test]
fn pressure_channels_minima_reports_legacy_and_real_pixels() {
    use paint_core::{Document, InputPoint};
    let token = CancellationToken::default();
    let source = pressure_preset();
    let shape = shape(false);
    let linked = [LinkedPreset {
        name: "pressure",
        preset: &source,
        shape: &shape,
        sampled_tip: None,
    }];
    let current = convert(&linked, &token).unwrap();
    let p = &current[0];
    assert_eq!(p.converter_version, 3);
    assert!(!p.is_partial());
    assert!(
        p.settings.tip.pressure_size
            && p.settings.tip.pressure_flow
            && p.settings.dynamics.pressure_opacity
    );
    for (control, min) in [
        (&p.settings.dynamics.size, 0.2),
        (&p.settings.dynamics.flow, 0.4),
        (&p.settings.dynamics.opacity, 0.3),
    ] {
        assert_eq!(control.minimum(), min);
        assert!((control.response(0.5).unwrap() - (min + (1. - min) * 0.5)).abs() < 1e-6);
    }
    for old in [
        paint_io::abr_conversion::convert_legacy(&linked, &token).unwrap(),
        paint_io::abr_conversion::convert_v2(&linked, &token).unwrap(),
    ] {
        assert_eq!(old[0].converter_version, 1);
        assert!(!old[0].settings.tip.pressure_size);
        assert_eq!(old[0].settings.dynamics, Default::default());
        assert!(old[0]
            .issues
            .iter()
            .any(|i| i.path == "szVr.bVTy" && i.status == Status::NotExecuted));
    }
    for pressure in [0., 0.25, 0.5, 1.] {
        let mut doc = Document::new(64, 32).unwrap();
        doc.begin_brush_stroke(
            p.settings.definition(),
            InputPoint::new(16.5, 16.5, pressure),
        )
        .unwrap();
        for x in [48.5, 16.5, 48.5] {
            doc.stroke_to(InputPoint::new(x, 16.5, pressure)).unwrap();
        }
        doc.end_stroke().unwrap();
        let cap = 0.3 + 0.7 * pressure;
        assert!(doc.pixel(32, 16).alpha() >= cap - 1e-5);
        assert!(doc.pixel(32, 16).alpha() <= cap + 1e-5);
        let after = (0..2048)
            .map(|i| doc.pixel(i % 64, i / 64))
            .collect::<Vec<_>>();
        assert!(after.iter().all(|c| c.alpha() <= cap + 1e-5));
        let radius = 8. * (0.2 + 0.8 * pressure);
        assert_eq!(doc.pixel(32, 16 + (radius.ceil() as u32) + 1).alpha(), 0.);
        doc.undo().unwrap();
        assert_eq!(doc.pixel(32, 16).alpha(), 0.);
        doc.redo().unwrap();
        assert_eq!(
            (0..2048)
                .map(|i| doc.pixel(i % 64, i / 64))
                .collect::<Vec<_>>(),
            after
        );
    }
}
#[test]
fn pressure_switches_overrides_unknown_controls_and_random_fields_are_honest() {
    let token = CancellationToken::default();
    let shape = shape(false);
    for mode in 0..6 {
        let mut source = pressure_preset();
        match mode {
            0 => {
                source
                    .entries
                    .iter_mut()
                    .find(|(k, _)| k == b"useTipDynamics")
                    .unwrap()
                    .1 = Value::Boolean(false);
            }
            1 => {
                if let Value::Object { descriptor, .. } = &mut source
                    .entries
                    .iter_mut()
                    .find(|(k, _)| k == b"szVr")
                    .unwrap()
                    .1
                {
                    descriptor.entries[0].1 = Value::Integer(1);
                }
            }
            2 => {
                if let Value::Object { descriptor, .. } = &mut source
                    .entries
                    .iter_mut()
                    .find(|(k, _)| k == b"szVr")
                    .unwrap()
                    .1
                {
                    descriptor.entries[2].1 = unit(*b"#Prc", 25.);
                }
            }
            3 => {
                if let Value::Object { descriptor, .. } = &mut source.entries[0].1 {
                    descriptor
                        .entries
                        .push((b"usePressureOverridesSize".to_vec(), Value::Boolean(true)));
                }
            }
            4 => {
                source
                    .entries
                    .push((b"useBrushPose".to_vec(), Value::Boolean(true)));
            }
            _ => {
                source
                    .entries
                    .iter_mut()
                    .find(|(k, _)| k == b"minimumDiameter")
                    .unwrap()
                    .1 = unit(*b"#Prc", 50.);
            }
        }
        let converted = convert(
            &[LinkedPreset {
                name: "partial",
                preset: &source,
                shape: &shape,
                sampled_tip: None,
            }],
            &token,
        )
        .unwrap();
        let p = &converted[0];
        assert!(p.is_partial());
        assert_eq!(p.settings.tip.pressure_size, mode == 2 || mode == 5);
        if mode == 5 {
            assert_eq!(p.settings.dynamics.size.minimum(), 0.5);
            assert!(p
                .issues
                .iter()
                .any(|i| i.path == "szVr.Mnm " && i.status == Status::NotExecuted));
        }
        if mode == 2 {
            assert!(p
                .issues
                .iter()
                .any(|i| i.path == "szVr.jitter" && i.status == Status::NotExecuted));
        }
        if mode == 4 {
            assert_eq!(p.converter_version, 1);
            assert!(!p.settings.tip.pressure_flow && !p.settings.dynamics.pressure_opacity);
        }
    }
}
#[test]
fn enabled_pressure_rejects_bad_units_types_and_pre_rounding_bounds() {
    let shape = shape(false);
    let token = CancellationToken::default();
    for bad in [
        unit(*b"#Prc", -1e-10),
        unit(*b"#Prc", 100. + 1e-10),
        unit(*b"#Prc", f64::NAN),
        unit(*b"#Pxl", 50.),
        Value::Integer(20),
    ] {
        let mut source = pressure_preset();
        if let Value::Object { descriptor, .. } = &mut source
            .entries
            .iter_mut()
            .find(|(k, _)| k == b"opVr")
            .unwrap()
            .1
        {
            descriptor.entries[1].1 = bad;
        }
        assert!(convert(
            &[LinkedPreset {
                name: "bad",
                preset: &source,
                shape: &shape,
                sampled_tip: None
            }],
            &token
        )
        .is_err());
    }
}

#[test]
fn published_pressure_source_contract_guards_downgrades_and_unknown_versions() {
    let valid: paint_io::brush_source::SourceInfo = serde_json::from_slice(include_bytes!(
        "../../../../contracts/fixtures/abr-pressure-source-v3-valid.json"
    ))
    .unwrap();
    valid.validate().unwrap();
    for bytes in [
        include_bytes!("../../../../contracts/fixtures/abr-pressure-source-invalid-version.json")
            .as_slice(),
        include_bytes!("../../../../contracts/fixtures/abr-pressure-source-invalid-downgrade.json")
            .as_slice(),
    ] {
        let info: paint_io::brush_source::SourceInfo = serde_json::from_slice(bytes).unwrap();
        assert!(info.validate().is_err());
    }
}

#[test]
fn pressure_channels_are_independent_and_literal_dotted_keys_stay_reported() {
    let token = CancellationToken::default();
    let shape = shape(false);
    for channel in [b"szVr".as_slice(), b"prVr", b"opVr"] {
        let mut source = pressure_preset();
        source.entries.retain(|(k, _)| {
            ![b"szVr".as_slice(), b"prVr", b"opVr"].contains(&k.as_slice()) || k == channel
        });
        source
            .entries
            .push((b"szVr.bVTy".to_vec(), Value::Boolean(true)));
        let p = convert(
            &[LinkedPreset {
                name: "independent",
                preset: &source,
                shape: &shape,
                sampled_tip: None,
            }],
            &token,
        )
        .unwrap();
        assert_eq!(p[0].settings.tip.pressure_size, channel == b"szVr");
        assert_eq!(p[0].settings.tip.pressure_flow, channel == b"prVr");
        assert_eq!(p[0].settings.dynamics.pressure_opacity, channel == b"opVr");
        assert!(p[0]
            .issues
            .iter()
            .any(|i| i.path == "szVr.bVTy" && i.status == Status::NotExecuted));
    }
}

fn scatter_preset(enabled: bool, both: bool) -> Descriptor {
    let mut source = pressure_preset();
    source
        .entries
        .push((b"useScatter".to_vec(), Value::Boolean(enabled)));
    source
        .entries
        .push((b"bothAxes".to_vec(), Value::Boolean(both)));
    source.entries.push((b"Cnt ".to_vec(), Value::Double(3.)));
    for (key, jitter) in [("scatterDynamics", 75.), ("countDynamics", 50.)] {
        source.entries.push((
            key.as_bytes().to_vec(),
            Value::Object {
                global: false,
                descriptor: Box::new(object(
                    "brushDynamics",
                    vec![
                        ("bVTy", Value::Integer(0)),
                        ("fStp", Value::Integer(25)),
                        ("Mnm ", unit(*b"#Prc", 0.)),
                        ("jitter", unit(*b"#Prc", jitter)),
                    ],
                )),
            },
        ));
    }
    source
}
#[test]
fn scatter_conversion_uses_real_program_image_pressure_and_two_stroke_history() {
    use paint_core::{BrushMask, Document, InputPoint, ScatterDynamics};
    let token = CancellationToken::default();
    let mask = SectionedTip {
        id: "scatter".into(),
        prefix: Box::new([]),
        bounds: [0, 0, 4, 4],
        mask: Arc::new(
            BrushMask::new(
                4,
                4,
                vec![
                    255, 255, 255, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 255, 255, 255,
                ],
            )
            .unwrap(),
        ),
        trailing_metadata: Box::new([]),
    };
    for enabled in [false, true] {
        for both in [false, true] {
            for sampled in [false, true] {
                let source = scatter_preset(enabled, both);
                let shape = shape(sampled);
                let linked = [LinkedPreset {
                    name: "scatter",
                    preset: &source,
                    shape: &shape,
                    sampled_tip: sampled.then_some(&mask),
                }];
                let converted = convert(&linked, &token).unwrap();
                let preset = &converted[0];
                assert_eq!(preset.converter_version, 4);
                assert_eq!(
                    preset.settings.dynamics.scatter,
                    ScatterDynamics::new(enabled, 0.75, both, 3, 0.5, 0).unwrap()
                );
                if enabled {
                    assert!(preset.is_partial());
                }
                for old in [
                    paint_io::abr_conversion::convert_legacy(&linked, &token).unwrap(),
                    paint_io::abr_conversion::convert_v2(&linked, &token).unwrap(),
                    paint_io::abr_conversion::convert_v3(&linked, &token).unwrap(),
                ] {
                    assert_eq!(old[0].settings.dynamics.scatter, Default::default());
                    assert!(old[0]
                        .issues
                        .iter()
                        .any(|i| i.path == "scatterDynamics.jitter"
                            && i.status == Status::NotExecuted));
                }
                let mut reference = preset.settings.clone();
                reference.dynamics.scatter =
                    ScatterDynamics::new(enabled, 0.75, both, 3, 0.5, 0).unwrap();
                let mut a = Document::new(64, 64).unwrap();
                let mut b = Document::new(64, 64).unwrap();
                for pressure in [0.25, 0.75] {
                    for (doc, s) in [(&mut a, &preset.settings), (&mut b, &reference)] {
                        doc.begin_brush_stroke(
                            s.definition(),
                            InputPoint::new(20.5, 32.5, pressure),
                        )
                        .unwrap();
                        doc.stroke_to(InputPoint::new(44.5, 32.5, pressure))
                            .unwrap();
                        doc.end_stroke().unwrap();
                    }
                }
                let after = (0..4096)
                    .map(|i| a.pixel(i % 64, i / 64))
                    .collect::<Vec<_>>();
                assert_eq!(
                    after,
                    (0..4096)
                        .map(|i| b.pixel(i % 64, i / 64))
                        .collect::<Vec<_>>()
                );
                assert_eq!(a.history_depth(), (2, 0));
                if enabled {
                    assert!((0..4096)
                        .any(|i| !(24..=40).contains(&(i / 64)) && after[i as usize].alpha() > 0.));
                }
                a.undo().unwrap();
                assert_eq!(a.history_depth(), (1, 1));
                a.redo().unwrap();
                assert_eq!(
                    after,
                    (0..4096)
                        .map(|i| a.pixel(i % 64, i / 64))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}
#[test]
fn scatter_preserves_unknown_controls_and_rejects_fractional_counts_and_bad_ranges() {
    let shape = shape(false);
    let token = CancellationToken::default();
    for bad in [
        Value::Double(3.000001),
        Value::Double(f64::NAN),
        Value::Integer(0),
        Value::Integer(17),
        unit(*b"#Prc", 3.),
    ] {
        let mut source = scatter_preset(true, true);
        source
            .entries
            .iter_mut()
            .find(|(k, _)| k == b"Cnt ")
            .unwrap()
            .1 = bad;
        assert!(convert(
            &[LinkedPreset {
                name: "bad",
                preset: &source,
                shape: &shape,
                sampled_tip: None
            }],
            &token
        )
        .is_err());
    }
    for value in [-1e-10, 1000. + 1e-9, f64::NAN] {
        let mut source = scatter_preset(true, true);
        if let Value::Object { descriptor, .. } = &mut source
            .entries
            .iter_mut()
            .find(|(k, _)| k == b"scatterDynamics")
            .unwrap()
            .1
        {
            descriptor.entries[3].1 = unit(*b"#Prc", value);
        }
        assert!(convert(
            &[LinkedPreset {
                name: "bad",
                preset: &source,
                shape: &shape,
                sampled_tip: None
            }],
            &token
        )
        .is_err());
    }
    let mut source = scatter_preset(true, true);
    if let Value::Object { descriptor, .. } = &mut source
        .entries
        .iter_mut()
        .find(|(k, _)| k == b"countDynamics")
        .unwrap()
        .1
    {
        descriptor.entries[0].1 = Value::Integer(2);
    }
    let converted = convert(
        &[LinkedPreset {
            name: "partial",
            preset: &source,
            shape: &shape,
            sampled_tip: None,
        }],
        &token,
    )
    .unwrap();
    assert!(converted[0]
        .issues
        .iter()
        .any(|i| i.path == "countDynamics.bVTy" && i.status == Status::NotExecuted));
    assert!(converted[0].settings.dynamics.scatter.enabled());
}

#[test]
fn published_scatter_source_contract_guards_old_versions_and_future_versions() {
    let info: paint_io::brush_source::SourceInfo = serde_json::from_slice(include_bytes!(
        "../../../../contracts/fixtures/abr-scatter-source-v4-valid.json"
    ))
    .unwrap();
    info.validate().unwrap();
    for bytes in [
        include_bytes!("../../../../contracts/fixtures/abr-scatter-source-invalid-version.json")
            .as_slice(),
        include_bytes!("../../../../contracts/fixtures/abr-scatter-source-invalid-downgrade.json")
            .as_slice(),
    ] {
        let info: paint_io::brush_source::SourceInfo = serde_json::from_slice(bytes).unwrap();
        assert!(info.validate().is_err());
    }
}

#[test]
fn default_closed_scatter_keeps_v3_reports_and_does_not_upgrade() {
    let mut source = scatter_preset(false, false);
    source
        .entries
        .iter_mut()
        .find(|(k, _)| k == b"Cnt ")
        .unwrap()
        .1 = Value::Double(1.);
    for (key, value) in &mut source.entries {
        if key == b"scatterDynamics" || key == b"countDynamics" {
            if let Value::Object { descriptor, .. } = value {
                descriptor
                    .entries
                    .iter_mut()
                    .find(|(k, _)| k == b"jitter")
                    .unwrap()
                    .1 = unit(*b"#Prc", 0.);
            }
        }
    }
    let shape = shape(false);
    let linked = [LinkedPreset {
        name: "unchanged",
        preset: &source,
        shape: &shape,
        sampled_tip: None,
    }];
    let token = CancellationToken::default();
    let current = convert(&linked, &token).unwrap();
    let old = paint_io::abr_conversion::convert_v3(&linked, &token).unwrap();
    assert_eq!(current[0].converter_version, 3);
    assert_eq!(current[0].settings.dynamics, old[0].settings.dynamics);
    assert_eq!(
        current[0]
            .issues
            .iter()
            .map(|i| (&i.path, i.status, i.reason))
            .collect::<Vec<_>>(),
        old[0]
            .issues
            .iter()
            .map(|i| (&i.path, i.status, i.reason))
            .collect::<Vec<_>>()
    );
}
