use paint_core::*;

fn control(points: Vec<(f32, f32)>) -> PressureControl {
    PressureControl::new(PressureCurve::new(points).unwrap(), 0.).unwrap()
}

fn dynamics(
    size: PressureControl,
    flow: PressureControl,
    opacity: PressureControl,
) -> TangentDynamics {
    TangentDynamics::new(true, size, flow, opacity)
}

fn brush(dynamics: TangentDynamics) -> BrushDefinition {
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
            tangent: dynamics,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn draw(dynamics: TangentDynamics, tangent: f32) -> Document {
    let mut d = Document::new(64, 64).unwrap();
    let mut start = InputPoint::new(16.5, 32.5, 1.);
    start.tangential_pressure = tangent;
    d.begin_brush_stroke(brush(dynamics), start).unwrap();
    let mut end = InputPoint::new(48.5, 32.5, 1.);
    end.tangential_pressure = tangent;
    d.stroke_to(end).unwrap();
    d.end_stroke().unwrap();
    d
}

#[test]
fn tangent_size_curve_uses_the_signed_device_range() {
    let dynamics = dynamics(
        control(vec![(0., 0.25), (1., 1.)]),
        PressureControl::default(),
        PressureControl::default(),
    );
    let negative = draw(dynamics.clone(), -1.);
    let positive = draw(dynamics, 1.);
    assert_eq!(negative.pixel(48, 38), Pixel::TRANSPARENT);
    assert!(positive.pixel(48, 38).alpha() > 0.);
    assert_eq!(positive.history_depth(), (1, 0));
}

#[test]
fn tangent_flow_and_opacity_curves_change_actual_deposition() {
    let flow = dynamics(
        PressureControl::default(),
        control(vec![(0., 1.), (1., 0.1)]),
        PressureControl::default(),
    );
    let slow = draw(flow.clone(), -1.);
    let fast = draw(flow, 1.);
    assert!(slow.pixel(48, 32).alpha() > fast.pixel(48, 32).alpha() + 0.2);

    let opacity = dynamics(
        PressureControl::default(),
        PressureControl::default(),
        control(vec![(0., 1.), (1., 0.2)]),
    );
    let negative = draw(opacity.clone(), -1.);
    let positive = draw(opacity, 1.);
    assert!(negative.pixel(48, 32).alpha() > 0.5);
    assert!(positive.pixel(48, 32).alpha() <= 0.200001);
}

#[test]
fn disabled_tangent_dynamics_preserves_pixels_and_history() {
    let mut reference = Document::new(64, 64).unwrap();
    let mut start = InputPoint::new(16.5, 32.5, 1.);
    start.tangential_pressure = -1.;
    reference
        .begin_brush_stroke(brush(TangentDynamics::default()), start)
        .unwrap();
    let mut end = InputPoint::new(48.5, 32.5, 1.);
    end.tangential_pressure = -1.;
    reference.stroke_to(end).unwrap();
    reference.end_stroke().unwrap();
    let plain = draw(TangentDynamics::default(), -1.);
    for y in 0..64 {
        for x in 0..64 {
            assert_eq!(plain.pixel(x, y), reference.pixel(x, y));
        }
    }
    assert_eq!(plain.history_depth(), (1, 0));
}

#[test]
fn secondary_tip_rejects_its_own_tangent_dynamics() {
    let tangent = TangentDynamics::new(
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
            tangent,
            ..Default::default()
        }
    )
    .is_err());
}
