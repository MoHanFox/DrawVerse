use paint_core::*;
use std::sync::Arc;
fn tip() -> BrushTip {
    BrushTip {
        pressure_size: false,
        pressure_flow: false,
        ..Default::default()
    }
}
fn brush(spacing: f32) -> BrushDefinition {
    let secondary_tip = BrushTip {
        roundness: 0.2,
        angle: 90.,
        ..tip()
    };
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            opacity: 0.4,
            spacing: 0.02,
            ..Default::default()
        },
        tip: tip(),
        dual: DualBrush::new(
            true,
            DualMode::Multiply,
            SecondaryTip::new(4., secondary_tip, None, Default::default())
                .unwrap()
                .with_spacing(spacing)
                .unwrap(),
        ),
        ..Default::default()
    }
}
fn pixels(doc: &Document) -> Vec<Pixel> {
    (0..doc.dimensions().1)
        .flat_map(|y| (0..doc.dimensions().0).map(move |x| doc.pixel(x, y)))
        .collect()
}
fn line(doc: &mut Document, b: BrushDefinition, split: bool) {
    doc.begin_brush_stroke(b, InputPoint::new(16.5, 32.5, 1.))
        .unwrap();
    if split {
        doc.stroke_to(InputPoint::new(32.5, 32.5, 1.)).unwrap();
    }
    doc.stroke_to(InputPoint::new(48.5, 32.5, 1.)).unwrap();
    doc.end_stroke().unwrap();
}
#[test]
fn independent_diameter_spacing_creates_real_stripes_and_exact_history() {
    let mut sparse = Document::new(64, 64).unwrap();
    line(&mut sparse, brush(1.), false);
    assert_eq!(sparse.pixel(20, 32), Pixel::TRANSPARENT);
    assert!((sparse.pixel(24, 32).alpha() - 0.4).abs() < 1e-6);
    let mut dense = Document::new(64, 64).unwrap();
    line(&mut dense, brush(0.25), false);
    assert!(dense.pixel(20, 32).alpha() > 0.3);
    assert_ne!(pixels(&sparse), pixels(&dense));
    let after = pixels(&sparse);
    sparse.undo().unwrap();
    assert!(pixels(&sparse).iter().all(|p| *p == Pixel::TRANSPARENT));
    sparse.redo().unwrap();
    assert_eq!(pixels(&sparse), after);
    assert!(SecondaryTip::default().with_spacing(0.0099999).is_err());
    assert!(SecondaryTip::default().with_spacing(f32::NAN).is_err());
}
#[test]
fn own_scatter_shape_sequence_is_input_chunk_invariant_and_endpoint_flushes() {
    let mut b = brush(0.17);
    b.brush.spacing = 0.3;
    b.dynamics.scatter = ScatterDynamics::new(true, 0.1, true, 2, 0.5, 9).unwrap();
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(
            4.,
            tip(),
            None,
            BrushDynamics {
                scatter: ScatterDynamics::new(true, 0.5, true, 3, 0.5, 9).unwrap(),
                shape_jitter: ShapeJitter::new(true, 0.5, 0.2, 0.4, 0.5, 0.2, 9).unwrap(),
                ..Default::default()
            },
        )
        .unwrap()
        .with_spacing(0.17)
        .unwrap(),
    );
    let mut whole = Document::new(64, 64).unwrap();
    let mut split = Document::new(64, 64).unwrap();
    line(&mut whole, b.clone(), false);
    line(&mut split, b, true);
    assert_eq!(pixels(&whole), pixels(&split));
    assert!(pixels(&whole).iter().any(|p| p.alpha() > 0.));
    let mut end = Document::new(64, 64).unwrap();
    let mut b = brush(1.);
    b.brush.spacing = 1.;
    end.begin_brush_stroke(b, InputPoint::new(16.5, 32.5, 1.))
        .unwrap();
    end.stroke_to(InputPoint::new(21.7, 32.5, 1.)).unwrap();
    end.end_stroke().unwrap();
    assert!(end.pixel(21, 32).alpha() > 0.3);
}
#[test]
fn paged_secondary_field_matches_resident_pixels_cancel_and_redo() {
    let options = DocumentOptions {
        max_resident_tiles: 1,
        ..Default::default()
    };
    let mut paged = Document::with_options(192, 64, options).unwrap();
    let mut resident = Document::new(192, 64).unwrap();
    for d in [&mut paged, &mut resident] {
        d.begin_brush_stroke(brush(0.5), InputPoint::new(16.5, 32.5, 1.))
            .unwrap();
        d.stroke_to(InputPoint::new(170.5, 32.5, 1.)).unwrap();
        d.end_stroke().unwrap();
    }
    assert_eq!(pixels(&paged), pixels(&resident));
    assert!(paged.storage_stats().page_writes > 0);
    assert!(paged.tile_count() <= 3);
    let after = pixels(&paged);
    paged.undo().unwrap();
    assert!(pixels(&paged).iter().all(|p| *p == Pixel::TRANSPARENT));
    paged.redo().unwrap();
    assert_eq!(pixels(&paged), after);
    paged
        .begin_brush_stroke(brush(0.5), InputPoint::new(16.5, 32.5, 1.))
        .unwrap();
    paged.stroke_to(InputPoint::new(80.5, 32.5, 1.)).unwrap();
    paged.cancel_stroke().unwrap();
    assert_eq!(pixels(&paged), after);
}
#[test]
fn secondary_field_limit_and_density_precheck_do_not_leave_partial_strokes() {
    let mut limited = Document::with_options(
        128,
        64,
        DocumentOptions {
            max_document_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let before_history = limited.history_depth();
    let mut b = brush(1.);
    b.brush.radius = 0.5;
    b.brush.spacing = 1.;
    limited
        .begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    assert!(matches!(
        limited.stroke_to(InputPoint::new(80.5, 32.5, 1.)),
        Err(Error::ResourceLimit(
            "secondary coverage tile limit; entire stroke rolled back"
        ))
    ));
    assert!(!limited.stroke_active());
    assert_eq!(limited.tile_count(), 0);
    assert_eq!(limited.history_depth(), before_history);
    let mut b = brush(0.01);
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(
            0.5,
            tip(),
            None,
            BrushDynamics {
                scatter: ScatterDynamics::new(true, 0.1, true, 2, 0.5, 13).unwrap(),
                ..Default::default()
            },
        )
        .unwrap()
        .with_spacing(0.01)
        .unwrap(),
    );
    let mut tested = Document::new(64, 64).unwrap();
    let mut reference = Document::new(64, 64).unwrap();
    for d in [&mut tested, &mut reference] {
        d.begin_brush_stroke(b.clone(), InputPoint::new(16.5, 32.5, 1.))
            .unwrap();
    }
    let before = pixels(&tested);
    assert!(matches!(
        tested.stroke_to(InputPoint::new(116.5, 32.5, 1.)),
        Err(Error::ResourceLimit(_))
    ));
    assert_eq!(pixels(&tested), before);
    for d in [&mut tested, &mut reference] {
        d.stroke_to(InputPoint::new(17.5, 32.5, 1.)).unwrap();
        d.end_stroke().unwrap();
    }
    assert_eq!(pixels(&tested), pixels(&reference));
}
#[test]
fn selection_mask_erase_and_moved_layer_read_the_same_secondary_field() {
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
        let mut b = brush(1.);
        b.brush.mode = if mask {
            BrushMode::Paint
        } else {
            BrushMode::Erase
        };
        line(&mut d, b, false);
        assert_eq!(d.pixel(24, 32), Pixel::WHITE);
        assert_eq!(d.pixel(36, 32), Pixel::WHITE);
        assert!((d.pixel(40, 32).alpha() - 0.6).abs() < 1e-6);
    }
    let mut normal = Document::new(64, 64).unwrap();
    let mut moved = Document::new(64, 64).unwrap();
    let mut appearance = moved.layer(1).unwrap().appearance();
    appearance.offset_x = 3;
    moved.set_layer_appearance(1, appearance).unwrap();
    line(&mut normal, brush(1.), false);
    line(&mut moved, brush(1.), false);
    assert_eq!(pixels(&normal), pixels(&moved));
}
#[test]
fn own_path_smudge_transports_canvas_color_and_stationary_pressure_preserves_phase() {
    let mut d = Document::new(64, 64).unwrap();
    d.begin_stroke(
        Brush {
            radius: 10.,
            color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
            ..Default::default()
        },
        InputPoint::new(16.5, 32.5, 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    let before = pixels(&d);
    let mut b = brush(0.25);
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(4., tip(), None, Default::default())
            .unwrap()
            .with_spacing(0.25)
            .unwrap(),
    );
    b.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: false,
        pressure_strength: false,
    };
    line(&mut d, b, false);
    let after = pixels(&d);
    assert!(before
        .iter()
        .zip(&after)
        .any(|(a, b)| a.alpha() == 0. && b.components()[0] > 0.0001));
    d.undo().unwrap();
    assert_eq!(pixels(&d), before);
    d.redo().unwrap();
    assert_eq!(pixels(&d), after);
    let mut b = brush(1.);
    let second = BrushTip {
        pressure_size: true,
        ..tip()
    };
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(8., second, None, Default::default())
            .unwrap()
            .with_spacing(1.)
            .unwrap(),
    );
    let mut d = Document::new(64, 64).unwrap();
    d.begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 0.25))
        .unwrap();
    assert_eq!(d.pixel(37, 32), Pixel::TRANSPARENT);
    d.stroke_to(InputPoint::new(32.5, 32.5, 0.75)).unwrap();
    assert!(d.pixel(37, 32).alpha() > 0.3);
    d.end_stroke().unwrap();
}
#[test]
fn image_holes_and_directional_turns_are_sampled_on_the_secondary_path() {
    let mut bytes = vec![255; 256];
    for y in 6..10 {
        for x in 6..10 {
            bytes[y * 16 + x] = 0;
        }
    }
    let mut b = brush(0.25);
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(
            8.,
            tip(),
            Some(Arc::new(BrushMask::new(16, 16, bytes).unwrap())),
            Default::default(),
        )
        .unwrap()
        .with_spacing(0.25)
        .unwrap(),
    );
    let mut d = Document::new(64, 64).unwrap();
    d.begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    d.end_stroke().unwrap();
    assert_eq!(d.pixel(32, 32), Pixel::TRANSPARENT);
    assert!(d.pixel(37, 32).alpha() > 0.3);
    let mut b = brush(1.);
    let second = BrushTip {
        roundness: 0.2,
        ..tip()
    };
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(
            4.,
            second,
            None,
            BrushDynamics {
                tip: TipDynamics::new(AngleControl::Direction, false, 0.01).unwrap(),
                ..Default::default()
            },
        )
        .unwrap()
        .with_spacing(1.)
        .unwrap(),
    );
    let mut whole = Document::new(64, 64).unwrap();
    let mut split = Document::new(64, 64).unwrap();
    for (d, parts) in [(&mut whole, false), (&mut split, true)] {
        d.begin_brush_stroke(b.clone(), InputPoint::new(16.5, 32.5, 1.))
            .unwrap();
        if parts {
            d.stroke_to(InputPoint::new(32.5, 32.5, 1.)).unwrap();
        }
        d.stroke_to(InputPoint::new(48.5, 32.5, 1.)).unwrap();
        if parts {
            d.stroke_to(InputPoint::new(48.5, 44.5, 1.)).unwrap();
        }
        d.stroke_to(InputPoint::new(48.5, 56.5, 1.)).unwrap();
        d.end_stroke().unwrap();
    }
    assert_eq!(pixels(&whole), pixels(&split));
    assert!(whole.pixel(48, 40).alpha() > 0.3);
    assert_eq!(whole.pixel(43, 40), Pixel::TRANSPARENT);
}
