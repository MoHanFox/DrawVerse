use paint_core::{
    Brush, BrushMode, Document, InputPoint, Selection, SelectionKind, SelectionOperation,
    SelectionShape,
};
use std::time::Instant;
fn main() {
    for mode in [BrushMode::Paint, BrushMode::Erase] {
        for (name, kind, feather) in [
            ("none", 0, 0.),
            ("rectangle", 1, 0.),
            ("ellipse", 3, 0.),
            ("lasso", 2, 0.),
            ("feather_lasso", 2, 8.),
        ] {
            let mut doc = Document::new(1024, 1024).unwrap();
            if mode == BrushMode::Erase {
                doc.begin_stroke(
                    Brush {
                        radius: 256.,
                        ..Brush::default()
                    },
                    InputPoint::new(180., 512., 1.),
                )
                .unwrap();
                for i in 1..=120 {
                    doc.stroke_to(InputPoint::new(
                        180. + i as f64 * 5.5,
                        512. + (i as f64 * 0.07).sin() * 160.,
                        1.,
                    ))
                    .unwrap();
                }
                doc.end_stroke().unwrap();
            }
            if kind != 0 {
                let shape = if kind == 1 || kind == 3 {
                    SelectionShape::geometry(
                        if kind == 1 {
                            SelectionKind::Rectangle
                        } else {
                            SelectionKind::Ellipse
                        },
                        100.,
                        100.,
                        824.,
                        824.,
                        true,
                    )
                } else {
                    let points = (0..4096)
                        .map(|i| {
                            let a = i as f64 * std::f64::consts::TAU / 4096.;
                            [512. + 410. * a.cos(), 512. + 410. * a.sin()]
                        })
                        .collect();
                    SelectionShape::polygon(points, true, feather).unwrap()
                };
                doc.set_selection(
                    Selection::default()
                        .apply(shape, SelectionOperation::Replace, 1024, 1024)
                        .unwrap(),
                )
                .unwrap();
            }
            let brush = Brush {
                mode,
                radius: 32.,
                spacing: 0.15,
                opacity: 0.5,
                ..Brush::default()
            };
            for repeat in 0..3 {
                let start = Instant::now();
                doc.begin_stroke(brush, InputPoint::new(180., 512., 1.))
                    .unwrap();
                for i in 1..=120 {
                    let x = 180. + i as f64 * 5.5;
                    let y = 512. + (i as f64 * 0.07).sin() * 160.;
                    doc.stroke_to(InputPoint::new(x, y, 1.)).unwrap();
                }
                doc.end_stroke().unwrap();
                println!(
                    "{mode:?} {name} pass={repeat} ms={:.3} alpha={:.6}",
                    start.elapsed().as_secs_f64() * 1000.,
                    doc.pixel(180, 512).alpha()
                );
                // Undo restores pixels while keeping the unchanged selection's working cache.
                doc.undo().unwrap();
            }
        }
    }
}
