use paint_core::{BrushMask, BrushOutlineSegment, Error};
use std::collections::BTreeMap;

#[test]
fn display_outline_bounds_complexity_preserves_thin_aspect_and_source() {
    let pixels: Vec<_> = (0..512 * 512)
        .map(|i| if (i / 512 + i % 512) % 2 == 0 { 255 } else { 0 })
        .collect();
    let mask = BrushMask::new(512, 512, pixels.clone()).unwrap();
    let outline = mask.display_outline().unwrap();
    assert!(!outline.is_empty() && outline.len() <= 2048);
    assert_eq!(components(&outline), 1);
    assert_eq!(mask.coverage(), pixels);
    let thin = BrushMask::new(4096, 1, vec![255; 4096]).unwrap();
    let outline = thin.display_outline().unwrap();
    assert!(!outline.is_empty() && outline.len() <= 2048);
    assert_eq!(components(&outline), 1);
    for segment in outline {
        for point in [segment.start, segment.end] {
            assert!((point[1] - 0.5).abs() <= 0.5 / 4096. + 1e-10);
        }
    }
    let simple = BrushMask::new(8, 4, vec![255; 32]).unwrap();
    assert_eq!(simple.display_outline().unwrap(), simple.outline().unwrap());
}

fn components(segments: &[BrushOutlineSegment]) -> usize {
    let key = |p: [f64; 2]| ((p[0] * 1e8).round() as i64, (p[1] * 1e8).round() as i64);
    let mut graph = BTreeMap::<_, Vec<_>>::new();
    for segment in segments {
        graph
            .entry(key(segment.start))
            .or_default()
            .push(key(segment.end));
        graph
            .entry(key(segment.end))
            .or_default()
            .push(key(segment.start));
    }
    assert!(
        graph.values().all(|neighbors| neighbors.len() == 2),
        "every contour must be closed"
    );
    let mut count = 0;
    while let Some((&point, _)) = graph.first_key_value() {
        count += 1;
        let mut pending = vec![point];
        while let Some(point) = pending.pop() {
            if let Some(next) = graph.remove(&point) {
                pending.extend(next);
            }
        }
    }
    count
}
#[test]
fn opaque_non_square_outline_closes_on_actual_rectangle_edges() {
    let outline = BrushMask::new(8, 4, vec![255; 32])
        .unwrap()
        .outline()
        .unwrap();
    assert_eq!(components(&outline), 1);
    for segment in outline {
        for [x, y] in [segment.start, segment.end] {
            assert!((0. ..=1.).contains(&x) && (0.25..=0.75).contains(&y));
            assert!(x == 0. || x == 1. || y == 0.25 || y == 0.75);
        }
    }
}
#[test]
fn holes_and_disconnected_islands_are_preserved_and_transparency_is_empty() {
    let mut pixels = vec![0; 24 * 12];
    for y in 1..11 {
        for x in 1..11 {
            pixels[y * 24 + x] = 255;
        }
    }
    for y in 4..8 {
        for x in 4..8 {
            pixels[y * 24 + x] = 0;
        }
    }
    for y in 3..7 {
        for x in 17..21 {
            pixels[y * 24 + x] = 255;
        }
    }
    let outline = BrushMask::new(24, 12, pixels).unwrap().outline().unwrap();
    assert_eq!(components(&outline), 3);
    for alpha in [0, 127] {
        assert!(BrushMask::new(1, 1, vec![alpha])
            .unwrap()
            .outline()
            .unwrap()
            .is_empty());
    }
    assert_eq!(
        components(&BrushMask::new(1, 1, vec![128]).unwrap().outline().unwrap()),
        1
    );
}
#[test]
fn interpolation_and_saddle_connectivity_follow_coverage_values() {
    for bits in 0..16 {
        let pixels = (0..4)
            .map(|i| if bits & (1 << i) != 0 { 255 } else { 0 })
            .collect();
        let outline = BrushMask::new(2, 2, pixels).unwrap().outline().unwrap();
        if bits == 0 {
            assert!(outline.is_empty());
        } else {
            assert!(components(&outline) > 0);
        }
    }
    // Low diagonal samples cannot join the strong other diagonal.
    for (pixels, expected) in [(vec![255, 100, 100, 255], 1), (vec![128, 0, 0, 128], 2)] {
        assert_eq!(
            components(&BrushMask::new(2, 2, pixels).unwrap().outline().unwrap()),
            expected
        );
    }
    let outline = BrushMask::new(2, 1, vec![255, 64])
        .unwrap()
        .outline()
        .unwrap();
    let expected = (0.5 + 127.5 / 191.) / 2.;
    assert!(outline
        .iter()
        .any(|s| (s.start[0] - expected).abs() < 1e-10));
}
#[test]
fn pathological_detail_returns_limit_instead_of_partial_outline() {
    let pixels = (0..512 * 512)
        .map(|i| if (i / 512 + i % 512) % 2 == 0 { 255 } else { 0 })
        .collect();
    assert!(matches!(
        BrushMask::new(512, 512, pixels).unwrap().outline(),
        Err(Error::ResourceLimit(_))
    ));
}
