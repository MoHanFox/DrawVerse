use paint_core::*;
use paint_io::{
    brush_state,
    brush_taper::TaperParameters,
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
fn taper(enabled: bool) -> StrokeTaper {
    let c = PressureControl::new(
        PressureCurve::new_smooth(vec![(0., 0.), (0.4, 0.2), (1., 1.)]).unwrap(),
        0.25,
    )
    .unwrap();
    StrokeTaper::new(
        enabled,
        TaperEnd::new(8.125, c.clone(), PressureControl::default()).unwrap(),
        TaperEnd::new(12.75, PressureControl::default(), c).unwrap(),
    )
}
fn settings(enabled: bool) -> BrushSettings {
    BrushSettings {
        diameter: 6.,
        opacity: 0.35,
        spacing: 0.3,
        taper: taper(enabled),
        tip: BrushTip {
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn package(s: &BrushSettings) -> Package {
    let mut caps = vec![
        if s.mask.is_some() {
            "image-tip"
        } else {
            "procedural-tip"
        },
        "stroke-taper",
    ];
    if !s.smoothing.is_default() {
        caps.push("path-smoothing");
    }
    if !s.wet.is_default() {
        caps.push("wet-mix");
    }
    if !s.texture.is_default() {
        caps.push("texture-cover");
    }
    if !s.dual.is_default() {
        caps.push("dual-cover");
    }
    if s.dual.sizing() != Default::default() {
        caps.push("dual-relative-size");
    }
    if s.dual.color() != Default::default() {
        caps.push("dual-color-stroke");
    }
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "Taper".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 15,
            required_capabilities: caps.into_iter().map(String::from).collect(),
            parameters: Parameters {
                airbrush: None,
                speed_dynamics: None,
                tangent_dynamics: None,
                distance_dynamics: None,
                tilt_dynamics: None,
                rotation_dynamics: None,
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
            stroke_taper: Some(TaperParameters::from_core(&s.taper)),
            wet_mix: (!s.wet.is_default())
                .then(|| paint_io::brush_wet::WetParameters::from_core(s.wet)),
            smoothing: (!s.smoothing.is_default())
                .then(|| paint_io::brush_smoothing::SmoothingParameters::from_core(s.smoothing)),
            texture: (!s.texture.is_default()).then(|| {
                paint_io::brush_texture::TextureParameters::from_core(&s.texture).unwrap()
            }),
            dual_brush: (!s.dual.is_default())
                .then(|| paint_io::brush_dual::DualParameters::from_core(&s.dual).unwrap()),
            tip_resource: s
                .mask
                .as_ref()
                .and_then(|m| m.resource_id())
                .map(str::to_owned),
            source_resource: None,
        }],
    }
}
fn draw(def: BrushDefinition) -> (Vec<Pixel>, Vec<Pixel>) {
    let mut d = Document::new(96, 48).unwrap();
    let mut retained = None;
    let mut ink = Vec::new();
    for shift in [0., 5.] {
        let first = InputPoint::new(12.5, 20.5 + shift, 0.5);
        if def.wet.enabled() {
            d.begin_configured_wet_brush_stroke(def.clone(), first, retained.as_ref())
                .unwrap();
        } else {
            d.begin_brush_stroke(def.clone(), first).unwrap();
        }
        for (i, (x, p)) in [(28.5, 0.8), (48.5, 0.6), (72.5, 1.)]
            .into_iter()
            .enumerate()
        {
            let mut point = InputPoint::new(x, 20.5 + shift, p);
            point.timestamp_ns = (i + 1) as u64 * 16_000_000;
            d.stroke_to(point).unwrap();
        }
        if def.wet.enabled() {
            let (_, carrier) = d.end_wet_brush_stroke().unwrap();
            ink.push(carrier.sample(0.3, 0.7));
            retained = Some(carrier);
        } else {
            assert!(d.end_stroke().unwrap());
        }
    }
    assert_eq!(d.history_depth(), (2, 0));
    let pixels: Vec<_> = (0..48)
        .flat_map(|y| (0..96).map(move |x| (x, y)))
        .map(|(x, y)| d.pixel(x, y))
        .collect();
    d.undo().unwrap();
    d.undo().unwrap();
    assert_eq!(d.tile_count(), 0);
    d.redo().unwrap();
    d.redo().unwrap();
    for y in 0..48 {
        for x in 0..96 {
            assert_eq!(d.pixel(x, y), pixels[(y * 96 + x) as usize]);
        }
    }
    (pixels, ink)
}
fn rewrite_minor(bytes: &[u8], minor: u32) -> Vec<u8> {
    let mut a = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut w = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..a.len() {
        let mut f = a.by_index(i).unwrap();
        let mut b = Vec::new();
        f.read_to_end(&mut b).unwrap();
        if f.name() == "manifest.json" {
            let mut m: serde_json::Value = serde_json::from_slice(&b).unwrap();
            assert_eq!(m["version"]["minor"], 22);
            m["version"]["minor"] = minor.into();
            b = serde_json::to_vec(&m).unwrap();
        }
        w.start_file(f.name(), SimpleFileOptions::default())
            .unwrap();
        w.write_all(&b).unwrap();
    }
    w.finish().unwrap().into_inner()
}
#[test]
fn state18_vbr22_preserve_closed_and_active_two_strokes_with_dual_or_wet() {
    let token = CancellationToken::default();
    for enabled in [false, true] {
        for kind in 0..3 {
            let mut s = settings(enabled);
            s.smoothing = BrushSmoothing::new(true, 4., 8., 0.2, 0.).unwrap();
            if kind == 1 {
                s.dual = DualBrush::new(
                    true,
                    DualMode::Multiply,
                    SecondaryTip::new(8., BrushTip::default(), None, Default::default())
                        .unwrap()
                        .with_spacing(0.15)
                        .unwrap(),
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
                );
            }
            if kind == 2 {
                s.wet = WetSettings::new(
                    true,
                    false,
                    false,
                    true,
                    WetMix::new(0.4, 0.3, 0.5, 0.8, 0.8, 0.1).unwrap(),
                );
            }
            let mut catalog = BrushCatalog::default();
            catalog.edit("round-pressure", s.clone()).unwrap();
            let state = brush_state::encode(&catalog).unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&state).unwrap()["version"],
                18
            );
            let restored = brush_state::decode(&state, MAX_CATALOG_MASK_BYTES).unwrap();
            assert_eq!(restored.selected().settings().taper, s.taper);
            assert_eq!(brush_state::encode(&restored).unwrap(), state);
            let bundle = ImageBundle::new(package(&s), BTreeMap::new(), &token).unwrap();
            let bytes = vbr_image::encode(&bundle, &token).unwrap();
            let reopened = vbr_image::decode(&bytes, &token).unwrap();
            assert_eq!(reopened.definition(0).unwrap().taper, s.taper);
            let original = draw(s.definition());
            assert_eq!(original, draw(restored.selected().settings().definition()));
            assert_eq!(original, draw(reopened.definition(0).unwrap()));
            for minor in [0, 15, 21] {
                assert!(vbr_image::decode(&rewrite_minor(&bytes, minor), &token).is_err());
            }
            let captured = catalog.selected();
            let mut changed = s.clone();
            changed.taper = StrokeTaper::default();
            catalog.edit("round-pressure", changed).unwrap();
            assert_eq!(captured.settings().taper, s.taper);
            assert_eq!(original, draw(captured.settings().definition()));
        }
    }
}
#[test]
fn taper_strict_nested_fields_and_pre_rounding_range_endpoints_and_order() {
    let valid = serde_json::to_value(TaperParameters::from_core(&taper(false))).unwrap();
    let mut cases = Vec::new();
    for path in ["/start/lengthPx", "/end/lengthPx"] {
        for value in [-0.0000000001, 4096.000000001] {
            let mut x = valid.clone();
            *x.pointer_mut(path).unwrap() = value.into();
            cases.push(x);
        }
    }
    for path in [
        "/start/size/minimum",
        "/start/size/curve/1/0",
        "/end/opacity/curve/1/1",
    ] {
        let mut x = valid.clone();
        *x.pointer_mut(path).unwrap() = 1.0000000001.into();
        cases.push(x);
    }
    for (path, value) in [
        ("/start/size/curve/0/0", 1e-50),
        ("/start/size/curve/0/1", 1e-50),
        ("/start/size/curve/2/1", 0.99999999999),
    ] {
        let mut x = valid.clone();
        *x.pointer_mut(path).unwrap() = value.into();
        cases.push(x);
    }
    for path in ["/start/size", "/end", "/enabled"] {
        let mut x = valid.clone();
        *x.pointer_mut(path).unwrap() = serde_json::Value::Null;
        cases.push(x);
    }
    for path in ["/model", "/start/size/interpolation"] {
        let mut x = valid.clone();
        *x.pointer_mut(path).unwrap() = "unknown".into();
        cases.push(x);
    }
    let mut unknown = valid.clone();
    unknown["start"]["extra"] = true.into();
    cases.push(unknown);
    let mut missing = valid.clone();
    missing["end"].as_object_mut().unwrap().remove("opacity");
    cases.push(missing);
    for curve in [
        serde_json::json!([[0., 0.], [0.5, 0.2], [0.50000000001, 0.7], [1., 1.]]),
        serde_json::json!([[0., 0.], [0.6, 0.2], [0.4, 0.7], [1., 1.]]),
        serde_json::json!([]),
        serde_json::json!(vec![[0., 0.]; 18]),
    ] {
        let mut x = valid.clone();
        x["start"]["size"]["curve"] = curve;
        cases.push(x);
    }
    for x in cases {
        assert!(serde_json::from_value::<TaperParameters>(x).is_err());
    }
    for length in [0., 4096.] {
        let mut x = valid.clone();
        x["start"]["lengthPx"] = length.into();
        assert!(serde_json::from_value::<TaperParameters>(x).is_ok());
    }
}
#[test]
fn default_old_versions_unchanged_and_taper_requires_own_state_behavior_and_capability() {
    let catalog = BrushCatalog::default();
    let original = brush_state::encode(&catalog).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&original).unwrap();
    assert_eq!(value["version"], 2);
    assert!(!String::from_utf8(original.clone())
        .unwrap()
        .contains("strokeTaper"));
    assert_eq!(
        original,
        brush_state::encode(&brush_state::decode(&original, MAX_CATALOG_MASK_BYTES).unwrap())
            .unwrap()
    );
    let mut catalog = catalog;
    catalog.edit("round-pressure", settings(false)).unwrap();
    let mut state: serde_json::Value =
        serde_json::from_slice(&brush_state::encode(&catalog).unwrap()).unwrap();
    for version in [2, 13, 17] {
        state["version"] = version.into();
        assert!(
            brush_state::decode(&serde_json::to_vec(&state).unwrap(), MAX_CATALOG_MASK_BYTES)
                .is_err()
        );
    }
    let mut p = package(&settings(false));
    p.brushes[0].behavior_version = 14;
    assert!(p.validate().is_err());
    p.brushes[0].behavior_version = 15;
    p.brushes[0]
        .required_capabilities
        .retain(|c| c != "stroke-taper");
    assert!(p.validate().is_err());
    p.brushes[0]
        .required_capabilities
        .push("stroke-taper".into());
    p.brushes[0].engine_profile = "legacy.round".into();
    assert!(p.validate().is_err());
    p.brushes[0].engine_profile = "drawverse.brush".into();
    p.brushes[0].stroke_taper = None;
    assert!(p.validate().is_err());
    let token = CancellationToken::default();
    let mut mixed = package(&settings(false));
    let mut legacy = mixed.brushes[0].clone();
    legacy.id = "12345678-1234-4234-8234-123456789abe".into();
    legacy.stroke_taper = None;
    legacy.behavior_version = 1;
    legacy.required_capabilities = vec!["procedural-tip".into()];
    mixed.brushes.push(legacy.clone());
    let bundle = ImageBundle::new(mixed, BTreeMap::new(), &token).unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    assert!(vbr_image::decode(&rewrite_minor(&bytes, 21), &token).is_err());
    assert_eq!(
        vbr_image::decode(&bytes, &token)
            .unwrap()
            .package
            .brushes
            .len(),
        2
    );
    let legacy = ImageBundle::new(
        Package {
            package_id: bundle.package.package_id.clone(),
            brushes: vec![legacy],
        },
        BTreeMap::new(),
        &token,
    )
    .unwrap();
    let bytes = vbr_image::encode(&legacy, &token).unwrap();
    let mut a = ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut manifest = Vec::new();
    a.by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut manifest)
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&manifest).unwrap()["version"]["minor"],
        0
    );
}

#[test]
fn published_taper_fixtures_execute_and_explicit_state_null_is_rejected() {
    let token = CancellationToken::default();
    let mut results = Vec::new();
    for bytes in [
        include_bytes!("../../../../contracts/fixtures/taper-v1.22-valid.json").as_slice(),
        include_bytes!("../../../../contracts/fixtures/taper-v1.22-closed.json").as_slice(),
    ] {
        let preset: Preset = serde_json::from_slice(bytes).unwrap();
        let p = Package {
            package_id: "12345678-1234-4234-8234-123456789abc".into(),
            brushes: vec![preset],
        };
        let bundle = ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
        results.push(draw(bundle.definition(0).unwrap()).0);
    }
    assert_ne!(results[0], results[1]);
    assert!(serde_json::from_slice::<Preset>(include_bytes!(
        "../../../../contracts/fixtures/taper-v1.22-invalid.json"
    ))
    .is_err());
    let mut c = BrushCatalog::default();
    c.edit("round-pressure", settings(false)).unwrap();
    let mut state: serde_json::Value =
        serde_json::from_slice(&brush_state::encode(&c).unwrap()).unwrap();
    state["presets"][0]["strokeTaper"] = serde_json::Value::Null;
    assert!(
        brush_state::decode(&serde_json::to_vec(&state).unwrap(), MAX_CATALOG_MASK_BYTES).is_err()
    );
}
#[test]
fn source_image_texture_identity_and_current_taper_survive_fresh_store_exchange() {
    use paint_io::{abr_import::ImportBatch, brush_resource::ResourceStore};
    let token = CancellationToken::default();
    let tmp = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(tmp.path().join("old"), MAX_CATALOG_MASK_BYTES).unwrap();
    let raw = Arc::new(support::abr_fixture::fixture(6, 2, false));
    let imported = ImportBatch::decode(raw.clone(), MAX_CATALOG_MASK_BYTES, &token)
        .unwrap()
        .persist(&store, &token)
        .unwrap();
    let source_id = imported[0].source_id().to_owned();
    let mask = imported[0].settings().mask.clone().unwrap();
    let id = mask.resource_id().unwrap().to_owned();
    let mut s = settings(true);
    s.mask = Some(mask.clone());
    s.texture = BrushTexture::new(
        true,
        Default::default(),
        Some(Arc::new(TextureResource::new(mask))),
    )
    .unwrap();
    let mut p = package(&s);
    p.brushes[0].source_resource = Some(source_id.clone());
    let sources = store
        .source_store()
        .unwrap()
        .export_batches(
            &std::collections::BTreeSet::from([source_id.clone()]),
            &token,
        )
        .unwrap();
    let records: Vec<_> = sources[0]
        .records()
        .map(|(_, bytes)| bytes.to_vec())
        .collect();
    let bundle = ImageBundle::with_sources(
        p,
        BTreeMap::from([(
            id,
            store
                .encoded(s.mask.as_ref().unwrap().resource_id().unwrap())
                .unwrap(),
        )]),
        sources,
        &token,
    )
    .unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    let reopened = vbr_image::decode(&bytes, &token).unwrap();
    assert_eq!(reopened.sources()[0].original_bytes(), raw.as_slice());
    assert_eq!(
        records,
        reopened.sources()[0]
            .records()
            .map(|(_, b)| b.to_vec())
            .collect::<Vec<_>>()
    );
    assert_eq!(draw(s.definition()), draw(reopened.definition(0).unwrap()));
    let new = ResourceStore::open(tmp.path().join("new"), MAX_CATALOG_MASK_BYTES).unwrap();
    for (id, asset) in reopened.assets() {
        new.put_verified(id, asset.bytes(), &token).unwrap();
    }
    for source in reopened.sources() {
        new.source_store()
            .unwrap()
            .save_batch(source, &token)
            .unwrap();
    }
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", s).unwrap();
    catalog
        .set_source("round-pressure", Some(source_id))
        .unwrap();
    let state = brush_state::encode(&catalog).unwrap();
    let restored = brush_state::decode_with_sources(
        &state,
        MAX_CATALOG_MASK_BYTES,
        Some(&new),
        &new.source_store().unwrap(),
        &token,
    )
    .unwrap();
    assert_eq!(
        draw(restored.selected().settings().definition()),
        draw(reopened.definition(0).unwrap())
    );
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    let path = tmp.path().join("untouched.vbr");
    std::fs::write(&path, b"keep").unwrap();
    assert!(vbr_image::save(&path, &bundle, &cancelled).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"keep");
}

#[test]
fn source_baselines_reject_nondefault_taper_including_closed_configuration() {
    use paint_io::{
        abr_conversion::ConvertedPreset,
        brush_source::{SourceArchive, SourceBatch},
        ps_descriptor::{Class, Descriptor},
    };
    let descriptor = Descriptor {
        class: Class {
            name: String::new(),
            id: b"null".to_vec(),
        },
        entries: Vec::new(),
    };
    for enabled in [false, true] {
        let converted = ConvertedPreset {
            converter_version: 1,
            name: "taper",
            settings: settings(enabled),
            source: &descriptor,
            issues: Vec::new(),
        };
        let raw = Arc::new(vec![0, 6, 0, 2, 42]);
        let token = CancellationToken::default();
        assert!(matches!(
            SourceArchive::capture(raw.clone(), 0, &converted),
            Err(paint_io::Error::Unsupported(_))
        ));
        assert!(matches!(
            SourceBatch::capture(raw, std::slice::from_ref(&converted), &token),
            Err(paint_io::Error::Unsupported(_))
        ));
    }
}
