use paint_core::*;
use paint_io::{
    brush_dual::DualParameters,
    brush_resource::{encode_mask, hash, ResourceStore},
    brush_state,
    vbr::{Package, Parameters, Preset},
    vbr_image::{self, ImageBundle},
};
use paint_task::CancellationToken;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
    sync::Arc,
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};
fn pixels(mut definition: BrushDefinition) -> Vec<Pixel> {
    definition.brush.color = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
    let mut d = Document::new(32, 32).unwrap();
    for _ in 0..2 {
        let mut p = InputPoint::new(8.5, 16.5, 1.);
        p.capabilities = 6;
        p.rotation = 20.;
        p.tilt_x = 30.;
        d.begin_brush_stroke(definition.clone(), p).unwrap();
        p.x = 24.5;
        d.stroke_to(p).unwrap();
        d.end_stroke().unwrap();
    }
    (0..32)
        .flat_map(|y| (0..32).map(move |x| (x, y)))
        .map(|(x, y)| d.pixel(x, y))
        .collect()
}
fn package(s: &BrushSettings) -> Package {
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            wet_mix: None,
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "dual".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 9,
            required_capabilities: vec![
                "image-tip".into(),
                "texture-cover".into(),
                "dual-cover".into(),
            ],
            tip_resource: s
                .mask
                .as_ref()
                .and_then(|m| m.resource_id())
                .map(str::to_owned),
            texture: Some(
                paint_io::brush_texture::TextureParameters::from_core(&s.texture).unwrap(),
            ),
            smoothing: None,
            dual_brush: Some(DualParameters::from_core(&s.dual).unwrap()),
            source_resource: None,
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
            },
        }],
    }
}
fn downgrade(bytes: &[u8]) -> Vec<u8> {
    let mut input = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut output = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..input.len() {
        let mut e = input.by_index(i).unwrap();
        let mut b = Vec::new();
        e.read_to_end(&mut b).unwrap();
        if e.name() == "manifest.json" {
            let mut v: serde_json::Value = serde_json::from_slice(&b).unwrap();
            v["version"]["minor"] = 9.into();
            b = serde_json::to_vec(&v).unwrap();
        }
        output
            .start_file(e.name(), SimpleFileOptions::default())
            .unwrap();
        output.write_all(&b).unwrap();
    }
    output.finish().unwrap().into_inner()
}
#[test]
fn state12_vbr110_restore_both_models_modes_disabled_dynamics_and_shared_pixels() {
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let store = ResourceStore::open(temp.path().join("assets"), MAX_CATALOG_MASK_BYTES).unwrap();
    let png = encode_mask(&BrushMask::new(4, 1, vec![0, 128, 255, 128]).unwrap()).unwrap();
    let id = hash(&png);
    store.put_verified(&id, &png, &token).unwrap();
    let mask = store.load(&id, &token).unwrap().mask;
    let texture = Arc::new(TextureResource::new(mask.clone()));
    let budget = mask.resident_bytes() + texture.mip_bytes();
    for path in [false, true] {
        for enabled in [false, true] {
            for mode in [
                DualMode::Multiply,
                DualMode::Darken,
                DualMode::Lighten,
                DualMode::Subtract,
                DualMode::Difference,
                DualMode::Screen,
            ] {
                let tip = BrushTip {
                    pressure_size: false,
                    pressure_flow: false,
                    ..Default::default()
                };
                let d = BrushDynamics {
                    size: PressureControl::new(
                        PressureCurve::new(vec![(0., 0.2), (1., 0.8)]).unwrap(),
                        0.3,
                    )
                    .unwrap(),
                    tip: TipDynamics::new(AngleControl::PenRotation, true, 0.2).unwrap(),
                    scatter: ScatterDynamics::new(true, 0.2, true, 2, 0.5, u64::MAX).unwrap(),
                    shape_jitter: ShapeJitter::new(false, 0.5, 0.2, 0.3, 0.4, 0.2, u64::MAX)
                        .unwrap(),
                    ..Default::default()
                };
                let mut second = SecondaryTip::new(
                    4.,
                    BrushTip {
                        roundness: 0.6,
                        angle: 13.,
                        flow: 0.8,
                        pressure_size: true,
                        pressure_flow: false,
                        ..Default::default()
                    },
                    Some(mask.clone()),
                    d,
                )
                .unwrap();
                if path {
                    second = second.with_spacing(0.01).unwrap();
                }
                let s = BrushSettings {
                    diameter: 12.,
                    opacity: 0.4,
                    spacing: 0.1,
                    tip,
                    mask: Some(mask.clone()),
                    texture: BrushTexture::new(
                        true,
                        TextureSettings::new(TextureSpace::Document, 1., 0., 0.5, false, 0., 0.)
                            .unwrap(),
                        Some(texture.clone()),
                    )
                    .unwrap(),
                    dual: DualBrush::new(enabled, mode, second),
                    ..Default::default()
                };
                let expected = pixels(s.definition());
                assert!(expected.iter().any(|p| p.alpha() > 0.));
                let mut c = BrushCatalog::default();
                c.edit("round-pressure", s.clone()).unwrap();
                let state = brush_state::encode(&c).unwrap();
                assert_eq!(
                    serde_json::from_slice::<serde_json::Value>(&state).unwrap()["version"],
                    12
                );
                let restored =
                    brush_state::decode_with_resources(&state, budget, &store, &token).unwrap();
                assert_eq!(
                    pixels(restored.selected().settings().definition()),
                    expected
                );
                assert_eq!(brush_state::encode(&restored).unwrap(), state);
                assert!(Arc::ptr_eq(
                    restored.selected().settings().mask.as_ref().unwrap(),
                    restored
                        .selected()
                        .settings()
                        .dual
                        .secondary()
                        .mask()
                        .unwrap()
                ));
                assert!(
                    brush_state::decode_with_resources(&state, budget - 1, &store, &token).is_err()
                );
                let bundle = ImageBundle::new(
                    package(&s),
                    BTreeMap::from([(id.clone(), png.clone())]),
                    &token,
                )
                .unwrap();
                let bytes = vbr_image::encode(&bundle, &token).unwrap();
                let recovered = vbr_image::decode(&bytes, &token).unwrap();
                assert_eq!(recovered.assets().len(), 1);
                let actual = recovered.definition(0).unwrap();
                assert!(Arc::ptr_eq(
                    actual.mask.as_ref().unwrap(),
                    actual.dual.secondary().mask().unwrap()
                ));
                assert_eq!(pixels(actual), expected);
                assert_eq!(recovered.assets()[&id].bytes(), png);
                assert!(vbr_image::decode(&downgrade(&bytes), &token).is_err());
            }
        }
    }
}
#[test]
fn dual_models_ranges_required_nulls_resources_and_capabilities_reject() {
    let config = DualParameters::from_core(&DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::default().with_spacing(0.01).unwrap(),
    ))
    .unwrap();
    let valid = serde_json::to_value(&config).unwrap();
    let mut package = serde_json::json!({"packageId":"12345678-1234-4234-8234-123456789abc","brushes":[{"id":"12345678-1234-4234-8234-123456789abd","name":"dual","engineProfile":"drawverse.brush","behaviorVersion":9,"requiredCapabilities":["procedural-tip","dual-cover"],"dualBrush":valid,"parameters":{"diameter":16,"spacing":0.01,"dabOpacity":1,"hardness":1,"roundness":1,"angle":0,"flow":1,"pressureSize":false,"pressureFlow":false}}]});
    assert!(Package::from_json(&serde_json::to_vec(&package).unwrap()).is_ok());
    package["brushes"][0]["behaviorVersion"] = 8.into();
    assert!(Package::from_json(&serde_json::to_vec(&package).unwrap()).is_err());
    package["brushes"][0]["behaviorVersion"] = 9.into();
    package["brushes"][0]["requiredCapabilities"] = serde_json::json!(["procedural-tip"]);
    assert!(Package::from_json(&serde_json::to_vec(&package).unwrap()).is_err());
    let mut opacity = valid.clone();
    opacity["pressureDynamics"] = serde_json::to_value(
        paint_io::brush_pressure::PressureParameters::from_core(&BrushDynamics {
            pressure_opacity: true,
            ..Default::default()
        }),
    )
    .unwrap();
    assert!(serde_json::from_value::<DualParameters>(opacity)
        .unwrap()
        .validate()
        .is_err());
    for (key, value) in [
        ("model", serde_json::json!("other")),
        ("mode", serde_json::json!("overlay")),
        ("spacing", serde_json::json!(0.009999999)),
        ("diameter", serde_json::json!(512.00000001)),
        ("roundness", serde_json::json!(0.009999999)),
        ("angle", serde_json::json!(180.00000001)),
        ("resource", serde_json::json!("../bad")),
        ("enabled", serde_json::json!(1)),
        ("extra", serde_json::json!(0)),
    ] {
        let mut bad = valid.clone();
        bad[key] = value;
        assert!(
            serde_json::from_value::<DualParameters>(bad).map_or(true, |p| p.validate().is_err())
        );
    }
    for key in ["spacing", "resource"] {
        let mut bad = valid.clone();
        bad.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<DualParameters>(bad).is_err());
    }
    let mut pair = valid.clone();
    pair["model"] = "paired-dab-v1".into();
    assert!(serde_json::from_value::<DualParameters>(pair)
        .unwrap()
        .validate()
        .is_err());
    let wrong = Arc::new(BrushMask::new(1, 1, vec![255]).unwrap());
    assert!(config.bind(Some(wrong)).is_err());
    let mut c = BrushCatalog::default();
    c.edit(
        "round-pressure",
        BrushSettings {
            dual: DualBrush::new(
                false,
                DualMode::Multiply,
                SecondaryTip::new(
                    4.,
                    BrushTip::default(),
                    Some(Arc::new(BrushMask::new(1, 1, vec![255]).unwrap())),
                    Default::default(),
                )
                .unwrap(),
            ),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(brush_state::encode(&c).is_err());
    let mut state: serde_json::Value =
        serde_json::from_slice(&brush_state::encode(&BrushCatalog::default()).unwrap()).unwrap();
    state["presets"][0]["dualBrush"] = serde_json::Value::Null;
    state["version"] = 12.into();
    assert!(
        brush_state::decode(&serde_json::to_vec(&state).unwrap(), MAX_CATALOG_MASK_BYTES).is_err()
    );
}
#[test]
fn published_dual_contract_preserves_models_and_resource_limits() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-dual-v1.10.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        10
    );
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["assets"]["maxItems"],
        6144
    );
    assert_eq!(
        schema["$defs"]["dualBrush"]["properties"]["model"]["enum"],
        serde_json::json!(["paired-dab-v1", "path-coverage-v1"])
    );
    assert!(schema["$defs"]["dualBrush"]["required"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("resource")));
}
