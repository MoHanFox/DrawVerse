use paint_core::*;

fn pixels(d: &Document) -> Vec<Pixel> {
    (0..64)
        .flat_map(|y| (0..64).map(move |x| d.pixel(x, y)))
        .collect()
}
fn definition(second_radius: f32, path: bool, sizing: DualSizing) -> BrushDefinition {
    let tip = BrushTip {
        pressure_size: false,
        pressure_flow: false,
        ..Default::default()
    };
    let secondary = SecondaryTip::new(second_radius, tip, None, Default::default()).unwrap();
    let secondary = if path {
        secondary.with_spacing(0.01).unwrap()
    } else {
        secondary
    };
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            opacity: 0.4,
            spacing: 0.02,
            ..Default::default()
        },
        tip,
        dual: DualBrush::new(true, DualMode::Multiply, secondary).with_sizing(sizing),
        ..Default::default()
    }
}
fn paint(d: &mut Document, b: BrushDefinition, split: bool, pressure: f32) {
    d.begin_brush_stroke(b, InputPoint::new(16.5, 32.5, pressure))
        .unwrap();
    if split {
        d.stroke_to(InputPoint::new(32.5, 32.5, pressure)).unwrap();
    }
    d.stroke_to(InputPoint::new(48.5, 32.5, pressure)).unwrap();
    d.end_stroke().unwrap();
}
#[test]
fn relative_nominal_size_matches_explicit_geometry_split_and_history() {
    for path in [false, true] {
        for offset in [-511., -8., 0., 8., 511.] {
            let size = DualSizing::new(
                DualSizeMode::Relative,
                offset,
                DualWidthRule::FollowPressure,
            )
            .unwrap();
            let radius = (8. + offset / 2.).clamp(0.5, MAX_BRUSH_RADIUS);
            let mut relative = Document::new(64, 64).unwrap();
            let mut explicit = Document::new(64, 64).unwrap();
            paint(&mut relative, definition(19., path, size), true, 1.);
            paint(
                &mut explicit,
                definition(radius, path, Default::default()),
                false,
                1.,
            );
            let result = pixels(&relative);
            assert_eq!(result, pixels(&explicit), "path={path} offset={offset}");
            assert_eq!(relative.history_depth(), (1, 0));
            relative.undo().unwrap();
            assert!(pixels(&relative).iter().all(|p| *p == Pixel::TRANSPARENT));
            relative.redo().unwrap();
            assert_eq!(pixels(&relative), result);
        }
    }
}
#[test]
fn fixed_width_uses_actual_primary_pressure_and_ignores_second_size_response() {
    for pressure in [0_f32, 0.25, 0.7, 1.] {
        let mut b = definition(
            19.,
            false,
            DualSizing::new(DualSizeMode::Relative, -2., DualWidthRule::FixedWidth).unwrap(),
        );
        b.tip.pressure_size = true;
        let secondary = SecondaryTip::new(
            19.,
            BrushTip {
                pressure_size: true,
                pressure_flow: false,
                ..Default::default()
            },
            None,
            BrushDynamics {
                size: PressureControl::new(PressureCurve::default(), 0.8).unwrap(),
                ..Default::default()
            },
        )
        .unwrap();
        b.dual = DualBrush::new(true, DualMode::Multiply, secondary).with_sizing(b.dual.sizing());
        let mut expected = definition(
            (8. * pressure - 1.).clamp(0.5, MAX_BRUSH_RADIUS),
            false,
            Default::default(),
        );
        expected.tip.pressure_size = true;
        let mut a = Document::new(64, 64).unwrap();
        let mut c = Document::new(64, 64).unwrap();
        paint(&mut a, b, false, pressure);
        paint(&mut c, expected, false, pressure);
        assert_eq!(pixels(&a), pixels(&c), "pressure={pressure}");
    }
}
#[test]
fn sizing_rejects_invalid_offsets_and_retains_inactive_options() {
    for offset in [f32::NAN, f32::INFINITY, -511.01, 511.01] {
        assert!(
            DualSizing::new(DualSizeMode::Relative, offset, DualWidthRule::FixedWidth).is_err()
        );
    }
    let sizing =
        DualSizing::new(DualSizeMode::Independent, 17., DualWidthRule::FixedWidth).unwrap();
    let b = DualBrush::default().with_sizing(sizing);
    assert!(!b.enabled());
    assert!(!b.is_default());
    assert_eq!(b.sizing(), sizing);
    assert_eq!(b.secondary().radius(), 12.);
}

#[test]
fn fixed_independent_path_is_split_invariant_with_primary_size_jitter() {
    for jitter in [false, true] {
        let sizing =
            DualSizing::new(DualSizeMode::Relative, 8., DualWidthRule::FixedWidth).unwrap();
        let mut b = definition(19., true, sizing);
        b.tip.pressure_size = true;
        if jitter {
            b.dynamics.shape_jitter = ShapeJitter::new(true, 0.4, 0.2, 0., 0., 0.01, 11).unwrap();
        }
        let mut a = Document::new(64, 64).unwrap();
        let mut c = Document::new(64, 64).unwrap();
        paint(&mut a, b.clone(), false, 0.7);
        paint(&mut c, b, true, 0.7);
        assert_eq!(pixels(&a), pixels(&c), "jitter={jitter}");
    }
}

#[test]
fn ratio_sizes_match_explicit_geometry_pressure_paths_and_legacy_stays_distinct() {
    for path in [false, true] {
        for ratio in [0.0005, 0.25, 1., 3., 2000.] {
            for fixed in [false, true] {
                for pressure in [0.25, 1.] {
                    let rule = if fixed {
                        DualWidthRule::FixedWidth
                    } else {
                        DualWidthRule::FollowPressure
                    };
                    let sizing =
                        DualSizing::new_ratio(DualSizeMode::Relative, ratio, rule).unwrap();
                    let mut relative = definition(19., path, sizing);
                    relative.tip.pressure_size = true;
                    let radius = (8. * ratio * if fixed { pressure } else { 1. })
                        .clamp(0.5, MAX_BRUSH_RADIUS);
                    let mut explicit = definition(radius, path, Default::default());
                    explicit.tip.pressure_size = true;
                    if path && fixed {
                        let nominal = (8. * ratio).clamp(0.5, MAX_BRUSH_RADIUS);
                        let second = explicit
                            .dual
                            .secondary()
                            .clone()
                            .with_spacing(0.01 * nominal / radius)
                            .unwrap();
                        explicit.dual = DualBrush::new(true, DualMode::Multiply, second);
                    }
                    let pose = BrushProgram::compile(relative.clone())
                        .unwrap()
                        .cursor_poses(8., InputPoint::new(0., 0., pressure), true, None)
                        .unwrap();
                    assert_eq!(pose.secondary.unwrap().radius, radius);
                    let mut a = Document::new(64, 64).unwrap();
                    let mut b = Document::new(64, 64).unwrap();
                    paint(&mut a, relative, true, pressure);
                    paint(&mut b, explicit, false, pressure);
                    assert_eq!(
                        pixels(&a),
                        pixels(&b),
                        "ratio={ratio} fixed={fixed} path={path} pressure={pressure}"
                    );
                }
            }
        }
    }
    for bad in [f32::NAN, 0., -1., 2000.25] {
        assert!(
            DualSizing::new_ratio(DualSizeMode::Relative, bad, DualWidthRule::FollowPressure)
                .is_err()
        );
    }
    assert_ne!(
        DualSizing::new_ratio(DualSizeMode::Relative, 1., Default::default()).unwrap(),
        DualSizing::new(DualSizeMode::Relative, 1., Default::default()).unwrap()
    );
}
