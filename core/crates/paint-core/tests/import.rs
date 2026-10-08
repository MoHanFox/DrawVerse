use paint_core::*;
use std::collections::BTreeMap;
fn layer() -> ImportedLayer {
    ImportedLayer {
        name: "Layer".into(),
        properties: LayerProperties::default(),
        tiles: BTreeMap::new(),
    }
}
#[test]
fn import_constructs_history_free_document_and_monotonic_ids() {
    let mut doc = Document::from_import(
        10,
        10,
        vec![layer(), layer()],
        1,
        DocumentOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.active_layer(), 2);
    assert_eq!(doc.history_depth().0, 0);
    assert_eq!(doc.revision(), 0);
    assert_eq!(doc.add_layer("Next").unwrap(), 3);
}
#[test]
fn import_rejects_invalid_metadata_pixels_and_budget_before_publishing() {
    assert!(Document::from_import(10, 10, vec![], 0, DocumentOptions::default()).is_err());
    let mut bad = layer();
    bad.properties.opacity = f32::NAN;
    assert!(Document::from_import(10, 10, vec![bad], 0, DocumentOptions::default()).is_err());
    let mut bad = layer();
    let mut pixels = vec![Pixel::TRANSPARENT; 4096];
    pixels[11] = Pixel::from_straight([1.; 4]).unwrap();
    bad.tiles
        .insert(TileCoord { x: 0, y: 0 }, Tile::from_pixels(pixels).unwrap());
    assert!(Document::from_import(10, 10, vec![bad], 0, DocumentOptions::default()).is_err());
    assert!(Tile::from_pixels(vec![]).is_err());
    let mut bad = layer();
    bad.tiles
        .insert(TileCoord { x: u32::MAX, y: 0 }, Tile::default());
    assert!(Document::from_import(10, 10, vec![bad], 0, DocumentOptions::default()).is_err());
    let mut bad = layer();
    let mut pixels = vec![Pixel::TRANSPARENT; 4096];
    pixels[0] = Pixel::from_straight([1.; 4]).unwrap();
    bad.tiles
        .insert(TileCoord { x: 0, y: 0 }, Tile::from_pixels(pixels).unwrap());
    assert!(Document::from_import(
        10,
        10,
        vec![bad],
        0,
        DocumentOptions {
            max_document_tiles: 0,
            ..Default::default()
        }
    )
    .is_err());
}
