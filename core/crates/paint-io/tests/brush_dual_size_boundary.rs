use paint_core::*;
use paint_io::brush_dual::DualParameters;

#[test]
fn old_dual_format_rejects_all_retained_sizing_and_keeps_default_roundtrip() {
    for enabled in [false, true] {
        for mode in [DualSizeMode::Independent, DualSizeMode::Relative] {
            let dual = DualBrush::new(enabled, DualMode::Multiply, SecondaryTip::default())
                .with_sizing(DualSizing::new(mode, 8., DualWidthRule::FixedWidth).unwrap());
            assert!(matches!(
                DualParameters::from_core(&dual),
                Err(paint_io::Error::Unsupported(_))
            ));
        }
    }
    let default = DualBrush::default();
    let decoded = DualParameters::from_core(&default)
        .unwrap()
        .bind(None)
        .unwrap();
    assert!(decoded.is_default());
    assert_eq!(decoded.sizing(), default.sizing());
}
