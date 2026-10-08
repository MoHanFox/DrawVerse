use paint_core::*;
fn paint(doc: &mut Document, color: [f32; 4], x: f64) {
    doc.begin_stroke(
        Brush {
            radius: 8.,
            color: Pixel::from_straight(color).unwrap(),
            ..Default::default()
        },
        InputPoint::new(x, 16., 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
}
#[test]
fn isolated_group_opacity_applies_once_and_structure_history_keeps_pixels() {
    let mut doc = Document::new(128, 64).unwrap();
    paint(&mut doc, [1., 0., 0., 1.], 16.);
    let group = doc.group_layer(1, "group").unwrap();
    let top = doc.add_layer("blue").unwrap();
    paint(&mut doc, [0., 0., 1., 1.], 16.);
    assert_eq!(doc.layer(top).unwrap().parent_id(), group);
    doc.set_layer_properties(
        group,
        LayerProperties {
            visible: true,
            opacity: 0.5,
        },
    )
    .unwrap();
    assert_eq!(doc.pixel(16, 16).components(), [0., 0., 0.5, 0.5]);
    let count = doc.tile_count();
    doc.ungroup_layer(group).unwrap();
    assert_eq!(doc.pixel(16, 16).components(), [0., 0., 1., 1.]);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(16, 16).components(), [0., 0., 0.5, 0.5]);
    doc.redo().unwrap();
    assert_eq!(doc.tile_count(), count);
    assert!(doc.layers().iter().all(|l| l.parent_id() == 0));
}
#[test]
fn nesting_cycle_depth_and_parent_validation_are_atomic() {
    let mut doc = Document::new(64, 64).unwrap();
    let a = doc.group_layer(1, "a").unwrap();
    let b = doc.group_layer(a, "b").unwrap();
    let rev = doc.revision();
    assert!(doc.reparent_layer(b, a).is_err());
    assert!(doc.reparent_layer(b, 1).is_err());
    assert_eq!(doc.revision(), rev);
    assert_eq!(doc.layer_hierarchy(1).unwrap().1, 2);
    for (layer, hierarchy) in doc.layers().iter().zip(doc.layer_hierarchies()) {
        assert_eq!(hierarchy, doc.layer_hierarchy(layer.id()).unwrap());
    }
    doc.reparent_layer(1, 0).unwrap();
    assert_eq!(doc.layer_hierarchy(1).unwrap().1, 0);
    doc.undo().unwrap();
    assert_eq!(doc.layer_hierarchy(1).unwrap().1, 2);
    let mut root = b;
    for _ in 2..16 {
        root = doc.group_layer(root, "nested").unwrap();
    }
    assert_eq!(doc.layer_hierarchy(1).unwrap().1, 16);
    let rev = doc.revision();
    assert!(matches!(
        doc.group_layer(root, "too deep"),
        Err(Error::ResourceLimit(_))
    ));
    assert_eq!(doc.revision(), rev);
    doc.set_active_layer(root).unwrap();
    assert!(doc
        .begin_stroke(Brush::default(), InputPoint::new(1., 1., 1.))
        .is_err());
}
#[test]
fn group_move_checks_every_descendant_before_mutation_and_inherits_locks() {
    let mut doc = Document::new(128, 64).unwrap();
    paint(&mut doc, [1., 0., 0., 1.], 16.);
    let group = doc.group_layer(1, "group").unwrap();
    let top = doc.add_layer("top").unwrap();
    paint(&mut doc, [0., 1., 0., 1.], 48.);
    let mut a = doc.layer(top).unwrap().appearance();
    a.locks = LOCK_POSITION;
    doc.set_layer_appearance(top, a).unwrap();
    let rev = doc.revision();
    assert!(matches!(
        doc.move_layer(group, 8, 0),
        Err(Error::LayerLocked)
    ));
    assert_eq!(doc.layer(1).unwrap().appearance().offset_x, 0);
    assert_eq!(doc.revision(), rev);
    a.locks = 0;
    doc.set_layer_appearance(top, a).unwrap();
    doc.move_layer(group, 8, 0).unwrap();
    assert_eq!(doc.layer(1).unwrap().appearance().offset_x, 8);
    assert_eq!(doc.layer(top).unwrap().appearance().offset_x, 8);
    doc.undo().unwrap();
    assert_eq!(doc.layer(1).unwrap().appearance().offset_x, 0);
    let mut a = doc.layer(group).unwrap().appearance();
    a.locks = LOCK_ALL;
    doc.set_layer_appearance(group, a).unwrap();
    doc.set_active_layer(1).unwrap();
    assert!(matches!(
        doc.begin_stroke(Brush::default(), InputPoint::new(1., 1., 1.)),
        Err(Error::LayerLocked)
    ));
    assert!(matches!(doc.reparent_layer(1, 0), Err(Error::LayerLocked)));
    assert!(matches!(doc.remove_layer(1), Err(Error::LayerLocked)));
    let mut a = doc.layer(1).unwrap().appearance();
    a.locks = 0;
    assert!(matches!(
        doc.set_layer_appearance(1, a),
        Err(Error::LayerLocked)
    ));
}
#[test]
fn deleting_subtree_uses_compressed_history_and_restores_exact_content_and_membership() {
    let mut doc = Document::with_options(
        256,
        256,
        DocumentOptions {
            max_history_bytes: 2 * TILE_BYTES + 256,
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    paint(&mut doc, [1., 0., 0., 1.], 16.);
    let group = doc.group_layer(1, "group").unwrap();
    let top = doc.add_layer("top").unwrap();
    doc.begin_stroke(
        Brush {
            radius: 100.,
            ..Default::default()
        },
        InputPoint::new(128., 128., 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    let before = doc.snapshot();
    let count = doc.tile_count();
    doc.set_active_layer(group).unwrap();
    assert!(matches!(doc.remove_layer(group), Err(Error::LastLayer)));
    doc.set_active_layer(group).unwrap();
    let outside = doc.add_layer("inside").unwrap();
    doc.reparent_layer(outside, 0).unwrap();
    doc.remove_layer(group).unwrap();
    assert_eq!(doc.layers().len(), 1);
    // Repetitive brush tiles may compress entirely in memory. The deletion
    // must remain undoable whether its bounded payload spills or compresses.
    assert!(doc.history_bytes() <= doc.document_options().max_history_bytes);
    doc.undo().unwrap();
    assert_eq!(doc.layer(top).unwrap().parent_id(), group);
    assert_eq!(doc.tile_count(), count);
    for y in [16, 64, 128, 192] {
        for x in [16, 64, 128, 192] {
            assert_eq!(doc.pixel(x, y), before.pixel(x, y));
        }
    }
    doc.redo().unwrap();
    assert!(doc.layer(group).is_none());
}

#[test]
fn deleting_noisy_cold_subtree_spills_to_disk_and_restores_exact_pixels() {
    let tiles = (0..4)
        .map(|x| {
            let pixels = (0..4096)
                .map(|i| {
                    Pixel::from_straight([
                        i as f32 / 4096.,
                        ((i * 17) % 4096) as f32 / 4096.,
                        x as f32 / 4.,
                        1.,
                    ])
                    .unwrap()
                })
                .collect();
            (TileCoord { x, y: 0 }, Tile::from_pixels(pixels).unwrap())
        })
        .collect();
    let mut doc = Document::from_import(
        256,
        64,
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
    )
    .unwrap();
    let group = doc.group_layer(1, "cold group").unwrap();
    let outside = doc.add_layer("outside").unwrap();
    doc.reparent_layer(outside, 0).unwrap();
    let before = doc.snapshot();
    doc.remove_layer(group).unwrap();
    assert!(doc.history_disk_bytes() > 0);
    assert!(doc.layer(1).is_none());
    doc.undo().unwrap();
    assert_eq!(doc.layer(1).unwrap().parent_id(), group);
    assert_eq!(doc.tile_count(), 4);
    for y in [0, 32, 63] {
        for x in [0, 63, 64, 127, 192, 255] {
            assert_eq!(
                doc.pixel(x, y).components().map(f32::to_bits),
                before.pixel(x, y).components().map(f32::to_bits)
            );
        }
    }
    doc.redo().unwrap();
    assert_eq!(doc.layers().len(), 1);
    assert_eq!(doc.tile_count(), 0);
}
#[test]
fn import_rejects_cycles_contiguity_and_raster_groups_before_mutation() {
    let mut doc = Document::new(64, 64).unwrap();
    assert!(doc.set_import_hierarchy(&[(1, true)]).is_err());
    assert!(!doc.layer(1).unwrap().is_group());
    assert!(doc.set_import_hierarchy(&[(9, false)]).is_err());
    assert_eq!(doc.layer(1).unwrap().parent_id(), 0);
}
