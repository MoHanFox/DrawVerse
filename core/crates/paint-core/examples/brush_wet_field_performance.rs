//! Real wet dual strokes; run the same executable source before and after optimization.
use paint_core::*;
use std::{sync::Arc, time::Instant};
fn main() {
    for radius in [16., 48.] {
        for image in [false, true] {
            for path in [false, true] {
                let mut times = Vec::new();
                let mut expected = None;
                for _ in 0..5 {
                    let mut doc = Document::new(192, 128).unwrap();
                    doc.initialize_white_background().unwrap();
                    let tip = BrushTip {
                        pressure_size: false,
                        pressure_flow: false,
                        flow: 0.6,
                        hardness: 0.5,
                        ..Default::default()
                    };
                    let mask = image.then(|| {
                        Arc::new(
                            BrushMask::new(
                                8,
                                8,
                                (0..64)
                                    .map(|i| if i % 8 < 3 || i / 8 < 2 { 0 } else { 180 })
                                    .collect(),
                            )
                            .unwrap(),
                        )
                    });
                    let second =
                        SecondaryTip::new(radius + 4., tip, mask.clone(), Default::default())
                            .unwrap();
                    let second = if path {
                        second.with_spacing(0.01).unwrap()
                    } else {
                        second
                    };
                    let definition = BrushDefinition {
                        brush: Brush {
                            radius,
                            spacing: 0.02,
                            opacity: 0.4,
                            color: Pixel::from_straight([0.9, 0.1, 0.2, 0.8]).unwrap(),
                            background_color: Pixel::from_straight([0.1, 0.3, 0.9, 0.6]).unwrap(),
                            ..Default::default()
                        },
                        tip,
                        mask,
                        dual: DualBrush::new(true, DualMode::Multiply, second).with_color(
                            DualColor::new(
                                true,
                                true,
                                true,
                                DualColorOrder::SecondBehind,
                                BlendMode::Multiply,
                                BlendMode::Screen,
                            )
                            .unwrap(),
                        ),
                        wet: WetSettings::new(
                            true,
                            false,
                            true,
                            true,
                            WetMix::new(0.6, 0.4, 0.3, 1., 0.8, 0.1).unwrap(),
                        ),
                        ..Default::default()
                    };
                    let start = Instant::now();
                    doc.begin_configured_dual_wet_brush_stroke(
                        definition,
                        InputPoint::new(64.5, 64.5, 1.),
                        None,
                    )
                    .unwrap();
                    for i in 1..=8 {
                        doc.begin_stroke_input(InputPoint::new(
                            64.5 + i as f64 * 8.,
                            64.5 + (i as f64 * 0.5).sin() * 8.,
                            1.,
                        ))
                        .unwrap();
                        while doc.stroke_input_pending() {
                            doc.continue_stroke_page(Default::default()).unwrap();
                        }
                    }
                    doc.begin_stroke_finish().unwrap();
                    let ink = loop {
                        if let StrokePageResult::StrokeComplete { wet_ink, .. } =
                            doc.continue_stroke_page(Default::default()).unwrap()
                        {
                            break wet_ink.unwrap();
                        }
                    };
                    times.push(start.elapsed().as_secs_f64() * 1000.);
                    let mut hash = 0xcbf29ce484222325_u64;
                    let mut add = |p: Pixel| {
                        for c in p.components() {
                            hash = (hash ^ u64::from(c.to_bits())).wrapping_mul(0x100000001b3);
                        }
                    };
                    for y in 0..128 {
                        for x in 0..192 {
                            add(doc.try_pixel(x, y).unwrap());
                        }
                    }
                    for carrier in [ink.primary(), ink.secondary().unwrap()] {
                        for y in 0..=64 {
                            for x in 0..=64 {
                                add(carrier.sample(x as f64 / 64., y as f64 / 64.));
                            }
                        }
                    }
                    assert_eq!(doc.history_depth(), (1, 0));
                    assert_eq!(*expected.get_or_insert(hash), hash);
                }
                times.sort_by(f64::total_cmp);
                println!(
                    "diameter={} image={image} path={path} median_ms={:.3} hash={:016x}",
                    radius * 2.,
                    times[2],
                    expected.unwrap()
                );
            }
        }
    }
}
