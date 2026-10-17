use paint_core::*;

fn control(points: Vec<(f32, f32)>) -> PressureControl {
    PressureControl::new(PressureCurve::new(points).unwrap(), 0.).unwrap()
}

fn dynamics(
    size: PressureControl,
    flow: PressureControl,
    opacity: PressureControl,
) -> DirectionDynamics {
    DirectionDynamics::new(true, size, flow, opacity)
}

fn brush(dynamics: DirectionDynamics) -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            opacity: 1.,
            spacing: 1.,
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
            direction: dynamics,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn draw(dynamics: DirectionDynamics, left: bool) -> Document {
    let mut d = Document::new(64, 64).unwrap();
    let (start, end) = if left { (48.5, 16.5) } else { (16.5, 48.5) };
    d.begin_brush_stroke(brush(dynamics), InputPoint::new(start, 32.5, 1.))
        .unwrap();
    d.stroke_to(InputPoint::new(end, 32.5, 1.)).unwrap();
    d.end_stroke().unwrap();
    d
}

#[test]
fn direction_size_curve_changes_radius_by_stroke_angle() {
    let size = dynamics(
        control(vec![(0., 1.), (1., 0.1)]),
        PressureControl::default(),
        PressureControl::default(),
    );
    let right = draw(size.clone(), false);
    let left = draw(size, true);
    assert!(right.pixel(48, 38).alpha() > 0.);
    assert_eq!(left.pixel(16, 38), Pixel::TRANSPARENT);
}

#[test]
fn direction_flow_and_opacity_curves_change_actual_deposition() {
    let flow = dynamics(
        PressureControl::default(),
        control(vec![(0., 1.), (1., 0.1)]),
        PressureControl::default(),
    );
    let right = draw(flow.clone(), false);
    let left = draw(flow, true);
    assert!(right.pixel(48, 32).alpha() > left.pixel(16, 32).alpha() + 0.2);

    let opacity = dynamics(
        PressureControl::default(),
        PressureControl::default(),
        control(vec![(0., 1.), (1., 0.2)]),
    );
    let right = draw(opacity.clone(), false);
    let left = draw(opacity, true);
    assert!(right.pixel(48, 32).alpha() > left.pixel(16, 32).alpha() + 0.15);
    assert!(left.pixel(16, 32).alpha() <= 0.6);
}

#[test]
fn secondary_tip_rejects_direction_dynamics() {
    let direction = DirectionDynamics::new(
        true,
        Default::default(),
        Default::default(),
        Default::default(),
    );
    assert!(SecondaryTip::new(
        8.,
        BrushTip::default(),
        None,
        BrushDynamics {
            direction,
            ..Default::default()
        }
    )
    .is_err());
}
