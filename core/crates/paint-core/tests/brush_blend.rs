use paint_core::*;
fn color(r: f32, g: f32, b: f32, a: f32) -> Pixel {
    Pixel::from_straight([r, g, b, a]).unwrap()
}
fn seed(background: Pixel) -> Document {
    let mut doc = Document::new(64, 64).unwrap();
    doc.begin_stroke(
        Brush {
            radius: 60.,
            color: background,
            ..Default::default()
        },
        InputPoint::new(32.5, 32.5, 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    doc
}
fn brush(mode: BlendMode) -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 6.,
            opacity: 0.4,
            spacing: 0.02,
            color: color(0.4, 0.2, 0.7, 0.75),
            ..Default::default()
        },
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        write_blend: mode,
        ..Default::default()
    }
}
fn dot(doc: &mut Document, b: BrushDefinition) {
    doc.begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    doc.end_stroke().unwrap();
}
fn pixels(d: &Document) -> Vec<Pixel> {
    (0..64)
        .flat_map(|y| (0..64).map(move |x| d.pixel(x, y)))
        .collect()
}
fn close(a: Pixel, b: Pixel) {
    for (x, y) in a.components().iter().zip(b.components()) {
        assert!((x - y).abs() < 1e-6, "{a:?} != {b:?}");
    }
}
#[test]
fn all_modes_write_actual_pixels_over_transparent_partial_and_opaque_targets() {
    for id in 0..27 {
        let mode = BlendMode::from_id(id).unwrap();
        for alpha in [0., 0.37, 1.] {
            let base = color(0.6, 0.5, 0.3, alpha);
            let mut doc = seed(base);
            let b = brush(mode);
            let expected =
                blend_pixel(b.brush.color.scaled(b.brush.opacity), base, mode, 32, 32, 1);
            let appearance = doc.layer(1).unwrap().appearance();
            dot(&mut doc, b);
            close(doc.pixel(32, 32), expected);
            assert_eq!(doc.layer(1).unwrap().appearance(), appearance);
        }
    }
    let mut doc = seed(color(0.8, 0.8, 0.8, 1.));
    let mut b = brush(BlendMode::Multiply);
    b.brush.color = color(0.5, 0.5, 0.5, 1.);
    dot(&mut doc, b);
    close(doc.pixel(32, 32), color(0.64, 0.64, 0.64, 1.));
    let mut doc = seed(color(0.25, 0.25, 0.25, 1.));
    let mut b = brush(BlendMode::ColorDodge);
    b.brush.color = color(0.5, 0.5, 0.5, 1.);
    dot(&mut doc, b);
    close(doc.pixel(32, 32), color(0.35, 0.35, 0.35, 1.));
}
#[test]
fn whole_stroke_limits_chunking_capture_and_new_stroke_blending_are_exact() {
    let b = brush(BlendMode::Multiply);
    let mut a = seed(color(0.8, 0.6, 0.4, 1.));
    let mut split = seed(color(0.8, 0.6, 0.4, 1.));
    for (doc, chunked) in [(&mut a, false), (&mut split, true)] {
        doc.begin_brush_stroke(b.clone(), InputPoint::new(16.5, 32.5, 1.))
            .unwrap();
        if chunked {
            doc.stroke_to(InputPoint::new(32.5, 32.5, 1.)).unwrap();
        }
        doc.stroke_to(InputPoint::new(48.5, 32.5, 1.)).unwrap();
        for _ in 0..3 {
            doc.stroke_to(InputPoint::new(16.5, 32.5, 1.)).unwrap();
            doc.stroke_to(InputPoint::new(48.5, 32.5, 1.)).unwrap();
        }
        doc.end_stroke().unwrap();
    }
    assert_eq!(pixels(&a), pixels(&split));
    let first = pixels(&a);
    let before = a.pixel(32, 32);
    dot(&mut a, b);
    let second = pixels(&a);
    assert_ne!(first, second);
    assert!(a.pixel(32, 32).components()[0] < before.components()[0]);
    a.undo().unwrap();
    assert_eq!(pixels(&a), first);
    a.redo().unwrap();
    assert_eq!(pixels(&a), second);
    let mut source = brush(BlendMode::Multiply);
    let mut capture = seed(color(0.8, 0.6, 0.4, 1.));
    capture
        .begin_brush_stroke(source.clone(), InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    source.write_blend = BlendMode::Screen;
    capture.end_stroke().unwrap();
    close(
        capture.pixel(32, 32),
        blend_pixel(
            source.brush.color.scaled(0.4),
            color(0.8, 0.6, 0.4, 1.),
            BlendMode::Multiply,
            32,
            32,
            0,
        ),
    );
}
#[test]
fn mask_gray_and_alpha_lock_use_color_blending_without_changing_alpha() {
    for id in 0..27 {
        let mode = BlendMode::from_id(id).unwrap();
        let base = color(0.6, 0.5, 0.3, 0.37);
        let mut locked = seed(base);
        let mut appearance = locked.layer(1).unwrap().appearance();
        appearance.locks = LOCK_TRANSPARENCY;
        locked.set_layer_appearance(1, appearance).unwrap();
        let b = brush(mode);
        let rgb = blend_pixel(
            b.brush.color.scaled(0.4),
            color(0.6, 0.5, 0.3, 1.),
            mode,
            32,
            32,
            1,
        )
        .components();
        let expected = color(rgb[0], rgb[1], rgb[2], base.alpha());
        dot(&mut locked, b);
        close(locked.pixel(32, 32), expected);
        assert_eq!(
            locked.pixel(32, 32).alpha().to_bits(),
            base.alpha().to_bits()
        );
        let mut mask = Document::new(64, 64).unwrap();
        mask.initialize_white_background().unwrap();
        mask.add_mask(1).unwrap();
        let mut b = brush(mode);
        b.brush.color = color(0.4, 0.4, 0.4, 1.);
        dot(&mut mask, b);
        let gray = 1.055 * 0.4f32.powf(1. / 2.4) - 0.055;
        let value = blend_pixel(color(gray, gray, gray, 0.4), Pixel::WHITE, mode, 32, 32, 1)
            .components()[0];
        assert!((mask.pixel(32, 32).alpha() - value).abs() < 1e-6);
    }
}
#[test]
fn dissolve_is_stable_within_stroke_and_varies_between_strokes() {
    let mut doc = seed(Pixel::WHITE);
    let mut b = brush(BlendMode::Dissolve);
    b.brush.color = color(0., 0., 0., 1.);
    b.brush.radius = 12.;
    doc.begin_brush_stroke(b.clone(), InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    let initial = pixels(&doc);
    doc.stroke_to(InputPoint::new(32.5, 32.5, 1.)).unwrap();
    doc.end_stroke().unwrap();
    assert_eq!(pixels(&doc), initial);
    let first = pixels(&doc);
    dot(&mut doc, b);
    assert_ne!(pixels(&doc), first);
    let second = pixels(&doc);
    doc.undo().unwrap();
    assert_eq!(pixels(&doc), first);
    doc.redo().unwrap();
    assert_eq!(pixels(&doc), second);
}
#[test]
fn unsupported_tools_fail_before_mutation_and_cancel_restores_target() {
    let mut doc = seed(Pixel::WHITE);
    let before = pixels(&doc);
    for mode in [
        BrushMode::Paint,
        BrushMode::Erase,
        BrushMode::Smudge {
            all_layers: false,
            finger_painting: false,
            pressure_strength: false,
        },
    ] {
        let mut b = brush(if mode == BrushMode::Paint {
            BlendMode::PassThrough
        } else {
            BlendMode::Multiply
        });
        b.brush.mode = mode;
        assert!(doc
            .begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.))
            .is_err());
        assert_eq!(pixels(&doc), before);
    }
    doc.begin_brush_stroke(brush(BlendMode::Multiply), InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    let started = pixels(&doc);
    assert!(doc
        .stroke_to(InputPoint::new(2_000_000., 32.5, 1.))
        .is_err());
    assert_eq!(pixels(&doc), started);
    doc.stroke_to(InputPoint::new(48.5, 32.5, 1.)).unwrap();
    doc.cancel_stroke().unwrap();
    assert_eq!(pixels(&doc), before);
}

#[test]
fn images_texture_dual_color_pressure_selection_and_paging_match_captured_contribution() {
    use std::sync::Arc;
    let background = color(0.7, 0.6, 0.3, 0.65);
    let mut b = brush(BlendMode::Multiply);
    b.brush.spacing = 0.2;
    b.mask = Some(Arc::new(
        BrushMask::new(4, 2, vec![0, 128, 255, 128, 255, 0, 128, 255]).unwrap(),
    ));
    b.texture = BrushTexture::new(
        true,
        TextureSettings::new(TextureSpace::Document, 1., 0., 0.7, false, 0., 0.).unwrap(),
        Some(Arc::new(TextureResource::new(Arc::new(
            BrushMask::new(2, 2, vec![0, 128, 255, 64]).unwrap(),
        )))),
    )
    .unwrap();
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(
            5.,
            BrushTip {
                pressure_size: false,
                pressure_flow: false,
                ..Default::default()
            },
            None,
            Default::default(),
        )
        .unwrap()
        .with_spacing(0.25)
        .unwrap(),
    );
    b.dynamics.color = ColorDynamics::new(true, 0.5, 0.1, 0.1, 0.1, true, 19).unwrap();
    b.dynamics.pressure_opacity = true;
    let mut expected = b.clone();
    expected.write_blend = BlendMode::Normal;
    let create = |base: Pixel, paged: bool| {
        let mut d = Document::with_options(
            192,
            64,
            DocumentOptions {
                max_resident_tiles: if paged { 1 } else { 128 },
                ..Default::default()
            },
        )
        .unwrap();
        d.begin_stroke(
            Brush {
                radius: 200.,
                color: base,
                ..Default::default()
            },
            InputPoint::new(96.5, 32.5, 1.),
        )
        .unwrap();
        d.end_stroke().unwrap();
        let mut selection =
            SelectionShape::geometry(SelectionKind::Rectangle, 30., 10., 140., 40., true);
        selection.feather = 2.;
        d.set_selection(
            Selection::default()
                .apply(selection, SelectionOperation::Replace, 192, 64)
                .unwrap(),
        )
        .unwrap();
        d
    };
    let mut painted = create(background, true);
    let mut contribution = create(Pixel::TRANSPARENT, false);
    for (d, b) in [(&mut painted, b), (&mut contribution, expected)] {
        d.begin_brush_stroke(b, InputPoint::new(16.5, 32.5, 0.5))
            .unwrap();
        d.stroke_to(InputPoint::new(96.5, 32.5, 1.)).unwrap();
        d.stroke_to(InputPoint::new(170.5, 32.5, 0.25)).unwrap();
        d.end_stroke().unwrap();
    }
    for y in 0..64 {
        for x in 0..192 {
            close(
                painted.pixel(x, y),
                blend_pixel(
                    contribution.pixel(x, y),
                    background,
                    BlendMode::Multiply,
                    x,
                    y,
                    1,
                ),
            );
        }
    }
    assert!(painted.storage_stats().page_writes > 0);
}
