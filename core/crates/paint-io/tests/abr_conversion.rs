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
            dual_sampled_tip: None,
        },
        LinkedPreset {
            name: "同名",
            preset: &preset,
            shape: &image_shape,
            sampled_tip: Some(&tip),
            dual_sampled_tip: None,
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
        dual_sampled_tip: None,
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
                dual_sampled_tip: None,
            },
            LinkedPreset {
                name: "非法",
                preset: &source,
                shape: &bad,
                sampled_tip: None,
                dual_sampled_tip: None,
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
            sampled_tip: None,
            dual_sampled_tip: None,
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
            sampled_tip: None,
            dual_sampled_tip: None,
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
            sampled_tip: None,
            dual_sampled_tip: None,
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
            dual_sampled_tip: None,
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
                sampled_tip: None,
                dual_sampled_tip: None,
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
                dual_sampled_tip: None,
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
            dual_sampled_tip: None,
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
fn set_control(source: &mut Descriptor, key: &str, control: i32, minimum: f64) {
    let value = source
        .entries
        .iter_mut()
        .find(|(name, _)| name == key.as_bytes())
        .map(|(_, value)| value)
        .unwrap();
    let Value::Object { descriptor, .. } = value else {
        panic!("dynamic object")
    };
    for (name, value) in &mut descriptor.entries {
        if name == b"bVTy" {
            *value = Value::Integer(control);
        } else if name == b"Mnm " {
            *value = unit(*b"#Prc", minimum);
        }
    }
}
fn set_steps(source: &mut Descriptor, key: &str, steps: i32) {
    let value = source
        .entries
        .iter_mut()
        .find(|(name, _)| name == key.as_bytes())
        .map(|(_, value)| value)
        .unwrap();
    let Value::Object { descriptor, .. } = value else {
        panic!("dynamic object")
    };
    for (name, value) in &mut descriptor.entries {
        if name == b"fStp" {
            *value = Value::Integer(steps);
        }
    }
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
        dual_sampled_tip: None,
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
fn device_controls_route_size_flow_and_opacity_without_pressure_flags() {
    let mut source = pressure_preset();
    set_control(&mut source, "szVr", 3, 20.);
    set_control(&mut source, "prVr", 8, 40.);
    set_control(&mut source, "opVr", 6, 30.);
    let shape = shape(false);
    let p = convert(
        &[LinkedPreset {
            name: "device",
            preset: &source,
            shape: &shape,
            sampled_tip: None,
            dual_sampled_tip: None,
        }],
        &CancellationToken::default(),
    )
    .unwrap();
    let settings = &p[0].settings;
    assert_eq!(p[0].converter_version, 9);
    assert!(!settings.tip.pressure_size);
    assert!(!settings.tip.pressure_flow);
    assert!(!settings.dynamics.pressure_opacity);
    assert_eq!(
        settings.dynamics.matrix,
        paint_core::DynamicsSourceMatrix::new(
            paint_core::DynamicsSource::Tilt,
            paint_core::DynamicsSource::Rotation,
            paint_core::DynamicsSource::Direction,
        )
    );
    assert!(settings.dynamics.tilt.enabled());
    assert!(settings.dynamics.rotation.enabled());
    assert!(settings.dynamics.direction.enabled());
    assert_eq!(settings.dynamics.tilt.size().minimum(), 0.2);
    assert_eq!(settings.dynamics.rotation.flow().minimum(), 0.4);
    assert_eq!(settings.dynamics.direction.opacity().minimum(), 0.3);
    for path in ["szVr.bVTy", "prVr.bVTy", "opVr.bVTy"] {
        assert!(p[0]
            .issues
            .iter()
            .any(|issue| issue.path == path && issue.status == Status::Mapped));
    }
}
#[test]
fn abr_fade_control_uses_stamp_steps_and_keeps_other_channels_flat() {
    let mut source = pressure_preset();
    source.entries.retain(|(key, _)| key != b"minimumDiameter");
    for key in [b"prVr".as_slice(), b"opVr"] {
        source.entries.retain(|(name, _)| name.as_slice() != key);
    }
    set_control(&mut source, "szVr", 1, 0.);
    set_steps(&mut source, "szVr", 4);
    let shape = shape(false);
    let p = convert(
        &[LinkedPreset {
            name: "fade",
            preset: &source,
            shape: &shape,
            sampled_tip: None,
            dual_sampled_tip: None,
        }],
        &CancellationToken::default(),
    )
    .unwrap();
    assert_eq!(p[0].converter_version, 10);
    assert!(p[0].settings.dynamics.fade.enabled());
    assert_eq!(p[0].settings.dynamics.fade.steps(), 4);
    assert_eq!(p[0].settings.dynamics.fade.size().minimum(), 0.);
    assert!(!p[0].settings.tip.pressure_size);
    assert!(!p[0].settings.tip.pressure_flow);
    assert!(!p[0].settings.dynamics.pressure_opacity);
    assert!(p[0]
        .issues
        .iter()
        .any(|issue| issue.path == "szVr.fStp" && issue.status == Status::Mapped));
}
#[test]
fn abr_computed_dual_brush_maps_shape_mode_and_spacing() {
    let mut source = preset();
    source.entries.push((
        "dualBrush".into(),
        Value::Object {
            global: false,
            descriptor: Box::new(object(
                "dualBrush",
                vec![
                    ("useDualBrush", Value::Boolean(true)),
                    (
                        "Brsh",
                        Value::Object {
                            global: false,
                            descriptor: Box::new(object(
                                "computedBrush",
                                vec![
                                    ("Dmtr", unit(*b"#Pxl", 8.)),
                                    ("Angl", unit(*b"#Ang", 0.)),
                                    ("Rndn", unit(*b"#Prc", 100.)),
                                    ("Hrdn", unit(*b"#Prc", 50.)),
                                    ("Spcn", unit(*b"#Prc", 2.)),
                                    ("flipX", Value::Boolean(true)),
                                    ("flipY", Value::Boolean(true)),
                                ],
                            )),
                        },
                    ),
                    (
                        "BlnM",
                        Value::Enumerated {
                            type_id: b"BlnM".to_vec(),
                            value: b"Mltp".to_vec(),
                        },
                    ),
                    ("Spcn", unit(*b"#Prc", 25.)),
                    ("Flip", Value::Boolean(true)),
                    ("useScatter", Value::Boolean(true)),
                    ("bothAxes", Value::Boolean(true)),
                    ("Cnt ", Value::Integer(3)),
                    (
                        "scatterDynamics",
                        Value::Object {
                            global: false,
                            descriptor: Box::new(object(
                                "brushDynamics",
                                vec![
                                    ("bVTy", Value::Integer(2)),
                                    ("jitter", unit(*b"#Prc", 50.)),
                                    ("Mnm ", unit(*b"#Prc", 0.)),
                                ],
                            )),
                        },
                    ),
                    (
                        "countDynamics",
                        Value::Object {
                            global: false,
                            descriptor: Box::new(object(
                                "brushDynamics",
                                vec![
                                    ("bVTy", Value::Integer(2)),
                                    ("jitter", unit(*b"#Prc", 25.)),
                                    ("Mnm ", unit(*b"#Prc", 0.)),
                                ],
                            )),
                        },
                    ),
                ],
            )),
        },
    ));
    let shape = shape(false);
    let p = paint_io::abr_conversion::convert_v9(
        &[LinkedPreset {
            name: "dual",
            preset: &source,
            shape: &shape,
            sampled_tip: None,
            dual_sampled_tip: None,
        }],
        &std::collections::BTreeMap::new(),
        &std::collections::BTreeMap::new(),
        &CancellationToken::default(),
    )
    .unwrap();
    assert_eq!(p[0].converter_version, 12);
    assert!(p[0].settings.dual.enabled());
    assert_eq!(p[0].settings.dual.mode(), paint_core::DualMode::Multiply);
    assert_eq!(p[0].settings.dual.secondary().radius(), 4.);
    assert_eq!(p[0].settings.dual.secondary().spacing(), Some(0.25));
    assert_eq!(p[0].settings.dual.secondary().tip().hardness, 0.5);
    assert!(p[0].settings.dual.secondary().flip());
    assert_eq!(p[0].settings.dual.secondary().shape_flip(), (true, true));
    assert!(p[0].settings.dual.secondary().dynamics().scatter.enabled());
    assert_eq!(p[0].settings.dual.secondary().dynamics().scatter.count(), 3);
    assert_eq!(
        p[0].settings.dual.secondary().dynamics().scatter.amount(),
        0.5
    );
    assert_eq!(
        p[0].settings
            .dual
            .secondary()
            .dynamics()
            .scatter
            .amount_control()
            .source(),
        paint_core::ScatterControlSource::Pressure
    );
    assert_eq!(
        p[0].settings
            .dual
            .secondary()
            .dynamics()
            .scatter
            .count_control()
            .source(),
        paint_core::ScatterControlSource::Pressure
    );
}
#[test]
fn abr_sampled_dual_brush_binds_secondary_mask() {
    let mut source = preset();
    source.entries.push((
        "dualBrush".into(),
        Value::Object {
            global: false,
            descriptor: Box::new(object(
                "dualBrush",
                vec![
                    ("useDualBrush", Value::Boolean(true)),
                    (
                        "Brsh",
                        Value::Object {
                            global: false,
                            descriptor: Box::new(object(
                                "sampledBrush",
                                vec![
                                    ("Dmtr", unit(*b"#Pxl", 8.)),
                                    ("Angl", unit(*b"#Ang", 0.)),
                                    ("Rndn", unit(*b"#Prc", 100.)),
                                    ("Spcn", unit(*b"#Prc", 2.)),
                                    ("sampledData", Value::Text("dual-sample\0".into())),
                                ],
                            )),
                        },
                    ),
                    (
                        "BlnM",
                        Value::Enumerated {
                            type_id: b"BlnM".to_vec(),
                            value: b"Mltp".to_vec(),
                        },
                    ),
                    ("Spcn", unit(*b"#Prc", 25.)),
                ],
            )),
        },
    ));
    let tip = SectionedTip {
        id: "dual-sample".into(),
        prefix: vec![].into(),
        bounds: [0, 0, 4, 4],
        mask: Arc::new(paint_core::BrushMask::new(4, 4, vec![255; 16]).unwrap()),
        trailing_metadata: vec![].into(),
    };
    let shape = shape(false);
    let p = paint_io::abr_conversion::convert_v9(
        &[LinkedPreset {
            name: "sampled-dual",
            preset: &source,
            shape: &shape,
            sampled_tip: None,
            dual_sampled_tip: Some(&tip),
        }],
        &std::collections::BTreeMap::new(),
        &std::collections::BTreeMap::new(),
        &CancellationToken::default(),
    )
    .unwrap();
    assert!(p[0].settings.dual.secondary().mask().is_some());
    assert_eq!(
        p[0].settings.dual.secondary().mask().unwrap().dimensions(),
        (4, 4)
    );
}
#[test]
fn abr_tilt_size_source_changes_real_brush_radius() {
    use paint_core::{Document, InputPoint};
    let mut source = pressure_preset();
    source.entries.retain(|(key, _)| key != b"minimumDiameter");
    for key in [b"prVr".as_slice(), b"opVr"] {
        source.entries.retain(|(name, _)| name.as_slice() != key);
    }
    set_control(&mut source, "szVr", 3, 0.);
    let shape = shape(false);
    let p = convert(
        &[LinkedPreset {
            name: "tilt-size",
            preset: &source,
            shape: &shape,
            sampled_tip: None,
            dual_sampled_tip: None,
        }],
        &CancellationToken::default(),
    )
    .unwrap();
    let draw = |tilt: f32| {
        let mut d = Document::new(64, 64).unwrap();
        let mut start = InputPoint::new(16.5, 32.5, 1.);
        start.capabilities = 2;
        start.tilt_x = tilt;
        d.begin_brush_stroke(p[0].settings.definition(), start)
            .unwrap();
        let mut end = InputPoint::new(48.5, 32.5, 1.);
        end.capabilities = 2;
        end.tilt_x = tilt;
        d.stroke_to(end).unwrap();
        d.end_stroke().unwrap();
        d
    };
    assert_eq!(draw(0.).pixel(32, 38).alpha(), 0.);
    assert!(draw(90.).pixel(32, 38).alpha() > 0.5);
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
        let converted = paint_io::abr_conversion::convert_v5(
            &[LinkedPreset {
                name: "partial",
                preset: &source,
                shape: &shape,
                sampled_tip: None,
                dual_sampled_tip: None,
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
                sampled_tip: None,
                dual_sampled_tip: None,
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
                dual_sampled_tip: None,
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
                    dual_sampled_tip: None,
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
                sampled_tip: None,
                dual_sampled_tip: None,
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
                sampled_tip: None,
                dual_sampled_tip: None,
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
            dual_sampled_tip: None,
        }],
        &token,
    )
    .unwrap();
    assert!(converted[0]
        .issues
        .iter()
        .any(|i| i.path == "countDynamics.bVTy" && i.status == Status::Mapped));
    assert!(converted[0].settings.dynamics.scatter.enabled());
    assert_eq!(
        converted[0]
            .settings
            .dynamics
            .scatter
            .count_control()
            .source(),
        paint_core::ScatterControlSource::Pressure
    );
    assert_eq!(converted[0].converter_version, 12);
}

#[test]
fn scatter_fade_controls_map_to_native_and_unsupported_controls_stay_reported() {
    let token = CancellationToken::default();
    let shape = shape(false);
    let mut source = scatter_preset(true, true);
    if let Value::Object { descriptor, .. } = &mut source
        .entries
        .iter_mut()
        .find(|(k, _)| k == b"scatterDynamics")
        .unwrap()
        .1
    {
        descriptor.entries[0].1 = Value::Integer(1);
        descriptor.entries[1].1 = Value::Integer(4);
        descriptor.entries[2].1 = unit(*b"#Prc", 20.);
    }
    let converted = convert(
        &[LinkedPreset {
            name: "fade",
            preset: &source,
            shape: &shape,
            sampled_tip: None,
            dual_sampled_tip: None,
        }],
        &token,
    )
    .unwrap();
    let control = converted[0].settings.dynamics.scatter.amount_control();
    assert_eq!(control.source(), paint_core::ScatterControlSource::Fade);
    assert_eq!(control.minimum(), 0.2);
    assert_eq!(control.fade_steps(), 4);
    assert!(converted[0]
        .issues
        .iter()
        .any(|i| i.path == "scatterDynamics.fStp" && i.status == Status::Mapped));
    assert_eq!(converted[0].converter_version, 12);

    let mut unsupported = scatter_preset(true, true);
    if let Value::Object { descriptor, .. } = &mut unsupported
        .entries
        .iter_mut()
        .find(|(k, _)| k == b"scatterDynamics")
        .unwrap()
        .1
    {
        descriptor.entries[0].1 = Value::Integer(99);
    }
    let converted = convert(
        &[LinkedPreset {
            name: "unsupported",
            preset: &unsupported,
            shape: &shape,
            sampled_tip: None,
            dual_sampled_tip: None,
        }],
        &token,
    )
    .unwrap();
    assert!(!converted[0].settings.dynamics.scatter.has_controls());
    assert!(converted[0]
        .issues
        .iter()
        .any(|i| i.path == "scatterDynamics.bVTy" && i.status == Status::NotExecuted));
    assert_eq!(converted[0].converter_version, 4);
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
        dual_sampled_tip: None,
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

fn color_preset(enabled: bool, per_tip: bool) -> Descriptor {
    let mut source = scatter_preset(true, true);
    source.entries.extend(
        object(
            "unused",
            vec![
                ("useColorDynamics", Value::Boolean(enabled)),
                ("colorDynamicsPerTip", Value::Boolean(per_tip)),
                (
                    "clVr",
                    Value::Object {
                        global: false,
                        descriptor: Box::new(object(
                            "brushDynamics",
                            vec![
                                ("bVTy", Value::Integer(0)),
                                ("Mnm ", unit(*b"#Prc", 0.)),
                                ("fStp", Value::Integer(25)),
                                ("jitter", unit(*b"#Prc", 80.)),
                            ],
                        )),
                    },
                ),
                ("H   ", unit(*b"#Prc", 20.)),
                ("Strt", unit(*b"#Prc", 30.)),
                ("Brgh", unit(*b"#Prc", 10.)),
                ("purity", unit(*b"#Prc", 50.)),
            ],
        )
        .entries,
    );
    source
}

#[test]
fn color_conversion_program_image_frequency_and_closed_state_use_real_history() {
    use paint_core::{BrushMask, ColorDynamics, Document, InputPoint, Pixel};
    let token = CancellationToken::default();
    let mask = SectionedTip {
        id: "color".into(),
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
        for per_tip in [false, true] {
            for sampled in [false, true] {
                let source = color_preset(enabled, per_tip);
                let shape = shape(sampled);
                let linked = [LinkedPreset {
                    name: "color",
                    preset: &source,
                    shape: &shape,
                    sampled_tip: sampled.then_some(&mask),
                    dual_sampled_tip: None,
                }];
                let result = convert(&linked, &token).unwrap();
                let p = &result[0];
                assert_eq!(p.converter_version, 5);
                let expected = ColorDynamics::new(enabled, 0.8, 0.2, 0.3, 0.1, per_tip, 0).unwrap();
                assert_eq!(p.settings.dynamics.color, expected);
                assert!(p
                    .issues
                    .iter()
                    .any(|i| i.path == "purity" && i.status == Status::NotExecuted));
                for old in [
                    paint_io::abr_conversion::convert_legacy(&linked, &token).unwrap(),
                    paint_io::abr_conversion::convert_v2(&linked, &token).unwrap(),
                    paint_io::abr_conversion::convert_v3(&linked, &token).unwrap(),
                    paint_io::abr_conversion::convert_v4(&linked, &token).unwrap(),
                ] {
                    assert_eq!(old[0].settings.dynamics.color, Default::default());
                    assert!(old[0]
                        .issues
                        .iter()
                        .any(|i| i.path == "clVr.jitter" && i.status == Status::NotExecuted));
                }
                let mut reference = p.settings.clone();
                reference.dynamics.color = expected;
                let mut a = Document::new(64, 64).unwrap();
                let mut b = Document::new(64, 64).unwrap();
                for pressure in [0.25, 0.75] {
                    for (doc, s) in [(&mut a, &p.settings), (&mut b, &reference)] {
                        let mut def = s.definition();
                        def.brush.color = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
                        def.brush.background_color =
                            Pixel::from_straight([0., 0., 1., 1.]).unwrap();
                        doc.begin_brush_stroke(def, InputPoint::new(20.5, 32.5, pressure))
                            .unwrap();
                        doc.stroke_to(InputPoint::new(44.5, 32.5, pressure))
                            .unwrap();
                        doc.end_stroke().unwrap();
                    }
                }
                let pixels = |d: &Document| {
                    (0..4096)
                        .map(|i| d.pixel(i % 64, i / 64))
                        .collect::<Vec<_>>()
                };
                let after = pixels(&a);
                assert_eq!(after, pixels(&b));
                assert!(after.iter().any(|p| p.alpha() > 0.));
                if enabled {
                    assert!(after
                        .iter()
                        .any(|p| p.components()[2] > 0. && p.alpha() > 0.));
                }
                assert_eq!(a.history_depth(), (2, 0));
                a.undo().unwrap();
                a.redo().unwrap();
                assert_eq!(pixels(&a), after);
            }
        }
    }
}

#[test]
fn color_conversion_rejects_invalid_units_ranges_and_preserves_unsupported_controls() {
    let shape = shape(false);
    let token = CancellationToken::default();
    for bad in [
        unit(*b"#Prc", -1e-10),
        unit(*b"#Prc", 100. + 1e-9),
        unit(*b"#Prc", f64::NAN),
        unit(*b"#Pxl", 20.),
        Value::Integer(20),
    ] {
        let mut source = color_preset(true, true);
        source
            .entries
            .iter_mut()
            .find(|(k, _)| k == b"H   ")
            .unwrap()
            .1 = bad;
        assert!(convert(
            &[LinkedPreset {
                name: "bad",
                preset: &source,
                shape: &shape,
                sampled_tip: None,
                dual_sampled_tip: None,
            }],
            &token
        )
        .is_err());
    }
    let mut source = color_preset(true, true);
    if let Value::Object { descriptor, .. } = &mut source
        .entries
        .iter_mut()
        .find(|(k, _)| k == b"clVr")
        .unwrap()
        .1
    {
        descriptor.entries[0].1 = Value::Integer(2);
    }
    source.entries.retain(|(k, _)| k != b"colorDynamicsPerTip");
    source
        .entries
        .push((b"clVr.jitter".to_vec(), unit(*b"#Prc", 90.)));
    let result = convert(
        &[LinkedPreset {
            name: "partial",
            preset: &source,
            shape: &shape,
            sampled_tip: None,
            dual_sampled_tip: None,
        }],
        &token,
    )
    .unwrap();
    assert!(!result[0].settings.dynamics.color.per_tip());
    for path in ["clVr.bVTy", "purity", "clVr.jitter"] {
        assert!(result[0]
            .issues
            .iter()
            .any(|i| i.path == path && i.status == Status::NotExecuted));
    }
    assert!(result[0]
        .issues
        .iter()
        .any(|i| i.path == "colorDynamicsPerTip" && i.status == Status::Approximate));
}

#[test]
fn default_or_incomplete_closed_color_keeps_v4_reports_and_source_contract_is_strict() {
    let shape = shape(false);
    let token = CancellationToken::default();
    for complete in [false, true] {
        let mut source = color_preset(false, true);
        for (k, v) in &mut source.entries {
            if [b"H   ".as_slice(), b"Strt", b"Brgh"].contains(&k.as_slice()) {
                *v = unit(*b"#Prc", 0.);
            }
            if let Value::Object { descriptor, .. } = v {
                if k == b"clVr" {
                    descriptor
                        .entries
                        .iter_mut()
                        .find(|(k, _)| k == b"jitter")
                        .unwrap()
                        .1 = unit(*b"#Prc", 0.);
                }
            }
        }
        if !complete {
            source.entries.retain(|(k, _)| k != b"colorDynamicsPerTip");
        }
        let linked = [LinkedPreset {
            name: "closed",
            preset: &source,
            shape: &shape,
            sampled_tip: None,
            dual_sampled_tip: None,
        }];
        let a = convert(&linked, &token).unwrap();
        let b = paint_io::abr_conversion::convert_v4(&linked, &token).unwrap();
        assert_eq!(a[0].converter_version, b[0].converter_version);
        assert_eq!(a[0].settings.dynamics, b[0].settings.dynamics);
        assert_eq!(
            a[0].issues
                .iter()
                .map(|i| (&i.path, i.status, i.reason))
                .collect::<Vec<_>>(),
            b[0].issues
                .iter()
                .map(|i| (&i.path, i.status, i.reason))
                .collect::<Vec<_>>()
        );
    }
    let info: paint_io::brush_source::SourceInfo = serde_json::from_slice(include_bytes!(
        "../../../../contracts/fixtures/abr-color-source-v5-valid.json"
    ))
    .unwrap();
    info.validate().unwrap();
    for bytes in [
        include_bytes!("../../../../contracts/fixtures/abr-color-source-invalid-version.json")
            .as_slice(),
        include_bytes!("../../../../contracts/fixtures/abr-color-source-invalid-downgrade.json")
            .as_slice(),
    ] {
        let info: paint_io::brush_source::SourceInfo = serde_json::from_slice(bytes).unwrap();
        assert!(info.validate().is_err());
    }
}

fn shape_preset(enabled: bool) -> Descriptor {
    let mut source = color_preset(true, true);
    source
        .entries
        .iter_mut()
        .find(|(k, _)| k == b"useTipDynamics")
        .unwrap()
        .1 = Value::Boolean(enabled);
    if let Value::Object { descriptor, .. } = &mut source
        .entries
        .iter_mut()
        .find(|(k, _)| k == b"szVr")
        .unwrap()
        .1
    {
        descriptor
            .entries
            .iter_mut()
            .find(|(k, _)| k == b"jitter")
            .unwrap()
            .1 = unit(*b"#Prc", 60.);
    }
    source
        .entries
        .push((b"minimumRoundness".to_vec(), unit(*b"#Prc", 25.)));
    for (key, amount) in [("angleDynamics", 40.), ("roundnessDynamics", 50.)] {
        source.entries.push((
            key.as_bytes().to_vec(),
            Value::Object {
                global: false,
                descriptor: Box::new(object(
                    "brushDynamics",
                    vec![
                        ("bVTy", Value::Integer(0)),
                        ("Mnm ", unit(*b"#Prc", 0.)),
                        ("fStp", Value::Integer(25)),
                        ("jitter", unit(*b"#Prc", amount)),
                    ],
                )),
            },
        ));
    }
    if let Value::Object { descriptor, .. } = &mut source
        .entries
        .iter_mut()
        .find(|(k, _)| k == b"toolOptions")
        .unwrap()
        .1
    {
        descriptor
            .entries
            .iter_mut()
            .find(|(k, _)| k == b"Opct")
            .unwrap()
            .1 = Value::Double(25.5);
    }
    source
}

#[test]
fn shape_conversion_with_pressure_scatter_color_and_images_matches_native_two_stroke_history() {
    use paint_core::{BrushMask, Document, InputPoint, Pixel, ShapeJitter};
    let token = CancellationToken::default();
    let mask = SectionedTip {
        id: "shape".into(),
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
        for sampled in [false, true] {
            let source = shape_preset(enabled);
            let mut shape = shape(sampled);
            shape
                .entries
                .iter_mut()
                .find(|(k, _)| k == b"Rndn")
                .unwrap()
                .1 = unit(*b"#Prc", 50.);
            let linked = [LinkedPreset {
                name: "shape",
                preset: &source,
                shape: &shape,
                sampled_tip: sampled.then_some(&mask),
                dual_sampled_tip: None,
            }];
            let converted = convert(&linked, &token).unwrap();
            let p = &converted[0];
            let expected = ShapeJitter::new(enabled, 0.6, 0.2, 0.4, 0.5, 0.5, 0).unwrap();
            assert_eq!(p.converter_version, 6);
            assert_eq!(p.settings.dynamics.shape_jitter, expected);
            assert_eq!(p.settings.tip.pressure_size, enabled);
            assert!(p.issues.iter().any(|i| i.path == "szVr.jitter"
                && i.status
                    == if enabled {
                        Status::Approximate
                    } else {
                        Status::Mapped
                    }));
            assert_eq!(
                p.issues.iter().filter(|i| i.path == "szVr.jitter").count(),
                1
            );
            for old in [
                paint_io::abr_conversion::convert_legacy(&linked, &token).unwrap(),
                paint_io::abr_conversion::convert_v2(&linked, &token).unwrap(),
                paint_io::abr_conversion::convert_v3(&linked, &token).unwrap(),
                paint_io::abr_conversion::convert_v4(&linked, &token).unwrap(),
                paint_io::abr_conversion::convert_v5(&linked, &token).unwrap(),
            ] {
                assert_eq!(old[0].settings.dynamics.shape_jitter, Default::default());
            }
            let mut reference = p.settings.clone();
            reference.dynamics.shape_jitter = expected;
            let mut a = Document::new(64, 64).unwrap();
            let mut b = Document::new(64, 64).unwrap();
            let mut alpha_cap = 0.;
            for pressure in [0.25, 0.75] {
                for (d, s) in [(&mut a, &p.settings), (&mut b, &reference)] {
                    let mut def = s.definition();
                    def.brush.color = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
                    def.brush.background_color = Pixel::from_straight([0., 0., 1., 1.]).unwrap();
                    d.begin_brush_stroke(def, InputPoint::new(16.5, 32.5, pressure))
                        .unwrap();
                    d.stroke_to(InputPoint::new(48.5, 32.5, pressure)).unwrap();
                    d.stroke_to(InputPoint::new(16.5, 32.5, pressure)).unwrap();
                    d.end_stroke().unwrap();
                }
                let cap = p.settings.opacity * (0.3 + 0.7 * pressure);
                alpha_cap = 1. - (1. - alpha_cap) * (1. - cap);
                assert!((0..4096).all(|i| a.pixel(i % 64, i / 64).alpha() <= alpha_cap + 1e-5));
            }
            let pixels = |d: &Document| {
                (0..4096)
                    .map(|i| d.pixel(i % 64, i / 64))
                    .collect::<Vec<_>>()
            };
            let after = pixels(&a);
            assert!(after.iter().any(|p| p.alpha() > 0.));
            assert_eq!(after, pixels(&b));
            assert_eq!(a.history_depth(), (2, 0));
            a.undo().unwrap();
            a.redo().unwrap();
            assert_eq!(pixels(&a), after);
        }
    }
}

#[test]
fn shape_conversion_channel_defaults_unknown_controls_and_tool_overrides_are_explicit() {
    let shape = shape(false);
    let token = CancellationToken::default();
    for target in ["szVr", "angleDynamics", "roundnessDynamics"] {
        let mut source = shape_preset(true);
        source.entries.retain(|(k, _)| {
            ![b"szVr".as_slice(), b"angleDynamics", b"roundnessDynamics"].contains(&k.as_slice())
                || k == target.as_bytes()
        });
        let c = convert(
            &[LinkedPreset {
                name: "channel",
                preset: &source,
                shape: &shape,
                sampled_tip: None,
                dual_sampled_tip: None,
            }],
            &token,
        )
        .unwrap();
        let native = c[0].settings.dynamics.shape_jitter;
        assert_eq!(native.size() > 0., target == "szVr");
        assert_eq!(native.angle() > 0., target == "angleDynamics");
        assert_eq!(native.roundness() > 0., target == "roundnessDynamics");
    }
    let mut source = shape_preset(true);
    if let Value::Object { descriptor, .. } = &mut source
        .entries
        .iter_mut()
        .find(|(k, _)| k == b"angleDynamics")
        .unwrap()
        .1
    {
        descriptor.entries[0].1 = Value::Integer(2);
    }
    if let Value::Object { descriptor, .. } = &mut source
        .entries
        .iter_mut()
        .find(|(k, _)| k == b"toolOptions")
        .unwrap()
        .1
    {
        descriptor
            .entries
            .push((b"usePressureOverridesSize".to_vec(), Value::Boolean(true)));
    }
    source
        .entries
        .push((b"angleDynamics.jitter".to_vec(), Value::Integer(7)));
    let c = convert(
        &[LinkedPreset {
            name: "partial",
            preset: &source,
            shape: &shape,
            sampled_tip: None,
            dual_sampled_tip: None,
        }],
        &token,
    )
    .unwrap();
    assert_eq!(c[0].settings.dynamics.shape_jitter.size(), 0.);
    assert!(c[0].settings.dynamics.shape_jitter.angle() > 0.);
    for key in [
        "szVr.jitter",
        "angleDynamics.bVTy",
        "angleDynamics.Mnm ",
        "angleDynamics.fStp",
        "angleDynamics.jitter",
    ] {
        assert!(
            c[0].issues
                .iter()
                .any(|i| i.path == key && i.status == Status::NotExecuted),
            "{key}"
        );
    }
}

#[test]
fn shape_conversion_bad_values_closed_defaults_and_source_contract_are_guarded() {
    let shape = shape(false);
    let token = CancellationToken::default();
    for bad in [
        unit(*b"#Prc", -1e-10),
        unit(*b"#Prc", 100. + 1e-9),
        unit(*b"#Prc", f64::NAN),
        unit(*b"#Pxl", 20.),
        Value::Integer(20),
    ] {
        let mut source = shape_preset(true);
        source
            .entries
            .iter_mut()
            .find(|(k, _)| k == b"minimumRoundness")
            .unwrap()
            .1 = bad;
        assert!(convert(
            &[LinkedPreset {
                name: "bad",
                preset: &source,
                shape: &shape,
                sampled_tip: None,
                dual_sampled_tip: None,
            }],
            &token
        )
        .is_err());
    }
    for complete in [false, true] {
        let mut source = shape_preset(false);
        source
            .entries
            .iter_mut()
            .find(|(k, _)| k == b"minimumDiameter")
            .unwrap()
            .1 = unit(*b"#Prc", 0.);
        source
            .entries
            .iter_mut()
            .find(|(k, _)| k == b"minimumRoundness")
            .unwrap()
            .1 = unit(*b"#Prc", 1.);
        for (key, value) in &mut source.entries {
            if [b"szVr".as_slice(), b"angleDynamics", b"roundnessDynamics"]
                .contains(&key.as_slice())
            {
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
        if !complete {
            source.entries.retain(|(k, _)| k != b"angleDynamics");
        }
        let linked = [LinkedPreset {
            name: "closed",
            preset: &source,
            shape: &shape,
            sampled_tip: None,
            dual_sampled_tip: None,
        }];
        let a = convert(&linked, &token).unwrap();
        let b = paint_io::abr_conversion::convert_v5(&linked, &token).unwrap();
        assert_eq!(a[0].converter_version, b[0].converter_version);
        assert_eq!(a[0].settings.dynamics, b[0].settings.dynamics);
        assert_eq!(
            a[0].issues
                .iter()
                .map(|i| (&i.path, i.status, i.reason))
                .collect::<Vec<_>>(),
            b[0].issues
                .iter()
                .map(|i| (&i.path, i.status, i.reason))
                .collect::<Vec<_>>()
        );
    }
    let info: paint_io::brush_source::SourceInfo = serde_json::from_slice(include_bytes!(
        "../../../../contracts/fixtures/abr-shape-source-v6-valid.json"
    ))
    .unwrap();
    info.validate().unwrap();
    for bytes in [
        include_bytes!("../../../../contracts/fixtures/abr-shape-source-invalid-version.json")
            .as_slice(),
        include_bytes!("../../../../contracts/fixtures/abr-shape-source-invalid-downgrade.json")
            .as_slice(),
    ] {
        let info: paint_io::brush_source::SourceInfo = serde_json::from_slice(bytes).unwrap();
        assert!(info.validate().is_err());
    }
}

#[test]
fn shape_closed_clamping_and_malformed_dynamic_fields_never_claim_exact_mapping() {
    let token = CancellationToken::default();
    let mut shape = shape(false);
    shape
        .entries
        .iter_mut()
        .find(|(k, _)| k == b"Rndn")
        .unwrap()
        .1 = unit(*b"#Prc", 50.);
    for (minimum, expected) in [(0., 0.01), (100., 1.)] {
        let mut source = shape_preset(false);
        source
            .entries
            .iter_mut()
            .find(|(k, _)| k == b"minimumRoundness")
            .unwrap()
            .1 = unit(*b"#Prc", minimum);
        let c = convert(
            &[LinkedPreset {
                name: "clamp",
                preset: &source,
                shape: &shape,
                sampled_tip: None,
                dual_sampled_tip: None,
            }],
            &token,
        )
        .unwrap();
        assert_eq!(
            c[0].settings.dynamics.shape_jitter.minimum_roundness(),
            expected
        );
        assert!(c[0]
            .issues
            .iter()
            .any(|i| i.path == "minimumRoundness" && i.status == Status::Approximate));
    }
    for (key, bad) in [
        ("jitter", unit(*b"#Prc", 100. + 1e-9)),
        ("jitter", unit(*b"#Ang", 20.)),
        ("Mnm ", unit(*b"#Prc", -1e-9)),
        ("bVTy", Value::Boolean(true)),
        ("fStp", Value::Integer(-1)),
    ] {
        let mut source = shape_preset(true);
        if let Value::Object { descriptor, .. } = &mut source
            .entries
            .iter_mut()
            .find(|(k, _)| k == b"angleDynamics")
            .unwrap()
            .1
        {
            descriptor
                .entries
                .iter_mut()
                .find(|(k, _)| k == key.as_bytes())
                .unwrap()
                .1 = bad;
        }
        assert!(convert(
            &[LinkedPreset {
                name: "bad",
                preset: &source,
                shape: &shape,
                sampled_tip: None,
                dual_sampled_tip: None,
            }],
            &token
        )
        .is_err());
    }
}

fn device_preset(control: i32, tilt: bool, combined: bool) -> Descriptor {
    let mut p = if combined {
        shape_preset(true)
    } else {
        preset()
    };
    p.entries.retain(|(k, _)| {
        ![
            b"angleDynamics".as_slice(),
            b"roundnessDynamics",
            b"minimumRoundness",
        ]
        .contains(&k.as_slice())
    });
    if !combined {
        p.entries
            .push((b"useTipDynamics".to_vec(), Value::Boolean(true)));
    }
    p.entries
        .push((b"minimumRoundness".to_vec(), unit(*b"#Prc", 25.)));
    for (key, control, jitter) in [
        ("angleDynamics", control, 40.),
        ("roundnessDynamics", if tilt { 3 } else { 0 }, 50.),
    ] {
        p.entries.push((
            key.as_bytes().to_vec(),
            Value::Object {
                global: false,
                descriptor: Box::new(object(
                    "brushDynamics",
                    vec![
                        ("bVTy", Value::Integer(control)),
                        ("Mnm ", unit(*b"#Prc", 0.)),
                        ("jitter", unit(*b"#Prc", if combined { jitter } else { 0. })),
                        ("fStp", Value::Integer(25)),
                    ],
                )),
            },
        ));
    }
    p
}
#[test]
fn device_conversion_reuses_native_geometry_with_images_pressure_and_actual_history() {
    use paint_core::{AngleControl, BrushMask, Document, InputPoint, TipDynamics};
    let token = CancellationToken::default();
    let mask = SectionedTip {
        id: "device".into(),
        prefix: Box::new([]),
        bounds: [0, 0, 4, 8],
        mask: Arc::new(
            BrushMask::new(
                8,
                4,
                (0..32)
                    .map(|i| {
                        if [10, 11, 18, 19].contains(&i) {
                            0
                        } else {
                            255
                        }
                    })
                    .collect(),
            )
            .unwrap(),
        ),
        trailing_metadata: Box::new([]),
    };
    for (control, angle) in [
        (6, AngleControl::Direction),
        (8, AngleControl::PenRotation),
        (3, AngleControl::Tilt),
    ] {
        for combined in [false, true] {
            for sampled in [false, true] {
                for caps in [0, 7] {
                    let p = device_preset(control, true, combined);
                    let mut sh = shape(sampled);
                    sh.entries.iter_mut().find(|(k, _)| k == b"Rndn").unwrap().1 =
                        unit(*b"#Prc", 50.);
                    let linked = [LinkedPreset {
                        name: "device",
                        preset: &p,
                        shape: &sh,
                        sampled_tip: sampled.then_some(&mask),
                        dual_sampled_tip: None,
                    }];
                    let c = convert(&linked, &token).unwrap();
                    let old = paint_io::abr_conversion::convert_v6(&linked, &token).unwrap();
                    assert_eq!(c[0].converter_version, 7);
                    assert_ne!(old[0].converter_version, 7);
                    let expected = TipDynamics::new(angle, true, 0.5).unwrap();
                    assert_eq!(c[0].settings.dynamics.tip, expected);
                    assert_eq!(old[0].settings.dynamics.tip, Default::default());
                    assert_eq!(
                        c[0].issues
                            .iter()
                            .filter(|i| i.path == "angleDynamics.bVTy")
                            .count(),
                        1
                    );
                    assert!(
                        c[0].issues
                            .iter()
                            .any(|i| i.path == "angleDynamics.bVTy"
                                && i.status == Status::Approximate)
                    );
                    let mut reference = old[0].settings.clone();
                    reference.dynamics.tip = expected;
                    let mut a = Document::new(64, 64).unwrap();
                    let mut b = Document::new(64, 64).unwrap();
                    let mut fixed = Document::new(64, 64).unwrap();
                    for pressure in [0.25, 0.75] {
                        for (doc, settings) in [
                            (&mut a, &c[0].settings),
                            (&mut b, &reference),
                            (&mut fixed, &old[0].settings),
                        ] {
                            let mut point = InputPoint::new(16.5, 32.5, pressure);
                            point.capabilities = caps;
                            point.tilt_x = 65.;
                            point.rotation = 90.;
                            doc.begin_brush_stroke(settings.definition(), point)
                                .unwrap();
                            point.x = 48.5;
                            point.y = 48.5;
                            point.rotation = 35.;
                            doc.stroke_to(point).unwrap();
                            doc.end_stroke().unwrap();
                        }
                    }
                    let pixels = |d: &Document| {
                        (0..4096)
                            .map(|i| d.pixel(i % 64, i / 64))
                            .collect::<Vec<_>>()
                    };
                    let after = pixels(&a);
                    assert!(after.iter().any(|p| p.alpha() > 0.));
                    assert_eq!(after, pixels(&b));
                    if control == 6 || caps == 7 {
                        assert_ne!(after, pixels(&fixed));
                    } else {
                        assert_eq!(after, pixels(&fixed));
                    }
                    assert_eq!(a.history_depth(), (2, 0));
                    a.undo().unwrap();
                    a.redo().unwrap();
                    assert_eq!(pixels(&a), after);
                }
            }
        }
    }
}
#[test]
fn device_disabled_unknown_pose_and_overrides_preserve_v6_behavior_and_reports() {
    let token = CancellationToken::default();
    let sh = shape(false);
    for case in 0..5 {
        let mut p = device_preset(8, true, true);
        match case {
            0 => {
                p.entries
                    .iter_mut()
                    .find(|(k, _)| k == b"useTipDynamics")
                    .unwrap()
                    .1 = Value::Boolean(false)
            }
            1 => p
                .entries
                .push((b"useBrushPose".to_vec(), Value::Boolean(true))),
            2 => {
                for key in [b"angleDynamics".as_slice(), b"roundnessDynamics"] {
                    if let Value::Object { descriptor, .. } =
                        &mut p.entries.iter_mut().find(|(k, _)| k == key).unwrap().1
                    {
                        descriptor.entries[0].1 = Value::Integer(5);
                    }
                }
            }
            3 => {
                if let Value::Object { descriptor, .. } = &mut p
                    .entries
                    .iter_mut()
                    .find(|(k, _)| k == b"toolOptions")
                    .unwrap()
                    .1
                {
                    for key in [b"angleDynamics".as_slice(), b"roundnessDynamics"] {
                        descriptor.entries.push((key.to_vec(), Value::Integer(8)));
                    }
                }
            }
            _ => {
                p.entries.retain(|(k, _)| k != b"useTipDynamics");
            }
        }
        let linked = [LinkedPreset {
            name: "closed",
            preset: &p,
            shape: &sh,
            sampled_tip: None,
            dual_sampled_tip: None,
        }];
        let a = convert(&linked, &token).unwrap();
        let b = paint_io::abr_conversion::convert_v6(&linked, &token).unwrap();
        assert_eq!(a[0].converter_version, b[0].converter_version);
        assert_eq!(a[0].settings.dynamics, b[0].settings.dynamics);
        assert_eq!(
            a[0].issues
                .iter()
                .map(|i| (&i.path, i.status, i.reason))
                .collect::<Vec<_>>(),
            b[0].issues
                .iter()
                .map(|i| (&i.path, i.status, i.reason))
                .collect::<Vec<_>>()
        );
    }
}
#[test]
fn device_invalid_units_ranges_types_and_source_contract_are_rejected() {
    let token = CancellationToken::default();
    let sh = shape(false);
    for (key, bad) in [
        ("jitter", unit(*b"#Prc", 100. + 1e-9)),
        ("Mnm ", unit(*b"#Prc", -1e-9)),
        ("Mnm ", unit(*b"#Ang", 5.)),
        ("bVTy", Value::Boolean(true)),
        ("fStp", Value::Integer(-1)),
    ] {
        let mut p = device_preset(8, true, false);
        if let Value::Object { descriptor, .. } = &mut p
            .entries
            .iter_mut()
            .find(|(k, _)| k == b"angleDynamics")
            .unwrap()
            .1
        {
            descriptor
                .entries
                .iter_mut()
                .find(|(k, _)| k == key.as_bytes())
                .unwrap()
                .1 = bad;
        }
        assert!(convert(
            &[LinkedPreset {
                name: "bad",
                preset: &p,
                shape: &sh,
                sampled_tip: None,
                dual_sampled_tip: None,
            }],
            &token
        )
        .is_err());
    }
    for (bytes, valid) in [
        (
            include_bytes!("../../../../contracts/fixtures/abr-device-source-v7-valid.json")
                .as_slice(),
            true,
        ),
        (
            include_bytes!("../../../../contracts/fixtures/abr-device-source-invalid-version.json")
                .as_slice(),
            false,
        ),
        (
            include_bytes!(
                "../../../../contracts/fixtures/abr-device-source-invalid-downgrade.json"
            )
            .as_slice(),
            false,
        ),
    ] {
        let info: paint_io::brush_source::SourceInfo = serde_json::from_slice(bytes).unwrap();
        assert_eq!(info.validate().is_ok(), valid);
    }
}

#[test]
fn device_channels_are_independent_and_tilt_minimum_is_bounded_before_execution() {
    use paint_core::{AngleControl, BrushProgram, InputPoint};
    let token = CancellationToken::default();
    let mut sh = shape(false);
    sh.entries.iter_mut().find(|(k, _)| k == b"Rndn").unwrap().1 = unit(*b"#Prc", 50.);
    for (angle_control, round_control) in [(6, 2), (5, 3)] {
        for minimum in [None, Some(0.), Some(25.), Some(100.)] {
            let mut p = device_preset(angle_control, true, false);
            if let Value::Object { descriptor, .. } = &mut p
                .entries
                .iter_mut()
                .find(|(k, _)| k == b"roundnessDynamics")
                .unwrap()
                .1
            {
                descriptor
                    .entries
                    .iter_mut()
                    .find(|(k, _)| k == b"bVTy")
                    .unwrap()
                    .1 = Value::Integer(round_control);
            }
            p.entries.retain(|(k, _)| k != b"minimumRoundness");
            if let Some(value) = minimum {
                p.entries
                    .push((b"minimumRoundness".to_vec(), unit(*b"#Prc", value)));
            }
            let converted = convert(
                &[LinkedPreset {
                    name: "independent",
                    preset: &p,
                    shape: &sh,
                    sampled_tip: None,
                    dual_sampled_tip: None,
                }],
                &token,
            )
            .unwrap();
            let c = &converted[0];
            assert_eq!(c.converter_version, 7);
            assert_eq!(
                c.settings.dynamics.tip.angle(),
                if angle_control == 6 {
                    AngleControl::Direction
                } else {
                    AngleControl::Fixed
                }
            );
            assert_eq!(c.settings.dynamics.tip.tilt_roundness(), round_control == 3);
            let unsupported = if round_control == 2 {
                "roundnessDynamics.bVTy"
            } else {
                "angleDynamics.bVTy"
            };
            assert!(c
                .issues
                .iter()
                .any(|i| i.path == unsupported && i.status == Status::NotExecuted));
            let mut point = InputPoint::new(16., 16., 1.);
            point.capabilities = 2;
            point.tilt_x = 90.;
            let pose = BrushProgram::compile(c.settings.definition())
                .unwrap()
                .cursor_pose(8., point, true, Some((0., 1.)))
                .unwrap();
            let floor = if round_control == 3 {
                (minimum.unwrap_or(0.5) as f32 / 50.).clamp(0.01, 1.)
            } else {
                1.
            };
            assert!((pose.roundness - (0.5 * floor).max(0.01)).abs() < 1e-6);
            assert!((pose.angle - if angle_control == 6 { 90. } else { 0. }).abs() < 1e-5);
        }
    }
}
