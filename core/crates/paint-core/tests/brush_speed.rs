use paint_core::*;

fn speed() -> SpeedDynamics {
    SpeedDynamics::new(
        true,
        1000.,
        PressureControl::new(PressureCurve::new(vec![(0., 1.), (1., 0.25)]).unwrap(), 0.).unwrap(),
        PressureControl::default(),
        PressureControl::default(),
    )
    .unwrap()
}

fn speed_with(flow: PressureControl, opacity: PressureControl) -> SpeedDynamics {
    SpeedDynamics::new(true, 1000., PressureControl::default(), flow, opacity).unwrap()
}

fn control(points: Vec<(f32, f32)>) -> PressureControl {
    PressureControl::new(PressureCurve::new(points).unwrap(), 0.).unwrap()
}

fn brush(dynamics: SpeedDynamics) -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            opacity: 1.,
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
        dynamics: BrushDynamics {
            speed: dynamics,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn draw(dynamics: SpeedDynamics, duration_ns: u64) -> Document {
    draw_spaced(dynamics, duration_ns, 0.25)
}

fn draw_spaced(dynamics: SpeedDynamics, duration_ns: u64, spacing: f32) -> Document {
    let mut d = Document::new(64, 64).unwrap();
    let mut start = InputPoint::new(16.5, 32.5, 1.);
    start.timestamp_ns = 0;
    let mut definition = brush(dynamics);
    definition.brush.spacing = spacing;
    d.begin_brush_stroke(definition, start).unwrap();
    let mut end = InputPoint::new(48.5, 32.5, 1.);
    end.timestamp_ns = duration_ns;
    d.stroke_to(end).unwrap();
    d.end_stroke().unwrap();
    d
}

fn colored_dual_brush(dynamics: SpeedDynamics) -> BrushDefinition {
    let tip = BrushTip {
        pressure_size: false,
        pressure_flow: false,
        ..Default::default()
    };
    let color = DualColor::new(
        true,
        true,
        true,
        DualColorOrder::SecondBehind,
        BlendMode::Normal,
        BlendMode::Normal,
    )
    .unwrap();
    BrushDefinition {
        brush: Brush {
            radius: 4.,
            spacing: 1.,
            color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
            background_color: Pixel::from_straight([0., 0., 1., 1.]).unwrap(),
            ..Default::default()
        },
        tip,
        dual: DualBrush::new(
            true,
            DualMode::Multiply,
            SecondaryTip::new(8., tip, None, Default::default()).unwrap(),
        )
        .with_color(color),
        dynamics: BrushDynamics {
            speed: dynamics,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn speed_size_curve_changes_radius_by_distance_per_time() {
    let fast = draw(speed(), 1_000_000);
    let slow = draw(speed(), 100_000_000);
    assert_eq!(fast.pixel(32, 38), Pixel::TRANSPARENT);
    assert!(slow.pixel(32, 38).alpha() > 0.);
    assert_eq!(fast.history_depth(), (1, 0));
}

#[test]
fn disabled_speed_preserves_pixels_and_history() {
    let plain = draw(SpeedDynamics::default(), 1_000_000);
    let mut reference = Document::new(64, 64).unwrap();
    let mut start = InputPoint::new(16.5, 32.5, 1.);
    start.timestamp_ns = 0;
    reference
        .begin_brush_stroke(brush(SpeedDynamics::default()), start)
        .unwrap();
    let mut end = InputPoint::new(48.5, 32.5, 1.);
    end.timestamp_ns = 1_000_000;
    reference.stroke_to(end).unwrap();
    reference.end_stroke().unwrap();
    for y in 0..64 {
        for x in 0..64 {
            assert_eq!(plain.pixel(x, y), reference.pixel(x, y));
        }
    }
    assert_eq!(plain.history_depth(), (1, 0));
}

#[test]
fn speed_flow_and_opacity_curves_change_actual_deposition() {
    let fast_flow = draw_spaced(
        speed_with(
            control(vec![(0., 1.), (1., 0.1)]),
            PressureControl::default(),
        ),
        1_000_000,
        1.,
    );
    let slow_flow = draw_spaced(
        speed_with(
            control(vec![(0., 1.), (1., 0.1)]),
            PressureControl::default(),
        ),
        1_000_000_000,
        1.,
    );
    assert!(fast_flow.pixel(48, 32).alpha() < slow_flow.pixel(48, 32).alpha() - 0.2);

    let fast_opacity = draw_spaced(
        speed_with(
            PressureControl::default(),
            control(vec![(0., 1.), (1., 0.2)]),
        ),
        1_000_000,
        1.,
    );
    let slow_opacity = draw_spaced(
        speed_with(
            PressureControl::default(),
            control(vec![(0., 1.), (1., 0.2)]),
        ),
        1_000_000_000,
        1.,
    );
    assert!(fast_opacity.pixel(48, 32).alpha() <= 0.200001);
    assert!(slow_opacity.pixel(48, 32).alpha() > 0.5);
}

#[test]
fn speed_size_applies_to_paired_secondary_tip() {
    let draw = |duration_ns| {
        let mut d = Document::new(64, 64).unwrap();
        let mut start = InputPoint::new(16.5, 32.5, 1.);
        start.timestamp_ns = 0;
        d.begin_brush_stroke(colored_dual_brush(speed()), start)
            .unwrap();
        let mut end = InputPoint::new(48.5, 32.5, 1.);
        end.timestamp_ns = duration_ns;
        d.stroke_to(end).unwrap();
        d.end_stroke().unwrap();
        d
    };
    let fast = draw(1_000_000);
    let slow = draw(1_000_000_000);
    assert_eq!(fast.pixel(48, 38), Pixel::TRANSPARENT);
    assert!(slow.pixel(48, 38).alpha() > 0.);
}
