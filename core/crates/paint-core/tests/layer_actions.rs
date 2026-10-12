use paint_core::*;
fn paint(doc: &mut Document, x: f64, color: [f32; 4]) {
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
fn copy_group_masks_cross_document_duplicate_and_disk_history_are_atomic() {
    let tiles = (0..12)
        .map(|x| {
            (
                TileCoord { x, y: 0 },
                Tile::from_pixels(
                    (0..4096)
                        .map(|i| {
                            Pixel::from_straight([
                                (i % 31) as f32 / 31.,
                                (i % 127) as f32 / 127.,
                                (i % 17) as f32 / 17.,
                                1.,
                            ])
                            .unwrap()
                        })
                        .collect(),
                )
                .unwrap(),
            )
        })
        .collect();
    let mut doc = Document::from_import(
        768,
        64,
        vec![ImportedLayer {
            name: "paint".into(),
            properties: Default::default(),
            tiles,
        }],
        0,
        DocumentOptions {
            max_history_bytes: 4 * TILE_BYTES + 256,
            max_resident_tiles: 2,
            ..Default::default()
        },
    )
    .unwrap();
    for x in [16., 80., 144.] {
        paint(&mut doc, x, [1., 0., 0., 1.]);
    }
    let mask = doc.add_mask(1).unwrap();
    doc.set_active_layer(mask).unwrap();
    paint(&mut doc, 80., [0., 0., 0., 1.]);
    let group = doc.group_layer(1, "组").unwrap();
    let count = doc.layers().len();
    let depth = doc.history_depth().0;
    let bytes = doc.copy_layers(&[group, 1]).unwrap();
    assert_eq!(doc.history_depth().0, depth);
    let duplicate = doc.duplicate_layers(&[group, 1]).unwrap();
    assert_eq!(doc.layers().len(), count * 2);
    assert_eq!(doc.history_depth().0, depth + 1);
    assert!(doc.history_disk_bytes() > 0);
    let copied = doc
        .layers()
        .iter()
        .find(|l| l.parent_id() == duplicate && !l.is_mask())
        .unwrap()
        .id();
    assert_ne!(copied, 1);
    assert!(doc.mask_for(copied).is_some());
    let original = doc.layer(1).unwrap().try_pixel(16, 16).unwrap();
    doc.set_active_layer(copied).unwrap();
    paint(&mut doc, 16., [0., 0., 1., 1.]);
    assert_eq!(doc.layer(1).unwrap().try_pixel(16, 16).unwrap(), original);
    assert_ne!(
        doc.layer(copied).unwrap().try_pixel(16, 16).unwrap(),
        original
    );
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.layers().len(), count);
    doc.redo().unwrap();
    assert_eq!(doc.layers().len(), count * 2);
    assert_eq!(
        doc.layer(copied).unwrap().try_pixel(16, 16).unwrap(),
        original
    );
    let mut other = Document::new(192, 64).unwrap();
    let pasted = other.paste_layers(&bytes, 1).unwrap();
    assert_eq!(other.layers().len(), count + 1);
    assert!(other.layer(pasted).unwrap().is_group());
    assert_eq!(other.pixel(16, 16), original);
    assert_eq!(other.pixel(80, 16), Pixel::TRANSPARENT);
    let revision = other.revision();
    let history = other.history_depth();
    let mut damaged = bytes.clone();
    damaged[8] ^= 1;
    assert!(other.paste_layers(&damaged, 1).is_err());
    assert_eq!(other.revision(), revision);
    assert_eq!(other.history_depth(), history);
    other.undo().unwrap();
    assert_eq!(other.layers().len(), 1);
    other.redo().unwrap();
    assert_eq!(other.pixel(16, 16), original);
    let g = other.group_layer(1, "locked parent").unwrap();
    let mut appearance = other.layer(g).unwrap().appearance();
    appearance.locks = LOCK_ALL;
    other.set_layer_appearance(g, appearance).unwrap();
    let revision = other.revision();
    assert!(other.paste_layers(&bytes, 1).is_err());
    assert_eq!(other.revision(), revision);
}
#[test]
fn merge_down_group_and_selected_subtrees_bake_pixels_with_atomic_undo() {
    let mut doc = Document::new(192, 64).unwrap();
    paint(&mut doc, 16., [1., 0., 0., 1.]);
    let b = doc.add_layer("blue").unwrap();
    paint(&mut doc, 80., [0., 0., 1., 1.]);
    let mask = doc.add_mask(b).unwrap();
    doc.set_active_layer(mask).unwrap();
    paint(&mut doc, 80., [0., 0., 0., 0.5]);
    let before = (doc.pixel(16, 16), doc.pixel(80, 16));
    let depth = doc.history_depth().0;
    let merged = doc.merge_layers(&[b]).unwrap();
    assert_eq!(doc.layers().len(), 1);
    assert_eq!((doc.pixel(16, 16), doc.pixel(80, 16)), before);
    assert_eq!(doc.history_depth().0, depth + 1);
    assert!(doc.mask_for(merged).is_none());
    doc.undo().unwrap();
    assert!(doc.mask_for(b).is_some());
    assert_eq!((doc.pixel(16, 16), doc.pixel(80, 16)), before);
    doc.redo().unwrap();
    assert_eq!((doc.pixel(16, 16), doc.pixel(80, 16)), before);
    doc.undo().unwrap();
    let group = doc.group_layer(b, "组").unwrap();
    let merged_group = doc.merge_layers(&[group]).unwrap();
    assert!(!doc.layer(merged_group).unwrap().is_group());
    assert_eq!((doc.pixel(16, 16), doc.pixel(80, 16)), before);
    doc.undo().unwrap();
    let merged_all = doc.merge_layers(&[1, group, b]).unwrap();
    assert_eq!(doc.layers().len(), 1);
    assert_eq!(doc.active_layer(), merged_all);
    assert_eq!((doc.pixel(16, 16), doc.pixel(80, 16)), before);
    doc.undo().unwrap();
    assert_eq!(doc.layers().len(), 4);
    let mut a = doc.layer(group).unwrap().appearance();
    a.locks = LOCK_ALL;
    doc.set_layer_appearance(group, a).unwrap();
    let rev = doc.revision();
    assert!(doc.merge_layers(&[group]).is_err());
    assert_eq!(doc.revision(), rev);
}
#[test]
fn lower_edge_drop_adopts_target_group_and_self_edge_is_noop() {
    let mut doc = Document::new(64, 64).unwrap();
    let group = doc.group_layer(1, "group").unwrap();
    let outside = doc.add_layer("outside").unwrap();
    doc.reparent_layer(outside, 0).unwrap();
    doc.drop_layers(&[outside], 1, 2).unwrap();
    assert_eq!(doc.layer(outside).unwrap().parent_id(), group);
    let order: Vec<_> = doc.layers().iter().map(|l| l.id()).collect();
    assert!(
        order.iter().position(|id| *id == outside).unwrap()
            < order.iter().position(|id| *id == 1).unwrap()
    );
    doc.undo().unwrap();
    assert_eq!(doc.layer(outside).unwrap().parent_id(), 0);
    doc.drop_layers(&[outside], group, 2).unwrap();
    assert_eq!(doc.layer(outside).unwrap().parent_id(), 0);
    for placement in [1, 2] {
        let rev = doc.revision();
        let depth = doc.history_depth();
        doc.drop_layers(&[outside], outside, placement).unwrap();
        assert_eq!(doc.revision(), rev);
        assert_eq!(doc.history_depth(), depth);
    }
    let rev = doc.revision();
    assert!(doc.drop_layers(&[group], 1, 2).is_err());
    assert_eq!(doc.revision(), rev);
}
