use paint_core::*;
use paint_io::{load, save, ExportOptions, Format};
use paint_task::CancellationToken;
use std::io::{Read, Write};
fn options(format: Format) -> ExportOptions {
    ExportOptions {
        format,
        quality: 100,
        background: [1.; 3],
    }
}
fn paint(doc: &mut Document, white: f32, radius: f32) {
    doc.begin_stroke(
        Brush {
            radius,
            color: Pixel::from_straight([white, white, white, 1.]).unwrap(),
            ..Default::default()
        },
        InputPoint::new(32., 32., 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
}
#[test]
fn ora_reopens_editable_masks_groups_background_and_exports_composite_alpha() {
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let mut doc = Document::new(128, 128).unwrap();
    doc.initialize_white_background().unwrap();
    let group = doc.group_layer(1, "画组").unwrap();
    let mask = doc.add_mask(1).unwrap();
    paint(&mut doc, 0., 10.);
    doc.set_active_layer(group).unwrap();
    let gm = doc.add_mask(group).unwrap();
    paint(&mut doc, 0.5, 20.);
    doc.move_layer(group, 4, 2).unwrap();
    let path = temp.path().join("editable.ora");
    save(
        &path,
        &doc.snapshot(),
        gm,
        options(Format::OpenRaster),
        &token,
    )
    .unwrap();
    let mut restored = load(&path, &token).unwrap();
    assert_eq!(restored.layers().len(), 4);
    assert!(restored.layer(restored.active_layer()).unwrap().is_mask());
    assert_eq!(restored.layers().iter().filter(|l| l.is_mask()).count(), 2);
    assert_eq!(
        restored
            .layers()
            .iter()
            .find(|l| l.name() == "背景")
            .unwrap()
            .white_extent(),
        (128, 128)
    );
    for y in 0..128 {
        for x in 0..128 {
            for (a, b) in doc
                .pixel(x, y)
                .components()
                .into_iter()
                .zip(restored.pixel(x, y).components())
            {
                assert!((a - b).abs() < 0.0041, "{x} {y}: {a} != {b}");
            }
        }
    }
    let restored_mask = restored
        .layers()
        .iter()
        .find(|l| l.is_mask() && restored.layer(l.parent_id()).unwrap().name() == "背景")
        .unwrap()
        .id();
    restored.set_active_layer(restored_mask).unwrap();
    paint(&mut restored, 1., 50.);
    assert!(restored.pixel(36, 34).alpha() > 0.49);
    restored.undo().unwrap();
    assert_eq!(restored.pixel(36, 34).alpha(), 0.);
    assert_eq!(doc.layer(mask).unwrap().appearance().offset_x, 4);
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
    let mut xml = String::new();
    zip.by_name("stack.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    assert!(xml.contains("dv:editable=\"2\""));
    assert!(xml.contains("src=\"mergedimage.png\""));
    assert_eq!(xml.matches("<layer ").count(), 1);
    for format in [Format::Png, Format::WebP, Format::Jpeg] {
        let path = temp.path().join(format!("flat-{format:?}"));
        save(&path, &doc.snapshot(), gm, options(format), &token).unwrap();
        let flat = load(&path, &token).unwrap();
        assert_eq!(flat.layers().len(), 1);
        if format != Format::Jpeg {
            assert_eq!(flat.pixel(36, 34).alpha(), 0.);
            assert!(flat.pixel(100, 100).alpha() > 0.99);
        } else {
            assert_eq!(flat.pixel(36, 34).alpha(), 1.);
        }
    }
}
#[test]
fn completely_erased_background_tiles_and_untouched_white_extent_survive_ora() {
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    for erase in [false, true] {
        let mut doc = Document::new(128, 128).unwrap();
        doc.initialize_white_background().unwrap();
        if erase {
            doc.begin_stroke(
                Brush {
                    radius: 50.,
                    mode: BrushMode::Erase,
                    ..Default::default()
                },
                InputPoint::new(32., 32., 1.),
            )
            .unwrap();
            doc.end_stroke().unwrap();
        }
        let path = temp.path().join(format!("background-{erase}.ora"));
        save(
            &path,
            &doc.snapshot(),
            1,
            options(Format::OpenRaster),
            &token,
        )
        .unwrap();
        let restored = load(&path, &token).unwrap();
        for y in 0..128 {
            for x in 0..128 {
                assert!((doc.pixel(x, y).alpha() - restored.pixel(x, y).alpha()).abs() < 0.004);
            }
        }
        assert_eq!(restored.pixel(100, 100), Pixel::WHITE);
    }
}
#[test]
fn invalid_editable_manifest_never_falls_back_to_a_flattened_success() {
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let mut doc = Document::new(64, 64).unwrap();
    doc.initialize_white_background().unwrap();
    doc.add_mask(1).unwrap();
    let path = temp.path().join("valid.ora");
    save(
        &path,
        &doc.snapshot(),
        2,
        options(Format::OpenRaster),
        &token,
    )
    .unwrap();
    let mut source = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut files = Vec::new();
    for i in 0..source.len() {
        let mut f = source.by_index(i).unwrap();
        let mut b = Vec::new();
        f.read_to_end(&mut b).unwrap();
        files.push((f.name().to_owned(), b));
    }
    for case in 0..3 {
        let path = temp.path().join(format!("bad-{case}.ora"));
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        for (name, b) in &files {
            if case == 0 && name == "drawverse/stack.xml" {
                continue;
            }
            zip.start_file(
                name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
            let content = if name == "drawverse/stack.xml" {
                String::from_utf8(b.clone())
                    .unwrap()
                    .replace(
                        "dv:parent=\"1\"",
                        if case == 1 {
                            "dv:parent=\"2\""
                        } else {
                            "dv:parent=\"999\""
                        },
                    )
                    .into_bytes()
            } else {
                b.clone()
            };
            zip.write_all(&content).unwrap();
        }
        zip.finish().unwrap();
        assert!(load(&path, &token).is_err());
    }
}
