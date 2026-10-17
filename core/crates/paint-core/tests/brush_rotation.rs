use paint_core::*;

fn control(points: Vec<(f32, f32)>) -> PressureControl {
    PressureControl::new(PressureCurve::new(points).unwrap(), 0.).unwrap()
}

fn dynamics(
    size: PressureControl,
    flow: PressureControl,
    opacity: PressureControl,
) -> RotationDynamics {
    RotationDynamics::new(true, size, flow, opacity)
}

fn brush(dynamics: RotationDynamics) -> BrushDefinition {
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
            rotation: dynamics,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn draw(dynamics: RotationDynamics, rotation: f32, capabilities: u32) -> Document {
    let mut d = Document::new(64, 64).unwrap();
    let mut start = InputPoint::new(16.5, 32.5, 1.);
    start.capabilities = capabilities;
    start.rotation = rotation;
    d.begin_brush_stroke(brush(dynamics), start).unwrap();
    let mut end = InputPoint::new(48.5, 32.5, 1.);
    end.capabilities = capabilities;
    end.rotation = rotation;
    d.stroke_to(end).unwrap();
    d.end_stroke().unwrap();
    d
}

#[test]
fn rotation_size_curve_uses_angle_and_capability() {
    let size = dynamics(
        control(vec![(0., 1.), (1., 0.1)]),
        PressureControl::default(),
        PressureControl::default(),
    );
    let zero = draw(size.clone(), 0., 4);
    let half = draw(size, 180., 4);
    let no_capability = draw(
        dynamics(
            control(vec![(0., 1.), (1., 0.1)]),
            PressureControl::default(),
            PressureControl::default(),
        ),
        180.,
        0,
    );
    assert!(zero.pixel(48, 38).alpha() > 0.);
    assert_eq!(half.pixel(48, 38), Pixel::TRANSPARENT);
    assert_eq!(no_capability.pixel(48, 38), zero.pixel(48, 38));
}

#[test]
fn rotation_flow_and_opacity_curves_change_actual_deposition() {
    let flow = dynamics(
        PressureControl::default(),
        control(vec![(0., 1.), (1., 0.1)]),
        PressureControl::default(),
    );
    let zero = draw(flow.clone(), 0., 4);
    let half = draw(flow, 180., 4);
    assert!(zero.pixel(48, 32).alpha() > half.pixel(48, 32).alpha() + 0.2);

    let opacity = dynamics(
        PressureControl::default(),
        PressureControl::default(),
        control(vec![(0., 1.), (1., 0.2)]),
    );
    let zero = draw(opacity.clone(), 0., 4);
    let half = draw(opacity, 359., 4);
    assert!(zero.pixel(48, 32).alpha() > half.pixel(48, 32).alpha() + 0.15);
    assert!(half.pixel(48, 32).alpha() <= 0.203);
}

#[test]
fn secondary_tip_rejects_rotation_dynamics() {
    let rotation = RotationDynamics::new(
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
            rotation,
            ..Default::default()
        }
    )
    .is_err());
}
