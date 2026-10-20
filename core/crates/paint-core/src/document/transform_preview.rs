//! CPU geometry only; colour sampling remains in the display or final materialisation.
use super::*;

impl Document {
    pub fn begin_transform_layers_deferred(&mut self, ids: &[u64]) -> Result<()> {
        self.begin_transform_layers(ids)?;
        self.enable_transform_deferred()
    }

    /// Also usable by a placement session already opened by the file worker.
    pub fn enable_transform_deferred(&mut self) -> Result<()> {
        let state = self
            .transform
            .as_ref()
            .ok_or(Error::InvalidArgument("no transform session"))?;
        if state.preview_source.is_some() {
            return Ok(());
        }
        if state.selection.is_some() || !state.extra.is_empty() || state.original.mask {
            return Ok(());
        }
        // Placed images use their immutable original pixels, including repeated scaling/warping.
        let mut layer = state.source.source.clone();
        layer.properties = state.original.properties;
        layer.appearance = state.original.appearance;
        layer.appearance.offset_x = 0;
        layer.appearance.offset_y = 0;
        layer.placed = None;
        let source = self.resident_source_layer(state.original.id, layer);
        self.transform.as_mut().expect("session").preview_source = source;
        self.dirty_all();
        Ok(())
    }

    pub fn transform_deferred(&self) -> bool {
        self.transform
            .as_ref()
            .is_some_and(|s| s.preview_source.is_some())
    }

    pub fn transform_preview(&self) -> Option<crate::ResidentPreview> {
        let state = self.transform.as_ref()?;
        let source = state.preview_source.as_ref()?.clone();
        let geometry = &state.current;
        let mesh = super::mesh::TransformMesh::new(geometry);
        let bounds = state.source.bounds;
        let vertices = mesh
            .vertices
            .into_iter()
            .map(|p| {
                [
                    p[0],
                    p[1],
                    ((bounds[0] + p[2] as f64 * bounds[2] - source.bounds[0] as f64)
                        / source.bounds[2] as f64) as f32,
                    ((bounds[1] + p[3] as f64 * bounds[3] - source.bounds[1] as f64)
                        / source.bounds[3] as f64) as f32,
                ]
            })
            .collect();
        Some(crate::ResidentPreview {
            source,
            revision: self.revision,
            focus: false,
            forward: true,
            columns: mesh.columns,
            rows: mesh.rows,
            vertices,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seed() -> Document {
        let mut d = Document::new(128, 96).unwrap();
        d.bucket_fill(crate::BucketFill {
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
        for (x, y, color) in [
            (32., 32., [1., 0., 0., 0.8]),
            (80., 60., [0., 0.2, 1., 0.6]),
        ] {
            d.begin_stroke(
                crate::Brush {
                    radius: 9.,
                    color: Pixel::from_straight(color).unwrap(),
                    ..Default::default()
                },
                crate::InputPoint::new(x, y, 1.),
            )
            .unwrap();
            d.end_stroke().unwrap();
        }
        d
    }
    #[test]
    fn deferred_transform_keeps_tiles_and_matches_final_pixels_and_history() {
        for warp in [false, true] {
            let mut a = seed();
            let mut b = seed();
            let id = a.active_layer();
            let original = a.layers[0].clone();
            let depth = a.history_depth();
            a.begin_transform_layers_deferred(&[id]).unwrap();
            b.begin_transform_layers(&[id]).unwrap();
            let source = a.transform_preview().unwrap().source;
            let mut g = a.transform_geometry().unwrap();
            g.warp = warp;
            for p in &mut g.points {
                p[0] += 12.;
                p[1] -= 7.;
            }
            if warp {
                g.points[5][0] += 15.;
                g.points[10][1] -= 9.;
            }
            a.update_transform(g.clone()).unwrap();
            b.update_transform(g.clone()).unwrap();
            assert_eq!(a.history_depth(), depth);
            for (key, tile) in &original.tiles {
                assert!(Arc::ptr_eq(tile, &a.layers[0].tiles[key]));
            }
            let p = a.transform_preview().unwrap();
            assert!(p.forward);
            assert!(Arc::ptr_eq(&source, &p.source));
            assert!(p.vertices.len() <= 145 * 145);
            let node =
                (p.rows as usize - 1) / 3 * p.columns as usize + (p.columns as usize - 1) / 3;
            assert!((p.vertices[node][0] as f64 - g.points[5][0]).abs() < 0.001);
            a.checkpoint_transform().unwrap();
            a.undo_transform().unwrap();
            a.redo_transform().unwrap();
            a.commit_transform().unwrap();
            b.commit_transform().unwrap();
            assert_eq!(a.history_depth().0, depth.0 + 1);
            for y in 0..96 {
                for x in 0..128 {
                    assert_eq!(a.pixel(x, y), b.pixel(x, y));
                }
            }
            a.undo().unwrap();
            assert_eq!(a.pixel(0, 0), Pixel::WHITE);
            a.redo().unwrap();
            assert_eq!(a.pixel(0, 0), b.pixel(0, 0));
        }
    }
    #[test]
    fn deferred_transform_cancel_placed_source_and_selection_fallback() {
        let mut d = seed();
        let id = d.active_layer();
        d.begin_transform_layers_deferred(&[id]).unwrap();
        let mut g = d.transform_geometry().unwrap();
        for p in &mut g.points {
            p[0] -= 200.;
        }
        d.update_transform(g).unwrap();
        d.cancel_transform().unwrap();
        assert_eq!(d.pixel(0, 0), Pixel::WHITE);
        let pixels = d.layers[0].clone();
        let id = d.place_image(pixels, 20, 10, "placed".into()).unwrap();
        d.enable_transform_deferred().unwrap();
        assert!(d.transform_deferred());
        d.commit_transform().unwrap();
        let model = d.layer(id).unwrap().placed.as_ref().unwrap().clone();
        d.begin_transform_layers_deferred(&[id]).unwrap();
        assert!(d.transform_deferred());
        let source = d.transform_preview().unwrap().source;
        assert!(source.image.layers[0].placed.is_none());
        assert!(Arc::ptr_eq(
            model.source.tiles.values().next().unwrap(),
            source.image.layers[0].tiles.values().next().unwrap()
        ));
        d.cancel_transform().unwrap();
        d.set_active_layer(d.layers[0].id).unwrap();
        d.set_selection(crate::Selection::all(128, 96)).unwrap();
        d.begin_transform_layers_deferred(&[d.active_layer()])
            .unwrap();
        assert!(!d.transform_deferred());
        d.cancel_transform().unwrap();
    }

    #[test]
    fn deferred_transform_failed_materialisation_keeps_session_pixels_and_history() {
        let mut d = seed();
        let id = d.active_layer();
        let depth = d.history_depth();
        let before = d.layers[0].clone();
        d.begin_transform_layers_deferred(&[id]).unwrap();
        let initial = d.transform_geometry().unwrap();
        let source = d.transform_preview().unwrap().source;
        d.update_transform(TransformGeometry::rectangle(0., 0., 5000., 5000.))
            .unwrap();
        assert!(d.commit_transform().is_err());
        assert!(d.transform_deferred());
        assert_eq!(d.history_depth(), depth);
        assert!(Arc::ptr_eq(&source, &d.transform_preview().unwrap().source));
        for (key, tile) in before.tiles {
            assert!(Arc::ptr_eq(&tile, &d.layers[0].tiles[&key]));
        }
        d.update_transform(initial).unwrap();
        d.cancel_transform().unwrap();
    }
    #[test]
    fn folded_warp_matches_independent_forward_triangles_and_history() {
        let mut d = seed();
        let original = d.layers[0].clone();
        let before = d.history_depth();
        d.begin_transform_layers_deferred(&[d.active_layer()])
            .unwrap();
        let mut g = d.transform_geometry().unwrap();
        g.warp = true;
        g.interpolation = 0;
        // Reproduce the screenshot's crossed rows, reversed cells and off-canvas lower edge.
        g.points = [
            [35., 6.],
            [78., 15.],
            [85., 5.],
            [103., 9.],
            [35., 46.],
            [54., 21.],
            [96., 39.],
            [126., 35.],
            [15., 43.],
            [59., 68.],
            [89., 69.],
            [66., 39.],
            [16., 71.],
            [39., 97.],
            [82., 99.],
            [125., 68.],
        ];
        d.update_transform(g).unwrap();
        let preview = d.transform_preview().unwrap();
        let mut expected = vec![[f64::NAN; 2]; 128 * 96];
        let mut hits = vec![0u32; 128 * 96];
        // Independent reference: brute-force barycentric ownership of published triangles.
        for row in 0..preview.rows - 1 {
            for col in 0..preview.columns - 1 {
                let a = (row * preview.columns + col) as usize;
                let c = a + preview.columns as usize;
                for ids in [[a, a + 1, c + 1], [a, c + 1, c]] {
                    let [a, b, c] = ids.map(|i| preview.vertices[i].map(f64::from));
                    let det = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
                    if det.abs() < 1e-12 {
                        continue;
                    }
                    for y in 0..96 {
                        for x in 0..128 {
                            let px = x as f64 + 0.5;
                            let py = y as f64 + 0.5;
                            let l0 =
                                ((b[1] - c[1]) * (px - c[0]) + (c[0] - b[0]) * (py - c[1])) / det;
                            let l1 =
                                ((c[1] - a[1]) * (px - c[0]) + (a[0] - c[0]) * (py - c[1])) / det;
                            let l2 = 1. - l0 - l1;
                            if l0 >= 0. && l1 >= 0. && l2 >= 0. {
                                let k = y * 128 + x;
                                hits[k] += 1;
                                expected[k] = [
                                    l0 * a[2] + l1 * b[2] + l2 * c[2],
                                    l0 * a[3] + l1 * b[3] + l2 * c[3],
                                ];
                            }
                        }
                    }
                }
            }
        }
        assert!(
            hits.iter().filter(|&&n| n > 1).count() > 100,
            "fixture must actually fold"
        );
        d.commit_transform().unwrap();
        let mut sampler = SourceSampler::new(&original);
        for y in 0..96 {
            for x in 0..128 {
                let uv = expected[y * 128 + x];
                let expected = if uv[0].is_finite() {
                    sample(
                        &mut sampler,
                        preview.source.bounds[0] as f64 + uv[0] * preview.source.bounds[2] as f64
                            - 0.5,
                        preview.source.bounds[1] as f64 + uv[1] * preview.source.bounds[3] as f64
                            - 0.5,
                        0,
                        1.,
                    )
                    .unwrap()
                } else {
                    Pixel::TRANSPARENT
                };
                assert_eq!(
                    d.pixel(x as u32, y as u32),
                    expected,
                    "folded pixel {x},{y}"
                );
            }
        }
        assert_eq!(d.history_depth().0, before.0 + 1);
        d.undo().unwrap();
        assert_eq!(d.pixel(0, 0), Pixel::WHITE);
        d.redo().unwrap();
        assert!(d.pixel(0, 0).alpha() < 1.);
    }
    #[test]
    fn selected_bounded_materialisation_matches_full_reference_with_all_interpolations() {
        for mode in 0..3 {
            for kind in 0..6 {
                for warp in [false, true] {
                    let mut d = Document::new(128, 96).unwrap();
                    if mode != 1 {
                        d.initialize_white_background().unwrap();
                    }
                    let pixels = (0..4096)
                        .map(|i| {
                            Pixel::from_straight([
                                (i % 17) as f32 / 17.,
                                0.2,
                                0.7,
                                if i % 7 == 0 { 0. } else { 0.65 },
                            ])
                            .unwrap()
                        })
                        .collect();
                    d.layers[0].tiles.insert(
                        TileCoord::from_signed(0, 0),
                        Arc::new(Tile::from_pixels(pixels).unwrap()),
                    );
                    let untouched = Arc::new(Tile::from_pixels(vec![Pixel::WHITE; 4096]).unwrap());
                    d.layers[0]
                        .tiles
                        .insert(TileCoord::from_signed(1, 0), untouched.clone());
                    if mode == 2 {
                        d.move_layer(1, 3, -2).unwrap();
                    }
                    let shape = crate::SelectionShape::polygon(
                        vec![[17., 13.], [43., 16.], [39., 37.], [14., 34.]],
                        true,
                        if mode == 0 { 2. } else { 0. },
                    )
                    .unwrap();
                    d.set_selection(
                        crate::Selection::default()
                            .apply(shape, crate::SelectionOperation::Replace, 128, 96)
                            .unwrap(),
                    )
                    .unwrap();
                    let before = d.snapshot();
                    let depth = d.history_depth();
                    d.begin_transform_layers(&[1]).unwrap();
                    let initial = d.transform_geometry().unwrap();
                    let mut g = initial.clone();
                    g.interpolation = kind;
                    g.warp = warp;
                    for p in &mut g.points {
                        p[0] = p[0] * 0.91 - 12.25;
                        p[1] = p[1] * 1.13 + 2.5;
                    }
                    if warp {
                        g.points[5][0] += 11.;
                        g.points[10][1] -= 10.;
                    }
                    let state = d.transform.as_ref().unwrap();
                    let reference = d
                        .render_transform_target_impl(&state.original, &state.source, &g, false)
                        .unwrap();
                    let reference_selection = d
                        .transformed_selection_impl(
                            state.selection.as_ref().unwrap(),
                            &initial,
                            &g,
                            false,
                        )
                        .unwrap();
                    d.update_transform(g).unwrap();
                    let actual = d.layer(1).unwrap();
                    for y in -16..112 {
                        let a = actual
                            .read_local_row(
                                -20 - actual.appearance.offset_x as i64,
                                y - actual.appearance.offset_y as i64,
                                176,
                            )
                            .unwrap();
                        let b = reference
                            .read_local_row(
                                -20 - reference.appearance.offset_x as i64,
                                y - reference.appearance.offset_y as i64,
                                176,
                            )
                            .unwrap();
                        for (x, (a, b)) in a.iter().zip(&b).enumerate() {
                            for (a, b) in a.components().into_iter().zip(b.components()) {
                                assert!((a-b).abs()<2e-6,"mode={mode} interpolation={kind} warp={warp} x={x} y={y}: {a}!={b}");
                            }
                        }
                    }
                    for y in 0..96 {
                        for x in 0..128 {
                            assert_eq!(
                                d.selection().coverage(x as f64 + 0.5, y as f64 + 0.5),
                                reference_selection.coverage(x as f64 + 0.5, y as f64 + 0.5)
                            );
                        }
                    }
                    if mode != 2 {
                        assert!(Arc::ptr_eq(
                            &untouched,
                            d.layers[0]
                                .tiles
                                .get(&TileCoord::from_signed(1, 0))
                                .unwrap()
                        ));
                    }
                    d.commit_transform().unwrap();
                    assert_eq!(d.history_depth().0, depth.0 + 1);
                    d.undo().unwrap();
                    for y in 0..96 {
                        for x in 0..128 {
                            assert_eq!(d.pixel(x, y), before.pixel(x, y));
                        }
                    }
                    d.redo().unwrap();
                }
            }
        }
    }
    #[test]
    fn cropped_selected_transform_empty_and_far_edges_keep_coverage() {
        let mut d = seed();
        let selection = crate::Selection::all(128, 96);
        let initial = TransformGeometry::rectangle(0., 0., 128., 96.);
        for (dx, dy) in [(-200., 0.), (0., 200.), (-10.25, -5.5), (120., 80.)] {
            for warp in [false, true] {
                let mut g = initial.clone();
                g.warp = warp;
                for p in &mut g.points {
                    p[0] += dx;
                    p[1] += dy;
                }
                if warp {
                    g.points[5][0] -= 25.;
                    g.points[10][1] += 20.;
                }
                let a = d
                    .transformed_selection_impl(&selection, &initial, &g, true)
                    .unwrap();
                let b = d
                    .transformed_selection_impl(&selection, &initial, &g, false)
                    .unwrap();
                for y in 0..96 {
                    for x in 0..128 {
                        assert_eq!(
                            a.coverage(x as f64 + 0.5, y as f64 + 0.5),
                            b.coverage(x as f64 + 0.5, y as f64 + 0.5)
                        );
                    }
                }
            }
        }
        d.begin_transform_layers(&[1]).unwrap();
        d.cancel_transform().unwrap();
    }
}
