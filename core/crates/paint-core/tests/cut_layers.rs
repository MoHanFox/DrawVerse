use paint_core::*;
fn dab(doc: &mut Document, x: f64, y: f64) {
    doc.begin_stroke(
        Brush {
            radius: 6.,
            color: Pixel::WHITE,
            ..Default::default()
        },
        InputPoint::new(x, y, 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
}
fn pixels(doc: &Document) -> Vec<Pixel> {
    (0..64)
        .flat_map(|y| (0..64).map(move |x| doc.pixel(x, y)))
        .collect()
}
#[test]
fn cut_group_copies_complete_forest_and_removes_once_with_exact_undo() {
    let mut doc = Document::new(64, 64).unwrap();
    let layer = doc.add_layer("cut").unwrap();
    dab(&mut doc, 24., 24.);
    doc.add_mask(layer).unwrap();
    dab(&mut doc, 20., 20.);
    let group = doc.group_layer(layer, "group").unwrap();
    let original = pixels(&doc);
    let before = doc.layers().len();
    let depth = doc.history_depth().0;
    let bytes = doc.cut_layers(&[group, layer], false).unwrap();
    assert_eq!(doc.layers().len(), 1);
    assert_eq!(doc.history_depth().0, depth + 1);
    let mut pasted = Document::new(64, 64).unwrap();
    pasted.paste_layers(&bytes, 1).unwrap();
    assert_eq!(pasted.layers().len(), before);
    assert_eq!(pixels(&pasted), original);
    assert!(doc.undo().unwrap());
    assert_eq!(doc.layers().len(), before);
    assert_eq!(pixels(&doc), original);
    assert!(doc.redo().unwrap());
    assert_eq!(doc.layers().len(), 1);
}
#[test]
fn cut_selected_pixels_preserves_layer_selection_and_outside_pixels() {
    let mut doc = Document::new(64, 64).unwrap();
    dab(&mut doc, 16., 16.);
    dab(&mut doc, 48., 48.);
    let selection = Selection::default()
        .apply(
            SelectionShape::geometry(SelectionKind::Rectangle, 0., 0., 32., 32., true),
            SelectionOperation::Replace,
            64,
            64,
        )
        .unwrap();
    doc.set_selection(selection.clone()).unwrap();
    let original = pixels(&doc);
    let depth = doc.history_depth().0;
    let bytes = doc.cut_layers(&[1], true).unwrap();
    assert_eq!(doc.layers().len(), 1);
    assert_eq!(doc.selection(), &selection);
    assert_eq!(doc.pixel(16, 16), Pixel::TRANSPARENT);
    assert_eq!(doc.pixel(48, 48), Pixel::WHITE);
    assert_eq!(doc.history_depth().0, depth + 1);
    let mut pasted = Document::new(64, 64).unwrap();
    pasted.paste_layers(&bytes, 1).unwrap();
    assert_eq!(pasted.pixel(16, 16), Pixel::WHITE);
    assert_eq!(pasted.pixel(48, 48), Pixel::TRANSPARENT);
    assert!(doc.undo().unwrap());
    assert_eq!(pixels(&doc), original);
    assert_eq!(doc.selection(), &selection);
}
#[test]
fn failed_cut_keeps_document_and_history_intact() {
    let mut doc = Document::new(64, 64).unwrap();
    dab(&mut doc, 16., 16.);
    doc.set_layer_appearance(
        1,
        LayerAppearance {
            locks: LOCK_ALL,
            ..Default::default()
        },
    )
    .unwrap();
    let original = pixels(&doc);
    let revision = doc.revision();
    let depth = doc.history_depth();
    assert!(doc.cut_layers(&[1], false).is_err());
    assert!(doc.cut_layers(&[1], true).is_err());
    assert!(doc.cut_layers(&[999], false).is_err());
    assert_eq!(doc.revision(), revision);
    assert_eq!(doc.history_depth(), depth);
    assert_eq!(pixels(&doc), original);
    let mut masked = Document::new(64, 64).unwrap();
    let mask = masked.add_mask(1).unwrap();
    let revision = masked.revision();
    let depth = masked.history_depth();
    assert!(masked.cut_layers(&[mask], false).is_err());
    assert_eq!(masked.layers().len(), 2);
    assert_eq!(masked.revision(), revision);
    assert_eq!(masked.history_depth(), depth);
}
