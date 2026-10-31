use super::*;

#[test]
fn large_canvas_small_lasso_transforms_white_and_content_without_canvas_mask() {
    for content in [false, true] {
        for warp in [false, true] {
            let mut d = Document::new(8888, 5000).unwrap();
            d.initialize_white_background().unwrap();
            if content {
                d.add_layer("content").unwrap();
                d.begin_stroke(
                    Brush {
                        radius: 8.,
                        ..Default::default()
                    },
                    InputPoint::new(70., 70., 1.),
                )
                .unwrap();
                d.end_stroke().unwrap();
            }
            let shape = crate::SelectionShape::polygon(
                vec![[40., 40.], [120., 45.], [110., 110.], [45., 105.]],
                true,
                4.,
            )
            .unwrap();
            let selection = crate::Selection::default()
                .apply(shape, crate::SelectionOperation::Replace, 8888, 5000)
                .unwrap();
            d.set_selection(selection.clone()).unwrap();
            let before = d.pixel(70, 70);
            let depth = d.history_depth();
            d.begin_transform_layers_deferred(&[d.active_layer()])
                .unwrap();
            assert!(d.transform_deferred());
            let source = d.transform_preparation_source().unwrap();
            let mut g = d.transform_geometry().unwrap();
            for p in &mut g.points {
                p[0] += 160.;
            }
            g.warp = warp;
            if warp {
                g.points[5][1] += 4.;
            }
            for _ in 0..20 {
                d.update_transform(g.clone()).unwrap();
            }
            assert!(Arc::ptr_eq(
                &source,
                &d.transform_preparation_source().unwrap()
            ));
            assert_eq!(d.history_depth(), depth);
            assert_eq!(d.pixel(70, 70), before);
            d.commit_transform().unwrap();
            assert_eq!(d.history_depth().0, depth.0 + 1);
            assert_eq!(
                d.pixel(70, 70),
                if content {
                    Pixel::WHITE
                } else {
                    Pixel::TRANSPARENT
                }
            );
            d.undo().unwrap();
            assert_eq!(d.selection, selection);
            assert_eq!(d.pixel(70, 70), before);
            d.redo().unwrap();
            assert_eq!(d.selection.coverage(70.5, 70.5), 0.);
            assert!(d.selection.coverage(230.5, 70.5) > 0.9);
        }
    }
}

#[test]
fn large_white_transform_materialises_above_sixteen_megapixels_and_undoes() {
    for selected in [false, true] {
        let mut d = Document::new(8888, 5000).unwrap();
        d.initialize_white_background().unwrap();
        if selected {
            d.set_selection(crate::Selection::all(8888, 5000)).unwrap();
        }
        let depth = d.history_depth();
        d.begin_transform_layers_deferred(&[d.active_layer()])
            .unwrap();
        assert!(d.transform_deferred());
        let mut g = d.transform_geometry().unwrap();
        for p in &mut g.points {
            p[0] += 64.;
            p[1] += 64.;
        }
        g.interpolation = 0;
        d.update_transform(g).unwrap();
        d.commit_transform().unwrap();
        assert_eq!(d.history_depth().0, depth.0 + 1);
        assert_eq!(d.pixel(0, 0), Pixel::TRANSPARENT);
        assert_eq!(d.pixel(64, 64), Pixel::WHITE);
        assert_eq!(d.pixel(8887, 4999), Pixel::WHITE);
        d.undo().unwrap();
        assert_eq!(d.pixel(0, 0), Pixel::WHITE);
        d.redo().unwrap();
        assert_eq!(d.pixel(0, 0), Pixel::TRANSPARENT);
    }
}

#[test]
fn large_transformed_selection_pages_preserve_seams_codec_and_budget() {
    let mut d = Document::new(8888, 5000).unwrap();
    let selection = crate::Selection::all(8888, 5000);
    let initial = TransformGeometry::rectangle(0., 0., 8888., 5000.);
    let mut moved = initial.clone();
    for p in &mut moved.points {
        p[0] += 2.;
    }
    let result = d
        .transformed_selection(&selection, &initial, &moved)
        .unwrap();
    assert!(result.steps().len() > 1);
    for step in result.steps() {
        let shape = step.shape.as_ref().unwrap();
        let y = shape.y as u32;
        for row in [y.saturating_sub(1), y, (y + 1).min(4999)] {
            assert_eq!(result.coverage(500.5, row as f64 + 0.5), 1.);
        }
        assert!(shape.mask.as_ref().unwrap().bytes() <= crate::MAX_SELECTION_MASK_BYTES);
    }
    assert_eq!(result.coverage(0.5, 100.), 0.);
    assert_eq!(
        crate::Selection::from_text(&result.to_text()).unwrap(),
        result
    );
    d.options.max_resident_tiles = 64;
    assert!(matches!(
        d.transformed_selection(&selection, &initial, &moved),
        Err(Error::ResourceLimit(_))
    ));
    assert!(!d.selection.enabled());
}

#[test]
fn large_sparse_selection_preview_budgets_pixels_instead_of_transparent_area() {
    let mut d = Document::new(8888, 5000).unwrap();
    d.begin_stroke(
        Brush {
            radius: 4.,
            ..Default::default()
        },
        InputPoint::new(100., 100., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    assert_eq!(d.tile_count(), 1);
    d.options.max_document_tiles = 2;
    d.set_selection(crate::Selection::all(8888, 5000)).unwrap();
    let before = d.snapshot();
    let depth = d.history_depth();
    d.begin_transform_layers_deferred(&[d.active_layer()])
        .unwrap();
    assert!(
        d.transform_deferred(),
        "only two actual split tiles fit the budget"
    );
    let source = d.transform_preparation_source().unwrap();
    assert_eq!(
        source
            .image
            .layers
            .iter()
            .map(|l| l.tiles.len())
            .sum::<usize>(),
        1
    );
    assert_eq!(
        source
            .below
            .layers
            .iter()
            .map(|l| l.tiles.len())
            .sum::<usize>(),
        1
    );
    for (x, y) in [(0, 0), (100, 100), (127, 127), (8887, 4999)] {
        assert_eq!(source.image.pixel(x, y), before.pixel(x, y));
        assert_eq!(source.below.pixel(x, y), Pixel::TRANSPARENT);
    }
    let mut g = d.transform_geometry().unwrap();
    for step in 1..=20 {
        for p in &mut g.points {
            p[0] += 1.;
        }
        d.update_transform(g.clone()).unwrap();
        assert!(
            Arc::ptr_eq(&source, &d.transform_preparation_source().unwrap()),
            "update {step}"
        );
    }
    d.cancel_transform().unwrap();
    assert_eq!(d.history_depth(), depth);
    assert_eq!(d.pixel(100, 100), before.pixel(100, 100));
}
