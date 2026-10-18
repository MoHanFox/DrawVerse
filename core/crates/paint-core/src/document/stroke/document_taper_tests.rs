use super::*;
use crate::*;

fn settings() -> StrokeTaper {
    let curve = PressureCurve::new_smooth(vec![(0., 0.), (0.4, 0.2), (1., 1.)]).unwrap();
    let c = PressureControl::new(curve, 0.15).unwrap();
    StrokeTaper::new(
        true,
        TaperEnd::new(12., c.clone(), c.clone()).unwrap(),
        TaperEnd::new(16., c.clone(), c).unwrap(),
    )
}
fn seed(state: usize, resident: usize) -> Document {
    let mut tiles = BTreeMap::new();
    for tx in 0..3 {
        let pixels = (0..4096)
            .map(|i| {
                Pixel::from_straight([
                    (i % 64) as f32 / 80.,
                    0.2,
                    (i / 64) as f32 / 90.,
                    if i % 7 == 0 { 0.4 } else { 0.9 },
                ])
                .unwrap()
            })
            .collect();
        tiles.insert(
            TileCoord { x: tx, y: 0 },
            Tile::from_pixels(pixels).unwrap(),
        );
    }
    let mut d = Document::from_import(
        192,
        64,
        vec![ImportedLayer {
            name: "seed".into(),
            properties: LayerProperties::default(),
            tiles,
        }],
        0,
        DocumentOptions {
            max_resident_tiles: resident,
            ..Default::default()
        },
    )
    .unwrap();
    match state {
        1 => {
            d.set_selection(
                Selection::default()
                    .apply(
                        SelectionShape::geometry(SelectionKind::Ellipse, 30., 15., 60., 42., true),
                        SelectionOperation::Replace,
                        192,
                        64,
                    )
                    .unwrap(),
            )
            .unwrap();
        }
        2 => {
            d.add_mask(d.active_layer()).unwrap();
        }
        3 => {
            d.set_layer_appearance(
                d.active_layer(),
                LayerAppearance {
                    offset_x: 7,
                    offset_y: -2,
                    locks: LOCK_TRANSPARENCY,
                    ..Default::default()
                },
            )
            .unwrap();
        }
        _ => {}
    }
    d
}
fn definition(kind: usize) -> BrushDefinition {
    let mut d = BrushDefinition {
        brush: Brush {
            radius: 5.,
            spacing: 0.4,
            opacity: 0.35,
            color: Pixel::from_straight([0.1, 0.3, 1., 0.8]).unwrap(),
            background_color: Pixel::from_straight([1., 0.1, 0.05, 1.]).unwrap(),
            ..Default::default()
        },
        tip: BrushTip {
            pressure_flow: false,
            hardness: 0.65,
            roundness: 0.7,
            ..Default::default()
        },
        ..Default::default()
    };
    if kind == 1 {
        d.brush.mode = BrushMode::Erase;
    }
    if kind == 2 {
        d.write_blend = BlendMode::Multiply;
    }
    if kind >= 3 {
        d.dynamics.tip = TipDynamics::new(AngleControl::Direction, true, 0.2).unwrap();
        d.dynamics.pressure_opacity = true;
        d.dynamics.scatter = ScatterDynamics::new(true, 0.3, true, 2, 0.3, 19).unwrap();
        d.dynamics.shape_jitter = ShapeJitter::new(true, 0.2, 0.3, 0.2, 0.15, 0.2, 37).unwrap();
        d.dynamics.color = ColorDynamics::new(true, 0.3, 0.2, 0.3, 0.1, true, 91).unwrap();
    }
    if kind == 4 || kind == 5 {
        let mask = Arc::new(
            BrushMask::new(
                5,
                3,
                vec![
                    0, 100, 255, 150, 0, 255, 255, 0, 255, 255, 0, 150, 255, 100, 0,
                ],
            )
            .unwrap(),
        );
        d.mask = Some(mask.clone());
        d.texture = BrushTexture::new(
            true,
            TextureSettings::new(
                if kind == 4 {
                    TextureSpace::Document
                } else {
                    TextureSpace::Tip
                },
                1.1,
                20.,
                0.6,
                false,
                0.2,
                0.5,
            )
            .unwrap(),
            Some(Arc::new(TextureResource::new(mask))),
        )
        .unwrap();
    }
    if kind >= 6 {
        let mut secondary = SecondaryTip::new(
            14.,
            BrushTip {
                pressure_flow: false,
                ..Default::default()
            },
            None,
            Default::default(),
        )
        .unwrap();
        if kind % 2 == 1 {
            secondary = secondary.with_spacing(0.18).unwrap();
        }
        d.dual = DualBrush::new(true, DualMode::Multiply, secondary).with_sizing(
            DualSizing::new(DualSizeMode::Relative, 4., DualWidthRule::FixedWidth).unwrap(),
        );
        if kind >= 8 {
            d.dual = d.dual.with_color(
                DualColor::new(
                    true,
                    true,
                    true,
                    DualColorOrder::SecondBehind,
                    BlendMode::Normal,
                    BlendMode::Multiply,
                )
                .unwrap(),
            );
        }
    }
    d
}
fn points() -> Vec<InputPoint> {
    [
        (24.5, 32.5, 0.5),
        (46.5, 32.5, 0.9),
        (64.5, 47.5, 0.7),
        (88.5, 47.5, 0.8),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (x, y, p))| InputPoint {
        tilt_x: 23.,
        tilt_y: -17.,
        rotation: 350.,
        capabilities: 15,
        timestamp_ns: i as u64 * 12_000_000,
        ..InputPoint::new(x, y, p)
    })
    .collect()
}
fn distance(points: &[InputPoint]) -> f64 {
    points
        .windows(2)
        .map(|p| (p[1].x - p[0].x).hypot(p[1].y - p[0].y))
        .sum()
}
fn draw(
    d: &mut Document,
    definition: BrushDefinition,
    points: &[InputPoint],
    s: &StrokeTaper,
) -> Option<WetCarrier> {
    if definition.wet.enabled() {
        d.begin_configured_tapered_wet_brush_stroke(definition, points[0], None, s)
            .unwrap();
    } else {
        d.begin_tapered_brush_stroke(definition, points[0], s)
            .unwrap();
    }
    for &point in &points[1..] {
        d.stroke_to(point).unwrap();
    }
    if d.stroke.as_ref().unwrap().wet.is_some() {
        Some(d.end_wet_brush_stroke().unwrap().1)
    } else {
        d.end_stroke().unwrap();
        None
    }
}
fn same(a: &Document, b: &Document) {
    for y in 0..a.height {
        for x in 0..a.width {
            assert_eq!(
                a.try_pixel(x, y).unwrap(),
                b.try_pixel(x, y).unwrap(),
                "pixel {x},{y}"
            );
        }
    }
}

#[test]
fn taper_document_pixels_match_one_pass_final_endpoint_across_operators_and_targets() {
    let points = points();
    for kind in 0..10 {
        for state in 0..4 {
            let mut actual = seed(state, 64);
            let mut reference = seed(state, 64);
            reference.taper_reference_endpoint = Some(distance(&points));
            draw(&mut actual, definition(kind), &points, &settings());
            draw(&mut reference, definition(kind), &points, &settings());
            same(&actual, &reference);
        }
    }
}

#[test]
fn taper_document_each_visible_tail_matches_current_endpoint_and_moves_without_ghosts() {
    let points = points();
    let s = settings();
    let mut actual = seed(0, 64);
    actual
        .begin_tapered_brush_stroke(definition(9), points[0], &s)
        .unwrap();
    for i in 1..points.len() {
        actual.stroke_to(points[i]).unwrap();
        let mut reference = seed(0, 64);
        reference.taper_reference_endpoint = Some(distance(&points[..=i]));
        reference
            .begin_tapered_brush_stroke(definition(9), points[0], &s)
            .unwrap();
        for &p in &points[1..=i] {
            reference.stroke_to(p).unwrap();
        }
        same(&actual, &reference); // Both are still in-progress: no final extra dab.
        reference.cancel_stroke().unwrap();
    }
    actual.end_stroke().unwrap();
}

#[test]
fn taper_document_wet_and_smudge_restore_carrier_with_tail_and_paging() {
    let points = points();
    for wet in [false, true] {
        for smudge in [false, true] {
            if !wet && !smudge {
                continue;
            }
            let mut d = definition(7);
            if smudge {
                d.brush.mode = BrushMode::Smudge {
                    all_layers: false,
                    finger_painting: true,
                    pressure_strength: true,
                };
            }
            if wet {
                d.wet = WetSettings::new(
                    true,
                    false,
                    true,
                    true,
                    WetMix::new(0.7, 0.5, 0.6, 0.8, 0.7, 0.2).unwrap(),
                );
            }
            let mut actual = seed(0, 1);
            let mut reference = seed(0, 64);
            reference.taper_reference_endpoint = Some(distance(&points));
            let a = draw(&mut actual, d.clone(), &points, &settings());
            let b = draw(&mut reference, d, &points, &settings());
            same(&actual, &reference);
            if let (Some(a), Some(b)) = (a, b) {
                assert_eq!(a.revision(), b.revision());
                assert_eq!(a.edge(), b.edge());
                for y in 0..a.edge() {
                    for x in 0..a.edge() {
                        assert_eq!(
                            a.sample(
                                x as f64 / (a.edge() - 1) as f64,
                                y as f64 / (a.edge() - 1) as f64
                            ),
                            b.sample(
                                x as f64 / (b.edge() - 1) as f64,
                                y as f64 / (b.edge() - 1) as f64
                            )
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn taper_document_single_history_snapshots_cancel_and_failure_roll_back_whole_stroke() {
    for kind in [0, 1, 9] {
        let mut d = seed(0, 1);
        let before = d.snapshot();
        let depth = d.history_depth();
        draw(&mut d, definition(kind), &points(), &settings());
        assert_eq!(d.history_depth(), (depth.0 + 1, 0));
        let after = d.snapshot();
        d.undo().unwrap();
        for y in 0..64 {
            for x in 0..192 {
                assert_eq!(d.pixel(x, y), before.pixel(x, y));
            }
        }
        d.redo().unwrap();
        for y in 0..64 {
            for x in 0..192 {
                assert_eq!(d.pixel(x, y), after.pixel(x, y));
            }
        }
        d.begin_tapered_brush_stroke(definition(kind), points()[0], &settings())
            .unwrap();
        d.stroke_to(points()[1]).unwrap();
        d.cancel_stroke().unwrap();
        for y in 0..64 {
            for x in 0..192 {
                assert_eq!(d.pixel(x, y), after.pixel(x, y));
            }
        }
        let depth = d.history_depth();
        d.begin_tapered_brush_stroke(definition(kind), points()[0], &settings())
            .unwrap();
        d.stroke_to(points()[1]).unwrap();
        let mut invalid = points()[2];
        invalid.pressure = f32::NAN;
        assert!(d.stroke_to(invalid).is_err());
        assert!(!d.stroke_active());
        assert_eq!(d.history_depth(), depth);
        for y in 0..64 {
            for x in 0..192 {
                assert_eq!(d.pixel(x, y), after.pixel(x, y));
            }
        }
    }
}

#[test]
fn taper_document_start_and_opacity_are_independent_and_disabled_is_exact() {
    let end = TaperEnd::new(
        20.,
        PressureControl::new(PressureCurve::default(), 1.).unwrap(),
        PressureControl::default(),
    )
    .unwrap();
    let s = StrokeTaper::new(true, end, TaperEnd::default());
    let mut d = Document::new(64, 32).unwrap();
    let mut def = definition(0);
    def.brush.color = Pixel::WHITE;
    def.brush.opacity = 0.3;
    def.tip.pressure_size = false;
    def.tip.pressure_flow = false;
    d.begin_tapered_brush_stroke(def.clone(), InputPoint::new(10.5, 16.5, 1.), &s)
        .unwrap();
    assert_eq!(d.pixel(10, 16).alpha(), 0.);
    d.stroke_to(InputPoint::new(20.5, 16.5, 1.)).unwrap();
    assert!(d.pixel(20, 16).alpha() > 0. && d.pixel(20, 16).alpha() <= 0.15 + 1e-6);
    d.end_stroke().unwrap();
    for setting in [
        StrokeTaper::default(),
        StrokeTaper::new(false, settings().start().clone(), settings().end().clone()),
        StrokeTaper::new(true, TaperEnd::default(), TaperEnd::default()),
    ] {
        let mut a = seed(0, 64);
        let mut b = seed(0, 64);
        let points = points();
        draw(&mut a, def.clone(), &points, &setting);
        b.begin_brush_stroke(def.clone(), points[0]).unwrap();
        for &p in &points[1..] {
            b.stroke_to(p).unwrap();
        }
        b.end_stroke().unwrap();
        same(&a, &b);
    }
}

#[test]
fn taper_document_long_stroke_keeps_only_tail_tiles_and_records() {
    let end = TaperEnd::new(8., PressureControl::default(), PressureControl::default()).unwrap();
    let s = StrokeTaper::new(true, TaperEnd::default(), end);
    let mut d = Document::new(2048, 32).unwrap();
    let def = BrushDefinition {
        brush: Brush {
            radius: 2.,
            spacing: 1.,
            opacity: 0.4,
            ..Default::default()
        },
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    };
    d.begin_tapered_brush_stroke(def, InputPoint::new(8.5, 16.5, 1.), &s)
        .unwrap();
    for i in 1..=128 {
        d.stroke_to(InputPoint::new(8.5 + i as f64 * 8., 16.5, 1.))
            .unwrap();
        let tail = d.stroke.as_ref().unwrap().taper.as_ref().unwrap();
        assert!(tail.pending.len() <= 4);
        assert!(tail.journal.layer.len() <= 2);
        assert!(tail.journal.primary.len() <= 2);
        assert!(tail.journal.secondary.len() <= 2);
    }
    assert!(d.stroke.as_ref().unwrap().before.len() > 8);
    assert!(d.end_stroke().unwrap());
    assert_eq!(d.history_depth(), (1, 0));
}

#[test]
fn taper_document_smoothing_preserves_filtered_endpoint_on_release() {
    let mut def = definition(0);
    def.smoothing = BrushSmoothing::new(true, 250., 250., 0., 128.).unwrap();
    let mut d = Document::new(192, 64).unwrap();
    let mut raw = InputPoint::new(100.5, 40.5, 1.);
    raw.timestamp_ns = 24_000_000;
    d.begin_tapered_brush_stroke(def, InputPoint::new(16.5, 32.5, 0.3), &settings())
        .unwrap();
    d.stroke_to(raw).unwrap();
    assert!(d.stroke.as_ref().unwrap().last.x < 40.);
    assert!(d.stroke.as_ref().unwrap().path_distance < 24.);
    let drawn = d.snapshot();
    d.end_stroke().unwrap();
    for y in 0..64 {
        for x in 0..192 {
            assert_eq!(d.pixel(x, y), drawn.pixel(x, y));
        }
    }
    assert_eq!(d.pixel(100, 40), Pixel::TRANSPARENT);
}

#[test]
fn taper_document_record_and_filtered_replay_budgets_rollback_prior_pixels() {
    let mut d = seed(0, 64);
    let before = d.snapshot();
    let depth = d.history_depth();
    d.begin_tapered_brush_stroke(definition(0), points()[0], &settings())
        .unwrap();
    let tail = d.stroke.as_mut().unwrap().taper.as_mut().unwrap();
    // Exercise the same preflight failure as a full dense pending window without
    // rendering 65536 stamps merely to reach the configured bound.
    tail.buffer = TaperBuffer::new(&settings(), 1, 1.).unwrap();
    assert!(matches!(
        d.stroke_to(points()[1]),
        Err(Error::ResourceLimit(_))
    ));
    assert!(!d.stroke_active());
    assert_eq!(d.history_depth(), depth);
    for y in 0..64 {
        for x in 0..192 {
            assert_eq!(d.pixel(x, y), before.pixel(x, y));
        }
    }

    let def = BrushDefinition {
        brush: Brush {
            radius: 256.,
            opacity: 0.4,
            ..Default::default()
        },
        dynamics: BrushDynamics {
            pressure_opacity: true,
            ..Default::default()
        },
        smoothing: BrushSmoothing::new(true, 0., 8., 0., 0.).unwrap(),
        ..Default::default()
    };
    let first = InputPoint::new(16., 16., 0.2);
    let mut last = InputPoint::new(16., 16., 1.);
    last.timestamp_ns = 28_000_000;
    let mut control = Document::new(32, 32).unwrap();
    control.begin_brush_stroke(def.clone(), first).unwrap();
    control.stroke_to(last).unwrap();
    control.end_stroke().unwrap();
    let mut d = Document::new(32, 32).unwrap();
    d.begin_tapered_brush_stroke(def, first, &settings())
        .unwrap();
    assert!(matches!(d.stroke_to(last), Err(Error::ResourceLimit(_))));
    assert!(!d.stroke_active());
    assert_eq!(d.history_depth(), (0, 0));
    for y in 0..32 {
        for x in 0..32 {
            assert_eq!(d.pixel(x, y), Pixel::TRANSPARENT);
        }
    }
}

#[test]
fn taper_document_scratch_write_failure_restores_whole_stroke() {
    for wet in [false, true] {
        let mut d = Document::with_options(
            192,
            64,
            DocumentOptions {
                max_resident_tiles: 1,
                ..Default::default()
            },
        )
        .unwrap();
        let mut def = definition(0);
        if wet {
            def.wet = WetSettings::new(
                true,
                false,
                true,
                true,
                WetMix::new(0., 0., 0., 1., 1., 0.).unwrap(),
            );
        }
        if wet {
            d.begin_configured_tapered_wet_brush_stroke(def, points()[0], None, &settings())
                .unwrap();
        } else {
            d.begin_tapered_brush_stroke(def, points()[0], &settings())
                .unwrap();
        }
        assert!(d.pixel(24, 32).alpha() > 0.);
        crate::page_pool::fail_writes_for_test(true);
        let result = d.stroke_to(points()[2]);
        crate::page_pool::fail_writes_for_test(false);
        assert!(result.is_err());
        assert!(!d.stroke_active());
        assert_eq!(d.history_depth(), (0, 0));
        for y in 0..64 {
            for x in 0..192 {
                assert_eq!(d.pixel(x, y), Pixel::TRANSPARENT);
            }
        }
    }
}

#[test]
fn taper_document_secondary_image_scaling_and_subpixel_stream_ties_are_stable() {
    for image in [false, true] {
        let mut def = definition(9);
        if image {
            let mask = Arc::new(
                BrushMask::new(3, 3, vec![255, 0, 255, 255, 255, 255, 0, 255, 0]).unwrap(),
            );
            let secondary = SecondaryTip::new(
                12.,
                BrushTip {
                    pressure_size: false,
                    pressure_flow: false,
                    ..Default::default()
                },
                Some(mask),
                BrushDynamics {
                    tip: TipDynamics::new(AngleControl::Direction, true, 0.2).unwrap(),
                    ..Default::default()
                },
            )
            .unwrap()
            .with_spacing(0.15)
            .unwrap();
            def.dual = DualBrush::new(true, DualMode::Multiply, secondary).with_color(
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
        } else {
            def.brush.radius = 0.5;
            def.brush.spacing = 0.02;
            def.tip.pressure_size = false;
            def.dynamics = BrushDynamics::default();
            let secondary = SecondaryTip::new(
                1.,
                BrushTip {
                    pressure_size: false,
                    pressure_flow: false,
                    ..Default::default()
                },
                None,
                Default::default(),
            )
            .unwrap()
            .with_spacing(0.010000001)
            .unwrap();
            def.dual = DualBrush::new(true, DualMode::Multiply, secondary);
        }
        let points = if image {
            points()
        } else {
            vec![
                InputPoint::new(24.5, 32.5, 1.),
                InputPoint::new(25.5, 32.5, 1.),
            ]
        };
        let mut a = seed(0, 64);
        let mut b = seed(0, 64);
        b.taper_reference_endpoint = Some(distance(&points));
        draw(&mut a, def.clone(), &points, &settings());
        draw(&mut b, def, &points, &settings());
        same(&a, &b);
    }
}
