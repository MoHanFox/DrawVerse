use paint_gpu::*;
use paint_ops::*;

fn elastic(tool: u32) -> Operator {
    Operator::Elastic(ElasticOp {
        grid: Grid {
            origin: [-32, -32],
            width: 33,
            height: 33,
            cell: 2.,
            vectors: (0..33 * 33)
                .map(|i| [(i % 33) as f32 * 0.03, (i / 33) as f32 * -0.02])
                .collect(),
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
    })
}
#[test]
fn missing_hardware_falls_back_to_identical_cpu() {
    let fallback = Accelerated::new("missing-device-for-test");
    let op = elastic(0);
    assert_eq!(fallback.execute(&op).unwrap(), Cpu.execute(&op).unwrap());
    assert!(fallback.status().starts_with("CPU"));
}
#[test]
fn actual_gpu_matches_cpu_elastic_and_remap() {
    let devices = hardware();
    if devices.is_empty() {
        eprintln!("No hardware adapter; GPU numerical comparison not run");
        return;
    }
    let gpu = Gpu::open(&devices[0].id).expect("available hardware must compile real shaders");
    eprintln!("Compute device: {}", gpu.name());
    let mut ops = (0..4).map(elastic).collect::<Vec<_>>();
    for (cubic, stride) in [(false, 1), (true, 1), (false, 2), (true, 4)] {
        ops.push(Operator::Remap(RemapOp {
            grid: Grid {
                origin: [-70, -8],
                width: 72,
                height: 40,
                cell: 2.,
                vectors: vec![[0.3, -0.7]; 72 * 40],
            },
            source: vec![
                SourceTile::new(
                    [-1, 0],
                    (0..4096)
                        .map(|i| [(i % 64) as f32 / 128., 0.1, 0.2, 0.7])
                        .collect(),
                ),
                SourceTile::new([0, 0], vec![[0.5, 0.2, 0.1, 0.7]; 4096]),
            ],
            white: [64, 64],
            tiles: vec![[-1, 0], [0, 0]],
            selection: (0..8192).map(|i| (i % 5) as f32 / 4.).collect(),
            stride,
            cubic,
        }));
    }
    let Operator::Remap(template) = ops.last().unwrap() else {
        unreachable!()
    };
    let mut preview = template.clone();
    preview.cubic = false;
    preview.selection.clear();
    for v in preview.grid.vectors.iter_mut().take(600) {
        *v = [0.; 2];
    }
    for stride in [2, 4] {
        preview.stride = stride;
        ops.push(Operator::RemapBlocks(preview.clone()));
    }
    gpu.prepare_sources(&preview.source).unwrap();
    preview.source[1] = SourceTile::new([0, 0], vec![[0.2, 0.3, 0.1, 0.8]; 4096]);
    ops.push(Operator::RemapBlocks(preview));
    for op in ops {
        let a = Cpu.execute(&op).unwrap();
        let b = gpu.execute(&op).unwrap();
        let max = a
            .iter()
            .flatten()
            .zip(b.iter().flatten())
            .map(|(a, b)| (a - b).abs())
            .fold(0_f32, f32::max);
        eprintln!("operator error={max}");
        assert!(max < 0.0001, "CPU/GPU error {max}");
    }
    assert_eq!(gpu.submissions(), 11);
}
