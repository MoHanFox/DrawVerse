use paint_core::*;

fn ids(doc: &Document) -> Vec<u64> {
    doc.layers().iter().map(|l| l.id()).collect()
}

#[test]
fn grouping_noncontiguous_selection_preserves_order_masks_pixels_and_one_history_step() {
    let mut doc = Document::new(64, 64).unwrap();
    let a = doc.add_layer("a").unwrap();
    doc.begin_stroke(Brush::default(), InputPoint::new(16., 16., 1.))
        .unwrap();
    doc.end_stroke().unwrap();
    let skipped = doc.add_layer("skipped").unwrap();
    let b = doc.add_layer("b").unwrap();
    let mask = doc.add_mask(b).unwrap();
    let pixels = doc.pixel(16, 16);
    let tiles = doc.tile_count();
    let before: Vec<_> = doc
        .layers()
        .iter()
        .map(|l| (l.id(), l.parent_id()))
        .collect();
    let depth = doc.history_depth().0;
    let group = doc.group_layers(&[b, a], "selection").unwrap();
    assert_eq!(ids(&doc), vec![1, skipped, a, mask, b, group]);
    assert_eq!(doc.layer(a).unwrap().parent_id(), group);
    assert_eq!(doc.layer(b).unwrap().parent_id(), group);
    assert_eq!(doc.layer(mask).unwrap().parent_id(), b);
    assert_eq!(doc.active_layer(), group);
    assert_eq!(doc.history_depth().0, depth + 1);
    assert_eq!(doc.pixel(16, 16), pixels);
    assert_eq!(doc.tile_count(), tiles);
    doc.undo().unwrap();
    assert_eq!(
        doc.layers()
            .iter()
            .map(|l| (l.id(), l.parent_id()))
            .collect::<Vec<_>>(),
        before
    );
    doc.redo().unwrap();
    assert_eq!(doc.layer(a).unwrap().parent_id(), group);
    assert_eq!(doc.layer(b).unwrap().parent_id(), group);
}

#[test]
fn grouping_across_parents_uses_common_parent_and_deduplicates_subtrees() {
    let mut doc = Document::new(64, 64).unwrap();
    let a = doc.add_layer("a").unwrap();
    let ga = doc.group_layer(a, "a group").unwrap();
    doc.set_active_layer(1).unwrap();
    let b = doc.add_layer("b").unwrap();
    let gb = doc.group_layer(b, "b group").unwrap();
    let wrapper = doc.group_layers(&[ga, gb], "common").unwrap();
    let new_group = doc.group_layers(&[a, b], "children").unwrap();
    assert_eq!(doc.layer(new_group).unwrap().parent_id(), wrapper);
    assert_eq!(doc.layer(a).unwrap().parent_id(), new_group);
    assert_eq!(doc.layer(b).unwrap().parent_id(), new_group);
    assert_eq!(ids(&doc), vec![1, gb, ga, b, a, new_group, wrapper]);
    let outer = doc.group_layers(&[a, new_group], "deduplicated").unwrap();
    assert_eq!(doc.layer(a).unwrap().parent_id(), new_group);
    assert_eq!(doc.layer(new_group).unwrap().parent_id(), outer);
    assert_eq!(doc.layer(outer).unwrap().parent_id(), wrapper);
}

#[test]
fn grouping_keeps_internal_clipping_but_releases_external_bases() {
    let mut doc = Document::new(64, 64).unwrap();
    let a = doc.add_layer("a").unwrap();
    let b = doc.add_layer("b").unwrap();
    doc.set_layer_clipping(b, true).unwrap();
    let group = doc.group_layers(&[b, a], "chain").unwrap();
    assert!(doc.layer(b).unwrap().is_clipped());
    assert_eq!(doc.layer(a).unwrap().parent_id(), group);
    doc.undo().unwrap();
    doc.group_layers(&[b], "external base").unwrap();
    assert!(!doc.layer(b).unwrap().is_clipped());
    doc.undo().unwrap();
    assert!(doc.layer(b).unwrap().is_clipped());
}

#[test]
fn invalid_or_locked_group_selection_never_changes_structure_history_or_next_id() {
    let mut doc = Document::new(64, 64).unwrap();
    let a = doc.add_layer("a").unwrap();
    let mask = doc.add_mask(a).unwrap();
    let depth = doc.history_depth();
    let revision = doc.revision();
    let before = ids(&doc);
    for selection in [vec![], vec![a, a], vec![1, 999], vec![mask]] {
        assert!(doc.group_layers(&selection, "group").is_err());
        assert_eq!(ids(&doc), before);
        assert_eq!(doc.history_depth(), depth);
        assert_eq!(doc.revision(), revision);
    }
    let mut appearance = doc.layer(a).unwrap().appearance();
    appearance.locks = LOCK_ALL;
    doc.set_layer_appearance(a, appearance).unwrap();
    let depth = doc.history_depth();
    let revision = doc.revision();
    assert!(doc.group_layers(&[1, a], "locked").is_err());
    assert_eq!(doc.history_depth(), depth);
    assert_eq!(doc.revision(), revision);
    assert_eq!(ids(&doc), before);
    doc.undo().unwrap();
    assert_eq!(doc.group_layers(&[1, a], "valid").unwrap(), mask + 1);
}

#[test]
fn grouping_at_maximum_depth_rejects_the_whole_selection() {
    let mut doc = Document::new(64, 64).unwrap();
    let id = doc.add_layer("nested").unwrap();
    let mut root = id;
    for _ in 0..16 {
        root = doc.group_layer(root, "nest").unwrap();
    }
    let before = ids(&doc);
    let depth = doc.history_depth();
    let revision = doc.revision();
    assert!(matches!(
        doc.group_layers(&[id], "too deep"),
        Err(Error::ResourceLimit(_))
    ));
    assert_eq!(ids(&doc), before);
    assert_eq!(doc.history_depth(), depth);
    assert_eq!(doc.revision(), revision);
}

#[test]
fn grouping_large_metadata_roundtrips_disk_history_without_losing_raster_nodes() {
    let layers = (0..512)
        .map(|_| imported(&"layer".repeat(200), false, false))
        .collect();
    let mut doc = Document::from_import(
        64,
        64,
        layers,
        0,
        DocumentOptions {
            max_history_bytes: 2 * TILE_BYTES + 256,
            ..Default::default()
        },
    )
    .unwrap();
    let before = ids(&doc);
    let group = doc.group_layers(&[512, 1], "group").unwrap();
    assert!(doc.history_disk_bytes() > 0);
    assert_eq!(doc.history_depth().0, 1);
    doc.undo().unwrap();
    assert_eq!(ids(&doc), before);
    doc.redo().unwrap();
    assert_eq!(doc.layer(1).unwrap().parent_id(), group);
    assert_eq!(doc.layer(512).unwrap().parent_id(), group);
    assert_eq!(doc.layers().iter().filter(|l| !l.is_group()).count(), 512);
}
fn imported(name: &str, noisy: bool, mask: bool) -> ImportedLayer {
    ImportedLayer {
        name: name.into(),
        properties: LayerProperties::default(),
        tiles: if noisy {
            (0..3)
                .map(|x| {
                    let pixels = (0..4096)
                        .map(|i| {
                            if mask {
                                Pixel::from_straight([0., 0., 0., (i + 1) as f32 / 4096.]).unwrap()
                            } else {
                                Pixel::from_straight([i as f32 / 4096., x as f32 / 4., 0.25, 1.])
                                    .unwrap()
                            }
                        })
                        .collect();
                    (TileCoord { x, y: 0 }, Tile::from_pixels(pixels).unwrap())
                })
                .collect()
        } else {
            Default::default()
        },
    }
}

#[test]
fn noncontiguous_delete_is_one_disk_backed_history_step_with_exact_pixels() {
    let mut doc = Document::from_import(
        192,
        64,
        vec![
            imported("base", false, false),
            imported("a", true, false),
            imported("b", false, false),
            imported("c", true, false),
        ],
        3,
        DocumentOptions {
            max_history_bytes: 2 * TILE_BYTES + 256,
            ..Default::default()
        },
    )
    .unwrap();
    let (a, b, c) = (2, 3, 4);
    let before = ids(&doc);
    let pixels = doc.pixel(16, 16);
    let tiles = doc.tile_count();
    doc.remove_layers(&[c, a], false).unwrap();
    assert_eq!(ids(&doc), vec![1, b]);
    assert_eq!(doc.history_depth().0, 1);
    assert!(doc.history_disk_bytes() > 0);
    doc.undo().unwrap();
    assert_eq!(ids(&doc), before);
    assert_eq!(doc.pixel(16, 16), pixels);
    assert_eq!(doc.tile_count(), tiles);
    doc.redo().unwrap();
    assert_eq!(ids(&doc), vec![1, b]);
    doc.undo().unwrap();
    assert_eq!(doc.tile_count(), tiles);
}

#[test]
fn group_only_removes_container_and_mask_but_keeps_children_once() {
    let mut doc = Document::from_import(
        192,
        64,
        vec![
            imported("base", false, false),
            imported("child", true, false),
            imported("mask", true, true),
            imported("group", false, false),
        ],
        1,
        DocumentOptions {
            max_history_bytes: 2 * TILE_BYTES + 256,
            ..Default::default()
        },
    )
    .unwrap();
    let (child, mask, group) = (2, 3, 4);
    doc.set_import_nodes(&[
        (group, false, false, (0, 0)),
        (group, false, false, (0, 0)),
        (group, false, true, (0, 0)),
        (0, true, false, (0, 0)),
    ])
    .unwrap();
    let mask_pixel = doc.layer(mask).unwrap().pixel(80, 16);
    let before = ids(&doc);
    let tiles = doc.tile_count();
    doc.remove_layers(&[child, group, 1], true).unwrap();
    assert_eq!(ids(&doc), vec![1, child]);
    assert!(doc.layers().iter().all(|l| l.parent_id() == 0));
    assert_eq!(doc.history_depth().0, 1);
    assert!(doc.history_disk_bytes() > 0);
    doc.undo().unwrap();
    assert_eq!(ids(&doc), before);
    assert_eq!(doc.tile_count(), tiles);
    assert_eq!(doc.mask_for(group).unwrap().id(), mask);
    assert_eq!(doc.layer(mask).unwrap().pixel(80, 16), mask_pixel);
    doc.redo().unwrap();
    assert_eq!(ids(&doc), vec![1, child]);
}

#[test]
fn subtree_delete_normalizes_overlaps_and_failures_are_atomic() {
    let mut doc = Document::new(64, 64).unwrap();
    let group = doc.group_layer(1, "group").unwrap();
    doc.set_active_layer(group).unwrap();
    let outside = doc.add_layer("outside").unwrap();
    doc.reparent_layer(outside, 0).unwrap();
    let before = ids(&doc);
    let depth = doc.history_depth();
    let revision = doc.revision();
    for request in [vec![group, 999], vec![group, group], vec![]] {
        assert!(doc.remove_layers(&request, false).is_err());
        assert_eq!(ids(&doc), before);
        assert_eq!(doc.history_depth(), depth);
        assert_eq!(doc.revision(), revision);
    }
    // The later empty-layer-stack contract permits deleting every subtree.
    // It still has to be one reversible transaction, including the active ID.
    doc.remove_layers(&[group, outside], false).unwrap();
    assert!(doc.layers().is_empty());
    assert_eq!(doc.active_layer(), 0);
    assert_eq!(doc.history_depth().0, depth.0 + 1);
    doc.undo().unwrap();
    assert_eq!(ids(&doc), before);
    doc.redo().unwrap();
    assert!(doc.layers().is_empty());
    doc.undo().unwrap();
    assert_eq!(ids(&doc), before);
    doc.remove_layers(&[1, group], false).unwrap();
    assert_eq!(ids(&doc), vec![outside]);
    doc.undo().unwrap();
    assert_eq!(ids(&doc), before);
}

#[test]
fn batch_drop_preserves_stack_order_masks_and_one_undo_step() {
    let mut doc = Document::new(64, 64).unwrap();
    let a = doc.add_layer("a").unwrap();
    let mask = doc.add_mask(a).unwrap();
    doc.set_active_layer(a).unwrap();
    let b = doc.add_layer("b").unwrap();
    let c = doc.add_layer("c").unwrap();
    let before = ids(&doc);
    let depth = doc.history_depth().0;
    doc.drop_layers(&[c, a], 1, 2).unwrap();
    assert_eq!(ids(&doc), vec![mask, a, c, 1, b]);
    assert_eq!(doc.mask_for(a).unwrap().id(), mask);
    assert_eq!(doc.history_depth().0, depth + 1);
    doc.undo().unwrap();
    assert_eq!(ids(&doc), before);
    doc.redo().unwrap();
    assert_eq!(ids(&doc), vec![mask, a, c, 1, b]);
}

#[test]
fn drop_checks_every_selected_subtree_and_target_before_change() {
    let mut doc = Document::new(64, 64).unwrap();
    let group = doc.group_layer(1, "g").unwrap();
    let child = doc.add_layer("child").unwrap();
    doc.set_active_layer(group).unwrap();
    let outside = doc.add_layer("outside").unwrap();
    doc.reparent_layer(outside, 0).unwrap();
    let before = ids(&doc);
    let depth = doc.history_depth();
    let revision = doc.revision();
    assert!(doc.drop_layers(&[group, outside], child, 1).is_err());
    assert_eq!(ids(&doc), before);
    assert_eq!(doc.history_depth(), depth);
    assert_eq!(doc.revision(), revision);
    let mut appearance = doc.layer(child).unwrap().appearance();
    appearance.locks = LOCK_ALL;
    doc.set_layer_appearance(child, appearance).unwrap();
    let depth = doc.history_depth();
    let revision = doc.revision();
    assert!(doc.drop_layers(&[outside, group], 0, 0).is_err());
    assert!(doc.remove_layers(&[outside, group], false).is_err());
    assert_eq!(ids(&doc), before);
    assert_eq!(doc.history_depth(), depth);
    assert_eq!(doc.revision(), revision);
}

#[test]
fn dropping_below_clipped_upper_sibling_joins_chain_and_undo_restores_flags() {
    let mut doc = Document::new(64, 64).unwrap();
    let clip = doc.add_layer("clip").unwrap();
    doc.set_layer_clipping(clip, true).unwrap();
    let moved = doc.add_layer("moved").unwrap();
    doc.set_layer_clipping(moved, false).unwrap();
    doc.drop_layer(moved, clip, 2).unwrap();
    assert_eq!(ids(&doc), vec![1, moved, clip]);
    assert!(doc.layer(moved).unwrap().is_clipped());
    assert!(doc.layer(clip).unwrap().is_clipped());
    assert_eq!(doc.layer_clipping_bases(), vec![0, 1, 1]);
    doc.undo().unwrap();
    assert!(!doc.layer(moved).unwrap().is_clipped());
    assert!(doc.layer(clip).unwrap().is_clipped());
    doc.redo().unwrap();
    assert_eq!(doc.layer_clipping_bases(), vec![0, 1, 1]);
}

#[test]
fn adjacent_batch_roots_join_destination_chain_and_old_chain_is_released() {
    let mut doc = Document::new(64, 64).unwrap();
    let clip = doc.add_layer("clip").unwrap();
    doc.set_layer_clipping(clip, true).unwrap();
    let a = doc.add_layer("a").unwrap();
    doc.set_layer_clipping(a, false).unwrap();
    let old_clip = doc.add_layer("old clip").unwrap();
    doc.set_layer_clipping(old_clip, true).unwrap();
    let b = doc.add_layer("b").unwrap();
    doc.set_layer_clipping(b, false).unwrap();
    doc.drop_layers(&[b, a], clip, 2).unwrap();
    assert_eq!(ids(&doc), vec![1, a, b, clip, old_clip]);
    assert!(doc.layer(a).unwrap().is_clipped());
    assert!(doc.layer(b).unwrap().is_clipped());
    assert!(doc.layer(clip).unwrap().is_clipped());
    assert!(!doc.layer(old_clip).unwrap().is_clipped());
}
