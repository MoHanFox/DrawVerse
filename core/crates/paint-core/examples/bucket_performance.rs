//! Actual 3840x2160 atomic fill, including staging/history; no UI frame time in this number.
use paint_core::*;
use std::time::Instant;
fn main() {
    for pass in 0..3 {
        let mut doc = Document::new(3840, 2160).unwrap();
        let start = Instant::now();
        assert!(doc
            .bucket_fill(BucketFill {
                x: 1920,
                y: 1080,
                color: Pixel::WHITE,
                opacity: 1.,
                tolerance: 10,
                antialias: true,
                contiguous: true,
                all_layers: false
            })
            .unwrap());
        println!(
            "3840x2160 empty pass={pass} ms={:.3} tiles={} alpha={}",
            start.elapsed().as_secs_f64() * 1000.,
            doc.tile_count(),
            doc.pixel(3839, 2159).alpha()
        );
        doc.undo().unwrap();
        assert_eq!(doc.pixel(3839, 2159), Pixel::TRANSPARENT);
        doc.redo().unwrap();
        assert_eq!(doc.pixel(3839, 2159), Pixel::WHITE);
    }
}
