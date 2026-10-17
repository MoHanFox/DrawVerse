use paint_core::*;
use std::time::Instant;

fn run() -> (u128, (usize, usize), u64) {
    let mut document = Document::new(2048, 2048).unwrap();
    let definition = BrushDefinition {
        brush: Brush {
            radius: 128.,
            opacity: 1.,
            spacing: 0.02,
            color: Pixel::from_straight([0.2, 0.6, 0.8, 1.]).unwrap(),
            ..Default::default()
        },
        tip: BrushTip {
            hardness: 1.,
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let started = Instant::now();
    document
        .begin_brush_stroke(definition, InputPoint::new(128., 1024., 1.))
        .unwrap();
    for step in 1..=240 {
        let x = 128. + step as f64 * 8.;
        let y = 1024. + (step as f64 * 0.12).sin() * 180.;
        document.stroke_to(InputPoint::new(x, y, 1.)).unwrap();
    }
    document.end_stroke().unwrap();
    let elapsed = started.elapsed().as_micros();
    let mut digest = 0u64;
    for y in 0..2048 {
        for x in 0..2048 {
            for value in document.pixel(x, y).components() {
                digest = digest.wrapping_add((value * 1_000_003.) as u64);
            }
        }
    }
    (elapsed, document.history_depth(), digest)
}

fn main() {
    let mut samples = Vec::new();
    let mut digest = 0;
    for _ in 0..3 {
        let (elapsed, history, value) = run();
        samples.push(elapsed);
        digest = value;
        eprintln!("run_us={elapsed} history={history:?} digest={value}");
    }
    samples.sort_unstable();
    println!(
        "{{\"median_us\":{},\"min_us\":{},\"max_us\":{},\"digest\":{}}}",
        samples[1], samples[0], samples[2], digest
    );
}
