use super::*;
fn action(h: &Harness, kind: u32) -> u64 {
    let mut sequence = 0;
    assert_eq!(
        unsafe {
            paint_session_layer_action(
                h.core,
                h.session,
                kind,
                [1].as_ptr(),
                1,
                0,
                ptr::null(),
                0,
                &mut sequence,
            )
        },
        PAINT_OK
    );
    sequence
}
fn clipboard(h: &Harness, sequence: u64) -> Vec<u8> {
    let mut size = 0;
    assert_eq!(
        unsafe {
            paint_session_read_layer_clipboard(
                h.core,
                h.session,
                sequence,
                ptr::null_mut(),
                0,
                &mut size,
            )
        },
        PAINT_BUFFER_TOO_SMALL
    );
    let mut bytes = vec![0; size as usize];
    assert_eq!(
        unsafe {
            paint_session_read_layer_clipboard(
                h.core,
                h.session,
                sequence,
                bytes.as_mut_ptr(),
                size,
                &mut size,
            )
        },
        PAINT_OK
    );
    bytes
}
#[test]
fn cut_abi_sequence_clipboard_single_history_and_failed_cut_keep_previous_copy() {
    let h = Harness::new(64, 64);
    h.wait(h.stroke(16., 16.));
    let copied = action(&h, PAINT_LAYER_ACTION_COPY);
    h.wait(copied);
    let original = clipboard(&h, copied);
    let failed = action(&h, PAINT_LAYER_ACTION_CUT_SELECTED_PIXELS);
    let info = h.wait(failed);
    assert_eq!(info.last_error_status, PAINT_INVALID_ARGUMENT);
    assert_eq!(info.undo_depth, 1);
    assert_eq!(clipboard(&h, copied), original);
    let cut = action(&h, PAINT_LAYER_ACTION_CUT);
    let info = h.wait(cut);
    // Session errors are sticky; the successful cut must not rewrite the
    // preceding rejection. Its sequence and clipboard prove completion.
    assert_eq!(info.last_error_sequence, failed);
    assert_eq!(info.completed_sequence, cut);
    assert_eq!(info.undo_depth, 2);
    assert_eq!(info.layer_count, 0);
    assert_eq!(clipboard(&h, cut), original);
    let other = Harness::new(64, 64);
    let mut sequence = 0;
    assert_eq!(
        unsafe {
            paint_session_layer_action(
                other.core,
                other.session,
                PAINT_LAYER_ACTION_PASTE,
                ptr::null(),
                0,
                1,
                original.as_ptr(),
                original.len() as u64,
                &mut sequence,
            )
        },
        PAINT_OK
    );
    let pasted = other.wait(sequence);
    assert_eq!(pasted.last_error_status, PAINT_OK);
    assert_eq!(pasted.undo_depth, 1);
    let undo = h.submit(PaintCommand {
        kind: PAINT_COMMAND_UNDO,
        ..dto!(PaintCommand)
    });
    let info = h.wait(undo);
    assert_eq!(info.layer_count, 1);
    assert_eq!(info.undo_depth, 1);
    assert_eq!(clipboard(&h, cut), original);
}
