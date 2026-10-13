use paint_io::vbr::*;
use paint_task::CancellationToken;
use std::io::{Cursor, Write};
use zip::{write::SimpleFileOptions, ZipWriter};
fn package() -> Package {
    Package {
        package_id: "12345678-1234-1234-1234-123456789abc".into(),
        brushes: vec![Preset {
            tip_resource: None,
            source_resource: None,
            id: "12345678-1234-1234-1234-123456789abd".into(),
            name: "椭圆软笔".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 1,
            required_capabilities: vec!["procedural-tip".into()],
            parameters: Parameters {
                diameter: 40.,
                spacing: 0.01,
                dab_opacity: 0.8,
                hardness: 0.3,
                roundness: 0.2,
                angle: 90.,
                flow: 0.4,
                pressure_size: false,
                pressure_flow: true,
            },
        }],
    }
}
#[test]
fn roundtrip_is_executable_preserves_identity_and_units() {
    let p = package();
    let token = CancellationToken::default();
    let bytes = encode(&p, &token).unwrap();
    let read = decode(&bytes, &token).unwrap();
    assert_eq!(read.to_json().unwrap(), p.to_json().unwrap());
    let definition = read.brushes[0].parameters.definition();
    assert_eq!(definition.brush.spacing, 0.02);
    assert_eq!(definition.brush.radius, 20.);
    let mut d = paint_core::Document::new(60, 60).unwrap();
    d.begin_brush_stroke(definition, paint_core::InputPoint::new(30.5, 30.5, 1.))
        .unwrap();
    d.end_stroke().unwrap();
    assert!(d.pixel(30, 40).alpha() > 0.);
    assert_eq!(d.pixel(40, 30).alpha(), 0.);
}
#[test]
fn invalid_definitions_and_cancelled_save_preserve_existing_file() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("test.vbr");
    let p = package();
    save(&path, &p, &CancellationToken::default()).unwrap();
    let original = std::fs::read(&path).unwrap();
    let mut bad = p.clone();
    bad.brushes[0].parameters.roundness = 0.;
    assert!(save(&path, &bad, &CancellationToken::default()).is_err());
    bad = p.clone();
    bad.brushes[0].required_capabilities.push("wet-mix".into());
    assert!(bad.validate().is_err());
    bad = p.clone();
    bad.brushes[0].engine_profile = "legacy.round".into();
    assert!(bad.validate().is_err());
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(save(&path, &p, &cancelled).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(Package::from_json(br#"{"packageId":"bad","brushes":[]}"#).is_err());
}
fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
#[test]
fn archive_alias_traversal_missing_unknown_entries_and_versions_reject() {
    let token = CancellationToken::default();
    for names in [
        ["../bad.json", "ok.json"],
        ["/bad.json", "ok.json"],
        ["C:/bad", "ok.json"],
        ["bad\\x", "ok.json"],
        ["A.json", "a.json"],
    ] {
        assert!(decode(&archive(&[(names[0], b"{}"), (names[1], b"{}")]), &token).is_err());
    }
    let p = package();
    let manifest = format!(
        r#"{{"format":"DrawVerseBrush","version":{{"major":1,"minor":0}},"packageId":"{}","brushes":[{{"id":"{}","path":"brushes/{}.json"}}],"groups":[],"assets":[]}}"#,
        p.package_id, p.brushes[0].id, p.brushes[0].id
    );
    assert!(decode(&archive(&[("manifest.json", manifest.as_bytes())]), &token).is_err());
    let oversized = vec![b' '; MAX_JSON as usize + 1];
    assert!(decode(&archive(&[("manifest.json", &oversized)]), &token).is_err());
    for version in [
        manifest.replace("\"major\":1", "\"major\":9"),
        manifest.replace("\"assets\":[]", "\"assets\":[{}]"),
    ] {
        assert!(decode(&archive(&[("manifest.json", version.as_bytes())]), &token).is_err());
    }
}

#[test]
fn published_schema_numeric_limits_match_executable_parameters() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/vbr-procedural-v1.schema.json"
    ))
    .unwrap();
    let properties = &schema["$defs"]["parameters"]["properties"];
    for field in [
        "diameter",
        "spacing",
        "dabOpacity",
        "hardness",
        "roundness",
        "angle",
        "flow",
    ] {
        let spec = if properties[field].get("$ref").is_some() {
            &schema["$defs"]["ratio"]
        } else {
            &properties[field]
        };
        let minimum = spec["minimum"].as_f64().unwrap();
        let maximum = spec["maximum"].as_f64().unwrap();
        for (value, valid) in [
            (minimum, true),
            (maximum, true),
            (minimum - 0.001, false),
            (maximum + 0.001, false),
        ] {
            let mut json = serde_json::to_value(package()).unwrap();
            json["brushes"][0]["parameters"][field] = serde_json::json!(value);
            assert_eq!(
                Package::from_json(&serde_json::to_vec(&json).unwrap()).is_ok(),
                valid,
                "{field}={value}"
            );
        }
    }
}
