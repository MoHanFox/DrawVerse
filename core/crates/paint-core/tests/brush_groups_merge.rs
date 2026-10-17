use paint_core::*;
use std::sync::Arc;
fn id(n: usize) -> String {
    format!("12345678-1234-4234-8234-{n:012x}")
}
fn p(n: usize) -> BrushCatalogItem {
    BrushCatalogItem::Preset(id(n))
}
fn g(n: usize) -> BrushCatalogItem {
    BrushCatalogItem::Group(id(n))
}
fn preset(n: usize) -> BrushPreset {
    BrushPreset::new(id(n), "same".into(), BrushSettings::default()).unwrap()
}
fn tree() -> BrushOrganization {
    BrushOrganization::new(
        vec![g(100), p(2)],
        vec![
            BrushGroup::new(id(100), "same".into(), vec![p(1), g(101)]).unwrap(),
            BrushGroup::new(id(101), "same".into(), vec![]).unwrap(),
        ],
    )
    .unwrap()
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
    ($c:ident, $e:expr) => {{
        let before = $c.clone();
        assert!($e.is_err());
        unchanged(&$c, &before);
    }};
}
#[test]
fn organized_import_inserts_subtree_once_and_retains_snapshots_and_capture() {
    for nested in [false, true] {
        let mut c = BrushCatalog::default();
        c.create_group(id(500), "target".into(), None, 1).unwrap();
        let before = c.clone();
        let capture = c.selected();
        let mut actual = Document::new(64, 64).unwrap();
        let mut reference = Document::new(64, 64).unwrap();
        for d in [&mut actual, &mut reference] {
            d.begin_brush_stroke(
                capture.settings().definition(),
                InputPoint::new(12., 12., 1.),
            )
            .unwrap();
        }
        let parent = nested.then(|| id(500));
        c.import_organized(
            vec![preset(1), preset(2)],
            tree(),
            parent.as_deref(),
            if nested { 0 } else { 1 },
        )
        .unwrap();
        assert_eq!(c.revision(), before.revision() + 1);
        assert_eq!(c.selected().id(), id(1));
        let layout = c.organization();
        let siblings = if nested {
            layout.groups()[&id(500)].items()
        } else {
            layout.roots()
        };
        let start = if nested { 0 } else { 1 };
        assert_eq!(&siblings[start..start + 2], &[g(100), p(2)]);
        assert_eq!(layout.groups()[&id(100)].items(), &[p(1), g(101)]);
        assert!(layout.groups()[&id(101)].items().is_empty());
        assert_eq!(layout.preset_order(), c.ids());
        for id in before.ids() {
            assert!(Arc::ptr_eq(
                &before.preset(id).unwrap(),
                &c.preset(id).unwrap()
            ));
        }
        assert_eq!(before.organization().groups().len(), 1);
        c.remove_group(&id(100), true).unwrap();
        for d in [&mut actual, &mut reference] {
            d.stroke_to(InputPoint::new(40., 35., 0.5)).unwrap();
            d.end_stroke().unwrap();
            assert_eq!(d.history_depth(), (1, 0));
        }
        for y in 0..64 {
            for x in 0..64 {
                assert_eq!(actual.pixel(x, y), reference.pixel(x, y));
            }
        }
        actual.undo().unwrap();
        actual.redo().unwrap();
        for y in 0..64 {
            for x in 0..64 {
                assert_eq!(actual.pixel(x, y), reference.pixel(x, y));
            }
        }
    }
}
#[test]
fn organized_import_empty_folders_selection_and_true_noop() {
    let mut c = BrushCatalog::default();
    let before = c.clone();
    let empty = || BrushOrganization::new(vec![], vec![]).unwrap();
    c.import_organized(vec![], empty(), None, 0).unwrap();
    unchanged(&c, &before);
    reject!(c, c.import_organized(vec![], empty(), None, 999));
    reject!(c, c.import_organized(vec![], empty(), Some(&id(999)), 0));
    let folder = BrushOrganization::new(
        vec![g(100)],
        vec![BrushGroup::new(id(100), "empty".into(), vec![]).unwrap()],
    )
    .unwrap();
    c.import_organized(vec![], folder, None, 0).unwrap();
    assert_eq!(c.revision(), before.revision() + 1);
    assert!(Arc::ptr_eq(&c.selected(), &before.selected()));
    assert_eq!(c.ids(), before.ids());
    assert!(c.organization().groups()[&id(100)].items().is_empty());
}
#[test]
fn organized_import_conflicts_projection_position_and_budgets_are_atomic() {
    let mut c = BrushCatalog::default();
    reject!(
        c,
        c.import_organized(vec![preset(2), preset(1)], tree(), None, 0)
    );
    reject!(c, c.import_organized(vec![preset(1)], tree(), None, 0));
    reject!(
        c,
        c.import_organized(vec![preset(1), preset(2)], tree(), None, 999)
    );
    reject!(
        c,
        c.import_organized(vec![preset(1), preset(2)], tree(), Some(&id(999)), 0)
    );
    c.create_group(id(1), "collision".into(), None, 0).unwrap();
    reject!(
        c,
        c.import_organized(vec![preset(1), preset(2)], tree(), None, 0)
    );
    c.remove_group(&id(1), false).unwrap();
    c.import(vec![preset(100)]).unwrap();
    reject!(
        c,
        c.import_organized(vec![preset(1), preset(2)], tree(), None, 0)
    );
    c.remove(&id(100)).unwrap();
    c.create_group(id(101), "collision".into(), None, 0)
        .unwrap();
    reject!(
        c,
        c.import_organized(vec![preset(1), preset(2)], tree(), None, 0)
    );
    c.remove_group(&id(101), false).unwrap();
    c.import(vec![preset(1)]).unwrap();
    reject!(
        c,
        c.import_organized(vec![preset(1), preset(2)], tree(), None, 0)
    );
    let mut tiny = BrushCatalog::with_mask_budget(1).unwrap();
    let settings = BrushSettings {
        mask: Some(Arc::new(BrushMask::new(2, 2, vec![255; 4]).unwrap())),
        ..Default::default()
    };
    reject!(
        tiny,
        tiny.import_organized(
            vec![
                BrushPreset::new(id(1), "large".into(), settings).unwrap(),
                preset(2)
            ],
            tree(),
            None,
            0
        )
    );
}
#[test]
fn organized_import_rechecks_combined_depth_and_counts() {
    let mut c = BrushCatalog::default();
    for n in 0..7 {
        c.create_group(
            id(500 + n),
            "depth".into(),
            (n > 0).then(|| id(499 + n)).as_deref(),
            0,
        )
        .unwrap();
    }
    reject!(
        c,
        c.import_organized(vec![preset(1), preset(2)], tree(), Some(&id(506)), 0)
    );
    let mut c = BrushCatalog::default();
    for n in 0..255 {
        c.create_group(id(500 + n), "group".into(), None, 0)
            .unwrap();
    }
    reject!(
        c,
        c.import_organized(vec![preset(1), preset(2)], tree(), None, 0)
    );
    let mut c = BrushCatalog::default();
    c.import((500..1520).map(preset).collect()).unwrap();
    reject!(
        c,
        c.import_organized(vec![preset(1), preset(2)], tree(), None, 0)
    );
}
