use paint_core::*;
use std::collections::BTreeMap;
fn layer(name: &str, pixel: impl Fn(u32, u32) -> Pixel) -> ImportedLayer {
    let mut tiles = BTreeMap::new();
    tiles.insert(
        TileCoord { x: 0, y: 0 },
        Tile::from_pixels((0..4096).map(|i| pixel(i % 64, i / 64)).collect()).unwrap(),
    );
    ImportedLayer {
        name: name.into(),
        properties: LayerProperties::default(),
        tiles,
    }
}
fn color(c: [f32; 4]) -> Pixel {
    Pixel::from_straight(c).unwrap()
}
fn doc() -> Document {
    Document::from_import(
        64,
        64,
        vec![
            layer("base", |x, _| {
                if x < 32 {
                    color([0., 1., 0., 0.5])
                } else {
                    Pixel::TRANSPARENT
                }
            }),
            layer("red", |_, _| color([1., 0., 0., 1.])),
        ],
        1,
        DocumentOptions::default(),
    )
    .unwrap()
}
#[test]
fn clipping_preserves_base_alpha_chain_opacity_blend_and_raw_preview() {
    let mut d = doc();
    d.set_layer_clipping(2, true).unwrap();
    assert_eq!(d.pixel(16, 16).components(), [0.5, 0., 0., 0.5]);
    assert_eq!(d.pixel(48, 16), Pixel::TRANSPARENT);
    assert_eq!(
        d.layer_preview(2).unwrap().pixel(48, 16).components(),
        [1., 0., 0., 1.]
    );
    let a = d.add_layer("blue").unwrap();
    d.begin_stroke(
        Brush {
            radius: 60.,
            color: color([0., 0., 1., 0.5]),
            ..Default::default()
        },
        InputPoint::new(16., 16., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    d.set_layer_clipping(a, true).unwrap();
    assert_eq!(d.layer_clipping_bases(), vec![0, 1, 1]);
    let p = d.pixel(16, 16).components();
    assert!((p[0] - 0.25).abs() < 1e-6 && (p[2] - 0.25).abs() < 1e-6);
    assert_eq!(p[3], 0.5);
    d.set_layer_properties(
        1,
        LayerProperties {
            visible: true,
            opacity: 0.5,
        },
    )
    .unwrap();
    assert_eq!(d.pixel(16, 16).alpha(), 0.25);
    d.set_layer_properties(
        1,
        LayerProperties {
            visible: false,
            opacity: 0.5,
        },
    )
    .unwrap();
    assert_eq!(d.pixel(16, 16), Pixel::TRANSPARENT);
    d.undo().unwrap();
    d.undo().unwrap();
    d.undo().unwrap();
    assert!(!d.layer(a).unwrap().is_clipped());
    d.redo().unwrap();
    assert!(d.layer(a).unwrap().is_clipped());
    d.set_layer_properties(
        a,
        LayerProperties {
            visible: false,
            opacity: 1.,
        },
    )
    .unwrap();
    let mut appearance = d.layer(2).unwrap().appearance();
    appearance.blend = BlendMode::Multiply;
    d.set_layer_appearance(2, appearance).unwrap();
    assert_eq!(d.pixel(16, 16).components(), [0., 0., 0., 0.5]);
}
#[test]
fn masks_groups_positions_locks_and_orphan_reorder_follow_current_siblings() {
    let mut d = doc();
    let mask = d.add_mask(1).unwrap();
    d.begin_stroke(
        Brush {
            radius: 7.,
            color: color([0., 0., 0., 1.]),
            ..Default::default()
        },
        InputPoint::new(16.5, 16.5, 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    d.set_layer_clipping(2, true).unwrap();
    assert_eq!(d.pixel(16, 16), Pixel::TRANSPARENT);
    assert_eq!(d.pixel(24, 16).alpha(), 0.5);
    d.set_layer_properties(
        mask,
        LayerProperties {
            visible: false,
            opacity: 1.,
        },
    )
    .unwrap();
    assert_eq!(d.pixel(16, 16).alpha(), 0.5);
    let group = d.group_layer(1, "base group").unwrap();
    assert_eq!(d.layer_clipping_bases().last(), Some(&group));
    assert_eq!(d.pixel(16, 16).alpha(), 0.5);
    d.move_layer(group, 32, 0).unwrap();
    assert_eq!(d.pixel(16, 16), Pixel::TRANSPARENT);
    assert_eq!(d.pixel(48, 16).alpha(), 0.5);
    d.drop_layer(2, group, 2).unwrap();
    // Dropped to the very end of the group it has no unclipped sibling below any more, so the clip
    // is released instead of being left as a layer the compositor would silently ignore.
    assert!(!d.layer(2).unwrap().is_clipped());
    assert_eq!(d.layer_clipping_bases()[0], 0);
    assert_eq!(d.pixel(48, 16).components(), [0.5, 0.5, 0., 1.]);
    d.undo().unwrap();
    // One undo restores both the position and the clip.
    assert!(d.layer(2).unwrap().is_clipped());
    assert_eq!(d.pixel(48, 16).components(), [0.5, 0., 0., 0.5]);
    d.reparent_layer(2, group).unwrap();
    assert_eq!(
        d.layer_clipping_bases().iter().copied().find(|&b| b != 0),
        Some(1)
    );
    let mut a = d.layer(group).unwrap().appearance();
    a.locks = LOCK_ALL;
    d.set_layer_appearance(group, a).unwrap();
    assert!(matches!(
        d.set_layer_clipping(2, false),
        Err(Error::LayerLocked)
    ));
    assert!(d.set_layer_clipping(mask, true).is_err());
}
#[test]
fn invalid_creation_is_atomic_and_disk_delete_history_preserves_clipping() {
    let mut upper = layer("noise", |_, _| Pixel::WHITE);
    for x in 0..6 {
        upper.tiles.insert(
            TileCoord { x, y: 0 },
            Tile::from_pixels(
                (0..4096)
                    .map(|i| color([((i * 71 + x * 31) % 997) as f32 / 997., 0.2, 0.1, 1.]))
                    .collect(),
            )
            .unwrap(),
        );
    }
    let mut d = Document::from_import(
        384,
        64,
        vec![layer("base", |_, _| Pixel::WHITE), upper],
        1,
        DocumentOptions {
            max_resident_tiles: 2,
            max_history_bytes: 2 * TILE_BYTES + 256,
            ..Default::default()
        },
    )
    .unwrap();
    let r = d.revision();
    assert!(d.set_layer_clipping(1, true).is_err());
    assert_eq!(d.revision(), r);
    assert_eq!(d.history_depth(), (0, 0));
    d.set_layer_clipping(2, true).unwrap();
    let p = d.layer(2).unwrap().pixel(40, 32);
    d.remove_layer(2).unwrap();
    assert!(d.history_disk_bytes() > 0);
    d.undo().unwrap();
    assert!(d.layer(2).unwrap().is_clipped());
    assert_eq!(d.layer(2).unwrap().pixel(40, 32), p);
    d.redo().unwrap();
    assert!(d.layer(2).is_none());
}
