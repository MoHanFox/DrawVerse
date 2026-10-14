use paint_core::*;
use paint_io::{brush_pressure::PressureParameters, brush_state};
#[test]
fn state10_preserves_color_but_pressure_payload_cannot_hide_it() {
    for enabled in [true, false] {
        let dynamics = BrushDynamics {
            color: ColorDynamics::new(enabled, 1., 0.5, 0.3, 0.2, true, u64::MAX).unwrap(),
            ..Default::default()
        };
        let mut catalog = BrushCatalog::default();
        catalog
            .edit(
                "round-pressure",
                BrushSettings {
                    dynamics: dynamics.clone(),
                    ..Default::default()
                },
            )
            .unwrap();
        let encoded = brush_state::encode(&catalog).unwrap();
        assert_eq!(
            brush_state::decode(&encoded, MAX_CATALOG_MASK_BYTES)
                .unwrap()
                .selected()
                .settings()
                .dynamics,
            dynamics
        );
        assert_eq!(catalog.selected().settings().dynamics, dynamics);
        assert!(serde_json::to_vec(&PressureParameters::from_core(&dynamics)).is_err());
    }
}
