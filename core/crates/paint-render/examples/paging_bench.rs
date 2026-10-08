//! End-to-end 8K painting, disk residency, cold overview and whole-stroke undo.
use paint_core::{Brush, Document, InputPoint, TILE_BYTES};
use paint_render::{CachedCpuRenderer, FramePixels, PixelFormat, Region, RenderRequest, Renderer};
use paint_task::CancellationToken;
use std::time::Instant;
fn ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.
}
fn main() {
    let mut doc = Document::new(8192, 8192).unwrap();
    let brush = Brush {
        radius: 256.,
        spacing: 1.,
        ..Default::default()
    };
    let start = Instant::now();
    doc.begin_stroke(brush, InputPoint::new(128., 128., 1.))
        .unwrap();
    for row in 0..32 {
        for col in 0..32 {
            let x = if row % 2 == 0 { col } else { 31 - col };
            doc.stroke_to(InputPoint::new(
                f64::from(x) * 256. + 128.,
                f64::from(row) * 256. + 128.,
                1.,
            ))
            .unwrap();
        }
    }
    let draw_ms = ms(start);
    let start = Instant::now();
    doc.end_stroke().unwrap();
    let commit_ms = ms(start);
    assert_eq!(doc.tile_count(), 16384);
    let snapshot = doc.snapshot();
    assert!(snapshot.layers()[0].tiles().all(|(_, tile)| tile
        .try_pixels()
        .unwrap()
        .iter()
        .all(|p| p.alpha() == 1.)));
    doc.trim_storage().unwrap();
    let stats = doc.storage_stats();
    assert!(stats.resident_tiles <= 4096);
    let renderer = CachedCpuRenderer::default();
    let token = CancellationToken::default();
    let request = RenderRequest {
        region: Region {
            x: 0.,
            y: 0.,
            width: 8192.,
            height: 8192.,
        },
        width: 1024,
        height: 1024,
        format: PixelFormat::SrgbRgba8Premultiplied,
    };
    let start = Instant::now();
    let frame = renderer.render(&snapshot, request, &token).unwrap();
    let cold_ms = ms(start);
    let FramePixels::Srgb(bytes) = frame.pixels else {
        panic!("wrong format")
    };
    assert!(bytes.chunks_exact(4).all(|p| p == [0, 0, 0, 255]));
    let reads = doc.storage_stats().page_reads;
    let start = Instant::now();
    renderer.render(&snapshot, request, &token).unwrap();
    let warm_ms = ms(start);
    assert_eq!(doc.storage_stats().page_reads, reads);
    let start = Instant::now();
    doc.undo().unwrap();
    let undo_ms = ms(start);
    assert_eq!(doc.tile_count(), 0);
    let start = Instant::now();
    doc.redo().unwrap();
    let redo_ms = ms(start);
    for (coord, tile) in snapshot.layers()[0].tiles() {
        assert_eq!(
            tile.try_pixels().unwrap().as_ref(),
            doc.layer(1)
                .unwrap()
                .tile(coord)
                .unwrap()
                .try_pixels()
                .unwrap()
                .as_ref()
        );
    }
    doc.trim_storage().unwrap();
    let final_stats = doc.storage_stats();
    assert!(final_stats.resident_tiles <= 4096);
    println!("final_resident_tiles={} final_registered_tiles={} final_scratch_allocated_bytes={} final_scratch_live_bytes={}",final_stats.resident_tiles,final_stats.registered_tiles,final_stats.scratch_allocated_bytes,final_stats.scratch_live_bytes);
    println!("canvas=8192x8192 live_tiles={} logical_pixel_bytes={} resident_tiles={} resident_pixel_bytes={} scratch_allocated_bytes={} history_memory_bytes={} history_disk_bytes={} draw_ms={draw_ms:.3} commit_ms={commit_ms:.3} cold_overview_ms={cold_ms:.3} warm_overview_ms={warm_ms:.3} undo_ms={undo_ms:.3} redo_ms={redo_ms:.3} final_page_reads={} final_page_writes={}",doc.tile_count(),doc.tile_count()*TILE_BYTES,stats.resident_tiles,stats.resident_tiles*TILE_BYTES,stats.scratch_allocated_bytes,doc.history_bytes(),doc.history_disk_bytes(),final_stats.page_reads,final_stats.page_writes);
}
