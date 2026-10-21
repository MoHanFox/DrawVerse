use paint_core::{
    BrushCatalog, BrushDynamics, BrushMask, BrushSettings, BrushTexture, BrushTip, Document,
    InputPoint, OutlinePlacement, OutlineSource, OutlineWidth, Pixel, SecondaryTip, StrokeOutline,
    TextureResource, TextureSettings, MAX_CATALOG_MASK_BYTES,
};
use paint_io::{
    brush_resource::{encode_mask, hash, ResourceStore},
    brush_state,
    brush_stroke_outline::OutlineParameters,
    vbr::{Package, Parameters, Preset},
    vbr_image::{self, ImageBundle},
};
use paint_task::CancellationToken;
use std::{collections::BTreeMap, sync::Arc};

fn resource(coverage: &[u8], edge: u32) -> (String, Vec<u8>, Arc<BrushMask>) {
    let raw = BrushMask::new(edge, edge, coverage.to_vec()).unwrap();
    let bytes = encode_mask(&raw).unwrap();
    let id = hash(&bytes);
    let mask = Arc::new(raw.with_resource_id(id.clone()).unwrap());
    (id, bytes, mask)
}

fn outline(
    enabled: bool,
    placement: OutlinePlacement,
    source: OutlineSource,
) -> Arc<StrokeOutline> {
    Arc::new(
        StrokeOutline::new(
            enabled,
            placement,
            OutlineWidth::new(true, 7., 12.5).unwrap(),
            source,
            true,
        )
        .unwrap(),
    )
}

fn settings(outline: Arc<StrokeOutline>) -> BrushSettings {
    BrushSettings {
        diameter: 20.,
        spacing: 0.15,
        outline: outline.as_ref().clone(),
        tip: BrushTip {
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn parameters(settings: &BrushSettings) -> Parameters {
    Parameters {
        transfer_jitter: None,
        airbrush: None,
        speed_dynamics: None,
        tangent_dynamics: None,
        distance_dynamics: None,
        tilt_dynamics: None,
        rotation_dynamics: None,
        direction_dynamics: None,
        source_matrix: None,
        fade_dynamics: None,
        diameter: settings.diameter,
        spacing: settings.spacing,
        dab_opacity: settings.opacity,
        hardness: settings.tip.hardness,
        roundness: settings.tip.roundness,
        angle: settings.tip.angle,
        flow: settings.tip.flow,
        pressure_size: settings.tip.pressure_size,
        pressure_flow: settings.tip.pressure_flow,
        pressure_dynamics: None,
        tip_dynamics: None,
        scatter_dynamics: None,
        shape_jitter: None,
        color_dynamics: None,
    }
}

fn package(settings: &BrushSettings) -> Package {
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "Outline".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 37,
            required_capabilities: {
                let mut caps = vec!["stroke-outline".into(), "stroke-outline-replay".into()];
                if matches!(settings.outline.source(), OutlineSource::SelfTextured(_)) {
                    caps.push("outline-self-texture".into());
                }
                caps
            },
            parameters: parameters(settings),
            stroke_outline: Some(OutlineParameters::from_core(&settings.outline).unwrap()),
            tip_resource: None,
            texture: None,
            dual_brush: None,
            smoothing: None,
            wet_mix: None,
            stroke_taper: None,
            source_resource: None,
        }],
    }
}

fn definition(settings: &BrushSettings) -> paint_core::BrushDefinition {
    settings.definition()
}

fn draw(mut definition: paint_core::BrushDefinition) -> Vec<[u32; 4]> {
    definition.brush.color = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
    definition.brush.background_color = Pixel::from_straight([0., 0., 1., 1.]).unwrap();
    let mut document = Document::new(72, 48).unwrap();
    document
        .begin_brush_stroke(definition, InputPoint::new(18., 24., 1.))
        .unwrap();
    document.stroke_to(InputPoint::new(54., 24., 1.)).unwrap();
    assert!(document.end_stroke().unwrap());
    (0..48 * 72)
        .map(|i| {
            document
                .pixel(i % 72, i / 72)
                .components()
                .map(f32::to_bits)
        })
        .collect()
}

#[test]
fn replay_state_and_image_vbr_preserve_closed_settings_and_self_outline_pixels() {
    let token = CancellationToken::default();
    let default = brush_state::encode(&BrushCatalog::default()).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&default).unwrap()["version"],
        2
    );
    assert!(!String::from_utf8(default.clone())
        .unwrap()
        .contains("strokeOutline"));
    for enabled in [false, true] {
        let settings = settings(outline(
            enabled,
            if enabled {
                OutlinePlacement::Outside
            } else {
                OutlinePlacement::Center
            },
            OutlineSource::SelfBrush,
        ));
        let mut catalog = BrushCatalog::default();
        catalog.edit("round-pressure", settings.clone()).unwrap();
        let state = brush_state::encode(&catalog).unwrap();
        let json: serde_json::Value = serde_json::from_slice(&state).unwrap();
        assert_eq!(json["version"], 45);
        assert_eq!(json["presets"][0]["strokeOutline"]["enabled"], enabled);
        let restored = brush_state::decode(&state, MAX_CATALOG_MASK_BYTES).unwrap();
        assert_eq!(brush_state::encode(&restored).unwrap(), state);
        assert_eq!(
            draw(definition(restored.selected().settings())),
            draw(definition(&settings))
        );
        let mut old = json;
        old["version"] = 41.into();
        old["presets"][0]["strokeOutline"]["model"] = "stroke-contour-v1".into();
        old["presets"][0]["strokeOutline"]
            .as_object_mut()
            .unwrap()
            .remove("allowOverflow");
        let migrated =
            brush_state::decode(&serde_json::to_vec(&old).unwrap(), MAX_CATALOG_MASK_BYTES)
                .unwrap();
        assert_eq!(brush_state::encode(&migrated).unwrap(), state);
        assert_eq!(
            draw(definition(migrated.selected().settings())),
            draw(definition(&settings))
        );
        old["version"] = 40.into();
        assert!(
            brush_state::decode(&serde_json::to_vec(&old).unwrap(), MAX_CATALOG_MASK_BYTES)
                .is_err()
        );

        let bundle = ImageBundle::new(package(&settings), BTreeMap::new(), &token).unwrap();
        let bytes = vbr_image::encode(&bundle, &token).unwrap();
        let reopened = vbr_image::decode(&bytes, &token).unwrap();
        assert_eq!(reopened.package.brushes[0].behavior_version, 37);
        assert_eq!(
            reopened.package.brushes[0].required_capabilities,
            ["stroke-outline", "stroke-outline-replay"]
        );
        assert_eq!(
            draw(reopened.definition(0).unwrap()),
            draw(definition(&settings))
        );
        assert!(paint_io::vbr::encode(&bundle.package, &token).is_err());
    }
}

#[test]
fn existing_and_texture_outline_resources_roundtrip_from_fresh_state_and_vbr() {
    let token = CancellationToken::default();
    let temp = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(temp.path().into(), MAX_CATALOG_MASK_BYTES).unwrap();
    let existing_coverage = (0..64)
        .map(|i| if (14..50).contains(&i) { 255 } else { 0 })
        .collect::<Vec<_>>();
    let texture_coverage = (0..81)
        .map(|i| if i % 3 == 0 { 255 } else { 64 })
        .collect::<Vec<_>>();
    let (existing_id, existing_bytes, existing_mask) = resource(&existing_coverage, 8);
    let (texture_id, texture_bytes, texture_mask) = resource(&texture_coverage, 9);
    store
        .put_verified(&existing_id, &existing_bytes, &token)
        .unwrap();
    store
        .put_verified(&texture_id, &texture_bytes, &token)
        .unwrap();
    let existing_tip = SecondaryTip::new(
        3.,
        BrushTip::default(),
        Some(existing_mask.clone()),
        BrushDynamics {
            scatter: paint_core::ScatterDynamics::new(true, 0.4, true, 2, 0.2, 7).unwrap(),
            ..Default::default()
        },
    )
    .unwrap()
    .with_spacing(0.12)
    .unwrap()
    .with_flip(true)
    .with_shape_flip(true, false);
    let texture = BrushTexture::new(
        true,
        TextureSettings::default(),
        Some(Arc::new(TextureResource::new(texture_mask))),
    )
    .unwrap();
    for source in [
        OutlineSource::Existing(Arc::new(existing_tip.clone())),
        OutlineSource::Texture(texture.clone()),
        OutlineSource::SelfTextured(texture.clone()),
    ] {
        let settings = settings(outline(true, OutlinePlacement::Inside, source));
        let mut catalog = BrushCatalog::default();
        catalog.edit("round-pressure", settings.clone()).unwrap();
        let state = brush_state::encode(&catalog).unwrap();
        let restored =
            brush_state::decode_with_resources(&state, MAX_CATALOG_MASK_BYTES, &store, &token)
                .unwrap();
        assert_eq!(
            draw(definition(restored.selected().settings())),
            draw(definition(&settings))
        );
        let all_assets = BTreeMap::from([
            (existing_id.clone(), existing_bytes.clone()),
            (texture_id.clone(), texture_bytes.clone()),
        ]);
        let parameters = OutlineParameters::from_core(&settings.outline).unwrap();
        let id = parameters
            .existing_resource_id()
            .or_else(|| parameters.texture_resource_id())
            .unwrap();
        let assets = BTreeMap::from([(id.to_owned(), all_assets[id].clone())]);
        let bundle = ImageBundle::new(package(&settings), assets, &token).unwrap();
        let bytes = vbr_image::encode(&bundle, &token).unwrap();
        let reopened = vbr_image::decode(&bytes, &token).unwrap();
        assert_eq!(
            draw(reopened.definition(0).unwrap()),
            draw(definition(&settings))
        );
        let fresh = ResourceStore::open(temp.path().join("fresh"), MAX_CATALOG_MASK_BYTES).unwrap();
        for (id, asset) in reopened.assets() {
            fresh.put_verified(id, asset.bytes(), &token).unwrap();
        }
        let restored = brush_state::decode_with_resources(
            &brush_state::encode(&catalog).unwrap(),
            MAX_CATALOG_MASK_BYTES,
            &fresh,
            &token,
        )
        .unwrap();
        assert_eq!(
            draw(definition(restored.selected().settings())),
            draw(definition(&settings))
        );
    }
}

#[test]
fn outline_parameters_reject_unknown_null_and_pre_rounding_out_of_range_fields() {
    let valid = serde_json::to_value(
        OutlineParameters::from_core(&outline(
            true,
            OutlinePlacement::Outside,
            OutlineSource::SelfBrush,
        ))
        .unwrap(),
    )
    .unwrap();
    for path in ["/width/pixels", "/width/percent"] {
        for value in [
            serde_json::json!(-0.0000000001),
            serde_json::json!(10000.00000001),
        ] {
            let mut bad = valid.clone();
            *bad.pointer_mut(path).unwrap() = value;
            assert!(serde_json::from_value::<OutlineParameters>(bad).is_err());
        }
    }
    for path in [
        "/enabled",
        "/placement",
        "/width",
        "/source",
        "/secondaryColor",
    ] {
        let mut bad = valid.clone();
        *bad.pointer_mut(path).unwrap() = serde_json::Value::Null;
        assert!(serde_json::from_value::<OutlineParameters>(bad).is_err());
    }
    for (path, value) in [
        ("/model", "unknown"),
        ("/placement", "outside-ish"),
        ("/source/model", "unknown"),
    ] {
        let mut bad = valid.clone();
        *bad.pointer_mut(path).unwrap() = value.into();
        assert!(serde_json::from_value::<OutlineParameters>(bad).is_err());
    }
    let mut extra = valid;
    extra["width"]["extra"] = true.into();
    assert!(serde_json::from_value::<OutlineParameters>(extra).is_err());
}

#[test]
fn replay_self_texture_preserves_closed_values_and_changes_only_outline() {
    let token = CancellationToken::default();
    let temp = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(temp.path().join("resources"), MAX_CATALOG_MASK_BYTES).unwrap();
    let (id, bytes, mask) = resource(&[0; 16], 4);
    store.put_verified(&id, &bytes, &token).unwrap();
    let plain = settings(outline(
        true,
        OutlinePlacement::Outside,
        OutlineSource::SelfBrush,
    ));
    let plain_pixels = draw(definition(&plain));
    for outline_enabled in [false, true] {
        for texture_enabled in [false, true] {
            let texture = BrushTexture::new(
                texture_enabled,
                TextureSettings::default(),
                Some(Arc::new(TextureResource::new(mask.clone()))),
            )
            .unwrap();
            let actual = settings(outline(
                outline_enabled,
                OutlinePlacement::Outside,
                OutlineSource::SelfTextured(texture),
            ));
            let mut catalog = BrushCatalog::default();
            catalog.edit("round-pressure", actual.clone()).unwrap();
            let state = brush_state::encode(&catalog).unwrap();
            let mut json: serde_json::Value = serde_json::from_slice(&state).unwrap();
            assert_eq!(json["version"], 45);
            assert_eq!(
                json["presets"][0]["strokeOutline"]["source"]["model"],
                "self-brush"
            );
            let restored =
                brush_state::decode_with_resources(&state, MAX_CATALOG_MASK_BYTES, &store, &token)
                    .unwrap();
            assert_eq!(brush_state::encode(&restored).unwrap(), state);
            assert!(!restored.selected().settings().texture.enabled());
            let pixels = draw(definition(restored.selected().settings()));
            assert_eq!(pixels, draw(definition(&actual)));
            if outline_enabled && !texture_enabled {
                assert_eq!(pixels, plain_pixels);
            }
            if outline_enabled && texture_enabled {
                assert_ne!(pixels, plain_pixels);
                // Main red stroke remains; the black outline-only texture removes blue.
                assert_eq!(pixels[24 * 72 + 36], plain_pixels[24 * 72 + 36]);
                assert!(pixels.iter().all(|p| f32::from_bits(p[2]) == 0.));
            }
            json["version"] = 42.into();
            json["presets"][0]["strokeOutline"]["model"] = "stroke-contour-v1".into();
            json["presets"][0]["strokeOutline"]
                .as_object_mut()
                .unwrap()
                .remove("allowOverflow");
            let migrated = brush_state::decode_with_resources(
                &serde_json::to_vec(&json).unwrap(),
                MAX_CATALOG_MASK_BYTES,
                &store,
                &token,
            )
            .unwrap();
            assert_eq!(brush_state::encode(&migrated).unwrap(), state);
            assert_eq!(draw(definition(migrated.selected().settings())), pixels);
            json["version"] = 41.into();
            assert!(brush_state::decode_with_resources(
                &serde_json::to_vec(&json).unwrap(),
                MAX_CATALOG_MASK_BYTES,
                &store,
                &token
            )
            .is_err());
            let package = package(&actual);
            package.validate().unwrap();
            assert_eq!(package.brushes[0].behavior_version, 37);
            assert!(package.brushes[0]
                .required_capabilities
                .iter()
                .any(|c| c == "outline-self-texture"));
            let bundle = ImageBundle::new(
                package,
                BTreeMap::from([(id.clone(), bytes.clone())]),
                &token,
            )
            .unwrap();
            let reopened =
                vbr_image::decode(&vbr_image::encode(&bundle, &token).unwrap(), &token).unwrap();
            assert_eq!(draw(reopened.definition(0).unwrap()), pixels);
            let mut old_metadata: serde_json::Value =
                serde_json::from_slice(&bundle.package.to_json().unwrap()).unwrap();
            old_metadata["brushes"][0]["behaviorVersion"] = 1.into();
            old_metadata["brushes"][0]["requiredCapabilities"] = serde_json::json!([]);
            let normalized =
                Package::from_json(&serde_json::to_vec(&old_metadata).unwrap()).unwrap();
            assert_eq!(normalized.brushes[0].behavior_version, 37);
        }
    }
}

#[test]
fn legacy_state43_migrates_closed_and_active_models_resources_and_actual_pixels() {
    let token = CancellationToken::default();
    let temp = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(temp.path().join("resources"), MAX_CATALOG_MASK_BYTES).unwrap();
    let (id, bytes, mask) = resource(
        &[
            255, 0, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255,
        ],
        4,
    );
    store.put_verified(&id, &bytes, &token).unwrap();
    let texture = BrushTexture::new(
        true,
        TextureSettings::default(),
        Some(Arc::new(TextureResource::new(mask.clone()))),
    )
    .unwrap();
    let existing =
        SecondaryTip::new(8., BrushTip::default(), Some(mask), Default::default()).unwrap();
    for enabled in [false, true] {
        for source in [
            OutlineSource::SelfBrush,
            OutlineSource::SelfTextured(texture.clone()),
            OutlineSource::Existing(Arc::new(existing.clone())),
            OutlineSource::Texture(texture.clone()),
        ] {
            let actual = settings(
                outline(enabled, OutlinePlacement::Inside, source)
                    .as_ref()
                    .clone()
                    .into(),
            );
            let expected = draw(definition(&actual));
            let mut catalog = BrushCatalog::default();
            catalog.edit("round-pressure", actual.clone()).unwrap();
            let state = brush_state::encode(&catalog).unwrap();
            let mut json: serde_json::Value = serde_json::from_slice(&state).unwrap();
            let current = state.clone();
            json["version"] = 43.into();
            json["presets"][0]["strokeOutline"]["model"] = "stroke-sdf-v1".into();
            json["presets"][0]["strokeOutline"]
                .as_object_mut()
                .unwrap()
                .remove("allowOverflow");
            let state = serde_json::to_vec(&json).unwrap();
            let reopened =
                brush_state::decode_with_resources(&state, MAX_CATALOG_MASK_BYTES, &store, &token)
                    .unwrap();
            assert_eq!(brush_state::encode(&reopened).unwrap(), current);
            assert_eq!(draw(definition(reopened.selected().settings())), expected);
            for version in [41, 42] {
                json["version"] = version.into();
                assert!(brush_state::decode_with_resources(
                    &serde_json::to_vec(&json).unwrap(),
                    MAX_CATALOG_MASK_BYTES,
                    &store,
                    &token
                )
                .is_err());
            }
            let mut package = package(&actual);
            let mut legacy =
                serde_json::to_value(package.brushes[0].stroke_outline.as_ref().unwrap()).unwrap();
            legacy["model"] = "stroke-sdf-v1".into();
            legacy.as_object_mut().unwrap().remove("allowOverflow");
            package.brushes[0].stroke_outline = Some(serde_json::from_value(legacy).unwrap());
            package.brushes[0].behavior_version = 35;
            package.brushes[0]
                .required_capabilities
                .retain(|c| c != "stroke-outline-replay");
            package.brushes[0]
                .required_capabilities
                .push("stroke-outline-sdf".into());
            package.validate().unwrap();
            let resources = if matches!(actual.outline.source(), OutlineSource::SelfBrush) {
                BTreeMap::new()
            } else {
                BTreeMap::from([(id.clone(), bytes.clone())])
            };
            let bundle = ImageBundle::new(package, resources, &token).unwrap();
            let decoded =
                vbr_image::decode(&vbr_image::encode(&bundle, &token).unwrap(), &token).unwrap();
            assert_eq!(
                decoded.definition(0).unwrap().outline.model(),
                "stroke-replay-v1"
            );
            assert_eq!(draw(decoded.definition(0).unwrap()), expected);
            let mut old: serde_json::Value =
                serde_json::from_slice(&bundle.package.to_json().unwrap()).unwrap();
            old["brushes"][0]["behaviorVersion"] = 1.into();
            old["brushes"][0]["requiredCapabilities"] = serde_json::json!([]);
            let normalized = Package::from_json(&serde_json::to_vec(&old).unwrap()).unwrap();
            assert_eq!(normalized.brushes[0].behavior_version, 35);
            assert!(normalized.brushes[0]
                .required_capabilities
                .iter()
                .any(|c| c == "stroke-outline-sdf"));
        }
    }
}

#[test]
fn replay_state45_and_vbr37_roundtrip_all_sources_overflow_and_real_pixels() {
    let token = CancellationToken::default();
    let temp = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(temp.path().into(), MAX_CATALOG_MASK_BYTES).unwrap();
    let (id, bytes, mask) = resource(&[0, 100, 220, 255], 2);
    store.put_verified(&id, &bytes, &token).unwrap();
    let texture = BrushTexture::new(
        true,
        TextureSettings::default(),
        Some(Arc::new(TextureResource::new(mask.clone()))),
    )
    .unwrap();
    let existing = SecondaryTip::new(15., BrushTip::default(), Some(mask), Default::default())
        .unwrap()
        .with_texture(texture.clone());
    for source in [
        OutlineSource::SelfBrush,
        OutlineSource::SelfTextured(texture.clone()),
        OutlineSource::Texture(texture),
        OutlineSource::Existing(Arc::new(existing)),
    ] {
        for enabled in [false, true] {
            for allow in [false, true] {
                let current = settings(Arc::new(
                    outline(enabled, OutlinePlacement::Outside, source.clone())
                        .as_ref()
                        .clone()
                        .with_allow_overflow(allow),
                ));
                let expected = draw(definition(&current));
                let mut catalog = BrushCatalog::default();
                catalog.edit("round-pressure", current.clone()).unwrap();
                let state = brush_state::encode(&catalog).unwrap();
                let json: serde_json::Value = serde_json::from_slice(&state).unwrap();
                assert_eq!(json["version"], 45);
                let reopened = brush_state::decode_with_resources(
                    &state,
                    MAX_CATALOG_MASK_BYTES,
                    &store,
                    &token,
                )
                .unwrap();
                assert_eq!(brush_state::encode(&reopened).unwrap(), state);
                assert_eq!(draw(definition(reopened.selected().settings())), expected);
                let mut old = json.clone();
                old["version"] = 44.into();
                assert!(brush_state::decode_with_resources(
                    &serde_json::to_vec(&old).unwrap(),
                    MAX_CATALOG_MASK_BYTES,
                    &store,
                    &token
                )
                .is_err());
                let package = package(&current);
                package.validate().unwrap();
                let resources = if matches!(source, OutlineSource::SelfBrush) {
                    BTreeMap::new()
                } else {
                    BTreeMap::from([(id.clone(), bytes.clone())])
                };
                let bundle = ImageBundle::new(package, resources, &token).unwrap();
                let reopened =
                    vbr_image::decode(&vbr_image::encode(&bundle, &token).unwrap(), &token)
                        .unwrap();
                let definition = reopened.definition(0).unwrap();
                assert_eq!(definition.outline.model(), "stroke-replay-v1");
                assert_eq!(definition.outline.allow_overflow(), allow);
                assert_eq!(draw(definition), expected);
                let mut stale: serde_json::Value =
                    serde_json::from_slice(&bundle.package.to_json().unwrap()).unwrap();
                stale["brushes"][0]["behaviorVersion"] = 1.into();
                stale["brushes"][0]["requiredCapabilities"] = serde_json::json!([]);
                let normalized = Package::from_json(&serde_json::to_vec(&stale).unwrap()).unwrap();
                assert_eq!(normalized.brushes[0].behavior_version, 37);
                assert!(normalized.brushes[0]
                    .required_capabilities
                    .iter()
                    .any(|c| c == "stroke-outline-replay"));
            }
        }
    }
}

#[test]
fn replay_fields_are_strict_and_legacy_models_keep_original_encoding() {
    let mut value =
        serde_json::to_value(OutlineParameters::from_core(&StrokeOutline::default()).unwrap())
            .unwrap();
    value["model"] = "stroke-contour-v1".into();
    value.as_object_mut().unwrap().remove("allowOverflow");
    let legacy: OutlineParameters = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(&legacy).unwrap(), value);
    let migrated = legacy.bind(None, None).unwrap();
    assert_eq!(migrated.model(), "stroke-replay-v1");
    assert!(!migrated.allow_overflow());
    assert!(value.get("allowOverflow").is_none());
    value["allowOverflow"] = false.into();
    assert!(serde_json::from_value::<OutlineParameters>(value).is_err());
    let settings = StrokeOutline::default();
    let value = serde_json::to_value(OutlineParameters::from_core(&settings).unwrap()).unwrap();
    assert_eq!(value["allowOverflow"], false);
    for bad in [
        serde_json::Value::Null,
        serde_json::json!(1),
        serde_json::json!("true"),
    ] {
        let mut invalid = value.clone();
        invalid["allowOverflow"] = bad;
        assert!(serde_json::from_value::<OutlineParameters>(invalid).is_err());
    }
    let mut missing = value;
    missing.as_object_mut().unwrap().remove("allowOverflow");
    assert!(serde_json::from_value::<OutlineParameters>(missing).is_err());
}
