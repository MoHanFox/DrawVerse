use paint_core::*;
use paint_render::*;
use paint_task::CancellationToken;
fn request(w: u32, h: u32, outw: u32, outh: u32) -> RenderRequest {
    RenderRequest {
        region: Region {
            x: 0.,
            y: 0.,
            width: w as f64,
            height: h as f64,
        },
        width: outw,
        height: outh,
        format: PixelFormat::LinearRgba32F,
    }
}
#[test]
fn opaque_odd_canvas_thumbnail_edges_never_average_in_outside_transparency() {
    let mut d = Document::new(641, 479).unwrap();
    d.initialize_white_background().unwrap();
    let r = CachedCpuRenderer::default();
    for (w, h) in [(64, 48), (5, 4), (1, 1)] {
        let frame = r
            .render(
                &d.snapshot(),
                request(641, 479, w, h),
                &CancellationToken::default(),
            )
            .unwrap();
        let FramePixels::Linear(p) = frame.pixels else {
            unreachable!()
        };
        assert!(
            p.iter().all(|p| p.iter().all(|v| (*v - 1.).abs() < 2e-5)),
            "{w}x{h}: {:?}",
            p.last()
        );
    }
    // Genuine edge transparency is retained inside the canvas.
    d.set_active_layer(1).unwrap();
    d.begin_stroke(
        Brush {
            radius: 8.,
            mode: BrushMode::Erase,
            ..Default::default()
        },
        InputPoint::new(640., 478., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    let frame = r
        .render(
            &d.snapshot(),
            request(641, 479, 64, 48),
            &CancellationToken::default(),
        )
        .unwrap();
    let FramePixels::Linear(p) = frame.pixels else {
        unreachable!()
    };
    assert!(p.last().unwrap()[3] < 1.);
}
#[test]
fn clipping_flag_and_base_mask_pages_invalidate_cached_lods() {
    let mut d = Document::new(128, 128).unwrap();
    d.begin_stroke(
        Brush {
            radius: 30.,
            color: Pixel::WHITE,
            ..Default::default()
        },
        InputPoint::new(64., 64., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    let top = d.add_layer("top").unwrap();
    d.begin_stroke(
        Brush {
            radius: 55.,
            color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
            ..Default::default()
        },
        InputPoint::new(64., 64., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    let r = CachedCpuRenderer::default();
    for clipped in [false, true, false, true] {
        d.set_layer_clipping(top, clipped).unwrap();
        for size in [128, 32, 1] {
            let req = request(128, 128, size, size);
            let reference = CpuRenderer
                .render(&d.snapshot(), req, &CancellationToken::default())
                .unwrap();
            for _ in 0..2 {
                let got = r
                    .render(&d.snapshot(), req, &CancellationToken::default())
                    .unwrap();
                let (FramePixels::Linear(a), FramePixels::Linear(b)) =
                    (&got.pixels, &reference.pixels)
                else {
                    unreachable!()
                };
                assert_eq!(a, b);
            }
        }
    }
    let mask = d.add_mask(1).unwrap();
    d.begin_stroke(
        Brush {
            radius: 20.,
            color: Pixel::from_straight([0., 0., 0., 1.]).unwrap(),
            ..Default::default()
        },
        InputPoint::new(64., 64., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    for enabled in [true, false, true] {
        d.set_layer_properties(
            mask,
            LayerProperties {
                visible: enabled,
                opacity: 1.,
            },
        )
        .unwrap();
        let frame = r
            .render(
                &d.snapshot(),
                request(128, 128, 128, 128),
                &CancellationToken::default(),
            )
            .unwrap();
        let FramePixels::Linear(p) = frame.pixels else {
            unreachable!()
        };
        assert_eq!(p[64 * 128 + 64][3], if enabled { 0. } else { 1. });
    }
}
