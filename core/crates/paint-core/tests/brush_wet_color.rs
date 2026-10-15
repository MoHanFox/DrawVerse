use paint_core::*;
use std::{collections::BTreeMap, sync::Arc};
fn rgba(c: [f32; 4]) -> Pixel {
    Pixel::from_straight(c).unwrap()
}
fn brush(per_tip: bool) -> BrushDefinition {
    BrushDefinition {
        brush: Brush {
            radius: 4.,
            spacing: 0.5,
            opacity: 0.4,
            color: rgba([1., 0., 0., 1.]),
            background_color: rgba([0., 0., 1., 0.7]),
            ..Default::default()
        },
        tip: BrushTip {
            pressure_size: false,
            pressure_flow: false,
            flow: 0.65,
            hardness: 0.7,
            ..Default::default()
        },
        dynamics: BrushDynamics {
            color: ColorDynamics::new(true, 0.5, 0.6, 0.2, 0.3, per_tip, u64::MAX).unwrap(),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn source(paged: bool) -> Document {
    let pixels = (0..4096)
        .map(|i| {
            if i / 64 < 32 {
                rgba([1., 0., 0., 0.7])
            } else {
                rgba([0., 0., 1., 0.7])
            }
        })
        .collect();
    Document::from_import(
        128,
        64,
        vec![ImportedLayer {
            name: "source".into(),
            properties: LayerProperties::default(),
            tiles: BTreeMap::from([(TileCoord { x: 0, y: 0 }, Tile::from_pixels(pixels).unwrap())]),
        }],
        0,
        DocumentOptions {
            max_resident_tiles: if paged { 1 } else { 4096 },
            ..Default::default()
        },
    )
    .unwrap()
}
fn pixels(d: &Document) -> Vec<Pixel> {
    (0..d.dimensions().1)
        .flat_map(|y| (0..d.dimensions().0).map(move |x| d.try_pixel(x, y).unwrap()))
        .collect()
}
fn close(a: &[Pixel], b: &[Pixel]) {
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        for (a, b) in a.components().into_iter().zip(b.components()) {
            assert!((a - b).abs() < 2e-6, "{a} != {b}");
        }
    }
}
fn dry(b: &BrushDefinition) -> WetCarrier {
    WetCarrier::new(
        WetMix::new(0., 0., 0., 1., 1., 0.).unwrap(),
        8.,
        b.brush.color,
    )
    .unwrap()
}
#[test]
fn one_dab_uses_same_color_transform_as_plain_brush_on_pixels_masks_and_alpha_lock() {
    for per_tip in [false, true] {
        for target in 0..3 {
            for mode in [
                BlendMode::Normal,
                BlendMode::Multiply,
                BlendMode::ColorDodge,
                BlendMode::Hue,
            ] {
                let mut wet = source(false);
                let mut plain = source(false);
                for d in [&mut wet, &mut plain] {
                    if target == 1 {
                        d.add_mask(1).unwrap();
                    } else if target == 2 {
                        d.set_layer_appearance(
                            1,
                            LayerAppearance {
                                locks: LOCK_TRANSPARENCY,
                                ..Default::default()
                            },
                        )
                        .unwrap();
                    }
                }
                let mut b = brush(per_tip);
                b.write_blend = mode;
                let c = dry(&b);
                let original = pixels(&wet);
                wet.begin_wet_brush_stroke(b.clone(), InputPoint::new(32.5, 32.5, 1.), &c, false)
                    .unwrap();
                let (changed, returned) = wet.end_wet_brush_stroke().unwrap();
                assert_eq!(returned.sample(0.25, 0.25), c.sample(0.25, 0.25));
                plain
                    .begin_brush_stroke(b, InputPoint::new(32.5, 32.5, 1.))
                    .unwrap();
                let reference_changed = plain.end_stroke().unwrap();
                assert_eq!(
                    changed, reference_changed,
                    "{mode:?} target {target} per_tip {per_tip}"
                );
                let result = pixels(&wet);
                close(&result, &pixels(&plain));
                if changed {
                    wet.undo().unwrap();
                    assert_eq!(pixels(&wet), original);
                    wet.redo().unwrap();
                    assert_eq!(pixels(&wet), result);
                } else {
                    assert_eq!(result, original);
                }
            }
        }
    }
}
#[test]
fn alpha_and_pressure_ceiling_survive_color_transform_without_tinting_the_carrier() {
    for alpha in [0., 0.3] {
        let mut d = Document::new(64, 64).unwrap();
        let mut b = brush(true);
        b.brush.color = rgba([1., 0., 0., alpha]);
        b.tip.flow = 1.;
        b.dynamics.pressure_opacity = true;
        let c = WetCarrier::new(
            WetMix::new(0., 0., 0., 0.5, 0.8, 0.).unwrap(),
            8.,
            b.brush.color,
        )
        .unwrap();
        d.begin_wet_brush_stroke(b, InputPoint::new(32.5, 32.5, 0.5), &c, false)
            .unwrap();
        let (changed, next) = d.end_wet_brush_stroke().unwrap();
        assert_eq!(changed, alpha > 0.);
        assert!((d.pixel(32, 32).alpha() - alpha * 0.5 * 0.8 * 0.5 * 0.4).abs() < 1e-6);
        assert_eq!(next.sample(0.5, 0.5), c.sample(0.5, 0.5));
        assert_eq!(c.revision(), 0);
        if alpha == 0. {
            assert!(pixels(&d).iter().all(|p| *p == Pixel::TRANSPARENT));
        }
    }
}
fn line(d: &mut Document, b: BrushDefinition, c: &WetCarrier, split: bool) -> WetCarrier {
    let point = |x: f64, t: u64| {
        let mut p = InputPoint::new(x, 32., 1.);
        p.timestamp_ns = t;
        p
    };
    d.begin_wet_brush_stroke(b, point(24., 0), c, false)
        .unwrap();
    if split {
        d.stroke_to(point(64., 40_000_000)).unwrap();
    }
    d.stroke_to(point(104., 80_000_000)).unwrap();
    let (changed, result) = d.end_wet_brush_stroke().unwrap();
    assert!(changed);
    result
}
#[test]
fn dynamic_wet_colors_share_scatter_texture_image_dual_smoothing_and_paging() {
    for bits in 0..4 {
        let mut b = brush(bits & 1 != 0);
        b.smoothing = BrushSmoothing::new(true, 8., 8., 0.5, 1.).unwrap();
        if bits & 2 != 0 {
            b.mask = Some(Arc::new(
                BrushMask::new(
                    4,
                    4,
                    vec![
                        255, 255, 255, 0, 255, 0, 255, 255, 255, 255, 0, 255, 0, 255, 255, 255,
                    ],
                )
                .unwrap(),
            ));
            b.texture = BrushTexture::new(
                true,
                TextureSettings::new(TextureSpace::Document, 1., 0., 0.5, false, 0., 0.).unwrap(),
                Some(Arc::new(TextureResource::new(Arc::new(
                    BrushMask::new(2, 1, vec![128, 255]).unwrap(),
                )))),
            )
            .unwrap();
            b.dynamics.scatter = ScatterDynamics::new(true, 0.2, true, 2, 0., 11).unwrap();
            b.dual = DualBrush::new(
                true,
                DualMode::Screen,
                SecondaryTip::new(
                    6.,
                    BrushTip {
                        pressure_size: false,
                        pressure_flow: false,
                        ..Default::default()
                    },
                    None,
                    Default::default(),
                )
                .unwrap()
                .with_spacing(0.25)
                .unwrap(),
            );
        }
        let c = WetCarrier::new(
            WetMix::new(0.7, 0.2, 0.25, 0.35, 0.8, 0.03).unwrap(),
            8.,
            b.brush.color,
        )
        .unwrap();
        let mut a = source(false);
        let mut paged = source(true);
        let returned = line(&mut a, b.clone(), &c, false);
        let other = line(&mut paged, b.clone(), &c, true);
        assert_eq!(pixels(&a), pixels(&paged), "combination {bits}");
        assert_eq!(returned.sample(0.25, 0.25), other.sample(0.25, 0.25));
        assert!(paged.storage_stats().page_writes > 0);
        let result = pixels(&paged);
        paged.undo().unwrap();
        assert_eq!(pixels(&paged), pixels(&source(false)));
        paged.redo().unwrap();
        assert_eq!(pixels(&paged), result);
        let mut disabled = source(false);
        b.dynamics.color = Default::default();
        line(&mut disabled, b, &c, false);
        assert_ne!(pixels(&disabled), result);
    }
}
#[test]
fn disabled_values_do_not_change_wet_pixels_and_frequency_has_visible_effect() {
    let mut b = brush(false);
    let c = dry(&b);
    b.dynamics.color = Default::default();
    let mut normal = Document::new(128, 64).unwrap();
    line(&mut normal, b.clone(), &c, false);
    b.dynamics.color = ColorDynamics::new(false, 1., 1., 1., 1., false, u64::MAX).unwrap();
    let mut closed = Document::new(128, 64).unwrap();
    line(&mut closed, b, &c, false);
    assert_eq!(pixels(&normal), pixels(&closed));
    let mut whole = Document::new(128, 64).unwrap();
    let mut per_tip = Document::new(128, 64).unwrap();
    let a = brush(false);
    let b = brush(true);
    line(&mut whole, a, &c, false);
    line(&mut per_tip, b, &c, false);
    assert_ne!(pixels(&whole), pixels(&per_tip));
    assert_eq!(whole.history_depth(), (1, 0));
    assert_eq!(per_tip.history_depth(), (1, 0));
}
#[test]
fn cancellation_keeps_source_carrier_and_pixels_with_color_dynamics() {
    let mut d = source(true);
    let before = pixels(&d);
    let b = brush(true);
    let c = WetCarrier::new(WetMix::default(), 8., b.brush.color).unwrap();
    let loaded = c.sample(0.5, 0.5);
    d.begin_wet_brush_stroke(b, InputPoint::new(24., 32., 1.), &c, true)
        .unwrap();
    d.stroke_to(InputPoint::new(104., 32., 1.)).unwrap();
    assert_ne!(pixels(&d), before);
    d.cancel_stroke().unwrap();
    assert_eq!(pixels(&d), before);
    assert_eq!(d.history_depth(), (0, 0));
    assert_eq!(c.revision(), 0);
    assert_eq!(c.sample(0.5, 0.5), loaded);
}
