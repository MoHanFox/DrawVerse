use paint_core::{
    Brush, Document, DocumentOptions, ImportedLayer, InputPoint, LayerProperties, Pixel, Tile,
    TileCoord, TILE_BYTES,
};
use std::collections::BTreeMap;

fn fixture(resident: usize) -> Document {
    let layers = (0..2)
        .map(|layer| ImportedLayer {
            name: format!("layer {layer}"),
            properties: LayerProperties {
                visible: true,
                opacity: 0.7,
            },
            tiles: (0..3)
                .flat_map(|x| {
                    (0..2).map(move |y| {
                        let pixels = (0..4096)
                            .map(|i| {
                                Pixel::from_straight([
                                    i as f32 / 4096.,
                                    (x + y + layer) as f32 / 8.,
                                    0.4,
                                    0.8,
                                ])
                                .unwrap()
                            })
                            .collect();
                        (TileCoord { x, y }, Tile::from_pixels(pixels).unwrap())
                    })
                })
                .collect::<BTreeMap<_, _>>(),
        })
        .collect();
    Document::from_import(
        192,
        128,
        layers,
        1,
        DocumentOptions {
            max_resident_tiles: resident,
            max_history_bytes: 2 * TILE_BYTES + 256,
            ..Default::default()
        },
    )
    .unwrap()
}
fn bits(doc: &paint_core::DocumentSnapshot) -> Vec<[u32; 4]> {
    (0..doc.height)
        .flat_map(|y| {
            doc.read_row(0, y, doc.width)
                .unwrap()
                .into_iter()
                .map(|p| p.components().map(f32::to_bits))
        })
        .collect()
}
#[test]
fn private_dabs_reuse_pixels_but_shared_snapshots_force_a_copy() {
    let mut doc = Document::new(64, 64).unwrap();
    doc.begin_stroke(
        Brush {
            radius: 4.,
            ..Default::default()
        },
        InputPoint::new(12., 12., 1.),
    )
    .unwrap();
    let coord = TileCoord { x: 0, y: 0 };
    let old = std::sync::Arc::downgrade(doc.layer(1).unwrap().tile_ref(coord).unwrap());
    let address = doc
        .layer(1)
        .unwrap()
        .tile(coord)
        .unwrap()
        .try_pixels()
        .unwrap()
        .as_ref()
        .as_ptr();
    doc.stroke_to(InputPoint::new(14., 12., 1.)).unwrap();
    assert!(old.upgrade().is_none());
    assert_eq!(
        doc.layer(1)
            .unwrap()
            .tile(coord)
            .unwrap()
            .try_pixels()
            .unwrap()
            .as_ref()
            .as_ptr(),
        address
    );
    let shared = doc.snapshot();
    let expected = bits(&shared);
    doc.stroke_to(InputPoint::new(16., 12., 1.)).unwrap();
    assert_ne!(
        doc.layer(1)
            .unwrap()
            .tile(coord)
            .unwrap()
            .try_pixels()
            .unwrap()
            .as_ref()
            .as_ptr(),
        address
    );
    assert_eq!(bits(&shared), expected);
    drop(shared);
    doc.end_stroke().unwrap();
    assert_eq!(doc.storage_stats().registered_tiles, 1);
    doc.undo().unwrap();
    assert_eq!(doc.tile_count(), 0);
}
#[test]
fn paged_multilayer_snapshot_cancel_and_disk_history_match_resident_document_exactly() {
    let mut paged = fixture(2);
    let mut resident = fixture(4096);
    let old = paged.snapshot();
    let old_bits = bits(&old);
    assert_eq!(old_bits, bits(&resident.snapshot()));
    let brush = Brush {
        radius: 128.,
        opacity: 0.6,
        ..Default::default()
    };
    for doc in [&mut paged, &mut resident] {
        doc.begin_stroke(brush, InputPoint::new(96., 64., 0.8))
            .unwrap();
        doc.end_stroke().unwrap();
    }
    let painted = bits(&paged.snapshot());
    assert_eq!(painted, bits(&resident.snapshot()));
    assert_ne!(painted, old_bits);
    assert_eq!(bits(&old), old_bits);
    assert!(paged.history_disk_bytes() > 0);
    paged
        .begin_stroke(brush, InputPoint::new(10., 10., 0.6))
        .unwrap();
    paged.cancel_stroke().unwrap();
    assert_eq!(bits(&paged.snapshot()), painted);
    paged.undo().unwrap();
    assert_eq!(bits(&paged.snapshot()), old_bits);
    paged.redo().unwrap();
    assert_eq!(bits(&paged.snapshot()), painted);
    paged.trim_storage().unwrap();
    let stats = paged.storage_stats();
    assert!(stats.resident_tiles <= 2);
    assert!(stats.page_reads > 0 && stats.page_writes > 0);
}
#[test]
fn full_canvas_stroke_crosses_default_resident_budget_and_keeps_one_undo() {
    let mut doc = Document::new(4096, 4160).unwrap();
    let brush = Brush {
        radius: 256.,
        spacing: 1.,
        ..Default::default()
    };
    doc.begin_stroke(brush, InputPoint::new(128., 128., 1.))
        .unwrap();
    for row in 0..17 {
        for col in 0..16 {
            let x = if row % 2 == 0 { col } else { 15 - col };
            doc.stroke_to(InputPoint::new(
                f64::from(x) * 256. + 128.,
                f64::from(row) * 256. + 128.,
                1.,
            ))
            .unwrap();
        }
    }
    doc.end_stroke().unwrap();
    assert_eq!(doc.tile_count(), 4160);
    let snapshot = doc.snapshot();
    assert!(snapshot.layers()[0].tiles().all(|(_, t)| t
        .try_pixels()
        .unwrap()
        .iter()
        .all(|p| p.alpha() == 1.)));
    doc.trim_storage().unwrap();
    let stats = doc.storage_stats();
    assert!(stats.resident_tiles <= 4096);
    assert!(stats.page_writes > 0);
    assert_eq!(doc.history_depth(), (1, 0));
    doc.undo().unwrap();
    assert_eq!(doc.tile_count(), 0);
    doc.redo().unwrap();
    assert_eq!(doc.tile_count(), 4160);
    for (coord, tile) in snapshot.layers()[0].tiles() {
        assert_eq!(
            tile.try_pixels().unwrap().as_ref(),
            doc.layer(1)
                .unwrap()
                .tile(coord)
                .unwrap()
                .try_pixels()
                .unwrap()
                .as_ref()
        );
    }
    doc.trim_storage().unwrap();
    assert!(doc.storage_stats().resident_tiles <= 4096);
}
