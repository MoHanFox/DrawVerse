use paint_core::*;
fn fill(x: u32, y: u32, color: Pixel) -> BucketFill {
    BucketFill {
        x,
        y,
        color,
        opacity: 1.,
        tolerance: 0,
        antialias: true,
        contiguous: true,
        all_layers: false,
    }
}
fn color(r: f32, g: f32, b: f32) -> Pixel {
    Pixel::from_straight([r, g, b, 1.]).unwrap()
}
fn split() -> Document {
    let mut doc = Document::new(8, 4).unwrap();
    doc.bucket_fill(fill(0, 0, Pixel::WHITE)).unwrap();
    let brush = Brush {
        radius: 0.5,
        color: color(0., 0., 0.),
        spacing: 0.1,
        ..Brush::default()
    };
    doc.begin_stroke(brush, InputPoint::new(3.5, 0.5, 1.))
        .unwrap();
    doc.stroke_to(InputPoint::new(3.5, 3.5, 1.)).unwrap();
    doc.end_stroke().unwrap();
    doc
}
#[test]
fn contiguous_global_snapshot_and_one_history_step() {
    let mut doc = split();
    let before = doc.snapshot();
    let depth = doc.history_depth().0;
    let red = color(1., 0., 0.);
    assert!(doc.bucket_fill(fill(0, 0, red)).unwrap());
    assert_eq!(doc.pixel(2, 2), red);
    assert_eq!(doc.pixel(4, 2), Pixel::WHITE);
    assert_eq!(doc.history_depth().0, depth + 1);
    assert_eq!(doc.history_actions().last(), Some(&HistoryAction::Bucket));
    doc.undo().unwrap();
    assert_eq!(doc.pixel(0, 0), before.pixel(0, 0));
    doc.redo().unwrap();
    assert_eq!(doc.pixel(0, 0), red);
    doc.undo().unwrap();
    let mut options = fill(0, 0, red);
    options.contiguous = false;
    doc.bucket_fill(options).unwrap();
    assert_eq!(doc.pixel(4, 2), red);
    assert_eq!(doc.pixel(3, 2), color(0., 0., 0.));
    let revision = doc.revision();
    assert!(!doc.bucket_fill(options).unwrap());
    assert_eq!(doc.revision(), revision);
}
#[test]
fn composite_sampling_writes_only_active_layer_with_offset_and_selection() {
    let mut doc = split();
    let base = doc.snapshot();
    let id = doc.add_layer("fill").unwrap();
    doc.move_layer(id, 2, -3).unwrap();
    let mut options = fill(0, 0, color(1., 0., 0.));
    options.all_layers = true;
    options.opacity = 0.5;
    assert!(doc.bucket_fill(options).unwrap());
    assert_eq!(doc.pixel(4, 2), Pixel::WHITE);
    assert_eq!(
        doc.layers()[0].canvas_pixel(0, 0).unwrap(),
        base.pixel(0, 0)
    );
    assert!((doc.layers()[1].canvas_pixel(0, 0).unwrap().alpha() - 0.5).abs() < 1e-6);
    assert_eq!(
        doc.layers()[1].canvas_pixel(4, 2).unwrap(),
        Pixel::TRANSPARENT
    );
    doc.undo().unwrap();
    options.all_layers = false;
    doc.bucket_fill(options).unwrap();
    assert!((doc.layers()[1].canvas_pixel(4, 2).unwrap().alpha() - 0.5).abs() < 1e-6);
    doc.undo().unwrap();
    let mut shape = SelectionShape::geometry(SelectionKind::Rectangle, 1.25, 0., 2., 4., true);
    shape.feather = 1.;
    let selection = Selection::default()
        .apply(shape, SelectionOperation::Replace, 8, 4)
        .unwrap();
    doc.set_selection(selection.clone()).unwrap();
    options.x = 2;
    options.contiguous = false;
    doc.bucket_fill(options).unwrap();
    for y in 0..4 {
        for x in 0..8 {
            assert!(
                (doc.layers()[1].canvas_pixel(x, y).unwrap().alpha()
                    - 0.5 * selection.coverage(x as f64 + 0.5, y as f64 + 0.5))
                .abs()
                    < 1e-6
            );
        }
    }
}
#[test]
fn tolerance_antialias_opacity_and_transparent_seed() {
    let mut doc = Document::new(2, 1).unwrap();
    let gray = (0.5f32 + 0.055) / 1.055;
    let gray = gray.powf(2.4);
    doc.bucket_fill(fill(0, 0, color(gray, gray, gray)))
        .unwrap();
    doc.begin_stroke(
        Brush {
            radius: 0.5,
            color: color(0., 0., 0.),
            ..Brush::default()
        },
        InputPoint::new(0.5, 0.5, 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    let mut options = fill(0, 0, color(1., 0., 0.));
    options.tolerance = 127;
    options.contiguous = false;
    doc.bucket_fill(options).unwrap();
    // The gray pixel differs by 127.5 sRGB levels: AA gives half coverage.
    assert!((doc.pixel(1, 0).components()[0] - (0.5 + gray * 0.5)).abs() < 1e-4);
    doc.undo().unwrap();
    options.antialias = false;
    doc.bucket_fill(options).unwrap();
    assert_eq!(doc.pixel(1, 0), color(gray, gray, gray));
    doc.undo().unwrap();
    options.tolerance = 128;
    doc.bucket_fill(options).unwrap();
    assert_eq!(doc.pixel(1, 0), color(1., 0., 0.));
    let mut transparent = Document::new(8, 4).unwrap();
    options = fill(0, 0, color(1., 0., 0.));
    options.opacity = 0.25;
    transparent.bucket_fill(options).unwrap();
    assert_eq!(transparent.pixel(7, 3).alpha(), 0.25);
    let depth = transparent.history_depth();
    options.opacity = 0.;
    assert!(!transparent.bucket_fill(options).unwrap());
    assert_eq!(transparent.history_depth(), depth);
}
#[test]
fn locks_masks_invalid_and_resource_errors_leave_pixels_and_history_intact() {
    let mut doc = split();
    let snapshot = doc.snapshot();
    let revision = doc.revision();
    let depth = doc.history_depth();
    let mut bad = fill(0, 0, Pixel::WHITE);
    bad.tolerance = 256;
    assert!(doc.bucket_fill(bad).is_err());
    bad.tolerance = 0;
    bad.x = 8;
    assert!(doc.bucket_fill(bad).is_err());
    assert_eq!(doc.revision(), revision);
    assert_eq!(doc.history_depth(), depth);
    assert_eq!(doc.pixel(3, 2), snapshot.pixel(3, 2));
    let mut appearance = doc.layers()[0].appearance();
    appearance.locks = LOCK_ALL;
    doc.set_layer_appearance(1, appearance).unwrap();
    assert!(doc.bucket_fill(fill(0, 0, Pixel::WHITE)).is_err());
    appearance.locks = LOCK_TRANSPARENCY;
    doc.set_layer_appearance(1, appearance).unwrap();
    let alpha = doc.pixel(0, 0).alpha();
    doc.bucket_fill(fill(0, 0, color(1., 0., 0.))).unwrap();
    assert_eq!(doc.pixel(0, 0).alpha(), alpha);
    let mask = doc.add_mask(1).unwrap();
    doc.set_active_layer(mask).unwrap();
    doc.bucket_fill(fill(0, 0, color(0., 0., 0.))).unwrap();
    assert_eq!(doc.pixel(0, 0).alpha(), 0.);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(0, 0).alpha(), 1.);
    bad = fill(0, 0, Pixel::WHITE);
    bad.all_layers = true;
    assert!(doc.bucket_fill(bad).is_err());
    let mut huge = Document::new(1000000, 1000000).unwrap();
    bad = fill(0, 0, Pixel::WHITE);
    bad.contiguous = false;
    assert!(huge.bucket_fill(bad).is_err());
    assert_eq!(huge.tile_count(), 0);
    assert_eq!(huge.history_depth(), (0, 0));
    let mut limited = Document::with_options(
        128,
        1,
        DocumentOptions {
            max_document_tiles: 1,
            ..DocumentOptions::default()
        },
    )
    .unwrap();
    assert!(limited.bucket_fill(fill(0, 0, Pixel::WHITE)).is_err());
    assert_eq!(limited.revision(), 0);
    assert_eq!(limited.tile_count(), 0);
    assert_eq!(limited.history_depth(), (0, 0));
}

#[test]
fn excluded_seeds_inverse_selections_and_locked_groups_do_not_leak() {
    let mut doc = Document::new(8, 4).unwrap();
    let selection = Selection::default()
        .apply(
            SelectionShape::geometry(SelectionKind::Rectangle, 2., 0., 2., 4., false),
            SelectionOperation::Replace,
            8,
            4,
        )
        .unwrap();
    doc.set_selection(selection.clone()).unwrap();
    let mut options = fill(0, 0, Pixel::WHITE);
    let depth = doc.history_depth();
    assert!(!doc.bucket_fill(options).unwrap());
    options.contiguous = false;
    assert!(!doc.bucket_fill(options).unwrap());
    assert_eq!(doc.history_depth(), depth);
    doc.set_selection(selection.inverted(8, 4).unwrap())
        .unwrap();
    options.contiguous = true;
    doc.bucket_fill(options).unwrap();
    assert_eq!(doc.pixel(0, 0), Pixel::WHITE);
    assert_eq!(doc.pixel(7, 0), Pixel::TRANSPARENT);
    doc.undo().unwrap();
    options.contiguous = false;
    doc.bucket_fill(options).unwrap();
    assert_eq!(doc.pixel(7, 0), Pixel::WHITE);
    assert_eq!(doc.pixel(3, 0), Pixel::TRANSPARENT);
    let group = doc.group_layer(1, "group").unwrap();
    doc.set_active_layer(group).unwrap();
    assert!(doc.bucket_fill(options).is_err());
    doc.set_layer_appearance(
        group,
        LayerAppearance {
            locks: LOCK_ALL,
            ..Default::default()
        },
    )
    .unwrap();
    doc.set_active_layer(1).unwrap();
    let depth = doc.history_depth();
    assert!(doc.bucket_fill(options).is_err());
    assert_eq!(doc.history_depth(), depth);
}
