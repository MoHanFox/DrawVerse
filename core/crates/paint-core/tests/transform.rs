use paint_core::{
    Brush, CanvasTransform, Document, DocumentOptions, InputPoint, Pixel, TILE_BYTES,
};

fn doc(width: u32, height: u32) -> Document {
    Document::with_storage(
        width,
        height,
        DocumentOptions {
            max_history_bytes: 8 * TILE_BYTES,
            ..DocumentOptions::default()
        },
        paint_storage::ScratchSpace::system(),
    )
    .unwrap()
}

fn paint(document: &mut Document, x: f64, y: f64, radius: f32, rgba: [f32; 4]) {
    document
        .begin_stroke(
            Brush {
                radius,
                color: Pixel::from_straight(rgba).unwrap(),
                ..Default::default()
            },
            InputPoint::new(x, y, 1.),
        )
        .unwrap();
    document.end_stroke().unwrap();
}

/// A single opaque red pixel at a known place, so every transform can be checked by position.
fn marker_document(width: u32, height: u32, x: f64, y: f64) -> Document {
    let mut document = doc(width, height);
    document.initialize_white_background().unwrap();
    paint(&mut document, x, y, 1.2, [1., 0., 0., 1.]);
    document
}

fn alpha_at(document: &Document, x: u32, y: u32) -> f32 {
    document.pixel(x, y).components()[3]
}

#[test]
fn quarter_turns_swap_dimensions_and_rotate_content() {
    let mut document = marker_document(16, 8, 2.5, 1.5);
    // 90° clockwise: the top-left marker lands at the top-right of the new canvas.
    document
        .transform_canvas(CanvasTransform::Rotate90Cw)
        .unwrap();
    assert_eq!(document.dimensions(), (8, 16));
    assert!(
        alpha_at(&document, 6, 2) > 0.5,
        "marker missing after cw rotate"
    );
    // Four quarter turns return to the original size and content.
    for _ in 0..3 {
        document
            .transform_canvas(CanvasTransform::Rotate90Cw)
            .unwrap();
    }
    assert_eq!(document.dimensions(), (16, 8));
    assert!(alpha_at(&document, 2, 1) > 0.5, "content did not come back");
}

#[test]
fn counter_clockwise_is_the_inverse_of_clockwise() {
    let mut document = marker_document(16, 8, 2.5, 1.5);
    let before = (2, 1);
    document
        .transform_canvas(CanvasTransform::Rotate90Ccw)
        .unwrap();
    assert_eq!(document.dimensions(), (8, 16));
    document
        .transform_canvas(CanvasTransform::Rotate90Cw)
        .unwrap();
    assert_eq!(document.dimensions(), (16, 8));
    assert!(alpha_at(&document, before.0, before.1) > 0.5);
}

#[test]
fn flips_mirror_without_changing_dimensions() {
    let mut document = marker_document(16, 8, 2.5, 1.5);
    document
        .transform_canvas(CanvasTransform::FlipHorizontal)
        .unwrap();
    assert_eq!(document.dimensions(), (16, 8));
    assert!(
        alpha_at(&document, 13, 1) > 0.5,
        "horizontal flip lost the marker"
    );
    document
        .transform_canvas(CanvasTransform::FlipHorizontal)
        .unwrap();
    assert!(
        alpha_at(&document, 2, 1) > 0.5,
        "double horizontal flip is not the identity"
    );

    document
        .transform_canvas(CanvasTransform::FlipVertical)
        .unwrap();
    assert!(
        alpha_at(&document, 2, 6) > 0.5,
        "vertical flip lost the marker"
    );

    document
        .transform_canvas(CanvasTransform::FlipVertical)
        .unwrap();
    document
        .transform_canvas(CanvasTransform::Rotate180)
        .unwrap();
    assert_eq!(document.dimensions(), (16, 8));
    assert!(
        alpha_at(&document, 13, 6) > 0.5,
        "180° rotation lost the marker"
    );
}

#[test]
fn transform_is_one_undoable_step_including_the_size() {
    let mut document = marker_document(16, 8, 2.5, 1.5);
    let before = document.history_actions().len();
    let revision = document.revision();
    document
        .transform_canvas(CanvasTransform::Rotate90Cw)
        .unwrap();
    assert_ne!(document.revision(), revision);
    // Exactly one new history entry, labelled as a canvas transform.
    assert_eq!(document.history_actions().len(), before + 1);
    assert_eq!(
        document.history_actions().last().copied(),
        Some(paint_core::HistoryAction::TransformCanvas)
    );
    assert!(document.undo().unwrap());
    assert_eq!(document.dimensions(), (16, 8));
    assert!(
        alpha_at(&document, 2, 1) > 0.5,
        "undo did not restore the pixels"
    );
    assert!(document.redo().unwrap());
    assert_eq!(document.dimensions(), (8, 16));
    assert!(
        alpha_at(&document, 6, 2) > 0.5,
        "redo did not restore the transform"
    );
}

#[test]
fn sparsity_and_layer_order_survive_a_transform() {
    let mut document = doc(64, 64);
    document.initialize_white_background().unwrap();
    let top = document.add_layer("top").unwrap();
    document.set_active_layer(top).unwrap();
    paint(&mut document, 4.5, 4.5, 1.2, [0., 1., 0., 1.]);
    let tiles_before = document.layer(top).unwrap().tiles().count();
    assert!(
        tiles_before > 0 && tiles_before < 4,
        "marker should stay sparse"
    );
    document
        .transform_canvas(CanvasTransform::Rotate90Cw)
        .unwrap();
    // The marker is still a single small tile, and the layer order is untouched.
    assert!(document.layer(top).unwrap().tiles().count() <= tiles_before + 1);
    assert_eq!(document.layers().len(), 2);
    assert_eq!(document.active_layer(), top);
}
