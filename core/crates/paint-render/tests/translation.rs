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
        width: 345u32.div_ceil(1 << lod),
        height: 280u32.div_ceil(1 << lod),
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

#[test]
fn actual_translation_counts_are_distinct_and_unsafe_backgrounds_are_rejected() {
    for background in [false, true] {
        let (mut doc, id) = fixture(true, background);
        let cache = CachedCpuRenderer::default();
        let r = request(PixelFormat::LinearRgba32F, 2);
        exact(&cache, &doc, r);
        doc.move_layer(id, 64, 64).unwrap();
        let (_, d) = cache.render_diagnosed(&doc.snapshot(), r, &CancellationToken::default());
        if background {
            assert_eq!(d.translation_reason, 4);
            assert_eq!(d.translations, 0);
        } else {
            assert_eq!(d.translation_reason, 1);
            assert!(d.translations > 0);
            assert!(d.misses > 0);
            assert_eq!(d.hits, 0);
        }
        exact(&cache, &doc, r);
        let (_, warm) = cache.render_diagnosed(&doc.snapshot(), r, &CancellationToken::default());
        assert_eq!(warm.translations, 0);
        assert_eq!(warm.misses, 0);
        assert!(warm.hits > 0);
    }
}
#[test]
fn non_aligned_special_stacks_and_configuration_changes_do_not_translate() {
    for change in 0..7 {
        let (mut doc, id) = fixture(true, false);
        let cache = CachedCpuRenderer::default();
        let r = request(PixelFormat::SrgbRgba8Premultiplied, 2);
        exact(&cache, &doc, r);
        match change {
            0 => doc.move_layer(id, 383, 127).unwrap(),
            1 => doc.move_layer(id, 1, 0).unwrap(),
            2 => {
                doc.group_layer(id, "group").unwrap();
                doc.move_layer(id, 64, 0).unwrap();
            }
            3 => {
                doc.add_mask(id).unwrap();
                doc.move_layer(id, 64, 0).unwrap();
            }
            4 => {
                let mut a = doc.layer(id).unwrap().appearance();
                a.blend = BlendMode::Multiply;
                doc.set_layer_appearance(id, a).unwrap();
                doc.move_layer(id, 64, 0).unwrap();
            }
            5 => {
                doc.set_layer_properties(
                    id,
                    LayerProperties {
                        visible: true,
                        opacity: 0.25,
                    },
                )
                .unwrap();
                doc.move_layer(id, 64, 0).unwrap();
            }
            _ => {
                doc.add_layer("empty but structure changed").unwrap();
                doc.move_layer(id, 64, 0).unwrap();
            }
        }
        let (_, d) = cache.render_diagnosed(&doc.snapshot(), r, &CancellationToken::default());
        assert_eq!(d.translations, 0);
        assert_eq!(
            d.translation_reason,
            if change < 2 {
                6
            } else if change < 5 {
                4
            } else {
                5
            }
        );
        exact(&cache, &doc, r);
    }
}
#[test]
fn edited_source_phase_and_panned_view_only_reuse_proven_existing_tiles() {
    let (mut doc, id) = fixture(true, false);
    let cache = CachedCpuRenderer::default();
    let r = request(PixelFormat::LinearRgba32F, 2);
    doc.move_layer(id, 1, 1).unwrap();
    exact(&cache, &doc, r);
    doc.begin_stroke(
        Brush {
            radius: 2.,
            color: Pixel::from_straight([0.8, 0.1, 0.3, 0.5]).unwrap(),
            ..Default::default()
        },
        InputPoint::new(70., 80., 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    doc.move_layer(id, 64, 64).unwrap();
    let (_, d) = cache.render_diagnosed(&doc.snapshot(), r, &CancellationToken::default());
    assert_eq!(d.translation_reason, 1);
    assert!(d.translations > 0);
    assert!(d.misses > 0);
    exact(&cache, &doc, r);
    doc.move_layer(id, -64, -64).unwrap();
    let mut pan = r;
    pan.region.x = 67.25;
    pan.region.y = 65.5;
    pan.region.width = 200.;
    pan.region.height = 190.;
    exact(&cache, &doc, pan);
    doc.undo().unwrap();
    exact(&cache, &doc, r);
    doc.redo().unwrap();
    exact(&cache, &doc, r);
}

#[test]
fn limited_budget_and_layer_order_changes_never_invent_missing_mips() {
    let (mut doc, id) = fixture(true, false);
    let cache = CachedCpuRenderer::with_cache_budget(25_000).unwrap();
    let r = request(PixelFormat::SrgbRgba8Premultiplied, 2);
    exact(&cache, &doc, r);
    let available = cache.cache_stats().entries;
    assert!(available > 0 && available < 25);
    doc.move_layer(id, 64, 0).unwrap();
    let (_, d) = cache.render_diagnosed(&doc.snapshot(), r, &CancellationToken::default());
    assert!(d.translations <= available as u64);
    assert!(d.translations > 0);
    assert!(d.misses > 0);
    assert!(d.retained_bytes <= 25_000);
    exact(&cache, &doc, r);
    let top = doc.add_layer("uniform transparent top").unwrap();
    exact(&cache, &doc, r);
    doc.drop_layer(id, top, 1).unwrap();
    doc.move_layer(id, 64, 0).unwrap();
    let (_, d) = cache.render_diagnosed(&doc.snapshot(), r, &CancellationToken::default());
    assert_eq!(d.translations, 0);
    assert_eq!(d.translation_reason, 5);
    exact(&cache, &doc, r);
}

#[test]
fn fixed_nonuniform_upper_layer_and_post_move_edits_do_not_leak_old_composition() {
    for format in [
        PixelFormat::LinearRgba32F,
        PixelFormat::SrgbRgba8Premultiplied,
    ] {
        let (mut doc, moving) = fixture(true, false);
        let upper = doc
            .add_layer("fixed nonuniform upper contribution")
            .unwrap();
        paint(&mut doc, 19);
        doc.set_active_layer(moving).unwrap();
        let cache = CachedCpuRenderer::default();
        let r = request(format, 2);
        exact(&cache, &doc, r);
        doc.move_layer(moving, 64, -64).unwrap();
        let (result, d) = cache.render_diagnosed(&doc.snapshot(), r, &CancellationToken::default());
        result.unwrap();
        assert_eq!(d.translation_reason, 4);
        assert_eq!(d.translations, 0);
        assert!(d.misses > 0);
        exact(&cache, &doc, r);
        paint(&mut doc, 7);
        exact(&cache, &doc, r);
        doc.undo().unwrap();
        exact(&cache, &doc, r);
        doc.redo().unwrap();
        exact(&cache, &doc, r);
        doc.remove_layer(upper).unwrap();
        exact(&cache, &doc, r);
        let mut appearance = doc.layer(moving).unwrap().appearance();
        appearance.fill = 0.43;
        doc.set_layer_appearance(moving, appearance).unwrap();
        doc.move_layer(moving, 64, 0).unwrap();
        let (result, d) = cache.render_diagnosed(&doc.snapshot(), r, &CancellationToken::default());
        result.unwrap();
        assert_eq!(d.translations, 0);
        assert_eq!(d.translation_reason, 5);
        exact(&cache, &doc, r);
    }
}
