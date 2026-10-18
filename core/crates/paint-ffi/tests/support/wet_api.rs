use super::*;
use paint_core::{
    BrushCatalog, BrushSettings, BrushTip, Document, InputPoint, Pixel, WetCarrier, WetMix,
    WetSettings,
};
fn settings(enabled: bool) -> BrushSettings {
    BrushSettings {
        diameter: 8.,
        spacing: 0.25,
        opacity: 0.4,
        wet: WetSettings::new(
            enabled,
            false,
            false,
            false,
            WetMix::new(0.6, 0.2, 0.2, 0.5, 0.8, 0.05).unwrap(),
        ),
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn prepare(library: *mut PaintBrushLibrary, s: BrushSettings) -> u64 {
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", s.clone()).unwrap();
    let json = paint_io::brush_state::encode(&catalog).unwrap();
    let mut seq = 0;
    unsafe {
        assert_eq!(
            paint_brush_library_prepare_state_wet(
                library,
                json.as_ptr(),
                json.len() as u64,
                &mut seq
            ),
            PAINT_OK
        );
    }
    let bytes = wait(|| unsafe {
        let mut required = 0;
        let status =
            paint_brush_library_prepared_state(library, seq, ptr::null_mut(), 0, &mut required);
        if status == PAINT_BUSY {
            return None;
        }
        assert_eq!(status, PAINT_BUFFER_TOO_SMALL);
        let mut bytes = vec![0; required as usize];
        assert_eq!(
            paint_brush_library_prepared_state(
                library,
                seq,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        Some(bytes)
    });
    assert_eq!(
        paint_io::brush_state::decode(&bytes, paint_core::MAX_CATALOG_MASK_BYTES)
            .unwrap()
            .selected()
            .settings()
            .wet,
        s.wet
    );
    seq
}
pub(super) fn ink(tool: *mut PaintWetTool) -> PaintWetToolInfo {
    let mut info = dto!(PaintWetToolInfo);
    unsafe {
        assert_eq!(paint_wet_tool_info(tool, &mut info), PAINT_OK);
    }
    info
}
pub(super) fn copied(h: &Harness) -> Vec<u8> {
    unsafe {
        let mut seq = 0;
        let id = 1;
        assert_eq!(
            paint_session_layer_action(
                h.core,
                h.session,
                PAINT_LAYER_ACTION_COPY,
                &id,
                1,
                0,
                ptr::null(),
                0,
                &mut seq
            ),
            PAINT_OK
        );
        h.wait(seq);
        let mut required = 0;
        assert_eq!(
            paint_session_read_layer_clipboard(
                h.core,
                h.session,
                seq,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        let mut bytes = vec![0; required as usize];
        assert_eq!(
            paint_session_read_layer_clipboard(
                h.core,
                h.session,
                seq,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        bytes
    }
}
pub(super) fn begin(h: &Harness, library: *mut PaintBrushLibrary, preparation: u64, x: f64) -> u64 {
    let mut tool = PaintBrushToolDesc {
        operation: PAINT_BRUSH_PAINT,
        radius: 4.,
        strength: 1.,
        linear_rgba: [0., 0., 1., 1.],
        ..dto!(PaintBrushToolDesc)
    };
    let background = PaintBrushBackgroundDesc {
        linear_rgba: [1.; 4],
        ..dto!(PaintBrushBackgroundDesc)
    };
    let blend = PaintBrushBlendDesc {
        mode: 0,
        ..dto!(PaintBrushBlendDesc)
    };
    let point = PaintPoint {
        x,
        y: 32.5,
        pressure: 1.,
        tool: PAINT_TOOL_MOUSE,
        ..dto!(PaintPoint)
    };
    let mut seq = 0;
    unsafe {
        assert_eq!(
            paint_session_begin_prepared_preset_with_blend(
                h.core,
                h.session,
                library,
                preparation,
                b"round-pressure".as_ptr(),
                14,
                &tool,
                &background,
                &blend,
                &point,
                &mut seq
            ),
            PAINT_OK
        );
    }
    tool.linear_rgba = [1.; 4];
    assert_eq!(tool.linear_rgba, [1.; 4]);
    seq
}
#[test]
fn wet_public_handles_layout_validation_and_manual_capture() {
    assert_eq!(size_of::<PaintWetLoadDesc>(), 32);
    assert_eq!(size_of::<PaintWetToolInfo>(), 40);
    assert_eq!(offset_of!(PaintWetToolInfo, center_linear_premul_rgba), 24);
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let s = settings(true);
        let seq = prepare(library, s.clone());
        let mut tool = ptr::null_mut();
        assert_eq!(paint_wet_tool_create(&mut tool), PAINT_OK);
        assert_eq!(ink(tool).flags, 0);
        assert_eq!(paint_wet_tool_clean(tool), PAINT_OK);
        let mut load = PaintWetLoadDesc {
            radius: 4.,
            linear_rgba: [1., 0., 0., 0.8],
            ..dto!(PaintWetLoadDesc)
        };
        assert_eq!(
            paint_wet_tool_reload_prepared(
                tool,
                library,
                seq,
                b"round-pressure".as_ptr(),
                14,
                &load
            ),
            PAINT_OK
        );
        assert_eq!(ink(tool).edge, 8);
        assert_eq!(ink(tool).center_linear_premul_rgba, [0.4, 0., 0., 0.4]);
        let original = ink(tool);
        for radius in [f32::NAN, 0., 1000.25] {
            load.radius = radius;
            assert_eq!(
                paint_wet_tool_reload_prepared(
                    tool,
                    library,
                    seq,
                    b"round-pressure".as_ptr(),
                    14,
                    &load
                ),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(ink(tool).carrier_revision, original.carrier_revision);
        }
        load.radius = 4.;
        load.reserved = 1;
        assert_eq!(
            paint_wet_tool_reload_prepared(
                tool,
                library,
                seq,
                b"round-pressure".as_ptr(),
                14,
                &load
            ),
            PAINT_INVALID_ARGUMENT
        );
        load.reserved = 0;
        load.struct_size = 31;
        assert_eq!(
            paint_wet_tool_reload_prepared(
                tool,
                library,
                seq,
                b"round-pressure".as_ptr(),
                14,
                &load
            ),
            PAINT_INVALID_ARGUMENT
        );
        load.struct_size = 32;
        let mut changed = s;
        changed.wet = WetSettings::new(
            true,
            false,
            false,
            false,
            WetMix::new(0., 0., 0., 0.1, 1., 0.).unwrap(),
        );
        let newseq = prepare(library, changed);
        assert_eq!(
            paint_wet_tool_reload_prepared(
                tool,
                library,
                seq,
                b"round-pressure".as_ptr(),
                14,
                &load
            ),
            PAINT_OK
        );
        assert_eq!(ink(tool).center_linear_premul_rgba[3], 0.4);
        assert_eq!(
            paint_wet_tool_reload_prepared(
                tool,
                library,
                newseq,
                b"round-pressure".as_ptr(),
                14,
                &load
            ),
            PAINT_OK
        );
        assert!((ink(tool).center_linear_premul_rgba[3] - 0.08).abs() < 1e-7);
        let mut pose = ptr::null_mut();
        assert_eq!(paint_brush_pose_create(&mut pose), PAINT_OK);
        let mut out = original;
        assert_eq!(
            paint_wet_tool_info(pose.cast(), &mut out),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(out.flags, 0);
        assert_eq!(out.edge, 0);
        assert_eq!(out.center_linear_premul_rgba, [0.; 4]);
        assert_eq!(paint_brush_pose_destroy(&mut pose), PAINT_OK);
        assert_eq!(paint_wet_tool_clean(tool), PAINT_OK);
        assert_eq!(ink(tool).center_linear_premul_rgba, [0.; 4]);
        let stale = tool;
        assert_eq!(paint_wet_tool_destroy(&mut tool), PAINT_OK);
        assert!(tool.is_null());
        assert_eq!(paint_wet_tool_info(stale, &mut out), PAINT_INVALID_HANDLE);
        assert_eq!(out.flags, 0);
        assert_eq!(paint_wet_tool_clean(stale), PAINT_INVALID_HANDLE);
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
}
#[test]
fn wet_public_binding_real_pixels_busy_cancel_and_destroyed_tokens_keep_captures() {
    let a = Harness::new(64, 64);
    let b = Harness::new(64, 64);
    unsafe {
        let mut version = dto!(PaintVersion);
        assert_eq!(paint_core_version(&mut version), PAINT_OK);
        assert!(version.minor >= 50);
        let mut capabilities = dto!(PaintCapabilities);
        assert_eq!(paint_core_capabilities(a.core, &mut capabilities), PAINT_OK);
        assert_ne!(capabilities.features & PAINT_FEATURE_WET_BRUSH, 0);
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let s = settings(true);
        let preparation = prepare(library, s.clone());
        let mut tool = ptr::null_mut();
        assert_eq!(paint_wet_tool_create(&mut tool), PAINT_OK);
        let load = PaintWetLoadDesc {
            radius: 4.,
            linear_rgba: [1., 0., 0., 1.],
            ..dto!(PaintWetLoadDesc)
        };
        assert_eq!(
            paint_wet_tool_reload_prepared(
                tool,
                library,
                preparation,
                b"round-pressure".as_ptr(),
                14,
                &load
            ),
            PAINT_OK
        );
        let mut seq = 0;
        for h in [&a, &b] {
            assert_eq!(
                paint_session_bind_wet_tool(h.core, h.session, tool, &mut seq),
                PAINT_OK
            );
            assert_eq!(h.wait(seq).last_error_status, PAINT_OK);
        }
        assert_eq!(
            paint_session_bind_wet_tool(b.core, a.session, tool, &mut seq),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(seq, 0);
        let mut reference = Document::new(64, 64).unwrap();
        let mut retained = WetCarrier::new(
            s.wet.mix(),
            8.,
            Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
        )
        .unwrap();
        let mut definition = s.definition();
        definition.brush.color = Pixel::from_straight([0., 0., 1., 1.]).unwrap();
        definition.brush.background_color = Pixel::WHITE;
        for (i, h) in [&a, &b].into_iter().enumerate() {
            let started = begin(h, library, preparation, 16.5);
            h.wait(started);
            reference
                .begin_configured_wet_brush_stroke(
                    definition.clone(),
                    InputPoint::new(16.5, 32.5, 1.),
                    Some(&retained),
                )
                .unwrap();
            let mut status = dto!(PaintWetToolInfo);
            assert_eq!(paint_wet_tool_info(tool, &mut status), PAINT_BUSY);
            assert_eq!(status.flags, 0);
            assert_eq!(status.edge, 0);
            assert_eq!(paint_wet_tool_clean(tool), PAINT_BUSY);
            assert_eq!(
                paint_wet_tool_reload_prepared(
                    tool,
                    library,
                    preparation,
                    b"round-pressure".as_ptr(),
                    14,
                    &load
                ),
                PAINT_BUSY
            );
            assert_eq!(
                paint_session_bind_wet_tool(h.core, h.session, tool, &mut seq),
                PAINT_OK
            );
            assert_eq!(h.wait(seq).last_error_status, PAINT_BUSY);
            assert_eq!(h.info().stroke_active, 1);
            if i == 1 {
                let stale = tool;
                assert_eq!(paint_wet_tool_destroy(&mut tool), PAINT_OK);
                assert_eq!(
                    paint_wet_tool_info(stale, &mut status),
                    PAINT_INVALID_HANDLE
                );
                assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
            }
            let point = PaintPoint {
                x: 48.5,
                y: 32.5,
                pressure: 1.,
                tool: PAINT_TOOL_MOUSE,
                ..dto!(PaintPoint)
            };
            h.wait(h.submit(PaintCommand {
                kind: PAINT_COMMAND_STROKE_TO,
                point,
                ..dto!(PaintCommand)
            }));
            reference
                .stroke_to(InputPoint::new(48.5, 32.5, 1.))
                .unwrap();
            h.wait(h.submit(PaintCommand {
                kind: PAINT_COMMAND_END_STROKE,
                ..dto!(PaintCommand)
            }));
            retained = reference.end_wet_brush_stroke().unwrap().1;
            assert_eq!(copied(h), reference.copy_layers(&[1]).unwrap());
            assert_eq!(h.info().undo_depth, 1);
            if i == 0 {
                assert_eq!(
                    ink(tool).center_linear_premul_rgba,
                    retained.sample(0.5, 0.5).components()
                );
                let before = copied(h);
                let carrier = ink(tool);
                h.wait(begin(h, library, preparation, 32.5));
                h.wait(h.submit(PaintCommand {
                    kind: PAINT_COMMAND_CANCEL_STROKE,
                    ..dto!(PaintCommand)
                }));
                assert_eq!(copied(h), before);
                assert_eq!(ink(tool).carrier_revision, carrier.carrier_revision);
                reference = Document::new(64, 64).unwrap();
            }
            let after = copied(h);
            h.wait(h.submit(PaintCommand {
                kind: PAINT_COMMAND_UNDO,
                ..dto!(PaintCommand)
            }));
            h.wait(h.submit(PaintCommand {
                kind: PAINT_COMMAND_REDO,
                ..dto!(PaintCommand)
            }));
            assert_eq!(copied(h), after);
        }
    }
}
#[test]
fn wet_public_package_opt_in_preserves_closed_parameters_and_legacy_rejects() {
    use paint_io::vbr::Package;
    use paint_task::CancellationToken;
    use std::collections::BTreeMap;
    let fixture = format!(
        "{{\"packageId\":\"12345678-1234-4234-8234-123456789abc\",\"brushes\":[{}]}}",
        include_str!("../../../../../contracts/brush/wet/fixtures/vbr-wet-valid.json")
    );
    let mut p = Package::from_json(fixture.as_bytes()).unwrap();
    p.brushes[0].wet_mix = Some(paint_io::brush_wet::WetParameters::from_core(
        settings(false).wet,
    ));
    let json = p.to_json().unwrap();
    let token = CancellationToken::default();
    let bundle = paint_io::vbr_image::ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("wet.vbr");
    let pathbytes = path.to_str().unwrap().as_bytes();
    let resources = temp.path().join("resources");
    let root = resources.to_str().unwrap().as_bytes();
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        assert_eq!(
            paint_brush_library_bind_resources(library, root.as_ptr(), root.len() as u64),
            PAINT_OK
        );
        assert_eq!(
            paint_brush_library_package_write_wet(
                library,
                pathbytes.as_ptr(),
                pathbytes.len() as u64,
                json.as_ptr(),
                json.len() as u64
            ),
            PAINT_OK
        );
        let before = std::fs::read(&path).unwrap();
        assert_eq!(
            paint_brush_library_package_write(
                library,
                pathbytes.as_ptr(),
                pathbytes.len() as u64,
                json.as_ptr(),
                json.len() as u64
            ),
            PAINT_UNSUPPORTED
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let mut required = 99;
        assert_eq!(
            paint_brush_library_package_read(
                library,
                pathbytes.as_ptr(),
                pathbytes.len() as u64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_UNSUPPORTED
        );
        assert_eq!(required, 0);
        assert_eq!(
            paint_brush_library_package_read_wet(
                library,
                pathbytes.as_ptr(),
                pathbytes.len() as u64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        let mut bytes = vec![0; required as usize];
        assert_eq!(
            paint_brush_library_package_read_wet(
                library,
                pathbytes.as_ptr(),
                pathbytes.len() as u64,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        assert_eq!(
            Package::from_json(&bytes).unwrap().brushes[0]
                .wet_mix
                .unwrap()
                .core(),
            settings(false).wet
        );
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
    assert_eq!(
        paint_io::vbr_image::load(&path, &token)
            .unwrap()
            .definition(0)
            .unwrap()
            .wet,
        bundle.definition(0).unwrap().wet
    );
}
