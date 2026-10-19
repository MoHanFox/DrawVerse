//! Reproducible end-to-end benchmark; includes uploads, readback, preview and final sampling.
use paint_core::*;
use paint_ops::Executor;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};
#[derive(Debug)]
struct Timed {
    executor: Arc<dyn Executor>,
    execution_us: AtomicU64,
}
impl Executor for Timed {
    fn prepare_sources(&self, tiles: &[paint_ops::SourceTile]) -> std::result::Result<(), String> {
        self.executor.prepare_sources(tiles)
    }
    fn name(&self) -> String {
        self.executor.name()
    }
    fn execute(
        &self,
        op: &paint_ops::Operator,
    ) -> std::result::Result<Vec<paint_ops::Rgba>, String> {
        let started = Instant::now();
        let result = self.executor.execute(op);
        self.execution_us
            .fetch_add(started.elapsed().as_micros() as u64, Ordering::Relaxed);
        result
    }
}
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
    let compute = Arc::new(Timed {
        executor: compute,
        execution_us: AtomicU64::new(0),
    });
    for tool in [0, 2] {
        let mut d = seed();
        d.set_compute_executor(compute.clone()).unwrap();
        let enter = Instant::now();
        d.begin_liquify().unwrap();
        println!("tool={tool} mode_enter_us={}", enter.elapsed().as_micros());
        let started = Instant::now();
        compute.execution_us.store(0, Ordering::Relaxed);
        let mut input_us = Vec::new();
        for i in 0..6 {
            let input_started = Instant::now();
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
            input_us.push(input_started.elapsed().as_micros());
        }
        let preview = started.elapsed().as_micros();
        println!(
            "tool={tool} per_input_us={input_us:?} executor_us={}",
            compute.execution_us.load(Ordering::Relaxed)
        );
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
