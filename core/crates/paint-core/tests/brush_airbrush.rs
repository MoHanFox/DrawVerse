use paint_core::*;

fn point(timestamp_ns: u64, pressure: f32) -> InputPoint {
    let mut p = InputPoint::new(32.5, 32.5, pressure);
    p.timestamp_ns = timestamp_ns;
    p
}

fn brush(airbrush: Airbrush) -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            opacity: 0.2,
            spacing: 0.25,
            color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
            ..Default::default()
        },
        tip: BrushTip {
            hardness: 1.,
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        airbrush,
        ..Default::default()
    }
}

#[test]
fn stationary_ticks_build_up_once_per_interval_and_share_history() {
    let mut d = Document::new(64, 64).unwrap();
    d.begin_brush_stroke(brush(Airbrush::new(true, 50.).unwrap()), point(0, 1.))
        .unwrap();
    let first = d.pixel(32, 32).alpha();
    d.airbrush_tick(point(40_000_000, 1.)).unwrap();
    assert_eq!(d.pixel(32, 32).alpha(), first);
    d.airbrush_tick(point(50_000_000, 1.)).unwrap();
    let second = d.pixel(32, 32).alpha();
    assert!(second > first);
    d.airbrush_tick(point(100_000_000, 1.)).unwrap();
    assert!(d.pixel(32, 32).alpha() > second);
    d.end_stroke().unwrap();
    assert_eq!(d.history_depth(), (1, 0));
}

#[test]
fn airbrush_rejects_disabled_moving_and_unsupported_modes() {
    let mut d = Document::new(64, 64).unwrap();
    d.begin_brush_stroke(brush(Airbrush::default()), point(0, 1.))
        .unwrap();
    assert!(d.airbrush_tick(point(50_000_000, 1.)).is_err());
    d.cancel_stroke().unwrap();

    d.begin_brush_stroke(brush(Airbrush::new(true, 10.).unwrap()), point(0, 1.))
        .unwrap();
    let mut moved = point(20_000_000, 1.);
    moved.x += 1.;
    assert!(d.airbrush_tick(moved).is_err());
    d.cancel_stroke().unwrap();

    let mut smudge = brush(Airbrush::new(true, 10.).unwrap());
    smudge.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: false,
        pressure_strength: false,
    };
    d.begin_brush_stroke(smudge, point(0, 1.)).unwrap();
    assert!(d.airbrush_tick(point(20_000_000, 1.)).is_err());
    d.cancel_stroke().unwrap();
}
