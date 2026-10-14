use paint_core::*;
use std::time::Instant;
fn main() {
    for radius in [64., 128.] {
        for (name, blend, dual, path) in [
            ("normal", BlendMode::Normal, false, false),
            ("multiply", BlendMode::Multiply, false, false),
            ("dual-path", BlendMode::Normal, true, true),
            ("dual-paired", BlendMode::Normal, true, false),
            ("multiply-dual", BlendMode::Multiply, true, true),
            ("soft-multiply", BlendMode::Multiply, false, false),
            ("soft-dual", BlendMode::Normal, true, true),
            ("soft-multiply-dual", BlendMode::Multiply, true, true),
        ] {
            let mut times = Vec::new();
            let mut hash = 0u64;
            for _ in 0..3 {
                let mut doc = Document::new(512, 512).unwrap();
                let base = Brush {
                    radius: 256.,
                    color: Pixel::from_straight([0.35, 0.45, 0.55, 1.]).unwrap(),
                    ..Default::default()
                };
                doc.begin_stroke(base, InputPoint::new(256.5, 256.5, 1.))
                    .unwrap();
                doc.end_stroke().unwrap();
                let tip = BrushTip {
                    hardness: if name.starts_with("soft-") { 0.5 } else { 1. },
                    flow: if name.starts_with("soft-") { 0.25 } else { 1. },
                    pressure_size: false,
                    pressure_flow: false,
                    ..Default::default()
                };
                let secondary =
                    SecondaryTip::new(radius, tip, None, BrushDynamics::default()).unwrap();
                let secondary = if path {
                    secondary.with_spacing(0.01).unwrap()
                } else {
                    secondary
                };
                let definition = BrushDefinition {
                    brush: Brush {
                        radius,
                        spacing: 0.02,
                        opacity: 0.4,
                        color: Pixel::from_straight([0.75, 0.3, 0.1, 1.]).unwrap(),
                        ..Default::default()
                    },
                    tip,
                    write_blend: blend,
                    dual: if dual {
                        DualBrush::new(true, DualMode::Multiply, secondary)
                    } else {
                        DualBrush::default()
                    },
                    ..Default::default()
                };
                let start = Instant::now();
                doc.begin_brush_stroke(definition, InputPoint::new(96.5, 256.5, 1.))
                    .unwrap();
                for pass in 0..if name.starts_with("soft-") { 4 } else { 1 } {
                    for i in 1..=80 {
                        doc.stroke_to(InputPoint::new(
                            96.5 + 4. * (if pass % 2 == 0 { i } else { 80 - i }) as f64,
                            256.5
                                + ((if pass % 2 == 0 { i } else { 80 - i }) as f64 * 0.08).sin()
                                    * 24.,
                            1.,
                        ))
                        .unwrap();
                    }
                }
                doc.end_stroke().unwrap();
                times.push(start.elapsed().as_secs_f64() * 1000.);
                hash = 0xcbf29ce484222325;
                for y in (0..512).step_by(4) {
                    for x in (0..512).step_by(4) {
                        for c in doc.pixel(x, y).components() {
                            hash = (hash ^ u64::from(c.to_bits())).wrapping_mul(0x100000001b3);
                        }
                    }
                }
                assert_eq!(doc.history_depth(), (2, 0));
            }
            times.sort_by(f64::total_cmp);
            println!(
                "{name} diameter={} median_ms={:.3} hash={hash:016x}",
                radius * 2.,
                times[1]
            );
        }
    }
}
