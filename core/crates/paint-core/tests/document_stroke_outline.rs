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
    let mask: Vec<_> = (0..64)
        .flat_map(|y| {
            let program = &program;
            (0..64).map(move |x| {
                if program.coverage(x as f64 + 0.5 - 32., y as f64 + 0.5 - 32., 8.) >= 0.5 {
                    255
                } else {
                    0
                }
            })
        })
        .collect();
    for placement in [
        OutlinePlacement::Outside,
        OutlinePlacement::Center,
        OutlinePlacement::Inside,
    ] {
        for width in [0.5, 3.] {
            let mut base = Document::new(64, 64).unwrap();
            base.begin_brush_stroke(definition.clone(), point).unwrap();
            base.end_stroke().unwrap();
            let band = paint_core::stroke_outline_band(64, 64, &mask, width, placement).unwrap();
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
    doc.begin_outlined_brush_stroke(definition, start, &settings)
        .unwrap();
    doc.stroke_to(end).unwrap();
    doc.end_stroke().unwrap();
    assert_eq!(doc.pixel(48, 24).components()[2], 1.);
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
    assert_eq!(doc.pixel(7, 32), definition.brush.background_color);
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
    large
        .begin_outlined_brush_stroke(
            definition,
            InputPoint::new(10., 10., 1.),
            &outline(OutlinePlacement::Outside, 2., OutlineSource::SelfBrush),
        )
        .unwrap();
    let mut failed = false;
    for step in 1..=40 {
        if large
            .stroke_to(InputPoint::new(
                10. + step as f64 * 100.,
                10. + step as f64 * 100.,
                1.,
            ))
            .is_err()
        {
            failed = true;
            break;
        }
    }
    assert!(failed);
    assert_eq!(large.history_depth(), (0, 0));
    assert_eq!(large.pixel(10, 10), Pixel::TRANSPARENT);
    assert_eq!(large.pixel(4010, 4010), Pixel::TRANSPARENT);
    large
        .begin_stroke(Brush::default(), InputPoint::new(32., 32., 1.))
        .unwrap();
    large.cancel_stroke().unwrap();
}

#[test]
fn active_outline_moves_without_internal_ghosts_and_pending_pages_do_not_publish_partial_overlay() {
    let mut doc = Document::new(96, 64).unwrap();
    let settings = outline(OutlinePlacement::Outside, 3., OutlineSource::SelfBrush);
    doc.begin_outlined_brush_stroke(brush(), InputPoint::new(24., 32., 1.), &settings)
        .unwrap();
    assert_eq!(doc.pixel(33, 32).components()[2], 1.);
    doc.begin_stroke_input(InputPoint::new(72., 32., 1.))
        .unwrap();
    assert!(!doc.stroke_page_ready_for_publication());
    let mut pages = 0;
    loop {
        let result = doc
            .continue_stroke_page(InputScheduleBudget::new(1, 100000, 1).unwrap())
            .unwrap();
        pages += 1;
        if matches!(result, StrokePageResult::InputComplete) {
            break;
        }
        assert!(!doc.stroke_page_ready_for_publication());
    }
    assert!(pages > 1);
    assert!(doc.stroke_page_ready_for_publication());
    assert_eq!(doc.pixel(33, 32), brush().brush.color);
    assert_eq!(doc.pixel(81, 32).components()[2], 1.);
    let active = pixels(&doc, 96, 64);
    doc.end_stroke().unwrap();
    assert_eq!(pixels(&doc, 96, 64), active);
    assert_eq!(doc.history_depth(), (1, 0));
    doc.undo().unwrap();
    assert!(pixels(&doc, 96, 64).iter().all(|p| *p == [0; 4]));
}

#[test]
fn existing_brush_snapshot_supports_image_spacing_scatter_and_hole_contours() {
    use paint_core::{BrushDynamics, ScatterDynamics, SecondaryTip};
    let primary_pixels: Vec<_> = (0..64)
        .map(|i| {
            if (2..6).contains(&(i % 8)) && (2..6).contains(&(i / 8)) {
                0
            } else {
                255
            }
        })
        .collect();
    let mut primary = brush();
    primary.brush.radius = 12.;
    primary.mask = Some(Arc::new(BrushMask::new(8, 8, primary_pixels).unwrap()));
    let image = Arc::new(
        BrushMask::new(4, 2, vec![255, 0, 255, 0, 255, 255, 0, 255])
            .unwrap()
            .with_resource_id("c".repeat(64))
            .unwrap(),
    );
    let mut results = Vec::new();
    for scattered in [false, true] {
        let mut dynamics = BrushDynamics::default();
        dynamics.scatter = ScatterDynamics::new(scattered, 0.4, true, 2, 0.2, 31).unwrap();
        let tip = SecondaryTip::new(
            3.,
            BrushTip {
                pressure_size: false,
                pressure_flow: false,
                ..Default::default()
            },
            Some(image.clone()),
            dynamics,
        )
        .unwrap()
        .with_spacing(if scattered { 0.7 } else { 0.1 })
        .unwrap();
        let settings = outline(
            OutlinePlacement::Outside,
            2.,
            OutlineSource::Existing(Arc::new(tip)),
        );
        for paged in [false, true] {
            let mut doc = Document::new(64, 64).unwrap();
            doc.begin_outlined_brush_stroke(
                primary.clone(),
                InputPoint::new(32., 32., 1.),
                &settings,
            )
            .unwrap();
            finish(&mut doc, paged);
            assert_eq!(doc.history_depth(), (1, 0));
            let image = pixels(&doc, 64, 64);
            assert!(image.iter().any(|p| f32::from_bits(p[2]) > 0.2));
            assert_eq!(doc.pixel(32, 32), Pixel::TRANSPARENT);
            assert!((25..30).any(|x| doc.pixel(x, 32).components()[2] > 0.));
            if paged {
                assert_eq!(*results.last().unwrap(), image);
            } else {
                results.push(image);
            }
        }
    }
    assert_ne!(results[0], results[1]);
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
