use paint_core::{BrushCatalog, MAX_CATALOG_MASK_BYTES};
use paint_io::{
    abr_conversion::ConvertedPreset,
    brush_source::{SourceArchive, SourceStore},
    brush_state,
    ps_descriptor::{Class, Descriptor},
};
use paint_task::CancellationToken;
use std::sync::Arc;
fn source(store: &SourceStore) -> String {
    let descriptor = Descriptor {
        class: Class {
            name: String::new(),
            id: b"null".to_vec(),
        },
        entries: Vec::new(),
    };
    let converted = ConvertedPreset {
        name: "导入记录",
        settings: paint_core::BrushSettings::default(),
        source: &descriptor,
        issues: Vec::new(),
    };
    let archive = SourceArchive::capture(Arc::new(vec![0, 6, 0, 2, 0, 37]), 0, &converted).unwrap();
    store.save(&archive, &CancellationToken::default()).unwrap()
}
#[test]
fn v4_preserves_source_references_without_changing_older_state_versions() {
    let temp = tempfile::tempdir().unwrap();
    let store = SourceStore::open(temp.path().into()).unwrap();
    let id = source(&store);
    let token = CancellationToken::default();
    let mut catalog = BrushCatalog::default();
    catalog
        .set_source("round-pressure", Some(id.clone()))
        .unwrap();
    catalog.set_source("round-fine", Some(id.clone())).unwrap();
    let bytes = brush_state::encode(&catalog).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["version"],
        4
    );
    assert!(brush_state::decode(&bytes, MAX_CATALOG_MASK_BYTES).is_err());
    let restored =
        brush_state::decode_with_sources(&bytes, MAX_CATALOG_MASK_BYTES, None, &store, &token)
            .unwrap();
    assert_eq!(restored.selected().source_resource(), Some(id.as_str()));
    assert_eq!(brush_state::encode(&restored).unwrap(), bytes);
    assert_eq!(restored.revision(), 1);
    let legacy = brush_state::encode(&BrushCatalog::default()).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&legacy).unwrap()["version"],
        2
    );
    assert_eq!(
        brush_state::encode(&brush_state::decode(&legacy, MAX_CATALOG_MASK_BYTES).unwrap())
            .unwrap(),
        legacy
    );
}
#[test]
fn missing_corrupt_wrong_version_and_cancel_refuse_the_whole_restore() {
    let temp = tempfile::tempdir().unwrap();
    let store = SourceStore::open(temp.path().into()).unwrap();
    let id = source(&store);
    let token = CancellationToken::default();
    let mut catalog = BrushCatalog::default();
    catalog
        .set_source("round-pressure", Some(id.clone()))
        .unwrap();
    let bytes = brush_state::encode(&catalog).unwrap();
    let mut wrong: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    wrong["version"] = 3.into();
    assert!(brush_state::decode_with_sources(
        &serde_json::to_vec(&wrong).unwrap(),
        MAX_CATALOG_MASK_BYTES,
        None,
        &store,
        &token
    )
    .is_err());
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(brush_state::decode_with_sources(
        &bytes,
        MAX_CATALOG_MASK_BYTES,
        None,
        &store,
        &cancelled
    )
    .is_err());
    let record = temp.path().join(format!("{id}.json"));
    let original = std::fs::read(&record).unwrap();
    std::fs::write(&record, b"corrupt").unwrap();
    assert!(
        brush_state::decode_with_sources(&bytes, MAX_CATALOG_MASK_BYTES, None, &store, &token)
            .is_err()
    );
    std::fs::write(&record, original).unwrap();
    std::fs::remove_file(record).unwrap();
    assert!(
        brush_state::decode_with_sources(&bytes, MAX_CATALOG_MASK_BYTES, None, &store, &token)
            .is_err()
    );
    assert_eq!(catalog.selected().source_resource(), Some(id.as_str()));
}
#[test]
fn images_after_source_rows_cannot_downgrade_the_state_version() {
    let temp = tempfile::tempdir().unwrap();
    let sources = SourceStore::open(temp.path().join("sources")).unwrap();
    let source_id = source(&sources);
    let resources =
        paint_io::brush_resource::ResourceStore::open(temp.path().join("tips"), 1024).unwrap();
    let token = CancellationToken::default();
    let pixels = paint_core::BrushMask::new(4, 2, vec![255; 8]).unwrap();
    let image = resources
        .import_png(
            &paint_io::brush_resource::encode_mask(&pixels).unwrap(),
            paint_io::brush_resource::Interpretation::Coverage,
            &token,
        )
        .unwrap();
    let mut catalog = BrushCatalog::default();
    catalog
        .set_source("round-pressure", Some(source_id))
        .unwrap();
    let mut settings = catalog.preset("round-fine").unwrap().settings().clone();
    settings.mask = Some(image.mask);
    catalog.edit("round-fine", settings).unwrap();
    let bytes = brush_state::encode(&catalog).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["version"],
        4
    );
    let restored =
        brush_state::decode_with_sources(&bytes, 1024, Some(&resources), &sources, &token).unwrap();
    assert_eq!(brush_state::encode(&restored).unwrap(), bytes);
}

#[test]
fn source_and_image_rows_after_pressure_cannot_downgrade_state5() {
    let temp = tempfile::tempdir().unwrap();
    let resources =
        paint_io::brush_resource::ResourceStore::open(temp.path().into(), 1024).unwrap();
    let sources = resources.source_store().unwrap();
    let id = source(&sources);
    let token = CancellationToken::default();
    let mask = resources
        .import_png(
            &paint_io::brush_resource::encode_mask(
                &paint_core::BrushMask::new(4, 4, vec![255; 16]).unwrap(),
            )
            .unwrap(),
            paint_io::brush_resource::Interpretation::Coverage,
            &token,
        )
        .unwrap();
    let mut catalog = BrushCatalog::default();
    let mut pressure = catalog.selected().settings().clone();
    pressure.dynamics.flow =
        paint_core::PressureControl::new(paint_core::PressureCurve::default(), 0.2).unwrap();
    catalog.edit("round-pressure", pressure).unwrap();
    catalog.set_source("round-fine", Some(id)).unwrap();
    let mut image = catalog.preset("round-wide").unwrap().settings().clone();
    image.mask = Some(mask.mask);
    catalog.edit("round-wide", image).unwrap();
    let bytes = brush_state::encode(&catalog).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["version"],
        5
    );
    let restored =
        brush_state::decode_with_sources(&bytes, 1024, Some(&resources), &sources, &token).unwrap();
    assert_eq!(brush_state::encode(&restored).unwrap(), bytes);
}
