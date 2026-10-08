use paint_core::{
    Brush, BrushMode, Document, DocumentOptions, Error, ImportedLayer, InputPoint, LayerProperties,
    Pixel, Tile, TileCoord, LOCK_ALL, TILE_BYTES,
};
use std::collections::BTreeMap;

fn dab(doc: &mut Document, color: f32, pressure: f32) {
    let color = if color <= 0.04045 {
        color / 12.92
    } else {
        ((color + 0.055) / 1.055).powf(2.4)
    };
    doc.begin_stroke(
        Brush {
            radius: 8.,
            color: Pixel::from_straight([color, color, color, 1.]).unwrap(),
            ..Default::default()
        },
        InputPoint::new(16.5, 16.5, pressure),
    )
    .unwrap();
    doc.end_stroke().unwrap();
}
#[test]
fn black_white_gray_and_pressure_edit_only_mask_and_history_restores() {
    let mut doc = Document::new(64, 64).unwrap();
    doc.initialize_white_background().unwrap();
    let mask = doc.add_mask(1).unwrap();
    assert_eq!(doc.tile_count(), 0);
    assert_eq!(doc.layer_hierarchy(mask).unwrap().1, 1);
    dab(&mut doc, 0., 1.);
    assert_eq!(doc.pixel(16, 16).alpha(), 0.);
    assert_eq!(doc.layer(1).unwrap().pixel(16, 16), Pixel::WHITE);
    assert_eq!(doc.layer_preview(1).unwrap().pixel(16, 16), Pixel::WHITE);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(16, 16), Pixel::WHITE);
    doc.redo().unwrap();
    assert_eq!(doc.pixel(16, 16).alpha(), 0.);
    dab(&mut doc, 1., 1.);
    assert_eq!(doc.pixel(16, 16), Pixel::WHITE);
    dab(&mut doc, 0.5, 0.5);
    assert!((doc.pixel(16, 16).alpha() - 0.75).abs() < 1e-6);
    let preview = doc.layer_preview(mask).unwrap();
    let preview_gray = ((0.75f32 + 0.055) / 1.055).powf(2.4);
    for channel in &preview.pixel(16, 16).components()[..3] {
        assert!((*channel - preview_gray).abs() < 1e-6);
    }
    assert_eq!(preview.pixel(16, 16).alpha(), 1.);
    doc.set_layer_properties(
        mask,
        LayerProperties {
            visible: false,
            opacity: 1.,
        },
    )
    .unwrap();
    assert_eq!(doc.pixel(16, 16), Pixel::WHITE);
    doc.undo().unwrap();
    assert!((doc.pixel(16, 16).alpha() - 0.75).abs() < 1e-6);
    doc.begin_stroke(
        Brush {
            radius: 8.,
            mode: BrushMode::Erase,
            ..Default::default()
        },
        InputPoint::new(16.5, 16.5, 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    assert_eq!(doc.pixel(16, 16), Pixel::WHITE);
}
#[test]
fn group_mask_is_applied_once_and_owner_move_delete_and_locks_are_atomic() {
    let mut doc = Document::new(64, 64).unwrap();
    dab(&mut doc, 1., 1.);
    let group = doc.group_layer(1, "group").unwrap();
    let mask = doc.add_mask(group).unwrap();
    dab(&mut doc, 0., 1.);
    assert_eq!(doc.pixel(16, 16).alpha(), 0.);
    assert!(doc.ungroup_layer(group).is_err());
    assert!(doc.add_mask(group).is_err());
    assert!(doc.drop_layer(mask, 0, 0).is_err());
    doc.move_layer(group, 4, 2).unwrap();
    assert_eq!(doc.layer(mask).unwrap().appearance().offset_x, 4);
    assert_eq!(doc.pixel(20, 18).alpha(), 0.);
    doc.undo().unwrap();
    let mut a = doc.layer(group).unwrap().appearance();
    a.locks = LOCK_ALL;
    doc.set_layer_appearance(group, a).unwrap();
    assert!(matches!(
        doc.begin_stroke(Brush::default(), InputPoint::new(16., 16., 1.)),
        Err(Error::LayerLocked)
    ));
    doc.undo().unwrap();
    doc.set_active_layer(group).unwrap();
    let other = doc.add_layer("other").unwrap();
    doc.drop_layer(other, 0, 0).unwrap();
    doc.remove_layer(group).unwrap();
    assert!(doc.layer(mask).is_none());
    doc.undo().unwrap();
    assert!(doc.layer(mask).unwrap().is_mask());
    assert_eq!(doc.pixel(16, 16).alpha(), 0.);
}
#[test]
fn opaque_background_is_sparse_erasure_overrides_and_undo_survive_empty_tiles() {
    let mut huge = Document::new(1_000_000, 1_000_000).unwrap();
    huge.initialize_white_background().unwrap();
    assert_eq!(huge.tile_count(), 0);
    assert_eq!(huge.pixel(999999, 999999), Pixel::WHITE);
    let mut doc = Document::new(64, 64).unwrap();
    doc.initialize_white_background().unwrap();
    doc.begin_stroke(
        Brush {
            radius: 100.,
            mode: BrushMode::Erase,
            ..Default::default()
        },
        InputPoint::new(32., 32., 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    assert_eq!(doc.pixel(32, 32).alpha(), 0.);
    assert_eq!(doc.tile_count(), 1);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(32, 32), Pixel::WHITE);
    doc.redo().unwrap();
    assert_eq!(doc.pixel(32, 32).alpha(), 0.);
    doc.move_layer(1, 4, 0).unwrap();
    assert_eq!(doc.pixel(2, 2).alpha(), 0.);
    doc.undo().unwrap();
    doc.undo().unwrap();
    doc.move_layer(1, 4, 0).unwrap();
    assert_eq!(doc.pixel(2, 2).alpha(), 0.);
    assert_eq!(doc.pixel(8, 8), Pixel::WHITE);
}

#[test]
fn transparent_pixel_lock_paints_implicit_opaque_background_and_preserves_erased_holes() {
    let mut doc = Document::new(64, 64).unwrap();
    doc.initialize_white_background().unwrap();
    let mut appearance = doc.layer(1).unwrap().appearance();
    appearance.locks = paint_core::LOCK_TRANSPARENCY;
    doc.set_layer_appearance(1, appearance).unwrap();
    dab(&mut doc, 0., 1.);
    assert_eq!(doc.pixel(16, 16).components(), [0., 0., 0., 1.]);
    assert_eq!(doc.pixel(60, 60), Pixel::WHITE);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(16, 16), Pixel::WHITE);
    appearance.locks = 0;
    doc.set_layer_appearance(1, appearance).unwrap();
    doc.begin_stroke(
        Brush {
            radius: 8.,
            mode: BrushMode::Erase,
            ..Default::default()
        },
        InputPoint::new(16.5, 16.5, 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    appearance.locks = paint_core::LOCK_TRANSPARENCY;
    doc.set_layer_appearance(1, appearance).unwrap();
    dab(&mut doc, 0., 1.);
    assert_eq!(doc.pixel(16, 16).alpha(), 0.);
    assert_eq!(doc.pixel(60, 60), Pixel::WHITE);
}
#[test]
fn drag_reorders_reparents_whole_subtree_and_cycle_failure_leaves_history_unchanged() {
    let mut doc = Document::new(64, 64).unwrap();
    let b = doc.add_layer("b").unwrap();
    let c = doc.add_layer("c").unwrap();
    doc.drop_layer(1, c, 1).unwrap();
    assert_eq!(
        doc.layers().iter().map(|l| l.id()).collect::<Vec<_>>(),
        [b, c, 1]
    );
    doc.undo().unwrap();
    let group = doc.group_layer(b, "group").unwrap();
    let mask = doc.add_mask(b).unwrap();
    doc.drop_layer(c, group, 0).unwrap();
    assert_eq!(doc.layer(c).unwrap().parent_id(), group);
    doc.drop_layer(group, 1, 2).unwrap();
    assert_eq!(doc.layers().last().unwrap().id(), 1);
    assert_eq!(doc.layer(mask).unwrap().parent_id(), b);
    let revision = doc.revision();
    let depth = doc.history_depth();
    assert!(doc.drop_layer(group, b, 1).is_err());
    assert_eq!(doc.revision(), revision);
    assert_eq!(doc.history_depth(), depth);
    doc.drop_layer(c, 1, 1).unwrap();
    assert_eq!(doc.layer(c).unwrap().parent_id(), 0);
    doc.undo().unwrap();
    assert_eq!(doc.layer(c).unwrap().parent_id(), group);
}
#[test]
fn disk_history_round_trip_keeps_mask_kind_parent_and_exact_hidden_coverage() {
    let mut tiles = BTreeMap::new();
    for x in 0..6 {
        tiles.insert(
            TileCoord { x, y: 0 },
            Tile::from_pixels(
                (0..4096)
                    .map(|i| {
                        Pixel::from_straight([
                            0.,
                            0.,
                            0.,
                            ((i * 37 + x as usize * 11) % 4096) as f32 / 4096.,
                        ])
                        .unwrap()
                    })
                    .collect(),
            )
            .unwrap(),
        );
    }
    let layers = vec![
        ImportedLayer {
            name: "mask".into(),
            properties: LayerProperties::default(),
            tiles,
        },
        ImportedLayer {
            name: "owner".into(),
            properties: LayerProperties::default(),
            tiles: BTreeMap::new(),
        },
        ImportedLayer {
            name: "other".into(),
            properties: LayerProperties::default(),
            tiles: BTreeMap::new(),
        },
    ];
    let mut doc = Document::from_import(
        384,
        64,
        layers,
        0,
        DocumentOptions {
            max_resident_tiles: 2,
            max_history_bytes: 2 * TILE_BYTES + 256,
            ..Default::default()
        },
    )
    .unwrap();
    doc.set_import_nodes(&[
        (2, false, true, (0, 0)),
        (0, false, false, (0, 0)),
        (0, false, false, (0, 0)),
    ])
    .unwrap();
    let bits = doc
        .layer(1)
        .unwrap()
        .read_row(0, 0, 384)
        .unwrap()
        .iter()
        .map(|p| p.alpha().to_bits())
        .collect::<Vec<_>>();
    doc.remove_layer(2).unwrap();
    assert!(doc.history_disk_bytes() > 0);
    doc.undo().unwrap();
    assert!(doc.layer(1).unwrap().is_mask());
    assert_eq!(doc.layer(1).unwrap().parent_id(), 2);
    assert_eq!(
        doc.layer(1)
            .unwrap()
            .read_row(0, 0, 384)
            .unwrap()
            .iter()
            .map(|p| p.alpha().to_bits())
            .collect::<Vec<_>>(),
        bits
    );
    doc.redo().unwrap();
    assert!(doc.layer(1).is_none());
}
