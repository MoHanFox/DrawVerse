use paint_ops::*;
pub fn elastic(tool: u32) -> ElasticOp {
    ElasticOp {
        grid: Grid {
            origin: [-32, -32],
            width: 33,
            height: 33,
            cell: 2.,
            vectors: vec![[0.; 2]; 33 * 33],
        },
        start: [1, 1],
        size: [31, 31],
        center: [0., 0.],
        delta: [5., 2.],
        radius: 20.,
        strength: 0.6,
        density: 0.4,
        time: 0.1,
        speed: 0.8,
        tool,
        selection: vec![1.; 31 * 31],
    }
}
#[test]
fn elastic_strength_density_and_smooth_boundary() {
    assert_eq!(smooth_kernel(1., 0.9), 0.);
    assert_eq!(smooth_kernel(0., 0.1), 1.);
    assert!(smooth_kernel(0.7, 0.2) < smooth_kernel(0.7, 0.8));
    assert!(smooth_kernel(0.9999, 0.8) < 0.00001);
    let v = elastic(0);
    let full = Cpu.execute(&Operator::Elastic(v.clone())).unwrap();
    let mut low = v.clone();
    low.strength *= 0.5;
    let low = Cpu.execute(&Operator::Elastic(low)).unwrap();
    let center = 15 * 31 + 15;
    assert!((full[center][0] - low[center][0] * 2.).abs() < 1e-6);
    assert_eq!(full[0], [0.; 4]);
}
#[test]
fn identity_remap_preserves_sparse_white_and_negative_tiles() {
    let mut source = vec![[0.1, 0.2, 0.3, 0.5]; 4096];
    source[63] = [0.; 4];
    let op = RemapOp {
        grid: Grid {
            origin: [-70, -6],
            width: 72,
            height: 38,
            cell: 2.,
            vectors: vec![[0.; 2]; 72 * 38],
        },
        source: vec![SourceTile {
            coord: [-1, 0],
            pixels: source.clone(),
        }],
        white: [64, 64],
        tiles: vec![[-1, 0], [0, 0]],
        selection: vec![],
        stride: 2,
        cubic: false,
    };
    let out = Cpu.execute(&Operator::Remap(op)).unwrap();
    assert_eq!(&out[..4096], source);
    assert!(out[4096..].iter().all(|v| *v == [1.; 4]));
}
#[test]
fn invalid_requests_reject_before_execution() {
    let mut v = elastic(0);
    v.selection[1] = f32::NAN;
    assert!(Cpu.execute(&Operator::Elastic(v)).is_err());
    let mut v = elastic(0);
    v.start[0] = u32::MAX;
    assert!(Cpu.execute(&Operator::Elastic(v)).is_err());
}
#[test]
fn reconstruct_restores_grid_exactly() {
    let mut v = elastic(3);
    v.grid.vectors.fill([3., -4.]);
    v.strength = 1.;
    v.density = 1.;
    let out = Cpu.execute(&Operator::Elastic(v)).unwrap();
    assert_eq!(out[15 * 31 + 15], [0.; 4]);
}

#[test]
fn fractional_remap_matches_independent_reads_across_negative_tile_and_white_edges() {
    let source = (0..4096)
        .map(|i| [(i % 64) as f32 / 128., 0.1, 0.2, 0.7])
        .collect::<Vec<_>>();
    let read = |x: i32, y: i32| {
        if (-64..0).contains(&x) && (0..64).contains(&y) {
            source[(y * 64 + x + 64) as usize]
        } else if (0..64).contains(&x) && (0..64).contains(&y) {
            [1.; 4]
        } else {
            [0.; 4]
        }
    };
    let mut op = RemapOp {
        grid: Grid {
            origin: [-70, -6],
            width: 72,
            height: 38,
            cell: 2.,
            vectors: vec![[0.25, 0.75]; 72 * 38],
        },
        source: vec![SourceTile {
            coord: [-1, 0],
            pixels: source.clone(),
        }],
        white: [64, 64],
        tiles: vec![[-1, 0], [0, 0]],
        selection: vec![0.5; 8192],
        stride: 1,
        cubic: false,
    };
    let actual = Cpu.execute(&Operator::Remap(op.clone())).unwrap();
    for (i, p) in actual.iter().enumerate() {
        let x = (i / 4096) as i32 * 64 - 64 + (i % 64) as i32;
        let y = (i % 4096 / 64) as i32;
        let a = read(x, y);
        let b = read(x + 1, y);
        let c = read(x, y + 1);
        let d = read(x + 1, y + 1);
        for k in 0..4 {
            let expected = a[k] * 0.5
                + ((a[k] * 0.75 + b[k] * 0.25) * 0.25 + (c[k] * 0.75 + d[k] * 0.25) * 0.75) * 0.5;
            assert!((p[k] - expected).abs() < 1e-6, "pixel={i} channel={k}");
        }
    }
    op.cubic = true;
    let actual = Cpu.execute(&Operator::Remap(op)).unwrap();
    assert_eq!(
        actual[4096 + 32 * 64 + 32],
        [1.; 4],
        "uniform cubic white must be exact"
    );
    for p in actual {
        assert!((0. ..=1.).contains(&p[3]));
        assert!(p[..3].iter().all(|c| *c >= 0. && *c <= p[3]));
    }
}
