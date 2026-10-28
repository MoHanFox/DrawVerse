use paint_core::*;
use paint_io::brush_state;
use serde_json::{json, Value};
fn id(n: usize) -> String {
    format!("12345678-1234-4234-8234-{n:012x}")
}
fn grouped() -> BrushCatalog {
    let mut c = BrushCatalog::default();
    c.create_group(id(1), "same".into(), None, 0).unwrap();
    c.create_group(id(2), "same".into(), Some(&id(1)), 0)
        .unwrap();
    c.move_catalog_item(
        BrushCatalogItem::Preset("round-pressure".into()),
        Some(&id(2)),
        0,
    )
    .unwrap();
    c
}
fn value() -> Value {
    serde_json::from_slice(&brush_state::encode(&grouped()).unwrap()).unwrap()
}
fn decode(v: &Value) -> paint_io::Result<BrushCatalog> {
    brush_state::decode(&serde_json::to_vec(v).unwrap(), MAX_CATALOG_MASK_BYTES)
}
#[test]
fn groups_state20_contract_examples_match_the_core_organization() {
    use paint_io::brush_organization::OrganizationParameters;
    let valid =
        include_str!("../../../../contracts/brush/library/examples/brush-organization-valid.json");
    let invalid = include_str!(
        "../../../../contracts/brush/library/examples/brush-organization-invalid.json"
    );
    let parameters: OrganizationParameters = serde_json::from_str(valid).unwrap();
    let mut catalog = BrushCatalog::default();
    catalog.set_organization(parameters.core().clone()).unwrap();
    assert_eq!(
        serde_json::to_value(parameters).unwrap(),
        serde_json::from_str::<Value>(valid).unwrap()
    );
    assert!(serde_json::from_str::<OrganizationParameters>(invalid).is_err());
}
#[test]
fn groups_state20_nested_empty_duplicate_names_order_and_closed_fields_roundtrip() {
    let mut c = grouped();
    c.create_group(id(3), "empty".into(), Some(&id(2)), 1)
        .unwrap();
    let mut s = c.selected().settings().clone();
    s.wet = WetSettings::new(
        false,
        true,
        false,
        true,
        WetMix::new(0.1, 0.2, 0.3, 0.4, 0.5, 0.6).unwrap(),
    )
    .with_dual_color(true);
    s.smoothing = BrushSmoothing::new(false, 4., 8., 0.2, 0.3).unwrap();
    s.taper = StrokeTaper::new(
        false,
        TaperEnd::new(8., Default::default(), Default::default()).unwrap(),
        TaperEnd::new(12., Default::default(), Default::default()).unwrap(),
    );
    c.edit("round-pressure", s).unwrap();
    let bytes = brush_state::encode(&c).unwrap();
    let restored = brush_state::decode(&bytes, MAX_CATALOG_MASK_BYTES).unwrap();
    assert_eq!(*restored.organization(), *c.organization());
    assert_eq!(restored.ids(), c.ids());
    assert_eq!(
        restored.selected().settings().wet,
        c.selected().settings().wet
    );
    assert_eq!(
        restored.selected().settings().smoothing,
        c.selected().settings().smoothing
    );
    assert_eq!(
        restored.selected().settings().taper,
        c.selected().settings().taper
    );
    assert_eq!(brush_state::encode(&restored).unwrap(), bytes);
}
#[test]
fn groups_state20_invalid_downgrade_types_reference_cycles_and_projection_reject() {
    let good = value();
    // 22 is now a supported extension of grouped state, not a future version.
    let mut supported = good.clone();
    supported["version"] = json!(22);
    assert_eq!(
        decode(&supported).unwrap().organization(),
        grouped().organization()
    );
    for version in [1, 2, 19, 46] {
        let mut bad = good.clone();
        bad["version"] = json!(version);
        assert!(decode(&bad).is_err());
    }
    for replacement in [
        Value::Null,
        json!(false),
        json!({}),
        json!({"model":"future","roots":[],"groups":[]}),
    ] {
        let mut bad = good.clone();
        bad["organization"] = replacement;
        assert!(decode(&bad).is_err());
    }
    let mut bad = good.clone();
    bad["organization"]["model"] = json!({"ordered-tree-v1":null});
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad["organization"]["roots"][0]["kind"] = json!({"group":null});
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad["organization"]["extra"] = json!(1);
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad["organization"]["roots"][0]["kind"] = json!("folder");
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad["organization"]["roots"][0]["extra"] = json!(1);
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad["organization"]["groups"][0]["items"][0]["id"] = json!(id(1));
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad["organization"]["groups"][1]["items"] =
        json!([{"kind":"preset","id":"round-pressure"},{"kind":"preset","id":"round-pressure"}]);
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad["organization"]["groups"][1]["items"][0]["id"] = json!(id(999));
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad["presets"].as_array_mut().unwrap().swap(0, 1);
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad["organization"]["groups"][1]["name"] = json!(" ");
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad["organization"]["groups"][1]["id"] = json!(id(1));
    assert!(decode(&bad).is_err());
    // Invalid organization must fail before trying to read an unavailable image.
    bad["presets"][0]["maskResource"] = json!("0".repeat(64));
    assert!(matches!(decode(&bad), Err(paint_io::Error::Codec(_))));
    let mut bad = good.clone();
    bad["presets"].as_array_mut().unwrap().swap(0, 1);
    bad["presets"][0]["maskResource"] = json!("0".repeat(64));
    assert!(matches!(
        decode(&bad),
        Err(paint_io::Error::Invalid(
            "organization projection differs from brush rows"
        ))
    ));
}
#[test]
fn groups_state20_serialized_entry_limits_depth_and_explicit_flat_normalization() {
    let good = value();
    let mut bad = good.clone();
    bad["organization"]["roots"] =
        Value::Array(vec![json!({"kind":"preset","id":"round-pressure"}); 1281]);
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad["organization"]["groups"] =
        Value::Array(vec![json!({"id":id(1),"name":"G","items":[]}); 257]);
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad["organization"]["groups"][0]["items"] =
        Value::Array(vec![json!({"kind":"preset","id":"round-pressure"}); 1281]);
    assert!(decode(&bad).is_err());
    let mut bad = good.clone();
    bad["organization"]["groups"]=Value::Array((1..=9).map(|n|json!({"id":id(n),"name":"G","items":if n==9 {json!([{"kind":"preset","id":"round-pressure"}])} else {json!([{"kind":"group","id":id(n+1)}])}})).collect());
    assert!(decode(&bad).is_err());
    let c = BrushCatalog::default();
    let original = brush_state::encode(&c).unwrap();
    let mut flat: Value = serde_json::from_slice(&original).unwrap();
    flat["version"] = json!(20);
    flat["organization"] = json!({"model":"ordered-tree-v1","roots":c.ids().iter().map(|id|json!({"kind":"preset","id":id})).collect::<Vec<_>>(),"groups":[]});
    let restored = decode(&flat).unwrap();
    assert_eq!(brush_state::encode(&restored).unwrap(), original);
    flat["version"] = json!(19);
    assert!(decode(&flat).is_err());
}
