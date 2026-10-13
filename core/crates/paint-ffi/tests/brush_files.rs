use paint_ffi::*;
use std::ptr;
const PACKAGE: &str = r#"{"packageId":"12345678-1234-1234-1234-123456789abc","brushes":[{"id":"12345678-1234-1234-1234-123456789abd","name":"Soft ellipse","engineProfile":"drawverse.brush","behaviorVersion":1,"requiredCapabilities":["procedural-tip"],"parameters":{"diameter":24,"spacing":0.01,"dabOpacity":1,"hardness":0.2,"roundness":0.4,"angle":45,"flow":0.8,"pressureSize":false,"pressureFlow":true}}]}"#;
#[test]
fn library_vbr_resources_are_portable_and_size_queries_do_not_write_assets() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let target = dir.path().join("target");
    let mask = paint_core::BrushMask::new(8, 8, vec![255; 64]).unwrap();
    let bytes = paint_io::brush_resource::encode_mask(&mask).unwrap();
    let id = paint_io::brush_resource::hash(&bytes);
    let store = paint_io::brush_resource::ResourceStore::open(source.clone(), 1024).unwrap();
    store
        .put_verified(&id, &bytes, &paint_task::CancellationToken::default())
        .unwrap();
    let json = PACKAGE
        .replace("\"procedural-tip\"", "\"image-tip\"")
        .replace("\"name\":", &format!("\"tipResource\":\"{id}\",\"name\":"));
    let path = dir.path().join("image.vbr");
    let path_bytes = path.to_str().unwrap().as_bytes();
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let source_bytes = source.to_str().unwrap().as_bytes();
        assert_eq!(
            paint_brush_library_bind_resources(
                library,
                source_bytes.as_ptr(),
                source_bytes.len() as u64
            ),
            PAINT_OK
        );
        assert_eq!(
            paint_brush_library_package_write(
                library,
                path_bytes.as_ptr(),
                path_bytes.len() as u64,
                json.as_ptr(),
                json.len() as u64
            ),
            PAINT_OK
        );
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let target_bytes = target.to_str().unwrap().as_bytes();
        assert_eq!(
            paint_brush_library_bind_resources(
                library,
                target_bytes.as_ptr(),
                target_bytes.len() as u64
            ),
            PAINT_OK
        );
        let mut required = 0;
        assert_eq!(
            paint_brush_library_package_read(
                library,
                path_bytes.as_ptr(),
                path_bytes.len() as u64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert!(required > 0);
        assert_eq!(std::fs::read_dir(&target).unwrap().count(), 0);
        assert_eq!(
            paint_brush_library_package_read(
                library,
                path_bytes.as_ptr(),
                path_bytes.len() as u64,
                ptr::null_mut(),
                required,
                &mut required
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(std::fs::read_dir(&target).unwrap().count(), 0);
        let mut buffer = vec![0; required as usize];
        assert_eq!(
            paint_brush_library_package_read(
                library,
                path_bytes.as_ptr(),
                path_bytes.len() as u64,
                buffer.as_mut_ptr(),
                buffer.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        assert_eq!(
            std::fs::read(target.join(format!("{id}.png"))).unwrap(),
            bytes
        );
        assert!(String::from_utf8(buffer).unwrap().contains(&id));
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
}
#[test]
fn worker_vbr_roundtrip_buffer_contract_and_failure_preserves_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("brush.vbr");
    let path_text = path.to_str().unwrap().as_bytes();
    unsafe {
        assert_eq!(
            paint_brush_package_write(
                path_text.as_ptr(),
                path_text.len() as u64,
                PACKAGE.as_ptr(),
                PACKAGE.len() as u64
            ),
            PAINT_OK
        );
        let original = std::fs::read(&path).unwrap();
        let mut required = 999;
        assert_eq!(
            paint_brush_package_read(
                path_text.as_ptr(),
                path_text.len() as u64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert!(required > 0);
        let mut buffer = vec![0; required as usize];
        assert_eq!(
            paint_brush_package_read(
                path_text.as_ptr(),
                path_text.len() as u64,
                buffer.as_mut_ptr(),
                buffer.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        assert!(String::from_utf8(buffer).unwrap().contains("Soft ellipse"));
        assert_eq!(
            paint_brush_package_write(
                path_text.as_ptr(),
                path_text.len() as u64,
                b"{}".as_ptr(),
                2
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert_eq!(
            paint_brush_package_write(
                path_text.as_ptr(),
                path_text.len() as u64,
                ptr::null(),
                2 * 1024 * 1024 + 1
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(
            paint_brush_package_read(ptr::null(), 0, ptr::null_mut(), 0, &mut required),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(required, 0);
    }
}
