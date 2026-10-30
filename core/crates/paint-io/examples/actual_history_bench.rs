use paint_core::{Document, InputPoint, LiquifyDab};
use paint_task::CancellationToken;
use serde_json::json;
use std::{path::Path, time::Instant};
fn fingerprint(doc: &Document) -> u64 {
    let mut hash = 1469598103934665603u64;
    let snapshot = doc.snapshot();
    for layer in snapshot.layers() {
        for (coord, tile) in layer.tiles() {
            hash = (hash ^ u64::from(coord.x)).wrapping_mul(1099511628211);
            hash = (hash ^ u64::from(coord.y)).wrapping_mul(1099511628211);
            for pixel in tile.pixels().iter() {
                for channel in pixel.components() {
                    hash = (hash ^ u64::from(channel.to_bits())).wrapping_mul(1099511628211);
                }
            }
        }
    }
    hash
}
fn main() {
    let path = std::env::args().nth(1).expect("original PNG path");
    let mut rows = Vec::new();
    for operation in ["transform", "liquify", "rotate"] {
        let resident = std::env::var("DRAWVERSE_HISTORY_RESIDENT")
            .ok()
            .map(|value| value.parse::<usize>().unwrap())
            .unwrap_or(4096);
        let (mut doc, _) = paint_io::load_with_storage(
            Path::new(&path),
            &CancellationToken::default(),
            paint_core::DocumentOptions {
                max_resident_tiles: resident,
                ..Default::default()
            },
            paint_storage::ScratchSpace::system(),
        )
        .unwrap();
        let original_size = doc.dimensions();
        let original = fingerprint(&doc);
        let start = Instant::now();
        match operation {
            "transform" => {
                doc.begin_transform().unwrap();
                let mut geometry = doc.transform_geometry().unwrap();
                for point in &mut geometry.points {
                    point[0] += 37.;
                    point[1] += 19.;
                }
                doc.update_transform(geometry).unwrap();
                assert!(doc.commit_transform().unwrap());
            }
            "liquify" => {
                doc.begin_liquify().unwrap();
                doc.begin_liquify_stroke().unwrap();
                assert!(doc
                    .liquify_dab(LiquifyDab {
                        tool: 0,
                        radius: 300.,
                        strength: 0.5,
                        density: 1.,
                        speed: 1.,
                        pressure_enabled: false,
                        from: InputPoint::new(2048., 1080., 1.),
                        to: InputPoint::new(2108., 1110., 1.),
                        elapsed_ms: 100.
                    })
                    .unwrap());
                doc.end_liquify_stroke().unwrap();
                assert!(doc.commit_liquify().unwrap());
            }
            _ => doc.rotate_image(true).unwrap(),
        }
        let operation_ms = start.elapsed().as_secs_f64() * 1000.;
        let changed_size = doc.dimensions();
        let changed = fingerprint(&doc);
        assert_ne!(original, changed);
        let mut samples = Vec::new();
        for round in 0..5 {
            for redo in [false, true] {
                let stats = doc.storage_stats();
                let start = Instant::now();
                assert!(if redo { doc.redo() } else { doc.undo() }.unwrap());
                let ms = start.elapsed().as_secs_f64() * 1000.;
                let after = doc.storage_stats();
                assert_eq!(
                    doc.dimensions(),
                    if redo { changed_size } else { original_size }
                );
                assert_eq!(fingerprint(&doc), if redo { changed } else { original });
                samples.push(json!({"round":round,"redo":redo,"ms":ms,
                    "page_reads":after.page_reads-stats.page_reads,"page_writes":after.page_writes-stats.page_writes}));
            }
        }
        rows.push(json!({"operation":operation,"dimensions":original_size,"operation_ms":operation_ms,
            "resident_limit":resident,"history_memory":doc.history_bytes(),"history_disk":doc.history_disk_bytes(),"samples":samples}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"image":path,"rows":rows,"validation":"full stored pixel bits and dimensions; outside timing"})).unwrap());
}
