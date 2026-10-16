use super::*;
use paint_core::{
    DualBrush, DualColor, DualColorOrder, DualMode, DualSizeMode, DualSizing, DualWidthRule,
    PressureControl, SecondaryTip, StrokeTaper, TaperEnd, WetInk,
};
fn pair(path: bool, taper: bool, dynamic: bool) -> BrushDefinition {
    let mut b = definition();
    let secondary = SecondaryTip::new(7., b.tip, None, Default::default()).unwrap();
    let secondary = if path {
        secondary.with_spacing(0.35).unwrap()
    } else {
        secondary
    };
    b.dual = DualBrush::new(true, DualMode::Multiply, secondary)
        .with_color(
            DualColor::new(
                true,
                true,
                true,
                DualColorOrder::SecondBehind,
                paint_core::BlendMode::Normal,
                paint_core::BlendMode::Screen,
            )
            .unwrap(),
        )
        .with_sizing(
            DualSizing::new(DualSizeMode::Relative, 8., DualWidthRule::FixedWidth).unwrap(),
        );
    b.brush.background_color = red();
    if !dynamic {
        b.dynamics.color = Default::default();
    }
    if taper {
        let c = PressureControl::default();
        b.taper = StrokeTaper::new(
            true,
            TaperEnd::new(8., c.clone(), c.clone()).unwrap(),
            TaperEnd::new(12., c.clone(), c).unwrap(),
        );
    }
    b
}
fn seeded() -> WetTool {
    let tool = WetTool::default();
    tool.reload_dual(mix(), [8., 16.], [red(), blue()]).unwrap();
    tool
}
fn same(a: &WetInk, b: &WetInk) {
    for (a, b) in [
        (a.primary(), b.primary()),
        (a.secondary().unwrap(), b.secondary().unwrap()),
    ] {
        assert_eq!(a.edge(), b.edge());
        assert_eq!(a.settings(), b.settings());
        assert_eq!(a.revision(), b.revision());
        for y in 0..a.edge() {
            for x in 0..a.edge() {
                let (u, v) = (
                    (x as f64 + 0.5) / a.edge() as f64,
                    (y as f64 + 0.5) / a.edge() as f64,
                );
                assert_eq!(a.sample(u, v), b.sample(u, v));
            }
        }
    }
}
fn unchanged(tool: &WetTool, before: &WetInk) {
    assert!(!tool.is_busy());
    same(&tool.snapshot_ink().unwrap().unwrap(), before);
}

#[test]
fn wet_dual_actor_two_strokes_capture_complete_ink_and_match_document_pixels() {
    for path in [false, true] {
        for taper in [false, true] {
            for dynamic in [false, true] {
                let tool = seeded();
                let session = Session::with_wet_tool(128, 64, tool.clone()).unwrap();
                let mut reference = Document::new(128, 64).unwrap();
                let mut retained = tool.snapshot_ink().unwrap().unwrap();
                for color in [blue(), red()] {
                    let mut definition = pair(path, taper, dynamic);
                    definition.brush.color = color;
                    let p = InputPoint::new(16.5, 32.5, 0.7);
                    assert_eq!(
                        run(
                            &session,
                            Operation::Begin(Box::new(definition.clone()), p, 0)
                        )
                        .last_error_status,
                        PAINT_OK
                    );
                    assert!(tool.is_busy());
                    assert!(matches!(tool.snapshot_ink(), Err(paint_core::Error::Busy)));
                    reference
                        .begin_configured_dual_wet_brush_stroke(
                            definition.clone(),
                            p,
                            Some(&retained),
                        )
                        .unwrap();
                    // The queued definition is captured; changing this caller's clone cannot
                    // change colors, taper, or wet policy already executing in the actor.
                    definition.brush.color = Pixel::WHITE;
                    definition.brush.background_color = Pixel::WHITE;
                    definition.wet = Default::default();
                    definition.dual = Default::default();
                    for (x, y) in [(48.5, 32.5), (72.5, 42.5), (104.5, 42.5)] {
                        let p = InputPoint::new(x, y, 0.9);
                        assert_eq!(
                            run(&session, Operation::Move(p)).last_error_status,
                            PAINT_OK
                        );
                        reference.stroke_to(p).unwrap();
                    }
                    assert_eq!(run(&session, Operation::End).last_error_status, PAINT_OK);
                    retained = reference.end_wet_ink_stroke().unwrap().1;
                    same(&tool.snapshot_ink().unwrap().unwrap(), &retained);
                    assert_eq!(copied(&session), reference.copy_layers(&[1]).unwrap());
                }
                assert_eq!(session.info().undo_depth, 2);
                let after = copied(&session);
                let ink = tool.snapshot_ink().unwrap().unwrap();
                run(&session, Operation::Undo);
                run(&session, Operation::Undo);
                unchanged(&tool, &ink);
                run(&session, Operation::Redo);
                run(&session, Operation::Redo);
                assert_eq!(copied(&session), after);
                unchanged(&tool, &ink);
                session.stop();
            }
        }
    }
}

#[test]
fn wet_dual_actor_cancel_invalid_input_new_and_stop_keep_both_previous_reservoirs() {
    for cause in 0..6 {
        let tool = seeded();
        let original = tool.snapshot_ink().unwrap().unwrap();
        let session = Session::with_wet_tool(128, 64, tool.clone()).unwrap();
        let before = copied(&session);
        assert_eq!(
            begin(&session, pair(true, true, true), 16.5).last_error_status,
            PAINT_OK
        );
        run(&session, Operation::Move(InputPoint::new(72.5, 32.5, 1.)));
        assert!(tool.is_busy());
        match cause {
            0 => {
                run(&session, Operation::Cancel(false));
            }
            1 => {
                assert_eq!(
                    run(&session, Operation::Cancel(true)).last_error_status,
                    PAINT_LIMIT_EXCEEDED
                );
            }
            2 => {
                assert_eq!(
                    run(
                        &session,
                        Operation::Move(InputPoint::new(1_000_000., 32.5, 1.))
                    )
                    .last_error_status,
                    PAINT_LIMIT_EXCEEDED
                );
            }
            3 => {
                run(&session, Operation::New(128, 64));
            }
            4 => {
                run(&session, Operation::NewWhite(128, 64));
            }
            _ => {
                session.stop();
            }
        }
        unchanged(&tool, &original);
        if cause < 5 {
            assert_eq!(session.info().stroke_active, 0);
            assert_eq!(session.info().undo_depth, 0);
            if cause != 4 {
                assert_eq!(copied(&session), before);
            }
            session.stop();
        }
    }
}

#[test]
fn wet_dual_actor_cross_document_busy_and_bad_target_do_not_steal_ink_or_selection() {
    let tool = seeded();
    let original = tool.snapshot_ink().unwrap().unwrap();
    let a = Session::with_wet_tool(128, 64, tool.clone()).unwrap();
    let b = Session::with_wet_tool(128, 64, tool.clone()).unwrap();
    begin(&a, pair(false, false, false), 16.5);
    let previous = run(&b, Operation::Add("other".into()));
    let busy = run(
        &b,
        Operation::Begin(
            Box::new(pair(false, false, false)),
            InputPoint::new(16.5, 32.5, 1.),
            1,
        ),
    );
    assert_eq!(busy.last_error_status, PAINT_BUSY);
    assert_eq!(busy.active_layer_id, previous.active_layer_id);
    assert_eq!(busy.stroke_active, 0);
    assert_eq!(
        begin(&a, pair(true, true, true), 64.5).last_error_status,
        PAINT_BUSY
    );
    assert_eq!(a.info().stroke_active, 1);
    assert!(tool.is_busy());
    run(&a, Operation::Cancel(false));
    unchanged(&tool, &original);
    let bad = run(
        &b,
        Operation::Begin(
            Box::new(pair(false, false, false)),
            InputPoint::new(16.5, 32.5, 1.),
            9999,
        ),
    );
    assert_eq!(bad.last_error_status, PAINT_NOT_FOUND);
    assert_eq!(bad.active_layer_id, previous.active_layer_id);
    unchanged(&tool, &original);
    // A successful complete pair can subsequently be retained by another document.
    begin(&a, pair(false, false, false), 16.5);
    run(&a, Operation::End);
    let adopted = tool.snapshot_ink().unwrap().unwrap();
    let mut reference = Document::new(128, 64).unwrap();
    let definition = pair(true, true, true);
    let p = InputPoint::new(48.5, 32.5, 1.);
    reference
        .begin_configured_dual_wet_brush_stroke(definition.clone(), p, Some(&adopted))
        .unwrap();
    run(&b, Operation::New(128, 64));
    begin(&b, definition, 48.5);
    run(&b, Operation::End);
    let expected = reference.end_wet_ink_stroke().unwrap().1;
    same(&tool.snapshot_ink().unwrap().unwrap(), &expected);
    assert_eq!(copied(&b), reference.copy_layers(&[1]).unwrap());
    a.stop();
    b.stop();
}

#[test]
fn wet_dual_actor_zero_write_adopts_both_and_single_wet_strokes_preserve_unused_second() {
    let tool = WetTool::default();
    let session = Session::with_wet_tool(128, 64, tool.clone()).unwrap();
    run(&session, Operation::NewWhite(128, 64));
    let before = copied(&session);
    let mut definition = pair(false, false, false);
    definition.brush.opacity = 0.;
    definition.wet = WetSettings::new(
        true,
        false,
        true,
        false,
        WetMix::new(1., 1., 0., 1., 1., 0.).unwrap(),
    );
    begin(&session, definition, 32.5);
    let result = run(&session, Operation::End);
    assert_eq!(result.last_error_status, PAINT_OK);
    assert_eq!(result.undo_depth, 0);
    assert_eq!(copied(&session), before);
    let ink = tool.snapshot_ink().unwrap().unwrap();
    assert_eq!(ink.primary().sample(0.5, 0.5), Pixel::WHITE);
    assert_eq!(ink.secondary().unwrap().sample(0.5, 0.5), Pixel::WHITE);
    begin(&session, super::definition(), 32.5);
    run(&session, Operation::End);
    let next = tool.snapshot_ink().unwrap().unwrap();
    let a = next.secondary().unwrap();
    let b = ink.secondary().unwrap();
    assert_eq!(a.revision(), b.revision());
    for y in 0..a.edge() {
        for x in 0..a.edge() {
            let (u, v) = (
                (x as f64 + 0.5) / a.edge() as f64,
                (y as f64 + 0.5) / a.edge() as f64,
            );
            assert_eq!(a.sample(u, v), b.sample(u, v));
        }
    }
    session.stop();
}

#[test]
fn wet_dual_actor_document_budget_failure_discards_both_and_releases_shared_owner() {
    let tool = seeded();
    let original = tool.snapshot_ink().unwrap().unwrap();
    let storage = Arc::new(crate::storage_api::CoreStorage {
        options: paint_core::DocumentOptions {
            max_document_tiles: 1,
            ..Default::default()
        },
        space: paint_storage::ScratchSpace::system(),
    });
    let session =
        Session::with_storage_and_wet_tool(192, 64, storage, Arc::new(tool.clone())).unwrap();
    let before = copied(&session);
    begin(&session, pair(true, false, true), 16.5);
    let error = run(&session, Operation::Move(InputPoint::new(160., 32.5, 1.)));
    assert_eq!(error.last_error_status, PAINT_LIMIT_EXCEEDED);
    assert_eq!(error.stroke_active, 0);
    assert_eq!(error.undo_depth, 0);
    assert_eq!(copied(&session), before);
    unchanged(&tool, &original);
    session.stop();
}

#[test]
fn wet_dual_actor_panic_releases_both_roles_without_adoption() {
    let tool = seeded();
    let original = tool.snapshot_ink().unwrap().unwrap();
    let session = Session::with_wet_tool(128, 64, tool.clone()).unwrap();
    begin(&session, pair(false, false, false), 16.5);
    assert!(tool.is_busy());
    session.submit(Operation::PanicForTest).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while session.sender.state() != WorkerState::Panicked {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
    assert_ne!(session.info().flags & PAINT_SESSION_FAILED, 0);
    session.stop();
    unchanged(&tool, &original);
}
