//! Opt-in diagnostic for externally held, non-redistributable sample files.
use paint_io::{
    abr_association::associate,
    abr_conversion::{convert, Status},
    abr_sampled::decode_sectioned,
    ps_descriptor::Value,
};
use paint_task::CancellationToken;
#[test]
#[ignore = "requires DRAWVERSE_ABR_DIAGNOSTIC_FILE; external assets are not vendored"]
fn external_sample_descriptor_and_pixels_are_inspected_without_claiming_conversion() {
    let path = std::env::var_os("DRAWVERSE_ABR_DIAGNOSTIC_FILE")
        .expect("explicit diagnostic fixture path");
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(
        paint_io::brush_resource::hash(&bytes),
        "e0d1ac1a947bb62d4b4eb31fda08e2be10b2a35434e722434226c8a8b158abeb"
    );
    let token = CancellationToken::default();
    let parsed = decode_sectioned(&bytes, &token).unwrap();
    println!(
        "ABR {}.{}: {} samples, {} source sections",
        parsed.version,
        parsed.subversion,
        parsed.tips.len(),
        parsed.metadata.len()
    );
    for tip in &parsed.tips {
        println!(
            "sample {:?}, prefix {:?}",
            tip.mask.dimensions(),
            String::from_utf8_lossy(&tip.prefix[..tip.prefix.len().min(37)])
        );
    }
    let descriptors = parsed.descriptors(&token).unwrap();
    let linked = associate(&parsed, &descriptors, &token).unwrap();
    assert_eq!(linked.len(), 1);
    assert_eq!(linked[0].name, "Bobbys Brush 743");
    assert_eq!(
        linked[0].sampled_tip.unwrap().id,
        "a8f34db0-4814-634e-807c-c61aecde18fd"
    );
    assert_eq!(linked[0].sampled_tip.unwrap().mask.dimensions(), (112, 105));
    let reference = std::fs::read(
        std::env::var_os("DRAWVERSE_ABR_DIAGNOSTIC_REFERENCE")
            .expect("explicit reference PNG path"),
    )
    .unwrap();
    assert_eq!(
        paint_io::brush_resource::hash(&reference),
        "b44e17959fa648d549d6c110f9e4816508f9943fdf938106abae9b66a2a09a23"
    );
    let image = image::load_from_memory(&reference).unwrap().to_rgba8();
    assert_eq!(image.dimensions(), (112, 105));
    let alpha = image.pixels().map(|p| p[3]).collect::<Vec<_>>();
    assert_eq!(linked[0].sampled_tip.unwrap().mask.coverage(), alpha);
    let converted = convert(&linked, &token).unwrap();
    let brush = &converted[0];
    assert!(brush.is_partial());
    assert_eq!(brush.settings.diameter, 112.);
    assert_eq!(brush.settings.spacing, 0.02);
    assert_eq!(brush.settings.tip.roundness, 1.);
    for path in ["szVr.bVTy", "useTexture", "textureDepth"] {
        assert!(
            brush
                .issues
                .iter()
                .any(|i| i.path == path && i.status == Status::NotExecuted),
            "{path}"
        );
    }
    let mut document = paint_core::Document::new(224, 224).unwrap();
    document
        .begin_brush_stroke(
            brush.settings.definition(),
            paint_core::InputPoint::new(112., 112.5, 1.),
        )
        .unwrap();
    document.end_stroke().unwrap();
    for y in 0..105 {
        for x in 0..112 {
            let expected = f32::from(alpha[(y * 112 + x) as usize]) / 255.;
            assert!(
                (document.pixel(56 + x, 60 + y).alpha() - expected).abs() < 0.000003,
                "baseline pixel {x},{y}"
            );
        }
    }
    println!("basic diameter/spacing/roundness and native unmodified-alpha baseline verified; {} explicit report items",brush.issues.len());
    let archive = paint_io::brush_source::SourceArchive::capture(
        std::sync::Arc::new(bytes.clone()),
        0,
        brush,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let store = paint_io::brush_source::SourceStore::open(directory.path().into()).unwrap();
    let source_id = store.save(&archive, &token).unwrap();
    let restored = store.load(&source_id, &token).unwrap();
    assert_eq!(restored.original(), bytes);
    assert_eq!(restored.info.original_name, brush.name);
    assert_eq!(restored.info.report.len(), brush.issues.len());
    let batch = paint_io::abr_import::ImportBatch::decode(
        std::sync::Arc::new(bytes.clone()),
        paint_core::MAX_CATALOG_MASK_BYTES,
        &token,
    )
    .unwrap();
    assert_eq!(batch.asset_count(), 1);
    let resources = paint_io::brush_resource::ResourceStore::open(
        directory.path().join("imported"),
        paint_core::MAX_CATALOG_MASK_BYTES,
    )
    .unwrap();
    let imported = batch.persist(&resources, &token).unwrap();
    assert_eq!(imported.len(), 1);
    assert!(imported[0].is_partial());
    assert_eq!(imported[0].name(), brush.name);
    let captured = resources
        .source_store()
        .unwrap()
        .load(imported[0].source_id(), &token)
        .unwrap();
    assert_eq!(captured.original(), bytes);
    assert_eq!(captured.info.report.len(), brush.issues.len());
    for root in &descriptors {
        println!(
            "root {:?}, {} properties",
            String::from_utf8_lossy(&root.class.id),
            root.entries.len()
        );
        if let Some(Value::List(presets)) = root.get(b"Brsh") {
            println!("{} declared presets", presets.len());
            for preset in presets {
                if let Value::Object { descriptor, .. } = preset {
                    println!("preset name {:?}", descriptor.get(b"Nm  "));
                    if let Some(Value::Object { descriptor, .. }) = descriptor.get(b"Brsh") {
                        println!(
                            "shape {:?}, fields {:?}",
                            String::from_utf8_lossy(&descriptor.class.id),
                            descriptor
                                .entries
                                .iter()
                                .map(|(k, _)| String::from_utf8_lossy(k))
                                .collect::<Vec<_>>()
                        );
                    }
                }
            }
        }
    }
    assert!(!parsed.tips.is_empty() && !descriptors.is_empty());
}
