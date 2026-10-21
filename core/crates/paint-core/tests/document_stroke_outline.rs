use paint_core::{
    Brush, BrushDefinition, BrushMask, BrushProgram, BrushTexture, BrushTip, Document,
    DocumentOptions, InputPoint, InputScheduleBudget, OutlinePlacement, OutlineSource,
    OutlineWidth, Pixel, PressureControl, PressureCurve, StrokeOutline, StrokePageResult,
    StrokeTaper, TaperEnd, TextureResource, TextureSettings, TextureSpace,
};
use std::sync::Arc;

fn brush() -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            spacing: 0.5,
            color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
            background_color: Pixel::from_straight([0., 0., 1., 1.]).unwrap(),
            ..Default::default()
        },
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn outline(placement: OutlinePlacement, width: f32, source: OutlineSource) -> StrokeOutline {
    StrokeOutline::new(
        true,
        placement,
        OutlineWidth::new(false, width, 10.).unwrap(),
        source,
        true,
    )
    .unwrap()
}
fn pixels(doc: &Document, w: u32, h: u32) -> Vec<[u32; 4]> {
    (0..h)
        .flat_map(|y| (0..w).map(move |x| doc.pixel(x, y).components().map(f32::to_bits)))
        .collect()
}
fn finish(doc: &mut Document, paged: bool) {
    if !paged {
        assert!(doc.end_stroke().unwrap());
        return;
    }
    doc.begin_stroke_finish().unwrap();
    loop {
        match doc
            .continue_stroke_page(InputScheduleBudget::new(3, 100000, 3).unwrap())
            .unwrap()
        {
            StrokePageResult::Pending => {}
            StrokePageResult::StrokeComplete { changed, .. } => {
                assert!(changed);
                break;
            }
            _ => panic!("unexpected finish result"),
        }
    }
}

#[test]
fn document_three_placements_match_independent_geometry_and_keep_one_history_command() {
    let definition = brush();
    let program = BrushProgram::compile(definition.clone()).unwrap();
    let point = InputPoint::new(32., 32., 1.);
    for placement in [
        OutlinePlacement::Outside,
        OutlinePlacement::Center,
        OutlinePlacement::Inside,
    ] {
        for width in [0.5, 3.] {
            let mut base = Document::new(64, 64).unwrap();
            base.begin_brush_stroke(definition.clone(), point).unwrap();
            base.end_stroke().unwrap();
            // Independent two-radius reference, without invoking the replay operator.
            let (outward, inward) = placement.extents(width);
            let coverage = |x: u32, y: u32, radius: f64| {
                if radius <= 0. {
                    return 0;
                }
                (program.coverage(
                    (x as f64 + 0.5 - 32.) * 8. / radius,
                    (y as f64 + 0.5 - 32.) * 8. / radius,
                    8.,
                ) * 255.)
                    .round() as u8
            };
            let band: Vec<_> = (0..64 * 64)
                .map(|i| {
                    let outer = coverage(i % 64, i / 64, 8. + f64::from(outward));
                    let inner = coverage(i % 64, i / 64, (8. - f64::from(inward)).max(0.));
                    f32::from(outer.saturating_sub(inner)) / 255.
                })
                .collect();
            let mut actual = Document::new(64, 64).unwrap();
            actual
                .begin_outlined_brush_stroke(
                    definition.clone(),
                    point,
                    &outline(placement, width, OutlineSource::SelfBrush),
                )
                .unwrap();
            actual.end_stroke().unwrap();
            assert_eq!(actual.history_depth(), (1, 0));
            for y in 0..64 {
                for x in 0..64 {
                    let expected = definition
                        .brush
                        .background_color
                        .scaled(band[(y * 64 + x) as usize])
                        .over(base.pixel(x, y));
                    assert_eq!(
                        actual.pixel(x, y),
                        expected,
                        "{placement:?}, {width}, {x},{y}"
                    );
                }
            }
            let final_pixels = pixels(&actual, 64, 64);
            actual.undo().unwrap();
            assert!(pixels(&actual, 64, 64).iter().all(|p| *p == [0; 4]));
            actual.redo().unwrap();
            assert_eq!(pixels(&actual, 64, 64), final_pixels);
        }
    }
}

#[test]
fn drawing_same_colour_still_has_outline_and_cancellation_restores_previous_pixels() {
    let definition = brush();
    let mut doc = Document::new(96, 64).unwrap();
    let start = InputPoint::new(24., 32., 1.);
    let end = InputPoint::new(72., 32., 1.);
    doc.begin_brush_stroke(definition.clone(), start).unwrap();
    doc.stroke_to(end).unwrap();
    doc.end_stroke().unwrap();
    let original = pixels(&doc, 96, 64);
    let settings = outline(OutlinePlacement::Inside, 2., OutlineSource::SelfBrush);
    doc.begin_outlined_brush_stroke(definition.clone(), start, &settings)
        .unwrap();
    doc.stroke_to(end).unwrap();
    doc.cancel_stroke().unwrap();
    assert_eq!(pixels(&doc, 96, 64), original);
    assert_eq!(doc.history_depth(), (1, 0));
    doc.begin_outlined_brush_stroke(definition.clone(), start, &settings)
        .unwrap();
    doc.stroke_to(end).unwrap();
    doc.end_stroke().unwrap();
    let primary = BrushProgram::compile(definition).unwrap();
    let edge = (primary.coverage(0.5, -7.5, 8.) * 255.).round() / 255.;
    assert_eq!(doc.pixel(48, 24).components()[2], edge);
    assert_eq!(doc.pixel(48, 32).components()[0], 1.);
    assert_eq!(doc.history_depth(), (2, 0));
    doc.undo().unwrap();
    assert_eq!(pixels(&doc, 96, 64), original);
}

#[test]
fn tapered_outline_paging_matches_direct_and_tail_replay_does_not_leave_old_shape() {
    let mut definition = brush();
    definition.brush.spacing = 0.3;
    let control = PressureControl::new(PressureCurve::default(), 0.).unwrap();
    definition.taper = StrokeTaper::new(
        true,
        TaperEnd::new(18., control.clone(), PressureControl::default()).unwrap(),
        TaperEnd::new(24., control, PressureControl::default()).unwrap(),
    );
    let points = [
        InputPoint::new(20., 20., 1.),
        InputPoint::new(56., 32., 0.7),
        InputPoint::new(98., 40., 1.),
    ];
    {
        let settings = outline(OutlinePlacement::Outside, 3., OutlineSource::SelfBrush);
        let mut results = Vec::new();
        for paged in [false, true] {
            let mut doc = Document::new(128, 64).unwrap();
            doc.begin_outlined_brush_stroke(definition.clone(), points[0], &settings)
                .unwrap();
            for point in &points[1..] {
                if paged {
                    doc.begin_stroke_input(*point).unwrap();
                    while matches!(
                        doc.continue_stroke_page(InputScheduleBudget::new(3, 100000, 3).unwrap())
                            .unwrap(),
                        StrokePageResult::Pending
                    ) {}
                } else {
                    doc.stroke_to(*point).unwrap();
                }
            }
            finish(&mut doc, paged);
            assert_eq!(doc.history_depth(), (1, 0));
            results.push(pixels(&doc, 128, 64));
        }
        assert_eq!(results[0], results[1]);
    }
}

#[test]
fn outlines_capture_geometry_outside_canvas_instead_of_outlining_canvas_border() {
    let mut definition = brush();
    definition.brush.radius = 12.;
    let mut doc = Document::new(64, 64).unwrap();
    doc.begin_outlined_brush_stroke(
        definition.clone(),
        InputPoint::new(-4., 32., 1.),
        &outline(OutlinePlacement::Inside, 2., OutlineSource::SelfBrush),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    assert_eq!(doc.pixel(0, 32), definition.brush.color);
    let primary = BrushProgram::compile(definition.clone()).unwrap();
    let edge = (primary.coverage(11.5, 0.5, 12.) * 255.).round() / 255.;
    assert_eq!(doc.pixel(7, 32).components()[2], edge);
    let mut outside = Document::new(64, 64).unwrap();
    definition.brush.radius = 8.;
    outside
        .begin_outlined_brush_stroke(
            definition.clone(),
            InputPoint::new(-10., 32., 1.),
            &outline(OutlinePlacement::Outside, 4., OutlineSource::SelfBrush),
        )
        .unwrap();
    assert!(outside.end_stroke().unwrap());
    assert_eq!(outside.pixel(0, 32), definition.brush.background_color);
}

#[test]
fn texture_outline_uses_captured_local_or_document_space_and_real_resource() {
    let mask = Arc::new(BrushMask::new(4, 2, vec![0, 0, 255, 255, 0, 0, 255, 255]).unwrap());
    let mut outputs = Vec::new();
    for space in [TextureSpace::Document, TextureSpace::Tip] {
        let texture = BrushTexture::new(
            true,
            TextureSettings::new(space, 1., 0., 1., false, 0., 0.).unwrap(),
            Some(Arc::new(TextureResource::new(mask.clone()))),
        )
        .unwrap();
        let mut doc = Document::new(64, 64).unwrap();
        doc.begin_outlined_brush_stroke(
            brush(),
            InputPoint::new(31., 32., 1.),
            &outline(
                OutlinePlacement::Outside,
                4.,
                OutlineSource::Texture(texture),
            ),
        )
        .unwrap();
        doc.end_stroke().unwrap();
        outputs.push(pixels(&doc, 64, 64));
        let blue: Vec<_> = (15..48).map(|x| doc.pixel(x, 21).components()[2]).collect();
        assert!(blue.iter().any(|v| *v > 0.5));
        assert!(blue.iter().any(|v| *v < 0.1));
    }
    assert_ne!(outputs[0], outputs[1]);
}

#[test]
fn outline_tile_and_bounding_area_failures_roll_back_entire_stroke() {
    let mut definition = brush();
    definition.brush.radius = 2.;
    let mut limited = Document::with_options(
        128,
        128,
        DocumentOptions {
            max_document_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(limited
        .begin_outlined_brush_stroke(
            definition.clone(),
            InputPoint::new(32., 32., 1.),
            &outline(OutlinePlacement::Outside, 40., OutlineSource::SelfBrush),
        )
        .is_err());
    assert_eq!(limited.history_depth(), (0, 0));
    assert!(pixels(&limited, 128, 128).iter().all(|p| *p == [0; 4]));
    let mut large = Document::new(8192, 8192).unwrap();
    definition.brush.radius = 0.5;
    definition.brush.spacing = 1.;
    let failed = large
        .begin_outlined_brush_stroke(
            definition,
            InputPoint::new(10., 10., 1.),
            &outline(OutlinePlacement::Outside, 2., OutlineSource::SelfBrush),
        )
        .and_then(|()| {
            for step in 1..=40 {
                large.stroke_to(InputPoint::new(
                    10. + step as f64 * 100.,
                    10. + step as f64 * 100.,
                    1.,
                ))?;
            }
            large.end_stroke().map(|_| ())
        });
    assert!(failed.is_err());
    assert_eq!(large.history_depth(), (0, 0));
    assert_eq!(large.pixel(10, 10), Pixel::TRANSPARENT);
    assert_eq!(large.pixel(4010, 4010), Pixel::TRANSPARENT);
    large
        .begin_stroke(Brush::default(), InputPoint::new(32., 32., 1.))
        .unwrap();
    large.cancel_stroke().unwrap();
}

#[test]
fn selection_layer_offset_and_transparency_lock_apply_to_outline_pixels() {
    use paint_core::{
        LayerAppearance, Selection, SelectionKind, SelectionOperation, SelectionShape,
        LOCK_TRANSPARENCY,
    };
    let mut shifted = Document::new(96, 64).unwrap();
    let mut unshifted = Document::new(96, 64).unwrap();
    shifted
        .set_layer_appearance(
            shifted.active_layer(),
            LayerAppearance {
                offset_x: 13,
                offset_y: -3,
                ..Default::default()
            },
        )
        .unwrap();
    let selection = Selection::default()
        .apply(
            SelectionShape::geometry(SelectionKind::Rectangle, 30., 0., 40., 64., false),
            SelectionOperation::Replace,
            96,
            64,
        )
        .unwrap();
    for doc in [&mut shifted, &mut unshifted] {
        doc.set_selection(selection.clone()).unwrap();
        doc.begin_outlined_brush_stroke(
            brush(),
            InputPoint::new(32., 32., 1.),
            &outline(OutlinePlacement::Outside, 4., OutlineSource::SelfBrush),
        )
        .unwrap();
        doc.end_stroke().unwrap();
        assert_eq!(doc.pixel(29, 32), Pixel::TRANSPARENT);
        assert_eq!(doc.pixel(42, 32).components()[2], 1.);
    }
    assert_eq!(pixels(&shifted, 96, 64), pixels(&unshifted, 96, 64));
    let mut locked = Document::new(64, 64).unwrap();
    locked
        .begin_brush_stroke(brush(), InputPoint::new(32., 32., 1.))
        .unwrap();
    locked.end_stroke().unwrap();
    let alpha: Vec<_> = pixels(&locked, 64, 64).iter().map(|p| p[3]).collect();
    locked
        .set_layer_appearance(
            locked.active_layer(),
            LayerAppearance {
                locks: LOCK_TRANSPARENCY,
                ..Default::default()
            },
        )
        .unwrap();
    let history = locked.history_depth();
    locked
        .begin_outlined_brush_stroke(
            brush(),
            InputPoint::new(32., 32., 1.),
            &outline(OutlinePlacement::Center, 4., OutlineSource::SelfBrush),
        )
        .unwrap();
    locked.end_stroke().unwrap();
    assert_eq!(
        pixels(&locked, 64, 64)
            .iter()
            .map(|p| p[3])
            .collect::<Vec<_>>(),
        alpha
    );
    assert_eq!(locked.history_depth().0, history.0 + 1);
    assert!(locked.pixel(38, 32).components()[2] > 0.);
    assert_eq!(locked.pixel(42, 32), Pixel::TRANSPARENT);
}

#[test]
fn replay_actual_stroke_has_all_placements_aa_max_opacity_and_exact_history() {
    for placement in [
        OutlinePlacement::Outside,
        OutlinePlacement::Inside,
        OutlinePlacement::Center,
    ] {
        let mut definition = brush();
        definition.brush.opacity = 0.5;
        let settings = outline(placement, 4., OutlineSource::SelfBrush);
        let mut doc = Document::new(96, 64).unwrap();
        doc.begin_outlined_brush_stroke(
            definition.clone(),
            InputPoint::new(28., 32., 1.),
            &settings,
        )
        .unwrap();
        doc.stroke_to(InputPoint::new(68., 32., 1.)).unwrap();
        doc.end_stroke().unwrap();
        let result = pixels(&doc, 96, 64);
        let blue: Vec<_> = result.iter().map(|p| f32::from_bits(p[2])).collect();
        assert!(blue.iter().any(|v| *v > 0.4), "{placement:?}");
        assert!(blue.iter().any(|v| *v > 0. && *v < 0.1));
        assert!(blue.iter().all(|v| *v <= 0.5), "stamp overlap must use max");
        assert_eq!(
            doc.pixel(48, 32).components()[2],
            0.,
            "inside must not cover eroded interior"
        );
        assert_eq!(doc.history_depth(), (1, 0));
        doc.undo().unwrap();
        assert!(pixels(&doc, 96, 64).iter().all(|p| *p == [0; 4]));
        doc.redo().unwrap();
        assert_eq!(pixels(&doc, 96, 64), result);
        doc.begin_outlined_brush_stroke(definition, InputPoint::new(48., 20., 1.), &settings)
            .unwrap();
        doc.cancel_stroke().unwrap();
        assert_eq!(pixels(&doc, 96, 64), result);
    }
}

#[test]
fn replay_captured_image_tip_and_outline_texture_change_only_the_outline() {
    let definition = brush();
    let image = Arc::new(
        BrushMask::new(
            4,
            4,
            vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0],
        )
        .unwrap(),
    );
    let texture = BrushTexture::new(
        true,
        TextureSettings::default(),
        Some(Arc::new(TextureResource::new(Arc::new(
            BrushMask::new(1, 1, vec![0]).unwrap(),
        )))),
    )
    .unwrap();
    let existing =
        paint_core::SecondaryTip::new(8., BrushTip::default(), Some(image), Default::default())
            .unwrap();
    let mut outputs = Vec::new();
    for source in [
        OutlineSource::SelfBrush,
        OutlineSource::Existing(Arc::new(existing)),
        OutlineSource::SelfTextured(texture),
    ] {
        let settings = outline(OutlinePlacement::Outside, 5., source);
        let mut doc = Document::new(64, 64).unwrap();
        doc.begin_outlined_brush_stroke(
            definition.clone(),
            InputPoint::new(32., 32., 1.),
            &settings,
        )
        .unwrap();
        doc.end_stroke().unwrap();
        assert_eq!(doc.pixel(32, 32), definition.brush.color);
        outputs.push(pixels(&doc, 64, 64));
    }
    assert_ne!(outputs[0], outputs[1]);
    assert!(outputs[2].iter().all(|p| f32::from_bits(p[2]) == 0.));
}
