use paint_core::{BlendMode, Brush, Document, InputPoint, LayerId, LayerProperties, Pixel};
use paint_render::{
    CachedCpuRenderer, CpuRenderer, FramePixels, PixelFormat, Region, RenderRequest, Renderer,
};
use paint_task::CancellationToken;

fn paint(doc: &mut Document, salt: u32) {
    for y in 0..5 {
        for x in 0..6 {
            let k = x + 7 * y + salt;
            doc.begin_stroke(
                Brush {
                    radius: 18. + (k % 7) as f32,
                    color: Pixel::from_straight([
                        (k % 5) as f32 / 5.,
                        (k % 7) as f32 / 7.,
                        (k % 3) as f32 / 3.,
                        0.25 + (k % 3) as f32 * 0.25,
                    ])
                    .unwrap(),
                    ..Default::default()
                },
                InputPoint::new(x as f64 * 64. + 12., y as f64 * 64. + 17., 1.),
            )
            .unwrap();
            doc.end_stroke().unwrap();
        }
    }
}
fn fixture(white: bool, patterned_background: bool) -> (Document, LayerId) {
    let mut doc = Document::new(333, 269).unwrap();
    if white {
        doc.initialize_white_background().unwrap();
    }
    if patterned_background {
        paint(&mut doc, 11);
    }
    let id = doc.add_layer("patterned moving layer").unwrap();
    paint(&mut doc, 0);
    (doc, id)
}
fn request(format: PixelFormat, lod: u32) -> RenderRequest {
    RenderRequest {
        region: Region {
            x: -3.5,
            y: -1.25,
            width: 345.,
            height: 280.,
        },
        width: (345 / (1 << lod)).max(1),
        height: (280 / (1 << lod)).max(1),
        format,
    }
}
fn exact(renderer: &CachedCpuRenderer, doc: &Document, r: RenderRequest) {
    let snapshot = doc.snapshot();
    let token = CancellationToken::default();
    let a = renderer.render(&snapshot, r, &token).unwrap();
    let b = CpuRenderer.render(&snapshot, r, &token).unwrap();
    assert_eq!(
        (a.region, a.width, a.height, a.lod, a.revision),
        (b.region, b.width, b.height, b.lod, b.revision)
    );
    match (a.pixels, b.pixels) {
        (FramePixels::Srgb(a), FramePixels::Srgb(b)) => assert_eq!(a, b),
        (FramePixels::Linear(a), FramePixels::Linear(b)) => assert_eq!(a, b),
        _ => panic!("format"),
    }
}
#[test]
fn patterned_signed_moves_return_and_non_aligned_fallback_are_pixel_exact() {
    for white in [false, true] {
        for background in [false, true] {
            for format in [
                PixelFormat::LinearRgba32F,
                PixelFormat::SrgbRgba8Premultiplied,
            ] {
                for lod in [0, 2, 6] {
                    let (mut doc, id) = fixture(white, background);
                    let cache = CachedCpuRenderer::default();
                    let r = request(format, lod);
                    exact(&cache, &doc, r);
                    for (dx, dy) in [
                        (64, 0),
                        (0, -64),
                        (-64, 64),
                        (0, 0),
                        (383, 127),
                        (1, 0),
                        (-384, -127),
                        (64, 64),
                        (-64, -64),
                    ] {
                        doc.move_layer(id, dx, dy).unwrap();
                        exact(&cache, &doc, r);
                    }
                }
            }
        }
    }
}
#[test]
fn cold_evicted_edited_and_changed_layer_configurations_remain_exact() {
    for budget in [0, 512, 96 * 1024 * 1024] {
        let (mut doc, id) = fixture(true, false);
        let cache = CachedCpuRenderer::with_cache_budget(budget).unwrap();
        let r = request(PixelFormat::SrgbRgba8Premultiplied, 2);
        doc.move_layer(id, -64, -64).unwrap();
        exact(&cache, &doc, r);
        doc.move_layer(id, 64, 64).unwrap();
        exact(&cache, &doc, r);
        paint(&mut doc, 4);
        doc.move_layer(id, 64, 0).unwrap();
        exact(&cache, &doc, r);
        doc.set_layer_properties(
            id,
            LayerProperties {
                visible: true,
                opacity: 0.4,
            },
        )
        .unwrap();
        doc.move_layer(id, 0, 64).unwrap();
        exact(&cache, &doc, r);
        let mut appearance = doc.layer(id).unwrap().appearance();
        appearance.fill = 0.5;
        appearance.blend = BlendMode::Multiply;
        doc.set_layer_appearance(id, appearance).unwrap();
        doc.move_layer(id, -64, 0).unwrap();
        exact(&cache, &doc, r);
        doc.set_layer_properties(
            id,
            LayerProperties {
                visible: false,
                opacity: 0.4,
            },
        )
        .unwrap();
        exact(&cache, &doc, r);
        let other = doc.add_layer("new raster").unwrap();
        paint(&mut doc, 3);
        exact(&cache, &doc, r);
        doc.remove_layer(other).unwrap();
        exact(&cache, &doc, r);
        assert!(cache.cache_stats().retained_bytes <= budget);
    }
}
#[test]
fn groups_linked_and_fixed_masks_are_exact_fallbacks() {
    for kind in 0..3 {
        let (mut doc, id) = fixture(true, false);
        let cache = CachedCpuRenderer::default();
        let r = request(PixelFormat::LinearRgba32F, 2);
        if kind == 0 {
            doc.group_layer(id, "PassThrough group").unwrap();
        } else {
            let mask = doc.add_mask(id).unwrap();
            doc.begin_stroke(
                Brush {
                    radius: 12.,
                    color: Pixel::from_straight([0., 0., 0., 1.]).unwrap(),
                    ..Default::default()
                },
                InputPoint::new(40., 40., 1.),
            )
            .unwrap();
            doc.end_stroke().unwrap();
            if kind == 2 {
                doc.set_mask_linked(mask, false).unwrap();
            }
        }
        exact(&cache, &doc, r);
        doc.move_layer(id, 64, 64).unwrap();
        exact(&cache, &doc, r);
    }
}
