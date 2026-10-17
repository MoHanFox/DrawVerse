use paint_core::*;
use paint_io::{
    brush_state,
    brush_wet::WetParameters,
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
mod support {
    pub mod abr_fixture;
}
fn settings(enabled: bool, clean: bool, load: bool) -> BrushSettings {
    let curve = PressureCurve::new_smooth(vec![(0., 0.), (0.5, 0.3), (1., 1.)]).unwrap();
    BrushSettings {
        diameter: 8.,
        spacing: 0.25,
        opacity: 0.4,
        wet: WetSettings::new(
            enabled,
            true,
            clean,
            load,
            WetMix::new(0.6, 0.2, 0.3, 0.4, 0.8, 0.1).unwrap(),
        ),
        dynamics: BrushDynamics {
            size: PressureControl::new(curve.clone(), 0.2).unwrap(),
            opacity: PressureControl::new(curve, 0.1).unwrap(),
            pressure_opacity: true,
            color: ColorDynamics::new(true, 0.3, 0.2, 0.1, 0.2, true, u64::MAX).unwrap(),
            ..Default::default()
        },
        tip: BrushTip {
            flow: 0.7,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn package(s: &BrushSettings) -> Package {
    let mut pressure = s.dynamics.clone();
    pressure.color = Default::default();
    pressure.scatter = Default::default();
    let mut caps = vec![
        if s.mask.is_some() {
            "image-tip"
        } else {
            "procedural-tip"
        },
        "pressure-curves",
        "opacity-pressure",
        "pressure-smooth",
        "color-dynamics",
        "wet-mix",
    ];
    if !s.smoothing.is_default() {
        caps.push("path-smoothing");
    }
    if !s.dual.is_default() {
        caps.extend(["dual-cover", "dual-relative-size", "dual-color-stroke"]);
    }
    if !s.texture.is_default() {
        caps.push("texture-cover");
    }
    if s.dynamics.scatter != ScatterDynamics::default() {
        caps.push("scatter-stamps");
    }
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "Wet".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 14,
            required_capabilities: caps.into_iter().map(String::from).collect(),
            parameters: Parameters {
                airbrush: None,
                speed_dynamics: None,
                tangent_dynamics: None,
                distance_dynamics: None,
                tilt_dynamics: None,
                diameter: s.diameter,
                spacing: s.spacing,
                dab_opacity: s.opacity,
                hardness: s.tip.hardness,
                roundness: s.tip.roundness,
                angle: s.tip.angle,
                flow: s.tip.flow,
                pressure_size: s.tip.pressure_size,
                pressure_flow: s.tip.pressure_flow,
                pressure_dynamics: Some(paint_io::brush_pressure::PressureParameters::from_core(
                    &pressure,
                )),
                tip_dynamics: None,
                scatter_dynamics: (s.dynamics.scatter != ScatterDynamics::default()).then(|| {
                    paint_io::brush_scatter::ScatterParameters::from_core(s.dynamics.scatter)
                }),
                shape_jitter: None,
                color_dynamics: Some(paint_io::brush_color_dynamics::ColorParameters::from_core(
                    s.dynamics.color,
                )),
            },
            tip_resource: s
                .mask
                .as_ref()
                .and_then(|m| m.resource_id())
                .map(str::to_owned),
            source_resource: None,
            stroke_taper: None,
            wet_mix: Some(WetParameters::from_core(s.wet)),
            smoothing: (!s.smoothing.is_default())
                .then(|| paint_io::brush_smoothing::SmoothingParameters::from_core(s.smoothing)),
            dual_brush: (!s.dual.is_default())
                .then(|| paint_io::brush_dual::DualParameters::from_core(&s.dual).unwrap()),
            texture: (!s.texture.is_default()).then(|| {
                paint_io::brush_texture::TextureParameters::from_core(&s.texture).unwrap()
            }),
        }],
    }
}
fn source() -> Document {
    let tile = Tile::from_pixels(
        (0..4096)
            .map(|i| {
                Pixel::from_straight(if i / 64 < 32 {
                    [1., 0., 0., 0.7]
                } else {
                    [0., 0., 1., 0.7]
                })
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    Document::from_import(
        128,
        64,
        vec![ImportedLayer {
            name: "source".into(),
            properties: Default::default(),
            tiles: BTreeMap::from([(TileCoord { x: 0, y: 0 }, tile)]),
        }],
        0,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap()
}
fn pixels(d: &Document) -> Vec<Pixel> {
    (0..64)
        .flat_map(|y| (0..128).map(move |x| d.try_pixel(x, y).unwrap()))
        .collect()
}
fn draw(d: &mut Document, mut b: BrushDefinition) -> Vec<Pixel> {
    let mut retained = Some(
        WetCarrier::new(
            b.wet.mix(),
            b.brush.radius * 2.,
            Pixel::from_straight([1., 0., 0., 0.6]).unwrap(),
        )
        .unwrap(),
    );
    let mut ink = Vec::new();
    for shift in [0., 5.] {
        b.brush.color = Pixel::from_straight(if shift == 0. {
            [1., 0., 0., 0.8]
        } else {
            [0., 0., 1., 0.8]
        })
        .unwrap();
        b.brush.background_color = Pixel::from_straight([0., 1., 0., 1.]).unwrap();
        let point = |x, y, p, t| {
            let mut q = InputPoint::new(x, y, p);
            q.timestamp_ns = t;
            q
        };
        let first = point(16.5, 28.5 + shift, 0.4, 1_000_000);
        if b.wet.enabled() {
            d.begin_configured_wet_brush_stroke(b.clone(), first, retained.as_ref())
                .unwrap();
        } else {
            d.begin_brush_stroke(b.clone(), first).unwrap();
        }
        d.stroke_to(point(48.5, 32.5 + shift, 1., 41_000_000))
            .unwrap();
        d.stroke_to(point(104.5, 30.5 + shift, 0.6, 81_000_000))
            .unwrap();
        if b.wet.enabled() {
            let (changed, next) = d.end_wet_brush_stroke().unwrap();
            assert!(changed);
            for (u, v) in [(0.25, 0.25), (0.75, 0.75)] {
                ink.push(next.sample(u, v));
            }
            retained = Some(next);
        } else {
            assert!(d.end_stroke().unwrap());
        }
    }
    ink
}
#[test]
fn state17_and_vbr15_restore_real_two_strokes_with_retained_ink_and_closed_policy() {
    let token = CancellationToken::default();
    for enabled in [false, true] {
        for clean in [false, true] {
            for load in [false, true] {
                let s = settings(enabled, clean, load);
                let bundle = ImageBundle::new(package(&s), BTreeMap::new(), &token).unwrap();
                let bytes = vbr_image::encode(&bundle, &token).unwrap();
                let reopened = vbr_image::decode(&bytes, &token).unwrap();
                assert_eq!(
                    bundle.package.to_json().unwrap(),
                    reopened.package.to_json().unwrap()
                );
                let mut catalog = BrushCatalog::default();
                catalog.edit("round-pressure", s.clone()).unwrap();
                let state = brush_state::encode(&catalog).unwrap();
                let restored = brush_state::decode(&state, MAX_CATALOG_MASK_BYTES).unwrap();
                assert_eq!(restored.selected().settings().wet, s.wet);
                let mut a = source();
                let mut b = source();
                let mut c = source();
                let before = pixels(&a);
                let ink = draw(&mut a, s.definition());
                assert_eq!(ink, draw(&mut b, reopened.definition(0).unwrap()));
                assert_eq!(
                    ink,
                    draw(&mut c, restored.selected().settings().definition())
                );
                assert_eq!(pixels(&a), pixels(&b));
                assert_eq!(pixels(&a), pixels(&c));
                assert_eq!(a.history_depth(), (2, 0));
                let after = pixels(&b);
                b.undo().unwrap();
                b.undo().unwrap();
                assert_eq!(pixels(&b), before);
                b.redo().unwrap();
                b.redo().unwrap();
                assert_eq!(pixels(&b), after);
            }
        }
    }
}
#[test]
fn wet_exchange_keeps_images_texture_dual_smoothing_scatter_and_original_source() {
    use paint_io::{abr_import::ImportBatch, brush_resource::ResourceStore};
    let token = CancellationToken::default();
    let temp = tempfile::tempdir().unwrap();
    let resources =
        ResourceStore::open(temp.path().join("resources"), MAX_CATALOG_MASK_BYTES).unwrap();
    let raw = Arc::new(support::abr_fixture::fixture(6, 2, false));
    let imported = ImportBatch::decode(raw.clone(), MAX_CATALOG_MASK_BYTES, &token)
        .unwrap()
        .persist(&resources, &token)
        .unwrap();
    let source_id = imported[0].source_id().to_owned();
    let mask = imported[0].settings().mask.clone().unwrap();
    let id = mask.resource_id().unwrap().to_owned();
    let mut s = settings(true, false, false);
    s.mask = Some(mask.clone());
    s.texture = BrushTexture::new(
        true,
        Default::default(),
        Some(Arc::new(TextureResource::new(mask.clone()))),
    )
    .unwrap();
    s.smoothing = BrushSmoothing::new(true, 8., 8., 0.5, 1.).unwrap();
    s.dynamics.scatter = ScatterDynamics::new(true, 0.3, true, 2, 0.2, u64::MAX).unwrap();
    s.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(3., BrushTip::default(), Some(mask), Default::default())
            .unwrap()
            .with_spacing(0.1)
            .unwrap(),
    )
    .with_sizing(DualSizing::new(DualSizeMode::Relative, 2., DualWidthRule::FixedWidth).unwrap())
    .with_color(
        DualColor::new(
            false,
            true,
            true,
            DualColorOrder::SecondBehind,
            BlendMode::Multiply,
            BlendMode::Screen,
        )
        .unwrap(),
    );
    let sources = resources
        .source_store()
        .unwrap()
        .export_batches(
            &std::collections::BTreeSet::from([source_id.clone()]),
            &token,
        )
        .unwrap();
    let mut p = package(&s);
    p.brushes[0].source_resource = Some(source_id.clone());
    let bundle = ImageBundle::with_sources(
        p,
        BTreeMap::from([(
            id,
            resources
                .encoded(s.mask.as_ref().unwrap().resource_id().unwrap())
                .unwrap(),
        )]),
        sources,
        &token,
    )
    .unwrap();
    let reopened = vbr_image::decode(&vbr_image::encode(&bundle, &token).unwrap(), &token).unwrap();
    assert_eq!(
        reopened.package.brushes[0].source_resource.as_deref(),
        Some(source_id.as_str())
    );
    assert_eq!(reopened.sources()[0].original_bytes(), raw.as_slice());
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", s.clone()).unwrap();
    catalog
        .set_source("round-pressure", Some(source_id))
        .unwrap();
    let state = brush_state::encode(&catalog).unwrap();
    let store = resources.source_store().unwrap();
    let restored = brush_state::decode_with_sources(
        &state,
        MAX_CATALOG_MASK_BYTES,
        Some(&resources),
        &store,
        &token,
    )
    .unwrap();
    let mut a = source();
    let mut b = source();
    let mut c = source();
    let ink = draw(&mut a, s.definition());
    assert_eq!(ink, draw(&mut b, reopened.definition(0).unwrap()));
    assert_eq!(
        ink,
        draw(&mut c, restored.selected().settings().definition())
    );
    assert_eq!(pixels(&a), pixels(&b));
    assert_eq!(pixels(&a), pixels(&c));
}
fn rewrite_manifest(bytes: &[u8], minor: u32) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).unwrap();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        if f.name() == "manifest.json" {
            let mut m: serde_json::Value = serde_json::from_slice(&data).unwrap();
            assert_eq!(m["version"]["minor"], 15);
            m["version"]["minor"] = minor.into();
            data = serde_json::to_vec(&m).unwrap();
        }
        writer
            .start_file(f.name(), SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
#[test]
fn wet_fields_versions_capabilities_and_archive_downgrade_are_strict() {
    let p = package(&settings(false, false, false));
    let good: serde_json::Value = serde_json::from_slice(&p.to_json().unwrap()).unwrap();
    let fields = ["wetness", "pickup", "mix", "load", "deposit", "consumption"];
    for field in fields {
        for value in [-0.0000000001, 1.0000000001] {
            let mut bad = good.clone();
            bad["brushes"][0]["wetMix"][field] = serde_json::json!(value);
            assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
    }
    for field in [
        "model",
        "enabled",
        "allLayers",
        "autoClean",
        "autoLoad",
        "wetness",
        "pickup",
        "mix",
        "load",
        "deposit",
        "consumption",
    ] {
        let mut bad = good.clone();
        bad["brushes"][0]["wetMix"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
        let mut bad = good.clone();
        bad["brushes"][0]["wetMix"][field] = serde_json::Value::Null;
        assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    for (field, value) in [
        ("model", serde_json::json!("spectral")),
        ("enabled", serde_json::json!(1)),
        ("autoLoad", serde_json::json!("true")),
        ("extra", serde_json::json!(0)),
    ] {
        let mut bad = good.clone();
        bad["brushes"][0]["wetMix"][field] = value;
        assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    for version in [1, 13, 15] {
        let mut bad = good.clone();
        bad["brushes"][0]["behaviorVersion"] = version.into();
        assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let mut bad = good.clone();
    bad["brushes"][0]["engineProfile"] = "legacy.round".into();
    assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    let mut bad = good.clone();
    bad["brushes"][0]["requiredCapabilities"]
        .as_array_mut()
        .unwrap()
        .retain(|v| v != "wet-mix");
    assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    let mut bad = good.clone();
    bad["brushes"][0]["wetMix"] = serde_json::Value::Null;
    assert!(Package::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    let token = CancellationToken::default();
    let bundle = ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    assert!(vbr_image::decode(&rewrite_manifest(&bytes, 14), &token).is_err());
    assert!(paint_io::vbr::encode(&bundle.package, &token).is_err());
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(vbr_image::encode(&bundle, &cancelled).is_err());
    assert!(vbr_image::decode(&bytes, &cancelled).is_err());
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-wet-v1.15.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["wetMix"]["properties"]["model"]["const"],
        WET_CARRIER_MODEL
    );
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        15
    );
}

#[test]
fn published_wet_fixtures_execute_and_failed_save_keeps_existing_file() {
    let closed: Preset = serde_json::from_str(include_str!(
        "../../../../contracts/fixtures/vbr-wet-dual-closed-valid.json"
    ))
    .unwrap();
    let token = CancellationToken::default();
    let closed_bundle = ImageBundle::new(
        Package {
            package_id: "12345678-1234-4234-8234-123456789abc".into(),
            brushes: vec![closed],
        },
        BTreeMap::new(),
        &token,
    )
    .unwrap();
    let mut closed_doc = source();
    draw(&mut closed_doc, closed_bundle.definition(0).unwrap());
    assert_eq!(closed_doc.history_depth(), (2, 0));

    let preset: Preset = serde_json::from_str(include_str!(
        "../../../../contracts/fixtures/vbr-wet-valid.json"
    ))
    .unwrap();
    assert!(serde_json::from_str::<Preset>(include_str!(
        "../../../../contracts/fixtures/vbr-wet-invalid-ratio.json"
    ))
    .is_err());
    let p = Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![preset],
    };
    let token = CancellationToken::default();
    let mut bundle = ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
    let mut doc = source();
    let before = pixels(&doc);
    draw(&mut doc, bundle.definition(0).unwrap());
    assert_ne!(pixels(&doc), before);
    assert_eq!(doc.history_depth(), (2, 0));
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("wet.vbr");
    std::fs::write(&path, b"keep original").unwrap();
    bundle.package.brushes[0].behavior_version = 13;
    assert!(vbr_image::save(&path, &bundle, &token).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"keep original");
    bundle.package.brushes[0].behavior_version = 14;
    bundle.package.brushes[0].dual_brush = Some(
        paint_io::brush_dual::DualParameters::from_core(
            &DualBrush::new(
                true,
                DualMode::Multiply,
                SecondaryTip::new(3., Default::default(), None, Default::default()).unwrap(),
            )
            .with_color(
                DualColor::new(
                    true,
                    true,
                    true,
                    DualColorOrder::SecondBehind,
                    BlendMode::Normal,
                    BlendMode::Normal,
                )
                .unwrap(),
            ),
        )
        .unwrap(),
    );
    bundle.package.brushes[0]
        .required_capabilities
        .extend(["dual-cover".into(), "dual-color-stroke".into()]);
    assert!(bundle.package.validate().is_err());
    assert!(vbr_image::save(&path, &bundle, &token).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"keep original");
}
