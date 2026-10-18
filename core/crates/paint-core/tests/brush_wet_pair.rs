use paint_core::*;
fn red() -> Pixel {
    Pixel::from_straight([1., 0., 0., 1.]).unwrap()
}
fn blue() -> Pixel {
    Pixel::from_straight([0., 0., 1., 0.5]).unwrap()
}
fn assert_carrier(a: &WetCarrier, b: &WetCarrier) {
    assert_eq!(a.edge(), b.edge());
    assert_eq!(a.revision(), b.revision());
    assert_eq!(a.settings(), b.settings());
    for y in 0..13 {
        for x in 0..13 {
            assert_eq!(
                a.sample(x as f64 / 12., y as f64 / 12.),
                b.sample(x as f64 / 12., y as f64 / 12.)
            );
        }
    }
}
fn assert_ink(a: &WetInk, b: &WetInk) {
    assert_carrier(a.primary(), b.primary());
    assert_eq!(a.secondary().is_some(), b.secondary().is_some());
    if let (Some(a), Some(b)) = (a.secondary(), b.secondary()) {
        assert_carrier(a, b)
    }
}
fn geometry(x: f64, diameter: f32) -> WetDabGeometry {
    WetDabGeometry {
        x,
        y: 8.,
        diameter,
        roundness: 0.6,
        angle: 35.,
    }
}
fn source(x: f64, y: f64) -> Result<WetSample> {
    Ok(WetSample {
        color: Pixel::from_straight([
            (x / 128.).clamp(0., 1.) as f32,
            (y / 64.).clamp(0., 1.) as f32,
            0.2,
            0.8,
        ])?,
        pickup_weight: if x < 40. { 0.25 } else { 1. },
    })
}

#[test]
fn roles_keep_independent_colors_sizes_and_complete_retained_configuration() {
    let policy = WetSettings::new(true, false, true, true, WetMix::default());
    let ink = WetInk::for_stroke(policy, 8., red(), Some((32., blue())), None).unwrap();
    assert_eq!(ink.primary().edge(), 8);
    assert_eq!(ink.secondary().unwrap().edge(), 32);
    assert_eq!(ink.primary().sample(0.5, 0.5), red());
    assert_eq!(ink.secondary().unwrap().sample(0.5, 0.5), blue());
    let retained = WetSettings::new(
        true,
        false,
        false,
        false,
        WetMix::new(0.1, 0.2, 0.3, 0.1, 0.4, 0.5).unwrap(),
    );
    let next = WetInk::for_stroke(retained, 16., blue(), Some((4., red())), Some(&ink)).unwrap();
    assert_eq!(next.primary().sample(0.5, 0.5), red());
    assert_eq!(next.secondary().unwrap().sample(0.5, 0.5), blue());
    assert_eq!(next.primary().edge(), 16);
    assert_eq!(next.secondary().unwrap().edge(), 4);
    assert_eq!(ink.primary().edge(), 8);
    assert_eq!(ink.secondary().unwrap().edge(), 32);
    let single = WetInk::for_stroke(retained, 8., red(), None, Some(&ink)).unwrap();
    assert!(single.secondary().is_none());
}

#[test]
fn real_two_dimensional_pickup_and_distance_consumption_match_two_single_carriers() {
    let mix = WetMix::new(0.7, 0.65, 0.4, 1., 0.8, 0.6).unwrap();
    let mut primary = WetCarrier::new(mix, 8., red()).unwrap();
    let mut secondary = WetCarrier::new(mix, 16., blue()).unwrap();
    let mut ink = WetInk::new(primary.clone(), Some(secondary.clone()));
    // Different schedules and movement distances must never advance the other role.
    for (second, x) in [
        (false, 16.),
        (false, 32.),
        (true, 24.),
        (false, 72.),
        (true, 96.),
    ] {
        let untouched = if second {
            ink.primary().clone()
        } else {
            ink.secondary().unwrap().clone()
        };
        let reference = if second { &mut secondary } else { &mut primary };
        let carrier = if second {
            ink.secondary_mut().unwrap()
        } else {
            ink.primary_mut()
        };
        let g = geometry(x, if second { 16. } else { 8. });
        let expected = reference.prepare_dab(g, source).unwrap();
        let actual = carrier.prepare_dab(g, source).unwrap();
        for y in 0..9 {
            for x in 0..9 {
                assert_eq!(
                    actual.sample(x as f64 / 8., y as f64 / 8.),
                    expected.sample(x as f64 / 8., y as f64 / 8.)
                );
            }
        }
        reference.commit(expected).unwrap();
        carrier.commit(actual).unwrap();
        assert_carrier(
            &untouched,
            if second {
                ink.primary()
            } else {
                ink.secondary().unwrap()
            },
        );
        assert_carrier(ink.primary(), &primary);
        assert_carrier(ink.secondary().unwrap(), &secondary);
    }
    assert_ne!(
        ink.primary().sample(0.1, 0.1),
        ink.primary().sample(0.9, 0.9)
    );
}

#[test]
fn stroke_policy_cleans_each_role_and_new_secondary_never_inherits_primary_ink() {
    let mix = WetMix::new(0., 0., 0., 1., 1., 1.).unwrap();
    let keep = WetSettings::new(true, false, false, false, mix);
    let mut ink = WetInk::for_stroke(
        WetSettings::new(true, false, false, true, mix),
        8.,
        red(),
        None,
        None,
    )
    .unwrap();
    let dab = ink.primary().prepare_dab(geometry(8., 8.), source).unwrap();
    ink.primary_mut().commit(dab).unwrap();
    let next = WetInk::for_stroke(keep, 8., blue(), Some((8., red())), Some(&ink)).unwrap();
    assert_eq!(
        next.secondary().unwrap().sample(0.5, 0.5),
        Pixel::TRANSPARENT
    );
    let first = next
        .primary()
        .prepare_dab(geometry(100., 8.), source)
        .unwrap();
    assert_eq!(first.sample(0.5, 0.5), red());
    let cleaned = WetInk::for_stroke(
        WetSettings::new(true, false, true, false, mix),
        8.,
        red(),
        Some((8., blue())),
        Some(&next),
    )
    .unwrap();
    assert_eq!(cleaned.primary().sample(0.5, 0.5), Pixel::TRANSPARENT);
    assert_eq!(
        cleaned.secondary().unwrap().sample(0.5, 0.5),
        Pixel::TRANSPARENT
    );
    let before = next.clone();
    assert!(WetInk::for_stroke(keep, 8., red(), Some((f32::NAN, blue())), Some(&next)).is_err());
    assert_ink(&next, &before);
}

#[test]
fn tool_reserves_and_adopts_the_whole_pair_but_cancel_preserves_both() {
    let tool = WetTool::default();
    tool.reload_dual(WetMix::default(), [8., 16.], [red(), blue()])
        .unwrap();
    let before = tool.snapshot_ink().unwrap().unwrap();
    let lease = tool.lease().unwrap();
    assert_ink(lease.retained_ink().unwrap(), &before);
    assert_carrier(lease.retained().unwrap(), before.primary());
    let other = tool.clone();
    std::thread::spawn(move || {
        assert!(matches!(other.snapshot_ink(), Err(Error::Busy)));
        assert!(matches!(other.lease(), Err(Error::Busy)));
        assert!(matches!(other.clean(), Err(Error::Busy)));
        assert!(matches!(
            other.reload_dual(WetMix::default(), [8., 8.], [red(), blue()]),
            Err(Error::Busy)
        ));
    })
    .join()
    .unwrap();
    let mut changed = before.clone();
    changed.primary_mut().reload(blue()).unwrap();
    changed.secondary_mut().unwrap().reload(red()).unwrap();
    drop(lease);
    assert_ink(&tool.snapshot_ink().unwrap().unwrap(), &before);
    tool.lease().unwrap().commit_ink(changed.clone());
    assert_ink(&tool.snapshot_ink().unwrap().unwrap(), &changed);
    let mut legacy = changed.primary().clone();
    legacy.clean().unwrap();
    tool.lease().unwrap().commit(legacy.clone());
    let adopted = tool.snapshot_ink().unwrap().unwrap();
    assert_carrier(adopted.primary(), &legacy);
    assert_carrier(adopted.secondary().unwrap(), changed.secondary().unwrap());
    tool.clean().unwrap();
    let clean = tool.snapshot_ink().unwrap().unwrap();
    assert_eq!(clean.primary().sample(0.5, 0.5), Pixel::TRANSPARENT);
    assert_eq!(
        clean.secondary().unwrap().sample(0.5, 0.5),
        Pixel::TRANSPARENT
    );
}

#[test]
fn bad_second_manual_load_is_atomic_and_legacy_primary_load_preserves_secondary() {
    let tool = WetTool::default();
    tool.reload_dual(WetMix::default(), [8., 16.], [red(), blue()])
        .unwrap();
    let before = tool.snapshot_ink().unwrap().unwrap();
    assert!(tool
        .reload_dual(WetMix::default(), [32., 2000.25], [blue(), red()])
        .is_err());
    assert_ink(&tool.snapshot_ink().unwrap().unwrap(), &before);
    tool.reload(WetMix::default(), 4., blue()).unwrap();
    let next = tool.snapshot_ink().unwrap().unwrap();
    assert_eq!(next.primary().sample(0.5, 0.5), blue());
    assert_carrier(next.secondary().unwrap(), before.secondary().unwrap());
    assert_carrier(&tool.snapshot().unwrap().unwrap(), next.primary());
    let fresh = WetTool::default();
    assert!(fresh
        .reload_dual(WetMix::default(), [8., 0.], [red(), blue()])
        .is_err());
    assert!(fresh.snapshot_ink().unwrap().is_none());
}
