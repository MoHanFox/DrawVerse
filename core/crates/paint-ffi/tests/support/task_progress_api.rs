use super::*;

#[test]
fn live_task_progress_is_readable_before_publication_and_retains_completion() {
    assert_eq!(size_of::<PaintTaskProgress>(), 72);
    assert_eq!(offset_of!(PaintTaskProgress, sequence), 24);
    let h = Harness::new(1024, 768);
    let read = || unsafe {
        let mut value = dto!(PaintTaskProgress);
        assert_eq!(
            paint_session_task_progress(h.core, h.session, &mut value),
            PAINT_OK
        );
        value
    };
    assert_eq!(read().state, 0);
    let request = PaintBucketFill {
        x: 0,
        y: 0,
        linear_rgba: [0.8, 0.2, 0.1, 0.75],
        opacity: 1.,
        contiguous: 1,
        ..dto!(PaintBucketFill)
    };
    let mut sequence = 0;
    unsafe {
        assert_eq!(
            paint_session_bucket_fill(h.core, h.session, &request, &mut sequence),
            PAINT_OK
        );
    }
    let before = h.wait(sequence);
    unsafe {
        assert_eq!(
            paint_session_transform(
                h.core,
                h.session,
                PAINT_TRANSFORM_FLIP_IMAGE_HORIZONTAL,
                ptr::null(),
                &mut sequence
            ),
            PAINT_OK
        );
    }
    let started = Instant::now();
    let mut saw_live = false;
    let mut saw_completed_unit = false;
    let mut previous = (0, 0);
    loop {
        let progress = read();
        assert!(started.elapsed() < Duration::from_secs(10));
        if progress.sequence == sequence {
            assert_eq!(progress.kind, PAINT_TASK_FLIP);
            assert_eq!(progress.document_generation, before.document_generation);
            assert!(progress.completed <= progress.total);
            if progress.stage == previous.0 {
                assert!(progress.completed >= previous.1);
            }
            previous = (progress.stage, progress.completed);
            if progress.state == PAINT_TASK_RUNNING && h.info().completed_sequence < sequence {
                saw_live = true;
                saw_completed_unit |= progress.total > 0
                    && progress.completed > 0
                    && progress.completed < progress.total;
            }
            if progress.state != PAINT_TASK_RUNNING {
                assert_eq!(progress.state, PAINT_TASK_SUCCEEDED);
                assert_eq!(progress.status, PAINT_OK);
                break;
            }
        }
        thread::yield_now();
    }
    assert!(
        saw_live && saw_completed_unit,
        "must observe actual tile work while the actor has not published completion"
    );
    let after = h.wait(sequence);
    assert_eq!(after.undo_depth, before.undo_depth + 1);
    let elapsed = read().elapsed_ms;
    thread::sleep(Duration::from_millis(3));
    assert_eq!(read().elapsed_ms, elapsed);
    unsafe {
        assert_eq!(
            paint_session_transform(
                h.core,
                h.session,
                PAINT_TRANSFORM_COMMIT,
                ptr::null(),
                &mut sequence
            ),
            PAINT_OK
        );
    }
    let failed = h.wait(sequence);
    assert_eq!(failed.revision, after.revision);
    let progress = read();
    assert_eq!(progress.sequence, sequence);
    assert_eq!(progress.state, PAINT_TASK_FAILED);
    assert_eq!(progress.status, PAINT_INVALID_ARGUMENT);
    unsafe {
        assert_eq!(
            paint_session_task_progress(h.core, h.session, ptr::null_mut()),
            PAINT_INVALID_ARGUMENT
        );
        let mut short = PaintTaskProgress {
            struct_size: 4,
            ..Default::default()
        };
        assert_eq!(
            paint_session_task_progress(h.core, h.session, &mut short),
            PAINT_INVALID_ARGUMENT
        );
        let mut invalid = dto!(PaintTaskProgress);
        invalid.sequence = 99;
        assert_eq!(
            paint_session_task_progress(h.core, ptr::null_mut(), &mut invalid),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(invalid.sequence, 0);
        let mut caps = dto!(PaintCapabilities);
        assert_eq!(paint_core_capabilities(h.core, &mut caps), PAINT_OK);
        assert_ne!(caps.features & PAINT_FEATURE_TASK_PROGRESS, 0);
    }
}
