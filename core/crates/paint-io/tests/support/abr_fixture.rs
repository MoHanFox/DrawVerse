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
#[allow(clippy::too_many_arguments)]
fn preset(
    sampled: bool,
    bad: bool,
    opacity: Option<f64>,
    pressure: bool,
    scatter: Option<bool>,
    color: Option<(bool, bool)>,
    (shape_jitter, device): (Option<bool>, Option<(bool, i32, bool)>),
    texture: Option<(&str, &str, f64, f64, f64, f64, bool)>,
) -> Vec<u8> {
    let mut shape = vec![
        ("Dmtr", unit(b"#Pxl", if bad { 513. } else { 8. })),
        ("Angl", unit(b"#Ang", 0.)),
        (
            "Rndn",
            unit(b"#Prc", if device.is_some() { 50. } else { 100. }),
        ),
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
    if let Some(enabled) = scatter {
        entries.push(("useScatter", typed(b"bool", &[u8::from(enabled)])));
        entries.push(("bothAxes", typed(b"bool", &[1])));
        entries.push(("Cnt ", typed(b"doub", &3f64.to_be_bytes())));
        for (key, jitter) in [("scatterDynamics", 75.), ("countDynamics", 50.)] {
            entries.push((
                key,
                typed(
                    b"Objc",
                    &object(
                        "brushDynamics",
                        vec![
                            ("bVTy", typed(b"long", &0i32.to_be_bytes())),
                            ("fStp", typed(b"long", &25i32.to_be_bytes())),
                            ("jitter", unit(b"#Prc", jitter)),
                            ("Mnm ", unit(b"#Prc", 0.)),
                        ],
                    ),
                ),
            ));
        }
    }
    if let Some((enabled, per_tip)) = color {
        entries.extend([
            ("useColorDynamics", typed(b"bool", &[u8::from(enabled)])),
            ("colorDynamicsPerTip", typed(b"bool", &[u8::from(per_tip)])),
            ("H   ", unit(b"#Prc", 20.)),
            ("Strt", unit(b"#Prc", 30.)),
            ("Brgh", unit(b"#Prc", 10.)),
            ("purity", unit(b"#Prc", 50.)),
            (
                "clVr",
                typed(
                    b"Objc",
                    &object(
                        "brushDynamics",
                        vec![
                            ("bVTy", typed(b"long", &0i32.to_be_bytes())),
                            ("fStp", typed(b"long", &25i32.to_be_bytes())),
                            ("Mnm ", unit(b"#Prc", 0.)),
                            ("jitter", unit(b"#Prc", 80.)),
                        ],
                    ),
                ),
            ),
        ]);
    }
    if pressure {
        for key in ["useTipDynamics", "usePaintDynamics"] {
            entries.push((key, typed(b"bool", &[1])));
        }
        entries.push(("minimumDiameter", unit(b"#Prc", 20.)));
        for (key, minimum) in [("szVr", 20.), ("prVr", 40.), ("opVr", 30.)] {
            entries.push((
                key,
                typed(
                    b"Objc",
                    &object(
                        "brushDynamics",
                        vec![
                            ("bVTy", typed(b"long", &2i32.to_be_bytes())),
                            ("fStp", typed(b"long", &0i32.to_be_bytes())),
                            (
                                "jitter",
                                unit(
                                    b"#Prc",
                                    if key == "szVr" && shape_jitter.is_some() {
                                        60.
                                    } else {
                                        0.
                                    },
                                ),
                            ),
                            ("Mnm ", unit(b"#Prc", minimum)),
                        ],
                    ),
                ),
            ));
        }
    }
    if let Some(enabled) = shape_jitter {
        if !pressure {
            entries.push(("useTipDynamics", typed(b"bool", &[u8::from(enabled)])));
            entries.push(("minimumDiameter", unit(b"#Prc", 20.)));
            entries.push((
                "szVr",
                typed(
                    b"Objc",
                    &object(
                        "brushDynamics",
                        vec![
                            ("bVTy", typed(b"long", &0i32.to_be_bytes())),
                            ("jitter", unit(b"#Prc", 60.)),
                            ("Mnm ", unit(b"#Prc", 0.)),
                        ],
                    ),
                ),
            ));
        }
        entries.push(("minimumRoundness", unit(b"#Prc", 25.)));
        for (key, jitter) in [("angleDynamics", 40.), ("roundnessDynamics", 50.)] {
            entries.push((
                key,
                typed(
                    b"Objc",
                    &object(
                        "brushDynamics",
                        vec![
                            ("bVTy", typed(b"long", &0i32.to_be_bytes())),
                            ("jitter", unit(b"#Prc", jitter)),
                            ("Mnm ", unit(b"#Prc", 0.)),
                            ("fStp", typed(b"long", &25i32.to_be_bytes())),
                        ],
                    ),
                ),
            ));
        }
    }
    if let Some((enabled, angle, tilt)) = device {
        if !pressure && shape_jitter.is_none() {
            entries.push(("useTipDynamics", typed(b"bool", &[u8::from(enabled)])));
        }
        // Replace just the two geometry controls, preserving combined jitter/pressure fields.
        entries.retain(|(key, _)| {
            !["angleDynamics", "roundnessDynamics", "minimumRoundness"].contains(key)
        });
        entries.push(("minimumRoundness", unit(b"#Prc", 25.)));
        for (key, control, jitter) in [
            ("angleDynamics", angle, 40.),
            ("roundnessDynamics", if tilt { 3 } else { 0 }, 50.),
        ] {
            entries.push((
                key,
                typed(
                    b"Objc",
                    &object(
                        "brushDynamics",
                        vec![
                            ("bVTy", typed(b"long", &control.to_be_bytes())),
                            ("Mnm ", unit(b"#Prc", 0.)),
                            ("fStp", typed(b"long", &25i32.to_be_bytes())),
                            (
                                "jitter",
                                unit(b"#Prc", if shape_jitter.is_some() { jitter } else { 0. }),
                            ),
                        ],
                    ),
                ),
            ));
        }
    }
    if let Some((
        pattern,
        mode,
        brightness,
        contrast,
        minimum_depth,
        depth_jitter,
        depth_pressure,
    )) = texture
    {
        entries.push(("useTexture", typed(b"bool", &[1])));
        entries.push((
            "Txtr",
            typed(
                b"Objc",
                &object(
                    "Ptrn",
                    vec![
                        ("Nm  ", typed(b"TEXT", &text("Tex\0"))),
                        ("Idnt", typed(b"TEXT", &text(&format!("{pattern}\0")))),
                    ],
                ),
            ),
        ));
        entries.push(("textureDepth", unit(b"#Prc", 100.)));
        entries.push(("textureScale", unit(b"#Prc", 100.)));
        entries.push(("TxtC", typed(b"bool", &[1])));
        entries.push(("InvT", typed(b"bool", &[0])));
        entries.push((
            "textureBlendMode",
            typed(b"enum", &[id("BlnM"), id(mode)].concat()),
        ));
        if brightness != 0. {
            entries.push((
                "textureBrightness",
                typed(b"long", &(brightness as i32).to_be_bytes()),
            ));
        }
        if contrast != 0. {
            entries.push((
                "textureContrast",
                typed(b"long", &(contrast as i32).to_be_bytes()),
            ));
        }
        entries.push(("minimumDepth", unit(b"#Prc", minimum_depth)));
        if depth_jitter != 0. {
            let control = if depth_pressure { 2i32 } else { 0i32 };
            entries.push((
                "textureDepthDynamics",
                typed(
                    b"Objc",
                    &object(
                        "brushDynamics",
                        vec![
                            ("bVTy", typed(b"long", &control.to_be_bytes())),
                            ("fStp", typed(b"long", &0i32.to_be_bytes())),
                            ("jitter", unit(b"#Prc", depth_jitter)),
                            ("Mnm ", unit(b"#Prc", minimum_depth)),
                        ],
                    ),
                ),
            ));
        }
    }
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
#[allow(dead_code)]
pub fn fixture_with_device(enabled: bool, angle: i32, combined: bool) -> Vec<u8> {
    fixture_inner(
        (6, 2),
        false,
        Some(25.5),
        enabled && combined,
        combined.then_some(true),
        combined.then_some((true, true)),
        (combined.then_some(enabled), Some((enabled, angle, true))),
        None,
    )
}
pub fn fixture(version: u16, sub: u16, bad: bool) -> Vec<u8> {
    fixture_with_opacity(version, sub, bad, None)
}
pub fn fixture_with_opacity(version: u16, sub: u16, bad: bool, opacity: Option<f64>) -> Vec<u8> {
    fixture_inner(
        (version, sub),
        bad,
        opacity,
        false,
        None,
        None,
        (None, None),
        None,
    )
}
#[allow(dead_code)]
pub fn fixture_with_pressure(version: u16, sub: u16, bad: bool) -> Vec<u8> {
    fixture_inner(
        (version, sub),
        bad,
        Some(25.5),
        true,
        None,
        None,
        (None, None),
        None,
    )
}
#[allow(dead_code)]
pub fn fixture_with_scatter(version: u16, sub: u16, bad: bool, enabled: bool) -> Vec<u8> {
    fixture_inner(
        (version, sub),
        bad,
        Some(25.5),
        true,
        Some(enabled),
        None,
        (None, None),
        None,
    )
}
#[allow(dead_code)]
pub fn fixture_with_color(enabled: bool, per_tip: bool, combined: bool) -> Vec<u8> {
    fixture_inner(
        (6, 2),
        false,
        Some(25.5),
        combined,
        combined.then_some(true),
        Some((enabled, per_tip)),
        (None, None),
        None,
    )
}
#[allow(dead_code)]
pub fn fixture_with_shape(enabled: bool, combined: bool) -> Vec<u8> {
    fixture_inner(
        (6, 2),
        false,
        Some(25.5),
        enabled && combined,
        combined.then_some(true),
        combined.then_some((true, true)),
        (Some(enabled), None),
        None,
    )
}
#[allow(dead_code)]
pub fn fixture_with_texture(mode: &str) -> Vec<u8> {
    let bytes = fixture_inner(
        (6, 2),
        false,
        Some(100.),
        false,
        None,
        None,
        (None, None),
        Some(("pattern-identity", mode, 0., 0., 0., 0., false)),
    );
    let mut output = bytes[..4].to_vec();
    let mut offset = 4;
    while offset < bytes.len() {
        let size = u32::from_be_bytes(bytes[offset + 8..offset + 12].try_into().unwrap()) as usize;
        output.extend_from_slice(&bytes[offset..offset + 12 + size]);
        output.resize(output.len() + (4 - size % 4) % 4, 0);
        offset += 12 + size;
    }
    output
}
#[allow(dead_code)]
pub fn fixture_with_texture_adjustments(mode: &str, brightness: f64, contrast: f64) -> Vec<u8> {
    let bytes = fixture_inner(
        (6, 2),
        false,
        Some(100.),
        false,
        None,
        None,
        (None, None),
        Some((
            "pattern-identity",
            mode,
            brightness,
            contrast,
            0.,
            0.,
            false,
        )),
    );
    let mut output = bytes[..4].to_vec();
    let mut offset = 4;
    while offset < bytes.len() {
        let size = u32::from_be_bytes(bytes[offset + 8..offset + 12].try_into().unwrap()) as usize;
        output.extend_from_slice(&bytes[offset..offset + 12 + size]);
        output.resize(output.len() + (4 - size % 4) % 4, 0);
        offset += 12 + size;
    }
    output
}
#[allow(dead_code)]
pub fn fixture_with_texture_depth(
    mode: &str,
    minimum_depth: f64,
    depth_jitter: f64,
    depth_pressure: bool,
) -> Vec<u8> {
    let bytes = fixture_inner(
        (6, 2),
        false,
        Some(100.),
        false,
        None,
        None,
        (None, None),
        Some((
            "pattern-identity",
            mode,
            0.,
            0.,
            minimum_depth,
            depth_jitter,
            depth_pressure,
        )),
    );
    let mut output = bytes[..4].to_vec();
    let mut offset = 4;
    while offset < bytes.len() {
        let size = u32::from_be_bytes(bytes[offset + 8..offset + 12].try_into().unwrap()) as usize;
        output.extend_from_slice(&bytes[offset..offset + 12 + size]);
        output.resize(output.len() + (4 - size % 4) % 4, 0);
        offset += 12 + size;
    }
    output
}
#[allow(clippy::too_many_arguments)]
fn fixture_inner(
    (version, sub): (u16, u16),
    bad: bool,
    opacity: Option<f64>,
    pressure: bool,
    scatter: Option<bool>,
    color: Option<(bool, bool)>,
    dynamics: (Option<bool>, Option<(bool, i32, bool)>),
    texture: Option<(&str, &str, f64, f64, f64, f64, bool)>,
) -> Vec<u8> {
    let mut bytes = version.to_be_bytes().to_vec();
    bytes.extend(sub.to_be_bytes());
    let mut list = 3u32.to_be_bytes().to_vec();
    list.extend(preset(
        true, false, opacity, pressure, scatter, color, dynamics, texture,
    ));
    list.extend(preset(
        false, false, opacity, pressure, scatter, color, dynamics, None,
    ));
    list.extend(preset(
        true, bad, opacity, pressure, scatter, color, dynamics, None,
    ));
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
    if let Some((pattern, _, _, _, _, _, _)) = texture {
        let mut body = 1u32.to_be_bytes().to_vec();
        body.extend(1u32.to_be_bytes());
        body.extend(4u16.to_be_bytes());
        body.extend(4u16.to_be_bytes());
        let name: Vec<u16> = "Tex\0".encode_utf16().collect();
        body.extend((name.len() as u32).to_be_bytes());
        for unit in name {
            body.extend(unit.to_be_bytes());
        }
        body.push(pattern.len() as u8);
        body.extend(pattern.as_bytes());
        body.extend(3u32.to_be_bytes());
        body.extend(0u32.to_be_bytes());
        for value in [0u32, 0, 4, 4] {
            body.extend(value.to_be_bytes());
        }
        body.extend(24u32.to_be_bytes());
        let pixels = vec![
            0u8, 255, 0, 255, 255, 0, 255, 0, 0, 255, 0, 255, 255, 0, 255, 0,
        ];
        let mut channel = 8u32.to_be_bytes().to_vec();
        for value in [0u32, 0, 4, 4] {
            channel.extend(value.to_be_bytes());
        }
        channel.extend(8u16.to_be_bytes());
        channel.push(0);
        channel.extend(&pixels);
        body.extend(1u32.to_be_bytes());
        body.extend((channel.len() as u32).to_be_bytes());
        body.extend(channel);
        body.extend(vec![0u8; 23 * 4]);
        let mut block = (body.len() as u32).to_be_bytes().to_vec();
        block.extend(body);
        block.resize(block.len() + (4 - block.len() % 4) % 4, 0);
        section(&mut bytes, b"patt", &block);
    }
    bytes
}
