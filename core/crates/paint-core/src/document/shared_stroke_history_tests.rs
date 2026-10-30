use super::*;

#[test]
fn small_strokes_retain_shared_results_without_pixel_budget_eviction() {
    let mut doc = Document::with_options(
        64,
        64,
        DocumentOptions {
            max_history_bytes: 2 * TILE_BYTES + 256,
            ..Default::default()
        },
    )
    .unwrap();
    for step in 0..100 {
        doc.begin_stroke(
            Brush {
                radius: 4.,
                color: if step % 2 == 0 {
                    Pixel::WHITE
                } else {
                    Pixel::from_straight([0., 0., 0., 1.]).unwrap()
                },
                ..Default::default()
            },
            InputPoint::new(16., 16., 1.),
        )
        .unwrap();
        doc.end_stroke().unwrap();
    }
    assert_eq!(doc.history_depth(), (100, 0));
    assert!(doc.history_bytes() <= 2 * TILE_BYTES + 256);
    assert_eq!(doc.history_disk_bytes(), 0);
    let final_tile = doc.layers[0].tiles.values().next().unwrap().clone();
    doc.select_history(0).unwrap();
    assert_eq!(doc.tile_count(), 0);
    doc.select_history(100).unwrap();
    assert!(Arc::ptr_eq(
        &final_tile,
        doc.layers[0].tiles.values().next().unwrap()
    ));
    doc.undo().unwrap();
    doc.begin_stroke(Brush::default(), InputPoint::new(32., 32., 1.))
        .unwrap();
    doc.end_stroke().unwrap();
    assert_eq!(doc.history_depth(), (100, 0));
}

#[test]
fn stroke_commit_shares_exact_tiles_even_when_history_encoding_fails() {
    for mode in [BrushMode::Paint, BrushMode::Erase] {
        let mut doc = Document::new(192, 128).unwrap();
        doc.initialize_white_background().unwrap();
        doc.begin_stroke(
            Brush {
                radius: 128.,
                mode,
                ..Default::default()
            },
            InputPoint::new(96., 64., 1.),
        )
        .unwrap();
        crate::history_storage::fail_writes_for_test(true);
        let result = doc.end_stroke();
        crate::history_storage::fail_writes_for_test(false);
        assert!(result.unwrap());
        let tiles = doc.layers[0].tiles.clone();
        assert!(matches!(
            doc.history.undo.back(),
            Some(StoredCommand::Paged(..))
        ));
        doc.undo().unwrap();
        doc.redo().unwrap();
        for (coord, tile) in tiles {
            assert!(Arc::ptr_eq(&tile, &doc.layers[0].tiles[&coord]));
        }
    }
}
