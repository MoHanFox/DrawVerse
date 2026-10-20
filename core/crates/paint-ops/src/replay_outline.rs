//! Replay an already resolved dab at two sizes, without reconstructing a contour.
//! Values are geometry coverage, independent of color and destination pixels.

pub const MAX_TILE_PIXELS: usize = 64 * 64;
pub const MAX_MASK_PIXELS: usize = 8 * 1024 * 1024;
pub type Coverage = [u8; 4];
pub type Result<T> = std::result::Result<T, &'static str>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Primary,
    Paint,
}
#[derive(Clone, Copy, Debug)]
pub struct ReplayDab {
    pub center: [f64; 2],
    /// Actual pressure/dynamic radius after resolving the primary dab.
    pub radius: f64,
    pub outside: f64,
    pub inside: f64,
    /// True only when the captured paint sampler equals the primary sampler.
    pub shared_source: bool,
}
impl ReplayDab {
    fn validate(&self) -> Result<()> {
        if self
            .center
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 2_000_000.)
            || !self.radius.is_finite()
            || self.radius <= 0.
            || self.radius > 1_000_000.
            || [self.outside, self.inside]
                .iter()
                .any(|v| !v.is_finite() || !(0. ..=2000.).contains(v))
        {
            return Err("invalid replay dab");
        }
        Ok(())
    }
    fn position(&self, pixel: [f64; 2], radius: f64) -> Option<[f64; 2]> {
        (radius > 0.).then(|| {
            [0, 1].map(|i| self.center[i] + (pixel[i] - self.center[i]) * self.radius / radius)
        })
    }
}
fn allocate<T: Clone>(n: usize, value: T) -> Result<Vec<T>> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(n)
        .map_err(|_| "replay allocation limit")?;
    output.resize(n, value);
    Ok(output)
}
fn quantize(value: f32) -> Result<u8> {
    if !value.is_finite() || !(0. ..=1.).contains(&value) {
        return Err("invalid replay coverage");
    }
    Ok((value * 255.).round() as u8)
}

/// Accumulate one bounded rectangle into a private max-union candidate.
/// The callback receives coordinates in the original dab's sampling space.
/// The caller supplies the scaled footprint bounds and must keep callbacks pure.
/// Errors never modify `previous`; clipped/cancelled tail replay can restore it.
pub fn accumulate(
    dab: &ReplayDab,
    origin: [i32; 2],
    size: [u32; 2],
    previous: &[Coverage],
    mut sample: impl FnMut(Source, [f64; 2]) -> Result<f32>,
) -> Result<Vec<Coverage>> {
    dab.validate()?;
    let count = u64::from(size[0]) * u64::from(size[1]);
    if size.contains(&0)
        || size.iter().any(|v| *v > 64)
        || count > MAX_TILE_PIXELS as u64
        || count as usize != previous.len()
        || origin.iter().any(|v| v.unsigned_abs() > 2_000_000)
    {
        return Err("invalid replay tile");
    }
    let mut output = allocate(count as usize, [0; 4])?;
    output.copy_from_slice(previous);
    for y in 0..size[1] {
        for x in 0..size[0] {
            let i = (y * size[0] + x) as usize;
            let pixel = [
                f64::from(origin[0]) + f64::from(x) + 0.5,
                f64::from(origin[1]) + f64::from(y) + 0.5,
            ];
            for (channel, radius) in [dab.radius + dab.outside, (dab.radius - dab.inside).max(0.)]
                .into_iter()
                .enumerate()
            {
                if output[i][channel] == 255 && output[i][channel + 2] == 255 {
                    continue;
                }
                let value = match dab.position(pixel, radius) {
                    Some(position) => quantize(sample(Source::Primary, position)?)?,
                    None => 0,
                };
                output[i][channel] = output[i][channel].max(value);
                let paint = if dab.shared_source {
                    value
                } else {
                    match dab.position(pixel, radius) {
                        Some(position) => quantize(sample(Source::Paint, position)?)?,
                        None => 0,
                    }
                };
                output[i][channel + 2] = output[i][channel + 2].max(paint);
            }
        }
    }
    Ok(output)
}
/// Saturating geometric subtraction, with an optional primary-envelope clip.
/// Unlike color-over, repeated identical dabs cannot darken or thicken the ring.
pub fn coverage(value: Coverage, allow_overflow: bool) -> f32 {
    let paint = value[2].saturating_sub(value[3]);
    f32::from(if allow_overflow {
        paint
    } else {
        paint.min(value[0].saturating_sub(value[1]))
    }) / 255.
}
pub fn compose(values: &[Coverage], allow_overflow: bool) -> Result<Vec<f32>> {
    if values.len() > MAX_MASK_PIXELS {
        return Err("replay mask pixel limit");
    }
    let mut output = allocate(values.len(), 0.)?;
    for (out, &value) in output.iter_mut().zip(values) {
        *out = coverage(value, allow_overflow);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dab(outside: f64, inside: f64) -> ReplayDab {
        ReplayDab {
            center: [16., 16.],
            radius: 6.,
            outside,
            inside,
            shared_source: true,
        }
    }
    fn disk(_: Source, p: [f64; 2]) -> Result<f32> {
        Ok(if (p[0] - 16.).hypot(p[1] - 16.) <= 6. {
            1.
        } else {
            0.
        })
    }
    #[test]
    fn two_replayed_circles_make_outside_center_and_inside_rings() {
        for (outside, inside) in [(3., 0.), (1.5, 1.5), (0., 3.)] {
            let masks = accumulate(
                &dab(outside, inside),
                [0, 0],
                [32, 32],
                &vec![[0; 4]; 1024],
                disk,
            )
            .unwrap();
            let result = compose(&masks, false).unwrap();
            for y in 0..32 {
                for x in 0..32 {
                    let distance = (x as f64 + 0.5 - 16.).hypot(y as f64 + 0.5 - 16.);
                    let expected = distance <= 6. + outside && distance > 6. - inside;
                    assert_eq!(result[y * 32 + x], if expected { 1. } else { 0. });
                }
            }
        }
    }
    #[test]
    fn max_union_keeps_crossings_and_repeated_dabs_color_independent() {
        let a = accumulate(&dab(3., 0.), [0, 0], [32, 32], &vec![[0; 4]; 1024], disk).unwrap();
        let b = accumulate(&dab(3., 0.), [0, 0], [32, 32], &a, disk).unwrap();
        assert_eq!(a, b);
        let mut other = dab(3., 0.);
        other.center[0] += 4.;
        let shifted = |_: Source, p: [f64; 2]| {
            Ok(if (p[0] - 20.).hypot(p[1] - 16.) <= 6. {
                1.
            } else {
                0.
            })
        };
        let ab = accumulate(&other, [0, 0], [32, 32], &a, shifted).unwrap();
        let first = accumulate(&other, [0, 0], [32, 32], &vec![[0; 4]; 1024], shifted).unwrap();
        let ba = accumulate(&dab(3., 0.), [0, 0], [32, 32], &first, disk).unwrap();
        assert_eq!(ab, ba);
        assert_eq!(coverage(ab[16 * 32 + 18], false), 0.);
    }
    #[test]
    fn overflow_preserves_independent_tip_but_never_invents_self_coverage() {
        let mut d = dab(3., 0.);
        d.shared_source = false;
        let masks = accumulate(&d, [0, 0], [32, 32], &vec![[0; 4]; 1024], |source, p| {
            if source == Source::Primary {
                disk(source, p)
            } else {
                Ok(if (p[0] - 16.).abs() <= 9. && (p[1] - 16.).abs() <= 2. {
                    1.
                } else {
                    0.
                })
            }
        })
        .unwrap();
        let clipped = compose(&masks, false).unwrap();
        let overflow = compose(&masks, true).unwrap();
        assert!(clipped.iter().zip(&overflow).all(|(a, b)| a <= b));
        assert_ne!(clipped, overflow);
        let own = accumulate(&dab(3., 0.), [0, 0], [32, 32], &vec![[0; 4]; 1024], disk).unwrap();
        assert_eq!(compose(&own, false).unwrap(), compose(&own, true).unwrap());
    }
    #[test]
    fn subpixel_soft_transformed_tips_reuse_identical_primary_actions() {
        let mut d = dab(2., 1.);
        d.center = [15.3, 18.7];
        d.radius = 4.25;
        let sample = |_: Source, p: [f64; 2]| {
            Ok(
                ((5. - ((p[0] - 15.3) * 0.7 + (p[1] - 18.7) * 0.3).abs()) / 5.).clamp(0., 1.)
                    as f32,
            )
        };
        let a = accumulate(&d, [0, 0], [32, 32], &vec![[0; 4]; 1024], sample).unwrap();
        assert!(a.iter().any(|v| v[0] > 0 && v[0] < 255));
        assert!(a.iter().all(|v| v[0] == v[2] && v[1] == v[3]));
        let values = compose(&a, false).unwrap();
        assert!(values
            .iter()
            .all(|v| v.is_finite() && (0. ..=1.).contains(v)));
    }
    #[test]
    fn tile_partition_and_negative_world_origin_preserve_sampling() {
        let whole =
            accumulate(&dab(3., 0.), [-3, -5], [32, 32], &vec![[0; 4]; 1024], disk).unwrap();
        for part in 0..2 {
            let tile = accumulate(
                &dab(3., 0.),
                [-3 + part * 16, -5],
                [16, 32],
                &vec![[0; 4]; 512],
                disk,
            )
            .unwrap();
            for y in 0..32 {
                for x in 0..16 {
                    assert_eq!(tile[y * 16 + x], whole[y * 32 + part as usize * 16 + x]);
                }
            }
        }
    }
    #[test]
    fn collapsed_inner_radius_is_empty_without_division() {
        let masks = accumulate(&dab(0., 20.), [0, 0], [32, 32], &vec![[0; 4]; 1024], disk).unwrap();
        assert!(masks.iter().all(|v| v[1] == 0 && v[3] == 0));
        assert!(compose(&masks, false).unwrap().contains(&1.));
    }
    #[test]
    fn bad_parameters_and_mid_sample_errors_leave_previous_unchanged() {
        let previous = vec![[10, 5, 10, 5]; 1024];
        let before = previous.clone();
        let mut calls = 0;
        assert!(
            accumulate(&dab(3., 0.), [0, 0], [32, 32], &previous, |source, p| {
                calls += 1;
                if calls == 10 {
                    Err("sample failed")
                } else {
                    disk(source, p)
                }
            })
            .is_err()
        );
        assert_eq!(previous, before);
        for value in [f64::NAN, f64::INFINITY, -1., 0.] {
            let mut d = dab(3., 0.);
            d.radius = value;
            assert!(accumulate(&d, [0, 0], [32, 32], &previous, disk).is_err());
        }
        for value in [f32::NAN, -0.1, 1.1] {
            assert!(
                accumulate(&dab(3., 0.), [0, 0], [32, 32], &previous, |_, _| Ok(value)).is_err()
            );
        }
        assert!(accumulate(&dab(3., 0.), [0, 0], [65, 32], &previous, disk).is_err());
        assert!(accumulate(&dab(3., 0.), [0, 0], [0, 32], &[], disk).is_err());
        assert!(accumulate(&dab(3., 0.), [2_000_001, 0], [32, 32], &previous, disk).is_err());
    }
}
