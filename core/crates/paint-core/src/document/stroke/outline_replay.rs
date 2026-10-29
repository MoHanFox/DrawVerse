//! Replay the resolved primary actions, without scheduling another contour path.
use super::*;
use paint_ops::replay_outline::{self as operator, ReplayDab, Source};

pub(super) fn accumulate(
    document: &Document,
    stroke: &Stroke,
    bounds: (f64, f64, f64, f64),
    point: InputPoint,
    radius: f64,
    sample: impl Fn(f64, f64) -> Result<f32>,
) -> Result<ShapeUpdates> {
    let outline = stroke.outline.as_ref().expect("outline stroke");
    let (outside, inside) = outline.settings.placement().extents(outline.width);
    let mut extent = [
        ((point.x - bounds.0).max(bounds.2 - point.x) - 1.).max(0.),
        ((point.y - bounds.1).max(bounds.3 - point.y) - 1.).max(0.),
    ];
    let paint = outline.replay_program.as_ref();
    let paint_radius = match outline.settings.source() {
        OutlineSource::Existing(tip) => radius * f64::from(tip.radius() / stroke.brush.radius),
        _ => radius,
    };
    let paint_geometry = paint.map(|p| {
        p.dab_geometry(
            point,
            if stroke.program.tip_dynamics.angle() == crate::AngleControl::InitialDirection {
                stroke.initial_direction
            } else {
                stroke.tip_direction
            },
            stroke.shape_sample,
        )
    });
    if let Some((program, geometry)) = paint.zip(paint_geometry) {
        let (x, y) = program.extent(paint_radius, geometry);
        extent[0] = extent[0].max(x);
        extent[1] = extent[1].max(y);
    }
    let scale = (radius + f64::from(outside)) / radius;
    let (left, top, right, bottom) = (
        (point.x - extent[0] * scale - 1.).floor().max(0.) as i64,
        (point.y - extent[1] * scale - 1.).floor().max(0.) as i64,
        (point.x + extent[0] * scale + 1.)
            .ceil()
            .min(f64::from(document.width)) as i64,
        (point.y + extent[1] * scale + 1.)
            .ceil()
            .min(f64::from(document.height)) as i64,
    );
    if left >= right || top >= bottom {
        return Ok(Vec::new());
    }
    let count = (right - left) as usize * (bottom - top) as usize;
    if count > MAX_STROKE_OUTLINE_PIXELS {
        return Err(Error::ResourceLimit("replayed outline dab pixel limit"));
    }
    let shared = matches!(outline.settings.source(), OutlineSource::SelfBrush);
    let dab = ReplayDab {
        center: [point.x, point.y],
        radius,
        outside: f64::from(outside),
        inside: f64::from(inside),
        shared_source: shared,
    };
    let mask = paint.zip(paint_geometry).and_then(|(p, g)| {
        p.has_mask()
            .then(|| p.mask_dab_with_geometry(paint_radius, g))
            .flatten()
    });
    let texture = match outline.settings.source() {
        OutlineSource::Texture(t) | OutlineSource::SelfTextured(t) => t.compile(),
        _ => None,
    };
    let geometry = stroke.program.dab_geometry(
        point,
        if stroke.program.tip_dynamics.angle() == crate::AngleControl::InitialDirection {
            stroke.initial_direction
        } else {
            stroke.tip_direction
        },
        stroke.shape_sample,
    );
    let texture = texture.as_ref().map(|t| geometry.texture_dab(t, point));
    let paint_texture = paint
        .zip(paint_geometry)
        .and_then(|(p, g)| p.texture_dab(point, g));
    let mut updates = Vec::new();
    for ty in top.div_euclid(64)..=(bottom - 1).div_euclid(64) {
        for tx in left.div_euclid(64)..=(right - 1).div_euclid(64) {
            let coord = TileCoord::from_signed(tx as i32, ty as i32);
            let previous = outline.shape.get(&coord);
            let (x0, y0, x1, y1) = (
                left.max(tx * 64),
                top.max(ty * 64),
                right.min((tx + 1) * 64),
                bottom.min((ty + 1) * 64),
            );
            let mut tile = ShapeCoverage {
                rows: previous.map_or([0; 64], |p| p.rows),
                replay: Vec::new(),
            };
            tile.replay
                .try_reserve_exact(4096)
                .map_err(|_| Error::ResourceLimit("replayed outline tile allocation"))?;
            if let Some(previous) = previous {
                tile.replay.extend_from_slice(&previous.replay);
            } else {
                tile.replay.resize(4096, [0; 4]);
            }
            let mut input = Vec::new();
            input
                .try_reserve_exact(((x1 - x0) * (y1 - y0)) as usize)
                .map_err(|_| Error::ResourceLimit("replayed outline scratch allocation"))?;
            for y in y0..y1 {
                for x in x0..x1 {
                    input.push(tile.replay[((y - ty * 64) * 64 + x - tx * 64) as usize]);
                }
            }
            // The operator error channel cannot carry document errors. Preserve the
            // original failure (e.g. a paged dual mask read) instead of hiding it.
            let mut failure = None;
            let output = operator::accumulate(
                &dab,
                [x0 as i32, y0 as i32],
                [(x1 - x0) as u32, (y1 - y0) as u32],
                &input,
                |source, p| {
                    let coverage = (|| -> Result<f32> {
                        if source == Source::Primary {
                            return sample(p[0], p[1]);
                        }
                        let coverage = if let Some((program, geometry)) = paint.zip(paint_geometry)
                        {
                            let coverage = mask.as_ref().map_or_else(
                                || {
                                    program.procedural_coverage_with_geometry(
                                        p[0] - point.x,
                                        p[1] - point.y,
                                        paint_radius,
                                        geometry,
                                    )
                                },
                                |m| m.coverage(p[0] - point.x, p[1] - point.y),
                            );
                            paint_texture
                                .as_ref()
                                .map_or(coverage, |t| t.apply(coverage, p[0], p[1]))
                        } else {
                            sample(p[0], p[1])?
                        };
                        Ok(texture
                            .as_ref()
                            .map_or(coverage, |t| t.apply(coverage, p[0], p[1])))
                    })();
                    coverage.map_err(|error| {
                        failure = Some(error);
                        "replayed outline source sampling"
                    })
                },
            );
            if let Some(error) = failure {
                return Err(error);
            }
            let output = output.map_err(|_| Error::InvalidArgument("replayed outline operator"))?;
            for y in y0..y1 {
                for x in x0..x1 {
                    let value = output[((y - y0) * (x1 - x0) + x - x0) as usize];
                    tile.replay[((y - ty * 64) * 64 + x - tx * 64) as usize] = value;
                    if value[0] != 0 || value[2] != 0 {
                        tile.rows[(y - ty * 64) as usize] |= 1 << (x - tx * 64);
                    }
                }
            }
            if previous.is_some_and(|old| **old == tile) || tile.rows.iter().all(|row| *row == 0) {
                continue;
            }
            updates.push((coord, tile));
        }
    }
    Ok(updates)
}

pub(super) fn compose(
    outline: &OutlineStroke,
    left: i64,
    top: i64,
    width: usize,
    height: usize,
) -> Result<Arc<[f32]>> {
    let mut band = Vec::new();
    band.try_reserve_exact(width * height)
        .map_err(|_| Error::ResourceLimit("replayed outline compose allocation"))?;
    band.resize(width * height, 0.);
    for (coord, tile) in &outline.shape {
        let (tx, ty) = (
            i64::from(coord.signed_x()) * 64,
            i64::from(coord.signed_y()) * 64,
        );
        for (y, row) in tile.rows.iter().enumerate() {
            let mut bits = *row;
            while bits != 0 {
                let x = bits.trailing_zeros() as usize;
                band[(ty + y as i64 - top) as usize * width + (tx + x as i64 - left) as usize] =
                    operator::coverage(tile.replay[y * 64 + x], outline.settings.allow_overflow());
                bits &= bits - 1;
            }
        }
    }
    Ok(Arc::from(band))
}
