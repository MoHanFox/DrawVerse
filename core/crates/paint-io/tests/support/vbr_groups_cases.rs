use super::*;

#[test]
fn groups_handoff_strict_complete_projection_and_legacy_isolation() {
    use paint_io::vbr_handoff::PackageHandoff;
    let token = CancellationToken::default();
    for b in [
        bundle(),
        grouped(bundle()),
        ImageBundle::new(package(None), BTreeMap::new(), &token).unwrap(),
    ] {
        let handoff = PackageHandoff::from_bundle(&b).unwrap();
        let json = handoff.to_json().unwrap();
        let restored = PackageHandoff::from_json(&json).unwrap();
        assert_eq!(restored.to_json().unwrap(), json);
        assert_eq!(
            restored.package().to_json().unwrap(),
            b.package.to_json().unwrap()
        );
        assert_eq!(
            restored.organization().preset_order(),
            b.package
                .brushes
                .iter()
                .map(|p| p.id.clone())
                .collect::<Vec<_>>()
        );
        assert!(Package::from_json(&json).is_err());
        assert!(PackageHandoff::from_json(&b.package.to_json().unwrap()).is_err());
        let good: Value = serde_json::from_slice(&json).unwrap();
        for (key, value) in [
            ("model", json!({"ordered-package-v1":null})),
            ("model", json!("unknown")),
            ("organization", Value::Null),
            ("extra", json!(true)),
            ("package", Value::Null),
        ] {
            let mut bad = good.clone();
            bad[key] = value;
            assert!(PackageHandoff::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        let mut bad = good.clone();
        bad.as_object_mut().unwrap().remove("organization");
        assert!(PackageHandoff::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
        if b.organization().is_some() {
            let mut bad = good.clone();
            bad["package"]["brushes"].as_array_mut().unwrap().reverse();
            assert!(PackageHandoff::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
    }
    assert!(PackageHandoff::from_json(&vec![b' '; paint_io::vbr::MAX_JSON as usize + 1]).is_err());
    let b = grouped(ImageBundle::new(package(None), BTreeMap::new(), &token).unwrap());
    let actual: Value =
        serde_json::from_slice(&PackageHandoff::from_bundle(&b).unwrap().to_json().unwrap())
            .unwrap();
    let expected: Value = serde_json::from_str(include_str!(
        "../../../../../contracts/examples/brush-package-handoff-valid.json"
    ))
    .unwrap();
    assert_eq!(actual, expected);
    assert!(PackageHandoff::from_json(include_bytes!(
        "../../../../../contracts/examples/brush-package-handoff-invalid.json"
    ))
    .is_err());
}

#[test]
fn groups_persist_preflight_cancellation_and_existing_corruption_preserve_catalog() {
    use paint_core::BrushCatalog;
    use paint_io::brush_resource::ResourceStore;
    let token = CancellationToken::default();
    let b = grouped(bundle());
    let tmp = tempfile::tempdir().unwrap();
    let directory = tmp.path().join("resources");
    let store = ResourceStore::open(directory.clone(), paint_core::MAX_CATALOG_MASK_BYTES).unwrap();
    let mut c = BrushCatalog::default();
    let before = c.clone();
    let unchanged = |c: &BrushCatalog| {
        assert_eq!(c.revision(), before.revision());
        assert!(Arc::ptr_eq(&c.organization(), &before.organization()));
        assert!(Arc::ptr_eq(&c.selected(), &before.selected()));
        for id in before.ids() {
            assert!(Arc::ptr_eq(
                &c.preset(id).unwrap(),
                &before.preset(id).unwrap()
            ));
        }
    };
    assert!(b
        .import_into_catalog_persisted(&mut c, &store, None, 999, &token)
        .is_err());
    unchanged(&c);
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(b
        .import_into_catalog_persisted(&mut c, &store, None, 0, &cancelled)
        .is_err());
    unchanged(&c);
    let mut tiny = BrushCatalog::with_mask_budget(1).unwrap();
    let tiny_revision = tiny.revision();
    assert!(b
        .import_into_catalog_persisted(&mut tiny, &store, None, 0, &token)
        .is_err());
    assert_eq!(tiny.revision(), tiny_revision);
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
    let (asset_id, asset) = b.assets().iter().next().unwrap();
    let path = directory.join(format!("{asset_id}.png"));
    std::fs::write(&path, b"corrupt existing").unwrap();
    assert!(b
        .import_into_catalog_persisted(&mut c, &store, None, 0, &token)
        .is_err());
    unchanged(&c);
    assert_eq!(std::fs::read(&path).unwrap(), b"corrupt existing");
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
    std::fs::write(&path, asset.bytes()).unwrap();
    b.import_into_catalog_persisted(&mut c, &store, None, 0, &token)
        .unwrap();
    let adopted = c.clone();
    assert!(b
        .import_into_catalog_persisted(&mut c, &store, None, 0, &token)
        .is_err());
    assert_eq!(c.revision(), adopted.revision());
    assert!(Arc::ptr_eq(&c.organization(), &adopted.organization()));
    assert_eq!(std::fs::read(&path).unwrap(), asset.bytes());
    assert!(std::fs::read_dir(&directory).unwrap().all(|e| !e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".brush-")));
}

#[test]
fn groups_persist_procedural_and_image_state_reopen_from_fresh_store() {
    use paint_core::{BrushCatalog, MAX_CATALOG_MASK_BYTES};
    use paint_io::{brush_resource::ResourceStore, brush_state};
    let token = CancellationToken::default();
    for b in [
        bundle(),
        ImageBundle::new(package(None), BTreeMap::new(), &token).unwrap(),
    ] {
        let b = grouped(b);
        let tmp = tempfile::tempdir().unwrap();
        let directory = tmp.path().join("resources");
        let store = ResourceStore::open(directory.clone(), MAX_CATALOG_MASK_BYTES).unwrap();
        let mut c = BrushCatalog::default();
        b.import_into_catalog_persisted(&mut c, &store, None, 1, &token)
            .unwrap();
        let state = brush_state::encode(&c).unwrap();
        drop(store);
        let fresh = ResourceStore::open(directory, MAX_CATALOG_MASK_BYTES).unwrap();
        let restored = brush_state::decode_with_sources(
            &state,
            MAX_CATALOG_MASK_BYTES,
            Some(&fresh),
            &fresh.source_store().unwrap(),
            &token,
        )
        .unwrap();
        assert_eq!(brush_state::encode(&restored).unwrap(), state);
        assert_eq!(restored.organization(), c.organization());
        if let Some(mask) = c.selected().settings().mask.as_ref() {
            assert_eq!(
                restored
                    .selected()
                    .settings()
                    .mask
                    .as_ref()
                    .unwrap()
                    .coverage(),
                mask.coverage()
            );
        }
    }
}

#[test]
fn groups_vbr24_actual_bundle_merges_and_failure_keeps_target_identity() {
    let token = CancellationToken::default();
    for b in [
        bundle(),
        ImageBundle::new(package(None), BTreeMap::new(), &token).unwrap(),
    ] {
        let b = grouped(b);
        let b = decode(&encode(&b, &token).unwrap(), &token).unwrap();
        let mut c = paint_core::BrushCatalog::default();
        let before = c.clone();
        b.import_into_catalog(&mut c, None, 1, &token).unwrap();
        assert_eq!(c.revision(), before.revision() + 1);
        assert_eq!(c.selected().id(), b.package.brushes[0].id);
        assert_eq!(
            c.organization().groups(),
            b.organization().unwrap().groups()
        );
        for (i, p) in b.package.brushes.iter().enumerate() {
            let actual = c.preset(&p.id).unwrap();
            let definition = b.definition(i).unwrap();
            assert_eq!(actual.settings().diameter, definition.brush.radius * 2.);
            if let Some(mask) = definition.mask {
                assert!(Arc::ptr_eq(&mask, actual.settings().mask.as_ref().unwrap()));
            }
        }
        for id in before.ids() {
            assert!(Arc::ptr_eq(
                &before.preset(id).unwrap(),
                &c.preset(id).unwrap()
            ));
        }
        let adopted = c.clone();
        assert!(b.import_into_catalog(&mut c, None, 0, &token).is_err());
        assert!(Arc::ptr_eq(&c.organization(), &adopted.organization()));
        assert_eq!(c.revision(), adopted.revision());
        let mut fresh = before.clone();
        assert!(b
            .import_into_catalog(&mut fresh, None, 999, &token)
            .is_err());
        let cancelled = CancellationToken::default();
        cancelled.cancel();
        assert!(b
            .import_into_catalog(&mut fresh, None, 0, &cancelled)
            .is_err());
        assert!(Arc::ptr_eq(&fresh.organization(), &before.organization()));
        assert_eq!(fresh.revision(), before.revision());
    }
}

#[test]
fn groups_vbr24_contract_manifest_is_actual_writer_output_and_downgrade_rejects() {
    let token = CancellationToken::default();
    let b = grouped(ImageBundle::new(package(None), BTreeMap::new(), &token).unwrap());
    let bytes = encode(&b, &token).unwrap();
    let valid: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../contracts/examples/vbr-groups-valid.json"
    ))
    .unwrap();
    assert_eq!(manifest(&bytes), valid);
    let invalid: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../contracts/examples/vbr-groups-invalid.json"
    ))
    .unwrap();
    assert!(decode(&edit(&bytes, |m| *m = invalid.clone()), &token).is_err());
}
use paint_core::{BrushCatalogItem, BrushGroup, BrushOrganization};
use serde_json::{json, Value};
fn id(n: usize) -> String {
    format!("12345678-1234-4234-8234-{n:012x}")
}
fn grouped(mut b: ImageBundle) -> ImageBundle {
    let mut second = b.package.brushes[0].clone();
    second.id = id(2);
    b.package.brushes.push(second);
    let roots = vec![
        BrushCatalogItem::Group(id(100)),
        BrushCatalogItem::Preset(id(2)),
    ];
    let groups = vec![
        BrushGroup::new(
            id(100),
            "same".into(),
            vec![
                BrushCatalogItem::Preset(b.package.brushes[0].id.clone()),
                BrushCatalogItem::Group(id(101)),
            ],
        )
        .unwrap(),
        BrushGroup::new(id(101), "same".into(), vec![]).unwrap(),
    ];
    b.with_organization(BrushOrganization::new(roots, groups).unwrap())
        .unwrap()
}
fn manifest(bytes: &[u8]) -> Value {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut data = Vec::new();
    zip.by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut data)
        .unwrap();
    serde_json::from_slice(&data).unwrap()
}
fn edit(bytes: &[u8], mut f: impl FnMut(&mut Value)) -> Vec<u8> {
    rewrite(bytes, |path, data| {
        if path == "manifest.json" {
            let mut m: Value = serde_json::from_slice(data).unwrap();
            f(&mut m);
            *data = serde_json::to_vec(&m).unwrap();
        }
    })
}
#[test]
fn groups_vbr24_procedural_image_empty_folders_order_and_flat_bytes_roundtrip() {
    let token = CancellationToken::default();
    for b in [
        bundle(),
        ImageBundle::new(package(None), BTreeMap::new(), &token).unwrap(),
    ] {
        let old = encode(&b, &token).unwrap();
        assert!(manifest(&old).get("organization").is_none());
        let flat = BrushOrganization::new(
            b.package
                .brushes
                .iter()
                .map(|p| BrushCatalogItem::Preset(p.id.clone()))
                .collect(),
            vec![],
        )
        .unwrap();
        let b = b.with_organization(flat).unwrap();
        assert_eq!(encode(&b, &token).unwrap(), old);
        let b = grouped(b);
        let bytes = encode(&b, &token).unwrap();
        let m = manifest(&bytes);
        assert_eq!(m["version"]["minor"], 24);
        assert_eq!(m["groups"], json!([]));
        let restored = decode(&bytes, &token).unwrap();
        assert_eq!(
            restored.package.to_json().unwrap(),
            b.package.to_json().unwrap()
        );
        assert_eq!(restored.organization(), b.organization());
        assert_eq!(restored.organization().unwrap().groups().len(), 2);
        assert!(restored.organization().unwrap().groups()[&id(101)]
            .items()
            .is_empty());
        assert_eq!(encode(&restored, &token).unwrap(), bytes);
    }
}
#[test]
fn groups_vbr24_downgrade_conflicting_legacy_fields_and_bad_projection_reject() {
    let token = CancellationToken::default();
    let b = grouped(bundle());
    let bytes = encode(&b, &token).unwrap();
    for minor in [0, 1, 23, super::MAX_EXECUTABLE_MINOR + 1] {
        assert!(decode(
            &edit(&bytes, |m| m["version"]["minor"] = json!(minor)),
            &token
        )
        .is_err());
    }
    assert!(decode(&edit(&bytes, |m| m["organization"] = Value::Null), &token).is_err());
    assert!(decode(
        &edit(&bytes, |m| m["groups"] = json!([{"legacy":true}])),
        &token
    )
    .is_err());
    assert!(decode(
        &edit(&bytes, |m| m["organization"]["model"] =
            json!({"ordered-tree-v1":null})),
        &token
    )
    .is_err());
    assert!(decode(
        &edit(&bytes, |m| m["organization"]["roots"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1)),
        &token
    )
    .is_err());
    assert!(decode(
        &edit(&bytes, |m| m["organization"]["groups"][0]["items"][0]
            ["id"] = json!(id(999))),
        &token
    )
    .is_err());
    let mut bad = grouped(bundle());
    bad.package.brushes.swap(0, 1);
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("existing.vbr");
    std::fs::write(&path, b"keep").unwrap();
    assert!(save(&path, &bad, &token).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"keep");
}
#[test]
fn groups_vbr24_copy_maps_are_complete_fresh_disjoint_and_atomic() {
    let token = CancellationToken::default();
    let mut b = grouped(bundle());
    let before = encode(&b, &token).unwrap();
    let presets = b
        .package
        .brushes
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.clone(), id(1000 + i)))
        .collect::<BTreeMap<_, _>>();
    let groups = BTreeMap::from([(id(100), id(2000)), (id(101), id(2001))]);
    let reject = |b: &mut ImageBundle,
                  p: &BTreeMap<String, String>,
                  g: &BTreeMap<String, String>,
                  package: String| {
        assert!(b.remap_copy_ids(package, p, g, &token).is_err());
        assert_eq!(encode(b, &token).unwrap(), before);
    };
    reject(&mut b, &BTreeMap::new(), &groups, id(3000));
    reject(&mut b, &presets, &BTreeMap::new(), id(3000));
    let mut bad = presets.clone();
    bad.insert(id(999), id(9999));
    reject(&mut b, &bad, &groups, id(3000));
    let mut bad = presets.clone();
    for value in bad.values_mut() {
        *value = id(1000);
    }
    reject(&mut b, &bad, &groups, id(3000));
    let mut bad = groups.clone();
    bad.insert(id(100), id(1000));
    reject(&mut b, &presets, &bad, id(3000));
    let mut bad = presets.clone();
    let first = bad.keys().next().unwrap().clone();
    bad.insert(first.clone(), first);
    reject(&mut b, &bad, &groups, id(3000));
    let mut bad = presets.clone();
    *bad.values_mut().next().unwrap() = "bad".into();
    reject(&mut b, &bad, &groups, id(3000));
    let mut bad = groups.clone();
    bad.insert(id(100), "bad".into());
    reject(&mut b, &presets, &bad, id(3000));
    reject(&mut b, &presets, &groups, id(1000));
    let old_package = b.package.package_id.clone();
    reject(&mut b, &presets, &groups, old_package);
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(b
        .remap_copy_ids(id(3000), &presets, &groups, &cancelled)
        .is_err());
    assert_eq!(encode(&b, &token).unwrap(), before);
    let asset = b.assets().values().next().unwrap().mask();
    let asset_bytes = b.assets().values().next().unwrap().bytes().to_vec();
    let old = b.package.brushes.clone();
    b.remap_copy_ids(id(3000), &presets, &groups, &token)
        .unwrap();
    assert!(Arc::ptr_eq(
        &asset,
        &b.assets().values().next().unwrap().mask()
    ));
    assert_eq!(b.assets().values().next().unwrap().bytes(), asset_bytes);
    for (before, after) in old.iter().zip(&b.package.brushes) {
        let mut expected = before.clone();
        expected.id = presets[&before.id].clone();
        assert_eq!(
            serde_json::to_value(expected).unwrap(),
            serde_json::to_value(after).unwrap()
        );
    }
    assert_eq!(
        b.organization().unwrap().preset_order(),
        b.package
            .brushes
            .iter()
            .map(|p| p.id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(b.organization().unwrap().groups()[&id(2000)].name(), "same");
    let reopened = decode(&encode(&b, &token).unwrap(), &token).unwrap();
    assert_eq!(reopened.organization(), b.organization());
}
