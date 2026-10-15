use paint_core::*;
use std::{collections::BTreeMap, sync::Arc};
fn rgba(c: [f32; 4]) -> Pixel {
    Pixel::from_straight(c).unwrap()
}
fn fixture(paged: bool) -> Document {
    let values = (0..4096)
        .map(|i| {
            if i / 64 < 32 {
                rgba([1., 0., 0., 1.])
            } else {
                rgba([0., 0., 1., 1.])
            }
        })
        .collect();
    Document::from_import(
        192,
        64,
        vec![ImportedLayer {
            name: "source".into(),
            properties: LayerProperties::default(),
            tiles: BTreeMap::from([(TileCoord { x: 0, y: 0 }, Tile::from_pixels(values).unwrap())]),
        }],
        0,
        DocumentOptions {
            max_resident_tiles: if paged { 1 } else { 4096 },
            ..Default::default()
        },
    )
    .unwrap()
}
fn brush() -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 6.,
            spacing: 0.5,
            opacity: 0.4,
            color: rgba([1., 0., 0., 1.]),
            ..Default::default()
        },
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn carrier(b: &BrushDefinition) -> WetCarrier {
    WetCarrier::new(
        WetMix::new(1., 0.15, 0., 0., 1., 0.).unwrap(),
        b.brush.radius * 2.,
        b.brush.color,
    )
    .unwrap()
}
fn pixels(d: &Document) -> Vec<Pixel> {
    (0..d.dimensions().1)
        .flat_map(|y| (0..d.dimensions().0).map(move |x| d.try_pixel(x, y).unwrap()))
        .collect()
}
fn local(d: &Document, id: LayerId) -> Vec<Pixel> {
    (0..64)
        .flat_map(|y| (0..192).map(move |x| d.layer(id).unwrap().try_pixel(x, y).unwrap()))
        .collect()
}
fn line(
    d: &mut Document,
    b: BrushDefinition,
    c: &WetCarrier,
    split: bool,
    offset: (f64, f64),
) -> WetCarrier {
    d.begin_wet_brush_stroke(
        b,
        InputPoint::new(32. + offset.0, 32. + offset.1, 1.),
        c,
        false,
    )
    .unwrap();
    if split {
        d.stroke_to(InputPoint::new(80. + offset.0, 32. + offset.1, 1.))
            .unwrap();
    }
    d.stroke_to(InputPoint::new(128. + offset.0, 32. + offset.1, 1.))
        .unwrap();
    let (changed, next) = d.end_wet_brush_stroke().unwrap();
    assert!(changed);
    next
}
#[test]
fn real_document_keeps_two_carried_colors_and_commits_one_undoable_stroke() {
    let mut d = fixture(false);
    let b = brush();
    let c = carrier(&b);
    let before = pixels(&d);
    let next = line(&mut d, b, &c, false, (0., 0.));
    assert_eq!(c.revision(), 0);
    assert_eq!(c.sample(0.5, 0.2), Pixel::TRANSPARENT);
    assert!(next.revision() > 1);
    let top = d.pixel(100, 28).components();
    let bottom = d.pixel(100, 36).components();
    assert!(top[0] > top[2] && top[3] > 0.);
    assert!(bottom[2] > bottom[0] && bottom[3] > 0.);
    assert_eq!(d.history_depth(), (1, 0));
    let after = pixels(&d);
    assert_ne!(after, before);
    d.undo().unwrap();
    assert_eq!(pixels(&d), before);
    d.redo().unwrap();
    assert_eq!(pixels(&d), after);
    d.begin_wet_brush_stroke(brush(), InputPoint::new(80., 32., 1.), &next, false)
        .unwrap();
    d.stroke_to(InputPoint::new(160., 32., 1.)).unwrap();
    d.cancel_stroke().unwrap();
    assert_eq!(pixels(&d), after);
    assert_eq!(d.history_depth(), (1, 0));
}
#[test]
fn paging_split_inputs_and_layer_offsets_preserve_local_pixels() {
    let mut a = fixture(false);
    let mut b = fixture(true);
    let definition = brush();
    let c = carrier(&definition);
    let first = line(&mut a, definition.clone(), &c, false, (0., 0.));
    let second = line(&mut b, definition.clone(), &c, true, (0., 0.));
    assert_eq!(pixels(&a), pixels(&b));
    assert_eq!(first.sample(0.25, 0.25), second.sample(0.25, 0.25));
    assert!(b.storage_stats().page_writes > 0);
    let mut shifted = fixture(true);
    shifted
        .set_layer_appearance(
            1,
            LayerAppearance {
                offset_x: 17,
                offset_y: -3,
                ..Default::default()
            },
        )
        .unwrap();
    line(&mut shifted, definition, &c, true, (17., -3.));
    assert_eq!(local(&a, 1), local(&shifted, 1));
}
#[test]
fn all_layer_pickup_only_writes_target_and_zero_opacity_still_picks() {
    for all in [false, true] {
        let mut d = fixture(false);
        let base = local(&d, 1);
        let target = d.add_layer("target").unwrap();
        let b = brush();
        let c = carrier(&b);
        let depth = d.history_depth();
        d.begin_wet_brush_stroke(b, InputPoint::new(32., 32., 1.), &c, all)
            .unwrap();
        let (changed, next) = d.end_wet_brush_stroke().unwrap();
        assert_eq!(changed, all);
        assert_eq!(local(&d, 1), base);
        assert_eq!(next.sample(0.5, 0.25).alpha() > 0., all);
        assert_eq!(d.layer(target).unwrap().pixel(32, 28).alpha() > 0., all);
        assert_eq!(d.history_depth().0, depth.0 + usize::from(all));
    }
    let mut d = fixture(false);
    let before = pixels(&d);
    let mut b = brush();
    b.brush.opacity = 0.;
    let c = carrier(&b);
    d.begin_wet_brush_stroke(b, InputPoint::new(32., 32., 1.), &c, false)
        .unwrap();
    let (changed, next) = d.end_wet_brush_stroke().unwrap();
    assert!(!changed);
    assert!(next.sample(0.5, 0.25).alpha() > 0.);
    assert_eq!(pixels(&d), before);
    assert_eq!(d.history_depth(), (0, 0));
}
#[test]
fn selection_limits_writes_without_clipping_pickup_and_pressure_ceiling_is_not_squared() {
    let mut d = fixture(false);
    d.set_selection(
        Selection::default()
            .apply(
                SelectionShape::geometry(SelectionKind::Rectangle, 64., 0., 128., 64., false),
                SelectionOperation::Replace,
                192,
                64,
            )
            .unwrap(),
    )
    .unwrap();
    let b = brush();
    let c = carrier(&b);
    line(&mut d, b, &c, false, (0., 0.));
    assert_eq!(d.pixel(20, 28), rgba([1., 0., 0., 1.]));
    assert!(d.pixel(80, 28).alpha() > 0.);
    let mut d = Document::new(64, 64).unwrap();
    d.set_selection(
        Selection::default()
            .apply(
                SelectionShape::geometry(SelectionKind::Rectangle, 32.5, 0., 31.5, 64., true),
                SelectionOperation::Replace,
                64,
                64,
            )
            .unwrap(),
    )
    .unwrap();
    let mut b = brush();
    b.brush.opacity = 0.4;
    b.dynamics.pressure_opacity = true;
    let c = WetCarrier::new(
        WetMix::new(0., 0., 0., 1., 1., 0.).unwrap(),
        12.,
        b.brush.color,
    )
    .unwrap();
    d.begin_wet_brush_stroke(b, InputPoint::new(32.5, 32.5, 0.5), &c, false)
        .unwrap();
    assert!((d.pixel(32, 32).alpha() - 0.1).abs() < 1e-6);
    d.stroke_to(InputPoint::new(35.5, 32.5, 0.25)).unwrap();
    d.stroke_to(InputPoint::new(32.5, 32.5, 0.25)).unwrap();
    d.end_wet_brush_stroke().unwrap();
    assert!((d.pixel(32, 32).alpha() - 0.1).abs() < 1e-6);
}
#[test]
fn masks_use_visible_gray_and_transparency_lock_preserves_exact_alpha() {
    let mut d = Document::new(64, 64).unwrap();
    d.initialize_white_background().unwrap();
    let mask = d.add_mask(1).unwrap();
    let mut b = brush();
    b.brush.color = rgba([0., 0., 0., 1.]);
    let c = WetCarrier::new(
        WetMix::new(0., 0., 0., 1., 1., 0.).unwrap(),
        12.,
        b.brush.color,
    )
    .unwrap();
    d.begin_wet_brush_stroke(b.clone(), InputPoint::new(32.5, 32.5, 1.), &c, false)
        .unwrap();
    d.end_wet_brush_stroke().unwrap();
    assert!((d.layer(mask).unwrap().pixel(32, 32).alpha() - 0.4).abs() < 1e-6);
    assert!((d.pixel(32, 32).alpha() - 0.6).abs() < 1e-6);
    let old = pixels(&d);
    assert!(d
        .begin_wet_brush_stroke(b.clone(), InputPoint::new(32.5, 32.5, 1.), &c, true)
        .is_err());
    assert_eq!(pixels(&d), old);
    let empty = WetCarrier::new(
        WetMix::new(1., 1., 0., 0., 0., 0.).unwrap(),
        12.,
        Pixel::TRANSPARENT,
    )
    .unwrap();
    d.begin_wet_brush_stroke(b, InputPoint::new(16., 16., 1.), &empty, false)
        .unwrap();
    let (changed, next) = d.end_wet_brush_stroke().unwrap();
    assert!(!changed);
    assert_eq!(next.sample(0.5, 0.5), Pixel::WHITE);
    let mut d = Document::new(64, 64).unwrap();
    d.begin_stroke(
        Brush {
            radius: 20.,
            color: rgba([0.3, 0.4, 0.5, 0.3]),
            ..Default::default()
        },
        InputPoint::new(32.5, 32.5, 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    let before = pixels(&d);
    d.set_layer_appearance(
        1,
        LayerAppearance {
            locks: LOCK_TRANSPARENCY,
            ..Default::default()
        },
    )
    .unwrap();
    let b = brush();
    let c = WetCarrier::new(
        WetMix::new(0., 0., 0., 1., 1., 0.).unwrap(),
        12.,
        b.brush.color,
    )
    .unwrap();
    d.begin_wet_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.), &c, false)
        .unwrap();
    d.end_wet_brush_stroke().unwrap();
    for (old, new) in before.into_iter().zip(pixels(&d)) {
        assert_eq!(old.alpha(), new.alpha());
    }
}
#[test]
fn dry_loaded_wet_stroke_agrees_with_plain_blending_for_all_27_modes() {
    let modes = [
        BlendMode::Normal,
        BlendMode::Dissolve,
        BlendMode::Darken,
        BlendMode::Multiply,
        BlendMode::ColorBurn,
        BlendMode::LinearBurn,
        BlendMode::DarkerColor,
        BlendMode::Lighten,
        BlendMode::Screen,
        BlendMode::ColorDodge,
        BlendMode::LinearDodge,
        BlendMode::LighterColor,
        BlendMode::Overlay,
        BlendMode::SoftLight,
        BlendMode::HardLight,
        BlendMode::VividLight,
        BlendMode::LinearLight,
        BlendMode::PinLight,
        BlendMode::HardMix,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Subtract,
        BlendMode::Divide,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
    ];
    for mode in modes {
        let mut a = fixture(false);
        let mut b = fixture(false);
        let mut definition = brush();
        definition.brush.color = rgba([0.2, 0.8, 0.4, 1.]);
        definition.write_blend = mode;
        let c = WetCarrier::new(
            WetMix::new(0., 0., 0., 1., 1., 0.).unwrap(),
            12.,
            definition.brush.color,
        )
        .unwrap();
        a.begin_wet_brush_stroke(
            definition.clone(),
            InputPoint::new(32.5, 32.5, 1.),
            &c,
            false,
        )
        .unwrap();
        a.end_wet_brush_stroke().unwrap();
        b.begin_brush_stroke(definition, InputPoint::new(32.5, 32.5, 1.))
            .unwrap();
        b.end_stroke().unwrap();
        for (x, y) in pixels(&a).into_iter().zip(pixels(&b)) {
            for (x, y) in x.components().into_iter().zip(y.components()) {
                assert!((x - y).abs() < 2e-6, "{mode:?}: {x} != {y}");
            }
        }
    }
}
#[test]
fn image_texture_scatter_and_dual_paths_share_paged_wet_execution() {
    for smudge in [false, true] {
        for bits in 0..16 {
            let mut a = fixture(false);
            let mut b = fixture(true);
            let mut definition = brush();
            if smudge {
                definition.brush.mode = BrushMode::Smudge {
                    all_layers: false,
                    finger_painting: false,
                    pressure_strength: true,
                };
                definition.dynamics.color =
                    ColorDynamics::new(true, 0.2, 0.1, 0.1, 0.1, true, 71).unwrap();
            }
            if bits & 1 != 0 {
                definition.mask = Some(Arc::new(
                    BrushMask::new(4, 2, vec![255, 0, 255, 255, 255, 255, 0, 255]).unwrap(),
                ));
            }
            if bits & 2 != 0 {
                definition.texture = BrushTexture::new(
                    true,
                    TextureSettings::new(
                        if bits & 1 == 0 {
                            TextureSpace::Document
                        } else {
                            TextureSpace::Tip
                        },
                        1.,
                        0.,
                        0.6,
                        false,
                        0.,
                        0.,
                    )
                    .unwrap(),
                    Some(Arc::new(TextureResource::new(Arc::new(
                        BrushMask::new(2, 2, vec![128, 255, 255, 128]).unwrap(),
                    )))),
                )
                .unwrap();
            }
            if bits & 4 != 0 {
                definition.dynamics.scatter =
                    ScatterDynamics::new(true, 0.2, true, 2, 0., 7).unwrap();
            }
            if bits & 8 != 0 {
                definition.dual = DualBrush::new(
                    true,
                    DualMode::Screen,
                    SecondaryTip::new(
                        8.,
                        BrushTip {
                            pressure_size: false,
                            pressure_flow: false,
                            ..Default::default()
                        },
                        None,
                        Default::default(),
                    )
                    .unwrap()
                    .with_spacing(0.25)
                    .unwrap(),
                );
            }
            let c = carrier(&definition);
            line(&mut a, definition.clone(), &c, false, (0., 0.));
            line(&mut b, definition, &c, true, (0., 0.));
            assert_eq!(pixels(&a), pixels(&b), "combination {bits}");
            assert_eq!(a.history_depth(), (1, 0));
        }
    }
}

#[test]
fn zero_pressure_footprints_and_rejected_segments_are_atomic() {
    let mut d = Document::new(64, 64).unwrap();
    let mut b = brush();
    b.brush.radius = 0.5;
    b.tip.pressure_size = true;
    let c = WetCarrier::new(WetMix::default(), 1., b.brush.color).unwrap();
    d.begin_wet_brush_stroke(b, InputPoint::new(32., 32., 0.01), &c, false)
        .unwrap();
    d.end_wet_brush_stroke().unwrap();
    let mut d = fixture(false);
    let before = pixels(&d);
    let b = brush();
    let c = carrier(&b);
    d.begin_wet_brush_stroke(b, InputPoint::new(32., 32., 1.), &c, false)
        .unwrap();
    let mut bad = InputPoint::new(40., 32., 1.);
    bad.tool = Tool::Eraser;
    assert!(d.stroke_to(bad).is_err());
    assert!(!d.stroke_active());
    assert_eq!(pixels(&d), before);
    assert_eq!(d.history_depth(), (0, 0));
    assert_eq!(c.revision(), 0);
    let mut b = brush();
    b.brush.spacing = 0.02;
    let c = carrier(&b);
    d.begin_wet_brush_stroke(b, InputPoint::new(32., 32., 1.), &c, false)
        .unwrap();
    assert!(d.stroke_to(InputPoint::new(1_000_000., 32., 1.)).is_err());
    assert!(!d.stroke_active());
    assert_eq!(pixels(&d), before);
}

#[test]
fn smoothing_uses_the_same_filtered_timeline_and_keeps_wet_undo_exact() {
    let mut a = fixture(false);
    let mut b = fixture(true);
    let mut definition = brush();
    definition.smoothing = BrushSmoothing::new(true, 12., 8., 0.5, 2.).unwrap();
    let c = carrier(&definition);
    let point = |x: f64, t: u64| {
        let mut p = InputPoint::new(x, 32., 1.);
        p.timestamp_ns = t;
        p
    };
    for (d, split) in [(&mut a, false), (&mut b, true)] {
        d.begin_wet_brush_stroke(definition.clone(), point(32., 0), &c, false)
            .unwrap();
        if split {
            d.stroke_to(point(80., 48_000_000)).unwrap();
        }
        d.stroke_to(point(128., 96_000_000)).unwrap();
        d.end_wet_brush_stroke().unwrap();
    }
    assert_eq!(pixels(&a), pixels(&b));
    let after = pixels(&b);
    b.undo().unwrap();
    assert_eq!(pixels(&b), pixels(&fixture(false)));
    b.redo().unwrap();
    assert_eq!(pixels(&b), after);
}
#[test]
fn off_canvas_contact_and_zero_flow_pickup_follow_the_same_rules() {
    let mut d = fixture(false);
    let mut definition = brush();
    definition.tip.flow = 0.;
    let c = carrier(&definition);
    let before = pixels(&d);
    d.begin_wet_brush_stroke(definition, InputPoint::new(32., 32., 1.), &c, false)
        .unwrap();
    let (changed, loaded) = d.end_wet_brush_stroke().unwrap();
    assert!(!changed);
    assert_eq!(pixels(&d), before);
    assert!(loaded.sample(0.5, 0.25).alpha() > 0.);
    let mut d = Document::new(64, 64).unwrap();
    let definition = brush();
    let c = WetCarrier::new(
        WetMix::new(1., 1., 0., 1., 1., 0.).unwrap(),
        12.,
        definition.brush.color,
    )
    .unwrap();
    d.begin_wet_brush_stroke(definition, InputPoint::new(-20., 32., 1.), &c, false)
        .unwrap();
    let (changed, empty) = d.end_wet_brush_stroke().unwrap();
    assert!(!changed);
    assert_eq!(empty.sample(0.5, 0.5), Pixel::TRANSPARENT);
    assert_eq!(c.sample(0.5, 0.5), c.sample(0.25, 0.25));
    assert_eq!(c.revision(), 0);
}

#[test]
fn visible_composite_and_raw_single_layer_see_the_same_updated_dabs() {
    let mut a = Document::new(64, 64).unwrap();
    let mut b = Document::new(64, 64).unwrap();
    let definition = brush();
    let c = WetCarrier::new(
        WetMix::new(1., 0.5, 0., 1., 1., 0.).unwrap(),
        12.,
        definition.brush.color,
    )
    .unwrap();
    for (d, all) in [(&mut a, false), (&mut b, true)] {
        d.begin_wet_brush_stroke(definition.clone(), InputPoint::new(32., 32., 1.), &c, all)
            .unwrap();
        d.stroke_to(InputPoint::new(38., 32., 1.)).unwrap();
        d.stroke_to(InputPoint::new(32., 32., 1.)).unwrap();
        d.end_wet_brush_stroke().unwrap();
    }
    assert_eq!(pixels(&a), pixels(&b));
    assert!(a.pixel(32, 32).alpha() > 0.);
}

#[test]
fn explicit_wet_end_keeps_tool_state_and_plain_end_contract_separate() {
    let mut d = Document::new(64, 64).unwrap();
    let b = brush();
    let c = WetCarrier::new(
        WetMix::new(0., 0., 0., 1., 1., 0.).unwrap(),
        12.,
        b.brush.color,
    )
    .unwrap();
    d.begin_wet_brush_stroke(b.clone(), InputPoint::new(32., 32., 1.), &c, false)
        .unwrap();
    assert!(d.end_stroke().is_err());
    assert!(d.stroke_active());
    let (changed, next) = d.end_wet_brush_stroke().unwrap();
    assert!(changed);
    assert!(next.revision() > 0);
    assert_eq!(c.revision(), 0);
    d.begin_brush_stroke(b, InputPoint::new(48., 32., 1.))
        .unwrap();
    assert!(d.end_wet_brush_stroke().is_err());
    assert!(d.stroke_active());
    assert!(d.end_stroke().unwrap());
    assert_eq!(d.history_depth(), (2, 0));
}
