use paint_core::*;

fn control(points: Vec<(f32, f32)>) -> PressureControl {
    PressureControl::new(PressureCurve::new(points).unwrap(), 0.).unwrap()
}

fn dynamics(
    size: PressureControl,
    flow: PressureControl,
    opacity: PressureControl,
) -> DistanceDynamics {
    DistanceDynamics::new(true, 32., size, flow, opacity).unwrap()
}

fn brush(dynamics: DistanceDynamics) -> BrushDefinition {
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
            distance: dynamics,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn draw(dynamics: DistanceDynamics) -> Document {
    let mut d = Document::new(64, 64).unwrap();
    d.begin_brush_stroke(brush(dynamics), InputPoint::new(16.5, 32.5, 1.))
        .unwrap();
    d.stroke_to(InputPoint::new(48.5, 32.5, 1.)).unwrap();
    d.end_stroke().unwrap();
    d
}

#[test]
fn distance_size_curve_changes_radius_along_the_stroke() {
    let d = draw(dynamics(
        control(vec![(0., 0.2), (1., 1.)]),
        PressureControl::default(),
        PressureControl::default(),
    ));
    assert_eq!(d.pixel(16, 38), Pixel::TRANSPARENT);
    assert!(d.pixel(48, 38).alpha() > 0.);
    assert_eq!(d.history_depth(), (1, 0));
}

#[test]
fn distance_flow_and_opacity_curves_change_actual_deposition() {
    let flow = draw(dynamics(
        PressureControl::default(),
        control(vec![(0., 1.), (1., 0.1)]),
        PressureControl::default(),
    ));
    assert!(flow.pixel(16, 32).alpha() > flow.pixel(48, 32).alpha() + 0.2);

    let opacity = draw(dynamics(
        PressureControl::default(),
        PressureControl::default(),
        control(vec![(0., 1.), (1., 0.2)]),
    ));
    assert!(
        opacity.pixel(16, 32).alpha() > opacity.pixel(48, 32).alpha() + 0.15,
        "start={} end={}",
        opacity.pixel(16, 32).alpha(),
        opacity.pixel(48, 32).alpha()
    );
    assert!(opacity.pixel(48, 32).alpha() <= 0.200001);
}

#[test]
fn disabled_distance_dynamics_preserves_pixels_and_history() {
    let mut reference = Document::new(64, 64).unwrap();
    reference
        .begin_brush_stroke(
            brush(DistanceDynamics::default()),
            InputPoint::new(16.5, 32.5, 1.),
        )
        .unwrap();
    reference
        .stroke_to(InputPoint::new(48.5, 32.5, 1.))
        .unwrap();
    reference.end_stroke().unwrap();
    let plain = draw(DistanceDynamics::default());
    for y in 0..64 {
        for x in 0..64 {
            assert_eq!(plain.pixel(x, y), reference.pixel(x, y));
        }
    }
    assert_eq!(plain.history_depth(), (1, 0));
}

#[test]
fn secondary_tip_rejects_distance_dynamics() {
    let distance = DistanceDynamics::new(
        true,
        32.,
        Default::default(),
        Default::default(),
        Default::default(),
    )
    .unwrap();
    assert!(SecondaryTip::new(
        8.,
        BrushTip::default(),
        None,
        BrushDynamics {
            distance,
            ..Default::default()
        }
    )
    .is_err());
}
