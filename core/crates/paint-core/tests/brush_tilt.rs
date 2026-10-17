use paint_core::*;

fn control(points: Vec<(f32, f32)>) -> PressureControl {
    PressureControl::new(PressureCurve::new(points).unwrap(), 0.).unwrap()
}

fn dynamics(
    size: PressureControl,
    flow: PressureControl,
    opacity: PressureControl,
) -> TiltDynamics {
    TiltDynamics::new(true, size, flow, opacity)
}

fn brush(dynamics: TiltDynamics) -> BrushDefinition {
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
            tilt: dynamics,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn draw(dynamics: TiltDynamics, tilt: f32, capabilities: u32) -> Document {
    let mut d = Document::new(64, 64).unwrap();
    let mut start = InputPoint::new(16.5, 32.5, 1.);
    start.capabilities = capabilities;
    start.tilt_x = tilt;
    d.begin_brush_stroke(brush(dynamics), start).unwrap();
    let mut end = InputPoint::new(48.5, 32.5, 1.);
    end.capabilities = capabilities;
    end.tilt_x = tilt;
    d.stroke_to(end).unwrap();
    d.end_stroke().unwrap();
    d
}

#[test]
fn tilt_size_curve_uses_magnitude_and_capability() {
    let size_dynamics = dynamics(
        control(vec![(0., 1.), (1., 0.2)]),
        PressureControl::default(),
        PressureControl::default(),
    );
    let flat = draw(size_dynamics.clone(), 0., 2);
    let tilted = draw(size_dynamics, 90., 2);
    let no_capability = draw(
        dynamics(
            control(vec![(0., 1.), (1., 0.2)]),
            PressureControl::default(),
            PressureControl::default(),
        ),
        90.,
        0,
    );
    assert!(flat.pixel(48, 38).alpha() > 0.);
    assert_eq!(tilted.pixel(48, 38), Pixel::TRANSPARENT);
    assert_eq!(no_capability.pixel(48, 38), flat.pixel(48, 38));
}

#[test]
fn tilt_flow_and_opacity_curves_change_actual_deposition() {
    let flow = dynamics(
        PressureControl::default(),
        control(vec![(0., 1.), (1., 0.1)]),
        PressureControl::default(),
    );
    let flat = draw(flow.clone(), 0., 2);
    let tilted = draw(flow, 90., 2);
    assert!(flat.pixel(48, 32).alpha() > tilted.pixel(48, 32).alpha() + 0.2);

    let opacity = dynamics(
        PressureControl::default(),
        PressureControl::default(),
        control(vec![(0., 1.), (1., 0.2)]),
    );
    let flat = draw(opacity.clone(), 0., 2);
    let tilted = draw(opacity, 90., 2);
    assert!(flat.pixel(48, 32).alpha() > tilted.pixel(48, 32).alpha() + 0.15);
    assert!(tilted.pixel(48, 32).alpha() <= 0.200001);
}

#[test]
fn secondary_tip_rejects_tilt_dynamics() {
    let tilt = TiltDynamics::new(
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
            tilt,
            ..Default::default()
        }
    )
    .is_err());
}
