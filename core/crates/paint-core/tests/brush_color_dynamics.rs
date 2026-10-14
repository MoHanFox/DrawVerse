use paint_core::*;
use std::{collections::BTreeSet, sync::Arc};
fn brush() -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 4.,
            spacing: 0.2,
            opacity: 0.25,
            color: Pixel::from_straight([1., 0., 0., 1.]).unwrap(),
            background_color: Pixel::from_straight([0., 0., 1., 1.]).unwrap(),
            ..Default::default()
        },
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            flow: 0.2,
            ..Default::default()
        },
        dynamics: BrushDynamics {
            color: ColorDynamics::new(true, 1., 0., 0., 0., true, 42).unwrap(),
            ..Default::default()
        },
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
fn two_colors_accumulate_under_opacity_and_chunking_history_are_exact() {
    let mut a = Document::new(96, 96).unwrap();
    let mut b = Document::new(96, 96).unwrap();
    line(&mut a, brush(), false);
    line(&mut b, brush(), true);
    assert_eq!(pixels(&a), pixels(&b));
    let frame = pixels(&a);
    assert!(frame.iter().all(|p| p.alpha() <= 0.250001));
    let mut colors = BTreeSet::new();
    for p in &frame {
        let c = p.components();
        if c[3] > 0.01 {
            assert!(c[0] > 0. && c[2] > 0.);
            colors.insert(((c[0] / c[3] * 100.) as u32, (c[2] / c[3] * 100.) as u32));
        }
    }
    assert!(colors.len() > 5);
    a.undo().unwrap();
    assert!(pixels(&a).iter().all(|p| *p == Pixel::TRANSPARENT));
    a.redo().unwrap();
    assert_eq!(pixels(&a), frame);
}
#[test]
fn per_stroke_color_is_constant_but_per_tip_and_channels_change_pixels() {
    let mut b = brush();
    b.dynamics.color = ColorDynamics::new(true, 1., 0., 0., 0., false, 42).unwrap();
    let mut d = Document::new(96, 96).unwrap();
    line(&mut d, b.clone(), false);
    let mut tint: Option<f32> = None;
    for p in pixels(&d) {
        let c = p.components();
        if c[3] > 0.01 {
            let ratio = c[0] / c[3];
            if let Some(before) = tint {
                assert!((ratio - before).abs() < 1e-5);
            } else {
                tint = Some(ratio);
            }
        }
    }
    let first = pixels(&d);
    line(&mut d, b, false);
    assert_ne!(pixels(&d), first);
    for (h, s, v) in [(1., 0., 0.), (0., 1., 0.), (0., 0., 1.)] {
        let mut b = brush();
        b.brush.color = Pixel::from_straight([0.8, 0.3, 0.1, 1.]).unwrap();
        b.dynamics.color = ColorDynamics::new(true, 0., h, s, v, true, 42).unwrap();
        let mut dynamic = Document::new(96, 96).unwrap();
        line(&mut dynamic, b.clone(), false);
        b.dynamics.color = Default::default();
        let mut plain = Document::new(96, 96).unwrap();
        line(&mut plain, b, false);
        assert_ne!(pixels(&dynamic), pixels(&plain));
    }
}
#[test]
fn disabled_and_zero_dynamics_preserve_legacy_pixels() {
    let mut b = brush();
    b.dynamics.color = Default::default();
    let mut normal = Document::new(96, 96).unwrap();
    line(&mut normal, b.clone(), false);
    for cfg in [
        ColorDynamics::new(false, 1., 1., 1., 1., true, 42).unwrap(),
        ColorDynamics::new(true, 0., 0., 0., 0., true, 42).unwrap(),
    ] {
        let mut d = Document::new(96, 96).unwrap();
        b.dynamics.color = cfg;
        line(&mut d, b.clone(), true);
        assert_eq!(pixels(&d), pixels(&normal));
    }
}
#[test]
fn fully_covered_stationary_color_freezes_and_pressure_adds_only_new_contributions() {
    let mut d = Document::new(96, 96).unwrap();
    let mut b = brush();
    b.tip.flow = 1.;
    b.dynamics.pressure_opacity = true;
    d.begin_brush_stroke(b, InputPoint::new(48.5, 48.5, 0.5))
        .unwrap();
    let first = d.pixel(48, 48);
    for _ in 0..20 {
        d.stroke_to(InputPoint::new(48.5, 48.5, 0.5)).unwrap();
    }
    assert_eq!(d.pixel(48, 48), first);
    d.stroke_to(InputPoint::new(48.5, 48.5, 1.)).unwrap();
    assert!((d.pixel(48, 48).alpha() - 0.25).abs() < 1e-6);
    let high = d.pixel(48, 48);
    d.stroke_to(InputPoint::new(48.5, 48.5, 0.1)).unwrap();
    assert_eq!(d.pixel(48, 48), high);
    d.cancel_stroke().unwrap();
    assert_eq!(d.pixel(48, 48), Pixel::TRANSPARENT);
}
#[test]
fn images_selection_mask_and_erase_keep_caps() {
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
        let mut b = brush();
        b.mask = Some(Arc::new(BrushMask::new(3, 1, vec![255, 0, 255]).unwrap()));
        if !mask {
            b.brush.mode = BrushMode::Erase;
        }
        line(&mut d, b, true);
        for y in 0..96 {
            for x in 0..96 {
                assert!(d.pixel(x, y).alpha() >= 0.749999);
                if x < 48 {
                    assert_eq!(d.pixel(x, y), Pixel::WHITE);
                }
            }
        }
        assert!((48..96).any(|x| d.pixel(x, 48).alpha() < 1.));
    }
}
#[test]
fn finger_smudge_uses_dynamic_load_and_cancel_restores() {
    let mut d = Document::new(96, 96).unwrap();
    let mut b = brush();
    b.brush.opacity = 1.;
    b.tip.flow = 1.;
    b.brush.mode = BrushMode::Smudge {
        all_layers: false,
        finger_painting: true,
        pressure_strength: false,
    };
    d.begin_brush_stroke(b, InputPoint::new(16.5, 48.5, 1.))
        .unwrap();
    let c = d.pixel(16, 48).components();
    assert!(c[0] > 0. && c[2] > 0.);
    d.stroke_to(InputPoint::new(32.5, 48.5, 1.)).unwrap();
    assert!((24..36).any(|x| d.pixel(x, 48).alpha() > 0.));
    d.cancel_stroke().unwrap();
    assert!(pixels(&d).iter().all(|p| *p == Pixel::TRANSPARENT));
}

#[test]
fn color_moments_page_and_partial_foreground_alpha_remains_capped() {
    let mut normal = Document::new(96, 96).unwrap();
    let mut paged = Document::with_options(
        96,
        96,
        DocumentOptions {
            max_resident_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let mut b = brush();
    b.brush.color = Pixel::from_straight([1., 0., 0., 0.5]).unwrap();
    b.brush.background_color = Pixel::from_straight([0., 0., 1., 0.8]).unwrap();
    line(&mut normal, b.clone(), false);
    line(&mut paged, b, true);
    assert_eq!(pixels(&normal), pixels(&paged));
    let frame = pixels(&paged);
    assert!(frame.iter().any(|p| p.alpha() > 0.));
    assert!(frame.iter().all(|p| p.alpha() <= 0.125001));
    paged.undo().unwrap();
    assert!(pixels(&paged).iter().all(|p| *p == Pixel::TRANSPARENT));
    paged.redo().unwrap();
    assert_eq!(pixels(&paged), frame);
}
#[test]
fn transparency_lock_and_failed_segment_preserve_real_state() {
    let mut d = Document::new(96, 96).unwrap();
    let mut base = brush();
    base.dynamics.color = Default::default();
    base.brush.opacity = 1.;
    base.tip.flow = 1.;
    base.brush.radius = 12.;
    base.brush.color = Pixel::from_straight([0.2, 0.2, 0.2, 0.4]).unwrap();
    d.begin_brush_stroke(base, InputPoint::new(48.5, 48.5, 1.))
        .unwrap();
    d.end_stroke().unwrap();
    let alpha = d.pixel(48, 48).alpha();
    d.set_layer_appearance(
        1,
        LayerAppearance {
            locks: LOCK_TRANSPARENCY,
            ..Default::default()
        },
    )
    .unwrap();
    d.begin_brush_stroke(brush(), InputPoint::new(48.5, 48.5, 1.))
        .unwrap();
    assert_eq!(d.pixel(48, 48).alpha(), alpha);
    d.cancel_stroke().unwrap();
    let mut retry = Document::new(96, 96).unwrap();
    let mut reference = Document::new(96, 96).unwrap();
    for d in [&mut retry, &mut reference] {
        d.begin_brush_stroke(brush(), InputPoint::new(16.5, 48.5, 1.))
            .unwrap();
    }
    let before = pixels(&retry);
    assert!(matches!(
        retry.stroke_to(InputPoint::new(10000., 48.5, 1.)),
        Err(Error::ResourceLimit(_))
    ));
    assert_eq!(pixels(&retry), before);
    for d in [&mut retry, &mut reference] {
        d.stroke_to(InputPoint::new(24.5, 48.5, 1.)).unwrap();
        d.end_stroke().unwrap();
    }
    assert_eq!(pixels(&retry), pixels(&reference));
}

#[test]
fn image_holes_keep_transparency_with_dynamic_tint() {
    let mut d = Document::new(96, 96).unwrap();
    let mut b = brush();
    b.brush.radius = 8.;
    b.tip.flow = 1.;
    let mut coverage = vec![255; 256];
    for y in 6..10 {
        for x in 6..10 {
            coverage[y * 16 + x] = 0;
        }
    }
    b.mask = Some(Arc::new(BrushMask::new(16, 16, coverage).unwrap()));
    d.begin_brush_stroke(b, InputPoint::new(48.5, 48.5, 1.))
        .unwrap();
    d.end_stroke().unwrap();
    assert_eq!(d.pixel(48, 48), Pixel::TRANSPARENT);
    let c = d.pixel(54, 48).components();
    assert!(c[0] > 0. && c[2] > 0.);
    assert!((c[3] - 0.25).abs() < 1e-6);
}
