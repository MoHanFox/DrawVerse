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
fn paint(d: &mut Document, c: [f32; 4], radius: f32) {
    d.begin_stroke(
        Brush {
            radius,
            color: Pixel::from_straight(c).unwrap(),
            ..Default::default()
        },
        InputPoint::new(32., 32., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
}
#[test]
fn clipping_ora_roundtrip_retains_editable_chain_and_flat_exports() {
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let mut d = Document::new(64, 64).unwrap();
    paint(&mut d, [0., 1., 0., 0.5], 15.);
    let group = d.group_layer(1, "base").unwrap();
    let mask = d.add_mask(group).unwrap();
    paint(&mut d, [0., 0., 0., 1.], 5.);
    d.set_active_layer(group).unwrap();
    d.add_layer("top").unwrap();
    paint(&mut d, [1., 0., 0., 1.], 30.);
    let top = d.active_layer();
    d.set_layer_clipping(top, true).unwrap();
    let path = temp.path().join("clip.ora");
    save(
        &path,
        &d.snapshot(),
        top,
        options(Format::OpenRaster),
        &token,
    )
    .unwrap();
    let mut restored = load(&path, &token).unwrap();
    assert!(restored
        .layer(restored.active_layer())
        .unwrap()
        .is_clipped());
    assert_eq!(restored.layers().iter().filter(|l| l.is_mask()).count(), 1);
    for y in 0..64 {
        for x in 0..64 {
            for (a, b) in d
                .pixel(x, y)
                .components()
                .into_iter()
                .zip(restored.pixel(x, y).components())
            {
                assert!((a - b).abs() < 0.006);
            }
        }
    }
    restored
        .set_layer_clipping(restored.active_layer(), false)
        .unwrap();
    assert!(restored.pixel(5, 32).alpha() > 0.);
    restored.undo().unwrap();
    assert_eq!(restored.pixel(5, 32).alpha(), 0.);
    for format in [Format::Png, Format::WebP, Format::Jpeg] {
        let path = temp.path().join(format!("flat.{format:?}"));
        save(&path, &d.snapshot(), mask, options(format), &token).unwrap();
        let image = load(&path, &token).unwrap();
        assert_eq!(
            image.pixel(32, 32).alpha(),
            if format == Format::Jpeg { 1. } else { 0. }
        );
    }
}
#[test]
fn v2_masks_remain_readable_and_invalid_clipping_metadata_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let mut d = Document::new(64, 64).unwrap();
    d.initialize_white_background().unwrap();
    d.add_mask(1).unwrap();
    let good = temp.path().join("good.ora");
    save(
        &good,
        &d.snapshot(),
        d.active_layer(),
        options(Format::OpenRaster),
        &token,
    )
    .unwrap();
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&good).unwrap()).unwrap();
    let mut entries = Vec::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        entries.push((f.name().to_owned(), data));
    }
    for case in 0..3 {
        let path = temp.path().join(format!("case{case}.ora"));
        let mut out = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        for (name, bytes) in &entries {
            out.start_file(
                name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
            let data = if name == "drawverse/stack.xml" {
                let s = String::from_utf8(bytes.clone()).unwrap();
                match case {
                    0 => s
                        .replace("dv:version=\"4\"", "dv:version=\"2\"")
                        .replace(" dv:selection=\"1|0\"", "")
                        .replace(" dv:clipped=\"0\"", ""),
                    1 => s.replace("dv:clipped=\"0\"", "dv:clipped=\"2\""),
                    _ => s
                        .replace("dv:mask=\"1\"", "dv:mask=\"1\" dv:clipped=\"1\"")
                        .replace(" dv:clipped=\"0\"", ""),
                }
                .into_bytes()
            } else {
                bytes.clone()
            };
            out.write_all(&data).unwrap();
        }
        out.finish().unwrap();
        if case == 0 {
            load(&path, &token).unwrap();
        } else {
            assert!(load(&path, &token).is_err());
        }
    }
}
