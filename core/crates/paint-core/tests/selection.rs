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
            SelectionShape::geometry(SelectionKind::Ellipse, 10., 10., 20., 10., true),
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

#[test]
fn polygon_concavity_crossings_antialias_and_shared_feather() {
    let points = vec![
        [0., 0.],
        [20., 0.],
        [20., 8.],
        [8., 8.],
        [8., 20.],
        [0., 20.],
    ];
    let polygon = SelectionShape::polygon(points, true, 0.).unwrap();
    assert_eq!(polygon.coverage(4.5, 15.5), 1.);
    assert_eq!(polygon.coverage(15.5, 15.5), 0.);
    let mut triangle =
        SelectionShape::polygon(vec![[0., 0.], [10., 0.], [0., 10.]], true, 0.).unwrap();
    let edge = triangle.coverage(5.5, 4.5);
    assert!(edge > 0. && edge < 1.);
    triangle.antialias = false;
    assert!(matches!(triangle.coverage(5.5, 4.5), 0. | 1.));
    let crossing =
        SelectionShape::polygon(vec![[0., 0.], [10., 10.], [0., 10.], [10., 0.]], false, 0.)
            .unwrap();
    assert_eq!(crossing.coverage(5., 1.), 1.);
    assert_eq!(crossing.coverage(1., 5.), 0.);
    let mut rectangle = rect(10., 10., 20., 20.);
    rectangle.feather = 4.;
    let feathered = SelectionShape::polygon(
        vec![[10., 10.], [30., 10.], [30., 30.], [10., 30.]],
        true,
        4.,
    )
    .unwrap();
    for x in [5., 7., 9., 10., 11., 13., 15., 30., 33., 35.] {
        assert!((rectangle.coverage(x, 20.) - feathered.coverage(x, 20.)).abs() < 1e-6);
    }
    assert_eq!(feathered.coverage(10., 20.), 0.5);
    assert!(feathered.coverage(9., 20.) > 0. && feathered.coverage(9., 20.) < 0.5);
    let mut ellipse = SelectionShape::geometry(SelectionKind::Ellipse, 10., 10., 20., 10., true);
    ellipse.feather = 2.;
    assert!((ellipse.coverage(20., 10.) - 0.5).abs() < 1e-5);
    assert!(ellipse.coverage(20., 9.) > 0. && ellipse.coverage(20., 9.) < 0.5);
}

#[test]
fn lasso_feather_combination_history_encoding_and_real_stroke() {
    let shape =
        SelectionShape::polygon(vec![[16., 8.], [48., 8.], [48., 56.], [16., 56.]], true, 4.)
            .unwrap();
    let selection = Selection::default()
        .apply(shape, Op::Replace, 64, 64)
        .unwrap();
    for op in [Op::Add, Op::Subtract, Op::Intersect] {
        let combined = selection
            .apply(rect(32., 0., 32., 64.), op, 64, 64)
            .unwrap();
        let a = selection.coverage(32.5, 32.5);
        let b = 1.;
        let expected = match op {
            Op::Add => a.max(b),
            Op::Subtract => (a - b).max(0.),
            _ => a.min(b),
        };
        assert_eq!(combined.coverage(32.5, 32.5), expected);
        assert_eq!(Selection::from_text(&combined.to_text()).unwrap(), combined);
    }
    let mut doc = Document::new(64, 64).unwrap();
    doc.set_selection(selection.clone()).unwrap();
    doc.begin_stroke(
        Brush {
            radius: 32.,
            ..Brush::default()
        },
        InputPoint::new(32., 32., 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    assert_eq!(doc.pixel(8, 32).alpha(), 0.);
    assert!(doc.pixel(15, 32).alpha() > 0. && doc.pixel(15, 32).alpha() < 1.);
    assert_eq!(doc.pixel(32, 32).alpha(), 1.);
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert!(!doc.selection().enabled());
    doc.redo().unwrap();
    assert_eq!(doc.selection(), &selection);
    assert!(Selection::from_text("1|1;0,0,1,0,0,10,10").is_ok());
    assert!(Selection::from_text("2|1;0,1,1,0,0,10,10").is_ok());
    for bad in [
        "3|1;0,2,1,0,0,10,10,NaN,0:0/10:0/0:10",
        "3|1;0,2,1,0,0,10,10,0,0:0/10:0",
        "3|1;0,0,1,0,0,10,10,-1",
    ] {
        assert!(Selection::from_text(bad).is_err());
    }
}

#[test]
fn polygon_rejects_degenerate_invalid_and_unbounded_paths() {
    for points in [
        vec![[0., 0.]; 3],
        vec![[0., 0.], [5., 5.], [10., 10.]],
        vec![[0., 0.], [f64::NAN, 10.], [10., 0.]],
        vec![[0., 0.]; 4097],
    ] {
        assert!(SelectionShape::polygon(points, true, 0.).is_err());
    }
    let points = vec![[0., 0.], [10., 0.], [0., 10.]];
    for feather in [-1., 1001., f64::INFINITY] {
        assert!(SelectionShape::polygon(points.clone(), true, feather).is_err());
    }
    let shape = SelectionShape::polygon(points, true, 0.).unwrap();
    let selection = Selection::default()
        .apply(shape, Op::Replace, 1_000_000, 1_000_000)
        .unwrap();
    assert_eq!(selection.coverage(999999., 999999.), 0.);
    assert!(selection.to_text().len() < 200);
}

#[test]
fn long_polygon_index_matches_unindexed_coverage() {
    let points: Vec<_> = (0..4096)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 4096.;
            [100. + 80. * a.cos(), 100. + 70. * a.sin()]
        })
        .collect();
    for feather in [0., 3.] {
        let indexed = SelectionShape::polygon(points.clone(), true, feather).unwrap();
        // Replacing the public point allocation invalidates the index and exercises the full scan.
        let mut unindexed = indexed.clone();
        unindexed.points = points.clone().into();
        for y in (15..190).step_by(7) {
            for x in (15..190).step_by(7) {
                assert!(
                    (indexed.coverage(x as f64 + 0.5, y as f64 + 0.5)
                        - unindexed.coverage(x as f64 + 0.5, y as f64 + 0.5))
                    .abs()
                        < 1e-6
                );
            }
        }
    }
}
