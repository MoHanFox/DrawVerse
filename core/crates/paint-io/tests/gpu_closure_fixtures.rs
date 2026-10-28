//! Real editable ORA fixtures plus independent, reloaded CPU pixel truth.
use paint_core::*;
use paint_io::{load, save, ExportOptions, Format};
use paint_task::CancellationToken;
use std::{fs, io::Write, path::Path};

fn dab(d: &mut Document, id: u64, point: (f64, f64), color: [f32; 4], radius: f32) {
    d.set_active_layer(id).unwrap();
    d.begin_stroke(
        Brush {
            radius,
            color: Pixel::from_straight(color).unwrap(),
            ..Default::default()
        },
        InputPoint::new(point.0, point.1, 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
}
fn appearance(d: &mut Document, id: u64, mode: BlendMode, seed: u32) {
    d.set_layer_appearance(
        id,
        LayerAppearance {
            blend: mode,
            fill: 0.73,
            dissolve_seed: seed,
            ..Default::default()
        },
    )
    .unwrap();
    d.set_layer_properties(
        id,
        LayerProperties {
            visible: true,
            opacity: 0.67,
        },
    )
    .unwrap();
}
fn mask(d: &mut Document, id: u64, fixed: bool) {
    let m = d.add_mask(id).unwrap();
    dab(d, m, (48., 31.), [0., 0., 0., 0.73], 16.);
    d.set_mask_linked(m, !fixed).unwrap();
    d.set_layer_properties(
        m,
        LayerProperties {
            visible: true,
            opacity: 0.61,
        },
    )
    .unwrap();
}
fn persist(out: &Path, name: &str, d: &Document) {
    let token = CancellationToken::default();
    let path = out.join(format!("{name}.ora"));
    save(
        &path,
        &d.snapshot(),
        d.active_layer(),
        ExportOptions {
            format: Format::OpenRaster,
            quality: 100,
            background: [1.; 3],
        },
        &token,
    )
    .unwrap();
    let loaded = load(&path, &token).unwrap();
    let snap = loaded.snapshot();
    let mut linear = fs::File::create(out.join(format!("{name}.linear"))).unwrap();
    for y in 0..64 {
        for x in 0..96 {
            for c in snap.try_pixel(x, y).unwrap().components() {
                linear.write_all(&c.to_le_bytes()).unwrap();
            }
        }
    }
}
fn background() -> Document {
    let mut d = Document::new(96, 64).unwrap();
    dab(&mut d, 1, (42., 32.), [0.2, 0.65, 0.4, 0.63], 35.);
    d
}
#[test]
#[ignore = "explicit real-window fixture generation; no GPU required"]
fn write_gpu_closure_fixtures() {
    let out = std::env::var_os("DRAWVERSE_G4_FIXTURES").expect("output directory");
    let out = Path::new(&out);
    fs::create_dir_all(out).unwrap();
    let mut count = 0;
    for seed in [0, 0x80000001, u32::MAX] {
        let mut d = background();
        let layer = d.add_layer("dissolve").unwrap();
        dab(&mut d, layer, (51., 34.), [0.8, 0.1, 0.6, 0.81], 31.);
        appearance(&mut d, layer, BlendMode::Dissolve, seed);
        mask(&mut d, layer, true);
        d.move_layer(layer, -7, 3).unwrap();
        persist(out, &format!("dissolve-{seed}"), &d);
        count += 1;
    }
    for base_mode in [BlendMode::Normal, BlendMode::PassThrough] {
        for id in 0..28 {
            let mode = BlendMode::from_id(id).unwrap();
            let mut d = background();
            let child = d.add_layer("base child").unwrap();
            dab(&mut d, child, (44., 32.), [0.85, 0.15, 0.4, 0.65], 24.);
            let base = d.group_layer(child, "base group").unwrap();
            appearance(&mut d, base, base_mode, 0);
            mask(&mut d, base, true);
            d.set_active_layer(base).unwrap();
            let clip = d.add_layer("clipped child").unwrap();
            dab(&mut d, clip, (54., 35.), [0.15, 0.4, 0.95, 0.84], 29.);
            let clip = if mode == BlendMode::PassThrough {
                d.group_layer(clip, "clipped through").unwrap()
            } else {
                clip
            };
            d.reparent_layer(clip, 0).unwrap();
            d.set_layer_clipping(clip, true).unwrap();
            appearance(&mut d, clip, mode, u32::MAX);
            mask(&mut d, clip, false);
            assert_eq!(
                d.layer(clip).unwrap().parent_id(),
                d.layer(base).unwrap().parent_id()
            );
            persist(out, &format!("clip-base{}-mode{id}", base_mode as u32), &d);
            count += 1;
            if id == 27 {
                d.set_layer_properties(
                    base,
                    LayerProperties {
                        visible: false,
                        opacity: 0.67,
                    },
                )
                .unwrap();
                persist(out, &format!("hidden-base{}", base_mode as u32), &d);
                count += 1;
            }
        }
    }
    for base_mode in [BlendMode::Normal, BlendMode::PassThrough] {
        for clip_mode in [
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::PassThrough,
        ] {
            for outer_mode in [BlendMode::Normal, BlendMode::PassThrough] {
                let mut d = background();
                let child = d.add_layer("base").unwrap();
                dab(&mut d, child, (40., 29.), [0.8, 0.1, 0.4, 0.65], 25.);
                let base = d.group_layer(child, "base").unwrap();
                appearance(&mut d, base, base_mode, 0x80000001);
                mask(&mut d, base, true);
                let clip = d.add_layer("clip").unwrap();
                dab(&mut d, clip, (53., 32.), [0.1, 0.4, 0.95, 0.8], 26.);
                let clip = d.group_layer(clip, "clip group").unwrap();
                d.reparent_layer(clip, 0).unwrap();
                d.set_layer_clipping(clip, true).unwrap();
                appearance(&mut d, clip, clip_mode, u32::MAX);
                mask(&mut d, clip, false);
                let outer = d.group_layers(&[base, clip], "ancestor").unwrap();
                appearance(&mut d, outer, outer_mode, 0);
                mask(&mut d, outer, true);
                persist(
                    out,
                    &format!(
                        "nested-base{}-clip{}-outer{}",
                        base_mode as u32, clip_mode as u32, outer_mode as u32
                    ),
                    &d,
                );
                count += 1;
            }
        }
    }

    for base_mode in [BlendMode::Normal, BlendMode::Multiply, BlendMode::Dissolve] {
        for clip_mode in [BlendMode::Dissolve, BlendMode::PassThrough] {
            let mut d = background();
            let base = d.add_layer("raster base").unwrap();
            dab(&mut d, base, (43., 29.), [0.8, 0.1, 0.4, 0.7], 25.);
            appearance(&mut d, base, base_mode, 0x80000001);
            mask(&mut d, base, true);
            let clip = d.add_layer("raster clip").unwrap();
            dab(&mut d, clip, (52., 34.), [0.1, 0.4, 0.9, 0.8], 28.);
            let clip = if clip_mode == BlendMode::PassThrough {
                d.group_layer(clip, "through clip").unwrap()
            } else {
                clip
            };
            d.set_layer_clipping(clip, true).unwrap();
            appearance(&mut d, clip, clip_mode, u32::MAX);
            mask(&mut d, clip, false);
            assert_eq!(
                d.layer(base).unwrap().parent_id(),
                d.layer(clip).unwrap().parent_id()
            );
            persist(
                out,
                &format!("raster-base{}-clip{}", base_mode as u32, clip_mode as u32),
                &d,
            );
            count += 1;
        }
    }
    let mut empty = Document::new(96, 64).unwrap();
    let clip = empty.add_layer("clipped over empty").unwrap();
    dab(&mut empty, clip, (48., 32.), [0.8, 0.1, 0.5, 0.9], 30.);
    empty.set_layer_clipping(clip, true).unwrap();
    persist(out, "empty-base", &empty);
    count += 1;

    for base_mode in [
        BlendMode::Normal,
        BlendMode::Dissolve,
        BlendMode::PassThrough,
    ] {
        let mut d = background();
        let child = d.add_layer("base").unwrap();
        dab(&mut d, child, (42., 30.), [0.8, 0.1, 0.4, 0.7], 24.);
        let base = d.group_layer(child, "multi base").unwrap();
        appearance(&mut d, base, base_mode, 0x80000001);
        mask(&mut d, base, true);
        let mut previous = base;
        for (index, mode) in [BlendMode::PassThrough, BlendMode::Dissolve, BlendMode::Hue]
            .into_iter()
            .enumerate()
        {
            d.set_active_layer(previous).unwrap();
            let c = d.add_layer("multi clip").unwrap();
            dab(
                &mut d,
                c,
                (48. + index as f64 * 3., 32.),
                [0.15, 0.4, 0.95, 0.76],
                27.,
            );
            let c = if mode == BlendMode::PassThrough {
                d.group_layer(c, "multi through").unwrap()
            } else {
                c
            };
            d.reparent_layer(c, 0).unwrap();
            d.set_layer_clipping(c, true).unwrap();
            appearance(&mut d, c, mode, u32::MAX);
            mask(&mut d, c, index % 2 == 0);
            previous = c;
        }
        persist(out, &format!("multi-base{}", base_mode as u32), &d);
        count += 1;
    }
    let mut budget = background();
    let mut base = 1;
    for _ in 0..8 {
        base = budget.group_layer(base, "through base").unwrap();
        let clip = budget.add_layer("empty clip").unwrap();
        budget.reparent_layer(clip, 0).unwrap();
        budget.set_layer_clipping(clip, true).unwrap();
        base = budget.group_layers(&[base, clip], "outer").unwrap();
    }
    let budgets = out.join("budgets");
    fs::create_dir_all(&budgets).unwrap();
    persist(&budgets, "operations", &budget);
    // A hash rounded upward to f32: a lossy float(random) comparison would
    // reject equality, while the authoritative f64 comparison keeps it.
    let mut d = Document::new(96, 64).unwrap();
    let (x, y) = (48u32, 32u32);
    let mut chosen = None;
    for seed in 0..10000u32 {
        let mut h = ((u64::from(x) * 0x9e3779b1) ^ (u64::from(y) * 0x85ebca77) ^ u64::from(seed))
            .wrapping_mul(0xbf58476d1ce4e5b9);
        h ^= h >> 31;
        let r = h as u32 as f64 / 4294967296.;
        let a = r as f32;
        if f64::from(a) > r && a > 0.1 && a < 0.9 {
            chosen = Some((seed, a));
            break;
        }
    }
    let (seed, a) = chosen.unwrap();
    dab(
        &mut d,
        1,
        (f64::from(x) + 0.5, f64::from(y) + 0.5),
        [0.8, 0.2, 0.4, 1.],
        20.,
    );
    d.set_layer_appearance(
        1,
        LayerAppearance {
            blend: BlendMode::Dissolve,
            dissolve_seed: seed,
            ..Default::default()
        },
    )
    .unwrap();
    d.set_layer_properties(
        1,
        LayerProperties {
            visible: true,
            opacity: a,
        },
    )
    .unwrap();
    assert_eq!(d.pixel(x, y).alpha(), 1.);
    persist(out, "dissolve-f32-threshold", &d);
    count += 1;

    // A preceding Normal group must preserve CPU alpha rounding too: a MAD
    // contraction can cross Dissolve's exact threshold by one f32 ULP.
    let mut chosen = None;
    'seeds: for seed in 0..128u32 {
        let mut h = ((48u64 * 0x9e3779b1) ^ (32u64 * 0x85ebca77) ^ u64::from(seed))
            .wrapping_mul(0xbf58476d1ce4e5b9);
        h ^= h >> 31;
        let r = h as u32 as f64 / 4294967296.;
        let target = r as f32;
        if f64::from(target) <= r || !(0.1..0.9).contains(&target) {
            continue;
        }
        for i in 1..4096 {
            let b = target * i as f32 / 4096.;
            let a = (target - b) / (1. - b);
            let core = a + b * (1. - a);
            let fused = b.mul_add(1. - a, a);
            if f64::from(core) > r && f64::from(fused) <= r {
                chosen = Some((seed, b, a));
                break 'seeds;
            }
        }
    }
    let (seed, b, a) = chosen.expect("bounded contraction counterexample");
    let mut d = Document::new(96, 64).unwrap();
    dab(&mut d, 1, (48.5, 32.5), [0.2, 0.7, 0.4, 1.], 20.);
    d.set_layer_properties(
        1,
        LayerProperties {
            visible: true,
            opacity: b,
        },
    )
    .unwrap();
    let top = d.add_layer("upstream alpha").unwrap();
    dab(&mut d, top, (48.5, 32.5), [0.8, 0.1, 0.6, 1.], 20.);
    d.set_layer_properties(
        top,
        LayerProperties {
            visible: true,
            opacity: a,
        },
    )
    .unwrap();
    let group = d.group_layers(&[1, top], "dissolve boundary").unwrap();
    d.set_layer_appearance(
        group,
        LayerAppearance {
            blend: BlendMode::Dissolve,
            dissolve_seed: seed,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(d.pixel(48, 32).alpha(), 1.);
    persist(out, "dissolve-upstream-fma", &d);
    count += 1;
    fs::write(out.join("count.txt"), count.to_string()).unwrap();
    assert_eq!(count, 85);
}
