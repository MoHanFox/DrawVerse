use paint_core::*;
use std::sync::Arc;
fn resource() -> Arc<TextureResource> {
    Arc::new(TextureResource::new(Arc::new(
        BrushMask::new(2, 1, vec![0, 255]).unwrap(),
    )))
}
fn brush(space: TextureSpace) -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            spacing: 0.1,
            opacity: 0.4,
            ..Default::default()
        },
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            ..Default::default()
        },
        texture: BrushTexture::new(
            true,
            TextureSettings::new(space, 1., 0., 1., false, 0., 0.).unwrap(),
            Some(resource()),
        )
        .unwrap(),
        ..Default::default()
    }
}
fn pixels(d: &Document) -> Vec<Pixel> {
    (0..96)
        .flat_map(|y| (0..96).map(move |x| d.pixel(x, y)))
        .collect()
}
fn line(d: &mut Document, b: BrushDefinition, split: bool) {
    d.begin_brush_stroke(b, InputPoint::new(16.5, 48.5, 1.))
        .unwrap();
    if split {
        d.stroke_to(InputPoint::new(48.5, 48.5, 1.)).unwrap();
    }
    d.stroke_to(InputPoint::new(80.5, 48.5, 1.)).unwrap();
    d.end_stroke().unwrap();
}
#[test]
fn document_texture_punches_holes_and_history_chunking_are_exact() {
    let mut a = Document::new(96, 96).unwrap();
    let mut b = Document::new(96, 96).unwrap();
    line(&mut a, brush(TextureSpace::Document), false);
    line(&mut b, brush(TextureSpace::Document), true);
    assert_eq!(pixels(&a), pixels(&b));
    for x in 24..72 {
        if x % 2 == 0 {
            assert_eq!(a.pixel(x, 48), Pixel::TRANSPARENT);
        } else {
            assert!((a.pixel(x, 48).alpha() - 0.4).abs() < 1e-6);
        }
    }
    let after = pixels(&a);
    a.undo().unwrap();
    assert!(pixels(&a).iter().all(|p| *p == Pixel::TRANSPARENT));
    a.redo().unwrap();
    assert_eq!(pixels(&a), after);
}
#[test]
fn fixed_paper_gray_is_not_filled_by_dense_spacing() {
    for opacity in [0.4, 1.] {
        let mut d = Document::new(96, 96).unwrap();
        let mut b = brush(TextureSpace::Document);
        b.brush.opacity = opacity;
        b.brush.spacing = 0.02;
        b.texture = BrushTexture::new(
            true,
            TextureSettings::default(),
            Some(Arc::new(TextureResource::new(Arc::new(
                BrushMask::new(1, 1, vec![128]).unwrap(),
            )))),
        )
        .unwrap();
        line(&mut d, b, false);
        assert!((d.pixel(48, 48).alpha() - opacity * 128. / 255.).abs() < 1e-6);
    }
}
#[test]
fn tip_space_moves_and_rotation_depth_inversion_change_actual_pixels() {
    let mut fixed = Document::new(96, 96).unwrap();
    let mut moving = Document::new(96, 96).unwrap();
    line(&mut fixed, brush(TextureSpace::Document), false);
    line(&mut moving, brush(TextureSpace::Tip), false);
    assert_ne!(pixels(&fixed), pixels(&moving));
    for (angle, depth, invert) in [(90., 1., false), (0., 0.5, false), (0., 1., true)] {
        let mut d = Document::new(96, 96).unwrap();
        let mut b = brush(TextureSpace::Document);
        b.texture = BrushTexture::new(
            true,
            TextureSettings::new(TextureSpace::Document, 1., angle, depth, invert, 0., 0.).unwrap(),
            Some(resource()),
        )
        .unwrap();
        line(&mut d, b, false);
        assert_ne!(pixels(&d), pixels(&fixed));
    }
    let mut disabled = brush(TextureSpace::Document);
    disabled.texture =
        BrushTexture::new(false, TextureSettings::default(), Some(resource())).unwrap();
    let mut old = disabled.clone();
    old.texture = Default::default();
    let mut a = Document::new(96, 96).unwrap();
    let mut b = Document::new(96, 96).unwrap();
    line(&mut a, disabled, false);
    line(&mut b, old, true);
    assert_eq!(pixels(&a), pixels(&b));
}
#[test]
fn shifted_layers_images_selection_erase_masks_and_finger_smudge_share_texture() {
    for mask in [false, true] {
        let mut d = Document::new(96, 96).unwrap();
        d.initialize_white_background().unwrap();
        if mask {
            d.add_mask(1).unwrap();
        }
        d.set_selection(
            Selection::default()
                .apply(
                    SelectionShape::geometry(SelectionKind::Rectangle, 48., 0., 48., 96., false),
                    SelectionOperation::Replace,
                    96,
                    96,
                )
                .unwrap(),
        )
        .unwrap();
        let mut b = brush(TextureSpace::Document);
        b.brush.mode = if mask {
            BrushMode::Paint
        } else {
            BrushMode::Erase
        };
        b.mask = Some(Arc::new(BrushMask::new(3, 1, vec![255, 0, 255]).unwrap()));
        line(&mut d, b, false);
        for y in 0..96 {
            for x in 0..96 {
                assert!(d.pixel(x, y).alpha() >= 0.599999);
                if x < 48 || x % 2 == 0 {
                    assert_eq!(d.pixel(x, y), Pixel::WHITE);
                }
            }
        }
        assert!((49..72).step_by(2).any(|x| d.pixel(x, 48).alpha() < 1.));
    }
    let mut d = Document::new(96, 96).unwrap();
    let mut a = d.layer(1).unwrap().appearance();
    a.offset_x = 3;
    d.set_layer_appearance(1, a).unwrap();
    line(&mut d, brush(TextureSpace::Document), false);
    assert_eq!(d.pixel(48, 48), Pixel::TRANSPARENT);
    assert!(d.pixel(49, 48).alpha() > 0.);
    let mut d = Document::new(96, 96).unwrap();
    let mut b = brush(TextureSpace::Document);
    b.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: true,
        pressure_strength: false,
    };
    b.brush.color = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
    d.begin_brush_stroke(b, InputPoint::new(48.5, 48.5, 1.))
        .unwrap();
    assert_eq!(d.pixel(48, 48), Pixel::TRANSPARENT);
    assert!(d.pixel(49, 48).components()[0] > 0.);
    d.cancel_stroke().unwrap();
    assert!(pixels(&d).iter().all(|p| *p == Pixel::TRANSPARENT));
}
#[test]
fn resource_capture_shared_budget_and_failure_leave_catalog_atomic() {
    let mask = Arc::new(BrushMask::new(4, 1, vec![0, 255, 0, 255]).unwrap());
    let r = Arc::new(TextureResource::new(mask.clone()));
    let bytes = mask.resident_bytes() + r.mip_bytes();
    let mut c = BrushCatalog::with_mask_budget(bytes).unwrap();
    let settings = BrushSettings {
        mask: Some(mask.clone()),
        texture: BrushTexture::new(false, TextureSettings::default(), Some(r.clone())).unwrap(),
        ..Default::default()
    };
    c.edit("round-pressure", settings.clone()).unwrap();
    c.save_copy(
        "12345678-1234-4234-8234-123456789abc".into(),
        "texture".into(),
    )
    .unwrap();
    let revision = c.revision();
    let mut other = settings.clone();
    other.texture = BrushTexture::new(
        true,
        TextureSettings::default(),
        Some(Arc::new(TextureResource::new(mask))),
    )
    .unwrap();
    assert!(matches!(
        c.edit("round-fine", other),
        Err(Error::ResourceLimit(_))
    ));
    assert_eq!(c.revision(), revision);
    let mut d = Document::new(96, 96).unwrap();
    let b = brush(TextureSpace::Document);
    d.begin_brush_stroke(b.clone(), InputPoint::new(16.5, 48.5, 1.))
        .unwrap();
    drop(b);
    d.stroke_to(InputPoint::new(80.5, 48.5, 1.)).unwrap();
    d.end_stroke().unwrap();
    assert!(d.pixel(49, 48).alpha() > 0.);
}

#[test]
fn identity_texture_and_depth_zero_preserve_old_pixels_and_budget_failure_is_atomic() {
    let mut plain = brush(TextureSpace::Document);
    plain.texture = Default::default();
    let mut reference = Document::new(96, 96).unwrap();
    line(&mut reference, plain.clone(), false);
    for (depth, invert, byte) in [(1., false, 255), (1., true, 0), (0., false, 0)] {
        let mut d = Document::new(96, 96).unwrap();
        let mut b = plain.clone();
        b.texture = BrushTexture::new(
            true,
            TextureSettings::new(TextureSpace::Document, 1., 0., depth, invert, 0., 0.).unwrap(),
            Some(Arc::new(TextureResource::new(Arc::new(
                BrushMask::new(1, 1, vec![byte]).unwrap(),
            )))),
        )
        .unwrap();
        line(&mut d, b, true);
        assert_eq!(pixels(&d), pixels(&reference));
    }
    let mut d = Document::new(96, 96).unwrap();
    let mut b = brush(TextureSpace::Document);
    b.brush.radius = 256.;
    b.brush.spacing = 1.;
    d.begin_brush_stroke(b, InputPoint::new(16.5, 48.5, 1.))
        .unwrap();
    let before = pixels(&d);
    assert!(matches!(
        d.stroke_to(InputPoint::new(1500., 48.5, 1.)),
        Err(Error::ResourceLimit(_))
    ));
    assert_eq!(pixels(&d), before);
    assert!(d.stroke_active());
    d.cancel_stroke().unwrap();
    assert!(pixels(&d).iter().all(|p| *p == Pixel::TRANSPARENT));
}

#[test]
fn tip_texture_follows_tip_angle_and_color_paper_caps_compose() {
    let mut frames = Vec::new();
    for angle in [0., 90.] {
        let mut d = Document::new(96, 96).unwrap();
        let mut b = brush(TextureSpace::Tip);
        b.tip.angle = angle;
        b.texture = BrushTexture::new(
            true,
            TextureSettings::new(TextureSpace::Tip, 1., 0., 1., false, 0.5, 0.5).unwrap(),
            Some(resource()),
        )
        .unwrap();
        d.begin_brush_stroke(b, InputPoint::new(48.5, 48.5, 1.))
            .unwrap();
        d.end_stroke().unwrap();
        frames.push(pixels(&d));
    }
    assert_ne!(frames[0], frames[1]);
    let mut d = Document::new(96, 96).unwrap();
    let mut b = brush(TextureSpace::Document);
    b.brush.color = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
    b.brush.background_color = Pixel::from_straight([0., 0., 1., 1.]).unwrap();
    b.dynamics.color = ColorDynamics::new(true, 1., 0., 0., 0., true, 42).unwrap();
    b.texture = BrushTexture::new(
        true,
        TextureSettings::default(),
        Some(Arc::new(TextureResource::new(Arc::new(
            BrushMask::new(1, 1, vec![128]).unwrap(),
        )))),
    )
    .unwrap();
    line(&mut d, b, false);
    let c = d.pixel(48, 48).components();
    assert!(c[0] > 0. && c[2] > 0.);
    assert!((c[3] - 0.4 * 128. / 255.).abs() < 1e-6);
}
