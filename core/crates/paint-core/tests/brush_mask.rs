use paint_core::*;
use std::sync::Arc;
fn definition(mask: Arc<BrushMask>, angle: f32) -> BrushDefinition {
    BrushDefinition {
        smoothing: Default::default(),
        texture: Default::default(),
        dual: Default::default(),
        dynamics: Default::default(),
        brush: Brush {
            radius: 8.,
            spacing: 1.,
            ..Brush::default()
        },
        tip: BrushTip {
            angle,
            pressure_size: false,
            pressure_flow: false,
            ..BrushTip::default()
        },
        mask: Some(mask),
    }
}
fn dot(d: &mut Document, definition: BrushDefinition, x: f64, y: f64) {
    d.begin_brush_stroke(definition, InputPoint::new(x, y, 1.))
        .unwrap();
    d.end_stroke().unwrap();
}
#[test]
fn mask_validation_mips_and_immutability_are_bounded() {
    for (w, h, bytes) in [(0, 1, vec![]), (4097, 1, vec![]), (2, 2, vec![1; 3])] {
        assert!(BrushMask::new(w, h, bytes).is_err());
    }
    let bytes = (0..64)
        .map(|i| if i % 2 == 0 { 255 } else { 0 })
        .collect::<Vec<_>>();
    let mask = Arc::new(BrushMask::new(8, 8, bytes.clone()).unwrap());
    assert_eq!(mask.coverage(), &bytes);
    assert_eq!(mask.resident_bytes(), 85);
    let program = BrushProgram::compile(definition(mask.clone(), 0.)).unwrap();
    let average = program.coverage(0., 0., 0.5);
    assert!((average - 0.5).abs() < 0.01, "{average}");
    assert_eq!(mask.coverage(), &bytes);
}
#[test]
fn image_holes_and_rotated_rectangle_corners_are_not_circle_clipped() {
    let mut pixels = vec![255; 256];
    for y in 6..10 {
        for x in 6..10 {
            pixels[y * 16 + x] = 0;
        }
    }
    let mask = Arc::new(BrushMask::new(16, 16, pixels).unwrap());
    let mut d = Document::new(50, 50).unwrap();
    dot(&mut d, definition(mask, 45.), 25.5, 25.5);
    assert_eq!(d.pixel(25, 25).alpha(), 0.);
    assert!(
        d.pixel(25, 35).alpha() > 0.9,
        "rotated square corner must extend past radius"
    );
    let before = d.snapshot();
    let history = d.history_depth();
    d.undo().unwrap();
    assert_eq!(d.pixel(25, 35).alpha(), 0.);
    d.redo().unwrap();
    assert_eq!(d.history_depth(), history);
    for y in 0..50 {
        for x in 0..50 {
            assert_eq!(d.pixel(x, y), before.pixel(x, y));
        }
    }
    let mask = Arc::new(BrushMask::new(16, 4, vec![255; 64]).unwrap());
    let mut d = Document::new(50, 50).unwrap();
    dot(&mut d, definition(mask, 0.), 25.5, 25.5);
    assert!(d.pixel(32, 25).alpha() > 0.9);
    assert_eq!(d.pixel(25, 32).alpha(), 0.);
}
#[test]
fn image_resource_snapshot_survives_drop_cancel_and_eraser_uses_same_tip() {
    let mask = Arc::new(BrushMask::new(8, 8, vec![255; 64]).unwrap());
    let mut d = Document::new(50, 50).unwrap();
    d.begin_brush_stroke(
        definition(mask.clone(), 0.),
        InputPoint::new(20.5, 20.5, 1.),
    )
    .unwrap();
    assert_eq!(Arc::strong_count(&mask), 2);
    drop(mask);
    d.stroke_to(InputPoint::new(24.5, 20.5, 1.)).unwrap();
    d.end_stroke().unwrap();
    let before = d.snapshot();
    let mask = Arc::new(BrushMask::new(8, 8, vec![255; 64]).unwrap());
    let mut erase = definition(mask, 0.);
    erase.brush.mode = BrushMode::Erase;
    erase.brush.radius = 2.;
    d.begin_brush_stroke(erase.clone(), InputPoint::new(20.5, 20.5, 1.))
        .unwrap();
    assert_eq!(d.pixel(20, 20).alpha(), 0.);
    d.cancel_stroke().unwrap();
    assert_eq!(d.pixel(20, 20), before.pixel(20, 20));
    dot(&mut d, erase, 20.5, 20.5);
    assert_eq!(d.pixel(20, 20).alpha(), 0.);
    d.undo().unwrap();
    assert_eq!(d.pixel(20, 20), before.pixel(20, 20));
}

#[test]
fn image_tip_selection_masks_and_finger_smudge_share_coverage() {
    let mut pixels = vec![255; 256];
    for y in 6..10 {
        for x in 6..10 {
            pixels[y * 16 + x] = 0;
        }
    }
    let mask = Arc::new(BrushMask::new(16, 16, pixels).unwrap());
    let mut d = Document::new(50, 50).unwrap();
    let selection = Selection::default()
        .apply(
            SelectionShape::geometry(SelectionKind::Rectangle, 25., 0., 25., 50., true),
            SelectionOperation::Replace,
            50,
            50,
        )
        .unwrap();
    d.set_selection(selection).unwrap();
    dot(&mut d, definition(mask.clone(), 0.), 25.5, 25.5);
    assert_eq!(d.pixel(20, 25).alpha(), 0.);
    assert!(d.pixel(31, 25).alpha() > 0.9);
    assert_eq!(d.pixel(25, 25).alpha(), 0.);
    let mut d = Document::new(50, 50).unwrap();
    d.initialize_white_background().unwrap();
    d.add_mask(1).unwrap();
    dot(&mut d, definition(mask.clone(), 0.), 25.5, 25.5);
    assert_eq!(d.pixel(31, 25).alpha(), 0.);
    assert_eq!(d.pixel(25, 25), Pixel::WHITE);
    d.undo().unwrap();
    assert_eq!(d.pixel(31, 25), Pixel::WHITE);
    let mut d = Document::new(50, 50).unwrap();
    let mut smudge = definition(mask, 0.);
    smudge.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: true,
        pressure_strength: false,
    };
    dot(&mut d, smudge, 25.5, 25.5);
    assert_eq!(d.pixel(25, 25).alpha(), 0.);
    assert!(d.pixel(31, 25).alpha() > 0.9);
    assert_eq!(d.history_actions().last(), Some(&HistoryAction::Smudge));
}

#[test]
fn rotated_image_pixel_budget_rejects_before_segment_mutation() {
    let mask = Arc::new(BrushMask::new(16, 16, vec![255; 256]).unwrap());
    let mut d = Document::new(1024, 1024).unwrap();
    let mut brush = definition(mask, 45.);
    brush.brush.radius = 256.;
    brush.brush.spacing = 0.02;
    d.begin_brush_stroke(brush, InputPoint::new(400., 400., 1.))
        .unwrap();
    let before = d.snapshot();
    assert!(matches!(
        d.stroke_to(InputPoint::new(500., 400., 1.)),
        Err(Error::ResourceLimit(_))
    ));
    assert_eq!(d.pixel(700, 400), before.pixel(700, 400));
    assert!(d.stroke_active());
    d.cancel_stroke().unwrap();
    assert_eq!(d.tile_count(), 0);
    assert_eq!(d.history_depth(), (0, 0));
}
