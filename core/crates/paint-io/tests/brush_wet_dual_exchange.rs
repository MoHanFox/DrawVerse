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
    let tip = BrushTip {
        pressure_flow: false,
        ..Default::default()
    };
    BrushSettings {
        diameter: 6.,
        opacity: 0.35,
        spacing: 0.3,
        tip,
        taper: taper(true),
        smoothing: BrushSmoothing::new(true, 4., 8., 0.2, 0.).unwrap(),
        wet: WetSettings::new(
            enabled,
            false,
            false,
            true,
            WetMix::new(0.4, 0.3, 0.5, 0.8, 0.8, 0.1).unwrap(),
        )
        .with_dual_color(true),
        dual: DualBrush::new(
            true,
            DualMode::Multiply,
            SecondaryTip::new(8., tip, None, Default::default())
                .unwrap()
                .with_spacing(0.25)
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
        ),
        ..Default::default()
    }
}
#[test]
fn groups_state20_three_image_source_and_two_wet_strokes_survive_fresh_directory() {
    use paint_io::{abr_import::ImportBatch, brush_resource::ResourceStore};
    for enabled in [false, true] {
        let token = CancellationToken::default();
        let tmp = tempfile::tempdir().unwrap();
        let old = ResourceStore::open(tmp.path().join("old"), MAX_CATALOG_MASK_BYTES).unwrap();
        let raw = Arc::new(support::abr_fixture::fixture(6, 2, false));
        let imported = ImportBatch::decode(raw.clone(), MAX_CATALOG_MASK_BYTES, &token)
            .unwrap()
            .persist(&old, &token)
            .unwrap();
        let source = imported[0].source_id().to_owned();
        let mask = imported[0].settings().mask.clone().unwrap();
        let resource = mask.resource_id().unwrap().to_owned();
        let mut s = settings(enabled);
        s.mask = Some(mask.clone());
        s.dual = DualBrush::new(
            true,
            DualMode::Multiply,
            SecondaryTip::new(8., s.tip, Some(mask.clone()), Default::default())
                .unwrap()
                .with_spacing(0.25)
                .unwrap(),
        )
        .with_color(s.dual.color());
        s.texture = BrushTexture::new(
            true,
            Default::default(),
            Some(Arc::new(TextureResource::new(mask))),
        )
        .unwrap();
        let mut c = BrushCatalog::default();
        c.edit("round-pressure", s).unwrap();
        c.set_source("round-pressure", Some(source.clone()))
            .unwrap();
        let group = "12345678-1234-4234-8234-000000000001";
        let empty = "12345678-1234-4234-8234-000000000002";
        c.create_group(group.into(), "same".into(), None, 0)
            .unwrap();
        c.create_group(empty.into(), "same".into(), Some(group), 0)
            .unwrap();
        c.move_catalog_item(
            BrushCatalogItem::Preset("round-pressure".into()),
            Some(group),
            1,
        )
        .unwrap();
        c.save_copy("12345678-1234-4234-8234-000000000003".into(), "copy".into())
            .unwrap();
        let bytes = brush_state::encode(&c).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["version"],
            20
        );
        let new = ResourceStore::open(tmp.path().join("new"), MAX_CATALOG_MASK_BYTES).unwrap();
        new.put_verified(&resource, &old.encoded(&resource).unwrap(), &token)
            .unwrap();
        let batches = old
            .source_store()
            .unwrap()
            .export_batches(&std::collections::BTreeSet::from([source.clone()]), &token)
            .unwrap();
        for batch in &batches {
            new.source_store()
                .unwrap()
                .save_batch(batch, &token)
                .unwrap();
        }
        std::fs::remove_file(old.directory().join(format!("{resource}.png"))).unwrap();
        let (restored, receipts) = brush_state::decode_captured(
            &bytes,
            MAX_CATALOG_MASK_BYTES,
            &new,
            &new.source_store().unwrap(),
            &token,
        )
        .unwrap();
        assert!(!receipts.is_empty());
        assert_eq!(*restored.organization(), *c.organization());
        assert_eq!(brush_state::encode(&restored).unwrap(), bytes);
        let again = new
            .source_store()
            .unwrap()
            .export_batches(&std::collections::BTreeSet::from([source]), &token)
            .unwrap();
        assert_eq!(again[0].original_bytes(), raw.as_slice());
        assert_eq!(
            draw(c.selected().settings().definition()),
            draw(restored.selected().settings().definition())
        );
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
        "wet-dual-color",
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
            behavior_version: 16,
            required_capabilities: caps.into_iter().map(String::from).collect(),
            parameters: Parameters {
                airbrush: None,
                speed_dynamics: None,
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
fn draw(mut def: BrushDefinition) -> (Vec<Pixel>, Vec<Pixel>) {
    def.brush.color = Pixel::from_straight([0.1, 0.3, 1., 0.8]).unwrap();
    def.brush.background_color = Pixel::from_straight([1., 0.1, 0.05, 0.7]).unwrap();
    let mut d = Document::new(96, 48).unwrap();
    let mut retained = None;
    let mut ink = Vec::new();
    for shift in [0., 5.] {
        let mut captured = def.clone();
        if shift > 0. {
            let w = captured.wet;
            captured.wet = WetSettings::new(w.enabled(), w.all_layers(), false, false, w.mix())
                .with_dual_color(w.dual_color());
        }
        let first = InputPoint::new(12.5, 20.5 + shift, 0.5);
        if captured.wet.enabled() {
            d.begin_configured_dual_wet_brush_stroke(captured, first, retained.as_ref())
                .unwrap();
        } else {
            d.begin_brush_stroke(captured, first).unwrap();
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
            let (_, state) = d.end_wet_ink_stroke().unwrap();
            for carrier in [state.primary(), state.secondary().unwrap()] {
                for y in 0..carrier.edge() {
                    for x in 0..carrier.edge() {
                        ink.push(carrier.sample(
                            (x as f64 + 0.5) / carrier.edge() as f64,
                            (y as f64 + 0.5) / carrier.edge() as f64,
                        ));
                    }
                }
            }
            retained = Some(state);
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
            assert_eq!(m["version"]["minor"], 23);
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
fn published_wet_dual_fixtures_execute_real_active_and_closed_models() {
    let token = CancellationToken::default();
    let mut results = Vec::new();
    for bytes in [
        include_bytes!("../../../../contracts/fixtures/wet-dual-v1.23-valid.json").as_slice(),
        include_bytes!("../../../../contracts/fixtures/wet-dual-v1.23-closed.json").as_slice(),
    ] {
        let preset: Preset = serde_json::from_slice(bytes).unwrap();
        let bundle = ImageBundle::new(
            Package {
                package_id: "12345678-1234-4234-8234-123456789abc".into(),
                brushes: vec![preset],
            },
            BTreeMap::new(),
            &token,
        )
        .unwrap();
        assert!(bundle.definition(0).unwrap().wet.dual_color());
        results.push(draw(bundle.definition(0).unwrap()).0);
    }
    assert_ne!(results[0], results[1]);
    assert!(serde_json::from_slice::<Preset>(include_bytes!(
        "../../../../contracts/fixtures/wet-dual-v1.23-invalid.json"
    ))
    .is_err());
}
#[test]
fn state19_vbr23_keep_closed_policy_and_real_two_stroke_pixels_and_both_carriers() {
    let token = CancellationToken::default();
    for enabled in [false, true] {
        for path in [false, true] {
            let mut s = settings(enabled);
            if !path {
                s.dual = DualBrush::new(
                    true,
                    DualMode::Multiply,
                    SecondaryTip::new(8., s.tip, None, Default::default()).unwrap(),
                )
                .with_color(s.dual.color());
            }
            let mut catalog = BrushCatalog::default();
            catalog.edit("round-pressure", s.clone()).unwrap();
            let held = catalog.selected();
            let expected = draw(s.definition());
            let bytes = brush_state::encode(&catalog).unwrap();
            let data: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(data["version"], 19);
            assert_eq!(
                data["presets"][0]["wetMix"]["dualColor"],
                "independent-carriers-v1"
            );
            let restored = brush_state::decode(&bytes, MAX_CATALOG_MASK_BYTES).unwrap();
            assert_eq!(restored.selected().settings().wet, s.wet);
            assert_eq!(brush_state::encode(&restored).unwrap(), bytes);
            assert_eq!(draw(restored.selected().settings().definition()), expected);
            let bundle = ImageBundle::new(package(&s), BTreeMap::new(), &token).unwrap();
            let archive = vbr_image::encode(&bundle, &token).unwrap();
            let reopened = vbr_image::decode(&archive, &token).unwrap();
            assert!(reopened.package.brushes[0].has_wet_dual());
            assert_eq!(draw(reopened.definition(0).unwrap()), expected);
            for minor in [0, 15, 22] {
                assert!(vbr_image::decode(&rewrite_minor(&archive, minor), &token).is_err());
            }
            let mut changed = s;
            changed.wet = WetSettings::default();
            catalog.edit("round-pressure", changed).unwrap();
            assert!(held.settings().wet.dual_color());
            assert_eq!(draw(held.settings().definition()), expected);
        }
    }
}
#[test]
fn wet_dual_marker_is_strict_and_requires_own_state_behavior_capability_and_minor() {
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", settings(true)).unwrap();
    let state: serde_json::Value =
        serde_json::from_slice(&brush_state::encode(&catalog).unwrap()).unwrap();
    for invalid in [
        serde_json::Value::Null,
        serde_json::json!(true),
        serde_json::json!("shared-v0"),
        serde_json::json!({}),
        serde_json::json!([]),
    ] {
        let mut bad = state.clone();
        bad["presets"][0]["wetMix"]["dualColor"] = invalid;
        assert!(
            brush_state::decode(&serde_json::to_vec(&bad).unwrap(), MAX_CATALOG_MASK_BYTES)
                .is_err()
        );
    }
    let mut old = state.clone();
    old["version"] = 18.into();
    assert!(
        brush_state::decode(&serde_json::to_vec(&old).unwrap(), MAX_CATALOG_MASK_BYTES).is_err()
    );
    let mut missing = state;
    missing["presets"][0]["wetMix"]
        .as_object_mut()
        .unwrap()
        .remove("dualColor");
    assert!(brush_state::decode(
        &serde_json::to_vec(&missing).unwrap(),
        MAX_CATALOG_MASK_BYTES
    )
    .is_err());
    for enabled in [false, true] {
        let good = package(&settings(enabled));
        good.validate().unwrap();
        let mut p = good.clone();
        p.brushes[0]
            .required_capabilities
            .retain(|c| c != "wet-dual-color");
        assert!(p.validate().is_err());
        p = good.clone();
        p.brushes[0].behavior_version = 15;
        assert!(p.validate().is_err());
        p = good.clone();
        p.brushes[0].engine_profile = "legacy.round".into();
        assert!(p.validate().is_err());
        let mut json: serde_json::Value = serde_json::from_slice(&good.to_json().unwrap()).unwrap();
        json["brushes"][0]["wetMix"]["dualColor"] = serde_json::Value::Null;
        assert!(Package::from_json(&serde_json::to_vec(&json).unwrap()).is_err());
    }
}
#[test]
fn default_legacy_bytes_and_mixed_highest_minor_are_preserved() {
    let mut c = BrushCatalog::default();
    let before = brush_state::encode(&c).unwrap();
    let mut s = c.selected().settings().clone();
    s.wet = s.wet.with_dual_color(true);
    c.edit("round-pressure", s.clone()).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&brush_state::encode(&c).unwrap()).unwrap()
            ["version"],
        19
    );
    s.wet = s.wet.with_dual_color(false);
    c.edit("round-pressure", s).unwrap();
    assert_eq!(brush_state::encode(&c).unwrap(), before);
    let mut mixed = package(&settings(false));
    let mut legacy = mixed.brushes[0].clone();
    legacy.id = "12345678-1234-4234-8234-123456789abe".into();
    legacy.stroke_taper = None;
    legacy.wet_mix = None;
    legacy.dual_brush = None;
    legacy.smoothing = None;
    legacy.behavior_version = 1;
    legacy.required_capabilities = vec!["procedural-tip".into()];
    mixed.brushes.push(legacy.clone());
    let token = CancellationToken::default();
    let minor = |bytes: &[u8]| {
        let mut zip = ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut bytes = Vec::new();
        zip.by_name("manifest.json")
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["version"]["minor"]
            .as_u64()
            .unwrap()
    };
    let data = vbr_image::encode(
        &ImageBundle::new(mixed, BTreeMap::new(), &token).unwrap(),
        &token,
    )
    .unwrap();
    assert_eq!(minor(&data), 23);
    assert_eq!(
        vbr_image::decode(&data, &token)
            .unwrap()
            .package
            .brushes
            .len(),
        2
    );
    let data = vbr_image::encode(
        &ImageBundle::new(
            Package {
                package_id: "12345678-1234-4234-8234-123456789abc".into(),
                brushes: vec![legacy],
            },
            BTreeMap::new(),
            &token,
        )
        .unwrap(),
        &token,
    )
    .unwrap();
    assert_eq!(minor(&data), 0);
}
#[test]
fn old_abr_baselines_reject_even_only_the_closed_new_strategy_marker() {
    use paint_io::{
        abr_conversion::ConvertedPreset,
        brush_source::SourceArchive,
        ps_descriptor::{Class, Descriptor},
    };
    let descriptor = Descriptor {
        class: Class {
            name: String::new(),
            id: b"null".to_vec(),
        },
        entries: vec![],
    };
    let mut settings = BrushSettings::default();
    settings.wet = settings.wet.with_dual_color(true);
    let converted = ConvertedPreset {
        converter_version: 1,
        name: "closed pair",
        settings,
        source: &descriptor,
        issues: vec![],
    };
    assert!(matches!(
        SourceArchive::capture(Arc::new(vec![0, 6, 0, 2, 42]), 0, &converted),
        Err(paint_io::Error::Unsupported(_))
    ));
}
#[test]
fn wet_dual_source_three_image_roles_and_taper_survive_fresh_store_exchange() {
    for grouped in [false, true] {
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
        s.dual = DualBrush::new(
            true,
            DualMode::Multiply,
            SecondaryTip::new(8., s.tip, Some(mask.clone()), Default::default())
                .unwrap()
                .with_spacing(0.25)
                .unwrap(),
        )
        .with_color(s.dual.color());
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
        let group = "12345678-1234-4234-8234-000000000100";
        let bundle = if grouped {
            let preset = bundle.package.brushes[0].id.clone();
            bundle
                .with_organization(
                    BrushOrganization::new(
                        vec![BrushCatalogItem::Group(group.into())],
                        vec![BrushGroup::new(
                            group.into(),
                            "source group".into(),
                            vec![BrushCatalogItem::Preset(preset)],
                        )
                        .unwrap()],
                    )
                    .unwrap(),
                )
                .unwrap()
        } else {
            bundle
        };
        let bytes = vbr_image::encode(&bundle, &token).unwrap();
        let mut reopened = vbr_image::decode(&bytes, &token).unwrap();
        assert_eq!(reopened.organization(), bundle.organization());
        if grouped {
            let old = reopened.package.brushes[0].id.clone();
            reopened
                .remap_copy_ids(
                    "12345678-1234-4234-8234-000000000200".into(),
                    &BTreeMap::from([(old, "12345678-1234-4234-8234-000000000201".into())]),
                    &BTreeMap::from([(
                        group.into(),
                        "12345678-1234-4234-8234-000000000202".into(),
                    )]),
                    &token,
                )
                .unwrap();
            let copy =
                vbr_image::decode(&vbr_image::encode(&reopened, &token).unwrap(), &token).unwrap();
            assert_eq!(copy.organization(), reopened.organization());
            reopened = copy;
        }

        assert_eq!(reopened.sources()[0].original_bytes(), raw.as_slice());
        assert_eq!(
            records,
            reopened.sources()[0]
                .records()
                .map(|(_, b)| b.to_vec())
                .collect::<Vec<_>>()
        );
        assert_eq!(draw(s.definition()), draw(reopened.definition(0).unwrap()));
        let mut catalog = paint_core::BrushCatalog::default();
        reopened
            .import_into_catalog(&mut catalog, None, 2, &token)
            .unwrap();
        assert_eq!(
            catalog.selected().source_resource(),
            Some(source_id.as_str())
        );
        assert_eq!(
            draw(s.definition()),
            draw(catalog.selected().settings().definition())
        );
        assert_eq!(catalog.organization().groups().len(), usize::from(grouped));
        let new = ResourceStore::open(tmp.path().join("new"), MAX_CATALOG_MASK_BYTES).unwrap();
        let mut imported = BrushCatalog::default();
        if grouped {
            let directory = tmp.path().join("fault");
            let fault = ResourceStore::open(directory.clone(), MAX_CATALOG_MASK_BYTES).unwrap();
            let obstruction = directory.join("sources");
            std::fs::write(&obstruction, b"keep source obstruction").unwrap();
            let before = imported.clone();
            assert!(reopened
                .import_into_catalog_persisted(&mut imported, &fault, None, 1, &token)
                .is_err());
            assert_eq!(imported.revision(), before.revision());
            assert!(Arc::ptr_eq(
                &imported.organization(),
                &before.organization()
            ));
            assert!(Arc::ptr_eq(&imported.selected(), &before.selected()));
            assert_eq!(
                std::fs::read(&obstruction).unwrap(),
                b"keep source obstruction"
            );
            for (id, asset) in reopened.assets() {
                assert_eq!(
                    std::fs::read(directory.join(format!("{id}.png"))).unwrap(),
                    asset.bytes()
                );
            }
            assert!(std::fs::read_dir(&directory).unwrap().all(|e| !e
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".brush-")));
            std::fs::remove_file(obstruction).unwrap();
            std::fs::create_dir(directory.join("sources")).unwrap();
            let batch = &reopened.sources()[0];
            let record_id = batch.ids().next().unwrap();
            let record_path = directory.join("sources").join(format!("{record_id}.json"));
            std::fs::write(&record_path, b"corrupt existing source record").unwrap();
            assert!(reopened
                .import_into_catalog_persisted(&mut imported, &fault, None, 1, &token)
                .is_err());
            assert_eq!(imported.revision(), before.revision());
            assert!(Arc::ptr_eq(
                &imported.organization(),
                &before.organization()
            ));
            assert!(Arc::ptr_eq(&imported.selected(), &before.selected()));
            assert_eq!(
                std::fs::read(&record_path).unwrap(),
                b"corrupt existing source record"
            );
            assert_eq!(
                std::fs::read(directory.join("sources").join(format!(
                    "{}.abr",
                    paint_io::brush_resource::hash(batch.original_bytes())
                )))
                .unwrap(),
                raw.as_slice()
            );
            assert!(std::fs::read_dir(directory.join("sources"))
                .unwrap()
                .all(|e| !e
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".brush-source-")));
            std::fs::remove_file(record_path).unwrap();
            reopened
                .import_into_catalog_persisted(&mut imported, &fault, None, 1, &token)
                .unwrap();
            assert_eq!(
                draw(imported.selected().settings().definition()),
                draw(reopened.definition(0).unwrap())
            );
            imported = before;
        }
        reopened
            .import_into_catalog_persisted(&mut imported, &new, None, 1, &token)
            .unwrap();
        let imported_state = brush_state::encode(&imported).unwrap();
        let fresh = ResourceStore::open(tmp.path().join("new"), MAX_CATALOG_MASK_BYTES).unwrap();
        let imported_restored = brush_state::decode_with_sources(
            &imported_state,
            MAX_CATALOG_MASK_BYTES,
            Some(&fresh),
            &fresh.source_store().unwrap(),
            &token,
        )
        .unwrap();
        assert_eq!(
            brush_state::encode(&imported_restored).unwrap(),
            imported_state
        );
        assert_eq!(imported_restored.organization(), imported.organization());
        assert_eq!(
            draw(imported_restored.selected().settings().definition()),
            draw(reopened.definition(0).unwrap())
        );
        let batches = fresh
            .source_store()
            .unwrap()
            .export_batches(
                &std::collections::BTreeSet::from([source_id.clone()]),
                &token,
            )
            .unwrap();
        assert_eq!(batches[0].original_bytes(), raw.as_slice());
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
}
