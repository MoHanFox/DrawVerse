use paint_core::*;
use std::time::Instant;
fn main() {
    for radius in [4., 32., 64.] {
        for path in [false, true] {
            let mut times = Vec::new();
            let mut hash = 0;
            for _ in 0..3 {
                let mut d = Document::new(256, 128).unwrap();
                let tip = BrushTip {
                    pressure_size: false,
                    pressure_flow: false,
                    ..Default::default()
                };
                let second = SecondaryTip::new(radius + 4., tip, None, Default::default()).unwrap();
                let second = if path {
                    second.with_spacing(0.01).unwrap()
                } else {
                    second
                };
                let color = DualColor::new(
                    true,
                    true,
                    true,
                    DualColorOrder::SecondBehind,
                    BlendMode::Normal,
                    BlendMode::Normal,
                )
                .unwrap();
                let b = BrushDefinition {
                    brush: Brush {
                        radius,
                        spacing: 0.02,
                        opacity: 0.4,
                        color: Pixel::from_straight([0.8, 0.2, 0.1, 1.]).unwrap(),
                        background_color: Pixel::from_straight([0.1, 0.3, 0.8, 1.]).unwrap(),
                        ..Default::default()
                    },
                    tip,
                    dual: DualBrush::new(true, DualMode::Multiply, second).with_color(color),
                    ..Default::default()
                };
                let start = Instant::now();
                d.begin_brush_stroke(b, InputPoint::new(80.5, 64.5, 1.))
                    .unwrap();
                for i in 1..=40 {
                    d.stroke_to(InputPoint::new(
                        80.5 + 2. * i as f64,
                        64.5 + (i as f64 * 0.12).sin() * 8.,
                        1.,
                    ))
                    .unwrap();
                }
                d.end_stroke().unwrap();
                times.push(start.elapsed().as_secs_f64() * 1000.);
                hash = 0xcbf29ce484222325;
                for y in 0..128 {
                    for x in 0..256 {
                        for c in d.pixel(x, y).components() {
                            hash = (hash ^ u64::from(c.to_bits())).wrapping_mul(0x100000001b3);
                        }
                    }
                }
                assert_eq!(d.history_depth(), (1, 0));
            }
            times.sort_by(f64::total_cmp);
            println!(
                "diameter={} path={path} median_ms={:.3} hash={hash:016x}",
                radius * 2.,
                times[1]
            );
        }
    }
}
