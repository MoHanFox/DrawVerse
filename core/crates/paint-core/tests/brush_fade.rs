use paint_core::*;

fn control() -> PressureControl {
    PressureControl::new(PressureCurve::new(vec![(0., 0.), (1., 1.)]).unwrap(), 0.).unwrap()
}
fn flat() -> PressureControl {
    PressureControl::new(PressureCurve::new(vec![(0., 1.), (1., 1.)]).unwrap(), 0.).unwrap()
}
fn brush(fade: FadeDynamics) -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            spacing: 0.5,
            color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
            ..Default::default()
        },
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        dynamics: BrushDynamics {
            fade,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn draw(fade: FadeDynamics) -> Document {
    let mut d = Document::new(64, 64).unwrap();
    d.begin_brush_stroke(brush(fade), InputPoint::new(16.5, 32.5, 1.))
        .unwrap();
    for x in [24.5, 32.5, 40.5, 48.5, 56.5] {
        d.stroke_to(InputPoint::new(x, 32.5, 1.)).unwrap();
    }
    d.end_stroke().unwrap();
    d
}

#[test]
fn fade_size_steps_reduce_real_stamp_radius() {
    let faded = draw(FadeDynamics::new(true, 4, control(), flat(), flat()).unwrap());
    let plain = draw(FadeDynamics::default());
    assert!(plain.pixel(56, 38).alpha() > 0.5);
    assert_eq!(faded.pixel(56, 38).alpha(), 0.);
    assert_eq!(faded.history_depth(), (1, 0));
}

#[test]
fn disabled_fade_preserves_plain_pixels() {
    let faded = draw(FadeDynamics::default());
    let mut reference = Document::new(64, 64).unwrap();
    reference
        .begin_brush_stroke(
            brush(FadeDynamics::default()),
            InputPoint::new(16.5, 32.5, 1.),
        )
        .unwrap();
    for x in [24.5, 32.5, 40.5, 48.5, 56.5] {
        reference.stroke_to(InputPoint::new(x, 32.5, 1.)).unwrap();
    }
    reference.end_stroke().unwrap();
    for y in 0..64 {
        for x in 0..64 {
            assert_eq!(faded.pixel(x, y), reference.pixel(x, y));
        }
    }
}

#[test]
fn secondary_tip_rejects_its_own_fade_configuration() {
    let fade = FadeDynamics::new(true, 4, control(), flat(), flat()).unwrap();
    assert!(SecondaryTip::new(
        8.,
        BrushTip::default(),
        None,
        BrushDynamics {
            fade,
            ..Default::default()
        }
    )
    .is_err());
}
