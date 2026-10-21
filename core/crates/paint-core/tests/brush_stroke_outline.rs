use paint_core::{
    BrushTexture, OutlinePlacement, OutlineSource, OutlineWidth, StrokeOutline,
    STROKE_REPLAY_OUTLINE_MODEL,
};

#[test]
fn width_modes_keep_both_values_and_closed_settings_keep_source_and_colour() {
    let absolute = OutlineWidth::new(false, 12.25, 17.5).unwrap();
    let relative = OutlineWidth::new(true, absolute.pixels(), absolute.percent()).unwrap();
    for diameter in [1., 24., 2000.] {
        assert_eq!(absolute.effective(diameter).unwrap(), 12.25);
        assert_eq!(
            relative.effective(diameter).unwrap(),
            (diameter * 0.175).min(2000.)
        );
    }
    assert_eq!(
        OutlineWidth::new(true, 0., 10000.)
            .unwrap()
            .effective(2000.)
            .unwrap(),
        2000.
    );
    let closed = StrokeOutline::new(
        false,
        OutlinePlacement::Inside,
        relative,
        OutlineSource::Texture(BrushTexture::default()),
        true,
    )
    .unwrap();
    assert!(!closed.enabled());
    assert_eq!(closed.placement(), OutlinePlacement::Inside);
    assert_eq!(closed.width(), relative);
    assert!(closed.secondary_color());
    assert!(matches!(closed.source(), OutlineSource::Texture(_)));
    assert!(StrokeOutline::new(
        true,
        OutlinePlacement::Outside,
        absolute,
        OutlineSource::Texture(BrushTexture::default()),
        false
    )
    .is_err());
    for bad in [-1., 2000.1, f32::NAN, f32::INFINITY] {
        assert!(OutlineWidth::new(false, bad, 10.).is_err());
    }
    for bad in [-1., 10000.1, f32::NAN, f32::INFINITY] {
        assert!(OutlineWidth::new(false, 2., bad).is_err());
    }
    for bad in [0., 2000.1, f32::NAN, f32::INFINITY] {
        assert!(absolute.effective(bad).is_err());
    }
}

#[test]
fn defaults_use_replay_and_overflow_is_explicit() {
    let default = StrokeOutline::default();
    assert!(default.is_default());
    assert_eq!(default.model(), STROKE_REPLAY_OUTLINE_MODEL);
    assert!(!default.allow_overflow());
    let overflow = default.with_allow_overflow(true);
    assert!(overflow.allow_overflow());
    assert!(!overflow.is_default());
}
