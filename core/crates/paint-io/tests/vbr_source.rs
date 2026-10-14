use paint_core::{BrushMask, Document, InputPoint, MAX_CATALOG_MASK_BYTES};
use paint_io::{
    abr_import::ImportBatch,
    brush_resource::{encode_mask, hash, ResourceStore},
    brush_source::SourceStore,
    vbr::{Package, Parameters, Preset},
    vbr_image::{self, ImageBundle},
};
use paint_task::CancellationToken;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read, Write},
    sync::Arc,
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};
mod support {
    pub mod abr_fixture;
}
fn bundle() -> ImageBundle {
    let token = CancellationToken::default();
    let temp = tempfile::tempdir().unwrap();
    let resources =
        ResourceStore::open(temp.path().join("assets"), MAX_CATALOG_MASK_BYTES).unwrap();
    let batch = ImportBatch::decode(
        Arc::new(support::abr_fixture::fixture(6, 2, false)),
        MAX_CATALOG_MASK_BYTES,
        &token,
    )
    .unwrap();
    let imported = batch.persist(&resources, &token).unwrap();
    let ids = imported
        .iter()
        .map(|p| p.source_id().to_owned())
        .collect::<BTreeSet<_>>();
    let sources = resources
        .source_store()
        .unwrap()
        .export_batches(&ids, &token)
        .unwrap();
    assert_eq!(sources.len(), 1);
    let mut pngs = BTreeMap::new();
    for preset in &imported {
        if let Some(mask) = &preset.settings().mask {
            let id = mask.resource_id().unwrap();
            pngs.insert(id.into(), resources.encoded(id).unwrap());
        }
    }
    let brushes = imported
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let s = p.settings();
            Preset {
                texture: None,
                dual_brush: None,
                id: format!("12345678-1234-4234-8234-{i:012x}"),
                name: p.name().into(),
                engine_profile: "drawverse.brush".into(),
                behavior_version: 1,
                required_capabilities: vec![if s.mask.is_some() {
                    "image-tip".into()
                } else {
                    "procedural-tip".into()
                }],
                tip_resource: s
                    .mask
                    .as_ref()
                    .and_then(|m| m.resource_id())
                    .map(str::to_owned),
                source_resource: Some(p.source_id().into()),
                parameters: Parameters {
                    scatter_dynamics: None,
                    shape_jitter: None,
                    color_dynamics: None,
                    tip_dynamics: None,
                    pressure_dynamics: None,
                    diameter: s.diameter,
                    spacing: s.spacing,
                    dab_opacity: s.opacity,
                    hardness: s.tip.hardness,
                    roundness: s.tip.roundness,
                    angle: s.tip.angle,
                    flow: s.tip.flow,
                    pressure_size: s.tip.pressure_size,
                    pressure_flow: s.tip.pressure_flow,
                },
            }
        })
        .collect();
    ImageBundle::with_sources(
        Package {
            package_id: "12345678-1234-4234-8234-123456789abc".into(),
            brushes,
        },
        pngs,
        sources,
        &token,
    )
    .unwrap()
}

#[test]
fn edited_device_geometry_preserves_source_original_baseline_and_image_pixels() {
    let token = CancellationToken::default();
    let mut input = bundle();
    let tip =
        paint_core::TipDynamics::new(paint_core::AngleControl::PenRotation, true, 0.2).unwrap();
    input.package.brushes[0].parameters.tip_dynamics =
        Some(paint_io::brush_tip_dynamics::TipParameters::from_core(tip));
    input.package.brushes[0].parameters.scatter_dynamics =
        Some(paint_io::brush_scatter::ScatterParameters::from_core(
            paint_core::ScatterDynamics::new(true, 1., true, 3, 0.5, u64::MAX).unwrap(),
        ));
    input.package.brushes[0]
        .required_capabilities
        .push("scatter-stamps".into());
    input.package.brushes[0].parameters.shape_jitter =
        Some(paint_io::brush_shape_jitter::ShapeParameters::from_core(
            paint_core::ShapeJitter::new(true, 0.5, 0.5, 0.8, 0.3, 0.7, u64::MAX).unwrap(),
        ));
    input.package.brushes[0]
        .required_capabilities
        .push("shape-jitter".into());
    input.package.brushes[0].parameters.color_dynamics =
        Some(paint_io::brush_color_dynamics::ColorParameters::from_core(
            paint_core::ColorDynamics::new(true, 0.8, 0.2, 0.3, 0.1, true, u64::MAX).unwrap(),
        ));
    input.package.brushes[0]
        .required_capabilities
        .push("color-dynamics".into());
    let texture_id = input.package.brushes[0].tip_resource.as_ref().unwrap();
    let texture = paint_core::BrushTexture::new(
        true,
        paint_core::TextureSettings::new(
            paint_core::TextureSpace::Document,
            1.,
            0.,
            0.25,
            false,
            0.,
            0.,
        )
        .unwrap(),
        Some(Arc::new(paint_core::TextureResource::new(
            input.assets()[texture_id].mask(),
        ))),
    )
    .unwrap();
    input.package.brushes[0].texture =
        Some(paint_io::brush_texture::TextureParameters::from_core(&texture).unwrap());
    input.package.brushes[0]
        .required_capabilities
        .push("texture-cover".into());
    let mask = input.assets()[input.package.brushes[0].tip_resource.as_ref().unwrap()].mask();
    let dual = paint_core::DualBrush::new(
        true,
        paint_core::DualMode::Multiply,
        paint_core::SecondaryTip::new(
            4.,
            paint_core::BrushTip::default(),
            Some(mask),
            Default::default(),
        )
        .unwrap()
        .with_spacing(0.25)
        .unwrap(),
    );
    input.package.brushes[0].dual_brush =
        Some(paint_io::brush_dual::DualParameters::from_core(&dual).unwrap());
    input.package.brushes[0]
        .required_capabilities
        .push("dual-cover".into());
    input.package.brushes[0].behavior_version = 9;
    input.package.brushes[0]
        .required_capabilities
        .push("tip-dynamics".into());
    let packed = vbr_image::encode(&input, &token).unwrap();
    let output = vbr_image::decode(&packed, &token).unwrap();
    assert_eq!(
        output.sources()[0].original_bytes(),
        input.sources()[0].original_bytes()
    );
    assert_eq!(
        output.package.brushes[0].source_resource,
        input.package.brushes[0].source_resource
    );
    assert_eq!(
        output.package.brushes[0].tip_resource,
        input.package.brushes[0].tip_resource
    );
    assert_eq!(
        output.definition(0).unwrap().dynamics,
        input.definition(0).unwrap().dynamics
    );
    let mut docs = Vec::new();
    for bundle in [&input, &output] {
        let mut doc = Document::new(40, 40).unwrap();
        let mut point = InputPoint::new(20.5, 20.5, 1.);
        point.capabilities = 6;
        point.rotation = 90.;
        point.tilt_x = 60.;
        let mut definition = bundle.definition(0).unwrap();
        definition.brush.color = paint_core::Pixel::from_straight([1., 0., 0., 0.8]).unwrap();
        definition.brush.background_color =
            paint_core::Pixel::from_straight([0., 0., 1., 1.]).unwrap();
        doc.begin_brush_stroke(definition, point).unwrap();
        doc.end_stroke().unwrap();
        docs.push(doc);
    }
    assert!((0..40).any(|y| (0..40).any(|x| docs[0].pixel(x, y).alpha() > 0.)));
    for y in 0..40 {
        for x in 0..40 {
            assert_eq!(docs[0].pixel(x, y), docs[1].pixel(x, y));
        }
    }
    let mut baseline: paint_io::brush_source::SourceInfo =
        serde_json::from_slice(input.sources()[0].records().next().unwrap().1).unwrap();
    baseline.baseline.tip_dynamics =
        Some(paint_io::brush_tip_dynamics::TipParameters::from_core(tip));
    assert!(baseline.validate().is_err());
    baseline.baseline.tip_dynamics = None;
    baseline.baseline.scatter_dynamics = input.package.brushes[0].parameters.scatter_dynamics;
    assert!(baseline.validate().is_err());
    baseline.baseline.scatter_dynamics = None;
    baseline.baseline.shape_jitter = input.package.brushes[0].parameters.shape_jitter;
    assert!(baseline.validate().is_err());
    baseline.baseline.shape_jitter = None;
    baseline.baseline.color_dynamics = input.package.brushes[0].parameters.color_dynamics;
    assert!(baseline.validate().is_err());
}

fn rewrite(bytes: &[u8], mut edit: impl FnMut(&str, &mut Vec<u8>) -> bool) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).unwrap();
        if edit(entry.name(), &mut data) {
            zip.start_file(entry.name(), SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&data).unwrap();
        }
    }
    zip.finish().unwrap().into_inner()
}
#[test]
fn source_identity_original_bytes_shared_assets_and_changed_tip_survive_fresh_store_roundtrip() {
    let token = CancellationToken::default();
    let mut input = bundle();
    let baseline = input.package.brushes[0].tip_resource.clone().unwrap();
    let solid = encode_mask(&BrushMask::new(8, 8, vec![255; 64]).unwrap()).unwrap();
    let solid_id = hash(&solid);
    let mut assets = input
        .assets()
        .iter()
        .map(|(id, a)| (id.clone(), a.bytes().to_vec()))
        .collect::<BTreeMap<_, _>>();
    assets.insert(solid_id.clone(), solid);
    input.package.brushes[0].tip_resource = Some(solid_id);
    input.package.brushes[2].tip_resource = None;
    input.package.brushes[2].required_capabilities = vec!["procedural-tip".into()];
    let original = Arc::new(input.sources()[0].original_bytes().to_vec());
    let records = input.sources()[0]
        .records()
        .map(|(id, b)| (id.to_owned(), b.to_vec()))
        .collect();
    let sources =
        vec![paint_io::brush_source::SourceBatch::from_records(original, records, &token).unwrap()];
    let dynamic = paint_core::BrushDynamics {
        size: paint_core::PressureControl::new(
            paint_core::PressureCurve::new(vec![(0., 0.), (0.5, 0.25), (1., 1.)]).unwrap(),
            0.1,
        )
        .unwrap(),
        ..Default::default()
    };
    input.package.brushes[0].parameters.pressure_dynamics = Some(
        paint_io::brush_pressure::PressureParameters::from_core(&dynamic),
    );
    input.package.brushes[0].behavior_version = 2;
    input.package.brushes[0]
        .required_capabilities
        .push("pressure-curves".into());
    let input = ImageBundle::with_sources(input.package, assets, sources, &token).unwrap();
    let bytes = vbr_image::encode(&input, &token).unwrap();
    let output = vbr_image::decode(&bytes, &token).unwrap();
    assert_eq!(
        output.package.to_json().unwrap(),
        input.package.to_json().unwrap()
    );
    assert_eq!(output.sources().len(), 1);
    assert_eq!(output.assets().len(), 2);
    assert_eq!(
        output.sources()[0].original_bytes(),
        input.sources()[0].original_bytes()
    );
    assert_eq!(
        output.sources()[0].records().collect::<Vec<_>>(),
        input.sources()[0].records().collect::<Vec<_>>()
    );
    let temp = tempfile::tempdir().unwrap();
    let resources =
        ResourceStore::open(temp.path().join("assets"), MAX_CATALOG_MASK_BYTES).unwrap();
    for (id, a) in output.assets() {
        resources.put_verified(id, a.bytes(), &token).unwrap();
    }
    let store = resources.source_store().unwrap();
    store.save_batch(&output.sources()[0], &token).unwrap();
    let source = store
        .load(
            output.package.brushes[0].source_resource.as_ref().unwrap(),
            &token,
        )
        .unwrap();
    assert_eq!(source.info.tip_resource.as_deref(), Some(baseline.as_str()));
    assert_eq!(
        source
            .info
            .report
            .iter()
            .filter(|r| r.status == paint_io::brush_source::Support::NotExecuted)
            .count(),
        1
    );
    let mut current = Document::new(24, 24).unwrap();
    current
        .begin_brush_stroke(
            output.definition(0).unwrap(),
            InputPoint::new(12.5, 12.5, 1.),
        )
        .unwrap();
    current.end_stroke().unwrap();
    assert!(current.pixel(12, 12).alpha() > 0.9);
    let mut definition = source.info.baseline.definition();
    definition.mask = Some(resources.load(&baseline, &token).unwrap().mask);
    let mut initial = Document::new(24, 24).unwrap();
    initial
        .begin_brush_stroke(definition, InputPoint::new(12.5, 12.5, 1.))
        .unwrap();
    initial.end_stroke().unwrap();
    assert_eq!(initial.pixel(12, 12).alpha(), 0.);
    let mut zip = ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut manifest = String::new();
    zip.by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    assert!(manifest.contains("\"minor\":3"));
}
#[test]
fn bad_original_record_missing_baseline_and_downgrade_never_return_partial_bundle() {
    let token = CancellationToken::default();
    let input = bundle();
    let bytes = vbr_image::encode(&input, &token).unwrap();
    for mutation in 0..5 {
        let edited = rewrite(&bytes, |name, data| {
            if mutation == 0 && name.starts_with("originals/") {
                data[4] ^= 1;
            }
            if mutation == 1 && name.starts_with("sources/") {
                data.push(b' ');
            }
            if mutation == 2 && name.starts_with("assets/") {
                return false;
            }
            if mutation == 3 && name == "manifest.json" {
                *data = String::from_utf8(data.clone())
                    .unwrap()
                    .replace("\"minor\":2", "\"minor\":1")
                    .into_bytes();
            }
            if mutation == 4 && name.starts_with("originals/") {
                return false;
            }
            true
        });
        assert!(vbr_image::decode(&edited, &token).is_err());
    }
    assert!(paint_io::vbr::encode(&input.package, &token).is_err());
    assert!(paint_io::vbr::decode(&bytes, &token).is_err());
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(vbr_image::decode(&bytes, &cancelled).is_err());
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("unchanged.vbr");
    std::fs::write(&target, b"old").unwrap();
    assert!(vbr_image::save(&target, &input, &cancelled).is_err());
    assert_eq!(std::fs::read(target).unwrap(), b"old");
}
#[test]
fn extra_source_assets_and_mutated_references_are_rejected_and_pretty_record_bytes_stay_exact() {
    let token = CancellationToken::default();
    let mut input = bundle();
    input.package.brushes.pop();
    assert!(input.validate().is_err());
    let input = bundle();
    let source = &input.sources()[0];
    let (id, record) = source.records().next().unwrap();
    let info: paint_io::brush_source::SourceInfo = serde_json::from_slice(record).unwrap();
    let pretty = serde_json::to_vec_pretty(&info).unwrap();
    let pretty_id = hash(&pretty);
    let batch = paint_io::brush_source::SourceBatch::from_records(
        Arc::new(source.original_bytes().to_vec()),
        BTreeMap::from([(pretty_id.clone(), pretty.clone())]),
        &token,
    )
    .unwrap();
    assert_ne!(pretty_id, id);
    assert_eq!(batch.records().next().unwrap().1, pretty);
    let temp = tempfile::tempdir().unwrap();
    let store = SourceStore::open(temp.path().into()).unwrap();
    store.save_batch(&batch, &token).unwrap();
    let restored = store
        .export_batches(&BTreeSet::from([pretty_id]), &token)
        .unwrap();
    assert_eq!(restored[0].records().next().unwrap().1, pretty);
}

#[test]
fn all_1024_presets_can_have_distinct_current_and_baseline_tips_and_originals() {
    use paint_io::{
        abr_conversion::ConvertedPreset,
        brush_source::{SourceArchive, SourceBatch},
        ps_descriptor::{Class, Descriptor},
    };
    let token = CancellationToken::default();
    let descriptor = Descriptor {
        class: Class {
            name: String::new(),
            id: b"null".to_vec(),
        },
        entries: Vec::new(),
    };
    let mut assets = BTreeMap::new();
    let mut sources = Vec::new();
    let mut brushes = Vec::new();
    for index in 0u32..1024 {
        let make =
            |last| BrushMask::new(2, 2, vec![(index >> 8) as u8, index as u8, 255, last]).unwrap();
        let baseline = Arc::new(make(123));
        let current = encode_mask(&make(124)).unwrap();
        let current_id = hash(&current);
        let secondary_png = encode_mask(&make(126)).unwrap();
        let secondary_id = hash(&secondary_png);
        assets.insert(secondary_id.clone(), secondary_png);
        let dual = paint_core::DualBrush::new(
            false,
            paint_core::DualMode::Multiply,
            paint_core::SecondaryTip::new(
                4.,
                paint_core::BrushTip::default(),
                Some(Arc::new(make(126).with_resource_id(secondary_id).unwrap())),
                Default::default(),
            )
            .unwrap()
            .with_spacing(0.25)
            .unwrap(),
        );
        let texture_png = encode_mask(&make(125)).unwrap();
        let texture_id = hash(&texture_png);
        assets.insert(texture_id.clone(), texture_png);
        let texture_source = Arc::new(make(125).with_resource_id(texture_id).unwrap());
        let texture = paint_core::BrushTexture::new(
            false,
            paint_core::TextureSettings::default(),
            Some(Arc::new(paint_core::TextureResource::new(texture_source))),
        )
        .unwrap();
        assets.insert(current_id.clone(), current);
        let png = encode_mask(&baseline).unwrap();
        assets.insert(hash(&png), png);
        let settings = paint_core::BrushSettings {
            mask: Some(baseline),
            ..Default::default()
        };
        let converted = ConvertedPreset {
            name: "bounded",
            settings,
            source: &descriptor,
            issues: Vec::new(),
        };
        // Valid opaque source headers, not complete ABR parsing fixtures.
        let mut original = vec![0, 6, 0, 2];
        original.extend(index.to_be_bytes());
        let original = Arc::new(original);
        let archive = SourceArchive::capture(original.clone(), index, &converted).unwrap();
        let record = archive.record().unwrap();
        let source_id = hash(&record);
        sources.push(
            SourceBatch::from_records(
                original,
                BTreeMap::from([(source_id.clone(), record)]),
                &token,
            )
            .unwrap(),
        );
        brushes.push(Preset {
            dual_brush: Some(paint_io::brush_dual::DualParameters::from_core(&dual).unwrap()),
            texture: Some(paint_io::brush_texture::TextureParameters::from_core(&texture).unwrap()),
            id: format!("12345678-1234-4234-8234-{index:012x}"),
            name: "bounded".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 9,
            required_capabilities: vec![
                "image-tip".into(),
                "texture-cover".into(),
                "dual-cover".into(),
            ],
            tip_resource: Some(current_id),
            source_resource: Some(source_id),
            parameters: archive.info.baseline,
        });
    }
    let bundle = ImageBundle::with_sources(
        Package {
            package_id: "12345678-1234-4234-8234-123456789abc".into(),
            brushes,
        },
        assets,
        sources,
        &token,
    )
    .unwrap();
    let bytes = vbr_image::encode(&bundle, &token).unwrap();
    assert_eq!(ZipArchive::new(Cursor::new(&bytes)).unwrap().len(), 7169);
    let restored = vbr_image::decode(&bytes, &token).unwrap();
    assert_eq!(restored.package.brushes.len(), 1024);
    assert_eq!(restored.assets().len(), 4096);
    assert_eq!(restored.sources().len(), 1024);
}

#[test]
fn published_source_contract_covers_record_identity_versions_and_asset_roles() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-source-v1.2.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        2
    );
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["assets"]["maxItems"],
        4096
    );
    assert_eq!(
        schema["$defs"]["preset"]["properties"]["sourceResource"]["$ref"],
        "#/$defs/hash"
    );
    assert_eq!(
        schema["$defs"]["sourceRecord"]["properties"]["presetIndex"]["maximum"],
        1023
    );
    assert_eq!(
        schema["$defs"]["sourceRecord"]["properties"]["report"]["maxItems"],
        4096
    );
    assert_eq!(
        schema["$defs"]["sourceRecord"]["properties"]["baseline"]["$ref"],
        "#/$defs/parameters"
    );
    let token = CancellationToken::default();
    let bytes = vbr_image::encode(&bundle(), &token).unwrap();
    for version in [
        vbr_image::MAX_EXECUTABLE_MINOR + 1,
        vbr_image::MAX_EXECUTABLE_MINOR + 2,
    ] {
        let changed = rewrite(&bytes, |name, data| {
            if name == "manifest.json" {
                let mut json: serde_json::Value = serde_json::from_slice(data).unwrap();
                json["version"]["minor"] = version.into();
                *data = serde_json::to_vec(&json).unwrap();
            }
            true
        });
        assert!(vbr_image::decode(&changed, &token).is_err());
    }
}
