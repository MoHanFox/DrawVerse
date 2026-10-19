//! Fixed selection discovery workload; run with --release, before and after optimization.
use paint_core::*;
use std::{collections::BTreeMap, time::Instant};

fn main() {
    let size = std::env::args()
        .nth(1)
        .map_or(1024, |s| s.parse::<u32>().unwrap());
    for layers in [1, 4] {
        let mut imported = Vec::new();
        for layer in 0..layers {
            let mut tiles = BTreeMap::new();
            for ty in 0..size.div_ceil(64) {
                for tx in 0..size.div_ceil(64) {
                    let pixels = (0..4096)
                        .map(|i| {
                            let (x, y) = (tx * 64 + i % 64, ty * 64 + i / 64);
                            if x >= size || y >= size {
                                return Pixel::TRANSPARENT;
                            }
                            let v = if x % 16 == 0 || y % 16 == 0 {
                                0.2
                            } else {
                                0.21 + ((x ^ y) % 11) as f32 / 1000.
                            };
                            Pixel::from_straight([
                                v,
                                v * 0.9,
                                v * 0.8,
                                if layer == 0 { 1. } else { 0.2 },
                            ])
                            .unwrap()
                        })
                        .collect();
                    tiles.insert(
                        TileCoord { x: tx, y: ty },
                        Tile::from_pixels(pixels).unwrap(),
                    );
                }
            }
            imported.push(ImportedLayer {
                name: format!("layer {layer}"),
                properties: LayerProperties::default(),
                tiles,
            });
        }
        let mut doc =
            Document::from_import(size, size, imported, layers - 1, DocumentOptions::default())
                .unwrap();
        for contiguous in [true, false] {
            for pass in 0..3 {
                let start = Instant::now();
                doc.select_wand(WandSelection {
                    x: 0,
                    y: 0,
                    sample_size: 1,
                    tolerance: 20,
                    antialias: true,
                    contiguous,
                    all_layers: layers > 1,
                    operation: SelectionOperation::Replace,
                })
                .unwrap();
                let elapsed = start.elapsed().as_secs_f64() * 1000.;
                let text = doc.selection().to_text();
                let hash = text.bytes().fold(0xcbf29ce484222325u64, |h, b| {
                    (h ^ b as u64).wrapping_mul(0x100000001b3)
                });
                println!("size={size} layers={layers} contiguous={contiguous} pass={pass} ms={elapsed:.3} hash={hash:016x}");
                doc.set_selection(Selection::default()).unwrap();
            }
        }
    }
}
