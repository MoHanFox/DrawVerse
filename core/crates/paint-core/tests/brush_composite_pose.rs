use paint_core::*;
use std::sync::Arc;
#[test]
fn pair_preserves_primary_and_independent_device_geometry() {
    let mask = Arc::new(BrushMask::new(2, 2, vec![255, 0, 0, 255]).unwrap());
    for enabled in [false, true] {
        for colored in [false, true] {
            for overflow in [false, true] {
                let secondary = SecondaryTip::new(
                    20.,
                    BrushTip {
                        roundness: 0.5,
                        angle: 10.,
                        ..Default::default()
                    },
                    Some(mask.clone()),
                    BrushDynamics {
                        tip: TipDynamics::new(AngleControl::PenRotation, true, 0.2).unwrap(),
                        ..Default::default()
                    },
                )
                .unwrap();
                let definition = BrushDefinition {
                    brush: Brush {
                        radius: 12.,
                        ..Default::default()
                    },
                    dual: DualBrush::new(enabled, DualMode::Multiply, secondary).with_color(
                        DualColor::new(
                            colored,
                            true,
                            overflow,
                            DualColorOrder::SecondBehind,
                            BlendMode::Normal,
                            BlendMode::Normal,
                        )
                        .unwrap(),
                    ),
                    ..Default::default()
                };
                let program = BrushProgram::compile(definition).unwrap();
                let mut point = InputPoint::new(32.5, 32.5, 0.5);
                point.rotation = 80.;
                point.tilt_x = 60.;
                point.capabilities = 7;
                let pair = program
                    .cursor_poses(8., point, true, Some((0., 1.)))
                    .unwrap();
                let primary = program
                    .cursor_pose(8., point, true, Some((0., 1.)))
                    .unwrap();
                assert_eq!(pair.primary.radius, primary.radius);
                assert_eq!(pair.primary.angle, primary.angle);
                assert_eq!(pair.primary.roundness, primary.roundness);
                assert!(!pair.primary_image);
                assert_eq!(pair.secondary_image, enabled);
                assert_eq!(pair.secondary_overflow, enabled && colored && overflow);
                if enabled {
                    let secondary = pair.secondary.unwrap();
                    assert_eq!(secondary.radius, 10.);
                    assert!((secondary.angle - 90.).abs() < 1e-4);
                    assert!((secondary.roundness - 0.25).abs() < 1e-5);
                    let hover = program.cursor_poses(8., point, false, None).unwrap();
                    assert_eq!(hover.secondary.unwrap().radius, 20.);
                    point.capabilities = 0;
                    let fallback = program
                        .cursor_poses(8., point, true, None)
                        .unwrap()
                        .secondary
                        .unwrap();
                    assert!((fallback.angle - 10.).abs() < 1e-4);
                    assert_eq!(fallback.roundness, 0.5);
                } else {
                    assert!(pair.secondary.is_none());
                }
                assert!(program
                    .cursor_poses(8., point, true, Some((0., 0.)))
                    .is_err());
                assert!(program.cursor_poses(f32::NAN, point, true, None).is_err());
            }
        }
    }
}
#[test]
fn relative_radius_uses_bound_nominal_and_primary_pressure_for_fixed_width() {
    for fixed in [false, true] {
        let program = BrushProgram::compile(BrushDefinition {
            brush: Brush {
                radius: 12.,
                ..Default::default()
            },
            dual: DualBrush::new(true, DualMode::Multiply, SecondaryTip::default()).with_sizing(
                DualSizing::new(
                    DualSizeMode::Relative,
                    8.,
                    if fixed {
                        DualWidthRule::FixedWidth
                    } else {
                        DualWidthRule::FollowPressure
                    },
                )
                .unwrap(),
            ),
            ..Default::default()
        })
        .unwrap();
        let point = InputPoint::new(1., 1., 0.5);
        let pair = program.cursor_poses(8., point, true, None).unwrap();
        assert_eq!(pair.primary.radius, 4.);
        assert_eq!(pair.secondary.unwrap().radius, if fixed { 8. } else { 6. });
        assert_eq!(
            program
                .cursor_poses(8., point, false, None)
                .unwrap()
                .secondary
                .unwrap()
                .radius,
            12.
        );
    }
}

#[test]
fn independent_pressure_curves_and_image_roles_stay_separate() {
    let mask = Arc::new(BrushMask::new(2, 2, vec![255; 4]).unwrap());
    for fixed in [false, true] {
        for primary_enabled in [false, true] {
            for secondary_enabled in [false, true] {
                let main = PressureControl::new(
                    PressureCurve::new_smooth(vec![(0., 0.), (0.5, 0.25), (1., 1.)]).unwrap(),
                    0.,
                )
                .unwrap();
                let child = PressureControl::new(
                    PressureCurve::new_smooth(vec![(0., 0.), (0.5, 0.75), (1., 1.)]).unwrap(),
                    0.,
                )
                .unwrap();
                let program = BrushProgram::compile(BrushDefinition {
                    brush: Brush {
                        radius: 12.,
                        ..Default::default()
                    },
                    mask: Some(mask.clone()),
                    tip: BrushTip {
                        pressure_size: primary_enabled,
                        ..Default::default()
                    },
                    dynamics: BrushDynamics {
                        size: main,
                        ..Default::default()
                    },
                    dual: DualBrush::new(
                        true,
                        DualMode::Multiply,
                        SecondaryTip::new(
                            20.,
                            BrushTip {
                                pressure_size: secondary_enabled,
                                ..Default::default()
                            },
                            None,
                            BrushDynamics {
                                size: child,
                                ..Default::default()
                            },
                        )
                        .unwrap(),
                    )
                    .with_sizing(
                        DualSizing::new(
                            DualSizeMode::Relative,
                            8.,
                            if fixed {
                                DualWidthRule::FixedWidth
                            } else {
                                DualWidthRule::FollowPressure
                            },
                        )
                        .unwrap(),
                    ),
                    ..Default::default()
                })
                .unwrap();
                let value = program
                    .cursor_poses(12., InputPoint::new(1., 1., 0.5), true, None)
                    .unwrap();
                assert!(value.primary_image);
                assert!(!value.secondary_image);
                assert_eq!(value.primary.radius, if primary_enabled { 3. } else { 12. });
                assert_eq!(
                    value.secondary.unwrap().radius,
                    if fixed {
                        value.primary.radius + 4.
                    } else if secondary_enabled {
                        12.
                    } else {
                        16.
                    }
                );
            }
        }
    }
}
