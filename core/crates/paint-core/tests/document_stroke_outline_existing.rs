use paint_core::{
    Brush, BrushDefinition, BrushDynamics, BrushMask, BrushTip, Document, InputPoint,
    InputScheduleBudget, OutlinePlacement, OutlineSource, OutlineWidth, Pixel, ScatterDynamics,
    SecondaryTip, StrokeOutline, StrokePageResult,
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

fn finish(doc: &mut Document, paged: bool) {
    if !paged {
        assert!(doc.end_stroke().unwrap());
        return;
    }
    doc.begin_stroke_finish().unwrap();
    loop {
        match doc
            .continue_stroke_page(InputScheduleBudget::new(3, 100_000, 3).unwrap())
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
fn existing_brush_replays_primary_actions_and_keeps_image_holes() {
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
        let dynamics = BrushDynamics {
            scatter: ScatterDynamics::new(scattered, 0.4, true, 2, 0.2, 31).unwrap(),
            ..Default::default()
        };
        let tip = SecondaryTip::new(
            12.,
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
        let settings = StrokeOutline::new(
            true,
            OutlinePlacement::Outside,
            OutlineWidth::new(false, 2., 10.).unwrap(),
            OutlineSource::Existing(Arc::new(tip)),
            true,
        )
        .unwrap();
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
            let output: Vec<_> = (0..64 * 64)
                .flat_map(|i| doc.pixel(i % 64, i / 64).components().map(f32::to_bits))
                .collect();
            assert!(output.iter().any(|pixel| f32::from_bits(*pixel) > 0.2));
            assert_eq!(doc.pixel(32, 32), Pixel::TRANSPARENT);
            assert!((0..64 * 64).any(|i| doc.pixel(i % 64, i / 64).components()[2] > 0.2));
            if paged {
                assert_eq!(*results.last().unwrap(), output);
            } else {
                results.push(output);
            }
        }
    }
    assert_eq!(
        results[0], results[1],
        "source spacing/scatter cannot schedule independent actions"
    );
}
