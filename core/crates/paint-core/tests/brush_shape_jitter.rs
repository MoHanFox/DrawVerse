use paint_core::*;
use std::sync::Arc;
fn brush() -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 5.,
            spacing: 0.4,
            opacity: 0.4,
            ..Default::default()
        },
        tip: BrushTip {
            roundness: 0.3,
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        dynamics: BrushDynamics {
            shape_jitter: ShapeJitter::new(true, 0.8, 0.2, 1., 0.7, 0.15, 42).unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn pixels(d: &Document) -> Vec<Pixel> {
    (0..96)
        .flat_map(|y| (0..96).map(move |x| d.pixel(x, y)))
        .collect()
}
fn line(d: &mut Document, b: BrushDefinition, split: bool) {
    d.begin_brush_stroke(b, InputPoint::new(16.5, 48.5, 1.))
        .unwrap();
    if split {
        d.stroke_to(InputPoint::new(48.5, 48.5, 1.)).unwrap();
    }
    d.stroke_to(InputPoint::new(80.5, 48.5, 1.)).unwrap();
    d.end_stroke().unwrap();
}
#[test]
fn geometry_jitter_is_chunk_invariant_and_history_is_exact() {
    let mut a = Document::new(96, 96).unwrap();
    let mut b = Document::new(96, 96).unwrap();
    line(&mut a, brush(), false);
    line(&mut b, brush(), true);
    assert_eq!(pixels(&a), pixels(&b));
    let first = pixels(&a);
    assert!(first.iter().any(|p| p.alpha() > 0.));
    assert!(first.iter().all(|p| p.alpha() <= 0.400001));
    a.undo().unwrap();
    assert!(pixels(&a).iter().all(|p| *p == Pixel::TRANSPARENT));
    a.redo().unwrap();
    assert_eq!(pixels(&a), first);
    line(&mut a, brush(), false);
    let second = pixels(&a);
    assert_ne!(first, second);
    assert!(first
        .iter()
        .zip(&second)
        .any(|(a, b)| a.alpha() == 0. && b.alpha() > 0.));
    a.undo().unwrap();
    assert_eq!(pixels(&a), first);
    a.redo().unwrap();
    assert_eq!(pixels(&a), second);
    let mut c = Document::new(96, 96).unwrap();
    let mut other = brush();
    other.dynamics.shape_jitter = ShapeJitter::new(true, 0.8, 0.2, 1., 0.7, 0.15, 43).unwrap();
    line(&mut c, other, false);
    assert_ne!(pixels(&c), first);
}
#[test]
fn disabled_and_zero_settings_preserve_pixels_and_scatter_lanes() {
    let mut base = brush();
    base.dynamics.shape_jitter = Default::default();
    base.tip.roundness = 1.;
    base.dynamics.scatter = ScatterDynamics::new(true, 2., true, 4, 0.3, 99).unwrap();
    let mut original = Document::new(96, 96).unwrap();
    line(&mut original, base.clone(), false);
    for shape in [
        ShapeJitter::new(false, 1., 0.1, 1., 1., 0.01, u64::MAX).unwrap(),
        ShapeJitter::new(true, 0., 0.8, 0., 0., 0.8, 11).unwrap(),
        ShapeJitter::new(true, 0., 0., 1., 0., 0.01, 123).unwrap(),
    ] {
        let mut d = Document::new(96, 96).unwrap();
        let mut b = base.clone();
        b.dynamics.shape_jitter = shape;
        line(&mut d, b, true);
        assert_eq!(pixels(&d), pixels(&original));
    }
}
#[test]
fn each_shape_channel_changes_actual_coverage_and_pressure_composes() {
    let mut fixed = brush();
    fixed.brush.spacing = 1.;
    fixed.brush.radius = 10.;
    fixed.dynamics.shape_jitter = Default::default();
    let mut base = Document::new(96, 96).unwrap();
    line(&mut base, fixed.clone(), false);
    for (size, angle, roundness) in [(1., 0., 0.), (0., 1., 0.), (0., 0., 1.)] {
        let mut d = Document::new(96, 96).unwrap();
        let mut b = fixed.clone();
        b.dynamics.shape_jitter =
            ShapeJitter::new(true, size, 0.05, angle, roundness, 0.05, 42).unwrap();
        line(&mut d, b, false);
        assert_ne!(pixels(&d), pixels(&base));
    }
    let mut count = Vec::new();
    for pressure in [0.5, 1.] {
        let mut d = Document::new(96, 96).unwrap();
        let mut b = brush();
        b.brush.radius = 12.;
        b.tip.pressure_size = true;
        b.dynamics.tip = TipDynamics::new(AngleControl::PenRotation, true, 0.3).unwrap();
        d.begin_brush_stroke(
            b,
            InputPoint {
                rotation: 60.,
                tilt_x: 45.,
                tilt_y: 20.,
                capabilities: 7,
                ..InputPoint::new(48.5, 48.5, pressure)
            },
        )
        .unwrap();
        d.end_stroke().unwrap();
        count.push(pixels(&d).iter().filter(|p| p.alpha() > 0.).count());
    }
    assert!(count[0] > 0 && count[1] > count[0]);
}
#[test]
fn rotated_image_jitter_retains_holes_and_full_non_square_bounds() {
    let mut rotated_corner = false;
    for seed in 0..12 {
        let mut d = Document::new(96, 96).unwrap();
        let mut b = brush();
        b.brush.radius = 8.;
        b.tip.roundness = 1.;
        let mut bytes = vec![255; 256];
        for y in 6..10 {
            for x in 6..10 {
                bytes[y * 16 + x] = 0;
            }
        }
        b.mask = Some(Arc::new(BrushMask::new(16, 16, bytes).unwrap()));
        b.dynamics.shape_jitter = ShapeJitter::new(true, 0., 0., 1., 0., 0.01, seed).unwrap();
        d.begin_brush_stroke(b, InputPoint::new(48.5, 48.5, 1.))
            .unwrap();
        d.end_stroke().unwrap();
        assert_eq!(d.pixel(48, 48), Pixel::TRANSPARENT);
        for y in 0..96 {
            for x in 0..96 {
                if (x as f64 - 48.5).hypot(y as f64 - 48.5) > 9. && d.pixel(x, y).alpha() > 0.2 {
                    rotated_corner = true;
                }
            }
        }
    }
    assert!(rotated_corner);
    let mut d = Document::new(96, 96).unwrap();
    let mut b = brush();
    b.brush.radius = 10.;
    b.tip.roundness = 1.;
    b.mask = Some(Arc::new(BrushMask::new(16, 4, vec![255; 64]).unwrap()));
    b.dynamics.shape_jitter = ShapeJitter::new(true, 0., 0., 1., 0., 0.01, 42).unwrap();
    d.begin_brush_stroke(b, InputPoint::new(48.5, 48.5, 1.))
        .unwrap();
    d.end_stroke().unwrap();
    assert!(
        (0..96).any(|y| (y as i32 - 48).abs() > 4 && (0..96).any(|x| d.pixel(x, y).alpha() > 0.2))
    );
}
#[test]
fn selected_erase_and_mask_share_jitter_and_opacity_limits() {
    for mask in [false, true] {
        let mut d = Document::new(96, 96).unwrap();
        d.initialize_white_background().unwrap();
        if mask {
            d.add_mask(1).unwrap();
        }
        d.set_selection(
            Selection::default()
                .apply(
                    SelectionShape::geometry(SelectionKind::Rectangle, 48., 0., 48., 96., false),
                    SelectionOperation::Replace,
                    96,
                    96,
                )
                .unwrap(),
        )
        .unwrap();
        let mut b = brush();
        b.brush.mode = if mask {
            BrushMode::Paint
        } else {
            BrushMode::Erase
        };
        b.brush.opacity = 0.5;
        b.mask = Some(Arc::new(BrushMask::new(3, 1, vec![255, 0, 255]).unwrap()));
        line(&mut d, b, true);
        let after = pixels(&d);
        assert!((48..96).any(|x| d.pixel(x, 48).alpha() < 1.));
        for y in 0..96 {
            for x in 0..96 {
                assert!(d.pixel(x, y).alpha() >= 0.499999);
                if x < 48 {
                    assert_eq!(d.pixel(x, y), Pixel::WHITE);
                }
            }
        }
        d.undo().unwrap();
        assert!(pixels(&d).iter().all(|p| *p == Pixel::WHITE));
        d.redo().unwrap();
        assert_eq!(pixels(&d), after);
    }
}
#[test]
fn smudge_shape_changes_real_carrier_pixels_and_cancel_restores() {
    let mut d = Document::new(96, 96).unwrap();
    let mut red = brush();
    red.brush.radius = 12.;
    red.brush.opacity = 1.;
    red.brush.color = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
    red.tip.roundness = 1.;
    red.dynamics.shape_jitter = Default::default();
    d.begin_brush_stroke(red, InputPoint::new(16.5, 48.5, 1.))
        .unwrap();
    d.stroke_to(InputPoint::new(28.5, 48.5, 1.)).unwrap();
    d.end_stroke().unwrap();
    let before = pixels(&d);
    let mut b = brush();
    b.brush.radius = 5.;
    b.brush.opacity = 1.;
    b.tip.roundness = 1.;
    b.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: false,
        pressure_strength: false,
    };
    b.dynamics.shape_jitter = ShapeJitter::new(true, 0.2, 0.8, 0.8, 0.2, 0.8, 8).unwrap();
    d.begin_brush_stroke(b, InputPoint::new(32.5, 48.5, 1.))
        .unwrap();
    d.stroke_to(InputPoint::new(64.5, 48.5, 1.)).unwrap();
    assert!((46..65).any(|x| d.pixel(x, 48).components()[0] > 0.));
    d.cancel_stroke().unwrap();
    assert_eq!(pixels(&d), before);
}
#[test]
fn failed_work_does_not_advance_random_sequence_and_scatter_children_vary() {
    let mut a = Document::new(96, 96).unwrap();
    let mut b = Document::new(96, 96).unwrap();
    let mut active = brush();
    active.dynamics.scatter = ScatterDynamics::new(true, 0.5, true, 4, 0., 4).unwrap();
    for d in [&mut a, &mut b] {
        d.begin_brush_stroke(active.clone(), InputPoint::new(16.5, 48.5, 1.))
            .unwrap();
    }
    let before = pixels(&a);
    assert!(matches!(
        a.stroke_to(InputPoint::new(10000., 48.5, 1.)),
        Err(Error::ResourceLimit(_))
    ));
    assert_eq!(pixels(&a), before);
    for d in [&mut a, &mut b] {
        d.stroke_to(InputPoint::new(24.5, 48.5, 1.)).unwrap();
        d.end_stroke().unwrap();
    }
    assert_eq!(pixels(&a), pixels(&b));
    let mut normal = Document::new(96, 96).unwrap();
    line(&mut normal, brush(), false);
    let mut scattered = Document::new(96, 96).unwrap();
    line(&mut scattered, active, false);
    assert_ne!(pixels(&normal), pixels(&scattered));
}

#[test]
fn collocated_scatter_children_get_distinct_actual_shape_samples() {
    let mut frames = Vec::new();
    for count in [1, 4] {
        let mut d = Document::new(96, 96).unwrap();
        let mut b = brush();
        b.brush.radius = 10.;
        b.dynamics.scatter = ScatterDynamics::new(true, 0., true, count, 0., 11).unwrap();
        d.begin_brush_stroke(b, InputPoint::new(48.5, 48.5, 1.))
            .unwrap();
        d.end_stroke().unwrap();
        frames.push(pixels(&d));
    }
    assert_ne!(frames[0], frames[1]);
    assert!(frames[0]
        .iter()
        .zip(&frames[1])
        .all(|(a, b)| b.alpha() >= a.alpha()));
    assert!(frames[0]
        .iter()
        .zip(&frames[1])
        .any(|(a, b)| a.alpha() == 0. && b.alpha() > 0.));
}

#[test]
fn random_geometry_does_not_jump_the_input_cursor() {
    let mut b = brush();
    b.tip.angle = 20.;
    let point = InputPoint::new(48.5, 48.5, 0.5);
    let active = BrushProgram::compile(b.clone())
        .unwrap()
        .cursor_pose(5., point, true, None)
        .unwrap();
    b.dynamics.shape_jitter = Default::default();
    let fixed = BrushProgram::compile(b)
        .unwrap()
        .cursor_pose(5., point, true, None)
        .unwrap();
    assert_eq!(active.radius, fixed.radius);
    assert_eq!(active.angle, fixed.angle);
    assert_eq!(active.roundness, fixed.roundness);
}
