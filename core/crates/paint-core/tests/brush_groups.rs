use paint_core::*;
use std::sync::Arc;
fn id(n: usize) -> String {
    format!("12345678-1234-4234-8234-{n:012x}")
}
fn p(id: &str) -> BrushCatalogItem {
    BrushCatalogItem::Preset(id.into())
}
fn g(n: usize) -> BrushCatalogItem {
    BrushCatalogItem::Group(id(n))
}
fn preset(n: usize) -> BrushPreset {
    BrushPreset::new(id(n), "same name".into(), BrushSettings::default()).unwrap()
}
fn unchanged(a: &BrushCatalog, b: &BrushCatalog) {
    assert_eq!(a.revision(), b.revision());
    assert_eq!(a.ids(), b.ids());
    assert_eq!(a.selected().id(), b.selected().id());
    assert!(Arc::ptr_eq(&a.organization(), &b.organization()));
    for id in a.ids() {
        assert!(Arc::ptr_eq(&a.preset(id).unwrap(), &b.preset(id).unwrap()));
    }
}
macro_rules! reject {
    ($catalog:ident,$operation:expr) => {{
        let before = $catalog.clone();
        assert!($operation.is_err());
        unchanged(&$catalog, &before);
    }};
}
#[test]
fn groups_default_dfs_snapshots_noops_and_ungroup_preserve_compiled_identity() {
    let mut c = BrushCatalog::default();
    let initial = c.ids().to_vec();
    let original = c.selected();
    assert!(c.organization().groups().is_empty());
    assert_eq!(c.organization().preset_order(), c.ids());
    c.create_group(id(1), " folder ".into(), None, 1).unwrap();
    c.create_group(id(2), "folder".into(), Some(&id(1)), 0)
        .unwrap();
    c.move_catalog_item(p("round-fine"), Some(&id(2)), 0)
        .unwrap();
    assert_eq!(c.ids(), initial);
    assert!(Arc::ptr_eq(&original, &c.selected()));
    let old = c.organization();
    let revision = c.revision();
    c.rename_group(&id(2), " folder ".into()).unwrap();
    assert_eq!(c.revision(), revision);
    assert!(Arc::ptr_eq(&old, &c.organization()));
    c.remove_group(&id(1), false).unwrap();
    assert_eq!(c.ids(), initial);
    assert!(old.groups().contains_key(&id(1)));
    assert!(!c.organization().groups().contains_key(&id(1)));
    let before = c.organization();
    let revision = c.revision();
    c.move_catalog_item(g(2), None, 1).unwrap();
    assert_eq!(c.revision(), revision);
    assert!(Arc::ptr_eq(&before, &c.organization()));
    c.rename_group(&id(2), "new name".into()).unwrap();
    assert_eq!(c.revision(), revision + 1);
    assert_eq!(old.groups()[&id(2)].name(), "folder");
    assert!(Arc::ptr_eq(&original, &c.selected()));
    c.move_catalog_item(g(2), None, 3).unwrap();
    assert_eq!(
        c.ids(),
        &[
            "round-pressure",
            "round-translucent",
            "round-wide",
            "round-fine"
        ]
    );
    c.move_catalog_item(p("round-fine"), None, 1).unwrap();
    assert_eq!(c.ids(), initial);
    assert!(c.organization().groups()[&id(2)].items().is_empty());
}
#[test]
fn groups_cycles_positions_membership_and_identity_fail_atomically() {
    let mut c = BrushCatalog::default();
    c.create_group(id(1), "A".into(), None, 0).unwrap();
    c.create_group(id(2), "B".into(), Some(&id(1)), 0).unwrap();
    reject!(c, c.move_catalog_item(g(1), Some(&id(2)), 0));
    reject!(c, c.move_catalog_item(g(1), Some(&id(1)), 0));
    reject!(c, c.move_catalog_item(p("round-pressure"), Some(&id(9)), 0));
    reject!(c, c.move_catalog_item(g(2), None, 999));
    reject!(c, c.move_catalog_item(p(&id(99)), None, 0));
    reject!(c, c.create_group(id(1), "again".into(), None, 0));
    reject!(c, c.rename_group(&id(9), "missing".into()));
    reject!(c, c.remove_group(&id(9), false));
    reject!(
        c,
        c.create_group("round-pressure".into(), "bad".into(), None, 0)
    );
    reject!(c, c.create_group(id(9), "bad".into(), Some(&id(99)), 0));
    reject!(
        c,
        c.set_organization(BrushOrganization::new(vec![p("round-pressure")], vec![]).unwrap())
    );
    let mut roots = c.organization().roots().to_vec();
    roots.push(p(&id(99)));
    let layout =
        BrushOrganization::new(roots, c.organization().groups().values().cloned().collect())
            .unwrap();
    reject!(c, c.set_organization(layout));
}
#[test]
fn groups_raw_tree_rejects_aliases_orphans_unknown_ids_cycles_and_excess_entries() {
    let empty = || BrushGroup::new(id(1), "A".into(), vec![]).unwrap();
    assert!(
        BrushOrganization::new(vec![p("round-pressure"), p("round-pressure")], vec![]).is_err()
    );
    assert!(BrushOrganization::new(vec![g(1), g(1)], vec![empty()]).is_err());
    assert!(BrushOrganization::new(vec![g(1)], vec![empty(), empty()]).is_err());
    assert!(BrushOrganization::new(vec![], vec![empty()]).is_err());
    assert!(BrushOrganization::new(vec![g(99)], vec![]).is_err());
    assert!(BrushOrganization::new(
        vec![g(1)],
        vec![BrushGroup::new(id(1), "A".into(), vec![g(1)]).unwrap()]
    )
    .is_err());
    assert!(BrushOrganization::new(
        vec![g(1)],
        vec![
            BrushGroup::new(id(1), "A".into(), vec![g(2)]).unwrap(),
            BrushGroup::new(id(2), "B".into(), vec![g(1)]).unwrap()
        ]
    )
    .is_err());
    assert!(BrushOrganization::new(vec![g(1), p(&id(1))], vec![empty()]).is_err());
    assert!(BrushOrganization::new(vec![p("bad")], vec![]).is_err());
    assert!(BrushOrganization::new(vec![p("round-pressure"); 1281], vec![]).is_err());
    assert!(BrushGroup::new(id(1), "name".into(), vec![p("round-pressure"); 1281]).is_err());
    for value in [
        "",
        "not-a-uuid",
        "00000000-0000-0000-0000-000000000000",
        "12345678-1234-4234-8234-00000000000A",
    ] {
        assert!(BrushGroup::new(value.into(), "name".into(), vec![]).is_err());
    }
    for name in [" ".into(), "x\0y".into(), "笔".repeat(81)] {
        assert!(BrushGroup::new(id(1), name, vec![]).is_err());
    }
    assert_eq!(
        BrushGroup::new(id(1), "笔".repeat(80), vec![])
            .unwrap()
            .name()
            .chars()
            .count(),
        80
    );
}
#[test]
fn groups_depth_and_full_catalog_counts_accept_boundaries_and_reject_overflow() {
    let mut c = BrushCatalog::default();
    for n in 1..=MAX_BRUSH_GROUP_DEPTH {
        let parent = (n > 1).then(|| id(n - 1));
        c.create_group(id(n), "same".into(), parent.as_deref(), 0)
            .unwrap();
    }
    c.move_catalog_item(p("round-pressure"), Some(&id(8)), 0)
        .unwrap();
    assert_eq!(c.ids()[0], "round-pressure");
    reject!(c, c.create_group(id(9), "too deep".into(), Some(&id(8)), 0));
    let mut c = BrushCatalog::default();
    for n in 1..=MAX_BRUSH_GROUPS {
        c.create_group(id(10000 + n), "empty".into(), None, n - 1)
            .unwrap();
    }
    c.import((1..=MAX_BRUSH_PRESETS - 4).map(preset).collect())
        .unwrap();
    assert_eq!(c.ids().len(), 1024);
    assert_eq!(c.organization().roots().len(), 1280);
    reject!(c, c.create_group(id(20000), "overflow".into(), None, 0));
    reject!(c, c.import(vec![preset(20001)]));
}
#[test]
fn groups_import_copy_remove_and_resource_failures_keep_membership_selection_and_sources() {
    let mut c = BrushCatalog::default();
    c.create_group(id(100), "A".into(), None, 0).unwrap();
    let source = "a".repeat(64);
    let value = preset(1).with_source(Some(source.clone())).unwrap();
    let before = c.revision();
    c.import_into_group(vec![value, preset(2)], Some(&id(100)))
        .unwrap();
    assert_eq!(c.revision(), before + 1);
    assert_eq!(
        c.organization().groups()[&id(100)].items(),
        &[p(&id(1)), p(&id(2))]
    );
    let original = c.selected();
    let rev = c.revision();
    c.save_copy(id(3), "copy".into()).unwrap();
    assert_eq!(c.revision(), rev + 1);
    assert_eq!(
        c.organization().groups()[&id(100)].items(),
        &[p(&id(1)), p(&id(2)), p(&id(3))]
    );
    assert_eq!(c.selected().source_resource(), Some(source.as_str()));
    assert!(Arc::ptr_eq(&original, &c.preset(&id(1)).unwrap()));
    reject!(c, c.import_into_group(vec![preset(4)], Some(&id(999))));
    reject!(
        c,
        c.import_into_group(vec![preset(4), preset(4)], Some(&id(100)))
    );
    reject!(c, c.import(vec![preset(100)]));
    c.remove(&id(3)).unwrap();
    assert_eq!(c.selected().id(), "round-pressure");
    c.remove(&id(1)).unwrap();
    c.remove(&id(2)).unwrap();
    assert!(c.organization().groups()[&id(100)].items().is_empty());
    let mut tiny = BrushCatalog::with_mask_budget(1).unwrap();
    tiny.create_group(id(100), "empty".into(), None, 0).unwrap();
    let settings = BrushSettings {
        mask: Some(Arc::new(BrushMask::new(2, 2, vec![255; 4]).unwrap())),
        ..Default::default()
    };
    reject!(
        tiny,
        tiny.import_into_group(
            vec![BrushPreset::new(id(1), "large".into(), settings).unwrap()],
            Some(&id(100))
        )
    );
}
#[test]
fn groups_cascade_builtin_protection_is_atomic_and_user_subtree_deletion_keeps_capture() {
    let mut c = BrushCatalog::default();
    c.create_group(id(100), "outer".into(), None, 0).unwrap();
    c.create_group(id(101), "inner".into(), Some(&id(100)), 0)
        .unwrap();
    let settings = BrushSettings {
        mask: Some(Arc::new(
            BrushMask::new(2, 2, vec![255, 128, 0, 255]).unwrap(),
        )),
        ..Default::default()
    };
    c.import_into_group(
        vec![BrushPreset::new(id(1), "active".into(), settings).unwrap()],
        Some(&id(101)),
    )
    .unwrap();
    let captured = c.selected();
    c.move_catalog_item(p("round-pressure"), Some(&id(100)), 1)
        .unwrap();
    reject!(c, c.remove_group(&id(100), true));
    c.move_catalog_item(p("round-pressure"), None, 0).unwrap();
    let mut actual = Document::new(128, 64).unwrap();
    let mut reference = Document::new(128, 64).unwrap();
    for d in [&mut actual, &mut reference] {
        d.begin_brush_stroke(
            captured.settings().definition(),
            InputPoint::new(20.5, 32.5, 1.),
        )
        .unwrap();
    }
    let revision = c.revision();
    c.remove_group(&id(100), true).unwrap();
    assert_eq!(c.revision(), revision + 1);
    assert!(c.preset(&id(1)).is_none());
    assert!(c.organization().groups().is_empty());
    assert_eq!(c.selected().id(), "round-pressure");
    assert_eq!(
        captured.settings().mask.as_ref().unwrap().coverage(),
        [255, 128, 0, 255]
    );
    for d in [&mut actual, &mut reference] {
        d.stroke_to(InputPoint::new(90.5, 30.5, 0.6)).unwrap();
        d.end_stroke().unwrap();
        assert_eq!(d.history_depth(), (1, 0));
    }
    for y in 0..64 {
        for x in 0..128 {
            assert_eq!(actual.pixel(x, y), reference.pixel(x, y));
        }
    }
    let image = actual.snapshot();
    actual.undo().unwrap();
    actual.redo().unwrap();
    for y in 0..64 {
        for x in 0..128 {
            assert_eq!(actual.pixel(x, y), image.pixel(x, y));
        }
    }
}
