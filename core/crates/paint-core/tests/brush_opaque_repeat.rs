use paint_core::*;

fn brush() -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            opacity: 1.,
            spacing: 0.25,
            color: Pixel::from_straight([0.2, 0.6, 0.8, 1.]).unwrap(),
            ..Default::default()
        },
        tip: BrushTip {
            hardness: 0.65,
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn repeated_opaque_stationary_dabs_do_not_change_pixels_or_history() {
    let point = InputPoint::new(16.5, 16.5, 1.);
    let mut single = Document::new(32, 32).unwrap();
    single.begin_brush_stroke(brush(), point).unwrap();
    single.end_stroke().unwrap();
    let mut repeated = Document::new(32, 32).unwrap();
    repeated.begin_brush_stroke(brush(), point).unwrap();
    for _ in 0..12 {
        repeated.stroke_to(point).unwrap();
    }
    repeated.end_stroke().unwrap();
    for y in 0..32 {
        for x in 0..32 {
            assert_eq!(repeated.pixel(x, y), single.pixel(x, y));
        }
    }
    assert_eq!(repeated.history_depth(), (1, 0));
}
