//! Licensed, unmodified externally authored assets; not generated wire fixtures.
use paint_core::{Document, InputPoint, MAX_CATALOG_MASK_BYTES};
use paint_io::{
    abr_association, abr_conversion,
    abr_import::ImportBatch,
    abr_sampled,
    brush_resource::{hash, ResourceStore},
};
use paint_task::CancellationToken;
use std::sync::Arc;
const ASSETS: [(&[u8], &str, usize); 3] = [
    (
        include_bytes!("fixtures/abr/myer-settlement-brushes.abr"),
        "4cb12679582ee908ed8f546d7b45071a2de48fb9af194e649a227f259838f1d6",
        148,
    ),
    (
        include_bytes!("fixtures/abr/photoshop-dynamics.abr"),
        "ea61c8d0485824265d4108f74da78c12fee19f40a3d64946df5c3131e55ae497",
        1,
    ),
    (
        include_bytes!("fixtures/abr/photoshop-dual-brush.abr"),
        "5fb19ed01bbd415841f1e505e9b2472bf547b2e971da0eb196cf534f7d784d69",
        1,
    ),
];
#[test]
fn licensed_real_packs_parse_associate_import_and_preserve_original_sources() {
    let token = CancellationToken::default();
    for (bytes, expected, count) in ASSETS {
        assert_eq!(hash(bytes), expected);
        let parsed = abr_sampled::decode_sectioned(bytes, &token).unwrap();
        assert_eq!(parsed.tips.len(), count);
        let descriptors = parsed.descriptors(&token).unwrap();
        let linked = abr_association::associate(&parsed, &descriptors, &token).unwrap();
        assert_eq!(linked.len(), count);
        let converted = abr_conversion::convert(&linked, &token).unwrap();
        assert_eq!(converted.len(), count);
        println!(
            "{} presets; first {} diameter {}",
            count, converted[0].name, converted[0].settings.diameter
        );
        let batch =
            ImportBatch::decode(Arc::new(bytes.to_vec()), MAX_CATALOG_MASK_BYTES, &token).unwrap();
        assert_eq!(batch.presets().len(), count);
        let temp = tempfile::tempdir().unwrap();
        let store = ResourceStore::open(temp.path().into(), MAX_CATALOG_MASK_BYTES).unwrap();
        let imported = batch.persist(&store, &token).unwrap();
        assert_eq!(imported.len(), count);
        for (index, preset) in imported.iter().enumerate() {
            assert_eq!(preset.name(), linked[index].name);
            assert_eq!(
                preset.settings().mask.as_ref().unwrap().coverage(),
                linked[index].sampled_tip.unwrap().mask.coverage()
            );
            let source = store
                .source_store()
                .unwrap()
                .load(preset.source_id(), &token)
                .unwrap();
            assert_eq!(source.original(), bytes);
            assert_eq!(source.info.preset_index, index as u32);
            assert_eq!(source.info.original_hash, expected);
        }
        for index in [0, count / 2, count - 1] {
            let settings = imported[index].settings();
            let mut doc = Document::new(1024, 1024).unwrap();
            doc.begin_brush_stroke(settings.definition(), InputPoint::new(512.5, 512.5, 1.))
                .unwrap();
            doc.end_stroke().unwrap();
            let after = (256..768)
                .flat_map(|y| (256..768).map(move |x| (x, y)))
                .map(|(x, y)| doc.pixel(x, y))
                .collect::<Vec<_>>();
            assert!(after.iter().any(|p| p.alpha() > 0.));
            assert!(after.iter().any(|p| p.alpha() == 0.));
            assert_eq!(doc.history_depth(), (1, 0));
            doc.undo().unwrap();
            assert!((256..768).all(|y| (256..768).all(|x| doc.pixel(x, y).alpha() == 0.)));
            doc.redo().unwrap();
            assert_eq!(
                (256..768)
                    .flat_map(|y| (256..768).map(move |x| (x, y)))
                    .map(|(x, y)| doc.pixel(x, y))
                    .collect::<Vec<_>>(),
                after
            );
        }
    }
}
