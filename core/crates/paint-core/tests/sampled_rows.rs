use paint_core::*;
use std::collections::BTreeMap;

fn imported(name: &str, index: u32) -> ImportedLayer {
    let tiles = (0..3)
        .map(|x| {
            let pixels = (0..4096)
                .map(|i| {
                    Pixel::from_straight([
                        ((i * 13 + index) % 257) as f32 / 256.,
                        ((i * 17 + x) % 251) as f32 / 250.,
                        index as f32 / 4.,
                        ((i + x * 3) % 11) as f32 / 10.,
                    ])
                    .unwrap()
                })
                .collect();
            (TileCoord { x, y: 0 }, Tile::from_pixels(pixels).unwrap())
        })
        .collect();
    ImportedLayer {
        name: name.into(),
        properties: LayerProperties::default(),
        tiles,
    }
}

#[test]
fn sampled_rows_reuse_all_blends_clipping_groups_masks_and_canvas_coordinates() {
    // Flat, isolated/clipped and pass-through/clipped paths all share the same
    // sampled coordinate abstraction. Dissolve must retain the original x/y.
    for path in 0..3 {
        let mut d = Document::from_import(
            192,
            64,
            vec![
                imported("backdrop", 1),
                imported("base", 2),
                imported("top", 3),
            ],
            2,
            DocumentOptions::default(),
        )
        .unwrap();
        let top = 3;
        if path != 0 {
            d.set_layer_clipping(top, true).unwrap();
            let group = d.group_layers(&[2, top], "group").unwrap();
            let mut a = d.layer(group).unwrap().appearance();
            a.blend = if path == 1 {
                BlendMode::Normal
            } else {
                BlendMode::PassThrough
            };
            a.fill = 0.7;
            d.set_layer_appearance(group, a).unwrap();
            d.set_layer_properties(
                group,
                LayerProperties {
                    visible: true,
                    opacity: 0.6,
                },
            )
            .unwrap();
            d.add_mask(group).unwrap();
            d.begin_stroke(
                Brush {
                    radius: 19.,
                    color: Pixel::from_straight([0., 0., 0., 1.]).unwrap(),
                    ..Default::default()
                },
                InputPoint::new(63., 32., 1.),
            )
            .unwrap();
            d.end_stroke().unwrap();
        }
        let xs = [0, 0, 1, 17, 62, 63, 64, 65, 126, 127, 128, 129, 191];
        for mode in 0..27 {
            let mut a = d.layer(top).unwrap().appearance();
            a.blend = BlendMode::from_id(mode).unwrap();
            a.offset_x = if mode % 2 == 0 { -17 } else { 23 };
            a.offset_y = if mode % 2 == 0 { 3 } else { -5 };
            a.fill = 0.8;
            a.dissolve_seed = 47;
            d.set_layer_appearance(top, a).unwrap();
            let snapshot = d.snapshot();
            for y in [0, 2, 3, 31, 58, 63] {
                let dense = snapshot.read_row(0, y, 192).unwrap();
                let expected: Vec<_> = xs.iter().map(|x| dense[*x as usize]).collect();
                assert_eq!(
                    snapshot.read_sampled_row(&xs, y).unwrap(),
                    expected,
                    "path={path} mode={mode} y={y}"
                );
            }
        }
    }
}

#[test]
fn sampled_rows_validate_empty_duplicates_bounds_and_order() {
    let d = Document::new(100, 64).unwrap();
    let s = d.snapshot();
    assert!(s.read_sampled_row(&[], 0).unwrap().is_empty());
    assert_eq!(
        s.read_sampled_row(&[0, 0, 99], 63).unwrap(),
        vec![Pixel::TRANSPARENT; 3]
    );
    for (xs, y) in [
        (&[100][..], 0),
        (&[0][..], 64),
        (&[4, 3][..], 0),
        (&[u32::MAX][..], 0),
    ] {
        assert!(s.read_sampled_row(xs, y).is_err());
    }
    assert!(s
        .read_sampled_row(&vec![0; MAX_DIMENSION as usize + 1], 0)
        .is_err());
    let mut white = Document::new(192, 64).unwrap();
    white.initialize_white_background().unwrap();
    white.move_layer(white.active_layer(), 17, -3).unwrap();
    let s = white.snapshot();
    for y in [0, 60, 61, 63] {
        let xs = [0, 16, 17, 63, 64, 127, 128, 191];
        assert_eq!(
            s.read_sampled_row(&xs, y).unwrap(),
            xs.iter().map(|x| s.pixel(*x, y)).collect::<Vec<_>>()
        );
    }
}

#[test]
fn sampled_rows_skip_intervening_cold_pages() {
    let tiles = (0..12)
        .map(|x| {
            (
                TileCoord { x, y: 0 },
                Tile::from_pixels(
                    (0..4096)
                        .map(|i| {
                            Pixel::from_straight([i as f32 / 4096., x as f32 / 12., 0.3, 1.])
                                .unwrap()
                        })
                        .collect(),
                )
                .unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let d = Document::from_import(
        768,
        64,
        vec![ImportedLayer {
            name: "cold".into(),
            properties: LayerProperties::default(),
            tiles,
        }],
        0,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let s = d.snapshot();
    s.trim_storage().unwrap();
    let before = s.storage_stats().page_reads;
    let samples = s.read_sampled_row(&[0, 1, 63, 704, 767], 12).unwrap();
    let sparse_reads = s.storage_stats().page_reads - before;
    assert!(
        sparse_reads <= 2,
        "only two sampled tiles may be read: {sparse_reads}"
    );
    s.trim_storage().unwrap();
    let before = s.storage_stats().page_reads;
    let dense = s.read_row(0, 12, 768).unwrap();
    assert!(s.storage_stats().page_reads - before >= 10);
    assert_eq!(
        samples,
        [0, 1, 63, 704, 767]
            .iter()
            .map(|x| dense[*x])
            .collect::<Vec<_>>()
    );
}
