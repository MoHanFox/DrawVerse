//! Independent generated framing fixtures, not a Photoshop compatibility corpus.
use paint_io::abr_sampled::decode_sectioned;
use paint_task::CancellationToken;
fn section(key: &[u8; 4], payload: &[u8], out: &mut Vec<u8>) {
    out.extend(b"8BIM");
    out.extend(key);
    out.extend((payload.len() as u32).to_be_bytes());
    out.extend(payload);
}
fn fixture(version: u16, subversion: u16, rle: bool) -> Vec<u8> {
    let mut output = Vec::new();
    output.extend(version.to_be_bytes());
    output.extend(subversion.to_be_bytes());
    section(b"desc", b"uninterpreted descriptor", &mut output);
    let mut samples = Vec::new();
    for _ in 0..2 {
        let mut record = vec![42; if subversion == 1 { 47 } else { 301 }];
        for bound in [-1i32, -2, 7, 6] {
            record.extend(bound.to_be_bytes());
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
                record.extend(9u16.to_be_bytes());
            }
            for row in pixels.chunks(8) {
                record.push(7);
                record.extend(row);
            }
        } else {
            record.extend(pixels);
        }
        record.extend(b"extra");
        samples.extend((record.len() as u32).to_be_bytes());
        samples.extend(&record);
        samples.resize(samples.len() + (4 - record.len() % 4) % 4, 99);
    }
    section(b"samp", &samples, &mut output);
    section(b"cust", b"first", &mut output);
    section(b"cust", b"second", &mut output);
    output
}
#[test]
fn both_versions_subversions_and_encodings_preserve_metadata_and_draw_holes() {
    let token = CancellationToken::default();
    for version in [6, 10] {
        for subversion in [1, 2] {
            for rle in [false, true] {
                let parsed = decode_sectioned(&fixture(version, subversion, rle), &token).unwrap();
                assert_eq!(parsed.version, version);
                assert_eq!(parsed.subversion, subversion);
                assert_eq!(parsed.tips.len(), 2);
                assert_eq!(parsed.metadata.len(), 3);
                assert_eq!(&*parsed.metadata[0].bytes, b"uninterpreted descriptor");
                assert_eq!(&*parsed.metadata[2].bytes, b"second");
                let tip = &parsed.tips[0];
                assert_eq!(tip.prefix.len(), if subversion == 1 { 47 } else { 301 });
                assert!(tip.prefix.iter().all(|b| *b == 42));
                assert_eq!(&*tip.trailing_metadata, b"extra");
                assert_eq!(tip.bounds, [-1, -2, 7, 6]);
                let mut d = paint_core::Document::new(32, 32).unwrap();
                let mut definition = paint_core::BrushDefinition::default();
                definition.brush.radius = 4.;
                definition.mask = Some(tip.mask.clone());
                d.begin_brush_stroke(definition, paint_core::InputPoint::new(16.5, 16.5, 1.))
                    .unwrap();
                d.end_stroke().unwrap();
                assert_eq!(d.pixel(16, 16).alpha(), 0.);
                assert!(d.pixel(19, 16).alpha() > 0.9);
            }
        }
    }
}
#[test]
fn section_and_record_lengths_never_cross_bounds_or_return_partial_data() {
    let token = CancellationToken::default();
    let bytes = fixture(6, 2, true);
    // Proper prefixes ending at a section boundary can be complete containers.
    for end in 0..bytes.len() {
        let result = decode_sectioned(&bytes[..end], &token);
        if result.is_ok() {
            assert!(bytes[end..].starts_with(b"8BIM"), "prefix {end}");
        }
    }
    let mut bad = bytes.clone();
    bad[4] = 0;
    assert!(decode_sectioned(&bad, &token).is_err());
    let mut bad = bytes.clone();
    bad[12..16].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(decode_sectioned(&bad, &token).is_err());
    let sample_offset = 4 + 12 + b"uninterpreted descriptor".len();
    let mut unsupported = bytes.clone();
    let depth = sample_offset + 12 + 4 + 301 + 16;
    unsupported[depth..depth + 2].copy_from_slice(&16u16.to_be_bytes());
    assert!(decode_sectioned(&unsupported, &token).is_err());
    let mut unsupported = bytes.clone();
    unsupported[2..4].copy_from_slice(&3u16.to_be_bytes());
    assert!(decode_sectioned(&unsupported, &token).is_err());
    let mut bad = bytes.clone();
    bad[sample_offset + 12..sample_offset + 16].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(decode_sectioned(&bad, &token).is_err());
    let mut duplicate = bytes.clone();
    let mut payload = Vec::new();
    section(b"samp", b"", &mut payload);
    duplicate.extend(payload);
    assert!(decode_sectioned(&duplicate, &token).is_err());
    let mut tail = bytes.clone();
    tail.push(0);
    assert!(decode_sectioned(&tail, &token).is_err());
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(decode_sectioned(&bytes, &cancelled).is_err());
}
