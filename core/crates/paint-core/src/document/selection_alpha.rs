//! Load source coverage, independently of the thumbnail/display and layer selection.
use super::*;
use crate::{SelectionKind, SelectionMask, SelectionShape};

impl Document {
    pub fn layer_alpha_shape(&self, id: LayerId) -> Result<SelectionShape> {
        self.idle()?;
        let layer = &self.layers[self.layer_index(id)?];
        // Implicit backgrounds/masks may be enormous: retain their exact rectangle
        // without scanning or allocating a canvas-sized coverage buffer.
        if !layer.group && layer.tiles.is_empty() {
            let (x, y, w, h) = if layer.mask {
                (0, 0, i64::from(self.width), i64::from(self.height))
            } else {
                let x = i64::from(layer.appearance.offset_x).max(0);
                let y = i64::from(layer.appearance.offset_y).max(0);
                let right = (i64::from(layer.appearance.offset_x) + i64::from(layer.white.0))
                    .min(i64::from(self.width));
                let bottom = (i64::from(layer.appearance.offset_y) + i64::from(layer.white.1))
                    .min(i64::from(self.height));
                (x, y, right - x, bottom - y)
            };
            if w > 0 && h > 0 {
                return Ok(SelectionShape::geometry(
                    SelectionKind::Rectangle,
                    x as f64,
                    y as f64,
                    w as f64,
                    h as f64,
                    false,
                ));
            }
        }
        let mut layers = if layer.group {
            self.layers[self.subtree(id)?].to_vec()
        } else {
            vec![layer.clone()]
        };
        let root = layers.last_mut().expect("layer subtree root");
        root.parent = 0;
        root.properties = LayerProperties::default();
        root.appearance.fill = 1.;
        root.appearance.blend = crate::BlendMode::Normal;
        root.clipped = false;
        let mut bounds = [i64::from(self.width), i64::from(self.height), 0, 0];
        for source in layers.iter().filter(|l| !l.group && !l.mask) {
            let ox = i64::from(source.appearance.offset_x);
            let oy = i64::from(source.appearance.offset_y);
            let mut include = |x: i64, y: i64, w: i64, h: i64| {
                bounds[0] = bounds[0].min(x.max(0));
                bounds[1] = bounds[1].min(y.max(0));
                bounds[2] = bounds[2].max((x + w).min(i64::from(self.width)));
                bounds[3] = bounds[3].max((y + h).min(i64::from(self.height)));
            };
            if source.white.0 > 0 && source.white.1 > 0 {
                include(ox, oy, source.white.0.into(), source.white.1.into());
            }
            for key in source.tiles.keys() {
                include(
                    ox + i64::from(key.signed_x()) * 64,
                    oy + i64::from(key.signed_y()) * 64,
                    64,
                    64,
                );
            }
        }
        if layer.mask {
            bounds = [0, 0, self.width.into(), self.height.into()];
        }
        let read = |x, y, count| -> Result<Vec<u8>> {
            let row = if layer.group {
                groups::composite_row(&layers, x, y, count)?
            } else {
                layer.read_canvas_row(x, y, count)?
            };
            Ok(row
                .into_iter()
                .map(|p| {
                    let alpha = if layer.mask {
                        1. - p.alpha()
                    } else {
                        p.alpha()
                    };
                    (alpha * 255.).round().clamp(0., 255.) as u8
                })
                .collect())
        };
        let mut tight = [self.width, self.height, 0, 0];
        let mut nonzero = 0u64;
        let mut opaque = true;
        if bounds[2] > bounds[0] && bounds[3] > bounds[1] {
            let left = bounds[0] as u32;
            let width = (bounds[2] - bounds[0]) as u32;
            for y in bounds[1] as u32..bounds[3] as u32 {
                for (i, alpha) in read(left, y, width)?.into_iter().enumerate() {
                    if alpha != 0 {
                        let x = left + i as u32;
                        tight = [
                            tight[0].min(x),
                            tight[1].min(y),
                            tight[2].max(x + 1),
                            tight[3].max(y + 1),
                        ];
                        nonzero += 1;
                        opaque &= alpha == 255;
                    }
                }
                self.pool.trim()?;
            }
        }
        let (left, top, width, height) = if nonzero == 0 {
            (0, 0, 1, 1)
        } else {
            (tight[0], tight[1], tight[2] - tight[0], tight[3] - tight[1])
        };
        let bytes = u64::from(width) * u64::from(height);
        let rectangle = opaque && nonzero == bytes;
        let mut shape = SelectionShape::geometry(
            if rectangle {
                SelectionKind::Rectangle
            } else {
                SelectionKind::Mask
            },
            left.into(),
            top.into(),
            width.into(),
            height.into(),
            false,
        );
        if !rectangle {
            if bytes > crate::MAX_SELECTION_MASK_BYTES as u64 {
                return Err(Error::ResourceLimit("layer alpha selection mask"));
            }
            let mut alpha = Vec::new();
            alpha
                .try_reserve_exact(bytes as usize)
                .map_err(|_| Error::ResourceLimit("layer alpha selection allocation"))?;
            if nonzero == 0 {
                alpha.push(0);
            } else {
                for y in top..top + height {
                    alpha.extend(read(left, y, width)?);
                    self.pool.trim()?;
                }
            }
            shape.mask = Some(SelectionMask::from_alpha(width, height, alpha)?);
        }
        Ok(shape)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HistoryAction, Selection, SelectionOperation};

    fn selected(doc: &Document, id: u64) -> Selection {
        doc.selection()
            .apply(
                doc.layer_alpha_shape(id).unwrap(),
                SelectionOperation::Replace,
                doc.width,
                doc.height,
            )
            .unwrap()
    }

    #[test]
    fn layer_alpha_uses_raw_coverage_offsets_and_atomic_history() {
        let mut doc = Document::new(96, 64).unwrap();
        let id = doc.active_layer();
        let layer = &mut doc.layers[0];
        layer.white = (0, 0);
        layer.properties.visible = false;
        layer.properties.opacity = 0.1;
        layer.appearance.fill = 0.2;
        layer.appearance.locks = crate::LOCK_ALL;
        layer.appearance.offset_x = -1;
        layer.appearance.offset_y = 3;
        let mut pixels = vec![Pixel::TRANSPARENT; 4096];
        pixels[0] = Pixel::WHITE; // clipped off the document
        pixels[1] = Pixel::from_straight([1., 0., 0., 0.5]).unwrap();
        pixels[3] = Pixel::WHITE; // transparent hole between the samples
        layer.tiles.insert(
            TileCoord { x: 0, y: 0 },
            Arc::new(Tile::from_pixels(pixels).unwrap()),
        );
        let before = doc.selection().clone();
        let next = selected(&doc, id);
        assert!((next.coverage(0.5, 3.5) - 128. / 255.).abs() < 1e-6);
        assert_eq!(next.coverage(1.5, 3.5), 0.);
        assert_eq!(next.coverage(2.5, 3.5), 1.);
        assert_eq!(next.coverage(0.5, 2.5), 0.);
        let depth = doc.history_depth();
        doc.set_selection_with_action(next.clone(), HistoryAction::Selection)
            .unwrap();
        assert_eq!(doc.history_depth().0, depth.0 + 1);
        assert_eq!(doc.active_layer(), id);
        doc.undo().unwrap();
        assert_eq!(doc.selection().to_text(), before.to_text());
        doc.redo().unwrap();
        assert_eq!(doc.selection().to_text(), next.to_text());
        let revision = doc.revision();
        assert!(doc.layer_alpha_shape(999).is_err());
        assert_eq!(doc.revision(), revision);
    }

    #[test]
    fn layer_alpha_empty_and_large_implicit_white_keep_distinct_selection() {
        let mut doc = Document::new(9000, 9000).unwrap();
        let id = doc.active_layer();
        doc.layers[0].white = (9000, 9000);
        let shape = doc.layer_alpha_shape(id).unwrap();
        assert_eq!(shape.kind, SelectionKind::Rectangle);
        assert_eq!(shape.width, 9000.);
        let empty = doc.add_layer("empty").unwrap();
        let selection = selected(&doc, empty);
        assert!(selection.enabled());
        assert_eq!(selection.coverage(12.5, 12.5), 0.);
        assert_eq!(selection.coverage(0.5, 0.5), 0.);
    }

    #[test]
    fn layer_alpha_combines_with_existing_selection_in_all_four_modes() {
        let mut doc = Document::new(16, 16).unwrap();
        let id = doc.active_layer();
        doc.layers[0].white = (4, 4);
        let old = Selection::default()
            .apply(
                SelectionShape::geometry(SelectionKind::Rectangle, 2., 0., 4., 4., false),
                SelectionOperation::Replace,
                16,
                16,
            )
            .unwrap();
        for (op, expected) in [
            (SelectionOperation::Replace, [1., 1., 0.]),
            (SelectionOperation::Add, [1., 1., 1.]),
            (SelectionOperation::Subtract, [0., 0., 1.]),
            (SelectionOperation::Intersect, [0., 1., 0.]),
        ] {
            let next = old
                .apply(doc.layer_alpha_shape(id).unwrap(), op, 16, 16)
                .unwrap();
            assert_eq!(
                [
                    next.coverage(0.5, 0.5),
                    next.coverage(2.5, 0.5),
                    next.coverage(4.5, 0.5)
                ],
                expected
            );
        }
    }

    #[test]
    fn layer_alpha_mask_uses_white_coverage_and_group_uses_its_subtree() {
        let mut doc = Document::new(64, 64).unwrap();
        let owner = doc.active_layer();
        doc.layers[0].white = (8, 8);
        let mask = doc.add_mask(owner).unwrap();
        let mut pixels = vec![Pixel::TRANSPARENT; 4096];
        pixels[0] = Pixel::WHITE;
        pixels[1] = Pixel::from_straight([0., 0., 0., 0.25]).unwrap();
        let index = doc.layer_index(mask).unwrap();
        doc.layers[index].tiles.insert(
            TileCoord { x: 0, y: 0 },
            Arc::new(Tile::from_pixels(pixels).unwrap()),
        );
        let raw = selected(&doc, owner);
        assert_eq!(raw.coverage(0.5, 0.5), 1.); // owner's mask is independent
        let white = selected(&doc, mask);
        assert_eq!(white.coverage(0.5, 0.5), 0.);
        assert!((white.coverage(1.5, 0.5) - 191. / 255.).abs() < 1e-6);
        assert_eq!(white.coverage(32.5, 32.5), 1.);
        let group = doc.group_layer(owner, "group").unwrap();
        let shape = selected(&doc, group);
        assert_eq!(shape.coverage(0.5, 0.5), 0.);
        assert!((shape.coverage(1.5, 0.5) - 191. / 255.).abs() < 1e-6);
        assert_eq!(shape.coverage(7.5, 7.5), 1.);
        assert_eq!(shape.coverage(8.5, 7.5), 0.);
    }

    #[test]
    fn layer_alpha_over_budget_rejects_without_changing_selection_or_history() {
        let mut doc = Document::new(8192, 4096).unwrap();
        let id = doc.active_layer();
        doc.layers[0].white = (8192, 4096);
        let mut pixels = vec![Pixel::WHITE; 4096];
        pixels[0] = Pixel::TRANSPARENT;
        doc.layers[0].tiles.insert(
            TileCoord { x: 0, y: 0 },
            Arc::new(Tile::from_pixels(pixels).unwrap()),
        );
        let revision = doc.revision();
        let history = doc.history_depth();
        assert!(matches!(
            doc.layer_alpha_shape(id),
            Err(Error::ResourceLimit(_))
        ));
        assert_eq!(doc.revision(), revision);
        assert_eq!(doc.history_depth(), history);
        assert!(!doc.selection().enabled());
    }
}
