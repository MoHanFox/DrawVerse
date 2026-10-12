use paint_core::*;
fn wand(x: u32, y: u32) -> WandSelection {
    WandSelection {
        x,
        y,
        sample_size: 1,
        tolerance: 0,
        antialias: true,
        contiguous: true,
        all_layers: false,
        operation: SelectionOperation::Replace,
    }
}
fn fill(color: Pixel) -> BucketFill {
    BucketFill {
        x: 0,
        y: 0,
        color,
        opacity: 1.,
        tolerance: 0,
        antialias: true,
        contiguous: true,
        all_layers: false,
    }
}
fn split() -> Document {
    let mut d = Document::new(8, 4).unwrap();
    d.bucket_fill(fill(Pixel::WHITE)).unwrap();
    d.begin_stroke(
        Brush {
            radius: 0.5,
            color: Pixel::from_straight([0., 0., 0., 1.]).unwrap(),
            spacing: 0.1,
            ..Brush::default()
        },
        InputPoint::new(3.5, 0.5, 1.),
    )
    .unwrap();
    d.stroke_to(InputPoint::new(3.5, 3.5, 1.)).unwrap();
    d.end_stroke().unwrap();
    d
}
#[test]
fn contiguous_global_combination_history_and_persistence() {
    let mut d = split();
    let pixels = d.snapshot();
    let depth = d.history_depth().0;
    d.select_wand(wand(0, 0)).unwrap();
    assert_eq!(d.selection().coverage(1.5, 1.5), 1.);
    assert_eq!(d.selection().coverage(5.5, 1.5), 0.);
    assert_eq!(
        d.history_actions().last(),
        Some(&HistoryAction::MagicSelection)
    );
    assert_eq!(d.history_depth().0, depth + 1);
    let mut o = wand(5, 0);
    o.operation = SelectionOperation::Add;
    d.select_wand(o).unwrap();
    assert_eq!(d.selection().coverage(5.5, 1.5), 1.);
    o.operation = SelectionOperation::Subtract;
    d.select_wand(o).unwrap();
    assert_eq!(d.selection().coverage(5.5, 1.5), 0.);
    o.operation = SelectionOperation::Intersect;
    d.select_wand(o).unwrap();
    assert_eq!(d.selection().coverage(1.5, 1.5), 0.);
    d.undo().unwrap();
    assert_eq!(d.selection().coverage(1.5, 1.5), 1.);
    d.redo().unwrap();
    o.operation = SelectionOperation::Replace;
    o.contiguous = false;
    d.select_wand(o).unwrap();
    assert_eq!(d.selection().coverage(1.5, 1.5), 1.);
    assert_eq!(d.selection().coverage(5.5, 1.5), 1.);
    assert_eq!(d.selection().coverage(3.5, 1.5), 0.);
    let restored = Selection::from_text(&d.selection().to_text()).unwrap();
    assert_eq!(&restored, d.selection());
    for y in 0..4 {
        for x in 0..8 {
            assert_eq!(d.pixel(x, y), pixels.pixel(x, y));
        }
    }
    let rev = d.revision();
    let history = d.history_depth();
    o.sample_size = 2;
    assert!(d.select_wand(o).is_err());
    assert_eq!(d.revision(), rev);
    assert_eq!(d.history_depth(), history);
    let mut huge = Document::new(1000000, 1000000).unwrap();
    o = wand(0, 0);
    o.contiguous = false;
    assert!(huge.select_wand(o).is_err());
    assert_eq!(huge.history_depth(), (0, 0));
    assert!(!huge.selection().enabled());
}
#[test]
fn clipped_average_all_sizes_composite_offsets_locks_and_partial_aa() {
    let mut d = Document::new(2, 1).unwrap();
    d.bucket_fill(fill(Pixel::WHITE)).unwrap();
    d.begin_stroke(
        Brush {
            radius: 0.5,
            color: Pixel::from_straight([0., 0., 0., 1.]).unwrap(),
            ..Brush::default()
        },
        InputPoint::new(0.5, 0.5, 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    let mut o = wand(0, 0);
    o.tolerance = 128;
    d.select_wand(o).unwrap();
    assert_eq!(d.selection().coverage(1.5, 0.5), 0.);
    for size in [3, 5, 11, 31, 51, 101] {
        o.sample_size = size;
        d.select_wand(o).unwrap();
        assert_eq!(d.selection().coverage(1.5, 0.5), 1.);
    }
    let id = d.add_layer("top").unwrap();
    d.move_layer(id, -1, 2).unwrap();
    o = wand(0, 0);
    o.all_layers = true;
    d.select_wand(o).unwrap();
    assert_eq!(d.selection().coverage(1.5, 0.5), 0.);
    o.all_layers = false;
    d.select_wand(o).unwrap();
    assert_eq!(d.selection().coverage(1.5, 0.5), 1.);
    d.set_layer_appearance(
        id,
        LayerAppearance {
            locks: LOCK_ALL,
            offset_x: -1,
            offset_y: 2,
            ..LayerAppearance::default()
        },
    )
    .unwrap();
    d.select_wand(o).unwrap();
    let mut a = Document::new(2, 1).unwrap();
    let gray = ((0.5f32 + 0.055) / 1.055).powf(2.4);
    a.bucket_fill(fill(Pixel::from_straight([gray, gray, gray, 1.]).unwrap()))
        .unwrap();
    a.begin_stroke(
        Brush {
            radius: 0.5,
            color: Pixel::from_straight([0., 0., 0., 1.]).unwrap(),
            ..Brush::default()
        },
        InputPoint::new(0.5, 0.5, 1.),
    )
    .unwrap();
    a.end_stroke().unwrap();
    o = wand(0, 0);
    o.tolerance = 127;
    a.select_wand(o).unwrap();
    assert!((a.selection().coverage(1.5, 0.5) - 0.5).abs() < 0.005);
    let s = Selection::from_text(&a.selection().to_text()).unwrap();
    assert_eq!(&s, a.selection());
    o.antialias = false;
    a.select_wand(o).unwrap();
    assert_eq!(a.selection().coverage(1.5, 0.5), 0.);
}
