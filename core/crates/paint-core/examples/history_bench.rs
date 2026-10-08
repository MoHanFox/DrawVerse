//! Reproduce a single full-canvas stroke that crossed the old 64MiB limit.
use paint_core::{Brush, Document, InputPoint, Pixel};
use std::time::Instant;
fn main() {
    let mut doc = Document::new(1536, 1536).unwrap();
    let brush = Brush {
        radius: 64.,
        spacing: 1.,
        color: Pixel::from_straight([0.2, 0.4, 0.6, 1.]).unwrap(),
        ..Default::default()
    };
    let start = Instant::now();
    doc.begin_stroke(brush, InputPoint::new(32., 32., 1.))
        .unwrap();
    for row in 0..24 {
        for column in 0..24 {
            let x = if row % 2 == 0 { column } else { 23 - column };
            doc.stroke_to(InputPoint::new(
                f64::from(x) * 64. + 32.,
                f64::from(row) * 64. + 32.,
                1.,
            ))
            .unwrap();
        }
    }
    let draw_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    doc.end_stroke().unwrap();
    let commit_ms = start.elapsed().as_secs_f64() * 1000.;
    let (memory, disk) = (doc.history_bytes(), doc.history_disk_bytes());
    let snapshot = doc.snapshot();
    assert_eq!(doc.tile_count(), 576);
    assert!(snapshot.layers()[0]
        .tiles()
        .all(|(_, tile)| tile.pixels().iter().all(|p| p.alpha() == 1.)));
    let start = Instant::now();
    doc.undo().unwrap();
    let undo_ms = start.elapsed().as_secs_f64() * 1000.;
    assert_eq!(doc.tile_count(), 0);
    let start = Instant::now();
    doc.redo().unwrap();
    let redo_ms = start.elapsed().as_secs_f64() * 1000.;
    for (coord, expected) in snapshot.layers()[0].tiles() {
        let actual = doc.layer(1).unwrap().tile(coord).unwrap();
        for (a, b) in actual.pixels().iter().zip(expected.pixels().iter()) {
            assert_eq!(
                a.components().map(f32::to_bits),
                b.components().map(f32::to_bits)
            );
        }
    }
    println!("canvas=1536x1536 filled_tiles=576 history_memory_bytes={memory} history_disk_bytes={disk} draw_ms={draw_ms:.3} commit_ms={commit_ms:.3} undo_ms={undo_ms:.3} redo_ms={redo_ms:.3}");
}
