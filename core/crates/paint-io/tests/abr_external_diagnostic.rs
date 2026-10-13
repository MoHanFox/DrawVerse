//! Opt-in diagnostic for externally held, non-redistributable sample files.
use paint_io::{abr_association::associate, abr_sampled::decode_sectioned, ps_descriptor::Value};
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
