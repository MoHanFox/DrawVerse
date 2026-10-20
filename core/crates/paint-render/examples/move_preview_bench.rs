//! Fixed complex-stack CPU frame cost, excluding input, UI and final presentation.
use paint_core::*;
use paint_render::*;
use paint_task::CancellationToken;
use std::time::Instant;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let width = args.get(1).map_or(2000, |s| s.parse::<u32>().unwrap());
    let height = args.get(2).map_or(width, |s| s.parse::<u32>().unwrap());
    let mut d = Document::new(width, height).unwrap();
    d.initialize_white_background().unwrap();
    let owner = d.add_layer("moving").unwrap();
    d.bucket_fill(BucketFill {
        x: 0,
        y: 0,
        color: Pixel::from_straight([0.8, 0.2, 0.4, 0.7]).unwrap(),
        opacity: 1.,
        tolerance: 0,
        antialias: false,
        contiguous: true,
        all_layers: false,
    })
    .unwrap();
    let mask = d.add_mask(owner).unwrap();
    d.begin_stroke(
        Brush {
            radius: 300.,
            color: Pixel::WHITE,
            ..Default::default()
        },
        InputPoint::new(f64::from(width) / 2., f64::from(height) / 2., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    d.set_mask_linked(mask, false).unwrap();
    let group = d.group_layer(owner, "ancestor").unwrap();
    d.set_layer_properties(
        group,
        LayerProperties {
            visible: true,
            opacity: 0.7,
        },
    )
    .unwrap();
    d.begin_layer_move(owner, true).unwrap();
    assert!(!d.layer_move_deferred());
    let request = RenderRequest {
        region: Region {
            x: 0.,
            y: 0.,
            width: f64::from(width),
            height: f64::from(height),
        },
        width: 1024,
        height: ((1024. * f64::from(height) / f64::from(width)).round() as u32).max(1),
        format: PixelFormat::SrgbRgba8Premultiplied,
    };
    let renderer = CachedCpuRenderer::default();
    let mut full = Vec::new();
    let mut coarse = Vec::new();
    for i in 0..5 {
        d.update_layer_move(i * 7, i * 3).unwrap();
        let snapshot = d.snapshot();
        let token = CancellationToken::default();
        let start = Instant::now();
        let a = renderer.render(&snapshot, request, &token).unwrap();
        full.push(start.elapsed().as_secs_f64() * 1000.);
        let start = Instant::now();
        let b = render_interactive(&snapshot, request, &token).unwrap();
        coarse.push(start.elapsed().as_secs_f64() * 1000.);
        std::hint::black_box((a, b));
    }
    full.sort_by(f64::total_cmp);
    coarse.sort_by(f64::total_cmp);
    println!(
        "{width}x{height} fixed-mask/group move CPU frame median full={:.3}ms coarse={:.3}ms",
        full[2], coarse[2]
    );
}
