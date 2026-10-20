use paint_ops::sdf::*;

fn rectangle(w: usize, h: usize, bounds: [usize; 4]) -> Vec<u8> {
    let mut a = vec![0; w * h];
    for y in bounds[1]..bounds[3] {
        a[y * w + bounds[0]..y * w + bounds[2]].fill(255);
    }
    a
}

#[test]
fn edt_matches_exhaustive_weighted_sites_and_rejects_before_mutation() {
    for w in 1..=11 {
        for h in 1..=9 {
            let input: Vec<_> = (0..w * h)
                .map(|i| {
                    if (i * 13 + w * h) % 7 < 2 {
                        (i % 5) as f64 * 0.125
                    } else {
                        f64::INFINITY
                    }
                })
                .collect();
            let expected: Vec<_> = (0..w * h)
                .map(|i| {
                    input
                        .iter()
                        .enumerate()
                        .map(|(j, s)| {
                            let dx = (i % w) as f64 - (j % w) as f64;
                            let dy = (i / w) as f64 - (j / w) as f64;
                            s + dx * dx + dy * dy
                        })
                        .fold(f64::INFINITY, f64::min)
                })
                .collect();
            let mut actual = input;
            squared_distance_transform(w as u32, h as u32, &mut actual).unwrap();
            assert_eq!(actual, expected, "{w}x{h}");
        }
    }
    for bad in [-1., f64::NAN, f64::NEG_INFINITY] {
        let mut seeds = [0., bad];
        let bits = seeds.map(f64::to_bits);
        assert!(squared_distance_transform(2, 1, &mut seeds).is_err());
        assert_eq!(seeds.map(f64::to_bits), bits);
    }
    let mut seeds = [0.];
    assert!(squared_distance_transform(u32::MAX, u32::MAX, &mut seeds).is_err());
    assert_eq!(seeds, [0.]);
}

#[test]
fn signed_field_has_half_pixel_boundary_and_transparent_outside() {
    let a = rectangle(9, 9, [2, 2, 7, 7]);
    let field = DistanceField::from_alpha(9, 9, &a).unwrap();
    let at = |x: usize, y: usize| field.values()[y * 9 + x];
    assert_eq!(at(1, 4), 0.5);
    assert_eq!(at(2, 4), -0.5);
    assert_eq!(at(4, 4), -2.5);
    assert!((at(0, 0) - (8_f32.sqrt() - 0.5)).abs() < 1e-6);
    let solid = DistanceField::from_alpha(5, 3, &[255; 15]).unwrap();
    assert_eq!(solid.values()[7], -1.5);
    assert_eq!(solid.values()[0], -0.5);
    let empty = DistanceField::from_alpha(5, 3, &[0; 15]).unwrap();
    assert!(empty.values().iter().all(|v| v.is_finite() && *v > 5.));
    assert_eq!(empty.isolines(0.).unwrap().paths().count(), 0);
    assert!(DistanceField::from_alpha(0, 3, &[]).is_err());
    assert!(DistanceField::from_alpha(2, 3, &[0; 5]).is_err());
}

#[test]
fn isolines_keep_holes_separate_and_resample_without_duplicate_seam() {
    let mut a = rectangle(40, 40, [6, 6, 34, 34]);
    for y in 15..25 {
        a[y * 40 + 15..y * 40 + 25].fill(0);
    }
    let field = DistanceField::from_alpha(40, 40, &a).unwrap();
    for placement in [Placement::Outside, Placement::Inside, Placement::Center] {
        let width = 3.;
        let lines = field.isolines(placement.level(width)).unwrap();
        assert_eq!(lines.paths().count(), 2);
        for (points, closed) in lines.paths() {
            assert!(closed);
            assert_ne!(points[0], points[points.len() - 1]);
            assert!(points.iter().all(|p| p.iter().all(|v| v.is_finite())));
        }
        let stamps = lines.stamps(&field, width).unwrap();
        assert!(stamps.len() > 90);
        for s in &stamps {
            assert_eq!(s.radius, 1.5);
            assert!((s.tangent[0].hypot(s.tangent[1]) - 1.).abs() < 1e-5);
            assert!((s.normal[0] * s.tangent[0] + s.normal[1] * s.tangent[1]).abs() < 1e-6);
            assert!(s.texture_rotation.is_finite());
        }
        assert_eq!(stamps, field.outline_stamps(width, placement).unwrap());
        // The gradient on a straight left outer edge points outwards.
        assert!(stamps.iter().any(|s| s.center[0] < 8.
            && (10. ..14.).contains(&s.center[1])
            && s.normal[0] < -0.99));
    }
}

#[test]
fn marching_saddles_are_deterministic_and_open_paths_are_not_split() {
    // Positive SDF regions touch diagonally; the equal saddle separates them.
    let field = DistanceField::from_alpha(2, 2, &[0, 255, 255, 0]).unwrap();
    let lines = field.isolines(0.).unwrap();
    let paths: Vec<_> = lines.paths().collect();
    assert_eq!(paths.len(), 2);
    assert!(paths.iter().all(|(p, closed)| p.len() == 2 && !closed));
    assert!(paths
        .iter()
        .any(|(p, _)| p.contains(&[1., 0.5]) && p.contains(&[0.5, 1.])));
    let a = rectangle(9, 7, [0, 0, 4, 7]);
    let field = DistanceField::from_alpha(9, 7, &a).unwrap();
    let lines = field.isolines(0.).unwrap();
    let paths: Vec<_> = lines.paths().collect();
    assert_eq!(paths.len(), 1);
    assert!(!paths[0].1);
    let stamps = lines.stamps(&field, 2.).unwrap();
    assert_eq!(stamps.first().unwrap().center[1], 0.5);
    assert_eq!(stamps.last().unwrap().center[1], 6.5);
    for pair in stamps.windows(2) {
        assert!((pair[1].center[1] - pair[0].center[1] - 0.75).abs() < 1e-6);
    }
    assert!(field.isolines(f32::NAN).is_err());
}

#[test]
fn exact_vertex_intersections_and_fragmented_masks_keep_closed_topology() {
    for seed in 0..8 {
        let mut a = vec![0; 24 * 22];
        for y in 3..19 {
            for x in 3..21 {
                if (x * 13 + y * 7 + seed) % 5 < 3 {
                    a[y * 24 + x] = 255;
                }
            }
        }
        let field = DistanceField::from_alpha(24, 22, &a).unwrap();
        for level in [-0.5, 0., 0.5, 1.] {
            let lines = field.isolines(level).unwrap();
            assert!(
                lines.paths().all(|(_, closed)| closed),
                "seed={seed} level={level}"
            );
            let stamps = lines.stamps(&field, 2.).unwrap();
            assert!(stamps
                .iter()
                .all(|s| s.center.iter().all(|v| v.is_finite())));
        }
    }
}

#[test]
fn max_stamps_do_not_accumulate_and_visit_budget_precedes_sampling() {
    let s = Stamp {
        center: [4.5, 4.5],
        tangent: [1., 0.],
        normal: [0., 1.],
        radius: 2.,
        texture_rotation: 0.,
    };
    let render = |stamps: &[Stamp]| {
        rasterize_max(9, 9, stamps, |s, p| Ok(disk_coverage(s, p) * 0.4)).unwrap()
    };
    assert_eq!(render(&[s]), render(&[s; 10]));
    assert_eq!(render(&[s])[4 * 9 + 4], 0.4);
    let s = Stamp {
        center: [10., 10.],
        radius: 1000.,
        ..s
    };
    let mut calls = 0;
    assert!(rasterize_max(1000, 1000, &[s; 65], |_, _| {
        calls += 1;
        Ok(1.)
    })
    .is_err());
    assert_eq!(calls, 0);
    assert!(rasterize_max(9, 9, &[s], |_, _| Ok(f32::NAN)).is_err());
    assert!(rasterize_max(
        9,
        9,
        &[Stamp {
            radius: f32::NAN,
            ..s
        }],
        |_, _| Ok(1.)
    )
    .is_err());
    let off = Stamp {
        center: [-10000., -10000.],
        radius: 2.,
        ..s
    };
    assert!(render(&[off]).iter().all(|v| *v == 0.));
}

#[test]
fn mask_boolean_algebra_preserves_fractional_alpha_and_empty_difference() {
    let (a, b) = ([0., 0.25, 0.75, 1.], [1., 0.5, 0.75, 0.]);
    assert_eq!(
        combine_masks(&a, &b, MaskOp::Max).unwrap(),
        [1., 0.5, 0.75, 1.]
    );
    assert_eq!(
        combine_masks(&a, &b, MaskOp::Union).unwrap(),
        [1., 0.625, 0.9375, 1.]
    );
    assert_eq!(
        combine_masks(&a, &b, MaskOp::Intersection).unwrap(),
        [0., 0.125, 0.5625, 0.]
    );
    assert_eq!(
        combine_masks(&a, &b, MaskOp::Difference).unwrap(),
        [0., 0.125, 0.1875, 1.]
    );
    assert!(combine_masks(&[1.1], &[0.], MaskOp::Max).is_err());
    assert!(combine_masks(&[], &[0.], MaskOp::Max).is_err());
}

#[test]
fn outside_inside_and_center_masks_have_nonempty_antialiased_bands() {
    let mut alpha = rectangle(64, 64, [16, 16, 48, 48]);
    for y in 17..47 {
        alpha[y * 64 + 16] = 180;
        alpha[y * 64 + 15] = 60;
    }
    let field = DistanceField::from_alpha(64, 64, &alpha).unwrap();
    let original = alpha.clone();
    for placement in [Placement::Outside, Placement::Inside, Placement::Center] {
        let stamps = field.outline_stamps(4., placement).unwrap();
        let layer = rasterize_max(64, 64, &stamps, |s, p| Ok(disk_coverage(s, p))).unwrap();
        let band = compose_outline(&field, &alpha, &layer, 4., placement).unwrap();
        assert!(band.iter().any(|v| *v > 0.9));
        assert!(band.iter().any(|v| *v > 0. && *v < 1.));
        assert_eq!(band[32 * 64 + 32], 0.);
        for (i, (&v, &a)) in band.iter().zip(&alpha).enumerate() {
            assert!(v.is_finite() && (0. ..=1.).contains(&v));
            match placement {
                Placement::Outside => assert!(v <= 1. - f32::from(a) / 255. + 1e-6),
                Placement::Inside => assert!(v <= f32::from(a) / 255. + 1e-6),
                Placement::Center => {
                    if field.values()[i].abs() > 2.5 {
                        assert_eq!(v, 0.);
                    }
                }
            }
        }
        assert_eq!(alpha, original);
    }
    assert!(field
        .outline_stamps(0., Placement::Inside)
        .unwrap()
        .is_empty());
    assert!(field
        .outline_stamps(f32::INFINITY, Placement::Inside)
        .is_err());
}

#[test]
fn subpixel_and_thin_inside_contours_remain_bounded() {
    let alpha = rectangle(30, 30, [10, 10, 20, 20]);
    let field = DistanceField::from_alpha(30, 30, &alpha).unwrap();
    let stamps = field.outline_stamps(0.25, Placement::Center).unwrap();
    assert!(!stamps.is_empty());
    assert!(field
        .outline_stamps(f32::MIN_POSITIVE, Placement::Center)
        .is_err());
    assert!(field
        .outline_stamps(40., Placement::Inside)
        .unwrap()
        .is_empty());
    let thin = rectangle(30, 30, [10, 10, 11, 20]);
    let field = DistanceField::from_alpha(30, 30, &thin).unwrap();
    assert!(!field
        .outline_stamps(0.5, Placement::Inside)
        .unwrap()
        .is_empty());
}

#[test]
fn fragmented_input_returns_geometry_limit_without_unbounded_growth() {
    let w = 600;
    let alpha: Vec<_> = (0..w * w)
        .map(|i| if (i / w + i % w) % 2 == 0 { 255 } else { 0 })
        .collect();
    let field = DistanceField::from_alpha(w as u32, w as u32, &alpha).unwrap();
    assert!(matches!(field.isolines(0.), Err(Error::Limit(_))));
}
