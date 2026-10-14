use paint_core::*;
use paint_io::{brush_pressure::PressureParameters, brush_state};
#[test]
fn color_settings_cannot_be_lost_through_existing_serializers() {
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
        assert!(brush_state::encode(&catalog).is_err());
        assert_eq!(catalog.selected().settings().dynamics, dynamics);
        assert!(serde_json::to_vec(&PressureParameters::from_core(&dynamics)).is_err());
    }
}
