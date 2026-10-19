use paint_core::*;
use std::{collections::BTreeMap, time::Instant};

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
fn run(radius: f32, tool: u32) -> (u128, u64) {
    let mut doc = seed();
    doc.begin_liquify().unwrap();
    let started = Instant::now();
    for i in 0..6 {
        let from = InputPoint::new(980. + i as f64 * 9., 740. + i as f64 * 4., 0.7);
        let to = InputPoint::new(from.x + 9., from.y + 4., 0.8);
        doc.liquify_dab(LiquifyDab {
            tool,
            radius,
            strength: 0.6,
            density: 0.7,
            speed: 0.8,
            pressure_enabled: true,
            from,
            to,
            elapsed_ms: 16.,
        })
        .unwrap();
    }
    let elapsed = started.elapsed().as_micros();
    let mut digest = 0xcbf29ce484222325u64;
    for y in 0..1536 {
        for x in 0..2048 {
            for v in doc.pixel(x, y).components() {
                digest = (digest ^ u64::from(v.to_bits())).wrapping_mul(0x100000001b3);
            }
        }
    }
    (elapsed, digest)
}
fn main() {
    for tool in [0, 2] {
        for radius in [128., 512., 1000.] {
            let mut times = Vec::new();
            let mut digest = None;
            for _ in 0..3 {
                let (time, value) = run(radius, tool);
                if let Some(before) = digest {
                    assert_eq!(before, value);
                }
                digest = Some(value);
                times.push(time);
            }
            times.sort_unstable();
            println!("tool={tool} radius={radius} inputs=6 median_us={} min_us={} max_us={} rgba_bits_digest={:016x}",times[1],times[0],times[2],digest.unwrap());
        }
    }
}
