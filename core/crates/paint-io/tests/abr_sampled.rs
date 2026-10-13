//! Generated protocol fixtures; these are not Photoshop-produced compatibility files.
use paint_io::abr_sampled::decode;
use paint_task::CancellationToken;
fn fixture(version: u16, rle: bool) -> Vec<u8> {
    let mut record = Vec::new();
    record.extend(0u32.to_be_bytes());
    record.extend(15u16.to_be_bytes());
    if version == 2 {
        let text = "孔洞笔尖\0".encode_utf16().collect::<Vec<_>>();
        record.extend((text.len() as u32).to_be_bytes());
        for unit in text {
            record.extend(unit.to_be_bytes());
        }
    }
    record.push(1);
    record.extend([0; 8]);
    for value in [-3i32, -2, 5, 6] {
        record.extend(value.to_be_bytes());
    }
    record.extend(8u16.to_be_bytes());
    record.push(u8::from(rle));
    let mut pixels = vec![255; 64];
    for y in 3..5 {
        for x in 3..5 {
            pixels[y * 8 + x] = 0;
        }
    }
    if rle {
        for _ in 0..8 {
            record.extend(10u16.to_be_bytes());
        }
        for row in pixels.chunks(8) {
            record.push(128);
            record.push(7);
            record.extend(row);
        }
    } else {
        record.extend(pixels);
    }
    let mut bytes = Vec::new();
    bytes.extend(version.to_be_bytes());
    bytes.extend(1u16.to_be_bytes());
    bytes.extend(2u16.to_be_bytes());
    bytes.extend((record.len() as u32).to_be_bytes());
    bytes.extend(record);
    bytes
}
#[test]
fn generated_v1_v2_raw_and_packbits_keep_names_bounds_and_execute_holes() {
    let token = CancellationToken::default();
    for version in [1, 2] {
        for rle in [false, true] {
            let tips = decode(&fixture(version, rle), &token).unwrap();
            assert_eq!(tips.len(), 1);
            assert_eq!(tips[0].mask.dimensions(), (8, 8));
            assert_eq!(tips[0].spacing_percent, 15);
            assert_eq!(tips[0].bounds, [-3, -2, 5, 6]);
            assert!(tips[0].antialias);
            assert_eq!(
                tips[0].name.as_deref(),
                if version == 1 {
                    None
                } else {
                    Some("孔洞笔尖")
                }
            );
            let mut d = paint_core::Document::new(32, 32).unwrap();
            let mut definition = paint_core::BrushDefinition::default();
            definition.brush.radius = 4.;
            definition.mask = Some(tips[0].mask.clone());
            d.begin_brush_stroke(definition, paint_core::InputPoint::new(16.5, 16.5, 1.))
                .unwrap();
            d.end_stroke().unwrap();
            assert_eq!(d.pixel(16, 16).alpha(), 0.);
            assert!(d.pixel(19, 16).alpha() > 0.9);
        }
    }
}
#[test]
fn every_truncation_unknown_record_tail_and_cancel_reject_without_partial_output() {
    let token = CancellationToken::default();
    let bytes = fixture(2, true);
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end], &token).is_err(), "truncation {end}");
    }
    let mut tail = bytes.clone();
    tail.push(0);
    assert!(decode(&tail, &token).is_err());
    let mut unknown = bytes.clone();
    unknown[5] = 1;
    assert!(decode(&unknown, &token).is_err());
    let mut count = bytes.clone();
    count[2..4].copy_from_slice(&1025u16.to_be_bytes());
    assert!(decode(&count, &token).is_err());
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(decode(&bytes, &cancelled).is_err());
}
#[test]
fn packbits_repeat_packets_are_exact_and_row_overflows_reject() {
    // v1's fixed sampled header is 34 bytes; row data begins after its 16-byte counts.
    let mut raw = fixture(1, false);
    let offset = 10 + 34;
    raw[offset - 1] = 1;
    let mut body = Vec::new();
    for _ in 0..8 {
        body.extend(2u16.to_be_bytes());
    }
    for _ in 0..8 {
        body.extend([249, 255]);
    }
    raw.truncate(offset);
    raw.extend(body);
    let length = (raw.len() - 10) as u32;
    raw[6..10].copy_from_slice(&length.to_be_bytes());
    let token = CancellationToken::default();
    assert_eq!(decode(&raw, &token).unwrap()[0].mask.coverage(), [255; 64]);
    for control in [248, 250] {
        let mut bad = raw.clone();
        bad[offset + 16] = control;
        assert!(decode(&bad, &token).is_err());
    }
}
