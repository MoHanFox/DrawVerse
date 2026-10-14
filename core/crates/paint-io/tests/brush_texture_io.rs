use paint_core::*;
use std::sync::Arc;
#[test]
fn unidentified_texture_is_never_discarded_by_state() {
    for enabled in [false, true] {
        let mut c = BrushCatalog::default();
        c.edit(
            "round-pressure",
            BrushSettings {
                texture: BrushTexture::new(
                    enabled,
                    TextureSettings::default(),
                    Some(Arc::new(TextureResource::new(Arc::new(
                        BrushMask::new(2, 1, vec![0, 255]).unwrap(),
                    )))),
                )
                .unwrap(),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(paint_io::brush_state::encode(&c).is_err());
        assert!(c.selected().settings().texture.resource().is_some());
    }
}
