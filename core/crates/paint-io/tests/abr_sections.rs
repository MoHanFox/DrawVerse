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
    for index in 0..2 {
        let mut record = vec![42; if subversion == 1 { 47 } else { 301 }];
        record[0] = 36;
        record[1] = b'A' + index;
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
                assert!(parsed.descriptors(&token).is_err());
                let tip = &parsed.tips[0];
                assert_eq!(tip.prefix.len(), if subversion == 1 { 47 } else { 301 });
                assert_eq!(tip.prefix[0], 36);
                assert_eq!(tip.prefix[1], b'A');
                assert!(tip.prefix[2..].iter().all(|b| *b == 42));
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
fn section_and_record_lengths_never_cross_declared_bounds() {
    let token = CancellationToken::default();
    let bytes = fixture(6, 2, true);
    // There is no outer byte-length/count for sections. A prefix ending after
    // a complete payload (or its exact optional alignment) can be a full file.
    let mut valid_ends = std::collections::BTreeSet::new();
    let mut offset = 4;
    while offset < bytes.len() {
        let size = u32::from_be_bytes(bytes[offset + 8..offset + 12].try_into().unwrap()) as usize;
        offset += 12 + size;
        valid_ends.insert(offset);
        valid_ends.insert(offset + (4 - size % 4) % 4);
    }
    for end in 0..bytes.len() {
        let result = decode_sectioned(&bytes[..end], &token);
        if result.is_ok() {
            assert!(valid_ends.contains(&end), "prefix {end}");
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
    unsupported[depth..depth + 2].copy_from_slice(&32u16.to_be_bytes());
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
#[test]
fn sample_ids_are_variable_pascal_fields_and_duplicates_are_rejected() {
    let token = CancellationToken::default();
    let mut output = vec![0, 6, 0, 1];
    let mut samples = Vec::new();
    for id in [b"short".as_slice(), b"longer-variable-identity".as_slice()] {
        let mut record = vec![id.len() as u8];
        record.extend(id);
        record.extend([0; 10]);
        for bound in [0i32, 0, 1, 1] {
            record.extend(bound.to_be_bytes());
        }
        record.extend(8u16.to_be_bytes());
        record.extend([0, 255]);
        samples.extend((record.len() as u32).to_be_bytes());
        samples.extend(&record);
        samples.resize(samples.len() + (4 - record.len() % 4) % 4, 0);
    }
    section(b"samp", &samples, &mut output);
    let parsed = decode_sectioned(&output, &token).unwrap();
    assert_eq!(parsed.tips[0].id, "short");
    assert_eq!(parsed.tips[1].id, "longer-variable-identity");
    assert_eq!(parsed.tips[0].prefix.len(), 16);
    let first_length = u32::from_be_bytes(samples[..4].try_into().unwrap()) as usize;
    let first_total = 4 + first_length.div_ceil(4) * 4;
    let mut repeated = samples[..first_total].to_vec();
    repeated.extend_from_within(..);
    let mut duplicate = vec![0, 6, 0, 1];
    section(b"samp", &repeated, &mut duplicate);
    assert!(decode_sectioned(&duplicate, &token).is_err());
}

fn padded_sections(bytes: &[u8]) -> Vec<u8> {
    let mut output = bytes[..4].to_vec();
    let mut offset = 4;
    while offset < bytes.len() {
        let size = u32::from_be_bytes(bytes[offset + 8..offset + 12].try_into().unwrap()) as usize;
        output.extend_from_slice(&bytes[offset..offset + 12 + size]);
        output.resize(output.len() + (4 - size % 4) % 4, 0xa5);
        offset += 12 + size;
    }
    output
}
#[test]
fn section_padding_and_unpadded_compatibility_preserve_every_payload() {
    let token = CancellationToken::default();
    for length in 1..=3 {
        let mut bytes = fixture(6, 2, true);
        section(b"tail", &vec![0x5a; length], &mut bytes);
        let old = decode_sectioned(&bytes, &token).unwrap();
        let aligned = padded_sections(&bytes);
        let new = decode_sectioned(&aligned, &token).unwrap();
        assert_eq!(old.tips.len(), new.tips.len());
        for (a, b) in old.tips.iter().zip(&new.tips) {
            assert_eq!(a.mask.coverage(), b.mask.coverage());
            assert_eq!(a.trailing_metadata, b.trailing_metadata);
        }
        assert_eq!(old.metadata.len(), new.metadata.len());
        for (a, b) in old.metadata.iter().zip(&new.metadata) {
            assert_eq!(a.key, b.key);
            assert_eq!(a.bytes, b.bytes);
        }
        let pad = (4 - length % 4) % 4;
        for cut in 1..pad {
            assert!(decode_sectioned(&aligned[..aligned.len() - cut], &token).is_err());
        }
        let mut extra = aligned.clone();
        extra.push(0);
        assert!(decode_sectioned(&extra, &token).is_err());
        let mut bad = aligned.clone();
        let next = 4 + 12 + b"uninterpreted descriptor".len().div_ceil(4) * 4;
        bad[next] = b'X';
        assert!(decode_sectioned(&bad, &token).is_err());
    }
}
