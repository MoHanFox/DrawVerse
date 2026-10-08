use paint_core::*;
use paint_render::*;
use paint_task::CancellationToken;
fn image(renderer: &dyn Renderer, doc: &Document, size: u32) -> Vec<[f32; 4]> {
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
fn blend_fill_position_and_signed_brush_edits_invalidate_cached_tiles_at_all_lods() {
    let mut doc = Document::with_options(
        128,
        128,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    paint(&mut doc, [0.4, 0.8, 0.2, 0.7]);
    let id = doc.add_layer("top").unwrap();
    paint(&mut doc, [0.8, 0.2, 0.4, 0.5]);
    let cache = CachedCpuRenderer::default();
    for mode in 0..27 {
        let mut a = doc.layer(id).unwrap().appearance();
        a.blend = BlendMode::from_id(mode).unwrap();
        a.fill = if mode % 2 == 0 { 0.6 } else { 1. };
        a.offset_x = if mode % 3 == 0 { -65 } else { 7 };
        a.offset_y = if mode % 2 == 0 { 3 } else { -1 };
        doc.set_layer_appearance(id, a).unwrap();
        for size in [128, 64, 16, 1] {
            assert_eq!(image(&cache, &doc, size), image(&CpuRenderer, &doc, size));
        }
        let full = image(&cache, &doc, 128);
        for (x, y) in [(0, 0), (2, 63), (63, 63), (64, 64), (127, 127)] {
            assert_eq!(full[(y * 128 + x) as usize], doc.pixel(x, y).components());
        }
    }
    paint(&mut doc, [1., 0., 0., 1.]);
    assert_eq!(image(&cache, &doc, 128), image(&CpuRenderer, &doc, 128));
    doc.undo().unwrap();
    assert_eq!(image(&cache, &doc, 16), image(&CpuRenderer, &doc, 16));
    doc.redo().unwrap();
    assert_eq!(image(&cache, &doc, 16), image(&CpuRenderer, &doc, 16));
    let mut a = doc.layer(id).unwrap().appearance();
    a.fill = 0.;
    doc.set_layer_appearance(id, a).unwrap();
    assert_eq!(image(&cache, &doc, 128), image(&CpuRenderer, &doc, 128));
}

#[test]
fn zero_fill_moved_layer_is_skipped_without_loading_its_cold_pixels() {
    let mut doc = Document::with_options(
        128,
        128,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    paint(&mut doc, [0.2, 0.3, 0.4, 1.]);
    let id = doc.add_layer("cold moved content").unwrap();
    paint(&mut doc, [1., 0.2, 0.3, 1.]);
    let mut a = doc.layer(id).unwrap().appearance();
    a.offset_x = 7;
    a.fill = 0.;
    doc.set_layer_appearance(id, a).unwrap();
    doc.trim_storage().unwrap();
    // Pin only the bottom tiles. Loading a fill=0 top tile would increment page_reads.
    let pins = doc
        .layer(1)
        .unwrap()
        .tiles()
        .map(|(_, tile)| tile.try_pixels().unwrap())
        .collect::<Vec<_>>();
    let reads = doc.storage_stats().page_reads;
    doc.snapshot().read_row(0, 63, 128).unwrap();
    doc.try_pixel(63, 63).unwrap();
    image(&CachedCpuRenderer::default(), &doc, 16);
    assert_eq!(doc.storage_stats().page_reads, reads);
    drop(pins);
}
