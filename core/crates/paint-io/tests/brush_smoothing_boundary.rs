use paint_core::*;
#[test]
fn old_state_cannot_silently_drop_enabled_or_closed_smoothing() {
    for enabled in [false, true] {
        let mut catalog = BrushCatalog::default();
        catalog
            .edit(
                "round-pressure",
                BrushSettings {
                    smoothing: BrushSmoothing::new(enabled, 24., 12., 0.7, 3.).unwrap(),
                    ..Default::default()
                },
            )
            .unwrap();
        assert!(matches!(
            paint_io::brush_state::encode(&catalog),
            Err(paint_io::Error::Unsupported(_))
        ));
    }
    let bytes = paint_io::brush_state::encode(&BrushCatalog::default()).unwrap();
    let restored = paint_io::brush_state::decode(&bytes, MAX_CATALOG_MASK_BYTES).unwrap();
    assert!(restored.selected().settings().smoothing.is_default());
    assert_eq!(bytes, paint_io::brush_state::encode(&restored).unwrap());
}
