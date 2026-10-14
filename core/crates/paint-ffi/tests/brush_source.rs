use paint_ffi::*;
use paint_io::{
    abr_conversion::ConvertedPreset,
    brush_resource::ResourceStore,
    brush_source::SourceArchive,
    ps_descriptor::{Class, Descriptor},
};
use paint_task::CancellationToken;
use std::{ptr, sync::Arc};

#[test]
fn color_package_public_entry_preserves_closed_model_and_seed() {
    use std::collections::BTreeMap;
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let json=br#"{"packageId":"12345678-1234-4234-8234-123456789abc","brushes":[{"id":"12345678-1234-4234-8234-123456789abd","name":"color","engineProfile":"drawverse.brush","behaviorVersion":7,"requiredCapabilities":["procedural-tip","color-dynamics"],"parameters":{"diameter":16,"spacing":0.01,"dabOpacity":1,"hardness":1,"roundness":1,"angle":0,"flow":1,"pressureSize":false,"pressureFlow":false,"colorDynamics":{"model":"linear-hsv-v1","enabled":false,"foregroundBackground":1,"hue":0.5,"saturation":0.3,"value":0.2,"perTip":true,"seed":"ffffffffffffffff"}}}]}"#;
    let p = paint_io::vbr::Package::from_json(json).unwrap();
    let b = paint_io::vbr_image::ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
    let input = temp.path().join("color.vbr");
    paint_io::vbr_image::save(&input, &b, &token).unwrap();
    let target = temp.path().join("old.vbr");
    std::fs::write(&target, b"original").unwrap();
    // Caller strings, owned handles and buffers stay alive throughout synchronous calls.
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let path = input.to_str().unwrap();
        let mut buffer = vec![42; 2 * 1024 * 1024];
        let mut required = 99;
        assert_eq!(
            paint_brush_library_package_read(
                library,
                path.as_ptr(),
                path.len() as u64,
                buffer.as_mut_ptr(),
                buffer.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        let package = paint_io::vbr::Package::from_json(&buffer[..required as usize]).unwrap();
        assert_eq!(
            package.brushes[0]
                .parameters
                .color_dynamics
                .unwrap()
                .core()
                .seed(),
            u64::MAX
        );
        assert!(buffer[required as usize..].iter().all(|b| *b == 42));
        let path = target.to_str().unwrap();
        assert_eq!(
            paint_brush_library_package_write(
                library,
                path.as_ptr(),
                path.len() as u64,
                json.as_ptr(),
                json.len() as u64
            ),
            PAINT_OK
        );
        let saved = paint_io::vbr_image::load(&target, &token).unwrap();
        let color = saved.package.brushes[0]
            .parameters
            .color_dynamics
            .unwrap()
            .core();
        assert_eq!(color.seed(), u64::MAX);
        assert!(!color.enabled());
        assert_eq!(color.hue(), 0.5);
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
}

#[test]
fn shape_package_public_entry_preserves_full_closed_configuration() {
    use std::collections::BTreeMap;
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let json=br#"{"packageId":"12345678-1234-4234-8234-123456789abc","brushes":[{"id":"12345678-1234-4234-8234-123456789abd","name":"shape","engineProfile":"drawverse.brush","behaviorVersion":6,"requiredCapabilities":["procedural-tip","shape-jitter"],"parameters":{"diameter":16,"spacing":0.01,"dabOpacity":1,"hardness":1,"roundness":1,"angle":0,"flow":1,"pressureSize":false,"pressureFlow":false,"shapeJitter":{"enabled":false,"size":0.5,"minimumSize":0.2,"angle":0.7,"roundness":0.6,"minimumRoundness":0.1,"seed":"ffffffffffffffff"}}}]}"#;
    let package = paint_io::vbr::Package::from_json(json).unwrap();
    let bundle = paint_io::vbr_image::ImageBundle::new(package, BTreeMap::new(), &token).unwrap();
    let input = temp.path().join("shape.vbr");
    paint_io::vbr_image::save(&input, &bundle, &token).unwrap();
    let target = temp.path().join("unchanged.vbr");
    std::fs::write(&target, b"original").unwrap();
    // Owned handles and all caller bytes remain live throughout each synchronous call.
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let path = input.to_str().unwrap();
        let mut buffer = vec![42; 2 * 1024 * 1024];
        let mut required = 99;
        assert_eq!(
            paint_brush_library_package_read(
                library,
                path.as_ptr(),
                path.len() as u64,
                buffer.as_mut_ptr(),
                buffer.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        let read = paint_io::vbr::Package::from_json(&buffer[..required as usize]).unwrap();
        assert_eq!(
            read.brushes[0]
                .parameters
                .shape_jitter
                .unwrap()
                .core()
                .seed(),
            u64::MAX
        );
        assert!(buffer[required as usize..].iter().all(|b| *b == 42));
        let path = target.to_str().unwrap();
        assert_eq!(
            paint_brush_library_package_write(
                library,
                path.as_ptr(),
                path.len() as u64,
                json.as_ptr(),
                json.len() as u64
            ),
            PAINT_OK
        );
        let saved = paint_io::vbr_image::load(&target, &token).unwrap();
        let shape = saved.package.brushes[0]
            .parameters
            .shape_jitter
            .unwrap()
            .core();
        assert_eq!(shape.seed(), u64::MAX);
        assert!(!shape.enabled());
        assert_eq!(shape.size(), 0.5);
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
}

#[test]
fn scatter_packages_round_trip_with_public_native_entry() {
    use std::collections::BTreeMap;
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let json=br#"{"packageId":"12345678-1234-4234-8234-123456789abc","brushes":[{"id":"12345678-1234-4234-8234-123456789abd","name":"scatter","engineProfile":"drawverse.brush","behaviorVersion":5,"requiredCapabilities":["procedural-tip","scatter-stamps"],"parameters":{"diameter":16,"spacing":0.01,"dabOpacity":1,"hardness":1,"roundness":1,"angle":0,"flow":1,"pressureSize":false,"pressureFlow":false,"scatterDynamics":{"enabled":false,"amount":1,"bothAxes":true,"count":2,"countJitter":0.25,"seed":"ffffffffffffffff"}}}]}"#;
    let p = paint_io::vbr::Package::from_json(json).unwrap();
    let b = paint_io::vbr_image::ImageBundle::new(p, BTreeMap::new(), &token).unwrap();
    let path = temp.path().join("scatter.vbr");
    paint_io::vbr_image::save(&path, &b, &token).unwrap();
    let target = temp.path().join("old.vbr");
    std::fs::write(&target, b"original").unwrap();
    // Buffers, strings and owned handles remain alive throughout each ABI call.
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let path = path.to_str().unwrap();
        let mut output = vec![42; 2 * 1024 * 1024];
        let mut required = 99;
        assert_eq!(
            paint_brush_library_package_read(
                library,
                path.as_ptr(),
                path.len() as u64,
                output.as_mut_ptr(),
                output.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        let read = paint_io::vbr::Package::from_json(&output[..required as usize]).unwrap();
        assert_eq!(
            read.brushes[0]
                .parameters
                .scatter_dynamics
                .unwrap()
                .core()
                .seed(),
            u64::MAX
        );
        assert!(output[required as usize..].iter().all(|b| *b == 42));
        let path = target.to_str().unwrap();
        assert_eq!(
            paint_brush_library_package_write(
                library,
                path.as_ptr(),
                path.len() as u64,
                json.as_ptr(),
                json.len() as u64
            ),
            PAINT_OK
        );
        let saved = paint_io::vbr_image::load(&target, &token).unwrap();
        let scatter = saved.package.brushes[0]
            .parameters
            .scatter_dynamics
            .unwrap()
            .core();
        assert_eq!(scatter.seed(), u64::MAX);
        assert!(!scatter.enabled());
        assert_eq!(scatter.count(), 2);
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
}

#[test]
fn device_packages_round_trip_with_public_native_entry() {
    use std::collections::BTreeMap;
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let json=br#"{"packageId":"12345678-1234-4234-8234-123456789abc","brushes":[{"id":"12345678-1234-4234-8234-123456789abd","name":"device","engineProfile":"drawverse.brush","behaviorVersion":4,"requiredCapabilities":["procedural-tip","tip-dynamics"],"parameters":{"diameter":16,"spacing":0.01,"dabOpacity":1,"hardness":1,"roundness":0.2,"angle":0,"flow":1,"pressureSize":false,"pressureFlow":false,"tipDynamics":{"angleControl":"direction","tiltRoundness":false,"minimumRoundness":0.01}}}]}"#;
    let package = paint_io::vbr::Package::from_json(json).unwrap();
    let bundle = paint_io::vbr_image::ImageBundle::new(package, BTreeMap::new(), &token).unwrap();
    let path = temp.path().join("device.vbr");
    paint_io::vbr_image::save(&path, &bundle, &token).unwrap();
    let target = temp.path().join("old.vbr");
    std::fs::write(&target, b"original").unwrap();
    // All caller buffers and handles remain alive for each synchronous ABI call.
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let encoded = path.to_str().unwrap();
        let mut output = vec![42; 2 * 1024 * 1024];
        let mut required = 99;
        assert_eq!(
            paint_brush_library_package_read(
                library,
                encoded.as_ptr(),
                encoded.len() as u64,
                output.as_mut_ptr(),
                output.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        let read = paint_io::vbr::Package::from_json(&output[..required as usize]).unwrap();
        assert_eq!(
            read.brushes[0]
                .parameters
                .tip_dynamics
                .unwrap()
                .core()
                .angle(),
            paint_core::AngleControl::Direction
        );
        assert!(output[required as usize..].iter().all(|b| *b == 42));
        let encoded = target.to_str().unwrap();
        assert_eq!(
            paint_brush_library_package_write(
                library,
                encoded.as_ptr(),
                encoded.len() as u64,
                json.as_ptr(),
                json.len() as u64
            ),
            PAINT_OK
        );
        let saved = paint_io::vbr_image::load(&target, &token).unwrap();
        assert_eq!(
            saved.definition(0).unwrap().dynamics.tip.angle(),
            paint_core::AngleControl::Direction
        );
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
}
#[test]
fn source_query_returns_exact_verified_record_and_clears_outputs_on_failure() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("resources");
    let resources =
        ResourceStore::open(directory.clone(), paint_core::MAX_CATALOG_MASK_BYTES).unwrap();
    let sources = resources.source_store().unwrap();
    let descriptor = Descriptor {
        class: Class {
            name: String::new(),
            id: b"null".to_vec(),
        },
        entries: Vec::new(),
    };
    let converted = ConvertedPreset {
        name: "original",
        settings: paint_core::BrushSettings::default(),
        source: &descriptor,
        issues: Vec::new(),
    };
    let archive = SourceArchive::capture(Arc::new(vec![0, 6, 0, 2, 42]), 0, &converted).unwrap();
    let token = paint_task::CancellationToken::default();
    let id = sources.save(&archive, &token).unwrap();
    let expected = sources.encoded_record(&id, &token).unwrap();
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let path = directory.to_str().unwrap();
        assert_eq!(
            paint_brush_library_bind_resources(library, path.as_ptr(), path.len() as u64),
            PAINT_OK
        );
        let mut required = 99;
        assert_eq!(
            paint_brush_library_source_info(
                library,
                id.as_ptr(),
                id.len() as u64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(required as usize, expected.len());
        let mut bytes = vec![0; required as usize];
        assert_eq!(
            paint_brush_library_source_info(
                library,
                id.as_ptr(),
                id.len() as u64,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        assert_eq!(bytes, expected);
        std::fs::write(
            directory.join("sources").join(format!("{id}.json")),
            b"corrupt",
        )
        .unwrap();
        required = 99;
        assert_eq!(
            paint_brush_library_source_info(
                library,
                id.as_ptr(),
                id.len() as u64,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut required
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(required, 0);
        assert_eq!(bytes, expected);
        assert_eq!(
            paint_brush_library_source_info(
                library,
                b"../bad".as_ptr(),
                6,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_INVALID_ARGUMENT
        );
        let stale = library;
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
        required = 99;
        assert_eq!(
            paint_brush_library_source_info(
                stale,
                id.as_ptr(),
                id.len() as u64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(required, 0);
    }
}

#[test]
fn native_reader_and_writer_exchange_source_bundles_and_never_drop_provenance() {
    use std::collections::BTreeMap;
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let original = Arc::new(vec![0, 6, 0, 2, 42]);
    let descriptor = Descriptor {
        class: Class {
            name: String::new(),
            id: b"null".to_vec(),
        },
        entries: Vec::new(),
    };
    let converted = ConvertedPreset {
        name: "source",
        settings: paint_core::BrushSettings::default(),
        source: &descriptor,
        issues: Vec::new(),
    };
    let archive = SourceArchive::capture(original.clone(), 0, &converted).unwrap();
    let record = archive.record().unwrap();
    let id = paint_io::brush_resource::hash(&record);
    let batch = paint_io::brush_source::SourceBatch::from_records(
        original,
        BTreeMap::from([(id.clone(), record)]),
        &token,
    )
    .unwrap();
    let package = paint_io::vbr::Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![paint_io::vbr::Preset {
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "source".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 1,
            required_capabilities: vec!["procedural-tip".into()],
            parameters: archive.info.baseline,
            tip_resource: None,
            source_resource: Some(id.clone()),
        }],
    };
    let bundle = paint_io::vbr_image::ImageBundle::with_sources(
        package,
        BTreeMap::new(),
        vec![batch],
        &token,
    )
    .unwrap();
    let path = temp.path().join("source.vbr");
    paint_io::vbr_image::save(&path, &bundle, &token).unwrap();
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let directory = temp.path().join("unused");
        let encoded = directory.to_str().unwrap();
        assert_eq!(
            paint_brush_library_bind_resources(library, encoded.as_ptr(), encoded.len() as u64),
            PAINT_OK
        );
        let encoded = path.to_str().unwrap();
        let mut required = 99;
        let mut output = vec![42; 2 * 1024 * 1024];
        assert_eq!(
            paint_brush_library_package_read(
                library,
                encoded.as_ptr(),
                encoded.len() as u64,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
        assert_eq!(
            paint_brush_library_package_read(
                library,
                encoded.as_ptr(),
                encoded.len() as u64,
                output.as_mut_ptr(),
                output.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        output.truncate(required as usize);
        assert_eq!(output, bundle.package.to_json().unwrap());
        let sources = paint_io::brush_source::SourceStore::open(directory.join("sources")).unwrap();
        assert_eq!(
            sources.load(&id, &token).unwrap().original(),
            [0, 6, 0, 2, 42]
        );
        let exported = temp.path().join("roundtrip.vbr");
        let target = exported.to_str().unwrap();
        assert_eq!(
            paint_brush_library_package_write(
                library,
                target.as_ptr(),
                target.len() as u64,
                output.as_ptr(),
                output.len() as u64
            ),
            PAINT_OK
        );
        let restored = paint_io::vbr_image::load(&exported, &token).unwrap();
        assert_eq!(restored.package.to_json().unwrap(), output);
        assert_eq!(
            restored.sources()[0].original_bytes(),
            bundle.sources()[0].original_bytes()
        );
        let saved = std::fs::read(&exported).unwrap();
        std::fs::write(
            directory
                .join("sources")
                .join(format!("{}.abr", restored.sources()[0].original_id())),
            b"corrupt",
        )
        .unwrap();
        assert_eq!(
            paint_brush_library_package_write(
                library,
                target.as_ptr(),
                target.len() as u64,
                output.as_ptr(),
                output.len() as u64
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(std::fs::read(exported).unwrap(), saved);
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
}

#[test]
fn pressure_packages_round_trip_through_native_entry() {
    use std::collections::BTreeMap;
    let temp = tempfile::tempdir().unwrap();
    let token = CancellationToken::default();
    let mut parameters = paint_io::vbr::Parameters {
        scatter_dynamics: None,
        shape_jitter: None,
        color_dynamics: None,
        tip_dynamics: None,
        diameter: 16.,
        spacing: 0.01,
        dab_opacity: 1.,
        hardness: 1.,
        roundness: 1.,
        angle: 0.,
        flow: 1.,
        pressure_size: true,
        pressure_flow: true,
        pressure_dynamics: None,
    };
    let dynamics = paint_core::BrushDynamics {
        pressure_opacity: true,
        flow: paint_core::PressureControl::new(paint_core::PressureCurve::default(), 0.2).unwrap(),
        ..Default::default()
    };
    parameters.pressure_dynamics = Some(paint_io::brush_pressure::PressureParameters::from_core(
        &dynamics,
    ));
    let package = paint_io::vbr::Package {
        package_id: "12345678-1234-4234-8234-123456789abc".into(),
        brushes: vec![paint_io::vbr::Preset {
            id: "12345678-1234-4234-8234-123456789abd".into(),
            name: "pressure".into(),
            engine_profile: "drawverse.brush".into(),
            behavior_version: 3,
            required_capabilities: vec![
                "procedural-tip".into(),
                "pressure-curves".into(),
                "opacity-pressure".into(),
            ],
            parameters,
            tip_resource: None,
            source_resource: None,
        }],
    };
    let json = package.to_json().unwrap();
    let bundle = paint_io::vbr_image::ImageBundle::new(package, BTreeMap::new(), &token).unwrap();
    let path = temp.path().join("pressure.vbr");
    paint_io::vbr_image::save(&path, &bundle, &token).unwrap();
    let target = temp.path().join("unchanged.vbr");
    std::fs::write(&target, b"old").unwrap();
    unsafe {
        let mut library = ptr::null_mut();
        assert_eq!(paint_brush_library_create(0, &mut library), PAINT_OK);
        let path = path.to_str().unwrap();
        let mut output = vec![42; 2 * 1024 * 1024];
        let mut required = 99;
        assert_eq!(
            paint_brush_library_package_read(
                library,
                path.as_ptr(),
                path.len() as u64,
                output.as_mut_ptr(),
                output.len() as u64,
                &mut required
            ),
            PAINT_OK
        );
        let data = paint_io::vbr::Package::from_json(&output[..required as usize]).unwrap();
        assert!(
            data.brushes[0]
                .parameters
                .pressure_dynamics
                .as_ref()
                .unwrap()
                .core()
                .pressure_opacity
        );
        let encoded = target.to_str().unwrap();
        assert_eq!(
            paint_brush_library_package_write(
                library,
                encoded.as_ptr(),
                encoded.len() as u64,
                json.as_ptr(),
                json.len() as u64
            ),
            PAINT_OK
        );
        let loaded = paint_io::vbr_image::load(&target, &token).unwrap();
        assert!(loaded.package.brushes[0]
            .parameters
            .pressure_dynamics
            .is_some());
        assert_eq!(paint_brush_library_destroy(&mut library), PAINT_OK);
    }
}
