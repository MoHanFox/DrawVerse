use crate::{
    history::{Command, History, StoredCommand, TileChange},
    Brush, BrushMode, Error, InputPoint, Layer, LayerId, LayerProperties, Pixel, Result, Tile,
    TileCoord, Tool, TILE_BYTES, TILE_SIZE,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub const MAX_DIMENSION: u32 = 1_000_000;
const MAX_DABS_PER_SEGMENT: usize = 8192;
const MAX_PIXEL_VISITS_PER_SEGMENT: usize = 8_000_000;
pub(crate) mod groups;

#[derive(Clone, Copy, Debug)]
pub struct DocumentOptions {
    /// Logical live tiles. Pixel residency has a separate cache budget.
    pub max_document_tiles: usize,
    pub max_resident_tiles: usize,
    pub max_scratch_bytes: u64,
    /// Retained in-memory history accounting. Oversized commands use lossless
    /// encoding and bounded temporary disk storage, not stroke rejection.
    pub max_history_bytes: usize,
    pub max_history_commands: usize,
}

#[cfg(test)]
mod history_failure_tests {
    use super::*;
    fn document() -> Document {
        let mut tiles = BTreeMap::new();
        for y in 0..2 {
            for x in 0..3 {
                let pixels = (0..4096)
                    .map(|i| Pixel::from_straight([i as f32 / 4096., 0.3, 0.2, 1.]).unwrap())
                    .collect();
                tiles.insert(TileCoord { x, y }, Tile::from_pixels(pixels).unwrap());
            }
        }
        Document::from_import(
            192,
            128,
            vec![crate::ImportedLayer {
                name: "noise".into(),
                properties: LayerProperties::default(),
                tiles,
            }],
            0,
            DocumentOptions {
                max_history_bytes: 2 * TILE_BYTES + 256,
                ..Default::default()
            },
        )
        .unwrap()
    }
    fn pixels(doc: &Document) -> Vec<[u32; 4]> {
        (0..128)
            .flat_map(|y| (0..192).map(move |x| doc.pixel(x, y).components().map(f32::to_bits)))
            .collect()
    }
    fn paint(doc: &mut Document) {
        doc.begin_stroke(
            Brush {
                radius: 128.,
                ..Default::default()
            },
            InputPoint::new(96., 64., 1.),
        )
        .unwrap();
    }
    #[test]
    fn history_write_failure_rolls_back_stroke_and_preserves_prior_history() {
        let mut doc = document();
        doc.set_layer_properties(
            1,
            LayerProperties {
                visible: true,
                opacity: 0.8,
            },
        )
        .unwrap();
        let original = pixels(&doc);
        let history = doc.history_depth();
        paint(&mut doc);
        crate::history_storage::fail_writes_for_test(true);
        let result = doc.end_stroke();
        crate::history_storage::fail_writes_for_test(false);
        assert!(matches!(result, Err(Error::HistoryStorage(_))));
        assert_eq!(pixels(&doc), original);
        assert_eq!(doc.history_depth(), history);
        assert!(!doc.stroke_active());
        doc.undo().unwrap();
        assert_eq!(doc.layer(1).unwrap().properties().opacity, 1.);
    }
    #[test]
    fn damaged_backing_does_not_partially_undo_or_redo() {
        for redo in [false, true] {
            let mut doc = document();
            paint(&mut doc);
            doc.end_stroke().unwrap();
            assert!(doc.history_disk_bytes() > 0);
            if redo {
                doc.undo().unwrap();
            }
            let original = pixels(&doc);
            let depth = doc.history_depth();
            let revision = doc.revision();
            let stored = if redo {
                doc.history.redo.last_mut().unwrap()
            } else {
                doc.history.undo.back_mut().unwrap()
            };
            let StoredCommand::Encoded(payload, _) = stored else {
                unreachable!()
            };
            payload.truncate_for_test();
            assert!(if redo { doc.redo() } else { doc.undo() }.is_err());
            assert_eq!(pixels(&doc), original);
            assert_eq!(doc.history_depth(), depth);
            assert_eq!(doc.revision(), revision);
        }
    }
    #[test]
    fn structural_history_write_failure_preserves_group_membership_revision_and_history() {
        let mut doc = Document::with_options(
            64,
            64,
            DocumentOptions {
                max_history_bytes: 2 * TILE_BYTES + 256,
                ..Default::default()
            },
        )
        .unwrap();
        for i in 0..120 {
            doc.add_layer(format!("{i:04}{}", "n".repeat(1000)))
                .unwrap();
        }
        let revision = doc.revision();
        let history = doc.history_depth();
        let active = doc.active_layer();
        crate::history_storage::fail_writes_for_test(true);
        let result = doc.group_layer(active, "failed group");
        crate::history_storage::fail_writes_for_test(false);
        assert!(matches!(result, Err(Error::HistoryStorage(_))));
        assert_eq!(doc.revision(), revision);
        assert_eq!(doc.history_depth(), history);
        assert_eq!(doc.layers().len(), 121);
        assert_eq!(doc.active_layer(), active);
        assert!(doc.layers().iter().all(|l| !l.group && l.parent == 0));
        let group = doc.group_layer(active, "working group").unwrap();
        assert!(doc.history_disk_bytes() > 0);
        doc.undo().unwrap();
        assert!(doc.layer(group).is_none());
        doc.redo().unwrap();
        assert_eq!(doc.layer(active).unwrap().parent, group);
    }
}

impl Default for DocumentOptions {
    fn default() -> Self {
        Self {
            max_document_tiles: 1_000_000,
            max_resident_tiles: 4096,
            max_scratch_bytes: 8 * 1024 * 1024 * 1024,
            max_history_bytes: 64 * 1024 * 1024,
            max_history_commands: 100,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirtyTiles {
    pub all: bool,
    pub tiles: BTreeSet<TileCoord>,
}

/// Cloning snapshots copies only layer metadata and Arc references, never pixel buffers.
/// Consumers must release old snapshots; external snapshots are outside the history budget.
#[derive(Clone, Debug)]
pub struct DocumentSnapshot {
    pub width: u32,
    pub height: u32,
    pub revision: u64,
    layers: Vec<Layer>,
    selection: crate::Selection,
    pool: Arc<crate::page_pool::PagePool>,
}

impl DocumentSnapshot {
    /// Temporary appearance override; shares all pixels and never changes the document/history.
    pub fn with_layer_blend(mut self, id: LayerId, blend: crate::BlendMode) -> Result<Self> {
        let layer = self
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.mask)
            .ok_or(Error::InvalidArgument(
                "blend preview requires a non-mask layer",
            ))?;
        layer.appearance.blend = blend;
        Ok(self)
    }
    pub fn selection(&self) -> &crate::Selection {
        &self.selection
    }
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Reference normal composition for sampling/tests, not a frame renderer.
    /// Convenience reference sampler; use try_pixel for fallible storage.
    pub fn pixel(&self, x: u32, y: u32) -> Pixel {
        self.try_pixel(x, y)
            .expect("document read failed; use try_pixel")
    }
    pub fn try_pixel(&self, x: u32, y: u32) -> Result<Pixel> {
        if x >= self.width || y >= self.height {
            return Ok(Pixel::TRANSPARENT);
        }
        composite(&self.layers, x, y)
    }
    pub fn read_row(&self, x: u32, y: u32, count: u32) -> Result<Vec<Pixel>> {
        if y >= self.height || x.checked_add(count).is_none_or(|end| end > self.width) {
            return Err(Error::InvalidArgument("row outside document"));
        }
        composite_row(&self.layers, x, y, count)
    }
    pub fn storage_stats(&self) -> crate::StorageStats {
        self.pool.stats()
    }
    pub fn trim_storage(&self) -> Result<()> {
        self.pool.trim()
    }
}

#[test]
fn blend_preview_snapshot_shares_tiles_without_mutating_document() {
    let mut doc = Document::new(32, 32).unwrap();
    doc.begin_stroke(Brush::default(), InputPoint::new(16., 16., 1.))
        .unwrap();
    doc.end_stroke().unwrap();
    let top = doc.add_layer("red").unwrap();
    doc.begin_stroke(
        Brush {
            color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
            ..Brush::default()
        },
        InputPoint::new(16., 16., 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    let snapshot = doc.snapshot();
    let revision = doc.revision();
    let history = doc.history_depth();
    let preview = snapshot
        .clone()
        .with_layer_blend(top, crate::BlendMode::Multiply)
        .unwrap();
    assert_eq!(preview.pixel(16, 16).components(), [0., 0., 0., 1.]);
    assert_eq!(snapshot.pixel(16, 16).components(), [1., 0., 0., 1.]);
    assert_eq!(doc.snapshot().pixel(16, 16), snapshot.pixel(16, 16));
    assert_eq!(doc.revision(), revision);
    assert_eq!(doc.history_depth(), history);
    assert!(Arc::ptr_eq(
        snapshot.layers[1].tiles.values().next().unwrap(),
        preview.layers[1].tiles.values().next().unwrap()
    ));
    assert!(snapshot
        .with_layer_blend(9999, crate::BlendMode::Multiply)
        .is_err());
}

#[test]
fn history_actions_survive_encoded_eviction_undo_redo_and_branching() {
    use crate::HistoryAction::*;
    let mut doc = Document::with_options(
        128,
        32,
        DocumentOptions {
            max_history_bytes: 2 * TILE_BYTES + 256,
            max_history_commands: 3,
            ..Default::default()
        },
    )
    .unwrap();
    let point = InputPoint::new(16., 16., 1.);
    for i in 0..150 {
        doc.begin_stroke(
            crate::Brush {
                radius: 1.,
                color: if i % 2 == 0 {
                    Pixel::WHITE
                } else {
                    Pixel::from_straight([0., 0., 0., 1.]).unwrap()
                },
                ..Default::default()
            },
            point,
        )
        .unwrap();
        doc.stroke_to(InputPoint::new(80., 16., 1.)).unwrap();
        assert!(doc.end_stroke().unwrap());
    }
    assert_eq!(doc.history_depth(), (3, 0));
    assert_eq!(doc.history_actions(), vec![Truncated, Brush, Brush, Brush]);
    assert!(matches!(
        doc.history.undo.back(),
        Some(StoredCommand::Encoded(..))
    ));
    doc.begin_stroke(
        crate::Brush {
            radius: 1.,
            mode: BrushMode::Erase,
            ..Default::default()
        },
        point,
    )
    .unwrap();
    doc.stroke_to(InputPoint::new(80., 16., 1.)).unwrap();
    doc.end_stroke().unwrap();
    let actions = doc.history_actions();
    assert_eq!(actions.last(), Some(&Eraser));
    doc.undo().unwrap();
    assert_eq!(doc.history_actions(), actions);
    doc.redo().unwrap();
    assert_eq!(doc.history_actions(), actions);
    doc.undo().unwrap();
    doc.set_selection_with_action(crate::Selection::all(128, 32), SelectAll)
        .unwrap();
    assert_eq!(doc.history_depth(), (3, 0));
    assert_eq!(doc.history_actions().last(), Some(&SelectAll));
    assert!(!doc.history_actions().contains(&Eraser));
}

#[derive(Debug)]
struct Stroke {
    layer: LayerId,
    brush: Brush,
    last: InputPoint,
    last_dab: InputPoint,
    distance_to_next: f64,
    before: BTreeMap<TileCoord, Option<Arc<Tile>>>,
    mask_paint: Option<(f32, f32)>,
}

#[derive(Debug)]
pub struct Document {
    pool: Arc<crate::page_pool::PagePool>,
    width: u32,
    height: u32,
    layers: Vec<Layer>,
    active_layer: LayerId,
    next_layer_id: LayerId,
    options: DocumentOptions,
    history: History,
    stroke: Option<Stroke>,
    revision: u64,
    dirty: DirtyTiles,
    selection: crate::Selection,
}

fn composite(layers: &[Layer], x: u32, y: u32) -> Result<Pixel> {
    if layers.iter().any(|l| l.group || l.mask || l.clipped) {
        return Ok(composite_row(layers, x, y, 1)?[0]);
    }
    layers
        .iter()
        .filter(|l| l.properties.visible && l.properties.opacity * l.appearance.fill > 0.)
        .try_fold(Pixel::TRANSPARENT, |destination, layer| {
            Ok(crate::blend_pixel(
                layer
                    .canvas_pixel(x, y)?
                    .scaled(layer.properties.opacity * layer.appearance.fill),
                destination,
                layer.appearance.blend,
                x,
                y,
                u64::from(layer.appearance.dissolve_seed),
            ))
        })
}

/// Batch pins keep disk access outside the pixel loop.
fn composite_row(layers: &[Layer], x: u32, y: u32, count: u32) -> Result<Vec<Pixel>> {
    if layers.iter().any(|l| l.group || l.mask || l.clipped) {
        return groups::composite_row(layers, x, y, count);
    }
    let mut output = vec![Pixel::TRANSPARENT; count as usize];
    for layer in layers
        .iter()
        .filter(|l| l.properties.visible && l.properties.opacity * l.appearance.fill > 0.)
    {
        let row = layer.read_canvas_row(x, y, count)?;
        for (i, (destination, source)) in output.iter_mut().zip(row).enumerate() {
            *destination = crate::blend_pixel(
                source.scaled(layer.properties.opacity * layer.appearance.fill),
                *destination,
                layer.appearance.blend,
                x + i as u32,
                y,
                u64::from(layer.appearance.dissolve_seed),
            );
        }
    }
    Ok(output)
}

impl Document {
    /// Import is construction, not a stream of undo commands. No partial document escapes.
    pub fn from_import(
        width: u32,
        height: u32,
        layers: Vec<crate::ImportedLayer>,
        active: usize,
        options: DocumentOptions,
    ) -> Result<Self> {
        Self::from_import_with_storage(
            width,
            height,
            layers,
            active,
            options,
            paint_storage::ScratchSpace::system(),
        )
    }
    pub fn from_import_with_storage(
        width: u32,
        height: u32,
        layers: Vec<crate::ImportedLayer>,
        active: usize,
        options: DocumentOptions,
        storage: Arc<paint_storage::ScratchSpace>,
    ) -> Result<Self> {
        Self::from_import_with_appearance(width, height, layers, active, options, storage, None)
    }
    // Import keeps explicit resource/storage configuration; this additive entry
    // point also carries optional versioned layer metadata.
    #[allow(clippy::too_many_arguments)]
    pub fn from_import_with_appearance(
        width: u32,
        height: u32,
        layers: Vec<crate::ImportedLayer>,
        active: usize,
        options: DocumentOptions,
        storage: Arc<paint_storage::ScratchSpace>,
        appearances: Option<Vec<crate::LayerAppearance>>,
    ) -> Result<Self> {
        if appearances
            .as_ref()
            .is_some_and(|a| a.len() != layers.len())
        {
            return Err(Error::InvalidArgument("appearance count"));
        }
        let mut document = Self::with_storage(width, height, options, storage)?;
        if layers.is_empty() || layers.len() > 4096 || active >= layers.len() {
            return Err(Error::InvalidArgument(
                "invalid imported layer count or active layer",
            ));
        }
        document.layers.clear();
        let mut count = 0;
        for (index, imported) in layers.into_iter().enumerate() {
            if imported.name.is_empty()
                || imported.name.len() > 1024
                || imported.name.contains('\0')
                || !imported.properties.opacity.is_finite()
                || !(0.0..=1.0).contains(&imported.properties.opacity)
            {
                return Err(Error::InvalidArgument("invalid imported layer metadata"));
            }
            let mut layer = Layer::new(index as u64 + 1, imported.name);
            layer.properties = imported.properties;
            if let Some(a) = &appearances {
                a[index].validate()?;
                layer.appearance = a[index];
            }
            for (coord, tile) in imported.tiles {
                let x = u64::from(coord.x) * u64::from(TILE_SIZE);
                let y = u64::from(coord.y) * u64::from(TILE_SIZE);
                if appearances.is_none() && (x >= u64::from(width) || y >= u64::from(height)) {
                    return Err(Error::InvalidArgument("imported tile outside document"));
                }
                if coord.signed_x().unsigned_abs() > MAX_DIMENSION.div_ceil(64) * 2
                    || coord.signed_y().unsigned_abs() > MAX_DIMENSION.div_ceil(64) * 2
                {
                    return Err(Error::InvalidArgument("local tile coordinate limit"));
                }
                let pixels = tile.try_pixels()?;
                for row in 0..TILE_SIZE {
                    for col in 0..TILE_SIZE {
                        if appearances.is_none()
                            && (x + u64::from(col) >= u64::from(width)
                                || y + u64::from(row) >= u64::from(height))
                            && pixels[(row * TILE_SIZE + col) as usize].alpha() != 0.
                        {
                            return Err(Error::InvalidArgument("imported pixels outside document"));
                        }
                    }
                }
                {
                    count += 1;
                    if count > options.max_document_tiles {
                        return Err(Error::ResourceLimit("imported tile budget"));
                    }
                    drop(pixels);
                    let tile = Arc::new(tile);
                    document.pool.attach(&tile)?;
                    layer.tiles.insert(coord, tile);
                }
            }
            document.layers.push(layer);
        }
        document.active_layer = active as u64 + 1;
        document.next_layer_id = document.layers.len() as u64 + 1;
        Ok(document)
    }
    pub fn new(width: u32, height: u32) -> Result<Self> {
        Self::with_options(width, height, DocumentOptions::default())
    }

    pub fn with_options(width: u32, height: u32, options: DocumentOptions) -> Result<Self> {
        Self::with_storage(
            width,
            height,
            options,
            paint_storage::ScratchSpace::system(),
        )
    }
    pub fn with_storage(
        width: u32,
        height: u32,
        options: DocumentOptions,
        storage: Arc<paint_storage::ScratchSpace>,
    ) -> Result<Self> {
        if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err(Error::InvalidArgument("dimensions must be 1..=1,000,000"));
        }
        if options.max_document_tiles == 0
            || options.max_document_tiles > 1_000_000
            || options.max_resident_tiles == 0
            || options.max_resident_tiles > 16384
            || options.max_scratch_bytes < 65552
            || options.max_scratch_bytes > 64 * 1024 * 1024 * 1024
            || options.max_history_bytes < 2 * TILE_BYTES + 256
            || options.max_history_bytes as u64 > 4 * 1024 * 1024 * 1024_u64
            || options.max_history_commands == 0
        {
            return Err(Error::InvalidArgument("invalid document/history budget"));
        }
        Ok(Self {
            pool: crate::page_pool::PagePool::with_storage(
                options.max_resident_tiles,
                options.max_scratch_bytes,
                storage,
            ),
            width,
            height,
            layers: vec![Layer::new(1, "Layer 1".into())],
            active_layer: 1,
            next_layer_id: 2,
            options,
            history: History::default(),
            stroke: None,
            selection: crate::Selection::default(),
            revision: 0,
            dirty: DirtyTiles {
                all: true,
                tiles: BTreeSet::new(),
            },
        })
    }

    /// Construction-only background; O(1) even for million-pixel dimensions.
    pub fn initialize_white_background(&mut self) -> Result<()> {
        self.idle()?;
        if self.revision != 0 || self.layers.len() != 1 || self.tile_count() != 0 {
            return Err(Error::InvalidArgument("background requires new document"));
        }
        self.layers[0].name = "背景".into();
        self.layers[0].white = (self.width, self.height);
        Ok(())
    }
    pub fn document_options(&self) -> DocumentOptions {
        self.options
    }
    pub fn storage(&self) -> Arc<paint_storage::ScratchSpace> {
        self.pool.storage()
    }
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }
    pub fn layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }
    pub fn active_layer(&self) -> LayerId {
        self.active_layer
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn selection(&self) -> &crate::Selection {
        &self.selection
    }
    /// Apply an already-rasterized selection shape (polygon or wand region).
    pub fn set_selection_path(
        &mut self,
        shape: crate::SelectionShape,
        operation: crate::SelectionOperation,
        action: crate::HistoryAction,
    ) -> Result<()> {
        let (width, height) = (self.width, self.height);
        let next = self.selection.apply(shape, operation, width, height)?;
        self.set_selection_with_action(next, action)
    }
    /// Freehand lasso: rasterize the closed path and apply it.
    /// Magic wand: flood fill from the seed and apply the resulting region.
    pub fn set_selection_magic(&mut self, seed_x: u32, seed_y: u32, tolerance: u32) -> Result<()> {
        let tolerance = f64::from(tolerance) / 255.;
        let shape = crate::wand_shape(self, seed_x, seed_y, tolerance as f32)?;
        self.set_selection_path(
            shape,
            crate::SelectionOperation::Replace,
            crate::HistoryAction::MagicSelection,
        )
    }
    pub fn set_selection(&mut self, selection: crate::Selection) -> Result<()> {
        self.set_selection_with_action(selection, crate::HistoryAction::Selection)
    }
    pub fn set_selection_with_action(
        &mut self,
        selection: crate::Selection,
        action: crate::HistoryAction,
    ) -> Result<()> {
        self.idle()?;
        if self.selection == selection {
            return Ok(());
        }
        self.commit_with_action(
            Command::Selection {
                before: self.selection.clone(),
                after: selection.clone(),
            },
            action,
        )?;
        self.selection = selection;
        self.bump_revision();
        Ok(())
    }
    pub fn initialize_selection(&mut self, selection: crate::Selection) -> Result<()> {
        self.idle()?;
        if self.revision != 0 || self.history_depth() != (0, 0) {
            return Err(Error::InvalidArgument(
                "selection initialization after edits",
            ));
        }
        self.selection = selection;
        Ok(())
    }
    pub fn tile_count(&self) -> usize {
        self.layers.iter().map(Layer::tile_count).sum()
    }
    pub fn history_depth(&self) -> (usize, usize) {
        (self.history.undo.len(), self.history.redo.len())
    }
    pub fn history_actions(&self) -> Vec<crate::HistoryAction> {
        let mut actions = vec![if self.history.evicted {
            crate::HistoryAction::Truncated
        } else {
            crate::HistoryAction::Initial
        }];
        actions.extend(
            self.history
                .undo
                .iter()
                .chain(self.history.redo.iter().rev())
                .map(StoredCommand::action),
        );
        actions
    }
    pub fn history_bytes(&self) -> usize {
        self.history.bytes
    }
    pub fn history_disk_bytes(&self) -> u64 {
        self.history.disk_bytes
    }
    /// Retarget the per-document command limit. A smaller limit evicts the oldest undo records
    /// immediately and marks the original boundary unreachable; a larger limit only affects
    /// future commits. Rejected while a stroke is being recorded so history stays consistent.
    pub fn set_max_history_commands(&mut self, max_commands: usize) -> Result<()> {
        if max_commands == 0 {
            return Err(Error::InvalidArgument("history limit must be positive"));
        }
        if self.stroke.is_some() {
            return Err(Error::Busy);
        }
        self.options.max_history_commands = max_commands;
        self.history.apply_command_limit(max_commands);
        Ok(())
    }
    pub fn stroke_active(&self) -> bool {
        self.stroke.is_some()
    }
    pub fn last_input(&self) -> Option<InputPoint> {
        self.stroke.as_ref().map(|s| s.last)
    }

    pub fn take_dirty(&mut self) -> DirtyTiles {
        std::mem::take(&mut self.dirty)
    }

    pub fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            pool: Arc::clone(&self.pool),
            width: self.width,
            height: self.height,
            revision: self.revision,
            layers: self.layers.clone(),
            selection: self.selection.clone(),
        }
    }

    /// Idle-only raw-content preview. Clone just this layer's tile references,
    /// never the document's other layers or any pixel buffer.
    pub fn layer_preview(&self, id: LayerId) -> Result<DocumentSnapshot> {
        self.idle()?;
        let mut layer = self.layers[self.layer_index(id)?].clone();
        layer.properties = LayerProperties::default();
        layer.appearance.fill = 1.;
        layer.appearance.blend = crate::BlendMode::Normal;
        layer.clipped = false;
        let range = self.subtree(id)?;
        // Raw content and its mask have independent thumbnails on the same UI row.
        // Group previews still compose descendants, including their masks.
        let mut layers = if layer.group {
            self.layers[range].to_vec()
        } else {
            vec![layer.clone()]
        };
        if layer.mask {
            layer.mask = false;
            layer.white = (self.width, self.height);
            for tile in layer.tiles.values_mut() {
                let pixels = tile
                    .try_pixels()?
                    .iter()
                    .map(|p| {
                        let gray = 1. - p.alpha();
                        let value = if gray <= 0.04045 {
                            gray / 12.92
                        } else {
                            ((gray + 0.055) / 1.055).powf(2.4)
                        };
                        Pixel::from_straight([value, value, value, 1.]).expect("mask preview")
                    })
                    .collect();
                *tile = Arc::new(Tile::from_pixels(pixels)?);
                self.pool.attach(tile)?;
            }
        }
        layer.parent = 0;
        *layers.last_mut().expect("preview root") = layer;
        Ok(DocumentSnapshot {
            selection: crate::Selection::default(),
            pool: Arc::clone(&self.pool),
            width: self.width,
            height: self.height,
            revision: self.revision,
            layers,
        })
    }

    /// Convenience reference sampler; use try_pixel for fallible storage.
    pub fn pixel(&self, x: u32, y: u32) -> Pixel {
        self.try_pixel(x, y)
            .expect("document read failed; use try_pixel")
    }
    pub fn try_pixel(&self, x: u32, y: u32) -> Result<Pixel> {
        if x >= self.width || y >= self.height {
            return Ok(Pixel::TRANSPARENT);
        }
        composite(&self.layers, x, y)
    }
    pub fn read_row(&self, x: u32, y: u32, count: u32) -> Result<Vec<Pixel>> {
        if y >= self.height || x.checked_add(count).is_none_or(|end| end > self.width) {
            return Err(Error::InvalidArgument("row outside document"));
        }
        composite_row(&self.layers, x, y, count)
    }
    pub fn storage_stats(&self) -> crate::StorageStats {
        self.pool.stats()
    }
    pub fn trim_storage(&self) -> Result<()> {
        self.pool.trim()
    }

    fn idle(&self) -> Result<()> {
        if self.stroke.is_some() {
            Err(Error::Busy)
        } else {
            Ok(())
        }
    }

    fn layer_index(&self, id: LayerId) -> Result<usize> {
        self.layers
            .iter()
            .position(|l| l.id == id)
            .ok_or(Error::LayerNotFound(id))
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    fn dirty_all(&mut self) {
        self.dirty.all = true;
        self.dirty.tiles.clear();
        self.bump_revision();
    }

    fn dirty_tile(&mut self, coord: TileCoord) {
        if !self.dirty.all {
            self.dirty.tiles.insert(coord);
        }
    }

    fn commit(&mut self, command: Command) -> Result<()> {
        let action = command.action();
        self.commit_with_action(command, action)
    }
    fn commit_with_action(&mut self, command: Command, action: crate::HistoryAction) -> Result<()> {
        let stored = StoredCommand::prepare_in(
            &command,
            self.options.max_history_bytes,
            self.pool.storage(),
        )?;
        self.history.commit(
            stored.with_action(action),
            self.options.max_history_bytes,
            self.options.max_history_commands,
        );
        Ok(())
    }

    pub fn set_active_layer(&mut self, id: LayerId) -> Result<()> {
        self.idle()?;
        self.layer_index(id)?;
        self.active_layer = id;
        Ok(())
    }

    pub fn add_layer(&mut self, name: impl Into<String>) -> Result<LayerId> {
        self.idle()?;
        let name = name.into();
        if name.is_empty() || name.len() > 1024 || name.contains('\0') {
            return Err(Error::InvalidArgument(
                "layer name must contain 1..=1024 UTF-8 bytes without NUL",
            ));
        }
        if self.layers.len() >= 4096 {
            return Err(Error::ResourceLimit("layer count"));
        }
        let id = self.next_layer_id;
        let next = id.checked_add(1).ok_or(Error::ResourceLimit("layer IDs"))?;
        let mut layer = Layer::new(id, name);
        let mut selected = self.layer_index(self.active_layer)?;
        if self.layers[selected].mask {
            selected = self.layer_index(self.layers[selected].parent)?;
        }
        let index = if self.layers[selected].group {
            selected
        } else {
            selected + 1
        };
        layer.parent = if self.layers[selected].group {
            self.layers[selected].id
        } else {
            self.layers[selected].parent
        };
        self.check_ancestors(layer.parent, crate::LOCK_ALL)?;
        if layer.parent != 0 && self.layer_hierarchy(layer.parent)?.1 >= 16 {
            return Err(Error::ResourceLimit("group nesting exceeds 16"));
        }
        self.commit(Command::AddLayer {
            index,
            layer: layer.clone(),
        })?;
        self.layers.insert(index, layer);
        self.next_layer_id = next;
        self.active_layer = id;
        self.dirty_all();
        Ok(id)
    }

    pub fn remove_layer(&mut self, id: LayerId) -> Result<()> {
        self.idle()?;
        let index = self.layer_index(id)?;
        if self.layers[index].group || self.mask_for(id).is_some() {
            return self.remove_group(id);
        }
        if !self.layers[index].mask
            && self.layers.iter().filter(|l| !l.group && !l.mask).count() == 1
        {
            return Err(Error::LastLayer);
        }
        self.check_ancestors(id, crate::LOCK_ALL)?;
        let command = Command::RemoveLayer {
            index,
            layer: self.layers[index].clone(),
        };
        self.commit(command)?;
        self.layers.remove(index);
        self.ensure_active_layer();
        self.dirty_all();
        Ok(())
    }

    pub fn set_layer_properties(&mut self, id: LayerId, after: LayerProperties) -> Result<()> {
        self.idle()?;
        if !after.opacity.is_finite() || !(0.0..=1.0).contains(&after.opacity) {
            return Err(Error::InvalidArgument(
                "layer opacity must be finite in 0..=1",
            ));
        }
        let index = self.layer_index(id)?;
        let before = self.layers[index].properties;
        if after.opacity != before.opacity {
            self.check_ancestors(id, crate::LOCK_ALL)?;
        }
        if before == after {
            return Ok(());
        }
        self.commit(Command::Properties {
            layer: id,
            before,
            after,
        })?;
        self.layers[index].properties = after;
        self.dirty_all();
        Ok(())
    }

    pub fn set_layer_appearance(
        &mut self,
        id: LayerId,
        after: crate::LayerAppearance,
    ) -> Result<()> {
        self.idle()?;
        after.validate()?;
        let index = self.layer_index(id)?;
        let before = self.layers[index].appearance;
        self.check_ancestors(self.layers[index].parent, crate::LOCK_ALL)?;
        if self.layers[index].group
            && (after.locks & crate::LOCK_TRANSPARENCY != 0
                || after.offset_x != 0
                || after.offset_y != 0)
        {
            return Err(Error::InvalidArgument(
                "groups have no transparent pixels or local offset",
            ));
        }
        // Unlock/visibility stay usable. All other edits to a fully locked layer fail.
        if before.locks & crate::LOCK_ALL != 0
            && (after.fill != before.fill
                || after.blend != before.blend
                || after.dissolve_seed != before.dissolve_seed)
        {
            return Err(Error::LayerLocked);
        }
        if (after.offset_x, after.offset_y) != (before.offset_x, before.offset_y) {
            self.check_ancestors(id, crate::LOCK_POSITION | crate::LOCK_ALL)?;
            before.translated(
                after.offset_x - before.offset_x,
                after.offset_y - before.offset_y,
            )?;
        }
        if before == after {
            return Ok(());
        }
        self.commit(Command::Appearance {
            layer: id,
            before,
            after,
        })?;
        self.layers[index].appearance = after;
        self.dirty_all();
        Ok(())
    }
    pub fn move_layer(&mut self, id: LayerId, dx: i32, dy: i32) -> Result<()> {
        self.idle()?;
        if self.layer(id).ok_or(Error::LayerNotFound(id))?.group || self.mask_for(id).is_some() {
            return self.move_group(id, dx, dy);
        }
        let before = self.layer(id).ok_or(Error::LayerNotFound(id))?.appearance;
        self.set_layer_appearance(id, before.translated(dx, dy)?)
    }
    fn ensure_active_layer(&mut self) {
        if self.layer(self.active_layer).is_none() {
            self.active_layer = self.layers.last().expect("at least one layer").id;
        }
    }

    pub fn begin_stroke(&mut self, brush: Brush, first: InputPoint) -> Result<()> {
        self.idle()?;
        brush.validate()?;
        first.validate()?;
        if self.layer(self.active_layer).expect("active layer").group {
            return Err(Error::InvalidArgument("select a pixel layer to paint"));
        }
        self.check_ancestors(self.active_layer, crate::LOCK_ALL)?;
        let mut stroke = Stroke {
            layer: self.active_layer,
            brush,
            last: first,
            last_dab: first,
            distance_to_next: brush.step(),
            before: BTreeMap::new(),
            mask_paint: if self.layer(self.active_layer).expect("active").mask {
                let c = brush.color.components();
                let a = c[3];
                let encode = |v: f32| {
                    if v >= 1. {
                        1.
                    } else if v <= 0.0031308 {
                        v * 12.92
                    } else {
                        1.055 * v.powf(1. / 2.4) - 0.055
                    }
                };
                // Mask values are coverage, not scene colors. UI 50% gray must mean 50% coverage.
                let gray = if a > 0. {
                    encode(c[1] / a)
                        + 0.2126 * (encode(c[0] / a) - encode(c[1] / a))
                        + 0.0722 * (encode(c[2] / a) - encode(c[1] / a))
                } else {
                    1.
                };
                Some(((1. - gray).clamp(0., 1.), a))
            } else {
                None
            },
        };
        match self.dab(&mut stroke, first) {
            Ok(changed) => {
                if changed {
                    self.bump_revision();
                }
                self.stroke = Some(stroke);
                Ok(())
            }
            Err(error) => {
                self.restore_stroke(&stroke);
                Err(error)
            }
        }
    }

    pub fn stroke_to(&mut self, point: InputPoint) -> Result<()> {
        point.validate()?;
        let current = self.stroke.as_ref().ok_or(Error::NoStroke)?;
        if point.timestamp_ns < current.last.timestamp_ns {
            return Err(Error::InvalidArgument("tablet time must be monotonic"));
        }
        let distance = (point.x - current.last.x).hypot(point.y - current.last.y);
        let step = current.brush.step();
        let count = if distance + 1e-9 < current.distance_to_next {
            0
        } else {
            ((distance - current.distance_to_next + 1e-9) / step).floor() as usize + 1
        };
        let diameter = (f64::from(current.brush.radius) * 2.0 + 2.0).ceil() as usize;
        if count > MAX_DABS_PER_SEGMENT
            || count.saturating_mul(diameter * diameter) > MAX_PIXEL_VISITS_PER_SEGMENT
        {
            // No mutation: a caller may resample a long device segment or cancel the stroke.
            return Err(Error::ResourceLimit(
                "input segment too large; resample into shorter segments",
            ));
        }
        let mut stroke = self.stroke.take().expect("checked active stroke");
        let mut position = stroke.distance_to_next;
        let mut changed = false;
        for _ in 0..count {
            let sample = stroke
                .last
                .interpolate(point, (position / distance).min(1.0));
            match self.dab(&mut stroke, sample) {
                Ok(dab_changed) => changed |= dab_changed,
                Err(error) => {
                    self.restore_stroke(&stroke);
                    return Err(error);
                }
            }
            stroke.last_dab = sample;
            position += step;
        }
        stroke.distance_to_next = (position - distance).max(step * 1e-9);
        stroke.last = point;
        if changed {
            self.bump_revision();
        }
        self.stroke = Some(stroke);
        Ok(())
    }

    pub fn end_stroke(&mut self) -> Result<bool> {
        let mut stroke = self.stroke.take().ok_or(Error::NoStroke)?;
        if (stroke.last.x - stroke.last_dab.x).hypot(stroke.last.y - stroke.last_dab.y) > 1e-7 {
            let last = stroke.last;
            match self.dab(&mut stroke, last) {
                Ok(changed) => {
                    if changed {
                        self.bump_revision();
                    }
                }
                Err(error) => {
                    self.restore_stroke(&stroke);
                    return Err(error);
                }
            }
        }
        let index = self.layer_index(stroke.layer)?;
        let mut changes = Vec::new();
        let result = (|| -> Result<()> {
            for (coord, before) in &stroke.before {
                if self.layers[index]
                    .tiles
                    .get(coord)
                    .is_some_and(|tile| tile.is_empty())
                    && self.layers[index].white == (0, 0)
                {
                    self.layers[index].tiles.remove(coord);
                }
                let after = self.layers[index].tiles.get(coord).cloned();
                let equal = match (before, &after) {
                    (None, None) => true,
                    (Some(a), Some(b)) if Arc::ptr_eq(a, b) => true,
                    (Some(a), Some(b)) => a.try_pixels()?.as_ref() == b.try_pixels()?.as_ref(),
                    _ => false,
                };
                if !equal {
                    changes.push(TileChange {
                        coord: *coord,
                        before: before.clone(),
                        after,
                    });
                }
            }
            self.pool.trim()?;
            Ok(())
        })();
        if let Err(error) = result {
            self.restore_stroke(&stroke);
            return Err(error);
        }
        if changes.is_empty() {
            return Ok(false);
        }
        let command = Command::Stroke {
            layer: stroke.layer,
            changes,
        };
        let action = if stroke.brush.mode == BrushMode::Erase {
            crate::HistoryAction::Eraser
        } else {
            crate::HistoryAction::Brush
        };
        if let Err(error) = self.commit_with_action(command.clone(), action) {
            // History preparation failed before eviction. Restore all touched
            // tiles, retain the old history, and leave no half-committed stroke.
            self.apply_history(&command, false);
            return Err(error);
        }
        Ok(true)
    }

    pub fn cancel_stroke(&mut self) -> Result<()> {
        let stroke = self.stroke.take().ok_or(Error::NoStroke)?;
        self.restore_stroke(&stroke);
        Ok(())
    }

    fn restore_stroke(&mut self, stroke: &Stroke) {
        let index = self
            .layer_index(stroke.layer)
            .expect("layers cannot change during strokes");
        for (coord, tile) in &stroke.before {
            Self::restore_tile(&mut self.layers[index], *coord, tile.clone());
            if self.layers[index].appearance.offset_x != 0
                || self.layers[index].appearance.offset_y != 0
            {
                self.dirty.all = true;
                self.dirty.tiles.clear();
            } else {
                self.dirty_tile(*coord);
            }
        }
        if !stroke.before.is_empty() {
            self.bump_revision();
        }
    }

    fn restore_tile(layer: &mut Layer, coord: TileCoord, tile: Option<Arc<Tile>>) {
        if let Some(tile) = tile {
            layer.tiles.insert(coord, tile);
        } else {
            layer.tiles.remove(&coord);
        }
    }

    fn dab(&mut self, stroke: &mut Stroke, point: InputPoint) -> Result<bool> {
        // Specialize once per dab: an inactive selection adds no pixel-loop
        // geometry branches or coverage multiplication to the original hot path.
        if self.selection.enabled() {
            self.dab_with_selection::<true>(stroke, point)
        } else {
            self.dab_with_selection::<false>(stroke, point)
        }
    }

    fn dab_with_selection<const SELECTED: bool>(
        &mut self,
        stroke: &mut Stroke,
        point: InputPoint,
    ) -> Result<bool> {
        let radius = f64::from(stroke.brush.radius * point.pressure);
        let flow = stroke.brush.opacity * point.pressure;
        if radius == 0.0 || flow == 0.0 {
            return Ok(false);
        }
        let x0 = ((point.x - radius - 1.0).floor() as i64).clamp(0, i64::from(self.width));
        let y0 = ((point.y - radius - 1.0).floor() as i64).clamp(0, i64::from(self.height));
        let x1 = ((point.x + radius + 1.0).ceil() as i64).clamp(0, i64::from(self.width));
        let y1 = ((point.y + radius + 1.0).ceil() as i64).clamp(0, i64::from(self.height));
        let index = self.layer_index(stroke.layer)?;
        let appearance = self.layers[index].appearance;
        let mask = self.layers[index].mask;
        let alpha_locked = !mask && appearance.locks & crate::LOCK_TRANSPARENCY != 0;
        let erase = stroke.brush.mode == BrushMode::Erase || point.tool == Tool::Eraser;
        let mut changed = false;
        if x0 >= x1 || y0 >= y1 {
            return Ok(false);
        }
        let (x0, x1) = (
            x0 - i64::from(appearance.offset_x),
            x1 - i64::from(appearance.offset_x),
        );
        let (y0, y1) = (
            y0 - i64::from(appearance.offset_y),
            y1 - i64::from(appearance.offset_y),
        );
        let (px, py) = (
            point.x - f64::from(appearance.offset_x),
            point.y - f64::from(appearance.offset_y),
        );
        for ty in y0.div_euclid(64)..=(y1 - 1).div_euclid(64) {
            for tx in x0.div_euclid(64)..=(x1 - 1).div_euclid(64) {
                let coord = TileCoord::from_signed(tx as i32, ty as i32);
                let source = self.layers[index]
                    .tiles
                    .get(&coord)
                    .map(|t| t.try_pixels())
                    .transpose()?;
                if alpha_locked
                    && (erase || (source.is_none() && self.layers[index].white == (0, 0)))
                {
                    continue;
                }
                let private = stroke.before.contains_key(&coord)
                    && self.layers[index]
                        .tiles
                        .get(&coord)
                        .is_some_and(Tile::can_edit_private);
                let mut updates = Vec::new();
                let mut pixels: Option<Vec<Pixel>> = None;
                for y in y0.max(ty * 64)..y1.min((ty + 1) * 64) {
                    for x in x0.max(tx * 64)..x1.min((tx + 1) * 64) {
                        let selected = if SELECTED {
                            self.selection.coverage(
                                (x + i64::from(appearance.offset_x)) as f64 + 0.5,
                                (y + i64::from(appearance.offset_y)) as f64 + 0.5,
                            )
                        } else {
                            1.
                        };
                        if selected == 0. {
                            continue;
                        }
                        let distance = (x as f64 + 0.5 - px).hypot(y as f64 + 0.5 - py);
                        let coverage = (radius + 0.5 - distance).clamp(0., 1.) as f32;
                        if coverage == 0. {
                            continue;
                        }
                        let offset = (y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize;
                        let before = source
                            .as_ref()
                            .map_or(self.layers[index].default_pixel(x, y), |p| p[offset]);
                        let amount = if SELECTED {
                            coverage * flow * selected
                        } else {
                            coverage * flow
                        };
                        let after = if mask {
                            let (tone, a) = stroke.mask_paint.expect("mask brush target");
                            let hide = if erase { 0. } else { tone };
                            let weight = amount * if erase { 1. } else { a };
                            Pixel::from_straight([
                                0.,
                                0.,
                                0.,
                                before.alpha() * (1. - weight) + hide * weight,
                            ])?
                        } else if alpha_locked {
                            before.paint_preserving_alpha(stroke.brush.color, amount)
                        } else if erase {
                            before.scaled(1. - amount)
                        } else {
                            stroke.brush.color.scaled(amount).over(before)
                        };
                        if before == after {
                            continue;
                        }
                        if private {
                            updates.push((offset, after));
                        } else {
                            pixels.get_or_insert_with(|| {
                                source.as_ref().map_or_else(
                                    || {
                                        (0..4096)
                                            .map(|i| {
                                                self.layers[index].default_pixel(
                                                    tx * 64 + i % 64,
                                                    ty * 64 + i / 64,
                                                )
                                            })
                                            .collect()
                                    },
                                    |p| p.to_vec(),
                                )
                            })[offset] = after;
                        }
                    }
                }
                drop(source);
                if !updates.is_empty() {
                    Tile::edit_private(
                        self.layers[index]
                            .tiles
                            .get_mut(&coord)
                            .expect("private tile exists"),
                        &self.pool,
                        &updates,
                    )?;
                    if appearance.offset_x == 0 && appearance.offset_y == 0 {
                        self.dirty_tile(coord);
                    } else {
                        self.dirty.all = true;
                        self.dirty.tiles.clear();
                    }
                    changed = true;
                }
                if let Some(pixels) = pixels {
                    if !self.layers[index].tiles.contains_key(&coord)
                        && self.tile_count() >= self.options.max_document_tiles
                    {
                        return Err(Error::ResourceLimit(
                            "logical document tile limit; entire stroke rolled back",
                        ));
                    }
                    stroke
                        .before
                        .entry(coord)
                        .or_insert_with(|| self.layers[index].tiles.get(&coord).cloned());
                    let tile = Arc::new(Tile::from_pixels(pixels)?);
                    self.pool.attach(&tile)?;
                    self.layers[index].tiles.insert(coord, tile);
                    if appearance.offset_x == 0 && appearance.offset_y == 0 {
                        self.dirty_tile(coord);
                    } else {
                        self.dirty.all = true;
                        self.dirty.tiles.clear();
                    }
                    changed = true;
                }
                self.pool.trim()?;
            }
        }
        Ok(changed)
    }

    pub fn undo(&mut self) -> Result<bool> {
        self.idle()?;
        let Some(stored) = self.history.undo.back() else {
            return Ok(false);
        };
        // Read and verify every byte before changing either pixels or the stack.
        let command = stored.materialize_in(Some(&self.pool))?;
        let stored = self.history.undo.pop_back().expect("checked history");
        self.apply_history(&command, false);
        self.history.redo.push(stored);
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool> {
        self.idle()?;
        let Some(stored) = self.history.redo.last() else {
            return Ok(false);
        };
        let command = stored.materialize_in(Some(&self.pool))?;
        let stored = self.history.redo.pop().expect("checked history");
        self.apply_history(&command, true);
        self.history.undo.push_back(stored);
        Ok(true)
    }

    fn apply_history(&mut self, command: &Command, forward: bool) {
        match command {
            Command::Selection { before, after } => {
                self.selection = if forward {
                    after.clone()
                } else {
                    before.clone()
                };
                self.bump_revision();
            }
            Command::Structure { before, after } => {
                self.apply_structure(if forward { after } else { before });
                self.dirty_all();
            }
            Command::RemoveTree { index, layers } => {
                if forward {
                    self.layers.drain(*index..index + layers.len());
                } else {
                    self.layers.splice(*index..*index, layers.iter().cloned());
                }
                self.dirty_all();
            }
            Command::Stroke { layer, changes } => {
                let index = self.layer_index(*layer).expect("history layer dependency");
                for change in changes {
                    Self::restore_tile(
                        &mut self.layers[index],
                        change.coord,
                        if forward {
                            change.after.clone()
                        } else {
                            change.before.clone()
                        },
                    );
                    if self.layers[index].appearance.offset_x != 0
                        || self.layers[index].appearance.offset_y != 0
                    {
                        self.dirty.all = true;
                        self.dirty.tiles.clear();
                    } else {
                        self.dirty_tile(change.coord);
                    }
                }
                self.bump_revision();
            }
            Command::AddLayer { index, layer } => {
                if forward {
                    self.layers.insert(*index, layer.clone());
                } else {
                    self.layers.remove(*index);
                }
                self.dirty_all();
            }
            Command::RemoveLayer { index, layer } => {
                if forward {
                    self.layers.remove(*index);
                } else {
                    self.layers.insert(*index, layer.clone());
                }
                self.dirty_all();
            }
            Command::Appearance {
                layer,
                before,
                after,
            } => {
                let index = self.layer_index(*layer).expect("history dependency");
                self.layers[index].appearance = if forward { *after } else { *before };
                self.dirty_all();
            }
            Command::Properties {
                layer,
                before,
                after,
            } => {
                let index = self.layer_index(*layer).expect("history layer dependency");
                self.layers[index].properties = if forward { *after } else { *before };
                self.dirty_all();
            }
        }
        self.ensure_active_layer();
    }
}

#[cfg(test)]
mod paging_failures {
    use super::*;
    #[test]
    fn corrupt_inline_history_page_does_not_change_pixels_revision_or_stack() {
        let tiles = (0..2)
            .map(|x| {
                let pixels = (0..4096)
                    .map(|i| Pixel::from_straight([i as f32 / 4096., 0.3, 0.4, 1.]).unwrap())
                    .collect();
                (TileCoord { x, y: 0 }, Tile::from_pixels(pixels).unwrap())
            })
            .collect();
        let mut doc = Document::from_import(
            128,
            64,
            vec![crate::ImportedLayer {
                name: "noise".into(),
                properties: LayerProperties::default(),
                tiles,
            }],
            0,
            DocumentOptions {
                max_resident_tiles: 1,
                ..Default::default()
            },
        )
        .unwrap();
        doc.begin_stroke(Brush::default(), InputPoint::new(16., 16., 1.))
            .unwrap();
        doc.end_stroke().unwrap();
        assert!(matches!(
            doc.history.undo.back(),
            Some(StoredCommand::Inline(..))
        ));
        let pixel = doc.try_pixel(16, 16).unwrap();
        let revision = doc.revision();
        let depth = doc.history_depth();
        doc.pool.corrupt_for_test();
        assert!(matches!(doc.undo(), Err(Error::Storage(_))));
        assert_eq!(doc.try_pixel(16, 16).unwrap(), pixel);
        assert_eq!(doc.revision(), revision);
        assert_eq!(doc.history_depth(), depth);
    }
    #[test]
    fn scratch_write_failure_restores_whole_stroke_and_prior_history() {
        let mut doc = Document::with_options(
            256,
            256,
            DocumentOptions {
                max_resident_tiles: 1,
                ..Default::default()
            },
        )
        .unwrap();
        doc.set_layer_properties(
            1,
            LayerProperties {
                opacity: 0.8,
                visible: true,
            },
        )
        .unwrap();
        let history = doc.history_depth();
        crate::page_pool::fail_writes_for_test(true);
        let result = doc.begin_stroke(
            Brush {
                radius: 128.,
                ..Default::default()
            },
            InputPoint::new(128., 128., 1.),
        );
        crate::page_pool::fail_writes_for_test(false);
        assert!(matches!(result, Err(Error::Storage(_))));
        assert_eq!(doc.tile_count(), 0);
        assert_eq!(doc.history_depth(), history);
        assert!(!doc.stroke_active());
        assert_eq!(doc.storage_stats().resident_tiles, 0);
        doc.begin_stroke(Brush::default(), InputPoint::new(20., 20., 1.))
            .unwrap();
        doc.end_stroke().unwrap();
        assert_eq!(doc.history_depth().0, history.0 + 1);
    }
}
