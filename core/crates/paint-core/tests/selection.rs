use paint_core::{
    Brush, BrushMode, Document, InputPoint, LayerAppearance, Selection, SelectionKind,
    SelectionOperation as Op, SelectionShape,
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

#[test]
fn selection_edits_do_not_touch_pixels_and_undo_restores_them() {
    let mut doc = Document::new(64, 64).unwrap();
    let revision = doc.revision();
    let selection = Selection::default()
        .apply(rect(8., 8., 16., 16.), Op::Replace, 64, 64)
        .unwrap();
    doc.set_selection(selection).unwrap();
    assert_eq!(doc.history_depth(), (1, 0));
    // A selection edit publishes a new revision but must not materialise any pixel tile.
    assert_ne!(doc.revision(), revision);
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
