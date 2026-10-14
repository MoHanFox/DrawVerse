use paint_core::*;
#[test]
fn old_io_rejects_even_disabled_retained_color_configuration() {
    for dual_enabled in [false, true] {
        for enabled in [false, true] {
            let settings = DualColor::new(
                enabled,
                true,
                true,
                DualColorOrder::SecondBehind,
                BlendMode::Normal,
                BlendMode::Normal,
            )
            .unwrap();
            let dual = DualBrush::new(dual_enabled, DualMode::Multiply, SecondaryTip::default())
                .with_color(settings);
            assert!(matches!(
                paint_io::brush_dual::DualParameters::from_core(&dual),
                Err(paint_io::Error::Unsupported(_))
            ));
            let mut c = BrushCatalog::default();
            let mut p = c.selected().settings().clone();
            p.dual = dual;
            c.edit("round-pressure", p).unwrap();
            assert!(paint_io::brush_state::encode(&c).is_err());
        }
    }
}
