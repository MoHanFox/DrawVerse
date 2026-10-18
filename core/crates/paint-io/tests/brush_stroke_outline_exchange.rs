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
            behavior_version: 33,
            required_capabilities: vec!["stroke-outline".into()],
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
fn state41_and_image_vbr_preserve_closed_settings_and_self_outline_pixels() {
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
        assert_eq!(json["version"], 41);
        assert_eq!(json["presets"][0]["strokeOutline"]["enabled"], enabled);
        let restored = brush_state::decode(&state, MAX_CATALOG_MASK_BYTES).unwrap();
        assert_eq!(brush_state::encode(&restored).unwrap(), state);
        assert_eq!(
            draw(definition(restored.selected().settings())),
            draw(definition(&settings))
        );
        let mut old = json;
        old["version"] = 40.into();
        assert!(
            brush_state::decode(&serde_json::to_vec(&old).unwrap(), MAX_CATALOG_MASK_BYTES)
                .is_err()
        );

        let bundle = ImageBundle::new(package(&settings), BTreeMap::new(), &token).unwrap();
        let bytes = vbr_image::encode(&bundle, &token).unwrap();
        let reopened = vbr_image::decode(&bytes, &token).unwrap();
        assert_eq!(reopened.package.brushes[0].behavior_version, 33);
        assert_eq!(
            reopened.package.brushes[0].required_capabilities,
            ["stroke-outline"]
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
