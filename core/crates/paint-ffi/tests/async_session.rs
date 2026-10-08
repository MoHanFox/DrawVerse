use paint_ffi::*;
use std::{
    mem::{offset_of, size_of},
    ptr, thread,
    time::{Duration, Instant},
};
macro_rules! dto {
    (PaintTile) => {
        PaintTile {
            struct_size: size_of::<PaintTile>() as u32,
            format: 0,
            data: ptr::null_mut(),
            capacity: 0,
            stride: 0,
            required: 0,
            width: 0,
            height: 0,
            revision: 0,
        }
    };
    ($t:ident) => {
        $t {
            struct_size: size_of::<$t>() as u32,
            ..Default::default()
        }
    };
}
struct Harness {
    core: *mut PaintCore,
    session: *mut PaintSession,
}

#[test]
fn selection_abi_publication_validation_history_and_stroke_gate() {
    assert_eq!(size_of::<PaintSelectionEdit>(), 64);
    assert_eq!(offset_of!(PaintSelectionEdit, x), 32);
    assert_eq!(size_of::<PaintSelectionInfo>(), 16);
    assert_eq!(size_of::<PaintSelectionStep>(), 48);
    let h = Harness::new(64, 64);
    let mut request = PaintSelectionEdit {
        width: 32.,
        height: 64.,
        antialias: 1,
        ..dto!(PaintSelectionEdit)
    };
    let mut seq = 0;
    unsafe {
        for bad in [
            PaintSelectionEdit {
                width: -1.,
                ..request
            },
            PaintSelectionEdit {
                shape: 3,
                ..request
            },
            PaintSelectionEdit {
                reserved: [1, 0, 0],
                ..request
            },
            PaintSelectionEdit {
                x: f64::NAN,
                ..request
            },
        ] {
            seq = 999;
            assert_eq!(
                paint_session_edit_selection(h.core, h.session, &bad, &mut seq),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(seq, 0);
        }
        assert_eq!(
            paint_session_edit_selection(h.core, h.session, &request, &mut seq),
            PAINT_OK
        );
        let state = h.wait(seq);
        assert_eq!(state.undo_depth, 1);
        let mut selection = dto!(PaintSelectionInfo);
        assert_eq!(
            paint_session_selection_info(h.core, h.session, state.publication, &mut selection),
            PAINT_OK
        );
        assert_eq!((selection.enabled, selection.step_count), (1, 1));
        let mut step = dto!(PaintSelectionStep);
        assert_eq!(
            paint_session_selection_step(h.core, h.session, state.publication, 0, &mut step),
            PAINT_OK
        );
        assert_eq!(step.width, 32.);
        assert_eq!(
            paint_session_selection_info(
                h.core,
                h.session,
                state.publication + 999,
                &mut selection
            ),
            PAINT_BUSY
        );
        assert_eq!(selection.enabled, 0);
        assert_eq!(
            paint_session_selection_step(h.core, h.session, state.publication, 99, &mut step),
            PAINT_NOT_FOUND
        );
        let stroke = h.wait(h.stroke(32., 32.));
        assert_eq!(stroke.undo_depth, 2);
        let view = PaintViewport {
            enabled: 1,
            width: 64.,
            height: 64.,
            pixel_width: 64,
            pixel_height: 64,
            document_generation: stroke.document_generation,
            ..dto!(PaintViewport)
        };
        let mut request_id = 0;
        assert_eq!(
            paint_session_set_viewport(h.core, h.session, &view, &mut request_id),
            PAINT_OK
        );
        let pixels = h.pixels(0, h.frame(0, stroke.revision));
        assert!(pixels[(32 * 64 + 28) * 4 + 3] > 0);
        assert_eq!(pixels[(32 * 64 + 36) * 4 + 3], 0);
        h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        let state = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(
            paint_session_selection_info(h.core, h.session, state.publication, &mut selection),
            PAINT_OK
        );
        assert_eq!(selection.enabled, 0);
        h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_REDO,
            ..dto!(PaintCommand)
        }));
        let begin = PaintCommand {
            kind: PAINT_COMMAND_BEGIN_STROKE,
            stroke: PaintStrokeDesc {
                mode: PAINT_MODE_PAINT,
                radius: 8.,
                opacity: 1.,
                spacing: 0.15,
                linear_rgba: [1., 0., 0., 1.],
                ..dto!(PaintStrokeDesc)
            },
            point: PaintPoint {
                x: 16.,
                y: 16.,
                pressure: 1.,
                tool: PAINT_TOOL_MOUSE,
                ..dto!(PaintPoint)
            },
            ..dto!(PaintCommand)
        };
        h.wait(h.submit(begin));
        request = PaintSelectionEdit {
            action: PAINT_SELECTION_CLEAR,
            ..dto!(PaintSelectionEdit)
        };
        assert_eq!(
            paint_session_edit_selection(h.core, h.session, &request, &mut seq),
            PAINT_OK
        );
        let state = h.wait(seq);
        assert_eq!(state.last_error_status, PAINT_BUSY);
        assert_eq!(state.stroke_active, 1);
        h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_END_STROKE,
            ..dto!(PaintCommand)
        }));
        assert_eq!(h.info().undo_depth, 2);
        let state = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_NEW_DOCUMENT,
            width: 64,
            height: 64,
            ..dto!(PaintCommand)
        }));
        assert_eq!(
            paint_session_selection_info(h.core, h.session, state.publication, &mut selection),
            PAINT_OK
        );
        assert_eq!((selection.enabled, selection.step_count), (0, 0));
    }
}

#[test]
fn clipping_abi_layout_validation_fifo_publication_and_undo() {
    assert_eq!(size_of::<PaintLayerClipping>(), 16);
    assert_eq!(offset_of!(PaintLayerClipping, base_layer_id), 8);
    let h = Harness::new(64, 64);
    h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_ADD_LAYER,
        text: b"top".as_ptr(),
        text_length: 3,
        ..dto!(PaintCommand)
    }));
    let id = h.info().active_layer_id;
    unsafe {
        let mut opts = PaintLayerClipping {
            enabled: 1,
            ..dto!(PaintLayerClipping)
        };
        let mut sequence = 0;
        assert_eq!(
            paint_session_set_layer_clipping(h.core, h.session, id, &opts, &mut sequence),
            PAINT_OK
        );
        opts.enabled = 0;
        let info = h.wait(sequence);
        assert_eq!(
            paint_session_layer_clipping(h.core, h.session, info.publication, id, &mut opts),
            PAINT_OK
        );
        assert_eq!((opts.enabled, opts.base_layer_id), (1, 1));
        assert_eq!(
            paint_session_layer_clipping(h.core, h.session, info.publication + 10, id, &mut opts),
            PAINT_BUSY
        );
        for (enabled, base) in [(2, 0), (1, 1)] {
            opts.enabled = enabled;
            opts.base_layer_id = base;
            sequence = 999;
            assert_eq!(
                paint_session_set_layer_clipping(h.core, h.session, id, &opts, &mut sequence),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(sequence, 0);
        }
        let info = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(
            paint_session_layer_clipping(h.core, h.session, info.publication, id, &mut opts),
            PAINT_OK
        );
        assert_eq!(opts.enabled, 0);
        opts.enabled = 1;
        opts.base_layer_id = 0;
        assert_eq!(
            paint_session_set_layer_clipping(h.core, h.session, 1, &opts, &mut sequence),
            PAINT_OK
        );
        assert_eq!(h.wait(sequence).last_error_status, PAINT_INVALID_ARGUMENT);
    }
}

#[test]
fn mask_and_white_canvas_commands_and_drop_abi_are_versioned_and_atomic() {
    assert_eq!(size_of::<PaintLayerDrop>(), 24);
    assert_eq!(offset_of!(PaintLayerDrop, target_id), 16);
    let h = Harness::new(64, 64);
    let info = h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_NEW_WHITE_DOCUMENT,
        width: 64,
        height: 64,
        ..dto!(PaintCommand)
    }));
    assert_eq!(info.undo_depth, 0);
    let info = h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_ADD_MASK,
        layer_id: 1,
        ..dto!(PaintCommand)
    }));
    assert_eq!(info.layer_count, 2);
    let mask = info.active_layer_id;
    unsafe {
        let mut node = dto!(PaintLayerHierarchy);
        assert_eq!(
            paint_session_layer_hierarchy(h.core, h.session, info.publication, mask, &mut node),
            PAINT_OK
        );
        assert_eq!(
            (node.kind, node.parent_id, node.depth),
            (PAINT_LAYER_MASK, 1, 1)
        );
        let mut sequence = 123;
        let mut request = PaintLayerDrop {
            layer_id: 1,
            target_id: 0,
            placement: 9,
            ..dto!(PaintLayerDrop)
        };
        assert_eq!(
            paint_session_drop_layer(h.core, h.session, &request, &mut sequence),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        request.placement = 0;
        request.layer_id = mask;
        assert_eq!(
            paint_session_drop_layer(h.core, h.session, &request, &mut sequence),
            PAINT_OK
        );
        let failure = h.wait(sequence);
        assert_eq!(failure.last_error_status, PAINT_INVALID_ARGUMENT);
        assert_eq!(failure.revision, info.revision);
        assert_eq!(failure.undo_depth, info.undo_depth);
    }
    let info = h.wait(h.stroke(32., 32.));
    assert_eq!(info.last_error_status, PAINT_INVALID_ARGUMENT); // Previous async error remains identifiable by sequence.
    assert_eq!(info.undo_depth, 2);
    let undone = h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_UNDO,
        ..dto!(PaintCommand)
    }));
    assert_eq!(undone.undo_depth, 1);
    let info = h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_NEW_DOCUMENT,
        width: 32,
        height: 32,
        ..dto!(PaintCommand)
    }));
    assert_eq!(info.layer_count, 1);
    assert_eq!(info.undo_depth, 0);
}

#[test]
fn group_abi_copies_names_exposes_hierarchy_and_serializes_undo_and_cycle_rejection() {
    assert_eq!(size_of::<PaintLayerHierarchy>(), 24);
    assert_eq!(size_of::<PaintGroupRequest>(), 40);
    assert_eq!(offset_of!(PaintGroupRequest, name), 24);
    let h = Harness::new(64, 64);
    h.wait(h.stroke(16.5, 16.5));
    let mut name = b"Owned group".to_vec();
    let mut request = PaintGroupRequest {
        struct_size: size_of::<PaintGroupRequest>() as u32,
        kind: PAINT_GROUP_WRAP,
        layer_id: 1,
        parent_id: 0,
        name: name.as_ptr(),
        name_length: name.len() as u64,
    };
    let mut sequence = 0;
    unsafe {
        assert_eq!(
            paint_session_group(h.core, h.session, &request, &mut sequence),
            PAINT_OK
        );
    }
    name.fill(b'x');
    let info = h.wait(sequence);
    assert_eq!(info.layer_count, 2);
    let group = info.active_layer_id;
    unsafe {
        let mut node = dto!(PaintLayerHierarchy);
        assert_eq!(
            paint_session_layer_hierarchy(h.core, h.session, info.publication, 1, &mut node),
            PAINT_OK
        );
        assert_eq!(
            (node.kind, node.parent_id, node.depth),
            (PAINT_LAYER_PIXEL, group, 1)
        );
        let mut output = [0; 32];
        let mut required = 0;
        assert_eq!(
            paint_session_layer_name(
                h.core,
                h.session,
                info.publication,
                group,
                output.as_mut_ptr(),
                32,
                &mut required
            ),
            PAINT_OK
        );
        assert_eq!(&output[..required as usize], b"Owned group");
        request.kind = PAINT_GROUP_REPARENT;
        request.layer_id = group;
        request.parent_id = group;
        request.name = ptr::null();
        request.name_length = 0;
        assert_eq!(
            paint_session_group(h.core, h.session, &request, &mut sequence),
            PAINT_OK
        );
        let failed = h.wait(sequence);
        assert_eq!(failed.last_error_sequence, sequence);
        assert_eq!(failed.last_error_status, PAINT_INVALID_ARGUMENT);
        assert_eq!(failed.revision, info.revision);
        request.layer_id = 1;
        request.parent_id = 0;
        assert_eq!(
            paint_session_group(h.core, h.session, &request, &mut sequence),
            PAINT_OK
        );
        let moved = h.wait(sequence);
        assert_eq!(
            paint_session_layer_hierarchy(h.core, h.session, moved.publication, 1, &mut node),
            PAINT_OK
        );
        assert_eq!(node.parent_id, 0);
        assert_eq!(
            paint_session_layer_hierarchy(h.core, h.session, info.publication, 1, &mut node),
            PAINT_BUSY
        );
        assert_eq!(node.parent_id, 0);
    }
    let undone = h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_UNDO,
        ..dto!(PaintCommand)
    }));
    unsafe {
        let mut node = dto!(PaintLayerHierarchy);
        assert_eq!(
            paint_session_layer_hierarchy(h.core, h.session, undone.publication, 1, &mut node),
            PAINT_OK
        );
        assert_eq!(node.parent_id, group);
        request.kind = PAINT_GROUP_UNGROUP;
        request.layer_id = group;
        assert_eq!(
            paint_session_group(h.core, h.session, &request, &mut sequence),
            PAINT_OK
        );
    }
    assert_eq!(h.wait(sequence).layer_count, 1);
    unsafe {
        request.kind = 999;
        sequence = 99;
        assert_eq!(
            paint_session_group(h.core, h.session, &request, &mut sequence),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
    }
}
impl Harness {
    fn file(&self, path: &std::path::Path, kind: u32, format: u32) -> u64 {
        let bytes = path.to_str().unwrap().as_bytes();
        let request = PaintFileRequest {
            struct_size: size_of::<PaintFileRequest>() as u32,
            kind,
            format,
            quality: 95,
            path: bytes.as_ptr(),
            path_length: bytes.len() as u64,
            document_generation: self.info().document_generation,
            linear_background: [1.; 4],
            reserved: [0; 2],
        };
        let mut id = 0;
        unsafe {
            assert_eq!(
                paint_session_file_submit(self.core, self.session, &request, &mut id),
                PAINT_OK
            );
        };
        assert!(id > 0);
        id
    }
    fn file_wait(&self, id: u64) -> PaintFileJobInfo {
        wait(|| unsafe {
            let mut info = dto!(PaintFileJobInfo);
            assert_eq!(
                paint_session_file_info(self.core, self.session, id, &mut info),
                PAINT_OK
            );
            (info.state >= PAINT_FILE_SUCCEEDED).then_some(info)
        })
    }
    fn new(w: u32, h: u32) -> Self {
        unsafe {
            let mut core = ptr::null_mut();
            assert_eq!(paint_core_create(&mut core), PAINT_OK);
            let desc = PaintDocumentDesc {
                width: w,
                height: h,
                working_space: PAINT_WORKING_LINEAR_SRGB,
                pixel_format: PAINT_STORAGE_RGBA32F_PREMULTIPLIED,
                ..dto!(PaintDocumentDesc)
            };
            let mut session = ptr::null_mut();
            assert_eq!(paint_session_create(core, &desc, &mut session), PAINT_OK);
            Self { core, session }
        }
    }
    fn info(&self) -> PaintSessionInfo {
        unsafe {
            let mut info = dto!(PaintSessionInfo);
            assert_eq!(
                paint_session_info(self.core, self.session, &mut info),
                PAINT_OK
            );
            info
        }
    }
    fn submit(&self, command: PaintCommand) -> u64 {
        unsafe {
            let mut id = 0;
            assert_eq!(
                paint_session_submit(self.core, self.session, &command, &mut id),
                PAINT_OK
            );
            assert!(id > 0);
            id
        }
    }
    fn wait(&self, id: u64) -> PaintSessionInfo {
        wait(|| {
            let info = self.info();
            (info.completed_sequence >= id).then_some(info)
        })
    }
    fn stroke(&self, x: f64, y: f64) -> u64 {
        let desc = PaintStrokeDesc {
            mode: PAINT_MODE_PAINT,
            radius: 8.,
            opacity: 1.,
            spacing: 0.15,
            linear_rgba: [1., 0., 0., 1.],
            ..dto!(PaintStrokeDesc)
        };
        let point = PaintPoint {
            x,
            y,
            pressure: 1.,
            tool: PAINT_TOOL_MOUSE,
            ..dto!(PaintPoint)
        };
        self.submit(PaintCommand {
            kind: PAINT_COMMAND_BEGIN_STROKE,
            stroke: desc,
            point,
            ..dto!(PaintCommand)
        });
        self.submit(PaintCommand {
            kind: PAINT_COMMAND_END_STROKE,
            ..dto!(PaintCommand)
        })
    }
    fn frame(&self, slot: u32, min_revision: u64) -> PaintFrameInfo {
        wait(|| unsafe {
            let mut info = dto!(PaintFrameInfo);
            let status = paint_session_frame_info(self.core, self.session, slot, &mut info);
            assert!(status == PAINT_OK || status == PAINT_BUSY);
            (status == PAINT_OK && info.revision >= min_revision).then_some(info)
        })
    }
    fn pixels(&self, slot: u32, frame: PaintFrameInfo) -> Vec<u8> {
        unsafe {
            let mut bytes = vec![0; frame.required_bytes as usize];
            let mut tile = PaintTile {
                format: PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED,
                data: bytes.as_mut_ptr(),
                capacity: bytes.len() as u64,
                stride: u64::from(frame.pixel_width) * 4,
                ..dto!(PaintTile)
            };
            assert_eq!(
                paint_session_read_frame(
                    self.core,
                    self.session,
                    slot,
                    frame.request_id,
                    frame.frame_id,
                    &mut tile
                ),
                PAINT_OK
            );
            bytes
        }
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        unsafe {
            assert_eq!(
                paint_session_destroy(self.core, &mut self.session),
                PAINT_OK
            );
            assert_eq!(paint_core_destroy(&mut self.core), PAINT_OK);
        }
    }
}

#[test]
fn additive_layer_abi_validates_copied_dtos_enforces_locks_and_reads_background_preview() {
    assert_eq!(size_of::<PaintLayerAppearance>(), 32);
    assert_eq!(offset_of!(PaintLayerAppearance, fill), 16);
    assert_eq!(offset_of!(PaintLayerAppearance, dissolve_seed), 28);
    let h = Harness::new(64, 64);
    let stroke = h.stroke(16.5, 16.5);
    h.wait(stroke);
    unsafe {
        let mut a = dto!(PaintLayerAppearance);
        let initial = h.info();
        assert_eq!(
            paint_session_layer_appearance(h.core, h.session, initial.publication, 1, &mut a),
            PAINT_OK
        );
        a.locks = PAINT_LOCK_ALL;
        let mut sequence = 0;
        assert_eq!(
            paint_session_set_layer_appearance(h.core, h.session, 1, &a, &mut sequence),
            PAINT_OK
        );
        let locked = h.wait(sequence);
        assert_eq!(
            paint_session_layer_appearance(
                h.core,
                h.session,
                initial.publication,
                1,
                &mut dto!(PaintLayerAppearance)
            ),
            PAINT_BUSY
        );
        assert_eq!(
            paint_session_move_layer(h.core, h.session, 1, 2, 0, &mut sequence),
            PAINT_OK
        );
        let denied = h.wait(sequence);
        assert_eq!(denied.last_error_status, PAINT_LAYER_LOCKED);
        assert_eq!(denied.revision, locked.revision);
        a.locks = 0;
        assert_eq!(
            paint_session_set_layer_appearance(h.core, h.session, 1, &a, &mut sequence),
            PAINT_OK
        );
        h.wait(sequence);
        a.fill = 0.5;
        a.blend_mode = 3;
        assert_eq!(
            paint_session_set_layer_appearance(h.core, h.session, 1, &a, &mut sequence),
            PAINT_OK
        );
        a.fill = 0.;
        assert_eq!(a.fill, 0.); // caller storage can change immediately after submit
        let configured = h.wait(sequence);
        let mut copied = dto!(PaintLayerAppearance);
        assert_eq!(
            paint_session_layer_appearance(
                h.core,
                h.session,
                configured.publication,
                1,
                &mut copied
            ),
            PAINT_OK
        );
        assert_eq!(copied.fill, 0.5);
        for bad in [
            PaintLayerAppearance {
                blend_mode: 27,
                ..copied
            },
            PaintLayerAppearance { locks: 8, ..copied },
            PaintLayerAppearance {
                fill: f32::NAN,
                ..copied
            },
            PaintLayerAppearance {
                reserved: 1,
                ..copied
            },
        ] {
            sequence = 999;
            assert_eq!(
                paint_session_set_layer_appearance(h.core, h.session, 1, &bad, &mut sequence),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(sequence, 0);
        }
        let mut view = PaintViewport {
            view_id: 2,
            enabled: 1,
            document_generation: configured.document_generation,
            width: 64.,
            height: 64.,
            pixel_width: 64,
            pixel_height: 64,
            ..dto!(PaintViewport)
        };
        let mut request = 0;
        assert_eq!(
            paint_session_set_layer_preview(h.core, h.session, 1, &view, &mut request),
            PAINT_OK
        );
        let frame = h.frame(2, configured.revision);
        assert_eq!(frame.request_id, request);
        let pixels = h.pixels(2, frame);
        assert_eq!(
            &pixels[(16 * 64 + 16) * 4..(16 * 64 + 16) * 4 + 4],
            &[255, 0, 0, 255]
        ); // raw preview ignores fill/blend/lock
        view.pixel_width = 97;
        assert_eq!(
            paint_session_set_layer_preview(h.core, h.session, 1, &view, &mut request),
            PAINT_INVALID_ARGUMENT
        );
        view.pixel_width = 64;
        view.view_id = 0;
        assert_eq!(
            paint_session_set_layer_preview(h.core, h.session, 1, &view, &mut request),
            PAINT_INVALID_ARGUMENT
        );
        view.view_id = 2;
        view.enabled = 0;
        assert_eq!(
            paint_session_set_viewport(h.core, h.session, &view, &mut request),
            PAINT_OK
        );
        assert_eq!(
            paint_session_frame_info(h.core, h.session, 2, &mut dto!(PaintFrameInfo)),
            PAINT_BUSY
        );
    }
}
fn wait<T>(mut predicate: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(value) = predicate() {
            return value;
        }
        assert!(Instant::now() < deadline, "async operation timed out");
        thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn async_dto_layout_is_fixed_and_old_prefixes_still_work() {
    assert_eq!(size_of::<PaintFileRequest>(), 64);
    assert_eq!(size_of::<PaintFileJobInfo>(), 64);
    assert_eq!(offset_of!(PaintFileRequest, path), 16);
    assert_eq!(size_of::<PaintCommand>(), 176);
    assert_eq!(size_of::<PaintSessionInfo>(), 104);
    assert_eq!(size_of::<PaintViewport>(), 64);
    assert_eq!(size_of::<PaintFrameInfo>(), 96);
    assert_eq!(offset_of!(PaintCommand, point), 64);
    assert_eq!(offset_of!(PaintViewport, x), 16);
    let h = Harness::new(64, 64);
    assert_eq!(h.info().document_generation, 1);
    unsafe {
        let mut old = dto!(PaintCapabilities);
        assert_eq!(paint_core_capabilities(h.core, &mut old), PAINT_OK);
        assert_eq!(old.features & 15, 15);
    }
}
#[test]
fn serial_stroke_history_and_barrier_frames() {
    let h = Harness::new(128, 128);
    let initial = h.frame(0, 0);
    assert!(h.pixels(0, initial).iter().all(|v| *v == 0));
    let last = h.stroke(64., 64.);
    let info = h.wait(last);
    assert_eq!(info.undo_depth, 1);
    let painted = h.frame(0, info.revision);
    let pixels = h.pixels(0, painted);
    assert_eq!(
        &pixels[(64 * 128 + 64) * 4..(64 * 128 + 64) * 4 + 4],
        &[255, 0, 0, 255]
    );
    let sequence = h.submit(PaintCommand {
        kind: PAINT_COMMAND_UNDO,
        ..dto!(PaintCommand)
    });
    let info = h.wait(sequence);
    assert_eq!(info.redo_depth, 1);
    let mut tile = PaintTile {
        format: PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED,
        stride: 512,
        ..dto!(PaintTile)
    };
    unsafe {
        assert_eq!(
            paint_session_read_frame(
                h.core,
                h.session,
                0,
                painted.request_id,
                painted.frame_id,
                &mut tile
            ),
            PAINT_BUSY
        );
    }
    let erased = h.frame(0, info.revision);
    assert!(h.pixels(0, erased).iter().all(|v| *v == 0));
    let last = h.submit(PaintCommand {
        kind: PAINT_COMMAND_REDO,
        ..dto!(PaintCommand)
    });
    let info = h.wait(last);
    let repainted = h.frame(0, info.revision);
    assert_eq!(pixels, h.pixels(0, repainted));
}
#[test]
fn viewport_replacement_generation_and_disabled_slots_reject_old_frames() {
    let h = Harness::new(1_000_000, 1_000_000);
    let generation = h.info().document_generation;
    let last = h.stroke(900_000., 900_000.);
    let info = h.wait(last);
    let view = PaintViewport {
        enabled: 1,
        document_generation: generation,
        x: 899_968.,
        y: 899_968.,
        width: 64.,
        height: 64.,
        pixel_width: 64,
        pixel_height: 64,
        ..dto!(PaintViewport)
    };
    let mut request = 0;
    unsafe {
        assert_eq!(
            paint_session_set_viewport(h.core, h.session, &view, &mut request),
            PAINT_OK
        );
    }
    let frame = h.frame(0, info.revision);
    assert_eq!(frame.request_id, request);
    assert_eq!(frame.required_bytes, 64 * 64 * 4);
    let bytes = h.pixels(0, frame);
    assert_eq!(bytes[(32 * 64 + 32) * 4 + 3], 255);
    let disabled = PaintViewport { enabled: 0, ..view };
    unsafe {
        assert_eq!(
            paint_session_set_viewport(h.core, h.session, &disabled, &mut request),
            PAINT_OK
        );
    }
    let mut out = dto!(PaintFrameInfo);
    unsafe {
        assert_eq!(
            paint_session_frame_info(h.core, h.session, 0, &mut out),
            PAINT_BUSY
        );
    }
    let last = h.submit(PaintCommand {
        kind: PAINT_COMMAND_NEW_DOCUMENT,
        width: 64,
        height: 64,
        ..dto!(PaintCommand)
    });
    let next = h.wait(last);
    assert!(next.document_generation > generation);
    unsafe {
        assert_eq!(
            paint_session_set_viewport(h.core, h.session, &view, &mut request),
            PAINT_BUSY
        );
    }
    let frame = h.frame(0, 0);
    assert_eq!(frame.document_generation, next.document_generation);
    assert!(h.pixels(0, frame).iter().all(|v| *v == 0));
}
#[test]
fn layer_enumeration_is_atomic_utf8_and_errors_are_async() {
    let h = Harness::new(64, 64);
    let before = h.info();
    let name = "颜料 🖌".as_bytes();
    let last = h.submit(PaintCommand {
        kind: PAINT_COMMAND_ADD_LAYER,
        text: name.as_ptr(),
        text_length: name.len() as u64,
        ..dto!(PaintCommand)
    });
    let info = h.wait(last);
    assert_eq!(info.layer_count, 2);
    let mut layer = dto!(PaintLayerInfo);
    unsafe {
        assert_eq!(
            paint_session_layer_info(h.core, h.session, before.publication, 1, &mut layer),
            PAINT_BUSY
        );
    }
    unsafe {
        assert_eq!(
            paint_session_layer_info(h.core, h.session, info.publication, 1, &mut layer),
            PAINT_OK
        );
    }
    assert_eq!(layer.layer_id, info.active_layer_id);
    let mut bytes = vec![0; name.len()];
    let mut required = 0;
    unsafe {
        assert_eq!(
            paint_session_layer_name(
                h.core,
                h.session,
                info.publication,
                layer.layer_id,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
    }
    assert_eq!(bytes, name);
    let last = h.submit(PaintCommand {
        kind: PAINT_COMMAND_REMOVE_LAYER,
        layer_id: u64::MAX,
        ..dto!(PaintCommand)
    });
    let info = h.wait(last);
    assert_eq!(info.last_error_sequence, last);
    assert_eq!(info.last_error_status, PAINT_NOT_FOUND);
    assert_eq!(info.layer_count, 2);
    unsafe {
        assert_eq!(
            paint_session_error_message(h.core, h.session, last, ptr::null_mut(), 0, &mut required),
            PAINT_BUFFER_TOO_SMALL
        );
    }
    let mut message = vec![0; required as usize];
    unsafe {
        assert_eq!(
            paint_session_error_message(
                h.core,
                h.session,
                last,
                message.as_mut_ptr(),
                required,
                &mut required
            ),
            PAINT_OK
        );
    }
    assert!(!String::from_utf8(message).unwrap().is_empty());
}
#[test]
fn frame_copy_capacity_stride_padding_format_and_stale_ids_are_atomic() {
    let h = Harness::new(2, 2);
    let frame = h.frame(0, 0);
    let mut bytes = [0xcc; 24];
    let mut tile = PaintTile {
        format: PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED,
        data: bytes.as_mut_ptr(),
        capacity: 19,
        stride: 12,
        ..dto!(PaintTile)
    };
    unsafe {
        assert_eq!(
            paint_session_read_frame(
                h.core,
                h.session,
                0,
                frame.request_id,
                frame.frame_id,
                &mut tile
            ),
            PAINT_BUFFER_TOO_SMALL
        );
    }
    assert_eq!(tile.required, 20);
    assert_eq!(bytes, [0xcc; 24]);
    tile.capacity = 24;
    unsafe {
        assert_eq!(
            paint_session_read_frame(
                h.core,
                h.session,
                0,
                frame.request_id,
                frame.frame_id,
                &mut tile
            ),
            PAINT_OK
        );
    }
    assert_eq!(&bytes[0..8], &[0; 8]);
    assert_eq!(&bytes[12..20], &[0; 8]);
    assert_eq!(&bytes[8..12], &[0xcc; 4]);
    assert_eq!(&bytes[20..24], &[0xcc; 4]);
    for (stride, format, status) in [
        (
            7,
            PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED,
            PAINT_INVALID_ARGUMENT,
        ),
        (
            u64::MAX,
            PAINT_TILE_RGBA8_SRGB_PREMULTIPLIED,
            PAINT_INVALID_ARGUMENT,
        ),
        (
            12,
            PAINT_TILE_RGBA32F_LINEAR_PREMULTIPLIED,
            PAINT_UNSUPPORTED,
        ),
    ] {
        bytes.fill(0xcc);
        tile.stride = stride;
        tile.format = format;
        unsafe {
            assert_eq!(
                paint_session_read_frame(
                    h.core,
                    h.session,
                    0,
                    frame.request_id,
                    frame.frame_id,
                    &mut tile
                ),
                status
            );
        }
        assert_eq!(bytes, [0xcc; 24]);
    }
}
#[test]
fn overload_rolls_back_and_session_lifetimes_are_owned() {
    let mut h = Harness::new(128, 128);
    let begin = PaintCommand {
        kind: PAINT_COMMAND_BEGIN_STROKE,
        stroke: PaintStrokeDesc {
            mode: PAINT_MODE_PAINT,
            radius: 8.,
            opacity: 1.,
            spacing: 0.15,
            linear_rgba: [1., 0., 0., 1.],
            ..dto!(PaintStrokeDesc)
        },
        point: PaintPoint {
            x: 32.,
            y: 64.,
            pressure: 1.,
            tool: PAINT_TOOL_MOUSE,
            ..dto!(PaintPoint)
        },
        ..dto!(PaintCommand)
    };
    h.submit(begin);
    let mut overflow = false;
    for i in 0..30000 {
        let command = PaintCommand {
            kind: PAINT_COMMAND_STROKE_TO,
            point: PaintPoint {
                x: 32. + f64::from(i % 2) * 32.,
                y: 64.,
                pressure: 1.,
                tool: PAINT_TOOL_MOUSE,
                ..dto!(PaintPoint)
            },
            ..dto!(PaintCommand)
        };
        let mut id = 0;
        let status = unsafe { paint_session_submit(h.core, h.session, &command, &mut id) };
        if status == PAINT_LIMIT_EXCEEDED {
            overflow = true;
            assert_eq!(id, 0);
            break;
        }
        assert_eq!(status, PAINT_OK);
    }
    assert!(overflow);
    let info = wait(|| {
        let info = h.info();
        (info.last_error_status == PAINT_LIMIT_EXCEEDED && info.stroke_active == 0).then_some(info)
    });
    assert_eq!(info.undo_depth, 0);
    let frame = h.frame(0, info.revision);
    assert!(h.pixels(0, frame).iter().all(|v| *v == 0));
    unsafe {
        let original = h.core;
        assert_eq!(paint_core_destroy(&mut h.core), PAINT_BUSY);
        assert_eq!(h.core, original);
        let mut other = ptr::null_mut();
        assert_eq!(paint_core_create(&mut other), PAINT_OK);
        let mut wrong = dto!(PaintSessionInfo);
        assert_eq!(
            paint_session_info(other, h.session, &mut wrong),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(paint_core_destroy(&mut other), PAINT_OK);
        let stale = h.session;
        assert_eq!(paint_session_destroy(h.core, &mut h.session), PAINT_OK);
        assert!(h.session.is_null());
        assert_eq!(
            paint_session_info(h.core, stale, &mut wrong),
            PAINT_INVALID_HANDLE
        );
    }
}
#[test]
fn invalid_commands_do_not_enqueue_or_borrow_text() {
    let h = Harness::new(64, 64);
    let mut id = 99;
    let invalid = PaintCommand {
        kind: PAINT_COMMAND_STROKE_TO,
        point: PaintPoint {
            x: f64::NAN,
            pressure: 1.,
            tool: PAINT_TOOL_MOUSE,
            ..dto!(PaintPoint)
        },
        ..dto!(PaintCommand)
    };
    unsafe {
        assert_eq!(
            paint_session_submit(h.core, h.session, &invalid, &mut id),
            PAINT_INVALID_ARGUMENT
        );
    }
    assert_eq!(id, 0);
    let bad = PaintCommand {
        kind: PAINT_COMMAND_ADD_LAYER,
        text: ptr::null(),
        text_length: 10,
        ..dto!(PaintCommand)
    };
    unsafe {
        assert_eq!(
            paint_session_submit(h.core, h.session, &bad, &mut id),
            PAINT_INVALID_ARGUMENT
        );
    }
    let mut name = b"copied".to_vec();
    let last = h.submit(PaintCommand {
        kind: PAINT_COMMAND_ADD_LAYER,
        text: name.as_ptr(),
        text_length: name.len() as u64,
        ..dto!(PaintCommand)
    });
    name.fill(b'x');
    let info = h.wait(last);
    let mut bytes = [0; 6];
    let mut required = 0;
    unsafe {
        assert_eq!(
            paint_session_layer_name(
                h.core,
                h.session,
                info.publication,
                info.active_layer_id,
                bytes.as_mut_ptr(),
                6,
                &mut required
            ),
            PAINT_OK
        );
    }
    assert_eq!(&bytes, b"copied");
}
#[test]
fn busy_control_does_not_cancel_a_live_stroke() {
    let h = Harness::new(64, 64);
    let begin = PaintCommand {
        kind: PAINT_COMMAND_BEGIN_STROKE,
        stroke: PaintStrokeDesc {
            mode: PAINT_MODE_PAINT,
            radius: 8.,
            opacity: 1.,
            spacing: 0.15,
            linear_rgba: [1., 0., 0., 1.],
            ..dto!(PaintStrokeDesc)
        },
        point: PaintPoint {
            x: 32.,
            y: 32.,
            pressure: 1.,
            tool: PAINT_TOOL_MOUSE,
            ..dto!(PaintPoint)
        },
        ..dto!(PaintCommand)
    };
    h.submit(begin);
    let id = h.submit(PaintCommand {
        kind: PAINT_COMMAND_UNDO,
        ..dto!(PaintCommand)
    });
    let info = h.wait(id);
    assert_eq!(info.last_error_status, PAINT_BUSY);
    assert_eq!(info.stroke_active, 1);
    let id = h.submit(PaintCommand {
        kind: PAINT_COMMAND_END_STROKE,
        ..dto!(PaintCommand)
    });
    assert_eq!(h.wait(id).undo_depth, 1);
}

#[test]
fn async_file_formats_and_modified_semantics_round_trip() {
    let h = Harness::new(64, 64);
    let seq = h.stroke(32., 32.);
    h.wait(seq);
    let temp = tempfile::tempdir().unwrap();
    for (format, name) in [
        (PAINT_FILE_PNG, "绘画.png"),
        (PAINT_FILE_JPEG, "a.jpg"),
        (PAINT_FILE_WEBP, "a.webp"),
        (PAINT_FILE_OPENRASTER, "a.ora"),
    ] {
        let path = temp.path().join(name);
        let id = h.file(&path, PAINT_FILE_SAVE, format);
        let job = h.file_wait(id);
        assert_eq!(job.status, PAINT_OK);
        assert_eq!(job.state, PAINT_FILE_SUCCEEDED);
        assert!(path.is_file());
        if format != PAINT_FILE_OPENRASTER {
            assert_ne!(h.info().flags & PAINT_SESSION_MODIFIED, 0);
        }
    }
    wait(|| (h.info().flags & PAINT_SESSION_MODIFIED == 0).then_some(()));
    let old = h.info().document_generation;
    let id = h.file(&temp.path().join("a.ora"), PAINT_FILE_OPEN, PAINT_FILE_AUTO);
    assert_eq!(h.file_wait(id).status, PAINT_OK);
    let info = wait(|| {
        let i = h.info();
        (i.document_generation > old).then_some(i)
    });
    assert_eq!(info.undo_depth, 0);
    assert_eq!(info.flags & PAINT_SESSION_MODIFIED, 0);
    let frame = h.frame(0, 0);
    assert_eq!(
        &h.pixels(0, frame)[(32 * 64 + 32) * 4..(32 * 64 + 32) * 4 + 4],
        &[255, 0, 0, 255]
    );
}

#[test]
fn async_file_failure_preserves_document_and_reports_retained_error() {
    let h = Harness::new(64, 64);
    let seq = h.stroke(32., 32.);
    h.wait(seq);
    let before = h.info();
    let temp = tempfile::tempdir().unwrap();
    for (path, kind, format) in [
        (
            temp.path().join("missing.png"),
            PAINT_FILE_OPEN,
            PAINT_FILE_AUTO,
        ),
        (
            temp.path().join("missing-dir/a.png"),
            PAINT_FILE_SAVE,
            PAINT_FILE_PNG,
        ),
    ] {
        let id = h.file(&path, kind, format);
        let info = h.file_wait(id);
        assert_eq!(info.state, PAINT_FILE_FAILED);
        assert_eq!(info.status, PAINT_IO_ERROR);
        let after = h.info();
        assert_eq!(after.document_generation, before.document_generation);
        assert_eq!(after.revision, before.revision);
        assert_ne!(after.flags & PAINT_SESSION_MODIFIED, 0);
        unsafe {
            let mut needed = 0;
            assert_eq!(
                paint_session_file_message(h.core, h.session, id, ptr::null_mut(), 0, &mut needed),
                PAINT_BUFFER_TOO_SMALL
            );
            assert!(needed > 0);
            let mut bytes = vec![0; needed as usize];
            assert_eq!(
                paint_session_file_message(
                    h.core,
                    h.session,
                    id,
                    bytes.as_mut_ptr(),
                    needed,
                    &mut needed
                ),
                PAINT_OK
            );
            assert!(!bytes.contains(&0));
            assert_eq!(paint_session_file_cancel(h.core, h.session, id), PAINT_BUSY);
        }
    }
}

#[test]
fn async_file_input_validation_and_open_cannot_clobber_subsequent_edits() {
    let h = Harness::new(64, 64);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("a.ora");
    let id = h.file(&path, PAINT_FILE_SAVE, PAINT_FILE_OPENRASTER);
    assert_eq!(h.file_wait(id).status, PAINT_OK);
    let old = h.info().document_generation;
    let id = h.file(&path, PAINT_FILE_OPEN, PAINT_FILE_AUTO);
    // Both commands are FIFO; the edit runs after open starts, before the next result tick.
    let text = b"Retained edit";
    let seq = h.submit(PaintCommand {
        kind: PAINT_COMMAND_ADD_LAYER,
        text: text.as_ptr(),
        text_length: text.len() as u64,
        ..dto!(PaintCommand)
    });
    h.wait(seq);
    let job = h.file_wait(id);
    // If the result tick raced ahead of the edit, the edit belongs to the newly
    // opened document. In either ordering it must survive; stale result checks
    // are tested deterministically by file_job's service tests.
    assert!(job.status == PAINT_BUSY || job.status == PAINT_OK);
    let current = h.info().document_generation;
    assert_eq!(
        current,
        if job.status == PAINT_BUSY {
            old
        } else {
            old + 1
        }
    );
    assert_eq!(h.info().layer_count, 2);
    let bytes = path.to_str().unwrap().as_bytes();
    let mut request = PaintFileRequest {
        struct_size: size_of::<PaintFileRequest>() as u32,
        kind: PAINT_FILE_OPEN,
        format: PAINT_FILE_AUTO,
        quality: 95,
        path: bytes.as_ptr(),
        path_length: bytes.len() as u64,
        document_generation: old + 99,
        linear_background: [1.; 4],
        reserved: [0; 2],
    };
    unsafe {
        let mut job = 999;
        assert_eq!(
            paint_session_file_submit(h.core, h.session, &request, &mut job),
            PAINT_BUSY
        );
        assert_eq!(job, 0);
        request.document_generation = current;
        request.path = b"a\0b".as_ptr();
        request.path_length = 3;
        assert_eq!(
            paint_session_file_submit(h.core, h.session, &request, &mut job),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(job, 0);
        let mut info = dto!(PaintFileJobInfo);
        assert_eq!(
            paint_session_file_info(h.core, h.session, 0, &mut info),
            PAINT_NOT_FOUND
        );
    }
}
