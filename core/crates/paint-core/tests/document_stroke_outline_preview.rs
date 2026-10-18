use paint_core::{
    Brush, BrushDefinition, BrushTip, Document, InputPoint, InputScheduleBudget, OutlinePlacement,
    OutlineSource, OutlineWidth, Pixel, StrokeOutline, StrokePageResult,
};

fn brush() -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            spacing: 0.5,
            color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
            background_color: Pixel::from_straight([0., 0., 1., 1.]).unwrap(),
            ..Default::default()
        },
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn active_outline_moves_without_ghosts_and_pages_publish_only_complete_overlay() {
    let settings = StrokeOutline::new(
        true,
        OutlinePlacement::Outside,
        OutlineWidth::new(false, 3., 10.).unwrap(),
        OutlineSource::SelfBrush,
        true,
    )
    .unwrap();
    let mut doc = Document::new(96, 64).unwrap();
    doc.begin_outlined_brush_stroke(brush(), InputPoint::new(24., 32., 1.), &settings)
        .unwrap();
    assert_eq!(doc.pixel(33, 32).components()[2], 1.);
    doc.begin_stroke_input(InputPoint::new(72., 32., 1.))
        .unwrap();
    assert!(!doc.stroke_page_ready_for_publication());
    let mut pages = 0;
    loop {
        let result = doc
            .continue_stroke_page(InputScheduleBudget::new(1, 100_000, 1).unwrap())
            .unwrap();
        pages += 1;
        if matches!(result, StrokePageResult::InputComplete) {
            break;
        }
        assert!(!doc.stroke_page_ready_for_publication());
    }
    assert!(pages > 1);
    assert!(doc.stroke_page_ready_for_publication());
    assert_eq!(doc.pixel(33, 32), brush().brush.color);
    assert_eq!(doc.pixel(81, 32).components()[2], 1.);
    let active: Vec<_> = (0..64 * 96)
        .flat_map(|i| doc.pixel(i % 96, i / 96).components().map(f32::to_bits))
        .collect();
    doc.end_stroke().unwrap();
    let final_pixels: Vec<_> = (0..64 * 96)
        .flat_map(|i| doc.pixel(i % 96, i / 96).components().map(f32::to_bits))
        .collect();
    assert_eq!(final_pixels, active);
    assert_eq!(doc.history_depth(), (1, 0));
    doc.undo().unwrap();
    assert!((0..64).all(|y| (0..96).all(|x| doc.pixel(x, y) == Pixel::TRANSPARENT)));
}
