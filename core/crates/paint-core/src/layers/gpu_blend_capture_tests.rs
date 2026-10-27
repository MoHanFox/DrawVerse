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
