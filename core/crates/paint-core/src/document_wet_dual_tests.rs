use super::*;
use crate::*;
fn def(path: bool, front: bool, overflow: bool, mode: BlendMode) -> BrushDefinition {
    let tip = BrushTip {
        pressure_size: false,
        pressure_flow: false,
        flow: 1.,
        ..Default::default()
    };
    let second = SecondaryTip::new(7., tip, None, Default::default()).unwrap();
    let second = if path {
        second.with_spacing(0.5).unwrap()
    } else {
        second
    };
    BrushDefinition {
        brush: Brush {
            radius: 4.,
            spacing: 0.5,
            opacity: 0.4,
            color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
            background_color: Pixel::from_straight([0., 0., 1., 1.]).unwrap(),
            ..Default::default()
        },
        tip,
        dual: DualBrush::new(true, DualMode::Multiply, second).with_color(
            DualColor::new(
                true,
                true,
                overflow,
                if front {
                    DualColorOrder::SecondInFront
                } else {
                    DualColorOrder::SecondBehind
                },
                mode,
                mode,
            )
            .unwrap(),
        ),
        wet: WetSettings::new(
            true,
            false,
            true,
            true,
            WetMix::new(0., 0., 0., 1., 1., 0.).unwrap(),
        ),
        write_blend: mode,
        ..Default::default()
    }
}
fn pixels(doc: &Document) -> Vec<Pixel> {
    (0..doc.height)
        .flat_map(|y| (0..doc.width).map(move |x| doc.try_pixel(x, y).unwrap()))
        .collect()
}
fn same_ink(a: &WetInk, b: &WetInk) {
    for (a, b) in [
        (a.primary(), b.primary()),
        (a.secondary().unwrap(), b.secondary().unwrap()),
    ] {
        assert_eq!(a.revision(), b.revision());
        for y in 0..9 {
            for x in 0..9 {
                assert_eq!(
                    a.sample(x as f64 / 8., y as f64 / 8.),
                    b.sample(x as f64 / 8., y as f64 / 8.)
                );
            }
        }
    }
}
fn draw(
    doc: &mut Document,
    d: BrushDefinition,
    points: &[InputPoint],
    retained: Option<&WetInk>,
) -> WetInk {
    doc.begin_configured_dual_wet_brush_stroke(d, points[0], retained)
        .unwrap();
    for &p in &points[1..] {
        doc.stroke_to(p).unwrap();
    }
    doc.end_wet_ink_stroke().unwrap().1
}
fn points() -> Vec<InputPoint> {
    [(20.5, 30.5), (46.5, 30.5), (70.5, 42.5), (90.5, 42.5)]
        .into_iter()
        .map(|(x, y)| InputPoint::new(x, y, 1.))
        .collect()
}

#[test]
fn wet_dual_loaded_constant_ink_matches_dry_for_all_modes_paths_orders_and_overflow() {
    for mode in 0..27 {
        for path in [false, true] {
            for front in [false, true] {
                for overflow in [false, true] {
                    let mut wet = Document::new(32, 32).unwrap();
                    let mut dry = Document::new(32, 32).unwrap();
                    wet.initialize_white_background().unwrap();
                    dry.initialize_white_background().unwrap();
                    let d = def(path, front, overflow, BlendMode::from_id(mode).unwrap());
                    let mut ordinary = d.clone();
                    ordinary.wet = WetSettings::default();
                    let points = [
                        InputPoint::new(12.5, 16.5, 1.),
                        InputPoint::new(20.5, 16.5, 1.),
                    ];
                    let ink = draw(&mut wet, d, &points, None);
                    dry.begin_brush_stroke(ordinary, points[0]).unwrap();
                    dry.stroke_to(points[1]).unwrap();
                    dry.end_stroke().unwrap();
                    for (a, b) in pixels(&wet).iter().zip(pixels(&dry)) {
                        for (a, b) in a.components().iter().zip(b.components()) {
                            assert!((a-b).abs()<0.000002,"mode {mode} path {path} front {front} overflow {overflow}: {a} {b}");
                        }
                    }
                    assert_eq!(
                        ink.primary().sample(0.5, 0.5).components(),
                        [1., 0., 0., 1.]
                    );
                    assert_eq!(
                        ink.secondary().unwrap().sample(0.5, 0.5).components(),
                        [0., 0., 1., 1.]
                    );
                }
            }
        }
    }
}

#[test]
fn wet_dual_opacity_end_contract_retained_colors_and_single_history_are_real() {
    for front in [false, true] {
        let mut doc = Document::new(32, 32).unwrap();
        let d = def(false, front, true, BlendMode::Normal);
        let mut changed = d.clone();
        changed.brush.opacity = 0.2;
        doc.begin_configured_dual_wet_brush_stroke(changed, InputPoint::new(16.5, 16.5, 1.), None)
            .unwrap();
        assert!(doc.end_wet_brush_stroke().is_err());
        assert!(doc.stroke_active());
        assert!(doc.end_stroke().is_err());
        for _ in 0..4 {
            doc.stroke_to(InputPoint::new(20.5, 16.5, 1.)).unwrap();
            doc.stroke_to(InputPoint::new(16.5, 16.5, 1.)).unwrap();
        }
        let (_, ink) = doc.end_wet_ink_stroke().unwrap();
        let image = pixels(&doc);
        assert!(image.iter().all(|p| p.alpha() <= 0.200001));
        assert_eq!(doc.history_depth(), (1, 0));
        let center = doc.pixel(16, 16).components();
        assert!(if front {
            center[2] > center[0]
        } else {
            center[0] > center[2]
        });
        assert!(doc.pixel(10, 16).components()[2] > 0.);
        doc.undo().unwrap();
        assert!(pixels(&doc).iter().all(|p| *p == Pixel::TRANSPARENT));
        doc.redo().unwrap();
        assert_eq!(pixels(&doc), image);
        let mut retained = d;
        retained.wet = WetSettings::new(
            true,
            false,
            false,
            false,
            WetMix::new(0., 0., 0., 0.1, 1., 0.).unwrap(),
        );
        retained.brush.color = Pixel::WHITE;
        retained.brush.background_color = Pixel::WHITE;
        let after = draw(
            &mut doc,
            retained,
            &[InputPoint::new(16.5, 8.5, 1.)],
            Some(&ink),
        );
        assert_eq!(
            after.primary().sample(0.5, 0.5),
            ink.primary().sample(0.5, 0.5)
        );
        assert_eq!(
            after.secondary().unwrap().sample(0.5, 0.5),
            ink.secondary().unwrap().sample(0.5, 0.5)
        );
    }
}

#[test]
fn wet_dual_paged_taper_matches_known_endpoint_for_masks_selection_lock_and_images() {
    for target in 0..4 {
        for path in [false, true] {
            let seed = |resident| {
                let mut d = Document::with_options(
                    128,
                    64,
                    DocumentOptions {
                        max_resident_tiles: resident,
                        ..Default::default()
                    },
                )
                .unwrap();
                d.initialize_white_background().unwrap();
                if target == 1 {
                    d.add_mask(1).unwrap();
                }
                if target == 2 {
                    d.set_selection(
                        Selection::default()
                            .apply(
                                SelectionShape::geometry(
                                    SelectionKind::Ellipse,
                                    25.,
                                    12.,
                                    65.,
                                    38.,
                                    true,
                                ),
                                SelectionOperation::Replace,
                                128,
                                64,
                            )
                            .unwrap(),
                    )
                    .unwrap();
                }
                if target == 3 {
                    d.set_layer_appearance(
                        1,
                        LayerAppearance {
                            offset_x: 5,
                            offset_y: -2,
                            locks: LOCK_TRANSPARENCY,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                }
                d
            };
            let mut actual = seed(1);
            let mut reference = seed(64);
            let before = pixels(&actual);
            let depth = actual.history_depth();
            let mut definition = def(path, false, false, BlendMode::Normal);
            definition.tip.hardness = 0.4;
            definition.tip.flow = 0.6;
            let mask = Arc::new(
                BrushMask::new(
                    4,
                    4,
                    vec![
                        255, 0, 128, 255, 255, 0, 128, 255, 255, 0, 128, 255, 255, 0, 128, 255,
                    ],
                )
                .unwrap(),
            );
            definition.mask = Some(mask.clone());
            let second =
                SecondaryTip::new(7., definition.tip, Some(mask), Default::default()).unwrap();
            let second = if path {
                second.with_spacing(0.35).unwrap()
            } else {
                second
            };
            definition.dual = DualBrush::new(true, DualMode::Multiply, second)
                .with_color(definition.dual.color());
            definition.wet = WetSettings::new(
                true,
                false,
                false,
                true,
                WetMix::new(0.5, 0.4, 0.3, 1., 0.8, 0.15).unwrap(),
            );
            let c = PressureControl::new(
                PressureCurve::new_smooth(vec![(0., 0.), (0.4, 0.2), (1., 1.)]).unwrap(),
                0.15,
            )
            .unwrap();
            definition.taper = StrokeTaper::new(
                true,
                TaperEnd::new(8., c.clone(), c.clone()).unwrap(),
                TaperEnd::new(16., c.clone(), c).unwrap(),
            );
            let points = points();
            reference.taper_reference_endpoint = Some(
                points
                    .windows(2)
                    .map(|p| (p[1].x - p[0].x).hypot(p[1].y - p[0].y))
                    .sum(),
            );
            let a = draw(&mut actual, definition.clone(), &points, None);
            let b = draw(&mut reference, definition, &points, None);
            assert_eq!(
                pixels(&actual),
                pixels(&reference),
                "target {target} path {path}"
            );
            same_ink(&a, &b);
            assert_eq!(actual.history_depth().0, depth.0 + 1);
            let painted = pixels(&actual);
            actual.undo().unwrap();
            assert_eq!(pixels(&actual), before);
            actual.redo().unwrap();
            assert_eq!(pixels(&actual), painted);
        }
    }
}

#[test]
fn wet_dual_cancel_storage_and_budget_failure_restore_the_whole_stroke() {
    let mut doc = Document::with_options(
        192,
        64,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let d = def(true, false, true, BlendMode::Normal);
    let retained = WetInk::for_stroke(
        d.wet,
        8.,
        d.brush.color,
        Some((14., d.brush.background_color)),
        None,
    )
    .unwrap();
    let original = retained.clone();
    doc.begin_configured_dual_wet_brush_stroke(d.clone(), points()[0], Some(&retained))
        .unwrap();
    doc.stroke_to(points()[1]).unwrap();
    doc.cancel_stroke().unwrap();
    assert!(pixels(&doc).iter().all(|p| *p == Pixel::TRANSPARENT));
    same_ink(&retained, &original);
    doc.begin_configured_dual_wet_brush_stroke(d.clone(), points()[0], Some(&retained))
        .unwrap();
    crate::page_pool::fail_writes_for_test(true);
    let result = doc.stroke_to(InputPoint::new(160., 32., 1.));
    crate::page_pool::fail_writes_for_test(false);
    assert!(result.is_err());
    assert!(!doc.stroke_active());
    assert!(pixels(&doc).iter().all(|p| *p == Pixel::TRANSPARENT));
    assert_eq!(doc.history_depth(), (0, 0));
    same_ink(&retained, &original);
    let mut doc = Document::with_options(
        192,
        64,
        DocumentOptions {
            max_document_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    doc.begin_configured_dual_wet_brush_stroke(d, points()[0], Some(&retained))
        .unwrap();
    assert!(doc.stroke_to(InputPoint::new(160., 32., 1.)).is_err());
    assert!(!doc.stroke_active());
    assert!(pixels(&doc).iter().all(|p| *p == Pixel::TRANSPARENT));
    assert_eq!(doc.history_depth(), (0, 0));
}

#[test]
fn wet_dual_budget_counts_each_actual_carrier_grid_independently() {
    let doc = Document::new(32, 32).unwrap();
    let definition = def(false, false, true, BlendMode::Normal);
    let program = BrushProgram::compile_wet_pair(definition.clone()).unwrap();
    let mut costs = Vec::new();
    for diameter in [4., 512.] {
        let mut wet = WetStroke {
            carrier: WetCarrier::new(definition.wet.mix(), 2., Pixel::TRANSPARENT).unwrap(),
            secondary: Some(
                WetCarrier::new(definition.wet.mix(), diameter, Pixel::TRANSPARENT).unwrap(),
            ),
            all_layers: false,
            work_per_dab: 0,
        };
        doc.validate_wet_start(
            &program,
            definition.brush,
            InputPoint::new(16., 16., 1.),
            &mut wet,
        )
        .unwrap();
        costs.push(wet.work_per_dab);
    }
    assert_eq!(costs[1] - costs[0], (64 * 64 - 4 * 4) * 6);
}
#[test]
fn wet_dual_extra_color_dynamics_leave_both_raw_reservoirs_unchanged() {
    let mut ordinary = Document::new(128, 64).unwrap();
    let mut dynamic = Document::new(128, 64).unwrap();
    let mut definition = def(true, false, true, BlendMode::Normal);
    definition.dual = definition.dual.with_sizing(
        DualSizing::new(DualSizeMode::Relative, 8., DualWidthRule::FixedWidth).unwrap(),
    );
    let reference = draw(&mut ordinary, definition.clone(), &points(), None);
    definition.dynamics.color = ColorDynamics::new(true, 0.5, 0.3, 0.4, 0.5, true, 17).unwrap();
    let ink = draw(&mut dynamic, definition, &points(), None);
    assert_ne!(pixels(&ordinary), pixels(&dynamic));
    same_ink(&reference, &ink);
    assert_eq!(ink.secondary().unwrap().edge(), 16);
}
#[test]
fn wet_dual_extra_all_layer_sampling_and_history_commit_failure_are_atomic() {
    for all_layers in [false, true] {
        let mut doc = Document::new(32, 32).unwrap();
        doc.initialize_white_background().unwrap();
        doc.add_layer("target").unwrap();
        let mut definition = def(false, false, true, BlendMode::Normal);
        definition.wet = WetSettings::new(
            true,
            all_layers,
            true,
            false,
            WetMix::new(1., 1., 1., 1., 0., 0.).unwrap(),
        );
        let before = pixels(&doc);
        let depth = doc.history_depth();
        let ink = draw(
            &mut doc,
            definition.clone(),
            &[InputPoint::new(16., 16., 1.)],
            None,
        );
        let expected = if all_layers {
            Pixel::WHITE
        } else {
            Pixel::TRANSPARENT
        };
        assert_eq!(ink.primary().sample(0.5, 0.5), expected);
        assert_eq!(ink.secondary().unwrap().sample(0.5, 0.5), expected);
        assert_eq!(pixels(&doc), before);
        assert_eq!(doc.history_depth(), depth);
        if all_layers {
            doc.add_mask(doc.active_layer()).unwrap();
            assert!(doc
                .begin_configured_dual_wet_brush_stroke(
                    definition,
                    InputPoint::new(16., 16., 1.),
                    None
                )
                .is_err());
            assert!(!doc.stroke_active());
        }
    }
    let mut tiles = BTreeMap::new();
    for y in 0..2 {
        for x in 0..3 {
            let pixels = (0..4096)
                .map(|i| Pixel::from_straight([i as f32 / 4096., 0.3, 0.2, 1.]).unwrap())
                .collect();
            tiles.insert(TileCoord { x, y }, Tile::from_pixels(pixels).unwrap());
        }
    }
    let mut doc = Document::from_import(
        192,
        128,
        vec![ImportedLayer {
            name: "noise".into(),
            properties: LayerProperties::default(),
            tiles,
        }],
        0,
        DocumentOptions {
            max_history_bytes: 2 * TILE_BYTES + 256,
            ..Default::default()
        },
    )
    .unwrap();
    let before = pixels(&doc);
    let mut definition = def(false, false, true, BlendMode::Normal);
    definition.brush.radius = 128.;
    let retained = WetInk::for_stroke(
        definition.wet,
        8.,
        definition.brush.color,
        Some((14., definition.brush.background_color)),
        None,
    )
    .unwrap();
    let original = retained.clone();
    doc.begin_configured_dual_wet_brush_stroke(
        definition,
        InputPoint::new(96., 64., 1.),
        Some(&retained),
    )
    .unwrap();
    crate::history_storage::fail_writes_for_test(true);
    let result = doc.end_wet_ink_stroke();
    crate::history_storage::fail_writes_for_test(false);
    assert!(matches!(result, Err(Error::HistoryStorage(_))));
    assert_eq!(pixels(&doc), before);
    assert_eq!(doc.history_depth(), (0, 0));
    assert!(!doc.stroke_active());
    same_ink(&retained, &original);
}
#[test]
fn wet_dual_soft_clip_uses_geometry_even_when_primary_ink_is_transparent() {
    for overflow in [false, true] {
        let mut doc = Document::new(32, 32).unwrap();
        let mut definition = def(false, false, overflow, BlendMode::Normal);
        definition.brush.color = Pixel::TRANSPARENT;
        definition.brush.background_color = Pixel::from_straight([0., 0., 1., 0.5]).unwrap();
        let ink = draw(
            &mut doc,
            definition,
            &[InputPoint::new(16.5, 16.5, 1.)],
            None,
        );
        assert!((doc.pixel(16, 16).alpha() - 0.2).abs() < 0.000001);
        assert_eq!(doc.pixel(10, 16).alpha() > 0., overflow);
        assert_eq!(ink.primary().sample(0.5, 0.5), Pixel::TRANSPARENT);
        assert_eq!(doc.history_depth(), (1, 0));
    }
}
#[test]
fn wet_dual_saturated_geometry_keeps_mixing_color_on_return_strokes() {
    let red = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
    let blue = Pixel::from_straight([0., 0., 1., 1.]).unwrap();
    let tile = Tile::from_pixels(
        (0..4096)
            .map(|i| if i % 64 < 16 { red } else { blue })
            .collect(),
    )
    .unwrap();
    let mut doc = Document::from_import(
        64,
        64,
        vec![ImportedLayer {
            name: "two colors".into(),
            properties: LayerProperties::default(),
            tiles: BTreeMap::from([(TileCoord { x: 0, y: 0 }, tile)]),
        }],
        0,
        DocumentOptions::default(),
    )
    .unwrap();
    let mut definition = def(false, false, true, BlendMode::Normal);
    definition.brush.opacity = 1.;
    definition.wet = WetSettings::new(
        true,
        false,
        true,
        false,
        WetMix::new(1., 0.5, 0., 1., 1., 0.).unwrap(),
    );
    doc.begin_configured_dual_wet_brush_stroke(definition, InputPoint::new(8.5, 16.5, 1.), None)
        .unwrap();
    let initial = doc.pixel(8, 16);
    let amount = |doc: &Document| {
        doc.stroke
            .as_ref()
            .unwrap()
            .coverage
            .get(&TileCoord { x: 0, y: 0 })
            .unwrap()
            .try_pixels()
            .unwrap()[16 * 64 + 8]
            .alpha()
    };
    assert_eq!(amount(&doc), 1.);
    doc.stroke_to(InputPoint::new(24.5, 16.5, 1.)).unwrap();
    doc.stroke_to(InputPoint::new(8.5, 16.5, 1.)).unwrap();
    assert_eq!(amount(&doc), 1.);
    assert_ne!(doc.pixel(8, 16), initial);
    assert!(doc.pixel(8, 16).components()[2] > 0.);
    let (_, ink) = doc.end_wet_ink_stroke().unwrap();
    assert!(ink.primary().sample(0.5, 0.5).components()[2] > 0.);
    assert_eq!(doc.history_depth(), (1, 0));
}
#[test]
fn wet_dual_zero_write_still_picks_up_and_legacy_preparation_stays_protected() {
    let mut d = def(true, false, true, BlendMode::Normal);
    assert!(BrushProgram::compile(d.clone()).is_err());
    let mut doc = Document::new(32, 32).unwrap();
    assert!(doc
        .begin_configured_wet_brush_stroke(d.clone(), InputPoint::new(16., 16., 1.), None)
        .is_err());
    assert!(!doc.stroke_active());
    doc.initialize_white_background().unwrap();
    d.brush.opacity = 0.;
    d.tip.flow = 0.;
    d.wet = WetSettings::new(
        true,
        false,
        true,
        false,
        WetMix::new(1., 1., 1., 1., 1., 0.).unwrap(),
    );
    let before = pixels(&doc);
    let depth = doc.history_depth();
    let ink = draw(&mut doc, d, &[InputPoint::new(16., 16., 1.)], None);
    assert_eq!(pixels(&doc), before);
    assert_eq!(doc.history_depth(), depth);
    assert_eq!(ink.primary().sample(0.5, 0.5), Pixel::WHITE);
    assert_eq!(ink.secondary().unwrap().sample(0.5, 0.5), Pixel::WHITE);
}
