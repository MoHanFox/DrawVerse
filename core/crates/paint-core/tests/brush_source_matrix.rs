use paint_core::*;

fn tilt_brush() -> BrushDefinition {
    let control =
        PressureControl::new(PressureCurve::new(vec![(0., 0.), (1., 1.)]).unwrap(), 0.).unwrap();
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            spacing: 0.25,
            color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
            ..Default::default()
        },
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        dynamics: BrushDynamics {
            tilt: TiltDynamics::new(
                true,
                control,
                PressureControl::default(),
                PressureControl::default(),
            ),
            matrix: DynamicsSourceMatrix::new(
                DynamicsSource::Tilt,
                DynamicsSource::All,
                DynamicsSource::All,
            ),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn draw(tilt: f32) -> Document {
    let mut d = Document::new(64, 64).unwrap();
    let mut start = InputPoint::new(16.5, 32.5, 1.);
    start.capabilities = 2;
    start.tilt_x = tilt;
    d.begin_brush_stroke(tilt_brush(), start).unwrap();
    let mut end = InputPoint::new(48.5, 32.5, 1.);
    end.capabilities = 2;
    end.tilt_x = tilt;
    d.stroke_to(end).unwrap();
    d.end_stroke().unwrap();
    d
}

#[test]
fn tilt_matrix_size_changes_real_radius() {
    let flat = draw(0.);
    let tilted = draw(90.);
    assert_eq!(flat.pixel(32, 38).alpha(), 0.);
    assert!(tilted.pixel(32, 38).alpha() > 0.5);
}
