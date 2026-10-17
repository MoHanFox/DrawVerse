use paint_core::*;
fn definition() -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 2.,
            spacing: 0.5,
            opacity: 0.25,
            ..Default::default()
        },
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        dynamics: BrushDynamics {
            scatter: ScatterDynamics::new(true, 3., true, 4, 0.5, 123).unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn line(doc: &mut Document, brush: BrushDefinition, split: bool) {
    doc.begin_brush_stroke(brush, InputPoint::new(16.5, 48.5, 1.))
        .unwrap();
    if split {
        doc.stroke_to(InputPoint::new(48.5, 48.5, 1.)).unwrap();
    }
    doc.stroke_to(InputPoint::new(80.5, 48.5, 1.)).unwrap();
    doc.end_stroke().unwrap();
}
fn pixels(doc: &Document) -> Vec<Pixel> {
    (0..96)
        .flat_map(|y| (0..96).map(move |x| doc.pixel(x, y)))
        .collect()
}
#[test]
fn seeded_scatter_is_chunk_invariant_and_history_restores_exact_pixels() {
    let mut whole = Document::new(96, 96).unwrap();
    let mut split = Document::new(96, 96).unwrap();
    line(&mut whole, definition(), false);
    line(&mut split, definition(), true);
    assert_eq!(pixels(&whole), pixels(&split));
    assert!((0..96).any(|x| whole.pixel(x, 38).alpha() > 0.));
    let first = pixels(&whole);
    whole.undo().unwrap();
    assert!(pixels(&whole).iter().all(|p| *p == Pixel::TRANSPARENT));
    whole.redo().unwrap();
    assert_eq!(pixels(&whole), first);
    line(&mut whole, definition(), false);
    let second = pixels(&whole);
    assert_ne!(second, first);
    assert!(first
        .iter()
        .zip(&second)
        .any(|(a, b)| a.alpha() == 0. && b.alpha() > 0.));
    whole.undo().unwrap();
    assert_eq!(pixels(&whole), first);
    whole.redo().unwrap();
    assert_eq!(pixels(&whole), second);
}
#[test]
fn count_accumulates_flow_and_respects_stroke_opacity() {
    let mut brush = definition();
    brush.brush.radius = 4.;
    brush.brush.opacity = 0.2;
    brush.tip.flow = 0.2;
    brush.dynamics.scatter = ScatterDynamics::new(true, 0., false, 4, 0., 1).unwrap();
    let mut doc = Document::new(96, 96).unwrap();
    doc.begin_brush_stroke(brush, InputPoint::new(48.5, 48.5, 1.))
        .unwrap();
    doc.end_stroke().unwrap();
    assert!((doc.pixel(48, 48).alpha() - 0.2 * (1. - 0.8f32.powi(4))).abs() < 1e-6);
}
#[test]
fn disabled_parameters_preserve_old_pixels_and_single_axis_stays_on_normal() {
    let mut normal = Document::new(96, 96).unwrap();
    let mut disabled = Document::new(96, 96).unwrap();
    let mut a = definition();
    a.dynamics.scatter = Default::default();
    let mut b = a.clone();
    b.dynamics.scatter = ScatterDynamics::new(false, 10., true, 16, 1., 9).unwrap();
    line(&mut normal, a, false);
    line(&mut disabled, b, true);
    assert_eq!(pixels(&normal), pixels(&disabled));
    let mut doc = Document::new(96, 96).unwrap();
    let mut brush = definition();
    brush.dynamics.scatter = ScatterDynamics::new(true, 3., false, 8, 0., 7).unwrap();
    doc.begin_brush_stroke(brush, InputPoint::new(48.5, 48.5, 1.))
        .unwrap();
    doc.end_stroke().unwrap();
    assert!((0..96).any(|y| doc.pixel(48, y).alpha() > 0.));
    assert!((0..96)
        .filter(|y| !(46..=50).contains(y))
        .any(|y| doc.pixel(48, y).alpha() > 0.));
    for y in 0..96 {
        for x in 0..96 {
            if !(46..=50).contains(&x) {
                assert_eq!(doc.pixel(x, y), Pixel::TRANSPARENT);
            }
        }
    }
}
#[test]
fn budget_rejects_before_mutation_and_retry_does_not_consume_random_index() {
    let mut doc = Document::new(96, 96).unwrap();
    let mut huge = definition();
    huge.brush.radius = 256.;
    huge.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: true,
        pressure_strength: false,
    };
    huge.dynamics.scatter = ScatterDynamics::new(true, 1., true, 16, 0., 0).unwrap();
    assert!(matches!(
        doc.begin_brush_stroke(huge, InputPoint::new(48.5, 48.5, 1.)),
        Err(Error::ResourceLimit(_))
    ));
    assert!(!doc.stroke_active());
    assert!(pixels(&doc).iter().all(|p| *p == Pixel::TRANSPARENT));
    let mut reference = Document::new(96, 96).unwrap();
    for d in [&mut doc, &mut reference] {
        d.begin_brush_stroke(definition(), InputPoint::new(16.5, 48.5, 1.))
            .unwrap();
    }
    let before = pixels(&doc);
    assert!(matches!(
        doc.stroke_to(InputPoint::new(10000., 48.5, 1.)),
        Err(Error::ResourceLimit(_))
    ));
    assert_eq!(pixels(&doc), before);
    assert!(matches!(
        doc.stroke_to(InputPoint::new(f64::MAX, 48.5, 1.)),
        Err(Error::InvalidArgument(_))
    ));
    assert_eq!(pixels(&doc), before);
    for d in [&mut doc, &mut reference] {
        d.stroke_to(InputPoint::new(24.5, 48.5, 1.)).unwrap();
        d.end_stroke().unwrap();
    }
    assert_eq!(pixels(&doc), pixels(&reference));
}
#[test]
fn selected_images_erase_and_mask_use_scattered_coverage() {
    for mask in [false, true] {
        let mut doc = Document::new(96, 96).unwrap();
        doc.initialize_white_background().unwrap();
        if mask {
            doc.add_mask(1).unwrap();
        }
        doc.set_selection(
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
        let mut brush = definition();
        brush.brush.opacity = 0.5;
        brush.brush.mode = if mask {
            BrushMode::Paint
        } else {
            BrushMode::Erase
        };
        brush.mask = Some(std::sync::Arc::new(
            BrushMask::new(3, 1, vec![255, 0, 255]).unwrap(),
        ));
        line(&mut doc, brush, false);
        assert!((48..96).any(|x| doc.pixel(x, 48).alpha() < 1.));
        for y in 0..96 {
            for x in 0..96 {
                let a = doc.pixel(x, y).alpha();
                assert!(a >= 0.499999);
                if x < 48 {
                    assert_eq!(a, 1.);
                }
            }
        }
        let after = pixels(&doc);
        doc.undo().unwrap();
        assert!(pixels(&doc).iter().all(|p| *p == Pixel::WHITE));
        doc.redo().unwrap();
        assert_eq!(pixels(&doc), after);
    }
}
#[test]
fn scattered_smudge_transports_real_color_and_cancel_restores() {
    let mut doc = Document::new(96, 96).unwrap();
    let mut paint = definition();
    paint.dynamics.scatter = Default::default();
    paint.brush.radius = 10.;
    paint.brush.opacity = 1.;
    paint.brush.color = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
    doc.begin_brush_stroke(paint, InputPoint::new(16.5, 48.5, 1.))
        .unwrap();
    doc.stroke_to(InputPoint::new(28.5, 48.5, 1.)).unwrap();
    doc.end_stroke().unwrap();
    let before = pixels(&doc);
    let mut smudge = definition();
    smudge.brush.radius = 4.;
    smudge.brush.opacity = 1.;
    smudge.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: false,
        pressure_strength: false,
    };
    smudge.dynamics.scatter = ScatterDynamics::new(true, 0.3, true, 3, 0., 5).unwrap();
    doc.begin_brush_stroke(smudge, InputPoint::new(32.5, 48.5, 1.))
        .unwrap();
    doc.stroke_to(InputPoint::new(64.5, 48.5, 1.)).unwrap();
    assert!((44..70).any(|x| doc.pixel(x, 48).components()[0] > 0.));
    doc.cancel_stroke().unwrap();
    assert_eq!(pixels(&doc), before);
}

#[test]
fn jitter_changes_actual_counts_within_bounds_and_invalid_settings_reject() {
    for (amount, count, jitter) in [
        (f32::NAN, 1, 0.),
        (10.1, 1, 0.),
        (1., 0, 0.),
        (1., 17, 0.),
        (1., 1, 1.1),
    ] {
        assert!(ScatterDynamics::new(true, amount, false, count, jitter, 0).is_err());
    }
    let mut doc = Document::new(96, 96).unwrap();
    let mut brush = definition();
    brush.brush.radius = 4.;
    brush.brush.opacity = 1.;
    brush.tip.flow = 0.1;
    brush.dynamics.scatter = ScatterDynamics::new(true, 0., false, 16, 1., 123).unwrap();
    let mut gains = std::collections::BTreeSet::new();
    for _ in 0..6 {
        let before = doc.pixel(48, 48).alpha();
        doc.begin_brush_stroke(brush.clone(), InputPoint::new(48.5, 48.5, 1.))
            .unwrap();
        doc.end_stroke().unwrap();
        let gain = (doc.pixel(48, 48).alpha() - before) / (1. - before);
        assert!((0.0999..=1. - 0.9f32.powi(16) + 0.0001).contains(&gain));
        gains.insert((gain * 10000.).round() as u32);
    }
    assert!(gains.len() > 1);
}

#[test]
fn pressure_controls_change_real_stamp_count_and_off_controls_preserve_pixels() {
    let mut base = definition();
    base.brush.radius = 4.;
    base.brush.opacity = 1.;
    base.tip.flow = 0.1;
    base.dynamics.scatter = ScatterDynamics::new(true, 0., false, 4, 0., 7).unwrap();

    let mut untouched = Document::new(96, 96).unwrap();
    untouched
        .begin_brush_stroke(base.clone(), InputPoint::new(48.5, 48.5, 1.))
        .unwrap();
    untouched.end_stroke().unwrap();

    let mut off_controls = Document::new(96, 96).unwrap();
    let mut off = base.clone();
    off.dynamics.scatter = base
        .dynamics
        .scatter
        .with_controls(ScatterControl::off(), ScatterControl::off())
        .unwrap();
    off_controls
        .begin_brush_stroke(off, InputPoint::new(48.5, 48.5, 1.))
        .unwrap();
    off_controls.end_stroke().unwrap();
    assert_eq!(pixels(&untouched), pixels(&off_controls));

    let mut controlled = base;
    controlled.dynamics.scatter = controlled
        .dynamics
        .scatter
        .with_controls(
            ScatterControl::off(),
            ScatterControl::new(ScatterControlSource::Pressure, 0., 0).unwrap(),
        )
        .unwrap();
    let mut low = Document::new(96, 96).unwrap();
    low.begin_brush_stroke(controlled.clone(), InputPoint::new(48.5, 48.5, 0.))
        .unwrap();
    low.end_stroke().unwrap();
    let mut high = Document::new(96, 96).unwrap();
    high.begin_brush_stroke(controlled, InputPoint::new(48.5, 48.5, 1.))
        .unwrap();
    high.end_stroke().unwrap();
    assert!(high.pixel(48, 48).alpha() > low.pixel(48, 48).alpha() + 0.1);
}
