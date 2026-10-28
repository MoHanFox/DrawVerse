use super::*;
use paint_core::{Brush, Document, InputPoint, Pixel};
fn fixture() -> (Document, u64, RenderRequest) {
    let mut doc = Document::new(256, 192).unwrap();
    doc.initialize_white_background().unwrap();
    let id = doc.add_layer("source").unwrap();
    for (x, y) in [(20., 20.), (90., 80.), (155., 145.)] {
        doc.begin_stroke(
            Brush {
                radius: 35.,
                color: Pixel::from_straight([0.2, 0.4, 0.8, 0.6]).unwrap(),
                ..Default::default()
            },
            InputPoint::new(x, y, 1.),
        )
        .unwrap();
        doc.end_stroke().unwrap();
    }
    let r = RenderRequest {
        region: Region {
            x: 0.,
            y: 0.,
            width: 256.,
            height: 192.,
        },
        width: 64,
        height: 48,
        format: PixelFormat::LinearRgba32F,
    };
    (doc, id, r)
}
#[test]
fn cancel_at_each_translation_boundary_never_commits_a_mixed_cache_or_frame() {
    for (stage, n) in [
        ("qualification", 1),
        ("after_qualification", 1),
        ("remap", 3),
        ("commit", 1),
        ("after_commit", 1),
        ("rebuild", 3),
    ] {
        let (mut doc, id, r) = fixture();
        let renderer = CachedCpuRenderer::default();
        renderer
            .render(&doc.snapshot(), r, &CancellationToken::default())
            .unwrap();
        doc.move_layer(id, 64, 0).unwrap();
        let cancelled = CancellationToken::default();
        CANCEL_AT.with(|at| at.set(Some((stage, n))));
        assert!(
            matches!(
                renderer.render(&doc.snapshot(), r, &cancelled),
                Err(RenderError::Cancelled)
            ),
            "{stage}"
        );
        assert!(CANCEL_AT.with(|at| at.get()).is_none());
        let cache = renderer.cache.lock().unwrap();
        if stage == "remap" || stage == "commit" {
            assert!(cache.entries.is_empty());
            assert_eq!(cache.stats.retained_bytes, 0);
        }
        assert_eq!(
            cache.stats.retained_bytes,
            cache.entries.values().map(|e| e.bytes).sum()
        );
        drop(cache);
        for (dx, dy) in [(0, 0), (-64, 0), (64, 64), (-64, -64)] {
            doc.move_layer(id, dx, dy).unwrap();
            let snapshot = doc.snapshot();
            let a = renderer
                .render(&snapshot, r, &CancellationToken::default())
                .unwrap();
            let b = CpuRenderer
                .render(&snapshot, r, &CancellationToken::default())
                .unwrap();
            match (a.pixels, b.pixels) {
                (FramePixels::Linear(a), FramePixels::Linear(b)) => assert_eq!(a, b, "{stage}"),
                _ => panic!("format"),
            }
        }
    }
}
