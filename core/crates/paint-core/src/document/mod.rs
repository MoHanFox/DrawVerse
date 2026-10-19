use crate::{
    history::{Command, History, StoredCommand, TileChange},
    Brush, BrushDefinition, BrushMode, BrushProgram, Error, InputPoint, Layer, LayerId,
    LayerProperties, Pixel, Result, Tile, TileCoord, Tool, TILE_BYTES, TILE_SIZE,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub const MAX_DIMENSION: u32 = 1_000_000;
const MAX_DABS_PER_SEGMENT: usize = crate::MAX_INPUT_PAGE_DABS;
const MAX_PIXEL_VISITS_PER_SEGMENT: usize = crate::MAX_INPUT_PAGE_VISITS;
mod batch;
#[path = "stroke/document_input_paging.rs"]
mod input_paging;
pub use input_paging::StrokePageResult;
#[path = "stroke/brush_dual_paint.rs"]
mod dual_paint;
pub(crate) mod groups;
mod layer_actions;
#[path = "stroke/document_outline.rs"]
mod outline;
#[path = "stroke/document_taper.rs"]
mod taper;
mod transform;
pub use transform::{PlacedImage, TransformGeometry};

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
    fn wet_history_failure_restores_pixels_and_keeps_external_carrier() {
        for colored in [false, true] {
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
            let depth = doc.history_depth();
            let mut definition = BrushDefinition {
                brush: Brush {
                    radius: 128.,
                    opacity: 0.4,
                    ..Default::default()
                },
                tip: crate::BrushTip {
                    pressure_size: false,
                    pressure_flow: false,
                    ..Default::default()
                },
                ..Default::default()
            };
            let carrier = crate::WetCarrier::new(
                crate::WetMix::new(0., 0., 0., 1., 1., 0.).unwrap(),
                256.,
                definition.brush.color,
            )
            .unwrap();
            if colored {
                definition.dynamics.color =
                    crate::ColorDynamics::new(true, 0.4, 0.6, 0.2, 0.2, true, u64::MAX).unwrap();
            }
            doc.begin_wet_brush_stroke(definition, InputPoint::new(96., 64., 1.), &carrier, false)
                .unwrap();
            crate::history_storage::fail_writes_for_test(true);
            let result = doc.end_wet_brush_stroke();
            crate::history_storage::fail_writes_for_test(false);
            assert!(matches!(result, Err(Error::HistoryStorage(_))));
            assert_eq!(pixels(&doc), original);
            assert_eq!(doc.history_depth(), depth);
            assert!(!doc.stroke_active());
            assert_eq!(carrier.revision(), 0);
            doc.undo().unwrap();
            assert_eq!(doc.layer(1).unwrap().properties().opacity, 1.);
        }
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
    fn liquify_history_failure_keeps_cancelable_preview_and_backed_undo_is_exact() {
        let mut doc = document();
        let original = pixels(&doc);
        let depth = doc.history_depth();
        let dab = crate::LiquifyDab {
            tool: 0,
            radius: 100.,
            strength: 0.5,
            density: 1.,
            speed: 1.,
            pressure_enabled: false,
            from: InputPoint::new(80., 64., 1.),
            to: InputPoint::new(104., 64., 1.),
            elapsed_ms: 16.,
        };
        doc.begin_liquify().unwrap();
        doc.liquify_dab(dab).unwrap();
        let preview = pixels(&doc);
        assert_ne!(preview, original);
        crate::history_storage::fail_writes_for_test(true);
        let result = doc.commit_liquify();
        crate::history_storage::fail_writes_for_test(false);
        assert!(matches!(result, Err(Error::HistoryStorage(_))));
        assert!(doc.liquify_active());
        assert_eq!(pixels(&doc), preview);
        assert_eq!(doc.history_depth(), depth);
        doc.cancel_liquify().unwrap();
        assert_eq!(pixels(&doc), original);
        doc.begin_liquify().unwrap();
        doc.liquify_dab(dab).unwrap();
        doc.commit_liquify().unwrap();
        let final_pixels = pixels(&doc);
        assert!(doc.history_disk_bytes() > 0);
        doc.undo().unwrap();
        assert_eq!(pixels(&doc), original);
        doc.redo().unwrap();
        assert_eq!(pixels(&doc), final_pixels);
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
        if blend == crate::BlendMode::PassThrough && !layer.group {
            return Err(Error::InvalidArgument("pass through requires a group"));
        }
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
    program: BrushProgram,
    last: InputPoint,
    path_filter: Option<crate::brush_smoothing::PathFilter>,
    last_dab: InputPoint,
    last_carrier: InputPoint,
    random_index: u64,
    random_stroke: u64,
    random_child: u32,
    dual_next: f64,
    dual_index: u64,
    dual_last: InputPoint,
    airbrush_last_ns: u64,
    airbrush_tick: bool,
    speed: f32,
    fade_index: u32,
    dual_coverage: BTreeMap<TileCoord, Arc<Tile>>,
    stamp_field: BTreeMap<TileCoord, Arc<Tile>>,
    wet_primary_field: BTreeMap<TileCoord, Arc<Tile>>,
    wet_secondary_field: BTreeMap<TileCoord, Arc<Tile>>,
    shape_sample: crate::brush_shape_jitter::ShapeSample,
    transfer_sample: crate::TransferSample,
    color_sample: Pixel,
    wet: Option<wet::WetStroke>,
    distance_to_next: f64,
    direction: Option<f64>,
    before: BTreeMap<TileCoord, Option<Arc<Tile>>>,
    coverage: BTreeMap<TileCoord, Arc<Tile>>,
    mask_paint: Option<(f32, f32)>,
    first_dab: bool,
    coverage_erase: bool,
    coverage_tool_stable: bool,
    taper: Option<taper::TailReplay>,
    outline: Option<outline::OutlineStroke>,
    taper_factors: crate::TaperFactors,
    path_distance: f64,
    dab_distance: f64,
}

#[derive(Debug)]
pub struct Document {
    compute: Arc<dyn paint_ops::Executor>,
    pool: Arc<crate::page_pool::PagePool>,
    width: u32,
    height: u32,
    layers: Vec<Layer>,
    active_layer: LayerId,
    next_layer_id: LayerId,
    options: DocumentOptions,
    history: History,
    stroke: Option<Stroke>,
    pending_input: Option<input_paging::PendingInput>,
    random_serial: u64,
    #[cfg(test)]
    reference_mix_path: bool,
    #[cfg(test)]
    reference_color_tiles: bool,
    #[cfg(test)]
    reference_wet_fields: bool,
    #[cfg(test)]
    taper_reference_endpoint: Option<f64>,
    #[cfg(test)]
    stagnant_mix_skips: usize,
    liquify: Option<liquify::LiquifyState>,
    transform: Option<transform::TransformState>,
    layer_move: Option<layer_move::LayerMoveState>,
    revision: u64,
    dirty: DirtyTiles,
    selection: crate::Selection,
    selection_cache: crate::selection::SelectionCoverageCache,
}

#[path = "tools/bucket.rs"]
mod bucket;
#[path = "tools/color_region.rs"]
mod color_region;
pub use bucket::BucketFill;
#[path = "tools/wand.rs"]
mod wand;
pub use wand::WandSelection;
#[path = "tools/layer_move.rs"]
mod layer_move;
#[path = "tools/liquify.rs"]
mod liquify;
#[path = "tools/smudge.rs"]
mod smudge;
#[path = "stroke/document_wet.rs"]
mod wet;
pub use liquify::{LiquifyDab, LiquifyPreview, LiquifyPreviewSource};
/// Shared resident presentation packet; original names remain source-compatible.
pub type ResidentPreview = LiquifyPreview;
pub type ResidentPreviewSource = LiquifyPreviewSource;

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
        if layers.len() > 4096
            || (layers.is_empty() && active != 0)
            || (!layers.is_empty() && active >= layers.len())
        {
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
        document.active_layer = if document.layers.is_empty() {
            0
        } else {
            active as u64 + 1
        };
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
            compute: Arc::new(paint_ops::Cpu),
            width,
            height,
            layers: vec![Layer::new(1, "Layer 1".into())],
            active_layer: 1,
            next_layer_id: 2,
            options,
            history: History::default(),
            stroke: None,
            pending_input: None,
            random_serial: 0,
            #[cfg(test)]
            reference_mix_path: false,
            #[cfg(test)]
            reference_color_tiles: false,
            #[cfg(test)]
            reference_wet_fields: false,
            #[cfg(test)]
            taper_reference_endpoint: None,
            #[cfg(test)]
            stagnant_mix_skips: 0,
            liquify: None,
            transform: None,
            layer_move: None,
            selection: crate::Selection::default(),
            selection_cache: Default::default(),
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
        self.layers[0].name = "鑳屾櫙".into();
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
    /// Apply an already-rasterized selection shape.
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
        self.selection_cache.clear();
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
        self.selection_cache.clear();
        Ok(())
    }
    pub fn tile_count(&self) -> usize {
        self.layers.iter().map(Layer::retained_tile_count).sum()
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
        if self.stroke.is_some()
            || self.liquify.is_some()
            || self.transform.is_some()
            || self.layer_move.is_some()
        {
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
        let index = if self.layers.is_empty() {
            0
        } else {
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
            if !self.layers[selected].group {
                layer.clipped = self.layers[selected].clipped
                    || self.layers[index..]
                        .iter()
                        .find(|node| !node.mask && node.parent == layer.parent)
                        .is_some_and(|node| node.clipped);
            }
            index
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
        if after.blend == crate::BlendMode::PassThrough && !self.layers[index].group {
            return Err(Error::InvalidArgument("pass through requires a group"));
        }
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
    /// Pick visible content, then optionally translate it in one normal history command.
    /// Empty canvas is a no-op. This runs on the document actor, never the UI thread.
    pub fn pick_layer(&mut self, x: u32, y: u32, dx: i32, dy: i32) -> Result<Option<LayerId>> {
        self.idle()?;
        if x >= self.width || y >= self.height {
            return Ok(None);
        }
        let mut picked = None;
        for (i, layer) in self.layers.iter().enumerate().rev() {
            if layer.group || layer.mask || !groups::visible(&self.layers, layer) {
                continue;
            }
            if self.pick_coverage(i, x, y)? > 0. {
                picked = Some(layer.id);
                break;
            }
        }
        if let Some(id) = picked {
            self.set_active_layer(id)?;
            if (dx != 0 || dy != 0)
                && self
                    .check_ancestors(id, crate::LOCK_POSITION | crate::LOCK_ALL)
                    .is_ok()
            {
                self.move_layer(id, dx, dy)?;
            }
        }
        Ok(picked)
    }
    fn pick_coverage(&self, i: usize, x: u32, y: u32) -> Result<f32> {
        let layer = &self.layers[i];
        let mut alpha = layer.read_canvas_row(x, y, 1)?[0].alpha();
        let mut node = layer;
        loop {
            if let Some(mask) = self.mask_for(node.id).filter(|m| m.properties.visible) {
                alpha *= 1. - mask.read_canvas_row(x, y, 1)?[0].alpha() * mask.properties.opacity;
            }
            if node.parent == 0 || alpha <= 0. {
                break;
            }
            node = self
                .layer(node.parent)
                .ok_or(Error::LayerNotFound(node.parent))?;
        }
        if layer.clipped {
            if let Some((base_i, base)) = self.layers[..i]
                .iter()
                .enumerate()
                .rev()
                .find(|(_, l)| !l.mask && !l.clipped && l.parent == layer.parent)
            {
                if !groups::visible(&self.layers, base) {
                    return Ok(0.);
                }
                alpha *= if base.group {
                    groups::composite_row(&self.layers[self.subtree(base.id)?], x, y, 1)?[0].alpha()
                } else {
                    self.pick_coverage(base_i, x, y)?
                };
            } else {
                return Ok(0.);
            }
        }
        Ok(alpha)
    }
    fn ensure_active_layer(&mut self) {
        if self.layer(self.active_layer).is_none() {
            self.active_layer = self.layers.last().map_or(0, |layer| layer.id);
        }
    }

    pub fn begin_stroke(&mut self, brush: Brush, first: InputPoint) -> Result<()> {
        self.begin_brush_stroke(
            BrushDefinition {
                brush,
                ..BrushDefinition::default()
            },
            first,
        )
    }

    pub fn begin_brush_stroke(
        &mut self,
        definition: BrushDefinition,
        first: InputPoint,
    ) -> Result<()> {
        if definition.wet.enabled() {
            return Err(Error::InvalidArgument("use configured wet stroke entry"));
        }
        self.begin_brush_stroke_inner(definition, first, None, None, None)
    }
    /// Explicit Rust execution entry until preset IO and the native bridge support taper.
    pub fn begin_tapered_brush_stroke(
        &mut self,
        definition: BrushDefinition,
        first: InputPoint,
        taper: &crate::StrokeTaper,
    ) -> Result<()> {
        if definition.wet.enabled() {
            return Err(Error::InvalidArgument("use configured wet stroke entry"));
        }
        self.begin_brush_stroke_inner(definition, first, None, Some(taper), None)
    }
    fn begin_brush_stroke_inner(
        &mut self,
        definition: BrushDefinition,
        first: InputPoint,
        mut wet: Option<wet::WetStroke>,
        taper_settings: Option<&crate::StrokeTaper>,
        outline_settings: Option<&crate::StrokeOutline>,
    ) -> Result<()> {
        self.idle()?;
        self.layer_index(self.active_layer)?;
        let outline_settings = outline_settings
            .cloned()
            .unwrap_or_else(|| definition.outline.clone());
        let brush = definition.brush;
        let mut program = if wet.as_ref().is_some_and(|w| w.secondary.is_some()) {
            BrushProgram::compile_wet_pair(definition)?
        } else {
            BrushProgram::compile(definition)?
        };
        program.outline_work = outline_settings.enabled()
            && outline_settings
                .width()
                .effective(brush.radius * 2.)
                .is_ok_and(|w| w > 0.);
        if let Some(wet) = &mut wet {
            self.validate_wet_start(&program, brush, first, wet)?;
        }
        first.validate()?;
        let path_filter = crate::brush_smoothing::PathFilter::new(program.smoothing, first)?;
        Self::validate_stroke_work(
            brush,
            &program,
            1,
            usize::from(program.dual.as_ref().is_some_and(|d| d.step().is_some())),
            wet.as_ref().map_or(0, |w| w.work_per_dab),
        )?;
        let random_serial = if program.write_blend == crate::BlendMode::Dissolve
            || program.scatter.enabled()
            || program.shape_jitter.active()
            || program.transfer_jitter.active()
            || program.color.active()
            || program.dual.as_ref().is_some_and(|d| d.random_active())
        {
            self.random_serial
                .checked_add(1)
                .ok_or(Error::ResourceLimit("random stroke serial exhausted"))?
        } else {
            self.random_serial
        };
        if self
            .layer(self.active_layer)
            .expect("active layer")
            .is_image()
        {
            return Err(Error::InvalidArgument("rasterize image before painting"));
        }
        if self.layer(self.active_layer).expect("active layer").group {
            return Err(Error::InvalidArgument("select a pixel layer to paint"));
        }
        self.check_ancestors(self.active_layer, crate::LOCK_ALL)?;
        if matches!(
            brush.mode,
            BrushMode::Smudge {
                all_layers: true,
                ..
            }
        ) && self.layer(self.active_layer).expect("active").mask
        {
            return Err(Error::InvalidArgument("all-layer smudge sampling on mask"));
        }
        let dual_next = program
            .dual
            .as_ref()
            .and_then(|d| d.step())
            .unwrap_or(f64::INFINITY);
        let initial_shape = program.shape_jitter.sample(random_serial, 0, 0);
        let primary_step = program.step(brush);
        let mut stroke = Stroke {
            layer: self.active_layer,
            brush,
            program,
            last: first,
            path_filter,
            last_dab: first,
            last_carrier: first,
            random_index: 0,
            random_stroke: random_serial,
            random_child: 0,
            dual_next,
            dual_index: 0,
            dual_last: first,
            airbrush_last_ns: first.timestamp_ns,
            airbrush_tick: false,
            speed: 0.,
            fade_index: 0,
            dual_coverage: BTreeMap::new(),
            stamp_field: BTreeMap::new(),
            wet_primary_field: BTreeMap::new(),
            wet_secondary_field: BTreeMap::new(),
            shape_sample: initial_shape,
            transfer_sample: crate::TransferSample::IDENTITY,
            color_sample: brush.color,
            wet,
            distance_to_next: primary_step,
            direction: None,
            before: BTreeMap::new(),
            coverage: BTreeMap::new(),
            first_dab: true,
            coverage_erase: brush.mode == BrushMode::Erase || first.tool == Tool::Eraser,
            coverage_tool_stable: true,
            taper: None,
            outline: outline::OutlineStroke::new(&outline_settings, brush.radius * 2., first)?,
            taper_factors: crate::TaperFactors {
                diameter: 1.,
                opacity: 1.,
            },
            path_distance: 0.,
            dab_distance: 0.,
            mask_paint: self
                .layer(self.active_layer)
                .expect("active")
                .mask
                .then(|| Self::mask_tone(brush.color)),
        };
        let captured_taper = taper_settings
            .cloned()
            .unwrap_or_else(|| stroke.program.taper.clone());
        stroke.initialize_taper(Some(&captured_taper))?;
        #[cfg(test)]
        stroke.taper_reference(Some(&captured_taper), self.taper_reference_endpoint)?;
        match self
            .secondary_dab(&mut stroke, first)
            .and_then(|()| self.dab(&mut stroke, first))
            .and_then(|changed| {
                self.render_taper(&mut stroke, false)
                    .map(|tail| changed || tail)
            })
            .and_then(|changed| {
                self.render_stroke_outline(&mut stroke)
                    .map(|outline| changed || outline)
            }) {
            Ok(changed) => {
                if changed {
                    self.bump_revision();
                }
                self.random_serial = random_serial;
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
        if self.pending_input.is_some() {
            return Err(Error::Busy);
        }
        self.prepare_outline_input();
        if let Some(tail) = self.stroke.as_mut().and_then(|s| s.taper.as_mut()) {
            tail.reset_work();
        }
        let transactional = self
            .stroke
            .as_ref()
            .is_some_and(|s| s.wet.is_some() || s.taper.is_some() || s.outline.is_some());
        let result = self
            .stroke_to_inner(point)
            .and_then(|()| self.publish_outline());
        if result.is_err() && transactional {
            if let Some(stroke) = self.stroke.take() {
                self.restore_stroke(&stroke);
            }
        }
        result
    }

    /// Deposit one stationary airbrush tick. The caller supplies a monotonically
    /// increasing timestamp while the pointer position remains unchanged.
    pub fn airbrush_tick(&mut self, point: InputPoint) -> Result<()> {
        if self.pending_input.is_some() {
            return Err(Error::Busy);
        }
        point.validate()?;
        let current = self.stroke.as_ref().ok_or(Error::NoStroke)?;
        if !current.program.airbrush.enabled() {
            return Err(Error::InvalidArgument("airbrush is not enabled"));
        }
        if current.wet.is_some() || matches!(current.brush.mode, BrushMode::Smudge { .. }) {
            return Err(Error::InvalidArgument(
                "airbrush requires dry paint or erase mode",
            ));
        }
        if (point.x - current.last.x).hypot(point.y - current.last.y) > 1e-9 {
            return Err(Error::InvalidArgument(
                "airbrush tick must remain at the last point",
            ));
        }
        if point.timestamp_ns < current.last.timestamp_ns {
            return Err(Error::InvalidArgument("airbrush time must be monotonic"));
        }
        let interval = current.program.airbrush.interval_ns();
        if point.timestamp_ns.saturating_sub(current.airbrush_last_ns) < interval {
            return Ok(());
        }
        let secondary = current
            .program
            .dual
            .as_ref()
            .is_some_and(|d| d.step().is_some());
        Self::validate_stroke_work(
            current.brush,
            &current.program,
            1,
            usize::from(secondary),
            0,
        )?;
        let mut stroke = self.stroke.take().expect("checked active stroke");
        self.remove_outline_preview(&mut stroke);
        stroke.last = point;
        stroke.airbrush_last_ns = point.timestamp_ns;
        stroke.airbrush_tick = true;
        stroke.speed = 0.;
        let mut changed = false;
        if secondary {
            if let Err(error) = self.secondary_dab(&mut stroke, point) {
                self.restore_stroke(&stroke);
                return Err(error);
            }
        }
        match self.dab(&mut stroke, point) {
            Ok(value) => changed |= value,
            Err(error) => {
                self.restore_stroke(&stroke);
                return Err(error);
            }
        }
        stroke.last_dab = point;
        match self.render_taper(&mut stroke, false) {
            Ok(value) => changed |= value,
            Err(error) => {
                self.restore_stroke(&stroke);
                return Err(error);
            }
        }
        if changed {
            self.bump_revision();
        }
        stroke.airbrush_tick = false;
        match self.render_stroke_outline(&mut stroke) {
            Ok(value) => {
                if value {
                    self.bump_revision();
                }
            }
            Err(error) => {
                self.restore_stroke(&stroke);
                return Err(error);
            }
        }
        self.stroke = Some(stroke);
        Ok(())
    }
    fn stroke_to_inner(&mut self, point: InputPoint) -> Result<()> {
        let current = self.stroke.as_ref().ok_or(Error::NoStroke)?;
        Self::validate_input_tool(current, point)?;
        let Some(mut filter) = current.path_filter.clone() else {
            return self.stroke_to_direct(point);
        };
        let points = filter.push(point)?;
        Self::validate_filtered_path(current, &points)?;
        self.stroke.as_mut().expect("active stroke").path_filter = Some(filter);
        self.apply_filtered_path(points)
    }

    fn apply_filtered_path(&mut self, points: Vec<InputPoint>) -> Result<()> {
        for point in points {
            if let Err(error) = self.stroke_to_direct(point) {
                if let Some(stroke) = self.stroke.take() {
                    self.restore_stroke(&stroke);
                }
                return Err(error);
            }
        }
        Ok(())
    }

    fn validate_filtered_path(stroke: &Stroke, points: &[InputPoint]) -> Result<()> {
        let mut last = stroke.last;
        let step = stroke.program.step(stroke.brush);
        let secondary_step = stroke.program.dual.as_ref().and_then(|d| d.step());
        let mut next = stroke.distance_to_next;
        let mut secondary_next = stroke.dual_next;
        let mut primary = 0usize;
        let mut secondary = 0usize;
        for &point in points {
            point.validate()?;
            if point.timestamp_ns < last.timestamp_ns {
                return Err(Error::InvalidArgument("filtered time must be monotonic"));
            }
            let distance = (point.x - last.x).hypot(point.y - last.y);
            let count = if distance + 1e-9 < next {
                0
            } else {
                ((distance - next + 1e-9) / step).floor() as usize + 1
            };
            let secondary_count = secondary_step.map_or(0, |s| {
                if distance + 1e-9 < secondary_next {
                    0
                } else {
                    ((distance - secondary_next + 1e-9) / s).floor() as usize + 1
                }
            });
            primary = primary
                .saturating_add(count)
                .saturating_add(usize::from(Self::stationary_primary(stroke, last, point)));
            secondary = secondary
                .saturating_add(secondary_count)
                .saturating_add(usize::from(Self::stationary_secondary(stroke, last, point)));
            next = (next + count as f64 * step - distance).max(step * 1e-9);
            if let Some(s) = secondary_step {
                secondary_next =
                    (secondary_next + secondary_count as f64 * s - distance).max(s * 1e-9);
            }
            last = point;
        }
        Self::validate_stroke_work(
            stroke.brush,
            &stroke.program,
            primary,
            secondary,
            stroke.wet.as_ref().map_or(0, |w| w.work_per_dab),
        )
    }

    fn stroke_to_direct(&mut self, point: InputPoint) -> Result<()> {
        point.validate()?;
        let current = self.stroke.as_ref().ok_or(Error::NoStroke)?;
        if point.timestamp_ns < current.last.timestamp_ns {
            return Err(Error::InvalidArgument("tablet time must be monotonic"));
        }
        let distance = (point.x - current.last.x).hypot(point.y - current.last.y);
        let elapsed_ns = point.timestamp_ns.saturating_sub(current.last.timestamp_ns);
        let speed = if distance > 1e-9 && elapsed_ns > 0 {
            (distance / (elapsed_ns as f64 / 1_000_000_000.)).min(1_000_000.) as f32
        } else {
            0.
        };
        let stationary_pressure = Self::stationary_primary(current, current.last, point);
        let step = current.program.step(current.brush);
        let count = if distance + 1e-9 < current.distance_to_next {
            0
        } else {
            ((distance - current.distance_to_next + 1e-9) / step).floor() as usize + 1
        };
        let dual_step = current.program.dual.as_ref().and_then(|d| d.step());
        let secondary_stationary = Self::stationary_secondary(current, current.last, point);
        let secondary_count = dual_step.map_or(0, |secondary_step| {
            if distance + 1e-9 < current.dual_next {
                0
            } else {
                ((distance - current.dual_next + 1e-9) / secondary_step).floor() as usize + 1
            }
        });
        Self::validate_stroke_work(
            current.brush,
            &current.program,
            count + usize::from(stationary_pressure),
            secondary_count + usize::from(secondary_stationary),
            current.wet.as_ref().map_or(0, |w| w.work_per_dab),
        )?;
        let mut stroke = self.stroke.take().expect("checked active stroke");
        stroke.speed = speed;
        if distance > 1e-9 {
            stroke.direction = Some((point.y - stroke.last.y).atan2(point.x - stroke.last.x));
        }
        let mut position = stroke.distance_to_next;
        let mut secondary_position = stroke.dual_next;
        let mut primary_remaining = count;
        let mut secondary_remaining = secondary_count;
        let mut changed = false;
        while primary_remaining > 0 || secondary_remaining > 0 {
            // Tie goes to the secondary stream; primary reads its current field.
            if secondary_remaining > 0
                && (primary_remaining == 0 || secondary_position <= position + 1e-9)
            {
                let sample = stroke
                    .last
                    .interpolate(point, (secondary_position / distance).min(1.));
                stroke.dab_distance = stroke.path_distance + secondary_position.min(distance);
                if let Err(error) = self.secondary_dab(&mut stroke, sample) {
                    self.restore_stroke(&stroke);
                    return Err(error);
                }
                secondary_position += dual_step.expect("secondary stream");
                secondary_remaining -= 1;
            } else {
                let sample = stroke
                    .last
                    .interpolate(point, (position / distance).min(1.));
                stroke.dab_distance = stroke.path_distance + position.min(distance);
                match self.dab(&mut stroke, sample) {
                    Ok(dab_changed) => changed |= dab_changed,
                    Err(error) => {
                        self.restore_stroke(&stroke);
                        return Err(error);
                    }
                }
                stroke.last_dab = sample;
                position += step;
                primary_remaining -= 1;
            }
        }
        stroke.distance_to_next = (position - distance).max(step * 1e-9);
        if let Some(secondary_step) = dual_step {
            stroke.dual_next = (secondary_position - distance).max(secondary_step * 1e-9);
        }
        stroke.path_distance += distance;
        stroke.dab_distance = stroke.path_distance;
        if secondary_stationary {
            if let Err(error) = self.secondary_dab(&mut stroke, point) {
                self.restore_stroke(&stroke);
                return Err(error);
            }
        }
        if stationary_pressure {
            match self.dab(&mut stroke, point) {
                Ok(dab_changed) => changed |= dab_changed,
                Err(error) => {
                    self.restore_stroke(&stroke);
                    return Err(error);
                }
            }
            stroke.last_dab = point;
        }
        stroke.last = point;
        match self.render_taper(&mut stroke, false) {
            Ok(tail) => changed |= tail,
            Err(error) => {
                self.restore_stroke(&stroke);
                return Err(error);
            }
        }
        if changed {
            self.bump_revision();
        }
        self.stroke = Some(stroke);
        Ok(())
    }

    fn validate_stroke_work(
        brush: Brush,
        program: &BrushProgram,
        count: usize,
        secondary_count: usize,
        wet_work: usize,
    ) -> Result<()> {
        let (primary, secondary) = Self::stroke_input_work(brush, program, wet_work);
        let work = primary
            .repeated(count)
            .combined(secondary.repeated(secondary_count));
        if work.dabs > MAX_DABS_PER_SEGMENT || work.visits > MAX_PIXEL_VISITS_PER_SEGMENT {
            return Err(Error::ResourceLimit(
                "input segment too large; resample into shorter segments",
            ));
        }
        Ok(())
    }

    fn stroke_input_work(
        brush: Brush,
        program: &BrushProgram,
        wet_work: usize,
    ) -> (crate::InputScheduleWork, crate::InputScheduleWork) {
        let scatter = program.scatter.maximum_count();
        let diameter = (f64::from(brush.radius)
            * 2.
            * if program.has_mask() {
                std::f64::consts::SQRT_2
            } else {
                1.
            }
            + 2.)
            .ceil() as usize;
        let dual = program.dual.as_ref();
        let paired = dual.filter(|d| d.step().is_none());
        let path = dual.filter(|d| d.step().is_some());
        let sample_cost = if matches!(brush.mode, BrushMode::Smudge { .. }) {
            5
        } else {
            1
        } + if program.has_texture() { 8 } else { 0 }
            + if program.write_blend != crate::BlendMode::Normal {
                8
            } else {
                0
            }
            + paired.map_or(0, |d| d.sample_cost())
            + usize::from(path.is_some());
        let primary_colored = dual.filter(|d| d.color.enabled()).map_or(0, |d| {
            (diameter + 1)
                .saturating_pow(2)
                .saturating_mul(d.colored_cost(program.write_blend))
                .saturating_add(if d.step().is_none() {
                    d.colored_work(1, program.write_blend)
                } else {
                    0
                })
        });
        let (secondary_dabs, secondary_visits) = path.map_or((0, 0), |d| d.path_work(1));
        let secondary_colored = path
            .filter(|d| d.color.enabled())
            .map_or(0, |d| d.colored_work(1, program.write_blend));
        let visits = diameter
            .saturating_pow(2)
            .saturating_mul(sample_cost)
            .saturating_add(primary_colored)
            .saturating_add(wet_work)
            .saturating_add(if wet_work > 0 {
                diameter
                    .saturating_pow(2)
                    .saturating_mul(12 + if program.color.active() { 8 } else { 0 })
            } else {
                0
            });
        (
            crate::InputScheduleWork {
                dabs: scatter.saturating_mul(paired.map_or(1, |d| d.maximum_count())),
                visits: visits
                    .saturating_mul(scatter)
                    .saturating_mul(if program.outline_work { 2 } else { 1 }),
            },
            crate::InputScheduleWork {
                dabs: secondary_dabs,
                visits: secondary_visits.saturating_add(secondary_colored),
            },
        )
    }

    pub fn end_stroke(&mut self) -> Result<bool> {
        if self.stroke.as_ref().is_some_and(|s| s.wet.is_some()) {
            return Err(Error::InvalidArgument(
                "use end_wet_brush_stroke for wet strokes",
            ));
        }
        self.finish_stroke().map(|(changed, _)| changed)
    }
    fn finish_stroke(&mut self) -> Result<(bool, Option<crate::WetInk>)> {
        if self.pending_input.is_some() {
            return Err(Error::Busy);
        }
        self.finish_stroke_with_end(true)
    }
    fn finish_stroke_with_end(
        &mut self,
        render_end: bool,
    ) -> Result<(bool, Option<crate::WetInk>)> {
        // The visible filtered endpoint is authoritative. Releasing the pen must not
        // synthesize input, release stabilizer distance, or raise pressure to the raw value.
        let mut stroke = self.stroke.take().ok_or(Error::NoStroke)?;
        self.remove_outline_preview(&mut stroke);
        if render_end {
            if let Some(tail) = &mut stroke.taper {
                tail.reset_work();
            }
            let secondary_final = stroke
                .program
                .dual
                .as_ref()
                .is_some_and(|d| d.step().is_some())
                && (stroke.last.x - stroke.dual_last.x).hypot(stroke.last.y - stroke.dual_last.y)
                    > 1e-7;
            let primary_final =
                (stroke.last.x - stroke.last_dab.x).hypot(stroke.last.y - stroke.last_dab.y) > 1e-7
                    || secondary_final;
            if let Err(error) = Self::validate_stroke_work(
                stroke.brush,
                &stroke.program,
                usize::from(primary_final),
                usize::from(secondary_final),
                stroke.wet.as_ref().map_or(0, |w| w.work_per_dab),
            ) {
                self.restore_stroke(&stroke);
                return Err(error);
            }
            if secondary_final {
                let last = stroke.last;
                if let Err(error) = self.secondary_dab(&mut stroke, last) {
                    self.restore_stroke(&stroke);
                    return Err(error);
                }
            }
            if primary_final {
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
            if let Err(error) = self.render_taper(&mut stroke, true) {
                self.restore_stroke(&stroke);
                return Err(error);
            }
        }
        match self.render_stroke_outline(&mut stroke) {
            Ok(true) => self.bump_revision(),
            Ok(false) => {}
            Err(error) => {
                self.restore_stroke(&stroke);
                return Err(error);
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
            return Ok((
                false,
                stroke
                    .wet
                    .map(|w| crate::WetInk::new(w.carrier, w.secondary)),
            ));
        }
        let command = Command::Stroke {
            layer: stroke.layer,
            changes,
        };
        let action = match stroke.brush.mode {
            BrushMode::Erase => crate::HistoryAction::Eraser,
            BrushMode::Paint => crate::HistoryAction::Brush,
            BrushMode::Smudge { .. } => crate::HistoryAction::Smudge,
        };
        if let Err(error) = self.commit_with_action(command.clone(), action) {
            // History preparation failed before eviction. Restore all touched
            // tiles, retain the old history, and leave no half-committed stroke.
            self.apply_history(&command, false);
            return Err(error);
        }
        Ok((
            true,
            stroke
                .wet
                .map(|w| crate::WetInk::new(w.carrier, w.secondary)),
        ))
    }

    pub fn cancel_stroke(&mut self) -> Result<()> {
        self.pending_input = None;
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

    fn mask_tone(color: Pixel) -> (f32, f32) {
        let c = color.components();
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
        let gray = if a > 0. {
            encode(c[1] / a)
                + 0.2126 * (encode(c[0] / a) - encode(c[1] / a))
                + 0.0722 * (encode(c[2] / a) - encode(c[1] / a))
        } else {
            1.
        };
        ((1. - gray).clamp(0., 1.), a)
    }
    fn secondary_dab(&mut self, stroke: &mut Stroke, point: InputPoint) -> Result<()> {
        if stroke
            .program
            .dual
            .as_ref()
            .is_some_and(|d| d.step().is_some())
            && stroke.queue_taper(point, crate::TaperStream::Secondary)?
        {
            stroke.dual_last = point;
            return Ok(());
        }
        if stroke.program.dual.as_ref().is_some_and(|d| {
            stroke.brush.mode == BrushMode::Paint && d.color.enabled() && d.step().is_some()
        }) && point.tool != Tool::Eraser
        {
            return self.colored_secondary_dab(stroke, point);
        }
        let Some(dual) = stroke.program.dual.as_ref().filter(|d| d.step().is_some()) else {
            return Ok(());
        };
        let next = stroke
            .dual_index
            .checked_add(1)
            .ok_or(Error::ResourceLimit("secondary dab serial exhausted"))?;
        stroke.transfer_sample =
            stroke
                .program
                .transfer_jitter
                .sample(stroke.random_stroke, stroke.dual_index, 0);
        let speed_size = stroke
            .program
            .factor_all(
                stroke.speed,
                point.tangential_pressure,
                stroke.dab_distance as f32,
                point.tilt_magnitude(),
                point.rotation_unit(),
                BrushProgram::direction_unit(stroke.direction),
                stroke.fade_index,
            )
            .0;
        let transfer = stroke.transfer_sample;
        let plan = dual
            .dab(
                point,
                stroke.direction,
                stroke.random_stroke,
                stroke.dual_index,
                0,
                f64::from(
                    stroke.brush.radius
                        * stroke.shape_sample.size
                        * if stroke.program.tip.pressure_size {
                            stroke.program.size_pressure(point.pressure)
                        } else {
                            1.
                        },
                ) * f64::from(speed_size)
                    * f64::from(transfer.size),
                f64::from(speed_size) * f64::from(transfer.size),
            )
            .scaled(stroke.taper_factors.diameter);
        #[cfg(test)]
        let skip_saturated = !self.reference_mix_path;
        #[cfg(not(test))]
        let skip_saturated = true;
        let appearance = self.layers[self.layer_index(stroke.layer)?].appearance;
        let ox = i64::from(appearance.offset_x);
        let oy = i64::from(appearance.offset_y);
        for child in 0..plan.count() {
            let (left, top, right, bottom) = plan.bounds(child);
            let x0 = (left.floor() as i64).clamp(0, i64::from(self.width)) - ox;
            let x1 = (right.ceil() as i64).clamp(0, i64::from(self.width)) - ox;
            let y0 = (top.floor() as i64).clamp(0, i64::from(self.height)) - oy;
            let y1 = (bottom.ceil() as i64).clamp(0, i64::from(self.height)) - oy;
            if x0 >= x1 || y0 >= y1 {
                continue;
            }
            for ty in y0.div_euclid(64)..=(y1 - 1).div_euclid(64) {
                for tx in x0.div_euclid(64)..=(x1 - 1).div_euclid(64) {
                    let coord = TileCoord::from_signed(tx as i32, ty as i32);
                    self.capture_taper_tile(
                        stroke.layer,
                        &mut stroke.taper,
                        &stroke.coverage,
                        &stroke.dual_coverage,
                        &stroke.stamp_field,
                        coord,
                    )?;
                    let source = stroke
                        .dual_coverage
                        .get(&coord)
                        .map(|t| t.try_pixels())
                        .transpose()?;
                    let mut updates = Vec::new();
                    for y in y0.max(ty * 64)..y1.min((ty + 1) * 64) {
                        for x in x0.max(tx * 64)..x1.min((tx + 1) * 64) {
                            let offset = (y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize;
                            let before = source.as_ref().map_or(0., |p| p[offset].alpha());
                            if skip_saturated && before == 1. {
                                continue;
                            }
                            let amount = plan.coverage(
                                child,
                                x as f64 + 0.5 + ox as f64,
                                y as f64 + 0.5 + oy as f64,
                            ) * transfer.flow;
                            if amount == 0. {
                                continue;
                            }
                            let after = before + amount * (1. - before);
                            if after != before {
                                updates.push((offset, Pixel::from_straight([0., 0., 0., after])?));
                            }
                        }
                    }
                    if !updates.is_empty() {
                        if stroke
                            .dual_coverage
                            .get(&coord)
                            .is_some_and(Tile::can_edit_private)
                        {
                            Tile::edit_private(
                                stroke
                                    .dual_coverage
                                    .get_mut(&coord)
                                    .expect("secondary field"),
                                &self.pool,
                                &updates,
                            )?;
                        } else {
                            if !stroke.dual_coverage.contains_key(&coord)
                                && stroke.dual_coverage.len() >= self.options.max_document_tiles
                            {
                                return Err(Error::ResourceLimit(
                                    "secondary coverage tile limit; entire stroke rolled back",
                                ));
                            }
                            let mut values = source
                                .as_ref()
                                .map_or_else(|| vec![Pixel::TRANSPARENT; 4096], |p| p.to_vec());
                            for (offset, value) in updates {
                                values[offset] = value;
                            }
                            let tile = Arc::new(Tile::from_pixels(values)?);
                            self.pool.attach(&tile)?;
                            stroke.dual_coverage.insert(coord, tile);
                        }
                    }
                    drop(source);
                    self.pool.trim()?;
                }
            }
        }
        stroke.dual_index = next;
        stroke.dual_last = point;
        Ok(())
    }
    fn dab(&mut self, stroke: &mut Stroke, point: InputPoint) -> Result<bool> {
        if stroke.queue_taper(point, crate::TaperStream::Primary)? {
            return Ok(false);
        }
        let logical = stroke.random_index;
        let plan = stroke.program.scatter.plan(
            stroke.brush.radius,
            stroke.random_stroke,
            logical,
            stroke.direction,
            point,
            stroke.fade_index,
        );
        if plan.is_some()
            || stroke.program.shape_jitter.active()
            || stroke.program.transfer_jitter.active()
            || stroke.program.color.active()
            || stroke
                .program
                .dual
                .as_ref()
                .is_some_and(|d| d.random_active())
        {
            stroke.random_index = logical
                .checked_add(1)
                .ok_or(Error::ResourceLimit("random dab serial exhausted"))?;
        }
        let result = if let Some(plan) = plan {
            let mut changed = false;
            for index in 0..plan.count {
                let sample = plan.point(index, point);
                if stroke.wet.is_none() {
                    stroke.color_sample = stroke.program.color.sample(
                        stroke.brush.color,
                        stroke.brush.background_color,
                        stroke.random_stroke,
                        logical,
                        index,
                    );
                }
                stroke.shape_sample =
                    stroke
                        .program
                        .shape_jitter
                        .sample(stroke.random_stroke, logical, index);
                stroke.transfer_sample =
                    stroke
                        .program
                        .transfer_jitter
                        .sample(stroke.random_stroke, logical, index);
                if stroke.first_dab {
                    stroke.last_carrier = sample;
                }
                stroke.random_child = index;
                changed |= self.dab_single(stroke, sample)?;
                stroke.last_carrier = sample;
            }
            Ok(changed)
        } else {
            stroke.shape_sample =
                stroke
                    .program
                    .shape_jitter
                    .sample(stroke.random_stroke, logical, 0);
            stroke.transfer_sample =
                stroke
                    .program
                    .transfer_jitter
                    .sample(stroke.random_stroke, logical, 0);
            if stroke.wet.is_none() {
                stroke.color_sample = stroke.program.color.sample(
                    stroke.brush.color,
                    stroke.brush.background_color,
                    stroke.random_stroke,
                    logical,
                    0,
                );
            }
            stroke.random_child = 0;
            stroke.last_carrier = stroke.last_dab;
            self.dab_single(stroke, point)
        };
        if result.is_ok() {
            stroke.fade_index = stroke.fade_index.saturating_add(1);
        }
        result
    }
    fn dab_single(&mut self, stroke: &mut Stroke, point: InputPoint) -> Result<bool> {
        if stroke
            .program
            .dual
            .as_ref()
            .is_some_and(|d| stroke.brush.mode == BrushMode::Paint && d.color.enabled())
            && point.tool != Tool::Eraser
        {
            return self.colored_primary_dab(stroke, point);
        }
        if stroke.program.dual.is_some() {
            self.dab_dispatch::<true>(stroke, point)
        } else {
            self.dab_dispatch::<false>(stroke, point)
        }
    }
    fn dab_dispatch<const DUAL: bool>(
        &mut self,
        stroke: &mut Stroke,
        point: InputPoint,
    ) -> Result<bool> {
        if stroke.wet.is_none() && stroke.mask_paint.is_some() && stroke.program.color.active() {
            stroke.mask_paint = Some(Self::mask_tone(stroke.color_sample));
        }
        // Specialize once per dab: an inactive selection adds no pixel-loop
        // geometry branches or coverage multiplication to the original hot path.
        match (
            self.selection.enabled(),
            stroke.program.has_mask(),
            stroke.program.has_texture(),
        ) {
            (true, true, true) => self.dab_with_selection::<true, true, true, DUAL>(stroke, point),
            (true, false, true) => {
                self.dab_with_selection::<true, false, true, DUAL>(stroke, point)
            }
            (false, true, true) => {
                self.dab_with_selection::<false, true, true, DUAL>(stroke, point)
            }
            (false, false, true) => {
                self.dab_with_selection::<false, false, true, DUAL>(stroke, point)
            }
            (true, true, false) => {
                self.dab_with_selection::<true, true, false, DUAL>(stroke, point)
            }
            (true, false, false) => {
                self.dab_with_selection::<true, false, false, DUAL>(stroke, point)
            }
            (false, true, false) => {
                self.dab_with_selection::<false, true, false, DUAL>(stroke, point)
            }
            (false, false, false) => {
                self.dab_with_selection::<false, false, false, DUAL>(stroke, point)
            }
        }
    }

    fn dab_with_selection<
        const SELECTED: bool,
        const IMAGED: bool,
        const TEXTURED: bool,
        const DUAL: bool,
    >(
        &mut self,
        stroke: &mut Stroke,
        point: InputPoint,
    ) -> Result<bool> {
        let (speed_size, speed_flow, speed_opacity) = stroke.program.factor_all(
            stroke.speed,
            point.tangential_pressure,
            stroke.dab_distance as f32,
            point.tilt_magnitude(),
            point.rotation_unit(),
            BrushProgram::direction_unit(stroke.direction),
            stroke.fade_index,
        );
        let transfer = stroke.transfer_sample;
        let opacity_factor = speed_opacity * transfer.opacity;
        let base_radius = f64::from(
            stroke.brush.radius
                * stroke.shape_sample.size
                * transfer.size
                * if stroke.program.tip.pressure_size {
                    stroke.program.size_pressure(point.pressure)
                } else {
                    1.
                },
        ) * f64::from(speed_size);
        let radius = base_radius * f64::from(stroke.taper_factors.diameter);
        let pressure_opacity = stroke
            .program
            .opacity_pressure(point.pressure)
            .map(|p| p * stroke.taper_factors.opacity * opacity_factor)
            .or_else(|| {
                (stroke.taper.is_some() || opacity_factor != 1.)
                    .then_some(stroke.taper_factors.opacity * opacity_factor)
            });
        let paper_texture = TEXTURED
            && stroke
                .program
                .texture
                .as_ref()
                .is_some_and(|t| t.document_space());
        let write_blend = stroke.program.write_blend;
        let capped = (stroke.wet.is_some()
            || stroke.brush.opacity < 1.
            || pressure_opacity.is_some()
            || paper_texture
            || write_blend != crate::BlendMode::Normal)
            && !matches!(stroke.brush.mode, BrushMode::Smudge { .. });
        let flow = if capped { 1. } else { stroke.brush.opacity }
            * stroke.program.tip.flow
            * match stroke.brush.mode {
                BrushMode::Smudge {
                    pressure_strength, ..
                } => {
                    if pressure_strength {
                        stroke.program.flow_pressure(point.pressure)
                    } else {
                        1.
                    }
                }
                _ => {
                    if stroke.program.tip.pressure_flow {
                        stroke.program.flow_pressure(point.pressure)
                    } else {
                        1.
                    }
                }
            }
            * if matches!(stroke.brush.mode, BrushMode::Smudge { .. }) {
                stroke.taper_factors.opacity
            } else {
                1.
            }
            * speed_flow
            * transfer.flow;
        if radius == 0. {
            if let Some(wet) = &mut stroke.wet {
                wet.carrier.reset_path()?;
            }
            return Ok(false);
        }
        if stroke.wet.is_none() && (flow == 0. || stroke.brush.opacity == 0.) {
            return Ok(false);
        }
        let geometry = stroke
            .program
            .dab_geometry(point, stroke.direction, stroke.shape_sample);
        let texture_dab = if TEXTURED {
            stroke.program.texture_dab(point, geometry)
        } else {
            None
        };
        let dual_path = DUAL
            && stroke
                .program
                .dual
                .as_ref()
                .is_some_and(|d| d.step().is_some());
        let dual_dab = if DUAL && !dual_path {
            Some(
                stroke
                    .program
                    .dual
                    .as_ref()
                    .expect("compiled dual")
                    .dab(
                        point,
                        stroke.direction,
                        stroke.random_stroke,
                        stroke.random_index.saturating_sub(1),
                        stroke.random_child,
                        base_radius,
                        f64::from(speed_size),
                    )
                    .scaled(stroke.taper_factors.diameter),
            )
        } else {
            None
        };
        let (extent_x, extent_y) = stroke.program.extent(radius, geometry);
        let mut left = point.x - extent_x - 1.0;
        let mut top = point.y - extent_y - 1.0;
        let mut right = point.x + extent_x + 1.0;
        let mut bottom = point.y + extent_y + 1.0;
        if stroke.first_dab
            && matches!(
                stroke.brush.mode,
                BrushMode::Smudge {
                    finger_painting: true,
                    ..
                }
            )
        {
            if let Some(plan) = &dual_dab {
                for child in 0..plan.count() {
                    let (l, t, r, b) = plan.bounds(child);
                    left = left.min(l);
                    top = top.min(t);
                    right = right.max(r);
                    bottom = bottom.max(b);
                }
            }
        }
        let x0 = (left.floor() as i64).clamp(0, i64::from(self.width));
        let y0 = (top.floor() as i64).clamp(0, i64::from(self.height));
        let x1 = (right.ceil() as i64).clamp(0, i64::from(self.width));
        let y1 = (bottom.ceil() as i64).clamp(0, i64::from(self.height));
        let index = self.layer_index(stroke.layer)?;
        let appearance = self.layers[index].appearance;
        let mask = self.layers[index].mask;
        let alpha_locked = !mask && appearance.locks & crate::LOCK_TRANSPARENCY != 0;
        let erase = stroke.brush.mode == BrushMode::Erase
            || (point.tool == Tool::Eraser
                && !matches!(stroke.brush.mode, BrushMode::Smudge { .. }));
        stroke.coverage_tool_stable &= erase == stroke.coverage_erase;
        let mut changed = false;
        let mask_dab = if IMAGED {
            stroke.program.mask_dab_with_geometry(radius, geometry)
        } else {
            None
        };
        if stroke.outline.is_some() && !erase && pressure_opacity != Some(0.) {
            let updates = self.outline_shape(stroke, (left, top, right, bottom), |x, y| {
                let mut coverage = if let Some(mask) = &mask_dab {
                    mask.coverage(x - point.x, y - point.y)
                } else {
                    stroke.program.procedural_coverage_with_geometry(
                        x - point.x,
                        y - point.y,
                        radius,
                        geometry,
                    )
                };
                if DUAL {
                    coverage = if dual_path {
                        stroke.program.dual.as_ref().expect("dual").combine(
                            coverage,
                            Self::wet_secondary_coverage(stroke, x, y, appearance)?,
                        )
                    } else {
                        dual_dab.as_ref().expect("dual").combine(coverage, x, y)
                    };
                }
                if TEXTURED {
                    let texture = texture_dab.as_ref().expect("texture");
                    coverage = if paper_texture {
                        coverage * texture.coverage(x, y)
                    } else {
                        texture.apply(coverage, x, y)
                    };
                }
                Ok(coverage)
            })?;
            outline::install_shape(
                &mut stroke.outline,
                &mut stroke.taper,
                updates,
                self.options.max_document_tiles,
            )?;
        }
        if stroke.wet.is_none() && (x0 >= x1 || y0 >= y1) {
            return Ok(false);
        }
        let wet = self.prepare_wet(stroke, point, radius, geometry, |x, y| {
            let mut coverage = if let Some(mask) = &mask_dab {
                mask.coverage(x - point.x, y - point.y)
            } else {
                stroke.program.procedural_coverage_with_geometry(
                    x - point.x,
                    y - point.y,
                    radius,
                    geometry,
                )
            };
            if DUAL {
                coverage = if dual_path {
                    stroke.program.dual.as_ref().expect("dual").combine(
                        coverage,
                        Self::wet_secondary_coverage(stroke, x, y, appearance)?,
                    )
                } else {
                    dual_dab.as_ref().expect("dual").combine(coverage, x, y)
                };
            }
            if TEXTURED && !paper_texture {
                coverage = texture_dab.as_ref().expect("texture").apply(coverage, x, y);
            }
            Ok(coverage)
        })?;
        if x0 >= x1 || y0 >= y1 || (wet.is_some() && (flow == 0. || stroke.brush.opacity == 0.)) {
            Self::commit_wet(stroke, wet)?;
            return Ok(false);
        }
        if stroke.wet.is_none()
            && matches!(stroke.brush.mode, BrushMode::Smudge { .. })
            && !(stroke.first_dab
                && matches!(
                    stroke.brush.mode,
                    BrushMode::Smudge {
                        finger_painting: true,
                        ..
                    }
                ))
            && (point.x - stroke.last_carrier.x).hypot(point.y - stroke.last_carrier.y) < 1e-9
        {
            stroke.first_dab = false;
            return Ok(false);
        }
        #[cfg(test)]
        let skip_saturated = !self.reference_mix_path;
        #[cfg(not(test))]
        let skip_saturated = true;
        let airbrush_tick = stroke.airbrush_tick;
        let color_capped = capped && stroke.program.color.active() && !erase && !airbrush_tick;
        let wet_color = (stroke.wet.is_some() && stroke.program.color.active()).then(|| {
            stroke.program.color.prepare(
                stroke.brush.background_color,
                stroke.random_stroke,
                stroke.random_index.saturating_sub(1),
                stroke.random_child,
            )
        });
        let wet_smudge = wet.is_some() && matches!(stroke.brush.mode, BrushMode::Smudge { .. });
        let smudge = if wet_smudge {
            None
        } else {
            self.prepare_smudge(stroke, point, x0, y0, x1, y1)?
        };
        let opaque_repeat = skip_saturated
            && !capped
            && stroke.wet.is_none()
            && !stroke
                .program
                .dual
                .as_ref()
                .is_some_and(|d| d.color.enabled())
            && smudge.is_none()
            && !mask
            && !alpha_locked
            && write_blend == crate::BlendMode::Normal
            && stroke.brush.opacity >= 1.
            && flow >= 1.
            && (erase || stroke.color_sample.alpha() >= 1.)
            && !stroke.program.color.active()
            && stroke.taper.is_none();
        stroke.first_dab = false;
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
                self.capture_taper_tile(
                    stroke.layer,
                    &mut stroke.taper,
                    &stroke.coverage,
                    &stroke.dual_coverage,
                    &stroke.stamp_field,
                    coord,
                )?;
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
                let original = if capped {
                    stroke
                        .before
                        .get(&coord)
                        .and_then(Option::as_ref)
                        .map(|t| t.try_pixels())
                        .transpose()?
                } else {
                    None
                };
                let prior_coverage = stroke
                    .coverage
                    .get(&coord)
                    .map(|t| t.try_pixels())
                    .transpose()?;
                let secondary_coverage = if dual_path {
                    stroke
                        .dual_coverage
                        .get(&coord)
                        .map(|t| t.try_pixels())
                        .transpose()?
                } else {
                    None
                };
                let mut coverage_updates = Vec::new();
                for y in y0.max(ty * 64)..y1.min((ty + 1) * 64) {
                    let start_x = x0.max(tx * 64);
                    let end_x = x1.min((tx + 1) * 64);
                    let selection_row = if SELECTED {
                        self.selection_cache.row(
                            &self.selection,
                            start_x + i64::from(appearance.offset_x),
                            y + i64::from(appearance.offset_y),
                            (end_x - start_x) as usize,
                        )
                    } else {
                        [1.; 64]
                    };
                    for x in start_x..end_x {
                        let selected = if SELECTED {
                            selection_row[(x - start_x) as usize]
                        } else {
                            1.
                        };
                        if selected == 0. {
                            continue;
                        }
                        // No new coverage or color moment can be contributed once this
                        // immutable stroke's current pressure ceiling is reached. Paper
                        // texture and a changed paint/erase operator must still recompute.
                        if skip_saturated
                            && stroke.wet.is_none()
                            && capped
                            && !paper_texture
                            && stroke.coverage_tool_stable
                            && !stroke.program.airbrush.enabled()
                        {
                            let offset = (y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize;
                            if prior_coverage.as_ref().map_or(0., |p| p[offset].alpha())
                                >= pressure_opacity.unwrap_or(1.)
                            {
                                continue;
                            }
                        }
                        if opaque_repeat {
                            let offset = (y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize;
                            if prior_coverage
                                .as_ref()
                                .is_some_and(|p| p[offset].alpha() >= 1.)
                            {
                                continue;
                            }
                        }
                        let mut coverage = if IMAGED {
                            mask_dab
                                .as_ref()
                                .expect("compiled image dab")
                                .coverage(x as f64 + 0.5 - px, y as f64 + 0.5 - py)
                        } else {
                            stroke.program.procedural_coverage_with_geometry(
                                x as f64 + 0.5 - px,
                                y as f64 + 0.5 - py,
                                radius,
                                geometry,
                            )
                        };
                        if DUAL {
                            if dual_path {
                                let offset = (y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize;
                                coverage =
                                    stroke.program.dual.as_ref().expect("dual path").combine(
                                        coverage,
                                        secondary_coverage
                                            .as_ref()
                                            .map_or(0., |p| p[offset].alpha()),
                                    );
                            } else {
                                coverage = dual_dab.as_ref().expect("compiled dual dab").combine(
                                    coverage,
                                    x as f64 + 0.5 + f64::from(appearance.offset_x),
                                    y as f64 + 0.5 + f64::from(appearance.offset_y),
                                );
                            }
                        }
                        let texture_amount = if TEXTURED {
                            texture_dab.as_ref().expect("compiled texture").coverage(
                                x as f64 + 0.5 + f64::from(appearance.offset_x),
                                y as f64 + 0.5 + f64::from(appearance.offset_y),
                            )
                        } else {
                            1.
                        };
                        if texture_amount == 0. && (!TEXTURED || paper_texture) {
                            continue;
                        }
                        if TEXTURED && !paper_texture {
                            coverage = texture_dab.as_ref().expect("compiled texture").apply(
                                coverage,
                                x as f64 + 0.5 + f64::from(appearance.offset_x),
                                y as f64 + 0.5 + f64::from(appearance.offset_y),
                            );
                        }
                        if smudge.as_ref().is_some_and(|s| s.colored) {
                            coverage = 1.;
                        }
                        if coverage == 0. {
                            continue;
                        }
                        let offset = (y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize;
                        let before = source
                            .as_ref()
                            .map_or(self.layers[index].default_pixel(x, y), |p| p[offset]);
                        let mut amount = (if SELECTED {
                            coverage * flow * selected
                        } else {
                            coverage * flow
                        }) * if paper_texture { texture_amount } else { 1. };
                        if airbrush_tick {
                            amount = coverage
                                * flow
                                * if SELECTED { selected } else { 1. }
                                * stroke.brush.opacity
                                * pressure_opacity.unwrap_or(1.)
                                * if paper_texture { texture_amount } else { 1. };
                        }
                        let mut paint_color = stroke.color_sample;
                        let base = if airbrush_tick {
                            before
                        } else if let Some(wet) = &wet {
                            let (u, v) = geometry.normalized(
                                x as f64 + 0.5 - px,
                                y as f64 + 0.5 - py,
                                radius,
                            );
                            let ink = wet.sample(u.clamp(0., 1.), v.clamp(0., 1.));
                            let ink = wet_color.map_or(ink, |transform| transform.apply(ink));
                            if wet_smudge {
                                // Smudge transports premultiplied color at tool strength,
                                // including transparent ink; it has no paint opacity ledger.
                                paint_color = ink;
                                before
                            } else {
                                let prior = prior_coverage
                                    .as_ref()
                                    .map_or(Pixel::TRANSPARENT, |p| p[offset]);
                                let contribution = Self::wet_contribution(
                                    prior,
                                    ink,
                                    coverage * flow,
                                    pressure_opacity.unwrap_or(1.),
                                )?;
                                if contribution != prior {
                                    coverage_updates.push((offset, contribution));
                                }
                                paint_color = contribution;
                                amount = stroke.brush.opacity
                                    * selected
                                    * if paper_texture { texture_amount } else { 1. };
                                if stroke.before.contains_key(&coord) {
                                    original
                                        .as_ref()
                                        .map_or(self.layers[index].default_pixel(x, y), |p| {
                                            p[offset]
                                        })
                                } else {
                                    before
                                }
                            }
                        } else if capped {
                            let previous =
                                prior_coverage.as_ref().map_or(0., |p| p[offset].alpha());
                            let accumulated = previous
                                + coverage
                                    * flow
                                    * (pressure_opacity.unwrap_or(1.) - previous).max(0.);
                            // f32 accumulation can stop below the ceiling at low flow.
                            // With no increment, the color moment and immutable-base
                            // blend are unchanged too. Keep paper and operator changes.
                            if skip_saturated
                                && accumulated == previous
                                && !paper_texture
                                && stroke.coverage_tool_stable
                            {
                                #[cfg(test)]
                                {
                                    self.stagnant_mix_skips += 1;
                                }
                                continue;
                            }
                            // RGB carries weighted straight color; alpha remains the
                            // pressure coverage ledger. Foreground alpha is captured per stroke.
                            let cache = if color_capped {
                                let prior = prior_coverage
                                    .as_ref()
                                    .map_or(Pixel::TRANSPARENT, |p| p[offset]);
                                let c = stroke.color_sample.components();
                                let a = c[3];
                                let delta = accumulated - previous;
                                let old = prior.components();
                                let values = std::array::from_fn::<_, 4, _>(|i| {
                                    if i == 3 {
                                        accumulated
                                    } else {
                                        (old[i] + if a > 0. { c[i] / a * delta } else { 0. })
                                            .min(accumulated)
                                    }
                                });
                                let moment = Pixel::from_premultiplied(values)?;
                                if accumulated > 0. {
                                    paint_color = Pixel::from_straight([
                                        values[0] / accumulated,
                                        values[1] / accumulated,
                                        values[2] / accumulated,
                                        stroke.brush.color.alpha(),
                                    ])?;
                                }
                                moment
                            } else {
                                Pixel::from_straight([0., 0., 0., accumulated])?
                            };
                            if accumulated != previous {
                                coverage_updates.push((offset, cache));
                            }
                            amount = accumulated
                                * stroke.brush.opacity
                                * selected
                                * if paper_texture { texture_amount } else { 1. };
                            if stroke.before.contains_key(&coord) {
                                original
                                    .as_ref()
                                    .map_or(self.layers[index].default_pixel(x, y), |p| p[offset])
                            } else {
                                before
                            }
                        } else {
                            before
                        };
                        let after = if wet_smudge {
                            if mask {
                                let (hide, alpha) = Self::mask_tone(paint_color);
                                before
                                    .mix(Pixel::from_straight([0., 0., 0., hide])?, amount * alpha)
                            } else if alpha_locked {
                                before.paint_preserving_alpha(paint_color, amount)
                            } else {
                                before.mix(paint_color, amount)
                            }
                        } else if let Some(smudge) = &smudge {
                            let dragged = smudge.at(
                                x + i64::from(appearance.offset_x),
                                y + i64::from(appearance.offset_y),
                            );
                            let smudge_amount = if smudge.colored {
                                (if SELECTED { flow * selected } else { flow })
                                    * if paper_texture { texture_amount } else { 1. }
                            } else {
                                amount
                            };
                            if smudge.finger && !mask {
                                if alpha_locked {
                                    before.paint_preserving_alpha(dragged, smudge_amount)
                                } else {
                                    dragged.scaled(smudge_amount).over(before)
                                }
                            } else if alpha_locked {
                                before.paint_preserving_alpha(dragged, amount)
                            } else {
                                before.mix(
                                    dragged,
                                    amount
                                        * if smudge.finger && mask {
                                            stroke.mask_paint.expect("mask target").1
                                        } else {
                                            1.
                                        },
                                )
                            }
                        } else if mask {
                            let (tone, a) = if color_capped || wet.is_some() {
                                Self::mask_tone(paint_color)
                            } else {
                                stroke.mask_paint.expect("mask brush target")
                            };
                            let hide = if erase { 0. } else { tone };
                            let weight = amount * if erase { 1. } else { a };
                            let value = if write_blend == crate::BlendMode::Normal || erase {
                                base.alpha() * (1. - weight) + hide * weight
                            } else {
                                let gray = 1. - hide;
                                let target = 1. - base.alpha();
                                1. - crate::blend_pixel(
                                    Pixel::from_straight([gray, gray, gray, weight])?,
                                    Pixel::from_straight([target, target, target, 1.])?,
                                    write_blend,
                                    (x + i64::from(appearance.offset_x)) as u32,
                                    (y + i64::from(appearance.offset_y)) as u32,
                                    stroke.random_stroke,
                                )
                                .components()[0]
                            };
                            Pixel::from_straight([0., 0., 0., value])?
                        } else if alpha_locked {
                            if write_blend == crate::BlendMode::Normal {
                                base.paint_preserving_alpha(paint_color, amount)
                            } else {
                                base.paint_blended_preserving_alpha(
                                    paint_color,
                                    amount,
                                    write_blend,
                                    (
                                        (x + i64::from(appearance.offset_x)) as u32,
                                        (y + i64::from(appearance.offset_y)) as u32,
                                    ),
                                    stroke.random_stroke,
                                )?
                            }
                        } else if erase {
                            base.scaled(1. - amount)
                        } else {
                            if write_blend == crate::BlendMode::Normal {
                                paint_color.scaled(amount).over(base)
                            } else {
                                crate::blend_pixel(
                                    paint_color.scaled(amount),
                                    base,
                                    write_blend,
                                    (x + i64::from(appearance.offset_x)) as u32,
                                    (y + i64::from(appearance.offset_y)) as u32,
                                    stroke.random_stroke,
                                )
                            }
                        };
                        if opaque_repeat && coverage >= 1. && selected >= 1. {
                            coverage_updates
                                .push((offset, Pixel::from_straight([0., 0., 0., 1.])?));
                        }
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
                if !coverage_updates.is_empty() {
                    if stroke
                        .coverage
                        .get(&coord)
                        .is_some_and(Tile::can_edit_private)
                    {
                        Tile::edit_private(
                            stroke.coverage.get_mut(&coord).expect("coverage tile"),
                            &self.pool,
                            &coverage_updates,
                        )?;
                    } else {
                        if !stroke.coverage.contains_key(&coord)
                            && stroke.coverage.len() >= self.options.max_document_tiles
                        {
                            return Err(Error::ResourceLimit(
                                "stroke coverage tile limit; entire stroke rolled back",
                            ));
                        }
                        let mut values = prior_coverage
                            .as_ref()
                            .map_or_else(|| vec![Pixel::TRANSPARENT; 4096], |p| p.to_vec());
                        for (offset, value) in coverage_updates {
                            values[offset] = value;
                        }
                        let tile = Arc::new(Tile::from_pixels(values)?);
                        self.pool.attach(&tile)?;
                        stroke.coverage.insert(coord, tile);
                    }
                }
                drop(prior_coverage);
                drop(original);
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
        Self::commit_wet(stroke, wet)?;
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
            Command::LayerEdit {
                before,
                after,
                selection,
            } => {
                if let Some((a, b)) = selection {
                    self.selection = if forward { b.clone() } else { a.clone() };
                    self.selection_cache.clear();
                }
                let layer = if forward { after } else { before };
                for layer in layer {
                    if let Some(at) = self.layers.iter().position(|l| l.id == layer.id) {
                        self.layers[at] = layer.clone();
                    }
                }
                self.dirty_all();
            }
            Command::Selection { before, after } => {
                self.selection = if forward {
                    after.clone()
                } else {
                    before.clone()
                };
                self.selection_cache.clear();
                self.bump_revision();
            }
            Command::Structure { before, after } => {
                self.apply_structure(if forward { after } else { before });
                self.dirty_all();
            }
            Command::ReplaceBatch {
                before,
                after,
                pixels,
            } => {
                let nodes = if forward { after } else { before };
                let ids: BTreeSet<_> = nodes.iter().map(|l| l.id).collect();
                self.layers.retain(|l| ids.contains(&l.id));
                for layer in pixels {
                    if ids.contains(&layer.id) && !self.layers.iter().any(|l| l.id == layer.id) {
                        self.layers.push(layer.clone());
                    }
                }
                self.apply_structure(nodes);
                self.dirty_all();
            }
            Command::RemoveBatch {
                before,
                after,
                removed,
            } => {
                if forward {
                    let ids: BTreeSet<_> = removed.iter().map(|l| l.id).collect();
                    self.layers.retain(|l| !ids.contains(&l.id));
                    self.apply_structure(after);
                } else {
                    self.layers.extend(removed.iter().cloned());
                    self.apply_structure(before);
                }
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

#[cfg(test)]
mod dual_path_tests {
    use super::*;
    use crate::{BrushDynamics, BrushTip, DualBrush, DualMode, ScatterDynamics, SecondaryTip};
    fn definition() -> BrushDefinition {
        let tip = BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        };
        BrushDefinition {
            brush: Brush {
                radius: 8.,
                opacity: 0.4,
                ..Default::default()
            },
            tip,
            dual: DualBrush::new(
                true,
                DualMode::Multiply,
                SecondaryTip::new(
                    3.,
                    tip,
                    None,
                    BrushDynamics {
                        scatter: ScatterDynamics::new(true, 0.5, true, 3, 0.5, 31).unwrap(),
                        ..Default::default()
                    },
                )
                .unwrap()
                .with_spacing(0.13)
                .unwrap(),
            ),
            ..Default::default()
        }
    }
    #[test]
    fn secondary_sequence_and_field_do_not_follow_primary_spacing_or_scatter() {
        let mut a = Document::new(96, 64).unwrap();
        let mut b = Document::new(96, 64).unwrap();
        let mut fine = definition();
        fine.brush.spacing = 0.02;
        let mut coarse = fine.clone();
        coarse.brush.spacing = 1.;
        coarse.dynamics.scatter = ScatterDynamics::new(true, 0.5, true, 5, 0.5, 3).unwrap();
        a.begin_brush_stroke(fine, InputPoint::new(16.5, 32.5, 1.))
            .unwrap();
        b.begin_brush_stroke(coarse, InputPoint::new(16.5, 32.5, 1.))
            .unwrap();
        a.stroke_to(InputPoint::new(70.5, 32.5, 1.)).unwrap();
        b.stroke_to(InputPoint::new(70.5, 32.5, 1.)).unwrap();
        let left = a.stroke.as_ref().unwrap();
        let right = b.stroke.as_ref().unwrap();
        assert_eq!(left.dual_index, right.dual_index);
        assert!(left.dual_index > 50);
        assert_ne!(left.random_index, right.random_index);
        assert_eq!(left.dual_coverage.len(), right.dual_coverage.len());
        for (coord, tile) in &left.dual_coverage {
            assert_eq!(
                tile.try_pixels().unwrap(),
                right.dual_coverage[coord].try_pixels().unwrap()
            );
        }
        let weak = Arc::downgrade(left.dual_coverage.values().next().unwrap());
        a.end_stroke().unwrap();
        assert!(weak.upgrade().is_none());
        assert!(a.stroke.is_none());
        b.cancel_stroke().unwrap();
        assert_eq!(b.tile_count(), 0);
    }
    #[test]
    fn secondary_paging_failure_rolls_back_pixels_history_and_field() {
        let mut doc = Document::with_options(
            128,
            64,
            DocumentOptions {
                max_resident_tiles: 1,
                ..Default::default()
            },
        )
        .unwrap();
        let history = doc.history_depth();
        doc.begin_brush_stroke(definition(), InputPoint::new(32.5, 32.5, 1.))
            .unwrap();
        crate::page_pool::fail_writes_for_test(true);
        let result = doc.stroke_to(InputPoint::new(96.5, 32.5, 1.));
        crate::page_pool::fail_writes_for_test(false);
        assert!(matches!(result, Err(Error::Storage(_))));
        assert!(doc.stroke.is_none());
        assert_eq!(doc.tile_count(), 0);
        assert_eq!(doc.history_depth(), history);
        doc.begin_brush_stroke(definition(), InputPoint::new(32.5, 32.5, 1.))
            .unwrap();
        doc.end_stroke().unwrap();
        assert_eq!(doc.history_depth().0, history.0 + 1);
    }
}

#[cfg(test)]
mod mix_performance_tests {
    use super::*;
    use crate::{
        BlendMode, BrushDynamics, BrushTip, DualBrush, DualMode, PressureControl, PressureCurve,
        SecondaryTip,
    };
    #[test]
    fn saturated_optimization_preserves_all_pixels_operators_and_history() {
        for soft in [false, true] {
            for mode in 0..27 {
                for target in 0..3 {
                    for dual in 0..3 {
                        let mut fast = Document::new(32, 32).unwrap();
                        let mut reference = Document::new(32, 32).unwrap();
                        reference.reference_mix_path = true;
                        let tip = BrushTip {
                            hardness: if soft { 0.5 } else { 1. },
                            flow: if soft { 0.25 } else { 1. },
                            pressure_size: false,
                            pressure_flow: false,
                            ..Default::default()
                        };
                        let second =
                            SecondaryTip::new(8., tip, None, BrushDynamics::default()).unwrap();
                        let second = if dual == 2 {
                            second.with_spacing(0.01).unwrap()
                        } else {
                            second
                        };
                        let dynamics = BrushDynamics {
                            pressure_opacity: true,
                            opacity: PressureControl::new(PressureCurve::default(), 0.).unwrap(),
                            color: if soft {
                                crate::ColorDynamics::new(true, 0.2, 0.2, 0.2, 0.2, true, 71)
                                    .unwrap()
                            } else {
                                crate::ColorDynamics::default()
                            },
                            ..Default::default()
                        };
                        let definition = BrushDefinition {
                            brush: Brush {
                                radius: 8.,
                                spacing: 0.02,
                                opacity: 0.4,
                                color: Pixel::from_straight([0.4, 0.2, 0.7, 0.75]).unwrap(),
                                ..Default::default()
                            },
                            tip,
                            dynamics,
                            write_blend: BlendMode::from_id(mode).unwrap(),
                            dual: if dual > 0 {
                                DualBrush::new(true, DualMode::Multiply, second)
                            } else {
                                DualBrush::default()
                            },
                            ..Default::default()
                        };
                        for doc in [&mut fast, &mut reference] {
                            doc.begin_stroke(
                                Brush {
                                    radius: 30.,
                                    color: Pixel::from_straight([0.6, 0.5, 0.3, 0.37]).unwrap(),
                                    ..Default::default()
                                },
                                InputPoint::new(16.5, 16.5, 1.),
                            )
                            .unwrap();
                            doc.end_stroke().unwrap();
                            if target == 1 {
                                let mut a = doc.layer(1).unwrap().appearance();
                                a.locks = crate::LOCK_TRANSPARENCY;
                                doc.set_layer_appearance(1, a).unwrap();
                            } else if target == 2 {
                                doc.add_mask(1).unwrap();
                            }
                            doc.begin_brush_stroke(
                                definition.clone(),
                                InputPoint::new(8.5, 16.5, 1.),
                            )
                            .unwrap();
                            if soft {
                                for i in 0..40 {
                                    doc.stroke_to(InputPoint::new(
                                        if i % 2 == 0 { 24.5 } else { 8.5 },
                                        16.5,
                                        1.,
                                    ))
                                    .unwrap();
                                }
                            }
                            for (x, p, eraser) in [
                                (24.5, 1., false),
                                (8.5, 0.25, false),
                                (24.5, 0.8, false),
                                (8.5, 1., true),
                                (24.5, 1., false),
                            ] {
                                let mut point = InputPoint::new(x, 16.5, p);
                                if eraser {
                                    point.tool = Tool::Eraser;
                                }
                                doc.stroke_to(point).unwrap();
                            }
                            doc.end_stroke().unwrap();
                        }
                        if soft {
                            assert!(
                                fast.stagnant_mix_skips > 0,
                                "low-flow branch mode={mode} target={target} dual={dual}"
                            );
                        }
                        let pixels = |d: &Document| {
                            d.layers
                                .iter()
                                .map(|layer| {
                                    layer
                                        .tiles
                                        .iter()
                                        .map(|(coord, tile)| {
                                            (*coord, tile.try_pixels().unwrap().to_vec())
                                        })
                                        .collect::<Vec<_>>()
                                })
                                .collect::<Vec<_>>()
                        };
                        assert_eq!(
                            pixels(&fast),
                            pixels(&reference),
                            "mode={mode} target={target} dual={dual}"
                        );
                        assert_eq!(fast.history_depth(), reference.history_depth());
                        fast.undo().unwrap();
                        reference.undo().unwrap();
                        assert_eq!(pixels(&fast), pixels(&reference));
                        fast.redo().unwrap();
                        reference.redo().unwrap();
                        assert_eq!(pixels(&fast), pixels(&reference));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod dual_sizing_tests {
    use super::*;
    #[test]
    fn fixed_independent_first_stamp_uses_real_primary_shape_sample() {
        let mut d = Document::new(64, 64).unwrap();
        let definition = BrushDefinition {
            brush: Brush {
                radius: 8.,
                ..Default::default()
            },
            tip: crate::BrushTip {
                pressure_size: true,
                ..Default::default()
            },
            dynamics: crate::BrushDynamics {
                shape_jitter: crate::ShapeJitter::new(true, 0.8, 0.1, 0., 0., 0.01, 17).unwrap(),
                ..Default::default()
            },
            dual: crate::DualBrush::new(
                true,
                crate::DualMode::Multiply,
                crate::SecondaryTip::default().with_spacing(0.01).unwrap(),
            )
            .with_sizing(
                crate::DualSizing::new(
                    crate::DualSizeMode::Relative,
                    8.,
                    crate::DualWidthRule::FixedWidth,
                )
                .unwrap(),
            ),
            ..Default::default()
        };
        let point = InputPoint::new(32.5, 32.5, 0.25);
        d.begin_brush_stroke(definition, point).unwrap();
        let stroke = d.stroke.as_ref().unwrap();
        assert_eq!(
            stroke.shape_sample.size,
            stroke
                .program
                .shape_jitter
                .sample(stroke.random_stroke, 0, 0)
                .size
        );
        let radius = f64::from(8. * stroke.shape_sample.size * 0.25);
        let expected = stroke.program.dual.as_ref().unwrap().dab(
            point,
            None,
            stroke.random_stroke,
            0,
            0,
            radius,
            1.,
        );
        for y in 0..64 {
            for x in 0..64 {
                let coord = TileCoord::from_signed(0, 0);
                let field = stroke
                    .dual_coverage
                    .get(&coord)
                    .unwrap()
                    .try_pixels()
                    .unwrap();
                assert_eq!(
                    field[y * 64 + x].alpha(),
                    expected.coverage(0, x as f64 + 0.5, y as f64 + 0.5)
                );
            }
        }
    }
}

#[cfg(test)]
#[path = "stroke/document_input_work_tests.rs"]
mod input_work_tests;
