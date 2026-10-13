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
