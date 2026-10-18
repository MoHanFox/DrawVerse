use super::*;
fn fields(d: &Document) -> Vec<Vec<(TileCoord, Vec<Pixel>)>> {
    let s = d.stroke.as_ref().unwrap();
    [
        &s.coverage,
        &s.dual_coverage,
        &s.wet_primary_field,
        &s.wet_secondary_field,
    ]
    .into_iter()
    .map(|f| {
        f.iter()
            .map(|(c, t)| (*c, t.try_pixels().unwrap().to_vec()))
            .collect()
    })
    .collect()
}
fn check(a: &Document, b: &Document) {
    assert_eq!(pixels(a), pixels(b));
    assert_eq!(fields(a), fields(b));
    let a = a.stroke.as_ref().unwrap().wet.as_ref().unwrap();
    let b = b.stroke.as_ref().unwrap().wet.as_ref().unwrap();
    for (a, b) in [
        (&a.carrier, &b.carrier),
        (a.secondary.as_ref().unwrap(), b.secondary.as_ref().unwrap()),
    ] {
        assert_eq!(a.revision(), b.revision());
        assert_eq!(a.edge(), b.edge());
        for y in 0..a.edge() {
            for x in 0..a.edge() {
                let u = (x as f64 + 0.5) / a.edge() as f64;
                let v = (y as f64 + 0.5) / a.edge() as f64;
                assert_eq!(
                    a.sample(u, v).components().map(f32::to_bits),
                    b.sample(u, v).components().map(f32::to_bits)
                );
            }
        }
    }
}
#[test]
fn wet_fields_all_modes_targets_paths_match_two_pass_fields_pixels_ink_and_history() {
    for id in 0..27 {
        for path in [false, true] {
            for target in 0..3 {
                let mut a = Document::new(32, 32).unwrap();
                let mut b = Document::new(32, 32).unwrap();
                b.reference_wet_fields = true;
                let mode = BlendMode::from_id(id).unwrap();
                let mut definition = def(path, id % 2 == 0, id % 3 == 0, mode);
                definition.tip.flow = 0.35;
                definition.tip.hardness = 0.4;
                let mask = Arc::new(
                    BrushMask::new(3, 3, vec![0, 255, 0, 255, 128, 255, 0, 255, 0]).unwrap(),
                );
                definition.mask = Some(mask.clone());
                definition.texture = BrushTexture::new(
                    true,
                    Default::default(),
                    Some(Arc::new(TextureResource::new(mask.clone()))),
                )
                .unwrap();
                let second =
                    SecondaryTip::new(7., definition.tip, Some(mask), Default::default()).unwrap();
                let second = if path {
                    second.with_spacing(0.1).unwrap()
                } else {
                    second
                };
                definition.dual = DualBrush::new(true, DualMode::Multiply, second)
                    .with_color(definition.dual.color());
                definition.wet = WetSettings::new(
                    true,
                    false,
                    true,
                    true,
                    WetMix::new(0.7, 0.4, 0.3, 1., 0.8, 0.1).unwrap(),
                );
                for d in [&mut a, &mut b] {
                    d.initialize_white_background().unwrap();
                    if target == 1 {
                        d.add_mask(1).unwrap();
                    }
                    if target == 2 {
                        let mut appearance = d.layer(1).unwrap().appearance();
                        appearance.offset_x = -3;
                        appearance.offset_y = 2;
                        appearance.locks = LOCK_TRANSPARENCY;
                        d.set_layer_appearance(1, appearance).unwrap();
                    }
                    d.set_selection(
                        Selection::default()
                            .apply(
                                SelectionShape::geometry(
                                    SelectionKind::Ellipse,
                                    3.,
                                    3.,
                                    28.,
                                    28.,
                                    true,
                                ),
                                SelectionOperation::Replace,
                                32,
                                32,
                            )
                            .unwrap(),
                    )
                    .unwrap();
                }
                let before = pixels(&a);
                let depth = a.history_depth();
                for d in [&mut a, &mut b] {
                    d.begin_configured_dual_wet_brush_stroke(
                        definition.clone(),
                        InputPoint::new(9.5, 16.5, 1.),
                        None,
                    )
                    .unwrap();
                }
                check(&a, &b);
                for x in [22.5, 9.5, 22.5] {
                    for d in [&mut a, &mut b] {
                        d.stroke_to(InputPoint::new(x, 16.5, 1.)).unwrap();
                    }
                    check(&a, &b);
                }
                let ai = a.end_wet_ink_stroke().unwrap().1;
                let bi = b.end_wet_ink_stroke().unwrap().1;
                same_ink(&ai, &bi);
                let painted = pixels(&a);
                assert_eq!(painted, pixels(&b));
                assert_eq!(a.history_depth(), b.history_depth());
                assert_eq!(
                    a.history_depth().0,
                    depth.0 + usize::from(painted != before)
                );
                if painted == before {
                    continue;
                }
                for d in [&mut a, &mut b] {
                    d.undo().unwrap();
                    assert_eq!(pixels(d), before);
                    d.redo().unwrap();
                    assert_eq!(pixels(d), painted);
                }
            }
        }
    }
}
#[test]
fn wet_fields_taper_paged_replay_and_cancellation_match_two_pass() {
    for path in [false, true] {
        let mut a = Document::with_options(
            128,
            64,
            DocumentOptions {
                max_resident_tiles: 1,
                ..Default::default()
            },
        )
        .unwrap();
        let mut b = Document::new(128, 64).unwrap();
        b.reference_wet_fields = true;
        let mut definition = def(path, false, true, BlendMode::Screen);
        let c = PressureControl::new(
            PressureCurve::new_smooth(vec![(0., 0.), (0.4, 0.2), (1., 1.)]).unwrap(),
            0.1,
        )
        .unwrap();
        definition.taper = StrokeTaper::new(
            true,
            TaperEnd::new(8., c.clone(), c.clone()).unwrap(),
            TaperEnd::new(16., c.clone(), c).unwrap(),
        );
        definition.wet = WetSettings::new(
            true,
            false,
            true,
            true,
            WetMix::new(0.7, 0.4, 0.3, 1., 0.8, 0.1).unwrap(),
        );
        for d in [&mut a, &mut b] {
            d.begin_configured_dual_wet_brush_stroke(
                definition.clone(),
                InputPoint::new(20.5, 32.5, 1.),
                None,
            )
            .unwrap();
        }
        for x in [40.5, 72.5, 36.5] {
            for d in [&mut a, &mut b] {
                d.begin_stroke_input(InputPoint::new(x, 32.5, 1.)).unwrap();
                while d.stroke_input_pending() {
                    d.continue_stroke_page(Default::default()).unwrap();
                }
            }
            check(&a, &b);
        }
        for d in [&mut a, &mut b] {
            d.cancel_stroke().unwrap();
            assert_eq!(d.history_depth(), (0, 0));
            assert!(pixels(d).iter().all(|p| *p == Pixel::TRANSPARENT));
        }
        let mut inks = Vec::new();
        for d in [&mut a, &mut b] {
            d.begin_configured_dual_wet_brush_stroke(
                definition.clone(),
                InputPoint::new(20.5, 32.5, 1.),
                None,
            )
            .unwrap();
            d.begin_stroke_input(InputPoint::new(84.5, 32.5, 1.))
                .unwrap();
            while d.stroke_input_pending() {
                d.continue_stroke_page(Default::default()).unwrap();
            }
            d.begin_stroke_finish().unwrap();
            loop {
                if let StrokePageResult::StrokeComplete { wet_ink, .. } =
                    d.continue_stroke_page(Default::default()).unwrap()
                {
                    inks.push(wet_ink.unwrap());
                    break;
                }
            }
            assert_eq!(d.history_depth(), (1, 0));
        }
        assert_eq!(pixels(&a), pixels(&b));
        same_ink(&inks[0], &inks[1]);
        let painted = pixels(&a);
        for d in [&mut a, &mut b] {
            d.undo().unwrap();
            assert!(pixels(d).iter().all(|p| *p == Pixel::TRANSPARENT));
            d.redo().unwrap();
            assert_eq!(pixels(d), painted);
        }
    }
}
