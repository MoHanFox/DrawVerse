use paint_core::{
    stroke_outline_band, BrushTexture, Error, OutlinePlacement, OutlineSource, OutlineWidth,
    StrokeOutline, MAX_STROKE_OUTLINE_PIXELS,
};

const PLACEMENTS: [OutlinePlacement; 3] = [
    OutlinePlacement::Outside,
    OutlinePlacement::Center,
    OutlinePlacement::Inside,
];

fn reference(w: usize, h: usize, mask: &[u8], width: f32, placement: OutlinePlacement) -> Vec<f32> {
    let (outward, inward) = placement.extents(width);
    mask.iter()
        .enumerate()
        .map(|(i, &value)| {
            if width == 0. {
                return 0.;
            }
            let (x, y) = (i % w, i / w);
            let inside = value >= 128;
            let mut distance = f64::INFINITY;
            for sy in -1..=h as i32 {
                for sx in -1..=w as i32 {
                    let covered = sx >= 0
                        && sy >= 0
                        && sx < w as i32
                        && sy < h as i32
                        && mask[sy as usize * w + sx as usize] >= 128;
                    if inside != covered {
                        distance = distance.min((x as f64 - sx as f64).hypot(y as f64 - sy as f64));
                    }
                }
            }
            let signed = (distance - 0.5) * if inside { 1. } else { -1. };
            ((signed + f64::from(outward) + 0.5).clamp(0., 1.)
                * (f64::from(inward) - signed + 0.5).clamp(0., 1.)) as f32
        })
        .collect()
}

#[test]
fn bands_match_independent_nearest_pixel_reference_including_holes_and_borders() {
    let mut random = 17_u64;
    for (w, h) in [(1, 1), (1, 19), (17, 1), (8, 7), (19, 23)] {
        for pattern in 0..6 {
            let mask: Vec<_> = (0..w * h)
                .map(|i| {
                    random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                    match pattern {
                        0 => 0,
                        1 => 255,
                        2 => {
                            if i % w > 1 && i % w < w - 1 && i / w > 1 && i / w < h - 1 {
                                0
                            } else {
                                255
                            }
                        }
                        _ => (random >> 32) as u8,
                    }
                })
                .collect();
            for placement in PLACEMENTS {
                for width in [0., 0.25, 1., 2.5, 20., 2000.] {
                    let actual =
                        stroke_outline_band(w as u32, h as u32, &mask, width, placement).unwrap();
                    let expected = reference(w, h, &mask, width, placement);
                    for (i, (a, b)) in actual.iter().zip(expected).enumerate() {
                        assert!((a - b).abs() < 1e-6, "{w}x{h}, pattern {pattern}, {placement:?}, width {width}, pixel {i}: {a} != {b}");
                    }
                }
            }
        }
    }
}

#[test]
fn placement_uses_complete_width_and_overlap_has_no_internal_seam() {
    // Two overlapping stamps form one rectangle; their former ends are internal.
    let (w, h) = (17, 13);
    let mut mask = vec![0; w * h];
    for (left, right) in [(3, 10), (7, 14)] {
        for y in 3..10 {
            for x in left..right {
                mask[y * w + x] = 255;
            }
        }
    }
    let original = mask.clone();
    let outside =
        stroke_outline_band(w as u32, h as u32, &mask, 2., OutlinePlacement::Outside).unwrap();
    let center =
        stroke_outline_band(w as u32, h as u32, &mask, 2., OutlinePlacement::Center).unwrap();
    let inside =
        stroke_outline_band(w as u32, h as u32, &mask, 2., OutlinePlacement::Inside).unwrap();
    let at = |values: &[f32], x, y| values[y * w + x];
    assert_eq!(
        [at(&outside, 1, 6), at(&outside, 2, 6), at(&outside, 3, 6)],
        [1., 1., 0.]
    );
    assert_eq!(
        [
            at(&center, 1, 6),
            at(&center, 2, 6),
            at(&center, 3, 6),
            at(&center, 4, 6)
        ],
        [0., 1., 1., 0.]
    );
    assert_eq!(
        [
            at(&inside, 2, 6),
            at(&inside, 3, 6),
            at(&inside, 4, 6),
            at(&inside, 5, 6)
        ],
        [0., 1., 1., 0.]
    );
    for x in 6..11 {
        for band in [&outside, &center, &inside] {
            assert_eq!(at(band, x, 6), 0., "no seams between overlapping stamps");
        }
    }
    assert_eq!(mask, original);
    let subpixel =
        stroke_outline_band(w as u32, h as u32, &mask, 0.5, OutlinePlacement::Center).unwrap();
    assert_eq!([at(&subpixel, 2, 6), at(&subpixel, 3, 6)], [0.25, 0.25]);
}

#[test]
fn width_modes_keep_both_values_and_closed_settings_keep_source_and_colour() {
    let absolute = OutlineWidth::new(false, 12.25, 17.5).unwrap();
    let relative = OutlineWidth::new(true, absolute.pixels(), absolute.percent()).unwrap();
    for diameter in [1., 24., 2000.] {
        assert_eq!(absolute.effective(diameter).unwrap(), 12.25);
        assert_eq!(
            relative.effective(diameter).unwrap(),
            (diameter * 0.175).min(2000.)
        );
    }
    assert_eq!(
        OutlineWidth::new(true, 0., 10000.)
            .unwrap()
            .effective(2000.)
            .unwrap(),
        2000.
    );
    let closed = StrokeOutline::new(
        false,
        OutlinePlacement::Inside,
        relative,
        OutlineSource::Texture(BrushTexture::default()),
        true,
    )
    .unwrap();
    assert!(!closed.enabled());
    assert_eq!(closed.placement(), OutlinePlacement::Inside);
    assert_eq!(closed.width(), relative);
    assert!(closed.secondary_color());
    assert!(matches!(closed.source(), OutlineSource::Texture(_)));
    assert!(StrokeOutline::new(
        true,
        OutlinePlacement::Outside,
        absolute,
        OutlineSource::Texture(BrushTexture::default()),
        false
    )
    .is_err());
    for bad in [-1., 2000.1, f32::NAN, f32::INFINITY] {
        assert!(OutlineWidth::new(false, bad, 10.).is_err());
    }
    for bad in [-1., 10000.1, f32::NAN, f32::INFINITY] {
        assert!(OutlineWidth::new(false, 2., bad).is_err());
    }
    for bad in [0., 2000.1, f32::NAN, f32::INFINITY] {
        assert!(absolute.effective(bad).is_err());
    }
}

#[test]
fn empty_fractional_and_invalid_rasters_have_defined_bounded_results() {
    for value in [0, 1, 127] {
        assert_eq!(
            stroke_outline_band(5, 3, &[value; 15], 2000., OutlinePlacement::Outside).unwrap(),
            vec![0.; 15]
        );
    }
    for width in [-1., 2000.1, f32::NAN, f32::INFINITY] {
        assert!(stroke_outline_band(1, 1, &[255], width, OutlinePlacement::Outside).is_err());
    }
    assert!(stroke_outline_band(0, 1, &[], 1., OutlinePlacement::Outside).is_err());
    assert!(stroke_outline_band(1, 1, &[], 1., OutlinePlacement::Outside).is_err());
    assert!(matches!(
        stroke_outline_band(
            MAX_STROKE_OUTLINE_PIXELS as u32 + 1,
            1,
            &[],
            1.,
            OutlinePlacement::Outside
        ),
        Err(Error::ResourceLimit(_))
    ));
    assert!(matches!(
        stroke_outline_band(u32::MAX, u32::MAX, &[], 1., OutlinePlacement::Outside),
        Err(Error::ResourceLimit(_))
    ));
    // Fully covered input has an implicit transparent exterior, even without padding.
    assert_eq!(
        stroke_outline_band(1, 1, &[128], 1., OutlinePlacement::Inside).unwrap(),
        vec![1.]
    );
}
