use super::*;

fn status(h: &Harness) -> String {
    unsafe {
        let mut required = 0;
        assert_eq!(
            paint_session_compute_status(h.core, h.session, ptr::null_mut(), 0, &mut required),
            PAINT_BUFFER_TOO_SMALL
        );
        let mut bytes = vec![0; required as usize];
        assert_eq!(
            paint_session_compute_status(
                h.core,
                h.session,
                bytes.as_mut_ptr(),
                required,
                &mut required
            ),
            PAINT_OK
        );
        String::from_utf8(bytes).unwrap()
    }
}
#[test]
fn compute_hardware_cpu_fallback_busy_and_new_document() {
    let h = Harness::new(32, 32);
    unsafe {
        let mut required = 0;
        assert_eq!(
            paint_compute_hardware(h.core, ptr::null_mut(), 0, &mut required),
            PAINT_BUFFER_TOO_SMALL
        );
        let mut bytes = vec![0; required as usize];
        assert_eq!(
            paint_compute_hardware(h.core, bytes.as_mut_ptr(), required, &mut required),
            PAINT_OK
        );
        let devices: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(devices[0]["id"], "cpu");
        let mut seq = 99;
        assert_eq!(
            paint_session_set_compute(h.core, h.session, b"".as_ptr(), 0, &mut seq),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(seq, 0);
        let id = b"missing-device-regression";
        assert_eq!(
            paint_session_set_compute(h.core, h.session, id.as_ptr(), id.len() as u64, &mut seq),
            PAINT_OK
        );
        assert_eq!(h.wait(seq).last_error_status, PAINT_OK);
        assert!(status(&h).starts_with("CPU"));
        let fallback = status(&h);
        h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_NEW_DOCUMENT,
            width: 24,
            height: 24,
            ..dto!(PaintCommand)
        }));
        assert_eq!(status(&h), fallback);
        assert_eq!(
            paint_session_liquify(
                h.core,
                h.session,
                PAINT_LIQUIFY_BEGIN,
                ptr::null(),
                &mut seq
            ),
            PAINT_OK
        );
        h.wait(seq);
        assert_eq!(
            paint_session_set_compute(h.core, h.session, b"cpu".as_ptr(), 3, &mut seq),
            PAINT_OK
        );
        assert_eq!(h.wait(seq).last_error_status, PAINT_BUSY);
        assert_eq!(status(&h), fallback);
        assert_eq!(
            paint_session_liquify(
                h.core,
                h.session,
                PAINT_LIQUIFY_CANCEL,
                ptr::null(),
                &mut seq
            ),
            PAINT_OK
        );
        h.wait(seq);
        assert_eq!(
            paint_session_set_compute(h.core, h.session, b"cpu".as_ptr(), 3, &mut seq),
            PAINT_OK
        );
        h.wait(seq);
        assert_eq!(status(&h), "CPU");
    }
}
