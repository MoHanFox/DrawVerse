use super::*;
use paint_core::{
    BrushCatalogItem, DualBrush, DualColor, DualMode, SecondaryTip, WetMix, WetSettings,
};

#[test]
fn groups_native_images_source_capture_mutation_cold_rebuild_and_failed_drafts() {
    for enabled in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let library = PreparedCatalog::new(BrushCatalog::default());
        library.bind_resources(tmp.path().join("assets")).unwrap();
        let store = library.resource_store().unwrap();
        let token = paint_task::CancellationToken::default();
        let original = Arc::new(super::abr_fixture::fixture(6, 2, false));
        let imported = paint_io::abr_import::ImportBatch::decode(
            original.clone(),
            paint_core::MAX_CATALOG_MASK_BYTES,
            &token,
        )
        .unwrap()
        .persist(&store, &token)
        .unwrap();
        let source_id = imported[0].source_id().to_owned();
        let mut settings = imported[0].settings().clone();
        let mask = settings.mask.clone().unwrap();
        settings.wet =
            WetSettings::new(enabled, false, false, true, WetMix::default()).with_dual_color(true);
        settings.dual = DualBrush::new(
            enabled,
            DualMode::Multiply,
            SecondaryTip::new(8., settings.tip, Some(mask.clone()), Default::default()).unwrap(),
        )
        .with_color(
            DualColor::new(
                enabled,
                true,
                true,
                Default::default(),
                Default::default(),
                Default::default(),
            )
            .unwrap(),
        );
        settings.texture = paint_core::BrushTexture::new(
            enabled,
            Default::default(),
            Some(Arc::new(paint_core::TextureResource::new(mask))),
        )
        .unwrap();
        let group = "12345678-1234-4234-8234-000000000100";
        let empty = "12345678-1234-4234-8234-000000000101";
        let mut catalog = BrushCatalog::default();
        catalog.edit("round-pressure", settings).unwrap();
        catalog
            .set_source("round-pressure", Some(source_id.clone()))
            .unwrap();
        catalog
            .create_group(group.into(), "folder".into(), None, 0)
            .unwrap();
        catalog
            .create_group(empty.into(), "empty".into(), Some(group), 0)
            .unwrap();
        catalog
            .move_catalog_item(
                BrushCatalogItem::Preset("round-pressure".into()),
                Some(group),
                0,
            )
            .unwrap();
        let json = String::from_utf8(paint_io::brush_state::encode(&catalog).unwrap()).unwrap();
        let sequence = library.submit_groups(json.clone()).unwrap();
        let ticket = library.ticket(sequence).unwrap();
        let snapshot = ticket.wait().unwrap();
        assert_eq!(
            paint_io::brush_state::encode(&snapshot).unwrap(),
            json.as_bytes()
        );
        assert_eq!(ticket.source_receipts().unwrap().len(), 1);
        library
            .mutate(|c| {
                c.rename_group(group, "changed".into())?;
                Ok(())
            })
            .unwrap();
        assert_eq!(snapshot.organization().groups()[group].name(), "folder");
        assert_eq!(
            library.current().unwrap().organization().groups()[group].name(),
            "changed"
        );
        let before = paint_io::brush_state::encode(&library.current().unwrap()).unwrap();
        for bad in [
            json.replace("ordered-tree-v1", "unknown-tree"),
            json.replace("\"version\":20", "\"version\":19"),
            "{".into(),
        ] {
            assert_ne!(bad, json);
            let seq = library.submit_groups(bad).unwrap();
            assert!(library.ticket(seq).unwrap().wait().is_err());
            assert_eq!(
                paint_io::brush_state::encode(&library.current().unwrap()).unwrap(),
                before
            );
        }
        drop(snapshot);
        let flat =
            String::from_utf8(paint_io::brush_state::encode(&BrushCatalog::default()).unwrap())
                .unwrap();
        let seq = library.submit_wet_dual(flat).unwrap();
        library.ticket(seq).unwrap().wait().unwrap();
        drop(library.current().unwrap());
        ticket.retire_image().unwrap();
        assert!(ticket.state.lock().unwrap().result.is_none());
        let rebuilt = ticket.wait().unwrap();
        assert_eq!(
            paint_io::brush_state::encode(&rebuilt).unwrap(),
            json.as_bytes()
        );
        assert_eq!(
            rebuilt.selected().source_resource(),
            Some(source_id.as_str())
        );
        let batches = store
            .source_store()
            .unwrap()
            .export_batches(&std::collections::BTreeSet::from([source_id]), &token)
            .unwrap();
        assert_eq!(batches[0].original_bytes(), original.as_slice());
    }
}
