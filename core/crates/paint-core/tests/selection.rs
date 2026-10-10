use paint_core::{
    Brush, BrushMode, Document, InputPoint, LayerAppearance, Selection, SelectionKind,
    SelectionOperation as Op, SelectionShape, MAX_SELECTION_STEPS,
};

fn rect(x: f64, y: f64, width: f64, height: f64) -> SelectionShape {
    SelectionShape::geometry(SelectionKind::Rectangle, x, y, width, height, true)
}

#[test]
fn geometry_combination_empty_unrestricted_inversion_and_encoding() {
    let none = Selection::default();
    assert_eq!(none.coverage(50.5, 50.5), 1.);
    let box_a = none
        .apply(rect(10., 10., 20., 20.), Op::Replace, 100, 100)
        .unwrap();
    assert_eq!(box_a.coverage(10.5, 10.5), 1.);
    assert_eq!(box_a.coverage(30.5, 20.5), 0.);
    let fractional = none
        .apply(rect(0.25, 0., 1., 1.), Op::Replace, 100, 100)
        .unwrap();
    assert_eq!(fractional.coverage(0.5, 0.5), 0.75);
    let union = box_a
        .apply(rect(25., 25., 20., 20.), Op::Add, 100, 100)
        .unwrap();
    assert_eq!(union.coverage(40.5, 40.5), 1.);
    let cut = union
        .apply(rect(15., 15., 20., 20.), Op::Subtract, 100, 100)
        .unwrap();
    assert_eq!(cut.coverage(20.5, 20.5), 0.);
    assert_eq!(cut.coverage(40.5, 40.5), 1.);
    let intersection = union
        .apply(rect(28., 28., 10., 10.), Op::Intersect, 100, 100)
        .unwrap();
    assert_eq!(intersection.coverage(29.5, 29.5), 1.);
    assert_eq!(intersection.coverage(20.5, 20.5), 0.);
    let invert = cut.inverted(100, 100).unwrap();
    assert_eq!(invert.coverage(20.5, 20.5), 1.);
    assert_eq!(invert.coverage(40.5, 40.5), 0.);
    assert_eq!(invert.inverted(100, 100).unwrap(), cut);
    let empty = Selection::all(100, 100)
        .apply(rect(0., 0., 100., 100.), Op::Subtract, 100, 100)
        .unwrap();
    assert!(empty.enabled());
    assert_eq!(empty.coverage(20.5, 20.5), 0.);
    let ellipse = none
        .apply(
            SelectionShape {
                kind: SelectionKind::Ellipse,
                ..rect(10., 10., 20., 10.)
            },
            Op::Replace,
            100,
            100,
        )
        .unwrap();
    assert_eq!(ellipse.coverage(20.5, 15.5), 1.);
    assert_eq!(ellipse.coverage(10.5, 10.5), 0.);
    for selection in [none, union, invert, empty, ellipse] {
        assert_eq!(
            Selection::from_text(&selection.to_text()).unwrap(),
            selection
        );
    }
    for bad in [
        "2|1",
        "1|0;4,0,0,0,0,0,0",
        "1|1;0,1,1,NaN,0,10,10",
        "1|1;0,0,1,0,0,-1,10",
        "1|1;4,0,1,0,0,0,0",
    ] {
        assert!(Selection::from_text(bad).is_err());
    }
}

/// Lasso paths and magic-wand regions are rasterized into masks at creation time; everything
/// downstream (combining, inversion, encoding) then behaves like the geometric shapes.
#[test]
fn polygon_and_mask_selections_rasterize_combine_and_roundtrip() {
    // Lasso: a diamond covering the pixel centres (1.5,1.5)..(2.5,2.5) inside a 4x4 box.
    let diamond = [[2., 0.], [4., 2.], [2., 4.], [0., 2.]];
    let shape = paint_core::polygon_shape(&diamond, 8, 8).unwrap();
    assert_eq!(shape.kind, SelectionKind::Polygon);
    assert_eq!(
        (shape.x, shape.y, shape.width, shape.height),
        (0., 0., 4., 4.)
    );
    let lasso = Selection::default()
        .apply(shape, Op::Replace, 8, 8)
        .unwrap();
    assert_eq!(lasso.coverage(1.5, 1.5), 1.);
    assert_eq!(lasso.coverage(2.5, 2.5), 1.);
    assert_eq!(lasso.coverage(0.5, 0.5), 0.);
    assert_eq!(lasso.coverage(6.5, 6.5), 0.);
    // Rectangles and masks combine with the same operations.
    let combined = lasso.apply(rect(2., 2., 3., 3.), Op::Add, 8, 8).unwrap();
    assert_eq!(combined.coverage(3.5, 3.5), 1.);
    let cut = combined
        .apply(rect(2., 2., 1., 1.), Op::Subtract, 8, 8)
        .unwrap();
    assert_eq!(cut.coverage(1.5, 1.5), 1.);
    assert_eq!(cut.coverage(2.5, 2.5), 0.);
    let inverted = lasso.inverted(8, 8).unwrap();
    assert_eq!(inverted.coverage(1.5, 1.5), 0.);
    assert_eq!(inverted.coverage(5.5, 5.5), 1.);
    // Text v2 keeps the mask; a geometric shape still encodes without one.
    for selection in [lasso.clone(), combined, cut, inverted] {
        let encoded = selection.to_text();
        assert!(encoded.starts_with("2|"));
        assert_eq!(Selection::from_text(&encoded).unwrap(), selection);
    }
    // A magic-wand region is the same kind of payload.
    let mask = paint_core::mask_shape(1, 1, 2, 2, vec![1, 0, 0, 1]).unwrap();
    assert_eq!(mask.kind, SelectionKind::Mask);
    let wand = Selection::default().apply(mask, Op::Replace, 8, 8).unwrap();
    assert_eq!(wand.coverage(1.5, 1.5), 1.);
    assert_eq!(wand.coverage(2.5, 1.5), 0.);
    assert_eq!(Selection::from_text(&wand.to_text()).unwrap(), wand);
}

#[test]
fn polygon_and_mask_input_validation_is_bounded() {
    // Too few or too many points, non-finite values and degenerate paths are rejected.
    for points in [
        vec![[0., 0.], [1., 1.]],
        vec![[0., 0.], [1., 0.], [f64::NAN, 1.]],
        vec![[0., 0.], [1., 0.], [0.5, 0.]],
        vec![[10., 10.], [11., 10.], [10.5, 10.5]],
    ] {
        assert!(paint_core::polygon_shape(&points, 8, 8).is_err());
    }
    assert!(paint_core::polygon_shape(&[[0., 0.], [1., 0.], [0., 1.]], 0, 0).is_err());
    let oversized = vec![[0., 0.]; paint_core::MAX_SELECTION_POINTS + 1];
    assert!(paint_core::polygon_shape(&oversized, 8, 8).is_err());
    // Mask payloads must be consistent and within budget.
    assert!(paint_core::mask_shape(0, 0, 2, 2, vec![1, 0, 0]).is_err());
    assert!(paint_core::mask_shape(0, 0, 2, 2, vec![2, 0, 0, 0]).is_err());
    assert!(paint_core::mask_shape(0, 0, 0, 0, vec![]).is_err());
    // Rasterized shapes require a mask, geometric ones must not carry one.
    let no_mask = SelectionShape {
        kind: SelectionKind::Polygon,
        x: 0.,
        y: 0.,
        width: 2.,
        height: 2.,
        antialias: false,
        mask: None,
    };
    assert!(no_mask.validate().is_err());
    // Corrupted v2 encodings are rejected instead of silently producing an empty selection.
    let encoded = Selection::default()
        .apply(
            paint_core::polygon_shape(&[[0., 0.], [4., 0.], [4., 4.], [0., 4.]], 8, 8).unwrap(),
            Op::Replace,
            8,
            8,
        )
        .unwrap()
        .to_text();
    let truncated = &encoded[..encoded.len() - 3];
    assert!(Selection::from_text(truncated).is_err());
    assert!(Selection::from_text("2|1;0,2,0,0,0,4,4,zz").is_err());
    assert!(Selection::from_text("3|1").is_err());
}

#[test]
fn million_pixel_selection_is_bounded_and_failure_preserves_history() {
    let mut doc = Document::new(1_000_000, 1_000_000).unwrap();
    let mut selection = Selection::all(1_000_000, 1_000_000);
    for i in 1..MAX_SELECTION_STEPS {
        selection = selection
            .apply(
                rect(i as f64, 0., 1., 1.),
                Op::Subtract,
                1_000_000,
                1_000_000,
            )
            .unwrap();
    }
    assert!(selection.to_text().len() < 32768);
    assert_eq!(selection.steps().len(), MAX_SELECTION_STEPS);
    assert!(selection
        .apply(rect(1., 1., 2., 2.), Op::Add, 1_000_000, 1_000_000)
        .is_err());
    doc.set_selection(selection.clone()).unwrap();
    let revision = doc.revision();
    assert_eq!(doc.history_depth(), (1, 0));
    doc.set_selection(selection).unwrap();
    assert_eq!(doc.revision(), revision);
    assert_eq!(
        doc.layers().iter().map(|l| l.tile_count()).sum::<usize>(),
        0
    );
    doc.undo().unwrap();
    assert!(!doc.selection().enabled());
    doc.redo().unwrap();
    assert!(doc.selection().enabled());
}

#[test]
fn strokes_erasure_offsets_and_masks_use_document_selection_and_undo() {
    let mut doc = Document::new(128, 128).unwrap();
    let selection = Selection::default()
        .apply(rect(32., 32., 32., 32.), Op::Replace, 128, 128)
        .unwrap();
    doc.set_selection(selection.clone()).unwrap();
    doc.begin_stroke(Brush::default(), InputPoint::new(0., 48., 1.))
        .unwrap();
    doc.stroke_to(InputPoint::new(127., 48., 1.)).unwrap();
    doc.end_stroke().unwrap();
    assert_eq!(doc.pixel(31, 48).alpha(), 0.);
    assert_eq!(doc.pixel(48, 48).alpha(), 1.);
    assert_eq!(doc.pixel(64, 48).alpha(), 0.);
    let painted = doc.snapshot();
    doc.set_selection(Selection::default()).unwrap();
    assert_eq!(doc.snapshot().pixel(48, 48), painted.pixel(48, 48));
    doc.undo().unwrap();
    assert_eq!(doc.selection(), &selection);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(48, 48).alpha(), 0.);
    doc.redo().unwrap();
    assert_eq!(doc.pixel(48, 48).alpha(), 1.);
    doc.begin_stroke(
        Brush {
            mode: BrushMode::Erase,
            ..Brush::default()
        },
        InputPoint::new(48., 48., 1.),
    )
    .unwrap();
    assert!(doc.set_selection(Selection::default()).is_err());
    doc.end_stroke().unwrap();
    assert_eq!(doc.pixel(48, 48).alpha(), 0.);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(48, 48).alpha(), 1.);
    let mut moved = Document::new(128, 128).unwrap();
    moved
        .set_layer_appearance(
            1,
            LayerAppearance {
                offset_x: 100,
                offset_y: 100,
                ..Default::default()
            },
        )
        .unwrap();
    moved.set_selection(selection.clone()).unwrap();
    moved
        .begin_stroke(Brush::default(), InputPoint::new(48., 48., 1.))
        .unwrap();
    moved.end_stroke().unwrap();
    assert_eq!(moved.pixel(48, 48).alpha(), 1.);
    assert_eq!(moved.pixel(64, 48).alpha(), 0.);
    let mask = doc.add_mask(1).unwrap();
    doc.set_active_layer(mask).unwrap();
    doc.begin_stroke(Brush::default(), InputPoint::new(48., 48., 1.))
        .unwrap();
    doc.end_stroke().unwrap();
    assert_eq!(doc.pixel(48, 48).alpha(), 0.);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(48, 48).alpha(), 1.);
}
