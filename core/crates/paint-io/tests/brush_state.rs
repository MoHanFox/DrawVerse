use paint_core::*;
use paint_io::brush_state;
#[test]
fn pressure_controls_including_redundant_nodes_are_roundtripped_without_discarding_structure() {
    let mut catalog = BrushCatalog::default();
    for points in [
        vec![(0., 0.), (0.5, 0.2), (1., 1.)],
        vec![(0., 0.), (0.5, 0.5), (1., 1.)],
    ] {
        let mut settings = catalog.selected().settings().clone();
        settings.dynamics.size =
            PressureControl::new(PressureCurve::new(points).unwrap(), 0.).unwrap();
        catalog.edit("round-pressure", settings).unwrap();
        let bytes = brush_state::encode(&catalog).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["version"],
            5
        );
        let restored = brush_state::decode(&bytes, MAX_CATALOG_MASK_BYTES).unwrap();
        assert_eq!(
            restored.selected().settings().dynamics,
            catalog.selected().settings().dynamics
        );
        assert_eq!(brush_state::encode(&restored).unwrap(), bytes);
    }
}
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
#[test]
fn v3_resource_identity_roundtrips_and_renders_without_original_source() {
    use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
    use paint_io::brush_resource::{Interpretation, ResourceStore};
    use paint_task::CancellationToken;
    let directory = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let resources = ResourceStore::open(directory.path().join("assets"), 1024).unwrap();
    let mut coverage = vec![255; 256];
    for y in 6..10 {
        for x in 6..10 {
            coverage[y * 16 + x] = 0;
        }
    }
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&coverage, 16, 16, ExtendedColorType::L8)
        .unwrap();
    let original = directory.path().join("source.png");
    std::fs::write(&original, &png).unwrap();
    let resource = resources
        .import_png(&png, Interpretation::Coverage, &token)
        .unwrap();
    assert_eq!(resource.mask.resource_id(), Some(resource.id.as_str()));
    let id = resource.id.clone();
    let mut catalog = BrushCatalog::default();
    let mut settings = catalog.selected().settings().clone();
    settings.diameter = 16.;
    settings.tip.angle = 45.;
    settings.mask = Some(resource.mask);
    catalog.edit("round-pressure", settings).unwrap();
    let state = brush_state::encode(&catalog).unwrap();
    assert!(String::from_utf8_lossy(&state).contains(&id));
    assert!(!String::from_utf8_lossy(&state).contains("source.png"));
    assert!(brush_state::decode(&state, 1024).is_err());
    std::fs::remove_file(original).unwrap();
    drop(catalog);
    let restored = brush_state::decode_with_resources(&state, 1024, &resources, &token).unwrap();
    assert_eq!(brush_state::encode(&restored).unwrap(), state);
    let mut document = Document::new(50, 50).unwrap();
    document
        .begin_brush_stroke(
            restored.selected().settings().definition(),
            InputPoint::new(25.5, 25.5, 1.),
        )
        .unwrap();
    document.end_stroke().unwrap();
    assert_eq!(document.pixel(25, 25).alpha(), 0.);
    assert!(document.pixel(25, 35).alpha() > 0.9);
    let mut bad: serde_json::Value = serde_json::from_slice(&state).unwrap();
    bad["version"] = serde_json::json!(2);
    assert!(brush_state::decode_with_resources(
        &serde_json::to_vec(&bad).unwrap(),
        1024,
        &resources,
        &token
    )
    .is_err());
    bad["version"] = serde_json::json!(3);
    bad["presets"][0]["maskResource"] = serde_json::json!("../source.png");
    assert!(brush_state::decode_with_resources(
        &serde_json::to_vec(&bad).unwrap(),
        1024,
        &resources,
        &token
    )
    .is_err());
    assert!(brush_state::decode_with_resources(&state, 100, &resources, &token).is_err());
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
}
