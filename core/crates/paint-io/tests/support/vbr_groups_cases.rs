use super::*;

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
    for minor in [0, 1, 23, 25] {
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
