use paint_core::*;
use paint_io::{
    brush_state,
    vbr::{Package, Parameters, Preset},
    vbr_image::{self, ImageBundle},
};
use paint_task::CancellationToken;
use std::collections::BTreeMap;

fn matrix() -> DynamicsSourceMatrix {
    DynamicsSourceMatrix::new(
        DynamicsSource::Speed,
        DynamicsSource::Distance,
        DynamicsSource::Tilt,
    )
}

fn package() -> Package {
    let settings = BrushSettings {
        dynamics: BrushDynamics {
            matrix: matrix(),
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
            name: "source-matrix".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 25,
            required_capabilities: vec!["procedural-tip".into(), "dynamics-source-matrix".into()],
            parameters: Parameters {
                transfer_jitter: None,
                source_matrix: Some(
                    paint_io::brush_source_matrix::SourceMatrixParameters::from_core(
                        settings.dynamics.matrix,
                    ),
                ),
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
fn state32_and_vbr138_preserve_dynamics_source_matrix() {
    let token = CancellationToken::default();
    let p = package();
    let json: serde_json::Value = serde_json::from_slice(&p.to_json().unwrap()).unwrap();
    assert_eq!(
        json["brushes"][0]["parameters"]["sourceMatrix"]["model"],
        "source-matrix-v1"
    );
    assert_eq!(
        json["brushes"][0]["parameters"]["sourceMatrix"]["size"],
        "speed"
    );
    let bundle = ImageBundle::new(p.clone(), BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_reader(archive.by_name("manifest.json").unwrap()).unwrap();
    assert_eq!(manifest["version"]["minor"], 38);
    let reopened = vbr_image::decode(&bytes, &token).unwrap();
    assert_eq!(p.to_json().unwrap(), reopened.package.to_json().unwrap());
    assert_eq!(reopened.definition(0).unwrap().dynamics.matrix, matrix());

    let mut catalog = BrushCatalog::default();
    let mut settings = catalog.selected().settings().clone();
    settings.dynamics.matrix = matrix();
    catalog.edit("round-pressure", settings).unwrap();
    let state = brush_state::encode(&catalog).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&state).unwrap();
    assert_eq!(value["version"], 32);
    assert_eq!(value["presets"][0]["sourceMatrix"]["opacity"], "tilt");
    let restored = brush_state::decode(&state, MAX_CATALOG_MASK_BYTES).unwrap();
    assert_eq!(restored.selected().settings().dynamics.matrix, matrix());
    value["version"] = serde_json::json!(31);
    assert!(
        brush_state::decode(&serde_json::to_vec(&value).unwrap(), MAX_CATALOG_MASK_BYTES).is_err()
    );
}
