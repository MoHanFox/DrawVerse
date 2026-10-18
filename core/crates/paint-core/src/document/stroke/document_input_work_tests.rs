use super::*;
fn legacy(
    brush: Brush,
    program: &BrushProgram,
    count: usize,
    secondary_count: usize,
    wet_work: usize,
) -> Result<()> {
    let count = count.saturating_mul(program.scatter.maximum_count());
    let diameter = (f64::from(brush.radius)
        * 2.
        * if program.has_mask() {
            std::f64::consts::SQRT_2
        } else {
            1.
        }
        + 2.)
        .ceil() as usize;
    let dual = program.dual.as_ref();
    let paired = dual.filter(|d| d.step().is_none());
    let path = dual.filter(|d| d.step().is_some());
    let sample_cost = if matches!(brush.mode, BrushMode::Smudge { .. }) {
        5
    } else {
        1
    } + if program.has_texture() { 8 } else { 0 }
        + if program.write_blend != crate::BlendMode::Normal {
            8
        } else {
            0
        }
        + paired.map_or(0, |d| d.sample_cost())
        + usize::from(path.is_some());
    let (secondary_dabs, secondary_visits) = path.map_or((0, 0), |d| d.path_work(secondary_count));
    let colored_visits = dual.filter(|d| d.color.enabled()).map_or(0, |d| {
        count
            .saturating_mul((diameter + 1) * (diameter + 1))
            .saturating_mul(d.colored_cost(program.write_blend))
            .saturating_add(d.colored_work(
                if d.step().is_some() {
                    secondary_count
                } else {
                    count
                },
                program.write_blend,
            ))
    });
    if count
        .saturating_mul(paired.map_or(1, |d| d.maximum_count()))
        .saturating_add(secondary_dabs)
        > MAX_DABS_PER_SEGMENT
        || count
            .saturating_mul(diameter * diameter)
            .saturating_mul(sample_cost)
            .saturating_add(secondary_visits)
            .saturating_add(colored_visits)
            .saturating_add(count.saturating_mul(wet_work))
            .saturating_add(if wet_work > 0 {
                count
                    .saturating_mul(diameter * diameter)
                    .saturating_mul(12 + if program.color.active() { 8 } else { 0 })
            } else {
                0
            })
            > MAX_PIXEL_VISITS_PER_SEGMENT
    {
        return Err(Error::ResourceLimit(
            "input segment too large; resample into shorter segments",
        ));
    }
    Ok(())
}

#[test]
fn input_page_work_preserves_legacy_budget_classification_for_every_supported_cost_branch() {
    use crate::{
        BlendMode, BrushMask, BrushTexture, DualBrush, DualColor, DualMode, ScatterDynamics,
        SecondaryTip, TextureResource,
    };
    let mask = Arc::new(BrushMask::new(2, 2, vec![255, 0, 255, 128]).unwrap());
    let mut cases = 0;
    for radius in [0.5, 4., 64., 256.] {
        for assets in [false, true] {
            for mode in [
                BrushMode::Paint,
                BrushMode::Erase,
                BrushMode::Smudge {
                    all_layers: true,
                    finger_painting: false,
                    pressure_strength: true,
                },
            ] {
                for model in 0..4 {
                    let mut d = BrushDefinition::from(Brush {
                        radius,
                        mode,
                        ..Default::default()
                    });
                    d.dynamics.scatter = ScatterDynamics::new(true, 0.3, true, 3, 0.5, 9).unwrap();
                    if assets {
                        d.mask = Some(mask.clone());
                        d.texture = BrushTexture::new(
                            true,
                            Default::default(),
                            Some(Arc::new(TextureResource::new(mask.clone()))),
                        )
                        .unwrap();
                    }
                    if model > 0 {
                        let child = SecondaryTip::new(
                            12.,
                            Default::default(),
                            assets.then(|| mask.clone()),
                            Default::default(),
                        )
                        .unwrap();
                        let child = if model == 1 {
                            child
                        } else {
                            child.with_spacing(0.2).unwrap()
                        };
                        d.dual = DualBrush::new(true, DualMode::Multiply, child);
                        if model == 3 && mode == BrushMode::Paint {
                            d.dual = d.dual.with_color(
                                DualColor::new(
                                    true,
                                    true,
                                    true,
                                    Default::default(),
                                    BlendMode::Multiply,
                                    BlendMode::Screen,
                                )
                                .unwrap(),
                            );
                            d.write_blend = BlendMode::Overlay;
                        }
                    }
                    let program = BrushProgram::compile(d.clone()).unwrap();
                    for wet in [0, 100, 40_000] {
                        for count in [0, 1, 2, 7, 16, 100, 8191, 8192, 8193, usize::MAX] {
                            for secondary in [0, 1, 8, 8192, usize::MAX] {
                                assert_eq!(
                                    Document::validate_stroke_work(
                                        d.brush, &program, count, secondary, wet
                                    ),
                                    legacy(d.brush, &program, count, secondary, wet)
                                );
                                cases += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 14400);
}
