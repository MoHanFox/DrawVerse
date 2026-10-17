use paint_core::*;
use paint_io::{
    brush_state,
    vbr::{Package, Parameters, Preset},
    vbr_image::{self, ImageBundle},
};
use paint_task::CancellationToken;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

fn settings() -> BrushSettings {
    BrushSettings {
        diameter: 12.,
        opacity: 0.4,
        spacing: 0.02,
        airbrush: Airbrush::new(true, 33.).unwrap(),
        ..Default::default()
    }
}

fn package() -> Package {
    let s = settings();
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            stroke_taper: None,
            wet_mix: None,
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "airbrush".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 18,
            required_capabilities: vec!["procedural-tip".into(), "airbrush".into()],
            parameters: Parameters {
                diameter: s.diameter,
                spacing: s.spacing,
                dab_opacity: s.opacity,
                hardness: s.tip.hardness,
                roundness: s.tip.roundness,
                angle: s.tip.angle,
                flow: s.tip.flow,
                pressure_size: s.tip.pressure_size,
                pressure_flow: s.tip.pressure_flow,
                pressure_dynamics: None,
                tip_dynamics: None,
                scatter_dynamics: None,
                shape_jitter: None,
                color_dynamics: None,
                airbrush: Some(paint_io::brush_airbrush::AirbrushParameters::from_core(
                    s.airbrush,
                )),
                speed_dynamics: None,
            },
            smoothing: None,
            texture: None,
            dual_brush: None,
            tip_resource: None,
            source_resource: None,
        }],
    }
}

fn downgrade(bytes: &[u8], minor: u32) -> Vec<u8> {
    let mut input = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut output = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..input.len() {
        let mut entry = input.by_index(i).unwrap();
        let mut body = Vec::new();
        entry.read_to_end(&mut body).unwrap();
        if entry.name() == "manifest.json" {
            let mut value: serde_json::Value = serde_json::from_slice(&body).unwrap();
            value["version"]["minor"] = minor.into();
            body = serde_json::to_vec(&value).unwrap();
        }
        output
            .start_file(entry.name(), SimpleFileOptions::default())
            .unwrap();
        output.write_all(&body).unwrap();
    }
    output.finish().unwrap().into_inner()
}

#[test]
fn state23_and_vbr129_preserve_airbrush() {
    let token = CancellationToken::default();
    let p = package();
    let bundle = ImageBundle::new(p.clone(), BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let reopened = vbr_image::decode(&bytes, &token).unwrap();
    assert_eq!(p.to_json().unwrap(), reopened.package.to_json().unwrap());
    assert_eq!(
        reopened.definition(0).unwrap().airbrush,
        Airbrush::new(true, 33.).unwrap()
    );
    assert!(vbr_image::decode(&downgrade(&bytes, 28), &token).is_err());

    let mut catalog = BrushCatalog::default();
    let mut s = catalog.selected().settings().clone();
    s.airbrush = Airbrush::new(true, 17.).unwrap();
    catalog.edit("round-pressure", s).unwrap();
    let state = brush_state::encode(&catalog).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&state).unwrap();
    assert_eq!(value["version"], 23);
    let restored = brush_state::decode(&state, MAX_CATALOG_MASK_BYTES).unwrap();
    assert_eq!(
        restored.selected().settings().airbrush,
        Airbrush::new(true, 17.).unwrap()
    );
    value["version"] = serde_json::json!(22);
    assert!(
        brush_state::decode(&serde_json::to_vec(&value).unwrap(), MAX_CATALOG_MASK_BYTES).is_err()
    );
}
