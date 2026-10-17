use paint_core::*;
#[test]
fn dual_color_keeps_both_module_switches_and_all_blend_names() {
    for dual_enabled in [false, true] {
        for enabled in [false, true] {
            let settings = DualColor::new(
                enabled,
                true,
                true,
                DualColorOrder::SecondBehind,
                BlendMode::Normal,
                BlendMode::Normal,
            )
            .unwrap();
            let dual = DualBrush::new(dual_enabled, DualMode::Multiply, SecondaryTip::default())
                .with_color(settings);
            let dto = paint_io::brush_dual::DualParameters::from_core(&dual).unwrap();
            assert_eq!(dto.bind(None).unwrap().color(), settings);
            let mut c = BrushCatalog::default();
            let mut p = c.selected().settings().clone();
            p.dual = dual;
            c.edit("round-pressure", p).unwrap();
            let bytes = paint_io::brush_state::encode(&c).unwrap();
            let restored = paint_io::brush_state::decode(&bytes, MAX_CATALOG_MASK_BYTES).unwrap();
            assert_eq!(restored.selected().settings().dual.color(), settings);
            assert_eq!(restored.selected().settings().dual.enabled(), dual_enabled);
        }
    }
}

#[test]
fn all_named_modes_preserve_exact_enums_and_default_bytes_remain_old() {
    for i in 0..27 {
        let mode = BlendMode::from_id(i).unwrap();
        let settings = DualColor::new(
            false,
            true,
            false,
            DualColorOrder::SecondInFront,
            mode,
            mode,
        )
        .unwrap();
        let dto = paint_io::brush_dual_color::ColorParameters::from_core(settings);
        let bytes = serde_json::to_vec(&dto).unwrap();
        let restored: paint_io::brush_dual_color::ColorParameters =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored.core(), settings);
    }
    let c = BrushCatalog::default();
    let bytes = paint_io::brush_state::encode(&c).unwrap();
    let reopened = paint_io::brush_state::decode(&bytes, MAX_CATALOG_MASK_BYTES).unwrap();
    assert_eq!(bytes, paint_io::brush_state::encode(&reopened).unwrap());
    assert!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["version"]
            .as_u64()
            .unwrap()
            < 16
    );
}

#[test]
fn interleaved_model_roundtrips_and_requires_state22() {
    let settings = DualColor::new_with_model(
        DualColorModel::InterleavedStamp,
        true,
        true,
        false,
        DualColorOrder::SecondBehind,
        BlendMode::Multiply,
        BlendMode::Screen,
    )
    .unwrap();
    let dto = paint_io::brush_dual_color::ColorParameters::from_core(settings);
    let json = serde_json::to_value(dto).unwrap();
    assert_eq!(json["model"], "interleaved-stamp-v1");
    let restored: paint_io::brush_dual_color::ColorParameters =
        serde_json::from_value(json).unwrap();
    assert_eq!(restored.core(), settings);
    let dual =
        DualBrush::new(true, DualMode::Multiply, SecondaryTip::default()).with_color(settings);
    let mut c = BrushCatalog::default();
    let mut p = c.selected().settings().clone();
    p.dual = dual;
    c.edit("round-pressure", p).unwrap();
    let bytes = paint_io::brush_state::encode(&c).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["version"], 22);
    let restored = paint_io::brush_state::decode(&bytes, MAX_CATALOG_MASK_BYTES).unwrap();
    assert_eq!(
        restored.selected().settings().dual.color().model(),
        DualColorModel::InterleavedStamp
    );
    value["version"] = serde_json::json!(21);
    assert!(paint_io::brush_state::decode(
        &serde_json::to_vec(&value).unwrap(),
        MAX_CATALOG_MASK_BYTES
    )
    .is_err());
}

#[test]
fn interleaved_state_rejects_an_enabled_independent_path() {
    let path = SecondaryTip::default().with_spacing(0.25).unwrap();
    let separated = DualColor::new(
        true,
        true,
        false,
        DualColorOrder::SecondBehind,
        BlendMode::Normal,
        BlendMode::Normal,
    )
    .unwrap();
    let mut c = BrushCatalog::default();
    let mut p = c.selected().settings().clone();
    p.dual = DualBrush::new(true, DualMode::Multiply, path).with_color(separated);
    c.edit("round-pressure", p).unwrap();
    let bytes = paint_io::brush_state::encode(&c).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["version"] = serde_json::json!(22);
    value["presets"][0]["dualBrush"]["color"]["model"] = serde_json::json!("interleaved-stamp-v1");
    assert!(paint_io::brush_state::decode(
        &serde_json::to_vec(&value).unwrap(),
        MAX_CATALOG_MASK_BYTES
    )
    .is_err());
}
