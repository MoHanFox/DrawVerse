use paint_core::*;
use std::collections::BTreeMap;
fn paint(doc: &mut Document, x: f64, y: f64, color: [f32; 4], mode: BrushMode) -> bool {
    doc.begin_stroke(
        Brush {
            radius: 6.,
            color: Pixel::from_straight(color).unwrap(),
            mode,
            ..Default::default()
        },
        InputPoint::new(x, y, 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap()
}
fn raw(doc: &Document) -> Vec<[u32; 4]> {
    let (w, h) = doc.dimensions();
    let layer = doc.layer(doc.active_layer()).unwrap();
    (0..h)
        .flat_map(|y| {
            (0..w).map(move |x| {
                layer
                    .canvas_pixel(x, y)
                    .unwrap()
                    .components()
                    .map(f32::to_bits)
            })
        })
        .collect()
}
#[test]
fn alpha_lock_preserves_every_alpha_bit_and_never_allocates_transparent_tiles() {
    let mut doc = Document::new(192, 64).unwrap();
    paint(&mut doc, 63.5, 32.5, [1., 0., 0., 0.4], BrushMode::Paint);
    let before = raw(&doc);
    let tiles = doc.layer(1).unwrap().tile_count();
    let mut a = doc.layer(1).unwrap().appearance();
    a.locks = LOCK_TRANSPARENCY;
    doc.set_layer_appearance(1, a).unwrap();
    paint(&mut doc, 63.5, 32.5, [0., 1., 0., 1.], BrushMode::Paint);
    let after = raw(&doc);
    assert_ne!(before, after);
    assert!(before.iter().zip(&after).all(|(b, a)| b[3] == a[3]));
    assert!(!paint(&mut doc, 130., 32., [1.; 4], BrushMode::Paint));
    assert!(!paint(&mut doc, 63.5, 32.5, [1.; 4], BrushMode::Erase));
    assert_eq!(raw(&doc), after);
    assert_eq!(doc.layer(1).unwrap().tile_count(), tiles);
    doc.undo().unwrap();
    assert_eq!(raw(&doc), before);
    doc.redo().unwrap();
    assert_eq!(raw(&doc), after);
}
#[test]
fn position_lock_all_lock_and_unlock_are_enforced_without_partial_edits() {
    let mut doc = Document::new(128, 64).unwrap();
    let id = doc.add_layer("top").unwrap();
    let mut a = doc.layer(id).unwrap().appearance();
    a.locks = LOCK_POSITION;
    doc.set_layer_appearance(id, a).unwrap();
    assert!(paint(&mut doc, 20., 20., [1.; 4], BrushMode::Paint));
    let revision = doc.revision();
    assert_eq!(doc.move_layer(id, 1, 0), Err(Error::LayerLocked));
    assert_eq!(revision, doc.revision());
    a.locks = LOCK_ALL;
    doc.set_layer_appearance(id, a).unwrap();
    let revision = doc.revision();
    let history = doc.history_depth();
    assert_eq!(
        doc.begin_stroke(Brush::default(), InputPoint::new(10., 10., 1.)),
        Err(Error::LayerLocked)
    );
    assert_eq!(doc.remove_layer(id), Err(Error::LayerLocked));
    let mut changed = a;
    changed.fill = 0.5;
    assert_eq!(
        doc.set_layer_appearance(id, changed),
        Err(Error::LayerLocked)
    );
    changed = a;
    changed.offset_x = 1;
    changed.locks = 0;
    assert_eq!(
        doc.set_layer_appearance(id, changed),
        Err(Error::LayerLocked)
    );
    changed = a;
    changed.dissolve_seed += 1;
    assert_eq!(
        doc.set_layer_appearance(id, changed),
        Err(Error::LayerLocked)
    );
    assert_eq!(
        doc.set_layer_properties(
            id,
            LayerProperties {
                visible: true,
                opacity: 0.5
            }
        ),
        Err(Error::LayerLocked)
    );
    assert_eq!(revision, doc.revision());
    assert_eq!(history, doc.history_depth());
    doc.set_layer_properties(
        id,
        LayerProperties {
            visible: false,
            opacity: 1.,
        },
    )
    .unwrap();
    a.locks = 0;
    doc.set_layer_appearance(id, a).unwrap();
    doc.move_layer(id, 3, 4).unwrap();
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.layer(id).unwrap().appearance().locks, LOCK_ALL);
}
#[test]
fn translation_retains_off_canvas_pixels_and_brush_writes_signed_local_coordinates() {
    let mut doc = Document::new(128, 64).unwrap();
    paint(&mut doc, 10.5, 20.5, [1., 0., 0., 1.], BrushMode::Paint);
    let before = raw(&doc);
    doc.move_layer(1, 140, 0).unwrap();
    assert_eq!(doc.pixel(10, 20), Pixel::TRANSPARENT);
    doc.move_layer(1, -140, 0).unwrap();
    assert_eq!(raw(&doc), before);
    doc.move_layer(1, 70, 0).unwrap();
    paint(&mut doc, 2.5, 20.5, [0., 1., 0., 1.], BrushMode::Paint);
    assert_eq!(doc.pixel(2, 20).components(), [0., 1., 0., 1.]);
    assert!(doc.layer(1).unwrap().tiles().any(|(c, _)| c.signed_x() < 0));
    doc.undo().unwrap();
    assert_eq!(doc.pixel(2, 20), Pixel::TRANSPARENT);
    doc.redo().unwrap();
    assert_eq!(doc.pixel(2, 20).components(), [0., 1., 0., 1.]);
    doc.move_layer(1, -70, 0).unwrap();
    assert_eq!(doc.pixel(10, 20).components(), [1., 0., 0., 1.]);
    doc.take_dirty();
    doc.begin_stroke(Brush::default(), InputPoint::new(63., 20., 1.))
        .unwrap();
    doc.cancel_stroke().unwrap();
    doc.move_layer(1, 5, 3).unwrap();
    doc.take_dirty();
    doc.begin_stroke(Brush::default(), InputPoint::new(63., 20., 1.))
        .unwrap();
    doc.cancel_stroke().unwrap();
    assert!(doc.take_dirty().all);
}
#[test]
fn fill_and_opacity_are_independent_validated_and_undoable() {
    let mut doc = Document::new(64, 64).unwrap();
    paint(&mut doc, 10.5, 10.5, [1., 0., 0., 1.], BrushMode::Paint);
    let mut a = doc.layer(1).unwrap().appearance();
    a.fill = 0.4;
    doc.set_layer_appearance(1, a).unwrap();
    doc.set_layer_properties(
        1,
        LayerProperties {
            visible: true,
            opacity: 0.5,
        },
    )
    .unwrap();
    assert_eq!(doc.pixel(10, 10).components(), [0.2, 0., 0., 0.2]);
    let rev = doc.revision();
    doc.set_layer_appearance(1, a).unwrap();
    assert_eq!(rev, doc.revision());
    for fill in [-1., 1.1, f32::NAN, f32::INFINITY] {
        let mut bad = a;
        bad.fill = fill;
        assert!(doc.set_layer_appearance(1, bad).is_err());
    }
    doc.undo().unwrap();
    assert_eq!(doc.pixel(10, 10).components(), [0.4, 0., 0., 0.4]);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(10, 10).components(), [1., 0., 0., 1.]);
}
#[test]
fn standard_mix_reference_values_and_transparent_premultiplied_boundaries() {
    let s = Pixel::from_straight([0.8, 0.4, 0.2, 1.]).unwrap();
    let b = Pixel::from_straight([0.25, 0.5, 0.75, 1.]).unwrap();
    for (mode, want) in [
        (BlendMode::Multiply, [0.2, 0.2, 0.15]),
        (BlendMode::Screen, [0.85, 0.7, 0.8]),
        (BlendMode::Overlay, [0.4, 0.4, 0.6]),
        (BlendMode::Difference, [0.55, 0.1, 0.55]),
        (BlendMode::Subtract, [0., 0.1, 0.55]),
    ] {
        let got = blend_pixel(s, b, mode, 0, 0, 1).components();
        for i in 0..3 {
            assert!((got[i] - want[i]).abs() < 1e-6, "{mode:?}: {got:?}");
        }
    }
    for id in 0..27 {
        let mode = BlendMode::from_id(id).unwrap();
        for sa in [0., 0.2, 1.] {
            for ba in [0., 0.5, 1.] {
                for color in [[0., 0., 0.], [1., 1., 1.], [0.9, 0.1, 0.5]] {
                    let src = Pixel::from_straight([color[0], color[1], color[2], sa]).unwrap();
                    let dest = Pixel::from_straight([0.2, 0.7, 0.4, ba]).unwrap();
                    let out = blend_pixel(src, dest, mode, 63, 64, 7).components();
                    assert!(out.iter().all(|v| v.is_finite()));
                    assert!((0. ..=1.).contains(&out[3]));
                    assert!(out[..3].iter().all(|v| *v >= 0. && *v <= out[3]));
                    if sa == 0. {
                        assert_eq!(out, dest.components());
                    }
                    if ba == 0. && mode != BlendMode::Dissolve {
                        assert_eq!(out, src.components());
                    }
                }
            }
        }
    }
    assert!(BlendMode::from_id(27).is_err());
}
#[test]
fn cold_spilled_deleted_layer_restores_signed_tiles_and_full_appearance() {
    let tiles = (0..6)
        .map(|n| {
            (
                TileCoord { x: n % 3, y: n / 3 },
                Tile::from_pixels(
                    (0..4096)
                        .map(|i| {
                            Pixel::from_straight([
                                i as f32 / 4096.,
                                ((i * 17 + n * 23) % 4096) as f32 / 4096.,
                                0.3,
                                1.,
                            ])
                            .unwrap()
                        })
                        .collect(),
                )
                .unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut doc = Document::from_import(
        192,
        128,
        vec![
            ImportedLayer {
                name: "bottom".into(),
                properties: LayerProperties::default(),
                tiles: BTreeMap::new(),
            },
            ImportedLayer {
                name: "top".into(),
                properties: LayerProperties::default(),
                tiles,
            },
        ],
        1,
        DocumentOptions {
            max_resident_tiles: 1,
            max_history_bytes: 2 * TILE_BYTES + 256,
            ..Default::default()
        },
    )
    .unwrap();
    let id = doc.active_layer();
    let mut a = doc.layer(id).unwrap().appearance();
    a.fill = 0.7;
    a.blend = BlendMode::SoftLight;
    a.locks = LOCK_TRANSPARENCY;
    a.offset_x = -65;
    a.offset_y = 3;
    doc.set_layer_appearance(id, a).unwrap();
    let before = raw(&doc);
    doc.remove_layer(id).unwrap();
    assert!(doc.history_disk_bytes() > 0);
    doc.undo().unwrap();
    doc.set_active_layer(id).unwrap();
    assert_eq!(doc.layer(id).unwrap().appearance(), a);
    assert_eq!(raw(&doc), before);
    doc.redo().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.layer(id).unwrap().appearance(), a);
}
