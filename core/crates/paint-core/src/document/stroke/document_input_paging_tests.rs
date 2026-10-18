use super::*;
use crate::*;
fn definition(wet: bool, dual: bool, taper: bool, images: bool) -> BrushDefinition {
    let tip = BrushTip {
        pressure_size: false,
        pressure_flow: false,
        ..Default::default()
    };
    let mask = images.then(|| {
        Arc::new(BrushMask::new(3, 3, vec![255, 0, 255, 255, 128, 255, 0, 255, 255]).unwrap())
    });
    let mut d = BrushDefinition {
        brush: Brush {
            radius: 4.,
            spacing: 0.15,
            opacity: 0.4,
            color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
            background_color: Pixel::from_straight([0., 0., 1., 1.]).unwrap(),
            ..Default::default()
        },
        tip,
        mask: mask.clone(),
        ..Default::default()
    };
    if dual {
        d.dual = DualBrush::new(
            true,
            DualMode::Multiply,
            SecondaryTip::new(8., tip, mask.clone(), Default::default())
                .unwrap()
                .with_spacing(0.25)
                .unwrap(),
        )
        .with_color(
            DualColor::new(
                true,
                true,
                true,
                Default::default(),
                BlendMode::Normal,
                BlendMode::Screen,
            )
            .unwrap(),
        );
    }
    if let Some(mask) = mask {
        d.texture = BrushTexture::new(
            true,
            Default::default(),
            Some(Arc::new(TextureResource::new(mask))),
        )
        .unwrap();
    }
    d.wet = WetSettings::new(
        wet,
        false,
        true,
        true,
        WetMix::new(0.5, 0.2, 0.3, 0.7, 0.8, 0.01).unwrap(),
    )
    .with_dual_color(dual && wet);
    if taper {
        let c = PressureControl::new(
            PressureCurve::new_smooth(vec![(0., 0.), (0.4, 0.2), (1., 1.)]).unwrap(),
            0.1,
        )
        .unwrap();
        d.taper = StrokeTaper::new(
            true,
            TaperEnd::new(4., c.clone(), c.clone()).unwrap(),
            TaperEnd::new(12., c.clone(), c).unwrap(),
        );
    }
    d
}
fn pixels(doc: &Document) -> Vec<Pixel> {
    (0..doc.height)
        .flat_map(|y| (0..doc.width).map(move |x| doc.try_pixel(x, y).unwrap()))
        .collect()
}
fn begin(doc: &mut Document, d: BrushDefinition, point: InputPoint) {
    if d.wet.enabled() {
        if d.dual.enabled() && d.dual.color().enabled() {
            doc.begin_configured_dual_wet_brush_stroke(d, point, None)
                .unwrap();
        } else {
            doc.begin_configured_wet_brush_stroke(d, point, None)
                .unwrap();
        }
    } else {
        doc.begin_brush_stroke(d, point).unwrap();
    }
}
fn drain(
    doc: &mut Document,
    budget: InputScheduleBudget,
) -> (usize, Option<(bool, Option<WetInk>)>) {
    let mut pages = 0;
    loop {
        pages += 1;
        assert!(pages < 10000);
        let before = doc.pending_input.as_ref().unwrap().work;
        let result = doc.continue_stroke_page(budget).unwrap();
        if let Some(pending) = &doc.pending_input {
            assert!(pending.work.dabs - before.dabs <= budget.work.dabs);
            assert!(pending.work.visits - before.visits <= budget.work.visits);
        }
        match result {
            StrokePageResult::Pending => assert!(doc.stroke_input_pending()),
            StrokePageResult::InputComplete => {
                assert!(!doc.stroke_input_pending());
                return (pages, None);
            }
            StrokePageResult::StrokeComplete { changed, wet_ink } => {
                assert!(!doc.stroke_input_pending());
                assert!(!doc.stroke_active());
                return (pages, Some((changed, wet_ink)));
            }
        }
    }
}
fn paged_finish(doc: &mut Document, budget: InputScheduleBudget) -> Option<WetInk> {
    doc.begin_stroke_finish().unwrap();
    let (_, result) = drain(doc, budget);
    let (changed, ink) = result.unwrap();
    assert!(changed);
    ink
}
fn ink_eq(a: Option<WetInk>, b: Option<WetInk>) {
    match (a, b) {
        (None, None) => {}
        (Some(a), Some(b)) => {
            for (a, b) in [
                (Some(a.primary()), Some(b.primary())),
                (a.secondary(), b.secondary()),
            ] {
                match (a, b) {
                    (None, None) => {}
                    (Some(a), Some(b)) => {
                        assert_eq!(a.revision(), b.revision());
                        assert_eq!(a.edge(), b.edge());
                        for y in 0..a.edge() {
                            for x in 0..a.edge() {
                                let u = (x as f64 + 0.5) / a.edge() as f64;
                                let v = (y as f64 + 0.5) / a.edge() as f64;
                                assert_eq!(a.sample(u, v), b.sample(u, v));
                            }
                        }
                    }
                    _ => panic!("ink role mismatch"),
                }
            }
        }
        _ => panic!("ink mismatch"),
    }
}
#[test]
fn input_paging_document_matches_synchronous_pixels_ink_and_history_across_page_sizes() {
    for wet in [false, true] {
        for dual in [false, true] {
            for taper in [false, true] {
                for events in [3, 19, 8192] {
                    let d = definition(wet, dual, taper, false);
                    let first = InputPoint::new(16.5, 32.5, 0.4);
                    let points = [
                        InputPoint::new(24.5, 32.5, 0.8),
                        InputPoint::new(31.5, 36.5, 0.2),
                    ];
                    let mut reference = Document::new(64, 64).unwrap();
                    let mut paged = Document::new(64, 64).unwrap();
                    begin(&mut reference, d.clone(), first);
                    begin(&mut paged, d, first);
                    let budget = InputScheduleBudget::new(8192, 8_000_000, events).unwrap();
                    for point in points {
                        reference.stroke_to(point).unwrap();
                        paged.begin_stroke_input(point).unwrap();
                        drain(&mut paged, budget);
                        assert_eq!(pixels(&paged), pixels(&reference));
                        assert_eq!(paged.history_depth(), (0, 0));
                    }
                    let expected = if wet {
                        Some(reference.end_wet_ink_stroke().unwrap().1)
                    } else {
                        reference.end_stroke().unwrap();
                        None
                    };
                    let ink = paged_finish(&mut paged, budget);
                    ink_eq(ink, expected);
                    assert_eq!(pixels(&paged), pixels(&reference));
                    assert_eq!(paged.history_depth(), (1, 0));
                    let after = pixels(&paged);
                    paged.undo().unwrap();
                    assert!(pixels(&paged).iter().all(|p| *p == Pixel::TRANSPARENT));
                    paged.redo().unwrap();
                    assert_eq!(pixels(&paged), after);
                }
            }
        }
    }
}
#[test]
fn input_paging_document_busy_cancel_and_page_failure_rollback_every_prior_pixel() {
    let mut doc = Document::new(128, 64).unwrap();
    begin(
        &mut doc,
        definition(true, true, true, false),
        InputPoint::new(16., 32., 1.),
    );
    doc.begin_stroke_input(InputPoint::new(80., 32., 0.5))
        .unwrap();
    let budget = InputScheduleBudget::new(8192, 8_000_000, 2).unwrap();
    assert!(matches!(
        doc.continue_stroke_page(budget).unwrap(),
        StrokePageResult::Pending
    ));
    assert!(matches!(
        doc.stroke_to(InputPoint::new(90., 32., 1.)),
        Err(Error::Busy)
    ));
    assert!(matches!(
        doc.begin_stroke_input(InputPoint::new(90., 32., 1.)),
        Err(Error::Busy)
    ));
    assert!(matches!(doc.begin_stroke_finish(), Err(Error::Busy)));
    assert!(matches!(doc.end_wet_ink_stroke(), Err(Error::Busy)));
    doc.cancel_stroke().unwrap();
    assert!(!doc.stroke_input_pending());
    assert!(pixels(&doc).iter().all(|p| *p == Pixel::TRANSPARENT));
    assert_eq!(doc.history_depth(), (0, 0));
    begin(
        &mut doc,
        definition(true, true, false, false),
        InputPoint::new(16., 32., 1.),
    );
    doc.begin_stroke_input(InputPoint::new(80., 32., 1.))
        .unwrap();
    doc.continue_stroke_page(budget).unwrap();
    assert!(pixels(&doc).iter().any(|p| *p != Pixel::TRANSPARENT));
    assert!(doc
        .continue_stroke_page(InputScheduleBudget::new(1, 1, 1).unwrap())
        .is_err());
    assert!(!doc.stroke_input_pending());
    assert!(!doc.stroke_active());
    assert!(pixels(&doc).iter().all(|p| *p == Pixel::TRANSPARENT));
    assert_eq!(doc.history_depth(), (0, 0));
}
#[test]
fn input_paging_document_large_images_taper_and_retained_ink_finish_in_bounded_pages() {
    for taper in [false, true] {
        let mut doc = Document::new(128, 64).unwrap();
        let d = definition(true, true, taper, true);
        begin(&mut doc, d.clone(), InputPoint::new(16.5, 32.5, 1.));
        doc.begin_stroke_input(InputPoint::new(80.5, 32.5, 1.))
            .unwrap();
        let budget = InputScheduleBudget::default();
        let (pages, _) = drain(&mut doc, budget);
        assert!(pages > 1);
        let ink = paged_finish(&mut doc, budget).unwrap();
        assert!(ink.secondary().is_some());
        assert!(pixels(&doc).iter().any(|p| *p != Pixel::TRANSPARENT));
        assert_eq!(doc.history_depth(), (1, 0));
        let painted = pixels(&doc);
        doc.undo().unwrap();
        doc.redo().unwrap();
        assert_eq!(pixels(&doc), painted);
        // The old bounded synchronous contract remains explicit.
        let mut old = Document::new(128, 64).unwrap();
        begin(&mut old, d, InputPoint::new(16.5, 32.5, 1.));
        assert!(old.stroke_to(InputPoint::new(80.5, 32.5, 1.)).is_err());
        assert!(pixels(&old).iter().all(|p| *p == Pixel::TRANSPARENT));
    }
}

// Independent known-endpoint pixel reference: no active-tail replay or pending input state.
fn one_pass_reference(
    d: BrushDefinition,
    first: InputPoint,
    last: InputPoint,
) -> (Document, Option<WetInk>) {
    let endpoint = (last.x - first.x).hypot(last.y - first.y);
    let mut doc = Document::new(128, 64).unwrap();
    if d.taper.enabled() {
        doc.taper_reference_endpoint = Some(endpoint);
    }
    let taper = d.taper.compile();
    begin(&mut doc, d, first);
    let mut stroke = doc.stroke.take().unwrap();
    stroke.taper = None;
    let (primary_work, secondary_work) = Document::stroke_input_work(
        stroke.brush,
        &stroke.program,
        stroke.wet.as_ref().map_or(0, |w| w.work_per_dab),
    );
    let mut plan = BrushInputSchedule::new(
        InputScheduleStart {
            point: first,
            primary_next: stroke.distance_to_next,
            secondary_next: stroke.dual_next,
            path_distance: 0.,
            direction: stroke.direction,
        },
        InputScheduleSteps {
            primary: stroke.brush.step(),
            secondary: stroke.program.dual.as_ref().and_then(|d| d.step()),
            primary_work,
            secondary_work,
        },
        vec![InputScheduleSegment {
            point: last,
            stationary_primary: false,
            stationary_secondary: false,
        }],
    )
    .unwrap();
    while !plan.complete() {
        let page = plan.prepare_page(InputScheduleBudget::default()).unwrap();
        for &event in page.events() {
            if let InputScheduleEvent::Dab { path_distance, .. } = event {
                stroke.taper_factors = taper.factors(path_distance, endpoint).unwrap();
            }
            doc.execute_input_event(&mut stroke, event).unwrap();
        }
        plan.adopt(page).unwrap();
    }
    stroke.taper_factors = taper.factors(endpoint, endpoint).unwrap();
    doc.stroke = Some(stroke);
    let (_, ink) = doc.finish_stroke_with_end(true).unwrap();
    (doc, ink)
}
#[test]
fn input_paging_document_large_wet_images_match_known_endpoint_one_pass_pixels_and_both_inks() {
    for taper in [false, true] {
        for paired in [false, true] {
            for events in [5, 8192] {
                let mut d = definition(true, true, taper, true);
                if paired {
                    d.dual = DualBrush::new(
                        true,
                        DualMode::Multiply,
                        SecondaryTip::new(8., d.tip, d.mask.clone(), Default::default()).unwrap(),
                    )
                    .with_color(d.dual.color());
                }
                let first = InputPoint::new(16.5, 32.5, 0.3);
                let last = InputPoint::new(80.5, 32.5, 0.9);
                let (reference, expected) = one_pass_reference(d.clone(), first, last);
                let mut doc = Document::new(128, 64).unwrap();
                begin(&mut doc, d, first);
                doc.begin_stroke_input(last).unwrap();
                let budget = InputScheduleBudget::new(8192, 8_000_000, events).unwrap();
                drain(&mut doc, budget);
                let ink = paged_finish(&mut doc, budget);
                assert_eq!(
                    pixels(&doc),
                    pixels(&reference),
                    "taper={taper} paired={paired} events={events}"
                );
                ink_eq(ink, expected);
            }
        }
    }
}
#[test]
fn input_paging_document_smoothing_stationary_pressure_and_device_dynamics_are_captured_once() {
    for taper in [false, true] {
        let mut d = definition(true, true, taper, false);
        d.smoothing = BrushSmoothing::new(true, 12., 8., 0.5, 2.).unwrap();
        d.tip.pressure_size = true;
        d.tip.pressure_flow = true;
        d.dynamics.tip = TipDynamics::new(AngleControl::PenRotation, true, 0.2).unwrap();
        d.dynamics.scatter = ScatterDynamics::new(true, 0.2, true, 2, 0.5, 99).unwrap();
        let first = InputPoint {
            timestamp_ns: 4_000_000,
            capabilities: 7,
            rotation: 170.,
            tilt_x: 30.,
            ..InputPoint::new(16., 32., 0.2)
        };
        let mut a = Document::new(64, 64).unwrap();
        let mut b = Document::new(64, 64).unwrap();
        begin(&mut a, d.clone(), first);
        begin(&mut b, d, first);
        let points = [
            InputPoint {
                timestamp_ns: 12_000_000,
                capabilities: 7,
                rotation: -175.,
                tilt_x: 45.,
                ..InputPoint::new(22., 32., 0.8)
            },
            InputPoint {
                timestamp_ns: 20_000_000,
                capabilities: 1,
                ..InputPoint::new(22., 32., 0.1)
            },
            InputPoint {
                timestamp_ns: 28_000_000,
                capabilities: 7,
                rotation: 150.,
                tilt_y: 50.,
                ..InputPoint::new(28., 36., 0.6)
            },
        ];
        let budget = InputScheduleBudget::new(8192, 8_000_000, 3).unwrap();
        for p in points {
            a.stroke_to(p).unwrap();
            b.begin_stroke_input(p).unwrap();
            drain(&mut b, budget);
            assert_eq!(pixels(&a), pixels(&b));
            assert_eq!(
                a.stroke.as_ref().unwrap().last,
                b.stroke.as_ref().unwrap().last
            );
        }
        let expected = a.end_wet_ink_stroke().unwrap().1;
        let actual = paged_finish(&mut b, budget);
        ink_eq(actual, Some(expected));
        assert_eq!(pixels(&a), pixels(&b));
    }
}
#[test]
fn input_paging_document_erase_smudge_mask_selection_and_offset_lock_keep_existing_pixels() {
    for target in 0..4 {
        let setup = || {
            let mut doc = Document::with_options(
                64,
                64,
                DocumentOptions {
                    max_resident_tiles: 1,
                    ..Default::default()
                },
            )
            .unwrap();
            doc.initialize_white_background().unwrap();
            if target == 2 {
                doc.add_mask(1).unwrap();
            }
            if target == 3 {
                doc.set_layer_appearance(
                    1,
                    LayerAppearance {
                        offset_x: 3,
                        offset_y: -2,
                        locks: LOCK_TRANSPARENCY,
                        ..Default::default()
                    },
                )
                .unwrap();
            }
            doc.set_selection(
                Selection::default()
                    .apply(
                        SelectionShape::geometry(SelectionKind::Ellipse, 15., 15., 24., 30., true),
                        SelectionOperation::Replace,
                        64,
                        64,
                    )
                    .unwrap(),
            )
            .unwrap();
            doc
        };
        let mut a = setup();
        let mut b = setup();
        let mut d = definition(target != 0, false, true, true);
        if target == 0 {
            d.brush.mode = BrushMode::Erase;
        }
        if target == 1 {
            d.brush.mode = BrushMode::Smudge {
                all_layers: false,
                finger_painting: true,
                pressure_strength: true,
            };
        }
        let first = InputPoint::new(20.5, 32.5, 0.5);
        begin(&mut a, d.clone(), first);
        begin(&mut b, d, first);
        let budget = InputScheduleBudget::new(8192, 8_000_000, 4).unwrap();
        for p in [
            InputPoint::new(25.5, 32.5, 0.9),
            InputPoint::new(32.5, 33.5, 0.3),
        ] {
            a.stroke_to(p).unwrap();
            b.begin_stroke_input(p).unwrap();
            drain(&mut b, budget);
            assert_eq!(pixels(&a), pixels(&b));
        }
        let expected = if target == 0 {
            a.end_stroke().unwrap();
            None
        } else {
            Some(a.end_wet_ink_stroke().unwrap().1)
        };
        b.begin_stroke_finish().unwrap();
        let (_, actual) = drain(&mut b, budget);
        ink_eq(actual.unwrap().1, expected);
        assert_eq!(pixels(&a), pixels(&b));
        assert_eq!(a.history_depth(), b.history_depth());
    }
}
#[test]
fn input_paging_document_invalid_input_and_mid_replay_cancel_restore_seed_and_history() {
    let mut doc = Document::new(128, 64).unwrap();
    doc.initialize_white_background().unwrap();
    let original = pixels(&doc);
    let history = doc.history_depth();
    let first = InputPoint::new(16., 32., 1.);
    let d = definition(true, true, true, true);
    begin(&mut doc, d.clone(), first);
    doc.begin_stroke_input(InputPoint::new(80., 32., 0.5))
        .unwrap();
    let budget = InputScheduleBudget::new(8192, 8_000_000, 2).unwrap();
    for _ in 0..100 {
        doc.continue_stroke_page(budget).unwrap();
        if doc
            .stroke
            .as_ref()
            .unwrap()
            .taper
            .as_ref()
            .unwrap()
            .pending_render()
        {
            break;
        }
    }
    assert!(doc
        .stroke
        .as_ref()
        .unwrap()
        .taper
        .as_ref()
        .unwrap()
        .pending_render());
    doc.continue_stroke_page(budget).unwrap();
    doc.cancel_stroke().unwrap();
    assert_eq!(pixels(&doc), original);
    assert_eq!(doc.history_depth(), history);
    assert!(!doc.stroke_input_pending());
    begin(&mut doc, d, first);
    assert!(doc
        .begin_stroke_input(InputPoint::new(f64::NAN, 32., 1.))
        .is_err());
    assert_eq!(pixels(&doc), original);
    assert!(!doc.stroke_active());
    assert_eq!(doc.history_depth(), history);
}
#[test]
fn input_paging_document_tile_and_history_storage_failures_discard_input_and_complete_ink() {
    let mut doc = Document::with_options(
        128,
        64,
        DocumentOptions {
            max_document_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let first = InputPoint::new(16., 32., 1.);
    let mut d = definition(true, true, false, false);
    d.brush.spacing = 0.3;
    begin(&mut doc, d.clone(), first);
    doc.begin_stroke_input(InputPoint::new(100., 32., 1.))
        .unwrap();
    let budget = InputScheduleBudget::new(8192, 8_000_000, 4).unwrap();
    let mut failed = false;
    for _ in 0..1000 {
        match doc.continue_stroke_page(budget) {
            Err(Error::ResourceLimit(_)) => {
                failed = true;
                break;
            }
            Ok(StrokePageResult::Pending) => {}
            other => panic!("expected tile failure: {other:?}"),
        }
    }
    assert!(failed);
    assert!(!doc.stroke_active());
    assert!(!doc.stroke_input_pending());
    assert!(pixels(&doc).iter().all(|p| *p == Pixel::TRANSPARENT));
    assert_eq!(doc.history_depth(), (0, 0));
    let mut doc = Document::with_options(
        128,
        64,
        DocumentOptions {
            max_history_bytes: 2 * TILE_BYTES + 256,
            ..Default::default()
        },
    )
    .unwrap();
    begin(&mut doc, d, first);
    doc.begin_stroke_input(InputPoint::new(100., 32., 1.))
        .unwrap();
    drain(&mut doc, budget);
    doc.begin_stroke_finish().unwrap();
    crate::history_storage::fail_writes_for_test(true);
    let result = loop {
        match doc.continue_stroke_page(budget) {
            Ok(StrokePageResult::Pending) => {}
            result => break result,
        }
    };
    crate::history_storage::fail_writes_for_test(false);
    assert!(
        matches!(result, Err(Error::HistoryStorage(_))),
        "{result:?}"
    );
    assert!(!doc.stroke_active());
    assert!(!doc.stroke_input_pending());
    assert!(pixels(&doc).iter().all(|p| *p == Pixel::TRANSPARENT));
    assert_eq!(doc.history_depth(), (0, 0));
}

#[test]
fn input_paging_document_stationary_independent_pressure_stamps_keep_order_across_single_event_pages(
) {
    for taper in [false, true] {
        let mut d = definition(true, true, taper, false);
        d.tip.pressure_size = true;
        d.tip.pressure_flow = true;
        d.dual = DualBrush::new(
            true,
            DualMode::Multiply,
            SecondaryTip::new(8., d.tip, None, Default::default())
                .unwrap()
                .with_spacing(0.25)
                .unwrap(),
        )
        .with_color(d.dual.color());
        let first = InputPoint::new(16.5, 32.5, 0.2);
        let mut a = Document::new(64, 64).unwrap();
        let mut b = Document::new(64, 64).unwrap();
        begin(&mut a, d.clone(), first);
        begin(&mut b, d, first);
        let budget = InputScheduleBudget::new(8192, 8_000_000, 1).unwrap();
        for pressure in [0.9, 0.3, 0.7] {
            let point = InputPoint::new(16.5, 32.5, pressure);
            a.stroke_to(point).unwrap();
            b.begin_stroke_input(point).unwrap();
            drain(&mut b, budget);
            assert_eq!(pixels(&a), pixels(&b));
        }
        let expected = a.end_wet_ink_stroke().unwrap().1;
        b.begin_stroke_finish().unwrap();
        let (_, actual) = drain(&mut b, budget);
        ink_eq(actual.unwrap().1, Some(expected));
        assert_eq!(pixels(&a), pixels(&b));
    }
}
#[test]
fn input_paging_document_scratch_write_failure_rolls_back_successful_pages() {
    let mut doc = Document::with_options(
        128,
        64,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let mut d = definition(true, true, true, true);
    d.brush.spacing = 0.25;
    begin(&mut doc, d, InputPoint::new(16.5, 32.5, 1.));
    doc.begin_stroke_input(InputPoint::new(100.5, 32.5, 1.))
        .unwrap();
    let budget = InputScheduleBudget::new(8192, 8_000_000, 3).unwrap();
    doc.continue_stroke_page(budget).unwrap();
    crate::page_pool::fail_writes_for_test(true);
    let mut result = None;
    for _ in 0..1000 {
        match doc.continue_stroke_page(budget) {
            Ok(StrokePageResult::Pending) => {}
            outcome => {
                result = Some(outcome);
                break;
            }
        }
    }
    crate::page_pool::fail_writes_for_test(false);
    assert!(matches!(result, Some(Err(Error::Storage(_)))), "{result:?}");
    assert!(!doc.stroke_active());
    assert!(!doc.stroke_input_pending());
    assert_eq!(doc.history_depth(), (0, 0));
    assert!(pixels(&doc).iter().all(|p| *p == Pixel::TRANSPARENT));
}

#[test]
fn input_paging_document_subtick_filter_input_never_replays_or_changes_visible_tail() {
    let mut d = definition(true, true, true, false);
    d.smoothing = BrushSmoothing::new(true, 12., 8., 0.5, 2.).unwrap();
    let first = InputPoint {
        timestamp_ns: 4_000_000,
        ..InputPoint::new(16., 32., 0.2)
    };
    let mut a = Document::new(64, 64).unwrap();
    let mut b = Document::new(64, 64).unwrap();
    begin(&mut a, d.clone(), first);
    begin(&mut b, d, first);
    let before = pixels(&b);
    let revision = b.revision();
    let next = InputPoint {
        timestamp_ns: 5_000_000,
        ..InputPoint::new(24., 34., 0.7)
    };
    a.stroke_to(next).unwrap();
    b.begin_stroke_input(next).unwrap();
    assert!(matches!(
        b.continue_stroke_page(InputScheduleBudget::default())
            .unwrap(),
        StrokePageResult::InputComplete
    ));
    assert_eq!(b.revision(), revision);
    assert_eq!(pixels(&b), before);
    let next = InputPoint {
        timestamp_ns: 12_000_000,
        ..InputPoint::new(32., 32., 0.9)
    };
    a.stroke_to(next).unwrap();
    b.begin_stroke_input(next).unwrap();
    drain(&mut b, InputScheduleBudget::default());
    assert_eq!(pixels(&a), pixels(&b));
    let expected = a.end_wet_ink_stroke().unwrap().1;
    b.begin_stroke_finish().unwrap();
    let (_, actual) = drain(&mut b, InputScheduleBudget::default());
    ink_eq(actual.unwrap().1, Some(expected));
    assert_eq!(pixels(&a), pixels(&b));
}

#[test]
fn input_paging_document_total_transaction_budget_includes_active_tail_replays() {
    let mut doc = Document::new(128, 64).unwrap();
    begin(
        &mut doc,
        definition(true, true, true, true),
        InputPoint::new(16.5, 32.5, 1.),
    );
    doc.begin_stroke_input(InputPoint::new(80.5, 32.5, 1.))
        .unwrap();
    let budget = InputScheduleBudget::new(8192, 8_000_000, 2).unwrap();
    doc.continue_stroke_page(budget).unwrap();
    assert!(doc
        .stroke
        .as_ref()
        .unwrap()
        .taper
        .as_ref()
        .unwrap()
        .pending_render());
    doc.pending_input.as_mut().unwrap().work.visits = MAX_INPUT_SCHEDULE_VISITS;
    assert!(matches!(
        doc.continue_stroke_page(budget),
        Err(Error::ResourceLimit(_))
    ));
    assert!(!doc.stroke_active());
    assert!(!doc.stroke_input_pending());
    assert!(pixels(&doc).iter().all(|p| *p == Pixel::TRANSPARENT));
    assert_eq!(doc.history_depth(), (0, 0));
}

#[test]
fn input_paging_document_zero_opacity_still_returns_picked_up_pair_without_new_history() {
    let mut doc = Document::new(128, 64).unwrap();
    doc.initialize_white_background().unwrap();
    let before = pixels(&doc);
    let history = doc.history_depth();
    let mut d = definition(true, true, false, false);
    d.brush.opacity = 0.;
    begin(&mut doc, d, InputPoint::new(16.5, 32.5, 1.));
    doc.begin_stroke_input(InputPoint::new(80.5, 32.5, 1.))
        .unwrap();
    drain(&mut doc, InputScheduleBudget::default());
    doc.begin_stroke_finish().unwrap();
    let (_, result) = drain(&mut doc, InputScheduleBudget::default());
    let (changed, ink) = result.unwrap();
    assert!(!changed);
    let ink = ink.unwrap();
    assert!(ink.primary().sample(0.5, 0.5).components()[1] > 0.);
    assert!(ink.secondary().unwrap().sample(0.5, 0.5).components()[1] > 0.);
    assert_eq!(pixels(&doc), before);
    assert_eq!(doc.history_depth(), history);
}
