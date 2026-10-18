use super::*;
use paint_core::{BrushCatalogItem, BrushGroup, BrushOrganization, Document, InputPoint};
use paint_io::{vbr_handoff::PackageHandoff, vbr_image::ImageBundle};

#[test]
fn groups_package_native_capacity_sources_roundtrip_preparation_and_target_atomicity() {
    let token = paint_task::CancellationToken::default();
    for grouped in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let old = ResourceStore::open(tmp.path().join("old"), paint_core::MAX_CATALOG_MASK_BYTES)
            .unwrap();
        let original = Arc::new(super::abr_fixture::fixture(6, 2, false));
        let imported = paint_io::abr_import::ImportBatch::decode(
            original.clone(),
            paint_core::MAX_CATALOG_MASK_BYTES,
            &token,
        )
        .unwrap()
        .persist(&old, &token)
        .unwrap();
        let source = imported[0].source_id().to_owned();
        let batches = old
            .source_store()
            .unwrap()
            .export_batches(&std::collections::BTreeSet::from([source.clone()]), &token)
            .unwrap();
        let base = super::tests::texture_bundle(true, true);
        let mut package = base.package.clone();
        package.brushes[0].source_resource = Some(source.clone());
        let mut assets = base
            .assets()
            .iter()
            .map(|(id, a)| (id.clone(), a.bytes().to_vec()))
            .collect::<std::collections::BTreeMap<_, _>>();
        for batch in &batches {
            for id in batch.tip_ids() {
                assets.insert(id.into(), old.encoded(id).unwrap());
            }
        }
        let bundle = ImageBundle::with_sources(package, assets, batches, &token).unwrap();
        let bundle = if grouped {
            let group = "12345678-1234-4234-8234-000000000100";
            let empty = "12345678-1234-4234-8234-000000000101";
            bundle
                .with_organization(
                    BrushOrganization::new(
                        vec![BrushCatalogItem::Group(group.into())],
                        vec![
                            BrushGroup::new(
                                group.into(),
                                "same".into(),
                                vec![
                                    BrushCatalogItem::Preset(base.package.brushes[0].id.clone()),
                                    BrushCatalogItem::Group(empty.into()),
                                ],
                            )
                            .unwrap(),
                            BrushGroup::new(empty.into(), "same".into(), vec![]).unwrap(),
                        ],
                    )
                    .unwrap(),
                )
                .unwrap()
        } else {
            bundle
        };
        let path = tmp.path().join("input.vbr");
        paint_io::vbr_image::save(&path, &bundle, &token).unwrap();
        let path_bytes = path.to_str().unwrap().as_bytes();
        let directory = tmp.path().join("fresh");
        let directory_bytes = directory.to_str().unwrap().as_bytes();
        unsafe {
            let mut handle = std::ptr::null_mut();
            assert_eq!(crate::paint_brush_library_create(0, &mut handle), PAINT_OK);
            assert_eq!(
                crate::paint_brush_library_bind_resources(
                    handle,
                    directory_bytes.as_ptr(),
                    directory_bytes.len() as u64
                ),
                PAINT_OK
            );
            let library = crate::runtime::registry()
                .unwrap()
                .brush_library(handle)
                .unwrap();
            let before = paint_io::brush_state::encode(&library.current().unwrap()).unwrap();
            let mut needed = 99;
            assert_eq!(
                crate::paint_brush_library_package_read_groups(
                    handle,
                    path_bytes.as_ptr(),
                    path_bytes.len() as u64,
                    std::ptr::null_mut(),
                    0,
                    &mut needed
                ),
                PAINT_BUFFER_TOO_SMALL
            );
            assert!(needed > 0);
            assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
            let mut short = vec![0xa5; needed as usize - 1];
            assert_eq!(
                crate::paint_brush_library_package_read_groups(
                    handle,
                    path_bytes.as_ptr(),
                    path_bytes.len() as u64,
                    short.as_mut_ptr(),
                    short.len() as u64,
                    &mut needed
                ),
                PAINT_BUFFER_TOO_SMALL
            );
            assert!(short.iter().all(|v| *v == 0xa5));
            assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
            let mut json = vec![0; needed as usize];
            assert_eq!(
                crate::paint_brush_library_package_read_groups(
                    handle,
                    path_bytes.as_ptr(),
                    path_bytes.len() as u64,
                    json.as_mut_ptr(),
                    json.len() as u64,
                    &mut needed
                ),
                PAINT_OK
            );
            let handoff = PackageHandoff::from_json(&json).unwrap();
            assert_eq!(
                json,
                PackageHandoff::from_bundle(&bundle)
                    .unwrap()
                    .to_json()
                    .unwrap()
            );
            assert_eq!(
                paint_io::brush_state::encode(&library.current().unwrap()).unwrap(),
                before
            );
            for (id, asset) in bundle.assets() {
                assert_eq!(
                    library.resource_store().unwrap().encoded(id).unwrap(),
                    asset.bytes()
                );
            }
            let target = tmp.path().join("output.vbr");
            let target_bytes = target.to_str().unwrap().as_bytes();
            std::fs::write(&target, b"keep").unwrap();
            let flat = bundle.package.to_json().unwrap();
            assert_ne!(
                crate::paint_brush_library_package_write_groups(
                    handle,
                    target_bytes.as_ptr(),
                    target_bytes.len() as u64,
                    flat.as_ptr(),
                    flat.len() as u64
                ),
                PAINT_OK
            );
            assert_eq!(std::fs::read(&target).unwrap(), b"keep");
            assert_eq!(
                crate::paint_brush_library_package_write_groups(
                    handle,
                    target_bytes.as_ptr(),
                    target_bytes.len() as u64,
                    json.as_ptr(),
                    json.len() as u64
                ),
                PAINT_OK
            );
            let reopened = paint_io::vbr_image::load(&target, &token).unwrap();
            assert_eq!(reopened.organization(), bundle.organization());
            assert_eq!(
                reopened.package.to_json().unwrap(),
                bundle.package.to_json().unwrap()
            );
            assert_eq!(reopened.sources()[0].original_bytes(), original.as_slice());
            for ((id, a), (other, b)) in bundle.sources()[0]
                .records()
                .zip(reopened.sources()[0].records())
            {
                assert_eq!(id, other);
                assert_eq!(a, b);
            }
            let draw = |definition| {
                let mut d = Document::new(64, 64).unwrap();
                d.begin_brush_stroke(definition, InputPoint::new(10., 20., 1.))
                    .unwrap();
                d.stroke_to(InputPoint::new(40., 25., 0.5)).unwrap();
                d.end_stroke().unwrap();
                d
            };
            let a = draw(bundle.definition(0).unwrap());
            let mut b = draw(reopened.definition(0).unwrap());
            b.undo().unwrap();
            b.redo().unwrap();
            for y in 0..64 {
                for x in 0..64 {
                    assert_eq!(a.pixel(x, y), b.pixel(x, y));
                }
            }
            let mut catalog = BrushCatalog::default();
            reopened
                .import_into_catalog(&mut catalog, None, 0, &token)
                .unwrap();
            let state = paint_io::brush_state::encode(&catalog).unwrap();
            let mut seq = 0;
            assert_eq!(
                crate::paint_brush_library_prepare_state_groups(
                    handle,
                    state.as_ptr(),
                    state.len() as u64,
                    &mut seq
                ),
                PAINT_OK
            );
            let snapshot = library.ticket(seq).unwrap().wait().unwrap();
            assert_eq!(paint_io::brush_state::encode(&snapshot).unwrap(), state);
            assert_eq!(snapshot.selected().source_resource(), Some(source.as_str()));
            assert_eq!(
                handoff.organization().groups().len(),
                if grouped { 2 } else { 0 }
            );
            assert_eq!(crate::paint_brush_library_destroy(&mut handle), PAINT_OK);
        }
    }
}
