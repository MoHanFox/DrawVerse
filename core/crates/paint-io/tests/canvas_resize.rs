use paint_core::{Document, Pixel};
use paint_io::{load, save, ExportOptions, Format};
use paint_task::CancellationToken;
#[test]
fn resized_canvas_dpi_and_retained_pixels_survive_ora() {
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let mut doc = Document::new(130, 80).unwrap();
    doc.initialize_white_background().unwrap();
    doc.resize_canvas(64, 0, 66, 80, 300, false, Pixel::WHITE)
        .unwrap();
    let path = temp.path().join("resize.ora");
    save(
        &path,
        &doc.snapshot(),
        1,
        ExportOptions {
            format: Format::OpenRaster,
            quality: 100,
            background: [1.; 3],
        },
        &token,
    )
    .unwrap();
    let mut loaded = load(&path, &token).unwrap();
    assert_eq!(loaded.dimensions(), (66, 80));
    assert_eq!(loaded.dpi(), 300);
    loaded
        .resize_canvas(-64, 0, 130, 80, 300, false, Pixel::TRANSPARENT)
        .unwrap();
    assert_eq!(loaded.pixel(2, 2), Pixel::WHITE);
}

#[test]
fn png_dpi_roundtrip() {
    let temp = tempfile::tempdir().unwrap();
    let mut doc = Document::new(2, 2).unwrap();
    doc.initialize_dpi(300).unwrap();
    let path = temp.path().join("dpi.png");
    let token = CancellationToken::default();
    save(
        &path,
        &doc.snapshot(),
        1,
        ExportOptions {
            format: Format::Png,
            quality: 100,
            background: [1.; 3],
        },
        &token,
    )
    .unwrap();
    let loaded = load(&path, &token).unwrap();
    assert_eq!(loaded.dpi(), 300);
    assert_eq!(loaded.dimensions(), (2, 2));
}
