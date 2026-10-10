use paint_core::{
    Brush, Document, DocumentOptions, InputPoint, Pixel, Selection, SelectionOperation,
    SelectionShape, TILE_BYTES,
};

fn doc(width: u32, height: u32) -> Document {
    Document::with_storage(
        width,
        height,
        DocumentOptions {
            max_history_bytes: 16 * TILE_BYTES,
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

fn color(rgba: [f32; 4]) -> Pixel {
    Pixel::from_straight(rgba).unwrap()
}

fn red() -> [f32; 4] {
    [1., 0., 0., 1.]
}

/// Transparent background with an opaque white square in the left half.
fn half_document() -> Document {
    let mut document = doc(16, 16);
    paint(&mut document, 4.5, 8., 3.5, [1., 1., 1., 1.]);
    document
}

#[test]
fn fill_covers_the_region_under_the_seed_only() {
    let mut document = half_document();
    let changed = document
        .fill_region(4, 8, 0.1, true, color(red()), 1.)
        .unwrap();
    assert!(changed, "fill reported no change");
    // Inside the white square: filled.
    assert!(document.pixel(4, 8).components()[0] > 0.9);
    // Outside it, still transparent.
    assert_eq!(document.pixel(12, 8).alpha(), 0.);
}

#[test]
fn fill_does_not_disturb_the_user_selection() {
    let mut document = half_document();
    // A deliberate user selection that the bucket must leave byte-identical.
    let user = Selection::default()
        .apply(
            SelectionShape::geometry(paint_core::SelectionKind::Rectangle, 2., 2., 5., 5., false),
            SelectionOperation::Replace,
            16,
            16,
        )
        .unwrap();
    document.set_selection(user.clone()).unwrap();
    let before = document.selection().to_text();
    document
        .fill_region(4, 8, 0.1, true, color(red()), 1.)
        .unwrap();
    assert_eq!(document.selection().to_text(), before);
    assert_eq!(document.selection(), &user);
    // And the fill still respected that selection: outside it the pixels are untouched.
    assert_eq!(document.pixel(4, 12).alpha(), 0.);
}

#[test]
fn one_fill_is_one_undoable_step_and_an_empty_fill_adds_nothing() {
    let mut document = half_document();
    let before = document.history_actions().len();
    assert!(document
        .fill_region(4, 8, 0.1, true, color(red()), 1.)
        .unwrap());
    assert_eq!(document.history_actions().len(), before + 1);
    assert_eq!(
        document.history_actions().last().copied(),
        Some(paint_core::HistoryAction::Fill)
    );
    assert!(document.undo().unwrap());
    // The white square that was there before the fill is intact in every channel again.
    let restored = document.pixel(4, 8).components();
    assert!(
        restored[0] > 0.9 && restored[1] > 0.9 && restored[2] > 0.9,
        "undo kept part of the fill: {restored:?}"
    );
    assert!(document.redo().unwrap());
    let refilled = document.pixel(4, 8).components();
    assert!(
        refilled[0] > 0.9 && refilled[1] < 0.1,
        "redo did not restore the fill: {refilled:?}"
    );

    // Filling a region with the colour it already has changes nothing and adds no history.
    let depth = document.history_actions().len();
    assert!(!document
        .fill_region(4, 8, 0.1, true, color(red()), 1.)
        .unwrap());
    assert_eq!(document.history_actions().len(), depth);
}

#[test]
fn tolerance_widens_the_region_and_discontiguous_mode_crosses_gaps() {
    // Two separate blocks of different colours, so both connectivity and tolerance are observable.
    let mut document = doc(16, 16);
    paint(&mut document, 3., 8., 2., [1., 1., 1., 1.]);
    paint(&mut document, 12., 8., 2., [0.2, 0.2, 0.2, 1.]);
    assert_eq!(document.pixel(8, 8).alpha(), 0., "blocks must not touch");
    // Connected mode fills the white block only; the dark block is a different colour anyway.
    assert!(document
        .fill_region(3, 8, 0.2, true, color(red()), 1.)
        .unwrap());
    assert!(document.pixel(3, 8).components()[0] > 0.9);
    let dark = document.pixel(12, 8).components();
    assert!(
        dark[0] < 0.5 && dark[1] < 0.5,
        "connected fill reached the other block: {dark:?}"
    );
    // Discontiguous mode with a seed on the white block also takes the similar white pixels, and a
    // high tolerance then reaches the dark block as well.
    document
        .fill_region(3, 8, 1., false, color(red()), 1.)
        .unwrap();
    let reached = document.pixel(12, 8).components();
    assert!(
        reached[0] > 0.9 && reached[1] < 0.1,
        "discontiguous fill with full tolerance missed the other block: {reached:?}"
    );
}

#[test]
fn layer_offsets_do_not_shift_the_filled_pixels() {
    let mut document = doc(16, 16);
    paint(&mut document, 4.5, 8., 3., [1., 1., 1., 1.]);
    let layer = document.active_layer();
    document.move_layer(layer, 2, 3).unwrap();
    // The white square now sits at document (6.5, 11); filling there must not use layer-local coords.
    document
        .fill_region(6, 11, 0.1, true, color(red()), 1.)
        .unwrap();
    assert!(
        document.pixel(6, 11).components()[0] > 0.9,
        "offset fill landed elsewhere"
    );
    assert_eq!(document.pixel(0, 0).alpha(), 0.);
}

#[test]
fn invalid_fills_are_rejected_and_change_nothing() {
    let mut document = half_document();
    let before = document.revision();
    let depth = document.history_actions().len();
    // Seed outside the document.
    assert!(document
        .fill_region(99, 0, 0.1, true, color(red()), 1.)
        .is_err());
    // Out-of-range tolerance and opacity.
    assert!(document
        .fill_region(4, 8, -0.1, true, color(red()), 1.)
        .is_err());
    assert!(document
        .fill_region(4, 8, 1.5, true, color(red()), 1.)
        .is_err());
    assert!(document
        .fill_region(4, 8, 0.1, true, color(red()), 0.)
        .is_err());
    assert!(document
        .fill_region(4, 8, 0.1, true, color(red()), f32::NAN)
        .is_err());
    assert!(document
        .fill_region(4, 8, 0.1, true, color([1., 1., 1., 0.]), 1.)
        .is_err());
    assert_eq!(document.revision(), before);
    assert_eq!(document.history_actions().len(), depth);
    // A locked layer refuses the fill instead of writing through the lock.
    let layer = document.active_layer();
    let mut appearance = document.layer(layer).unwrap().appearance();
    appearance.locks = paint_core::LOCK_ALL;
    document.set_layer_appearance(layer, appearance).unwrap();
    assert!(document
        .fill_region(4, 8, 0.1, true, color(red()), 1.)
        .is_err());
}
