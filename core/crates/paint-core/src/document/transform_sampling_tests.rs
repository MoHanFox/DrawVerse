use super::*;

fn source() -> Layer {
    let mut layer = Layer::new(1, "sampling".into());
    layer.white = (133, 91);
    for x in -40..40 {
        let pixels = (0..4096)
            .map(|i| {
                Pixel::from_straight([
                    (i % 64) as f32 / 63.,
                    (i / 64) as f32 / 63.,
                    (x + 40) as f32 / 79.,
                    0.7,
                ])
                .unwrap()
            })
            .collect();
        layer.tiles.insert(
            TileCoord::from_signed(x, -1),
            Arc::new(Tile::from_pixels(pixels).unwrap()),
        );
    }
    layer
}

fn selection() -> crate::Selection {
    crate::Selection::default()
        .apply(
            crate::SelectionShape::polygon(
                vec![[12., 8.], [190., 21.], [141., 151.], [4., 132.]],
                true,
                5.,
            )
            .unwrap(),
            crate::SelectionOperation::Replace,
            256,
            192,
        )
        .unwrap()
}

fn reference(layer: &Layer, selection: Option<&crate::Selection>, x: i64, y: i64) -> Pixel {
    let pixel = layer
        .tiles
        .get(&TileCoord::from_signed(
            x.div_euclid(64) as i32,
            y.div_euclid(64) as i32,
        ))
        .map_or_else(
            || layer.default_pixel(x, y),
            |tile| tile.try_pixels().unwrap()[(y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize],
        );
    pixel.scaled(selection.map_or(1., |s| {
        s.coverage(x as f64 + 0.5 - 13., y as f64 + 0.5 + 9.)
    }))
}

#[test]
fn sampling_matches_independent_pixels_across_eviction_and_implicit_edges() {
    let layer = source();
    let selection = selection();
    for selected in [false, true] {
        let mut sampler = SourceSampler::new(&layer);
        sampler.selection = selected.then_some(&selection);
        sampler.offset = [-13., 9.];
        // All of these points share one missing tile but cross both white edges.
        for (x, y) in [(128, 90), (133, 90), (128, 91), (132, 90), (129, 92)] {
            assert_eq!(
                sampler.pixel(x, y).unwrap(),
                reference(&layer, sampler.selection, x, y)
            );
        }
        for round in 0..3 {
            for tx in -42..42 {
                let tx = if round == 1 { -tx } else { tx };
                for (dx, y) in [(0, -64), (63, -1), (64, -2), (1, 0), (63, 90), (64, 91)] {
                    let x = tx * 64 + dx;
                    assert_eq!(
                        sampler.pixel(x, y).unwrap(),
                        reference(&layer, sampler.selection, x, y),
                        "selection={selected} round={round} at {x},{y}"
                    );
                }
            }
        }
    }
}

#[test]
fn failed_sampling_does_not_replace_cached_pixels_or_hide_the_error() {
    let mut layer = source();
    let broken = Arc::new(Tile::default());
    let pixels = broken.state.lock().unwrap().resident.take();
    layer
        .tiles
        .insert(TileCoord::from_signed(0, 0), broken.clone());
    let mut sampler = SourceSampler::new(&layer);
    let original = sampler.pixel(-1, -1).unwrap();
    for _ in 0..2 {
        assert!(sampler.pixel(0, 0).is_err());
        assert_eq!(sampler.pixel(-1, -1).unwrap(), original);
    }
    broken.state.lock().unwrap().resident = pixels;
    assert_eq!(sampler.pixel(0, 0).unwrap(), Pixel::TRANSPARENT);
}

#[test]
fn sampling_does_not_retain_page_pins_under_one_tile_residency() {
    let pool = crate::page_pool::PagePool::new(1, 16 * 1024 * 1024);
    let layer = source();
    for tile in layer.tiles.values() {
        pool.attach(tile).unwrap();
    }
    let mut sampler = SourceSampler::new(&layer);
    for tx in -40..40 {
        assert_eq!(
            sampler.pixel(tx * 64, -1).unwrap(),
            reference(&layer, None, tx * 64, -1)
        );
        pool.trim().unwrap();
        assert!(pool.stats().resident_tiles <= 1);
    }
}

#[test]
#[ignore = "manual core CPU fallback workload; not UI latency"]
fn complex_mask_sampling_workload() {
    use std::time::Instant;
    for selected in [false, true] {
        for warp in [false, true] {
            let mut d = Document::new(1024, 1024).unwrap();
            let id = d.active_layer();
            let seed = source();
            let tile = seed.tiles.values().next().unwrap().clone();
            for y in 0..16 {
                for x in 0..16 {
                    d.layers[0]
                        .tiles
                        .insert(TileCoord::from_signed(x, y), tile.clone());
                }
            }
            let owner = if selected {
                d.group_layer(id, "masked ancestor").unwrap()
            } else {
                id
            };
            let mask = d.add_mask(owner).unwrap();
            let at = d.layer_index(mask).unwrap();
            d.layers[at]
                .tiles
                .insert(TileCoord::from_signed(4, 4), tile);
            d.set_mask_linked(mask, false).unwrap();
            d.set_active_layer(id).unwrap();
            if selected {
                d.set_selection(
                    crate::Selection::default()
                        .apply(
                            crate::SelectionShape::polygon(
                                vec![[256., 256.], [768., 260.], [760., 768.], [260., 760.]],
                                true,
                                3.,
                            )
                            .unwrap(),
                            crate::SelectionOperation::Replace,
                            1024,
                            1024,
                        )
                        .unwrap(),
                )
                .unwrap();
            }
            d.begin_transform_layers_deferred(&[id]).unwrap();
            assert!(!d.transform_deferred());
            let initial = d.transform_geometry().unwrap();
            let start = Instant::now();
            for step in 1..=2 {
                let mut geometry = initial.clone();
                geometry.warp = warp;
                for p in &mut geometry.points {
                    p[0] += step as f64 * 9.;
                }
                if warp {
                    geometry.points[5][1] += 17.;
                    geometry.points[10][0] -= 11.;
                }
                d.update_transform(geometry).unwrap();
            }
            let elapsed = start.elapsed();
            d.commit_transform().unwrap();
            let mut hash = 0xcbf29ce484222325u64;
            for y in 0..1024 {
                for p in d.snapshot().read_row(0, y, 1024).unwrap() {
                    for component in p.components() {
                        hash ^= u64::from(component.to_bits());
                        hash = hash.wrapping_mul(0x100000001b3);
                    }
                }
            }
            eprintln!("selected={selected} warp={warp} two updates={elapsed:?} digest={hash:016x}");
        }
    }
}
