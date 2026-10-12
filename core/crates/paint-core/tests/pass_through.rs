use paint_core::*;
fn paint(d: &mut Document, c: [f32; 4]) {
    d.begin_stroke(
        Brush {
            radius: 8.,
            color: Pixel::from_straight(c).unwrap(),
            ..Brush::default()
        },
        InputPoint::new(16., 16., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
}
fn mode(d: &mut Document, id: u64, m: BlendMode) {
    let mut a = d.layer(id).unwrap().appearance();
    a.blend = m;
    d.set_layer_appearance(id, a).unwrap();
}
fn near(a: Pixel, b: Pixel) {
    for (a, b) in a.components().into_iter().zip(b.components()) {
        assert!((a - b).abs() < 1e-6, "{a} vs {b}");
    }
}
fn setup() -> (Document, u64, u64, Pixel, Pixel) {
    let mut d = Document::new(32, 32).unwrap();
    d.begin_stroke(
        Brush {
            radius: 32.,
            color: Pixel::from_straight([0.2, 0.3, 0.4, 1.]).unwrap(),
            ..Brush::default()
        },
        InputPoint::new(16., 16., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    let backdrop = d.pixel(16, 16);
    let top = d.add_layer("dodge").unwrap();
    paint(&mut d, [0.5, 0.25, 0.2, 0.8]);
    mode(&mut d, top, BlendMode::ColorDodge);
    let flat = d.pixel(16, 16);
    let group = d.group_layer(top, "group").unwrap();
    (d, top, group, backdrop, flat)
}
#[test]
fn pass_through_uses_lower_backdrop_and_group_weights_history() {
    let (mut d, top, g, b, flat) = setup();
    assert_ne!(d.pixel(16, 16), flat);
    let isolated = d.pixel(16, 16);
    let depth = d.history_depth();
    mode(&mut d, g, BlendMode::PassThrough);
    near(d.pixel(16, 16), flat);
    assert_eq!(d.history_depth().0, depth.0 + 1);
    d.undo().unwrap();
    assert_eq!(d.pixel(16, 16), isolated);
    d.redo().unwrap();
    near(d.pixel(16, 16), flat);
    assert!(d
        .set_layer_appearance(
            top,
            LayerAppearance {
                blend: BlendMode::PassThrough,
                ..d.layer(top).unwrap().appearance()
            }
        )
        .is_err());
    assert!(d
        .snapshot()
        .with_layer_blend(top, BlendMode::PassThrough)
        .is_err());
    d.set_layer_properties(
        g,
        LayerProperties {
            visible: true,
            opacity: 0.5,
        },
    )
    .unwrap();
    let mut expected = [0.; 4];
    for (i, e) in expected.iter_mut().enumerate() {
        *e = (b.components()[i] + flat.components()[i]) * 0.5;
    }
    near(d.pixel(16, 16), Pixel::from_straight(expected).unwrap());
    let mut a = d.layer(g).unwrap().appearance();
    a.fill = 0.5;
    d.set_layer_appearance(g, a).unwrap();
    for (i, e) in expected.iter_mut().enumerate() {
        *e = b.components()[i] * 0.75 + flat.components()[i] * 0.25;
    }
    near(d.pixel(16, 16), Pixel::from_straight(expected).unwrap());
    let row = d.snapshot().read_row(0, 16, 32).unwrap();
    for (x, p) in row.iter().enumerate() {
        assert_eq!(*p, d.pixel(x as u32, 16));
    }
    d.set_layer_properties(
        g,
        LayerProperties {
            visible: false,
            opacity: 1.,
        },
    )
    .unwrap();
    assert_eq!(d.pixel(16, 16), b);
    d.set_active_layer(1).unwrap();
    let blank = d.add_layer("empty").unwrap();
    let empty = d.group_layer(blank, "empty pass").unwrap();
    mode(&mut d, empty, BlendMode::PassThrough);
    d.set_layer_properties(
        empty,
        LayerProperties {
            visible: true,
            opacity: 0.3,
        },
    )
    .unwrap();
    assert_eq!(d.pixel(16, 16), b);
}
#[test]
fn nested_isolation_masks_and_internal_clipping_remain_bounded() {
    let (mut d, _top, g, b, flat) = setup();
    mode(&mut d, g, BlendMode::PassThrough);
    let outer = d.group_layer(g, "outer").unwrap();
    assert_ne!(d.pixel(16, 16), flat);
    mode(&mut d, outer, BlendMode::PassThrough);
    near(d.pixel(16, 16), flat);
    let mask = d.add_mask(g).unwrap();
    d.set_active_layer(mask).unwrap();
    paint(&mut d, [0., 0., 0., 1.]);
    assert_eq!(d.pixel(16, 16), b);
    d.undo().unwrap();
    near(d.pixel(16, 16), flat);
    let (mut flatdoc, top, g, _, _) = setup();
    flatdoc.ungroup_layer(g).unwrap();
    flatdoc.set_active_layer(top).unwrap();
    let clip = flatdoc.add_layer("clip").unwrap();
    paint(&mut flatdoc, [0.2, 0.8, 0.4, 0.6]);
    flatdoc.set_layer_clipping(clip, true).unwrap();
    mode(&mut flatdoc, clip, BlendMode::Screen);
    let flat = flatdoc.pixel(16, 16);
    let group = flatdoc.group_layers(&[top, clip], "clip chain").unwrap();
    mode(&mut flatdoc, group, BlendMode::PassThrough);
    near(flatdoc.pixel(16, 16), flat);
}
#[test]
fn pass_through_clipping_groups_do_not_borrow_backdrop_alpha() {
    let (mut d, _, g, b, _) = setup();
    mode(&mut d, g, BlendMode::PassThrough);
    d.set_active_layer(g).unwrap();
    let upper = d.add_layer("upper").unwrap();
    d.reparent_layer(upper, 0).unwrap();
    paint(&mut d, [0., 1., 0., 1.]);
    d.set_layer_clipping(upper, true).unwrap();
    assert_eq!(d.pixel(0, 0), b);
    assert_eq!(d.pixel(5, 16), b);
    assert!(d.pixel(16, 16).components()[1] > b.components()[1]);
    let (mut d, top, g, _, _) = setup();
    mode(&mut d, g, BlendMode::PassThrough);
    d.set_layer_clipping(g, true).unwrap();
    assert_eq!(d.pixel(0, 0).alpha(), 1.);
    assert_eq!(d.pixel(16, 16).alpha(), 1.);
    d.set_layer_properties(
        top,
        LayerProperties {
            visible: false,
            opacity: 1.,
        },
    )
    .unwrap();
    assert_eq!(d.pixel(16, 16).components(), [0.2, 0.3, 0.4, 1.]);
}
