use paint_ffi::*;
use std::ptr;

#[test]
fn cursor_image_public_capacity_errors_exact_bytes_and_shared_lifecycle() {
    let directory = tempfile::tempdir().unwrap();
    let assets = directory.path().join("assets");
    let path = assets.to_str().unwrap().as_bytes();
    let source = directory.path().join("tip.png");
    std::fs::write(
        &source,
        paint_io::brush_resource::encode_mask(
            &paint_core::BrushMask::new(4, 2, vec![255, 0, 128, 255, 0, 255, 255, 128]).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    let input = source.to_str().unwrap().as_bytes();
    unsafe {
        let mut library = ptr::null_mut();
        let mut shared = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        assert_eq!(paint_brush_library_create(0, &mut shared), PAINT_OK);
        let mut required = 999;
        let id_missing = [b'0'; 64];
        assert_eq!(
            paint_brush_library_tip_cursor_png(
                library,
                id_missing.as_ptr(),
                64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_NOT_FOUND
        );
        assert_eq!(required, 0);
        assert_eq!(
            paint_brush_library_bind_resources(library, path.as_ptr(), path.len() as u64),
            PAINT_OK
        );
        assert_eq!(
            paint_brush_library_share_resources(shared, library),
            PAINT_OK
        );
        let mut id = [0; 64];
        let mut depth = 0;
        assert_eq!(
            paint_brush_library_import_tip_file(
                library,
                input.as_ptr(),
                input.len() as u64,
                0,
                id.as_mut_ptr(),
                64,
                &mut required,
                &mut depth
            ),
            PAINT_OK
        );
        let mut before = PaintBrushLibraryInfo {
            struct_size: std::mem::size_of::<PaintBrushLibraryInfo>() as u32,
            ..Default::default()
        };
        assert_eq!(paint_brush_library_info(library, &mut before), PAINT_OK);
        assert_eq!(
            paint_brush_library_tip_cursor_png(
                library,
                id.as_ptr(),
                64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert!(required > 32 && required < 512 * 1024);
        let needed = required;
        let mut bytes = vec![0xa5; needed as usize + 2];
        assert_eq!(
            paint_brush_library_tip_cursor_png(
                library,
                id.as_ptr(),
                64,
                bytes.as_mut_ptr().add(1),
                needed - 1,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(required, needed);
        assert!(bytes.iter().all(|b| *b == 0xa5));
        assert_eq!(
            paint_brush_library_tip_cursor_png(
                library,
                id.as_ptr(),
                64,
                ptr::null_mut(),
                needed,
                &mut required
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(required, 0);
        assert_eq!(
            paint_brush_library_tip_cursor_png(
                library,
                id.as_ptr(),
                64,
                bytes.as_mut_ptr().add(1),
                needed,
                &mut required
            ),
            PAINT_OK
        );
        assert_eq!(bytes[0], 0xa5);
        assert_eq!(bytes[bytes.len() - 1], 0xa5);
        let store = paint_io::brush_resource::ResourceStore::open(
            assets.clone(),
            paint_io::brush_resource::DEFAULT_RESOURCE_BUDGET,
        )
        .unwrap();
        let expected = store
            .cursor_png(
                std::str::from_utf8(&id).unwrap(),
                &paint_task::CancellationToken::default(),
            )
            .unwrap();
        assert_eq!(&bytes[1..bytes.len() - 1], expected.as_slice());
        assert_eq!(&bytes[1..9], b"\x89PNG\r\n\x1a\n");
        assert_eq!(u32::from_be_bytes(bytes[17..21].try_into().unwrap()), 256);
        assert_eq!(
            PAINT_BRUSH_CURSOR_IMAGE_EDGE,
            paint_core::BRUSH_CURSOR_IMAGE_EDGE
        );
        let mut after = PaintBrushLibraryInfo {
            struct_size: std::mem::size_of::<PaintBrushLibraryInfo>() as u32,
            ..Default::default()
        };
        assert_eq!(paint_brush_library_info(library, &mut after), PAINT_OK);
        assert_eq!(
            (before.revision, before.preset_count),
            (after.revision, after.preset_count)
        );
        let bad = b"../bad";
        assert_eq!(
            paint_brush_library_tip_cursor_png(
                library,
                bad.as_ptr(),
                bad.len() as u64,
                bytes.as_mut_ptr(),
                needed,
                &mut required
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(required, 0);
        assert_eq!(
            paint_brush_library_tip_cursor_png(
                library,
                id.as_ptr(),
                64,
                bytes.as_mut_ptr(),
                u64::MAX,
                &mut required
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(required, 0);
        std::fs::remove_file(&source).unwrap();
        let stale = library;
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
        assert_eq!(
            paint_brush_library_tip_cursor_png(
                stale,
                id.as_ptr(),
                64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(required, 0);
        assert_eq!(
            paint_brush_library_tip_cursor_png(
                shared,
                id.as_ptr(),
                64,
                bytes.as_mut_ptr().add(1),
                needed,
                &mut required
            ),
            PAINT_OK
        );
        assert_eq!(&bytes[1..bytes.len() - 1], expected.as_slice());
        assert_eq!(paint_brush_library_destroy(&mut shared), PAINT_OK);
    }
}

#[test]
fn cursor_image_public_version_and_feature_are_advertised() {
    unsafe {
        let mut version = PaintVersion {
            struct_size: std::mem::size_of::<PaintVersion>() as u32,
            ..Default::default()
        };
        assert_eq!(paint_core_version(&mut version), PAINT_OK);
        assert_eq!(version.minor, 53);
        let mut core = ptr::null_mut();
        assert_eq!(paint_core_create(&mut core), PAINT_OK);
        let mut caps = PaintCapabilities {
            struct_size: std::mem::size_of::<PaintCapabilities>() as u32,
            ..Default::default()
        };
        assert_eq!(paint_core_capabilities(core, &mut caps), PAINT_OK);
        assert_ne!(caps.features & PAINT_FEATURE_BRUSH_CURSOR_IMAGE, 0);
        assert_eq!(paint_core_destroy(&mut core), PAINT_OK);
    }
}

#[test]
fn resource_import_validates_outputs_before_writing_and_shared_store_outlives_source() {
    let directory = tempfile::tempdir().unwrap();
    let assets = directory.path().join("assets");
    let path = assets.to_str().unwrap().as_bytes();
    let source = directory.path().join("tip.png");
    let mask = paint_core::BrushMask::new(8, 8, vec![255; 64]).unwrap();
    std::fs::write(
        &source,
        paint_io::brush_resource::encode_mask(&mask).unwrap(),
    )
    .unwrap();
    let input = source.to_str().unwrap().as_bytes();
    unsafe {
        let mut library = ptr::null_mut();
        let mut preview = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        assert_eq!(paint_brush_library_create(0, &mut preview), PAINT_OK);
        assert_eq!(
            paint_brush_library_share_resources(preview, library),
            PAINT_NOT_FOUND
        );
        assert_eq!(
            paint_brush_library_bind_resources(library, path.as_ptr(), path.len() as u64),
            PAINT_OK
        );
        assert_eq!(
            paint_brush_library_bind_resources(library, path.as_ptr(), path.len() as u64),
            PAINT_OK
        );
        let other = directory.path().join("other");
        let other_path = other.to_str().unwrap().as_bytes();
        assert_eq!(
            paint_brush_library_bind_resources(
                library,
                other_path.as_ptr(),
                other_path.len() as u64
            ),
            PAINT_BUSY
        );
        assert!(!other.exists());
        assert_eq!(
            paint_brush_library_share_resources(preview, library),
            PAINT_OK
        );
        let mut required = 0;
        let mut depth = 0;
        assert_eq!(
            paint_brush_library_import_tip_file(
                library,
                input.as_ptr(),
                input.len() as u64,
                0,
                ptr::null_mut(),
                0,
                &mut required,
                &mut depth
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(required, 64);
        assert_eq!(
            paint_brush_library_import_tip_file(
                library,
                input.as_ptr(),
                input.len() as u64,
                0,
                ptr::null_mut(),
                64,
                &mut required,
                &mut depth
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(std::fs::read_dir(&assets).unwrap().count(), 0);
        let mut id = [0; 64];
        assert_eq!(
            paint_brush_library_import_tip_file(
                library,
                input.as_ptr(),
                input.len() as u64,
                99,
                id.as_mut_ptr(),
                64,
                &mut required,
                &mut depth
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(
            paint_brush_library_import_tip_file(
                library,
                input.as_ptr(),
                input.len() as u64,
                0,
                id.as_mut_ptr(),
                64,
                &mut required,
                &mut depth
            ),
            PAINT_OK
        );
        assert_eq!(required, 64);
        assert_eq!(depth, 8);
        assert_eq!(
            paint_brush_library_tip_outline(
                library,
                id.as_ptr(),
                64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert!(required > 0 && required < 4 * 1024 * 1024);
        let mut path = vec![0; required as usize];
        assert_eq!(
            paint_brush_library_tip_outline(
                library,
                id.as_ptr(),
                64,
                path.as_mut_ptr(),
                path.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        assert!(std::str::from_utf8(&path).unwrap().starts_with('M'));
        assert!(assets
            .join(format!("{}.png", std::str::from_utf8(&id).unwrap()))
            .exists());
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
        let mut duplicate = [0; 64];
        assert_eq!(
            paint_brush_library_import_tip_file(
                preview,
                input.as_ptr(),
                input.len() as u64,
                0,
                duplicate.as_mut_ptr(),
                64,
                &mut required,
                &mut depth
            ),
            PAINT_OK
        );
        assert_eq!(duplicate, id);
        assert_eq!(paint_brush_library_destroy(&mut preview), PAINT_OK);
    }
}
