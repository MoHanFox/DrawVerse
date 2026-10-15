use paint_core::{BrushCatalog, MAX_CATALOG_MASK_BYTES};
use paint_ffi::*;
use paint_io::{abr_import::ImportBatch, brush_resource::ResourceStore, brush_state};
use paint_task::CancellationToken;
use std::{
    ptr,
    time::{Duration, Instant},
};
#[path = "../../paint-io/tests/support/abr_fixture.rs"]
mod fixture;
#[test]
fn abr_buffer_guards_and_whole_import_prepare_without_implicit_catalog_changes() {
    check_import(None, false);
}
#[test]
fn opacity_v2_import_preserves_source_and_prepared_catalog_through_public_abi() {
    check_import(Some(25.5), false);
}
#[test]
fn pressure_v3_import_prepares_and_preserves_native_baseline() {
    check_import(Some(25.5), true);
}
#[test]
fn licensed_real_pack_imports_all_148_and_prepares_sources_through_public_abi() {
    check_bytes(
        include_bytes!("../../paint-io/tests/fixtures/abr/myer-settlement-brushes.abr").to_vec(),
        None,
        false,
        false,
    );
}
fn check_import(opacity: Option<f64>, pressure: bool) {
    let bytes = if pressure {
        fixture::fixture_with_pressure(6, 2, false)
    } else {
        fixture::fixture_with_opacity(6, 2, false, opacity)
    };
    check_bytes(bytes, opacity, pressure, false);
}
#[test]
fn scatter_v4_import_preserves_native_source_and_full_state() {
    check_bytes(
        fixture::fixture_with_scatter(6, 2, false, true),
        Some(25.5),
        true,
        true,
    );
}
fn check_bytes(bytes: Vec<u8>, opacity: Option<f64>, pressure: bool, scatter: bool) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("brush.abr");
    std::fs::write(&path, bytes).unwrap();
    let token = CancellationToken::default();
    let batch = ImportBatch::read(&path, MAX_CATALOG_MASK_BYTES, &token).unwrap();
    let expected = batch.to_json().unwrap();
    let directory = temp.path().join("resources");
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let directory_text = directory.to_str().unwrap();
        assert_eq!(
            paint_brush_library_bind_resources(
                library,
                directory_text.as_ptr(),
                directory_text.len() as u64
            ),
            PAINT_OK
        );
        let path_text = path.to_str().unwrap();
        let mut required = 99;
        assert_eq!(
            paint_brush_library_abr_read(
                library,
                path_text.as_ptr(),
                path_text.len() as u64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(required as usize, expected.len());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
        assert_eq!(
            paint_brush_library_abr_read(
                library,
                path_text.as_ptr(),
                path_text.len() as u64,
                ptr::null_mut(),
                required,
                &mut required
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(required, 0);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
        let mut bytes = vec![0; expected.len()];
        assert_eq!(
            paint_brush_library_abr_read(
                library,
                path_text.as_ptr(),
                path_text.len() as u64,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        assert_eq!(bytes, expected);
        let mut canonical = vec![0; 2 * 1024 * 1024];
        assert_eq!(
            paint_brush_library_current_state(
                library,
                canonical.as_mut_ptr(),
                canonical.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        assert_eq!(
            &canonical[..required as usize],
            brush_state::encode(&BrushCatalog::default()).unwrap()
        );
        let resources = ResourceStore::open(directory, MAX_CATALOG_MASK_BYTES).unwrap();
        let imported = batch.persist(&resources, &token).unwrap();
        let mut catalog = BrushCatalog::default();
        for (index, preset) in imported.iter().enumerate() {
            let id = format!("00000000-0000-4000-8000-{index:012x}");
            catalog.save_copy(id.clone(), preset.name().into()).unwrap();
            catalog.edit(&id, preset.settings().clone()).unwrap();
            catalog
                .set_source(&id, Some(preset.source_id().into()))
                .unwrap();
        }
        let state = brush_state::encode(&catalog).unwrap();
        let mut seq = 0;
        assert_eq!(
            paint_brush_library_prepare_state(
                library,
                state.as_ptr(),
                state.len() as u64,
                &mut seq
            ),
            PAINT_OK
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let status = paint_brush_library_prepared_state(
                library,
                seq,
                canonical.as_mut_ptr(),
                canonical.len() as u64,
                &mut required,
            );
            if status == PAINT_BUSY {
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            assert_eq!(status, PAINT_OK);
            break;
        }
        assert_eq!(&canonical[..required as usize], state);
        std::fs::remove_file(&path).unwrap();
        let source = imported[0].source_id();
        assert_eq!(
            paint_brush_library_source_info(
                library,
                source.as_ptr(),
                source.len() as u64,
                canonical.as_mut_ptr(),
                canonical.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        let archive = resources
            .source_store()
            .unwrap()
            .load(source, &token)
            .unwrap();
        assert_eq!(&canonical[..required as usize], archive.record().unwrap());
        assert_eq!(
            archive.info.converter_version,
            if scatter {
                4
            } else if pressure {
                3
            } else if opacity.is_some() {
                2
            } else {
                1
            }
        );
        assert!(
            (f64::from(archive.info.baseline.dab_opacity) - opacity.unwrap_or(100.) / 100.).abs()
                < 1e-6
        );
        std::fs::write(&path, fixture::fixture(6, 2, true)).unwrap();
        required = 99;
        assert_eq!(
            paint_brush_library_abr_read(
                library,
                path_text.as_ptr(),
                path_text.len() as u64,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut required
            ),
            PAINT_LIMIT_EXCEEDED
        );
        assert_eq!(required, 0);
        assert_eq!(
            paint_brush_library_current_state(
                library,
                canonical.as_mut_ptr(),
                canonical.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        assert_eq!(&canonical[..required as usize], state);
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
}
