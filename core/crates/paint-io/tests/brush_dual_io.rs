use paint_core::*;
#[test]
fn state12_preserves_program_dual_and_disabled_path_and_rejects_downgrade() {
    for enabled in [false, true] {
        let mut catalog = BrushCatalog::default();
        catalog
            .edit(
                "round-pressure",
                BrushSettings {
                    dual: DualBrush::new(enabled, DualMode::Subtract, SecondaryTip::default()),
                    ..Default::default()
                },
            )
            .unwrap();
        let encoded = paint_io::brush_state::encode(&catalog).unwrap();
        let restored = paint_io::brush_state::decode(&encoded, MAX_CATALOG_MASK_BYTES).unwrap();
        assert_eq!(restored.selected().settings().dual.enabled(), enabled);
        assert_eq!(
            restored.selected().settings().dual.mode(),
            DualMode::Subtract
        );
        assert_eq!(encoded, paint_io::brush_state::encode(&restored).unwrap());
        let mut old: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        old["version"] = 11.into();
        assert!(paint_io::brush_state::decode(
            &serde_json::to_vec(&old).unwrap(),
            MAX_CATALOG_MASK_BYTES
        )
        .is_err());
    }
    let mut path = BrushCatalog::default();
    path.edit(
        "round-pressure",
        BrushSettings {
            dual: DualBrush::new(
                false,
                DualMode::Multiply,
                SecondaryTip::default().with_spacing(0.01).unwrap(),
            ),
            ..Default::default()
        },
    )
    .unwrap();
    let encoded = paint_io::brush_state::encode(&path).unwrap();
    let restored = paint_io::brush_state::decode(&encoded, MAX_CATALOG_MASK_BYTES).unwrap();
    assert_eq!(
        restored.selected().settings().dual.secondary().spacing(),
        Some(0.01)
    );
    let plain = paint_io::brush_state::encode(&BrushCatalog::default()).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&plain).unwrap()["version"],
        2
    );
}

#[test]
fn original_source_baseline_cannot_silently_drop_dual_settings() {
    use paint_io::{
        abr_conversion::ConvertedPreset,
        brush_source::SourceArchive,
        ps_descriptor::{Class, Descriptor},
    };
    use std::sync::Arc;
    let descriptor = Descriptor {
        class: Class {
            name: String::new(),
            id: b"null".to_vec(),
        },
        entries: Vec::new(),
    };
    let preset = ConvertedPreset {
        converter_version: 1,
        name: "dual",
        settings: BrushSettings {
            dual: DualBrush::new(false, DualMode::Subtract, SecondaryTip::default()),
            ..Default::default()
        },
        source: &descriptor,
        issues: Vec::new(),
    };
    assert!(matches!(
        SourceArchive::capture(Arc::new(vec![0, 6, 0, 2, 37]), 0, &preset),
        Err(paint_io::Error::Unsupported(_))
    ));
}
#[test]
fn large_dual_catalog_cannot_emit_a_state_the_reader_rejects_for_size() {
    let points = (0..17)
        .map(|i| {
            (
                i as f32 / 16.,
                if i == 0 {
                    0.
                } else if i == 16 {
                    1.
                } else {
                    0.12345678 + i as f32 * 0.04
                },
            )
        })
        .collect();
    let control = PressureControl::new(PressureCurve::new(points).unwrap(), 0.12345678).unwrap();
    let dynamics = BrushDynamics {
        size: control.clone(),
        flow: control.clone(),
        opacity: control,
        pressure_opacity: true,
        tip: TipDynamics::new(AngleControl::PenRotation, true, 0.12345678).unwrap(),
        scatter: ScatterDynamics::new(false, 10., true, 16, 1., u64::MAX).unwrap(),
        shape_jitter: ShapeJitter::new(false, 1., 1., 1., 1., 1., u64::MAX).unwrap(),
        color: ColorDynamics::new(false, 1., 1., 1., 1., true, u64::MAX).unwrap(),
        speed: Default::default(),
        tangent: Default::default(),
        distance: Default::default(),
        tilt: Default::default(),
        rotation: Default::default(),
    };
    let mut second = dynamics.clone();
    second.opacity = Default::default();
    second.pressure_opacity = false;
    second.color = Default::default();
    let settings = BrushSettings {
        dynamics,
        dual: DualBrush::new(
            false,
            DualMode::Screen,
            SecondaryTip::new(4., BrushTip::default(), None, second)
                .unwrap()
                .with_spacing(0.01)
                .unwrap(),
        ),
        ..Default::default()
    };
    let mut catalog = BrushCatalog::default();
    catalog.edit("round-pressure", settings).unwrap();
    for i in 0..1020 {
        catalog
            .save_copy(
                format!("12345678-1234-4234-8234-{i:012x}"),
                "bounded".into(),
            )
            .unwrap();
    }
    assert!(matches!(
        paint_io::brush_state::encode(&catalog),
        Err(paint_io::Error::Limit("brush state bytes"))
    ));
}
