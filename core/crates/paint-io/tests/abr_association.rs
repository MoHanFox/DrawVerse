use paint_io::{
    abr_association::associate,
    abr_sampled::{SectionedAbr, SectionedTip},
    ps_descriptor::{Class, Descriptor, Value},
};
use paint_task::CancellationToken;
use std::sync::Arc;
fn descriptor(id: &str, entries: Vec<(&str, Value)>) -> Descriptor {
    Descriptor {
        class: Class {
            name: String::new(),
            id: id.as_bytes().to_vec(),
        },
        entries: entries
            .into_iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v))
            .collect(),
    }
}
fn object(descriptor: Descriptor) -> Value {
    Value::Object {
        global: false,
        descriptor: Box::new(descriptor),
    }
}
fn preset(class: &str, sample: Option<&str>) -> Value {
    let mut shape = vec![(
        "Dmtr",
        Value::UnitDouble {
            unit: *b"#Pxl",
            value: 12.,
        },
    )];
    if let Some(id) = sample {
        shape.push(("sampledData", Value::Text(id.into())));
    }
    object(descriptor(
        "brushPreset",
        vec![
            ("Nm  ", Value::Text("同名\0".into())),
            ("Brsh", object(descriptor(class, shape))),
            ("unmappedSetting", Value::Boolean(true)),
        ],
    ))
}
fn source() -> SectionedAbr {
    SectionedAbr {
        version: 6,
        subversion: 1,
        tips: vec![SectionedTip {
            id: "sample-identity".into(),
            prefix: vec![].into(),
            bounds: [0, 0, 1, 1],
            mask: Arc::new(paint_core::BrushMask::new(1, 1, vec![255]).unwrap()),
            trailing_metadata: vec![].into(),
        }],
        metadata: Vec::new(),
        patterns: Default::default(),
        texture_presets: Default::default(),
    }
}
#[test]
fn repeated_names_link_shared_samples_by_identity_and_keep_unmapped_properties() {
    let source = source();
    let roots = vec![descriptor(
        "null",
        vec![(
            "Brsh",
            Value::List(vec![
                preset("sampledBrush", Some("sample-identity\0")),
                preset("computedBrush", None),
                preset("sampledBrush", Some("sample-identity")),
            ]),
        )],
    )];
    let linked = associate(&source, &roots, &CancellationToken::default()).unwrap();
    assert_eq!(linked.len(), 3);
    assert!(linked.iter().all(|p| p.name == "同名"));
    assert!(linked[1].sampled_tip.is_none());
    assert!(std::ptr::eq(
        linked[0].sampled_tip.unwrap(),
        linked[2].sampled_tip.unwrap()
    ));
    assert_eq!(
        linked[0].preset.get(b"unmappedSetting"),
        Some(&Value::Boolean(true))
    );
}
#[test]
fn missing_reference_unknown_shape_bad_name_and_duplicate_identity_reject_whole_batch() {
    let mut source = source();
    let token = CancellationToken::default();
    for shape in [
        preset("sampledBrush", Some("missing")),
        preset("sampledBrush", None),
        preset("dBrush", None),
    ] {
        let roots = vec![descriptor(
            "null",
            vec![(
                "Brsh",
                Value::List(vec![preset("computedBrush", None), shape]),
            )],
        )];
        assert!(associate(&source, &roots, &token).is_err());
    }
    let mut bad = preset("computedBrush", None);
    if let Value::Object { descriptor, .. } = &mut bad {
        descriptor.entries[0].1 = Value::Text("bad\0name".into());
    }
    let roots = vec![descriptor("null", vec![("Brsh", Value::List(vec![bad]))])];
    assert!(associate(&source, &roots, &token).is_err());
    let tip = &source.tips[0];
    source.tips.push(SectionedTip {
        id: tip.id.clone(),
        prefix: vec![].into(),
        bounds: tip.bounds,
        mask: tip.mask.clone(),
        trailing_metadata: vec![].into(),
    });
    assert!(associate(&source, &roots, &token).is_err());
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(associate(&source, &roots, &cancelled).is_err());
}
