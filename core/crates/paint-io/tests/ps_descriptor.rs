//! Independently generated descriptor wire fixtures, not Photoshop-produced ABR files.
use paint_io::ps_descriptor::{decode, Reference, Value};
use paint_task::CancellationToken;
fn text(value: &str) -> Vec<u8> {
    let units = value.encode_utf16().collect::<Vec<_>>();
    let mut bytes = (units.len() as u32).to_be_bytes().to_vec();
    for unit in units {
        bytes.extend(unit.to_be_bytes());
    }
    bytes
}
fn id(value: &[u8]) -> Vec<u8> {
    let mut bytes = (if value.len() == 4 {
        0
    } else {
        value.len() as u32
    })
    .to_be_bytes()
    .to_vec();
    bytes.extend(value);
    bytes
}
fn class() -> Vec<u8> {
    let mut bytes = text("笔刷");
    bytes.extend(id(b"Brsh"));
    bytes
}
fn descriptor(entries: &[(&[u8], Vec<u8>)]) -> Vec<u8> {
    let mut bytes = class();
    bytes.extend((entries.len() as u32).to_be_bytes());
    for (key, value) in entries {
        bytes.extend(id(key));
        bytes.extend(value);
    }
    bytes
}
fn root(entries: &[(&[u8], Vec<u8>)]) -> Vec<u8> {
    let mut bytes = 16u32.to_be_bytes().to_vec();
    bytes.extend(descriptor(entries));
    bytes
}
fn typed(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut bytes = tag.to_vec();
    bytes.extend(data);
    bytes
}
#[test]
fn typed_values_nested_objects_units_and_references_keep_source_identity() {
    let mut unit = b"#Pxl".to_vec();
    unit.extend(12.5f64.to_be_bytes());
    let mut float = b"#Prc".to_vec();
    float.extend(25.5f32.to_be_bytes());
    let mut enumeration = id(b"brushType");
    enumeration.extend(id(b"sampled"));
    let mut refs = 7u32.to_be_bytes().to_vec();
    for tag in [
        b"prop", b"Clss", b"Enmr", b"rele", b"Idnt", b"indx", b"name",
    ] {
        refs.extend(tag);
        if !matches!(tag, b"Idnt" | b"indx") {
            refs.extend(class());
        }
        match tag {
            b"prop" => refs.extend(id(b"Nm  ")),
            b"Enmr" => refs.extend(&enumeration),
            b"rele" | b"Idnt" | b"indx" => refs.extend((-7i32).to_be_bytes()),
            b"name" => refs.extend(text("来源")),
            _ => {}
        }
    }
    let mut list = 2u32.to_be_bytes().to_vec();
    list.extend(typed(b"bool", &[1]));
    list.extend(typed(b"TEXT", &text("孔洞")));
    let mut blob = 3u32.to_be_bytes().to_vec();
    blob.extend([0, 128, 255]);
    let entries: Vec<(&[u8], Vec<u8>)> = vec![
        (b"doub", typed(b"doub", &1.25f64.to_be_bytes())),
        (b"unit", typed(b"UntF", &unit)),
        (b"fl32", typed(b"UnFl", &float)),
        (b"text", typed(b"TEXT", &text("名称"))),
        (b"enum", typed(b"enum", &enumeration)),
        (b"long", typed(b"long", &(-42i32).to_be_bytes())),
        (b"comp", typed(b"comp", &i64::MIN.to_be_bytes())),
        (b"bool", typed(b"bool", &[0])),
        (b"list", typed(b"VlLs", &list)),
        (b"obj1", typed(b"Objc", &descriptor(&[]))),
        (b"obj2", typed(b"GlbO", &descriptor(&[]))),
        (b"cls1", typed(b"type", &class())),
        (b"cls2", typed(b"GlbC", &class())),
        (b"alia", typed(b"alis", &blob)),
        (b"raw ", typed(b"tdta", &blob)),
        (b"path", typed(b"Pth ", &blob)),
        (b"refs", typed(b"obj ", &refs)),
        (&[0, 255, 0, 128], typed(b"long", &3i32.to_be_bytes())),
    ];
    let parsed = decode(&root(&entries), &CancellationToken::default()).unwrap();
    assert_eq!(parsed.class.name, "笔刷");
    assert_eq!(parsed.class.id, b"Brsh");
    assert_eq!(parsed.entries.len(), 18);
    assert_eq!(
        parsed.get(b"unit"),
        Some(&Value::UnitDouble {
            unit: *b"#Pxl",
            value: 12.5
        })
    );
    assert_eq!(
        parsed.get(b"fl32"),
        Some(&Value::UnitFloat {
            unit: *b"#Prc",
            value: 25.5
        })
    );
    assert_eq!(parsed.get(b"comp"), Some(&Value::LargeInteger(i64::MIN)));
    assert!(matches!(
        parsed.get(b"obj2"),
        Some(Value::Object { global: true, .. })
    ));
    assert!(matches!(
        parsed.get(b"cls2"),
        Some(Value::Class { global: true, .. })
    ));
    assert_eq!(
        parsed.get(b"raw "),
        Some(&Value::Raw(vec![0, 128, 255].into()))
    );
    let Some(Value::Reference(references)) = parsed.get(b"refs") else {
        panic!("typed references")
    };
    assert_eq!(references.len(), 7);
    assert!(matches!(&references[0],Reference::Property{key,..}if key==b"Nm  "));
    assert!(matches!(references[4], Reference::Identifier(-7)));
    assert_eq!(parsed.get(&[0, 255, 0, 128]), Some(&Value::Integer(3)));
}
#[test]
fn truncation_invalid_types_values_duplicate_keys_and_tail_reject_atomically() {
    let token = CancellationToken::default();
    let bytes = root(&[(b"list", typed(b"VlLs", &[0, 0, 0, 0]))]);
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end], &token).is_err(), "truncation {end}");
    }
    for value in [
        typed(b"bool", &[2]),
        typed(b"doub", &f64::NAN.to_be_bytes()),
        typed(b"UnFl", &[b'#', b'P', b'x', b'l', 127, 128, 0, 0]),
        typed(b"TEXT", &[0, 0, 0, 1, 216, 0]),
        typed(b"zzzz", &[]),
    ] {
        assert!(decode(&root(&[(b"bad ", value)]), &token).is_err());
    }
    assert!(decode(
        &root(&[
            (b"same", typed(b"bool", &[0])),
            (b"same", typed(b"bool", &[1]))
        ]),
        &token
    )
    .is_err());
    let mut tail = bytes.clone();
    tail.push(0);
    assert!(decode(&tail, &token).is_err());
    let mut version = bytes.clone();
    version[3] = 17;
    assert!(decode(&version, &token).is_err());
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(decode(&bytes, &cancelled).is_err());
}
#[test]
fn recursive_and_aggregate_count_limits_stop_before_unbounded_allocation() {
    let token = CancellationToken::default();
    let mut nested = typed(b"bool", &[1]);
    for _ in 0..34 {
        let mut list = 1u32.to_be_bytes().to_vec();
        list.extend(nested);
        nested = typed(b"VlLs", &list);
    }
    assert!(decode(&root(&[(b"nest", nested)]), &token).is_err());
    assert!(decode(
        &root(&[(b"list", typed(b"VlLs", &4097u32.to_be_bytes()))]),
        &token
    )
    .is_err());
    let mut list = 16u32.to_be_bytes().to_vec();
    for _ in 0..16 {
        let mut values = 4096u32.to_be_bytes().to_vec();
        for _ in 0..4096 {
            values.extend(typed(b"bool", &[1]));
        }
        list.extend(typed(b"VlLs", &values));
    }
    assert!(decode(&root(&[(b"many", typed(b"VlLs", &list))]), &token).is_err());
    let mut oversized = 16u32.to_be_bytes().to_vec();
    oversized.extend(8193u32.to_be_bytes());
    assert!(decode(&oversized, &token).is_err());
}
