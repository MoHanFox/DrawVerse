//! Independent wire fixtures; not Photoshop-produced compatibility claims.
use paint_core::{Document, InputPoint, MAX_CATALOG_MASK_BYTES};
use paint_io::{abr_import::ImportBatch, brush_resource::ResourceStore, Error};
use paint_task::CancellationToken;
use std::sync::Arc;
mod support {
    pub mod abr_fixture;
}
use support::abr_fixture::fixture;
#[test]
fn whole_batches_compile_share_real_holes_and_store_exact_sources_in_order() {
    let token = CancellationToken::default();
    for version in [6, 10] {
        for sub in [1, 2] {
            let original = Arc::new(fixture(version, sub, false));
            let batch =
                ImportBatch::decode(original.clone(), MAX_CATALOG_MASK_BYTES, &token).unwrap();
            assert_eq!(batch.presets().len(), 3);
            assert_eq!(batch.asset_count(), 1);
            assert!(batch
                .presets()
                .iter()
                .all(|p| p.name() == "Same" && p.is_partial()));
            let temp = tempfile::tempdir().unwrap();
            let store =
                ResourceStore::open(temp.path().join("assets"), MAX_CATALOG_MASK_BYTES).unwrap();
            let presets = batch.persist(&store, &token).unwrap();
            assert!(presets[1].settings().mask.is_none());
            assert!(Arc::ptr_eq(
                presets[0].settings().mask.as_ref().unwrap(),
                presets[2].settings().mask.as_ref().unwrap()
            ));
            assert_ne!(presets[0].source_id(), presets[2].source_id());
            let sources = store.source_store().unwrap();
            for (index, preset) in presets.iter().enumerate() {
                let source = sources.load(preset.source_id(), &token).unwrap();
                assert_eq!(source.original(), original.as_slice());
                assert_eq!(source.info.preset_index, index as u32);
                assert_eq!(
                    source.info.tip_resource.as_deref(),
                    preset
                        .settings()
                        .mask
                        .as_ref()
                        .and_then(|m| m.resource_id())
                );
                assert!(source.info.report.iter().any(|r| r.path == "unknownTexture"
                    && r.status == paint_io::brush_source::Support::NotExecuted));
            }
            assert_eq!(
                std::fs::read_dir(temp.path().join("assets/sources"))
                    .unwrap()
                    .count(),
                4
            );
            let mut document = Document::new(24, 24).unwrap();
            document
                .begin_brush_stroke(
                    presets[0].settings().definition(),
                    InputPoint::new(12.5, 12.5, 1.),
                )
                .unwrap();
            document.end_stroke().unwrap();
            assert_eq!(document.pixel(12, 12).alpha(), 0.);
            assert!(document.pixel(15, 12).alpha() > 0.9);
        }
    }
}
#[test]
fn invalid_late_parameter_budget_cancel_and_unsupported_versions_return_no_batch() {
    let token = CancellationToken::default();
    assert!(ImportBatch::decode(
        Arc::new(fixture(6, 2, true)),
        MAX_CATALOG_MASK_BYTES,
        &token
    )
    .is_err());
    assert!(ImportBatch::decode(Arc::new(fixture(6, 2, false)), 1, &token).is_err());
    for header in [[0, 1, 0, 1], [0, 2, 0, 1], [0, 7, 0, 2], [0, 6, 0, 3]] {
        assert!(matches!(
            ImportBatch::decode(Arc::new(header.to_vec()), MAX_CATALOG_MASK_BYTES, &token),
            Err(Error::Unsupported(_))
        ));
    }
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(matches!(
        ImportBatch::decode(
            Arc::new(fixture(6, 1, false)),
            MAX_CATALOG_MASK_BYTES,
            &cancelled
        ),
        Err(Error::Cancelled)
    ));
    let temp = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(temp.path().into(), MAX_CATALOG_MASK_BYTES).unwrap();
    let batch = ImportBatch::decode(
        Arc::new(fixture(6, 1, false)),
        MAX_CATALOG_MASK_BYTES,
        &token,
    )
    .unwrap();
    assert!(matches!(
        batch.persist(&store, &cancelled),
        Err(Error::Cancelled)
    ));
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}
#[test]
fn read_captures_file_bytes_and_batch_save_refuses_existing_corrupt_original() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("input.abr");
    let bytes = fixture(6, 2, false);
    std::fs::write(&path, &bytes).unwrap();
    let token = CancellationToken::default();
    let batch = ImportBatch::read(&path, MAX_CATALOG_MASK_BYTES, &token).unwrap();
    std::fs::remove_file(path).unwrap();
    let store = ResourceStore::open(temp.path().join("assets"), MAX_CATALOG_MASK_BYTES).unwrap();
    batch.persist(&store, &token).unwrap();
    let original = store
        .directory()
        .join("sources")
        .join(format!("{}.abr", paint_io::brush_resource::hash(&bytes)));
    std::fs::write(&original, b"corrupt").unwrap();
    let next = ImportBatch::decode(Arc::new(bytes), MAX_CATALOG_MASK_BYTES, &token).unwrap();
    assert!(next.persist(&store, &token).is_err());
    assert_eq!(std::fs::read(original).unwrap(), b"corrupt");
}

#[test]
fn texture_pattern_handoff_binds_mapped_mode_and_paints_real_pixels() {
    let token = CancellationToken::default();
    for (abr_mode, expected_mode) in [
        ("Mltp", paint_core::TextureMode::Multiply),
        ("CBrn", paint_core::TextureMode::ColorBurn),
    ] {
        let original = Arc::new(support::abr_fixture::fixture_with_texture(abr_mode));
        let parsed = paint_io::abr_sampled::decode_sectioned(&original, &token).unwrap();
        assert_eq!(parsed.patterns.len(), 1);
        assert_eq!(parsed.texture_presets.len(), 1);
        let batch = ImportBatch::decode(original.clone(), MAX_CATALOG_MASK_BYTES, &token).unwrap();
        assert_eq!(batch.asset_count(), 2);
        let wire: serde_json::Value = serde_json::from_slice(&batch.to_json().unwrap()).unwrap();
        assert_eq!(wire["version"], 7);
        assert_eq!(wire["presets"][0]["texture"]["enabled"], true);
        assert_eq!(
            wire["presets"][0]["texture"]["mode"],
            if expected_mode == paint_core::TextureMode::ColorBurn {
                "colorBurn"
            } else {
                "multiply"
            }
        );
        assert!(wire["presets"][0]["texture"]["resource"].is_string());
        let temp = tempfile::tempdir().unwrap();
        let store = ResourceStore::open(temp.path().into(), MAX_CATALOG_MASK_BYTES).unwrap();
        let imported = batch.persist(&store, &token).unwrap();
        let settings = imported[0].settings();
        assert!(settings.texture.enabled());
        assert_eq!(settings.texture.settings().mode(), expected_mode);
        assert_eq!(
            settings.texture.resource().unwrap().source().coverage(),
            [0, 255, 0, 255, 255, 0, 255, 0, 0, 255, 0, 255, 255, 0, 255, 0]
        );
        let source = store
            .source_store()
            .unwrap()
            .load(imported[0].source_id(), &token)
            .unwrap();
        assert_eq!(source.info.converter_version, 8);
        let mut with_texture = Document::new(24, 24).unwrap();
        with_texture
            .begin_brush_stroke(settings.definition(), InputPoint::new(12.5, 12.5, 1.))
            .unwrap();
        with_texture.end_stroke().unwrap();
        let mut without_texture = Document::new(24, 24).unwrap();
        let mut plain = settings.clone();
        plain.texture = Default::default();
        without_texture
            .begin_brush_stroke(plain.definition(), InputPoint::new(12.5, 12.5, 1.))
            .unwrap();
        without_texture.end_stroke().unwrap();
        assert!((0..24)
            .flat_map(|y| (0..24).map(move |x| (x, y)))
            .any(|(x, y)| {
                with_texture.pixel(x, y).alpha() < without_texture.pixel(x, y).alpha()
            }));
        let painted = (0..24)
            .flat_map(|y| (0..24).map(move |x| (x, y)))
            .map(|(x, y)| with_texture.pixel(x, y))
            .collect::<Vec<_>>();
        with_texture.undo().unwrap();
        assert!((0..24).all(|y| (0..24).all(|x| with_texture.pixel(x, y).alpha() == 0.)));
        with_texture.redo().unwrap();
        assert_eq!(
            (0..24)
                .flat_map(|y| (0..24).map(move |x| (x, y)))
                .map(|(x, y)| with_texture.pixel(x, y))
                .collect::<Vec<_>>(),
            painted
        );
    }
}

#[test]
fn abr_texture_brightness_and_contrast_survive_handoff_and_source_report() {
    let token = CancellationToken::default();
    let original = Arc::new(support::abr_fixture::fixture_with_texture_adjustments(
        "Mltp", -25., 50.,
    ));
    let batch = ImportBatch::decode(original, MAX_CATALOG_MASK_BYTES, &token).unwrap();
    let wire: serde_json::Value = serde_json::from_slice(&batch.to_json().unwrap()).unwrap();
    assert_eq!(wire["presets"][0]["texture"]["brightness"], -0.25);
    assert_eq!(wire["presets"][0]["texture"]["contrast"], 0.5);
    let temp = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(temp.path().into(), MAX_CATALOG_MASK_BYTES).unwrap();
    let imported = batch.persist(&store, &token).unwrap();
    let settings = imported[0].settings().texture.settings();
    assert_eq!(settings.brightness(), -0.25);
    assert_eq!(settings.contrast(), 0.5);
    let source = store
        .source_store()
        .unwrap()
        .load(imported[0].source_id(), &token)
        .unwrap();
    for path in ["textureBrightness", "textureContrast"] {
        assert!(source.info.report.iter().any(
            |item| item.path == path && item.status == paint_io::brush_source::Support::Mapped
        ));
    }
}

#[test]
fn pressure_handoff_and_source_reset_preserve_image_and_program_across_two_strokes() {
    let token = CancellationToken::default();
    let original = Arc::new(support::abr_fixture::fixture_with_pressure(6, 2, false));
    let batch = ImportBatch::decode(original.clone(), MAX_CATALOG_MASK_BYTES, &token).unwrap();
    let wire: serde_json::Value = serde_json::from_slice(&batch.to_json().unwrap()).unwrap();
    assert_eq!(wire["version"], 2);
    assert_eq!(
        wire["presets"][0]["parameters"]["pressureDynamics"]["opacity"]["enabled"],
        true
    );
    let temp = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(temp.path().into(), MAX_CATALOG_MASK_BYTES).unwrap();
    let imported = batch.persist(&store, &token).unwrap();
    for p in imported {
        let source = store
            .source_store()
            .unwrap()
            .load(p.source_id(), &token)
            .unwrap();
        assert_eq!(source.original(), original.as_slice());
        assert_eq!(source.info.converter_version, 3);
        let mut baseline = source.info.baseline.settings();
        baseline.mask = p.settings().mask.clone();
        let mut a = Document::new(64, 32).unwrap();
        let mut b = Document::new(64, 32).unwrap();
        for pressure in [0.25, 0.75] {
            for (doc, s) in [(&mut a, p.settings()), (&mut b, &baseline)] {
                doc.begin_brush_stroke(s.definition(), InputPoint::new(10.5, 16.5, pressure))
                    .unwrap();
                doc.stroke_to(InputPoint::new(50.5, 16.5, pressure))
                    .unwrap();
                doc.end_stroke().unwrap();
            }
        }
        assert_eq!(a.history_depth(), (2, 0));
        for y in 0..32 {
            for x in 0..64 {
                assert_eq!(a.pixel(x, y), b.pixel(x, y));
            }
        }
        a.undo().unwrap();
        a.redo().unwrap();
        for y in 0..32 {
            for x in 0..64 {
                assert_eq!(a.pixel(x, y), b.pixel(x, y));
            }
        }
        let mut downgraded = source.info.clone();
        downgraded.converter_version = 2;
        assert!(downgraded.validate().is_err());
    }
}

#[test]
fn scatter_closed_and_active_handoff_and_source_reset_preserve_two_strokes() {
    for enabled in [false, true] {
        let token = CancellationToken::default();
        let original = Arc::new(support::abr_fixture::fixture_with_scatter(
            6, 2, false, enabled,
        ));
        let batch = ImportBatch::decode(original.clone(), MAX_CATALOG_MASK_BYTES, &token).unwrap();
        let wire: serde_json::Value = serde_json::from_slice(&batch.to_json().unwrap()).unwrap();
        assert_eq!(wire["version"], 3);
        assert_eq!(
            wire["presets"][0]["parameters"]["pressureDynamics"]["opacity"]["enabled"],
            true
        );
        let temp = tempfile::tempdir().unwrap();
        let store = ResourceStore::open(temp.path().into(), MAX_CATALOG_MASK_BYTES).unwrap();
        let imported = batch.persist(&store, &token).unwrap();
        for p in imported {
            let source = store
                .source_store()
                .unwrap()
                .load(p.source_id(), &token)
                .unwrap();
            assert_eq!(source.original(), original.as_slice());
            assert_eq!(source.info.converter_version, 4);
            let mut baseline = source.info.baseline.settings();
            baseline.mask = p.settings().mask.clone();
            let mut a = Document::new(64, 32).unwrap();
            let mut b = Document::new(64, 32).unwrap();
            for pressure in [0.25, 0.75] {
                for (doc, s) in [(&mut a, p.settings()), (&mut b, &baseline)] {
                    doc.begin_brush_stroke(s.definition(), InputPoint::new(10.5, 16.5, pressure))
                        .unwrap();
                    doc.stroke_to(InputPoint::new(50.5, 16.5, pressure))
                        .unwrap();
                    doc.end_stroke().unwrap();
                }
            }
            assert_eq!(a.history_depth(), (2, 0));
            for y in 0..32 {
                for x in 0..64 {
                    assert_eq!(a.pixel(x, y), b.pixel(x, y));
                }
            }
            a.undo().unwrap();
            a.redo().unwrap();
            for y in 0..32 {
                for x in 0..64 {
                    assert_eq!(a.pixel(x, y), b.pixel(x, y));
                }
            }
            let mut downgraded = source.info.clone();
            downgraded.converter_version = 3;
            assert!(downgraded.validate().is_err());
        }
    }
}
