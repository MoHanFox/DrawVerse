//! CPU update + presentation geometry only; excludes setup, GPU display and final materialisation.
use paint_core::*;
use std::time::Instant;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let size = args.get(1).map_or(512, |s| s.parse::<u32>().unwrap());
    let deferred = args.get(2).is_none_or(|s| s == "mesh");
    let warp = args.get(3).is_none_or(|s| s == "warp");
    let scope = args.get(4).map_or("layer", String::as_str);
    let selected = scope == "selection";
    let height = if selected { size * 2160 / 4096 } else { size };
    let mut d = Document::new(size, height.max(1)).unwrap();
    if selected {
        d.initialize_white_background().unwrap();
        let x = f64::from(size) * 0.3;
        let y = f64::from(height) * 0.3;
        let r = f64::from(size.min(1024)) * 0.25;
        let shape = SelectionShape::polygon(
            vec![
                [x, y],
                [x + r, y + 3.],
                [x + r * 0.8, y + r],
                [x - 3., y + r * 0.7],
            ],
            true,
            0.,
        )
        .unwrap();
        d.set_selection(
            Selection::default()
                .apply(shape, SelectionOperation::Replace, size, height)
                .unwrap(),
        )
        .unwrap();
    } else {
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
    }
    let mut id = d.active_layer();
    if matches!(scope, "mask" | "group") {
        d.add_mask(id).unwrap();
        if scope == "group" {
            id = d.group_layer(id, "transform group").unwrap();
            d.add_mask(id).unwrap();
        }
    }
    let start = Instant::now();
    if deferred {
        d.begin_transform_layers_deferred(&[id]).unwrap();
    } else {
        d.begin_transform_layers(&[id]).unwrap();
    }
    println!(
        "scope={scope} begin_ms={:.3} resident={}",
        start.elapsed().as_secs_f64() * 1000.,
        d.transform_deferred()
    );
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
        println!("selected={selected} height={height} size={size} mesh={deferred} warp={warp} pass={pass} six_inputs_ms={:.3} vertices={vertices}",start.elapsed().as_secs_f64()*1000.);
    }
    if selected {
        let start = Instant::now();
        d.commit_transform().unwrap();
        println!("commit_ms={:.3}", start.elapsed().as_secs_f64() * 1000.);
        use std::hash::{Hash, Hasher};
        let mut digest = std::collections::hash_map::DefaultHasher::new();
        for y in 0..height {
            for p in d.read_row(0, y, size).unwrap() {
                p.components().map(f32::to_bits).hash(&mut digest);
            }
        }
        println!(
            "final_digest={:016x} retained_tiles={}",
            digest.finish(),
            d.layers()[0].tile_count()
        );
    }
    if !selected {
        d.cancel_transform().unwrap();
    }
}
