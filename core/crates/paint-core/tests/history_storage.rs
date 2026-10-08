use paint_core::{
    Brush, Document, DocumentOptions, ImportedLayer, InputPoint, LayerProperties, Pixel, Tile,
    TileCoord, TILE_BYTES,
};
use std::collections::BTreeMap;
fn noisy_document() -> Document {
    let options = DocumentOptions {
        max_history_bytes: 2 * TILE_BYTES + 256,
        ..Default::default()
    };
    let mut tiles = BTreeMap::new();
    for y in 0..2 {
        for x in 0..3 {
            let pixels = (0..4096)
                .map(|i| {
                    Pixel::from_straight([
                        i as f32 / 4096.,
                        ((i * 17 + x * 5 + y * 11) % 4096) as f32 / 4096.,
                        0.25,
                        1.,
                    ])
                    .unwrap()
                })
                .collect();
            tiles.insert(TileCoord { x, y }, Tile::from_pixels(pixels).unwrap());
        }
    }
    Document::from_import(
        192,
        128,
        vec![
            ImportedLayer {
                name: "bottom".into(),
                properties: LayerProperties::default(),
                tiles: BTreeMap::new(),
            },
            ImportedLayer {
                name: "noise".into(),
                properties: LayerProperties {
                    visible: true,
                    opacity: 0.7,
                },
                tiles,
            },
        ],
        1,
        options,
    )
    .unwrap()
}
fn pixels(doc: &Document) -> Vec<[u32; 4]> {
    let (w, h) = doc.dimensions();
    (0..h)
        .flat_map(|y| (0..w).map(move |x| doc.pixel(x, y).components().map(f32::to_bits)))
        .collect()
}
#[test]
fn spilled_layer_deletion_undo_redo_and_branching_preserve_exact_pixels() {
    let mut doc = noisy_document();
    let original = pixels(&doc);
    let id = doc.active_layer();
    doc.remove_layer(id).unwrap();
    assert!(doc.history_disk_bytes() > 0);
    assert!(doc.history_bytes() <= 2 * TILE_BYTES + 256);
    assert_eq!(doc.history_depth(), (1, 0));
    doc.undo().unwrap();
    assert_eq!(pixels(&doc), original);
    doc.redo().unwrap();
    assert!(doc.layer(id).is_none());
    doc.undo().unwrap();
    doc.set_active_layer(id).unwrap();
    doc.begin_stroke(Brush::default(), InputPoint::new(12., 12., 1.))
        .unwrap();
    doc.end_stroke().unwrap();
    assert_eq!(doc.history_disk_bytes(), 0);
    assert_eq!(doc.history_depth().1, 0);
}
#[test]
fn noisy_large_stroke_spills_and_cancellation_preserves_existing_history() {
    let mut doc = noisy_document();
    let original = pixels(&doc);
    let brush = Brush {
        radius: 128.,
        ..Default::default()
    };
    doc.begin_stroke(brush, InputPoint::new(96., 64., 1.))
        .unwrap();
    doc.end_stroke().unwrap();
    let painted = pixels(&doc);
    assert_ne!(painted, original);
    assert!(doc.history_disk_bytes() > 0);
    doc.undo().unwrap();
    assert_eq!(pixels(&doc), original);
    doc.redo().unwrap();
    assert_eq!(pixels(&doc), painted);
    let depth = doc.history_depth();
    let disk = doc.history_disk_bytes();
    doc.begin_stroke(brush, InputPoint::new(20., 20., 0.5))
        .unwrap();
    doc.cancel_stroke().unwrap();
    assert_eq!(pixels(&doc), painted);
    assert_eq!(doc.history_depth(), depth);
    assert_eq!(doc.history_disk_bytes(), disk);
}
#[test]
fn count_eviction_releases_disk_history_without_changing_current_canvas() {
    let mut doc = noisy_document();
    // A disk-backed deletion followed by small commands makes its earlier
    // history expire under the unchanged 100-command limit.
    let id = doc.active_layer();
    doc.remove_layer(id).unwrap();
    assert!(doc.history_disk_bytes() > 0);
    for i in 0..101 {
        doc.set_layer_properties(
            1,
            LayerProperties {
                visible: i % 2 == 0,
                opacity: 0.5,
            },
        )
        .unwrap();
    }
    assert_eq!(doc.history_disk_bytes(), 0);
    assert_eq!(doc.history_depth().0, 100);
    assert!(doc.layer(id).is_none());
}
