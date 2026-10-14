use paint_core::*;
use paint_io::{
    brush_pressure::PressureParameters,
    brush_state,
    vbr::{Package, Parameters, Preset},
};
#[test]
fn unimplemented_shape_io_cannot_discard_enabled_or_closed_configuration() {
    for enabled in [true, false] {
        let dynamics = BrushDynamics {
            shape_jitter: ShapeJitter::new(enabled, 0.5, 0.2, 0.7, 0.6, 0.1, u64::MAX).unwrap(),
            ..Default::default()
        };
        assert!(serde_json::to_vec(&PressureParameters::from_core(&dynamics)).is_err());
        let mut catalog = BrushCatalog::default();
        let settings = BrushSettings {
            dynamics: dynamics.clone(),
            ..Default::default()
        };
        catalog.edit("round-pressure", settings).unwrap();
        assert!(brush_state::encode(&catalog).is_err());
        assert_eq!(catalog.selected().settings().dynamics, dynamics);
        let package = Package {
            package_id: "12345678-1234-4234-8234-123456789abc".into(),
            brushes: vec![Preset {
                id: "12345678-1234-4234-8234-123456789abd".into(),
                name: "shape".into(),
                engine_profile: "drawverse.brush".into(),
                behavior_version: 2,
                required_capabilities: vec!["procedural-tip".into(), "pressure-curves".into()],
                tip_resource: None,
                source_resource: None,
                parameters: Parameters {
                    diameter: 24.,
                    spacing: 0.15,
                    dab_opacity: 1.,
                    hardness: 1.,
                    roundness: 1.,
                    angle: 0.,
                    flow: 1.,
                    pressure_size: true,
                    pressure_flow: true,
                    pressure_dynamics: Some(PressureParameters::from_core(&dynamics)),
                    tip_dynamics: None,
                    scatter_dynamics: None,
                },
            }],
        };
        assert!(package.validate().is_err());
        assert!(package.to_json().is_err());
    }
}
