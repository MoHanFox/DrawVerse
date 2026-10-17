use paint_core::*;
use paint_io::{
    brush_resource::{encode_mask, hash, ResourceStore},
    brush_state,
    brush_texture::TextureParameters,
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
fn package(s: &BrushSettings, id: &str) -> Package {
    let p = Preset {
        stroke_taper: None,
        wet_mix: None,
        id: "12345678-1234-4234-8234-123456789abc".into(),
        name: "texture".into(),
        engine_profile: "drawverse.brush".into(),
        behavior_version: 8,
        required_capabilities: vec!["image-tip".into(), "texture-cover".into()],
        tip_resource: Some(id.into()),
        smoothing: None,
        dual_brush: None,
        texture: Some(TextureParameters::from_core(&s.texture).unwrap()),
        source_resource: None,
        parameters: Parameters {
            airbrush: None,
            speed_dynamics: None,
            tangent_dynamics: None,
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
    };
    let mut second = p.clone();
    second.id = "12345678-1234-4234-8234-123456789abd".into();
    Package {
        package_id: "12345678-1234-4234-8234-123456789abe".into(),
        brushes: vec![p, second],
    }
}
fn pixels(mut definition: BrushDefinition) -> Vec<Pixel> {
    definition.brush.color = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
    let mut d = Document::new(96, 96).unwrap();
    for _ in 0..2 {
        d.begin_brush_stroke(definition.clone(), InputPoint::new(16.5, 48.5, 1.))
            .unwrap();
        d.stroke_to(InputPoint::new(80.5, 48.5, 1.)).unwrap();
        d.end_stroke().unwrap();
    }
    let d = &d;
    (0..96)
        .flat_map(|y| (0..96).map(move |x| d.pixel(x, y)))
        .collect()
}
fn rewrite_minor(bytes: &[u8], minor: u32) -> Vec<u8> {
    let mut input = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..input.len() {
        let mut entry = input.by_index(i).unwrap();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).unwrap();
        if entry.name() == "manifest.json" {
            let mut json: serde_json::Value = serde_json::from_slice(&data).unwrap();
            json["version"]["minor"] = minor.into();
            data = serde_json::to_vec(&json).unwrap();
        }
        out.start_file(entry.name(), SimpleFileOptions::default())
            .unwrap();
        out.write_all(&data).unwrap();
    }
    out.finish().unwrap().into_inner()
}
fn downgrade(bytes: &[u8]) -> Vec<u8> {
    rewrite_minor(bytes, 8)
}
#[test]
fn state11_vbr19_share_tip_texture_and_restore_actual_pixels() {
    let token = CancellationToken::default();
    let temp = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(temp.path().join("assets"), MAX_CATALOG_MASK_BYTES).unwrap();
    let png = encode_mask(&BrushMask::new(4, 1, vec![0, 255, 0, 255]).unwrap()).unwrap();
    let id = hash(&png);
    store.put_verified(&id, &png, &token).unwrap();
    let mask = store.load(&id, &token).unwrap().mask;
    let resource = Arc::new(TextureResource::new(mask.clone()));
    let budget = mask.resident_bytes() + resource.mip_bytes();
    for enabled in [true, false] {
        for space in [TextureSpace::Document, TextureSpace::Tip] {
            let s = BrushSettings {
                diameter: 16.,
                opacity: 0.4,
                spacing: 0.1,
                tip: BrushTip {
                    pressure_size: false,
                    pressure_flow: false,
                    ..Default::default()
                },
                mask: Some(mask.clone()),
                texture: BrushTexture::new(
                    enabled,
                    TextureSettings::new(space, 1.3, 23., 0.7, true, 0.2, -0.4).unwrap(),
                    Some(resource.clone()),
                )
                .unwrap(),
                ..Default::default()
            };
            let expected = pixels(s.definition());
            assert!(expected.iter().any(|p| p.alpha() > 0.));
            let mut c = BrushCatalog::default();
            c.edit("round-pressure", s.clone()).unwrap();
            c.save_copy("12345678-1234-4234-8234-123456789abc".into(), "copy".into())
                .unwrap();
            let encoded = brush_state::encode(&c).unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&encoded).unwrap()["version"],
                11
            );
            let restored =
                brush_state::decode_with_resources(&encoded, budget, &store, &token).unwrap();
            assert_eq!(
                pixels(restored.selected().settings().definition()),
                expected
            );
            assert!(Arc::ptr_eq(
                restored
                    .preset("round-pressure")
                    .unwrap()
                    .settings()
                    .texture
                    .resource()
                    .unwrap(),
                restored.selected().settings().texture.resource().unwrap()
            ));
            assert!(
                brush_state::decode_with_resources(&encoded, budget - 1, &store, &token).is_err()
            );
            let bundle = ImageBundle::new(
                package(&s, &id),
                BTreeMap::from([(id.clone(), png.clone())]),
                &token,
            )
            .unwrap();
            let bytes = vbr_image::encode(&bundle, &token).unwrap();
            let restored = vbr_image::decode(&bytes, &token).unwrap();
            assert_eq!(restored.assets().len(), 1);
            let a = restored.definition(0).unwrap();
            let b = restored.definition(1).unwrap();
            assert!(Arc::ptr_eq(
                a.texture.resource().unwrap(),
                b.texture.resource().unwrap()
            ));
            assert!(Arc::ptr_eq(
                a.mask.as_ref().unwrap(),
                a.texture.resource().unwrap().source()
            ));
            assert_eq!(pixels(a), expected);
            assert_eq!(restored.assets()[&id].bytes(), png);
            assert!(vbr_image::decode(&downgrade(&bytes), &token).is_err());
        }
    }
}
#[test]
fn invalid_texture_fields_and_missing_references_reject() {
    let resource = Arc::new(TextureResource::new(Arc::new(
        BrushMask::new(2, 1, vec![0, 255])
            .unwrap()
            .with_resource_id("a".repeat(64))
            .unwrap(),
    )));
    let texture = BrushTexture::new(true, TextureSettings::default(), Some(resource)).unwrap();
    let original = serde_json::to_value(TextureParameters::from_core(&texture).unwrap()).unwrap();
    for (k, v) in [
        ("model", serde_json::json!("unknown")),
        ("space", serde_json::json!("world")),
        ("scale", serde_json::json!(0.0099999999)),
        ("depth", serde_json::json!(1.00000001)),
        ("angle", serde_json::json!(180.00000001)),
        ("resource", serde_json::Value::Null),
        ("enabled", serde_json::json!(1)),
        ("extra", serde_json::json!(0)),
    ] {
        let mut bad = original.clone();
        bad[k] = v;
        let t: std::result::Result<TextureParameters, _> = serde_json::from_value(bad);
        assert!(t.is_err() || t.unwrap().validate().is_err());
    }
    let mut missing = original.clone();
    missing.as_object_mut().unwrap().remove("resource");
    assert!(serde_json::from_value::<TextureParameters>(missing).is_err());
    let minimum = BrushTexture::new(
        true,
        TextureSettings::new(TextureSpace::Document, 0.01, 0., 1., false, 0., 0.).unwrap(),
        texture.resource().cloned(),
    )
    .unwrap();
    TextureParameters::from_core(&minimum)
        .unwrap()
        .validate()
        .unwrap();
    let settings = BrushSettings {
        texture,
        ..Default::default()
    };
    let p = package(&settings, &"a".repeat(64));
    let token = CancellationToken::default();
    assert!(ImageBundle::new(p, BTreeMap::new(), &token).is_err());
}

#[test]
fn color_burn_texture_requires_vbr125_and_roundtrips_real_pixels() {
    let token = CancellationToken::default();
    let temp = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(temp.path().join("assets"), MAX_CATALOG_MASK_BYTES).unwrap();
    let png = encode_mask(&BrushMask::new(4, 1, vec![0, 255, 64, 192]).unwrap()).unwrap();
    let id = hash(&png);
    store.put_verified(&id, &png, &token).unwrap();
    let mask = store.load(&id, &token).unwrap().mask;
    let resource = Arc::new(TextureResource::new(mask.clone()));
    let s = BrushSettings {
        diameter: 16.,
        opacity: 0.5,
        spacing: 0.1,
        mask: Some(mask.clone()),
        texture: BrushTexture::new(
            true,
            TextureSettings::new(TextureSpace::Tip, 1., 0., 0.75, false, 0., 0.)
                .unwrap()
                .with_mode(TextureMode::ColorBurn),
            Some(resource),
        )
        .unwrap(),
        ..Default::default()
    };
    let expected = pixels(s.definition());
    let bundle = ImageBundle::new(package(&s, &id), BTreeMap::from([(id, png)]), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let manifest: serde_json::Value = {
        let mut archive = ZipArchive::new(Cursor::new(&bytes)).unwrap();
        serde_json::from_reader(archive.by_name("manifest.json").unwrap()).unwrap()
    };
    assert_eq!(manifest["version"]["minor"], 25);
    let restored = vbr_image::decode(&bytes, &token).unwrap();
    assert_eq!(
        restored.definition(0).unwrap().texture.settings().mode(),
        TextureMode::ColorBurn
    );
    assert_eq!(pixels(restored.definition(0).unwrap()), expected);
    assert!(vbr_image::decode(&rewrite_minor(&bytes, 24), &token).is_err());
}

#[test]
fn texture_adjustments_require_vbr126_and_roundtrip_real_pixels() {
    let token = CancellationToken::default();
    let temp = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(temp.path().join("assets"), MAX_CATALOG_MASK_BYTES).unwrap();
    let png = encode_mask(&BrushMask::new(4, 1, vec![0, 255, 64, 192]).unwrap()).unwrap();
    let id = hash(&png);
    store.put_verified(&id, &png, &token).unwrap();
    let mask = store.load(&id, &token).unwrap().mask;
    let s = BrushSettings {
        diameter: 16.,
        opacity: 0.5,
        spacing: 0.1,
        mask: Some(mask.clone()),
        texture: BrushTexture::new(
            true,
            TextureSettings::new(TextureSpace::Tip, 1., 0., 0.75, false, 0., 0.)
                .unwrap()
                .with_adjustments(-0.25, 0.5)
                .unwrap(),
            Some(Arc::new(TextureResource::new(mask))),
        )
        .unwrap(),
        ..Default::default()
    };
    let expected = pixels(s.definition());
    let bundle = ImageBundle::new(package(&s, &id), BTreeMap::from([(id, png)]), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let manifest: serde_json::Value = {
        let mut archive = ZipArchive::new(Cursor::new(&bytes)).unwrap();
        serde_json::from_reader(archive.by_name("manifest.json").unwrap()).unwrap()
    };
    assert_eq!(manifest["version"]["minor"], 26);
    let restored = vbr_image::decode(&bytes, &token).unwrap();
    let settings = restored.definition(0).unwrap().texture.settings();
    assert_eq!(settings.brightness(), -0.25);
    assert_eq!(settings.contrast(), 0.5);
    assert_eq!(pixels(restored.definition(0).unwrap()), expected);
    assert!(vbr_image::decode(&rewrite_minor(&bytes, 25), &token).is_err());
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", s.clone()).unwrap();
    let state = brush_state::encode(&catalog).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&state).unwrap();
    assert_eq!(value["version"], 21);
    let restored =
        brush_state::decode_with_resources(&state, MAX_CATALOG_MASK_BYTES, &store, &token).unwrap();
    assert_eq!(
        pixels(restored.selected().settings().definition()),
        expected
    );
    let mut old = value.clone();
    old["version"] = 20.into();
    assert!(brush_state::decode_with_resources(
        &serde_json::to_vec(&old).unwrap(),
        MAX_CATALOG_MASK_BYTES,
        &store,
        &token
    )
    .is_err());
}

#[test]
fn texture_depth_dynamics_require_vbr127_and_roundtrip_real_pixels() {
    let token = CancellationToken::default();
    let temp = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(temp.path().join("assets"), MAX_CATALOG_MASK_BYTES).unwrap();
    let png = encode_mask(&BrushMask::new(4, 1, vec![0, 255, 64, 192]).unwrap()).unwrap();
    let id = hash(&png);
    store.put_verified(&id, &png, &token).unwrap();
    let mask = store.load(&id, &token).unwrap().mask;
    let s = BrushSettings {
        diameter: 16.,
        opacity: 0.5,
        spacing: 0.1,
        mask: Some(mask.clone()),
        texture: BrushTexture::new(
            true,
            TextureSettings::new(TextureSpace::Tip, 1., 0., 0.8, false, 0., 0.)
                .unwrap()
                .with_depth_dynamics(0.2, 0.5, true)
                .unwrap(),
            Some(Arc::new(TextureResource::new(mask))),
        )
        .unwrap(),
        ..Default::default()
    };
    let expected = pixels(s.definition());
    let bundle = ImageBundle::new(package(&s, &id), BTreeMap::from([(id, png)]), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let manifest: serde_json::Value = {
        let mut archive = ZipArchive::new(Cursor::new(&bytes)).unwrap();
        serde_json::from_reader(archive.by_name("manifest.json").unwrap()).unwrap()
    };
    assert_eq!(manifest["version"]["minor"], 27);
    let restored = vbr_image::decode(&bytes, &token).unwrap();
    let settings = restored.definition(0).unwrap().texture.settings();
    assert_eq!(settings.minimum_depth(), 0.2);
    assert_eq!(settings.depth_jitter(), 0.5);
    assert!(settings.depth_pressure());
    assert_eq!(pixels(restored.definition(0).unwrap()), expected);
    assert!(vbr_image::decode(&rewrite_minor(&bytes, 26), &token).is_err());
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", s).unwrap();
    let state = brush_state::encode(&catalog).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&state).unwrap();
    assert_eq!(value["version"], 21);
    let restored =
        brush_state::decode_with_resources(&state, MAX_CATALOG_MASK_BYTES, &store, &token).unwrap();
    assert_eq!(
        pixels(restored.selected().settings().definition()),
        expected
    );
}

#[test]
fn published_texture_contract_matches_resource_roles_and_new_limits() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-texture-v1.9.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["texture"]["properties"]["model"]["const"],
        "periodic-coverage-v1"
    );
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["assets"]["maxItems"],
        5120
    );
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        9
    );
    assert_eq!(
        schema["$defs"]["sourceRecord"]["properties"]["baseline"]["$ref"],
        "#/$defs/legacyParameters"
    );
}

#[test]
fn published_color_burn_contract_requires_vbr125_and_explicit_mode() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-texture-modes-v1.25.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        25
    );
    assert!(schema["$defs"]["texture"]["required"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("mode")));
    assert!(schema["$defs"]["texture"]["properties"]["mode"]["enum"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("colorBurn")));
}

#[test]
fn published_texture_adjustment_contract_requires_vbr126() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-texture-adjustments-v1.26.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        26
    );
    for key in ["brightness", "contrast"] {
        assert_eq!(schema["$defs"]["texture"]["properties"][key]["minimum"], -1);
        assert_eq!(schema["$defs"]["texture"]["properties"][key]["maximum"], 1);
    }
}

#[test]
fn published_texture_source_contract_requires_vbr127() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-texture-source-v1.27.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        27
    );
    assert!(
        schema["$defs"]["sourceRecord"]["properties"]["version"]["enum"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(2))
    );
    assert_eq!(
        schema["$defs"]["sourceRecord"]["properties"]["texture"]["$ref"],
        "#/$defs/texture"
    );
    for key in ["minimumDepth", "depthJitter"] {
        assert_eq!(schema["$defs"]["texture"]["properties"][key]["maximum"], 1);
    }
    assert_eq!(
        schema["$defs"]["texture"]["properties"]["depthPressure"]["type"],
        "boolean"
    );
}
