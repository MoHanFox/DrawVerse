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
