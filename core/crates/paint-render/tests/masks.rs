use paint_core::*;
use paint_render::*;
use paint_task::CancellationToken;
fn request(size: u32) -> RenderRequest {
    RenderRequest {
        region: Region {
            x: 0.,
            y: 0.,
            width: 128.,
            height: 128.,
        },
        width: size,
        height: size,
        format: PixelFormat::LinearRgba32F,
    }
}
#[test]
fn mask_pages_are_cache_dependencies_and_all_lods_match_reference_after_edits() {
    let mut doc = Document::new(128, 128).unwrap();
    doc.initialize_white_background().unwrap();
    let mask = doc.add_mask(1).unwrap();
    let renderer = CachedCpuRenderer::default();
    for size in [128, 64, 32, 1] {
        for color in [0., 1., 0.5] {
            doc.begin_stroke(
                Brush {
                    radius: 40.,
                    color: Pixel::from_straight([color, color, color, 1.]).unwrap(),
                    ..Default::default()
                },
                InputPoint::new(64., 64., 1.),
            )
            .unwrap();
            doc.end_stroke().unwrap();
            for _ in 0..2 {
                let frame = renderer
                    .render(
                        &doc.snapshot(),
                        request(size),
                        &CancellationToken::default(),
                    )
                    .unwrap();
                let FramePixels::Linear(pixels) = frame.pixels else {
                    unreachable!()
                };
                let block = 128 / size;
                for y in 0..size {
                    for x in 0..size {
                        let mut sum = [0.; 4];
                        for py in y * block..(y + 1) * block {
                            for p in doc.read_row(x * block, py, block).unwrap() {
                                for (v, c) in sum.iter_mut().zip(p.components()) {
                                    *v += c / (block * block) as f32;
                                }
                            }
                        }
                        for (actual, expected) in
                            pixels[(y * size + x) as usize].into_iter().zip(sum)
                        {
                            assert!(
                                (actual - expected).abs() < 2e-5,
                                "{size} {actual} {expected}"
                            );
                        }
                    }
                }
            }
        }
    }
    doc.set_layer_properties(
        mask,
        LayerProperties {
            visible: false,
            opacity: 1.,
        },
    )
    .unwrap();
    let frame = renderer
        .render(&doc.snapshot(), request(64), &CancellationToken::default())
        .unwrap();
    let FramePixels::Linear(p) = frame.pixels else {
        unreachable!()
    };
    assert!(p.iter().all(|p| *p == [1.; 4]));
}
#[test]
fn enormous_implicit_canvas_preview_is_white_without_allocating_document_tiles() {
    let mut doc = Document::new(1_000_000, 1_000_000).unwrap();
    doc.initialize_white_background().unwrap();
    let mut r = request(16);
    r.region.width = 1_000_000.;
    r.region.height = 1_000_000.;
    let f = CpuRenderer
        .render(&doc.snapshot(), r, &CancellationToken::default())
        .unwrap();
    let FramePixels::Linear(p) = f.pixels else {
        unreachable!()
    };
    assert!(p.iter().all(|p| *p == [1.; 4]));
    assert_eq!(doc.tile_count(), 0);
}
