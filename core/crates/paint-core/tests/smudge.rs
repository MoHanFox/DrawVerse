use paint_core::*;
fn red() -> Pixel {
    Pixel::from_straight([1., 0., 0., 1.]).unwrap()
}
fn smudge(strength: f32, all_layers: bool, finger: bool, pressure: bool) -> Brush {
    Brush {
        background_color: Pixel::WHITE,
        radius: 1.5,
        opacity: strength,
        spacing: 1.,
        color: Pixel::from_straight([0., 0., 1., 1.]).unwrap(),
        mode: BrushMode::Smudge {
            all_layers,
            finger_painting: finger,
            pressure_strength: pressure,
        },
    }
}
fn seed() -> Document {
    let mut d = Document::new(12, 5).unwrap();
    for y in 0..5 {
        for x in 0..3 {
            d.begin_stroke(
                Brush {
                    radius: 0.5,
                    color: red(),
                    ..Brush::default()
                },
                InputPoint::new(x as f64 + 0.5, y as f64 + 0.5, 1.),
            )
            .unwrap();
            d.end_stroke().unwrap();
        }
    }
    d
}
fn drag(d: &mut Document, b: Brush, pressure: f32) {
    d.begin_stroke(b, InputPoint::new(2.5, 2.5, pressure))
        .unwrap();
    d.stroke_to(InputPoint::new(4., 2.5, pressure)).unwrap();
    d.end_stroke().unwrap();
}
#[test]
fn actual_drag_single_history_cancel_undo_and_no_foreground_without_finger() {
    let mut d = seed();
    let before = d.snapshot();
    let depth = d.history_depth();
    let b = smudge(1., false, false, true);
    d.begin_stroke(b, InputPoint::new(2.5, 2.5, 1.)).unwrap();
    assert!(!d.end_stroke().unwrap());
    assert_eq!(d.history_depth(), depth);
    drag(&mut d, b, 1.);
    assert!(d.pixel(3, 2).alpha() > 0.);
    assert_eq!(d.pixel(3, 2).components()[2], 0.);
    assert_eq!(d.history_actions().last(), Some(&HistoryAction::Smudge));
    assert_eq!(d.history_depth().0, depth.0 + 1);
    let after = d.snapshot();
    d.undo().unwrap();
    for y in 0..5 {
        for x in 0..12 {
            assert_eq!(d.pixel(x, y), before.pixel(x, y));
        }
    }
    d.redo().unwrap();
    assert_eq!(d.pixel(3, 2), after.pixel(3, 2));
    d.begin_stroke(b, InputPoint::new(3.5, 2.5, 1.)).unwrap();
    d.stroke_to(InputPoint::new(6., 2.5, 1.)).unwrap();
    d.cancel_stroke().unwrap();
    for y in 0..5 {
        for x in 0..12 {
            assert_eq!(d.pixel(x, y), after.pixel(x, y));
        }
    }
}
#[test]
fn pressure_strength_finger_and_one_percent_spacing() {
    let mut weak = seed();
    let mut strong = seed();
    drag(&mut weak, smudge(0.8, false, false, true), 0.5);
    drag(&mut strong, smudge(0.8, false, false, false), 0.5);
    assert!(strong.pixel(3, 2).alpha() > weak.pixel(3, 2).alpha());
    let mut d = Document::new(12, 5).unwrap();
    let b = smudge(0.5, false, true, false);
    d.begin_stroke(b, InputPoint::new(2.5, 2.5, 1.)).unwrap();
    assert!(d.pixel(2, 2).components()[2] > 0.);
    d.stroke_to(InputPoint::new(4., 2.5, 1.)).unwrap();
    d.end_stroke().unwrap();
    assert!(d.pixel(3, 2).components()[2] > 0.);
    d.undo().unwrap();
    assert_eq!(d.pixel(2, 2), Pixel::TRANSPARENT);
    let brush = Brush {
        spacing: 0.02,
        ..Brush::default()
    };
    brush.validate().unwrap();
    assert!(Brush {
        spacing: 0.019,
        ..brush
    }
    .validate()
    .is_err());
    let mut d = Document::new(32, 8).unwrap();
    d.begin_stroke(
        Brush {
            radius: 2.,
            ..brush
        },
        InputPoint::new(2., 4., 1.),
    )
    .unwrap();
    d.stroke_to(InputPoint::new(20., 4., 1.)).unwrap();
    d.end_stroke().unwrap();
    assert!(d.pixel(10, 4).alpha() > 0.);
}
#[test]
fn composite_sampling_offsets_selection_alpha_lock_and_masks_share_pipeline() {
    let mut d = seed();
    let base = d.snapshot();
    let id = d.add_layer("top").unwrap();
    d.move_layer(id, -2, 1).unwrap();
    let depth = d.history_depth();
    d.begin_stroke(
        smudge(1., true, false, false),
        InputPoint::new(2.5, 2.5, 1.),
    )
    .unwrap();
    assert!(!d.end_stroke().unwrap());
    assert_eq!(d.history_depth(), depth);
    drag(&mut d, smudge(1., true, false, false), 1.);
    assert!(d.layers()[1].canvas_pixel(3, 2).unwrap().alpha() > 0.);
    for y in 0..5 {
        for x in 0..12 {
            assert_eq!(d.layers()[0].canvas_pixel(x, y).unwrap(), base.pixel(x, y));
        }
    }
    d.undo().unwrap();
    let selection = Selection::default()
        .apply(
            SelectionShape::geometry(SelectionKind::Rectangle, 0., 0., 3., 5., false),
            SelectionOperation::Replace,
            12,
            5,
        )
        .unwrap();
    d.set_selection(selection).unwrap();
    drag(&mut d, smudge(1., true, false, false), 1.);
    assert_eq!(
        d.layers()[1].canvas_pixel(3, 2).unwrap(),
        Pixel::TRANSPARENT
    );
    d.set_selection(Selection::default()).unwrap();
    d.set_layer_appearance(
        id,
        LayerAppearance {
            offset_x: -2,
            offset_y: 1,
            locks: LOCK_TRANSPARENCY,
            ..LayerAppearance::default()
        },
    )
    .unwrap();
    drag(&mut d, smudge(1., true, false, false), 1.);
    assert_eq!(d.layers()[1].canvas_pixel(3, 2).unwrap().alpha(), 0.);
    d.set_layer_appearance(
        id,
        LayerAppearance {
            locks: LOCK_ALL,
            ..LayerAppearance::default()
        },
    )
    .unwrap();
    assert!(d
        .begin_stroke(smudge(1., false, false, true), InputPoint::new(2., 2., 1.))
        .is_err());
    let mask = d.add_mask(1).unwrap();
    d.set_active_layer(mask).unwrap();
    assert!(d
        .begin_stroke(smudge(1., true, false, true), InputPoint::new(2., 2., 1.))
        .is_err());
    let mut b = smudge(1., false, true, false);
    b.color = Pixel::from_straight([0., 0., 0., 1.]).unwrap();
    d.begin_stroke(b, InputPoint::new(2.5, 2.5, 1.)).unwrap();
    d.end_stroke().unwrap();
    assert_eq!(d.pixel(2, 2).alpha(), 0.);
    d.undo().unwrap();
    assert_eq!(d.pixel(2, 2).alpha(), 1.);
}
