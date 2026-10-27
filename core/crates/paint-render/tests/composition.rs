use paint_core::*;
use paint_render::{composition::*, *};
use paint_task::CancellationToken;

fn request() -> RenderRequest {
    RenderRequest {
        region: Region {
            x: 0.,
            y: 0.,
            width: 130.,
            height: 90.,
        },
        width: 130,
        height: 90,
        format: PixelFormat::LinearRgba32F,
    }
}
fn dab(doc: &mut Document, point: (f64, f64), color: [f32; 4]) {
    doc.begin_stroke(
        Brush {
            radius: 23.,
            color: Pixel::from_straight(color).unwrap(),
            ..Default::default()
        },
        InputPoint::new(point.0, point.1, 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
}
fn plan(doc: &Document, ids: &mut ContentIdentities) -> Plan {
    prepare(
        doc.dimensions(),
        doc.revision(),
        doc.layers(),
        request(),
        Limits::default(),
        ids,
        &CancellationToken::default(),
    )
    .unwrap()
}

#[test]
fn nested_isolated_and_through_groups_masks_match_authoritative_core() {
    let mut doc = Document::new(130, 90).unwrap();
    dab(&mut doc, (48., 40.), [0.2, 0.6, 0.4, 0.7]);
    let bottom = doc.add_layer("inside").unwrap();
    dab(&mut doc, (53., 42.), [0.8, 0.1, 0.4, 0.8]);
    let inner = doc.group_layer(bottom, "inner").unwrap();
    let top = doc.add_layer("top").unwrap();
    dab(&mut doc, (42., 36.), [0.1, 0.3, 0.9, 0.65]);
    let outer = doc.group_layer(inner, "outer").unwrap();
    let mask = doc.add_mask(outer).unwrap();
    dab(&mut doc, (47., 37.), [0., 0., 0., 0.7]);
    doc.set_layer_properties(
        mask,
        LayerProperties {
            visible: true,
            opacity: 0.53,
        },
    )
    .unwrap();
    doc.set_mask_linked(mask, false).unwrap();
    doc.add_mask(bottom).unwrap();
    dab(&mut doc, (58., 44.), [0., 0., 0., 0.5]);
    let mut identities = ContentIdentities::default();
    for mode in [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::SoftLight,
        BlendMode::Hue,
        BlendMode::PassThrough,
    ] {
        doc.set_layer_appearance(
            inner,
            LayerAppearance {
                blend: mode,
                fill: 0.71,
                ..Default::default()
            },
        )
        .unwrap();
        doc.set_layer_properties(
            inner,
            LayerProperties {
                visible: true,
                opacity: 0.63,
            },
        )
        .unwrap();
        for outside in [BlendMode::Normal, BlendMode::PassThrough] {
            doc.set_layer_appearance(
                outer,
                LayerAppearance {
                    blend: outside,
                    fill: 0.83,
                    ..Default::default()
                },
            )
            .unwrap();
            doc.set_layer_properties(
                outer,
                LayerProperties {
                    visible: true,
                    opacity: 0.61,
                },
            )
            .unwrap();
            doc.set_layer_appearance(
                top,
                LayerAppearance {
                    blend: BlendMode::Screen,
                    ..Default::default()
                },
            )
            .unwrap();
            let p = plan(&doc, &mut identities);
            assert_eq!(p.surface_count, 6);
            assert!(p.required_features & GROUPS != 0);
            assert!(p.operations.len() <= p.nodes.len() * 4 + 1);
            let token = CancellationToken::default();
            for y in 0..90 {
                for x in 0..130 {
                    let actual = p.try_pixel(x, y, &token).unwrap().components();
                    let expected = doc.snapshot().try_pixel(x, y).unwrap().components();
                    for c in 0..4 {
                        assert!(
                            (actual[c] - expected[c]).abs() < 2e-6,
                            "{mode:?}/{outside:?} at {x},{y}: {actual:?} != {expected:?}"
                        );
                    }
                }
            }
        }
    }
    doc.set_layer_properties(
        outer,
        LayerProperties {
            visible: false,
            opacity: 1.,
        },
    )
    .unwrap();
    let p = plan(&doc, &mut identities);
    assert_eq!(p.tiles.len(), doc.layer(1).unwrap().tile_count());
    assert_eq!(p.surface_count, 2);
    assert!(p
        .nodes
        .iter()
        .filter(|n| n.layer_id != 1)
        .all(|n| !n.visible && n.tile_count == 0));
    let old = Limits {
        features: FEATURES & !GROUPS,
        ..Default::default()
    };
    assert!(matches!(
        estimate(doc.layers(), old),
        Err(PlanError::Unsupported(_))
    ));
}

#[test]
fn full_normal_mask_offsets_implicit_white_and_explicit_transparency_match_core() {
    let mut doc = Document::new(130, 90).unwrap();
    doc.initialize_white_background().unwrap();
    let mut ids = ContentIdentities::default();
    let token = CancellationToken::default();
    // Erase an explicit tile in the implicit white layer.
    doc.begin_stroke(
        Brush {
            radius: 20.,
            mode: BrushMode::Erase,
            ..Default::default()
        },
        InputPoint::new(12., 20., 1.),
    )
    .unwrap();
    doc.end_stroke().unwrap();
    let top = doc.add_layer("normal").unwrap();
    dab(&mut doc, (67., 42.), [0.8, 0.2, 0.4, 0.7]);
    doc.set_layer_properties(
        top,
        LayerProperties {
            visible: true,
            opacity: 0.6,
        },
    )
    .unwrap();
    doc.set_layer_appearance(
        top,
        LayerAppearance {
            fill: 0.7,
            offset_x: -7,
            offset_y: 3,
            ..Default::default()
        },
    )
    .unwrap();
    let mask = doc.add_mask(top).unwrap();
    dab(&mut doc, (56., 45.), [0., 0., 0., 1.]);
    for (visible, opacity, fill) in [(true, 0.45, 1.), (false, 1., 1.), (true, 1., 0.)] {
        doc.set_layer_properties(mask, LayerProperties { visible, opacity })
            .unwrap();
        let mut appearance = doc
            .layers()
            .iter()
            .find(|l| l.id() == mask)
            .unwrap()
            .appearance();
        appearance.fill = fill;
        doc.set_layer_appearance(mask, appearance).unwrap();
        let p = plan(&doc, &mut ids);
        for y in 0..90 {
            for x in 0..130 {
                assert_eq!(
                    p.try_pixel(x, y, &token).unwrap(),
                    doc.pixel(x, y),
                    "{x},{y}"
                );
            }
        }
        assert_eq!(p.try_pixel(130, 0, &token).unwrap(), Pixel::TRANSPARENT);
        assert!(p.pinned_bytes <= MAX_PIN_BYTES);
    }
}

#[test]
fn source_ids_survive_new_plans_but_never_reuse_changed_tile_content() {
    let mut doc = Document::new(130, 90).unwrap();
    dab(&mut doc, (24., 24.), [1., 0., 0., 1.]);
    let mut ids = ContentIdentities::default();
    let a = plan(&doc, &mut ids);
    let b = plan(&doc, &mut ids);
    assert_eq!(a.tiles[0].content_id, b.tiles[0].content_id);
    let before = a.try_pixel(24, 24, &CancellationToken::default()).unwrap();
    dab(&mut doc, (24., 24.), [0., 1., 0., 1.]);
    let c = plan(&doc, &mut ids);
    assert_ne!(a.tiles[0].content_id, c.tiles[0].content_id);
    assert_eq!(
        a.try_pixel(24, 24, &CancellationToken::default()).unwrap(),
        before
    );
    assert_ne!(
        c.try_pixel(24, 24, &CancellationToken::default()).unwrap(),
        before
    );
    let last = c.tiles[0].content_id;
    drop(a);
    drop(b);
    drop(c);
    // Old Weak identities also invalidate when no plan strongly retains pixels.
    dab(&mut doc, (24., 24.), [0., 0., 1., 1.]);
    assert_ne!(last, plan(&doc, &mut ids).tiles[0].content_id);
}

#[test]
fn capabilities_and_budgets_reject_entire_closure_without_partial_plan() {
    let mut doc = Document::new(130, 90).unwrap();
    let mut ids = ContentIdentities::default();
    dab(&mut doc, (24., 24.), [1.; 4]);
    let run = |doc: &Document, limits, ids: &mut ContentIdentities| {
        prepare(
            doc.dimensions(),
            doc.revision(),
            doc.layers(),
            request(),
            limits,
            ids,
            &CancellationToken::default(),
        )
    };
    for limits in [
        Limits {
            tiles: 0,
            ..Default::default()
        },
        Limits {
            pinned_bytes: 0,
            ..Default::default()
        },
        Limits {
            metadata_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            run(&doc, limits, &mut ids),
            Err(PlanError::Budget)
        ));
        assert!(ids.is_empty());
    }
    assert!(matches!(
        run(
            &doc,
            Limits {
                features: FEATURES | (1 << 63),
                ..Default::default()
            },
            &mut ids
        ),
        Err(PlanError::Unsupported(_))
    ));
    let mask = doc.add_mask(doc.active_layer()).unwrap();
    assert!(matches!(
        run(
            &doc,
            Limits {
                features: NORMAL | LINEAR_RGBA32F,
                ..Default::default()
            },
            &mut ids
        ),
        Err(PlanError::Unsupported(_))
    ));
    doc.set_mask_linked(mask, false).unwrap();
    assert!(matches!(
        run(
            &doc,
            Limits {
                features: NORMAL | SELF_MASK | LINEAR_RGBA32F,
                ..Default::default()
            },
            &mut ids
        ),
        Err(PlanError::Unsupported(_))
    ));
    doc.set_mask_linked(mask, true).unwrap();
    let top = doc.layers().iter().find(|l| !l.is_mask()).unwrap().id();
    doc.set_layer_appearance(
        top,
        LayerAppearance {
            blend: BlendMode::Multiply,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(
        run(
            &doc,
            Limits {
                features: NORMAL | SELF_MASK | LINEAR_RGBA32F,
                ..Default::default()
            },
            &mut ids
        ),
        Err(PlanError::Unsupported(_))
    ));
}

#[test]
fn tile_read_is_bounded_little_endian_and_cancel_never_fabricates_pixels() {
    let mut doc = Document::new(130, 90).unwrap();
    dab(&mut doc, (24., 24.), [0.2, 0.7, 0.4, 0.9]);
    let p = plan(&doc, &mut ContentIdentities::default());
    let token = CancellationToken::default();
    let bytes = p.tiles[0].read(&token).unwrap();
    assert_eq!(bytes.len(), TILE_BYTES);
    let at = (24 * 64 + 24) * 16;
    for c in 0..4 {
        assert_eq!(
            f32::from_le_bytes(bytes[at + c * 4..at + c * 4 + 4].try_into().unwrap()),
            doc.pixel(24, 24).components()[c]
        );
    }
    token.cancel();
    assert_eq!(p.tiles[0].read(&token).unwrap_err(), PlanError::Cancelled);
    assert!(matches!(
        prepare(
            doc.dimensions(),
            doc.revision(),
            doc.layers(),
            request(),
            Limits::default(),
            &mut ContentIdentities::default(),
            &token
        ),
        Err(PlanError::Cancelled)
    ));
}

#[test]
fn plan_composes_before_lod_instead_of_downsampling_sources_individually() {
    let mut doc = Document::new(130, 90).unwrap();
    doc.initialize_white_background().unwrap();
    let top = doc.add_layer("half transparent").unwrap();
    dab(&mut doc, (62., 42.), [1., 0., 0., 0.5]);
    let mask = doc.add_mask(top).unwrap();
    dab(&mut doc, (67., 45.), [0., 0., 0., 1.]);
    doc.set_layer_properties(
        mask,
        LayerProperties {
            visible: true,
            opacity: 0.7,
        },
    )
    .unwrap();
    let p = plan(&doc, &mut ContentIdentities::default());
    let cancel = CancellationToken::default();
    let r = RenderRequest {
        region: Region {
            x: 0.,
            y: 0.,
            width: 128.,
            height: 88.,
        },
        width: 32,
        height: 22,
        ..request()
    };
    let frame = CpuRenderer.render(&doc.snapshot(), r, &cancel).unwrap();
    let FramePixels::Linear(pixels) = frame.pixels else {
        panic!()
    };
    for y in 0..22 {
        for x in 0..32 {
            let mut sum = [0.; 4];
            for py in y * 4..y * 4 + 4 {
                for px in x * 4..x * 4 + 4 {
                    for (s, c) in sum
                        .iter_mut()
                        .zip(p.try_pixel(px, py, &cancel).unwrap().components())
                    {
                        *s += c / 16.;
                    }
                }
            }
            for (actual, expected) in pixels[(y * 32 + x) as usize].iter().zip(sum) {
                assert!((actual - expected).abs() < 2e-6);
            }
        }
    }
}

#[test]
fn capture_never_loads_cold_pages_and_worker_reads_keep_original_page_budget() {
    let mut doc = Document::with_options(
        130,
        90,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    for point in [(8., 8.), (65., 40.), (125., 80.)] {
        dab(&mut doc, point, [1., 0., 0., 0.8]);
    }
    doc.trim_storage().unwrap();
    let before = doc.storage_stats();
    assert!(before.registered_tiles > 1);
    let p = plan(&doc, &mut ContentIdentities::default());
    assert_eq!(doc.storage_stats().page_reads, before.page_reads);
    assert!(doc.storage_stats().resident_tiles <= 1);
    for source in &p.tiles {
        assert_eq!(
            source.read(&CancellationToken::default()).unwrap().len(),
            TILE_BYTES
        );
    }
    assert!(doc.storage_stats().page_reads > before.page_reads);
    doc.trim_storage().unwrap();
    assert!(doc.storage_stats().resident_tiles <= 1);
}

#[test]
fn root_blends_and_fixed_masks_match_authoritative_pixels_and_old_caps_reject() {
    let token = CancellationToken::default();
    for white in [false, true] {
        let mut doc = Document::new(130, 90).unwrap();
        if white {
            doc.initialize_white_background().unwrap();
        }
        dab(&mut doc, (58., 40.), [0.2, 0.7, 0.4, 0.7]);
        let top = doc.add_layer("blend").unwrap();
        dab(&mut doc, (66., 43.), [0.9, 0.1, 0.6, 0.8]);
        let mask = doc.add_mask(top).unwrap();
        dab(&mut doc, (62., 41.), [0., 0., 0., 0.8]);
        doc.set_layer_properties(
            mask,
            LayerProperties {
                visible: true,
                opacity: 0.63,
            },
        )
        .unwrap();
        doc.set_mask_linked(mask, false).unwrap();
        let mut ids = ContentIdentities::default();
        for id in (0..27).filter(|i| *i != 1) {
            doc.set_layer_appearance(
                top,
                LayerAppearance {
                    blend: BlendMode::from_id(id).unwrap(),
                    fill: 0.71,
                    offset_x: -7,
                    offset_y: 3,
                    ..Default::default()
                },
            )
            .unwrap();
            let p = plan(&doc, &mut ids);
            assert!(p.required_features & FIXED_MASK != 0);
            assert!(
                p.nodes
                    .iter()
                    .find(|n| n.layer_id == mask)
                    .unwrap()
                    .fixed_mask
            );
            for y in 0..90 {
                for x in 0..130 {
                    assert_eq!(
                        p.try_pixel(x, y, &token).unwrap(),
                        doc.pixel(x, y),
                        "mode={id}, white={white}, {x},{y}"
                    );
                }
            }
            assert!(matches!(
                prepare(
                    doc.dimensions(),
                    doc.revision(),
                    doc.layers(),
                    request(),
                    Limits {
                        features: 7,
                        ..Default::default()
                    },
                    &mut ids,
                    &token
                ),
                Err(PlanError::Unsupported(_))
            ));
        }
        doc.set_layer_appearance(
            top,
            LayerAppearance {
                blend: BlendMode::Dissolve,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(matches!(
            prepare(
                doc.dimensions(),
                doc.revision(),
                doc.layers(),
                request(),
                Limits::default(),
                &mut ids,
                &token
            ),
            Err(PlanError::Unsupported(_))
        ));
    }
}
