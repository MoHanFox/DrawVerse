use paint_core::*;
use paint_render::*;
use paint_task::CancellationToken;
fn render(renderer: &dyn Renderer, doc: &Document, size: u32) -> Vec<[f32; 4]> {
    let frame = renderer
        .render(
            &doc.snapshot(),
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
            },
            &CancellationToken::default(),
        )
        .unwrap();
    match frame.pixels {
        FramePixels::Linear(p) => p,
        _ => unreachable!(),
    }
}
fn paint(doc: &mut Document, color: [f32; 4]) {
    doc.begin_stroke(
        Brush {
            radius: 40.,
            color: Pixel::from_straight(color).unwrap(),
            ..Default::default()
        },
        InputPoint::new(63.5, 63.5, 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
}
#[test]
fn nested_isolation_blending_visibility_membership_and_moves_invalidate_cache() {
    let mut doc = Document::new(128, 128).unwrap();
    paint(&mut doc, [0.2, 0.7, 0.4, 0.8]);
    let bottom = doc.add_layer("grouped").unwrap();
    paint(&mut doc, [0.8, 0.2, 0.4, 0.6]);
    let group = doc.group_layer(bottom, "inner").unwrap();
    let top = doc.add_layer("child").unwrap();
    paint(&mut doc, [0.2, 0.4, 0.8, 0.5]);
    let root = doc.group_layer(group, "outer").unwrap();
    let cache = CachedCpuRenderer::default();
    for mode in 0..27 {
        let mut a = doc.layer(root).unwrap().appearance();
        a.blend = BlendMode::from_id(mode).unwrap();
        a.fill = 0.7;
        doc.set_layer_appearance(root, a).unwrap();
        doc.set_layer_properties(
            group,
            LayerProperties {
                visible: mode != 5,
                opacity: 0.6,
            },
        )
        .unwrap();
        for size in [128, 16, 1] {
            assert_eq!(render(&cache, &doc, size), render(&CpuRenderer, &doc, size));
        }
        let full = render(&cache, &doc, 128);
        for y in [1, 63, 64, 127] {
            for x in [1, 63, 64, 127] {
                assert_eq!(full[(y * 128 + x) as usize], doc.pixel(x, y).components());
            }
        }
    }
    for action in 0..4 {
        match action {
            0 => doc.move_layer(root, 7, -1).unwrap(),
            1 => doc.reparent_layer(top, 0).unwrap(),
            2 => doc.undo().map(|_| ()).unwrap(),
            _ => doc.ungroup_layer(root).unwrap(),
        };
        assert_eq!(render(&cache, &doc, 128), render(&CpuRenderer, &doc, 128));
    }
    render(&cache, &doc, 16);
    let stats = cache.cache_stats();
    render(&cache, &doc, 16);
    assert!(cache.cache_stats().hits > stats.hits);
}
#[test]
fn hidden_or_zero_fill_ancestors_never_read_cold_pixels() {
    let mut doc = Document::with_options(
        128,
        128,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    paint(&mut doc, [1., 0., 0., 1.]);
    let group = doc.group_layer(1, "cold").unwrap();
    for hidden in [true, false] {
        let mut a = doc.layer(group).unwrap().appearance();
        a.fill = if hidden { 1. } else { 0. };
        doc.set_layer_appearance(group, a).unwrap();
        doc.set_layer_properties(
            group,
            LayerProperties {
                visible: !hidden,
                opacity: 1.,
            },
        )
        .unwrap();
        doc.trim_storage().unwrap();
        let reads = doc.storage_stats().page_reads;
        doc.snapshot().read_row(0, 63, 128).unwrap();
        doc.try_pixel(63, 63).unwrap();
        assert!(render(&CachedCpuRenderer::default(), &doc, 16)
            .iter()
            .all(|p| *p == [0.; 4]));
        assert_eq!(doc.storage_stats().page_reads, reads);
    }
}

#[test]
fn pass_through_switch_preview_and_mask_invalidate_cached_backdrop() {
    let mut doc = Document::new(128, 128).unwrap();
    paint(&mut doc, [0.2, 0.3, 0.4, 1.]);
    let top = doc.add_layer("dodge").unwrap();
    paint(&mut doc, [0.5, 0.25, 0.2, 0.8]);
    let mut a = doc.layer(top).unwrap().appearance();
    a.blend = BlendMode::ColorDodge;
    doc.set_layer_appearance(top, a).unwrap();
    let flat = doc.pixel(64, 64);
    let group = doc.group_layer(top, "pass").unwrap();
    let mut a = doc.layer(group).unwrap().appearance();
    a.blend = BlendMode::Normal;
    doc.set_layer_appearance(group, a).unwrap();
    let cached = CachedCpuRenderer::default();
    let before = render(&cached, &doc, 128);
    let mut a = doc.layer(group).unwrap().appearance();
    a.blend = BlendMode::PassThrough;
    doc.set_layer_appearance(group, a).unwrap();
    let after = render(&cached, &doc, 128);
    assert_ne!(before, after);
    assert_eq!(after, render(&CpuRenderer, &doc, 128));
    assert_eq!(after[64 * 128 + 64], flat.components());
    for size in [16, 1] {
        assert_eq!(
            render(&cached, &doc, size),
            render(&CpuRenderer, &doc, size)
        );
    }
    doc.undo().unwrap();
    assert_eq!(render(&cached, &doc, 128), before);
    doc.redo().unwrap();
    assert_eq!(render(&cached, &doc, 128), after);
    let mask = doc.add_mask(group).unwrap();
    doc.set_active_layer(mask).unwrap();
    paint(&mut doc, [0., 0., 0., 1.]);
    assert_eq!(render(&cached, &doc, 128), render(&CpuRenderer, &doc, 128));
    assert_eq!(doc.pixel(64, 64).components(), [0.2, 0.3, 0.4, 1.]);
}
