use paint_core::*;

fn brush(flow: f32, mode: BrushMode) -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            opacity: 0.1,
            spacing: 0.02,
            mode,
            ..Default::default()
        },
        tip: BrushTip {
            hardness: 1.,
            flow,
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn scrub(doc: &mut Document, definition: BrushDefinition) {
    doc.begin_brush_stroke(definition, InputPoint::new(12.5, 16.5, 1.))
        .unwrap();
    for x in [24.5, 12.5, 24.5, 12.5] {
        doc.stroke_to(InputPoint::new(x, 16.5, 1.)).unwrap();
    }
    doc.end_stroke().unwrap();
}
#[test]
fn dense_single_stroke_has_opacity_cap_new_stroke_builds_and_history_is_exact() {
    let mut doc = Document::new(40, 32).unwrap();
    scrub(&mut doc, brush(1., BrushMode::Paint));
    assert!((doc.pixel(16, 16).alpha() - 0.1).abs() < 1e-6);
    for y in 0..32 {
        for x in 0..40 {
            assert!(doc.pixel(x, y).alpha() <= 0.100001);
        }
    }
    let first = doc.pixel(16, 16);
    scrub(&mut doc, brush(1., BrushMode::Paint));
    assert!((doc.pixel(16, 16).alpha() - 0.19).abs() < 1e-6);
    let second = doc.pixel(16, 16);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(16, 16), first);
    doc.redo().unwrap();
    assert_eq!(doc.pixel(16, 16), second);
    doc.begin_brush_stroke(brush(1., BrushMode::Paint), InputPoint::new(16.5, 16.5, 1.))
        .unwrap();
    doc.cancel_stroke().unwrap();
    assert_eq!(doc.pixel(16, 16), second);
}
#[test]
fn flow_builds_within_the_cap_and_erase_uses_original_pixels() {
    let mut doc = Document::new(40, 32).unwrap();
    doc.begin_brush_stroke(
        brush(0.1, BrushMode::Paint),
        InputPoint::new(16.5, 16.5, 1.),
    )
    .unwrap();
    assert!((doc.pixel(16, 16).alpha() - 0.01).abs() < 1e-6);
    doc.stroke_to(InputPoint::new(24.5, 16.5, 1.)).unwrap();
    assert!(doc.pixel(16, 16).alpha() > 0.01 && doc.pixel(16, 16).alpha() <= 0.1);
    doc.end_stroke().unwrap();
    let before = doc.pixel(16, 16).alpha();
    scrub(&mut doc, brush(1., BrushMode::Erase));
    assert!((doc.pixel(16, 16).alpha() - before * 0.9).abs() < 1e-6);
}

#[test]
fn selected_mask_and_paged_coverage_keep_stroke_limit_and_cancel() {
    let mut doc = Document::with_options(
        40,
        32,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    doc.initialize_white_background().unwrap();
    doc.add_mask(1).unwrap();
    doc.set_selection(
        Selection::default()
            .apply(
                SelectionShape::geometry(SelectionKind::Rectangle, 16., 0., 24., 32., true),
                SelectionOperation::Replace,
                40,
                32,
            )
            .unwrap(),
    )
    .unwrap();
    scrub(&mut doc, brush(1., BrushMode::Paint));
    assert_eq!(doc.pixel(14, 16), Pixel::WHITE);
    assert!((doc.pixel(18, 16).alpha() - 0.9).abs() < 1e-6);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(18, 16), Pixel::WHITE);
    doc.redo().unwrap();
    assert!((doc.pixel(18, 16).alpha() - 0.9).abs() < 1e-6);
    let before = doc.pixel(18, 16);
    doc.begin_brush_stroke(brush(1., BrushMode::Paint), InputPoint::new(18.5, 16.5, 1.))
        .unwrap();
    doc.stroke_to(InputPoint::new(24.5, 16.5, 1.)).unwrap();
    doc.cancel_stroke().unwrap();
    assert_eq!(doc.pixel(18, 16), before);
}

#[test]
fn opacity_pressure_is_independent_of_flow_and_never_lightens_existing_marks() {
    let mut definition = brush(1., BrushMode::Paint);
    definition.brush.opacity = 0.4;
    definition.dynamics.pressure_opacity = true;
    let mut doc = Document::new(40, 32).unwrap();
    doc.begin_brush_stroke(definition.clone(), InputPoint::new(16.5, 16.5, 0.25))
        .unwrap();
    assert!((doc.pixel(16, 16).alpha() - 0.1).abs() < 1e-6);
    for x in [20.5, 16.5, 20.5, 16.5] {
        doc.stroke_to(InputPoint::new(x, 16.5, 0.25)).unwrap();
    }
    assert!((doc.pixel(16, 16).alpha() - 0.1).abs() < 1e-6);
    doc.stroke_to(InputPoint::new(20.5, 16.5, 0.75)).unwrap();
    doc.stroke_to(InputPoint::new(16.5, 16.5, 0.75)).unwrap();
    assert!((doc.pixel(16, 16).alpha() - 0.3).abs() < 1e-6);
    doc.stroke_to(InputPoint::new(20.5, 16.5, 0.1)).unwrap();
    assert!((doc.pixel(16, 16).alpha() - 0.3).abs() < 1e-6);
    doc.end_stroke().unwrap();
    let after = doc.pixel(16, 16);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(16, 16), Pixel::TRANSPARENT);
    doc.redo().unwrap();
    assert_eq!(doc.pixel(16, 16), after);
    definition.tip.flow = 0.1;
    let mut low_flow = Document::new(40, 32).unwrap();
    low_flow
        .begin_brush_stroke(definition, InputPoint::new(16.5, 16.5, 0.25))
        .unwrap();
    assert!((low_flow.pixel(16, 16).alpha() - 0.01).abs() < 1e-6);
    low_flow
        .stroke_to(InputPoint::new(20.5, 16.5, 0.25))
        .unwrap();
    assert!(low_flow.pixel(16, 16).alpha() > 0.01 && low_flow.pixel(16, 16).alpha() <= 0.1);
    low_flow.cancel_stroke().unwrap();
    assert_eq!(low_flow.pixel(16, 16), Pixel::TRANSPARENT);
}

#[test]
fn opacity_minimum_and_disabled_control_apply_to_erase_and_masks() {
    for mask in [false, true] {
        let mut doc = Document::new(40, 32).unwrap();
        doc.initialize_white_background().unwrap();
        if mask {
            doc.add_mask(1).unwrap();
        }
        let mut definition = brush(
            1.,
            if mask {
                BrushMode::Paint
            } else {
                BrushMode::Erase
            },
        );
        definition.brush.opacity = 1.;
        definition.dynamics.pressure_opacity = true;
        definition.dynamics.opacity = PressureControl::new(PressureCurve::default(), 0.2).unwrap();
        doc.begin_brush_stroke(definition.clone(), InputPoint::new(16.5, 16.5, 0.))
            .unwrap();
        assert!((doc.pixel(16, 16).alpha() - 0.8).abs() < 1e-6);
        doc.end_stroke().unwrap();
        doc.undo().unwrap();
        definition.dynamics.pressure_opacity = false;
        doc.begin_brush_stroke(definition, InputPoint::new(16.5, 16.5, 0.))
            .unwrap();
        assert_eq!(doc.pixel(16, 16).alpha(), 0.);
        doc.cancel_stroke().unwrap();
        assert_eq!(doc.pixel(16, 16), Pixel::WHITE);
    }
}

#[test]
fn image_holes_and_selection_obey_pressure_opacity() {
    let mut definition = brush(1., BrushMode::Paint);
    definition.brush.opacity = 0.4;
    definition.dynamics.pressure_opacity = true;
    definition.mask = Some(std::sync::Arc::new(
        BrushMask::new(3, 1, vec![255, 0, 255]).unwrap(),
    ));
    let mut doc = Document::new(40, 32).unwrap();
    doc.set_selection(
        Selection::default()
            .apply(
                SelectionShape::geometry(SelectionKind::Rectangle, 16., 0., 24., 32., true),
                SelectionOperation::Replace,
                40,
                32,
            )
            .unwrap(),
    )
    .unwrap();
    doc.begin_brush_stroke(definition, InputPoint::new(16.5, 16.5, 0.5))
        .unwrap();
    assert_eq!(doc.pixel(12, 16), Pixel::TRANSPARENT);
    assert_eq!(doc.pixel(16, 16), Pixel::TRANSPARENT);
    assert!(doc.pixel(22, 16).alpha() > 0. && doc.pixel(22, 16).alpha() <= 0.2);
    doc.end_stroke().unwrap();
    let after = doc.pixel(22, 16);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(22, 16), Pixel::TRANSPARENT);
    doc.redo().unwrap();
    assert_eq!(doc.pixel(22, 16), after);
}

#[test]
fn stationary_pen_pressure_changes_update_opacity_without_airbrush_buildup() {
    let mut definition = brush(1., BrushMode::Paint);
    definition.brush.opacity = 0.4;
    definition.dynamics.pressure_opacity = true;
    let mut doc = Document::new(40, 32).unwrap();
    doc.begin_brush_stroke(definition, InputPoint::new(16.5, 16.5, 0.25))
        .unwrap();
    assert!((doc.pixel(16, 16).alpha() - 0.1).abs() < 1e-6);
    doc.stroke_to(InputPoint::new(16.5, 16.5, 0.75)).unwrap();
    assert!((doc.pixel(16, 16).alpha() - 0.3).abs() < 1e-6);
    let after = doc.pixel(16, 16);
    for p in [0.75, 0.1, 0.1, 0.75] {
        doc.stroke_to(InputPoint::new(16.5, 16.5, p)).unwrap();
        assert_eq!(doc.pixel(16, 16), after);
    }
    doc.end_stroke().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.pixel(16, 16), Pixel::TRANSPARENT);
    doc.redo().unwrap();
    assert_eq!(doc.pixel(16, 16), after);
}
