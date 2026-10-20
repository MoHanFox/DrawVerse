use super::*;

fn document(resident: usize) -> Document {
    let tiles = (0..4)
        .map(|index| {
            let pixels = (0..4096)
                .map(|i| {
                    Pixel::from_straight([
                        ((i * 17 + index * 41) % 4096) as f32 / 4096.,
                        i as f32 / 4096.,
                        0.3,
                        1.,
                    ])
                    .unwrap()
                })
                .collect();
            (
                TileCoord {
                    x: index % 2,
                    y: index / 2,
                },
                Tile::from_pixels(pixels).unwrap(),
            )
        })
        .collect();
    Document::from_import(
        128,
        128,
        vec![crate::ImportedLayer {
            name: "noise".into(),
            properties: LayerProperties::default(),
            tiles,
        }],
        0,
        DocumentOptions {
            max_history_bytes: 2 * TILE_BYTES + 256,
            max_resident_tiles: resident,
            ..Default::default()
        },
    )
    .unwrap()
}
fn liquify(doc: &mut Document) {
    doc.begin_liquify().unwrap();
    doc.begin_liquify_stroke().unwrap();
    assert!(doc
        .liquify_dab(LiquifyDab {
            tool: 0,
            radius: 96.,
            strength: 0.5,
            density: 0.5,
            speed: 1.,
            pressure_enabled: false,
            from: InputPoint::new(56., 56., 1.),
            to: InputPoint::new(76., 70., 1.),
            elapsed_ms: 100.,
        })
        .unwrap());
    doc.end_liquify_stroke().unwrap();
    assert!(doc.commit_liquify().unwrap());
    assert!(matches!(
        doc.history.undo.back(),
        Some(StoredCommand::Paged(..))
    ));
    assert!(doc.history_bytes() <= doc.options.max_history_bytes);
    assert_eq!(doc.history_disk_bytes(), 0);
}
fn same_tiles(a: &BTreeMap<TileCoord, Arc<Tile>>, b: &BTreeMap<TileCoord, Arc<Tile>>) {
    assert_eq!(a.len(), b.len());
    for (key, tile) in a {
        assert!(Arc::ptr_eq(tile, &b[key]));
    }
}
#[test]
fn completed_liquify_reuses_exact_result_identities_and_keeps_redo_branching() {
    let mut doc = document(128);
    let before = doc.layers[0].tiles.clone();
    liquify(&mut doc);
    let after = doc.layers[0].tiles.clone();
    let stats = doc.storage_stats();
    for _ in 0..4 {
        assert!(doc.select_history(0).unwrap());
        same_tiles(&doc.layers[0].tiles, &before);
        assert!(doc.select_history(1).unwrap());
        same_tiles(&doc.layers[0].tiles, &after);
    }
    assert_eq!(doc.storage_stats().page_reads, stats.page_reads);
    assert_eq!(doc.storage_stats().registered_tiles, stats.registered_tiles);
    doc.undo().unwrap();
    doc.begin_stroke(Brush::default(), InputPoint::new(20., 20., 1.))
        .unwrap();
    doc.end_stroke().unwrap();
    assert_eq!(doc.history_depth().1, 0);
}
#[test]
fn completed_liquify_cold_results_stay_bounded_and_reload_without_new_identities() {
    let mut doc = document(1);
    let before = doc.layers[0].tiles.clone();
    liquify(&mut doc);
    let after = doc.layers[0].tiles.clone();
    let sample = doc.try_pixel(64, 64).unwrap();
    let reads = doc.storage_stats().page_reads;
    doc.undo().unwrap();
    same_tiles(&doc.layers[0].tiles, &before);
    doc.redo().unwrap();
    same_tiles(&doc.layers[0].tiles, &after);
    assert_eq!(doc.try_pixel(64, 64).unwrap(), sample);
    doc.trim_storage().unwrap();
    assert!(doc.storage_stats().page_reads > reads);
    assert!(doc.storage_stats().resident_tiles <= 1);
}
#[test]
fn corrupt_paged_result_rolls_back_history_pixels_and_revision() {
    let mut doc = document(1);
    liquify(&mut doc);
    let tiles = doc.layers[0].tiles.clone();
    let revision = doc.revision;
    let dirty = doc.dirty.clone();
    let depth = doc.history_depth();
    doc.pool.corrupt_for_test();
    assert!(doc.select_history(0).is_err());
    same_tiles(&doc.layers[0].tiles, &tiles);
    assert_eq!(doc.revision, revision);
    assert_eq!(doc.dirty, dirty);
    assert_eq!(doc.history_depth(), depth);
}

#[test]
#[ignore = "manual Release comparison of repeated history materialization"]
fn large_result_history_restore_benchmark() {
    use std::time::Instant;
    let pool = crate::page_pool::PagePool::new(4096, 1 << 30);
    let changes = (0..600)
        .map(|x| {
            let make = |side: usize| {
                let tile = Arc::new(
                    Tile::from_pixels(
                        (0..4096)
                            .map(|i| {
                                Pixel::from_straight([
                                    ((i * 17 + x as usize * 41 + side * 97) % 4096) as f32 / 4096.,
                                    i as f32 / 4096.,
                                    0.3,
                                    1.,
                                ])
                                .unwrap()
                            })
                            .collect(),
                    )
                    .unwrap(),
                );
                pool.attach(&tile).unwrap();
                tile
            };
            TileChange {
                coord: TileCoord { x, y: 0 },
                before: Some(make(0)),
                after: Some(make(1)),
            }
        })
        .collect();
    let command = Command::Stroke { layer: 1, changes };
    let budget = 64 * 1024 * 1024;
    let start = Instant::now();
    let encoded = StoredCommand::prepare_in(&command, budget, pool.storage()).unwrap();
    let encode_time = start.elapsed();
    let start = Instant::now();
    let paged = StoredCommand::prepare_result_in(&command, budget, &pool).unwrap();
    let retain_time = start.elapsed();
    assert!(matches!(encoded, StoredCommand::Encoded(..)));
    assert!(matches!(paged, StoredCommand::Paged(..)));
    let mut old = Vec::new();
    let mut new = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        let decoded = encoded.materialize_in(Some(&pool)).unwrap();
        old.push(start.elapsed().as_secs_f64() * 1000.);
        if let Command::Stroke { changes, .. } = &decoded {
            assert_eq!(changes.len(), 600);
        }
        drop(decoded);
        let start = Instant::now();
        let retained = paged.materialize_in(Some(&pool)).unwrap();
        new.push(start.elapsed().as_secs_f64() * 1000.);
        if let (Command::Stroke { changes: a, .. }, Command::Stroke { changes: b, .. }) =
            (&command, &retained)
        {
            for (a, b) in a.iter().zip(b) {
                assert!(Arc::ptr_eq(
                    a.after.as_ref().unwrap(),
                    b.after.as_ref().unwrap()
                ));
            }
        }
    }
    old.sort_by(f64::total_cmp);
    new.sort_by(f64::total_cmp);
    eprintln!("600 tile changes / 75MiB logical pixels: prepare encoded {:.3}ms vs paged {:.3}ms; five restores median {:.3}ms vs {:.3}ms", encode_time.as_secs_f64()*1000.,retain_time.as_secs_f64()*1000.,old[2],new[2]);
}
