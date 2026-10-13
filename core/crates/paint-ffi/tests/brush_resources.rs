use paint_ffi::*;
use std::ptr;

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
