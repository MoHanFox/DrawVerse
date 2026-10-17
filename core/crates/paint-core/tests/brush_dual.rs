use paint_core::*;
use std::sync::Arc;
fn tip() -> BrushTip {
    BrushTip {
        pressure_size: false,
        pressure_flow: false,
        ..Default::default()
    }
}
fn secondary(radius: f32, mask: Option<Arc<BrushMask>>, dynamics: BrushDynamics) -> SecondaryTip {
    SecondaryTip::new(radius, tip(), mask, dynamics).unwrap()
}
fn brush() -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 8.,
            opacity: 0.4,
            spacing: 0.02,
            ..Default::default()
        },
        tip: tip(),
        dual: DualBrush::new(
            true,
            DualMode::Multiply,
            secondary(4., None, Default::default()),
        ),
        ..Default::default()
    }
}
fn pixels(doc: &Document) -> Vec<Pixel> {
    (0..64)
        .flat_map(|y| (0..64).map(move |x| doc.pixel(x, y)))
        .collect()
}
fn line(doc: &mut Document, b: BrushDefinition, split: bool) {
    doc.begin_brush_stroke(b, InputPoint::new(16.5, 32.5, 1.))
        .unwrap();
    if split {
        doc.stroke_to(InputPoint::new(32.5, 32.5, 1.)).unwrap();
    }
    doc.stroke_to(InputPoint::new(48.5, 32.5, 1.)).unwrap();
    doc.end_stroke().unwrap();
}
fn dab(doc: &mut Document, b: BrushDefinition, p: InputPoint) {
    doc.begin_brush_stroke(b, p).unwrap();
    doc.end_stroke().unwrap();
}
#[test]
fn six_modes_clip_primary_and_real_secondary_bounds_and_opacity_history_are_exact() {
    for (mode, expected) in [
        (DualMode::Multiply, 0.1),
        (DualMode::Darken, 0.2),
        (DualMode::Lighten, 0.5),
        (DualMode::Subtract, 0.4),
        (DualMode::Difference, 0.3),
        (DualMode::Screen, 0.6),
    ] {
        assert!((mode.combine(0.5, 0.2) - expected).abs() < 1e-6);
        assert_eq!(mode.combine(0., 1.), 0.);
        let mut d = Document::new(64, 64).unwrap();
        let mut b = brush();
        b.dual = DualBrush::new(true, mode, secondary(20., None, Default::default()));
        dab(&mut d, b, InputPoint::new(32.5, 32.5, 1.));
        assert_eq!(d.pixel(45, 32), Pixel::TRANSPARENT);
    }
    let mut d = Document::new(64, 64).unwrap();
    line(&mut d, brush(), false);
    assert!((d.pixel(32, 32).alpha() - 0.4).abs() < 1e-6);
    assert_eq!(d.pixel(32, 38), Pixel::TRANSPARENT);
    assert!(pixels(&d).iter().all(|p| p.alpha() <= 0.400001));
    let first = pixels(&d);
    d.undo().unwrap();
    assert!(pixels(&d).iter().all(|p| *p == Pixel::TRANSPARENT));
    d.redo().unwrap();
    assert_eq!(pixels(&d), first);
    line(&mut d, brush(), false);
    assert!((d.pixel(32, 32).alpha() - 0.64).abs() < 1e-6);
}
#[test]
fn independent_pressure_device_geometry_and_second_flow_are_executed() {
    let mut stationary = brush();
    let mut pressure_tip = tip();
    pressure_tip.pressure_size = true;
    stationary.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(8., pressure_tip, None, Default::default()).unwrap(),
    );
    let mut stationary_doc = Document::new(64, 64).unwrap();
    stationary_doc
        .begin_brush_stroke(stationary, InputPoint::new(32.5, 32.5, 0.25))
        .unwrap();
    assert_eq!(stationary_doc.pixel(37, 32), Pixel::TRANSPARENT);
    stationary_doc
        .stroke_to(InputPoint::new(32.5, 32.5, 0.75))
        .unwrap();
    assert!(stationary_doc.pixel(37, 32).alpha() > 0.3);
    stationary_doc.end_stroke().unwrap();
    let mut b = brush();
    let mut second = tip();
    second.pressure_size = true;
    second.pressure_flow = true;
    let dynamics = BrushDynamics {
        size: PressureControl::new(PressureCurve::new(vec![(0., 0.5), (1., 0.5)]).unwrap(), 0.)
            .unwrap(),
        flow: PressureControl::new(PressureCurve::new(vec![(0., 0.5), (1., 0.5)]).unwrap(), 0.)
            .unwrap(),
        ..Default::default()
    };
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(8., second, None, dynamics).unwrap(),
    );
    let mut d = Document::new(64, 64).unwrap();
    d.begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 0.2))
        .unwrap();
    d.stroke_to(InputPoint::new(32.5, 32.5, 0.8)).unwrap();
    d.end_stroke().unwrap();
    assert!((d.pixel(32, 32).alpha() - 0.2).abs() < 1e-6);
    assert_eq!(d.pixel(32, 38), Pixel::TRANSPARENT);
    let mut b = brush();
    let mut second = tip();
    second.roundness = 0.2;
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(
            8.,
            second,
            None,
            BrushDynamics {
                tip: TipDynamics::new(AngleControl::PenRotation, false, 0.01).unwrap(),
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let mut p = InputPoint::new(32.5, 32.5, 1.);
    p.capabilities = 4;
    p.rotation = 90.;
    let mut d = Document::new(64, 64).unwrap();
    dab(&mut d, b, p);
    assert!(d.pixel(32, 38).alpha() > 0.3);
    assert_eq!(d.pixel(38, 32), Pixel::TRANSPARENT);
    assert!(SecondaryTip::new(f32::NAN, tip(), None, Default::default()).is_err());
    assert!(SecondaryTip::new(
        8.,
        tip(),
        None,
        BrushDynamics {
            pressure_opacity: true,
            ..Default::default()
        }
    )
    .is_err());
    assert!(SecondaryTip::new(
        8.,
        tip(),
        None,
        BrushDynamics {
            color: ColorDynamics::new(true, 1., 0., 0., 0., true, 0).unwrap(),
            ..Default::default()
        }
    )
    .is_err());
}

#[test]
fn secondary_scatter_pressure_control_changes_real_stamp_count() {
    let mut base = brush();
    base.brush.radius = 4.;
    base.brush.opacity = 1.;
    base.tip.flow = 1.;
    let scatter = ScatterDynamics::new(true, 0., false, 4, 0., 7)
        .unwrap()
        .with_controls(
            ScatterControl::off(),
            ScatterControl::new(ScatterControlSource::Pressure, 0., 0).unwrap(),
        )
        .unwrap();
    base.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(
            4.,
            BrushTip { flow: 0.2, ..tip() },
            None,
            BrushDynamics {
                scatter,
                ..Default::default()
            },
        )
        .unwrap(),
    );
    let mut low = Document::new(64, 64).unwrap();
    dab(&mut low, base.clone(), InputPoint::new(32.5, 32.5, 0.));
    let mut high = Document::new(64, 64).unwrap();
    dab(&mut high, base, InputPoint::new(32.5, 32.5, 1.));
    assert!(high.pixel(32, 32).alpha() > low.pixel(32, 32).alpha());
}
#[test]
fn secondary_images_keep_holes_rotation_and_primary_boundary() {
    let mut bytes = vec![255; 256];
    for y in 6..10 {
        for x in 6..10 {
            bytes[y * 16 + x] = 0;
        }
    }
    let mask = Arc::new(BrushMask::new(16, 16, bytes).unwrap());
    let mut b = brush();
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        secondary(8., Some(mask), Default::default()),
    );
    let mut d = Document::new(64, 64).unwrap();
    dab(&mut d, b, InputPoint::new(32.5, 32.5, 1.));
    assert_eq!(d.pixel(32, 32), Pixel::TRANSPARENT);
    assert!(d.pixel(37, 32).alpha() > 0.3);
    let mask = Arc::new(BrushMask::new(8, 2, vec![255; 16]).unwrap());
    let mut second = tip();
    second.angle = 90.;
    let mut b = brush();
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        SecondaryTip::new(8., second, Some(mask), Default::default()).unwrap(),
    );
    let mut d = Document::new(64, 64).unwrap();
    dab(&mut d, b, InputPoint::new(32.5, 32.5, 1.));
    assert!(d.pixel(32, 38).alpha() > 0.3);
    assert_eq!(d.pixel(38, 32), Pixel::TRANSPARENT);
}
#[test]
fn nested_scatter_shape_jitter_is_chunk_invariant_and_disabled_keeps_old_pixels() {
    let mut b = brush();
    b.dynamics.scatter = ScatterDynamics::new(true, 0.2, true, 2, 0.5, 19).unwrap();
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        secondary(
            4.,
            None,
            BrushDynamics {
                scatter: ScatterDynamics::new(true, 0.7, true, 4, 0.5, 19).unwrap(),
                shape_jitter: ShapeJitter::new(true, 0.5, 0.2, 0.5, 0.5, 0.2, 19).unwrap(),
                ..Default::default()
            },
        ),
    );
    let mut a = Document::new(64, 64).unwrap();
    let mut c = Document::new(64, 64).unwrap();
    line(&mut a, b.clone(), false);
    line(&mut c, b.clone(), true);
    assert_eq!(pixels(&a), pixels(&c));
    assert!(pixels(&a).iter().any(|p| p.alpha() > 0.));
    let mut changed = b.clone();
    changed.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        secondary(
            4.,
            None,
            BrushDynamics {
                scatter: ScatterDynamics::new(true, 0.7, true, 4, 0.5, 20).unwrap(),
                ..Default::default()
            },
        ),
    );
    let mut other = Document::new(64, 64).unwrap();
    line(&mut other, changed, false);
    assert_ne!(pixels(&a), pixels(&other));
    b.dual = DualBrush::new(false, DualMode::Subtract, b.dual.secondary().clone());
    let mut plain = b.clone();
    plain.dual = Default::default();
    let mut a = Document::new(64, 64).unwrap();
    let mut c = Document::new(64, 64).unwrap();
    line(&mut a, b, false);
    line(&mut c, plain, true);
    assert_eq!(pixels(&a), pixels(&c));
}
#[test]
fn selection_erase_masks_smudge_and_layer_offsets_share_secondary_coverage() {
    for mask in [false, true] {
        let mut d = Document::new(64, 64).unwrap();
        d.initialize_white_background().unwrap();
        if mask {
            d.add_mask(1).unwrap();
        }
        d.set_selection(
            Selection::default()
                .apply(
                    SelectionShape::geometry(SelectionKind::Rectangle, 32., 0., 32., 64., false),
                    SelectionOperation::Replace,
                    64,
                    64,
                )
                .unwrap(),
        )
        .unwrap();
        let mut b = brush();
        b.brush.mode = if mask {
            BrushMode::Paint
        } else {
            BrushMode::Erase
        };
        dab(&mut d, b, InputPoint::new(32.5, 32.5, 1.));
        assert_eq!(d.pixel(31, 32), Pixel::WHITE);
        assert!((d.pixel(33, 32).alpha() - 0.6).abs() < 1e-6);
        assert_eq!(d.pixel(38, 32), Pixel::WHITE);
    }
    let mut d = Document::new(64, 64).unwrap();
    let mut b = brush();
    b.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: true,
        pressure_strength: false,
    };
    b.brush.color = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
    d.begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.))
        .unwrap();
    assert!(d.pixel(32, 32).components()[0] > 0.3);
    assert_eq!(d.pixel(38, 32), Pixel::TRANSPARENT);
    d.cancel_stroke().unwrap();
    assert!(pixels(&d).iter().all(|p| *p == Pixel::TRANSPARENT));
    let mut d = Document::new(64, 64).unwrap();
    let mut a = d.layer(1).unwrap().appearance();
    a.offset_x = 3;
    d.set_layer_appearance(1, a).unwrap();
    dab(&mut d, brush(), InputPoint::new(32.5, 32.5, 1.));
    assert!((d.pixel(32, 32).alpha() - 0.4).abs() < 1e-6);
    assert_eq!(d.pixel(38, 32), Pixel::TRANSPARENT);
}
#[test]
fn shared_resources_closed_budget_and_inflight_capture_are_exact() {
    let mask = Arc::new(BrushMask::new(4, 1, vec![0, 255, 0, 255]).unwrap());
    let resource = Arc::new(TextureResource::new(mask.clone()));
    let budget = mask.resident_bytes() + resource.mip_bytes();
    let settings = BrushSettings {
        mask: Some(mask.clone()),
        texture: BrushTexture::new(false, Default::default(), Some(resource)).unwrap(),
        dual: DualBrush::new(
            false,
            DualMode::Multiply,
            secondary(8., Some(mask.clone()), Default::default()),
        ),
        ..Default::default()
    };
    let mut catalog = BrushCatalog::with_mask_budget(budget).unwrap();
    catalog.edit("round-pressure", settings.clone()).unwrap();
    catalog
        .save_copy("12345678-1234-4234-8234-123456789abc".into(), "copy".into())
        .unwrap();
    let revision = catalog.revision();
    let mut other = settings;
    other.dual = DualBrush::new(
        false,
        DualMode::Multiply,
        secondary(
            8.,
            Some(Arc::new(BrushMask::new(4, 1, vec![255; 4]).unwrap())),
            Default::default(),
        ),
    );
    assert!(matches!(
        catalog.edit("round-fine", other),
        Err(Error::ResourceLimit(_))
    ));
    assert_eq!(catalog.revision(), revision);
    let weak = Arc::downgrade(&mask);
    let mut b = brush();
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        secondary(8., Some(mask.clone()), Default::default()),
    );
    let mut d = Document::new(64, 64).unwrap();
    d.begin_brush_stroke(b, InputPoint::new(16.5, 32.5, 1.))
        .unwrap();
    drop(catalog);
    drop(mask);
    assert!(weak.upgrade().is_some());
    d.stroke_to(InputPoint::new(48.5, 32.5, 1.)).unwrap();
    d.end_stroke().unwrap();
    assert!(pixels(&d).iter().any(|p| p.alpha() > 0.));
    assert!(weak.upgrade().is_none());
}
#[test]
fn expanded_work_precheck_is_atomic_and_failed_retry_keeps_random_sequence() {
    let mut huge = brush();
    huge.brush.radius = 256.;
    huge.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        secondary(
            256.,
            Some(Arc::new(BrushMask::new(1, 1, vec![255]).unwrap())),
            BrushDynamics {
                scatter: ScatterDynamics::new(true, 1., true, 16, 0., 0).unwrap(),
                ..Default::default()
            },
        ),
    );
    let mut a = Document::new(64, 64).unwrap();
    assert!(matches!(
        a.begin_brush_stroke(huge, InputPoint::new(32.5, 32.5, 1.)),
        Err(Error::ResourceLimit(_))
    ));
    assert!(!a.stroke_active());
    assert!(pixels(&a).iter().all(|p| *p == Pixel::TRANSPARENT));
    let mut b = brush();
    b.dual = DualBrush::new(
        true,
        DualMode::Multiply,
        secondary(
            4.,
            None,
            BrushDynamics {
                scatter: ScatterDynamics::new(true, 0.5, true, 4, 0.5, 17).unwrap(),
                ..Default::default()
            },
        ),
    );
    let mut reference = Document::new(64, 64).unwrap();
    for d in [&mut a, &mut reference] {
        d.begin_brush_stroke(b.clone(), InputPoint::new(16.5, 32.5, 1.))
            .unwrap();
    }
    let before = pixels(&a);
    assert!(matches!(
        a.stroke_to(InputPoint::new(10000., 32.5, 1.)),
        Err(Error::ResourceLimit(_))
    ));
    assert_eq!(pixels(&a), before);
    for d in [&mut a, &mut reference] {
        d.stroke_to(InputPoint::new(48.5, 32.5, 1.)).unwrap();
        d.end_stroke().unwrap();
    }
    assert_eq!(pixels(&a), pixels(&reference));
}
#[test]
fn dual_composes_with_paper_color_caps_locks_and_real_smudge_transport() {
    let paper = Arc::new(TextureResource::new(Arc::new(
        BrushMask::new(1, 1, vec![128]).unwrap(),
    )));
    let mut b = brush();
    b.texture = BrushTexture::new(true, Default::default(), Some(paper)).unwrap();
    b.brush.color = Pixel::from_straight([1., 0., 0., 1.]).unwrap();
    b.brush.background_color = Pixel::from_straight([0., 0., 1., 1.]).unwrap();
    b.dynamics.color = ColorDynamics::new(true, 1., 0., 0., 0., true, 9).unwrap();
    let mut d = Document::new(64, 64).unwrap();
    line(&mut d, b.clone(), false);
    assert!((d.pixel(32, 32).alpha() - 0.4 * 128. / 255.).abs() < 1e-6);
    assert!(d.pixel(32, 32).components()[0] > 0.);
    assert!(d.pixel(32, 32).components()[2] > 0.);
    let mut locked = Document::new(64, 64).unwrap();
    let mut appearance = locked.layer(1).unwrap().appearance();
    appearance.locks = LOCK_TRANSPARENCY;
    locked.set_layer_appearance(1, appearance).unwrap();
    line(&mut locked, b, false);
    assert!(pixels(&locked).iter().all(|p| *p == Pixel::TRANSPARENT));
    let mut moved = Document::new(64, 64).unwrap();
    moved
        .begin_stroke(
            Brush {
                radius: 10.,
                color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
                ..Default::default()
            },
            InputPoint::new(16.5, 32.5, 1.),
        )
        .unwrap();
    moved.end_stroke().unwrap();
    let before = pixels(&moved);
    let mut b = brush();
    b.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: false,
        pressure_strength: false,
    };
    line(&mut moved, b, false);
    let after = pixels(&moved);
    assert!(before
        .iter()
        .zip(&after)
        .any(|(a, b)| a.alpha() == 0. && b.components()[0] > 0.0001));
    moved.undo().unwrap();
    assert_eq!(pixels(&moved), before);
    moved.redo().unwrap();
    assert_eq!(pixels(&moved), after);
}

#[test]
fn default_closed_path_and_explicit_paired_are_distinct() {
    let default = DualBrush::default();
    assert!(default.is_default());
    assert!(!default.enabled());
    assert_eq!(default.secondary().spacing(), Some(0.25));
    let paired = DualBrush::new(false, DualMode::Multiply, SecondaryTip::default());
    assert!(!paired.is_default());
    assert_eq!(paired.secondary().spacing(), None);
}
