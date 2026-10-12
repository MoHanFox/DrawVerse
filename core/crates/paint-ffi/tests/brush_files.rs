use paint_ffi::*;
use std::ptr;
const PACKAGE: &str = r#"{"packageId":"12345678-1234-1234-1234-123456789abc","brushes":[{"id":"12345678-1234-1234-1234-123456789abd","name":"Soft ellipse","engineProfile":"drawverse.brush","behaviorVersion":1,"requiredCapabilities":["procedural-tip"],"parameters":{"diameter":24,"spacing":0.01,"dabOpacity":1,"hardness":0.2,"roundness":0.4,"angle":45,"flow":0.8,"pressureSize":false,"pressureFlow":true}}]}"#;
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
