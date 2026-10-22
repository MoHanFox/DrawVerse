use super::*;

fn setup(context: usize, zero_opacity: bool, linked: bool, selected: bool) -> (Document, LayerId) {
    let mut d = Document::new(128, 96).unwrap();
    let id = d.active_layer();
    let pixels = (0..4096)
        .map(|i| {
            Pixel::from_straight([(i % 64) as f32 / 63., (i / 64) as f32 / 63., 0.2, 0.7]).unwrap()
        })
        .collect();
    let tile = Arc::new(Tile::from_pixels(pixels).unwrap());
    for (x, y) in [(-1, 0), (0, 0), (1, 0), (0, 1), (1, 1)] {
        d.layers[0]
            .tiles
            .insert(TileCoord::from_signed(x, y), tile.clone());
    }
    d.move_layer(id, 7, -3).unwrap();
    let owner = if context == 0 {
        id
    } else {
        let group = d.group_layer(id, "identity masked ancestor").unwrap();
        let at = d.layer_index(group).unwrap();
        d.layers[at].appearance.blend = if context == 1 {
            crate::BlendMode::Normal
        } else {
            crate::BlendMode::PassThrough
        };
        group
    };
    let mask = d.add_mask(owner).unwrap();
    let at = d.layer_index(mask).unwrap();
    if zero_opacity {
        d.layers[at]
            .tiles
            .insert(TileCoord::from_signed(-1, 0), tile);
        d.layers[at].properties.opacity = 0.;
    }
    d.set_mask_linked(mask, linked).unwrap();
    d.set_active_layer(id).unwrap();
    if selected {
        d.set_selection(
            crate::Selection::default()
                .apply(
                    crate::SelectionShape::polygon(
                        vec![[16., 12.], [82., 16.], [76., 70.], [20., 64.]],
                        true,
                        2.,
                    )
                    .unwrap(),
                    crate::SelectionOperation::Replace,
                    128,
                    96,
                )
                .unwrap(),
        )
        .unwrap();
    }
    (d, id)
}

fn equal(a: &Document, b: &Document) {
    assert_eq!(a.layers.len(), b.layers.len());
    for (a, b) in a.layers.iter().zip(&b.layers) {
        assert_eq!(
            (a.id, a.parent, a.mask, a.mask_linked, a.white),
            (b.id, b.parent, b.mask, b.mask_linked, b.white)
        );
        assert_eq!(a.properties, b.properties);
        assert_eq!(a.appearance, b.appearance);
        assert_eq!(a.tiles.len(), b.tiles.len());
        for (key, tile) in &a.tiles {
            assert_eq!(
                *tile.try_pixels().unwrap(),
                *b.tiles[key].try_pixels().unwrap()
            );
        }
    }
    assert_eq!(a.selection.steps(), b.selection.steps());
    assert_eq!(a.history_depth(), b.history_depth());
    for y in 0..96 {
        assert_eq!(
            a.snapshot().read_row(0, y, 128).unwrap(),
            b.snapshot().read_row(0, y, 128).unwrap()
        );
    }
}

fn readable_source(d: &Document) {
    let preview = d.resident_preview().unwrap();
    for snapshot in [
        &preview.source.image,
        &preview.source.below,
        &preview.source.above,
    ] {
        crate::document::groups::validate(&snapshot.layers).unwrap();
        snapshot.read_row(0, 0, snapshot.width.min(256)).unwrap();
    }
}

#[test]
fn identity_masks_keep_shared_previews_and_exact_final_history() {
    for context in 0..3 {
        for zero in [false, true] {
            for linked in [false, true] {
                for selected in [false, true] {
                    for operation in 0..3 {
                        let (mut a, id) = setup(context, zero, linked, selected);
                        let (mut b, _) = setup(context, zero, linked, selected);
                        let originals = a.layers.clone();
                        match operation {
                            0 => {
                                a.begin_transform_layers_deferred(&[id]).unwrap();
                                b.begin_transform_layers(&[id]).unwrap();
                                assert!(a.transform_deferred(), "transform context={context} zero={zero} linked={linked} selected={selected}");
                                readable_source(&a);
                                let mut geometry = a.transform_geometry().unwrap();
                                geometry.warp = true;
                                geometry.points[5][0] += 8.;
                                geometry.points[10][1] -= 5.;
                                a.update_transform(geometry.clone()).unwrap();
                                b.update_transform(geometry).unwrap();
                            }
                            1 => {
                                a.begin_layer_move(id, true).unwrap();
                                b.begin_layer_move(id, false).unwrap();
                                assert!(a.layer_move_deferred());
                                readable_source(&a);
                                a.update_layer_move(7, -3).unwrap();
                                b.update_layer_move(7, -3).unwrap();
                            }
                            _ => {
                                a.begin_liquify_deferred().unwrap();
                                b.begin_liquify().unwrap();
                                assert!(a.liquify_deferred());
                                readable_source(&a);
                                for d in [&mut a, &mut b] {
                                    d.set_liquify_focus(true).unwrap();
                                    d.begin_liquify_stroke().unwrap();
                                    d.liquify_dab(LiquifyDab {
                                        tool: 0,
                                        radius: 30.,
                                        strength: 0.7,
                                        density: 0.5,
                                        speed: 1.,
                                        pressure_enabled: false,
                                        from: InputPoint::new(32., 32., 1.),
                                        to: InputPoint::new(39., 29., 1.),
                                        elapsed_ms: 16.,
                                    })
                                    .unwrap();
                                    d.end_liquify_stroke().unwrap();
                                }
                            }
                        }
                        readable_source(&a);
                        for (old, current) in originals.iter().zip(&a.layers) {
                            assert_eq!(old.parent, current.parent);
                            for (key, tile) in &old.tiles {
                                assert!(Arc::ptr_eq(tile, &current.tiles[key]));
                            }
                        }
                        for d in [&mut a, &mut b] {
                            match operation {
                                0 => {
                                    d.commit_transform().unwrap();
                                }
                                1 => {
                                    d.commit_layer_move(7, -3).unwrap();
                                }
                                _ => {
                                    d.commit_liquify().unwrap();
                                }
                            }
                        }
                        equal(&a, &b);
                        a.undo().unwrap();
                        b.undo().unwrap();
                        equal(&a, &b);
                        a.redo().unwrap();
                        b.redo().unwrap();
                        equal(&a, &b);
                    }
                }
            }
        }
    }
}

#[test]
fn hidden_or_fill_zero_masks_with_coverage_still_use_the_safe_fallback() {
    for kind in 0..2 {
        let (mut d, id) = setup(1, true, false, false);
        let parent = d.layer(id).unwrap().parent;
        let mask = d.mask_for(parent).unwrap().id;
        let at = d.layer_index(mask).unwrap();
        d.layers[at].properties.opacity = 1.;
        match kind {
            0 => d.layers[at].properties.visible = false,
            _ => d.layers[at].appearance.fill = 0.,
        }
        d.begin_transform_layers_deferred(&[id]).unwrap();
        assert!(!d.transform_deferred());
        d.cancel_transform().unwrap();
        d.begin_layer_move(id, true).unwrap();
        assert!(!d.layer_move_deferred());
        d.cancel_layer_move().unwrap();
        d.begin_liquify_deferred().unwrap();
        assert!(!d.liquify_deferred());
        d.cancel_liquify().unwrap();
    }
}

#[test]
fn cancelling_identity_mask_previews_keeps_the_original_document() {
    for operation in 0..3 {
        let (mut d, id) = setup(1, true, false, true);
        let (original, _) = setup(1, true, false, true);
        match operation {
            0 => {
                d.begin_transform_layers_deferred(&[id]).unwrap();
                let mut geometry = d.transform_geometry().unwrap();
                geometry.warp = true;
                geometry.points[5][0] += 8.;
                d.update_transform(geometry).unwrap();
                d.cancel_transform().unwrap();
            }
            1 => {
                d.begin_layer_move(id, true).unwrap();
                d.update_layer_move(7, -3).unwrap();
                d.cancel_layer_move().unwrap();
            }
            _ => {
                d.begin_liquify_deferred().unwrap();
                d.begin_liquify_stroke().unwrap();
                d.liquify_dab(LiquifyDab {
                    tool: 0,
                    radius: 30.,
                    strength: 0.7,
                    density: 0.5,
                    speed: 1.,
                    pressure_enabled: false,
                    from: InputPoint::new(32., 32., 1.),
                    to: InputPoint::new(39., 29., 1.),
                    elapsed_ms: 16.,
                })
                .unwrap();
                d.end_liquify_stroke().unwrap();
                d.cancel_liquify().unwrap();
            }
        }
        equal(&d, &original);
    }
}

#[test]
#[ignore = "manual core-only preview workload; not an end-to-end UI benchmark"]
fn identity_mask_transform_preview_workload() {
    use std::time::Instant;
    let make = || {
        let mut d = Document::new(1024, 1024).unwrap();
        let id = d.active_layer();
        let tile = Arc::new(
            Tile::from_pixels(vec![
                Pixel::from_straight([0.8, 0.2, 0.1, 1.]).unwrap();
                4096
            ])
            .unwrap(),
        );
        for y in 0..16 {
            for x in 0..16 {
                d.layers[0]
                    .tiles
                    .insert(TileCoord::from_signed(x, y), tile.clone());
            }
        }
        let group = d.group_layer(id, "empty ancestor mask").unwrap();
        d.add_mask(group).unwrap();
        d.set_active_layer(id).unwrap();
        (d, id)
    };
    let mut outputs = Vec::new();
    for deferred in [false, true] {
        let (mut d, id) = make();
        let start = Instant::now();
        if deferred {
            d.begin_transform_layers_deferred(&[id]).unwrap();
        } else {
            d.begin_transform_layers(&[id]).unwrap();
        }
        let prepare = start.elapsed();
        assert_eq!(d.transform_deferred(), deferred);
        let initial = d.transform_geometry().unwrap();
        let start = Instant::now();
        for i in 1..=6 {
            let mut geometry = initial.clone();
            geometry.warp = true;
            geometry.points[5][0] += 4. * i as f64;
            d.update_transform(geometry).unwrap();
        }
        let updates = start.elapsed();
        let start = Instant::now();
        d.commit_transform().unwrap();
        eprintln!("identity mask deferred={deferred} prepare={prepare:?} six updates={updates:?} commit={:?}", start.elapsed());
        outputs.push(d);
    }
    equal(&outputs[0], &outputs[1]);
}
