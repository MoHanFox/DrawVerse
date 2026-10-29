//! CPU geometry only; colour sampling remains in the display or final materialisation.
use super::*;

impl Document {
    /// Immutable presentation assets needed by the first actual drag. The
    /// identity preview may use another source to preserve feathered seams.
    pub fn transform_preparation_source(&self) -> Option<Arc<crate::ResidentPreviewSource>> {
        self.transform.as_ref()?.preview_source.clone()
    }

    /// Stable moving/identity sources for this selected transform. Geometry
    /// updates can switch the active source without invalidating either image.
    pub fn transform_preview_sources(&self) -> Option<[Arc<crate::ResidentPreviewSource>; 2]> {
        let state = self.transform.as_ref()?;
        Some([
            state.preview_source.clone()?,
            state.preview_identity.clone()?,
        ])
    }

    pub fn begin_transform_layers_deferred(&mut self, ids: &[u64]) -> Result<()> {
        self.begin_transform_layers(ids)?;
        if let Err(error) = self.enable_transform_deferred() {
            self.cancel_transform()?;
            return Err(error);
        }
        Ok(())
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
        if state.selection.is_some() {
            if let Some((source, identity)) = self.selected_transform_preview()? {
                let state = self.transform.as_mut().expect("session");
                state.preview_bounds = Some([
                    state.initial.points[0][0],
                    state.initial.points[0][1],
                    state.initial.points[3][0] - state.initial.points[0][0],
                    state.initial.points[12][1] - state.initial.points[0][1],
                ]);
                state.preview_source = Some(source);
                state.preview_identity = Some(identity);
                self.dirty_all();
            }
            return Ok(());
        }
        let (source, bounds) = if state.extra.is_empty() && !state.original.mask {
            // Placed images retain their immutable full-resolution model.
            let mut layer = state.source.source.clone();
            layer.properties = state.original.properties;
            layer.appearance = state.original.appearance;
            layer.appearance.offset_x = 0;
            layer.appearance.offset_y = 0;
            layer.placed = None;
            (
                self.resident_source_layer(state.original.id, layer),
                state.source.bounds,
            )
        } else {
            (None, state.source.bounds)
        };
        let (source, bounds) = if source.is_some() {
            (source, bounds)
        } else {
            let originals: Vec<_> = std::iter::once(state.original.clone())
                .chain(state.extra.iter().map(|(l, _)| l.clone()))
                .collect();
            let bounds = [
                state.initial.points[0][0],
                state.initial.points[0][1],
                state.initial.points[3][0] - state.initial.points[0][0],
                state.initial.points[12][1] - state.initial.points[0][1],
            ];
            (
                self.resident_hierarchy_source(&state.roots, &originals),
                bounds,
            )
        };
        let state = self.transform.as_mut().expect("session");
        state.preview_bounds = source.as_ref().map(|_| bounds);
        state.preview_source = source;
        self.dirty_all();
        Ok(())
    }

    /// Capture the selected pixels and the cut-out remainder once. Only the
    /// moving part is sampled by the display during subsequent geometry updates.
    fn selected_transform_preview(
        &self,
    ) -> Result<
        Option<(
            Arc<crate::ResidentPreviewSource>,
            Arc<crate::ResidentPreviewSource>,
        )>,
    > {
        let state = self.transform.as_ref().expect("session");
        let original = &state.original;
        // Partial opacity must be applied after combining the moving pixels and
        // the remainder. The existing three-image protocol cannot represent that.
        if !state.extra.is_empty()
            || original.is_image()
            || original.group
            || original.mask
            || original.properties.opacity != 1.
            || original.appearance.fill != 1.
            || self.resident_has_effective_mask(original.id)
            || self
                .resident_hierarchy_source(&[original.id], std::slice::from_ref(original))
                .is_none()
        {
            return Ok(None);
        }
        let selection = state.selection.as_ref().expect("selected session");
        let origin = state.initial.points[0];
        let end = state.initial.points[15];
        let bounds = [
            origin[0] as i64,
            origin[1] as i64,
            end[0] as i64,
            end[1] as i64,
        ];
        let ox = i64::from(original.appearance.offset_x);
        let oy = i64::from(original.appearance.offset_y);
        let [x0, y0, x1, y1] = [
            bounds[0] - ox,
            bounds[1] - oy,
            bounds[2] - ox,
            bounds[3] - oy,
        ];
        let tile_count = ((x1 - 1).div_euclid(64) - x0.div_euclid(64) + 1) as usize
            * ((y1 - 1).div_euclid(64) - y0.div_euclid(64) + 1) as usize;
        if tile_count.saturating_mul(2) > self.options.max_document_tiles {
            return Ok(None);
        }
        let mut moving = original.clone();
        moving.tiles.clear();
        moving.white = (0, 0);
        let mut remaining = original.clone();
        for ty in y0.div_euclid(64)..=(y1 - 1).div_euclid(64) {
            for tx in x0.div_euclid(64)..=(x1 - 1).div_euclid(64) {
                let key = TileCoord::from_signed(tx as i32, ty as i32);
                let input = original
                    .tiles
                    .get(&key)
                    .map(|t| t.try_pixels())
                    .transpose()?;
                let mut selected = Vec::new();
                let mut rest = Vec::new();
                selected
                    .try_reserve_exact(4096)
                    .map_err(|_| Error::ResourceLimit("selection preview allocation"))?;
                rest.try_reserve_exact(4096)
                    .map_err(|_| Error::ResourceLimit("selection preview allocation"))?;
                let mut touched = false;
                let mut nonempty = false;
                for row in 0..64 {
                    let mut coverage = [0.; 64];
                    selection.coverage_span(
                        (tx * 64 + ox) as f64 + 0.5,
                        (ty * 64 + row + oy) as f64 + 0.5,
                        &mut coverage,
                    );
                    for (col, alpha) in coverage.into_iter().enumerate() {
                        let index = row as usize * 64 + col;
                        let pixel = input.as_ref().map_or_else(
                            || original.default_pixel(tx * 64 + col as i64, ty * 64 + row),
                            |v| v[index],
                        );
                        let sample = pixel.scaled(alpha);
                        nonempty |= sample != Pixel::TRANSPARENT;
                        touched |= alpha > 0.;
                        selected.push(sample);
                        rest.push(pixel.scaled(1. - alpha));
                    }
                }
                drop(input);
                if nonempty {
                    let tile = Arc::new(Tile::from_pixels(selected)?);
                    self.pool.attach(&tile)?;
                    moving.tiles.insert(key, tile);
                }
                if touched {
                    let tile = Arc::new(Tile::from_pixels(rest)?);
                    self.pool.attach(&tile)?;
                    remaining.tiles.insert(key, tile);
                }
                self.pool.trim()?;
            }
        }
        Ok(self.resident_selection_source(original, moving, remaining, bounds))
    }

    pub fn transform_deferred(&self) -> bool {
        self.transform
            .as_ref()
            .is_some_and(|s| s.preview_source.is_some())
    }

    pub fn transform_preview(&self) -> Option<crate::ResidentPreview> {
        let state = self.transform.as_ref()?;
        let source = if state.current == state.initial {
            state
                .preview_identity
                .as_ref()
                .or(state.preview_source.as_ref())?
                .clone()
        } else {
            state.preview_source.as_ref()?.clone()
        };
        let geometry = &state.current;
        let mesh = super::mesh::TransformMesh::new(geometry);
        let bounds = state.preview_bounds?;
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
    fn selected_seed(transparent: bool) -> Document {
        let mut d = seed();
        if transparent {
            d.add_layer("selected transparent pixels").unwrap();
            d.begin_stroke(
                Brush {
                    radius: 22.,
                    color: Pixel::from_straight([0.8, 0.1, 0.2, 0.6]).unwrap(),
                    ..Default::default()
                },
                InputPoint::new(48., 42., 1.),
            )
            .unwrap();
            d.end_stroke().unwrap();
            d.move_layer(d.active_layer(), 13, -3).unwrap();
        }
        d.set_selection(
            crate::Selection::default()
                .apply(
                    crate::SelectionShape::polygon(
                        vec![[22.25, 15.5], [75.75, 21.25], [69.5, 67.75], [26.5, 61.25]],
                        true,
                        1.5,
                    )
                    .unwrap(),
                    crate::SelectionOperation::Replace,
                    128,
                    96,
                )
                .unwrap(),
        )
        .unwrap();
        d
    }
    #[test]
    fn selected_resident_preview_retains_tiles_and_matches_all_final_interpolations() {
        for kind in 0..4 {
            for warp in [false, true] {
                for interpolation in 0..6 {
                    let make = || {
                        let mut d = selected_seed(kind != 0);
                        if kind >= 2 {
                            let id = d.active_layer();
                            let inner = d.group_layer(id, "selection context").unwrap();
                            let outer = d.group_layer(inner, "outer selection context").unwrap();
                            for group in [inner, outer] {
                                let at = d.layer_index(group).unwrap();
                                d.layers[at].appearance.blend = if kind == 2 {
                                    crate::BlendMode::Normal
                                } else {
                                    crate::BlendMode::PassThrough
                                };
                            }
                            d.set_active_layer(id).unwrap();
                        }
                        d
                    };
                    let mut a = make();
                    let mut b = make();
                    let original = a.snapshot();
                    let baseline = a.layers.clone();
                    let selection = a.selection.clone();
                    let depth = a.history_depth();
                    let id = a.active_layer();
                    a.begin_transform_layers_deferred(&[id]).unwrap();
                    b.begin_transform_layers(&[id]).unwrap();
                    assert!(a.transform_deferred());
                    let identity = a.transform_preview().unwrap().source;
                    for y in 0..96 {
                        for x in 0..128 {
                            assert_eq!(
                                identity.below.pixel(x, y),
                                original.pixel(x, y),
                                "exact initial feather/alpha"
                            );
                        }
                    }
                    assert!(identity.image.layers.is_empty() && identity.above.layers.is_empty());
                    let initial = a.transform_geometry().unwrap();
                    let mut source = None;
                    for pass in 1..=4 {
                        let mut g = initial.clone();
                        g.warp = warp;
                        g.interpolation = interpolation;
                        for p in &mut g.points {
                            p[0] += pass as f64 * 3.;
                            p[1] -= 2.;
                        }
                        if warp {
                            g.points[5][0] += 9.;
                            g.points[10][1] -= 7.;
                        }
                        a.update_transform(g.clone()).unwrap();
                        b.update_transform(g).unwrap();
                        let now = a.transform_preview().unwrap().source;
                        assert_ne!(now.id, identity.id);
                        if let Some(source) = &source {
                            assert!(Arc::ptr_eq(source, &now));
                        }
                        source = Some(now);
                        assert_eq!(a.history_depth(), depth);
                        for (old, new) in baseline.iter().zip(&a.layers) {
                            for (key, tile) in &old.tiles {
                                assert!(Arc::ptr_eq(tile, &new.tiles[key]));
                            }
                        }
                    }
                    a.checkpoint_transform().unwrap();
                    a.undo_transform().unwrap();
                    assert!(Arc::ptr_eq(
                        &identity,
                        &a.transform_preview().unwrap().source
                    ));
                    a.redo_transform().unwrap();
                    assert!(Arc::ptr_eq(
                        source.as_ref().unwrap(),
                        &a.transform_preview().unwrap().source
                    ));
                    a.commit_transform().unwrap();
                    b.commit_transform().unwrap();
                    assert_eq!(pixels(&a), pixels(&b));
                    assert_eq!(a.selection.steps(), b.selection.steps());
                    assert_eq!(a.history_depth().0, depth.0 + 1);
                    a.undo().unwrap();
                    assert_eq!(
                        pixels(&a),
                        (0..96)
                            .flat_map(|y| original.read_row(0, y, 128).unwrap())
                            .collect::<Vec<_>>()
                    );
                    assert_eq!(a.selection.steps(), selection.steps());
                    a.redo().unwrap();
                    assert_eq!(pixels(&a), pixels(&b));
                }
            }
        }
    }
    #[test]
    fn selected_preview_split_late_enable_cancel_and_safe_fallbacks() {
        let mut d = selected_seed(true);
        let before = pixels(&d);
        let selected = d.selection.clone();
        let depth = d.history_depth();
        let id = d.active_layer();
        d.begin_transform_layers(&[id]).unwrap();
        let mut g = d.transform_geometry().unwrap();
        for p in &mut g.points {
            p[0] += 12.;
            p[1] -= 2.;
        }
        d.update_transform(g).unwrap();
        let expected = pixels(&d);
        d.enable_transform_deferred().unwrap();
        assert!(d.transform_deferred());
        let source = d.transform_preview().unwrap().source;
        // Integer translation + bilinear sampling at pixel centres is exact;
        // compare every pixel, including fractional selection and source alpha.
        for y in 0..96 {
            for x in 0..128 {
                let moving = source.image.pixel(
                    (x as i64 - 12 - i64::from(source.bounds[0])) as u32,
                    (y as i64 + 2 - i64::from(source.bounds[1])) as u32,
                );
                let split = source
                    .above
                    .pixel(x, y)
                    .over(moving.over(source.below.pixel(x, y)));
                for (a, b) in split
                    .components()
                    .into_iter()
                    .zip(expected[(y * 128 + x) as usize].components())
                {
                    assert!((a - b).abs() < 2e-6, "split {x},{y}: {a}!={b}");
                }
            }
        }
        d.cancel_transform().unwrap();
        assert_eq!(pixels(&d), before);
        assert_eq!(d.history_depth(), depth);
        assert_eq!(d.selection.steps(), selected.steps());
        for kind in 0..5 {
            let mut d = selected_seed(true);
            let id = d.active_layer();
            match kind {
                0 => d
                    .set_layer_properties(
                        id,
                        LayerProperties {
                            opacity: 0.5,
                            ..Default::default()
                        },
                    )
                    .unwrap(),
                1 => d.layers.last_mut().unwrap().appearance.fill = 0.5,
                2 => {
                    let mask = d.add_mask(id).unwrap();
                    let at = d.layer_index(mask).unwrap();
                    d.layers[at].tiles.insert(
                        TileCoord::from_signed(0, 0),
                        Arc::new(
                            Tile::from_pixels(vec![
                                Pixel::from_straight([0., 0., 0., 1.]).unwrap();
                                4096
                            ])
                            .unwrap(),
                        ),
                    );
                }
                3 => {
                    let group = d.group_layer(id, "masked ancestor").unwrap();
                    let mask = d.add_mask(group).unwrap();
                    let at = d.layer_index(mask).unwrap();
                    d.layers[at].tiles.insert(
                        TileCoord::from_signed(0, 0),
                        Arc::new(
                            Tile::from_pixels(vec![
                                Pixel::from_straight([0., 0., 0., 1.]).unwrap();
                                4096
                            ])
                            .unwrap(),
                        ),
                    );
                }
                _ => {
                    d.options.max_document_tiles = 1;
                }
            }
            d.begin_transform_layers_deferred(&[id]).unwrap();
            assert!(!d.transform_deferred(), "fallback {kind}");
            d.cancel_transform().unwrap();
        }
    }
    #[test]
    fn selected_nested_late_enable_and_cancel_preserve_original_hierarchy() {
        for blend in [crate::BlendMode::Normal, crate::BlendMode::PassThrough] {
            let mut d = selected_seed(true);
            let id = d.active_layer();
            let inner = d.group_layer(id, "selection context").unwrap();
            let outer = d.group_layer(inner, "outer context").unwrap();
            for group in [inner, outer] {
                let at = d.layer_index(group).unwrap();
                d.layers[at].appearance.blend = blend;
            }
            d.set_active_layer(id).unwrap();
            let before = pixels(&d);
            let selection = d.selection.clone();
            let depth = d.history_depth();
            d.begin_transform_layers(&[id]).unwrap();
            let mut geometry = d.transform_geometry().unwrap();
            for p in &mut geometry.points {
                p[0] += 12.;
                p[1] -= 2.;
            }
            d.update_transform(geometry).unwrap();
            let expected = pixels(&d);
            d.enable_transform_deferred().unwrap();
            assert!(d.transform_deferred());
            let source = d.transform_preview().unwrap().source;
            for y in 0..96 {
                for x in 0..128 {
                    let moving = source.image.pixel(
                        (x as i64 - 12 - i64::from(source.bounds[0])) as u32,
                        (y as i64 + 2 - i64::from(source.bounds[1])) as u32,
                    );
                    let actual = source
                        .above
                        .pixel(x, y)
                        .over(moving.over(source.below.pixel(x, y)));
                    for (a, b) in actual
                        .components()
                        .into_iter()
                        .zip(expected[(y * 128 + x) as usize].components())
                    {
                        assert!(
                            (a - b).abs() < 2e-6,
                            "nested selected split {x},{y}: {a}!={b}"
                        );
                    }
                }
            }
            assert_eq!(d.layer(id).unwrap().parent, inner);
            assert_eq!(d.layer(inner).unwrap().parent, outer);
            d.cancel_transform().unwrap();
            assert_eq!(pixels(&d), before);
            assert_eq!(d.selection.steps(), selection.steps());
            assert_eq!(d.history_depth(), depth);
            assert_eq!(d.layer(id).unwrap().parent, inner);
        }
    }
    #[test]
    fn selected_preview_allocation_failure_cancels_new_session_without_pixel_or_history_changes() {
        let blocked = tempfile::NamedTempFile::new().unwrap();
        let storage = paint_storage::ScratchSpace::new(paint_storage::Config {
            directory: blocked.path().to_path_buf(),
            min_free_bytes: 0,
        });
        let mut d = Document::with_storage(
            128,
            96,
            crate::DocumentOptions {
                max_resident_tiles: 1,
                max_scratch_bytes: 65552,
                ..Default::default()
            },
            storage,
        )
        .unwrap();
        d.initialize_white_background().unwrap();
        d.set_selection(
            crate::Selection::default()
                .apply(
                    crate::SelectionShape::geometry(
                        crate::SelectionKind::Rectangle,
                        20.,
                        20.,
                        80.,
                        30.,
                        false,
                    ),
                    crate::SelectionOperation::Replace,
                    128,
                    96,
                )
                .unwrap(),
        )
        .unwrap();
        let before = pixels(&d);
        let depth = d.history_depth();
        let selection = d.selection.clone();
        assert!(d
            .begin_transform_layers_deferred(&[d.active_layer()])
            .is_err());
        assert!(d.transform.is_none());
        assert_eq!(pixels(&d), before);
        assert_eq!(d.history_depth(), depth);
        assert_eq!(d.selection.steps(), selection.steps());
        d.begin_transform_layers(&[d.active_layer()]).unwrap();
        d.cancel_transform().unwrap();
    }
    #[test]
    fn attached_masks_never_define_transform_handles() {
        for deferred in [false, true] {
            for grouped in [false, true] {
                for linked in [false, true] {
                    let mut d = seed();
                    let owner = d.active_layer();
                    d.begin_transform_layers(&[owner]).unwrap();
                    let expected = d.transform_geometry().unwrap().points;
                    d.cancel_transform().unwrap();
                    let mask = d.add_mask(owner).unwrap();
                    let at = d.layer_index(mask).unwrap();
                    d.layers[at].tiles.insert(
                        TileCoord::from_signed(-1, -1),
                        Arc::new(Tile::from_pixels(vec![Pixel::WHITE; 4096]).unwrap()),
                    );
                    // Hide every source pixel as well as extending outside its bounds.
                    for tx in 0..2 {
                        for ty in 0..2 {
                            d.layers[at].tiles.insert(
                                TileCoord::from_signed(tx, ty),
                                Arc::new(Tile::from_pixels(vec![Pixel::WHITE; 4096]).unwrap()),
                            );
                        }
                    }
                    d.set_mask_linked(mask, linked).unwrap();
                    let root = if grouped {
                        d.group_layer(owner, "masked content").unwrap()
                    } else {
                        owner
                    };
                    let before = pixels(&d);
                    let depth = d.history_depth();
                    if deferred {
                        d.begin_transform_layers_deferred(&[root]).unwrap();
                    } else {
                        d.begin_transform_layers(&[root]).unwrap();
                    }
                    assert_eq!(d.transform_geometry().unwrap().points, expected);
                    let mut geometry = d.transform_geometry().unwrap();
                    for p in &mut geometry.points {
                        p[0] += 3.;
                    }
                    d.update_transform(geometry).unwrap();
                    d.cancel_transform().unwrap();
                    assert_eq!(pixels(&d), before);
                    assert_eq!(d.history_depth(), depth);
                    // Explicit mask editing still uses its own full extent.
                    d.begin_transform_layers(&[mask]).unwrap();
                    let g = d.transform_geometry().unwrap();
                    assert_eq!(g.points[0], [-64., -64.]);
                    assert_eq!(g.points[15], [128., 128.]);
                    d.cancel_transform().unwrap();
                }
            }
        }
    }
    fn compound_seed(kind: u8) -> (Document, Vec<LayerId>) {
        let mut d = seed();
        let owner = d.active_layer();
        d.move_layer(owner, 5, -3).unwrap();
        let mask = d.add_mask(owner).unwrap();
        let at = d.layer_index(mask).unwrap();
        let pixels = (0..4096)
            .map(|i| Pixel::from_straight([(i % 64) as f32 / 63., 0.3, 0.3, 1.]).unwrap())
            .collect();
        d.layers[at].tiles.insert(
            TileCoord::from_signed(0, 0),
            Arc::new(Tile::from_pixels(pixels).unwrap()),
        );
        d.layers[at].appearance.offset_x = -4;
        let mut roots = vec![owner];
        if kind == 1 {
            let group = d.group_layer(owner, "root group").unwrap();
            let outer = d.group_layer(group, "outer group").unwrap();
            d.add_mask(outer).unwrap();
            let at = d.layer_index(outer).unwrap();
            d.layers[at].appearance.blend = crate::BlendMode::Normal;
            d.layers[at].properties.opacity = 0.65;
            let at = d.layer_index(owner).unwrap();
            d.layers[at].appearance.blend = crate::BlendMode::Multiply;
            roots = vec![outer];
        } else if kind == 2 {
            let other = d.add_layer("second root").unwrap();
            d.begin_stroke(
                crate::Brush {
                    radius: 12.,
                    color: Pixel::from_straight([0., 0., 0., 1.]).unwrap(),
                    ..Default::default()
                },
                InputPoint::new(70., 50., 1.),
            )
            .unwrap();
            d.end_stroke().unwrap();
            roots.push(other);
        } else if kind >= 3 {
            let inner = d.group_layer(owner, "inner context").unwrap();
            d.set_active_layer(owner).unwrap();
            let sibling = d.add_layer("stationary sibling").unwrap();
            d.begin_stroke(
                crate::Brush {
                    radius: 8.,
                    color: Pixel::from_straight([0., 1., 0.3, 0.65]).unwrap(),
                    ..Default::default()
                },
                InputPoint::new(100., 75., 1.),
            )
            .unwrap();
            d.end_stroke().unwrap();
            assert_eq!(d.layer(sibling).unwrap().parent, inner);
            let outer = d.group_layer(inner, "outer context").unwrap();
            for id in [inner, outer] {
                let at = d.layer_index(id).unwrap();
                d.layers[at].appearance.blend = if kind == 3 {
                    crate::BlendMode::Normal
                } else {
                    crate::BlendMode::PassThrough
                };
            }
            let at = d.layer_index(owner).unwrap();
            d.layers[at].properties.opacity = 0.65;
        }
        (d, roots)
    }
    fn pixels(d: &Document) -> Vec<Pixel> {
        (0..96)
            .flat_map(|y| d.snapshot().read_row(0, y, 128).unwrap())
            .collect()
    }
    #[test]
    fn compound_transform_preserves_sources_and_final_layer_mask_history() {
        for kind in 0..5 {
            for warp in [false, true] {
                let (mut a, roots) = compound_seed(kind);
                let (mut b, _) = compound_seed(kind);
                let baseline = a.layers.clone();
                let before = pixels(&a);
                let depth = a.history_depth();
                a.begin_transform_layers_deferred(&roots).unwrap();
                b.begin_transform_layers(&roots).unwrap();
                assert!(a.transform_deferred(), "kind {kind}");
                let source = a.transform_preview().unwrap().source;
                for y in (0..96).step_by(5) {
                    for x in (0..128).step_by(5) {
                        let image = source.image.pixel(
                            (x as i32 - source.bounds[0]) as u32,
                            (y as i32 - source.bounds[1]) as u32,
                        );
                        let split = source
                            .above
                            .pixel(x, y)
                            .over(image.over(source.below.pixel(x, y)));
                        for (p, q) in split
                            .components()
                            .into_iter()
                            .zip(a.pixel(x, y).components())
                        {
                            assert!((p - q).abs() < 1e-6, "initial split {kind} {x},{y}");
                        }
                    }
                }
                let initial = a.transform_geometry().unwrap();
                let preview = a.transform_preview().unwrap();
                let expected =
                    (initial.points[0][0] - source.bounds[0] as f64) / source.bounds[2] as f64;
                assert!((preview.vertices[0][2] as f64 - expected).abs() < 1e-6);
                for i in 1..=8 {
                    let mut g = initial.clone();
                    g.warp = warp;
                    for p in &mut g.points {
                        p[0] += i as f64;
                        p[1] -= 3.;
                    }
                    if warp {
                        g.points[5][0] += 7.;
                        g.points[10][1] -= 5.;
                    }
                    a.update_transform(g.clone()).unwrap();
                    b.update_transform(g).unwrap();
                    assert!(Arc::ptr_eq(&source, &a.transform_preview().unwrap().source));
                    assert_eq!(a.history_depth(), depth);
                    for (old, new) in baseline.iter().zip(&a.layers) {
                        assert_eq!(old.parent, new.parent);
                        assert_eq!(old.properties, new.properties);
                        assert_eq!(old.appearance, new.appearance);
                        for (key, tile) in &old.tiles {
                            assert!(Arc::ptr_eq(tile, &new.tiles[key]));
                        }
                    }
                }
                a.checkpoint_transform().unwrap();
                a.undo_transform().unwrap();
                a.redo_transform().unwrap();
                a.commit_transform().unwrap();
                b.commit_transform().unwrap();
                assert_eq!(pixels(&a), pixels(&b));
                assert_eq!(a.history_depth().0, depth.0 + 1);
                a.undo().unwrap();
                assert_eq!(pixels(&a), before);
                a.redo().unwrap();
                assert_eq!(pixels(&a), pixels(&b));
                a.begin_transform_layers_deferred(&roots).unwrap();
                let mut g = a.transform_geometry().unwrap();
                g.points[5][0] += 2.;
                g.warp = true;
                a.update_transform(g).unwrap();
                a.cancel_transform().unwrap();
                assert_eq!(pixels(&a), pixels(&b));
            }
        }
    }
    #[test]
    fn compound_transform_late_opt_in_failure_and_incomplete_sources_stay_safe() {
        let (mut d, roots) = compound_seed(1);
        let before = pixels(&d);
        let depth = d.history_depth();
        d.begin_transform_layers(&roots).unwrap();
        let mut g = d.transform_geometry().unwrap();
        for p in &mut g.points {
            p[0] += 10.;
        }
        d.update_transform(g).unwrap();
        d.enable_transform_deferred().unwrap();
        assert!(d.transform_deferred());
        let source = d.transform_preview().unwrap().source;
        for y in (0..96).step_by(5) {
            for x in (0..128).step_by(5) {
                let pixel = source
                    .image
                    .pixel((x - source.bounds[0]) as u32, (y - source.bounds[1]) as u32);
                for (p, q) in pixel
                    .components()
                    .into_iter()
                    .zip(before[(y * 128 + x) as usize].components())
                {
                    assert!((p - q).abs() < 1e-6, "late source must use originals");
                }
            }
        }
        d.options.max_document_tiles = 0;
        assert!(d.commit_transform().is_err());
        assert!(d.transform_deferred());
        assert_eq!(d.history_depth(), depth);
        assert!(Arc::ptr_eq(&source, &d.transform_preview().unwrap().source));
        d.cancel_transform().unwrap();
        assert_eq!(pixels(&d), before);
        let (mut d, roots) = compound_seed(0);
        let mask = d.mask_for(roots[0]).unwrap().id;
        d.set_mask_linked(mask, false).unwrap();
        d.begin_transform_layers_deferred(&roots).unwrap();
        assert!(!d.transform_deferred());
        d.cancel_transform().unwrap();
        d.set_mask_linked(mask, true).unwrap();
        d.set_active_layer(roots[0]).unwrap();
        d.begin_transform().unwrap();
        d.enable_transform_deferred().unwrap();
        assert!(
            !d.transform_deferred(),
            "direct single-layer session must not animate its untouched mask"
        );
        d.cancel_transform().unwrap();
        let (mut d, _) = compound_seed(1);
        let child = d.layers.iter().find(|l| !l.group && !l.mask).unwrap().id;
        d.begin_transform_layers_deferred(&[child]).unwrap();
        assert!(!d.transform_deferred());
        d.cancel_transform().unwrap();
        let (mut gap, _) = compound_seed(2);
        let extra = gap.add_layer("third root").unwrap();
        gap.begin_stroke(crate::Brush::default(), InputPoint::new(10., 10., 1.))
            .unwrap();
        gap.end_stroke().unwrap();
        let bottom = gap.layers.iter().find(|l| !l.group && !l.mask).unwrap().id;
        gap.begin_transform_layers_deferred(&[bottom, extra])
            .unwrap();
        assert!(!gap.transform_deferred());
        gap.cancel_transform().unwrap();
    }

    #[test]
    fn nested_transform_late_originals_failure_and_unsafe_ancestors() {
        for kind in 3..5 {
            let (mut d, roots) = compound_seed(kind);
            let before = pixels(&d);
            let hierarchy: Vec<_> = d.layers.iter().map(|l| (l.id, l.parent)).collect();
            let depth = d.history_depth();
            d.begin_transform_layers(&roots).unwrap();
            let mut g = d.transform_geometry().unwrap();
            for p in &mut g.points {
                p[0] += 15.;
            }
            d.update_transform(g).unwrap();
            d.enable_transform_deferred().unwrap();
            assert!(d.transform_deferred());
            let source = d.transform_preview().unwrap().source;
            for y in 0..96 {
                for x in 0..128 {
                    let image = source.image.pixel(
                        (x as i32 - source.bounds[0]) as u32,
                        (y as i32 - source.bounds[1]) as u32,
                    );
                    let split = source
                        .above
                        .pixel(x, y)
                        .over(image.over(source.below.pixel(x, y)));
                    for (a, b) in split
                        .components()
                        .into_iter()
                        .zip(before[(y * 128 + x) as usize].components())
                    {
                        assert!(
                            (a - b).abs() < 1e-6,
                            "nested late original {kind} {x},{y}: {a}/{b}"
                        );
                    }
                }
            }
            d.options.max_document_tiles = 0;
            assert!(d.commit_transform().is_err());
            assert!(Arc::ptr_eq(&source, &d.transform_preview().unwrap().source));
            assert_eq!(d.history_depth(), depth);
            d.cancel_transform().unwrap();
            assert_eq!(pixels(&d), before);
            assert_eq!(
                d.layers
                    .iter()
                    .map(|l| (l.id, l.parent))
                    .collect::<Vec<_>>(),
                hierarchy
            );
        }
        for kind in 0..8 {
            let (mut d, roots) = compound_seed(3);
            let parent = d.layer(roots[0]).unwrap().parent;
            let at = d.layer_index(parent).unwrap();
            match kind {
                0 => {
                    let mask = d.add_mask(parent).unwrap();
                    let at = d.layer_index(mask).unwrap();
                    d.layers[at].tiles.insert(
                        TileCoord::from_signed(0, 0),
                        Arc::new(
                            Tile::from_pixels(vec![
                                Pixel::from_straight([0., 0., 0., 1.]).unwrap();
                                4096
                            ])
                            .unwrap(),
                        ),
                    );
                }
                1 => d.layers[at].properties.opacity = 0.5,
                2 => d.layers[at].appearance.fill = 0.5,
                3 => d.layers[at].appearance.blend = crate::BlendMode::Multiply,
                4 => d.layers[at].properties.visible = false,
                5 => d.layers[at].clipped = true,
                6 => {
                    let mask = d.mask_for(roots[0]).unwrap().id;
                    d.set_mask_linked(mask, false).unwrap();
                }
                _ => {
                    let sibling = d
                        .layers
                        .iter_mut()
                        .find(|l| l.name == "stationary sibling")
                        .unwrap();
                    sibling.appearance.blend = crate::BlendMode::Multiply;
                }
            }
            d.begin_transform_layers_deferred(&roots).unwrap();
            assert!(!d.transform_deferred(), "unsafe ancestor {kind}");
            d.cancel_transform().unwrap();
        }
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
    fn deferred_transform_cancel_placed_source_and_selection_capture() {
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
        assert!(d.transform_deferred());
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
