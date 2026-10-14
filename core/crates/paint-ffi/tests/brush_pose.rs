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
