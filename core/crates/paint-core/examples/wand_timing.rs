//! Ad-hoc timing for the wand and the bucket on a realistic document. Run with `--nocapture`.
use paint_core::{Brush, Document, DocumentOptions, InputPoint, Pixel, TILE_BYTES};
use std::time::Instant;

fn main() {
    for (width, height) in [(960u32, 640u32), (512, 512)] {
        let mut document = Document::with_storage(
            width,
            height,
            DocumentOptions {
                max_history_bytes: 32 * TILE_BYTES,
                ..DocumentOptions::default()
            },
            paint_storage::ScratchSpace::system(),
        )
        .unwrap();
        document.initialize_white_background().unwrap();
        document
            .begin_stroke(
                Brush {
                    radius: 40.,
                    color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
                    ..Default::default()
                },
                InputPoint::new(200., 200., 1.),
            )
            .unwrap();
        document.end_stroke().unwrap();

        let start = Instant::now();
        let shape = paint_core::wand_shape(&document, 20, 20, 0.1).unwrap();
        println!(
            "wand {width}x{height} ({} px): {:?}  region {}x{}",
            width * height,
            start.elapsed(),
            shape.width,
            shape.height
        );

        let start = Instant::now();
        let changed = document
            .fill_region(20, 20, 0.1, true, Pixel::from_straight([0., 0., 1., 1.]).unwrap(), 1.)
            .unwrap();
        println!("fill {width}x{height}: {:?} changed={changed}", start.elapsed());
    }
}
