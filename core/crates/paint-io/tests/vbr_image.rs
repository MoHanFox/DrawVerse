use paint_core::{BrushMask, Document, InputPoint};
use paint_io::{
    brush_resource::{encode_mask, hash},
    vbr::{Package, Parameters, Preset},
    vbr_image::*,
};
use paint_task::CancellationToken;
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
    sync::Arc,
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};
fn package(resource: Option<String>) -> Package {
    let caps = if resource.is_some() {
        vec!["image-tip".into()]
    } else {
        vec!["procedural-tip".into()]
    };
    Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![Preset {
            texture: None,
            dual_brush: None,
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "图片孔洞".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 1,
            required_capabilities: caps,
            tip_resource: resource,
            source_resource: None,
            parameters: Parameters {
                scatter_dynamics: None,
                shape_jitter: None,
                color_dynamics: None,
                tip_dynamics: None,
                pressure_dynamics: None,
                diameter: 16.,
                spacing: 0.01,
                dab_opacity: 1.,
                hardness: 1.,
                roundness: 1.,
                angle: 0.,
                flow: 1.,
                pressure_size: false,
                pressure_flow: false,
            },
        }],
    }
}
fn bundle() -> ImageBundle {
    let mut pixels = vec![255; 16 * 16];
    for y in 6..10 {
        for x in 6..10 {
            pixels[y * 16 + x] = 0;
        }
    }
    let png = encode_mask(&BrushMask::new(16, 16, pixels).unwrap()).unwrap();
    let id = hash(&png);
    ImageBundle::new(
        package(Some(id.clone())),
        BTreeMap::from([(id, png)]),
        &CancellationToken::default(),
    )
    .unwrap()
}
fn rewrite(bytes: &[u8], mut edit: impl FnMut(&str, &mut Vec<u8>)) -> Vec<u8> {
    let mut source = ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut target = ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..source.len() {
        let mut file = source.by_index(i).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        edit(file.name(), &mut data);
        target
            .start_file(file.name(), SimpleFileOptions::default())
            .unwrap();
        target.write_all(&data).unwrap();
    }
    target.finish().unwrap().into_inner()
}
#[test]
fn portable_resources_roundtrip_exact_bytes_and_execute_shared_holes() {
    let mut original = bundle();
    let mut second = original.package.brushes[0].clone();
    second.id = "12345678-1234-4234-8234-123456789abe".into();
    original.package.brushes.push(second);
    let token = CancellationToken::default();
    let bytes = encode(&original, &token).unwrap();
    let restored = decode(&bytes, &token).unwrap();
    assert_eq!(
        restored.package.to_json().unwrap(),
        original.package.to_json().unwrap()
    );
    assert_eq!(restored.assets().len(), 1);
    let id = original.package.brushes[0].tip_resource.as_ref().unwrap();
    assert_eq!(restored.assets()[id].bytes(), original.assets()[id].bytes());
    assert!(Arc::ptr_eq(
        restored.definition(0).unwrap().mask.as_ref().unwrap(),
        restored.definition(1).unwrap().mask.as_ref().unwrap()
    ));
    let mut d = Document::new(40, 40).unwrap();
    d.begin_brush_stroke(
        restored.definition(0).unwrap(),
        InputPoint::new(20.5, 20.5, 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    assert_eq!(d.pixel(20, 20).alpha(), 0.);
    assert!(d.pixel(25, 20).alpha() > 0.9);
    assert!(paint_io::vbr::decode(&bytes, &token).is_err());
    assert!(paint_io::vbr::encode(&original.package, &token).is_err());
    let procedural = ImageBundle::new(package(None), BTreeMap::new(), &token).unwrap();
    let bytes = encode(&procedural, &token).unwrap();
    assert!(paint_io::vbr::decode(&bytes, &token).is_ok());
    assert!(decode(&bytes, &token).unwrap().assets().is_empty());
}
#[test]
fn invalid_manifest_resources_references_and_versions_never_publish_partial_bundle() {
    let token = CancellationToken::default();
    let bytes = encode(&bundle(), &token).unwrap();
    for mode in 0..8 {
        let changed = rewrite(&bytes, |name, data| {
            if name == "manifest.json" {
                let mut json: serde_json::Value = serde_json::from_slice(data).unwrap();
                match mode {
                    0 => {
                        json["version"]["minor"] =
                            (paint_io::vbr_image::MAX_EXECUTABLE_MINOR + 1).into()
                    }
                    1 => json["assets"][0]["kind"] = "grain".into(),
                    2 => json["assets"][0]["sha256"] = "0".repeat(64).into(),
                    3 => json["assets"][0]["path"] = "assets/../bad.png".into(),
                    4 => json["assets"] = serde_json::json!([]),
                    _ => {}
                }
                *data = serde_json::to_vec(&json).unwrap();
            } else if name.starts_with("assets/") && mode == 5 {
                data[30] ^= 1;
            } else if name.starts_with("brushes/") && mode >= 6 {
                let mut json: serde_json::Value = serde_json::from_slice(data).unwrap();
                if mode == 6 {
                    json["tipResource"] = "0".repeat(64).into();
                } else {
                    json["requiredCapabilities"] = serde_json::json!(["procedural-tip"]);
                }
                *data = serde_json::to_vec(&json).unwrap();
            }
        });
        assert!(decode(&changed, &token).is_err(), "mutation {mode}");
    }
    let mut original = bundle();
    original.package.brushes[0].tip_resource = None;
    assert!(original.validate().is_err());
}
#[test]
fn cancelled_or_invalid_image_save_preserves_existing_package() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("image.vbr");
    let mut bundle = bundle();
    let token = CancellationToken::default();
    save(&path, &bundle, &token).unwrap();
    let saved = std::fs::read(&path).unwrap();
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(save(&path, &bundle, &cancelled).is_err());
    bundle.package.brushes[0].parameters.roundness = 0.;
    assert!(save(&path, &bundle, &token).is_err());
    assert_eq!(std::fs::read(path).unwrap(), saved);
}
#[test]
fn sixteen_bit_resource_keeps_original_identity_and_published_image_contract() {
    use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
    let pixels = [65535u16, 32768, 0, 65535]
        .into_iter()
        .flat_map(u16::to_ne_bytes)
        .collect::<Vec<_>>();
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&pixels, 4, 1, ExtendedColorType::L16)
        .unwrap();
    let id = hash(&png);
    let token = CancellationToken::default();
    let bundle = ImageBundle::new(
        package(Some(id.clone())),
        BTreeMap::from([(id.clone(), png.clone())]),
        &token,
    )
    .unwrap();
    let restored = decode(&encode(&bundle, &token).unwrap(), &token).unwrap();
    assert_eq!(restored.assets()[&id].bytes(), png);
    assert_eq!(restored.assets()[&id].mask().coverage(), [255, 128, 0, 255]);
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-image-v1.1.schema.json"
    ))
    .unwrap();
    assert_eq!(
        schema["$defs"]["manifest"]["properties"]["version"]["properties"]["minor"]["const"],
        1
    );
    assert_eq!(
        schema["$defs"]["preset"]["properties"]["tipResource"]["$ref"],
        "#/$defs/hash"
    );
    assert!(
        schema["$defs"]["preset"]["properties"]["requiredCapabilities"]["items"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "image-tip")
    );
}
