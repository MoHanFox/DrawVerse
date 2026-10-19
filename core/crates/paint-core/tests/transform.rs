use paint_core::*;
fn source() -> Document {
    let mut d = Document::new(12, 12).unwrap();
    d.begin_stroke(
        Brush {
            radius: 4.,
            color: Pixel::from_straight([0.8, 0.2, 0.1, 1.]).unwrap(),
            ..Default::default()
        },
        InputPoint::new(6., 6., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    d
}
fn pixels(d: &Document) -> Vec<Pixel> {
    (0..24)
        .flat_map(|y| (0..24).map(move |x| d.pixel(x, y)))
        .collect()
}
fn scale(g: &TransformGeometry, s: f64) -> TransformGeometry {
    let mut g = g.clone();
    for p in &mut g.points {
        p[0] = 12. + (p[0] - 12.) * s;
        p[1] = 12. + (p[1] - 12.) * s;
    }
    g.interpolation = 0;
    g
}
#[test]
fn image_source_survives_transform_rasterize_clipboard_and_embedded_model() {
    let src = source();
    let mut d = Document::new(24, 24).unwrap();
    let id = d
        .place_image(src.layers()[0].clone(), 12, 12, "原图".into())
        .unwrap();
    assert!(d.transform_active());
    assert_eq!(d.history_depth(), (0, 0));
    d.commit_transform().unwrap();
    assert!(d.layer(id).unwrap().is_image());
    let original = pixels(&d);
    d.begin_transform().unwrap();
    let g = d.transform_geometry().unwrap();
    d.update_transform(scale(&g, 0.25)).unwrap();
    d.checkpoint_transform().unwrap();
    assert_eq!(d.transform_history_depth(), Some((1, 0)));
    assert_ne!(pixels(&d), original);
    d.undo_transform().unwrap();
    assert_eq!(pixels(&d), original);
    d.redo_transform().unwrap();
    d.commit_transform().unwrap();
    assert_eq!(d.history_depth().0, 2);
    d.begin_transform().unwrap();
    let g = d.transform_geometry().unwrap();
    d.update_transform(scale(&g, 4.)).unwrap();
    d.commit_transform().unwrap();
    assert_eq!(pixels(&d), original);
    let bytes = d.embedded_layers().unwrap();
    let mut loaded = Document::from_embedded_layers(
        24,
        24,
        &bytes,
        id,
        DocumentOptions::default(),
        paint_storage::ScratchSpace::system(),
    )
    .unwrap();
    assert_eq!(pixels(&loaded), original);
    assert!(loaded.layer(id).unwrap().is_image());
    loaded.rasterize_image(id).unwrap();
    assert!(!loaded.layer(id).unwrap().is_image());
    loaded.undo().unwrap();
    assert!(loaded.layer(id).unwrap().is_image());
    loaded.redo().unwrap();
    assert!(!loaded.layer(id).unwrap().is_image());
    let copy = d.copy_layers(&[id]).unwrap();
    let pasted = d.paste_layers(&copy, id).unwrap();
    assert!(d.layer(pasted).unwrap().is_image());
    d.undo().unwrap();
    assert!(d.layer(pasted).is_none());
}
#[test]
fn placement_cancel_preserves_redo_and_failed_transform_is_atomic() {
    let mut d = source();
    d.undo().unwrap();
    let history = d.history_depth();
    let src = source();
    d.place_image(src.layers()[0].clone(), 12, 12, "临时".into())
        .unwrap();
    d.cancel_transform().unwrap();
    assert_eq!(d.history_depth(), history);
    assert_eq!(d.layers().len(), 1);
    d.redo().unwrap();
    let original = pixels(&d);
    d.begin_transform().unwrap();
    let mut g = d.transform_geometry().unwrap();
    g.points[0][0] = f64::NAN;
    assert!(d.update_transform(g).is_err());
    assert_eq!(pixels(&d), original);
    d.cancel_transform().unwrap();
}
#[test]
fn quad_warp_interpolation_and_history_limit() {
    let mut d = source();
    d.set_max_history_commands(2).unwrap();
    d.begin_transform().unwrap();
    let mut g = d.transform_geometry().unwrap();
    g.points[3][0] += 2.;
    g.points[15][0] += 2.;
    d.update_transform(g.clone()).unwrap();
    d.checkpoint_transform().unwrap();
    g.warp = true;
    g.points[5][0] += 1.;
    g.points[5][1] -= 1.;
    d.update_transform(g.clone()).unwrap();
    d.checkpoint_transform().unwrap();
    let preview = pixels(&d);
    for mode in 0..6 {
        g.interpolation = mode;
        d.update_transform(g.clone()).unwrap();
        d.checkpoint_transform().unwrap();
    }
    assert_eq!(d.transform_history_depth(), Some((2, 0)));
    d.undo_transform().unwrap();
    d.redo_transform().unwrap();
    assert_ne!(pixels(&d), preview);
    d.cancel_transform().unwrap();
}

#[test]
fn warp_switch_is_identity_and_spilled_image_history_is_lossless() {
    let mut g = TransformGeometry::rectangle(3., 5., 13., 17.);
    g.warp = true;
    for y in 0..=19 {
        for x in 0..=19 {
            let u = x as f64 / 19.;
            let v = y as f64 / 19.;
            let p = g.map(u, v);
            assert!((p[0] - (3. + u * 13.)).abs() < 1e-10);
            assert!((p[1] - (5. + v * 17.)).abs() < 1e-10);
        }
    }
    let mut d = Document::with_options(
        24,
        24,
        DocumentOptions {
            max_history_bytes: 2 * TILE_BYTES + 2048,
            ..Default::default()
        },
    )
    .unwrap();
    let id = d
        .place_image(source().layers()[0].clone(), 12, 12, "image".into())
        .unwrap();
    d.commit_transform().unwrap();
    let original = pixels(&d);
    d.begin_transform().unwrap();
    let mut g = d.transform_geometry().unwrap();
    for p in &mut g.points {
        p[0] += 3.;
    }
    d.update_transform(g).unwrap();
    d.commit_transform().unwrap();
    d.rasterize_image(id).unwrap();
    d.undo().unwrap();
    assert!(d.layer(id).unwrap().is_image());
    d.undo().unwrap();
    assert_eq!(pixels(&d), original);
    d.redo().unwrap();
    d.redo().unwrap();
    assert!(!d.layer(id).unwrap().is_image());
}

#[test]
fn group_and_overlapping_multi_selection_transform_atomically() {
    let mut d = source();
    let a = d.active_layer();
    let image = d
        .place_image(source().layers()[0].clone(), 12, 12, "image".into())
        .unwrap();
    d.commit_transform().unwrap();
    let group = d.group_layers(&[a, image], "group").unwrap();
    let before = pixels(&d);
    let depth = d.history_depth();
    d.begin_transform_layers(&[group, image]).unwrap();
    let mut g = d.transform_geometry().unwrap();
    for p in &mut g.points {
        p[0] += 3.;
    }
    d.update_transform(g).unwrap();
    d.checkpoint_transform().unwrap();
    let moved = pixels(&d);
    assert_ne!(before, moved);
    assert!(d.layer(image).unwrap().is_image());
    d.undo_transform().unwrap();
    assert_eq!(pixels(&d), before);
    d.redo_transform().unwrap();
    assert_eq!(pixels(&d), moved);
    d.commit_transform().unwrap();
    assert_eq!(d.history_depth().0, depth.0 + 1);
    d.undo().unwrap();
    assert_eq!(pixels(&d), before);
    d.redo().unwrap();
    assert_eq!(pixels(&d), moved);
    assert!(d.layer(group).unwrap().is_group());
    d.begin_transform_layers(&[group]).unwrap();
    let original = d.transform_geometry().unwrap();
    let mut g = original.clone();
    g.interpolation = 0;
    d.update_transform(g).unwrap();
    d.checkpoint_transform().unwrap();
    d.undo_transform().unwrap();
    assert_eq!(pixels(&d), moved);
    assert!(!d.commit_transform().unwrap());
    assert_eq!(pixels(&d), moved);
    d.begin_transform().unwrap();
    let mut g = d.transform_geometry().unwrap();
    for p in &mut g.points {
        p[1] += 4.;
    }
    d.update_transform(g).unwrap();
    d.cancel_transform().unwrap();
    assert_eq!(pixels(&d), moved);
}

#[test]
fn selected_pixels_move_without_touching_the_rest_and_history_restores_selection() {
    let mut d = source();
    let selection = Selection::default()
        .apply(
            SelectionShape::geometry(SelectionKind::Rectangle, 6., 0., 6., 12., true),
            SelectionOperation::Replace,
            12,
            12,
        )
        .unwrap();
    d.set_selection(selection.clone()).unwrap();
    let before = pixels(&d);
    let unselected = d.pixel(4, 6);
    let selected = d.pixel(8, 6);
    d.begin_transform_layers(&[d.active_layer()]).unwrap();
    let mut g = d.transform_geometry().unwrap();
    for p in &mut g.points {
        p[0] += 3.;
    }
    d.update_transform(g).unwrap();
    d.checkpoint_transform().unwrap();
    assert_eq!(d.pixel(4, 6), unselected);
    assert_eq!(d.pixel(8, 6), Pixel::TRANSPARENT);
    assert_eq!(d.pixel(11, 6), selected);
    assert_eq!(d.selection().coverage(8.5, 6.5), 0.);
    assert_eq!(d.selection().coverage(11.5, 6.5), 1.);
    d.undo_transform().unwrap();
    assert_eq!(pixels(&d), before);
    assert_eq!(d.selection().to_text(), selection.to_text());
    d.redo_transform().unwrap();
    d.commit_transform().unwrap();
    d.undo().unwrap();
    assert_eq!(pixels(&d), before);
    assert_eq!(d.selection().to_text(), selection.to_text());
    d.redo().unwrap();
    assert_eq!(d.pixel(11, 6), selected);
    d.begin_transform().unwrap();
    let mut g = d.transform_geometry().unwrap();
    for p in &mut g.points {
        p[1] += 2.;
    }
    d.update_transform(g).unwrap();
    d.cancel_transform().unwrap();
    assert_eq!(d.pixel(11, 6), selected);
}

#[test]
fn folded_selection_and_content_share_the_same_surface_coverage() {
    let mut d = Document::new(96, 96).unwrap();
    d.bucket_fill(BucketFill {
        x: 0,
        y: 0,
        color: Pixel::WHITE,
        opacity: 1.,
        tolerance: 0,
        antialias: false,
        contiguous: true,
        all_layers: false,
    })
    .unwrap();
    d.set_selection(Selection::all(96, 96)).unwrap();
    d.begin_transform_layers_deferred(&[d.active_layer()])
        .unwrap();
    assert!(!d.transform_deferred());
    let mut g = d.transform_geometry().unwrap();
    g.warp = true;
    g.interpolation = 0;
    g.points = [
        [26., 6.],
        [59., 15.],
        [64., 5.],
        [77., 9.],
        [26., 46.],
        [41., 21.],
        [72., 39.],
        [94., 35.],
        [11., 43.],
        [44., 68.],
        [67., 69.],
        [49., 39.],
        [12., 71.],
        [29., 97.],
        [61., 99.],
        [94., 68.],
    ];
    d.update_transform(g).unwrap();
    let mut covered = 0;
    for y in 0..96 {
        for x in 0..96 {
            let alpha = d.pixel(x, y).alpha();
            assert_eq!(
                alpha,
                d.selection().coverage(x as f64 + 0.5, y as f64 + 0.5),
                "{x},{y}"
            );
            covered += usize::from(alpha > 0.);
        }
    }
    assert!(covered > 1000 && covered < 8000);
    d.commit_transform().unwrap();
    d.undo().unwrap();
    assert_eq!(d.pixel(0, 0), Pixel::WHITE);
    assert_eq!(d.selection().coverage(0.5, 0.5), 1.);
}
