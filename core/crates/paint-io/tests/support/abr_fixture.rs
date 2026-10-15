fn text(value: &str) -> Vec<u8> {
    let units = value.encode_utf16().collect::<Vec<_>>();
    let mut output = (units.len() as u32).to_be_bytes().to_vec();
    for unit in units {
        output.extend(unit.to_be_bytes());
    }
    output
}
fn id(value: &str) -> Vec<u8> {
    let mut output = (if value.len() == 4 {
        0
    } else {
        value.len() as u32
    })
    .to_be_bytes()
    .to_vec();
    output.extend(value.as_bytes());
    output
}
fn typed(tag: &[u8; 4], value: &[u8]) -> Vec<u8> {
    let mut bytes = tag.to_vec();
    bytes.extend(value);
    bytes
}
fn object(class: &str, entries: Vec<(&str, Vec<u8>)>) -> Vec<u8> {
    let mut bytes = text("");
    bytes.extend(id(class));
    bytes.extend((entries.len() as u32).to_be_bytes());
    for (key, value) in entries {
        bytes.extend(id(key));
        bytes.extend(value);
    }
    bytes
}
fn unit(kind: &[u8; 4], value: f64) -> Vec<u8> {
    let mut bytes = kind.to_vec();
    bytes.extend(value.to_be_bytes());
    typed(b"UntF", &bytes)
}
fn preset(sampled: bool, bad: bool, opacity: Option<f64>) -> Vec<u8> {
    let mut shape = vec![
        ("Dmtr", unit(b"#Pxl", if bad { 513. } else { 8. })),
        ("Angl", unit(b"#Ang", 0.)),
        ("Rndn", unit(b"#Prc", 100.)),
        ("Spcn", unit(b"#Prc", 2.)),
    ];
    if sampled {
        shape.push(("sampledData", typed(b"TEXT", &text("shared\0"))));
    } else {
        shape.push(("Hrdn", unit(b"#Prc", 100.)));
    }
    let mut entries = vec![
        ("Nm  ", typed(b"TEXT", &text("Same\0"))),
        (
            "Brsh",
            typed(
                b"Objc",
                &object(
                    if sampled {
                        "sampledBrush"
                    } else {
                        "computedBrush"
                    },
                    shape,
                ),
            ),
        ),
        ("unknownTexture", typed(b"bool", &[1])),
    ];
    if let Some(opacity) = opacity {
        entries.push((
            "toolOptions",
            typed(
                b"Objc",
                &object(
                    "paintbrushTool",
                    vec![("Opct", typed(b"doub", &opacity.to_be_bytes()))],
                ),
            ),
        ));
    }
    typed(b"Objc", &object("brushPreset", entries))
}

fn section(out: &mut Vec<u8>, key: &[u8; 4], bytes: &[u8]) {
    out.extend(b"8BIM");
    out.extend(key);
    out.extend((bytes.len() as u32).to_be_bytes());
    out.extend(bytes);
}
pub fn fixture(version: u16, sub: u16, bad: bool) -> Vec<u8> {
    fixture_with_opacity(version, sub, bad, None)
}
pub fn fixture_with_opacity(version: u16, sub: u16, bad: bool, opacity: Option<f64>) -> Vec<u8> {
    let mut bytes = version.to_be_bytes().to_vec();
    bytes.extend(sub.to_be_bytes());
    let mut list = 3u32.to_be_bytes().to_vec();
    list.extend(preset(true, false, opacity));
    list.extend(preset(false, false, opacity));
    list.extend(preset(true, bad, opacity));
    let mut descriptor = 16u32.to_be_bytes().to_vec();
    descriptor.extend(object("null", vec![("Brsh", typed(b"VlLs", &list))]));
    section(&mut bytes, b"desc", &descriptor);
    let mut record = vec![6];
    record.extend(b"shared");
    record.extend(vec![0; if sub == 1 { 10 } else { 264 }]);
    for bound in [0i32, 0, 8, 8] {
        record.extend(bound.to_be_bytes());
    }
    record.extend(8u16.to_be_bytes());
    record.push(0);
    let mut coverage = vec![255; 64];
    for y in 3..5 {
        for x in 3..5 {
            coverage[y * 8 + x] = 0;
        }
    }
    record.extend(coverage);
    let mut samples = (record.len() as u32).to_be_bytes().to_vec();
    samples.extend(&record);
    samples.resize(samples.len() + (4 - record.len() % 4) % 4, 0);
    section(&mut bytes, b"samp", &samples);
    bytes
}
