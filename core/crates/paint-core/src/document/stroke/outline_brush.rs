//! Paint a captured secondary tip along closed contours, then clip to the outline band.
use super::*;

type Point = (u32, u32);
const MAX_CONTOUR_EDGES: usize = 65_536;

fn direction(a: Point, b: Point) -> usize {
    if b.0 > a.0 {
        0
    } else if b.1 > a.1 {
        1
    } else if b.0 < a.0 {
        2
    } else {
        3
    }
}

fn contours(width: usize, height: usize, shape: &[u8]) -> Result<Vec<Vec<[f64; 2]>>> {
    let inside = |x: i64, y: i64| {
        x >= 0
            && y >= 0
            && x < width as i64
            && y < height as i64
            && shape[y as usize * width + x as usize] >= 128
    };
    let mut edges: BTreeMap<Point, Vec<Point>> = BTreeMap::new();
    let mut count = 0;
    for (i, &value) in shape.iter().enumerate().filter(|(_, v)| **v >= 128) {
        let _ = value;
        let (x, y) = ((i % width) as u32, (i / width) as u32);
        for (outside, start, end) in [
            (!inside(i64::from(x), i64::from(y) - 1), (x, y), (x + 1, y)),
            (
                !inside(i64::from(x) + 1, i64::from(y)),
                (x + 1, y),
                (x + 1, y + 1),
            ),
            (
                !inside(i64::from(x), i64::from(y) + 1),
                (x + 1, y + 1),
                (x, y + 1),
            ),
            (!inside(i64::from(x) - 1, i64::from(y)), (x, y + 1), (x, y)),
        ] {
            if !outside {
                continue;
            }
            if count >= MAX_CONTOUR_EDGES {
                return Err(Error::ResourceLimit("stroke outline contour edge limit"));
            }
            edges.entry(start).or_default().push(end);
            count += 1;
        }
    }
    let mut output = Vec::new();
    while let Some((&start, _)) = edges.first_key_value() {
        let mut current = start;
        let mut previous_direction = 0;
        let mut path = vec![start];
        loop {
            let options = edges
                .get_mut(&current)
                .ok_or(Error::InvalidArgument("open stroke outline contour"))?;
            // At diagonally touching pixels, turn right to keep each occupied cell
            // on the right. This closes distinct loops instead of crossing them.
            let next = options
                .iter()
                .enumerate()
                .min_by_key(|(_, next)| {
                    match (direction(current, **next) + 4 - previous_direction) % 4 {
                        1 => 0,
                        0 => 1,
                        3 => 2,
                        _ => 3,
                    }
                })
                .map(|(i, _)| i)
                .expect("nonempty edge list");
            let next = options.swap_remove(next);
            if options.is_empty() {
                edges.remove(&current);
            }
            previous_direction = direction(current, next);
            current = next;
            path.push(current);
            if current == start {
                break;
            }
        }
        let mut simplified = Vec::new();
        for i in 0..path.len() - 1 {
            let prev = path[if i == 0 { path.len() - 2 } else { i - 1 }];
            let next = path[i + 1];
            if direction(prev, path[i]) != direction(path[i], next) {
                simplified.push([f64::from(path[i].0), f64::from(path[i].1)]);
            }
        }
        if simplified.len() >= 3 {
            output.push(simplified);
        }
    }
    Ok(output)
}

fn offset_path(path: &mut [[f64; 2]], distance: f64) {
    if distance == 0. {
        return;
    }
    let original = path.to_vec();
    let unit = |a: [f64; 2], b: [f64; 2]| {
        let length = (b[0] - a[0]).hypot(b[1] - a[1]);
        [(b[0] - a[0]) / length, (b[1] - a[1]) / length]
    };
    for (i, point) in path.iter_mut().enumerate() {
        let a = unit(
            original[(i + original.len() - 1) % original.len()],
            original[i],
        );
        let b = unit(original[i], original[(i + 1) % original.len()]);
        let scale = distance / (1. + a[0] * b[0] + a[1] * b[1]).max(0.5);
        point[0] += (a[1] + b[1]) * scale;
        point[1] -= (a[0] + b[0]) * scale;
    }
}

pub(super) fn apply_brush(
    width: usize,
    height: usize,
    shape: &[u8],
    band: &mut [f32],
    settings: &StrokeOutline,
    band_width: f32,
    options: DocumentOptions,
) -> Result<()> {
    let OutlineSource::Existing(tip) = settings.source() else {
        return Ok(());
    };
    let mut paths = contours(width, height, shape)?;
    let (outside, inside) = settings.placement().extents(band_width);
    for path in &mut paths {
        offset_path(path, f64::from(outside - inside) / 2.);
    }
    let definition = BrushDefinition {
        brush: Brush {
            radius: tip.radius(),
            spacing: tip.spacing().unwrap_or(0.25) * 2.,
            color: Pixel::WHITE,
            ..Default::default()
        },
        tip: tip.tip(),
        mask: tip.mask().cloned(),
        dynamics: tip.dynamics().clone(),
        ..Default::default()
    };
    let program = BrushProgram::compile(definition.clone())?;
    let (work, _) = Document::stroke_input_work(definition.brush, &program, 0);
    let step = program.step(definition.brush);
    let dabs: usize = paths
        .iter()
        .map(|path| {
            path.iter()
                .zip(path.iter().cycle().skip(1))
                .take(path.len())
                .map(|(a, b)| ((b[0] - a[0]).hypot(b[1] - a[1]) / step).ceil() as usize + 2)
                .sum::<usize>()
        })
        .sum();
    if dabs.saturating_mul(work.visits) > crate::MAX_INPUT_SCHEDULE_VISITS
        || dabs.saturating_mul(work.dabs) > crate::MAX_INPUT_SCHEDULE_DABS
    {
        return Err(Error::ResourceLimit(
            "stroke outline brush total work limit",
        ));
    }
    let mut paint = Document::with_options(width as u32, height as u32, options)?;
    for path in paths {
        let input = |p: [f64; 2]| InputPoint::new(p[0], p[1], 1.);
        paint.begin_brush_stroke(definition.clone(), input(path[0]))?;
        for point in path.iter().skip(1).chain(path.first()) {
            paint.begin_stroke_input(input(*point))?;
            while matches!(
                paint.continue_stroke_page(crate::InputScheduleBudget::default())?,
                crate::StrokePageResult::Pending
            ) {}
        }
        paint.begin_stroke_finish()?;
        while matches!(
            paint.continue_stroke_page(crate::InputScheduleBudget::default())?,
            crate::StrokePageResult::Pending
        ) {}
        paint.history = History::default();
    }
    for ty in 0..height.div_ceil(64) {
        for tx in 0..width.div_ceil(64) {
            let coord = TileCoord::from_signed(tx as i32, ty as i32);
            let pixels = paint.layers[0]
                .tiles
                .get(&coord)
                .map(|t| t.try_pixels())
                .transpose()?;
            for y in ty * 64..((ty + 1) * 64).min(height) {
                for x in tx * 64..((tx + 1) * 64).min(width) {
                    band[y * width + x] *= pixels
                        .as_ref()
                        .map_or(0., |p| p[(y % 64) * 64 + x % 64].alpha());
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contour_paths_preserve_holes_and_split_diagonal_contacts() {
        let shape = [
            255, 255, 255, 0, 255, 0, 255, 0, 255, 255, 255, 0, 0, 0, 0, 255,
        ];
        let paths = contours(4, 4, &shape).unwrap();
        assert_eq!(paths.len(), 3);
        assert!(paths.iter().all(|p| p.len() == 4));
        let mut signed_areas: Vec<_> = paths
            .iter()
            .map(|path| {
                path.iter()
                    .zip(path.iter().cycle().skip(1))
                    .take(path.len())
                    .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
                    .sum::<f64>()
                    / 2.
            })
            .collect();
        signed_areas.sort_by(f64::total_cmp);
        assert_eq!(signed_areas, [-1., 1., 9.]);
    }
}
