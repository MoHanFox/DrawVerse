use super::*;
#[path = "session_wet_dual_tests.rs"]
mod dual;
use paint_core::{
    Brush, BrushDefinition, BrushMode, BrushSettings, BrushTip, Pixel, WetCarrier, WetMix,
    WetSettings, WetTool,
};
use std::{thread, time::Instant};
fn run(s: &Session, operation: Operation) -> PaintSessionInfo {
    let sequence = s.submit(operation).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while s.info().completed_sequence < sequence {
        assert!(
            Instant::now() < deadline,
            "native task {sequence} timed out"
        );
        thread::sleep(Duration::from_millis(1));
    }
    s.info()
}
fn copied(s: &Session) -> Vec<u8> {
    let info = run(s, Operation::CopyLayers(vec![1]));
    s.layer_clipboard(info.completed_sequence)
        .unwrap()
        .as_ref()
        .clone()
}
fn red() -> Pixel {
    Pixel::from_straight([1., 0., 0., 1.]).unwrap()
}
fn blue() -> Pixel {
    Pixel::from_straight([0., 0., 1., 1.]).unwrap()
}
fn mix() -> WetMix {
    WetMix::new(0.6, 0.2, 0.2, 0.5, 0.8, 0.05).unwrap()
}
fn definition() -> BrushDefinition {
    let mut b = BrushSettings {
        diameter: 8.,
        spacing: 0.25,
        opacity: 0.4,
        wet: WetSettings::new(true, false, false, false, mix()),
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
    .definition();
    b.brush.color = blue();
    b.brush.background_color = Pixel::WHITE;
    b.dynamics.color =
        paint_core::ColorDynamics::new(true, 0.2, 0.2, 0.1, 0.1, true, u64::MAX).unwrap();
    b
}
fn begin(s: &Session, b: BrushDefinition, x: f64) -> PaintSessionInfo {
    run(
        s,
        Operation::Begin(Box::new(b), InputPoint::new(x, 32.5, 1.), 0),
    )
}
#[test]
fn wet_actor_two_strokes_match_reference_and_keep_tool_out_of_document_history() {
    actor_matches_reference(BrushMode::Paint);
}
#[test]
fn wet_smudge_actor_adopts_ink_and_matches_real_pixels_and_history() {
    actor_matches_reference(BrushMode::Smudge {
        all_layers: false,
        finger_painting: false,
        pressure_strength: true,
    });
}
fn actor_matches_reference(mode: BrushMode) {
    let tool = WetTool::default();
    tool.reload(mix(), 8., red()).unwrap();
    let session = Session::with_wet_tool(128, 64, tool.clone()).unwrap();
    let mut reference = Document::new(128, 64).unwrap();
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
    begin(&session, seed.clone(), 32.5);
    run(&session, Operation::End);
    reference
        .begin_brush_stroke(seed, InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    reference.end_stroke().unwrap();
    let mut retained = WetCarrier::new(mix(), 8., red()).unwrap();
    for color in [blue(), red()] {
        let mut b = definition();
        b.brush.mode = mode;
        b.brush.color = color;
        let first = InputPoint::new(16.5, 32.5, 1.);
        assert_eq!(
            run(&session, Operation::Begin(Box::new(b.clone()), first, 0)).last_error_status,
            PAINT_OK
        );
        reference
            .begin_configured_wet_brush_stroke(b.clone(), first, Some(&retained))
            .unwrap();
        b.brush.color = Pixel::WHITE;
        b.wet = Default::default();
        for x in [48.5, 104.5] {
            let p = InputPoint::new(x, 32.5, 1.);
            run(&session, Operation::Move(p));
            reference.stroke_to(p).unwrap();
        }
        assert_eq!(run(&session, Operation::End).last_error_status, PAINT_OK);
        retained = reference.end_wet_brush_stroke().unwrap().1;
        assert_eq!(copied(&session), reference.copy_layers(&[1]).unwrap());
        let adopted = tool.snapshot().unwrap().unwrap();
        for (u, v) in [(0.25, 0.25), (0.5, 0.5), (0.75, 0.75)] {
            assert_eq!(adopted.sample(u, v), retained.sample(u, v));
        }
    }
    assert_eq!(session.info().undo_depth, 3);
    let after = copied(&session);
    let adopted = tool.snapshot().unwrap().unwrap();
    run(&session, Operation::Undo);
    run(&session, Operation::Undo);
    assert_eq!(
        tool.snapshot().unwrap().unwrap().revision(),
        adopted.revision()
    );
    run(&session, Operation::Redo);
    run(&session, Operation::Redo);
    assert_eq!(copied(&session), after);
    session.stop();
}
#[test]
fn wet_actor_cancel_error_new_and_stop_release_uncommitted_ink() {
    let tool = WetTool::default();
    tool.reload(mix(), 8., red()).unwrap();
    let session = Session::with_wet_tool(128, 64, tool.clone()).unwrap();
    let before = copied(&session);
    let original = tool.snapshot().unwrap().unwrap();
    begin(&session, definition(), 16.5);
    run(&session, Operation::Move(InputPoint::new(96.5, 32.5, 1.)));
    assert!(tool.is_busy());
    run(&session, Operation::Cancel(false));
    assert!(!tool.is_busy());
    assert_eq!(copied(&session), before);
    assert_eq!(session.info().undo_depth, 0);
    assert_eq!(
        tool.snapshot().unwrap().unwrap().revision(),
        original.revision()
    );
    begin(&session, definition(), 16.5);
    let error = run(
        &session,
        Operation::Move(InputPoint::new(1_000_000., 32.5, 1.)),
    );
    assert_eq!(error.last_error_status, PAINT_LIMIT_EXCEEDED);
    assert_eq!(error.stroke_active, 0);
    assert!(!tool.is_busy());
    assert_eq!(copied(&session), before);
    assert_eq!(
        tool.snapshot().unwrap().unwrap().revision(),
        original.revision()
    );
    begin(&session, definition(), 16.5);
    assert!(tool.is_busy());
    let replacement = run(&session, Operation::New(128, 64));
    assert_eq!(replacement.stroke_active, 0);
    assert!(!tool.is_busy());
    assert_eq!(
        tool.snapshot().unwrap().unwrap().sample(0.5, 0.5),
        original.sample(0.5, 0.5)
    );
    begin(&session, definition(), 16.5);
    run(&session, Operation::Cancel(true));
    assert!(!tool.is_busy());
    assert_eq!(session.info().undo_depth, 0);
    begin(&session, definition(), 16.5);
    assert!(tool.is_busy());
    session.stop();
    assert!(!tool.is_busy());
    assert_eq!(
        tool.snapshot().unwrap().unwrap().revision(),
        original.revision()
    );
}
#[test]
fn wet_actor_shared_documents_and_duplicate_begin_cannot_take_active_lease() {
    let tool = WetTool::default();
    tool.reload(mix(), 8., red()).unwrap();
    let a = Session::with_wet_tool(128, 64, tool.clone()).unwrap();
    let b = Session::with_wet_tool(128, 64, tool.clone()).unwrap();
    begin(&a, definition(), 16.5);
    assert!(tool.is_busy());
    let previous = run(&b, Operation::Add("other".into()));
    let conflict = run(
        &b,
        Operation::Begin(Box::new(definition()), InputPoint::new(16.5, 32.5, 1.), 1),
    );
    assert_eq!(conflict.last_error_status, PAINT_BUSY);
    assert_eq!(conflict.stroke_active, 0);
    assert_eq!(conflict.active_layer_id, previous.active_layer_id);
    let duplicate = begin(&a, definition(), 64.5);
    assert_eq!(duplicate.last_error_status, PAINT_BUSY);
    assert_eq!(duplicate.stroke_active, 1);
    assert!(tool.is_busy());
    run(&a, Operation::End);
    assert!(!tool.is_busy());
    assert_eq!(a.info().undo_depth, 1);
    begin(&b, definition(), 64.5);
    assert!(tool.is_busy());
    run(&b, Operation::End);
    assert!(!tool.is_busy());
    assert_eq!(b.info().undo_depth, previous.undo_depth + 1);
    a.stop();
    b.stop();
}
#[test]
fn wet_actor_zero_opacity_pickup_commits_tool_without_pixel_history() {
    let tool = WetTool::default();
    let session = Session::with_wet_tool(128, 64, tool.clone()).unwrap();
    run(&session, Operation::NewWhite(128, 64));
    let before = copied(&session);
    let mut b = definition();
    b.brush.opacity = 0.;
    b.wet = WetSettings::new(
        true,
        false,
        false,
        false,
        WetMix::new(1., 1., 0., 1., 1., 0.).unwrap(),
    );
    begin(&session, b, 32.5);
    let result = run(&session, Operation::End);
    assert_eq!(result.last_error_status, PAINT_OK);
    assert_eq!(result.undo_depth, 0);
    assert_eq!(copied(&session), before);
    assert_eq!(
        tool.snapshot().unwrap().unwrap().sample(0.5, 0.5),
        Pixel::WHITE
    );
    session.stop();
}
