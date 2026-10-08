use paint_core::{
    Brush, Document, DocumentOptions, ImportedLayer, InputPoint, LayerProperties, Pixel, Tile,
    TileCoord, TILE_BYTES,
};
use paint_storage::{Config, ScratchSpace};
use std::{collections::BTreeMap, fs};

#[test]
fn configured_directory_contains_pages_and_history_and_survives_snapshot() {
    let base = tempfile::Builder::new()
        .prefix("暂存盘-")
        .tempdir()
        .unwrap();
    let space = ScratchSpace::new(Config {
        directory: base.path().into(),
        min_free_bytes: 0,
    });
    let tiles = (0..6)
        .map(|y| {
            let pixels = (0..4096)
                .map(|i| Pixel::from_straight([i as f32 / 4096., y as f32 / 8., 0.4, 1.]).unwrap())
                .collect();
            (TileCoord { x: 0, y }, Tile::from_pixels(pixels).unwrap())
        })
        .collect::<BTreeMap<_, _>>();
    let mut doc = Document::from_import_with_storage(
        64,
        384,
        vec![ImportedLayer {
            name: "noise".into(),
            properties: LayerProperties::default(),
            tiles,
        }],
        0,
        DocumentOptions {
            max_resident_tiles: 1,
            max_history_bytes: 2 * TILE_BYTES + 256,
            ..Default::default()
        },
        space.clone(),
    )
    .unwrap();
    let before = doc.snapshot();
    let original = before.read_row(0, 120, 64).unwrap();
    doc.begin_stroke(
        Brush {
            radius: 256.,
            ..Default::default()
        },
        InputPoint::new(32., 192., 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    assert!(doc.history_disk_bytes() > 0);
    assert!(doc.storage_stats().page_writes > 0);
    let root = base.path().join(".drawverse-scratch-v1");
    let run = fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    let names = fs::read_dir(&run)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(names.iter().any(|n| n.starts_with("pages-")));
    assert!(names.iter().any(|n| n.starts_with("history-")));
    doc.undo().unwrap();
    assert_eq!(doc.snapshot().read_row(0, 120, 64).unwrap(), original);
    doc.redo().unwrap();
    assert_ne!(doc.snapshot().read_row(0, 120, 64).unwrap(), original);
    drop(doc);
    drop(space);
    assert!(run.is_dir());
    assert_eq!(before.read_row(0, 120, 64).unwrap(), original);
    drop(before);
    assert!(!run.exists());
}
#[test]
fn reserve_failure_during_paging_keeps_existing_history_and_pixels() {
    let base = tempfile::tempdir().unwrap();
    let storage = ScratchSpace::new(Config {
        directory: base.path().into(),
        min_free_bytes: u64::MAX,
    });
    let mut doc = Document::with_storage(
        128,
        64,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
        storage,
    )
    .unwrap();
    doc.add_layer("preserved").unwrap();
    let depth = doc.history_depth();
    let result = doc.begin_stroke(
        Brush {
            radius: 64.,
            ..Default::default()
        },
        InputPoint::new(64., 32., 1.),
    );
    assert!(matches!(result, Err(paint_core::Error::Storage(_))));
    assert!(!doc.stroke_active());
    assert_eq!(doc.history_depth(), depth);
    assert_eq!(doc.pixel(64, 32), Pixel::TRANSPARENT);
    doc.undo().unwrap();
    assert_eq!(doc.layers().len(), 1);
}
