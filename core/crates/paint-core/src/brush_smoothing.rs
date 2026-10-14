//! Fixed-time path filtering shared by all stroke operators.
use crate::{Error, InputPoint, Result};
const TICK: u64 = 4_000_000;
const MAX_POINTS: u64 = 4096;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushSmoothing {
    enabled: bool,
    position_ms: f64,
    pressure_ms: f64,
    speed_response: f64,
    stabilizer_px: f64,
}
impl Default for BrushSmoothing {
    fn default() -> Self {
        Self {
            enabled: false,
            position_ms: 12.,
            pressure_ms: 8.,
            speed_response: 0.5,
            stabilizer_px: 0.,
        }
    }
}
impl BrushSmoothing {
    pub fn new(
        enabled: bool,
        position_ms: f64,
        pressure_ms: f64,
        speed_response: f64,
        stabilizer_px: f64,
    ) -> Result<Self> {
        for (value, maximum) in [
            (position_ms, 250.),
            (pressure_ms, 250.),
            (speed_response, 1.),
            (stabilizer_px, 128.),
        ] {
            if !value.is_finite() || !(0. ..=maximum).contains(&value) {
                return Err(Error::InvalidArgument("smoothing parameters"));
            }
        }
        Ok(Self {
            enabled,
            position_ms,
            pressure_ms,
            speed_response,
            stabilizer_px,
        })
    }
    pub fn enabled(self) -> bool {
        self.enabled
    }
    pub fn position_ms(self) -> f64 {
        self.position_ms
    }
    pub fn pressure_ms(self) -> f64 {
        self.pressure_ms
    }
    pub fn speed_response(self) -> f64 {
        self.speed_response
    }
    pub fn stabilizer_px(self) -> f64 {
        self.stabilizer_px
    }
    pub fn is_default(self) -> bool {
        self == Self::default()
    }
    fn active(self) -> bool {
        self.enabled && (self.position_ms > 0. || self.pressure_ms > 0. || self.stabilizer_px > 0.)
    }
}
#[derive(Clone, Debug)]
pub(crate) struct PathFilter {
    settings: BrushSmoothing,
    raw: InputPoint,
    tick_raw: InputPoint,
    filtered: InputPoint,
    emitted: InputPoint,
    next: u64,
}
fn next_tick(time: u64) -> Result<u64> {
    time.checked_add(TICK)
        .ok_or(Error::InvalidArgument("smoothing time overflow"))
}
fn alpha(milliseconds: f64) -> f64 {
    if milliseconds == 0. {
        1.
    } else {
        -(-4. / milliseconds).exp_m1()
    }
}
impl PathFilter {
    pub fn new(settings: BrushSmoothing, first: InputPoint) -> Result<Option<Self>> {
        if !settings.active() {
            return Ok(None);
        }
        Ok(Some(Self {
            settings,
            raw: first,
            tick_raw: first,
            filtered: first,
            emitted: first,
            next: next_tick(first.timestamp_ns)?,
        }))
    }
    fn tick(&mut self, raw: InputPoint, finishing: bool) -> InputPoint {
        let speed = (raw.x - self.tick_raw.x).hypot(raw.y - self.tick_raw.y) / 0.004;
        let position =
            alpha(self.settings.position_ms / (1. + self.settings.speed_response * speed / 100.));
        let pressure = alpha(self.settings.pressure_ms);
        let mut point = raw;
        point.x = self.filtered.x + (raw.x - self.filtered.x) * position;
        point.y = self.filtered.y + (raw.y - self.filtered.y) * position;
        point.pressure = (f64::from(self.filtered.pressure)
            + f64::from(raw.pressure - self.filtered.pressure) * pressure)
            .clamp(0., 1.) as f32;
        self.filtered = point;
        let distance = (point.x - self.emitted.x).hypot(point.y - self.emitted.y);
        if !finishing && self.settings.stabilizer_px > 0. {
            let movement = if distance <= self.settings.stabilizer_px {
                0.
            } else {
                1. - self.settings.stabilizer_px / distance
            };
            point.x = self.emitted.x + (point.x - self.emitted.x) * movement;
            point.y = self.emitted.y + (point.y - self.emitted.y) * movement;
        }
        self.tick_raw = raw;
        self.emitted = point;
        point
    }
    pub fn push(&mut self, point: InputPoint) -> Result<Vec<InputPoint>> {
        point.validate()?;
        if point.timestamp_ns < self.raw.timestamp_ns {
            return Err(Error::InvalidArgument("tablet time must be monotonic"));
        }
        if point.timestamp_ns == self.raw.timestamp_ns {
            self.next = next_tick(point.timestamp_ns)?;
            self.raw = point;
            self.tick_raw = point;
            self.filtered = point;
            self.emitted = point;
            return Ok(vec![point]);
        }
        let count = if point.timestamp_ns < self.next {
            0
        } else {
            (point.timestamp_ns - self.next) / TICK + 1
        };
        if count > MAX_POINTS {
            return Err(Error::ResourceLimit("smoothing input time span"));
        }
        // Check arithmetic before mutating state or producing points.
        let next = self
            .next
            .checked_add(count * TICK)
            .ok_or(Error::InvalidArgument("smoothing time overflow"))?;
        let start = self.raw;
        let mut result = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let t = (self.next - start.timestamp_ns) as f64
                / (point.timestamp_ns - start.timestamp_ns) as f64;
            let mut raw = start.interpolate(point, t);
            raw.timestamp_ns = self.next;
            result.push(self.tick(raw, false));
            self.next += TICK;
        }
        self.next = next;
        self.raw = point;
        Ok(result)
    }
    pub fn finish(&mut self) -> Result<Vec<InputPoint>> {
        let mut points = Vec::new();
        for _ in 0..512 {
            if (self.emitted.x - self.raw.x).hypot(self.emitted.y - self.raw.y) < 0.001
                && (self.emitted.pressure - self.raw.pressure).abs() < 0.00001
            {
                break;
            }
            let mut raw = self.raw;
            raw.timestamp_ns = self.next;
            self.next = next_tick(self.next)?;
            points.push(self.tick(raw, true));
        }
        // Preserve the real endpoint and channels; synthetic time remains monotonic.
        let mut endpoint = self.raw;
        endpoint.timestamp_ns = self.next;
        points.push(endpoint);
        Ok(points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    fn point(x: f64, y: f64, p: f32, time: u64) -> InputPoint {
        let mut v = InputPoint::new(x, y, p);
        v.timestamp_ns = time;
        v
    }
    fn settings() -> BrushSmoothing {
        BrushSmoothing::new(true, 24., 20., 0.5, 2.).unwrap()
    }
    #[test]
    fn fixed_time_grid_is_independent_of_linear_event_splits() {
        let first = point(0., 0., 0.25, 1_000_000);
        let last = point(48., 24., 0.75, 49_000_000);
        let mut full = PathFilter::new(settings(), first).unwrap().unwrap();
        let mut split = full.clone();
        let mut expected = full.push(last).unwrap();
        expected.extend(full.finish().unwrap());
        let mut actual = Vec::new();
        for i in 1..=16 {
            let mut p = first.interpolate(last, i as f64 / 16.);
            p.timestamp_ns = first.timestamp_ns + 3_000_000 * i;
            actual.extend(split.push(p).unwrap());
        }
        actual.extend(split.finish().unwrap());
        assert_eq!(expected.len(), actual.len());
        for (a, b) in expected.iter().zip(actual) {
            assert!((a.x - b.x).abs() < 1e-9);
            assert!((a.y - b.y).abs() < 1e-9);
            assert!((a.pressure - b.pressure).abs() < 1e-6);
            assert_eq!(a.timestamp_ns, b.timestamp_ns);
        }
        let final_point = expected.last().unwrap();
        assert_eq!(
            (final_point.x, final_point.y, final_point.pressure),
            (last.x, last.y, last.pressure)
        );
    }
    #[test]
    fn independent_filters_speed_response_stabilization_and_device_channels() {
        let start = point(0., 0., 0., 0);
        let mut end = point(40., 0., 1., 4_000_000);
        end.capabilities = 6;
        end.rotation = 45.;
        end.tilt_x = 30.;
        let mut pressure =
            PathFilter::new(BrushSmoothing::new(true, 0., 20., 0., 0.).unwrap(), start)
                .unwrap()
                .unwrap();
        let p = pressure.push(end).unwrap()[0];
        assert_eq!(p.x, 40.);
        assert!(p.pressure > 0. && p.pressure < 0.3);
        assert_eq!(p.rotation, 45.);
        assert_eq!(p.tilt_x, 30.);
        let mut slow = PathFilter::new(BrushSmoothing::new(true, 40., 0., 0., 0.).unwrap(), start)
            .unwrap()
            .unwrap();
        let mut fast = PathFilter::new(BrushSmoothing::new(true, 40., 0., 1., 0.).unwrap(), start)
            .unwrap()
            .unwrap();
        assert!(fast.push(end).unwrap()[0].x > slow.push(end).unwrap()[0].x);
        let mut stable =
            PathFilter::new(BrushSmoothing::new(true, 0., 0., 0., 10.).unwrap(), start)
                .unwrap()
                .unwrap();
        assert_eq!(stable.push(point(5., 0., 1., 4_000_000)).unwrap()[0].x, 0.);
        assert_eq!(
            stable.push(point(20., 0., 1., 8_000_000)).unwrap()[0].x,
            10.
        );
        assert_eq!(stable.finish().unwrap().last().unwrap().x, 20.);
    }
    #[test]
    fn missing_time_falls_back_and_bad_time_or_ranges_are_bounded() {
        assert!(BrushSmoothing::new(true, f64::NAN, 0., 0., 0.).is_err());
        assert!(BrushSmoothing::new(false, 251., 0., 0., 0.).is_err());
        let p = point(0., 0., 1., 12_000_000);
        let mut filter = PathFilter::new(settings(), p).unwrap().unwrap();
        assert!(filter.push(point(1., 0., 1., 11_000_000)).is_err());
        assert_eq!(filter.raw, p);
        assert!(filter.push(point(1., 0., 1., u64::MAX)).is_err());
        assert_eq!(filter.raw, p);
        assert_eq!(
            filter.push(point(3., 0., 0.5, p.timestamp_ns)).unwrap()[0].x,
            3.
        );
        assert!(PathFilter::new(settings(), point(0., 0., 1., u64::MAX)).is_err());
        let zero = BrushSmoothing::new(true, 0., 0., 1., 0.).unwrap();
        assert!(PathFilter::new(zero, p).unwrap().is_none());
        assert!(PathFilter::new(BrushSmoothing::default(), p)
            .unwrap()
            .is_none());
    }
    fn pixels(doc: &Document) -> Vec<Pixel> {
        (0..64)
            .flat_map(|y| (0..64).map(move |x| doc.pixel(x, y)))
            .collect()
    }
    fn seed() -> Document {
        let mut d = Document::new(64, 64).unwrap();
        d.begin_stroke(
            Brush {
                radius: 9.,
                color: Pixel::WHITE,
                ..Default::default()
            },
            InputPoint::new(16.5, 32.5, 1.),
        )
        .unwrap();
        d.end_stroke().unwrap();
        d
    }
    fn definition(mode: BrushMode) -> BrushDefinition {
        BrushDefinition {
            brush: Brush {
                radius: 4.,
                opacity: 0.4,
                spacing: 0.1,
                mode,
                ..Default::default()
            },
            tip: BrushTip {
                pressure_size: false,
                pressure_flow: false,
                ..Default::default()
            },
            dual: DualBrush::new(
                true,
                DualMode::Multiply,
                SecondaryTip::new(
                    4.,
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
            ),
            smoothing: settings(),
            ..Default::default()
        }
    }
    #[test]
    fn actual_operators_match_common_filtered_path_and_exact_history() {
        for mode in [
            BrushMode::Paint,
            BrushMode::Erase,
            BrushMode::Smudge {
                all_layers: false,
                finger_painting: false,
                pressure_strength: false,
            },
        ] {
            let first = point(16.5, 32.5, 0.5, 1_000_000);
            let last = point(48.5, 32.5, 1., 49_000_000);
            let b = definition(mode);
            let mut direct = b.clone();
            direct.smoothing = Default::default();
            let mut a = seed();
            let mut reference = seed();
            let before = pixels(&a);
            let depth = a.history_depth().0;
            a.begin_brush_stroke(b, first).unwrap();
            a.stroke_to(last).unwrap();
            a.end_stroke().unwrap();
            let mut filter = PathFilter::new(settings(), first).unwrap().unwrap();
            let mut points = filter.push(last).unwrap();
            points.extend(filter.finish().unwrap());
            reference.begin_brush_stroke(direct, first).unwrap();
            for p in points {
                reference.stroke_to(p).unwrap();
            }
            reference.end_stroke().unwrap();
            let after = pixels(&a);
            assert_ne!(before, after);
            assert_eq!(after, pixels(&reference));
            assert_eq!(a.history_depth().0, depth + 1);
            a.undo().unwrap();
            assert_eq!(pixels(&a), before);
            a.redo().unwrap();
            assert_eq!(pixels(&a), after);
        }
    }
    #[test]
    fn rejected_filtered_work_keeps_capture_and_cancel_restores_every_pixel() {
        let mut a = Document::new(64, 64).unwrap();
        let first = point(16.5, 32.5, 1., 1_000_000);
        let original = pixels(&a);
        a.begin_brush_stroke(definition(BrushMode::Paint), first)
            .unwrap();
        let started = pixels(&a);
        assert!(a
            .stroke_to(point(2_000_000., 32.5, 1., 49_000_000))
            .is_err());
        assert_eq!(pixels(&a), started);
        assert!(a.stroke_to(point(32., 32.5, 1., 0)).is_err());
        assert_eq!(pixels(&a), started);
        a.stroke_to(point(48.5, 32.5, 1., 49_000_000)).unwrap();
        a.cancel_stroke().unwrap();
        assert_eq!(pixels(&a), original);
        assert_eq!(a.history_depth().0, 0);
    }
    #[test]
    fn disabled_and_zero_filters_preserve_old_pixels() {
        let first = point(16.5, 16.5, 0.3, 1_000_000);
        let last = point(48.5, 48.5, 0.8, 49_000_000);
        let mut expected = None;
        for smoothing in [
            BrushSmoothing::default(),
            BrushSmoothing::new(false, 250., 250., 1., 128.).unwrap(),
            BrushSmoothing::new(true, 0., 0., 0., 0.).unwrap(),
        ] {
            let mut b = definition(BrushMode::Paint);
            b.smoothing = smoothing;
            let mut doc = Document::new(64, 64).unwrap();
            doc.begin_brush_stroke(b, first).unwrap();
            doc.stroke_to(last).unwrap();
            doc.end_stroke().unwrap();
            if let Some(ref p) = expected {
                assert_eq!(p, &pixels(&doc));
            } else {
                expected = Some(pixels(&doc));
            }
        }
    }
    #[test]
    fn selection_masks_offsets_and_paging_keep_filtered_pixels_and_history() {
        let first = point(16.5, 32.5, 1., 1_000_000);
        let last = point(48.5, 32.5, 1., 49_000_000);
        for mask in [false, true] {
            let mut a = Document::new(64, 64).unwrap();
            a.initialize_white_background().unwrap();
            if mask {
                a.add_mask(1).unwrap();
            }
            a.set_selection(
                Selection::default()
                    .apply(
                        SelectionShape::geometry(
                            SelectionKind::Rectangle,
                            32.,
                            0.,
                            32.,
                            64.,
                            false,
                        ),
                        SelectionOperation::Replace,
                        64,
                        64,
                    )
                    .unwrap(),
            )
            .unwrap();
            let mut b = definition(if mask {
                BrushMode::Paint
            } else {
                BrushMode::Erase
            });
            b.brush.color = Pixel::from_straight([0.,0.,0.,1.]).unwrap();
            a.begin_brush_stroke(b, first).unwrap();
            a.stroke_to(last).unwrap();
            a.end_stroke().unwrap();
            assert_eq!(a.pixel(24, 32), Pixel::WHITE);
            assert!(a.pixel(40, 32).alpha() < 1.);
            let after = pixels(&a);
            a.undo().unwrap();
            assert_eq!(a.pixel(40, 32), Pixel::WHITE);
            a.redo().unwrap();
            assert_eq!(pixels(&a), after);
        }
        let mut normal = Document::new(64, 64).unwrap();
        let mut shifted = Document::new(64, 64).unwrap();
        let mut appearance = shifted.layer(1).unwrap().appearance();
        appearance.offset_x = 3;
        shifted.set_layer_appearance(1, appearance).unwrap();
        for d in [&mut normal, &mut shifted] {
            d.begin_brush_stroke(definition(BrushMode::Paint), first)
                .unwrap();
            d.stroke_to(last).unwrap();
            d.end_stroke().unwrap();
        }
        assert_eq!(pixels(&normal), pixels(&shifted));
        let mut paged = Document::with_options(
            192,
            64,
            DocumentOptions {
                max_resident_tiles: 1,
                ..Default::default()
            },
        )
        .unwrap();
        let mut resident = Document::new(192, 64).unwrap();
        for d in [&mut paged, &mut resident] {
            d.begin_brush_stroke(definition(BrushMode::Paint), first)
                .unwrap();
            d.stroke_to(point(170.5, 32.5, 1., 193_000_000)).unwrap();
            d.end_stroke().unwrap();
        }
        let image = |d: &Document| {
            (0..64)
                .flat_map(|y| (0..192).map(move |x| d.pixel(x, y)))
                .collect::<Vec<_>>()
        };
        assert_eq!(image(&paged), image(&resident));
        assert!(paged.storage_stats().page_writes > 0);
        let after = image(&paged);
        paged.undo().unwrap();
        assert!(image(&paged).iter().all(|p| *p == Pixel::TRANSPARENT));
        paged.redo().unwrap();
        assert_eq!(image(&paged), after);
    }
}
