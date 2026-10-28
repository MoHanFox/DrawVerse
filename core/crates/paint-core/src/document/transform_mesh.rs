//! One piecewise-affine surface for resident display and full-source Warp sampling.
use super::*;

pub(super) struct TransformMesh {
    pub columns: u32,
    pub rows: u32,
    /// Destination x/y and normalised source u/v, matching the presentation packet precision.
    pub vertices: Vec<[f32; 4]>,
}
impl TransformMesh {
    pub fn new(geometry: &TransformGeometry) -> Self {
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
        // Include every interpolating control node and preserve the existing interaction budget.
        let columns = ((extent(0) / 18.).ceil() as u32).clamp(6, 48) * 3 + 1;
        let rows = ((extent(1) / 18.).ceil() as u32).clamp(6, 48) * 3 + 1;
        let mut vertices = Vec::with_capacity((columns * rows) as usize);
        // map() repeats these six extended-row interpolations for every y.
        // Cache the same expressions, preserving their f64 evaluation order.
        let horizontal: Vec<[[f64; 2]; 6]> = if geometry.warp {
            (0..columns)
                .map(|x| {
                    let u = x as f64 / (columns - 1) as f64;
                    let (ix, tx) = cell(u);
                    std::array::from_fn(|r| {
                        std::array::from_fn(|axis| {
                            catmull(
                                std::array::from_fn(|k| {
                                    geometry.extended_node(ix + k as i32 - 1, r as i32 - 1, axis)
                                }),
                                tx,
                            )
                        })
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        // Hoist projective coefficients out of the dense display mesh loop.
        // Keep map() as the independent scalar reference.
        let projective = if !geometry.warp {
            let [a, b, c, d] = [
                geometry.points[0],
                geometry.points[3],
                geometry.points[15],
                geometry.points[12],
            ];
            let dx = b[0] + d[0] - a[0] - c[0];
            let dy = b[1] + d[1] - a[1] - c[1];
            let bx = c[0] - b[0];
            let by = c[1] - b[1];
            let cx = c[0] - d[0];
            let cy = c[1] - d[1];
            let det = bx * cy - cx * by;
            let (g, h) = if dx.abs() + dy.abs() < 1e-10 {
                (0., 0.)
            } else if det.abs() > 1e-12 {
                ((dx * cy - cx * dy) / det, (bx * dy - dx * by) / det)
            } else {
                (0., 0.)
            };
            [
                b[0] - a[0] + g * b[0],
                d[0] - a[0] + h * d[0],
                a[0],
                b[1] - a[1] + g * b[1],
                d[1] - a[1] + h * d[1],
                a[1],
                g,
                h,
            ]
        } else {
            [0.; 8]
        };
        for y in 0..rows {
            let v = y as f64 / (rows - 1) as f64;
            for x in 0..columns {
                let u = x as f64 / (columns - 1) as f64;
                let p = if geometry.warp {
                    let (iy, ty) = cell(v);
                    std::array::from_fn(|axis| {
                        catmull(
                            std::array::from_fn(|r| horizontal[x as usize][iy as usize + r][axis]),
                            ty,
                        )
                    })
                } else {
                    let m = projective;
                    let z = 1. + m[6] * u + m[7] * v;
                    [
                        (m[0] * u + m[1] * v + m[2]) / z,
                        (m[3] * u + m[4] * v + m[5]) / z,
                    ]
                };
                vertices.push([p[0] as f32, p[1] as f32, u as f32, v as f32]);
            }
        }
        Self {
            columns,
            rows,
            vertices,
        }
    }
    pub fn bounds(&self) -> [f64; 4] {
        self.vertices.iter().fold(
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
            |b, p| {
                [
                    b[0].min(p[0] as f64),
                    b[1].min(p[1] as f64),
                    b[2].max(p[0] as f64),
                    b[3].max(p[1] as f64),
                ]
            },
        )
    }
}

struct Triangle {
    vertices: [[f64; 4]; 3],
    area: f64,
    bounds: [i64; 4],
}
fn edge(a: [f64; 4], b: [f64; 4], x: f64, y: f64) -> f64 {
    (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
}
fn top_left(a: [f64; 4], b: [f64; 4]) -> bool {
    b[1] < a[1] || (b[1] == a[1] && b[0] > a[0])
}
impl Triangle {
    fn new(vertices: [[f32; 4]; 3]) -> Option<Self> {
        let mut vertices = vertices.map(|v| v.map(f64::from));
        let area = edge(vertices[0], vertices[1], vertices[2][0], vertices[2][1]);
        if area.abs() < 1e-12 {
            return None;
        }
        if area < 0. {
            vertices.swap(1, 2);
        }
        let bounds = [
            vertices
                .iter()
                .map(|v| v[0])
                .fold(f64::INFINITY, f64::min)
                .floor() as i64,
            vertices
                .iter()
                .map(|v| v[1])
                .fold(f64::INFINITY, f64::min)
                .floor() as i64,
            vertices
                .iter()
                .map(|v| v[0])
                .fold(f64::NEG_INFINITY, f64::max)
                .ceil() as i64,
            vertices
                .iter()
                .map(|v| v[1])
                .fold(f64::NEG_INFINITY, f64::max)
                .ceil() as i64,
        ];
        Some(Self {
            vertices,
            area: area.abs(),
            bounds,
        })
    }
    fn uv(&self, x: f64, y: f64) -> Option<[f64; 2]> {
        let [a, b, c] = self.vertices;
        let e = [edge(b, c, x, y), edge(c, a, x, y), edge(a, b, x, y)];
        let inclusive = [top_left(b, c), top_left(c, a), top_left(a, b)];
        if e.iter()
            .zip(inclusive)
            .any(|(&v, include)| v < 0. || (v == 0. && !include))
        {
            return None;
        }
        Some(std::array::from_fn(|axis| {
            (e[0] * a[axis + 2] + e[1] * b[axis + 2] + e[2] * c[axis + 2]) / self.area
        }))
    }
}

/// Tile-local UV ownership, not an unbounded full-image displacement buffer.
pub(super) struct MeshRaster {
    triangles: Vec<Triangle>,
    bins: Vec<Vec<u32>>,
    tx0: i64,
    ty0: i64,
    columns: usize,
    rows: usize,
}
impl MeshRaster {
    pub fn new(mesh: &TransformMesh, extent: [i64; 4]) -> Result<Self> {
        let [x0, y0, x1, y1] = extent;
        let tx0 = x0.div_euclid(64);
        let ty0 = y0.div_euclid(64);
        let columns = ((x1 - 1).div_euclid(64) - tx0 + 1) as usize;
        let rows = ((y1 - 1).div_euclid(64) - ty0 + 1) as usize;
        if columns.saturating_mul(rows) > 524_288 {
            return Err(Error::ResourceLimit("transform mesh bins"));
        }
        let mut result = Self {
            triangles: Vec::new(),
            bins: vec![Vec::new(); columns * rows],
            tx0,
            ty0,
            columns,
            rows,
        };
        let mut references = 0usize;
        for y in 0..mesh.rows - 1 {
            for x in 0..mesh.columns - 1 {
                let a = (y * mesh.columns + x) as usize;
                let c = a + mesh.columns as usize;
                for indices in [[a, a + 1, c + 1], [a, c + 1, c]] {
                    let Some(triangle) = Triangle::new(indices.map(|i| mesh.vertices[i])) else {
                        continue;
                    };
                    let b = triangle.bounds;
                    let left = b[0].max(x0);
                    let top = b[1].max(y0);
                    let right = b[2].min(x1);
                    let bottom = b[3].min(y1);
                    if left >= right || top >= bottom {
                        continue;
                    }
                    let id = result.triangles.len() as u32;
                    for ty in top.div_euclid(64)..=(bottom - 1).div_euclid(64) {
                        for tx in left.div_euclid(64)..=(right - 1).div_euclid(64) {
                            references += 1;
                            if references > 4_194_304 {
                                return Err(Error::ResourceLimit("transform mesh references"));
                            }
                            result.bins[(ty - ty0) as usize * columns + (tx - tx0) as usize]
                                .push(id);
                        }
                    }
                    result.triangles.push(triangle);
                }
            }
        }
        Ok(result)
    }
    pub fn tile_uvs(&self, tx: i64, ty: i64) -> Vec<[f64; 2]> {
        let mut uv = vec![[f64::NAN; 2]; 4096];
        let col = tx - self.tx0;
        let row = ty - self.ty0;
        if col < 0 || row < 0 || col as usize >= self.columns || row as usize >= self.rows {
            return uv;
        }
        for &id in &self.bins[row as usize * self.columns + col as usize] {
            let t = &self.triangles[id as usize];
            for y in t.bounds[1].max(ty * 64)..t.bounds[3].min((ty + 1) * 64) {
                for x in t.bounds[0].max(tx * 64)..t.bounds[2].min((tx + 1) * 64) {
                    if let Some(p) = t.uv(x as f64 + 0.5, y as f64 + 0.5) {
                        // Later row-major triangles own folded pixels, including transparent ones.
                        uv[(y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize] = p;
                    }
                }
            }
        }
        uv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_mesh_matches_original_map_bits() {
        for side in [96., 1024., 4096.] {
            for mode in 0..4 {
                let mut g = TransformGeometry::rectangle(-137.25, -19.75, side, side * 0.7);
                g.warp = mode < 2;
                if mode == 3 {
                    g.points[3][0] += side * 0.17;
                    g.points[15][1] += side * 0.11;
                }
                g.points[5][0] += side * if mode == 1 { 1.3 } else { 0.13 };
                g.points[10][1] -= side * 0.21;
                let mesh = TransformMesh::new(&g);
                for y in 0..mesh.rows {
                    for x in 0..mesh.columns {
                        let u = x as f64 / (mesh.columns - 1) as f64;
                        let v = y as f64 / (mesh.rows - 1) as f64;
                        let expected = g.map(u, v);
                        let actual = mesh.vertices[(y * mesh.columns + x) as usize];
                        assert_eq!(
                            actual.map(f32::to_bits),
                            [expected[0] as f32, expected[1] as f32, u as f32, v as f32]
                                .map(f32::to_bits)
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn shared_edges_tiles_negative_coordinates_and_reversed_winding_have_no_holes() {
        for reflected in [false, true] {
            let mut g = TransformGeometry::rectangle(-16., -16., 128., 96.);
            g.warp = true;
            if reflected {
                for p in &mut g.points {
                    p[0] = 96. - p[0];
                }
            }
            let mesh = TransformMesh::new(&g);
            let raster = MeshRaster::new(&mesh, [-16, -16, 112, 80]).unwrap();
            for ty in -1..=1 {
                for tx in -1..=1 {
                    let tile = raster.tile_uvs(tx, ty);
                    for y in (-16).max(ty * 64)..80.min((ty + 1) * 64) {
                        for x in (-16).max(tx * 64)..112.min((tx + 1) * 64) {
                            let uv = tile[(y.rem_euclid(64) * 64 + x.rem_euclid(64)) as usize];
                            let u = if reflected {
                                (112. - x as f64 - 0.5) / 128.
                            } else {
                                (x as f64 + 16.5) / 128.
                            };
                            assert!(
                                (uv[0] - u).abs() < 1e-6
                                    && (uv[1] - (y as f64 + 16.5) / 96.).abs() < 1e-6,
                                "{reflected} {x},{y} {uv:?}"
                            );
                        }
                    }
                }
            }
        }
        let mut g = TransformGeometry::rectangle(0., 0., 96., 96.);
        g.warp = true;
        g.points.fill([32., 32.]);
        let raster = MeshRaster::new(&TransformMesh::new(&g), [0, 0, 96, 96]).unwrap();
        assert!(raster.tile_uvs(0, 0).iter().all(|uv| uv[0].is_nan()));
    }
}
