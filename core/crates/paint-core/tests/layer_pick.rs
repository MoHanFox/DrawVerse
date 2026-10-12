use paint_core::*;

fn dab(d: &mut Document, x: f64, y: f64) {
    d.begin_stroke(
        Brush {
            radius: 3.,
            ..Default::default()
        },
        InputPoint::new(x, y, 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
}

#[test]
fn pick_layer_transparency_visibility_and_atomic_move() {
    let mut d = Document::new(32, 32).unwrap();
    let bottom = d.active_layer();
    dab(&mut d, 8., 8.);
    let top = d.add_layer("top").unwrap();
    dab(&mut d, 16., 16.);
    let depth = d.history_depth();
    assert_eq!(d.pick_layer(8, 8, 0, 0).unwrap(), Some(bottom));
    assert_eq!(d.history_depth(), depth);
    assert_eq!(d.pick_layer(30, 30, 10, 10).unwrap(), None);
    assert_eq!(d.active_layer(), bottom);
    assert_eq!(d.history_depth(), depth);
    assert_eq!(d.pick_layer(16, 16, 4, -2).unwrap(), Some(top));
    assert_eq!(d.history_depth().0, depth.0 + 1);
    assert_eq!(d.layer(top).unwrap().appearance().offset_x, 4);
    d.undo().unwrap();
    assert_eq!(d.layer(top).unwrap().appearance().offset_x, 0);
    d.redo().unwrap();
    assert_eq!(d.layer(top).unwrap().appearance().offset_x, 4);
    let group = d.group_layer(top, "group").unwrap();
    d.set_layer_properties(
        group,
        LayerProperties {
            visible: false,
            opacity: 1.,
        },
    )
    .unwrap();
    assert_eq!(d.pick_layer(20, 14, 0, 0).unwrap(), None);
    d.set_layer_properties(group, LayerProperties::default())
        .unwrap();
    let mut a = d.layer(group).unwrap().appearance();
    a.locks = LOCK_POSITION;
    d.set_layer_appearance(group, a).unwrap();
    let depth = d.history_depth();
    assert_eq!(d.pick_layer(20, 14, 8, 8).unwrap(), Some(top));
    assert_eq!(d.history_depth(), depth);
    assert_eq!(d.layer(top).unwrap().appearance().offset_x, 4);
}

#[test]
fn pick_layer_respects_masks_clipping_and_placed_images() {
    let mut d = Document::new(32, 32).unwrap();
    let base = d.active_layer();
    dab(&mut d, 8., 8.);
    let top = d.add_layer("clipped").unwrap();
    dab(&mut d, 16., 16.);
    d.set_layer_clipping(top, true).unwrap();
    assert_eq!(d.pick_layer(16, 16, 0, 0).unwrap(), None);
    d.set_layer_clipping(top, false).unwrap();
    let mask = d.add_mask(top).unwrap();
    dab(&mut d, 16., 16.);
    assert_eq!(d.active_layer(), mask);
    assert_eq!(d.pick_layer(16, 16, 0, 0).unwrap(), None);
    d.set_layer_properties(
        mask,
        LayerProperties {
            visible: false,
            opacity: 1.,
        },
    )
    .unwrap();
    assert_eq!(d.pick_layer(16, 16, 0, 0).unwrap(), Some(top));
    assert_eq!(d.pick_layer(8, 8, 0, 0).unwrap(), Some(base));
    let mut src = Document::new(8, 8).unwrap();
    dab(&mut src, 4., 4.);
    d.set_active_layer(top).unwrap();
    let image = d
        .place_image(src.layers()[0].clone(), 8, 8, "image".into())
        .unwrap();
    d.commit_transform().unwrap();
    assert_eq!(d.pick_layer(16, 16, 2, 3).unwrap(), Some(image));
    assert!(d.layer(image).unwrap().is_image());
}
