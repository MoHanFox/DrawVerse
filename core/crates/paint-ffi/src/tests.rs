use super::*;
use std::{
    mem::{align_of, offset_of},
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc, Mutex,
    },
    thread,
    time::Duration,
};

struct Fixture {
    core: *mut PaintCore,
    doc: *mut PaintDocument,
}
impl Fixture {
    fn new() -> Self {
        unsafe {
            let mut core = ptr::null_mut();
            let mut doc = ptr::null_mut();
            assert_eq!(paint_core_create(&mut core), PAINT_OK);
            assert_eq!(
                paint_core_new_document(core, &document_desc(), &mut doc),
                PAINT_OK
            );
            Self { core, doc }
        }
    }
    fn info(&self) -> PaintDocumentInfo {
        unsafe {
            let mut info = PaintDocumentInfo {
                struct_size: size_of::<PaintDocumentInfo>() as u32,
                ..Default::default()
            };
            assert_eq!(
                paint_document_info(self.core, self.doc, &mut info),
                PAINT_OK
            );
            info
        }
    }
    fn dot(&self) {
        unsafe {
            assert_eq!(
                paint_core_begin_stroke(self.core, self.doc, &stroke_desc(), &sample()),
                PAINT_OK
            );
            assert_eq!(paint_core_end_stroke(self.core, self.doc), PAINT_OK);
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            paint_document_destroy(self.core, &mut self.doc);
            paint_core_destroy(&mut self.core);
        }
    }
}
fn document_desc() -> PaintDocumentDesc {
    PaintDocumentDesc {
        struct_size: size_of::<PaintDocumentDesc>() as u32,
        width: 128,
        height: 64,
        working_space: PAINT_WORKING_LINEAR_SRGB,
        pixel_format: PAINT_STORAGE_RGBA32F_PREMULTIPLIED,
        ..Default::default()
    }
}
fn stroke_desc() -> PaintStrokeDesc {
    PaintStrokeDesc {
        struct_size: size_of::<PaintStrokeDesc>() as u32,
        mode: PAINT_MODE_PAINT,
        layer_id: 1,
        radius: 4.0,
        opacity: 1.0,
        spacing: 0.25,
        linear_rgba: [1.0, 0.0, 0.0, 0.5],
        ..Default::default()
    }
}
fn sample() -> PaintPoint {
    PaintPoint {
        struct_size: size_of::<PaintPoint>() as u32,
        tool: PAINT_TOOL_PEN,
        capabilities: PAINT_INPUT_PRESSURE,
        x: 16.5,
        y: 16.5,
        pressure: 1.0,
        ..Default::default()
    }
}
fn tile(buffer: *mut u8, capacity: u64, stride: u64) -> PaintTile {
    PaintTile {
        struct_size: size_of::<PaintTile>() as u32,
        format: PAINT_TILE_RGBA32F_LINEAR_PREMULTIPLIED,
        data: buffer,
        capacity,
        stride,
        required: 999,
        width: 99,
        height: 99,
        revision: 99,
    }
}
fn error_text() -> Vec<u8> {
    unsafe {
        let mut len = 0;
        let status = paint_error_message(ptr::null_mut(), 0, &mut len);
        assert_eq!(
            status,
            if len == 0 {
                PAINT_OK
            } else {
                PAINT_BUFFER_TOO_SMALL
            }
        );
        let mut bytes = vec![0; len as usize];
        assert_eq!(
            paint_error_message(bytes.as_mut_ptr(), len, &mut len),
            PAINT_OK
        );
        bytes
    }
}

#[test]
fn abi_layout_matches_frozen_64_bit_contract() {
    assert_eq!(size_of::<PaintVersion>(), 16);
    assert_eq!(size_of::<PaintCapabilities>(), 32);
    assert_eq!(size_of::<PaintDocumentDesc>(), 32);
    assert_eq!(size_of::<PaintDocumentInfo>(), 64);
    assert_eq!(size_of::<PaintLayerInfo>(), 40);
    assert_eq!(size_of::<PaintPoint>(), 64);
    assert_eq!(size_of::<PaintStrokeDesc>(), 56);
    assert_eq!(size_of::<PaintTile>(), 56);
    assert_eq!(size_of::<PaintEvent>(), 56);
    assert_eq!(align_of::<PaintPoint>(), 8);
    assert_eq!(offset_of!(PaintPoint, timestamp_ns), 56);
    assert_eq!(offset_of!(PaintStrokeDesc, linear_rgba), 28);
    assert_eq!(offset_of!(PaintTile, revision), 48);
    assert_eq!(offset_of!(PaintDocumentInfo, document_id), 16);
}

#[test]
fn capabilities_report_only_real_current_features() {
    let f = Fixture::new();
    unsafe {
        let mut caps = PaintCapabilities {
            struct_size: size_of::<PaintCapabilities>() as u32,
            ..Default::default()
        };
        assert_eq!(paint_core_capabilities(f.core, &mut caps), PAINT_OK);
        assert_eq!(caps.features & 15, 15);
        assert_eq!(
            caps.features & (PAINT_FEATURE_ASYNC_SESSION | PAINT_FEATURE_CPU_VIEWPORT),
            48
        );
        assert_eq!(caps.max_read_tile_edge, 256);
        let mut version = PaintVersion {
            struct_size: 16,
            ..Default::default()
        };
        assert_eq!(paint_core_version(&mut version), PAINT_OK);
        assert_eq!((version.major, version.minor, version.patch), (1, 11, 0));
        assert_eq!(
            caps.features & PAINT_FEATURE_CLIPPING,
            PAINT_FEATURE_CLIPPING
        );
        assert_eq!(caps.features & PAINT_FEATURE_FILE_IO, PAINT_FEATURE_FILE_IO);
        assert_eq!(
            caps.features & PAINT_FEATURE_SELECTION,
            PAINT_FEATURE_SELECTION
        );
        assert_eq!(
            caps.features & PAINT_FEATURE_LAYER_GROUPS,
            PAINT_FEATURE_LAYER_GROUPS
        );
        assert_eq!(
            caps.features & PAINT_FEATURE_STORAGE_SETTINGS,
            PAINT_FEATURE_STORAGE_SETTINGS
        );
    }
}

#[test]
fn lifecycle_busy_cross_core_and_idempotent_release() {
    let mut f = Fixture::new();
    let other = Fixture::new();
    unsafe {
        assert_eq!(paint_core_destroy(&mut f.core), PAINT_BUSY);
        let original = f.doc;
        assert_eq!(
            paint_document_destroy(other.core, &mut f.doc),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(f.doc, original);
        assert_eq!(paint_document_destroy(f.core, &mut f.doc), PAINT_OK);
        assert!(f.doc.is_null());
        assert_eq!(paint_document_destroy(f.core, &mut f.doc), PAINT_OK);
        assert_eq!(paint_core_destroy(&mut f.core), PAINT_OK);
        assert_eq!(paint_core_destroy(&mut f.core), PAINT_OK);
    }
}

#[test]
fn released_wrong_type_and_unregistered_handles_are_never_dereferenced() {
    let mut f = Fixture::new();
    let old = f.doc;
    unsafe {
        assert_eq!(paint_document_destroy(f.core, &mut f.doc), PAINT_OK);
        assert_eq!(
            paint_core_new_document(f.core, &document_desc(), &mut f.doc),
            PAINT_OK
        );
        assert_ne!(f.doc, old);
        assert_eq!(paint_core_undo(f.core, old), PAINT_INVALID_HANDLE);
        assert_eq!(paint_core_undo(f.core, f.core.cast()), PAINT_INVALID_HANDLE);
        assert_eq!(
            paint_core_undo(f.core, ptr::without_provenance_mut(12345)),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(
            paint_core_undo(ptr::null_mut(), f.doc),
            PAINT_INVALID_HANDLE
        );
    }
}

#[test]
fn null_misaligned_and_small_struct_parameters_are_rejected_without_writes() {
    unsafe {
        assert_eq!(paint_core_create(ptr::null_mut()), PAINT_INVALID_ARGUMENT);
        assert_eq!(paint_core_version(ptr::null_mut()), PAINT_INVALID_ARGUMENT);
        let mut header = 4_u32;
        assert_eq!(
            paint_core_version((&mut header as *mut u32).cast()),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(header, 4);
        let mut bytes = [0xa5_u8; 64];
        let offset = (4 - bytes.as_mut_ptr().addr() % 4) % 4 + 1;
        assert_eq!(
            paint_core_version(bytes.as_mut_ptr().add(offset).cast()),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(bytes, [0xa5; 64]);
    }
}

#[test]
fn larger_struct_prefix_does_not_overwrite_extension_tail() {
    #[repr(C)]
    struct Extended {
        prefix: PaintVersion,
        tail: [u32; 2],
    }
    let mut value = Extended {
        prefix: PaintVersion {
            struct_size: size_of::<Extended>() as u32,
            ..Default::default()
        },
        tail: [0xdeadbeef; 2],
    };
    unsafe {
        assert_eq!(paint_core_version(&mut value.prefix), PAINT_OK);
    }
    assert_eq!(value.tail, [0xdeadbeef; 2]);
    assert_eq!(value.prefix.major, 1);
}

#[test]
fn creation_failure_clears_out_pointer_and_rejects_unknown_format_reserved() {
    let f = Fixture::new();
    unsafe {
        let mut output = f.doc;
        let bad = PaintDocumentDesc {
            width: 0,
            ..document_desc()
        };
        assert_eq!(
            paint_core_new_document(f.core, &bad, &mut output),
            PAINT_INVALID_ARGUMENT
        );
        assert!(output.is_null());
        let bad = PaintDocumentDesc {
            pixel_format: 999,
            ..document_desc()
        };
        assert_eq!(
            paint_core_new_document(f.core, &bad, &mut output),
            PAINT_UNSUPPORTED
        );
        let bad = PaintDocumentDesc {
            reserved: [1, 0, 0],
            ..document_desc()
        };
        assert_eq!(
            paint_core_new_document(f.core, &bad, &mut output),
            PAINT_INVALID_ARGUMENT
        );
        assert!(output.is_null());
    }
}

#[test]
fn utf8_layer_copy_small_buffer_and_layer_history() {
    let f = Fixture::new();
    unsafe {
        let name = "绘画 / ink".as_bytes();
        let mut id = 999;
        assert_eq!(
            paint_layer_add(f.core, f.doc, name.as_ptr(), name.len() as u64, &mut id),
            PAINT_OK
        );
        let mut len = 0;
        assert_eq!(
            paint_layer_name(f.core, f.doc, id, ptr::null_mut(), 0, &mut len),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(len, name.len() as u64);
        let mut small = [0x55; 2];
        assert_eq!(
            paint_layer_name(f.core, f.doc, id, small.as_mut_ptr(), 2, &mut len),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(small, [0x55; 2]);
        let mut bytes = vec![0; len as usize];
        assert_eq!(
            paint_layer_name(f.core, f.doc, id, bytes.as_mut_ptr(), len, &mut len),
            PAINT_OK
        );
        assert_eq!(bytes, name);
        assert_eq!(
            paint_layer_set_properties(f.core, f.doc, id, 2, 0.5),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(
            paint_layer_set_properties(f.core, f.doc, id, 1, 0.5),
            PAINT_OK
        );
        assert_eq!(paint_layer_remove(f.core, f.doc, id), PAINT_OK);
        assert_eq!(f.info().layer_count, 1);
        assert_eq!(paint_core_undo(f.core, f.doc), PAINT_OK);
        let mut info = PaintLayerInfo {
            struct_size: size_of::<PaintLayerInfo>() as u32,
            ..Default::default()
        };
        assert_eq!(paint_layer_info(f.core, f.doc, 1, &mut info), PAINT_OK);
        assert_eq!((info.layer_id, info.opacity), (id, 0.5));
    }
}

#[test]
fn invalid_utf8_length_nul_and_missing_layer_do_not_mutate() {
    let f = Fixture::new();
    unsafe {
        let mut id = 77;
        for name in [b"bad\0name".as_slice(), &[0xff_u8], b""] {
            assert_eq!(
                paint_layer_add(f.core, f.doc, name.as_ptr(), name.len() as u64, &mut id),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(id, 0);
        }
        assert_eq!(
            paint_layer_add(f.core, f.doc, ptr::null(), u64::MAX, &mut id),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(paint_layer_set_active(f.core, f.doc, 77), PAINT_NOT_FOUND);
        let mut info = PaintLayerInfo {
            struct_size: size_of::<PaintLayerInfo>() as u32,
            layer_id: 999,
            ..Default::default()
        };
        assert_eq!(
            paint_layer_info(f.core, f.doc, 999, &mut info),
            PAINT_NOT_FOUND
        );
        assert_eq!(info.layer_id, 0);
        assert_eq!(f.info().layer_count, 1);
    }
}

#[test]
fn invalid_begin_preserves_selected_layer_and_active_stroke() {
    let f = Fixture::new();
    unsafe {
        let mut second = 0;
        assert_eq!(
            paint_layer_add(f.core, f.doc, b"top".as_ptr(), 3, &mut second),
            PAINT_OK
        );
        let bad = PaintPoint {
            pressure: f32::NAN,
            ..sample()
        };
        assert_eq!(
            paint_core_begin_stroke(f.core, f.doc, &stroke_desc(), &bad),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(f.info().active_layer_id, second);
        assert_eq!(f.info().stroke_active, 0);
        assert_eq!(
            paint_core_begin_stroke(f.core, f.doc, &stroke_desc(), &sample()),
            PAINT_OK
        );
        assert_eq!(
            paint_core_begin_stroke(f.core, f.doc, &stroke_desc(), &sample()),
            PAINT_BUSY
        );
        assert_eq!(f.info().stroke_active, 1);
        assert_eq!(paint_core_cancel_stroke(f.core, f.doc), PAINT_OK);
        assert_eq!(f.info().allocated_tiles, 0);
    }
}

#[test]
fn stroke_history_and_float_buffer_roundtrip() {
    let f = Fixture::new();
    f.dot();
    unsafe {
        let mut bytes = [0_u8; 16];
        let mut output = tile(bytes.as_mut_ptr(), 16, 16);
        assert_eq!(
            paint_core_read_tile(f.core, f.doc, 16, 16, 1, 1, &mut output),
            PAINT_OK
        );
        let components: Vec<_> = bytes
            .chunks_exact(4)
            .map(|bytes| f32::from_ne_bytes(bytes.try_into().unwrap()))
            .collect();
        assert_eq!(components, [0.5, 0.0, 0.0, 0.5]);
        let original = bytes;
        assert_eq!(paint_core_undo(f.core, f.doc), PAINT_OK);
        assert_eq!(
            paint_core_read_tile(f.core, f.doc, 16, 16, 1, 1, &mut output),
            PAINT_OK
        );
        assert_eq!(bytes, [0; 16]);
        assert_eq!(paint_core_redo(f.core, f.doc), PAINT_OK);
        assert_eq!(
            paint_core_read_tile(f.core, f.doc, 16, 16, 1, 1, &mut output),
            PAINT_OK
        );
        assert_eq!(bytes, original);
        assert_eq!(output.revision, f.info().revision);
    }
}

#[test]
fn tile_query_and_padding_are_atomic() {
    let f = Fixture::new();
    f.dot();
    unsafe {
        let mut output = tile(ptr::null_mut(), 0, 40);
        assert_eq!(
            paint_core_read_tile(f.core, f.doc, 15, 15, 2, 2, &mut output),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(output.required, 72);
        assert_eq!((output.width, output.height, output.revision), (0, 0, 0));
        let mut bytes = [0xa5_u8; 80];
        output.data = bytes.as_mut_ptr();
        output.capacity = 71;
        assert_eq!(
            paint_core_read_tile(f.core, f.doc, 15, 15, 2, 2, &mut output),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(bytes, [0xa5; 80]);
        output.capacity = 72;
        assert_eq!(
            paint_core_read_tile(f.core, f.doc, 15, 15, 2, 2, &mut output),
            PAINT_OK
        );
        assert_eq!(&bytes[32..40], &[0xa5; 8]);
        assert_eq!(&bytes[72..80], &[0xa5; 8]);
    }
}

#[test]
fn tile_invalid_roi_stride_overflow_capacity_and_format_never_write_pixels() {
    let f = Fixture::new();
    unsafe {
        let mut bytes = [0x99_u8; 32];
        let cases = [
            (u32::MAX, 0, 1, 1, 16),
            (128, 0, 1, 1, 16),
            (0, 0, 0, 1, 16),
            (0, 0, 257, 1, 4112),
            (0, 0, 2, 1, 16),
            (0, 0, 1, 3, u64::MAX),
        ];
        for (x, y, w, h, stride) in cases {
            let mut output = tile(bytes.as_mut_ptr(), 32, stride);
            assert_eq!(
                paint_core_read_tile(f.core, f.doc, x, y, w, h, &mut output),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(output.required, 0);
            assert_eq!(output.width, 0);
            assert_eq!(bytes, [0x99; 32]);
        }
        let mut output = tile(bytes.as_mut_ptr(), u64::MAX, 16);
        assert_eq!(
            paint_core_read_tile(f.core, f.doc, 0, 0, 1, 1, &mut output),
            PAINT_INVALID_ARGUMENT
        );
        output.capacity = 32;
        output.format = 999;
        assert_eq!(
            paint_core_read_tile(f.core, f.doc, 0, 0, 1, 1, &mut output),
            PAINT_UNSUPPORTED
        );
        assert_eq!(bytes, [0x99; 32]);
    }
}

#[test]
fn errors_are_thread_local_queries_preserve_error_and_success_clears() {
    let f = Fixture::new();
    unsafe {
        assert_eq!(paint_core_end_stroke(f.core, f.doc), PAINT_INVALID_ARGUMENT);
    }
    let original = error_text();
    let worker = thread::spawn(|| {
        unsafe {
            assert_eq!(paint_core_create(ptr::null_mut()), PAINT_INVALID_ARGUMENT);
        }
        error_text()
    })
    .join()
    .unwrap();
    assert_ne!(original, worker);
    assert_eq!(error_text(), original);
    assert!(String::from_utf8(original)
        .unwrap()
        .contains("no active stroke"));
    f.info();
    assert!(error_text().is_empty());
}

#[test]
fn panic_is_contained_poisoned_document_is_quarantined_and_can_be_released() {
    let f = Fixture::new();
    assert_eq!(
        boundary(|| runtime::mutate_document(f.core, f.doc, |_| panic!(
            "deliberate boundary regression"
        ))),
        PAINT_INTERNAL_ERROR
    );
    assert!(String::from_utf8(error_text())
        .unwrap()
        .contains("panic contained"));
    unsafe {
        let mut info = PaintDocumentInfo {
            struct_size: 64,
            ..Default::default()
        };
        assert_eq!(
            paint_document_info(f.core, f.doc, &mut info),
            PAINT_INTERNAL_ERROR
        );
        assert_eq!(paint_core_undo(f.core, f.doc), PAINT_INTERNAL_ERROR);
    }
    // Fixture drops this poisoned document through registry ownership without locking it.
}

struct ReentrantContext {
    core: usize,
    doc: usize,
    subscription: usize,
    read: AtomicU32,
    mutate: AtomicU32,
    unsubscribe: AtomicU32,
    calls: AtomicU32,
}
unsafe extern "C" fn reentrant_callback(_event: *const PaintEvent, user: *mut c_void) {
    // SAFETY: test keeps the boxed context alive until explicit unsubscribe completes.
    let context = unsafe { &*user.cast::<ReentrantContext>() };
    let mut info = PaintDocumentInfo {
        struct_size: 64,
        ..Default::default()
    };
    let core = context.core as *mut PaintCore;
    let doc = context.doc as *mut PaintDocument;
    unsafe {
        context.read.store(
            paint_document_info(core, doc, &mut info) as u32,
            Ordering::SeqCst,
        );
        context
            .mutate
            .store(paint_core_undo(core, doc) as u32, Ordering::SeqCst);
        let mut subscription = context.subscription as *mut PaintSubscription;
        context.unsubscribe.store(
            paint_core_unsubscribe(core, &mut subscription) as u32,
            Ordering::SeqCst,
        );
    }
    context.calls.fetch_add(1, Ordering::SeqCst);
}

#[test]
fn callback_queries_run_outside_locks_and_mutating_reentry_is_busy() {
    let f = Fixture::new();
    let mut context = Box::new(ReentrantContext {
        core: f.core as usize,
        doc: f.doc as usize,
        subscription: 0,
        read: AtomicU32::new(99),
        mutate: AtomicU32::new(99),
        unsubscribe: AtomicU32::new(99),
        calls: AtomicU32::new(0),
    });
    unsafe {
        let mut sub = ptr::null_mut();
        assert_eq!(
            paint_core_subscribe(
                f.core,
                Some(reentrant_callback),
                (&mut *context as *mut ReentrantContext).cast(),
                &mut sub
            ),
            PAINT_OK
        );
        context.subscription = sub as usize;
        f.dot();
        assert_eq!(context.read.load(Ordering::SeqCst), PAINT_OK as u32);
        assert_eq!(context.mutate.load(Ordering::SeqCst), PAINT_BUSY as u32);
        assert_eq!(
            context.unsubscribe.load(Ordering::SeqCst),
            PAINT_BUSY as u32
        );
        assert!(context.calls.load(Ordering::SeqCst) > 0);
        assert_eq!(paint_core_unsubscribe(f.core, &mut sub), PAINT_OK);
        let calls = context.calls.load(Ordering::SeqCst);
        f.dot();
        assert_eq!(context.calls.load(Ordering::SeqCst), calls);
        assert_eq!(paint_core_unsubscribe(f.core, &mut sub), PAINT_OK);
    }
}

struct BlockingContext {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}
unsafe extern "C" fn blocking_callback(_event: *const PaintEvent, user: *mut c_void) {
    let context = unsafe { &*user.cast::<BlockingContext>() };
    let _ = context.entered.send(());
    if let Ok(receiver) = context.release.lock() {
        let _ = receiver.recv_timeout(Duration::from_secs(5));
    }
}

#[test]
fn unsubscribe_drains_inflight_callback_before_user_data_can_be_freed() {
    let f = Fixture::new();
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let mut context = Box::new(BlockingContext {
        entered: entered_tx,
        release: Mutex::new(release_rx),
    });
    let mut sub = ptr::null_mut();
    unsafe {
        assert_eq!(
            paint_core_subscribe(
                f.core,
                Some(blocking_callback),
                (&mut *context as *mut BlockingContext).cast(),
                &mut sub
            ),
            PAINT_OK
        );
    }
    let core = f.core as usize;
    let doc = f.doc as usize;
    let subscription = sub as usize;
    let (done_tx, done_rx) = mpsc::channel();
    thread::scope(|scope| {
        let draw = scope.spawn(move || unsafe {
            paint_core_begin_stroke(
                core as *mut PaintCore,
                doc as *mut PaintDocument,
                &stroke_desc(),
                &sample(),
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let unsubscribe = scope.spawn(move || unsafe {
            let mut sub = subscription as *mut PaintSubscription;
            let status = paint_core_unsubscribe(core as *mut PaintCore, &mut sub);
            done_tx.send((status, sub.is_null())).unwrap();
        });
        assert!(matches!(
            done_rx.recv_timeout(Duration::from_millis(100)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        release_tx.send(()).unwrap();
        assert_eq!(draw.join().unwrap(), PAINT_OK);
        assert_eq!(
            done_rx.recv_timeout(Duration::from_secs(3)).unwrap(),
            (PAINT_OK, true)
        );
        unsubscribe.join().unwrap();
    });
    unsafe {
        assert_eq!(paint_core_cancel_stroke(f.core, f.doc), PAINT_OK);
    }
}

#[test]
fn subscription_ownership_blocks_core_destruction_and_rejects_cross_core() {
    let mut f = Fixture::new();
    let other = Fixture::new();
    unsafe extern "C" fn callback(_: *const PaintEvent, _: *mut c_void) {}
    unsafe {
        let mut sub = ptr::null_mut();
        assert_eq!(
            paint_core_subscribe(f.core, None, ptr::null_mut(), &mut sub),
            PAINT_INVALID_ARGUMENT
        );
        assert!(sub.is_null());
        assert_eq!(
            paint_core_subscribe(f.core, Some(callback), ptr::null_mut(), &mut sub),
            PAINT_OK
        );
        assert_eq!(paint_document_destroy(f.core, &mut f.doc), PAINT_OK);
        assert_eq!(paint_core_destroy(&mut f.core), PAINT_BUSY);
        assert_eq!(
            paint_core_unsubscribe(other.core, &mut sub),
            PAINT_INVALID_HANDLE
        );
        assert!(!sub.is_null());
        let old = sub;
        assert_eq!(paint_core_unsubscribe(f.core, &mut sub), PAINT_OK);
        let mut stale = old;
        assert_eq!(
            paint_core_unsubscribe(f.core, &mut stale),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(paint_core_destroy(&mut f.core), PAINT_OK);
    }
}
