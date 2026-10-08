use image::{ExtendedColorType, ImageEncoder};
use paint_core::{
    Document, DocumentOptions, ImportedLayer, LayerProperties, Pixel, Tile, TileCoord,
};
use paint_io::{load, save, Error, ExportOptions, Format};
use paint_task::CancellationToken;
use std::{
    collections::BTreeMap,
    io::{Cursor, Write},
    path::Path,
};
fn document() -> Document {
    let mut pixels = vec![Pixel::TRANSPARENT; 4096];
    pixels[4 * 64 + 3] = Pixel::from_straight([0.2, 0.6, 0.1, 0.5]).unwrap();
    let bottom = ImportedLayer {
        name: "底层 & <🖌>\n".into(),
        properties: LayerProperties::default(),
        tiles: BTreeMap::from([(TileCoord { x: 0, y: 0 }, Tile::from_pixels(pixels).unwrap())]),
    };
    let top = ImportedLayer {
        name: "Hidden".into(),
        properties: LayerProperties {
            visible: false,
            opacity: 0.3,
        },
        tiles: BTreeMap::new(),
    };
    Document::from_import(17, 19, vec![bottom, top], 1, DocumentOptions::default()).unwrap()
}
fn options(format: Format) -> ExportOptions {
    ExportOptions {
        format,
        quality: 100,
        background: [1.; 3],
    }
}

#[test]
fn nested_ora_preserves_groups_properties_selection_and_raster_exports() {
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let mut doc = document();
    let inner = doc.group_layer(1, "Inner & <组>").unwrap();
    doc.set_active_layer(inner).unwrap();
    let child = doc.add_layer("paint").unwrap();
    doc.begin_stroke(
        paint_core::Brush {
            radius: 4.,
            color: Pixel::from_straight([0.6, 0.2, 0.8, 1.]).unwrap(),
            ..Default::default()
        },
        paint_core::InputPoint::new(8.5, 8.5, 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    let outer = doc.group_layer(inner, "Outer").unwrap();
    doc.set_layer_properties(
        outer,
        LayerProperties {
            visible: true,
            opacity: 0.6,
        },
    )
    .unwrap();
    let mut a = doc.layer(inner).unwrap().appearance();
    a.blend = paint_core::BlendMode::Screen;
    a.fill = 0.7;
    a.locks = paint_core::LOCK_POSITION;
    doc.set_layer_appearance(inner, a).unwrap();
    let path = temp.path().join("nested.ora");
    save(
        &path,
        &doc.snapshot(),
        outer,
        options(Format::OpenRaster),
        &token,
    )
    .unwrap();
    let loaded = load(&path, &token).unwrap();
    assert_eq!(loaded.layers().len(), doc.layers().len());
    assert_eq!(loaded.layer(loaded.active_layer()).unwrap().name(), "Outer");
    for (before, after) in doc.layers().iter().zip(loaded.layers()) {
        assert_eq!(before.name(), after.name());
        assert_eq!(before.is_group(), after.is_group());
        assert_eq!(
            doc.layer(before.parent_id()).map(|l| l.name()),
            loaded.layer(after.parent_id()).map(|l| l.name())
        );
        assert_eq!(before.appearance(), after.appearance());
        assert_eq!(before.properties(), after.properties());
    }
    let imported_child = loaded
        .layers()
        .iter()
        .find(|l| l.name() == doc.layer(child).unwrap().name())
        .unwrap();
    assert_eq!(
        loaded.layer(imported_child.parent_id()).unwrap().name(),
        doc.layer(inner).unwrap().name()
    );
    for y in 0..19 {
        for x in 0..17 {
            for (a, b) in doc
                .pixel(x, y)
                .components()
                .into_iter()
                .zip(loaded.pixel(x, y).components())
            {
                assert!((a - b).abs() < 0.006);
            }
        }
    }
    let mut zip = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
    let mut xml = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("stack.xml").unwrap(), &mut xml).unwrap();
    assert_eq!(xml.matches("isolation=\"isolate\"").count(), 2);
    for (format, name) in [
        (Format::Png, "p.png"),
        (Format::Jpeg, "p.jpg"),
        (Format::WebP, "p.webp"),
    ] {
        let path = temp.path().join(name);
        save(&path, &doc.snapshot(), outer, options(format), &token).unwrap();
        assert_eq!(load(&path, &token).unwrap().dimensions(), doc.dimensions());
    }
}

#[test]
fn external_isolated_groups_import_but_excessive_depth_and_passthrough_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("external.ora");
    let token = CancellationToken::default();
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&[255, 0, 0, 255], 1, 1, ExtendedColorType::Rgba8)
        .unwrap();
    archive(&path,"<image w='1' h='1'><stack><stack name='outer' opacity='0.5' composite-op='svg:screen'><stack name='inner' isolation='isolate'><layer src='data/a.png'/></stack></stack></stack></image>",None,&png);
    let doc = load(&path, &token).unwrap();
    assert_eq!(doc.layers().len(), 3);
    assert_eq!(doc.layer_hierarchy(1).unwrap().1, 2);
    assert_eq!(doc.pixel(0, 0).components(), [0.5, 0., 0., 0.5]);
    assert_eq!(
        doc.layer(3).unwrap().appearance().blend,
        paint_core::BlendMode::Screen
    );
    for group in [
        "<stack isolation='auto'>",
        "<stack x='1'>",
        "<stack opacity='NaN'>",
        "<stack isolation='unknown'>",
    ] {
        archive(&path,&format!("<image w='1' h='1'><stack>{group}<layer src='data/a.png'/></stack></stack></image>"),None,&png);
        assert!(load(&path, &token).is_err());
    }
    let xml = format!(
        "<image w='1' h='1'><stack>{}<layer src='data/a.png'/>{}</stack></image>",
        "<stack>".repeat(17),
        "</stack>".repeat(17)
    );
    archive(&path, &xml, None, &png);
    assert!(matches!(load(&path, &token), Err(Error::Limit(_))));
}

#[test]
fn ora_layer_extension_preserves_fill_blend_locks_position_and_off_canvas_content() {
    let files = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let mut doc = Document::new(128, 64).unwrap();
    doc.begin_stroke(
        paint_core::Brush {
            radius: 6.,
            color: Pixel::from_straight([0.8, 0.2, 0.3, 0.7]).unwrap(),
            ..Default::default()
        },
        paint_core::InputPoint::new(10.5, 20.5, 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    doc.move_layer(1, 70, 0).unwrap();
    doc.begin_stroke(
        paint_core::Brush {
            radius: 4.,
            ..Default::default()
        },
        paint_core::InputPoint::new(2.5, 20.5, 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    doc.move_layer(1, 140, -2).unwrap();
    let mut a = doc.layer(1).unwrap().appearance();
    a.fill = 0.37;
    a.blend = paint_core::BlendMode::Dissolve;
    a.locks = 7;
    a.dissolve_seed = 8765;
    doc.set_layer_appearance(1, a).unwrap();
    doc.set_layer_properties(
        1,
        LayerProperties {
            visible: false,
            opacity: 1.,
        },
    )
    .unwrap();
    let path = files.path().join("layers.ora");
    save(
        &path,
        &doc.snapshot(),
        1,
        options(Format::OpenRaster),
        &token,
    )
    .unwrap();
    let mut restored = load(&path, &token).unwrap();
    assert_eq!(restored.layer(1).unwrap().appearance(), a);
    assert!(!restored.layer(1).unwrap().properties().visible);
    assert!(restored
        .layer(1)
        .unwrap()
        .tiles()
        .any(|(c, _)| c.signed_x() < 0));
    a.locks = 0;
    doc.set_layer_appearance(1, a).unwrap();
    restored.set_layer_appearance(1, a).unwrap();
    a.blend = paint_core::BlendMode::Normal;
    a.fill = 1.;
    doc.set_layer_appearance(1, a).unwrap();
    restored.set_layer_appearance(1, a).unwrap();
    for document in [&mut doc, &mut restored] {
        document
            .set_layer_properties(1, LayerProperties::default())
            .unwrap();
        document.move_layer(1, -210, 2).unwrap();
    }
    for (x, y) in [(10, 20), (9, 18)] {
        let actual = restored.pixel(x, y).components();
        let expected = doc.pixel(x, y).components();
        assert!(actual
            .iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 0.01));
    }
}
#[test]
fn imports_keep_selected_storage_and_pixel_budget_for_all_formats() {
    let source = Document::from_import(
        192,
        64,
        vec![ImportedLayer {
            name: "paged".into(),
            properties: LayerProperties::default(),
            tiles: (0..3)
                .map(|x| {
                    (
                        TileCoord { x, y: 0 },
                        Tile::from_pixels(vec![
                            Pixel::from_straight([0.2, 0.5, 0.7, 1.]).unwrap();
                            4096
                        ])
                        .unwrap(),
                    )
                })
                .collect(),
        }],
        0,
        DocumentOptions::default(),
    )
    .unwrap();
    let files = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let space = paint_storage::ScratchSpace::new(paint_storage::Config {
        directory: scratch.path().into(),
        min_free_bytes: 0,
    });
    for (format, name) in [
        (Format::Png, "p.png"),
        (Format::Jpeg, "p.jpg"),
        (Format::WebP, "p.webp"),
        (Format::OpenRaster, "p.ora"),
    ] {
        let path = files.path().join(name);
        save(&path, &source.snapshot(), 1, options(format), &token).unwrap();
        let (loaded, actual) = paint_io::load_with_storage(
            &path,
            &token,
            DocumentOptions {
                max_resident_tiles: 1,
                ..Default::default()
            },
            space.clone(),
        )
        .unwrap();
        assert_eq!(actual, format);
        assert_eq!(loaded.storage().config().directory, scratch.path());
        assert_eq!(loaded.document_options().max_resident_tiles, 1);
        assert!(loaded.storage_stats().page_writes > 0);
        for x in [0, 80, 191] {
            let a = loaded.try_pixel(x, 32).unwrap().components();
            let b = source.pixel(x, 32).components();
            assert!(a.iter().zip(b).all(|(a, b)| (a - b).abs() < 0.02));
        }
    }
    drop(space);
    assert_eq!(
        std::fs::read_dir(scratch.path().join(".drawverse-scratch-v1"))
            .unwrap()
            .count(),
        1
    );
}
#[test]
fn paged_snapshot_exports_all_formats_and_reopens_without_missing_pixels() {
    let tiles = (0..3)
        .map(|x| {
            (
                TileCoord { x, y: 0 },
                Tile::from_pixels(vec![
                    Pixel::from_straight([0.2, 0.5, 0.7, 1.]).unwrap();
                    4096
                ])
                .unwrap(),
            )
        })
        .collect();
    let doc = Document::from_import(
        192,
        64,
        vec![ImportedLayer {
            name: "paged".into(),
            properties: LayerProperties::default(),
            tiles,
        }],
        0,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    for (format, name) in [
        (Format::Png, "p.png"),
        (Format::Jpeg, "p.jpg"),
        (Format::WebP, "p.webp"),
        (Format::OpenRaster, "p.ora"),
    ] {
        let path = temp.path().join(name);
        save(&path, &doc.snapshot(), 1, options(format), &token).unwrap();
        let loaded = load(&path, &token).unwrap();
        assert_eq!(loaded.dimensions(), (192, 64));
        for (x, y) in [(0, 0), (80, 32), (191, 63)] {
            let actual = loaded.try_pixel(x, y).unwrap().components();
            let expected = doc.try_pixel(x, y).unwrap().components();
            for (a, b) in actual.into_iter().zip(expected) {
                assert!((a - b).abs() < 0.02, "{name}: {a} != {b}");
            }
        }
    }
    doc.trim_storage().unwrap();
    assert!(doc.storage_stats().resident_tiles <= 1);
    assert!(doc.storage_stats().page_reads > 0);
}
#[test]
fn scratch_exhaustion_during_export_preserves_existing_destination() {
    // One uncompressible cold tile fills the minimum scratch budget. Loading
    // it requires eviction of a second raw tile, which must fail explicitly.
    let tiles = (0..2)
        .map(|x| {
            let pixels = (0..4096)
                .map(|i| Pixel::from_straight([i as f32 / 4096., 0.5, 0.7, 1.]).unwrap())
                .collect();
            (TileCoord { x, y: 0 }, Tile::from_pixels(pixels).unwrap())
        })
        .collect();
    let doc = Document::from_import(
        128,
        64,
        vec![ImportedLayer {
            name: "noise".into(),
            properties: LayerProperties::default(),
            tiles,
        }],
        0,
        DocumentOptions {
            max_resident_tiles: 1,
            max_scratch_bytes: 65552,
            ..Default::default()
        },
    )
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("keep.png");
    std::fs::write(&path, b"original").unwrap();
    let result = save(
        &path,
        &doc.snapshot(),
        1,
        options(Format::Png),
        &CancellationToken::default(),
    );
    assert!(matches!(
        result,
        Err(Error::Core(paint_core::Error::Storage(_)))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    assert!(doc.storage_stats().resident_tiles <= 1);
    let retry = save(
        &path,
        &doc.snapshot(),
        1,
        options(Format::Png),
        &CancellationToken::default(),
    );
    assert!(matches!(
        retry,
        Err(Error::Core(paint_core::Error::Storage(_)))
    ));
    assert!(doc.storage_stats().resident_tiles <= 1);
}
#[test]
fn rgba_files_use_straight_srgb_and_jpeg_uses_linear_matte() {
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let doc = document();
    for (format, name) in [
        (Format::Png, "a.png"),
        (Format::WebP, "a.webp"),
        (Format::Jpeg, "a.jpg"),
    ] {
        let path = temp.path().join(name);
        save(
            &path,
            &doc.snapshot(),
            doc.active_layer(),
            options(format),
            &token,
        )
        .unwrap();
        let result = load(&path, &token).unwrap();
        assert_eq!(result.dimensions(), (17, 19));
        assert_eq!(result.history_depth().0, 0);
        let p = result.snapshot().pixel(3, 4).components();
        if format == Format::Jpeg {
            assert_eq!(p[3], 1.);
            let white = result.snapshot().pixel(16, 18).components();
            assert!(white[..3].iter().all(|v| *v > 0.97));
        } else {
            let source = doc.snapshot().pixel(3, 4).components();
            for (a, b) in p.into_iter().zip(source) {
                assert!((a - b).abs() < 0.005, "{name}: {a} != {b}");
            }
        }
    }
}
#[test]
fn openraster_preserves_order_names_offsets_selection_properties_and_zip_layout() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("绘画.ora");
    let token = CancellationToken::default();
    let doc = document();
    save(
        &path,
        &doc.snapshot(),
        doc.active_layer(),
        options(Format::OpenRaster),
        &token,
    )
    .unwrap();
    let result = load(&path, &token).unwrap();
    assert_eq!(result.layers().len(), 2);
    assert_eq!(result.layers()[0].name(), doc.layers()[0].name());
    assert_eq!(
        result.layers()[1].properties(),
        doc.layers()[1].properties()
    );
    assert_eq!(result.active_layer(), 2);
    assert_eq!(result.history_depth().0, 0);
    let p = result.snapshot().pixel(3, 4).components();
    assert!((p[3] - 0.5).abs() < 0.003);
    assert_eq!(result.snapshot().pixel(0, 0), Pixel::TRANSPARENT);
    let bytes = std::fs::read(path).unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let f = zip.by_index(0).unwrap();
    assert_eq!(f.name(), "mimetype");
    assert_eq!(f.compression(), zip::CompressionMethod::Stored);
    drop(f);
    assert!(zip.by_name("mergedimage.png").is_ok());
    assert!(zip.by_name("Thumbnails/thumbnail.png").is_ok());
}
#[test]
fn cancelled_or_failed_atomic_save_leaves_existing_file_and_no_temporary() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("a.ora");
    std::fs::write(&path, b"existing").unwrap();
    let token = CancellationToken::default();
    token.cancel();
    let doc = document();
    assert!(matches!(
        save(
            &path,
            &doc.snapshot(),
            2,
            options(Format::OpenRaster),
            &token
        ),
        Err(Error::Cancelled)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), b"existing");
    let invalid = ExportOptions {
        quality: 0,
        ..options(Format::Png)
    };
    assert!(save(
        &path,
        &doc.snapshot(),
        2,
        invalid,
        &CancellationToken::default()
    )
    .is_err());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    save(
        &path,
        &doc.snapshot(),
        2,
        options(Format::OpenRaster),
        &CancellationToken::default(),
    )
    .unwrap();
    assert!(load(&path, &CancellationToken::default()).is_ok());
}
#[test]
fn sixteen_bit_png_is_not_reduced_to_eight_bits_on_import() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("16.png");
    let mut bytes = Vec::new();
    let data = [10001u16, 20002, 30003, 40004]
        .into_iter()
        .flat_map(u16::to_ne_bytes)
        .collect::<Vec<_>>();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(&data, 1, 1, ExtendedColorType::Rgba16)
        .unwrap();
    std::fs::write(&path, bytes).unwrap();
    let doc = load(&path, &CancellationToken::default()).unwrap();
    assert!((doc.snapshot().pixel(0, 0).alpha() - 40004. / 65535.).abs() < 1e-6);
}
#[test]
fn embedded_profiles_and_truncated_images_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("profile.png");
    let mut bytes = Vec::new();
    let mut encoder = image::codecs::png::PngEncoder::new(&mut bytes);
    encoder.set_icc_profile(vec![0; 128]).unwrap();
    encoder
        .write_image(&[1, 2, 3, 255], 1, 1, ExtendedColorType::Rgba8)
        .unwrap();
    std::fs::write(&path, &bytes).unwrap();
    assert!(matches!(
        load(&path, &CancellationToken::default()),
        Err(Error::Unsupported(_))
    ));
    std::fs::write(&path, &bytes[..20]).unwrap();
    assert!(load(&path, &CancellationToken::default()).is_err());
    std::fs::write(&path, b"not an image").unwrap();
    assert!(load(&path, &CancellationToken::default()).is_err());
    let mut plain = Vec::new();
    image::codecs::png::PngEncoder::new(&mut plain)
        .write_image(&[1, 2, 3, 255], 1, 1, ExtendedColorType::Rgba8)
        .unwrap();
    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut body = kind.to_vec();
        body.extend(data);
        let mut crc = u32::MAX;
        for b in &body {
            crc ^= u32::from(*b);
            for _ in 0..8 {
                crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 };
            }
        }
        let mut chunk = (data.len() as u32).to_be_bytes().to_vec();
        chunk.extend(body);
        chunk.extend((!crc).to_be_bytes());
        chunk
    }
    for (kind, data) in [
        (b"gAMA", 100000u32.to_be_bytes().to_vec()),
        (b"acTL", [1u32.to_be_bytes(), 0u32.to_be_bytes()].concat()),
    ] {
        let mut bytes = plain[..33].to_vec();
        bytes.extend(chunk(kind, &data));
        bytes.extend(&plain[33..]);
        std::fs::write(&path, bytes).unwrap();
        assert!(matches!(
            load(&path, &CancellationToken::default()),
            Err(Error::Unsupported(_))
        ));
    }
    let mut bytes = plain[..33].to_vec();
    bytes.extend(chunk(b"gAMA", &45455u32.to_be_bytes()));
    bytes.extend(&plain[33..]);
    std::fs::write(&path, bytes).unwrap();
    assert!(load(&path, &CancellationToken::default()).is_ok());
}
fn archive(path: &Path, xml: &str, extra: Option<&str>, layer: &[u8]) {
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let o = zip::write::SimpleFileOptions::default();
    for (name, bytes) in [
        ("mimetype", b"image/openraster".as_slice()),
        ("stack.xml", xml.as_bytes()),
        ("data/a.png", layer),
        ("mergedimage.png", layer),
        ("Thumbnails/thumbnail.png", layer),
    ] {
        zip.start_file(
            name,
            o.compression_method(if name == "mimetype" {
                zip::CompressionMethod::Stored
            } else {
                zip::CompressionMethod::Deflated
            }),
        )
        .unwrap();
        zip.write_all(bytes).unwrap();
    }
    if let Some(name) = extra {
        zip.start_file(name, o).unwrap();
        zip.write_all(b"x").unwrap();
    }
    zip.finish().unwrap();
}
#[test]
fn hostile_or_unsupported_openraster_does_not_partially_import() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("bad.ora");
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&[255, 0, 0, 255], 1, 1, ExtendedColorType::Rgba8)
        .unwrap();
    for xml in [
        "<image w='1' h='1'><stack><layer src='../a.png'/></stack></image>",
        "<!DOCTYPE image [<!ENTITY bad SYSTEM 'file:///x'>]><image w='1' h='1'><stack/></image>",
        "<image w='1' h='1'><stack><stack isolation='auto'><layer src='data/a.png'/></stack></stack></image>",
        "<image w='1' h='1'><stack><layer src='data/a.png' composite-op='svg:dst-in'/></stack></image>",
        "<image w='1' h='1'><stack><layer src='data/a.png' x='1'/></stack></image>",
        "<image w='999999' h='999999'><stack><layer src='data/a.png'/></stack></image>",
        "<image w='1' h='1'><stack><layer src='data/a.png' opacity='NaN'/></stack></image>",
        "<image w='1' h='1' xmlns:dv='urn:drawverse:layers:1' dv:version='2'><stack><layer src='data/a.png'/></stack></image>",
        "<image w='1' h='1' xmlns:dv='urn:drawverse:layers:1' dv:version='1'><stack><layer src='data/a.png' dv:locks='8'/></stack></image>",
        "<image w='1' h='1' xmlns:dv='urn:drawverse:layers:1' dv:version='1'><stack><layer src='data/a.png' dv:blend='27'/></stack></image>",
        "<image w='1' h='1' xmlns:dv='urn:drawverse:layers:1' dv:version='1'><stack><layer src='data/a.png' dv:offset-x='1' dv:local-x='0'/></stack></image>",
        "<image w='1' h='1' xmlns:dv='urn:drawverse:layers:1' dv:version='1'><stack><layer src='data/a.png' composite-op='svg:multiply' dv:blend='0'/></stack></image>",
        "<image w='1' h='1'><stack><layer src='data/a.png'/></stack>",
    ] {archive(&path,xml,None,&png);assert!(load(&path,&CancellationToken::default()).is_err(),"{xml}");}
    archive(
        &path,
        "<image w='1' h='1'><stack><layer src='data/a.png'/></stack></image>",
        Some("../escape"),
        &png,
    );
    assert!(load(&path, &CancellationToken::default()).is_err());
    assert!(!temp.path().join("escape").exists());
}

#[test]
fn external_standard_blends_import_and_export_without_private_attributes() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("standard.ora");
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&[255, 0, 0, 255], 1, 1, ExtendedColorType::Rgba8)
        .unwrap();
    let token = CancellationToken::default();
    for (name, mode) in [
        ("svg:multiply", 3),
        ("svg:soft-light", 13),
        ("svg:color", 25),
        ("svg:hue", 23),
        ("svg:screen", 8),
    ] {
        archive(&path,&format!("<image w='1' h='1'><stack><layer src='data/a.png' composite-op='{name}'/></stack></image>"),None,&png);
        let doc = load(&path, &token).unwrap();
        assert_eq!(doc.layer(1).unwrap().appearance().blend as u32, mode);
        save(
            &path,
            &doc.snapshot(),
            1,
            options(Format::OpenRaster),
            &token,
        )
        .unwrap();
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("stack.xml").unwrap(), &mut xml).unwrap();
        assert!(xml.contains(&format!("composite-op=\"{name}\"")));
    }
}

#[test]
fn jpeg_exif_orientation_is_applied_before_document_creation() {
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
        .encode(&[100; 18], 2, 3, ExtendedColorType::Rgb8)
        .unwrap();
    // Little-endian TIFF IFD: orientation tag 0x0112, SHORT, one value, rotate 90.
    let exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    let mut bytes = jpeg[..2].to_vec();
    bytes.extend([255, 225]);
    bytes.extend(((exif.len() + 2) as u16).to_be_bytes());
    bytes.extend(exif);
    bytes.extend(&jpeg[2..]);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("orientation.jpg");
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(
        load(&path, &CancellationToken::default())
            .unwrap()
            .dimensions(),
        (3, 2)
    );
}

#[test]
fn archive_budgets_duplicates_and_late_save_failure_are_atomic() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("bad.ora");
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&[0, 0, 0, 255], 1, 1, ExtendedColorType::Rgba8)
        .unwrap();
    let xml = "<image w='1' h='1'><stack><layer src='data/a.png'/></stack></image>";
    archive(&path, xml, Some("data/b.png"), &png);
    let mut bytes = std::fs::read(&path).unwrap();
    for start in 0..bytes.len() - 10 {
        if &bytes[start..start + 10] == b"data/b.png" {
            bytes[start + 5] = b'a';
        }
    }
    std::fs::write(&path, bytes).unwrap();
    assert!(load(&path, &CancellationToken::default()).is_err());
    archive(&path, xml, None, &png);
    let mut bytes = std::fs::read(&path).unwrap();
    let header = bytes
        .windows(4)
        .enumerate()
        .filter(|(_, p)| *p == b"PK\x01\x02")
        .nth(1)
        .unwrap()
        .0;
    bytes[header + 24..header + 28].copy_from_slice(&(513u32 * 1024 * 1024).to_le_bytes());
    std::fs::write(&path, bytes).unwrap();
    assert!(matches!(
        load(&path, &CancellationToken::default()),
        Err(Error::Limit(_))
    ));
    let oversized = temp.path().join("large.png");
    std::fs::File::create(&oversized)
        .unwrap()
        .set_len(paint_io::MAX_INPUT + 1)
        .unwrap();
    assert!(matches!(
        load(&oversized, &CancellationToken::default()),
        Err(Error::Limit(_))
    ));
    std::fs::remove_file(oversized).unwrap();
    let mut doc = document();
    doc.add_layer("XML\u{1}control").unwrap();
    std::fs::write(&path, b"original").unwrap();
    assert!(matches!(
        save(
            &path,
            &doc.snapshot(),
            doc.active_layer(),
            options(Format::OpenRaster),
            &CancellationToken::default()
        ),
        Err(Error::Unsupported(_))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
}
