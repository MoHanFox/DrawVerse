use paint_core::{
    Brush, Document, DocumentOptions, InputPoint, Selection, SelectionKind,
    SelectionOperation as Op, TILE_BYTES,
};

/// White background plus a thin black stroke across the middle: the wand must separate the bands.
fn striped_document(width: u32, height: u32) -> Document {
    let mut doc = Document::with_storage(
        width,
        height,
        DocumentOptions {
            max_history_bytes: 2 * TILE_BYTES + 256,
            ..DocumentOptions::default()
        },
        paint_storage::ScratchSpace::system(),
    )
    .unwrap();
    doc.initialize_white_background().unwrap();
    let brush = Brush {
        radius: 1.,
        ..Brush::default()
    };
    doc.begin_stroke(brush, InputPoint::new(0., 8., 1.))
        .unwrap();
    doc.stroke_to(InputPoint::new(f64::from(width), 8., 1.))
        .unwrap();
    doc.end_stroke().unwrap();
    doc
}

#[test]
fn wand_flood_fills_the_similar_region_only() {
    let doc = striped_document(16, 16);
    // White above and below the black band, so the flood fill stays inside its own band.
    let top = paint_core::wand_shape(&doc, 4, 4, 0.1).unwrap();
    assert_eq!(top.kind, SelectionKind::Mask);
    // The stroke is anti-aliased, so only its exact extent is checked: width covers the row and
    // the fill must stay above the stroke instead of leaking into the lower band.
    assert_eq!((top.x, top.y, top.width), (0., 0., 16.));
    assert!(top.height < 8., "wand leaked across the stroke: {top:?}");
    let selection = Selection::default()
        .apply(top, Op::Replace, 16, 16)
        .unwrap();
    assert_eq!(selection.coverage(4.5, 4.5), 1.);
    assert_eq!(selection.coverage(4.5, 9.5), 0.);
    // The region below the stroke is a separate connected component: seeding there stays below.
    let bottom = paint_core::wand_shape(&doc, 4, 12, 0.1).unwrap();
    assert!(
        bottom.y + bottom.height > 8.,
        "lower band did not start below the stroke: {bottom:?}"
    );
    let bottom_selection = Selection::default()
        .apply(bottom, Op::Replace, 16, 16)
        .unwrap();
    assert_eq!(bottom_selection.coverage(4.5, 12.5), 1.);
    assert_eq!(bottom_selection.coverage(4.5, 4.5), 0.);
    // Seeding inside the black band with a small tolerance stays on the band.
    let band = paint_core::wand_shape(&doc, 8, 8, 0.1).unwrap();
    let band_selection = Selection::default()
        .apply(band, Op::Replace, 16, 16)
        .unwrap();
    assert_eq!(band_selection.coverage(8.5, 8.5), 1.);
    assert_eq!(band_selection.coverage(8.5, 4.5), 0.);
}

#[test]
fn wand_tolerance_widens_or_narrows_the_region() {
    let doc = striped_document(16, 16);
    // A near-zero tolerance keeps the fill inside the exact colour band.
    let tight = paint_core::wand_shape(&doc, 4, 4, 0.).unwrap();
    let tight_selection = Selection::default()
        .apply(tight, Op::Replace, 16, 16)
        .unwrap();
    assert_eq!(tight_selection.coverage(4.5, 9.5), 0.);
    // A full tolerance reaches everything: the result is the whole document as a rectangle,
    // which needs no mask payload at all.
    let wide = paint_core::wand_shape(&doc, 4, 4, 1.).unwrap();
    assert_eq!(wide.kind, SelectionKind::Rectangle);
    assert!(wide.mask.is_none());
    assert_eq!(
        (wide.x, wide.y, wide.width, wide.height),
        (0., 0., 16., 16.)
    );
    let wide_selection = Selection::default()
        .apply(wide, Op::Replace, 16, 16)
        .unwrap();
    assert_eq!(wide_selection.coverage(4.5, 4.5), 1.);
    assert_eq!(wide_selection.coverage(8.5, 8.5), 1.);
    assert_eq!(wide_selection.coverage(15.5, 15.5), 1.);
}

#[test]
fn wand_rejects_bad_seeds_and_tolerance() {
    let doc = striped_document(8, 8);
    assert!(paint_core::wand_shape(&doc, 8, 0, 0.1).is_err());
    assert!(paint_core::wand_shape(&doc, 0, 8, 0.1).is_err());
    assert!(paint_core::wand_shape(&doc, 0, 0, -0.1).is_err());
    assert!(paint_core::wand_shape(&doc, 0, 0, f32::NAN).is_err());
}
