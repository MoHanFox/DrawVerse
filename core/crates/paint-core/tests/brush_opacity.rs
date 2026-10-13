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
