use paint_core::*;

fn definition(jitter: Option<TransferJitter>) -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            spacing: 0.35,
            opacity: 0.7,
            ..Default::default()
        },
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        dynamics: BrushDynamics {
            transfer_jitter: jitter.unwrap_or_default(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn draw(brush: BrushDefinition) -> Vec<Pixel> {
    let mut document = Document::new(96, 96).unwrap();
    document
        .begin_brush_stroke(brush, InputPoint::new(12.5, 48.5, 1.))
        .unwrap();
    document.stroke_to(InputPoint::new(84.5, 48.5, 1.)).unwrap();
    document.end_stroke().unwrap();
    let mut pixels = Vec::with_capacity(96 * 96);
    for y in 0..96 {
        for x in 0..96 {
            pixels.push(document.pixel(x, y));
        }
    }
    pixels
}

#[test]
fn transfer_jitter_changes_real_stamps_and_is_deterministic() {
    let plain = draw(definition(None));
    let jitter = TransferJitter::new(0.8, 0.7, 0.6).unwrap();
    let first = draw(definition(Some(jitter)));
    let second = draw(definition(Some(jitter)));
    assert_eq!(first, second);
    assert_ne!(first, plain);
    assert!(first.iter().any(|pixel| pixel.alpha() > 0.));
    assert!(first.iter().all(|pixel| pixel.alpha() <= 0.700001));
}
