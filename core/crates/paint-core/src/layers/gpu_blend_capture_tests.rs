//! Offline correctness only: interpret actual Qt source samples with the
//! authoritative Rust blend implementation, independently of the GLSL port.
use crate::{blend_pixel, BlendMode, Pixel};
use std::{fs, path::Path};

fn word(bytes: &[u8], at: &mut usize) -> u32 {
    let value = u32::from_le_bytes(bytes[*at..*at + 4].try_into().unwrap());
    *at += 4;
    value
}
fn scalar(bytes: &[u8], at: &mut usize) -> f32 {
    f32::from_bits(word(bytes, at))
}
fn pixel(bytes: &[u8], at: &mut usize) -> Pixel {
    Pixel::from_premultiplied(std::array::from_fn(|_| scalar(bytes, at))).unwrap()
}
fn verify(path: &Path) -> f32 {
    assert!(fs::metadata(path).unwrap().len() <= 2 * 1024 * 1024);
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[..4], b"G4L1");
    let mut at = 4;
    let (width, height, nodes) = (
        word(&bytes, &mut at),
        word(&bytes, &mut at),
        word(&bytes, &mut at),
    );
    assert_eq!((width, height, nodes), (96, 64, 2));
    let mut result = vec![Pixel::TRANSPARENT; (width * height) as usize];
    for _ in 0..nodes {
        let mode = BlendMode::from_id(word(&bytes, &mut at)).unwrap();
        assert!(!matches!(
            mode,
            BlendMode::Dissolve | BlendMode::PassThrough
        ));
        let opacity = scalar(&bytes, &mut at);
        let fill = scalar(&bytes, &mut at);
        let density = scalar(&bytes, &mut at);
        for (i, destination) in result.iter_mut().enumerate() {
            let source = pixel(&bytes, &mut at);
            let mask_alpha = scalar(&bytes, &mut at);
            let source = source
                .scaled(1. - mask_alpha * density)
                .scaled(opacity * fill);
            *destination = blend_pixel(
                source,
                *destination,
                mode,
                i as u32 % width,
                i as u32 / width,
                0,
            );
        }
    }
    let mut maximum = 0f32;
    for expected in result {
        for component in expected.components() {
            let actual = scalar(&bytes, &mut at);
            assert!(actual.is_finite());
            maximum = maximum.max((component - actual).abs());
        }
    }
    assert_eq!(at, bytes.len());
    assert!(maximum <= 2e-5, "{}: {maximum}", path.display());
    maximum
}

#[test]
#[ignore = "requires a complete real Qt G4 capture directory in DRAWVERSE_G4_CAPTURES"]
fn real_window_linear_results_match_rust_blends() {
    let directory = std::env::var("DRAWVERSE_G4_CAPTURES").expect("capture directory required");
    let mut maximum = 0f32;
    for mode in (0..27).filter(|m| *m != 1) {
        for variant in ["fixed-white-lod1", "linked-transparent"] {
            let path = Path::new(&directory).join(format!("mode-{mode}-{variant}.g4linear"));
            maximum = maximum.max(verify(&path));
        }
    }
    for name in [
        "dependency-opacity",
        "dependency-backdrop",
        "dependency-mask",
        "dependency-blend",
    ] {
        maximum = maximum.max(verify(
            &Path::new(&directory).join(format!("{name}.g4linear")),
        ));
    }
    println!("56 actual window RGBA32F captures, maximum error {maximum}");
}

#[test]
#[ignore = "requires real Qt group captures in DRAWVERSE_G4_GROUP_CAPTURES"]
fn real_window_group_results_match_rust_boundaries() {
    let directory =
        std::env::var("DRAWVERSE_G4_GROUP_CAPTURES").expect("capture directory required");
    let mut count = 0;
    let mut maximum = 0f32;
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "g4group") {
            continue;
        }
        assert!(fs::metadata(&path).unwrap().len() <= 4 * 1024 * 1024);
        let bytes = fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], b"G4G1");
        let mut at = 4;
        let width = word(&bytes, &mut at);
        let height = word(&bytes, &mut at);
        assert_eq!((width, height), (96, 64));
        let pixels = (width * height) as usize;
        let nodes = word(&bytes, &mut at) as usize;
        let ops = word(&bytes, &mut at) as usize;
        let surfaces = word(&bytes, &mut at) as usize;
        let output = word(&bytes, &mut at) as usize;
        assert!(nodes <= 64 && ops <= 4 * nodes + 1 && (2..=34).contains(&surfaces) && output < 2);
        let mut descriptions = Vec::new();
        let mut samples = Vec::new();
        for _ in 0..nodes {
            let kind = word(&bytes, &mut at);
            let mode = BlendMode::from_id(word(&bytes, &mut at)).unwrap();
            let mask = word(&bytes, &mut at);
            let visible = word(&bytes, &mut at);
            let opacity = scalar(&bytes, &mut at);
            let fill = scalar(&bytes, &mut at);
            descriptions.push((kind, mode, mask, visible, opacity, fill));
            samples.push(
                (0..pixels)
                    .map(|_| pixel(&bytes, &mut at))
                    .collect::<Vec<_>>(),
            );
        }
        let operations: Vec<[u32; 5]> = (0..ops)
            .map(|_| std::array::from_fn(|_| word(&bytes, &mut at)))
            .collect();
        let mut result = vec![Pixel::TRANSPARENT; pixels];
        for (i, result) in result.iter_mut().enumerate() {
            let mut state = vec![Pixel::TRANSPARENT; surfaces];
            for [kind, node, out, source, backdrop] in &operations {
                let (kind, node, out, source, backdrop) = (
                    *kind,
                    *node as usize,
                    *out as usize,
                    *source as usize,
                    *backdrop as usize,
                );
                let (_, mode, mask, _, opacity, fill) = descriptions[node];
                let before = state[backdrop];
                state[out] = match kind {
                    0 => Pixel::TRANSPARENT,
                    1 => before,
                    2..=4 => {
                        let coverage = if (mask as usize) < nodes
                            && descriptions[mask as usize].3 != 0
                        {
                            1. - samples[mask as usize][i].alpha() * descriptions[mask as usize].4
                        } else {
                            1.
                        };
                        let src = if kind == 2 {
                            samples[node][i]
                        } else {
                            state[source]
                        };
                        if kind == 4 {
                            if before == src {
                                before
                            } else {
                                before.mix(src, opacity * fill * coverage)
                            }
                        } else {
                            blend_pixel(
                                src.scaled(coverage).scaled(opacity * fill),
                                before,
                                mode,
                                i as u32 % width,
                                i as u32 / width,
                                0,
                            )
                        }
                    }
                    _ => panic!("unknown operation"),
                };
            }
            *result = state[output];
        }
        let mut error = 0f32;
        for expected in result {
            for component in expected.components() {
                let actual = scalar(&bytes, &mut at);
                assert!(actual.is_finite());
                error = error.max((component - actual).abs());
            }
        }
        assert_eq!(at, bytes.len());
        assert!(error <= 2e-5, "{}: {error}", path.display());
        maximum = maximum.max(error);
        count += 1;
    }
    assert!(count >= 28, "missing group captures: {count}");
    println!("{count} actual window group RGBA32F captures, maximum error {maximum}");
}
