//! Generated ABR framing with a typed descriptor, no Photoshop compatibility claim.
use paint_io::{abr_sampled::decode_sectioned, ps_descriptor::Value};
use paint_task::CancellationToken;
fn section(key: &[u8; 4], bytes: &[u8], out: &mut Vec<u8>) {
    out.extend(b"8BIM");
    out.extend(key);
    out.extend((bytes.len() as u32).to_be_bytes());
    out.extend(bytes);
}
#[test]
fn sectioned_samples_and_descriptors_keep_independent_names_and_pixel_resources() {
    let mut file = vec![0, 6, 0, 1];
    let mut descriptor = 16u32.to_be_bytes().to_vec();
    descriptor.extend(0u32.to_be_bytes());
    descriptor.extend(0u32.to_be_bytes());
    descriptor.extend(b"Brsh");
    descriptor.extend(1u32.to_be_bytes());
    descriptor.extend(0u32.to_be_bytes());
    descriptor.extend(b"Nm  ");
    descriptor.extend(b"TEXT");
    descriptor.extend(2u32.to_be_bytes());
    descriptor.extend([0, 65, 0, 66]);
    section(b"desc", &descriptor, &mut file);
    let mut sample = vec![0; 47];
    sample[0] = 36;
    sample[1..37].fill(b'A');
    for bound in [0i32, 0, 1, 1] {
        sample.extend(bound.to_be_bytes());
    }
    sample.extend(8u16.to_be_bytes());
    sample.extend([0, 255]);
    let mut samples = (sample.len() as u32).to_be_bytes().to_vec();
    samples.extend(&sample);
    samples.resize(samples.len() + (4 - sample.len() % 4) % 4, 0);
    section(b"samp", &samples, &mut file);
    let token = CancellationToken::default();
    let parsed = decode_sectioned(&file, &token).unwrap();
    assert_eq!(parsed.tips[0].mask.coverage(), [255]);
    let descriptors = parsed.descriptors(&token).unwrap();
    assert_eq!(descriptors.len(), 1);
    assert_eq!(descriptors[0].get(b"Nm  "), Some(&Value::Text("AB".into())));
    assert_eq!(&*parsed.metadata[0].bytes, descriptor);
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(parsed.descriptors(&cancelled).is_err());
}
