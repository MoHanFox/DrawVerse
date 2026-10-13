use paint_ffi::*;
use paint_io::{
    abr_conversion::ConvertedPreset,
    brush_resource::ResourceStore,
    brush_source::SourceArchive,
    ps_descriptor::{Class, Descriptor},
};
use std::{ptr, sync::Arc};
#[test]
fn source_query_returns_exact_verified_record_and_clears_outputs_on_failure() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("resources");
    let resources =
        ResourceStore::open(directory.clone(), paint_core::MAX_CATALOG_MASK_BYTES).unwrap();
    let sources = resources.source_store().unwrap();
    let descriptor = Descriptor {
        class: Class {
            name: String::new(),
            id: b"null".to_vec(),
        },
        entries: Vec::new(),
    };
    let converted = ConvertedPreset {
        name: "original",
        settings: paint_core::BrushSettings::default(),
        source: &descriptor,
        issues: Vec::new(),
    };
    let archive = SourceArchive::capture(Arc::new(vec![0, 6, 0, 2, 42]), 0, &converted).unwrap();
    let token = paint_task::CancellationToken::default();
    let id = sources.save(&archive, &token).unwrap();
    let expected = sources.encoded_record(&id, &token).unwrap();
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let path = directory.to_str().unwrap();
        assert_eq!(
            paint_brush_library_bind_resources(library, path.as_ptr(), path.len() as u64),
            PAINT_OK
        );
        let mut required = 99;
        assert_eq!(
            paint_brush_library_source_info(
                library,
                id.as_ptr(),
                id.len() as u64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(required as usize, expected.len());
        let mut bytes = vec![0; required as usize];
        assert_eq!(
            paint_brush_library_source_info(
                library,
                id.as_ptr(),
                id.len() as u64,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        assert_eq!(bytes, expected);
        std::fs::write(
            directory.join("sources").join(format!("{id}.json")),
            b"corrupt",
        )
        .unwrap();
        required = 99;
        assert_eq!(
            paint_brush_library_source_info(
                library,
                id.as_ptr(),
                id.len() as u64,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut required
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(required, 0);
        assert_eq!(bytes, expected);
        assert_eq!(
            paint_brush_library_source_info(
                library,
                b"../bad".as_ptr(),
                6,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_INVALID_ARGUMENT
        );
        let stale = library;
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
        required = 99;
        assert_eq!(
            paint_brush_library_source_info(
                stale,
                id.as_ptr(),
                id.len() as u64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(required, 0);
    }
}
