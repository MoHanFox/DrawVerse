use paint_core::{Brush, Document, InputPoint, Pixel};
use paint_render::{CachedCpuRenderer, CpuRenderer, PixelFormat, Region, RenderRequest, Renderer};
use paint_task::CancellationToken;
use std::time::Instant;
fn main() {
    let mut doc = Document::new(960, 640).unwrap();
    let white = std::env::args().any(|arg| arg == "--white");
    let masked = std::env::args().any(|arg| arg == "--mask");
    if white {
        doc.initialize_white_background().unwrap();
    }
    let mut brush = Brush {
        radius: 24.,
        color: Pixel::from_straight([0.025, 0.396, 0.337, 1.]).unwrap(),
        ..Default::default()
    };
    for row in 0..4 {
        doc.begin_stroke(brush, InputPoint::new(60., 80. + f64::from(row) * 120., 1.))
            .unwrap();
        for i in 1..=100 {
            doc.stroke_to(InputPoint::new(
                60. + f64::from(i) * 8.,
                80. + f64::from(row) * 120. + (f64::from(i) * 0.08).sin() * 40.,
                1.,
            ))
            .unwrap();
        }
        doc.end_stroke().unwrap();
    }
    let request = RenderRequest {
        region: Region {
            x: 0.,
            y: 0.,
            width: 960.,
            height: 640.,
        },
        width: 1800,
        height: 1200,
        format: PixelFormat::SrgbRgba8Premultiplied,
    };
    let grouped = std::env::args().any(|arg| arg == "--groups");
    if grouped {
        let inner = doc.group_layer(1, "inner").unwrap();
        doc.group_layer(inner, "outer").unwrap();
        doc.set_active_layer(1).unwrap();
    }
    if masked {
        doc.add_mask(1).unwrap();
        brush.color = Pixel::from_straight([0., 0., 0., 1.]).unwrap();
    }
    let clipped = std::env::args().any(|arg| arg == "--clip");
    if clipped {
        doc.set_active_layer(1).unwrap();
        let top = doc.add_layer("clipped paint").unwrap();
        doc.set_layer_clipping(top, true).unwrap();
    }
    let cancel = CancellationToken::default();
    let cached = CachedCpuRenderer::default();
    let uncached = std::env::args().any(|arg| arg == "--uncached");
    let renderer: &dyn Renderer = if uncached { &CpuRenderer } else { &cached };
    let mut durations = Vec::new();
    let start = Instant::now();
    for i in 0..120 {
        let s = Instant::now();
        let p = InputPoint::new(
            100. + f64::from(i) * 5.,
            if clipped { 440. } else { 560. },
            0.7,
        );
        if i == 0 {
            doc.begin_stroke(brush, p).unwrap();
        } else {
            doc.stroke_to(p).unwrap();
        }
        let frame = renderer.render(&doc.snapshot(), request, &cancel).unwrap();
        std::hint::black_box(frame);
        durations.push(s.elapsed().as_secs_f64() * 1000.);
    }
    durations.sort_by(f64::total_cmp);
    println!(
        "frames=120 output=1800x1200 average_ms={:.3} p50_ms={:.3} p95_ms={:.3} total_ms={:.3}",
        durations.iter().sum::<f64>() / 120.,
        durations[60],
        durations[114],
        start.elapsed().as_secs_f64() * 1000.
    );
    println!(
        "mode={} cache={:?}",
        if uncached { "uncached" } else { "cached" },
        cached.cache_stats()
    );
    println!(
        "scene={}",
        if grouped {
            "two_isolated_groups"
        } else {
            "flat"
        }
    );
    println!("white_background={white} painting_mask={masked} clipping={clipped}");
}
