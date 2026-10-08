//! Developer-only raster preview, not the future paint-io format subsystem.
use paint_core::{Brush, Document, InputPoint, Pixel};
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

fn srgb(linear: f32) -> u8 {
    let encoded = if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (encoded.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| "stroke-preview.ppm".into());
    let mut doc = Document::new(512, 320)?;
    for (row, rgba) in [
        [0.015, 0.26, 0.21, 1.0],
        [0.52, 0.17, 0.055, 1.0],
        [0.09, 0.12, 0.24, 1.0],
    ]
    .into_iter()
    .enumerate()
    {
        let b = Brush {
            radius: 13.0,
            opacity: 0.45,
            spacing: 0.2,
            color: Pixel::from_straight(rgba)?,
            ..Brush::default()
        };
        let y = 70.0 + row as f64 * 80.0;
        doc.begin_stroke(b, InputPoint::new(40.0, y, 0.12))?;
        for step in 1..=220 {
            let t = step as f64 / 220.0;
            let pressure = (0.12 + 0.88 * (std::f64::consts::PI * t).sin()) as f32;
            doc.stroke_to(InputPoint::new(
                40.0 + 430.0 * t,
                y + (t * 12.0).sin() * 15.0,
                pressure,
            ))?;
        }
        doc.end_stroke()?;
    }
    let painted = doc.snapshot();
    doc.undo()?;
    doc.redo()?;
    for y in 0..320 {
        for x in 0..512 {
            assert_eq!(doc.pixel(x, y), painted.pixel(x, y));
        }
    }
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = BufWriter::new(File::create(&output)?);
    write!(file, "P6\n512 320\n255\n")?;
    for y in 0..320 {
        for x in 0..512 {
            let rgba = doc.pixel(x, y).components();
            // Present transparency over a warm white background, outside the document.
            let background = [0.95, 0.94, 0.91];
            let rgb =
                std::array::from_fn::<_, 3, _>(|i| srgb(rgba[i] + background[i] * (1.0 - rgba[3])));
            file.write_all(&rgb)?;
        }
    }
    file.flush()?;
    println!(
        "Preview: {} ({} allocated tiles; undo/redo verified)",
        output.display(),
        doc.tile_count()
    );
    Ok(())
}
