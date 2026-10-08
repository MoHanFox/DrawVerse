use paint_core::{Document, Selection, SelectionKind, SelectionOperation, SelectionShape};
use paint_io::{load, save, ExportOptions, Format};
use paint_task::CancellationToken;
use std::io::{Read, Write};

#[test]
fn ora_selection_roundtrip_does_not_crop_pixels_or_create_history() {
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let mut doc = Document::new(48, 32).unwrap();
    doc.initialize_white_background().unwrap();
    let shape = SelectionShape {
        kind: SelectionKind::Ellipse,
        x: 4.25,
        y: 3.5,
        width: 20.,
        height: 12.,
        antialias: true,
    };
    let selection = Selection::default()
        .apply(shape, SelectionOperation::Replace, 48, 32)
        .unwrap()
        .inverted(48, 32)
        .unwrap();
    doc.set_selection(selection.clone()).unwrap();
    let path = temp.path().join("selection.ora");
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
    let loaded = load(&path, &token).unwrap();
    assert_eq!(loaded.selection(), &selection);
    assert_eq!(loaded.history_depth(), (0, 0));
    assert_eq!(loaded.pixel(0, 0).alpha(), 1.);
    assert_eq!(loaded.pixel(10, 10).alpha(), 1.);
    let png = temp.path().join("all.png");
    save(
        &png,
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
    let image = image::open(png).unwrap().to_rgba8();
    assert!(image.pixels().all(|p| p.0 == [255; 4]));
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
    let mut entries = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes).unwrap();
        entries.push((f.name().to_owned(), bytes));
    }
    let bad = temp.path().join("bad.ora");
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&bad).unwrap());
    for (name, bytes) in entries {
        writer
            .start_file(&name, zip::write::SimpleFileOptions::default())
            .unwrap();
        if name == "stack.xml" || name == "drawverse/stack.xml" {
            let text = String::from_utf8(bytes)
                .unwrap()
                .replace(&selection.to_text(), "1|1;0,0,1,NaN,0,20,20");
            writer.write_all(text.as_bytes()).unwrap();
        } else {
            writer.write_all(&bytes).unwrap();
        }
    }
    writer.finish().unwrap();
    assert!(load(&bad, &token).is_err());
}
