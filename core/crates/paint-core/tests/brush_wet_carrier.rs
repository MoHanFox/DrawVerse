use paint_core::*;
use std::cell::RefCell;
fn color(rgb: [f32; 3], a: f32) -> Pixel {
    Pixel::from_straight([rgb[0], rgb[1], rgb[2], a]).unwrap()
}
fn red() -> Pixel {
    color([1., 0., 0.], 1.)
}
fn blue() -> Pixel {
    color([0., 0., 1.], 1.)
}
fn geometry(x: f64, y: f64) -> WetDabGeometry {
    WetDabGeometry {
        x,
        y,
        diameter: 4.,
        roundness: 1.,
        angle: 0.,
    }
}
fn source(color: Pixel) -> WetSample {
    WetSample {
        color,
        pickup_weight: 1.,
    }
}
fn close(a: Pixel, b: Pixel) {
    for (a, b) in a.components().into_iter().zip(b.components()) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }
}

#[test]
fn two_dimensional_colors_are_carried_without_becoming_a_uniform_average() {
    let mut brush = WetCarrier::new(
        WetMix::new(1., 1., 0., 0., 1., 0.).unwrap(),
        4.,
        Pixel::WHITE,
    )
    .unwrap();
    let snapshot = brush.clone();
    let dab = brush
        .prepare_dab(geometry(0., 0.), |x, _| {
            Ok(source(if x < 0. { red() } else { blue() }))
        })
        .unwrap();
    assert_eq!(brush.sample(0.125, 0.125), Pixel::TRANSPARENT);
    assert_eq!(dab.sample(0.125, 0.125), red());
    assert_eq!(dab.sample(0.875, 0.875), blue());
    close(dab.sample(0.5, 0.5), color([0.5, 0., 0.5], 1.));
    brush.commit(dab).unwrap();
    assert_eq!(snapshot.sample(0.125, 0.125), Pixel::TRANSPARENT);
    let carried = brush
        .prepare_dab(geometry(20., 20.), |_, _| {
            Ok(WetSample {
                color: Pixel::TRANSPARENT,
                pickup_weight: 0.,
            })
        })
        .unwrap();
    assert_eq!(carried.sample(0.125, 0.875), red());
    assert_eq!(carried.sample(0.875, 0.125), blue());
    assert_eq!(carried.sample(-0.1, 0.5), Pixel::TRANSPARENT);
    assert_eq!(carried.sample(f64::NAN, 0.5), Pixel::TRANSPARENT);
}
#[test]
fn premultiplied_transparency_load_pickup_mix_and_deposit_are_applied_once() {
    let settings = WetMix::new(1., 0.5, 0.5, 0.5, 0.5, 0.).unwrap();
    let brush = WetCarrier::new(settings, 4., color([1., 0., 0.], 0.5)).unwrap();
    close(brush.sample(0.5, 0.5), color([1., 0., 0.], 0.25));
    let dab = brush
        .prepare_dab(geometry(0., 0.), |_, _| {
            Ok(source(color([0., 0., 1.], 0.5)))
        })
        .unwrap();
    // carry=[.25,0,0,.25], pickup=[.125,0,.25,.375], mixed=[.0625,0,.375,.4375], deposit*0.5.
    close(dab.sample(0.5, 0.5), color([1. / 7., 0., 6. / 7.], 0.21875));
    let transparent = brush
        .prepare_dab(geometry(0., 0.), |_, _| Ok(source(Pixel::TRANSPARENT)))
        .unwrap();
    close(transparent.sample(0.5, 0.5), color([1., 0., 0.], 0.03125));
    let dry = WetCarrier::new(WetMix::new(0., 1., 1., 1., 1., 0.).unwrap(), 4., red()).unwrap();
    let dry_dab = dry
        .prepare_dab(geometry(0., 0.), |_, _| panic!("dry brush must not sample"))
        .unwrap();
    assert_eq!(dry_dab.sample(0.5, 0.5), red());
    let no_deposit =
        WetCarrier::new(WetMix::new(1., 1., 0., 0., 0., 0.).unwrap(), 4., red()).unwrap();
    let dab = no_deposit
        .prepare_dab(geometry(0., 0.), |_, _| Ok(source(blue())))
        .unwrap();
    assert_eq!(dab.sample(0.5, 0.5), Pixel::TRANSPARENT);
}
#[test]
fn rotating_flattening_and_pressure_sizing_map_normalized_carrier_cells() {
    let brush = WetCarrier::new(WetMix::new(1., 1., 0., 0., 1., 0.).unwrap(), 4., red()).unwrap();
    let mut seen = Vec::new();
    let mut g = geometry(10., 20.);
    g.roundness = 0.5;
    g.angle = 90.;
    let dab = brush
        .prepare_dab(g, |x, y| {
            seen.push((x, y));
            Ok(source(if y < 20. { red() } else { blue() }))
        })
        .unwrap();
    assert_eq!(seen.len(), 16);
    assert!((seen[0].0 - 10.75).abs() < 1e-12);
    assert!((seen[0].1 - 18.5).abs() < 1e-12);
    assert_eq!(dab.sample(0.125, 0.125), red());
    assert_eq!(dab.sample(0.875, 0.125), blue());
    seen.clear();
    g.diameter = 2.;
    brush
        .prepare_dab(g, |x, y| {
            seen.push((x, y));
            Ok(source(blue()))
        })
        .unwrap();
    assert!((seen[0].0 - 10.375).abs() < 1e-12);
    assert!((seen[0].1 - 19.25).abs() < 1e-12);
}
#[test]
fn sampling_finishes_before_pixels_write_and_next_dab_reads_previous_pixels() {
    let mut brush =
        WetCarrier::new(WetMix::new(1., 0.5, 0., 1., 1., 0.).unwrap(), 4., red()).unwrap();
    let canvas = RefCell::new(vec![Pixel::TRANSPARENT; 16]);
    let mut reads = 0;
    let dab = brush
        .prepare_dab(geometry(2., 2.), |x, y| {
            reads += 1;
            assert!(canvas.borrow().iter().all(|p| *p == Pixel::TRANSPARENT));
            Ok(source(
                canvas.borrow()[y.floor() as usize * 4 + x.floor() as usize],
            ))
        })
        .unwrap();
    assert_eq!(reads, 16);
    assert_eq!(brush.revision(), 0);
    for y in 0..4 {
        for x in 0..4 {
            let ink = dab.sample((x as f64 + 0.5) / 4., (y as f64 + 0.5) / 4.);
            let index = y * 4 + x;
            let old = canvas.borrow()[index];
            canvas.borrow_mut()[index] =
                blend_pixel(ink, old, BlendMode::Normal, x as u32, y as u32, 0);
        }
    }
    brush.commit(dab).unwrap();
    assert_eq!(canvas.borrow()[0], red().scaled(0.5));
    let next = brush
        .prepare_dab(geometry(2., 2.), |x, y| {
            Ok(source(
                canvas.borrow()[y.floor() as usize * 4 + x.floor() as usize],
            ))
        })
        .unwrap();
    assert_eq!(next.sample(0.5, 0.5), red().scaled(0.5));
    let wrong_old = brush
        .prepare_dab(geometry(2., 2.), |_, _| Ok(source(Pixel::TRANSPARENT)))
        .unwrap();
    assert_eq!(wrong_old.sample(0.5, 0.5), red().scaled(0.25));
}
#[test]
fn distance_consumption_dwell_clean_reload_and_path_reset_are_defined() {
    let settings = WetMix::new(0., 0., 0., 1., 1., 0.5).unwrap();
    let mut whole = WetCarrier::new(settings, 4., red()).unwrap();
    let first = whole
        .prepare_dab(geometry(0., 0.), |_, _| unreachable!())
        .unwrap();
    whole.commit(first).unwrap();
    let mut split = whole.clone();
    let next = whole
        .prepare_dab(geometry(4., 0.), |_, _| unreachable!())
        .unwrap();
    assert_eq!(next.sample(0.5, 0.5), red().scaled(0.5));
    whole.commit(next).unwrap();
    for x in [1., 2., 3., 4.] {
        let dab = split
            .prepare_dab(geometry(x, 0.), |_, _| unreachable!())
            .unwrap();
        split.commit(dab).unwrap();
    }
    close(whole.sample(0.5, 0.5), split.sample(0.5, 0.5));
    let dwell = whole
        .prepare_dab(geometry(4., 0.), |_, _| unreachable!())
        .unwrap();
    assert_eq!(dwell.sample(0.5, 0.5), red().scaled(0.5));
    whole.commit(dwell).unwrap();
    whole.reset_path().unwrap();
    let reset = whole
        .prepare_dab(geometry(1000., 0.), |_, _| unreachable!())
        .unwrap();
    assert_eq!(reset.sample(0.5, 0.5), red().scaled(0.5));
    whole.clean().unwrap();
    assert_eq!(whole.sample(0.5, 0.5), Pixel::TRANSPARENT);
    assert!(whole.commit(reset).is_err());
    whole.reload(blue()).unwrap();
    assert_eq!(whole.sample(0.5, 0.5), blue());
    let mut exhausted =
        WetCarrier::new(WetMix::new(0., 0., 0., 1., 1., 1.).unwrap(), 4., red()).unwrap();
    let first = exhausted
        .prepare_dab(geometry(0., 0.), |_, _| unreachable!())
        .unwrap();
    exhausted.commit(first).unwrap();
    assert_eq!(
        exhausted
            .prepare_dab(geometry(1., 0.), |_, _| unreachable!())
            .unwrap()
            .sample(0.5, 0.5),
        Pixel::TRANSPARENT
    );
}
#[test]
fn invalid_sources_geometry_parameters_and_stale_commits_are_atomic() {
    for index in 0..6 {
        for value in [f32::NAN, f32::INFINITY, -0.01, 1.01] {
            let mut p = [0.5; 6];
            p[index] = value;
            assert!(WetMix::new(p[0], p[1], p[2], p[3], p[4], p[5]).is_err());
        }
    }
    let mut brush = WetCarrier::new(WetMix::default(), 4., red()).unwrap();
    let original = brush.sample(0.5, 0.5);
    let mut reads = 0;
    assert!(brush
        .prepare_dab(geometry(0., 0.), |_, _| {
            reads += 1;
            if reads == 7 {
                Err(Error::InvalidArgument("test source failure"))
            } else {
                Ok(source(blue()))
            }
        })
        .is_err());
    assert_eq!(reads, 7);
    assert_eq!(brush.revision(), 0);
    assert_eq!(brush.sample(0.5, 0.5), original);
    assert!(brush
        .prepare_dab(geometry(0., 0.), |_, _| Ok(WetSample {
            color: blue(),
            pickup_weight: f32::NAN
        }))
        .is_err());
    let mut bad = geometry(0., 0.);
    bad.diameter = 0.;
    assert!(brush
        .prepare_dab(bad, |_, _| panic!("invalid geometry sampled"))
        .is_err());
    bad = geometry(f64::INFINITY, 0.);
    assert!(brush.prepare_dab(bad, |_, _| unreachable!()).is_err());
    let foreign = WetCarrier::new(WetMix::default(), 4., red()).unwrap();
    let alien = foreign
        .prepare_dab(geometry(0., 0.), |_, _| Ok(source(blue())))
        .unwrap();
    assert!(brush.commit(alien).is_err());
    assert_eq!(brush.revision(), 0);
    let first = brush
        .prepare_dab(geometry(0., 0.), |_, _| Ok(source(blue())))
        .unwrap();
    let stale = brush
        .prepare_dab(geometry(0., 0.), |_, _| Ok(source(red())))
        .unwrap();
    brush.commit(first).unwrap();
    let current = brush.sample(0.5, 0.5);
    assert!(brush.commit(stale).is_err());
    assert_eq!(brush.sample(0.5, 0.5), current);
    assert_eq!(brush.revision(), 1);
}
#[test]
fn maximum_grid_and_empty_pickup_remain_bounded() {
    let brush = WetCarrier::new(WetMix::new(1., 1., 0., 0., 1., 0.).unwrap(), 512., red()).unwrap();
    assert_eq!(brush.edge(), MAX_WET_CARRIER_EDGE);
    let mut reads = 0;
    let dab = brush
        .prepare_dab(geometry(0., 0.), |_, _| {
            reads += 1;
            Ok(source(Pixel::TRANSPARENT))
        })
        .unwrap();
    assert_eq!(reads, 4096);
    assert_eq!(dab.edge(), 64);
    assert_eq!(dab.sample(0., 1.), Pixel::TRANSPARENT);
    assert_eq!(std::mem::size_of::<WetSample>(), 20);
    assert!(WetCarrier::new(WetMix::default(), 2000.25, red()).is_err());
    assert!(WetCarrier::new(WetMix::default(), f32::NAN, red()).is_err());
    assert_eq!(
        WetCarrier::new(WetMix::default(), 1., red())
            .unwrap()
            .edge(),
        2
    );
}
