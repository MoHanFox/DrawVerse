//! Reproducible end-to-end benchmark; includes uploads, readback, preview and final sampling.
use paint_core::*;
use paint_ops::Executor;
use std::{collections::BTreeMap, sync::Arc, time::Instant};
fn seed() -> Document {
    let mut tiles = BTreeMap::new();
    for ty in 0..24 {
        for tx in 0..32 {
            let pixels = (0..4096)
                .map(|i| {
                    let x = tx * 64 + i % 64;
                    let y = ty * 64 + i / 64;
                    let alpha = if (x / 96 + y / 96) % 7 == 0 { 0. } else { 0.75 };
                    Pixel::from_straight([
                        (x % 257) as f32 / 256.,
                        (y % 257) as f32 / 256.,
                        ((x + y) % 257) as f32 / 256.,
                        alpha,
                    ])
                    .unwrap()
                })
                .collect();
            tiles.insert(
                TileCoord::from_signed(tx, ty),
                Tile::from_pixels(pixels).unwrap(),
            );
        }
    }
    Document::from_import(
        2048,
        1536,
        vec![ImportedLayer {
            name: "Pattern".into(),
            properties: LayerProperties::default(),
            tiles,
        }],
        0,
        DocumentOptions::default(),
    )
    .unwrap()
}

fn main() {
    let id = std::env::args().nth(1).unwrap_or_else(|| "cpu".into());
    let compute: Arc<dyn Executor> = if id == "cpu" {
        Arc::new(paint_ops::Cpu)
    } else {
        Arc::new(paint_gpu::Gpu::open(&id).expect("real GPU required"))
    };
    println!("backend={}", compute.name());
    for tool in [0, 2] {
        let mut d = seed();
        d.set_compute_executor(compute.clone()).unwrap();
        d.begin_liquify().unwrap();
        let started = Instant::now();
        for i in 0..6 {
            let from = InputPoint::new(980. + f64::from(i) * 9., 740. + f64::from(i) * 4., 0.7);
            d.liquify_dab(LiquifyDab {
                tool,
                radius: 1000.,
                strength: 0.6,
                density: 0.7,
                speed: 0.8,
                pressure_enabled: true,
                from,
                to: InputPoint::new(from.x + 9., from.y + 4., 0.8),
                elapsed_ms: 16.,
            })
            .unwrap();
        }
        let preview = started.elapsed().as_micros();
        let started = Instant::now();
        d.commit_liquify().unwrap();
        let final_us = started.elapsed().as_micros();
        let mut digest = 0xcbf29ce484222325_u64;
        for y in 0..1536 {
            for x in 0..2048 {
                for v in d.pixel(x, y).components() {
                    // Quantization makes this readable; numerical equivalence is asserted by tests.
                    digest =
                        (digest ^ u64::from((v * 255.).round() as u8)).wrapping_mul(0x100000001b3);
                }
            }
        }
        println!("tool={tool} radius=1000 inputs=6 preview_us={preview} final_us={final_us} rgba8_digest={digest:016x}");
    }
}
