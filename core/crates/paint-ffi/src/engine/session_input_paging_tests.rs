use super::*;
use paint_core::{
    BrushDefinition, BrushMask, BrushSettings, BrushTexture, BrushTip, DualBrush, DualColor,
    DualMode, SecondaryTip, StrokeTaper, TaperEnd, TextureResource, WetInk, WetMix, WetSettings,
    WetTool,
};
fn definition(taper: bool) -> BrushDefinition {
    let tip = BrushTip {
        pressure_size: false,
        pressure_flow: false,
        ..Default::default()
    };
    let mask =
        Arc::new(BrushMask::new(3, 3, vec![255, 0, 255, 255, 128, 255, 0, 255, 255]).unwrap());
    let mut d = BrushSettings {
        diameter: 8.,
        spacing: 0.075,
        opacity: 0.4,
        tip,
        ..Default::default()
    }
    .definition();
    d.mask = Some(mask.clone());
    d.texture = BrushTexture::new(
        true,
        Default::default(),
        Some(Arc::new(TextureResource::new(mask.clone()))),
    )
    .unwrap();
    d.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(8., tip, Some(mask), Default::default())
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
            paint_core::BlendMode::Normal,
            paint_core::BlendMode::Screen,
        )
        .unwrap(),
    );
    d.wet = WetSettings::new(true, false, false, false, WetMix::default()).with_dual_color(true);
    if taper {
        let c = paint_core::PressureControl::default();
        d.taper = StrokeTaper::new(
            true,
            TaperEnd::new(4., c.clone(), c.clone()).unwrap(),
            TaperEnd::new(12., c.clone(), c).unwrap(),
        );
    }
    d
}
fn ink(tool: &WetTool) -> WetInk {
    tool.snapshot_ink().unwrap().unwrap()
}
fn equal_ink(a: &WetInk, b: &WetInk) {
    for (a, b) in [
        (a.primary(), b.primary()),
        (a.secondary().unwrap(), b.secondary().unwrap()),
    ] {
        assert_eq!(a.edge(), b.edge());
        assert_eq!(a.revision(), b.revision());
        for y in 0..a.edge() {
            for x in 0..a.edge() {
                let u = (x as f64 + 0.5) / a.edge() as f64;
                let v = (y as f64 + 0.5) / a.edge() as f64;
                assert_eq!(a.sample(u, v), b.sample(u, v));
            }
        }
    }
}
fn tool() -> WetTool {
    let t = WetTool::default();
    t.reload_dual(
        WetMix::default(),
        [8., 16.],
        [
            paint_core::Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
            paint_core::Pixel::from_straight([0., 0., 1., 1.]).unwrap(),
        ],
    )
    .unwrap();
    t
}
fn first() -> InputPoint {
    InputPoint::new(16.5, 32.5, 1.)
}
fn last() -> InputPoint {
    InputPoint::new(80.5, 32.5, 1.)
}
fn command(sequence: u64, operation: Operation) -> Command {
    Command {
        sequence,
        operation,
    }
}
fn engine(tool: WetTool) -> Engine {
    let storage = crate::storage_api::CoreStorage::system();
    let document = Document::with_storage(128, 64, storage.options, storage.space.clone()).unwrap();
    let initial = publish(&document, 1, 1, 0, None, false);
    let shared = Arc::new(Shared {
        composition: crate::engine::composition::Service::default(),
        task_progress: Mutex::new(None),
        preview_assets: Mutex::new(Vec::new()),
        transform_preview_assets: Mutex::new(Vec::new()),
        preview_selection: Mutex::new(None),
        move_preparation: Mutex::new(None),
        move_preparation_request: AtomicU64::new(0),
        generation: AtomicU64::new(1),
        revision: AtomicU64::new(1),
        floor: AtomicU64::new(0),
        publication: Mutex::new(Arc::new(initial)),
        clipboard: Mutex::new(None),
        views: Mutex::new(std::array::from_fn(|_| ViewSlot {
            preview_layer: 0,
            blend_preview: None,
            request_id: 0,
            generation: 1,
            request: None,
            frame: None,
        })),
    });
    let pool = Arc::new(TaskPool::new(2, 8).unwrap());
    let file_pool = Arc::new(TaskPool::new(1, 1).unwrap());
    let file = Arc::new(Mutex::new(crate::file_job::FileState::default()));
    Engine {
        committed_strokes: Arc::new(VecDeque::new()),
        files: crate::file_job::FileService::new(file, file_pool),
        document,
        active_input: None,
        deferred: VecDeque::new(),
        next_service: Instant::now(),
        wet_tool: Arc::new(tool),
        wet_lease: None,
        shared,
        pool,
        jobs: std::array::from_fn(|_| None),
        renderers: std::array::from_fn(|_| Arc::new(CachedCpuRenderer::default())),
        generation: 1,
        publication: 1,
        completed: 0,
        error: None,
        modified: false,
        dirty: true,
        aborted: false,
        floor: 0,
        frames: 0,
        equivalent_pixels: (0, 0),
        failed_views: std::array::from_fn(|_| None),
    }
}
fn pump(e: &mut Engine, sequence: u64) {
    for _ in 0..10000 {
        if e.completed >= sequence {
            return;
        }
        e.resume_input();
        e.tick();
    }
    panic!("continuation did not finish");
}
fn run(s: &Session, operation: Operation) -> PaintSessionInfo {
    let seq = s.submit(operation).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while s.info().completed_sequence < seq {
        assert!(Instant::now() < deadline, "{}", s.info().last_error_status);
        std::thread::sleep(Duration::from_millis(1));
    }
    s.info()
}
fn reference_stroke(doc: &mut Document, d: BrushDefinition, retained: &WetInk) -> WetInk {
    doc.begin_configured_dual_wet_brush_stroke(d, first(), Some(retained))
        .unwrap();
    doc.begin_stroke_input(last()).unwrap();
    while !matches!(
        doc.continue_stroke_page(paint_core::InputScheduleBudget::default())
            .unwrap(),
        paint_core::StrokePageResult::InputComplete
    ) {}
    doc.begin_stroke_finish().unwrap();
    loop {
        if let paint_core::StrokePageResult::StrokeComplete { wet_ink, .. } = doc
            .continue_stroke_page(paint_core::InputScheduleBudget::default())
            .unwrap()
        {
            return wet_ink.unwrap();
        }
    }
}
#[test]
fn paging_actor_delays_completion_and_end_until_pixels_tail_and_history_finish() {
    let t = tool();
    let mut e = engine(t.clone());
    let d = definition(true);
    e.receive(command(
        1,
        Operation::Begin(Box::new(d.clone()), first(), 0),
    ));
    assert_eq!(e.completed, 1);
    assert!(t.is_busy());
    e.receive(command(2, Operation::Move(last())));
    e.receive(command(3, Operation::End));
    assert_eq!(e.completed, 1);
    assert_eq!(e.deferred.len(), 1);
    assert!(e.committed_strokes.is_empty());
    e.resume_input();
    assert!(e.active_input.is_some());
    assert_eq!(e.completed, 1);
    assert!(!e.document.stroke_page_ready_for_publication());
    let publication = e.shared.publication.lock().unwrap().info.publication;
    e.tick();
    assert_eq!(
        e.shared.publication.lock().unwrap().info.publication,
        publication
    );
    pump(&mut e, 3);
    assert_eq!(e.document.history_depth(), (1, 0));
    assert_eq!(
        e.committed_strokes.iter().copied().collect::<Vec<_>>(),
        vec![3]
    );
    assert!(e.modified);
    assert!(!t.is_busy());
    assert_eq!(e.error.as_ref().map(|e| e.1.status), None);
    assert_eq!(
        e.shared.publication.lock().unwrap().info.completed_sequence,
        3
    );
    let mut reference = Document::new(128, 64).unwrap();
    let expected = reference_stroke(&mut reference, d, &ink(&tool()));
    assert_eq!(
        e.document.copy_layers(&[1]).unwrap(),
        reference.copy_layers(&[1]).unwrap()
    );
    equal_ink(&ink(&t), &expected);
}
#[test]
fn paging_actor_cancel_interrupts_only_current_move_prefix_without_adopting_ink() {
    let t = tool();
    let before = ink(&t);
    let mut e = engine(t.clone());
    e.receive(command(
        1,
        Operation::Begin(Box::new(definition(true)), first(), 0),
    ));
    e.receive(command(2, Operation::Move(last())));
    e.receive(command(3, Operation::Move(InputPoint::new(90., 32., 1.))));
    e.resume_input();
    e.receive(command(4, Operation::Cancel(false)));
    assert_eq!(e.completed, 4);
    assert!(e.active_input.is_none() && e.deferred.is_empty());
    assert!(!e.document.stroke_active());
    assert_eq!(e.document.history_depth(), (0, 0));
    assert!(e.document.snapshot().layers()[0].tiles().next().is_none());
    assert!(!t.is_busy());
    equal_ink(&ink(&t), &before);
    assert!(e.error.is_none());
}
#[test]
fn paging_actor_later_cancel_preserves_ended_stroke_and_following_nonstroke_commands() {
    let t = tool();
    let mut e = engine(t.clone());
    let d = definition(true);
    let mut reference = Document::new(128, 64).unwrap();
    let expected = reference_stroke(&mut reference, d.clone(), &ink(&t));
    e.receive(command(
        1,
        Operation::Begin(Box::new(d.clone()), first(), 0),
    ));
    e.receive(command(2, Operation::Move(last())));
    e.receive(command(3, Operation::End));
    e.receive(command(4, Operation::Begin(Box::new(d), first(), 0)));
    e.receive(command(5, Operation::Move(last())));
    e.receive(command(6, Operation::Cancel(false)));
    e.receive(command(7, Operation::CopyLayers(vec![1])));
    assert_eq!(e.completed, 1);
    pump(&mut e, 6);
    assert_eq!(e.document.history_depth(), (1, 0));
    assert!(!t.is_busy());
    equal_ink(&ink(&t), &expected);
    assert_eq!(
        e.document.copy_layers(&[1]).unwrap(),
        reference.copy_layers(&[1]).unwrap()
    );
    assert_eq!(e.deferred.front().unwrap().sequence, 7);
    pump(&mut e, 7);
    assert!(e.shared.clipboard.lock().unwrap().is_some());
    assert!(e.error.is_none());
}
#[test]
fn paging_actor_new_and_invalid_new_discard_owned_pages_and_release_both_roles() {
    let t = tool();
    let before = ink(&t);
    let mut e = engine(t.clone());
    e.receive(command(
        1,
        Operation::Begin(Box::new(definition(true)), first(), 0),
    ));
    e.receive(command(2, Operation::Move(last())));
    e.receive(command(3, Operation::End));
    e.receive(command(4, Operation::New(64, 64)));
    assert_eq!(e.completed, 4);
    assert_eq!(e.generation, 2);
    assert_eq!(e.document.dimensions(), (64, 64));
    assert_eq!(e.document.history_depth(), (0, 0));
    assert!(!t.is_busy());
    equal_ink(&ink(&t), &before);
    e.receive(command(
        5,
        Operation::Begin(Box::new(definition(false)), first(), 0),
    ));
    e.receive(command(6, Operation::Move(last())));
    e.receive(command(7, Operation::New(0, 0)));
    assert_eq!(e.completed, 7);
    assert!(!e.document.stroke_input_pending());
    assert!(!e.document.stroke_active());
    assert!(!t.is_busy());
    equal_ink(&ink(&t), &before);
    assert_eq!(e.error.as_ref().unwrap().1.status, PAINT_INVALID_ARGUMENT);
}
#[test]
fn paging_actor_failed_page_discards_the_pair_and_end_cannot_commit_failed_pixels() {
    let t = tool();
    let before = ink(&t);
    let mut e = engine(t.clone());
    e.document = Document::with_options(
        128,
        64,
        paint_core::DocumentOptions {
            max_document_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    e.receive(command(
        1,
        Operation::Begin(Box::new(definition(false)), first(), 0),
    ));
    e.receive(command(2, Operation::Move(last())));
    e.receive(command(3, Operation::End));
    pump(&mut e, 3);
    assert_eq!(e.error.as_ref().unwrap().0, 2);
    assert_eq!(e.error.as_ref().unwrap().1.status, PAINT_LIMIT_EXCEEDED);
    assert_eq!(e.document.history_depth(), (0, 0));
    assert!(e.committed_strokes.is_empty());
    assert!(!e.document.stroke_active());
    assert!(!t.is_busy());
    equal_ink(&ink(&t), &before);
}
#[test]
fn paging_actor_real_thread_captures_two_large_input_strokes_and_releases_on_stop() {
    let t = tool();
    let s = Session::with_wet_tool(128, 64, t.clone()).unwrap();
    let d = definition(true);
    let mut reference = Document::new(128, 64).unwrap();
    let mut retained = ink(&t);
    for depth in 1..=2 {
        assert_eq!(
            run(&s, Operation::Begin(Box::new(d.clone()), first(), 0)).last_error_status,
            PAINT_OK
        );
        s.submit(Operation::Move(last())).unwrap();
        let info = run(&s, Operation::End);
        assert_eq!(info.last_error_status, PAINT_OK);
        assert_eq!(info.undo_depth, depth);
        retained = reference_stroke(&mut reference, d.clone(), &retained);
        equal_ink(&ink(&t), &retained);
        let info = run(&s, Operation::CopyLayers(vec![1]));
        assert_eq!(
            s.layer_clipboard(info.completed_sequence).unwrap().as_ref(),
            &reference.copy_layers(&[1]).unwrap()
        );
    }
    let before = ink(&t);
    let mut dense = definition(false);
    dense.brush.spacing = 0.15;
    let begun = run(&s, Operation::Begin(Box::new(dense), first(), 0));
    let moving = s
        .submit(Operation::Move(InputPoint::new(500.5, 32.5, 1.)))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let info = s.info();
        if info.revision > begun.revision && info.completed_sequence < moving {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "did not observe a published intermediate page"
        );
        assert_eq!(info.last_error_status, PAINT_OK);
        std::thread::yield_now();
    }
    s.stop();
    assert!(!t.is_busy());
    equal_ink(&ink(&t), &before);
}
