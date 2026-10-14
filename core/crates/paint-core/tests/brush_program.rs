use paint_core::*;

fn dot(tip: BrushTip, pressure: f32) -> Document {
    let mut d = Document::new(40, 40).unwrap();
    d.begin_brush_stroke(
        BrushDefinition {
            texture: Default::default(),
            dual: Default::default(),
            dynamics: Default::default(),
            mask: None,
            brush: Brush {
                radius: 10.,
                ..Brush::default()
            },
            tip,
        },
        InputPoint::new(20.5, 20.5, pressure),
    )
    .unwrap();
    d.end_stroke().unwrap();
    d
}

#[test]
fn legacy_and_default_definition_are_pixel_identical() {
    let mut legacy = Document::new(40, 40).unwrap();
    legacy
        .begin_stroke(
            Brush {
                radius: 10.,
                ..Brush::default()
            },
            InputPoint::new(20.5, 20.5, 0.6),
        )
        .unwrap();
    legacy.end_stroke().unwrap();
    let advanced = dot(BrushTip::default(), 0.6);
    for y in 0..40 {
        for x in 0..40 {
            assert_eq!(legacy.pixel(x, y), advanced.pixel(x, y));
        }
    }
}

#[test]
fn rotated_ellipse_softness_flow_and_pressure_are_real_pixels() {
    let horizontal = dot(
        BrushTip {
            roundness: 0.2,
            ..BrushTip::default()
        },
        1.,
    );
    let vertical = dot(
        BrushTip {
            roundness: 0.2,
            angle: 90.,
            ..BrushTip::default()
        },
        1.,
    );
    assert!(horizontal.pixel(27, 20).alpha() > 0.9);
    assert_eq!(horizontal.pixel(20, 27).alpha(), 0.);
    assert!(vertical.pixel(20, 27).alpha() > 0.9);
    assert_eq!(vertical.pixel(27, 20).alpha(), 0.);
    let soft = dot(
        BrushTip {
            hardness: 0.,
            flow: 0.4,
            pressure_size: false,
            pressure_flow: false,
            ..BrushTip::default()
        },
        0.1,
    );
    assert!((soft.pixel(20, 20).alpha() - 0.4).abs() < 1e-6);
    assert!(soft.pixel(27, 20).alpha() > 0. && soft.pixel(27, 20).alpha() < 0.2);
    assert!(dot(BrushTip::default(), 0.1).pixel(27, 20).alpha() == 0.);
}

#[test]
fn validation_is_atomic_and_new_tip_history_is_reversible() {
    let mut d = dot(
        BrushTip {
            roundness: 0.3,
            angle: 45.,
            ..BrushTip::default()
        },
        1.,
    );
    let before = d.snapshot();
    let history = d.history_depth();
    for invalid in [
        BrushTip {
            roundness: 0.,
            ..BrushTip::default()
        },
        BrushTip {
            angle: f32::NAN,
            ..BrushTip::default()
        },
    ] {
        assert!(d
            .begin_brush_stroke(
                BrushDefinition {
                    texture: Default::default(),
                    dual: Default::default(),
                    tip: invalid,
                    ..BrushDefinition::default()
                },
                InputPoint::new(10., 10., 1.)
            )
            .is_err());
        assert!(!d.stroke_active());
        assert_eq!(d.history_depth(), history);
    }
    d.undo().unwrap();
    assert_eq!(d.pixel(20, 20).alpha(), 0.);
    d.redo().unwrap();
    for y in 0..40 {
        for x in 0..40 {
            assert_eq!(d.pixel(x, y), before.pixel(x, y));
        }
    }
    d.begin_brush_stroke(
        BrushDefinition {
            texture: Default::default(),
            dual: Default::default(),
            dynamics: Default::default(),
            brush: Brush {
                radius: 10.,
                mode: BrushMode::Erase,
                ..Brush::default()
            },
            mask: None,
            tip: BrushTip {
                roundness: 0.2,
                ..BrushTip::default()
            },
        },
        InputPoint::new(20.5, 20.5, 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    assert_eq!(d.pixel(20, 20).alpha(), 0.);
    d.undo().unwrap();
    assert_eq!(d.pixel(20, 20), before.pixel(20, 20));
}
