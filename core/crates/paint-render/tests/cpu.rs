use paint_core::{Brush, Document, InputPoint, LayerProperties, Pixel};
use paint_render::{
    display_pixel, CpuRenderer, FramePixels, PixelFormat, Region, RenderError, RenderRequest,
    Renderer,
};
use paint_task::CancellationToken;
fn request(x: f64, y: f64, width: f64, height: f64, w: u32, h: u32) -> RenderRequest {
    RenderRequest {
        region: Region {
            x,
            y,
            width,
            height,
        },
        width: w,
        height: h,
        format: PixelFormat::LinearRgba32F,
    }
}
fn dab(doc: &mut Document, x: f64, y: f64, radius: f32, rgba: [f32; 4]) {
    doc.begin_stroke(
        Brush {
            radius,
            color: Pixel::from_straight(rgba).unwrap(),
            ..Brush::default()
        },
        InputPoint::new(x, y, 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
}
fn compare(a: [f32; 4], b: [f32; 4]) {
    for i in 0..4 {
        assert!((a[i] - b[i]).abs() < 1e-5, "{a:?} != {b:?}");
    }
}
#[test]
fn tiled_render_matches_reference_composition_and_layer_properties() {
    let mut doc = Document::new(130, 90).unwrap();
    dab(&mut doc, 62., 42., 25., [1., 0., 0., 0.8]);
    let second = doc.add_layer("Blue").unwrap();
    dab(&mut doc, 74., 48., 20., [0., 0., 1., 0.5]);
    doc.set_layer_properties(
        second,
        LayerProperties {
            visible: true,
            opacity: 0.7,
        },
    )
    .unwrap();
    for visible in [true, false] {
        doc.set_layer_properties(
            second,
            LayerProperties {
                visible,
                opacity: 0.7,
            },
        )
        .unwrap();
        let snap = doc.snapshot();
        let frame = CpuRenderer
            .render(
                &snap,
                request(0., 0., 130., 90., 130, 90),
                &CancellationToken::default(),
            )
            .unwrap();
        let FramePixels::Linear(values) = frame.pixels else {
            panic!("linear output");
        };
        for y in 0..90 {
            for x in 0..130 {
                compare(
                    values[(y * 130 + x) as usize],
                    snap.pixel(x, y).components(),
                );
            }
        }
    }
}
#[test]
fn lod_averages_after_compositing_and_preserves_cross_tile_edges() {
    let mut doc = Document::new(128, 64).unwrap();
    dab(&mut doc, 63., 32., 20., [1., 0., 0., 1.]);
    doc.add_layer("Top").unwrap();
    dab(&mut doc, 65., 32., 16., [0., 0., 1., 0.7]);
    let snap = doc.snapshot();
    let frame = CpuRenderer
        .render(
            &snap,
            request(0., 0., 128., 64., 2, 1),
            &CancellationToken::default(),
        )
        .unwrap();
    assert_eq!(frame.lod, 6);
    let FramePixels::Linear(values) = frame.pixels else {
        panic!();
    };
    for x in 0..2 {
        let mut expected = [0.; 4];
        for y in 0..64 {
            for px in x * 64..x * 64 + 64 {
                let p = snap.pixel(px, y).components();
                for i in 0..4 {
                    expected[i] += p[i] / 4096.;
                }
            }
        }
        compare(values[x as usize], expected);
    }
}
#[test]
fn huge_sparse_document_has_bounded_output_and_high_lod_includes_distant_tiles() {
    let mut doc = Document::new(1_000_000, 1_000_000).unwrap();
    dab(&mut doc, 900_000., 900_000., 12., [0., 1., 0., 1.]);
    let frame = CpuRenderer
        .render(
            &doc.snapshot(),
            request(0., 0., 1_000_000., 1_000_000., 1, 1),
            &CancellationToken::default(),
        )
        .unwrap();
    assert_eq!(frame.lod, 20);
    let FramePixels::Linear(values) = frame.pixels else {
        panic!();
    };
    assert_eq!(values.len(), 1);
    assert!(values[0][3] > 0. && values[0][3] < 1e-6);
    let crop = CpuRenderer
        .render(
            &doc.snapshot(),
            request(899_984., 899_984., 32., 32., 32, 32),
            &CancellationToken::default(),
        )
        .unwrap();
    let FramePixels::Linear(values) = crop.pixels else {
        panic!();
    };
    assert!(values[16 * 32 + 16][3] > 0.99);
}
#[test]
fn snapshot_isolation_crop_transparency_display_and_cancellation() {
    let mut doc = Document::new(64, 64).unwrap();
    let old = doc.snapshot();
    dab(&mut doc, 32., 32., 12., [0.5, 0.25, 0.1, 0.5]);
    let frame = CpuRenderer
        .render(
            &old,
            request(0., 0., 64., 64., 64, 64),
            &CancellationToken::default(),
        )
        .unwrap();
    let FramePixels::Linear(values) = frame.pixels else {
        panic!();
    };
    assert!(values.iter().all(|p| *p == [0.; 4]));
    let frame = CpuRenderer
        .render(
            &doc.snapshot(),
            request(-64., -64., 32., 32., 32, 32),
            &CancellationToken::default(),
        )
        .unwrap();
    let FramePixels::Linear(values) = frame.pixels else {
        panic!();
    };
    assert!(values.iter().all(|p| *p == [0.; 4]));
    assert_eq!(display_pixel([0.25, 0.25, 0.25, 0.5]), [94, 94, 94, 128]);
    assert_eq!(display_pixel([0.; 4]), [0; 4]);
    let token = CancellationToken::default();
    token.cancel();
    assert!(matches!(
        CpuRenderer.render(&doc.snapshot(), request(0., 0., 64., 64., 64, 64), &token),
        Err(RenderError::Cancelled)
    ));
    let mut invalid = request(0., 0., 64., 64., 4096, 4096);
    assert!(matches!(
        CpuRenderer.render(&doc.snapshot(), invalid, &CancellationToken::default()),
        Err(RenderError::InvalidRequest)
    ));
    invalid.region.x = f64::NAN;
    assert!(invalid.validate().is_err());
}
