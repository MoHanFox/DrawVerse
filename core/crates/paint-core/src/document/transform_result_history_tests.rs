use super::liquify_result_history_tests::document;
use super::*;

fn finish(doc: &mut Document, kind: u8) {
    match kind {
        0 => {
            doc.begin_transform().unwrap();
            let mut geometry = doc.transform_geometry().unwrap();
            for point in &mut geometry.points {
                point[0] += 8.;
                point[1] += 3.;
            }
            doc.update_transform(geometry).unwrap();
            assert!(doc.commit_transform().unwrap());
        }
        1 => doc.flip_image(true).unwrap(),
        _ => assert!(doc
            .bucket_fill(crate::BucketFill {
                x: 64,
                y: 64,
                color: Pixel::WHITE,
                opacity: 1.,
                tolerance: 255,
                antialias: false,
                contiguous: false,
                all_layers: false
            })
            .unwrap()),
    }
    assert!(matches!(
        doc.history.undo.back(),
        Some(StoredCommand::Paged(..))
    ));
    assert_eq!(doc.history_disk_bytes(), 0);
    assert!(doc.history_bytes() <= doc.options.max_history_bytes);
}
fn same_tiles(a: &BTreeMap<TileCoord, Arc<Tile>>, b: &BTreeMap<TileCoord, Arc<Tile>>) {
    assert_eq!(a.len(), b.len());
    for (key, tile) in a {
        assert!(Arc::ptr_eq(tile, &b[key]));
    }
}
#[test]
#[ignore = "manual Release comparison of cold flip history"]
fn cold_flip_history_selection_benchmark() {
    use std::time::Instant;
    let mut doc = Document::with_options(
        1024,
        1024,
        DocumentOptions {
            max_resident_tiles: 16,
            max_history_bytes: 1024 * 1024,
            ..Default::default()
        },
    )
    .unwrap();
    for y in 0..16 {
        for x in 0..16 {
            let tile = Arc::new(
                Tile::from_pixels(
                    (0..4096)
                        .map(|i| {
                            Pixel::from_straight([
                                ((i * 17 + x * 31 + y * 71) % 4096) as f32 / 4096.,
                                (i % 641) as f32 / 641.,
                                0.3,
                                1.,
                            ])
                            .unwrap()
                        })
                        .collect(),
                )
                .unwrap(),
            );
            doc.pool.attach(&tile).unwrap();
            doc.layers[0].tiles.insert(TileCoord { x, y }, tile);
        }
    }
    doc.flip_image(true).unwrap();
    let mut times = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        for target in [0, 1, 0, 1, 0, 1] {
            doc.select_history(target).unwrap();
        }
        times.push(start.elapsed().as_secs_f64() * 1000.);
    }
    times.sort_by(f64::total_cmp);
    eprintln!(
        "cold flip six history selections median {:.3}ms, resident {}, page reads {}",
        times[2],
        doc.storage_stats().resident_tiles,
        doc.storage_stats().page_reads
    );
    assert_eq!(doc.history_depth(), (1, 0));
    assert!(doc.storage_stats().resident_tiles <= 16);
}
#[test]
fn completed_transform_flip_and_bucket_reuse_results_and_preserve_selection() {
    for kind in 0..3 {
        for resident in [1, 128] {
            let mut doc = document(resident);
            doc.set_selection(crate::Selection::all(128, 128)).unwrap();
            let depth = doc.history_depth().0;
            let before = doc.layers[0].tiles.clone();
            let before_selection = doc.selection.clone();
            finish(&mut doc, kind);
            let after = doc.layers[0].tiles.clone();
            let after_selection = doc.selection.clone();
            let sample = doc.try_pixel(64, 64).unwrap();
            let stats = doc.storage_stats();
            for _ in 0..4 {
                doc.select_history(depth).unwrap();
                same_tiles(&doc.layers[0].tiles, &before);
                assert_eq!(doc.selection, before_selection);
                doc.select_history(depth + 1).unwrap();
                same_tiles(&doc.layers[0].tiles, &after);
                assert_eq!(doc.selection, after_selection);
            }
            assert_eq!(doc.try_pixel(64, 64).unwrap(), sample);
            let final_stats = doc.storage_stats();
            assert_eq!(stats.registered_tiles, final_stats.registered_tiles);
            if resident == 128 {
                assert_eq!(stats.page_reads, final_stats.page_reads);
            } else {
                assert!(final_stats.page_reads > stats.page_reads);
                doc.trim_storage().unwrap();
                assert!(doc.storage_stats().resident_tiles <= 1);
            }
            doc.undo().unwrap();
            doc.begin_stroke(Brush::default(), InputPoint::new(8., 8., 1.))
                .unwrap();
            doc.end_stroke().unwrap();
            assert_eq!(doc.history_depth().1, 0);
        }
    }
}
#[test]
fn corrupt_transform_flip_and_bucket_results_leave_history_and_pixels_unchanged() {
    for kind in 0..3 {
        for redo in [false, true] {
            let mut doc = document(1);
            finish(&mut doc, kind);
            if redo {
                doc.undo().unwrap();
            }
            let tiles = doc.layers[0].tiles.clone();
            let selection = doc.selection.clone();
            let revision = doc.revision;
            let dirty = doc.dirty.clone();
            let depth = doc.history_depth();
            doc.pool.corrupt_for_test();
            assert!(if redo { doc.redo() } else { doc.undo() }.is_err());
            same_tiles(&doc.layers[0].tiles, &tiles);
            assert_eq!(doc.selection, selection);
            assert_eq!(doc.revision, revision);
            assert_eq!(doc.dirty, dirty);
            assert_eq!(doc.history_depth(), depth);
        }
    }
}

#[test]
#[ignore = "manual Release comparison of large transform history"]
fn large_transform_result_history_restore_benchmark() {
    use std::time::Instant;
    let pool = crate::page_pool::PagePool::new(4096, 1 << 30);
    let mut before = Layer::new(1, "before".into());
    let mut after = Layer::new(1, "after".into());
    for x in 0..600 {
        for (side, layer) in [&mut before, &mut after].into_iter().enumerate() {
            let tile = Arc::new(
                Tile::from_pixels(
                    (0..4096)
                        .map(|i| {
                            Pixel::from_straight([
                                ((i * 17 + x * 41 + side * 97) % 4096) as f32 / 4096.,
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
            layer.tiles.insert(TileCoord { x: x as u32, y: 0 }, tile);
        }
    }
    let command = Command::LayerEdit {
        before: vec![before],
        after: vec![after],
        selection: Some((
            crate::Selection::all(38400, 64),
            crate::Selection::all(38400, 64),
        )),
    };
    let budget = 64 * 1024 * 1024;
    let start = Instant::now();
    let encoded = StoredCommand::prepare_in(&command, budget, pool.storage()).unwrap();
    let encode_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    let paged = StoredCommand::prepare_result_in(&command, budget, &pool).unwrap();
    let retain_ms = start.elapsed().as_secs_f64() * 1000.;
    assert!(matches!(paged, StoredCommand::Paged(..)));
    let mut old = Vec::new();
    let mut new = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        drop(encoded.materialize_in(Some(&pool)).unwrap());
        old.push(start.elapsed().as_secs_f64() * 1000.);
        let start = Instant::now();
        let retained = paged.materialize_in(Some(&pool)).unwrap();
        new.push(start.elapsed().as_secs_f64() * 1000.);
        if let (Command::LayerEdit { after: a, .. }, Command::LayerEdit { after: b, .. }) =
            (&command, &retained)
        {
            same_tiles(&a[0].tiles, &b[0].tiles);
        } else {
            panic!("layer edit");
        }
    }
    old.sort_by(f64::total_cmp);
    new.sort_by(f64::total_cmp);
    eprintln!("600 before/after transform tiles / 75MiB: prepare encoded {encode_ms:.3}ms vs paged {retain_ms:.3}ms; five restores median {:.3}ms vs {:.3}ms",old[2],new[2]);
}

#[test]
fn rasterize_history_keeps_placed_source_and_managed_tile_identities() {
    for resident in [1, 128] {
        let mut doc = document(resident);
        let id = doc
            .place_image(doc.layers[0].clone(), 128, 128, "placed".into())
            .unwrap();
        doc.commit_transform().unwrap();
        let source = doc.layer(id).unwrap().placed.clone().unwrap();
        let tiles = doc.layer(id).unwrap().tiles.clone();
        let sample = doc.try_pixel(64, 64).unwrap();
        let disk = doc.history_disk_bytes();
        assert!(doc.rasterize_image(id).unwrap());
        assert!(matches!(
            doc.history.undo.back(),
            Some(StoredCommand::Paged(..))
        ));
        assert_eq!(doc.history_disk_bytes(), disk);
        for _ in 0..4 {
            doc.undo().unwrap();
            assert!(Arc::ptr_eq(
                doc.layer(id).unwrap().placed.as_ref().unwrap(),
                &source
            ));
            same_tiles(&doc.layer(id).unwrap().tiles, &tiles);
            doc.redo().unwrap();
            assert!(doc.layer(id).unwrap().placed.is_none());
            same_tiles(&doc.layer(id).unwrap().tiles, &tiles);
        }
        assert_eq!(doc.try_pixel(64, 64).unwrap(), sample);
        doc.trim_storage().unwrap();
        assert!(doc.storage_stats().resident_tiles <= resident);
    }
}
