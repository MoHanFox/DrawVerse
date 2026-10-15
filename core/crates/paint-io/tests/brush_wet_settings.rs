use paint_core::*;
use paint_io::brush_state;
#[test]
fn state17_preserves_open_and_closed_wet_settings_and_keeps_default_bytes() {
    let catalog = BrushCatalog::default();
    let original = brush_state::encode(&catalog).unwrap();
    for enabled in [false, true] {
        let mut edited = catalog.clone();
        let mut settings = edited.selected().settings().clone();
        settings.wet = WetSettings::new(
            enabled,
            true,
            false,
            false,
            WetMix::new(0.4, 0.3, 0.2, 0.5, 0.6, 0.7).unwrap(),
        );
        edited.edit("round-pressure", settings.clone()).unwrap();
        let bytes = brush_state::encode(&edited).unwrap();
        let decoded = brush_state::decode(&bytes, MAX_CATALOG_MASK_BYTES).unwrap();
        assert_eq!(decoded.selected().settings().wet, settings.wet);
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["version"], 17);
        for value in [
            serde_json::Value::Null,
            serde_json::json!({"model":"linear-premul-grid-v1","enabled":false,"allLayers":true,"autoClean":false,"autoLoad":false,"wetness":1.0000000001,"pickup":0.,"mix":0.,"load":0.,"deposit":0.,"consumption":0.}),
        ] {
            let mut bad = json.clone();
            bad["presets"][0]["wetMix"] = value;
            assert!(brush_state::decode(
                &serde_json::to_vec(&bad).unwrap(),
                MAX_CATALOG_MASK_BYTES
            )
            .is_err());
        }
        let mut old = json.clone();
        old["version"] = 16.into();
        assert!(
            brush_state::decode(&serde_json::to_vec(&old).unwrap(), MAX_CATALOG_MASK_BYTES)
                .is_err()
        );
        settings.wet = Default::default();
        edited.edit("round-pressure", settings).unwrap();
        assert_eq!(brush_state::encode(&edited).unwrap(), original);
    }
    assert_eq!(
        brush_state::encode(&brush_state::decode(&original, MAX_CATALOG_MASK_BYTES).unwrap())
            .unwrap(),
        original
    );
}

#[test]
fn source_baseline_does_not_silently_drop_disabled_wet_policy() {
    use paint_io::{
        abr_conversion::ConvertedPreset,
        brush_source::{SourceArchive, SourceBatch},
        ps_descriptor::{Class, Descriptor},
    };
    let descriptor = Descriptor {
        class: Class {
            name: String::new(),
            id: b"null".to_vec(),
        },
        entries: Vec::new(),
    };
    let original = std::sync::Arc::new(vec![0, 6, 0, 2, 42]);
    let token = paint_task::CancellationToken::default();
    for enabled in [false, true] {
        let preset = ConvertedPreset {
            name: "wet",
            settings: BrushSettings {
                wet: WetSettings::new(enabled, true, false, false, WetMix::default()),
                ..Default::default()
            },
            source: &descriptor,
            issues: Vec::new(),
        };
        assert!(matches!(
            SourceArchive::capture(original.clone(), 0, &preset),
            Err(paint_io::Error::Unsupported(_))
        ));
        assert!(matches!(
            SourceBatch::capture(original.clone(), std::slice::from_ref(&preset), &token),
            Err(paint_io::Error::Unsupported(_))
        ));
    }
}
