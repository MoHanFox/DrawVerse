use paint_core::*;
use paint_io::brush_state;
#[test]
fn v1_v2_migration_and_canonical_parameters_are_executable() {
    let mut c = BrushCatalog::default();
    let mut s = c.selected().settings().clone();
    s.tip.roundness = 0.2;
    s.tip.angle = 90.;
    c.edit("round-pressure", s).unwrap();
    let json = brush_state::encode(&c).unwrap();
    let restored = brush_state::decode(&json, MAX_CATALOG_MASK_BYTES).unwrap();
    assert_eq!(restored.selected().settings().tip.roundness, 0.2);
    assert_eq!(brush_state::encode(&restored).unwrap(), json);
    let mut root: serde_json::Value = serde_json::from_slice(&json).unwrap();
    root["version"] = serde_json::json!(1);
    let old =
        brush_state::decode(&serde_json::to_vec(&root).unwrap(), MAX_CATALOG_MASK_BYTES).unwrap();
    assert_eq!(old.selected().settings().tip, BrushTip::default());
    let mut d = Document::new(40, 40).unwrap();
    d.begin_brush_stroke(
        restored.selected().settings().definition(),
        InputPoint::new(20.5, 20.5, 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    assert!(d.pixel(20, 27).alpha() > 0.9);
    assert_eq!(d.pixel(27, 20).alpha(), 0.);
}
#[test]
fn invalid_partial_duplicate_future_state_rejects_and_images_are_never_discarded() {
    let c = BrushCatalog::default();
    let original = brush_state::encode(&c).unwrap();
    for mutation in 0..5 {
        let mut root: serde_json::Value = serde_json::from_slice(&original).unwrap();
        match mutation {
            0 => root["presets"][0]["roundness"] = serde_json::json!(0),
            1 => {
                root["presets"].as_array_mut().unwrap().pop();
            }
            2 => root["presets"][1]["id"] = root["presets"][0]["id"].clone(),
            3 => root["presets"][0]["wetMix"] = serde_json::json!({"enabled":true}),
            _ => root["selected"] = serde_json::json!("unknown"),
        };
        assert!(
            brush_state::decode(&serde_json::to_vec(&root).unwrap(), MAX_CATALOG_MASK_BYTES)
                .is_err()
        );
    }
    let mut c = c;
    let mut s = c.selected().settings().clone();
    s.mask = Some(std::sync::Arc::new(
        BrushMask::new(8, 8, vec![255; 64]).unwrap(),
    ));
    c.edit("round-pressure", s).unwrap();
    assert!(brush_state::encode(&c).is_err());
}
