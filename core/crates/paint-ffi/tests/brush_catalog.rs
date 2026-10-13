use paint_ffi::*;
use std::{
    mem::{offset_of, size_of},
    ptr,
};
macro_rules! dto {
    ($t:ident) => {
        $t {
            struct_size: size_of::<$t>() as u32,
            ..Default::default()
        }
    };
}
fn info(library: *mut PaintBrushLibrary) -> PaintBrushLibraryInfo {
    let mut info = dto!(PaintBrushLibraryInfo);
    unsafe {
        assert_eq!(paint_brush_library_info(library, &mut info), PAINT_OK);
    }
    info
}
fn settings() -> PaintBrushSettings {
    PaintBrushSettings {
        diameter: 24.,
        opacity: 1.,
        spacing: 0.075,
        tip: PaintBrushTip {
            hardness: 1.,
            roundness: 1.,
            flow: 1.,
            pressure_size: 1,
            pressure_flow: 1,
            ..dto!(PaintBrushTip)
        },
        ..dto!(PaintBrushSettings)
    }
}
#[test]
fn catalog_dtos_lifecycle_revision_and_text_buffers() {
    assert_eq!(size_of::<PaintBrushSettings>(), 64);
    assert_eq!(offset_of!(PaintBrushSettings, tip), 24);
    assert_eq!(size_of::<PaintBrushPresetInfo>(), 64);
    assert_eq!(size_of::<PaintBrushLibraryInfo>(), 32);
    assert_eq!(size_of::<PaintBrushToolDesc>(), 56);
    assert_eq!(offset_of!(PaintBrushToolDesc, linear_rgba), 24);
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let publication = info(library);
        assert_eq!(publication.preset_count, 4);
        assert_eq!(publication.selected_index, 0);
        let mut preset = dto!(PaintBrushPresetInfo);
        assert_eq!(
            paint_brush_library_preset_info(library, publication.revision, 0, &mut preset),
            PAINT_OK
        );
        assert_eq!(preset.diameter, 24.);
        assert_eq!(preset.builtin, 1);
        let mut required = 0;
        assert_eq!(
            paint_brush_library_text(
                library,
                publication.revision,
                0,
                1,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        let mut text = vec![0; required as usize];
        assert_eq!(
            paint_brush_library_text(
                library,
                publication.revision,
                0,
                1,
                text.as_mut_ptr(),
                text.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        assert_eq!(String::from_utf8(text).unwrap(), "压感圆笔");
        let id = b"round-fine";
        assert_eq!(
            paint_brush_library_select(library, id.as_ptr(), id.len() as u64),
            PAINT_OK
        );
        assert_eq!(
            paint_brush_library_preset_info(library, publication.revision, 0, &mut preset),
            PAINT_BUSY
        );
        assert_eq!(preset.diameter, 0.);
        assert_eq!(info(library).selected_index, 1);
        let stale = library;
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
        assert!(library.is_null());
        assert_eq!(
            paint_brush_library_info(stale, &mut dto!(PaintBrushLibraryInfo)),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(
            paint_brush_library_destroy(&mut library),
            PAINT_INVALID_HANDLE
        );
    }
}
#[test]
fn catalog_atomic_validation_copy_rename_remove_and_core_independence() {
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let original = info(library);
        let id = b"round-pressure";
        let mut bad = settings();
        bad.tip.roundness = 0.;
        assert_eq!(
            paint_brush_library_edit(library, id.as_ptr(), id.len() as u64, &bad),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(info(library).revision, original.revision);
        bad = settings();
        bad.tip.pressure_size = 2;
        assert_eq!(
            paint_brush_library_edit(library, id.as_ptr(), id.len() as u64, &bad),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(
            paint_brush_library_remove(library, id.as_ptr(), id.len() as u64),
            PAINT_INVALID_ARGUMENT
        );
        let copy = b"12345678-1234-4234-8234-123456789abc";
        let name = b"Copy";
        assert_eq!(
            paint_brush_library_copy(
                library,
                copy.as_ptr(),
                copy.len() as u64,
                name.as_ptr(),
                name.len() as u64
            ),
            PAINT_OK
        );
        assert_eq!(info(library).preset_count, 5);
        assert_eq!(info(library).selected_index, 4);
        let mut settings = settings();
        settings.tip.roundness = 0.2;
        settings.tip.angle = 90.;
        assert_eq!(
            paint_brush_library_edit(library, copy.as_ptr(), copy.len() as u64, &settings),
            PAINT_OK
        );
        assert_eq!(
            paint_brush_library_rename(
                library,
                copy.as_ptr(),
                copy.len() as u64,
                b"New".as_ptr(),
                3
            ),
            PAINT_OK
        );
        let mut preset = dto!(PaintBrushPresetInfo);
        assert_eq!(
            paint_brush_library_preset_info(library, info(library).revision, 4, &mut preset),
            PAINT_OK
        );
        assert_eq!(preset.builtin, 0);
        assert_eq!(preset.tip.roundness, 0.2);
        let mut core = ptr::null_mut();
        assert_eq!(paint_core_create(&mut core), PAINT_OK);
        assert_eq!(paint_core_destroy(&mut core), PAINT_OK);
        assert_eq!(info(library).preset_count, 5);
        assert_eq!(
            paint_brush_library_remove(library, copy.as_ptr(), copy.len() as u64),
            PAINT_OK
        );
        assert_eq!(info(library).selected_index, 0);
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
}
