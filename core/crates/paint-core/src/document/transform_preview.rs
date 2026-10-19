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
        let extent = |axis| {
            let lo = geometry
                .points
                .iter()
                .map(|p| p[axis])
                .fold(f64::INFINITY, f64::min);
            let hi = geometry
                .points
                .iter()
                .map(|p| p[axis])
                .fold(f64::NEG_INFINITY, f64::max);
            hi - lo
        };
        // Include all 1/3 control-node boundaries. At most 145² vertices, independent of pixels.
        // Projective maps also tessellate: UV interpolation is affine within each triangle.
        let columns = ((extent(0) / 18.).ceil() as u32).clamp(6, 48) * 3 + 1;
        let rows = ((extent(1) / 18.).ceil() as u32).clamp(6, 48) * 3 + 1;
        let bounds = state.source.bounds;
        let mut vertices = Vec::with_capacity((columns * rows) as usize);
        for y in 0..rows {
            let v = y as f64 / (rows - 1) as f64;
            for x in 0..columns {
                let u = x as f64 / (columns - 1) as f64;
                let p = geometry.map(u, v);
                vertices.push([
                    p[0] as f32,
                    p[1] as f32,
                    ((bounds[0] + u * bounds[2] - source.bounds[0] as f64)
                        / source.bounds[2] as f64) as f32,
                    ((bounds[1] + v * bounds[3] - source.bounds[1] as f64)
                        / source.bounds[3] as f64) as f32,
                ]);
            }
        }
        Some(crate::ResidentPreview {
            source,
            revision: self.revision,
            focus: false,
            forward: true,
            columns,
            rows,
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
}
