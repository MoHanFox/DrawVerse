use super::tests::texture_bundle;
use super::*;
#[test]
fn old_native_package_entries_reject_grouped_vbr24_before_resource_install() {
    use paint_core::{BrushCatalogItem, BrushGroup, BrushOrganization};
    type Reader = unsafe extern "C" fn(
        *mut PaintBrushLibrary,
        *const u8,
        u64,
        *mut u8,
        u64,
        *mut u64,
    ) -> PaintStatus;
    type Writer =
        unsafe extern "C" fn(*mut PaintBrushLibrary, *const u8, u64, *const u8, u64) -> PaintStatus;
    let readers: [Reader; 4] = [
        crate::paint_brush_library_package_read,
        crate::paint_brush_library_package_read_wet,
        crate::paint_brush_library_package_read_taper,
        crate::paint_brush_library_package_read_wet_dual,
    ];
    let writers: [Writer; 4] = [
        crate::paint_brush_library_package_write,
        crate::paint_brush_library_package_write_wet,
        crate::paint_brush_library_package_write_taper,
        crate::paint_brush_library_package_write_wet_dual,
    ];
    for empty in [false, true] {
        let token = paint_task::CancellationToken::default();
        let bundle = texture_bundle(false, false);
        let preset = bundle.package.brushes[0].id.clone();
        let group = "12345678-1234-4234-8234-000000000100";
        let roots = if empty {
            vec![
                BrushCatalogItem::Group(group.into()),
                BrushCatalogItem::Preset(preset.clone()),
            ]
        } else {
            vec![BrushCatalogItem::Group(group.into())]
        };
        let children = if empty {
            vec![]
        } else {
            vec![BrushCatalogItem::Preset(preset)]
        };
        let bundle = bundle
            .with_organization(
                BrushOrganization::new(
                    roots,
                    vec![BrushGroup::new(group.into(), "group".into(), children).unwrap()],
                )
                .unwrap(),
            )
            .unwrap();
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("grouped.vbr");
        paint_io::vbr_image::save(&path, &bundle, &token).unwrap();
        let original = std::fs::read(&path).unwrap();
        let path = path.to_str().unwrap().as_bytes();
        let assets = temp.path().join("fresh");
        let directory = assets.to_str().unwrap().as_bytes();
        unsafe {
            let mut handle = std::ptr::null_mut();
            assert_eq!(crate::paint_brush_library_create(0, &mut handle), PAINT_OK);
            assert_eq!(
                crate::paint_brush_library_bind_resources(
                    handle,
                    directory.as_ptr(),
                    directory.len() as u64
                ),
                PAINT_OK
            );
            for reader in readers {
                for size in [0, 65536] {
                    let mut out = vec![0xa5; size];
                    let mut required = 999;
                    assert_eq!(
                        reader(
                            handle,
                            path.as_ptr(),
                            path.len() as u64,
                            if size == 0 {
                                std::ptr::null_mut()
                            } else {
                                out.as_mut_ptr()
                            },
                            size as u64,
                            &mut required
                        ),
                        PAINT_UNSUPPORTED
                    );
                    assert_eq!(required, 0);
                    assert!(out.iter().all(|v| *v == 0xa5));
                    assert_eq!(std::fs::read_dir(&assets).unwrap().count(), 0);
                }
            }
            let mut json = bundle.package.to_json().unwrap();
            assert_eq!(json.pop(), Some(b'}'));
            json.extend_from_slice(br#","organization":null}"#);
            for writer in writers {
                assert_ne!(
                    writer(
                        handle,
                        path.as_ptr(),
                        path.len() as u64,
                        json.as_ptr(),
                        json.len() as u64
                    ),
                    PAINT_OK
                );
                assert_eq!(
                    std::fs::read(std::str::from_utf8(path).unwrap()).unwrap(),
                    original
                );
            }
            assert_eq!(crate::paint_brush_library_destroy(&mut handle), PAINT_OK);
        }
    }
}
#[test]
fn old_native_entries_reject_grouped_state20_without_adoption() {
    for populated in [false, true] {
        let mut c = BrushCatalog::default();
        let id = "12345678-1234-4234-8234-000000000001";
        c.create_group(id.into(), "empty or populated".into(), None, 0)
            .unwrap();
        if populated {
            c.move_catalog_item(
                paint_core::BrushCatalogItem::Preset("round-pressure".into()),
                Some(id),
                0,
            )
            .unwrap();
        }
        let json = String::from_utf8(paint_io::brush_state::encode(&c).unwrap()).unwrap();
        let library = PreparedCatalog::new(BrushCatalog::default());
        let before = paint_io::brush_state::encode(&library.current().unwrap()).unwrap();
        for level in 0..4 {
            let seq = match level {
                0 => library.submit(json.clone()),
                1 => library.submit_wet(json.clone()),
                2 => library.submit_taper(json.clone()),
                _ => library.submit_wet_dual(json.clone()),
            }
            .unwrap();
            assert_eq!(
                library.ticket(seq).unwrap().wait().unwrap_err().status,
                PAINT_UNSUPPORTED
            );
            assert_eq!(
                paint_io::brush_state::encode(&library.current().unwrap()).unwrap(),
                before
            );
        }
    }
}
#[test]
fn old_native_entries_reject_wet_dual_policy_including_closed_without_publication_or_resource_install(
) {
    use paint_core::{DualBrush, DualColor, DualMode, SecondaryTip, WetMix, WetSettings};
    for enabled in [false, true] {
        let wet =
            WetSettings::new(enabled, false, false, true, WetMix::default()).with_dual_color(true);
        let mut catalog = BrushCatalog::default();
        let mut settings = catalog.selected().settings().clone();
        settings.wet = wet;
        settings.dual = DualBrush::new(true, DualMode::Multiply, SecondaryTip::default())
            .with_color(
                DualColor::new(
                    true,
                    true,
                    true,
                    Default::default(),
                    Default::default(),
                    Default::default(),
                )
                .unwrap(),
            );
        catalog.edit("round-pressure", settings).unwrap();
        let library = PreparedCatalog::new(BrushCatalog::default());
        let before = paint_io::brush_state::encode(&library.current().unwrap()).unwrap();
        let json = String::from_utf8(paint_io::brush_state::encode(&catalog).unwrap()).unwrap();
        for level in 0..3 {
            let seq = match level {
                0 => library.submit(json.clone()),
                1 => library.submit_wet(json.clone()),
                _ => library.submit_taper(json.clone()),
            }
            .unwrap();
            assert_eq!(
                library.ticket(seq).unwrap().wait().unwrap_err().status,
                PAINT_UNSUPPORTED
            );
            assert_eq!(
                paint_io::brush_state::encode(&library.current().unwrap()).unwrap(),
                before
            );
        }
        let token = paint_task::CancellationToken::default();
        let fixture = texture_bundle(false, false);
        let mut package = fixture.package.clone();
        package.brushes[0].wet_mix = Some(paint_io::brush_wet::WetParameters::from_core(wet));
        package.brushes[0].behavior_version = 16;
        package.brushes[0]
            .required_capabilities
            .extend(["wet-mix".into(), "wet-dual-color".into()]);
        let assets = fixture
            .assets()
            .iter()
            .map(|(id, a)| (id.clone(), a.bytes().to_vec()))
            .collect();
        let bundle = paint_io::vbr_image::ImageBundle::new(package, assets, &token).unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("wet-dual.vbr");
        paint_io::vbr_image::save(&path, &bundle, &token).unwrap();
        let original = std::fs::read(&path).unwrap();
        let path = path.to_str().unwrap().as_bytes();
        let json = bundle.package.to_json().unwrap();
        let resources = tmp.path().join("fresh");
        let directory = resources.to_str().unwrap().as_bytes();
        type Reader = unsafe extern "C" fn(
            *mut PaintBrushLibrary,
            *const u8,
            u64,
            *mut u8,
            u64,
            *mut u64,
        ) -> PaintStatus;
        type Writer = unsafe extern "C" fn(
            *mut PaintBrushLibrary,
            *const u8,
            u64,
            *const u8,
            u64,
        ) -> PaintStatus;
        let readers: [Reader; 3] = [
            crate::paint_brush_library_package_read,
            crate::paint_brush_library_package_read_wet,
            crate::paint_brush_library_package_read_taper,
        ];
        let writers: [Writer; 3] = [
            crate::paint_brush_library_package_write,
            crate::paint_brush_library_package_write_wet,
            crate::paint_brush_library_package_write_taper,
        ];
        unsafe {
            let mut handle = std::ptr::null_mut();
            assert_eq!(crate::paint_brush_library_create(0, &mut handle), PAINT_OK);
            assert_eq!(
                crate::paint_brush_library_bind_resources(
                    handle,
                    directory.as_ptr(),
                    directory.len() as u64
                ),
                PAINT_OK
            );
            for reader in readers {
                for capacity in [0, 65536] {
                    let mut output = vec![0; capacity];
                    let mut required = 99;
                    assert_eq!(
                        reader(
                            handle,
                            path.as_ptr(),
                            path.len() as u64,
                            if capacity == 0 {
                                std::ptr::null_mut()
                            } else {
                                output.as_mut_ptr()
                            },
                            capacity as u64,
                            &mut required
                        ),
                        PAINT_UNSUPPORTED
                    );
                    assert_eq!(required, 0);
                }
            }
            for writer in writers {
                assert_eq!(
                    writer(
                        handle,
                        path.as_ptr(),
                        path.len() as u64,
                        json.as_ptr(),
                        json.len() as u64
                    ),
                    PAINT_UNSUPPORTED
                );
                assert_eq!(
                    std::fs::read(std::str::from_utf8(path).unwrap()).unwrap(),
                    original
                );
            }
            assert_eq!(
                crate::paint_brush_package_write(
                    path.as_ptr(),
                    path.len() as u64,
                    json.as_ptr(),
                    json.len() as u64
                ),
                PAINT_UNSUPPORTED
            );
            assert_eq!(crate::paint_brush_library_destroy(&mut handle), PAINT_OK);
        }
        for id in bundle.assets().keys() {
            assert!(!resources.join(format!("{id}.png")).exists());
        }
        assert_eq!(
            std::fs::read(std::str::from_utf8(path).unwrap()).unwrap(),
            original
        );
    }
}
