use paint_core::*;
fn rgba(c: [f32; 4]) -> Pixel {
    Pixel::from_straight(c).unwrap()
}
fn colors(order: DualColorOrder, overflow: bool) -> DualColor {
    DualColor::new(
        true,
        true,
        overflow,
        order,
        BlendMode::Normal,
        BlendMode::Normal,
    )
    .unwrap()
}
fn brush(path: bool, color: DualColor) -> BrushDefinition {
    let tip = BrushTip {
        pressure_size: false,
        pressure_flow: false,
        ..Default::default()
    };
    let secondary = SecondaryTip::new(8., tip, None, Default::default()).unwrap();
    let secondary = if path {
        secondary.with_spacing(0.25).unwrap()
    } else {
        secondary
    };
    BrushDefinition {
        brush: Brush {
            radius: 4.,
            spacing: 0.5,
            opacity: 0.4,
            color: rgba([1., 0., 0., 1.]),
            background_color: rgba([0., 0., 1., 1.]),
            ..Default::default()
        },
        tip,
        dual: DualBrush::new(true, DualMode::Multiply, secondary).with_color(color),
        ..Default::default()
    }
}
fn pixels(d: &Document) -> Vec<Pixel> {
    (0..d.dimensions().1)
        .flat_map(|y| (0..d.dimensions().0).map(move |x| d.pixel(x, y)))
        .collect()
}
fn dab(d: &mut Document, b: BrushDefinition) {
    d.begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    d.end_stroke().unwrap();
}

#[test]
fn interleaved_model_is_paged_split_deterministic_and_undoable() {
    let settings =
        colors(DualColorOrder::SecondBehind, true).with_model(DualColorModel::InterleavedStamp);
    let mut paged = Document::with_options(
        192,
        64,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let mut resident = Document::new(192, 64).unwrap();
    for (d, split) in [(&mut paged, true), (&mut resident, false)] {
        d.begin_brush_stroke(brush(false, settings), InputPoint::new(16.5, 32.5, 1.))
            .unwrap();
        if split {
            d.stroke_to(InputPoint::new(80.5, 32.5, 1.)).unwrap();
        }
        d.stroke_to(InputPoint::new(170.5, 32.5, 1.)).unwrap();
        d.end_stroke().unwrap();
    }
    let paged_pixels = pixels(&paged);
    let resident_pixels = pixels(&resident);
    if paged_pixels != resident_pixels {
        let index = paged_pixels
            .iter()
            .zip(&resident_pixels)
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "first difference at {index}: paged={:?} resident={:?}",
            paged_pixels[index], resident_pixels[index]
        );
    }
    let expected = pixels(&paged);
    paged.undo().unwrap();
    assert!(pixels(&paged).iter().all(|p| *p == Pixel::TRANSPARENT));
    paged.redo().unwrap();
    assert_eq!(pixels(&paged), expected);
}

#[test]
fn interleaved_model_requires_paired_dry_painting() {
    let settings =
        colors(DualColorOrder::SecondBehind, true).with_model(DualColorModel::InterleavedStamp);
    let tip = BrushTip {
        pressure_size: false,
        pressure_flow: false,
        ..Default::default()
    };
    let path = SecondaryTip::new(8., tip, None, Default::default())
        .unwrap()
        .with_spacing(0.25)
        .unwrap();
    let path_brush = BrushDefinition {
        brush: Brush {
            radius: 4.,
            spacing: 0.5,
            ..Default::default()
        },
        tip,
        dual: DualBrush::new(true, DualMode::Multiply, path).with_color(settings),
        ..Default::default()
    };
    assert!(BrushProgram::compile(path_brush).is_err());

    let mut wet = brush(false, settings);
    wet.wet = WetSettings::new(
        true,
        false,
        false,
        true,
        WetMix::new(0., 0., 0., 1., 1., 0.).unwrap(),
    );
    assert!(BrushProgram::compile(wet).is_err());

    let closed = brush(
        false,
        DualColor::new_with_model(
            DualColorModel::InterleavedStamp,
            false,
            true,
            true,
            DualColorOrder::SecondBehind,
            BlendMode::Normal,
            BlendMode::Normal,
        )
        .unwrap(),
    );
    assert!(BrushProgram::compile(closed).is_ok());
}
#[test]
fn paired_outline_order_overflow_return_stroke_and_single_opacity_are_real() {
    for path in [false, true] {
        let mut d = Document::new(64, 64).unwrap();
        let b = brush(path, colors(DualColorOrder::SecondBehind, true));
        d.begin_brush_stroke(b, InputPoint::new(16.5, 32.5, 1.))
            .unwrap();
        for x in [48.5, 16.5, 48.5] {
            d.stroke_to(InputPoint::new(x, 32.5, 1.)).unwrap();
        }
        d.end_stroke().unwrap();
        assert_eq!(d.pixel(32, 32), rgba([1., 0., 0., 0.4]));
        assert_eq!(d.pixel(32, 38), rgba([0., 0., 1., 0.4]));
        assert_eq!(d.history_depth(), (1, 0));
        let after = pixels(&d);
        d.undo().unwrap();
        assert!(pixels(&d).iter().all(|p| *p == Pixel::TRANSPARENT));
        d.redo().unwrap();
        assert_eq!(pixels(&d), after);
        let mut front = Document::new(64, 64).unwrap();
        dab(
            &mut front,
            brush(path, colors(DualColorOrder::SecondInFront, true)),
        );
        assert_eq!(front.pixel(32, 32), rgba([0., 0., 1., 0.4]));
        let mut clipped = Document::new(64, 64).unwrap();
        dab(
            &mut clipped,
            brush(path, colors(DualColorOrder::SecondBehind, false)),
        );
        assert_eq!(clipped.pixel(38, 32), Pixel::TRANSPARENT);
    }
}
#[test]
fn semi_transparent_primary_and_soft_clipping_do_not_square_coverage() {
    for alpha in [0., 0.5, 1.] {
        let mut b = brush(false, colors(DualColorOrder::SecondBehind, true));
        b.brush.color = rgba([1., 0., 0., alpha]);
        let mut d = Document::new(64, 64).unwrap();
        dab(&mut d, b);
        let expected = [alpha * 0.4, 0., (1. - alpha) * 0.4, 0.4];
        assert_eq!(d.pixel(32, 32).components(), expected);
    }
    let mut b = brush(false, colors(DualColorOrder::SecondBehind, false));
    b.brush.radius = 8.;
    b.tip.hardness = 0.;
    b.brush.color = Pixel::TRANSPARENT;
    let mut primary = b.clone();
    primary.dual = Default::default();
    let program = BrushProgram::compile(primary).unwrap();
    let expected = program.coverage(4., 0., 8.) * 0.4;
    let mut d = Document::new(64, 64).unwrap();
    dab(&mut d, b);
    assert!((d.pixel(36, 32).alpha() - expected).abs() < 1e-6);
}
#[test]
fn independent_paging_split_cancel_and_offset_match_resident_pixels() {
    let mut paged = Document::with_options(
        192,
        64,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let mut resident = Document::new(192, 64).unwrap();
    for (d, split) in [(&mut paged, true), (&mut resident, false)] {
        d.begin_brush_stroke(
            brush(true, colors(DualColorOrder::SecondBehind, true)),
            InputPoint::new(16.5, 32.5, 1.),
        )
        .unwrap();
        if split {
            d.stroke_to(InputPoint::new(80.5, 32.5, 1.)).unwrap();
        }
        d.stroke_to(InputPoint::new(170.5, 32.5, 1.)).unwrap();
        d.end_stroke().unwrap();
    }
    assert_eq!(pixels(&paged), pixels(&resident));
    assert!(paged.storage_stats().page_writes > 0);
    let expected = pixels(&paged);
    paged
        .begin_brush_stroke(
            brush(true, colors(DualColorOrder::SecondInFront, true)),
            InputPoint::new(16.5, 32.5, 1.),
        )
        .unwrap();
    paged.stroke_to(InputPoint::new(80.5, 32.5, 1.)).unwrap();
    paged.cancel_stroke().unwrap();
    assert_eq!(pixels(&paged), expected);
    let mut shifted = Document::new(64, 64).unwrap();
    let mut plain = Document::new(64, 64).unwrap();
    let mut a = shifted.layer(1).unwrap().appearance();
    a.offset_x = 73;
    a.offset_y = -13;
    shifted.set_layer_appearance(1, a).unwrap();
    dab(
        &mut shifted,
        brush(false, colors(DualColorOrder::SecondBehind, true)),
    );
    dab(
        &mut plain,
        brush(false, colors(DualColorOrder::SecondBehind, true)),
    );
    assert_eq!(pixels(&shifted), pixels(&plain));
}
#[test]
fn selection_masks_and_transparency_lock_apply_to_composite_once() {
    for mask in [false, true] {
        let mut d = Document::new(64, 64).unwrap();
        d.initialize_white_background().unwrap();
        if mask {
            d.add_mask(1).unwrap();
        }
        d.set_selection(
            Selection::default()
                .apply(
                    SelectionShape::geometry(SelectionKind::Rectangle, 32., 0., 32., 64., false),
                    SelectionOperation::Replace,
                    64,
                    64,
                )
                .unwrap(),
        )
        .unwrap();
        let mut b = brush(false, colors(DualColorOrder::SecondBehind, true));
        b.brush.color = rgba([0., 0., 0., 1.]);
        b.brush.background_color = Pixel::WHITE;
        dab(&mut d, b);
        assert_eq!(d.pixel(31, 32), Pixel::WHITE);
        if !mask {
            assert_eq!(d.pixel(33, 32), rgba([0.6, 0.6, 0.6, 1.]));
        }
        assert_eq!(d.pixel(33, 32).alpha(), if mask { 0.6 } else { 1. });
    }
    let mut d = Document::new(64, 64).unwrap();
    let base = Brush {
        radius: 30.,
        color: rgba([0.5, 0.5, 0.5, 0.3]),
        ..Default::default()
    };
    d.begin_stroke(base, InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    d.end_stroke().unwrap();
    let before = pixels(&d);
    let mut a = d.layer(1).unwrap().appearance();
    a.locks = LOCK_TRANSPARENCY;
    d.set_layer_appearance(1, a).unwrap();
    dab(
        &mut d,
        brush(false, colors(DualColorOrder::SecondBehind, true)),
    );
    let after = pixels(&d);
    assert!(before
        .iter()
        .zip(&after)
        .all(|(a, b)| a.alpha().to_bits() == b.alpha().to_bits()));
    assert_ne!(after, before);
}

#[test]
fn stationary_opacity_pressure_updates_both_fields_and_size_rules_are_captured() {
    for path in [false, true] {
        let mut b = brush(path, colors(DualColorOrder::SecondBehind, true));
        b.dynamics.pressure_opacity = true;
        let mut d = Document::new(64, 64).unwrap();
        d.begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 0.25))
            .unwrap();
        assert_eq!(d.pixel(38, 32), rgba([0., 0., 1., 0.1]));
        d.stroke_to(InputPoint::new(32.5, 32.5, 1.)).unwrap();
        assert_eq!(d.pixel(38, 32), rgba([0., 0., 1., 0.4]));
        d.stroke_to(InputPoint::new(32.5, 32.5, 0.5)).unwrap();
        d.end_stroke().unwrap();
        assert_eq!(d.pixel(32, 32), rgba([1., 0., 0., 0.4]));
        assert_eq!(d.pixel(38, 32), rgba([0., 0., 1., 0.4]));
    }
    for rule in [DualWidthRule::FollowPressure, DualWidthRule::FixedWidth] {
        let mut b = brush(false, colors(DualColorOrder::SecondBehind, true));
        b.tip.pressure_size = true;
        let second = SecondaryTip::new(19., b.tip, None, Default::default()).unwrap();
        b.dual = DualBrush::new(true, DualMode::Multiply, second)
            .with_sizing(DualSizing::new(DualSizeMode::Relative, 8., rule).unwrap())
            .with_color(colors(DualColorOrder::SecondBehind, true));
        let mut d = Document::new(64, 64).unwrap();
        d.begin_brush_stroke(b.clone(), InputPoint::new(32.5, 32.5, 0.25))
            .unwrap();
        d.end_stroke().unwrap();
        assert_eq!(
            d.pixel(35, 32),
            if rule == DualWidthRule::FixedWidth {
                rgba([0., 0., 1., 0.4])
            } else {
                Pixel::TRANSPARENT
            }
        );
        let mut captured = Document::new(64, 64).unwrap();
        captured
            .begin_brush_stroke(b.clone(), InputPoint::new(16.5, 32.5, 1.))
            .unwrap();
        b.brush.background_color = rgba([0., 1., 0., 1.]);
        b.dual = b
            .dual
            .with_color(colors(DualColorOrder::SecondInFront, true));
        captured.stroke_to(InputPoint::new(48.5, 32.5, 1.)).unwrap();
        captured.end_stroke().unwrap();
        assert_eq!(captured.pixel(32, 38), rgba([0., 0., 1., 0.4]));
        let mut next = Document::new(64, 64).unwrap();
        dab(&mut next, b);
        assert_eq!(next.pixel(32, 32), rgba([0., 1., 0., 0.4]));
    }
}
#[test]
fn contribution_modes_and_background_switch_use_real_color_not_gray_coverage() {
    for mode in 0..27 {
        let mode = BlendMode::from_id(mode).unwrap();
        let color = DualColor::new(
            true,
            true,
            true,
            DualColorOrder::SecondBehind,
            mode,
            BlendMode::Normal,
        )
        .unwrap();
        let mut d = Document::new(64, 64).unwrap();
        dab(&mut d, brush(false, color));
        let expected = blend_pixel(
            rgba([1., 0., 0., 1.]),
            rgba([0., 0., 1., 1.]),
            mode,
            32,
            32,
            0,
        )
        .scaled(0.4);
        assert_eq!(d.pixel(32, 32), expected, "mode={mode:?}");
        let base = rgba([0.3, 0.5, 0.7, 0.6]);
        let mut canvas = Document::new(64, 64).unwrap();
        canvas
            .begin_stroke(
                Brush {
                    radius: 30.,
                    color: base,
                    ..Default::default()
                },
                InputPoint::new(32.5, 32.5, 1.),
            )
            .unwrap();
        canvas.end_stroke().unwrap();
        let mut b = brush(false, color);
        b.write_blend = mode;
        dab(&mut canvas, b);
        let expected = blend_pixel(
            expected,
            base,
            mode,
            32,
            32,
            u64::from(mode == BlendMode::Dissolve),
        );
        assert_eq!(canvas.pixel(32, 32), expected, "tool mode={mode:?}");
    }
    let same = DualColor::new(
        true,
        false,
        true,
        DualColorOrder::SecondBehind,
        BlendMode::Normal,
        BlendMode::Normal,
    )
    .unwrap();
    let mut d = Document::new(64, 64).unwrap();
    dab(&mut d, brush(false, same));
    assert_eq!(d.pixel(38, 32), rgba([1., 0., 0., 0.4]));
}
#[test]
fn dual_color_is_ignored_for_eraser_and_regular_smudge_but_tile_failure_rolls_back() {
    assert!(DualColor::new(
        true,
        true,
        true,
        DualColorOrder::SecondBehind,
        BlendMode::PassThrough,
        BlendMode::Normal
    )
    .is_err());
    let mut input = InputPoint::new(32.5, 32.5, 1.);
    input.tool = Tool::Eraser;
    let mut tool_eraser = Document::new(64, 64).unwrap();
    tool_eraser.initialize_white_background().unwrap();
    assert!(tool_eraser
        .begin_brush_stroke(
            brush(false, colors(DualColorOrder::SecondBehind, true)),
            input
        )
        .is_ok());
    assert!(tool_eraser.stroke_active());
    tool_eraser.cancel_stroke().unwrap();
    let mut painted = Document::new(64, 64).unwrap();
    dab(
        &mut painted,
        brush(false, colors(DualColorOrder::SecondBehind, true)),
    );
    let before = pixels(&painted);
    let mut eraser = brush(false, colors(DualColorOrder::SecondBehind, true));
    eraser.brush.mode = BrushMode::Erase;
    painted
        .begin_brush_stroke(eraser, InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    painted.end_stroke().unwrap();
    assert_ne!(pixels(&painted), before);
    for mode in [
        BrushMode::Erase,
        BrushMode::Smudge {
            all_layers: false,
            finger_painting: false,
            pressure_strength: false,
        },
        BrushMode::Smudge {
            all_layers: false,
            finger_painting: true,
            pressure_strength: false,
        },
    ] {
        let mut b = brush(false, colors(DualColorOrder::SecondBehind, true));
        b.brush.mode = mode;
        assert!(BrushProgram::compile(b).is_ok());
    }
    let mut d = Document::with_options(
        192,
        64,
        DocumentOptions {
            max_document_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    d.begin_brush_stroke(
        brush(false, colors(DualColorOrder::SecondBehind, true)),
        InputPoint::new(32.5, 32.5, 1.),
    )
    .unwrap();
    assert!(d.stroke_to(InputPoint::new(150.5, 32.5, 1.)).is_err());
    assert_eq!(d.history_depth(), (0, 0));
    assert!(pixels(&d).iter().all(|p| *p == Pixel::TRANSPARENT));
}

#[test]
fn finger_painting_smudge_deposits_both_dual_color_contributions() {
    let mut b = brush(false, colors(DualColorOrder::SecondBehind, true));
    b.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: true,
        pressure_strength: false,
    };
    b.brush.opacity = 1.;
    let mut d = Document::new(64, 64).unwrap();
    d.begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    d.end_stroke().unwrap();
    assert_eq!(d.pixel(32, 32), rgba([1., 0., 0., 1.]));
    assert_eq!(d.pixel(32, 38), rgba([0., 0., 1., 1.]));
    assert_eq!(d.history_depth(), (1, 0));
}

#[test]
fn regular_smudge_does_not_inject_the_second_color() {
    let mut d = Document::new(64, 64).unwrap();
    d.begin_stroke(
        Brush {
            radius: 24.,
            color: rgba([1., 0., 0., 1.]),
            ..Default::default()
        },
        InputPoint::new(32.5, 32.5, 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    let mut b = brush(false, colors(DualColorOrder::SecondBehind, true));
    b.brush.radius = 8.;
    b.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: false,
        pressure_strength: false,
    };
    d.begin_brush_stroke(b, InputPoint::new(24.5, 32.5, 1.))
        .unwrap();
    d.stroke_to(InputPoint::new(40.5, 32.5, 1.)).unwrap();
    d.end_stroke().unwrap();
    assert!(pixels(&d).iter().all(|p| p.components()[2] < 1e-6));
    assert!(d.history_depth().0 >= 1);
}

#[test]
fn image_holes_and_document_paper_with_color_dynamics_share_real_composition() {
    use std::sync::Arc;
    let mut b = brush(false, colors(DualColorOrder::SecondBehind, true));
    b.mask = Some(Arc::new(
        BrushMask::new(4, 1, vec![255, 0, 255, 0]).unwrap(),
    ));
    let mut single = b.clone();
    single.dual = Default::default();
    let mut reference = Document::new(64, 64).unwrap();
    dab(&mut reference, single);
    let second = SecondaryTip::new(
        8.,
        b.tip,
        Some(Arc::new(BrushMask::new(1, 1, vec![255]).unwrap())),
        Default::default(),
    )
    .unwrap();
    b.dual = DualBrush::new(true, DualMode::Multiply, second)
        .with_color(colors(DualColorOrder::SecondBehind, true));
    let mut colored = Document::new(64, 64).unwrap();
    dab(&mut colored, b.clone());
    for y in 31..34 {
        for x in 29..36 {
            let original = reference.pixel(x, y);
            let actual = colored.pixel(x, y).components();
            assert!((actual[0] - original.components()[0]).abs() < 1e-6);
            assert!((actual[2] - (0.4 - original.alpha())).abs() < 1e-6);
        }
    }
    let empty = SecondaryTip::new(
        8.,
        b.tip,
        Some(Arc::new(BrushMask::new(1, 1, vec![0]).unwrap())),
        Default::default(),
    )
    .unwrap();
    b.dual = DualBrush::new(true, DualMode::Multiply, empty)
        .with_color(colors(DualColorOrder::SecondBehind, true));
    let mut empty = Document::new(64, 64).unwrap();
    dab(&mut empty, b);
    assert_eq!(pixels(&empty), pixels(&reference));
    let paper = Arc::new(TextureResource::new(Arc::new(
        BrushMask::new(1, 1, vec![128]).unwrap(),
    )));
    let mut b = brush(true, colors(DualColorOrder::SecondBehind, true));
    b.texture = BrushTexture::new(true, Default::default(), Some(paper)).unwrap();
    b.dynamics.color = ColorDynamics::new(true, 1., 0., 0., 0., true, 17).unwrap();
    let mut d = Document::new(64, 64).unwrap();
    d.begin_brush_stroke(b, InputPoint::new(16.5, 32.5, 1.))
        .unwrap();
    for x in [48.5, 16.5, 48.5] {
        d.stroke_to(InputPoint::new(x, 32.5, 1.)).unwrap();
    }
    d.end_stroke().unwrap();
    assert!((d.pixel(32, 32).alpha() - 0.4 * 128. / 255.).abs() < 1e-6);
    assert!((d.pixel(32, 38).alpha() - 0.4 * 128. / 255.).abs() < 1e-6);
}

#[test]
fn overflow_sampling_budget_rejects_large_secondary_before_any_pixels_change() {
    let mut b = brush(false, colors(DualColorOrder::SecondBehind, true));
    b.brush.radius = 1.;
    let second = SecondaryTip::new(
        256.,
        b.tip,
        None,
        BrushDynamics {
            scatter: ScatterDynamics::new(true, 1., true, 16, 0., 7).unwrap(),
            ..Default::default()
        },
    )
    .unwrap();
    b.dual = DualBrush::new(true, DualMode::Multiply, second)
        .with_color(colors(DualColorOrder::SecondBehind, true));
    let mut d = Document::new(64, 64).unwrap();
    assert!(matches!(
        d.begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.)),
        Err(Error::ResourceLimit(_))
    ));
    assert!(!d.stroke_active());
    assert_eq!(d.history_depth(), (0, 0));
    assert!(pixels(&d).iter().all(|p| *p == Pixel::TRANSPARENT));
}
