use super::*;

#[test]
fn history_selection_jumps_once_rejects_stale_scopes_and_preserves_redo() {
    let h = Harness::new(256, 32);
    for i in 0..24 {
        h.wait(h.stroke(5. + i as f64 * 10., 16.));
    }
    let initial = h.info();
    assert_eq!(initial.undo_depth, 24);
    let request = |info: PaintSessionInfo, depth: u32| PaintHistorySelect {
        scope: PAINT_HISTORY_SCOPE_DOCUMENT,
        depth,
        document_generation: info.document_generation,
        revision: info.revision,
        ..dto!(PaintHistorySelect)
    };
    let submit = |value: &PaintHistorySelect| unsafe {
        let mut sequence = 0;
        assert_eq!(
            paint_session_select_history(h.core, h.session, value, &mut sequence),
            PAINT_OK
        );
        sequence
    };
    let select = |value: &PaintHistorySelect| h.wait(submit(value));
    let first = select(&request(initial, 0));
    assert_eq!((first.undo_depth, first.redo_depth), (0, 24));
    assert_eq!(first.revision, initial.revision + 1);
    let final_state = select(&request(first, 24));
    assert_eq!((final_state.undo_depth, final_state.redo_depth), (24, 0));
    assert_eq!(final_state.revision, first.revision + 1);
    let same = select(&request(final_state, 24));
    assert_eq!(same.revision, final_state.revision);
    // Two requests copied from the same state: only the first may execute.
    let sequence = submit(&request(same, 1));
    let stale_sequence = submit(&request(same, 0));
    h.wait(sequence);
    let stale = h.wait(stale_sequence);
    assert_eq!(stale.last_error_status, PAINT_BUSY);
    assert_eq!(stale.last_error_sequence, stale_sequence);
    assert_eq!((stale.undo_depth, stale.redo_depth), (1, 23));
    assert_eq!(stale.revision, same.revision + 1);
    let invalid = select(&request(stale, 25));
    assert_eq!(invalid.last_error_status, PAINT_INVALID_ARGUMENT);
    assert_eq!(invalid.revision, stale.revision);
    assert_eq!((invalid.undo_depth, invalid.redo_depth), (1, 23));
    let mut wrong_scope = request(invalid, 0);
    wrong_scope.scope = PAINT_HISTORY_SCOPE_LIQUIFY;
    let wrong = select(&wrong_scope);
    assert_eq!(wrong.last_error_status, PAINT_BUSY);
    assert_eq!(wrong.revision, invalid.revision);
    let before_new = request(wrong, 0);
    let new = h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_NEW_DOCUMENT,
        width: 32,
        height: 32,
        ..dto!(PaintCommand)
    }));
    let old = select(&before_new);
    assert_eq!(old.last_error_status, PAINT_BUSY);
    assert_eq!(old.document_generation, new.document_generation);
    assert_eq!((old.undo_depth, old.redo_depth), (0, 0));
    unsafe {
        let mut sequence = 99;
        let mut descriptor = request(old, 0);
        descriptor.reserved = 1;
        assert_eq!(
            paint_session_select_history(h.core, h.session, &descriptor, &mut sequence),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        assert_eq!(
            paint_session_select_history(h.core, h.session, ptr::null(), &mut sequence),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        descriptor.reserved = 0;
        descriptor.scope = 99;
        assert_eq!(
            paint_session_select_history(h.core, h.session, &descriptor, &mut sequence),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        let mut capabilities = dto!(PaintCapabilities);
        assert_eq!(paint_core_capabilities(h.core, &mut capabilities), PAINT_OK);
        assert_ne!(capabilities.features & PAINT_FEATURE_HISTORY_SELECT, 0);
    }
}
