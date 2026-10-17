use paint_core::*;
use paint_io::{
    brush_state,
    vbr::{Package, Parameters, Preset},
    vbr_image::{self, ImageBundle},
};
use paint_task::CancellationToken;
use std::collections::BTreeMap;

fn tilt() -> TiltDynamics {
    TiltDynamics::new(
        true,
        PressureControl::new(
            PressureCurve::new_smooth(vec![(0., 1.), (0.5, 0.6), (1., 0.2)]).unwrap(),
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
            tilt: tilt(),
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
            name: "tilt".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 22,
            required_capabilities: vec!["procedural-tip".into(), "tilt-dynamics".into()],
            parameters: Parameters {
                source_matrix: None,
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
                tilt_dynamics: Some(paint_io::brush_tilt::TiltParameters::from_core(
                    settings.dynamics.tilt.clone(),
                )),
                rotation_dynamics: None,
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
fn state27_and_vbr133_preserve_tilt_dynamics() {
    let token = CancellationToken::default();
    let p = package();
    let bundle = ImageBundle::new(p.clone(), BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let reopened = vbr_image::decode(&bytes, &token).unwrap();
    assert_eq!(p.to_json().unwrap(), reopened.package.to_json().unwrap());
    assert_eq!(reopened.definition(0).unwrap().dynamics.tilt, tilt());

    let mut catalog = BrushCatalog::default();
    let mut settings = catalog.selected().settings().clone();
    settings.dynamics.tilt = tilt();
    catalog.edit("round-pressure", settings).unwrap();
    let state = brush_state::encode(&catalog).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&state).unwrap();
    assert_eq!(value["version"], 27);
    let restored = brush_state::decode(&state, MAX_CATALOG_MASK_BYTES).unwrap();
    assert_eq!(restored.selected().settings().dynamics.tilt, tilt());
    value["version"] = serde_json::json!(26);
    assert!(
        brush_state::decode(&serde_json::to_vec(&value).unwrap(), MAX_CATALOG_MASK_BYTES).is_err()
    );
}
