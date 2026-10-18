use super::*;
use paint_core::{
    BrushCatalog, BrushSettings, BrushTip, Document, InputPoint, Pixel, PressureControl,
    PressureCurve, StrokeTaper, TaperEnd, WetMix, WetSettings,
};
use paint_task::CancellationToken;

fn grouped_state(s: &BrushSettings) -> Vec<u8> {
    use paint_core::BrushCatalogItem;
    let mut c = BrushCatalog::default();
    c.edit("round-pressure", s.clone()).unwrap();
    let outer = "12345678-1234-4234-8234-000000000100";
    let inner = "12345678-1234-4234-8234-000000000101";
    let empty = "12345678-1234-4234-8234-000000000102";
    c.create_group(outer.into(), "same".into(), None, 0)
        .unwrap();
    c.create_group(inner.into(), "same".into(), Some(outer), 0)
        .unwrap();
    c.create_group(empty.into(), "empty".into(), Some(outer), 1)
        .unwrap();
    c.move_catalog_item(
        BrushCatalogItem::Preset("round-pressure".into()),
        Some(inner),
        0,
    )
    .unwrap();
    paint_io::brush_state::encode(&c).unwrap()
}
fn read_group_preparation(library: *mut PaintBrushLibrary, sequence: u64) -> Vec<u8> {
    wait(|| unsafe {
        let mut required = 99;
        let status = paint_brush_library_prepared_state(
            library,
            sequence,
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
                library,
                sequence,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        Some(bytes)
    })
}
#[test]
fn groups_public_state20_exact_capture_two_strokes_and_history() {
    for wet in [false, true] {
        let h = Harness::new(64, 64);
        let s = settings(true, wet);
        let json = grouped_state(&s);
        let mut reference = Document::new(64, 64).unwrap();
        let mut def = s.definition();
        def.brush.color = Pixel::from_straight([0., 0., 1., 1.]).unwrap();
        def.brush.background_color = Pixel::WHITE;
        unsafe {
            let mut library = ptr::null_mut();
            assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
            let mut version = dto!(PaintVersion);
            assert_eq!(paint_core_version(&mut version), PAINT_OK);
            assert_eq!(version.minor, PAINT_ABI_MINOR);
            assert!(version.minor >= 54);
            let mut caps = dto!(PaintCapabilities);
            assert_eq!(paint_core_capabilities(h.core, &mut caps), PAINT_OK);
            assert_ne!(caps.features & PAINT_FEATURE_BRUSH_GROUP_STATE, 0);
            assert_ne!(caps.features & PAINT_FEATURE_DUAL_COLOR_INTERLEAVED, 0);
            let mut capture = 99;
            assert_eq!(
                paint_brush_library_prepare_state_groups(
                    library,
                    json.as_ptr(),
                    json.len() as u64,
                    &mut capture
                ),
                PAINT_OK
            );
            assert_eq!(read_group_preparation(library, capture), json);
            // Adopt a different flat state while retaining the exact grouped ticket for drawing.
            prepared(library, &settings(false, false));
            assert_eq!(read_group_preparation(library, capture), json);
            let mut tool = ptr::null_mut();
            if wet {
                assert_eq!(paint_wet_tool_create(&mut tool), PAINT_OK);
                let mut seq = 0;
                assert_eq!(
                    paint_session_bind_wet_tool(h.core, h.session, tool, &mut seq),
                    PAINT_OK
                );
                h.wait(seq);
            }
            let mut retained = None;
            for i in 0..2 {
                h.wait(super::wet_api::begin(&h, library, capture, 16.5));
                if wet {
                    reference
                        .begin_configured_dual_wet_brush_stroke(
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
                h.wait(h.submit(PaintCommand {
                    kind: PAINT_COMMAND_STROKE_TO,
                    point: PaintPoint {
                        x: 40.5,
                        y: 32.5,
                        pressure: 0.5,
                        tool: PAINT_TOOL_MOUSE,
                        ..dto!(PaintPoint)
                    },
                    ..dto!(PaintCommand)
                }));
                reference
                    .stroke_to(InputPoint::new(40.5, 32.5, 0.5))
                    .unwrap();
                let info = h.wait(h.submit(PaintCommand {
                    kind: PAINT_COMMAND_END_STROKE,
                    ..dto!(PaintCommand)
                }));
                assert_eq!(info.last_error_status, PAINT_OK);
                assert_eq!(info.undo_depth, i + 1);
                if wet {
                    retained = Some(reference.end_wet_ink_stroke().unwrap().1);
                    let info = dual_info(tool);
                    let ink = retained.as_ref().unwrap();
                    assert_eq!(info.primary_revision, ink.primary().revision());
                    assert_eq!(
                        info.primary_center_linear_premul_rgba,
                        ink.primary().sample(0.5, 0.5).components()
                    );
                    assert_eq!(info.secondary_revision, ink.secondary().unwrap().revision());
                    assert_eq!(
                        info.secondary_center_linear_premul_rgba,
                        ink.secondary().unwrap().sample(0.5, 0.5).components()
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
            assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
        }
    }
}

#[test]
fn groups_public_closed_state_and_invalid_arguments_preserve_output_contracts() {
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let json = grouped_state(&settings(false, false));
        let mut seq = 99;
        assert_eq!(
            paint_brush_library_prepare_state_groups(
                library,
                json.as_ptr(),
                json.len() as u64,
                &mut seq
            ),
            PAINT_OK
        );
        assert_eq!(read_group_preparation(library, seq), json);
        let mut failed = 99;
        assert_eq!(
            paint_brush_library_prepare_state_groups(library, ptr::null(), 1, &mut failed),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(failed, 0);
        failed = 99;
        assert_eq!(
            paint_brush_library_prepare_state_groups(
                ptr::null_mut(),
                json.as_ptr(),
                json.len() as u64,
                &mut failed
            ),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(failed, 0);
        assert_eq!(read_group_preparation(library, seq), json);
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
}

fn settings(enabled: bool, wet: bool) -> BrushSettings {
    let c = PressureControl::new(
        PressureCurve::new_smooth(vec![(0., 0.), (0.4, 0.2), (1., 1.)]).unwrap(),
        0.2,
    )
    .unwrap();
    let mut settings = BrushSettings {
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
    };
    settings.wet = settings.wet.with_dual_color(true);
    settings.dual = paint_core::DualBrush::new(
        true,
        paint_core::DualMode::Multiply,
        paint_core::SecondaryTip::default(),
    )
    .with_sizing(
        paint_core::DualSizing::new(
            paint_core::DualSizeMode::Relative,
            4.,
            paint_core::DualWidthRule::FixedWidth,
        )
        .unwrap(),
    )
    .with_color(
        paint_core::DualColor::new(
            true,
            true,
            true,
            paint_core::DualColorOrder::SecondBehind,
            paint_core::BlendMode::Normal,
            paint_core::BlendMode::Screen,
        )
        .unwrap(),
    );
    settings
}
fn prepared(library: *mut PaintBrushLibrary, s: &BrushSettings) -> u64 {
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", s.clone()).unwrap();
    let json = paint_io::brush_state::encode(&catalog).unwrap();
    let mut seq = 99;
    unsafe {
        assert_eq!(
            paint_brush_library_prepare_state_wet_dual(
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
fn wet_dual_public_preparation_capture_two_strokes_pixels_history_and_wet_carrier() {
    for enabled in [false, true] {
        for wet in [false, true] {
            for independent in [false, true] {
                let h = Harness::new(64, 64);
                let mut s = settings(enabled, wet);
                if independent {
                    s.dual = paint_core::DualBrush::new(
                        true,
                        paint_core::DualMode::Multiply,
                        paint_core::SecondaryTip::default()
                            .with_spacing(0.2)
                            .unwrap(),
                    )
                    .with_sizing(s.dual.sizing())
                    .with_color(s.dual.color());
                }
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
                                .begin_configured_dual_wet_brush_stroke(
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
                            retained = Some(reference.end_wet_ink_stroke().unwrap().1);
                            let ink = dual_info(tool);
                            assert_eq!(
                                ink.primary_revision,
                                retained.as_ref().unwrap().primary().revision()
                            );
                            assert_eq!(
                                ink.primary_center_linear_premul_rgba,
                                retained
                                    .as_ref()
                                    .unwrap()
                                    .primary()
                                    .sample(0.5, 0.5)
                                    .components()
                            );
                            let second = retained.as_ref().unwrap().secondary().unwrap();
                            assert_eq!(ink.secondary_revision, second.revision());
                            assert_eq!(
                                ink.secondary_center_linear_premul_rgba,
                                second.sample(0.5, 0.5).components()
                            );
                            assert_eq!(ink.flags, 3);
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
}
#[test]
fn wet_dual_public_image_package_roundtrip_and_capacity_checks_do_not_install_resources() {
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
        let stored = store.put_verified(&id, &png, &token).unwrap();
        let resource = stored.mask;
        let fixture = format!(
            "{{\"packageId\":\"12345678-1234-4234-8234-123456789abc\",\"brushes\":[{}]}}",
            include_str!("../../../../../contracts/brush/wet/fixtures/wet-dual-valid.json")
        );
        let mut p = Package::from_json(fixture.as_bytes()).unwrap();
        let preset = &mut p.brushes[0];
        preset
            .required_capabilities
            .retain(|c| c != "procedural-tip");
        preset.required_capabilities.push("image-tip".into());
        preset.tip_resource = Some(id.clone());
        preset.stroke_taper = Some(paint_io::brush_taper::TaperParameters::from_core(
            &settings(enabled, true).taper,
        ));
        preset.wet_mix = Some(paint_io::brush_wet::WetParameters::from_core(
            settings(enabled, true).wet,
        ));
        let mut image_settings = settings(enabled, true);
        image_settings.mask = Some(resource.clone());
        image_settings.dual = paint_core::DualBrush::new(
            true,
            paint_core::DualMode::Multiply,
            paint_core::SecondaryTip::new(
                8.,
                image_settings.tip,
                Some(resource.clone()),
                Default::default(),
            )
            .unwrap(),
        )
        .with_color(image_settings.dual.color());
        image_settings.texture = paint_core::BrushTexture::new(
            true,
            Default::default(),
            Some(std::sync::Arc::new(paint_core::TextureResource::new(
                resource,
            ))),
        )
        .unwrap();
        preset.dual_brush =
            Some(paint_io::brush_dual::DualParameters::from_core(&image_settings.dual).unwrap());
        preset.texture = Some(
            paint_io::brush_texture::TextureParameters::from_core(&image_settings.texture).unwrap(),
        );
        preset.required_capabilities.push("texture-cover".into());
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
                paint_brush_library_package_write_wet_dual(
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
                paint_brush_library_package_read_wet_dual(
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
                paint_brush_library_package_read_wet_dual(
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
                paint_brush_library_package_read_wet_dual(
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
                paint_brush_library_package_write_wet_dual(
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
            let mut s = image_settings.clone();
            s.spacing = 0.075;
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
            assert!(restored.selected().settings().wet.dual_color());
            assert_eq!(
                restored
                    .selected()
                    .settings()
                    .dual
                    .secondary()
                    .mask()
                    .unwrap()
                    .resource_id(),
                Some(id.as_str())
            );
            assert_eq!(
                restored
                    .selected()
                    .settings()
                    .texture
                    .resource()
                    .unwrap()
                    .source()
                    .resource_id(),
                Some(id.as_str())
            );
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

            // Use the retired/rebuilt exact image receipt for the original single 64px move.
            let h = Harness::new(128, 64);
            let mut wet_tool = ptr::null_mut();
            assert_eq!(paint_wet_tool_create(&mut wet_tool), PAINT_OK);
            let mut binding = 0;
            assert_eq!(
                paint_session_bind_wet_tool(h.core, h.session, wet_tool, &mut binding),
                PAINT_OK
            );
            h.wait(binding);
            let mut reference = Document::new(128, 64).unwrap();
            let mut definition = s.definition();
            definition.brush.color = Pixel::from_straight([0., 0., 1., 1.]).unwrap();
            definition.brush.background_color = Pixel::WHITE;
            reference
                .begin_configured_dual_wet_brush_stroke(
                    definition,
                    InputPoint::new(16.5, 32.5, 1.),
                    None,
                )
                .unwrap();
            reference
                .begin_stroke_input(InputPoint::new(80.5, 32.5, 1.))
                .unwrap();
            while !matches!(
                reference
                    .continue_stroke_page(paint_core::InputScheduleBudget::default())
                    .unwrap(),
                paint_core::StrokePageResult::InputComplete
            ) {}
            reference.begin_stroke_finish().unwrap();
            let expected = loop {
                if let paint_core::StrokePageResult::StrokeComplete { wet_ink, .. } = reference
                    .continue_stroke_page(paint_core::InputScheduleBudget::default())
                    .unwrap()
                {
                    break wet_ink.unwrap();
                }
            };
            h.wait(super::wet_api::begin(&h, b, captured, 16.5));
            h.submit(PaintCommand {
                kind: PAINT_COMMAND_STROKE_TO,
                point: PaintPoint {
                    x: 80.5,
                    y: 32.5,
                    pressure: 1.,
                    tool: PAINT_TOOL_MOUSE,
                    ..dto!(PaintPoint)
                },
                ..dto!(PaintCommand)
            });
            let done = h.wait(h.submit(PaintCommand {
                kind: PAINT_COMMAND_END_STROKE,
                ..dto!(PaintCommand)
            }));
            assert_eq!(done.last_error_status, PAINT_OK);
            assert_eq!(done.undo_depth, 1);
            assert_eq!(
                super::wet_api::copied(&h),
                reference.copy_layers(&[1]).unwrap()
            );
            let actual = dual_info(wet_tool);
            assert_eq!(actual.flags, 3);
            assert_eq!(actual.primary_revision, expected.primary().revision());
            assert_eq!(
                actual.secondary_revision,
                expected.secondary().unwrap().revision()
            );
            assert_eq!(
                actual.primary_center_linear_premul_rgba,
                expected.primary().sample(0.5, 0.5).components()
            );
            assert_eq!(
                actual.secondary_center_linear_premul_rgba,
                expected.secondary().unwrap().sample(0.5, 0.5).components()
            );
            assert_eq!(paint_wet_tool_destroy(&mut wet_tool), PAINT_OK);
            assert_eq!(paint_brush_library_destroy(&mut a), PAINT_OK);
            assert_eq!(paint_brush_library_destroy(&mut b), PAINT_OK);
        }
        let bundle = paint_io::vbr_image::load(&path, &token).unwrap();
        assert_eq!(
            bundle.definition(0).unwrap().taper,
            settings(enabled, true).taper
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

fn dual_info(tool: *mut PaintWetTool) -> PaintWetToolInfoDual {
    let mut info = dto!(PaintWetToolInfoDual);
    unsafe {
        assert_eq!(paint_wet_tool_info_dual(tool, &mut info), PAINT_OK);
    }
    info
}
#[test]
fn wet_dual_public_atomic_manual_load_layout_busy_and_error_outputs() {
    assert_eq!(size_of::<PaintWetToolInfoDual>(), 80);
    assert_eq!(offset_of!(PaintWetToolInfoDual, secondary_revision), 16);
    assert_eq!(
        offset_of!(PaintWetToolInfoDual, primary_center_linear_premul_rgba),
        32
    );
    assert_eq!(
        offset_of!(PaintWetToolInfoDual, secondary_center_linear_premul_rgba),
        48
    );
    assert_eq!(offset_of!(PaintWetToolInfoDual, reserved), 64);
    let h = Harness::new(64, 64);
    unsafe {
        let mut v = dto!(PaintVersion);
        assert_eq!(paint_core_version(&mut v), PAINT_OK);
        assert!(v.minor >= 52);
        let mut caps = dto!(PaintCapabilities);
        assert_eq!(paint_core_capabilities(h.core, &mut caps), PAINT_OK);
        assert_ne!(caps.features & PAINT_FEATURE_WET_DUAL_COLOR, 0);
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let mut tool = ptr::null_mut();
        assert_eq!(paint_wet_tool_create(&mut tool), PAINT_OK);
        assert_eq!(dual_info(tool).flags, 0);
        let s = settings(true, true);
        let capture = prepared(library, &s);
        let d = PaintWetLoadDesc {
            radius: 10.,
            linear_rgba: [0.8, 0., 0., 0.5],
            ..dto!(PaintWetLoadDesc)
        };
        let bg = PaintBrushBackgroundDesc {
            linear_rgba: [0., 0.6, 0., 0.8],
            ..dto!(PaintBrushBackgroundDesc)
        };
        let load =
            |ticket, desc: *const PaintWetLoadDesc, back: *const PaintBrushBackgroundDesc| {
                paint_wet_tool_reload_prepared_dual(
                    tool,
                    library,
                    ticket,
                    b"round-pressure".as_ptr(),
                    14,
                    desc,
                    back,
                )
            };
        assert_eq!(load(capture, &d, &bg), PAINT_OK);
        let before = dual_info(tool);
        let reference = paint_core::WetTool::default();
        reference
            .reload_dual(
                s.wet.mix(),
                [20., 24.],
                [
                    Pixel::from_straight(d.linear_rgba).unwrap(),
                    Pixel::from_straight(bg.linear_rgba).unwrap(),
                ],
            )
            .unwrap();
        let expected = reference.snapshot_ink().unwrap().unwrap();
        assert_eq!(before.flags, 3);
        assert_eq!(before.primary_edge, expected.primary().edge() as u32);
        assert_eq!(
            before.secondary_edge,
            expected.secondary().unwrap().edge() as u32
        );
        assert_eq!(
            before.primary_center_linear_premul_rgba,
            expected.primary().sample(0.5, 0.5).components()
        );
        assert_eq!(
            before.secondary_center_linear_premul_rgba,
            expected.secondary().unwrap().sample(0.5, 0.5).components()
        );
        assert_eq!(
            paint_wet_tool_reload_prepared(
                tool,
                library,
                capture,
                b"round-pressure".as_ptr(),
                14,
                &d
            ),
            PAINT_UNSUPPORTED
        );
        let mut invalid = d;
        invalid.radius = f32::NAN;
        assert_eq!(load(capture, &invalid, &bg), PAINT_INVALID_ARGUMENT);
        let mut badbg = bg;
        badbg.reserved = 1;
        assert_eq!(load(capture, &d, &badbg), PAINT_INVALID_ARGUMENT);
        badbg = bg;
        badbg.linear_rgba[2] = f32::NAN;
        assert_eq!(load(capture, &d, &badbg), PAINT_INVALID_ARGUMENT);
        assert_eq!(load(capture, &d, ptr::null()), PAINT_INVALID_ARGUMENT);
        assert_ne!(load(u64::MAX, &d, &bg), PAINT_OK);
        assert_eq!(
            dual_info(tool).secondary_revision,
            before.secondary_revision
        );
        let mut seq = 0;
        assert_eq!(
            paint_session_bind_wet_tool(h.core, h.session, tool, &mut seq),
            PAINT_OK
        );
        h.wait(seq);
        h.wait(super::wet_api::begin(&h, library, capture, 16.5));
        assert_eq!(load(capture, &d, &bg), PAINT_BUSY);
        let mut info = before;
        assert_eq!(paint_wet_tool_info_dual(tool, &mut info), PAINT_BUSY);
        assert_eq!(info.struct_size, 80);
        assert_eq!(info.flags, 0);
        assert_eq!(info.primary_revision, 0);
        assert_eq!(info.secondary_revision, 0);
        assert_eq!(info.secondary_center_linear_premul_rgba, [0.; 4]);
        h.wait(h.submit(PaintCommand {
            kind: PAINT_COMMAND_CANCEL_STROKE,
            ..dto!(PaintCommand)
        }));
        assert_eq!(
            dual_info(tool).secondary_revision,
            before.secondary_revision
        );
        let mut ordinary = s.clone();
        ordinary.wet = ordinary.wet.with_dual_color(false);
        ordinary.dual = Default::default();
        let old = prepared(library, &ordinary);
        assert_eq!(load(old, &d, &bg), PAINT_OK);
        assert_eq!(
            dual_info(tool).secondary_revision,
            before.secondary_revision
        );
        let mut closed = s.clone();
        closed.wet =
            WetSettings::new(false, false, false, false, s.wet.mix()).with_dual_color(true);
        let closed = prepared(library, &closed);
        assert_eq!(load(closed, &d, &bg), PAINT_INVALID_ARGUMENT);
        let mut independent = s.clone();
        independent.dual = paint_core::DualBrush::new(
            true,
            paint_core::DualMode::Multiply,
            paint_core::SecondaryTip::new(3., s.tip, None, Default::default()).unwrap(),
        )
        .with_color(
            paint_core::DualColor::new(
                true,
                false,
                true,
                Default::default(),
                Default::default(),
                Default::default(),
            )
            .unwrap(),
        );
        let independent = prepared(library, &independent);
        assert_eq!(load(independent, &d, &bg), PAINT_OK);
        let foreground = dual_info(tool);
        assert_eq!(
            foreground.primary_center_linear_premul_rgba,
            foreground.secondary_center_linear_premul_rgba
        );
        assert_eq!(foreground.secondary_edge, 6);
        let mut failed = 99;
        assert_eq!(
            paint_brush_library_prepare_state_wet_dual(library, ptr::null(), 1, &mut failed),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(failed, 0);
        assert_eq!(paint_wet_tool_clean(tool), PAINT_OK);
        let clean = dual_info(tool);
        assert_eq!(clean.primary_center_linear_premul_rgba, [0.; 4]);
        assert_eq!(clean.secondary_center_linear_premul_rgba, [0.; 4]);
        assert_eq!(paint_wet_tool_destroy(&mut tool), PAINT_OK);
        let mut invalid = before;
        assert_eq!(
            paint_wet_tool_info_dual(tool, &mut invalid),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(invalid.flags, 0);
        assert_eq!(invalid.reserved, [0; 4]);
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
}
