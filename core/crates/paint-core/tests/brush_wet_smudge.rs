use paint_core::*;
fn color(r: f32, b: f32, a: f32) -> Pixel {
    Pixel::from_straight([r, 0., b, a]).unwrap()
}
fn definition(finger: bool, pressure: bool) -> BrushDefinition {
    let mut b = BrushSettings {
        diameter: 8.,
        spacing: 0.25,
        wet: WetSettings::new(
            true,
            false,
            true,
            true,
            WetMix::new(0., 0., 0., 1., 1., 0.).unwrap(),
        ),
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
    .definition();
    b.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: finger,
        pressure_strength: pressure,
    };
    b.brush.color = color(1., 0., 1.);
    b
}
fn pixels(d: &Document) -> Vec<Pixel> {
    (0..64)
        .flat_map(|y| (0..128).map(move |x| d.try_pixel(x, y).unwrap()))
        .collect()
}
fn dab(
    d: &mut Document,
    b: BrushDefinition,
    retained: Option<&WetCarrier>,
    x: f64,
    pressure: f32,
) -> WetCarrier {
    d.begin_configured_wet_brush_stroke(b, InputPoint::new(x, 32.5, pressure), retained)
        .unwrap();
    d.end_wet_brush_stroke().unwrap().1
}
fn seed(d: &mut Document, ink: Pixel, x: f64) {
    d.begin_stroke(
        Brush {
            radius: 12.,
            color: ink,
            ..Default::default()
        },
        InputPoint::new(x, 32.5, 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
}
#[test]
fn foreground_loading_requires_finger_and_strength_pressure_scale_once() {
    for finger in [false, true] {
        for pressure_on in [false, true] {
            let mut d = Document::new(128, 64).unwrap();
            let mut b = definition(finger, pressure_on);
            b.brush.opacity = 0.4;
            b.tip.flow = 0.5;
            let carrier = dab(&mut d, b, None, 32.5, 0.5);
            let expected = if finger {
                0.2 * if pressure_on { 0.5 } else { 1. }
            } else {
                0.
            };
            assert!((d.pixel(32, 32).alpha() - expected).abs() < 1e-6);
            assert_eq!(
                carrier.sample(0.5, 0.5),
                if finger {
                    color(1., 0., 1.)
                } else {
                    Pixel::TRANSPARENT
                }
            );
            assert_eq!(d.history_depth().0, usize::from(finger));
        }
    }
}
#[test]
fn retained_ink_is_not_replaced_without_finger_and_transparency_is_transportable() {
    let mut d = Document::new(128, 64).unwrap();
    seed(&mut d, color(0., 1., 1.), 32.5);
    let before = pixels(&d);
    let mut b = definition(false, false);
    b.brush.opacity = 0.5;
    b.wet = WetSettings::new(true, false, false, true, b.wet.mix());
    let retained = WetCarrier::new(b.wet.mix(), 8., color(1., 0., 0.4)).unwrap();
    let ink = retained.sample(0.5, 0.5);
    let returned = dab(&mut d, b.clone(), Some(&retained), 32.5, 1.);
    for (actual, expected) in d
        .pixel(32, 32)
        .components()
        .into_iter()
        .zip([0.2, 0., 0.5, 0.7])
    {
        assert!((actual - expected).abs() < 1e-6);
    }
    assert_eq!(returned.sample(0.5, 0.5), ink);
    assert_eq!(retained.revision(), 0);
    let after = pixels(&d);
    d.undo().unwrap();
    assert_eq!(pixels(&d), before);
    d.redo().unwrap();
    assert_eq!(pixels(&d), after);
    let empty = WetCarrier::new(b.wet.mix(), 8., Pixel::TRANSPARENT).unwrap();
    let previous = d.pixel(32, 32);
    dab(&mut d, b, Some(&empty), 32.5, 1.);
    assert_eq!(d.pixel(32, 32), previous.scaled(0.5));
}
#[test]
fn current_position_pickup_moves_color_and_cancel_and_errors_restore_pixels_and_ink() {
    let mut d = Document::new(128, 64).unwrap();
    seed(&mut d, color(1., 0., 1.), 24.5);
    let before = pixels(&d);
    let depth = d.history_depth();
    let mut b = definition(false, false);
    b.wet = WetSettings::new(
        true,
        false,
        true,
        false,
        WetMix::new(1., 0.1, 0., 1., 1., 0.).unwrap(),
    );
    let original = WetCarrier::new(b.wet.mix(), 8., Pixel::TRANSPARENT).unwrap();
    d.begin_configured_wet_brush_stroke(
        b.clone(),
        InputPoint::new(24.5, 32.5, 1.),
        Some(&original),
    )
    .unwrap();
    d.stroke_to(InputPoint::new(96.5, 32.5, 1.)).unwrap();
    assert!(d.pixel(96, 32).alpha() > 0.);
    let (_, returned) = d.end_wet_brush_stroke().unwrap();
    assert!(returned.sample(0.5, 0.5).alpha() > 0.);
    let after = pixels(&d);
    d.undo().unwrap();
    assert_eq!(pixels(&d), before);
    d.redo().unwrap();
    assert_eq!(pixels(&d), after);
    for fail in [false, true] {
        d.begin_configured_wet_brush_stroke(
            b.clone(),
            InputPoint::new(24.5, 32.5, 1.),
            Some(&returned),
        )
        .unwrap();
        d.stroke_to(InputPoint::new(64.5, 32.5, 1.)).unwrap();
        if fail {
            assert!(d.stroke_to(InputPoint::new(f64::NAN, 0., 1.)).is_err());
        } else {
            d.cancel_stroke().unwrap();
        }
        assert_eq!(pixels(&d), after);
        assert_eq!(d.history_depth().0, depth.0 + 1);
    }
    assert_eq!(original.revision(), 0);
}
#[test]
fn tool_sampling_overrides_preset_and_only_changes_target_layer() {
    for all in [false, true] {
        let mut d = Document::new(128, 64).unwrap();
        seed(&mut d, color(1., 0., 1.), 32.5);
        let source = d.layer(1).unwrap().pixel(32, 32);
        let target = d.add_layer("target").unwrap();
        let mut b = definition(false, false);
        b.brush.mode = BrushMode::Smudge {
            all_layers: all,
            finger_painting: false,
            pressure_strength: false,
        };
        b.wet = WetSettings::new(
            true,
            !all,
            true,
            false,
            WetMix::new(1., 1., 0., 1., 1., 0.).unwrap(),
        );
        let returned = dab(&mut d, b, None, 32.5, 1.);
        assert_eq!(d.layer(1).unwrap().pixel(32, 32), source);
        assert_eq!(d.layer(target).unwrap().pixel(32, 32).alpha() > 0., all);
        assert_eq!(returned.sample(0.5, 0.5).alpha() > 0., all);
    }
}
#[test]
fn alpha_lock_preserves_exact_alpha_and_zero_strength_still_picks_up() {
    let mut d = Document::new(128, 64).unwrap();
    seed(&mut d, color(0., 1., 0.35), 32.5);
    let mut appearance = d.layer(1).unwrap().appearance();
    appearance.locks = LOCK_TRANSPARENCY;
    d.set_layer_appearance(1, appearance).unwrap();
    let before = pixels(&d);
    dab(&mut d, definition(true, false), None, 32.5, 1.);
    for (a, b) in before.into_iter().zip(pixels(&d)) {
        assert_eq!(a.alpha(), b.alpha());
    }
    assert!(d.pixel(32, 32).components()[0] > 0.);
    let mut b = definition(false, false);
    b.brush.opacity = 0.;
    b.wet = WetSettings::new(
        true,
        false,
        true,
        false,
        WetMix::new(1., 1., 0., 1., 1., 0.).unwrap(),
    );
    let before = pixels(&d);
    let depth = d.history_depth();
    let carried = dab(&mut d, b, None, 32.5, 1.);
    assert!(carried.sample(0.5, 0.5).alpha() > 0.);
    assert_eq!(pixels(&d), before);
    assert_eq!(d.history_depth(), depth);
}

#[test]
fn selection_does_not_clip_pickup_and_mask_visible_gray_roundtrips() {
    let mut d = Document::new(128, 64).unwrap();
    seed(&mut d, color(1., 0., 1.), 24.5);
    d.set_selection(
        Selection::default()
            .apply(
                SelectionShape::geometry(SelectionKind::Rectangle, 64., 0., 64., 64., false),
                SelectionOperation::Replace,
                128,
                64,
            )
            .unwrap(),
    )
    .unwrap();
    let original = d.layer(1).unwrap().pixel(24, 32);
    let mut b = definition(false, false);
    b.wet = WetSettings::new(
        true,
        false,
        true,
        false,
        WetMix::new(1., 0.1, 0., 1., 1., 0.).unwrap(),
    );
    d.begin_configured_wet_brush_stroke(b, InputPoint::new(24.5, 32.5, 1.), None)
        .unwrap();
    d.stroke_to(InputPoint::new(96.5, 32.5, 1.)).unwrap();
    d.end_wet_brush_stroke().unwrap();
    assert_eq!(d.layer(1).unwrap().pixel(24, 32), original);
    assert!(d.pixel(96, 32).alpha() > 0.);

    let mut d = Document::new(128, 64).unwrap();
    d.initialize_white_background().unwrap();
    let mask = d.add_mask(1).unwrap();
    let mut b = definition(true, false);
    b.brush.color = Pixel::from_straight([0., 0., 0., 1.]).unwrap();
    b.brush.opacity = 0.5;
    b.brush.radius = 12.;
    dab(&mut d, b.clone(), None, 32.5, 1.);
    assert_eq!(d.layer(mask).unwrap().pixel(32, 32).alpha(), 0.5);
    let before = pixels(&d);
    b.brush.radius = 4.;
    b.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: false,
        pressure_strength: false,
    };
    b.wet = WetSettings::new(
        true,
        true,
        true,
        false,
        WetMix::new(1., 1., 0., 1., 1., 0.).unwrap(),
    );
    let carried = dab(&mut d, b.clone(), None, 32.5, 1.);
    let sampled = carried.sample(0.5, 0.5).components();
    assert!((sampled[0] - 0.21404114).abs() < 1e-6);
    assert_eq!(sampled[0], sampled[1]);
    assert_eq!(sampled[1], sampled[2]);
    assert_eq!(sampled[3], 1.);
    for (a, b) in pixels(&d).into_iter().zip(&before) {
        for (a, b) in a.components().into_iter().zip(b.components()) {
            assert!((a - b).abs() < 1e-6);
        }
    }
    let before = pixels(&d);
    b.brush.mode = BrushMode::Smudge {
        all_layers: true,
        finger_painting: false,
        pressure_strength: false,
    };
    assert!(d
        .begin_configured_wet_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.), None)
        .is_err());
    assert!(!d.stroke_active());
    assert_eq!(pixels(&d), before);
}
