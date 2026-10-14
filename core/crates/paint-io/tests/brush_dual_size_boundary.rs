use paint_core::*;
use paint_io::brush_dual::DualParameters;

#[test]
fn dual_format_preserves_retained_sizing_and_default_roundtrip() {
    for enabled in [false, true] {
        for mode in [DualSizeMode::Independent, DualSizeMode::Relative] {
            let dual = DualBrush::new(enabled, DualMode::Multiply, SecondaryTip::default())
                .with_sizing(DualSizing::new(mode, 8., DualWidthRule::FixedWidth).unwrap());
            let decoded = DualParameters::from_core(&dual)
                .unwrap()
                .bind(None)
                .unwrap();
            assert_eq!(decoded.sizing(), dual.sizing());
            assert_eq!(decoded.enabled(), enabled);
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
