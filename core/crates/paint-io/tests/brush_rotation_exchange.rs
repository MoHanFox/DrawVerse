use paint_core::*;
use paint_io::{
    brush_state,
    vbr::{Package, Parameters, Preset},
    vbr_image::{self, ImageBundle},
};
use paint_task::CancellationToken;
use std::collections::BTreeMap;

fn rotation() -> RotationDynamics {
    RotationDynamics::new(
        true,
        PressureControl::new(
            PressureCurve::new_smooth(vec![(0., 1.), (0.5, 0.4), (1., 0.2)]).unwrap(),
            0.,
        )
        .unwrap(),
        PressureControl::default(),
        PressureControl::default(),
    )
}

fn package() -> Package {
    let settings = BrushSettings {
        dynamics: BrushDynamics {
            rotation: rotation(),
            ..Default::default()
        },
        ..Default::default()
    };
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            stroke_taper: None,
            wet_mix: None,
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "rotation".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 23,
            required_capabilities: vec!["procedural-tip".into(), "rotation-dynamics".into()],
            parameters: Parameters {
                source_matrix: None,
                fade_dynamics: None,
                diameter: 12.,
                spacing: 0.02,
                dab_opacity: 0.5,
                hardness: 1.,
                roundness: 1.,
                angle: 0.,
                flow: 1.,
                pressure_size: false,
                pressure_flow: false,
                pressure_dynamics: None,
                tip_dynamics: None,
                scatter_dynamics: None,
                shape_jitter: None,
                color_dynamics: None,
                airbrush: None,
                speed_dynamics: None,
                tangent_dynamics: None,
                distance_dynamics: None,
                tilt_dynamics: None,
                rotation_dynamics: Some(paint_io::brush_rotation::RotationParameters::from_core(
                    settings.dynamics.rotation.clone(),
                )),
                direction_dynamics: None,
            },
            smoothing: None,
            texture: None,
            dual_brush: None,
            tip_resource: None,
            source_resource: None,
        }],
    }
}

#[test]
fn state28_and_vbr134_preserve_rotation_dynamics() {
    let token = CancellationToken::default();
    let p = package();
    let bundle = ImageBundle::new(p.clone(), BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let reopened = vbr_image::decode(&bytes, &token).unwrap();
    assert_eq!(p.to_json().unwrap(), reopened.package.to_json().unwrap());
    assert_eq!(
        reopened.definition(0).unwrap().dynamics.rotation,
        rotation()
    );

    let mut catalog = BrushCatalog::default();
    let mut settings = catalog.selected().settings().clone();
    settings.dynamics.rotation = rotation();
    catalog.edit("round-pressure", settings).unwrap();
    let state = brush_state::encode(&catalog).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&state).unwrap();
    assert_eq!(value["version"], 28);
    let restored = brush_state::decode(&state, MAX_CATALOG_MASK_BYTES).unwrap();
    assert_eq!(restored.selected().settings().dynamics.rotation, rotation());
    value["version"] = serde_json::json!(27);
    assert!(
        brush_state::decode(&serde_json::to_vec(&value).unwrap(), MAX_CATALOG_MASK_BYTES).is_err()
    );
}
