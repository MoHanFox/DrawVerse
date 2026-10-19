//! CPU update + presentation geometry only; excludes setup, GPU display and final materialisation.
use paint_core::*;
use std::time::Instant;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let size = args.get(1).map_or(512, |s| s.parse::<u32>().unwrap());
    let deferred = args.get(2).is_none_or(|s| s == "mesh");
    let warp = args.get(3).is_none_or(|s| s == "warp");
    let mut d = Document::new(size, size).unwrap();
    d.bucket_fill(BucketFill {
        x: 0,
        y: 0,
        color: Pixel::WHITE,
        opacity: 1.,
        tolerance: 0,
        antialias: false,
        contiguous: true,
        all_layers: false,
    })
    .unwrap();
    let id = d.active_layer();
    if deferred {
        d.begin_transform_layers_deferred(&[id]).unwrap();
    } else {
        d.begin_transform_layers(&[id]).unwrap();
    }
    let initial = d.transform_geometry().unwrap();
    for pass in 0..3 {
        let start = Instant::now();
        let mut vertices = 0;
        for i in 0..6 {
            let mut g = initial.clone();
            g.warp = warp;
            for p in &mut g.points {
                p[0] += i as f64 * 2.;
            }
            if warp {
                g.points[5][0] += size as f64 * 0.1;
                g.points[10][1] -= size as f64 * 0.08;
            }
            d.update_transform(g).unwrap();
            if let Some(p) = d.transform_preview() {
                vertices = p.vertices.len();
                std::hint::black_box(p);
            }
        }
        println!("size={size} mesh={deferred} warp={warp} pass={pass} six_inputs_ms={:.3} vertices={vertices}",start.elapsed().as_secs_f64()*1000.);
    }
    d.cancel_transform().unwrap();
}
