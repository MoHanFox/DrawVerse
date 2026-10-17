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

fn dual() -> DualBrush {
    let tip = BrushTip {
        hardness: 0.4,
        flow: 0.7,
        pressure_size: false,
        pressure_flow: false,
        ..Default::default()
    };
    let secondary = SecondaryTip::new(7., tip, None, Default::default()).unwrap();
    let color = DualColor::new_with_model(
        DualColorModel::InterleavedStamp,
        true,
        true,
        false,
        DualColorOrder::SecondBehind,
        BlendMode::Multiply,
        BlendMode::Screen,
    )
    .unwrap();
    DualBrush::new(true, DualMode::Multiply, secondary).with_color(color)
}

fn package() -> Package {
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            stroke_taper: None,
            wet_mix: None,
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "interleaved".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 17,
            required_capabilities: vec![
                "procedural-tip".into(),
                "dual-cover".into(),
                "dual-color-stroke".into(),
                "dual-color-stamp".into(),
            ],
            parameters: Parameters {
                airbrush: None,
                speed_dynamics: None,
                tangent_dynamics: None,
                distance_dynamics: None,
                tilt_dynamics: None,
                rotation_dynamics: None,
                direction_dynamics: None,
                source_matrix: None,
                diameter: 12.,
                spacing: 0.25,
                dab_opacity: 0.7,
                hardness: 0.8,
                roundness: 1.,
                angle: 0.,
                flow: 0.6,
                pressure_size: true,
                pressure_flow: true,
                pressure_dynamics: None,
                tip_dynamics: None,
                scatter_dynamics: None,
                shape_jitter: None,
                color_dynamics: None,
            },
            smoothing: None,
            texture: None,
            dual_brush: Some(paint_io::brush_dual::DualParameters::from_core(&dual()).unwrap()),
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

fn draw(definition: &mut BrushDefinition) -> Vec<Pixel> {
    definition.brush.color = Pixel::from_straight([0.8, 0.2, 0.1, 0.7]).unwrap();
    definition.brush.background_color = Pixel::from_straight([0.1, 0.3, 0.8, 0.8]).unwrap();
    let mut d = Document::new(64, 64).unwrap();
    for shift in [0., 4.] {
        let mut p = InputPoint::new(12.5, 20.5 + shift, 0.4);
        p.timestamp_ns = 1_000_000;
        d.begin_brush_stroke(definition.clone(), p).unwrap();
        for (x, y, pressure, time) in [
            (24.5, 32.5 + shift, 0.8, 17_000_000),
            (48.5, 20.5 + shift, 1., 41_000_000),
        ] {
            let mut p = InputPoint::new(x, y, pressure);
            p.timestamp_ns = time;
            d.stroke_to(p).unwrap();
        }
        d.end_stroke().unwrap();
    }
    assert_eq!(d.history_depth(), (2, 0));
    (0..64)
        .flat_map(|y| (0..64).map(move |x| (x, y)))
        .map(|(x, y)| d.pixel(x, y))
        .collect()
}

#[test]
fn vbr128_preserves_interleaved_model_and_real_pixels() {
    let token = CancellationToken::default();
    let p = package();
    let mut definition = p.brushes[0].parameters.definition();
    definition.dual = dual();
    let reference = draw(&mut definition);

    let bundle = ImageBundle::new(p.clone(), BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let reopened = vbr_image::decode(&bytes, &token).unwrap();
    assert_eq!(p.to_json().unwrap(), reopened.package.to_json().unwrap());
    let mut restored = reopened.definition(0).unwrap();
    assert_eq!(reference, draw(&mut restored));
}

#[test]
fn interleaved_state_and_vbr_versions_are_strict() {
    let token = CancellationToken::default();
    let p = package();
    let bundle = ImageBundle::new(p.clone(), BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let mut downgraded = p;
    downgraded.brushes[0].behavior_version = 16;
    assert!(downgraded.validate().is_err());

    let mut catalog = BrushCatalog::default();
    let mut settings = catalog.selected().settings().clone();
    settings.dual = dual();
    catalog.edit("round-pressure", settings).unwrap();
    let state = brush_state::encode(&catalog).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&state).unwrap();
    assert_eq!(value["version"], 22);
    value["version"] = serde_json::json!(21);
    assert!(
        brush_state::decode(&serde_json::to_vec(&value).unwrap(), MAX_CATALOG_MASK_BYTES).is_err()
    );
    assert!(vbr_image::decode(&downgrade(&bytes, 27), &token).is_err());
}

#[test]
fn interleaved_package_rejects_independent_secondary_path() {
    let original = dual();
    let path = original.secondary().clone().with_spacing(0.25).unwrap();
    let path_dual = DualBrush::new(true, DualMode::Multiply, path).with_color(original.color());
    let mut p = package();
    p.brushes[0].dual_brush =
        Some(paint_io::brush_dual::DualParameters::from_core(&path_dual).unwrap());
    assert!(p.validate().is_err());
}
