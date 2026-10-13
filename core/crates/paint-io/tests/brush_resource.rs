use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use paint_io::brush_resource::*;
use paint_task::CancellationToken;
use std::sync::Arc;
fn png(w: u32, h: u32, pixels: &[u8], color: ExtendedColorType) -> Vec<u8> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(pixels, w, h, color)
        .unwrap();
    bytes
}
#[test]
fn outlines_share_immutable_geometry_and_evict_old_entries_without_pinning_masks() {
    let directory = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(directory.path().into(), DEFAULT_RESOURCE_BUDGET).unwrap();
    let token = CancellationToken::default();
    let mut first = None;
    for size in 1..=10 {
        let resource = store
            .import_png(
                &png(size, 1, &vec![255; size as usize], ExtendedColorType::L8),
                Interpretation::Coverage,
                &token,
            )
            .unwrap();
        let path = store.outline_svg(&resource.id, &token).unwrap();
        assert!(path.contains('M') && path.contains('L'));
        assert!(Arc::ptr_eq(
            &path,
            &store.outline_svg(&resource.id, &token).unwrap()
        ));
        if size == 1 {
            first = Some((resource.id.clone(), Arc::downgrade(&path)));
        }
    }
    let (id, weak) = first.unwrap();
    assert!(weak.upgrade().is_none());
    assert_eq!(store.resident_bytes(), 0);
    let rebuilt = store.outline_svg(&id, &token).unwrap();
    assert!(!rebuilt.is_empty());
    assert!(store.outline_svg("../bad", &token).is_err());
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(store.outline_svg(&id, &cancelled).is_err());
}
#[test]
fn known_sha_grayscale_alpha_and_sixteen_bit_data_have_exact_coverage() {
    assert_eq!(
        hash(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let directory = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(directory.path().into(), DEFAULT_RESOURCE_BUDGET).unwrap();
    let token = CancellationToken::default();
    let bytes = png(2, 1, &[255, 128, 128, 128], ExtendedColorType::La8);
    let id = hash(&bytes);
    let stored = store.put_verified(&id, &bytes, &token).unwrap();
    assert_eq!(stored.mask.coverage(), [128, 64]);
    assert_eq!(stored.mask.dimensions(), (2, 1));
    assert_eq!(store.encoded(&id).unwrap(), bytes);
    assert!(!stored.quantized_from_16());
    let values = [65535u16, 32768, 32768, 65535]
        .into_iter()
        .flat_map(u16::to_ne_bytes)
        .collect::<Vec<_>>();
    let bytes = png(2, 1, &values, ExtendedColorType::La16);
    let stored = store.put_verified(&hash(&bytes), &bytes, &token).unwrap();
    assert_eq!(stored.mask.coverage(), [128, 128]);
    assert!(stored.quantized_from_16());
}
#[test]
fn explicit_user_interpretation_normalizes_portable_masks_and_deduplicates() {
    let directory = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(directory.path().into(), 1).unwrap();
    let token = CancellationToken::default();
    let colored = png(1, 1, &[255, 0, 0, 128], ExtendedColorType::Rgba8);
    assert!(store
        .import_png(&colored, Interpretation::Coverage, &token)
        .is_err());
    let alpha = store
        .import_png(&colored, Interpretation::Alpha, &token)
        .unwrap();
    assert_eq!(alpha.mask.coverage(), [128]);
    let same = store
        .import_png(&colored, Interpretation::Alpha, &token)
        .unwrap();
    assert!(Arc::ptr_eq(&alpha.mask, &same.mask));
    assert_eq!(store.resident_bytes(), 1);
    let id = alpha.id.clone();
    drop(alpha);
    drop(same);
    let loaded = store.load(&id, &token).unwrap();
    assert_eq!(loaded.mask.coverage(), [128]);
    assert_eq!(loaded.source_depth, 8);
    drop(loaded);
    let inverse = store
        .import_png(&colored, Interpretation::InverseLuminance, &token)
        .unwrap();
    assert_eq!(inverse.mask.coverage(), [101]);
}
#[test]
fn live_budget_duplicate_arc_release_and_failed_store_are_atomic() {
    let directory = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(directory.path().into(), 5).unwrap();
    let token = CancellationToken::default();
    let first = png(2, 2, &[255; 4], ExtendedColorType::L8);
    let second = png(2, 2, &[128; 4], ExtendedColorType::L8);
    let a = store.put_verified(&hash(&first), &first, &token).unwrap();
    assert_eq!(store.resident_bytes(), 5);
    let same = store.load(&a.id, &token).unwrap();
    assert!(Arc::ptr_eq(&a.mask, &same.mask));
    assert!(store.put_verified(&hash(&second), &second, &token).is_err());
    assert!(!directory
        .path()
        .join(format!("{}.png", hash(&second)))
        .exists());
    drop(a);
    drop(same);
    let b = store.put_verified(&hash(&second), &second, &token).unwrap();
    assert_eq!(b.mask.coverage(), [128; 4]);
    assert_eq!(store.resident_bytes(), 5);
    let cancelled = CancellationToken::default();
    cancelled.cancel();
    assert!(store
        .import_png(&first, Interpretation::Coverage, &cancelled)
        .is_err());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
}
#[test]
fn bad_hash_path_corruption_and_source_removal_do_not_mutate_captured_pixels() {
    let directory = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let store = ResourceStore::open(directory.path().into(), 100).unwrap();
    let bytes = png(2, 2, &[255, 0, 0, 255], ExtendedColorType::L8);
    let id = hash(&bytes);
    assert!(store.put_verified(&"a".repeat(64), &bytes, &token).is_err());
    assert!(store.load("../wrong.png", &token).is_err());
    assert!(store
        .import_png(b"invalid png", Interpretation::Coverage, &token)
        .is_err());
    let captured = store.put_verified(&id, &bytes, &token).unwrap();
    std::fs::write(directory.path().join(format!("{id}.png")), b"corrupt").unwrap();
    assert_eq!(captured.mask.coverage(), [255, 0, 0, 255]);
    let cold = ResourceStore::open(directory.path().into(), 100).unwrap();
    assert!(cold.load(&id, &token).is_err());
    assert!(cold.put_verified(&id, &bytes, &token).is_err());
    assert_eq!(
        std::fs::read(directory.path().join(format!("{id}.png"))).unwrap(),
        b"corrupt"
    );
}
#[test]
fn concurrent_imports_share_one_immutable_resource() {
    let directory = tempfile::tempdir().unwrap();
    let store = Arc::new(ResourceStore::open(directory.path().into(), 5).unwrap());
    let bytes = Arc::new(png(2, 2, &[255; 4], ExtendedColorType::L8));
    let threads = (0..4)
        .map(|_| {
            let store = store.clone();
            let bytes = bytes.clone();
            std::thread::spawn(move || {
                store
                    .put_verified(&hash(&bytes), &bytes, &CancellationToken::default())
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let masks = threads
        .into_iter()
        .map(|t| t.join().unwrap())
        .collect::<Vec<_>>();
    for m in &masks {
        assert!(Arc::ptr_eq(&m.mask, &masks[0].mask));
    }
    assert_eq!(store.resident_bytes(), 5);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}
fn chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut bytes = (data.len() as u32).to_be_bytes().to_vec();
    bytes.extend_from_slice(name);
    bytes.extend_from_slice(data);
    let mut crc = !0u32;
    for byte in name.iter().chain(data) {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 };
        }
    }
    bytes.extend_from_slice(&(!crc).to_be_bytes());
    bytes
}
#[test]
fn oversized_dimensions_and_actual_animated_png_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let store = ResourceStore::open(directory.path().into(), DEFAULT_RESOURCE_BUDGET).unwrap();
    let token = CancellationToken::default();
    let oversized = png(4097, 1, &vec![255; 4097], ExtendedColorType::L8);
    assert!(store
        .import_png(&oversized, Interpretation::Coverage, &token)
        .is_err());
    let base = png(2, 2, &[255; 4], ExtendedColorType::L8);
    let mut animated = base[..33].to_vec();
    animated.extend(chunk(
        b"acTL",
        &[1u32.to_be_bytes(), 0u32.to_be_bytes()].concat(),
    ));
    let mut control = [0u32, 2, 2, 0, 0]
        .into_iter()
        .flat_map(u32::to_be_bytes)
        .collect::<Vec<_>>();
    control.extend_from_slice(&[0, 1, 0, 10, 0, 0]);
    animated.extend(chunk(b"fcTL", &control));
    animated.extend_from_slice(&base[33..]);
    let decoder = image::codecs::png::PngDecoder::new(std::io::Cursor::new(&animated)).unwrap();
    assert!(decoder.is_apng().unwrap());
    assert!(store
        .import_png(&animated, Interpretation::Coverage, &token)
        .is_err());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}
