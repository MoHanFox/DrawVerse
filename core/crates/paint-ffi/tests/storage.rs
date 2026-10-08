use paint_ffi::*;
use std::{mem::size_of, ptr};
fn options(path: &[u8]) -> PaintStorageOptions {
    PaintStorageOptions {
        struct_size: size_of::<PaintStorageOptions>() as u32,
        reserved: 0,
        path: path.as_ptr(),
        path_length: path.len() as u64,
        resident_bytes: 65536,
        scratch_bytes: 1024 * 1024,
        min_free_bytes: 0,
    }
}
#[test]
fn storage_probe_core_lifecycle_and_bad_configuration() {
    let base = tempfile::Builder::new().prefix("配置-").tempdir().unwrap();
    let path = base.path().to_str().unwrap().as_bytes();
    let mut config = options(path);
    let mut info = PaintStorageInfo {
        struct_size: size_of::<PaintStorageInfo>() as u32,
        ..Default::default()
    };
    unsafe {
        assert_eq!(paint_storage_inspect(&config, &mut info), PAINT_OK);
        assert!(info.available_bytes > 0);
        let mut core = ptr::null_mut();
        assert_eq!(paint_core_create_with_storage(&config, &mut core), PAINT_OK);
        let mut caps = PaintCapabilities {
            struct_size: size_of::<PaintCapabilities>() as u32,
            ..Default::default()
        };
        assert_eq!(paint_core_capabilities(core, &mut caps), PAINT_OK);
        assert_ne!(caps.features & PAINT_FEATURE_STORAGE_SETTINGS, 0);
        let desc = PaintDocumentDesc {
            struct_size: size_of::<PaintDocumentDesc>() as u32,
            width: 128,
            height: 128,
            working_space: PAINT_WORKING_LINEAR_SRGB,
            pixel_format: PAINT_STORAGE_RGBA32F_PREMULTIPLIED,
            reserved: [0; 3],
        };
        let mut doc = ptr::null_mut();
        assert_eq!(paint_core_new_document(core, &desc, &mut doc), PAINT_OK);
        assert_eq!(paint_core_destroy(&mut core), PAINT_BUSY);
        assert!(!core.is_null());
        assert_eq!(paint_document_destroy(core, &mut doc), PAINT_OK);
        assert_eq!(paint_core_destroy(&mut core), PAINT_OK);
        assert_eq!(
            std::fs::read_dir(base.path().join(".drawverse-scratch-v1"))
                .unwrap()
                .count(),
            1
        );
        for bytes in [0, 65537, 2 * 1024 * 1024 * 1024] {
            config.resident_bytes = bytes;
            assert_eq!(
                paint_core_create_with_storage(&config, &mut core),
                PAINT_INVALID_ARGUMENT
            );
            assert!(core.is_null());
        }
        config = options(path);
        config.min_free_bytes = 1024 * 1024 * 1024 * 1024;
        assert_eq!(
            paint_core_create_with_storage(&config, &mut core),
            PAINT_IO_ERROR
        );
        assert!(core.is_null());
        config = options(path);
        config.reserved = 1;
        assert_eq!(
            paint_storage_inspect(&config, &mut info),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(info.available_bytes, 0);
        config = options(b"relative");
        assert_eq!(
            paint_storage_inspect(&config, &mut info),
            PAINT_INVALID_ARGUMENT
        );
    }
}
