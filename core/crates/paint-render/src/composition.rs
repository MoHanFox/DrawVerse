//! Immutable, device-independent display inputs. This module never reads pixels
//! while building a plan and never owns a document mutation or a GPU object.
use crate::{RenderError, RenderRequest};
use paint_core::{blend_pixel, BlendMode, Layer, Pixel, Tile, TileCoord, TILE_BYTES, TILE_SIZE};
use paint_task::CancellationToken;
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Weak,
    },
};

pub const VERSION: u32 = 1;
pub const NORMAL: u64 = 1;
pub const SELF_MASK: u64 = 2;
pub const LINEAR_RGBA32F: u64 = 4;
pub const FIXED_MASK: u64 = 8;
pub const BLEND_MODES: u64 = 16;
pub const GROUPS: u64 = 32;
pub const DISSOLVE: u64 = 64;
pub const CLIPPING: u64 = 128;
pub const FEATURES: u64 =
    NORMAL | SELF_MASK | LINEAR_RGBA32F | FIXED_MASK | BLEND_MODES | GROUPS | DISSOLVE | CLIPPING;
#[path = "composition_closures.rs"]
mod closures;
pub const MAX_NODES: usize = 4096;
pub const MAX_TILES: usize = 65536;
pub const MAX_PIN_BYTES: usize = MAX_TILES * TILE_BYTES;
pub const MAX_METADATA_BYTES: usize = 128 * 1024 * 1024;
const IDENTITY_CACHE_ENTRIES: usize = MAX_TILES * 2;
static NEXT_CONTENT: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub features: u64,
    pub nodes: usize,
    pub tiles: usize,
    pub pinned_bytes: usize,
    pub metadata_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            features: FEATURES,
            nodes: MAX_NODES,
            tiles: MAX_TILES,
            pinned_bytes: MAX_PIN_BYTES,
            metadata_bytes: MAX_METADATA_BYTES,
        }
    }
}
impl Limits {
    pub fn validate(self) -> Result<(), PlanError> {
        if self.features & !FEATURES != 0
            || self.features & (NORMAL | LINEAR_RGBA32F) != NORMAL | LINEAR_RGBA32F
        {
            return Err(PlanError::Unsupported("composition capabilities"));
        }
        if self.nodes == 0
            || self.nodes > MAX_NODES
            || self.tiles > MAX_TILES
            || self.pinned_bytes > MAX_PIN_BYTES
            || self.metadata_bytes > MAX_METADATA_BYTES
            || self.metadata_bytes == 0
        {
            return Err(PlanError::Invalid);
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PlanError {
    Invalid,
    Unsupported(&'static str),
    Budget,
    Cancelled,
    Storage(String),
}
impl From<RenderError> for PlanError {
    fn from(error: RenderError) -> Self {
        match error {
            RenderError::Cancelled => Self::Cancelled,
            RenderError::ResourceLimit => Self::Budget,
            RenderError::Storage(s) => Self::Storage(s),
            RenderError::InvalidRequest => Self::Invalid,
        }
    }
}
fn checkpoint(cancel: &CancellationToken) -> Result<(), PlanError> {
    if cancel.is_cancelled() {
        Err(PlanError::Cancelled)
    } else {
        Ok(())
    }
}

/// Weak identities cannot pin pixel pages or mistake an in-place private edit
/// for old content: Arc::make_mut invalidates the Weak before editing a Tile.
/// IDs are process-unique; eviction only causes a safe cache miss.
#[derive(Default)]
pub struct ContentIdentities {
    entries: HashMap<usize, (Weak<Tile>, u64)>,
    order: VecDeque<(usize, u64)>,
}
impl ContentIdentities {
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    fn identify(&mut self, tile: &Arc<Tile>) -> Result<u64, PlanError> {
        let address = Arc::as_ptr(tile) as usize; // private lookup key, never serialized
        if let Some((weak, id)) = self.entries.get(&address) {
            if weak.upgrade().is_some_and(|old| Arc::ptr_eq(&old, tile)) {
                return Ok(*id);
            }
        }
        let mut id = NEXT_CONTENT.load(Ordering::Relaxed);
        loop {
            let next = id.checked_add(1).ok_or(PlanError::Budget)?;
            match NEXT_CONTENT.compare_exchange_weak(id, next, Ordering::Relaxed, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(current) => id = current,
            }
        }
        if self.entries.len() >= IDENTITY_CACHE_ENTRIES {
            while let Some((key, old_id)) = self.order.pop_front() {
                if self
                    .entries
                    .get(&key)
                    .is_some_and(|(_, current)| *current == old_id)
                {
                    self.entries.remove(&key);
                    break;
                }
            }
        }
        // Replacing an expired entry must remove its queue record, otherwise a
        // stream of private edits could grow the queue despite a bounded map.
        if self.entries.contains_key(&address) {
            self.order.retain(|(key, _)| *key != address);
        }
        self.entries.insert(address, (Arc::downgrade(tile), id));
        self.order.push_back((address, id));
        Ok(id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Raster,
    Mask,
    Group,
}
#[derive(Debug)]
pub struct Node {
    pub clipped: bool,
    pub dissolve_seed: u32,
    pub parent: Option<usize>,
    pub blend: BlendMode,
    pub fixed_mask: bool,
    pub layer_id: u64,
    pub kind: NodeKind,
    pub mask: Option<usize>,
    pub visible: bool,
    pub opacity: f32,
    pub fill: f32,
    pub offset: (i32, i32),
    pub white_extent: (u32, u32),
    pub first_tile: usize,
    pub tile_count: usize,
}
#[derive(Debug)]
pub struct SourceTile {
    pub content_id: u64,
    pub coord: TileCoord,
    tile: Arc<Tile>,
}
impl SourceTile {
    /// A worker-only read: may page in. One full linear little-endian tile,
    /// bounded to 64KiB; storage errors are never replaced with transparency.
    pub fn read(&self, cancel: &CancellationToken) -> Result<Vec<u8>, PlanError> {
        checkpoint(cancel)?;
        let pixels = self
            .tile
            .try_pixels()
            .map_err(|e| PlanError::Storage(e.to_string()))?;
        checkpoint(cancel)?;
        let mut bytes = Vec::with_capacity(TILE_BYTES);
        for row in pixels.chunks(TILE_SIZE as usize) {
            checkpoint(cancel)?;
            for pixel in row {
                for value in pixel.components() {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
            }
        }
        Ok(bytes)
    }
}

#[derive(Debug)]
pub struct Plan {
    pub dimensions: (u32, u32),
    pub revision: u64,
    pub request: RenderRequest,
    pub required_features: u64,
    pub nodes: Vec<Node>,
    pub operations: Vec<Operation>,
    pub surface_count: u32,
    pub output_surface: u32,
    pub tiles: Vec<SourceTile>,
    pub pinned_bytes: usize,
    pub metadata_bytes: usize,
}

/// Sequential full-resolution work. Surface indices are abstract, plan-local
/// scratch slots, never device handles. Each operation writes a distinct slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum OperationKind {
    Clear = 0,
    Copy = 1,
    Raster = 2,
    BlendSurface = 3,
    Influence = 4,
    MaskedRaster = 5,
    MaskSurface = 6,
    ClipRaster = 7,
    ClipSurface = 8,
    ClipInfluence = 9,
    ApplyPrepared = 10,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Operation {
    pub coverage: Option<u32>,
    pub kind: OperationKind,
    pub node: u32,
    pub output: u32,
    pub source: u32,
    pub backdrop: u32,
}

fn compile_groups(nodes: &[Node]) -> Result<(Vec<Operation>, u32, u32), PlanError> {
    fn stack(
        nodes: &[Node],
        children: &[Vec<usize>],
        parent: usize,
        depth: u32,
        mut current: u32,
        operations: &mut Vec<Operation>,
        surfaces: &mut u32,
    ) -> Result<u32, PlanError> {
        if depth > 16 {
            return Err(PlanError::Budget);
        }
        let first = depth * 2;
        for &i in &children[parent] {
            let node = &nodes[i];
            if !node.visible {
                continue;
            }
            let output = first + u32::from(current == first);
            if node.kind == NodeKind::Group {
                let start = (depth + 1) * 2;
                *surfaces = (*surfaces).max(start + 2);
                let through = node.blend == BlendMode::PassThrough;
                operations.push(Operation {
                    coverage: None,
                    kind: if through {
                        OperationKind::Copy
                    } else {
                        OperationKind::Clear
                    },
                    node: i as u32,
                    output: start,
                    source: current,
                    backdrop: current,
                });
                let source = stack(nodes, children, i, depth + 1, start, operations, surfaces)?;
                operations.push(Operation {
                    coverage: None,
                    kind: if through {
                        OperationKind::Influence
                    } else {
                        OperationKind::BlendSurface
                    },
                    node: i as u32,
                    output,
                    source,
                    backdrop: current,
                });
            } else {
                operations.push(Operation {
                    coverage: None,
                    kind: OperationKind::Raster,
                    node: i as u32,
                    output,
                    source: current,
                    backdrop: current,
                });
            }
            current = output;
        }
        Ok(current)
    }
    let mut children = vec![Vec::new(); nodes.len() + 1];
    for (i, node) in nodes.iter().enumerate() {
        if node.kind != NodeKind::Mask {
            children[node.parent.unwrap_or(nodes.len())].push(i);
        }
    }
    let mut operations = vec![Operation {
        coverage: None,
        kind: OperationKind::Clear,
        node: 0,
        output: 0,
        source: 1,
        backdrop: 1,
    }];
    let mut surfaces = 2;
    let output = stack(
        nodes,
        &children,
        nodes.len(),
        0,
        0,
        &mut operations,
        &mut surfaces,
    )?;
    if operations.len() > nodes.len() * 4 + 1 || surfaces > 34 {
        return Err(PlanError::Budget);
    }
    Ok((operations, surfaces, output))
}

/// Preflight before cloning any Tile Arc or allocating per-layer/tile arrays.
/// Conservative metadata includes vectors, allocator overhead and identity
/// lookup work. All nodes are checked, so unsupported closures stay on CPU.
pub fn estimate(layers: &[Layer], limits: Limits) -> Result<(usize, usize), PlanError> {
    limits.validate()?;
    if layers.len() > limits.nodes {
        return Err(PlanError::Budget);
    }
    let mut tiles = 0usize;
    for layer in layers {
        if (layer.is_group() && limits.features & GROUPS == 0)
            || (layer.is_clipped() && limits.features & CLIPPING == 0)
            || layer.is_image()
            || (layer.appearance().blend != BlendMode::Normal
                && (layer.is_mask()
                    || limits.features & BLEND_MODES == 0
                    || (layer.appearance().blend == BlendMode::Dissolve
                        && limits.features & DISSOLVE == 0)
                    || (layer.appearance().blend == BlendMode::PassThrough && !layer.is_group())))
            || (!layer.is_mask() && layer.parent_id() != 0 && limits.features & GROUPS == 0)
            || (layer.is_mask()
                && (limits.features & SELF_MASK == 0
                    || (!layer.mask_linked() && limits.features & FIXED_MASK == 0)))
        {
            return Err(PlanError::Unsupported("layer closure requires CPU"));
        }
        tiles = tiles
            .checked_add(layer.tile_count())
            .ok_or(PlanError::Budget)?;
        if tiles > limits.tiles {
            return Err(PlanError::Budget);
        }
    }
    let pins = tiles.checked_mul(TILE_BYTES).ok_or(PlanError::Budget)?;
    let metadata = 1024usize
        .checked_add(layers.len() * 1536)
        .and_then(|n| n.checked_add(tiles * 1024))
        .ok_or(PlanError::Budget)?;
    if pins > limits.pinned_bytes || metadata > limits.metadata_bytes {
        return Err(PlanError::Budget);
    }
    Ok((pins, metadata))
}

pub fn prepare(
    dimensions: (u32, u32),
    revision: u64,
    layers: &[Layer],
    request: RenderRequest,
    limits: Limits,
    identities: &mut ContentIdentities,
    cancel: &CancellationToken,
) -> Result<Plan, PlanError> {
    request.validate()?;
    checkpoint(cancel)?;
    let (pins, metadata_bytes) = estimate(layers, limits)?;
    let mut nodes = Vec::with_capacity(layers.len());
    let mut tiles = Vec::with_capacity(pins / TILE_BYTES);
    let index: HashMap<_, _> = layers
        .iter()
        .enumerate()
        .map(|(i, l)| (l.id(), i))
        .collect();
    for (at, layer) in layers.iter().enumerate() {
        checkpoint(cancel)?;
        if layer.is_mask()
            && !index
                .get(&layer.parent_id())
                .is_some_and(|i| !layers[*i].is_mask())
        {
            return Err(PlanError::Invalid);
        }
        // Follow only validated, strictly later post-order parents. Hidden
        // ancestors suppress source pins and subsequent cold reads.
        let mut ancestor = layer.parent_id();
        let mut owner_visible = true;
        let mut previous = at;
        let mut depth = 0;
        while ancestor != 0 {
            let parent = *index.get(&ancestor).ok_or(PlanError::Invalid)?;
            if parent <= previous
                || layers[parent].is_mask()
                || (!layer.is_mask() || depth > 0) && !layers[parent].is_group()
            {
                return Err(PlanError::Invalid);
            }
            depth += 1;
            if depth > 17 {
                return Err(PlanError::Budget);
            }
            let owner = &layers[parent];
            owner_visible &= owner.properties().visible
                && owner.properties().opacity * owner.appearance().fill > 0.;
            ancestor = owner.parent_id();
            previous = parent;
        }
        let visible =
            layer.properties().visible && layer.properties().opacity * layer.appearance().fill > 0.;
        nodes.push(Node {
            clipped: layer.is_clipped(),
            dissolve_seed: layer.appearance().dissolve_seed,
            parent: index.get(&layer.parent_id()).copied(),
            blend: layer.appearance().blend,
            fixed_mask: layer.is_mask() && !layer.mask_linked(),
            layer_id: layer.id(),
            kind: if layer.is_mask() {
                NodeKind::Mask
            } else if layer.is_group() {
                NodeKind::Group
            } else {
                NodeKind::Raster
            },
            mask: None,
            visible: visible && owner_visible,
            opacity: layer.properties().opacity,
            fill: layer.appearance().fill,
            offset: (layer.appearance().offset_x, layer.appearance().offset_y),
            white_extent: layer.white_extent(),
            first_tile: 0,
            tile_count: 0,
        });
    }
    for (i, layer) in layers.iter().enumerate().filter(|(_, l)| l.is_mask()) {
        let owner = *index.get(&layer.parent_id()).ok_or(PlanError::Invalid)?;
        if nodes[owner].mask.replace(i).is_some() {
            return Err(PlanError::Invalid);
        }
    }
    let grouped = layers.iter().any(Layer::is_group);
    let clipped = layers.iter().any(Layer::is_clipped);
    let dissolve = layers
        .iter()
        .any(|l| l.appearance().blend == BlendMode::Dissolve);
    let (operations, surface_count, output_surface) = if clipped {
        closures::compile(&nodes)?
    } else if grouped || dissolve {
        compile_groups(&nodes)?
    } else {
        (Vec::new(), 2, 0)
    };
    let mut needed = vec![false; nodes.len()];
    if operations.is_empty() {
        for (i, node) in nodes.iter().enumerate() {
            if node.kind == NodeKind::Raster && node.visible {
                needed[i] = true;
                if let Some(m) = node.mask {
                    needed[m] = true;
                }
            }
        }
    } else {
        for op in &operations {
            if matches!(
                op.kind,
                OperationKind::Clear | OperationKind::Copy | OperationKind::ApplyPrepared
            ) {
                continue;
            }
            let i = op.node as usize;
            if matches!(
                op.kind,
                OperationKind::Raster | OperationKind::MaskedRaster | OperationKind::ClipRaster
            ) {
                needed[i] = true;
            }
            if let Some(m) = nodes[i].mask {
                needed[m] = true;
            }
        }
    }
    for (i, (node, layer)) in nodes.iter_mut().zip(layers).enumerate() {
        node.first_tile = tiles.len();
        if needed[i] && node.visible {
            for (coord, tile) in layer.tiles() {
                checkpoint(cancel)?;
                tiles.push(SourceTile {
                    content_id: identities.identify(tile)?,
                    coord,
                    tile: Arc::clone(tile),
                });
            }
        }
        node.tile_count = tiles.len() - node.first_tile;
    }
    let required_features = if grouped { GROUPS } else { 0 }
        | if clipped { CLIPPING } else { 0 }
        | if dissolve { DISSOLVE } else { 0 }
        | if layers
            .iter()
            .any(|l| !l.is_mask() && l.appearance().blend != BlendMode::Normal)
        {
            BLEND_MODES
        } else {
            0
        }
        | if layers.iter().any(|l| l.is_mask() && !l.mask_linked()) {
            FIXED_MASK
        } else {
            0
        }
        | NORMAL
        | LINEAR_RGBA32F
        | if layers.iter().any(Layer::is_mask) {
            SELF_MASK
        } else {
            0
        };
    let pinned_bytes = tiles.len() * TILE_BYTES;
    checkpoint(cancel)?;
    Ok(Plan {
        dimensions,
        revision,
        request,
        required_features,
        nodes,
        operations,
        surface_count,
        output_surface,
        tiles,
        pinned_bytes,
        metadata_bytes,
    })
}

impl Plan {
    fn sample(&self, node: &Node, x: u32, y: u32) -> Result<Pixel, PlanError> {
        let x = i64::from(x) - i64::from(node.offset.0);
        let y = i64::from(y) - i64::from(node.offset.1);
        let coord = TileCoord::from_signed(x.div_euclid(64) as i32, y.div_euclid(64) as i32);
        let sources = &self.tiles[node.first_tile..node.first_tile + node.tile_count];
        match sources.binary_search_by_key(&coord, |source| source.coord) {
            Ok(i) => sources[i]
                .tile
                .try_pixel(x.rem_euclid(64) as u32, y.rem_euclid(64) as u32)
                .map_err(|e| PlanError::Storage(e.to_string())),
            Err(_) => Ok(
                if x >= 0
                    && y >= 0
                    && x < i64::from(node.white_extent.0)
                    && y < i64::from(node.white_extent.1)
                {
                    Pixel::WHITE
                } else {
                    Pixel::TRANSPARENT
                },
            ),
        }
    }
    /// Worker-only CPU interpretation of version 1, before LOD. This is a
    /// reference for transport/semantic tests, not a replacement frame renderer.
    pub fn try_pixel(
        &self,
        x: u32,
        y: u32,
        cancel: &CancellationToken,
    ) -> Result<Pixel, PlanError> {
        checkpoint(cancel)?;
        if x >= self.dimensions.0 || y >= self.dimensions.1 {
            return Ok(Pixel::TRANSPARENT);
        }
        let mut result = Pixel::TRANSPARENT;
        if !self.operations.is_empty() {
            let mut surfaces = vec![Pixel::TRANSPARENT; self.surface_count as usize];
            for op in &self.operations {
                checkpoint(cancel)?;
                let node = &self.nodes[op.node as usize];
                let before = surfaces[op.backdrop as usize];
                surfaces[op.output as usize] = match op.kind {
                    OperationKind::Clear => Pixel::TRANSPARENT,
                    OperationKind::Copy => before,
                    kind => {
                        let source = if matches!(
                            kind,
                            OperationKind::Raster
                                | OperationKind::MaskedRaster
                                | OperationKind::ClipRaster
                        ) {
                            self.sample(node, x, y)?
                        } else {
                            surfaces[op.source as usize]
                        };
                        let mask = if let Some(mask) =
                            node.mask.map(|i| &self.nodes[i]).filter(|m| m.visible)
                        {
                            1. - self.sample(mask, x, y)?.alpha() * mask.opacity
                        } else {
                            1.
                        };
                        if matches!(
                            kind,
                            OperationKind::MaskedRaster | OperationKind::MaskSurface
                        ) {
                            source.scaled(mask)
                        } else if matches!(
                            kind,
                            OperationKind::ClipRaster
                                | OperationKind::ClipSurface
                                | OperationKind::ClipInfluence
                        ) {
                            let coverage = op.coverage.map_or(1., |i| surfaces[i as usize].alpha());
                            clip_pixel(source, before, node, mask, coverage, x, y)
                        } else if kind == OperationKind::Influence {
                            if before == source {
                                before
                            } else {
                                before.mix(source, node.opacity * node.fill * mask)
                            }
                        } else {
                            blend_pixel(
                                source
                                    .scaled(if kind == OperationKind::ApplyPrepared {
                                        1.
                                    } else {
                                        mask
                                    })
                                    .scaled(node.opacity * node.fill),
                                before,
                                node.blend,
                                x,
                                y,
                                u64::from(node.dissolve_seed),
                            )
                        }
                    }
                };
            }
            return Ok(surfaces[self.output_surface as usize]);
        }
        for node in self
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Raster && n.visible)
        {
            let mut source = self.sample(node, x, y)?;
            if let Some(mask) = node.mask.map(|i| &self.nodes[i]).filter(|m| m.visible) {
                source = source.scaled(1. - self.sample(mask, x, y)?.alpha() * mask.opacity);
            }
            result = blend_pixel(
                source.scaled(node.opacity * node.fill),
                result,
                node.blend,
                x,
                y,
                u64::from(node.dissolve_seed),
            );
        }
        Ok(result)
    }
}

fn clip_pixel(
    source: Pixel,
    before: Pixel,
    node: &Node,
    mask: f32,
    coverage: f32,
    x: u32,
    y: u32,
) -> Pixel {
    let weight = node.opacity * node.fill;
    if node.blend == BlendMode::PassThrough {
        if before == source {
            return before;
        }
        let a = source.alpha();
        let color = if a == 0. {
            Pixel::TRANSPARENT
        } else {
            let c = source.components();
            Pixel::from_straight([c[0] / a, c[1] / a, c[2] / a, before.alpha()])
                .expect("normalized source")
        };
        return before.mix(color, weight * mask * coverage);
    }
    let a = before.alpha();
    if a == 0. {
        return before;
    }
    let source = source.scaled(mask);
    let next = if node.blend == BlendMode::Normal {
        // Reconstruct the core's preserving-alpha result through its public
        // primitive; this keeps exact finite arithmetic and normalized pixels.
        before.paint_preserving_alpha(source, weight)
    } else {
        let c = before.components();
        let opaque = Pixel::from_straight([
            (c[0] / a).clamp(0., 1.),
            (c[1] / a).clamp(0., 1.),
            (c[2] / a).clamp(0., 1.),
            1.,
        ])
        .expect("normalized backdrop");
        blend_pixel(
            source.scaled(weight),
            opaque,
            node.blend,
            x,
            y,
            u64::from(node.dissolve_seed),
        )
        .scaled(a)
    };
    before.mix(next, coverage)
}
