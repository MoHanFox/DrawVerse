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

#[test]
fn advanced_brush_copies_validated_tip_and_preserves_atomic_history() {
    let h = Harness::new(40, 40);
    let mut request = PaintBrushStrokeDesc {
        brush: PaintStrokeDesc {
            mode: PAINT_MODE_PAINT,
            radius: 10.,
            opacity: 1.,
            spacing: 0.15,
            linear_rgba: [1., 0., 0., 1.],
            ..dto!(PaintStrokeDesc)
        },
        tip: PaintBrushTip {
            hardness: 1.,
            roundness: 0.2,
            flow: 1.,
            pressure_size: 1,
            pressure_flow: 1,
            ..dto!(PaintBrushTip)
        },
        strength: 1.,
        ..dto!(PaintBrushStrokeDesc)
    };
    let point = PaintPoint {
        x: 20.5,
        y: 20.5,
        pressure: 1.,
        tool: PAINT_TOOL_MOUSE,
        ..dto!(PaintPoint)
    };
    let mut seq = 99;
    unsafe {
        request.tip.roundness = 0.;
        assert_eq!(
            paint_session_begin_brush(h.core, h.session, &request, &point, &mut seq),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(seq, 0);
        assert_eq!(h.info().undo_depth, 0);
        request.tip.roundness = 0.2;
        assert_eq!(
            paint_session_begin_brush(h.core, h.session, &request, &point, &mut seq),
            PAINT_OK
        );
        request.tip.roundness = 1.;
        request.tip.flow = 0.;
        assert_eq!(request.tip.flow, 0.);
        assert_eq!(request.tip.roundness, 1.);
        let ended = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_END_STROKE,
            ..dto!(PaintCommand)
        }));
        assert_eq!(ended.last_error_status, PAINT_OK);
        assert_eq!(ended.undo_depth, 1);
        let frame = h.frame(0, ended.revision);
        let pixels = h.pixels(0, frame);
        let alpha = |x: usize, y: usize| pixels[(y * 40 + x) * 4 + 3];
        assert!(alpha(27, 20) > 200);
        assert_eq!(alpha(20, 27), 0);
        let undo = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(undo.undo_depth, 0);
        let redo = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_REDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(h.pixels(0, h.frame(0, redo.revision)), pixels);
    }
}

#[test]
fn preset_stroke_captures_definition_before_library_edit_remove_destroy() {
    let h = Harness::new(40, 40);
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let id = b"12345678-1234-4234-8234-123456789abc";
        assert_eq!(
            paint_brush_library_copy(
                library,
                id.as_ptr(),
                id.len() as u64,
                b"Ellipse".as_ptr(),
                7
            ),
            PAINT_OK
        );
        let mut settings = PaintBrushSettings {
            diameter: 24.,
            opacity: 1.,
            spacing: 0.075,
            tip: PaintBrushTip {
                hardness: 1.,
                roundness: 0.2,
                flow: 1.,
                pressure_size: 1,
                pressure_flow: 1,
                ..dto!(PaintBrushTip)
            },
            ..dto!(PaintBrushSettings)
        };
        assert_eq!(
            paint_brush_library_edit(library, id.as_ptr(), id.len() as u64, &settings),
            PAINT_OK
        );
        let mut tool = PaintBrushToolDesc {
            radius: 10.,
            strength: 1.,
            linear_rgba: [1., 0., 0., 1.],
            ..dto!(PaintBrushToolDesc)
        };
        let mut point = PaintPoint {
            x: 20.5,
            y: 20.5,
            pressure: 1.,
            tool: PAINT_TOOL_MOUSE,
            ..dto!(PaintPoint)
        };
        let mut sequence = 0;
        tool.radius = 0.;
        assert_eq!(
            paint_session_begin_preset(
                h.core,
                h.session,
                library,
                id.as_ptr(),
                id.len() as u64,
                &tool,
                &point,
                &mut sequence
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        tool.radius = 10.;
        assert_eq!(
            paint_session_begin_preset(
                h.core,
                h.session,
                library,
                id.as_ptr(),
                id.len() as u64,
                &tool,
                &point,
                &mut sequence
            ),
            PAINT_OK
        );
        settings.tip.roundness = 1.;
        settings.tip.flow = 0.;
        assert_eq!(
            paint_brush_library_edit(library, id.as_ptr(), id.len() as u64, &settings),
            PAINT_OK
        );
        point.x = 0.;
        tool.linear_rgba = [0.; 4];
        assert_eq!(point.x, 0.);
        assert_eq!(tool.linear_rgba, [0.; 4]);
        assert_eq!(
            paint_brush_library_remove(library, id.as_ptr(), id.len() as u64),
            PAINT_OK
        );
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
        let ended = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_END_STROKE,
            ..dto!(PaintCommand)
        }));
        assert_eq!(ended.last_error_status, PAINT_OK);
        assert_eq!(ended.undo_depth, 1);
        let pixels = h.pixels(0, h.frame(0, ended.revision));
        assert!(pixels[(20 * 40 + 27) * 4 + 3] > 200);
        assert_eq!(pixels[(27 * 40 + 20) * 4 + 3], 0);
        let undone = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(undone.undo_depth, 0);
        let redone = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_REDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(h.pixels(0, h.frame(0, redone.revision)), pixels);
    }
}
#[test]
fn prepared_blend_validates_copies_mode_and_keeps_old_normal_pixels() {
    assert_eq!(size_of::<PaintBrushBlendDesc>(), 16);
    assert_eq!(offset_of!(PaintBrushBlendDesc, reserved), 8);
    let mut results = Vec::new();
    for mode in [u32::MAX, 0, 3, 8] {
        let h = Harness::new(40, 40);
        h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_BEGIN_STROKE,
            stroke: PaintStrokeDesc {
                mode: PAINT_MODE_PAINT,
                radius: 30.,
                opacity: 1.,
                spacing: 0.15,
                linear_rgba: [0.6, 0.4, 0.2, 1.],
                ..dto!(PaintStrokeDesc)
            },
            point: PaintPoint {
                x: 20.5,
                y: 20.5,
                pressure: 1.,
                tool: PAINT_TOOL_MOUSE,
                ..dto!(PaintPoint)
            },
            ..dto!(PaintCommand)
        }));
        h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_END_STROKE,
            ..dto!(PaintCommand)
        }));
        // Sized caller data are owned and live; invalid pointers are rejected before dereference.
        unsafe {
            let mut library = ptr::null_mut();
            assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
            let mut catalog = paint_core::BrushCatalog::default();
            let mut settings = catalog.selected().settings().clone();
            settings.opacity = 0.4;
            settings.tip.pressure_size = false;
            settings.tip.pressure_flow = false;
            catalog.edit("round-pressure", settings).unwrap();
            let state = paint_io::brush_state::encode(&catalog).unwrap();
            let mut prepared = 0;
            assert_eq!(
                paint_brush_library_prepare_state(
                    library,
                    state.as_ptr(),
                    state.len() as u64,
                    &mut prepared
                ),
                PAINT_OK
            );
            let id = b"round-pressure";
            let mut tool = PaintBrushToolDesc {
                radius: 8.,
                strength: 1.,
                linear_rgba: [0.4, 0.2, 0.7, 0.75],
                ..dto!(PaintBrushToolDesc)
            };
            let mut background = PaintBrushBackgroundDesc {
                linear_rgba: [0., 0., 1., 1.],
                ..dto!(PaintBrushBackgroundDesc)
            };
            let point = PaintPoint {
                x: 20.5,
                y: 20.5,
                pressure: 1.,
                tool: PAINT_TOOL_MOUSE,
                ..dto!(PaintPoint)
            };
            let mut blend = PaintBrushBlendDesc {
                mode: if mode == u32::MAX { 0 } else { mode },
                ..dto!(PaintBrushBlendDesc)
            };
            let mut sequence = 99;
            for invalid in [
                PaintBrushBlendDesc {
                    struct_size: 15,
                    ..blend
                },
                PaintBrushBlendDesc {
                    reserved: 1,
                    ..blend
                },
                PaintBrushBlendDesc { mode: 27, ..blend },
                PaintBrushBlendDesc {
                    mode: u32::MAX,
                    ..blend
                },
            ] {
                assert_eq!(
                    paint_session_begin_prepared_preset_with_blend(
                        h.core,
                        h.session,
                        library,
                        prepared,
                        id.as_ptr(),
                        id.len() as u64,
                        &tool,
                        &background,
                        &invalid,
                        &point,
                        &mut sequence
                    ),
                    PAINT_INVALID_ARGUMENT
                );
                assert_eq!(sequence, 0);
            }
            assert_eq!(
                paint_session_begin_prepared_preset_with_blend(
                    h.core,
                    h.session,
                    library,
                    prepared,
                    id.as_ptr(),
                    id.len() as u64,
                    &tool,
                    &background,
                    ptr::null(),
                    &point,
                    &mut sequence
                ),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(sequence, 0);
            let misaligned = (&blend as *const PaintBrushBlendDesc)
                .cast::<u8>()
                .add(1)
                .cast::<PaintBrushBlendDesc>();
            assert_eq!(
                paint_session_begin_prepared_preset_with_blend(
                    h.core,
                    h.session,
                    library,
                    prepared,
                    id.as_ptr(),
                    id.len() as u64,
                    &tool,
                    &background,
                    misaligned,
                    &point,
                    &mut sequence
                ),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(sequence, 0);
            let erased = PaintBrushToolDesc {
                operation: PAINT_BRUSH_ERASE,
                ..tool
            };
            let nonnormal = PaintBrushBlendDesc { mode: 3, ..blend };
            assert_eq!(
                paint_session_begin_prepared_preset_with_blend(
                    h.core,
                    h.session,
                    library,
                    prepared,
                    id.as_ptr(),
                    id.len() as u64,
                    &erased,
                    &background,
                    &nonnormal,
                    &point,
                    &mut sequence
                ),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(sequence, 0);
            let status = if mode == u32::MAX {
                paint_session_begin_prepared_preset_with_background(
                    h.core,
                    h.session,
                    library,
                    prepared,
                    id.as_ptr(),
                    id.len() as u64,
                    &tool,
                    &background,
                    &point,
                    &mut sequence,
                )
            } else {
                paint_session_begin_prepared_preset_with_blend(
                    h.core,
                    h.session,
                    library,
                    prepared,
                    id.as_ptr(),
                    id.len() as u64,
                    &tool,
                    &background,
                    &blend,
                    &point,
                    &mut sequence,
                )
            };
            assert_eq!(status, PAINT_OK);
            blend.mode = 8;
            tool.linear_rgba = [0., 1., 0., 1.];
            background.linear_rgba = [1., 0., 0., 1.];
            assert_eq!(blend.mode, 8);
            assert_eq!(tool.linear_rgba[1], 1.);
            assert_eq!(background.linear_rgba[0], 1.);
            assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
            let ended = h.wait(h.submit(PaintCommand {
                kind: PAINT_COMMAND_END_STROKE,
                ..dto!(PaintCommand)
            }));
            assert_eq!(ended.last_error_status, PAINT_OK);
            assert_eq!(ended.undo_depth, 2);
            let frame = h.pixels(0, h.frame(0, ended.revision));
            let undo = h.wait(h.submit(PaintCommand {
                kind: PAINT_COMMAND_UNDO,
                ..dto!(PaintCommand)
            }));
            assert_eq!(undo.undo_depth, 1);
            let redo = h.wait(h.submit(PaintCommand {
                kind: PAINT_COMMAND_REDO,
                ..dto!(PaintCommand)
            }));
            assert_eq!(h.pixels(0, h.frame(0, redo.revision)), frame);
            results.push(frame);
        }
    }
    assert_eq!(results[0], results[1]);
    let center = (20 * 40 + 20) * 4;
    assert!(results[2][center] < results[1][center]);
    assert!(results[3][center] > results[1][center]);
}

#[test]
fn background_prepared_begin_captures_both_colors_and_keeps_old_entry() {
    assert_eq!(size_of::<PaintBrushBackgroundDesc>(), 24);
    assert_eq!(offset_of!(PaintBrushBackgroundDesc, linear_rgba), 8);
    let mut frames = Vec::new();
    for entry in 0..3 {
        let legacy = entry == 1;
        let h = Harness::new(40, 40);
        // Owned DTOs, bytes and handles remain alive through synchronous ABI calls.
        unsafe {
            let mut library = ptr::null_mut();
            assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
            let mut catalog = paint_core::BrushCatalog::default();
            let mut settings = catalog.selected().settings().clone();
            settings.opacity = 0.5;
            settings.tip.pressure_size = false;
            settings.tip.pressure_flow = false;
            settings.dynamics.color =
                paint_core::ColorDynamics::new(true, 1., 0., 0., 0., false, u64::MAX).unwrap();
            catalog.edit("round-pressure", settings).unwrap();
            let json = paint_io::brush_state::encode(&catalog).unwrap();
            let mut prepared = 0;
            assert_eq!(
                paint_brush_library_prepare_state(
                    library,
                    json.as_ptr(),
                    json.len() as u64,
                    &mut prepared
                ),
                PAINT_OK
            );
            let id = b"round-pressure";
            let mut tool = PaintBrushToolDesc {
                radius: 8.,
                strength: 1.,
                linear_rgba: [1., 0., 0., 1.],
                ..dto!(PaintBrushToolDesc)
            };
            let point = PaintPoint {
                x: 20.5,
                y: 20.5,
                pressure: 1.,
                tool: PAINT_TOOL_MOUSE,
                ..dto!(PaintPoint)
            };
            let mut background = PaintBrushBackgroundDesc {
                linear_rgba: [0., 0., 1., 1.],
                ..dto!(PaintBrushBackgroundDesc)
            };
            let mut sequence = 99;
            if !legacy {
                for bad in [
                    PaintBrushBackgroundDesc {
                        struct_size: 23,
                        ..background
                    },
                    PaintBrushBackgroundDesc {
                        reserved: 1,
                        ..background
                    },
                    PaintBrushBackgroundDesc {
                        linear_rgba: [f32::NAN, 0., 1., 1.],
                        ..background
                    },
                ] {
                    assert_eq!(
                        paint_session_begin_prepared_preset_with_background(
                            h.core,
                            h.session,
                            library,
                            prepared,
                            id.as_ptr(),
                            id.len() as u64,
                            &tool,
                            &bad,
                            &point,
                            &mut sequence
                        ),
                        PAINT_INVALID_ARGUMENT
                    );
                    assert_eq!(sequence, 0);
                }
                assert_eq!(
                    paint_session_begin_prepared_preset_with_background(
                        h.core,
                        h.session,
                        library,
                        prepared,
                        id.as_ptr(),
                        id.len() as u64,
                        &tool,
                        ptr::null(),
                        &point,
                        &mut sequence
                    ),
                    PAINT_INVALID_ARGUMENT
                );
                assert_eq!(sequence, 0);
            }
            let status = if legacy {
                paint_session_begin_prepared_preset(
                    h.core,
                    h.session,
                    library,
                    prepared,
                    id.as_ptr(),
                    id.len() as u64,
                    &tool,
                    &point,
                    &mut sequence,
                )
            } else if entry == 2 {
                let blend = dto!(PaintBrushBlendDesc);
                paint_session_begin_prepared_preset_with_blend(
                    h.core,
                    h.session,
                    library,
                    prepared,
                    id.as_ptr(),
                    id.len() as u64,
                    &tool,
                    &background,
                    &blend,
                    &point,
                    &mut sequence,
                )
            } else {
                paint_session_begin_prepared_preset_with_background(
                    h.core,
                    h.session,
                    library,
                    prepared,
                    id.as_ptr(),
                    id.len() as u64,
                    &tool,
                    &background,
                    &point,
                    &mut sequence,
                )
            };
            assert_eq!(status, PAINT_OK);
            background.linear_rgba = [0., 1., 0., 1.];
            tool.linear_rgba = [0., 1., 0., 1.];
            assert_eq!(background.linear_rgba, tool.linear_rgba);
            assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
            let ended = h.wait(h.submit(PaintCommand {
                kind: PAINT_COMMAND_END_STROKE,
                ..dto!(PaintCommand)
            }));
            assert_eq!(ended.last_error_status, PAINT_OK);
            assert_eq!(ended.undo_depth, 1);
            let pixels = h.pixels(0, h.frame(0, ended.revision));
            let center = (20 * 40 + 20) * 4;
            assert!(pixels[center] > 0 && pixels[center + 2] > 0);
            if !legacy {
                assert_eq!(pixels[center + 1], 0);
            }
            frames.push(pixels);
        }
    }
    assert_ne!(frames[0], frames[1]);
    assert_eq!(frames[0], frames[2]);
}

#[test]
fn prepared_preset_begin_uses_exact_revision_after_newer_draft_and_library_destroy() {
    let h = Harness::new(40, 40);
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let mut c = paint_core::BrushCatalog::default();
        let mut s = c.selected().settings().clone();
        s.tip.roundness = 0.2;
        c.edit("round-pressure", s).unwrap();
        let mut json = paint_io::brush_state::encode(&c).unwrap();
        let mut prepared = 0;
        assert_eq!(
            paint_brush_library_prepare_state(
                library,
                json.as_ptr(),
                json.len() as u64,
                &mut prepared
            ),
            PAINT_OK
        );
        json.fill(b' ');
        let id = b"round-pressure";
        let tool = PaintBrushToolDesc {
            radius: 10.,
            strength: 1.,
            linear_rgba: [1., 0., 0., 1.],
            ..dto!(PaintBrushToolDesc)
        };
        let point = PaintPoint {
            x: 20.5,
            y: 20.5,
            pressure: 1.,
            tool: PAINT_TOOL_MOUSE,
            ..dto!(PaintPoint)
        };
        let mut sequence = 0;
        assert_eq!(
            paint_session_begin_prepared_preset(
                h.core,
                h.session,
                library,
                prepared,
                id.as_ptr(),
                id.len() as u64,
                &tool,
                &point,
                &mut sequence
            ),
            PAINT_OK
        );
        let mut s = c.selected().settings().clone();
        s.tip.roundness = 1.;
        s.tip.flow = 0.;
        c.edit("round-pressure", s).unwrap();
        let next = paint_io::brush_state::encode(&c).unwrap();
        assert_eq!(
            paint_brush_library_prepare_state(
                library,
                next.as_ptr(),
                next.len() as u64,
                &mut prepared
            ),
            PAINT_OK
        );
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
        let ended = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_END_STROKE,
            ..dto!(PaintCommand)
        }));
        assert_eq!(ended.last_error_status, PAINT_OK);
        assert_eq!(ended.undo_depth, 1);
        let pixels = h.pixels(0, h.frame(0, ended.revision));
        assert!(pixels[(20 * 40 + 27) * 4 + 3] > 200);
        assert_eq!(pixels[(27 * 40 + 20) * 4 + 3], 0);
    }
}

#[test]
fn pick_layer_queues_selection_and_one_move_history() {
    let h = Harness::new(32, 32);
    let painted = h.wait(h.stroke(8., 8.));
    let mut seq = 0;
    unsafe {
        assert_eq!(
            paint_session_pick_layer(h.core, h.session, 8, 8, 0, 0, &mut seq),
            PAINT_OK
        );
        let selected = h.wait(seq);
        assert_eq!(selected.undo_depth, painted.undo_depth);
        assert_eq!(selected.active_layer_id, painted.active_layer_id);
        assert_eq!(
            paint_session_pick_layer(h.core, h.session, 8, 8, 4, 3, &mut seq),
            PAINT_OK
        );
        let moved = h.wait(seq);
        assert_eq!(moved.last_error_status, PAINT_OK);
        assert_eq!(moved.undo_depth, painted.undo_depth + 1);
        assert_eq!(
            paint_session_pick_layer(h.core, h.session, 31, 31, 4, 3, &mut seq),
            PAINT_OK
        );
        assert_eq!(h.wait(seq).undo_depth, moved.undo_depth);
        assert_eq!(
            paint_session_pick_layer(h.core, h.session, 8, 8, i32::MAX, 0, &mut seq),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(seq, 0);
    }
}

#[test]
fn liquify_blocks_edits_and_file_job_finishes_busy_without_writing() {
    let h = Harness::new(32, 32);
    let mut sequence = 0;
    unsafe {
        assert_eq!(
            paint_session_liquify(
                h.core,
                h.session,
                PAINT_LIQUIFY_BEGIN,
                ptr::null(),
                &mut sequence
            ),
            PAINT_OK
        );
        h.wait(sequence);
    }
    let failed = h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_UNDO,
        ..dto!(PaintCommand)
    }));
    assert_eq!(failed.last_error_status, PAINT_BUSY);
    assert_ne!(failed.flags & PAINT_SESSION_LIQUIFY, 0);
    let path = std::env::temp_dir().join(format!(
        "drawverse-liquify-{}.ora",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let job = h.file(&path, PAINT_FILE_SAVE, PAINT_FILE_OPENRASTER);
    let done = h.file_wait(job);
    assert_eq!(done.state, PAINT_FILE_FAILED);
    assert_eq!(done.status, PAINT_BUSY);
    assert!(!path.exists());
    unsafe {
        assert_eq!(
            paint_session_liquify(
                h.core,
                h.session,
                PAINT_LIQUIFY_CANCEL,
                ptr::null(),
                &mut sequence
            ),
            PAINT_OK
        );
        assert_eq!(h.wait(sequence).flags & PAINT_SESSION_LIQUIFY, 0);
    }
    let job = h.file(&path, PAINT_FILE_SAVE, PAINT_FILE_OPENRASTER);
    assert_eq!(h.file_wait(job).state, PAINT_FILE_SUCCEEDED);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn liquify_abi_validation_preview_cancel_and_single_history() {
    let h = Harness::new(64, 64);
    let point = PaintPoint {
        x: 24.,
        y: 24.,
        pressure: 1.,
        tool: PAINT_TOOL_PEN,
        ..dto!(PaintPoint)
    };
    let brush = PaintStrokeDesc {
        mode: PAINT_MODE_PAINT,
        radius: 8.,
        opacity: 1.,
        spacing: 0.1,
        linear_rgba: [1., 0., 0., 1.],
        ..dto!(PaintStrokeDesc)
    };
    let mut seq = 0;
    unsafe {
        h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_BEGIN_STROKE,
            stroke: brush,
            point,
            ..dto!(PaintCommand)
        }));
        let original = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_END_STROKE,
            ..dto!(PaintCommand)
        }));
        let before = h.pixels(0, h.frame(0, original.revision));
        let mut dab = PaintLiquifyDab {
            tool: 0,
            radius: 16.,
            strength: 0.5,
            density: 1.,
            speed: 1.,
            elapsed_ms: 16.,
            from: point,
            to: PaintPoint { x: 32., ..point },
            ..dto!(PaintLiquifyDab)
        };
        for bad in 0..5 {
            let mut v = dab;
            match bad {
                0 => v.struct_size = 4,
                1 => v.reserved = 1,
                2 => v.from.struct_size = 4,
                3 => v.pressure_enabled = 2,
                _ => v.strength = f32::NAN,
            };
            assert_eq!(
                paint_session_liquify(h.core, h.session, PAINT_LIQUIFY_DAB, &v, &mut seq),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(seq, 0);
        }
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
        let active = h.wait(seq);
        assert_ne!(active.flags & PAINT_SESSION_LIQUIFY, 0);
        assert_eq!(active.undo_depth, 0);
        assert_eq!(
            paint_session_liquify(
                h.core,
                h.session,
                PAINT_LIQUIFY_STROKE_BEGIN,
                ptr::null(),
                &mut seq
            ),
            PAINT_OK
        );
        h.wait(seq);
        assert_eq!(
            paint_session_liquify(h.core, h.session, PAINT_LIQUIFY_DAB, &dab, &mut seq),
            PAINT_OK
        );
        dab.to.x = 0.;
        let preview = h.wait(seq);
        assert_eq!(preview.undo_depth, 0);
        assert_eq!(
            preview.flags & PAINT_SESSION_MODIFIED,
            original.flags & PAINT_SESSION_MODIFIED
        );
        assert_ne!(h.pixels(0, h.frame(0, preview.revision)), before);
        assert_eq!(
            paint_session_liquify(
                h.core,
                h.session,
                PAINT_LIQUIFY_STROKE_END,
                ptr::null(),
                &mut seq
            ),
            PAINT_OK
        );
        assert_eq!(h.wait(seq).undo_depth, 1);
        assert_eq!(
            paint_session_liquify(h.core, h.session, PAINT_LIQUIFY_UNDO, ptr::null(), &mut seq),
            PAINT_OK
        );
        let undone = h.wait(seq);
        assert_eq!(undone.undo_depth, 0);
        assert_eq!(undone.redo_depth, 1);
        assert_eq!(h.pixels(0, h.frame(0, undone.revision)), before);
        assert_eq!(
            paint_session_liquify(h.core, h.session, PAINT_LIQUIFY_REDO, ptr::null(), &mut seq),
            PAINT_OK
        );
        assert_eq!(h.wait(seq).undo_depth, 1);

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
        let cancelled = h.wait(seq);
        assert_eq!(cancelled.flags & PAINT_SESSION_LIQUIFY, 0);
        assert_eq!(h.pixels(0, h.frame(0, cancelled.revision)), before);
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
        dab.to.x = 32.;
        assert_eq!(
            paint_session_liquify(h.core, h.session, PAINT_LIQUIFY_DAB, &dab, &mut seq),
            PAINT_OK
        );
        h.wait(seq);
        assert_eq!(
            paint_session_liquify(
                h.core,
                h.session,
                PAINT_LIQUIFY_COMMIT,
                ptr::null(),
                &mut seq
            ),
            PAINT_OK
        );
        let done = h.wait(seq);
        assert_eq!(done.undo_depth, original.undo_depth + 1);
        let mut entry = dto!(PaintHistoryEntry);
        assert_eq!(
            paint_session_history_entry(
                h.core,
                h.session,
                done.publication,
                done.undo_depth,
                &mut entry
            ),
            PAINT_OK
        );
        assert_eq!(entry.kind, PAINT_HISTORY_LIQUIFY);
    }
}

#[test]
fn deferred_transform_packets_and_latest_geometry_commit() {
    let h = Harness::new(128, 96);
    let initial = h.wait(h.stroke(20., 20.));
    unsafe {
        let mut seq = 99;
        assert_eq!(
            paint_session_begin_transform_layers_deferred(
                h.core,
                h.session,
                ptr::null(),
                0,
                &mut seq
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(seq, 0);
        assert_eq!(
            paint_session_begin_transform_layers_deferred(
                h.core,
                h.session,
                &initial.active_layer_id,
                1,
                &mut seq
            ),
            PAINT_OK
        );
        let begun = h.wait(seq);
        assert_eq!(begun.last_error_status, PAINT_OK);
        let mut packet = dto!(PaintResidentPreviewInfo);
        assert_eq!(
            paint_session_resident_preview_info(h.core, h.session, &mut packet),
            PAINT_OK
        );
        assert_eq!(packet.focus, 2);
        assert!(packet.columns <= 145 && packet.rows <= 145);
        let mut g = dto!(PaintTransform);
        assert_eq!(
            paint_session_transform_info(h.core, h.session, begun.publication, &mut g),
            PAINT_OK
        );
        let original = g.points;
        for i in 1..=1000 {
            g.points = original;
            g.warp = 1;
            for p in g.points.chunks_exact_mut(2) {
                p[0] += i as f64 / 100.;
            }
            g.points[11] += 3.;
            assert_eq!(
                paint_session_transform(h.core, h.session, PAINT_TRANSFORM_UPDATE, &g, &mut seq),
                PAINT_OK
            );
        }
        let updated = h.wait(seq);
        assert_eq!(updated.last_error_status, PAINT_OK);
        assert_eq!(updated.undo_depth, begun.undo_depth);
        let mut latest = dto!(PaintResidentPreviewInfo);
        assert_eq!(
            paint_session_resident_preview_info(h.core, h.session, &mut latest),
            PAINT_OK
        );
        assert_eq!(latest.source_id, packet.source_id);
        let mut required = 0;
        assert_eq!(
            paint_session_read_resident_preview(
                h.core,
                h.session,
                packet.document_generation,
                packet.revision,
                packet.source_id,
                0,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUSY
        );
        assert_eq!(
            paint_session_read_resident_preview(
                h.core,
                h.session,
                latest.document_generation,
                latest.revision,
                latest.source_id,
                0,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        let mut bytes = vec![0x5a; required as usize];
        assert_eq!(
            paint_session_read_resident_preview(
                h.core,
                h.session,
                latest.document_generation,
                latest.revision,
                latest.source_id,
                0,
                bytes.as_mut_ptr(),
                required - 1,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert!(bytes.iter().all(|&b| b == 0x5a));
        let mut current = dto!(PaintTransform);
        assert_eq!(
            paint_session_transform_info(h.core, h.session, updated.publication, &mut current),
            PAINT_OK
        );
        assert_eq!(current.points, g.points);
        assert_eq!(
            paint_session_transform(
                h.core,
                h.session,
                PAINT_TRANSFORM_CHECKPOINT,
                ptr::null(),
                &mut seq
            ),
            PAINT_OK
        );
        h.wait(seq);
        assert_eq!(
            paint_session_transform(
                h.core,
                h.session,
                PAINT_TRANSFORM_COMMIT,
                ptr::null(),
                &mut seq
            ),
            PAINT_OK
        );
        let committed = h.wait(seq);
        assert_eq!(committed.last_error_status, PAINT_OK);
        assert_eq!(committed.undo_depth, initial.undo_depth + 1);
        assert_eq!(committed.flags & PAINT_SESSION_TRANSFORM, 0);
        assert_eq!(
            paint_session_resident_preview_info(h.core, h.session, &mut latest),
            PAINT_NOT_FOUND
        );
        let pixels = h.pixels(0, h.frame(0, committed.revision));
        assert!(pixels[(20 * 128 + 30) * 4] > 200);
        h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        // File placement opens an ordinary transform first; explicit opt-in must work too.
        for kind in [PAINT_TRANSFORM_BEGIN, PAINT_TRANSFORM_ENABLE_RESIDENT] {
            assert_eq!(
                paint_session_transform(h.core, h.session, kind, ptr::null(), &mut seq),
                PAINT_OK
            );
            assert_eq!(h.wait(seq).last_error_status, PAINT_OK);
        }
        assert_eq!(
            paint_session_resident_preview_info(h.core, h.session, &mut latest),
            PAINT_OK
        );
        assert_eq!(latest.focus, 2);
        assert_eq!(
            paint_session_transform(
                h.core,
                h.session,
                PAINT_TRANSFORM_CANCEL,
                ptr::null(),
                &mut seq
            ),
            PAINT_OK
        );
        h.wait(seq);
    }
}

#[test]
fn layer_move_resident_packets_absolute_updates_and_final_barrier() {
    let h = Harness::new(128, 96);
    let initial = h.wait(h.stroke(20., 20.));
    unsafe {
        let mut seq = 0;
        let run = |kind, x, y, seq: &mut u64| {
            assert_eq!(
                paint_session_layer_move(
                    h.core,
                    h.session,
                    kind,
                    initial.active_layer_id,
                    x,
                    y,
                    seq
                ),
                PAINT_OK
            );
            h.wait(*seq)
        };
        let begun = run(PAINT_LAYER_MOVE_BEGIN_DEFERRED, 0, 0, &mut seq);
        assert_ne!(begun.flags & PAINT_SESSION_LAYER_MOVE, 0);
        let mut packet = dto!(PaintResidentPreviewInfo);
        assert_eq!(
            paint_session_resident_preview_info(h.core, h.session, &mut packet),
            PAINT_OK
        );
        assert_eq!((packet.columns, packet.rows), (2, 2));
        let mut required = 99;
        let mut short = [0xAB; 63];
        assert_eq!(
            paint_session_read_resident_preview(
                h.core,
                h.session,
                packet.document_generation,
                packet.revision,
                packet.source_id,
                0,
                short.as_mut_ptr(),
                63,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(required, 64);
        assert_eq!(short, [0xAB; 63]);
        let mut assets = 0;
        assert_eq!(
            paint_session_read_resident_preview(
                h.core,
                h.session,
                packet.document_generation,
                packet.revision,
                packet.source_id,
                1,
                ptr::null_mut(),
                0,
                &mut assets
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert!(assets > 24);
        // Saturate the input tail: absolute updates may be coalesced, the final barrier may not.
        for x in 0..1000 {
            assert_eq!(
                paint_session_layer_move(
                    h.core,
                    h.session,
                    PAINT_LAYER_MOVE_UPDATE,
                    0,
                    x,
                    -x,
                    &mut seq
                ),
                PAINT_OK
            );
        }
        let updated = h.wait(seq);
        assert_eq!(updated.undo_depth, initial.undo_depth);
        let mut latest = dto!(PaintResidentPreviewInfo);
        assert_eq!(
            paint_session_resident_preview_info(h.core, h.session, &mut latest),
            PAINT_OK
        );
        assert_eq!(latest.source_id, packet.source_id);
        let mut mesh = [0; 64];
        assert_eq!(
            paint_session_read_resident_preview(
                h.core,
                h.session,
                packet.document_generation,
                packet.revision,
                packet.source_id,
                0,
                mesh.as_mut_ptr(),
                64,
                &mut required
            ),
            PAINT_BUSY
        );
        let committed = run(PAINT_LAYER_MOVE_COMMIT, 15, 3, &mut seq);
        assert_eq!(committed.flags & PAINT_SESSION_LAYER_MOVE, 0);
        assert_eq!(committed.undo_depth, initial.undo_depth + 1);
        let mut appearance = dto!(PaintLayerAppearance);
        assert_eq!(
            paint_session_layer_appearance(
                h.core,
                h.session,
                committed.publication,
                initial.active_layer_id,
                &mut appearance
            ),
            PAINT_OK
        );
        assert_eq!((appearance.offset_x, appearance.offset_y), (15, 3));
        assert_eq!(
            paint_session_resident_preview_info(h.core, h.session, &mut latest),
            PAINT_NOT_FOUND
        );
        run(PAINT_LAYER_MOVE_BEGIN, 0, 0, &mut seq);
        let preview = run(PAINT_LAYER_MOVE_UPDATE, 10, 12, &mut seq);
        let pixels = h.pixels(0, h.frame(0, preview.revision));
        assert!(pixels[(35 * 128 + 45) * 4 + 3] > 200);
        assert_eq!(pixels[(20 * 128 + 20) * 4 + 3], 0);
        let cancelled = run(PAINT_LAYER_MOVE_CANCEL, 0, 0, &mut seq);
        assert_eq!(cancelled.undo_depth, committed.undo_depth);
        assert_eq!(
            paint_session_layer_appearance(
                h.core,
                h.session,
                cancelled.publication,
                initial.active_layer_id,
                &mut appearance
            ),
            PAINT_OK
        );
        assert_eq!((appearance.offset_x, appearance.offset_y), (15, 3));
    }
}

#[test]
fn deferred_liquify_packets_are_stamped_and_drag_keeps_cpu_frame() {
    let h = Harness::new(128, 96);
    let initial = h.wait(h.stroke(40., 40.));
    let before = h.frame(0, initial.revision);
    unsafe {
        let mut seq = 0;
        assert_eq!(
            paint_session_liquify(
                h.core,
                h.session,
                PAINT_LIQUIFY_BEGIN_DEFERRED,
                ptr::null(),
                &mut seq
            ),
            PAINT_OK
        );
        h.wait(seq);
        let mut info = dto!(PaintLiquifyPreviewInfo);
        assert_eq!(
            paint_session_liquify_preview_info(h.core, h.session, &mut info),
            PAINT_OK
        );
        let mut required = 0;
        assert_eq!(
            paint_session_read_liquify_preview(
                h.core,
                h.session,
                info.document_generation,
                info.revision,
                info.source_id,
                0,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(
            required,
            u64::from(info.columns) * u64::from(info.rows) * 16
        );
        let mut short = vec![0xAB; required as usize - 1];
        assert_eq!(
            paint_session_read_liquify_preview(
                h.core,
                h.session,
                info.document_generation,
                info.revision,
                info.source_id,
                0,
                short.as_mut_ptr(),
                short.len() as u64,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert!(short.iter().all(|b| *b == 0xAB));
        let dab = PaintLiquifyDab {
            radius: 7500.,
            strength: 0.8,
            density: 0.7,
            speed: 1.,
            elapsed_ms: 16.,
            from: PaintPoint {
                x: 40.,
                y: 40.,
                pressure: 1.,
                tool: PAINT_TOOL_MOUSE,
                ..dto!(PaintPoint)
            },
            to: PaintPoint {
                x: 70.,
                y: 40.,
                pressure: 1.,
                tool: PAINT_TOOL_MOUSE,
                ..dto!(PaintPoint)
            },
            ..dto!(PaintLiquifyDab)
        };
        assert_eq!(
            paint_session_liquify(h.core, h.session, PAINT_LIQUIFY_DAB, &dab, &mut seq),
            PAINT_OK
        );
        let dragged = h.wait(seq);
        assert_eq!(dragged.last_error_status, PAINT_OK);
        assert_eq!(
            paint_session_read_liquify_preview(
                h.core,
                h.session,
                info.document_generation,
                info.revision,
                info.source_id,
                0,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUSY
        );
        assert_eq!(
            paint_session_read_liquify_preview(
                h.core,
                h.session,
                info.document_generation + 1,
                info.revision,
                info.source_id,
                1,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUSY
        );
        assert_eq!(
            paint_session_read_liquify_preview(
                h.core,
                h.session,
                info.document_generation,
                info.revision,
                info.source_id,
                1,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        let mut assets = vec![0; required as usize];
        assert_eq!(
            paint_session_read_liquify_preview(
                h.core,
                h.session,
                info.document_generation,
                info.revision,
                info.source_id,
                1,
                assets.as_mut_ptr(),
                assets.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        assert!(assets.len() > 24);
        let mut frame = dto!(PaintFrameInfo);
        assert_eq!(
            paint_session_frame_info(h.core, h.session, 0, &mut frame),
            PAINT_BUSY,
            "mesh drag must not generate a CPU frame at the new render floor"
        );
        assert_eq!(
            paint_session_liquify(
                h.core,
                h.session,
                PAINT_LIQUIFY_COMMIT,
                ptr::null(),
                &mut seq
            ),
            PAINT_OK
        );
        let committed = h.wait(seq);
        assert_eq!(committed.last_error_status, PAINT_OK);
        assert_eq!(
            paint_session_liquify_preview_info(h.core, h.session, &mut info),
            PAINT_NOT_FOUND
        );
        assert!(h.frame(0, committed.revision).revision > before.revision);
    }
}

#[test]
fn smudge_validation_copy_real_pixels_and_single_history() {
    assert_eq!(size_of::<PaintSmudgeStrokeDesc>(), 88);
    assert_eq!(offset_of!(PaintSmudgeStrokeDesc, brush), 32);
    let h = Harness::new(8, 8);
    let mut request = PaintSmudgeStrokeDesc {
        strength: 0.5,
        finger_painting: 1,
        pressure_strength: 1,
        brush: PaintStrokeDesc {
            mode: PAINT_MODE_PAINT,
            radius: 2.,
            opacity: 1.,
            spacing: 0.02,
            linear_rgba: [1., 0., 0., 1.],
            ..dto!(PaintStrokeDesc)
        },
        ..dto!(PaintSmudgeStrokeDesc)
    };
    let point = PaintPoint {
        tool: PAINT_TOOL_PEN,
        x: 3.,
        y: 3.,
        pressure: 1.,
        ..dto!(PaintPoint)
    };
    let mut sequence = 99;
    unsafe {
        assert_eq!(
            paint_session_begin_smudge(h.core, h.session, ptr::null(), &point, &mut sequence),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        for field in 0..6 {
            let mut bad = request;
            match field {
                0 => bad.reserved[0] = 1,
                1 => bad.all_layers = 2,
                2 => bad.strength = f32::NAN,
                3 => bad.brush.struct_size = 4,
                4 => bad.brush.spacing = 0.019,
                _ => bad.brush.mode = PAINT_MODE_ERASE,
            }
            assert_eq!(
                paint_session_begin_smudge(h.core, h.session, &bad, &point, &mut sequence),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(sequence, 0);
        }
        assert_eq!(
            paint_session_begin_smudge(h.core, h.session, &request, &point, &mut sequence),
            PAINT_OK
        );
        request.brush.linear_rgba = [0., 1., 0., 1.];
        assert_eq!(request.brush.linear_rgba[1], 1.);
        assert_eq!(h.wait(sequence).last_error_status, PAINT_OK);
        let info = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_END_STROKE,
            ..dto!(PaintCommand)
        }));
        assert_eq!(info.last_error_status, PAINT_OK);
        assert_eq!(info.undo_depth, 1);
        let bytes = h.pixels(0, h.frame(0, info.revision));
        assert!(bytes
            .chunks_exact(4)
            .any(|p| p[0] > 0 && p[1] == 0 && p[3] > 0));
        let mut history = dto!(PaintHistoryEntry);
        assert_eq!(
            paint_session_history_entry(h.core, h.session, info.publication, 1, &mut history),
            PAINT_OK
        );
        assert_eq!(history.kind, PAINT_HISTORY_SMUDGE);
        let info = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(info.undo_depth, 0);
        assert!(h
            .pixels(0, h.frame(0, info.revision))
            .iter()
            .all(|b| *b == 0));
    }
}

#[test]
fn bucket_dto_validation_copy_real_pixels_and_single_history() {
    assert_eq!(size_of::<PaintBucketFill>(), 64);
    assert_eq!(offset_of!(PaintBucketFill, linear_rgba), 16);
    let h = Harness::new(32, 32);
    let mut request = PaintBucketFill {
        x: 0,
        y: 0,
        linear_rgba: [1., 0., 0., 1.],
        opacity: 1.,
        antialias: 1,
        contiguous: 1,
        ..dto!(PaintBucketFill)
    };
    let mut sequence = 99;
    unsafe {
        assert_eq!(
            paint_session_bucket_fill(h.core, h.session, ptr::null(), &mut sequence),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        for field in 0..5 {
            let mut bad = request;
            match field {
                0 => bad.reserved = 1,
                1 => bad.tolerance = 256,
                2 => bad.contiguous = 2,
                3 => bad.opacity = f32::NAN,
                _ => bad.struct_size = 4,
            };
            assert_eq!(
                paint_session_bucket_fill(h.core, h.session, &bad, &mut sequence),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(sequence, 0);
        }
        assert_eq!(
            paint_session_bucket_fill(h.core, h.session, &request, &mut sequence),
            PAINT_OK
        );
        request.linear_rgba = [0., 1., 0., 1.];
        let info = h.wait(sequence);
        assert_eq!(info.last_error_status, PAINT_OK);
        assert_eq!(info.undo_depth, 1);
        let frame = h.frame(0, info.revision);
        let bytes = h.pixels(0, frame);
        assert!(bytes.chunks_exact(4).all(|p| p == [255, 0, 0, 255]));
        let mut history = dto!(PaintHistoryEntry);
        assert_eq!(
            paint_session_history_entry(h.core, h.session, info.publication, 1, &mut history),
            PAINT_OK
        );
        assert_eq!(history.kind, PAINT_HISTORY_BUCKET);
        let undo = h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        });
        let info = h.wait(undo);
        assert_eq!(info.undo_depth, 0);
        assert!(h
            .pixels(0, h.frame(0, info.revision))
            .iter()
            .all(|b| *b == 0));
        request.x = 32;
        assert_eq!(
            paint_session_bucket_fill(h.core, h.session, &request, &mut sequence),
            PAINT_OK
        );
        let info = h.wait(sequence);
        assert_eq!(info.last_error_status, PAINT_INVALID_ARGUMENT);
        assert_eq!(info.undo_depth, 0);
    }
}
#[test]
fn wand_request_validation_copy_mask_query_history_and_undo() {
    assert_eq!(size_of::<PaintWandSelection>(), 48);
    let h = Harness::new(8, 4);
    let mut sequence = 99;
    let mut request = PaintWandSelection {
        sample_size: 1,
        tolerance: 10,
        antialias: 1,
        contiguous: 1,
        ..dto!(PaintWandSelection)
    };
    unsafe {
        assert_eq!(
            paint_session_select_wand(h.core, h.session, ptr::null(), &mut sequence),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        for i in 0..6 {
            let mut bad = request;
            match i {
                0 => bad.struct_size = 4,
                1 => bad.sample_size = 2,
                2 => bad.tolerance = 256,
                3 => bad.antialias = 2,
                4 => bad.operation = 4,
                _ => bad.reserved[0] = 1,
            };
            assert_eq!(
                paint_session_select_wand(h.core, h.session, &bad, &mut sequence),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(sequence, 0);
        }
        assert_eq!(
            paint_session_select_wand(h.core, h.session, &request, &mut sequence),
            PAINT_OK
        );
        request.x = 999;
        let info = h.wait(sequence);
        assert_eq!(info.last_error_status, PAINT_OK);
        assert_eq!(info.undo_depth, 1);
        assert_ne!(info.flags & PAINT_SESSION_MODIFIED, 0);
        let mut length = 0;
        assert_eq!(
            paint_session_selection_mask(
                h.core,
                h.session,
                info.publication,
                0,
                ptr::null_mut(),
                0,
                &mut length
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(length, 32);
        let mut bytes = vec![0; length as usize];
        assert_eq!(
            paint_session_selection_mask(
                h.core,
                h.session,
                info.publication,
                0,
                bytes.as_mut_ptr(),
                length,
                &mut length
            ),
            PAINT_OK
        );
        assert!(bytes.iter().all(|&v| v == 255));
        assert_eq!(
            paint_session_selection_mask(
                h.core,
                h.session,
                info.publication.wrapping_sub(1),
                0,
                bytes.as_mut_ptr(),
                length,
                &mut length
            ),
            PAINT_BUSY
        );
        assert_eq!(length, 0);
        let mut history = dto!(PaintHistoryEntry);
        assert_eq!(
            paint_session_history_entry(h.core, h.session, info.publication, 1, &mut history),
            PAINT_OK
        );
        assert_eq!(history.kind, PAINT_HISTORY_MAGIC_SELECTION);
        let info = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(info.undo_depth, 0);
        let mut summary = dto!(PaintSelectionInfo);
        assert_eq!(
            paint_session_selection_info(h.core, h.session, info.publication, &mut summary),
            PAINT_OK
        );
        assert_eq!(summary.enabled, 0);
        assert_eq!(
            paint_session_select_wand(h.core, h.session, &request, &mut sequence),
            PAINT_OK
        );
        let bad = h.wait(sequence);
        assert_eq!(bad.last_error_status, PAINT_INVALID_ARGUMENT);
        assert_eq!(bad.undo_depth, 0);
    }
}
#[test]
fn batch_layer_abi_copies_ids_rejects_bad_buffers_and_commits_one_step() {
    let h = Harness::new(32, 32);
    for _ in 0..3 {
        h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_ADD_LAYER,
            text: b"L".as_ptr(),
            text_length: 1,
            ..dto!(PaintCommand)
        }));
    }
    let before = h.info();
    let mut sequence = 99;
    unsafe {
        for (pointer, count, mode) in [
            (ptr::null(), 1, 0),
            ([1u64].as_ptr(), 0, 0),
            ([1u64].as_ptr(), 4097, 0),
            ([1u64].as_ptr(), 1, 9),
        ] {
            assert_eq!(
                paint_session_remove_layers(h.core, h.session, pointer, count, mode, &mut sequence),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(sequence, 0);
        }
        let duplicate = [2, 2];
        assert_eq!(
            paint_session_drop_layers(
                h.core,
                h.session,
                duplicate.as_ptr(),
                2,
                1,
                2,
                &mut sequence
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        let mut ids = [2, 4];
        assert_eq!(
            paint_session_drop_layers(h.core, h.session, ids.as_ptr(), 2, 1, 2, &mut sequence),
            PAINT_OK
        );
        ids.fill(999); // Must not read caller memory from the actor.
        let moved = h.wait(sequence);
        assert_eq!(moved.last_error_status, PAINT_OK);
        assert_eq!(moved.undo_depth, before.undo_depth + 1);
        let mut layer = dto!(PaintLayerInfo);
        for (index, id) in [2, 4, 1, 3].into_iter().enumerate() {
            assert_eq!(
                paint_session_layer_info(
                    h.core,
                    h.session,
                    moved.publication,
                    index as u32,
                    &mut layer
                ),
                PAINT_OK
            );
            assert_eq!(layer.layer_id, id);
        }
        ids = [2, 4];
        assert_eq!(
            paint_session_remove_layers(h.core, h.session, ids.as_ptr(), 2, 0, &mut sequence),
            PAINT_OK
        );
        ids.fill(999);
        let removed = h.wait(sequence);
        assert_eq!(removed.layer_count, 2);
        assert_eq!(removed.undo_depth, moved.undo_depth + 1);
        let restored = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(restored.layer_count, 4);
        assert_eq!(restored.undo_depth, moved.undo_depth);
        let invalid = [1, 999];
        assert_eq!(
            paint_session_remove_layers(h.core, h.session, invalid.as_ptr(), 2, 0, &mut sequence),
            PAINT_OK
        );
        let failed = h.wait(sequence);
        assert_eq!(failed.last_error_status, PAINT_NOT_FOUND);
        assert_eq!(failed.layer_count, restored.layer_count);
        assert_eq!(failed.revision, restored.revision);
        assert_eq!(failed.undo_depth, restored.undo_depth);
    }
}

struct Harness {
    core: *mut PaintCore,
    session: *mut PaintSession,
}

#[test]
fn layer_rename_abi_copies_utf8_and_preserves_undo() {
    let h = Harness::new(64, 64);
    let mut name = "图层改名".as_bytes().to_vec();
    let request = PaintGroupRequest {
        struct_size: size_of::<PaintGroupRequest>() as u32,
        kind: PAINT_GROUP_RENAME_LAYER,
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
    assert_eq!(info.undo_depth, 1);
    let read_name = |publication| {
        let mut out = [0; 64];
        let mut required = 0;
        unsafe {
            assert_eq!(
                paint_session_layer_name(
                    h.core,
                    h.session,
                    publication,
                    1,
                    out.as_mut_ptr(),
                    64,
                    &mut required
                ),
                PAINT_OK
            );
        }
        String::from_utf8(out[..required as usize].to_vec()).unwrap()
    };
    assert_eq!(read_name(info.publication), "图层改名");
    let undone = h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_UNDO,
        ..dto!(PaintCommand)
    }));
    assert_eq!(read_name(undone.publication), "Layer 1");
    let redone = h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_REDO,
        ..dto!(PaintCommand)
    }));
    assert_eq!(read_name(redone.publication), "图层改名");
}

#[test]
fn batch_group_abi_copies_selection_and_name_and_is_one_undo_step() {
    let h = Harness::new(32, 32);
    for _ in 0..3 {
        h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_ADD_LAYER,
            text: b"L".as_ptr(),
            text_length: 1,
            ..dto!(PaintCommand)
        }));
    }
    let before = h.info();
    let mut sequence = 99;
    unsafe {
        assert_eq!(
            paint_session_group_layers(
                h.core,
                h.session,
                ptr::null(),
                1,
                b"G".as_ptr(),
                1,
                &mut sequence
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        let mut selected = [2u64, 4];
        let mut name = *b"chosen";
        assert_eq!(
            paint_session_group_layers(
                h.core,
                h.session,
                selected.as_ptr(),
                2,
                name.as_ptr(),
                name.len() as u64,
                &mut sequence
            ),
            PAINT_OK
        );
        selected.fill(999);
        name.fill(b'x');
        let grouped = h.wait(sequence);
        assert_eq!(grouped.layer_count, before.layer_count + 1);
        assert_eq!(grouped.undo_depth, before.undo_depth + 1);
        let group = grouped.active_layer_id;
        for id in [2, 4] {
            let mut node = dto!(PaintLayerHierarchy);
            assert_eq!(
                paint_session_layer_hierarchy(
                    h.core,
                    h.session,
                    grouped.publication,
                    id,
                    &mut node
                ),
                PAINT_OK
            );
            assert_eq!(node.parent_id, group);
        }
        let mut buffer = [0u8; 32];
        let mut required = 0;
        assert_eq!(
            paint_session_layer_name(
                h.core,
                h.session,
                grouped.publication,
                group,
                buffer.as_mut_ptr(),
                buffer.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        assert_eq!(&buffer[..required as usize], b"chosen");
        let undone = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(undone.layer_count, before.layer_count);
        for id in [2, 4] {
            let mut node = dto!(PaintLayerHierarchy);
            assert_eq!(
                paint_session_layer_hierarchy(h.core, h.session, undone.publication, id, &mut node),
                PAINT_OK
            );
            assert_eq!(node.parent_id, 0);
        }
        let redone = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_REDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(redone.layer_count, grouped.layer_count);
    }
}

#[test]
fn history_entry_metadata_has_exact_publication_tool_labels_and_stable_redo() {
    assert_eq!(size_of::<PaintHistoryEntry>(), 16);
    assert_eq!(offset_of!(PaintHistoryEntry, depth), 8);
    let h = Harness::new(32, 32);
    h.wait(h.stroke(16., 16.));
    let mut sequence = 0;
    unsafe {
        assert_eq!(
            paint_session_edit_selection(
                h.core,
                h.session,
                &PaintSelectionEdit {
                    action: PAINT_SELECTION_ALL,
                    ..dto!(PaintSelectionEdit)
                },
                &mut sequence
            ),
            PAINT_OK
        );
    }
    h.wait(sequence);
    h.submit(PaintCommand {
        kind: PAINT_COMMAND_BEGIN_STROKE,
        stroke: PaintStrokeDesc {
            mode: PAINT_MODE_ERASE,
            radius: 2.,
            opacity: 1.,
            spacing: 0.15,
            linear_rgba: [0., 0., 0., 1.],
            ..dto!(PaintStrokeDesc)
        },
        point: PaintPoint {
            x: 16.,
            y: 16.,
            pressure: 1.,
            tool: PAINT_TOOL_ERASER,
            ..dto!(PaintPoint)
        },
        ..dto!(PaintCommand)
    });
    let state = h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_END_STROKE,
        ..dto!(PaintCommand)
    }));
    unsafe {
        for (depth, kind) in [
            PAINT_HISTORY_INITIAL,
            PAINT_HISTORY_BRUSH,
            PAINT_HISTORY_SELECT_ALL,
            PAINT_HISTORY_ERASER,
        ]
        .into_iter()
        .enumerate()
        {
            let mut entry = dto!(PaintHistoryEntry);
            assert_eq!(
                paint_session_history_entry(
                    h.core,
                    h.session,
                    state.publication,
                    depth as u32,
                    &mut entry
                ),
                PAINT_OK
            );
            assert_eq!(
                (entry.kind, entry.depth, entry.reserved),
                (kind, depth as u32, 0)
            );
        }
        let undone = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        let mut entry = PaintHistoryEntry {
            kind: 999,
            depth: 999,
            ..dto!(PaintHistoryEntry)
        };
        assert_eq!(
            paint_session_history_entry(h.core, h.session, state.publication, 3, &mut entry),
            PAINT_BUSY
        );
        assert_eq!((entry.kind, entry.depth), (0, 0));
        assert_eq!(
            paint_session_history_entry(h.core, h.session, undone.publication, 3, &mut entry),
            PAINT_OK
        );
        assert_eq!(entry.kind, PAINT_HISTORY_ERASER);
        assert_eq!(
            paint_session_history_entry(h.core, h.session, undone.publication, 99, &mut entry),
            PAINT_NOT_FOUND
        );
        assert_eq!((entry.kind, entry.depth), (0, 0));
    }
}

#[test]
fn blend_preview_is_composite_non_mutating_replaceable_and_validated() {
    let h = Harness::new(32, 32);
    h.submit(PaintCommand {
        kind: PAINT_COMMAND_BEGIN_STROKE,
        stroke: PaintStrokeDesc {
            mode: PAINT_MODE_PAINT,
            radius: 8.,
            opacity: 1.,
            spacing: 0.15,
            linear_rgba: [0., 0., 0., 1.],
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
    });
    h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_END_STROKE,
        ..dto!(PaintCommand)
    }));
    h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_ADD_LAYER,
        text: b"red".as_ptr(),
        text_length: 3,
        ..dto!(PaintCommand)
    }));
    let top = h.info().active_layer_id;
    let state = h.wait(h.stroke(16., 16.));
    let view = PaintViewport {
        enabled: 1,
        document_generation: state.document_generation,
        width: 32.,
        height: 32.,
        pixel_width: 32,
        pixel_height: 32,
        ..dto!(PaintViewport)
    };
    let mut request = 0;
    unsafe {
        for (layer, mode, slot, status) in [
            (0, 3, 0, PAINT_INVALID_ARGUMENT),
            (9999, 3, 0, PAINT_NOT_FOUND),
            (top, 28, 0, PAINT_INVALID_ARGUMENT),
            (top, 3, 2, PAINT_INVALID_ARGUMENT),
        ] {
            request = 999;
            assert_eq!(
                paint_session_set_blend_preview(
                    h.core,
                    h.session,
                    layer,
                    mode,
                    &PaintViewport {
                        view_id: slot,
                        ..view
                    },
                    &mut request
                ),
                status
            );
            assert_eq!(request, 0);
        }
        for slot in 0..2 {
            let v = PaintViewport {
                view_id: slot,
                ..view
            };
            assert_eq!(
                paint_session_set_blend_preview(h.core, h.session, top, 3, &v, &mut request),
                PAINT_OK
            );
            let frame = h.frame(slot, state.revision);
            assert_eq!(frame.request_id, request);
            let pixels = h.pixels(slot, frame);
            assert_eq!(
                &pixels[(16 * 32 + 16) * 4..(16 * 32 + 16) * 4 + 4],
                &[0, 0, 0, 255]
            );
            // Replacing a pending preview with the regular viewport cancels the override.
            assert_eq!(
                paint_session_set_blend_preview(h.core, h.session, top, 8, &v, &mut request),
                PAINT_OK
            );
            assert_eq!(
                paint_session_set_viewport(h.core, h.session, &v, &mut request),
                PAINT_OK
            );
            let frame = h.frame(slot, state.revision);
            assert_eq!(frame.request_id, request);
            let pixels = h.pixels(slot, frame);
            assert_eq!(
                &pixels[(16 * 32 + 16) * 4..(16 * 32 + 16) * 4 + 4],
                &[255, 0, 0, 255]
            );
        }
        let after = h.info();
        assert_eq!(
            (
                after.revision,
                after.undo_depth,
                after.redo_depth,
                after.completed_sequence
            ),
            (
                state.revision,
                state.undo_depth,
                state.redo_depth,
                state.completed_sequence
            )
        );
        let mut appearance = dto!(PaintLayerAppearance);
        assert_eq!(
            paint_session_layer_appearance(
                h.core,
                h.session,
                after.publication,
                top,
                &mut appearance
            ),
            PAINT_OK
        );
        assert_eq!(appearance.blend_mode, 0);
        let stale = PaintViewport {
            document_generation: state.document_generation + 1,
            ..view
        };
        assert_eq!(
            paint_session_set_blend_preview(h.core, h.session, top, 3, &stale, &mut request),
            PAINT_BUSY
        );
        assert_eq!(request, 0);
    }
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
                blend_mode: 28,
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

#[test]
fn common_selection_abi_copies_lasso_points_and_publishes_settings() {
    assert_eq!(size_of::<PaintSelectionShapeEdit>(), 72);
    assert_eq!(offset_of!(PaintSelectionShapeEdit, feather), 24);
    assert_eq!(size_of::<PaintSelectionDetails>(), 16);
    let h = Harness::new(64, 64);
    let mut path = vec![
        PaintSelectionPoint { x: 8., y: 8. },
        PaintSelectionPoint { x: 56., y: 8. },
        PaintSelectionPoint { x: 8., y: 56. },
    ];
    let mut request = PaintSelectionShapeEdit {
        shape: PAINT_SELECTION_POLYGON,
        tool: 2,
        antialias: 1,
        feather: 3.,
        point_count: 3,
        points: path.as_ptr(),
        ..dto!(PaintSelectionShapeEdit)
    };
    let mut sequence = 999;
    unsafe {
        request.point_count = 4097;
        assert_eq!(
            paint_session_edit_selection_shape(h.core, h.session, &request, &mut sequence),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        request.point_count = 3;
        request.points = ptr::null();
        assert_eq!(
            paint_session_edit_selection_shape(h.core, h.session, &request, &mut sequence),
            PAINT_INVALID_ARGUMENT
        );
        request.points = path.as_ptr();
        assert_eq!(
            paint_session_edit_selection_shape(h.core, h.session, &request, &mut sequence),
            PAINT_OK
        );
        for point in &mut path {
            point.x = 999.;
            point.y = 999.;
        }
        let info = h.wait(sequence);
        assert_eq!(info.undo_depth, 1);
        let mut details = dto!(PaintSelectionDetails);
        assert_eq!(
            paint_session_selection_details(
                h.core,
                h.session,
                info.publication,
                0,
                &mut details,
                ptr::null_mut(),
                0
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(details.point_count, 3);
        assert_eq!(details.feather, 3.);
        let mut copied = [PaintSelectionPoint::default(); 3];
        assert_eq!(
            paint_session_selection_details(
                h.core,
                h.session,
                info.publication,
                0,
                &mut details,
                copied.as_mut_ptr(),
                3
            ),
            PAINT_OK
        );
        assert_eq!(copied[0].x, 8.);
        assert_eq!(copied[1].x, 56.);
        let mut entry = dto!(PaintHistoryEntry);
        assert_eq!(
            paint_session_history_entry(h.core, h.session, info.publication, 1, &mut entry),
            PAINT_OK
        );
        assert_eq!(entry.kind, PAINT_HISTORY_POLYGON_SELECTION);
        assert_eq!(
            paint_session_selection_details(
                h.core,
                h.session,
                info.publication.wrapping_sub(1),
                0,
                &mut details,
                copied.as_mut_ptr(),
                3
            ),
            PAINT_BUSY
        );
        let revision = info.revision;
        request.feather = -1.;
        request.points = path.as_ptr();
        assert_eq!(
            paint_session_edit_selection_shape(h.core, h.session, &request, &mut sequence),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(h.info().revision, revision);
    }
}

#[test]
fn pass_through_group_appearance_validation_publication_and_undo() {
    let h = Harness::new(8, 8);
    let mut seq = 99;
    let appearance = PaintLayerAppearance {
        blend_mode: PAINT_BLEND_PASS_THROUGH,
        fill: 1.,
        ..dto!(PaintLayerAppearance)
    };
    unsafe {
        let mut version = dto!(PaintVersion);
        assert_eq!(paint_core_version(&mut version), PAINT_OK);
        assert_eq!(version.minor, PAINT_ABI_MINOR);
        let mut caps = dto!(PaintCapabilities);
        assert_eq!(paint_core_capabilities(h.core, &mut caps), PAINT_OK);
        assert_ne!(caps.features & PAINT_FEATURE_PASS_THROUGH, 0);
        let name = b"pass";
        let req = PaintGroupRequest {
            kind: PAINT_GROUP_WRAP,
            layer_id: 1,
            name: name.as_ptr(),
            name_length: name.len() as u64,
            struct_size: size_of::<PaintGroupRequest>() as u32,
            parent_id: 0,
        };
        assert_eq!(
            paint_session_group(h.core, h.session, &req, &mut seq),
            PAINT_OK
        );
        let info = h.wait(seq);
        let group = info.active_layer_id;
        let normal = PaintLayerAppearance {
            blend_mode: 0,
            ..appearance
        };
        assert_eq!(
            paint_session_set_layer_appearance(h.core, h.session, group, &normal, &mut seq),
            PAINT_OK
        );
        let info = h.wait(seq);
        assert_eq!(info.last_error_status, PAINT_OK);
        assert_eq!(
            paint_session_set_layer_appearance(h.core, h.session, group, &appearance, &mut seq),
            PAINT_OK
        );
        let state = h.wait(seq);
        assert_eq!(state.last_error_status, PAINT_OK);
        assert_eq!(state.undo_depth, info.undo_depth + 1);
        let mut actual = dto!(PaintLayerAppearance);
        assert_eq!(
            paint_session_layer_appearance(
                h.core,
                h.session,
                state.publication,
                group,
                &mut actual
            ),
            PAINT_OK
        );
        assert_eq!(actual.blend_mode, 27);
        let state = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(
            paint_session_layer_appearance(
                h.core,
                h.session,
                state.publication,
                group,
                &mut actual
            ),
            PAINT_OK
        );
        assert_eq!(actual.blend_mode, 0);
        assert_eq!(
            paint_session_set_layer_appearance(h.core, h.session, 1, &appearance, &mut seq),
            PAINT_OK
        );
        let info = h.wait(seq);
        assert_eq!(info.last_error_status, PAINT_INVALID_ARGUMENT);
        assert_eq!(info.undo_depth, state.undo_depth);
    }
}

#[test]
fn layer_actions_copy_paste_duplicate_merge_validate_and_keep_fifo_history() {
    let h = Harness::new(64, 64);
    h.wait(h.stroke(16., 16.));
    let ids = [1];
    let mut sequence = 99;
    unsafe {
        assert_eq!(
            paint_session_layer_action(
                h.core,
                h.session,
                99,
                ids.as_ptr(),
                1,
                0,
                ptr::null(),
                0,
                &mut sequence
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        assert_eq!(
            paint_session_layer_action(
                h.core,
                h.session,
                PAINT_LAYER_ACTION_COPY,
                ids.as_ptr(),
                1,
                0,
                ptr::null(),
                0,
                &mut sequence
            ),
            PAINT_OK
        );
        let info = h.wait(sequence);
        assert_eq!(info.undo_depth, 1);
        assert_eq!(info.last_error_status, PAINT_OK);
        let mut size = 0;
        assert_eq!(
            paint_session_read_layer_clipboard(
                h.core,
                h.session,
                sequence,
                ptr::null_mut(),
                0,
                &mut size
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert!(size > 16);
        let mut bytes = vec![0; size as usize];
        assert_eq!(
            paint_session_read_layer_clipboard(
                h.core,
                h.session,
                sequence,
                bytes.as_mut_ptr(),
                size,
                &mut size
            ),
            PAINT_OK
        );
        let other = Harness::new(64, 64);
        assert_eq!(
            paint_session_layer_action(
                other.core,
                other.session,
                PAINT_LAYER_ACTION_PASTE,
                ptr::null(),
                0,
                1,
                bytes.as_ptr(),
                size,
                &mut sequence
            ),
            PAINT_OK
        );
        bytes.fill(0);
        let info = other.wait(sequence);
        assert_eq!(info.last_error_status, PAINT_OK);
        assert_eq!(info.layer_count, 2);
        assert_eq!(info.undo_depth, 1);
        let copied = [info.active_layer_id];
        assert_eq!(
            paint_session_layer_action(
                other.core,
                other.session,
                PAINT_LAYER_ACTION_DUPLICATE,
                copied.as_ptr(),
                1,
                0,
                ptr::null(),
                0,
                &mut sequence
            ),
            PAINT_OK
        );
        let info = other.wait(sequence);
        assert_eq!(info.layer_count, 3);
        assert_eq!(info.undo_depth, 2);
        let selected = [1, copied[0], info.active_layer_id];
        assert_eq!(
            paint_session_layer_action(
                other.core,
                other.session,
                PAINT_LAYER_ACTION_MERGE,
                selected.as_ptr(),
                3,
                0,
                ptr::null(),
                0,
                &mut sequence
            ),
            PAINT_OK
        );
        let info = other.wait(sequence);
        assert_eq!(info.layer_count, 1);
        assert_eq!(info.undo_depth, 3);
        assert_eq!(info.last_error_status, PAINT_OK);
        let undo = other.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        });
        assert_eq!(other.wait(undo).layer_count, 3);
        assert_eq!(
            paint_session_layer_action(
                other.core,
                other.session,
                PAINT_LAYER_ACTION_PASTE,
                ptr::null(),
                0,
                1,
                ptr::null(),
                1,
                &mut sequence
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
    }
}

#[test]
fn transform_abi_session_history_and_atomic_invalid_geometry() {
    let h = Harness::new(32, 32);
    let mut sequence = 0;
    h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_BEGIN_STROKE,
        stroke: PaintStrokeDesc {
            mode: PAINT_MODE_PAINT,
            radius: 4.,
            opacity: 1.,
            spacing: 0.15,
            linear_rgba: [1., 0., 0., 1.],
            ..dto!(PaintStrokeDesc)
        },
        point: PaintPoint {
            x: 12.,
            y: 12.,
            pressure: 1.,
            tool: PAINT_TOOL_PEN,
            ..dto!(PaintPoint)
        },
        ..dto!(PaintCommand)
    }));
    h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_END_STROKE,
        ..dto!(PaintCommand)
    }));
    unsafe {
        assert_eq!(
            paint_session_transform(
                h.core,
                h.session,
                PAINT_TRANSFORM_BEGIN,
                ptr::null(),
                &mut sequence
            ),
            PAINT_OK
        );
        let info = h.wait(sequence);
        assert_ne!(info.flags & PAINT_SESSION_TRANSFORM, 0);
        assert_eq!(info.undo_depth, 0);
        let mut g = dto!(PaintTransform);
        assert_eq!(
            paint_session_transform_info(h.core, h.session, info.publication, &mut g),
            PAINT_OK
        );
        let original = g;
        for i in 0..16 {
            g.points[i * 2] += 3.;
        }
        assert_eq!(
            paint_session_transform(h.core, h.session, PAINT_TRANSFORM_UPDATE, &g, &mut sequence),
            PAINT_OK
        );
        h.wait(sequence);
        assert_eq!(
            paint_session_transform(
                h.core,
                h.session,
                PAINT_TRANSFORM_CHECKPOINT,
                ptr::null(),
                &mut sequence
            ),
            PAINT_OK
        );
        assert_eq!(h.wait(sequence).undo_depth, 1);
        assert_eq!(
            paint_session_transform(
                h.core,
                h.session,
                PAINT_TRANSFORM_UNDO,
                ptr::null(),
                &mut sequence
            ),
            PAINT_OK
        );
        let info = h.wait(sequence);
        assert_eq!(info.redo_depth, 1);
        let mut restored = dto!(PaintTransform);
        assert_eq!(
            paint_session_transform_info(h.core, h.session, info.publication, &mut restored),
            PAINT_OK
        );
        assert_eq!(restored.points, original.points);
        g.points[0] = f64::NAN;
        assert_eq!(
            paint_session_transform(h.core, h.session, PAINT_TRANSFORM_UPDATE, &g, &mut sequence),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        assert_eq!(
            paint_session_transform(
                h.core,
                h.session,
                PAINT_TRANSFORM_CANCEL,
                ptr::null(),
                &mut sequence
            ),
            PAINT_OK
        );
        let info = h.wait(sequence);
        assert_eq!(info.flags & PAINT_SESSION_TRANSFORM, 0);
        assert_eq!(info.undo_depth, 1);
    }
}

#[path = "support/taper_api.rs"]
mod taper_api;
#[path = "support/wet_api.rs"]
mod wet_api;

#[path = "support/outline_api.rs"]
mod outline_api;

#[path = "support/wet_dual_api.rs"]
mod wet_dual_api;

#[path = "support/compute_api.rs"]
mod compute_api;

#[test]
fn full_image_flip_abi_pixels_single_history_and_descriptor_validation() {
    let h = Harness::new(33, 19);
    h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_BEGIN_STROKE,
        stroke: PaintStrokeDesc {
            mode: PAINT_MODE_PAINT,
            radius: 3.,
            opacity: 1.,
            spacing: 0.15,
            linear_rgba: [1., 0., 0., 0.5],
            ..dto!(PaintStrokeDesc)
        },
        point: PaintPoint {
            x: 7.,
            y: 6.,
            pressure: 1.,
            tool: PAINT_TOOL_MOUSE,
            ..dto!(PaintPoint)
        },
        ..dto!(PaintCommand)
    }));
    let info = h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_END_STROKE,
        ..dto!(PaintCommand)
    }));
    let original = h.pixels(0, h.frame(0, info.revision));
    unsafe {
        let mut sequence = 99;
        assert_eq!(
            paint_session_transform(
                h.core,
                h.session,
                PAINT_TRANSFORM_FLIP_IMAGE_HORIZONTAL,
                &dto!(PaintTransform),
                &mut sequence
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        assert_eq!(
            paint_session_transform(
                h.core,
                h.session,
                PAINT_TRANSFORM_FLIP_IMAGE_VERTICAL + 1,
                ptr::null(),
                &mut sequence
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(sequence, 0);
        for kind in [
            PAINT_TRANSFORM_FLIP_IMAGE_HORIZONTAL,
            PAINT_TRANSFORM_FLIP_IMAGE_VERTICAL,
        ] {
            assert_eq!(
                paint_session_transform(h.core, h.session, kind, ptr::null(), &mut sequence),
                PAINT_OK
            );
            let info = h.wait(sequence);
            assert_eq!(info.last_error_status, PAINT_OK);
            assert_eq!(info.undo_depth, 2);
            assert_eq!(info.flags & PAINT_SESSION_TRANSFORM, 0);
            let flipped = h.pixels(0, h.frame(0, info.revision));
            for y in 0..19 {
                for x in 0..33 {
                    let (sx, sy) = if kind == PAINT_TRANSFORM_FLIP_IMAGE_HORIZONTAL {
                        (32 - x, y)
                    } else {
                        (x, 18 - y)
                    };
                    assert_eq!(
                        &flipped[(y * 33 + x) * 4..(y * 33 + x + 1) * 4],
                        &original[(sy * 33 + sx) * 4..(sy * 33 + sx + 1) * 4]
                    );
                }
            }
            let info = h.wait(h.submit(PaintCommand {
                kind: PAINT_COMMAND_UNDO,
                ..dto!(PaintCommand)
            }));
            assert_eq!(h.pixels(0, h.frame(0, info.revision)), original);
            let info = h.wait(h.submit(PaintCommand {
                kind: PAINT_COMMAND_REDO,
                ..dto!(PaintCommand)
            }));
            assert_eq!(h.pixels(0, h.frame(0, info.revision)), flipped);
            h.wait(h.submit(PaintCommand {
                kind: PAINT_COMMAND_UNDO,
                ..dto!(PaintCommand)
            }));
        }
    }
}

#[test]
fn layer_moves_abi_copies_targets_and_final_delta_is_one_history() {
    let h = Harness::new(64, 64);
    h.wait(h.stroke(12., 12.));
    h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_ADD_LAYER,
        text: b"second".as_ptr(),
        text_length: 6,
        ..dto!(PaintCommand)
    }));
    let before = h.wait(h.stroke(32., 32.));
    unsafe {
        let mut seq = 99;
        let valid = [1u64, 2];
        let duplicate = [1u64, 1];
        let zero = [1u64, 0];
        for (ids, count, flags) in [
            (ptr::null(), 1, 0),
            (valid.as_ptr(), 0, 0),
            (duplicate.as_ptr(), 2, 0),
            (zero.as_ptr(), 2, 0),
            (valid.as_ptr(), 2, 2),
        ] {
            assert_eq!(
                paint_session_begin_layer_moves(h.core, h.session, ids, count, flags, &mut seq),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(seq, 0);
        }
        let mut ids = [1u64, 2];
        assert_eq!(
            paint_session_begin_layer_moves(h.core, h.session, ids.as_ptr(), 2, 1, &mut seq),
            PAINT_OK
        );
        ids.fill(9999);
        let begun = h.wait(seq);
        assert_ne!(begun.flags & PAINT_SESSION_LAYER_MOVE, 0);
        assert_eq!(begun.undo_depth, before.undo_depth);
        for (kind, x, y) in [
            (PAINT_LAYER_MOVE_UPDATE, 4, 3),
            (PAINT_LAYER_MOVE_COMMIT, 9, -2),
        ] {
            assert_eq!(
                paint_session_layer_move(h.core, h.session, kind, 0, x, y, &mut seq),
                PAINT_OK
            );
            let info = h.wait(seq);
            assert_eq!(info.last_error_status, PAINT_OK);
            assert_eq!(
                info.undo_depth,
                before.undo_depth + u32::from(kind == PAINT_LAYER_MOVE_COMMIT)
            );
            for id in [1, 2] {
                let mut a = dto!(PaintLayerAppearance);
                assert_eq!(
                    paint_session_layer_appearance(h.core, h.session, info.publication, id, &mut a),
                    PAINT_OK
                );
                assert_eq!((a.offset_x, a.offset_y), (x, y));
            }
        }
        let original = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        for id in [1, 2] {
            let mut a = dto!(PaintLayerAppearance);
            assert_eq!(
                paint_session_layer_appearance(h.core, h.session, original.publication, id, &mut a),
                PAINT_OK
            );
            assert_eq!(a.offset_x, 0);
        }
    }
}

#[test]
fn mask_link_abi_publication_history_movement_and_validation() {
    let h = Harness::new(64, 64);
    h.wait(h.stroke(20., 20.));
    let original = h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_ADD_MASK,
        layer_id: 1,
        ..dto!(PaintCommand)
    }));
    let mask = original.active_layer_id;
    unsafe {
        let query = |publication, id| {
            let mut linked = 99;
            let status =
                paint_session_layer_mask_linked(h.core, h.session, publication, id, &mut linked);
            (status, linked)
        };
        assert_eq!(query(original.publication, mask), (PAINT_OK, 1));
        assert_eq!(query(original.publication, 1), (PAINT_OK, 1));
        assert_eq!(query(original.publication, 9999), (PAINT_NOT_FOUND, 0));
        assert_eq!(
            paint_session_layer_mask_linked(
                h.core,
                h.session,
                original.publication,
                mask,
                ptr::null_mut()
            ),
            PAINT_INVALID_ARGUMENT
        );
        let mut seq = 99;
        let mut request = PaintGroupRequest {
            kind: PAINT_GROUP_MASK_LINK,
            layer_id: mask,
            parent_id: 2,
            struct_size: size_of::<PaintGroupRequest>() as u32,
            name: ptr::null(),
            name_length: 0,
        };
        assert_eq!(
            paint_session_group(h.core, h.session, &request, &mut seq),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(seq, 0);
        request.parent_id = 0;
        request.name = b"x".as_ptr();
        request.name_length = 1;
        assert_eq!(
            paint_session_group(h.core, h.session, &request, &mut seq),
            PAINT_INVALID_ARGUMENT
        );
        request.name = ptr::null();
        request.name_length = 0;
        assert_eq!(
            paint_session_group(h.core, h.session, &request, &mut seq),
            PAINT_OK
        );
        let info = h.wait(seq);
        assert_eq!(info.last_error_status, PAINT_OK);
        assert_eq!(info.undo_depth, original.undo_depth + 1);
        assert_ne!(info.flags & PAINT_SESSION_MODIFIED, 0);
        assert_eq!(query(info.publication, mask), (PAINT_OK, 0));
        assert_eq!(query(original.publication, mask), (PAINT_BUSY, 0));
        assert_eq!(
            paint_session_group(h.core, h.session, &request, &mut seq),
            PAINT_OK
        );
        let info = h.wait(seq);
        assert_eq!(info.undo_depth, original.undo_depth + 1);
        assert_eq!(
            paint_session_layer_move(h.core, h.session, PAINT_LAYER_MOVE_BEGIN, 1, 0, 0, &mut seq),
            PAINT_OK
        );
        h.wait(seq);
        assert_eq!(
            paint_session_layer_move(
                h.core,
                h.session,
                PAINT_LAYER_MOVE_COMMIT,
                0,
                8,
                3,
                &mut seq
            ),
            PAINT_OK
        );
        let moved = h.wait(seq);
        for (id, x) in [(1, 8), (mask, 0)] {
            let mut a = dto!(PaintLayerAppearance);
            assert_eq!(
                paint_session_layer_appearance(h.core, h.session, moved.publication, id, &mut a),
                PAINT_OK
            );
            assert_eq!(a.offset_x, x);
        }
        h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        let undone = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_UNDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(query(undone.publication, mask), (PAINT_OK, 1));
        let redone = h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_REDO,
            ..dto!(PaintCommand)
        }));
        assert_eq!(query(redone.publication, mask), (PAINT_OK, 0));
        request.layer_id = 1;
        assert_eq!(
            paint_session_group(h.core, h.session, &request, &mut seq),
            PAINT_OK
        );
        let failed = h.wait(seq);
        assert_eq!(failed.last_error_status, PAINT_INVALID_ARGUMENT);
        assert_eq!(failed.undo_depth, redone.undo_depth);
    }
}
