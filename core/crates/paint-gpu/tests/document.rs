use paint_core::*;
use paint_gpu::{hardware, Gpu};
use std::sync::Arc;

fn pixels(d: &Document) -> Vec<f32> {
    (0..96)
        .flat_map(|y| (0..96).flat_map(move |x| d.pixel(x, y).components()))
        .collect()
}
fn image() -> Document {
    let mut d = Document::new(96, 96).unwrap();
    d.begin_stroke(
        Brush {
            radius: 5.,
            ..Default::default()
        },
        InputPoint::new(8., 48., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    d
}
fn push(from: f64, to: f64) -> LiquifyDab {
    LiquifyDab {
        tool: 0,
        radius: 48.,
        strength: 1.,
        density: 0.8,
        speed: 1.,
        pressure_enabled: false,
        from: InputPoint::new(from, 48., 1.),
        to: InputPoint::new(to, 48., 1.),
        elapsed_ms: 16.,
    }
}
#[test]
fn actual_gpu_document_keeps_outside_content_final_quality_and_history() {
    if hardware().is_empty() {
        eprintln!("No hardware; GPU document comparison not run");
        return;
    }
    let gpu = Arc::new(Gpu::open("auto").unwrap());
    let mut a = image();
    let mut b = image();
    b.set_compute_executor(gpu.clone()).unwrap();
    let original = pixels(&a);
    for dab in [push(8., -26.), push(-26., 8.)] {
        for doc in [&mut a, &mut b] {
            if !doc.liquify_active() {
                doc.begin_liquify().unwrap();
            }
            doc.begin_liquify_stroke().unwrap();
            doc.liquify_dab(dab).unwrap();
            doc.end_liquify_stroke().unwrap();
        }
        let error = pixels(&a)
            .iter()
            .zip(pixels(&b))
            .map(|(a, b)| (a - b).abs())
            .fold(0_f32, f32::max);
        assert!(error < 0.0001, "preview CPU/GPU error={error}");
    }
    for doc in [&mut a, &mut b] {
        doc.commit_liquify().unwrap();
    }
    let final_pixels = pixels(&b);
    let error = pixels(&a)
        .iter()
        .zip(&final_pixels)
        .map(|(a, b)| (a - b).abs())
        .fold(0_f32, f32::max);
    assert!(error < 0.0001, "final CPU/GPU error={error}");
    b.undo().unwrap();
    assert_eq!(pixels(&b), original);
    b.redo().unwrap();
    assert_eq!(pixels(&b), final_pixels);
    assert!(
        gpu.submissions() > 10,
        "document must use real GPU dispatches"
    );
}
