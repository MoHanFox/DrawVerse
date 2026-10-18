use super::*;
use paint_core::{
    BrushCatalog, BrushSettings, BrushTip, Document, InputPoint, Pixel, PressureControl,
    PressureCurve, StrokeTaper, TaperEnd, WetMix, WetSettings,
};
use paint_task::CancellationToken;

fn settings(enabled: bool, wet: bool) -> BrushSettings {
    let c = PressureControl::new(
        PressureCurve::new_smooth(vec![(0., 0.), (0.4, 0.2), (1., 1.)]).unwrap(),
        0.2,
    )
    .unwrap();
    BrushSettings {
        diameter: 8.,
        spacing: 0.25,
        opacity: 0.4,
        taper: StrokeTaper::new(
            enabled,
            TaperEnd::new(8., c.clone(), c.clone()).unwrap(),
            TaperEnd::new(12., c.clone(), c).unwrap(),
        ),
        wet: WetSettings::new(
            wet,
            false,
            false,
            true,
            WetMix::new(0.6, 0.3, 0.4, 0.6, 0.8, 0.1).unwrap(),
        ),
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn prepared(library: *mut PaintBrushLibrary, s: &BrushSettings) -> u64 {
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", s.clone()).unwrap();
    let json = paint_io::brush_state::encode(&catalog).unwrap();
    let mut seq = 99;
    unsafe {
        assert_eq!(
            paint_brush_library_prepare_state_taper(
                library,
                json.as_ptr(),
                json.len() as u64,
                &mut seq
            ),
            PAINT_OK
        );
    }
    let bytes = wait(|| unsafe {
        let mut required = 99;
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
    assert_eq!(bytes, json);
    seq
}
#[test]
fn taper_public_preparation_capture_two_strokes_pixels_history_and_wet_carrier() {
    for enabled in [false, true] {
        for wet in [false, true] {
            let h = Harness::new(64, 64);
            let s = settings(enabled, wet);
            let mut reference = Document::new(64, 64).unwrap();
            let mut def = s.definition();
            def.brush.color = Pixel::from_straight([0., 0., 1., 1.]).unwrap();
            def.brush.background_color = Pixel::WHITE;
            unsafe {
                let mut library = ptr::null_mut();
                assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
                let capture = prepared(library, &s);
                let mut tool = ptr::null_mut();
                let mut retained = None;
                if wet {
                    assert_eq!(paint_wet_tool_create(&mut tool), PAINT_OK);
                    let mut seq = 0;
                    assert_eq!(
                        paint_session_bind_wet_tool(h.core, h.session, tool, &mut seq),
                        PAINT_OK
                    );
                    h.wait(seq);
                }
                for i in 0..2 {
                    h.wait(super::wet_api::begin(&h, library, capture, 16.5));
                    if wet {
                        reference
                            .begin_configured_wet_brush_stroke(
                                def.clone(),
                                InputPoint::new(16.5, 32.5, 1.),
                                retained.as_ref(),
                            )
                            .unwrap();
                    } else {
                        reference
                            .begin_brush_stroke(def.clone(), InputPoint::new(16.5, 32.5, 1.))
                            .unwrap();
                    }
                    if i == 0 {
                        let mut changed = s.clone();
                        changed.taper = StrokeTaper::default();
                        prepared(library, &changed);
                    } else {
                        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
                    }
                    h.wait(h.submit(PaintCommand {
                        kind: PAINT_COMMAND_STROKE_TO,
                        point: PaintPoint {
                            x: 48.5,
                            y: 32.5,
                            pressure: 1.,
                            tool: PAINT_TOOL_MOUSE,
                            ..dto!(PaintPoint)
                        },
                        ..dto!(PaintCommand)
                    }));
                    reference
                        .stroke_to(InputPoint::new(48.5, 32.5, 1.))
                        .unwrap();
                    let info = h.wait(h.submit(PaintCommand {
                        kind: PAINT_COMMAND_END_STROKE,
                        ..dto!(PaintCommand)
                    }));
                    assert_eq!(info.last_error_status, PAINT_OK);
                    assert_eq!(info.undo_depth, i + 1);
                    if wet {
                        retained = Some(reference.end_wet_brush_stroke().unwrap().1);
                        let ink = super::wet_api::ink(tool);
                        assert_eq!(ink.carrier_revision, retained.as_ref().unwrap().revision());
                        assert_eq!(
                            ink.center_linear_premul_rgba,
                            retained.as_ref().unwrap().sample(0.5, 0.5).components()
                        );
                    } else {
                        reference.end_stroke().unwrap();
                    }
                    assert_eq!(
                        super::wet_api::copied(&h),
                        reference.copy_layers(&[1]).unwrap()
                    );
                }
                let after = super::wet_api::copied(&h);
                h.wait(h.submit(PaintCommand {
                    kind: PAINT_COMMAND_UNDO,
                    ..dto!(PaintCommand)
                }));
                h.wait(h.submit(PaintCommand {
                    kind: PAINT_COMMAND_REDO,
                    ..dto!(PaintCommand)
                }));
                assert_eq!(super::wet_api::copied(&h), after);
                if wet {
                    assert_eq!(paint_wet_tool_destroy(&mut tool), PAINT_OK);
                }
            }
        }
    }
}
#[test]
fn taper_public_version_validation_and_failed_draft_leave_current_state() {
    let h = Harness::new(32, 32);
    unsafe {
        let mut version = dto!(PaintVersion);
        assert_eq!(paint_core_version(&mut version), PAINT_OK);
        assert!(version.minor >= 51);
        let mut caps = dto!(PaintCapabilities);
        assert_eq!(paint_core_capabilities(h.core, &mut caps), PAINT_OK);
        assert_ne!(caps.features & PAINT_FEATURE_STROKE_TAPER, 0);
        let mut seq = 99;
        assert_eq!(
            paint_brush_library_prepare_state_taper(ptr::null_mut(), ptr::null(), 0, &mut seq),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(seq, 0);
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let good = prepared(library, &settings(true, false));
        assert_eq!(
            paint_brush_library_prepare_state_taper(library, b"{}".as_ptr(), 2, &mut seq),
            PAINT_OK
        );
        wait(|| {
            let mut required = 99;
            let status =
                paint_brush_library_prepared_state(library, seq, ptr::null_mut(), 0, &mut required);
            if status == PAINT_BUSY {
                return None;
            }
            assert_ne!(status, PAINT_OK);
            assert_ne!(status, PAINT_BUFFER_TOO_SMALL);
            assert_eq!(required, 0);
            Some(())
        });
        let mut good_len = 0;
        assert_eq!(
            paint_brush_library_prepared_state(library, good, ptr::null_mut(), 0, &mut good_len),
            PAINT_BUFFER_TOO_SMALL
        );
        let mut bytes = vec![0; good_len as usize];
        assert_eq!(
            paint_brush_library_current_state(
                library,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut good_len
            ),
            PAINT_OK
        );
        let c = paint_io::brush_state::decode(&bytes, paint_core::MAX_CATALOG_MASK_BYTES).unwrap();
        assert_eq!(c.selected().settings().taper, settings(true, false).taper);
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
}
#[test]
fn taper_public_image_package_roundtrip_and_capacity_checks_do_not_install_resources() {
    use paint_io::{brush_resource::ResourceStore, vbr::Package};
    use std::collections::BTreeMap;
    for enabled in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("source");
        let target = tmp.path().join("target");
        let path = tmp.path().join("taper.vbr");
        let token = CancellationToken::default();
        let mask =
            paint_core::BrushMask::new(3, 3, vec![255, 0, 255, 255, 255, 255, 0, 255, 0]).unwrap();
        let png = paint_io::brush_resource::encode_mask(&mask).unwrap();
        let id = paint_io::brush_resource::hash(&png);
        let store = ResourceStore::open(root.clone(), paint_core::MAX_CATALOG_MASK_BYTES).unwrap();
        store.put_verified(&id, &png, &token).unwrap();
        let fixture = format!(
            "{{\"packageId\":\"12345678-1234-4234-8234-123456789abc\",\"brushes\":[{}]}}",
            include_str!("../../../../../contracts/brush/wet/fixtures/taper-valid.json")
        );
        let mut p = Package::from_json(fixture.as_bytes()).unwrap();
        let preset = &mut p.brushes[0];
        preset
            .required_capabilities
            .retain(|c| c != "procedural-tip");
        preset.required_capabilities.push("image-tip".into());
        preset.tip_resource = Some(id.clone());
        preset.stroke_taper = Some(paint_io::brush_taper::TaperParameters::from_core(
            &settings(enabled, false).taper,
        ));
        let json = p.to_json().unwrap();
        let pathbytes = path.to_str().unwrap().as_bytes();
        let rootbytes = root.to_str().unwrap().as_bytes();
        let targetbytes = target.to_str().unwrap().as_bytes();
        unsafe {
            let mut a = ptr::null_mut();
            let mut b = ptr::null_mut();
            assert_eq!(paint_brush_library_create(0, &mut a), PAINT_OK);
            assert_eq!(paint_brush_library_create(0, &mut b), PAINT_OK);
            assert_eq!(
                paint_brush_library_bind_resources(a, rootbytes.as_ptr(), rootbytes.len() as u64),
                PAINT_OK
            );
            assert_eq!(
                paint_brush_library_bind_resources(
                    b,
                    targetbytes.as_ptr(),
                    targetbytes.len() as u64
                ),
                PAINT_OK
            );
            assert_eq!(
                paint_brush_library_package_write_taper(
                    a,
                    pathbytes.as_ptr(),
                    pathbytes.len() as u64,
                    json.as_ptr(),
                    json.len() as u64
                ),
                PAINT_OK
            );
            let before = std::fs::read(&path).unwrap();
            let mut required = 99;
            assert_eq!(
                paint_brush_library_package_read_taper(
                    b,
                    pathbytes.as_ptr(),
                    pathbytes.len() as u64,
                    ptr::null_mut(),
                    0,
                    &mut required
                ),
                PAINT_BUFFER_TOO_SMALL
            );
            assert!(!target.join(format!("{id}.png")).exists());
            let mut data = vec![0; required as usize];
            assert_eq!(
                paint_brush_library_package_read_taper(
                    b,
                    pathbytes.as_ptr(),
                    pathbytes.len() as u64,
                    ptr::null_mut(),
                    required,
                    &mut required
                ),
                PAINT_INVALID_ARGUMENT
            );
            assert!(!target.join(format!("{id}.png")).exists());
            assert_eq!(
                paint_brush_library_package_read_taper(
                    b,
                    pathbytes.as_ptr(),
                    pathbytes.len() as u64,
                    data.as_mut_ptr(),
                    data.len() as u64,
                    &mut required
                ),
                PAINT_OK
            );
            assert_eq!(data, json);
            assert!(target.join(format!("{id}.png")).exists());
            let invalid = b"{}";
            assert_ne!(
                paint_brush_library_package_write_taper(
                    a,
                    pathbytes.as_ptr(),
                    pathbytes.len() as u64,
                    invalid.as_ptr(),
                    invalid.len() as u64
                ),
                PAINT_OK
            );
            assert_eq!(std::fs::read(&path).unwrap(), before);
            let installed =
                ResourceStore::open(target.clone(), paint_core::MAX_CATALOG_MASK_BYTES).unwrap();
            let mut s = settings(enabled, false);
            s.mask = Some(installed.load(&id, &token).unwrap().mask);
            let captured = prepared(b, &s);
            prepared(b, &settings(false, false));
            let old = wait(|| {
                let mut required = 0;
                let status = paint_brush_library_prepared_state(
                    b,
                    captured,
                    ptr::null_mut(),
                    0,
                    &mut required,
                );
                if status == PAINT_BUSY {
                    return None;
                }
                assert_eq!(status, PAINT_BUFFER_TOO_SMALL);
                let mut bytes = vec![0; required as usize];
                assert_eq!(
                    paint_brush_library_prepared_state(
                        b,
                        captured,
                        bytes.as_mut_ptr(),
                        bytes.len() as u64,
                        &mut required
                    ),
                    PAINT_OK
                );
                Some(bytes)
            });
            let restored = paint_io::brush_state::decode_with_resources(
                &old,
                paint_core::MAX_CATALOG_MASK_BYTES,
                &installed,
                &token,
            )
            .unwrap();
            assert_eq!(restored.selected().settings().taper, s.taper);
            assert_eq!(
                restored
                    .selected()
                    .settings()
                    .mask
                    .as_ref()
                    .unwrap()
                    .resource_id(),
                Some(id.as_str())
            );
            assert_eq!(paint_brush_library_destroy(&mut a), PAINT_OK);
            assert_eq!(paint_brush_library_destroy(&mut b), PAINT_OK);
        }
        let bundle = paint_io::vbr_image::load(&path, &token).unwrap();
        assert_eq!(
            bundle.definition(0).unwrap().taper,
            settings(enabled, false).taper
        );
        assert_eq!(
            bundle.assets().keys().cloned().collect::<Vec<_>>(),
            BTreeMap::from([(id, ())])
                .keys()
                .cloned()
                .collect::<Vec<_>>()
        );
    }
}
