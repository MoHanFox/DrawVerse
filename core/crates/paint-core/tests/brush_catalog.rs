use paint_core::*;
use std::sync::Arc;
fn id(n: usize) -> String {
    format!("12345678-1234-4234-8234-{n:012x}")
}
#[test]
fn defaults_preserve_legacy_parameters_and_edit_keeps_captured_revision() {
    let mut library = BrushCatalog::default();
    assert_eq!(library.ids().len(), 4);
    for (id, radius, opacity, spacing) in [
        ("round-pressure", 12., 1., 0.15),
        ("round-fine", 3., 1., 0.15),
        ("round-translucent", 18., 0.25, 0.15),
        ("round-wide", 48., 1., 0.25),
    ] {
        let preset = library.preset(id).unwrap();
        let definition = preset.settings().definition();
        assert_eq!(definition.brush.radius, radius);
        assert_eq!(definition.brush.opacity, opacity);
        assert_eq!(definition.brush.spacing, spacing);
        assert!(preset.builtin());
    }
    let old = library.selected();
    let revision = library.revision();
    let mut settings = old.settings().clone();
    settings.tip.roundness = 0.2;
    settings.tip.angle = 90.;
    library.edit(old.id(), settings).unwrap();
    assert_eq!(library.revision(), revision + 1);
    assert!(old.program().coverage(7., 0., 12.) > 0.9);
    assert_eq!(library.selected().program().coverage(7., 0., 12.), 0.);
    assert_eq!(old.settings().tip.roundness, 1.);
    let revision = library.revision();
    let mut invalid = old.settings().clone();
    invalid.tip.flow = f32::NAN;
    assert!(library.edit(old.id(), invalid).is_err());
    assert_eq!(library.revision(), revision);
    assert_eq!(library.selected().settings().tip.roundness, 0.2);
}
#[test]
fn identity_batch_selection_copy_remove_and_failures_are_atomic() {
    let mut library = BrushCatalog::default();
    for bad in [
        "bad",
        "00000000-0000-0000-0000-000000000000",
        "round-pressure",
    ] {
        assert!(BrushPreset::new(bad.into(), "name".into(), BrushSettings::default()).is_err());
    }
    assert!(BrushPreset::new(id(1), " ".into(), BrushSettings::default()).is_err());
    assert!(library.remove("round-pressure").is_err());
    let before = library.ids().to_vec();
    let rev = library.revision();
    let batch = vec![
        BrushPreset::new(id(1), "A".into(), BrushSettings::default()).unwrap(),
        BrushPreset::new(id(1), "B".into(), BrushSettings::default()).unwrap(),
    ];
    assert!(library.import(batch).is_err());
    assert_eq!(library.ids(), before);
    assert_eq!(library.revision(), rev);
    assert_eq!(library.selected().id(), "round-pressure");
    library.save_copy(id(1), "New".into()).unwrap();
    assert_eq!(library.selected().id(), id(1));
    assert!(!library.selected().builtin());
    let snapshot = library.selected();
    library.rename(&id(1), "Renamed".into()).unwrap();
    assert_eq!(snapshot.name(), "New");
    assert_eq!(library.selected().name(), "Renamed");
    library.remove(&id(1)).unwrap();
    assert_eq!(library.selected().id(), "round-pressure");
    assert_eq!(snapshot.name(), "New");
    assert!(library.select("unknown").is_err());
}
#[test]
fn capacity_is_atomic_and_shared_mask_snapshots_survive_removal() {
    let mut library = BrushCatalog::default();
    let mask = Arc::new(BrushMask::new(8, 8, vec![255; 64]).unwrap());
    let settings = BrushSettings {
        mask: Some(mask.clone()),
        ..BrushSettings::default()
    };
    library
        .import(vec![
            BrushPreset::new(id(1), "Image".into(), settings).unwrap()
        ])
        .unwrap();
    let captured = library.selected();
    drop(mask);
    library.remove(&id(1)).unwrap();
    assert!(captured.program().coverage(0., 0., 12.) > 0.9);
    let mut batch = Vec::new();
    for i in 2..MAX_BRUSH_PRESETS - 2 {
        batch.push(BrushPreset::new(id(i), "Brush".into(), BrushSettings::default()).unwrap());
    }
    library.import(batch).unwrap();
    assert_eq!(library.ids().len(), MAX_BRUSH_PRESETS);
    let revision = library.revision();
    assert!(library
        .save_copy(id(MAX_BRUSH_PRESETS), "Too many".into())
        .is_err());
    assert_eq!(library.revision(), revision);
    assert_eq!(library.ids().len(), MAX_BRUSH_PRESETS);
}

#[test]
fn resource_budget_counts_shared_resources_once_and_rejects_import_and_edit_atomically() {
    let mut library = BrushCatalog::with_mask_budget(100).unwrap();
    let mask = Arc::new(BrushMask::new(8, 8, vec![255; 64]).unwrap());
    let settings = BrushSettings {
        mask: Some(mask),
        ..BrushSettings::default()
    };
    library
        .import(vec![
            BrushPreset::new(id(1), "A".into(), settings.clone()).unwrap(),
            BrushPreset::new(id(2), "B".into(), settings).unwrap(),
        ])
        .unwrap();
    let before = library.ids().to_vec();
    let revision = library.revision();
    let different = BrushSettings {
        mask: Some(Arc::new(BrushMask::new(8, 8, vec![128; 64]).unwrap())),
        ..BrushSettings::default()
    };
    assert!(library
        .import(vec![
            BrushPreset::new(id(3), "C".into(), different.clone()).unwrap()
        ])
        .is_err());
    assert!(library.edit(&id(1), different).is_err());
    assert_eq!(library.ids(), before);
    assert_eq!(library.revision(), revision);
    assert_eq!(
        library
            .preset(&id(1))
            .unwrap()
            .settings()
            .mask
            .as_ref()
            .unwrap()
            .coverage()[0],
        255
    );
    assert!(BrushCatalog::with_mask_budget(0).is_err());
    assert!(BrushCatalog::with_mask_budget(MAX_CATALOG_MASK_BYTES + 1).is_err());
}
