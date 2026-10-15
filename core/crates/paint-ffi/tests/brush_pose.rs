use paint_ffi::*;
use std::{
    mem::size_of,
    ptr,
    time::{Duration, Instant},
};
#[test]
fn pose_reuses_binding_retains_revision_and_clears_failures() {
    let mut catalog = paint_core::BrushCatalog::default();
    let mut settings = catalog.selected().settings().clone();
    settings.diameter = 16.;
    settings.tip.roundness = 0.2;
    settings.tip.pressure_flow = false;
    settings.dynamics.tip =
        paint_core::TipDynamics::new(paint_core::AngleControl::PenRotation, true, 0.2).unwrap();
    catalog.edit("round-pressure", settings).unwrap();
    let json = paint_io::brush_state::encode(&catalog).unwrap();
    // All sized DTOs, ID/JSON and writable outputs live for these synchronous calls.
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let mut preparation = 0;
        assert_eq!(
            paint_brush_library_prepare_state(
                library,
                json.as_ptr(),
                json.len() as u64,
                &mut preparation
            ),
            PAINT_OK
        );
        let mut pose = ptr::null_mut();
        assert_eq!(paint_brush_pose_create(&mut pose), PAINT_OK);
        assert_eq!(
            paint_brush_pose_bind_prepared(
                pose,
                library,
                preparation,
                b"round-pressure".as_ptr(),
                14,
                8.
            ),
            PAINT_OK
        );
        let point = PaintPoint {
            struct_size: size_of::<PaintPoint>() as u32,
            pressure: 0.5,
            rotation: 90.,
            tilt_x: 60.,
            capabilities: 7,
            tool: PAINT_TOOL_PEN,
            ..Default::default()
        };
        let mut out = PaintTipPose::default();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let status = paint_brush_pose_read(pose, &point, 1, 0, 0., 0., &mut out);
            if status == PAINT_BUSY {
                assert_eq!(out.radius, 0.);
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            assert_eq!(status, PAINT_OK);
            break;
        }
        assert_eq!(out.radius, 4.);
        let mut composite = PaintCompositeTipPose::default();
        assert_eq!(
            paint_brush_pose_read_composite(pose, &point, 1, 0, 0., 0., &mut composite),
            PAINT_OK
        );
        assert_eq!(composite.flags, 0);
        assert_eq!(composite.primary.radius, out.radius);
        assert_eq!(composite.primary.roundness, out.roundness);
        assert_eq!(composite.secondary.radius, 0.);

        assert!((out.angle - 90.).abs() < 1e-5);
        assert!((out.roundness - 0.1).abs() < 1e-5);
        assert_eq!(
            paint_brush_pose_read(pose, &point, 0, 0, 0., 0., &mut out),
            PAINT_OK
        );
        assert_eq!(out.radius, 8.);
        assert_eq!(
            paint_brush_pose_read(pose, &point, 2, 0, 0., 0., &mut out),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(out.radius, 0.);
        assert_eq!(out.angle, 0.);
        assert_eq!(
            paint_brush_pose_bind_prepared(
                pose,
                library,
                preparation,
                b"round-pressure".as_ptr(),
                14,
                12.
            ),
            PAINT_OK
        );
        assert_eq!(
            paint_brush_pose_read(pose, &point, 1, 0, 0., 0., &mut out),
            PAINT_OK
        );
        assert_eq!(out.radius, 6.);
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
        assert_eq!(
            paint_brush_pose_read(pose, &point, 1, 0, 0., 0., &mut out),
            PAINT_OK
        );
        assert_eq!(out.radius, 6.);
        assert_eq!(paint_brush_pose_clear(pose), PAINT_OK);
        assert_eq!(
            paint_brush_pose_read(pose, &point, 1, 0, 0., 0., &mut out),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(out.radius, 0.);
        let stale = pose;
        assert_eq!(paint_brush_pose_destroy(&mut pose), PAINT_OK);
        assert!(pose.is_null());
        assert_eq!(
            paint_brush_pose_read(stale, &point, 1, 0, 0., 0., &mut out),
            PAINT_INVALID_HANDLE
        );
    }
}

#[test]
fn composite_pose_captures_pair_and_clears_all_error_outputs() {
    use paint_core::*;
    assert_eq!(size_of::<PaintCompositeTipPose>(), 64);
    let mut catalog = BrushCatalog::default();
    let mut settings = catalog.selected().settings().clone();
    settings.diameter = 24.;
    settings.dual = DualBrush::new(true, DualMode::Multiply, SecondaryTip::default())
        .with_sizing(
            DualSizing::new(DualSizeMode::Relative, 8., DualWidthRule::FixedWidth).unwrap(),
        )
        .with_color(
            DualColor::new(
                true,
                true,
                true,
                DualColorOrder::SecondBehind,
                BlendMode::Normal,
                BlendMode::Normal,
            )
            .unwrap(),
        );
    catalog.edit("round-pressure", settings).unwrap();
    let json = paint_io::brush_state::encode(&catalog).unwrap();
    // Synchronous calls borrow live sized DTOs and independent outputs only.
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let mut preparation = 0;
        assert_eq!(
            paint_brush_library_prepare_state(
                library,
                json.as_ptr(),
                json.len() as u64,
                &mut preparation
            ),
            PAINT_OK
        );
        let mut pose = ptr::null_mut();
        assert_eq!(paint_brush_pose_create(&mut pose), PAINT_OK);
        assert_eq!(
            paint_brush_pose_bind_prepared(
                pose,
                library,
                preparation,
                b"round-pressure".as_ptr(),
                14,
                10.
            ),
            PAINT_OK
        );
        let mut point = PaintPoint {
            struct_size: size_of::<PaintPoint>() as u32,
            pressure: 0.5,
            tool: PAINT_TOOL_PEN,
            ..Default::default()
        };
        let mut out = PaintCompositeTipPose::default();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let status = paint_brush_pose_read_composite(pose, &point, 1, 0, 0., 0., &mut out);
            if status == PAINT_BUSY {
                assert_eq!(out.flags, 0);
                assert_eq!(out.primary.radius, 0.);
                assert_eq!(out.secondary.radius, 0.);
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            assert_eq!(status, PAINT_OK);
            break;
        }
        assert_eq!(
            out.flags,
            PAINT_POSE_SECONDARY | PAINT_POSE_SECONDARY_OVERFLOW
        );
        assert_eq!(out.primary.radius, 5.);
        assert_eq!(out.secondary.radius, 9.);
        assert_eq!(out.reserved, 0);
        assert_eq!(out.primary.struct_size, 24);
        assert_eq!(out.secondary.struct_size, 24);
        let mut single = PaintTipPose::default();
        assert_eq!(
            paint_brush_pose_read(pose, &point, 1, 0, 0., 0., &mut single),
            PAINT_OK
        );
        assert_eq!(single.radius, out.primary.radius);
        assert_eq!(single.angle, out.primary.angle);
        assert_eq!(
            paint_brush_pose_read_composite(pose, &point, 0, 0, 0., 0., &mut out),
            PAINT_OK
        );
        assert_eq!(out.primary.radius, 10.);
        assert_eq!(out.secondary.radius, 14.);
        for (pressure, direction, dx, dy) in [
            (2, 0, 0., 0.),
            (1, 2, 0., 0.),
            (1, 0, 1., 0.),
            (1, 1, 0., 0.),
            (1, 1, f64::NAN, 1.),
        ] {
            assert_eq!(
                paint_brush_pose_read_composite(
                    pose, &point, pressure, direction, dx, dy, &mut out
                ),
                PAINT_INVALID_ARGUMENT
            );
            assert_eq!(out.flags, 0);
            assert_eq!(out.primary.radius, 0.);
            assert_eq!(out.secondary.radius, 0.);
            assert_eq!(out.primary.angle, 0.);
            assert_eq!(out.secondary.roundness, 0.);
        }
        point.struct_size = 0;
        assert_eq!(
            paint_brush_pose_read_composite(pose, &point, 1, 0, 0., 0., &mut out),
            PAINT_INVALID_ARGUMENT
        );
        point.struct_size = size_of::<PaintPoint>() as u32;
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
        assert_eq!(
            paint_brush_pose_read_composite(pose, &point, 1, 0, 0., 0., &mut out),
            PAINT_OK
        );
        assert_eq!(out.secondary.radius, 9.);
        assert_eq!(paint_brush_pose_clear(pose), PAINT_OK);
        assert_eq!(
            paint_brush_pose_read_composite(pose, &point, 1, 0, 0., 0., &mut out),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(out.flags, 0);
        assert_eq!(out.secondary.radius, 0.);
        let stale = pose;
        assert_eq!(paint_brush_pose_destroy(&mut pose), PAINT_OK);
        assert_eq!(
            paint_brush_pose_read_composite(stale, &point, 1, 0, 0., 0., &mut out),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(out.primary.radius, 0.);
    }
}
