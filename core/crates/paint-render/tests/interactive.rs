use paint_core::*;
use paint_render::*;
use paint_task::CancellationToken;

fn seed(mode: BlendMode) -> Document {
    let mut d = Document::new(130, 90).unwrap();
    d.initialize_white_background().unwrap();
    let owner = d.add_layer("moving").unwrap();
    d.begin_stroke(
        Brush {
            radius: 30.,
            color: Pixel::from_straight([0.7, 0.1, 0.3, 0.6]).unwrap(),
            ..Default::default()
        },
        InputPoint::new(50., 40., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    let mask = d.add_mask(owner).unwrap();
    d.begin_stroke(
        Brush {
            radius: 18.,
            color: Pixel::WHITE,
            ..Default::default()
        },
        InputPoint::new(55., 40., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    d.set_mask_linked(mask, false).unwrap();
    let group = d.group_layer(owner, "masked ancestor").unwrap();
    let mut a = d.layer(group).unwrap().appearance();
    a.blend = mode;
    d.set_layer_appearance(group, a).unwrap();
    d.set_layer_properties(
        group,
        LayerProperties {
            visible: true,
            opacity: 0.6,
        },
    )
    .unwrap();
    d.add_mask(group).unwrap();
    d.begin_stroke(
        Brush {
            radius: 12.,
            color: Pixel::WHITE,
            ..Default::default()
        },
        InputPoint::new(45., 35., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    d.begin_layer_move(owner, true).unwrap();
    assert!(!d.layer_move_deferred());
    d.update_layer_move(-7, 3).unwrap();
    d
}

#[test]
fn resident_proxy_keeps_requested_resolution_and_original_composition() {
    for mode in [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::PassThrough,
    ] {
        let d = seed(mode);
        let snapshot = d.snapshot();
        let depth = d.history_depth();
        let request = RenderRequest {
            region: Region {
                x: -20.,
                y: -10.,
                width: 180.,
                height: 120.,
            },
            width: 700,
            height: 37,
            format: PixelFormat::LinearRgba32F,
        };
        let frame =
            render_sampled_preview(&snapshot, request, &CancellationToken::default()).unwrap();
        assert_eq!((frame.width, frame.height), (700, 37));
        let FramePixels::Linear(values) = frame.pixels else {
            panic!()
        };
        let encoded = render_sampled_preview(
            &snapshot,
            RenderRequest {
                format: PixelFormat::SrgbRgba8Premultiplied,
                ..request
            },
            &CancellationToken::default(),
        )
        .unwrap();
        let FramePixels::Srgb(bytes) = encoded.pixels else {
            panic!()
        };
        for y in 0..37 {
            for x in 0..700 {
                let sx = (-20. + (x as f64 + 0.5) * 180. / 700.).floor();
                let sy = (-10. + (y as f64 + 0.5) * 120. / 37.).floor();
                let reference = if sx < 0. || sy < 0. {
                    Pixel::TRANSPARENT
                } else {
                    snapshot.pixel(sx as u32, sy as u32)
                };
                let index = y * 700 + x;
                assert_eq!(values[index], reference.components());
                for (actual, expected) in bytes[index * 4..index * 4 + 4]
                    .iter()
                    .zip(display_pixel(reference.components()))
                {
                    assert!(actual.abs_diff(expected) <= 1);
                }
            }
        }
        assert_eq!(d.history_depth(), depth);
        let cancelled = CancellationToken::default();
        cancelled.cancel();
        assert_eq!(
            render_sampled_preview(&snapshot, request, &cancelled).unwrap_err(),
            RenderError::Cancelled
        );
    }
}

#[test]
fn coarse_preview_matches_original_pixel_centres_for_masks_groups_and_offsets() {
    for mode in [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::PassThrough,
    ] {
        let mut d = seed(mode);
        let depth = d.history_depth();
        let snapshot = d.snapshot();
        let request = RenderRequest {
            region: Region {
                x: -20.,
                y: -10.,
                width: 180.,
                height: 120.,
            },
            width: 1024,
            height: 768,
            format: PixelFormat::LinearRgba32F,
        };
        let frame = render_interactive(&snapshot, request, &CancellationToken::default()).unwrap();
        assert_eq!((frame.width, frame.height), (1024, 768));
        assert_eq!(frame.revision, snapshot.revision);
        let FramePixels::Linear(values) = frame.pixels else {
            panic!()
        };
        for y in 0..frame.height {
            let sy = (request.region.y
                + (f64::from(y) + 0.5) * request.region.height / f64::from(frame.height))
            .floor();
            for x in 0..frame.width {
                let sx = (request.region.x
                    + (f64::from(x) + 0.5) * request.region.width / f64::from(frame.width))
                .floor();
                let reference = if sx < 0. || sy < 0. {
                    Pixel::TRANSPARENT
                } else {
                    snapshot.pixel(sx as u32, sy as u32)
                };
                assert_eq!(
                    values[(y * frame.width + x) as usize],
                    reference.components()
                );
            }
        }
        let small = RenderRequest {
            region: Region {
                x: 0.,
                y: 0.,
                width: 130.,
                height: 90.,
            },
            width: 130,
            height: 90,
            format: PixelFormat::SrgbRgba8Premultiplied,
        };
        let FramePixels::Srgb(actual) =
            render_interactive(&snapshot, small, &CancellationToken::default())
                .unwrap()
                .pixels
        else {
            panic!()
        };
        let FramePixels::Srgb(reference) = CpuRenderer
            .render(&snapshot, small, &CancellationToken::default())
            .unwrap()
            .pixels
        else {
            panic!()
        };
        assert!(actual
            .iter()
            .zip(reference)
            .all(|(a, b)| a.abs_diff(b) <= 1));
        assert_eq!(d.history_depth(), depth);
        d.commit_layer_move(-7, 3).unwrap();
        assert_eq!(d.history_depth().0, depth.0 + 1);
        assert_eq!(
            d.snapshot().read_row(0, 40, 130).unwrap(),
            snapshot.read_row(0, 40, 130).unwrap()
        );
        let full = CpuRenderer
            .render(&d.snapshot(), small, &CancellationToken::default())
            .unwrap();
        assert_eq!((full.width, full.height), (130, 90));
    }
}

#[test]
fn interactive_preview_validates_requests_and_honours_cancellation() {
    let d = Document::new(64, 64).unwrap();
    let mut request = RenderRequest {
        region: Region {
            x: 0.,
            y: 0.,
            width: 64.,
            height: 64.,
        },
        width: 4096,
        height: 1024,
        format: PixelFormat::LinearRgba32F,
    };
    let token = CancellationToken::default();
    token.cancel();
    assert!(matches!(
        render_interactive(&d.snapshot(), request, &token),
        Err(RenderError::Cancelled)
    ));
    request.width = 0;
    assert!(matches!(
        render_interactive(&d.snapshot(), request, &CancellationToken::default()),
        Err(RenderError::InvalidRequest)
    ));
}

#[test]
fn sparse_large_canvas_preview_keeps_exact_samples_and_clips_empty_viewports() {
    for width in [16_384, 1_000_000] {
        let mut d = Document::new(width, 64).unwrap();
        d.initialize_white_background().unwrap();
        let owner = d.add_layer("distant").unwrap();
        d.begin_stroke(
            Brush {
                radius: 15.,
                color: Pixel::from_straight([0.8, 0.2, 0.4, 0.6]).unwrap(),
                ..Default::default()
            },
            InputPoint::new(f64::from(width) / 2., 32., 1.),
        )
        .unwrap();
        d.end_stroke().unwrap();
        d.add_mask(owner).unwrap();
        d.begin_layer_move(owner, false).unwrap();
        d.update_layer_move(7, -3).unwrap();
        let snapshot = d.snapshot();
        for x in [-f64::from(width), -32., 0., f64::from(width)] {
            let request = RenderRequest {
                region: Region {
                    x,
                    y: -8.,
                    width: f64::from(width) + 64.,
                    height: 80.,
                },
                width: 1024,
                height: 16,
                format: PixelFormat::LinearRgba32F,
            };
            let frame =
                render_interactive(&snapshot, request, &CancellationToken::default()).unwrap();
            assert_eq!((frame.width, frame.height), (1024, 16));
            let FramePixels::Linear(pixels) = frame.pixels else {
                panic!()
            };
            for y in 0..frame.height {
                for x in 0..frame.width {
                    let sx = (request.region.x
                        + (f64::from(x) + 0.5) * request.region.width / f64::from(frame.width))
                    .floor();
                    let sy = (request.region.y
                        + (f64::from(y) + 0.5) * request.region.height / f64::from(frame.height))
                    .floor();
                    let expected = if sx < 0. || sy < 0. {
                        Pixel::TRANSPARENT
                    } else {
                        snapshot.pixel(sx as u32, sy as u32)
                    };
                    assert_eq!(
                        pixels[(y * frame.width + x) as usize],
                        expected.components()
                    );
                }
            }
        }
    }
}
