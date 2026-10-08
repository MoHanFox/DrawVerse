#[path = "support/dense_reference.rs"]
mod dense_reference;
use dense_reference::DenseReference;
use paint_core::{
    Brush, Document, DocumentSnapshot, InputPoint, LayerProperties, Pixel, TileCoord,
};
use paint_render::{
    CachedCpuRenderer, CpuRenderer, FramePixels, PixelFormat, Region, RenderError, RenderFrame,
    RenderRequest, Renderer,
};
use paint_task::CancellationToken;
use std::sync::Arc;

fn request() -> RenderRequest {
    RenderRequest {
        region: Region {
            x: 0.,
            y: 0.,
            width: 130.,
            height: 90.,
        },
        width: 260,
        height: 180,
        format: PixelFormat::SrgbRgba8Premultiplied,
    }
}
fn dab(doc: &mut Document, x: f64, y: f64) {
    doc.begin_stroke(brush(), InputPoint::new(x, y, 0.8))
        .unwrap();
    doc.end_stroke().unwrap();
}
fn brush() -> Brush {
    Brush {
        radius: 5.,
        color: Pixel::from_straight([0.5, 0.2, 0.8, 0.6]).unwrap(),
        ..Default::default()
    }
}
fn document() -> Document {
    let mut doc = Document::new(130, 90).unwrap();
    for (x, y) in [(10., 10.), (62., 40.), (85., 70.), (128., 89.)] {
        dab(&mut doc, x, y);
    }
    doc.add_layer("top").unwrap();
    dab(&mut doc, 63., 41.);
    doc
}
fn equal(a: RenderFrame, b: RenderFrame) {
    assert_eq!(
        (a.region, a.width, a.height, a.revision, a.lod),
        (b.region, b.width, b.height, b.revision, b.lod)
    );
    match (a.pixels, b.pixels) {
        (FramePixels::Linear(a), FramePixels::Linear(b)) => assert_eq!(a, b),
        (FramePixels::Srgb(a), FramePixels::Srgb(b)) => assert_eq!(a, b),
        _ => panic!("format changed"),
    }
}
fn golden(renderer: &dyn Renderer, snapshot: &DocumentSnapshot, r: RenderRequest) {
    let token = CancellationToken::default();
    equal(
        renderer.render(snapshot, r, &token).unwrap(),
        DenseReference.render(snapshot, r, &token).unwrap(),
    );
}
#[test]
fn cold_paged_tiles_match_dense_gold_and_cached_frames_do_not_reload_pages() {
    let mut doc = Document::with_options(
        130,
        90,
        paint_core::DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    for (x, y) in [(10., 10.), (62., 40.), (85., 70.), (128., 89.)] {
        dab(&mut doc, x, y);
    }
    doc.add_layer("top").unwrap();
    dab(&mut doc, 63., 41.);
    let snapshot = doc.snapshot();
    let renderer = CachedCpuRenderer::default();
    golden(&renderer, &snapshot, request());
    let reads = doc.storage_stats().page_reads;
    let token = CancellationToken::default();
    renderer.render(&snapshot, request(), &token).unwrap();
    assert_eq!(doc.storage_stats().page_reads, reads);
    assert!(reads > 0);
    for scale in [1., 2., 8., 64., 128.] {
        let mut r = request();
        r.region.width = r.width as f64 * scale;
        r.region.height = r.height as f64 * scale;
        golden(&renderer, &snapshot, r);
    }
    doc.trim_storage().unwrap();
    assert!(doc.storage_stats().resident_tiles <= 1);
}
#[test]
fn sparse_and_cached_output_exactly_match_dense_reference_at_all_lods() {
    let mut doc = document();
    let renderer = CachedCpuRenderer::default();
    for properties in [
        LayerProperties::default(),
        LayerProperties {
            visible: true,
            opacity: 0.3,
        },
        LayerProperties {
            visible: false,
            opacity: 0.7,
        },
        LayerProperties {
            visible: true,
            opacity: 0.,
        },
    ] {
        doc.set_layer_properties(doc.active_layer(), properties)
            .unwrap();
        let snapshot = doc.snapshot();
        let mut requests = vec![
            request(),
            RenderRequest {
                region: Region {
                    x: -19.2,
                    y: -3.7,
                    width: 160.3,
                    height: 120.8,
                },
                width: 333,
                height: 181,
                ..request()
            },
            RenderRequest {
                region: Region {
                    x: 61.25,
                    y: 38.4,
                    width: 70.9,
                    height: 52.7,
                },
                width: 17,
                height: 93,
                ..request()
            },
            RenderRequest {
                region: Region {
                    x: 300.,
                    y: 400.,
                    width: 100.,
                    height: 100.,
                },
                width: 19,
                height: 20,
                ..request()
            },
        ];
        for lod in 0..=20 {
            let size = f64::from(1u32 << lod);
            requests.push(RenderRequest {
                region: Region {
                    x: 0.,
                    y: 0.,
                    width: size,
                    height: size,
                },
                width: 1,
                height: 1,
                ..request()
            });
        }
        for mut r in requests {
            for format in [
                PixelFormat::SrgbRgba8Premultiplied,
                PixelFormat::LinearRgba32F,
            ] {
                r.format = format;
                golden(&CpuRenderer, &snapshot, r);
                golden(&renderer, &snapshot, r);
                golden(&renderer, &snapshot, r);
            }
        }
    }
}
#[test]
fn only_dirty_tiles_recompose_and_history_cancel_remain_exact() {
    let mut doc = document();
    let renderer = CachedCpuRenderer::default();
    let r = request();
    golden(&renderer, &doc.snapshot(), r);
    let cold = renderer.cache_stats();
    golden(&renderer, &doc.snapshot(), r);
    let warm = renderer.cache_stats();
    assert_eq!(warm.misses, cold.misses);
    assert_eq!(warm.hits, cold.hits + cold.entries as u64);
    dab(&mut doc, 10., 10.);
    golden(&renderer, &doc.snapshot(), r);
    let changed = renderer.cache_stats();
    assert_eq!(changed.misses, warm.misses + 1);
    assert!(changed.hits > warm.hits);
    doc.undo().unwrap();
    golden(&renderer, &doc.snapshot(), r);
    doc.redo().unwrap();
    golden(&renderer, &doc.snapshot(), r);
    doc.begin_stroke(brush(), InputPoint::new(80., 75., 1.))
        .unwrap();
    golden(&renderer, &doc.snapshot(), r);
    doc.stroke_to(InputPoint::new(90., 76., 1.)).unwrap();
    golden(&renderer, &doc.snapshot(), r);
    doc.cancel_stroke().unwrap();
    golden(&renderer, &doc.snapshot(), r);
    let new = Document::new(130, 90).unwrap();
    golden(&renderer, &new.snapshot(), r);
    assert_eq!(renderer.cache_stats().entries, 0);
}
#[test]
fn weak_cache_identity_detects_mid_stroke_writes_without_retaining_pixels() {
    let mut doc = Document::new(64, 64).unwrap();
    let renderer = CachedCpuRenderer::default();
    let r = RenderRequest {
        region: Region {
            x: 0.,
            y: 0.,
            width: 64.,
            height: 64.,
        },
        width: 64,
        height: 64,
        ..request()
    };
    doc.begin_stroke(brush(), InputPoint::new(10., 10., 1.))
        .unwrap();
    golden(&renderer, &doc.snapshot(), r);
    let weak = Arc::downgrade(
        doc.snapshot().layers()[0]
            .tile_ref(TileCoord { x: 0, y: 0 })
            .unwrap(),
    );
    // No strong snapshot/history retains this newly created tile. Arc::make_mut
    // must dissociate weak identities rather than mutate a cached identity.
    doc.stroke_to(InputPoint::new(20., 10., 1.)).unwrap();
    assert!(weak.upgrade().is_none());
    golden(&renderer, &doc.snapshot(), r);
    assert_eq!(renderer.cache_stats().misses, 2);
    let weak = Arc::downgrade(
        doc.snapshot().layers()[0]
            .tile_ref(TileCoord { x: 0, y: 0 })
            .unwrap(),
    );
    drop(doc);
    assert!(weak.upgrade().is_none());
    assert_eq!(renderer.cache_stats().entries, 1);
}
#[test]
fn bounded_cache_pan_format_and_cancel_keep_valid_output() {
    let doc = document();
    let snapshot = doc.snapshot();
    let renderer = CachedCpuRenderer::with_cache_budget(83_000).unwrap();
    for r in [
        request(),
        RenderRequest {
            region: Region {
                x: 64.,
                y: 0.,
                width: 66.,
                height: 90.,
            },
            ..request()
        },
        RenderRequest {
            format: PixelFormat::LinearRgba32F,
            ..request()
        },
        request(),
    ] {
        golden(&renderer, &snapshot, r);
        assert!(renderer.cache_stats().retained_bytes <= 83_000);
    }
    let token = CancellationToken::default();
    token.cancel();
    assert!(matches!(
        renderer.render(&snapshot, request(), &token),
        Err(RenderError::Cancelled)
    ));
    golden(&renderer, &snapshot, request());
    let disabled = CachedCpuRenderer::with_cache_budget(0).unwrap();
    golden(&disabled, &snapshot, request());
    assert_eq!(disabled.cache_stats().entries, 0);
    assert!(matches!(
        CachedCpuRenderer::with_cache_budget(96 * 1024 * 1024 + 1),
        Err(RenderError::ResourceLimit)
    ));
}

#[test]
fn coarse_lod_and_same_size_document_replacement_match_reference() {
    let mut doc = Document::new(1_000_000, 1_000_000).unwrap();
    for (x, y) in [(63., 63.), (128., 128.), (900_000., 900_000.)] {
        dab(&mut doc, x, y);
    }
    let renderer = CachedCpuRenderer::default();
    for lod in 7..=20 {
        let size = f64::from(1u32 << lod);
        let r = RenderRequest {
            region: Region {
                x: 0.,
                y: 0.,
                width: size,
                height: size,
            },
            width: 1,
            height: 1,
            format: PixelFormat::LinearRgba32F,
        };
        golden(&renderer, &doc.snapshot(), r);
        golden(&renderer, &doc.snapshot(), r);
    }
    let mut other = Document::new(1_000_000, 1_000_000).unwrap();
    dab(&mut other, 59., 59.);
    golden(
        &renderer,
        &other.snapshot(),
        RenderRequest {
            region: Region {
                x: 0.,
                y: 0.,
                width: 1_000_000.,
                height: 1_000_000.,
            },
            width: 1,
            height: 1,
            format: PixelFormat::SrgbRgba8Premultiplied,
        },
    );
}
