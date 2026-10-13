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
