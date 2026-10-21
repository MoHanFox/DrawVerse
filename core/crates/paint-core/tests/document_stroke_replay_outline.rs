use paint_core::{
    Brush, BrushDefinition, BrushMask, BrushProgram, BrushTip, Document, DocumentOptions,
    InputPoint, InputScheduleBudget, OutlinePlacement, OutlineSource, OutlineWidth, Pixel,
    PressureControl, PressureCurve, SecondaryTip, StrokeOutline, StrokePageResult, StrokeTaper,
    TaperEnd,
};
use paint_ops::replay_outline::{self, ReplayDab};
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
fn outline(placement: OutlinePlacement, source: OutlineSource) -> StrokeOutline {
    StrokeOutline::new(
        true,
        placement,
        OutlineWidth::new(false, 3., 10.).unwrap(),
        source,
        true,
    )
    .unwrap()
}
fn pixels(doc: &Document) -> Vec<Pixel> {
    (0..64)
        .flat_map(|y| (0..96).map(move |x| doc.pixel(x, y)))
        .collect()
}

#[test]
fn actual_soft_and_image_dabs_match_two_scale_operator_for_all_placements() {
    for image in [false, true] {
        let mut definition = brush();
        definition.tip.hardness = 0.4;
        definition.tip.angle = 27.;
        definition.tip.roundness = 0.65;
        if image {
            definition.mask = Some(Arc::new(
                BrushMask::new(3, 2, vec![0, 40, 255, 190, 90, 0]).unwrap(),
            ));
        }
        let program = BrushProgram::compile(definition.clone()).unwrap();
        for placement in [
            OutlinePlacement::Outside,
            OutlinePlacement::Center,
            OutlinePlacement::Inside,
        ] {
            let point = InputPoint::new(32.3, 31.7, 1.);
            let mut primary = Document::new(96, 64).unwrap();
            primary
                .begin_brush_stroke(definition.clone(), point)
                .unwrap();
            primary.end_stroke().unwrap();
            let (outside, inside) = placement.extents(3.);
            let coverage = replay_outline::accumulate(
                &ReplayDab {
                    center: [point.x, point.y],
                    radius: 8.,
                    outside: outside.into(),
                    inside: inside.into(),
                    shared_source: true,
                },
                [0, 0],
                [64, 64],
                &vec![[0; 4]; 4096],
                |_, p| Ok(program.coverage(p[0] - point.x, p[1] - point.y, 8.)),
            )
            .unwrap();
            let mut actual = Document::new(96, 64).unwrap();
            actual
                .begin_outlined_brush_stroke(
                    definition.clone(),
                    point,
                    &outline(placement, OutlineSource::SelfBrush),
                )
                .unwrap();
            actual.end_stroke().unwrap();
            for y in 0..64 {
                for x in 0..64 {
                    let band = replay_outline::coverage(coverage[(y * 64 + x) as usize], false);
                    assert_eq!(
                        actual.pixel(x, y),
                        definition
                            .brush
                            .background_color
                            .scaled(band)
                            .over(primary.pixel(x, y)),
                        "image={image}, {placement:?} at {x},{y}"
                    );
                }
            }
            let result = pixels(&actual);
            assert_eq!(actual.history_depth(), (1, 0));
            actual.undo().unwrap();
            assert!(pixels(&actual).iter().all(|p| *p == Pixel::TRANSPARENT));
            actual.redo().unwrap();
            assert_eq!(pixels(&actual), result);
        }
    }
}

#[test]
fn independent_tip_overflow_changes_geometry_but_self_overflow_does_not() {
    let tip = SecondaryTip::new(
        18.,
        BrushTip {
            roundness: 0.3,
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        None,
        Default::default(),
    )
    .unwrap();
    let mut results = Vec::new();
    for allow in [false, true] {
        let mut doc = Document::new(96, 64).unwrap();
        doc.begin_outlined_brush_stroke(
            brush(),
            InputPoint::new(32., 32., 1.),
            &outline(
                OutlinePlacement::Outside,
                OutlineSource::Existing(Arc::new(tip.clone())),
            )
            .with_allow_overflow(allow),
        )
        .unwrap();
        doc.end_stroke().unwrap();
        assert_eq!(doc.pixel(54, 32).components()[2] > 0., allow);
        results.push(pixels(&doc));
    }
    assert_ne!(results[0], results[1]);
    let mut own = Vec::new();
    for allow in [false, true] {
        let mut doc = Document::new(96, 64).unwrap();
        doc.begin_outlined_brush_stroke(
            brush(),
            InputPoint::new(32., 32., 1.),
            &outline(OutlinePlacement::Outside, OutlineSource::SelfBrush)
                .with_allow_overflow(allow),
        )
        .unwrap();
        doc.end_stroke().unwrap();
        own.push(pixels(&doc));
    }
    assert_eq!(own[0], own[1]);
}

#[test]
fn paged_pressure_taper_and_crossing_preview_match_single_history_and_cancel() {
    let mut definition = brush();
    definition.tip.pressure_size = true;
    definition.taper = StrokeTaper::new(
        true,
        TaperEnd::default(),
        TaperEnd::new(
            18.,
            PressureControl::new(PressureCurve::default(), 0.).unwrap(),
            Default::default(),
        )
        .unwrap(),
    );
    let points = [
        InputPoint::new(20., 20., 0.6),
        InputPoint::new(68., 44., 1.),
        InputPoint::new(20., 44., 0.8),
        InputPoint::new(68., 20., 0.5),
    ];
    let settings = outline(OutlinePlacement::Outside, OutlineSource::SelfBrush);
    let mut results = Vec::new();
    for paged in [false, true] {
        let mut doc = Document::new(96, 64).unwrap();
        doc.begin_outlined_brush_stroke(definition.clone(), points[0], &settings)
            .unwrap();
        for p in &points[1..] {
            if paged {
                doc.begin_stroke_input(*p).unwrap();
                while matches!(
                    doc.continue_stroke_page(InputScheduleBudget::new(1, 100000, 1).unwrap())
                        .unwrap(),
                    StrokePageResult::Pending
                ) {}
            } else {
                doc.stroke_to(*p).unwrap();
            }
        }
        doc.end_stroke().unwrap();
        assert_eq!(doc.history_depth(), (1, 0));
        let original = pixels(&doc);
        doc.begin_outlined_brush_stroke(definition.clone(), points[0], &settings)
            .unwrap();
        doc.stroke_to(points[1]).unwrap();
        doc.cancel_stroke().unwrap();
        assert_eq!(pixels(&doc), original);
        assert_eq!(doc.history_depth(), (1, 0));
        results.push(original);
    }
    assert_eq!(results[0], results[1]);
}

#[test]
fn replay_limits_roll_back_pixels_and_leave_document_usable() {
    let mut doc = Document::with_options(
        128,
        128,
        DocumentOptions {
            max_document_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let wide = StrokeOutline::new(
        true,
        OutlinePlacement::Outside,
        OutlineWidth::new(false, 40., 10.).unwrap(),
        OutlineSource::SelfBrush,
        true,
    )
    .unwrap();
    assert!(doc
        .begin_outlined_brush_stroke(brush(), InputPoint::new(64., 64., 1.), &wide)
        .is_err());
    assert_eq!(doc.history_depth(), (0, 0));
    assert!((0..128).all(|y| (0..128).all(|x| doc.pixel(x, y) == Pixel::TRANSPARENT)));
    doc.begin_brush_stroke(brush(), InputPoint::new(20., 20., 1.))
        .unwrap();
    assert!(doc.end_stroke().unwrap());
}

#[test]
fn off_canvas_replay_does_not_outline_viewport_boundary() {
    let mut definition = brush();
    definition.brush.radius = 12.;
    let mut doc = Document::new(96, 64).unwrap();
    doc.begin_outlined_brush_stroke(
        definition.clone(),
        InputPoint::new(-4., 32., 1.),
        &outline(OutlinePlacement::Inside, OutlineSource::SelfBrush),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    assert_eq!(doc.pixel(0, 32), definition.brush.color);
    assert!(doc.pixel(7, 32).components()[2] > 0.);
}
