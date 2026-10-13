use paint_io::{
    abr_conversion::{ConvertedPreset, Issue, Status},
    brush_source::*,
    ps_descriptor::{Class, Descriptor},
};
use paint_task::CancellationToken;
use std::sync::Arc;
fn archive() -> SourceArchive {
    // Generated opaque source bytes: valid source header; no claim of full ABR validity.
    let bytes = Arc::new(vec![0, 6, 0, 2, 255, 0, 37, 42]);
    let source = Descriptor {
        class: Class {
            name: String::new(),
            id: b"null".to_vec(),
        },
        entries: Vec::new(),
    };
    let converted = ConvertedPreset {
        name: "导入时名称",
        settings: paint_core::BrushSettings::default(),
        source: &source,
        issues: vec![Issue {
            path: "textureDepth".into(),
            status: Status::NotExecuted,
            reason: "未执行纹理",
        }],
    };
    SourceArchive::capture(bytes, 0, &converted).unwrap()
}
#[test]
fn receipts_are_weak_scoped_and_fresh_inspection_still_detects_corruption() {
    use std::collections::BTreeSet;
    let temp = tempfile::tempdir().unwrap();
    let store = SourceStore::open(temp.path().into()).unwrap();
    let token = CancellationToken::default();
    let original = archive();
    let id = store.save(&original, &token).unwrap();
    let ids = BTreeSet::from([id.clone()]);
    let first = store.resolve_references(&ids, &token).unwrap();
    let weak = Arc::downgrade(&first[0]);
    assert_eq!(first[0].id(), id);
    let second = store.resolve_references(&ids, &token).unwrap();
    assert!(Arc::ptr_eq(&first[0], &second[0]));
    let record = temp.path().join(format!("{id}.json"));
    std::fs::write(record, b"corrupt").unwrap();
    assert!(store.verified_record(&id, &token).is_err());
    assert!(store.validate_references(&ids, &token).is_err());
    assert!(SourceStore::open(temp.path().into())
        .unwrap()
        .resolve_references(&ids, &token)
        .is_err());
    assert!(store.resolve_references(&ids, &token).is_ok());
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(store.resolve_references(&ids, &cancelled).is_err());
    drop(first);
    drop(second);
    assert!(weak.upgrade().is_none());
    assert!(store.resolve_references(&ids, &token).is_err());
}
#[test]
fn immutable_source_and_import_time_report_roundtrip_without_original_path() {
    let temp = tempfile::tempdir().unwrap();
    let store = SourceStore::open(temp.path().join("sources")).unwrap();
    let original = archive();
    let token = CancellationToken::default();
    let id = store.save(&original, &token).unwrap();
    assert_eq!(store.save(&original, &token).unwrap(), id);
    let restored = store.load(&id, &token).unwrap();
    assert_eq!(restored.original(), original.original());
    assert_eq!(restored.record().unwrap(), original.record().unwrap());
    assert_eq!(restored.info.report[0].status, Support::NotExecuted);
    assert_eq!(restored.info.original_name, "导入时名称");
    let mut renamed = archive();
    renamed.info.original_name = "另一个预设".into();
    let second = store.save(&renamed, &token).unwrap();
    assert_ne!(second, id);
    assert_eq!(
        std::fs::read_dir(temp.path().join("sources"))
            .unwrap()
            .count(),
        3
    );
}
#[test]
fn bad_identity_corruption_and_cancel_never_overwrite_original_content() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("sources");
    let store = SourceStore::open(directory.clone()).unwrap();
    let archive = archive();
    let token = CancellationToken::default();
    let id = store.save(&archive, &token).unwrap();
    let captured = store.load(&id, &token).unwrap();
    let path = directory.join(format!("{}.abr", archive.info.original_hash));
    std::fs::write(&path, b"corrupt").unwrap();
    assert!(store.load(&id, &token).is_err());
    assert!(store.save(&archive, &token).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"corrupt");
    assert_eq!(captured.original(), archive.original());
    assert!(store.load("../bad", &token).is_err());
    let empty = SourceStore::open(temp.path().join("cancelled")).unwrap();
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(empty.save(&archive, &cancelled).is_err());
    assert_eq!(
        std::fs::read_dir(temp.path().join("cancelled"))
            .unwrap()
            .count(),
        0
    );
}
#[test]
fn changed_metadata_versions_and_report_bounds_reject_before_file_writes() {
    let temp = tempfile::tempdir().unwrap();
    let store = SourceStore::open(temp.path().into()).unwrap();
    let token = CancellationToken::default();
    for mutation in 0..5 {
        let mut bad = archive();
        match mutation {
            0 => bad.info.version = 2,
            1 => bad.info.original_hash = "0".repeat(64),
            2 => bad.info.file_subversion = Some(1),
            3 => bad.info.baseline.roundness = 0.,
            _ => bad.info.report = vec![bad.info.report[0].clone(); 4097],
        };
        assert!(store.save(&bad, &token).is_err());
    }
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}
#[test]
fn concurrent_equal_content_saves_share_ids_and_preserve_complete_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let store = Arc::new(SourceStore::open(temp.path().into()).unwrap());
    let original = Arc::new(archive());
    let threads = (0..4)
        .map(|_| {
            let store = store.clone();
            let original = original.clone();
            std::thread::spawn(move || {
                store
                    .save(&original, &CancellationToken::default())
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let ids = threads
        .into_iter()
        .map(|t| t.join().unwrap())
        .collect::<Vec<_>>();
    assert!(ids.iter().all(|id| id == &ids[0]));
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 2);
}
#[test]
fn portable_records_keep_exact_bytes_even_when_json_whitespace_is_not_canonical() {
    let temp = tempfile::tempdir().unwrap();
    let store = SourceStore::open(temp.path().into()).unwrap();
    let archive = archive();
    let record = serde_json::to_vec_pretty(&archive.info).unwrap();
    let id = paint_io::brush_resource::hash(&record);
    let token = CancellationToken::default();
    let imported = store
        .put_verified(&id, &record, Arc::new(archive.original().to_vec()), &token)
        .unwrap();
    assert_eq!(imported.original(), archive.original());
    assert_eq!(store.encoded_record(&id, &token).unwrap(), record);
    assert_eq!(
        store.load(&id, &token).unwrap().info.original_name,
        archive.info.original_name
    );
    assert!(store
        .put_verified(
            &"0".repeat(64),
            &record,
            Arc::new(archive.original().to_vec()),
            &token
        )
        .is_err());
}
