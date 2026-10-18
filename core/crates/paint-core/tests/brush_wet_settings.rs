use paint_core::*;
fn red() -> Pixel {
    Pixel::from_straight([1., 0., 0., 1.]).unwrap()
}
fn blue() -> Pixel {
    Pixel::from_straight([0., 0., 1., 1.]).unwrap()
}
fn definition(clean: bool, load: bool, mix: WetMix) -> BrushDefinition {
    BrushSettings {
        diameter: 8.,
        spacing: 0.25,
        wet: WetSettings::new(true, false, clean, load, mix),
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
    .definition()
}
fn dry_mix() -> WetMix {
    WetMix::new(0., 0., 0., 0.25, 1., 0.).unwrap()
}
fn pixels(doc: &Document) -> Vec<Pixel> {
    (0..64)
        .flat_map(|y| (0..128).map(move |x| doc.try_pixel(x, y).unwrap()))
        .collect()
}
fn dab(
    doc: &mut Document,
    b: BrushDefinition,
    previous: Option<&WetCarrier>,
    x: f64,
) -> WetCarrier {
    doc.begin_configured_wet_brush_stroke(b, InputPoint::new(x, 32.5, 1.), previous)
        .unwrap();
    doc.end_wet_brush_stroke().unwrap().1
}
#[test]
fn four_clean_load_policies_use_private_ink_and_capture_foreground() {
    for clean in [false, true] {
        for load in [false, true] {
            let mix = dry_mix();
            let retained = WetCarrier::new(mix, 8., red()).unwrap();
            let before = retained.sample(0.5, 0.5);
            let mut b = definition(clean, load, mix);
            b.brush.color = blue();
            let mut doc = Document::new(128, 64).unwrap();
            doc.begin_configured_wet_brush_stroke(
                b.clone(),
                InputPoint::new(24.5, 32.5, 1.),
                Some(&retained),
            )
            .unwrap();
            b.brush.color = red();
            b.wet = WetSettings::default();
            let (changed, returned) = doc.end_wet_brush_stroke().unwrap();
            let expected = if load {
                blue().scaled(0.25)
            } else if clean {
                Pixel::TRANSPARENT
            } else {
                before
            };
            assert_eq!(doc.pixel(24, 32), expected);
            assert_eq!(returned.sample(0.5, 0.5), expected);
            assert_eq!(retained.sample(0.5, 0.5), before);
            assert_eq!(retained.revision(), 0);
            assert_eq!(changed, load || !clean);
            if changed {
                let after = pixels(&doc);
                doc.undo().unwrap();
                assert!(pixels(&doc).iter().all(|p| *p == Pixel::TRANSPARENT));
                doc.redo().unwrap();
                assert_eq!(pixels(&doc), after);
            }
        }
    }
}
#[test]
fn retained_carrier_resets_air_distance_and_reconfigures_load_without_rescaling() {
    let mix = WetMix::new(0., 0., 0., 0.5, 1., 1.).unwrap();
    let retained = WetCarrier::new(mix, 8., red()).unwrap();
    let mut doc = Document::new(128, 64).unwrap();
    let mut b = definition(false, false, mix);
    let first = dab(&mut doc, b.clone(), Some(&retained), 16.5);
    assert_eq!(doc.pixel(16, 32), red().scaled(0.5));
    b.brush.radius = 6.;
    b.wet = WetSettings::new(
        true,
        false,
        false,
        false,
        WetMix::new(0., 0., 0., 0.1, 1., 1.).unwrap(),
    );
    let second = dab(&mut doc, b.clone(), Some(&first), 104.5);
    assert_eq!(doc.pixel(104, 32), red().scaled(0.5));
    assert_eq!(second.edge(), 12);
    assert_eq!(second.settings().load(), 0.1);
    assert_eq!(first.edge(), 8);
    assert_eq!(first.settings().load(), 0.5);
    assert_eq!(doc.history_depth(), (2, 0));
    b.wet = WetSettings::new(true, false, false, true, second.settings());
    b.brush.color = blue();
    dab(&mut doc, b, Some(&second), 64.5);
    assert_eq!(doc.pixel(64, 32), blue().scaled(0.1));
}
#[test]
fn empty_initial_carrier_picks_visible_ink_and_cancel_preserves_both_states() {
    let mut doc = Document::new(128, 64).unwrap();
    let seed = BrushDefinition {
        brush: Brush {
            radius: 12.,
            color: red(),
            ..Default::default()
        },
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    };
    doc.begin_brush_stroke(seed, InputPoint::new(24.5, 32.5, 1.))
        .unwrap();
    doc.end_stroke().unwrap();
    let before = pixels(&doc);
    let depth = doc.history_depth();
    let b = definition(false, false, WetMix::new(1., 0.1, 0., 1., 1., 0.).unwrap());
    doc.begin_configured_wet_brush_stroke(b.clone(), InputPoint::new(24.5, 32.5, 1.), None)
        .unwrap();
    doc.stroke_to(InputPoint::new(96.5, 32.5, 1.)).unwrap();
    let (_, retained) = doc.end_wet_brush_stroke().unwrap();
    assert!(doc.pixel(96, 32).alpha() > 0.);
    assert!(retained.sample(0.5, 0.5).alpha() > 0.);
    let after = pixels(&doc);
    doc.undo().unwrap();
    assert_eq!(pixels(&doc), before);
    doc.redo().unwrap();
    assert_eq!(pixels(&doc), after);
    let original = retained.sample(0.5, 0.5);
    let revision = retained.revision();
    let mut next = definition(true, true, dry_mix());
    next.brush.color = blue();
    doc.begin_configured_wet_brush_stroke(next, InputPoint::new(64.5, 48.5, 1.), Some(&retained))
        .unwrap();
    assert_ne!(pixels(&doc), after);
    doc.cancel_stroke().unwrap();
    assert_eq!(pixels(&doc), after);
    assert_eq!(retained.sample(0.5, 0.5), original);
    assert_eq!(retained.revision(), revision);
    assert_eq!(doc.history_depth().0, depth.0 + 1);
    doc.begin_configured_wet_brush_stroke(b, InputPoint::new(48.5, 32.5, 1.), Some(&retained))
        .unwrap();
    assert!(doc.stroke_to(InputPoint::new(f64::NAN, 0., 1.)).is_err());
    assert!(!doc.stroke_active());
    assert_eq!(pixels(&doc), after);
    assert_eq!(retained.revision(), revision);
}
#[test]
fn reservoir_reconfiguration_resamples_cell_centers_and_preserves_existing_amount() {
    let mix = WetMix::new(1., 1., 0., 1., 1., 0.).unwrap();
    let mut previous = WetCarrier::new(mix, 4., Pixel::TRANSPARENT).unwrap();
    let prepared = previous
        .prepare_dab(
            WetDabGeometry {
                x: 0.,
                y: 0.,
                diameter: 4.,
                roundness: 1.,
                angle: 0.,
            },
            |x, y| {
                Ok(WetSample {
                    color: Pixel::from_straight([
                        ((x + 2.) / 4.) as f32,
                        ((y + 2.) / 4.) as f32,
                        0.,
                        0.5,
                    ])
                    .unwrap(),
                    pickup_weight: 1.,
                })
            },
        )
        .unwrap();
    previous.commit(prepared).unwrap();
    let settings = WetMix::new(0., 0., 0., 0., 0.5, 1.).unwrap();
    let resized = previous.reconfigured(settings, 7.).unwrap();
    for y in 0..7 {
        for x in 0..7 {
            let u = (x as f64 + 0.5) / 7.;
            let v = (y as f64 + 0.5) / 7.;
            assert_eq!(resized.sample(u, v), previous.sample(u, v));
        }
    }
    let same = previous.reconfigured(settings, 4.).unwrap();
    for y in 0..4 {
        for x in 0..4 {
            let u = (x as f64 + 0.5) / 4.;
            let v = (y as f64 + 0.5) / 4.;
            assert_eq!(same.sample(u, v), previous.sample(u, v));
        }
    }
    assert_eq!(resized.settings(), settings);
    assert_eq!(previous.settings(), mix);
    for d in [f32::NAN, 0., 2000.25] {
        assert!(previous.reconfigured(settings, d).is_err());
    }
}
#[test]
fn catalog_snapshot_closed_values_and_entry_contracts_are_explicit() {
    let mut catalog = BrushCatalog::default();
    let closed = WetSettings::new(false, true, false, false, dry_mix());
    let mut settings = catalog.selected().settings().clone();
    settings.wet = closed;
    catalog.edit("round-pressure", settings.clone()).unwrap();
    let captured = catalog.selected();
    assert_eq!(captured.program().wet_settings(), closed);
    settings.wet = WetSettings::new(true, false, true, true, dry_mix());
    catalog.edit("round-pressure", settings.clone()).unwrap();
    assert_eq!(captured.settings().wet, closed);
    let active = catalog.selected().settings().definition();
    let mut doc = Document::new(128, 64).unwrap();
    let first = InputPoint::new(32.5, 32.5, 1.);
    assert!(doc.begin_brush_stroke(active.clone(), first).is_err());
    let carrier = WetCarrier::new(dry_mix(), 24., red()).unwrap();
    assert!(doc
        .begin_wet_brush_stroke(active.clone(), first, &carrier, false)
        .is_err());
    assert!(!doc.stroke_active());
    assert_eq!(doc.history_depth(), (0, 0));
    let mut invalid = active.clone();
    invalid.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: false,
        pressure_strength: false,
    };
    assert!(BrushProgram::compile(invalid).is_ok());
    let mut invalid = active.clone();
    invalid.brush.mode = BrushMode::Erase;
    assert!(BrushProgram::compile(invalid).is_err());
    let mut invalid = active.clone();
    invalid.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(8., BrushTip::default(), None, Default::default()).unwrap(),
    )
    .with_color(
        DualColor::new(
            true,
            true,
            true,
            DualColorOrder::SecondBehind,
            BlendMode::Normal,
            BlendMode::Normal,
        )
        .unwrap(),
    );
    assert!(BrushProgram::compile(invalid.clone()).is_err());
    invalid.wet = closed;
    assert!(BrushProgram::compile(invalid).is_ok());
    assert!(doc
        .begin_configured_wet_brush_stroke(captured.settings().definition(), first, None)
        .is_err());
    let mut enabled = active;
    enabled.wet = WetSettings::new(true, true, true, true, dry_mix());
    doc.add_mask(1).unwrap();
    assert!(doc
        .begin_configured_wet_brush_stroke(enabled, first, Some(&carrier))
        .is_err());
    assert_eq!(carrier.revision(), 0);
    assert!(!doc.stroke_active());
}

#[test]
fn configured_line_matches_explicit_snapshot_and_closed_policy_keeps_plain_pixels() {
    let mut b = definition(true, true, dry_mix());
    b.brush.color = blue();
    let captured = b.clone();
    let carrier = WetCarrier::new(dry_mix(), 8., blue()).unwrap();
    let mut configured = Document::new(128, 64).unwrap();
    let mut explicit = Document::new(128, 64).unwrap();
    configured
        .begin_configured_wet_brush_stroke(b.clone(), InputPoint::new(16.5, 32.5, 1.), None)
        .unwrap();
    b.brush.color = red();
    b.wet = WetSettings::new(true, false, true, true, WetMix::default());
    let mut raw = captured;
    raw.wet = Default::default();
    explicit
        .begin_wet_brush_stroke(raw, InputPoint::new(16.5, 32.5, 1.), &carrier, false)
        .unwrap();
    for doc in [&mut configured, &mut explicit] {
        doc.stroke_to(InputPoint::new(96.5, 32.5, 1.)).unwrap();
        doc.end_wet_brush_stroke().unwrap();
    }
    assert_eq!(pixels(&configured), pixels(&explicit));
    assert!(pixels(&configured).iter().all(|p| p.components()[0] == 0.));
    let mut normal = Document::new(128, 64).unwrap();
    let mut closed = Document::new(128, 64).unwrap();
    b.wet = WetSettings::new(false, true, false, false, dry_mix());
    closed
        .begin_brush_stroke(b.clone(), InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    closed.end_stroke().unwrap();
    b.wet = Default::default();
    normal
        .begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    normal.end_stroke().unwrap();
    assert_eq!(pixels(&closed), pixels(&normal));
}

#[test]
fn configured_sampling_policy_uses_visible_layers_and_only_writes_target() {
    for all in [false, true] {
        let mut doc = Document::new(128, 64).unwrap();
        doc.begin_stroke(
            Brush {
                radius: 12.,
                color: red(),
                ..Default::default()
            },
            InputPoint::new(32.5, 32.5, 1.),
        )
        .unwrap();
        doc.end_stroke().unwrap();
        let source = doc.layer(1).unwrap().pixel(32, 32);
        let target = doc.add_layer("target").unwrap();
        let depth = doc.history_depth();
        let mut b = definition(true, false, WetMix::new(1., 1., 0., 1., 1., 0.).unwrap());
        b.wet = WetSettings::new(true, all, true, false, b.wet.mix());
        doc.begin_configured_wet_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.), None)
            .unwrap();
        let (changed, next) = doc.end_wet_brush_stroke().unwrap();
        assert_eq!(changed, all);
        assert_eq!(doc.layer(1).unwrap().pixel(32, 32), source);
        assert_eq!(doc.layer(target).unwrap().pixel(32, 32).alpha() > 0., all);
        assert_eq!(next.sample(0.5, 0.5).alpha() > 0., all);
        assert_eq!(doc.history_depth().0, depth.0 + usize::from(all));
    }
}
