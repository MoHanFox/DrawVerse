use paint_core::*;
use std::sync::Arc;
fn image() -> Document {
    let mut d = Document::new(96, 96).unwrap();
    d.begin_stroke(
        Brush {
            radius: 5.,
            spacing: 0.1,
            ..Default::default()
        },
        InputPoint::new(8., 48., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    d
}
fn pixels(d: &Document) -> Vec<Pixel> {
    (0..96)
        .flat_map(|y| (0..96).map(move |x| d.pixel(x, y)))
        .collect()
}
fn mass(d: &Document) -> f32 {
    pixels(d).iter().map(|p| p.alpha()).sum()
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
fn off_canvas_pixels_can_be_pulled_back_and_reconstructed() {
    let mut d = image();
    let original = pixels(&d);
    let initial = mass(&d);
    d.begin_liquify().unwrap();
    d.begin_liquify_stroke().unwrap();
    d.liquify_dab(push(8., -26.)).unwrap();
    d.end_liquify_stroke().unwrap();
    let outside = pixels(&d);
    let low = mass(&d);
    assert!(
        low < initial * 0.2,
        "content should leave the clipped canvas: {low}/{initial}"
    );
    d.begin_liquify_stroke().unwrap();
    d.liquify_dab(push(-26., 8.)).unwrap();
    d.end_liquify_stroke().unwrap();
    assert!(
        mass(&d) > low + initial * 0.3,
        "outside-to-inside drag must recover source content"
    );
    let returned = pixels(&d);
    d.undo_liquify().unwrap();
    assert!(pixels(&d) == outside);
    d.redo_liquify().unwrap();
    assert!(pixels(&d) == returned);
    let mut restore = push(0., 0.);
    restore.tool = 3;
    restore.radius = 180.;
    restore.density = 1.;
    d.liquify_dab(restore).unwrap();
    assert!(
        pixels(&d) == original,
        "reconstruction restores immutable source"
    );
    d.cancel_liquify().unwrap();
    assert!(pixels(&d) == original);
}
#[derive(Debug)]
struct Fail;
impl paint_ops::Executor for Fail {
    fn name(&self) -> String {
        "failed test device".into()
    }
    fn execute(&self, _: &paint_ops::Operator) -> std::result::Result<Vec<[f32; 4]>, String> {
        Err("lost".into())
    }
}
#[test]
fn failed_compute_replays_cpu_without_partial_adoption() {
    let mut a = image();
    let mut b = image();
    b.set_compute_executor(Arc::new(Fail)).unwrap();
    for d in [&mut a, &mut b] {
        d.begin_liquify().unwrap();
        d.liquify_dab(push(8., 20.)).unwrap();
        d.commit_liquify().unwrap();
    }
    assert!(pixels(&a) == pixels(&b));
}
#[test]
fn full_quality_commit_is_independent_of_preview_pixels() {
    let mut d = image();
    let original = pixels(&d);
    let depth = d.history_depth();
    d.begin_liquify().unwrap();
    let mut dab = push(8., 18.);
    dab.radius = 80.;
    d.liquify_dab(dab).unwrap();
    let preview = pixels(&d);
    d.commit_liquify().unwrap();
    let final_pixels = pixels(&d);
    assert!(final_pixels != preview);
    assert_eq!(d.history_depth().0, depth.0 + 1);
    d.undo().unwrap();
    assert!(pixels(&d) == original);
    d.redo().unwrap();
    assert!(pixels(&d) == final_pixels);
}
