use paint_core::*;
use std::sync::Arc;
fn control(points: Vec<(f32, f32)>, minimum: f32) -> PressureControl {
    PressureControl::new(PressureCurve::new(points).unwrap(), minimum).unwrap()
}
fn response(value: f32) -> PressureControl {
    control(vec![(0., value), (1., value)], 0.)
}
fn definition() -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            spacing: 0.02,
            ..Default::default()
        },
        ..Default::default()
    }
}
#[test]
fn node_modes_preserve_legacy_samples_bound_mixed_segments_and_only_change_adjacent_spans() {
    for ys in [[0., 0.2, 0.8, 1.], [0., 0.9, 0.1, 1.], [0.4, 0.4, 0.4, 0.4]] {
        let points: Vec<_> = [0., 0.25, 0.75, 1.].into_iter().zip(ys).collect();
        let smooth =
            PressureControl::new(PressureCurve::new_smooth(points.clone()).unwrap(), 0.).unwrap();
        let linear = control(points.clone(), 0.);
        for bits in 0..16 {
            let modes: Vec<_> = (0..4).map(|i| bits & (1 << i) != 0).collect();
            let mixed = PressureControl::new(
                PressureCurve::new_smooth(points.clone())
                    .unwrap()
                    .with_node_modes(modes)
                    .unwrap(),
                0.,
            )
            .unwrap();
            let samples = mixed.samples(1025).unwrap();
            if bits == 0 {
                assert_eq!(samples, linear.samples(1025).unwrap());
            }
            if bits == 15 {
                assert_eq!(samples, smooth.samples(1025).unwrap());
            }
            for (i, y) in samples.iter().enumerate() {
                let x = i as f32 / 1024.;
                let pair = points.windows(2).find(|p| x <= p[1].0).unwrap();
                assert!(
                    *y >= pair[0].1.min(pair[1].1) - 1e-6 && *y <= pair[0].1.max(pair[1].1) + 1e-6
                );
                if bits == 13 && x >= 0.75 {
                    assert_eq!(*y, smooth.response(x).unwrap());
                }
            }
        }
        assert!(PressureCurve::new_smooth(points)
            .unwrap()
            .with_node_modes(vec![true; 3])
            .is_err());
    }
}
#[test]
fn node_modes_change_real_pressure_pixels_and_preserve_single_history() {
    let curve =
        PressureCurve::new_smooth(vec![(0., 0.), (0.25, 0.1), (0.75, 0.9), (1., 1.)]).unwrap();
    let mut original = definition();
    original.tip.pressure_size = false;
    original.dynamics.flow = PressureControl::new(curve.clone(), 0.).unwrap();
    let mut mixed = original.clone();
    mixed.dynamics.flow = PressureControl::new(
        curve
            .with_node_modes(vec![true, false, true, true])
            .unwrap(),
        0.,
    )
    .unwrap();
    let mut a = Document::new(32, 32).unwrap();
    let mut b = Document::new(32, 32).unwrap();
    dot(&mut a, original, 0.125);
    dot(&mut b, mixed, 0.125);
    assert_ne!(a.pixel(16, 16), b.pixel(16, 16));
    assert_eq!(a.history_depth(), b.history_depth());
    let pixel = b.pixel(16, 16);
    b.undo().unwrap();
    assert_eq!(b.pixel(16, 16).alpha(), 0.);
    b.redo().unwrap();
    assert_eq!(b.pixel(16, 16), pixel);
}
fn dot(document: &mut Document, definition: BrushDefinition, pressure: f32) {
    document
        .begin_brush_stroke(definition, InputPoint::new(16.5, 16.5, pressure))
        .unwrap();
    document.end_stroke().unwrap();
}
#[test]
fn curves_validate_bounds_nonmonotone_segments_and_tiny_changes_without_lut_loss() {
    for points in [
        vec![],
        vec![(0., 0.)],
        vec![(0., 0.); 18],
        vec![(0.1, 0.), (1., 1.)],
        vec![(0., 0.), (0.5, 1.), (0.5, 0.), (1., 1.)],
        vec![(0., 0.), (1., f32::NAN)],
        vec![(0., 0.), (1., 2.)],
    ] {
        assert!(PressureCurve::new(points).is_err());
    }
    for minimum in [-1., 2., f32::NAN] {
        assert!(PressureControl::new(PressureCurve::default(), minimum).is_err());
    }
    let curve = control(vec![(0., 1.), (0.5, 0.), (1., 1.)], 0.25);
    assert_eq!(curve.response(0.).unwrap(), 1.);
    assert_eq!(curve.response(0.5).unwrap(), 0.25);
    assert_eq!(curve.response(0.25).unwrap(), 0.625);
    let narrow = f32::from_bits(0.5f32.to_bits() + 1);
    let curve = control(vec![(0., 0.), (0.5, 0.), (narrow, 1.), (1., 1.)], 0.);
    assert_eq!(curve.response(0.5).unwrap(), 0.);
    assert_eq!(curve.response(narrow).unwrap(), 1.);
    let tiny = f32::from_bits(1);
    let curve = control(vec![(0., 0.), (tiny, 1.), (1., 0.)], 0.);
    assert_eq!(curve.response(tiny).unwrap(), 1.);
    assert!(curve.response(f32::NAN).is_err());
    assert!(curve.response(-1.).is_err());
}
#[test]
fn explicit_linear_controls_preserve_every_old_pixel_and_history() {
    let mut baseline = Document::new(40, 40).unwrap();
    let mut controlled = Document::new(40, 40).unwrap();
    let brush = definition();
    let mut with_curve = brush.clone();
    with_curve.dynamics.size = control(vec![(0., 0.), (0.1, 0.1), (0.7, 0.7), (1., 1.)], 0.);
    with_curve.dynamics.flow = with_curve.dynamics.size.clone();
    assert!(with_curve.dynamics.is_identity());
    for (document, definition) in [(&mut baseline, brush), (&mut controlled, with_curve)] {
        document
            .begin_brush_stroke(definition, InputPoint::new(5.5, 20.5, 0.))
            .unwrap();
        for (x, p) in [(8.5, 0.15), (14.5, 0.45), (23.5, 0.8), (33.5, 1.)] {
            document.stroke_to(InputPoint::new(x, 20.5, p)).unwrap();
        }
        document.end_stroke().unwrap();
    }
    for y in 0..40 {
        for x in 0..40 {
            assert_eq!(baseline.pixel(x, y), controlled.pixel(x, y));
        }
    }
    assert_eq!(baseline.history_depth(), controlled.history_depth());
    controlled.undo().unwrap();
    controlled.redo().unwrap();
    for y in 0..40 {
        for x in 0..40 {
            assert_eq!(baseline.pixel(x, y), controlled.pixel(x, y));
        }
    }
}
#[test]
fn size_flow_minimums_and_switches_change_actual_dabs_independently() {
    let mut brush = definition();
    brush.dynamics.size = control(vec![(0., 0.), (0.5, 0.25), (1., 1.)], 0.);
    brush.dynamics.flow = response(0.8);
    let mut document = Document::new(32, 32).unwrap();
    dot(&mut document, brush.clone(), 0.5);
    assert_eq!(document.pixel(16, 16).alpha(), 0.8);
    assert_eq!(document.pixel(20, 16).alpha(), 0.);
    let mut baseline = Document::new(32, 32).unwrap();
    dot(&mut baseline, definition(), 0.5);
    assert!(baseline.pixel(20, 16).alpha() > 0.);
    brush.dynamics.size = PressureControl::new(PressureCurve::default(), 0.25).unwrap();
    brush.dynamics.flow = PressureControl::new(PressureCurve::default(), 0.3).unwrap();
    let mut minimum = Document::new(32, 32).unwrap();
    dot(&mut minimum, brush.clone(), 0.);
    assert_eq!(minimum.pixel(16, 16).alpha(), 0.3);
    brush.tip.pressure_size = false;
    brush.tip.pressure_flow = false;
    let mut disabled = Document::new(32, 32).unwrap();
    dot(&mut disabled, brush, 0.);
    assert_eq!(disabled.pixel(16, 16).alpha(), 1.);
    assert!(disabled.pixel(22, 16).alpha() > 0.9);
}
#[test]
fn image_erase_mask_and_selection_share_pressure_response_with_atomic_history() {
    let mut brush = definition();
    brush.mask = Some(Arc::new(BrushMask::new(8, 8, vec![255; 64]).unwrap()));
    brush.tip.pressure_size = false;
    brush.dynamics.flow = response(0.8);
    let mut image = Document::new(32, 32).unwrap();
    let selection = Selection::default()
        .apply(
            SelectionShape::geometry(SelectionKind::Rectangle, 16., 0., 16., 32., true),
            SelectionOperation::Replace,
            32,
            32,
        )
        .unwrap();
    image.set_selection(selection).unwrap();
    dot(&mut image, brush.clone(), 0.5);
    assert_eq!(image.pixel(14, 16).alpha(), 0.);
    assert_eq!(image.pixel(18, 16).alpha(), 0.8);
    let mut erased = Document::new(32, 32).unwrap();
    erased.initialize_white_background().unwrap();
    let mut erase = brush.clone();
    erase.brush.mode = BrushMode::Erase;
    dot(&mut erased, erase, 0.5);
    assert!((erased.pixel(16, 16).alpha() - 0.2).abs() < 0.000001);
    erased.undo().unwrap();
    assert_eq!(erased.pixel(16, 16), Pixel::WHITE);
    let mut masked = Document::new(32, 32).unwrap();
    masked.initialize_white_background().unwrap();
    masked.add_mask(1).unwrap();
    dot(&mut masked, brush, 0.5);
    assert!((masked.pixel(16, 16).alpha() - 0.2).abs() < 0.000001);
    masked.undo().unwrap();
    assert_eq!(masked.pixel(16, 16), Pixel::WHITE);
}
#[test]
fn smudge_uses_strength_pressure_switch_and_in_flight_program_is_immutable() {
    let mut brush = definition();
    brush.tip.pressure_size = false;
    brush.tip.pressure_flow = false;
    brush.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: true,
        pressure_strength: true,
    };
    brush.dynamics.flow = response(0.75);
    let mut smudge = Document::new(32, 32).unwrap();
    dot(&mut smudge, brush.clone(), 0.2);
    assert_eq!(smudge.pixel(16, 16).alpha(), 0.75);
    assert_eq!(
        smudge.history_actions().last(),
        Some(&HistoryAction::Smudge)
    );
    brush.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: true,
        pressure_strength: false,
    };
    let mut disabled = Document::new(32, 32).unwrap();
    dot(&mut disabled, brush, 0.2);
    assert_eq!(disabled.pixel(16, 16).alpha(), 1.);
    let mut settings = BrushSettings::default();
    settings.tip.pressure_size = false;
    settings.dynamics.flow = response(0.25);
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", settings.clone()).unwrap();
    let captured = catalog.selected();
    let mut document = Document::new(40, 40).unwrap();
    document
        .begin_brush_stroke(
            captured.settings().definition(),
            InputPoint::new(10.5, 20.5, 0.8),
        )
        .unwrap();
    settings.dynamics.flow = response(1.);
    catalog.edit("round-pressure", settings).unwrap();
    document
        .stroke_to(InputPoint::new(30.5, 20.5, 0.8))
        .unwrap();
    document.end_stroke().unwrap();
    let mut old = Document::new(40, 40).unwrap();
    old.begin_brush_stroke(
        captured.settings().definition(),
        InputPoint::new(10.5, 20.5, 0.8),
    )
    .unwrap();
    old.stroke_to(InputPoint::new(30.5, 20.5, 0.8)).unwrap();
    old.end_stroke().unwrap();
    for y in 0..40 {
        for x in 0..40 {
            assert_eq!(document.pixel(x, y), old.pixel(x, y));
        }
    }
}

#[test]
fn smooth_curve_passes_nodes_is_c1_bounded_and_handles_tiny_intervals() {
    let points = vec![(0., 0.), (0.5, 0.25), (1., 1.)];
    let c = PressureControl::new(PressureCurve::new_smooth(points.clone()).unwrap(), 0.).unwrap();
    assert_eq!(c.response(0.25).unwrap(), 0.078125);
    for (x, y) in points {
        assert_eq!(c.response(x).unwrap(), y);
    }
    let e = 0.0001;
    let left = (c.response(0.5).unwrap() - c.response(0.5 - e).unwrap()) / e;
    let right = (c.response(0.5 + e).unwrap() - c.response(0.5).unwrap()) / e;
    assert!((left - right).abs() < 0.003);
    let nonmonotone = PressureControl::new(
        PressureCurve::new_smooth(vec![(0., 1.), (0.2, 0.1), (0.7, 0.9), (1., 0.)]).unwrap(),
        0.,
    )
    .unwrap();
    for i in 0..1001 {
        let x = i as f32 / 1000.;
        let y = nonmonotone.response(x).unwrap();
        let (lo, hi) = if x <= 0.2 {
            (0.1, 1.)
        } else if x <= 0.7 {
            (0.1, 0.9)
        } else {
            (0., 0.9)
        };
        assert!((lo..=hi).contains(&y));
    }
    let tiny = f32::from_bits(1);
    let narrow = f32::from_bits(0.5f32.to_bits() + 1);
    let c = PressureControl::new(
        PressureCurve::new_smooth(vec![
            (0., 0.),
            (tiny, 1.),
            (0.5, 0.),
            (narrow, 1.),
            (1., 0.),
        ])
        .unwrap(),
        0.,
    )
    .unwrap();
    for (x, y) in c.curve().points() {
        assert_eq!(c.response(*x).unwrap(), *y);
    }
    assert!(c
        .samples(257)
        .unwrap()
        .iter()
        .all(|y| y.is_finite() && (0. ..=1.).contains(y)));
    assert!(c.samples(1026).is_err());
}
#[test]
fn smooth_pressure_changes_real_primary_and_secondary_pixels_and_history() {
    let smooth = PressureControl::new(
        PressureCurve::new_smooth(vec![(0., 0.), (0.5, 0.25), (1., 1.)]).unwrap(),
        0.,
    )
    .unwrap();
    let flat = response(0.078125);
    for role in 0..4 {
        let mut a = definition();
        let mut b = definition();
        a.brush.radius = 16.;
        b.brush.radius = 16.;
        match role {
            0 => {
                a.dynamics.size = smooth.clone();
                b.dynamics.size = flat.clone();
            }
            1 => {
                a.dynamics.flow = smooth.clone();
                b.dynamics.flow = flat.clone();
            }
            2 => {
                a.dynamics.opacity = smooth.clone();
                b.dynamics.opacity = flat.clone();
                a.dynamics.pressure_opacity = true;
                b.dynamics.pressure_opacity = true;
            }
            _ => {
                let make = |c: PressureControl| {
                    DualBrush::new(
                        true,
                        DualMode::Multiply,
                        SecondaryTip::new(
                            16.,
                            BrushTip::default(),
                            None,
                            BrushDynamics {
                                size: c,
                                ..Default::default()
                            },
                        )
                        .unwrap(),
                    )
                };
                a.dual = make(smooth.clone());
                b.dual = make(flat.clone());
            }
        }
        let mut da = Document::new(40, 40).unwrap();
        let mut db = Document::new(40, 40).unwrap();
        for (doc, d) in [(&mut da, a), (&mut db, b)] {
            doc.begin_brush_stroke(d, InputPoint::new(12.5, 20.5, 0.25))
                .unwrap();
            doc.stroke_to(InputPoint::new(28.5, 20.5, 0.25)).unwrap();
            doc.end_stroke().unwrap();
        }
        for y in 0..40 {
            for x in 0..40 {
                assert_eq!(da.pixel(x, y), db.pixel(x, y));
            }
        }
        let before = da.pixel(20, 20);
        da.undo().unwrap();
        assert_eq!(da.pixel(20, 20).alpha(), 0.);
        da.redo().unwrap();
        assert_eq!(da.pixel(20, 20), before);
    }
}
