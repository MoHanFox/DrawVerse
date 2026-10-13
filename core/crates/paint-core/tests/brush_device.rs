use paint_core::*;
use std::sync::Arc;
fn brush(angle: AngleControl, tilt: bool, minimum: f32) -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            spacing: 1.,
            ..Default::default()
        },
        tip: BrushTip {
            roundness: 0.2,
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        dynamics: BrushDynamics {
            tip: TipDynamics::new(angle, tilt, minimum).unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn dot(definition: BrushDefinition, point: InputPoint) -> Document {
    let mut doc = Document::new(40, 40).unwrap();
    doc.begin_brush_stroke(definition, point).unwrap();
    doc.end_stroke().unwrap();
    doc
}
#[test]
fn rotation_and_tilt_azimuth_require_capabilities_and_change_real_pixels() {
    for (control, capability) in [(AngleControl::PenRotation, 4), (AngleControl::Tilt, 2)] {
        let mut point = InputPoint::new(16.5, 16.5, 1.);
        point.rotation = 90.;
        point.tilt_y = 60.;
        point.capabilities = capability;
        let definition = brush(control, false, 0.01);
        let doc = dot(definition.clone(), point);
        assert!(doc.pixel(16, 22).alpha() > 0.9);
        assert_eq!(doc.pixel(22, 16), Pixel::TRANSPARENT);
        point.capabilities = 0;
        let fallback = dot(definition, point);
        assert!(fallback.pixel(22, 16).alpha() > 0.9);
        assert_eq!(fallback.pixel(16, 22), Pixel::TRANSPARENT);
    }
    for minimum in [0., 1.01, f32::NAN] {
        assert!(TipDynamics::new(AngleControl::Fixed, true, minimum).is_err());
    }
}
#[test]
fn tilt_roundness_has_minimum_and_missing_device_keeps_base_shape() {
    let mut definition = brush(AngleControl::Fixed, true, 0.75);
    definition.tip.roundness = 1.;
    let mut point = InputPoint::new(16.5, 16.5, 1.);
    point.tilt_x = 60.;
    point.capabilities = 2;
    let doc = dot(definition.clone(), point);
    assert!(doc.pixel(16, 21).alpha() > 0.9);
    assert_eq!(doc.pixel(16, 23), Pixel::TRANSPARENT);
    point.capabilities = 0;
    let fallback = dot(definition, point);
    assert!(fallback.pixel(16, 23).alpha() > 0.9);
}
#[test]
fn direction_follows_segments_and_preserves_first_dab_fallback() {
    let mut doc = Document::new(40, 40).unwrap();
    doc.begin_brush_stroke(
        brush(AngleControl::Direction, false, 0.01),
        InputPoint::new(16.5, 8.5, 1.),
    )
    .unwrap();
    assert!(doc.pixel(22, 8).alpha() > 0.9);
    doc.stroke_to(InputPoint::new(16.5, 24.5, 1.)).unwrap();
    doc.stroke_to(InputPoint::new(32.5, 24.5, 1.)).unwrap();
    doc.end_stroke().unwrap();
    assert!(doc.pixel(38, 24).alpha() > 0.9);
    assert!(doc.pixel(16, 30).alpha() > 0.9);
    assert_eq!(doc.pixel(22, 16), Pixel::TRANSPARENT);
    let after = doc.pixel(16, 30);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(16, 30), Pixel::TRANSPARENT);
    doc.redo().unwrap();
    assert_eq!(doc.pixel(16, 30), after);
}

#[test]
fn direction_is_independent_of_collinear_input_frequency() {
    let mut whole = Document::new(40, 40).unwrap();
    let mut split = Document::new(40, 40).unwrap();
    for doc in [&mut whole, &mut split] {
        doc.begin_brush_stroke(
            brush(AngleControl::Direction, false, 0.01),
            InputPoint::new(16.5, 8.5, 1.),
        )
        .unwrap();
    }
    split.stroke_to(InputPoint::new(16.5, 12.5, 1.)).unwrap();
    for doc in [&mut whole, &mut split] {
        doc.stroke_to(InputPoint::new(16.5, 24.5, 1.)).unwrap();
        doc.end_stroke().unwrap();
    }
    for y in 0..40 {
        for x in 0..40 {
            assert_eq!(whole.pixel(x, y), split.pixel(x, y));
        }
    }
}

#[test]
fn device_geometry_is_shared_by_erase_mask_and_finger_smudge() {
    let mut point = InputPoint::new(16.5, 16.5, 1.);
    point.rotation = 90.;
    point.capabilities = 4;
    for mode in [
        BrushMode::Erase,
        BrushMode::Paint,
        BrushMode::Smudge {
            all_layers: false,
            finger_painting: true,
            pressure_strength: false,
        },
    ] {
        let mut doc = Document::new(40, 40).unwrap();
        if !matches!(mode, BrushMode::Smudge { .. }) {
            doc.initialize_white_background().unwrap();
        }
        if mode == BrushMode::Paint {
            doc.add_mask(1).unwrap();
        }
        doc.set_selection(
            Selection::default()
                .apply(
                    SelectionShape::geometry(SelectionKind::Rectangle, 0., 16., 40., 24., true),
                    SelectionOperation::Replace,
                    40,
                    40,
                )
                .unwrap(),
        )
        .unwrap();
        let mut definition = brush(AngleControl::PenRotation, false, 0.01);
        definition.brush.mode = mode;
        doc.begin_brush_stroke(definition, point).unwrap();
        doc.end_stroke().unwrap();
        let vertical = doc.pixel(16, 22).alpha();
        let horizontal = doc.pixel(22, 16).alpha();
        if matches!(mode, BrushMode::Smudge { .. }) {
            assert!(vertical > 0.9);
            assert_eq!(horizontal, 0.);
            assert_eq!(doc.pixel(16, 10).alpha(), 0.);
        } else {
            assert!(vertical < 0.1);
            assert_eq!(horizontal, 1.);
            assert_eq!(doc.pixel(16, 10).alpha(), 1.);
        }
        let after = doc.pixel(16, 22);
        doc.undo().unwrap();
        doc.redo().unwrap();
        assert_eq!(doc.pixel(16, 22), after);
    }
}

#[test]
fn in_flight_geometry_keeps_captured_definition() {
    let mut definition = brush(AngleControl::PenRotation, false, 0.01);
    let mut point = InputPoint::new(16.5, 8.5, 1.);
    point.rotation = 90.;
    point.capabilities = 4;
    let mut doc = Document::new(40, 40).unwrap();
    doc.begin_brush_stroke(definition.clone(), point).unwrap();
    definition.dynamics.tip = TipDynamics::default();
    point.y = 24.5;
    doc.stroke_to(point).unwrap();
    doc.end_stroke().unwrap();
    assert!(doc.pixel(16, 30).alpha() > 0.9);
    assert_eq!(doc.pixel(22, 16), Pixel::TRANSPARENT);
    let fixed = dot(definition, point);
    assert!(fixed.pixel(22, 24).alpha() > 0.9);
}
#[test]
fn image_rotated_corners_survive_and_rotation_interpolation_crosses_zero() {
    let mut definition = brush(AngleControl::PenRotation, false, 0.01);
    definition.tip.roundness = 1.;
    definition.mask = Some(Arc::new(BrushMask::new(3, 2, vec![255; 6]).unwrap()));
    let mut point = InputPoint::new(16.5, 16.5, 1.);
    point.rotation = 45.;
    point.capabilities = 4;
    let doc = dot(definition.clone(), point);
    assert!(doc.pixel(18, 25).alpha() > 0.);
    definition.mask = Some(Arc::new(BrushMask::new(3, 1, vec![255, 0, 0]).unwrap()));
    let mut doc = Document::new(40, 40).unwrap();
    point.x = 8.5;
    point.rotation = 350.;
    doc.begin_brush_stroke(definition, point).unwrap();
    point.x = 24.5;
    point.rotation = 10.;
    doc.stroke_to(point).unwrap();
    doc.end_stroke().unwrap();
    assert!(doc.pixel(12, 16).alpha() > 0.5);
}
