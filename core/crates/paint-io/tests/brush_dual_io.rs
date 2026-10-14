use paint_core::*;
#[test]
fn nondefault_closed_or_active_dual_is_never_silently_discarded() {
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
        assert!(paint_io::brush_state::encode(&catalog).is_err());
        assert_eq!(catalog.selected().settings().dual.enabled(), enabled);
    }
    assert!(paint_io::brush_state::encode(&BrushCatalog::default()).is_ok());
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
