use super::*;
use paint_core::{
    BrushCatalog, BrushMask, BrushSettings, BrushTip, Document, InputPoint, OutlinePlacement,
    OutlineSource, OutlineWidth, Pixel, SecondaryTip, StrokeOutline,
};
use std::sync::Arc;

fn settings(enabled: bool) -> BrushSettings {
    BrushSettings {
        diameter: 8.,
        spacing: 0.15,
        outline: StrokeOutline::new(
            enabled,
            if enabled {
                OutlinePlacement::Outside
            } else {
                OutlinePlacement::Center
            },
            OutlineWidth::new(true, 5., 12.5).unwrap(),
            OutlineSource::SelfBrush,
            true,
        )
        .unwrap(),
        tip: BrushTip {
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn prepared(library: *mut PaintBrushLibrary, settings: &BrushSettings) -> (u64, Vec<u8>) {
    prepared_model(library, settings, "stroke-replay-v1")
}

fn prepared_model(
    library: *mut PaintBrushLibrary,
    settings: &BrushSettings,
    model: &str,
) -> (u64, Vec<u8>) {
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", settings.clone()).unwrap();
    let json = paint_io::brush_state::encode(&catalog).unwrap();
    let mut wire: serde_json::Value = serde_json::from_slice(&json).unwrap();
    if model != "stroke-replay-v1" {
        wire["version"] = if model == "stroke-sdf-v1" { 43 } else { 41 }.into();
        wire["presets"][0]["strokeOutline"]["model"] = model.into();
        wire["presets"][0]["strokeOutline"]
            .as_object_mut()
            .unwrap()
            .remove("allowOverflow");
    }
    let wire = serde_json::to_vec(&wire).unwrap();
    let mut sequence = 0;
    unsafe {
        assert_eq!(
            paint_brush_library_prepare_state_outline(
                library,
                wire.as_ptr(),
                wire.len() as u64,
                &mut sequence
            ),
            PAINT_OK
        );
    }
    let bytes = wait(|| unsafe {
        let mut required = 0;
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
    });
    assert_eq!(bytes, json);
    (sequence, bytes)
}

#[test]
fn outline_public_preparation_executes_real_pixels_history_and_rejects_old_entries() {
    for model in ["stroke-contour-v1", "stroke-sdf-v1", "stroke-replay-v1"] {
        for enabled in [false, true] {
            let h = Harness::new(64, 64);
            unsafe {
                let mut version = dto!(PaintVersion);
                assert_eq!(paint_core_version(&mut version), PAINT_OK);
                assert!(version.minor >= 75);
                let mut capabilities = dto!(PaintCapabilities);
                assert_eq!(paint_core_capabilities(h.core, &mut capabilities), PAINT_OK);
                assert_ne!(capabilities.features & PAINT_FEATURE_STROKE_OUTLINE, 0);
                assert_ne!(
                    capabilities.features & PAINT_FEATURE_STROKE_REPLAY_OUTLINE,
                    0
                );
                assert_ne!(capabilities.features & PAINT_FEATURE_DUAL_TIP_TEXTURE, 0);

                let mut library = ptr::null_mut();
                assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
                let preset = settings(enabled);
                let (capture, json) = prepared_model(library, &preset, model);
                let mut sequence = 99;
                assert_eq!(
                    paint_brush_library_prepare_state_taper(
                        library,
                        json.as_ptr(),
                        json.len() as u64,
                        &mut sequence
                    ),
                    PAINT_OK
                );
                let rejected = wait(|| {
                    let mut required = 0;
                    let status = paint_brush_library_prepared_state(
                        library,
                        sequence,
                        ptr::null_mut(),
                        0,
                        &mut required,
                    );
                    if status == PAINT_BUSY {
                        None
                    } else {
                        Some((status, required))
                    }
                });
                assert_eq!(rejected, (PAINT_UNSUPPORTED, 0));

                let mut reference = Document::new(64, 64).unwrap();
                let mut definition = preset.definition();
                definition.brush.color = Pixel::from_straight([0., 0., 1., 1.]).unwrap();
                definition.brush.background_color = Pixel::WHITE;
                h.wait(super::wet_api::begin(&h, library, capture, 16.5));
                reference
                    .begin_brush_stroke(definition.clone(), InputPoint::new(16.5, 32.5, 1.))
                    .unwrap();
                prepared(library, &settings(false));
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
                h.wait(h.submit(PaintCommand {
                    kind: PAINT_COMMAND_END_STROKE,
                    ..dto!(PaintCommand)
                }));
                assert!(reference.end_stroke().unwrap());
                assert_eq!(
                    super::wet_api::copied(&h),
                    reference.copy_layers(&[1]).unwrap()
                );
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
                assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
            }
        }
    }
}

#[test]
fn outline_public_package_entry_preserves_resources_and_old_entries_reject() {
    use paint_io::{
        brush_resource::{encode_mask, hash, ResourceStore},
        brush_stroke_outline::OutlineParameters,
        vbr::Package,
    };
    use paint_task::CancellationToken;

    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let target = temp.path().join("target");
    let path = temp.path().join("outline.vbr");
    let token = CancellationToken::default();
    let raw = BrushMask::new(
        4,
        4,
        [
            255, 0, 255, 0, 255, 255, 0, 255, 0, 255, 255, 0, 255, 0, 255, 255,
        ]
        .to_vec(),
    )
    .unwrap();
    let png = encode_mask(&raw).unwrap();
    let id = hash(&png);
    let source_store =
        ResourceStore::open(source.clone(), paint_core::MAX_CATALOG_MASK_BYTES).unwrap();
    source_store.put_verified(&id, &png, &token).unwrap();
    let mask = source_store.load(&id, &token).unwrap().mask;
    let tip = SecondaryTip::new(2., BrushTip::default(), Some(mask), Default::default())
        .unwrap()
        .with_spacing(0.15)
        .unwrap();
    let outline = StrokeOutline::new(
        false,
        OutlinePlacement::Inside,
        OutlineWidth::new(false, 4., 20.).unwrap(),
        OutlineSource::Existing(Arc::new(tip)),
        true,
    )
    .unwrap();
    let fixture = format!(
        "{{\"packageId\":\"12345678-1234-4234-8234-123456789abc\",\"brushes\":[{}]}}",
        include_str!("../../../../../contracts/brush/wet/fixtures/vbr-wet-valid.json")
    );
    let mut package = Package::from_json(fixture.as_bytes()).unwrap();
    package.brushes[0].behavior_version = 33;
    package.brushes[0].stroke_outline = Some(OutlineParameters::from_core(&outline).unwrap());
    package.brushes[0]
        .required_capabilities
        .push("stroke-outline".into());
    let json = package.to_json().unwrap();
    let path_bytes = path.to_str().unwrap().as_bytes();
    let source_bytes = source.to_str().unwrap().as_bytes();
    let target_bytes = target.to_str().unwrap().as_bytes();
    unsafe {
        let mut a = ptr::null_mut();
        let mut b = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut a), PAINT_OK);
        assert_eq!(paint_brush_library_create(0, &mut b), PAINT_OK);
        assert_eq!(
            paint_brush_library_bind_resources(a, source_bytes.as_ptr(), source_bytes.len() as u64),
            PAINT_OK
        );
        assert_eq!(
            paint_brush_library_bind_resources(b, target_bytes.as_ptr(), target_bytes.len() as u64),
            PAINT_OK
        );
        assert_eq!(
            paint_brush_library_package_write_outline(
                a,
                path_bytes.as_ptr(),
                path_bytes.len() as u64,
                json.as_ptr(),
                json.len() as u64
            ),
            PAINT_OK
        );
        let before = std::fs::read(&path).unwrap();
        assert_eq!(
            paint_brush_library_package_write(
                a,
                path_bytes.as_ptr(),
                path_bytes.len() as u64,
                json.as_ptr(),
                json.len() as u64
            ),
            PAINT_UNSUPPORTED
        );
        for reader in [
            paint_brush_library_package_read,
            paint_brush_library_package_read_taper,
            paint_brush_library_package_read_wet_dual,
        ] {
            let mut required = 99;
            assert_eq!(
                reader(
                    b,
                    path_bytes.as_ptr(),
                    path_bytes.len() as u64,
                    ptr::null_mut(),
                    0,
                    &mut required
                ),
                PAINT_UNSUPPORTED
            );
            assert_eq!(required, 0);
        }
        let mut required = 99;
        assert_eq!(
            paint_brush_library_package_read_outline(
                b,
                path_bytes.as_ptr(),
                path_bytes.len() as u64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        let mut bytes = vec![0; required as usize];
        assert_eq!(
            paint_brush_library_package_read_outline(
                b,
                path_bytes.as_ptr(),
                path_bytes.len() as u64,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        assert_eq!(
            Package::from_json(&bytes).unwrap().brushes[0]
                .stroke_outline
                .as_ref()
                .unwrap()
                .existing_resource_id(),
            Some(id.as_str())
        );
        assert!(target.join(format!("{id}.png")).exists());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(paint_brush_library_destroy(&mut a), PAINT_OK);
        assert_eq!(paint_brush_library_destroy(&mut b), PAINT_OK);
    }
    let bundle = paint_io::vbr_image::load(&path, &token).unwrap();
    assert!(!bundle.definition(0).unwrap().outline.enabled());
    assert_eq!(
        bundle.definition(0).unwrap().outline.placement(),
        OutlinePlacement::Inside
    );
}
