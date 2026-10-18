use paint_core::{
    InputPoint, PressureControl, PressureCurve, StrokeTaper, TaperBuffer, TaperEnd, TaperFactors,
    TaperRecord, TaperStream, TaperedRecord, Tool, MAX_TAPER_DISTANCE, MAX_TAPER_RECORDS,
};

fn control(minimum: f32) -> PressureControl {
    PressureControl::new(PressureCurve::default(), minimum).unwrap()
}
#[test]
fn relative_lengths_capture_diameter_and_keep_independent_values_and_opacity() {
    for diameter in [1., 6., 32., 2000.] {
        for percent in [0., 12.5, 100., 10000.] {
            let end = TaperEnd::new(17., control(0.2), control(1.))
                .unwrap()
                .with_relative_length(true, percent)
                .unwrap();
            let relative = StrokeTaper::new(true, end.clone(), end);
            let length = (diameter * percent / 100.).min(4096.);
            let absolute = StrokeTaper::new(
                true,
                TaperEnd::new(length, control(0.2), control(1.)).unwrap(),
                TaperEnd::new(length, control(0.2), control(1.)).unwrap(),
            );
            for distance in [0., 0.5, 12., 48., 100.] {
                let actual = relative
                    .compile(diameter)
                    .unwrap()
                    .factors(distance, 100.)
                    .unwrap();
                assert_eq!(
                    actual,
                    absolute
                        .compile(diameter)
                        .unwrap()
                        .factors(distance, 100.)
                        .unwrap()
                );
                assert_eq!(actual.opacity, 1.);
            }
            let closed = relative
                .start()
                .clone()
                .with_relative_length(false, percent)
                .unwrap();
            assert_eq!(closed.effective_length(diameter), 17.);
            assert_eq!(closed.relative_length(), Some((false, percent)));
        }
    }
    for percent in [-0.01, 10000.01, f64::NAN, f64::INFINITY] {
        assert!(TaperEnd::default()
            .with_relative_length(true, percent)
            .is_err());
    }
    for diameter in [0., 2000.01, f64::NAN] {
        assert!(StrokeTaper::default().compile(diameter).is_err());
    }
}
fn settings(start: f64, end: f64) -> StrokeTaper {
    StrokeTaper::new(
        true,
        TaperEnd::new(start, control(0.2), control(0.4)).unwrap(),
        TaperEnd::new(end, control(0.1), control(0.3)).unwrap(),
    )
}
fn record(id: u64, distance: f64) -> TaperRecord {
    TaperRecord {
        id,
        distance,
        stream: if id % 2 == 0 {
            TaperStream::Secondary
        } else {
            TaperStream::Primary
        },
        point: InputPoint {
            x: distance,
            y: (id % 11) as f64,
            pressure: (id % 101) as f32 / 100.,
            tilt_x: 17.,
            tilt_y: -31.,
            rotation: 315.,
            tangential_pressure: -0.25,
            buttons: 5,
            capabilities: 15,
            tool: Tool::Pen,
            timestamp_ns: id * 4_000_000,
        },
    }
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}

#[test]
fn independent_envelopes_minima_overlap_and_smooth_curve() {
    let s = settings(10., 20.);
    let p = s.compile(1.).unwrap();
    assert_eq!(
        p.factors(0., 100.).unwrap(),
        TaperFactors {
            diameter: 0.2,
            opacity: 0.4
        }
    );
    let middle = p.factors(5., 100.).unwrap();
    close(middle.diameter, 0.6);
    close(middle.opacity, 0.7);
    let tail = p.factors(90., 100.).unwrap();
    close(tail.diameter, 0.55);
    close(tail.opacity, 0.65);
    assert_eq!(
        p.factors(10., 100.).unwrap(),
        TaperFactors {
            diameter: 1.,
            opacity: 1.
        }
    );
    let overlap = p.factors(5., 10.).unwrap();
    close(overlap.diameter, 0.325);
    close(overlap.opacity, 0.475);
    let curve = PressureCurve::new_smooth(vec![(0., 0.), (0.5, 0.2), (1., 1.)]).unwrap();
    let c = PressureControl::new(curve, 0.25).unwrap();
    let end = TaperEnd::new(10., c.clone(), control(0.)).unwrap();
    let smooth = StrokeTaper::new(true, end, TaperEnd::default())
        .compile(1.)
        .unwrap();
    close(
        smooth.factors(2.5, 100.).unwrap().diameter,
        c.response(0.25).unwrap(),
    );
    assert!((c.response(0.25).unwrap() - 0.325).abs() > 0.005);
    for i in 0..=1000 {
        let f = smooth.factors(i as f64 / 100., 100.).unwrap();
        assert!((0.25..=1.).contains(&f.diameter));
    }
}

#[test]
fn disabled_zero_length_and_single_tap_have_explicit_semantics() {
    let s = settings(10., 20.);
    let disabled = StrokeTaper::new(false, s.start().clone(), s.end().clone());
    assert_eq!(disabled.start(), s.start());
    assert_eq!(disabled.end(), s.end());
    assert!(!disabled.enabled());
    assert_eq!(disabled.end().length(), 20.);
    assert_eq!(disabled.start().size().minimum(), 0.2);
    assert_eq!(disabled.start().opacity().minimum(), 0.4);
    for p in [
        disabled.compile(1.).unwrap(),
        settings(0., 0.).compile(1.).unwrap(),
    ] {
        assert_eq!(
            p.factors(0., 0.).unwrap(),
            TaperFactors {
                diameter: 1.,
                opacity: 1.
            }
        );
    }
    let zero = TaperEnd::new(10., control(0.), control(0.)).unwrap();
    assert_eq!(
        StrokeTaper::new(true, zero.clone(), zero)
            .compile(1.)
            .unwrap()
            .factors(0., 0.)
            .unwrap(),
        TaperFactors {
            diameter: 0.,
            opacity: 0.
        }
    );
}

#[test]
fn invalid_configuration_and_distances_are_rejected_before_use() {
    for length in [-1., 4096.00001, f64::NAN, f64::INFINITY] {
        assert!(TaperEnd::new(length, control(0.), control(0.)).is_err());
    }
    for points in [vec![(0., 0.1), (1., 1.)], vec![(0., 0.), (1., 0.9)]] {
        let c = PressureControl::new(PressureCurve::new(points).unwrap(), 0.).unwrap();
        assert!(TaperEnd::new(10., c.clone(), control(0.)).is_err());
        assert!(TaperEnd::new(10., control(0.), c).is_err());
    }
    let p = settings(4096., 4096.).compile(1.).unwrap();
    for (distance, endpoint) in [
        (f64::NAN, 1.),
        (-0.01, 1.),
        (2., 1.),
        (0., f64::NAN),
        (0., -1.),
        (0., MAX_TAPER_DISTANCE + 1.),
    ] {
        assert!(p.factors(distance, endpoint).is_err());
    }
    assert!(p.factors(MAX_TAPER_DISTANCE, MAX_TAPER_DISTANCE).is_ok());
    assert!(TaperBuffer::new(&settings(0., 0.), 0, 1.).is_err());
    assert!(TaperBuffer::new(&settings(0., 0.), MAX_TAPER_RECORDS + 1, 1.).is_err());
}

#[test]
fn smooth_nonmonotonic_curves_stay_bounded_and_retired_samples_remain_final() {
    let curve =
        PressureCurve::new_smooth(vec![(0., 0.), (0.2, 0.8), (0.6, 0.1), (1., 1.)]).unwrap();
    let end = TaperEnd::new(4., PressureControl::new(curve, 0.15).unwrap(), control(0.6)).unwrap();
    let s = StrokeTaper::new(true, TaperEnd::default(), end);
    let p = s.compile(1.).unwrap();
    for i in 0..=1000 {
        let f = p.factors(10. - i as f64 / 250., 10.).unwrap();
        assert!((0.15..=1.).contains(&f.diameter));
        assert!((0.6..=1.).contains(&f.opacity));
    }
    let records: Vec<_> = (0..100).map(|i| record(i, i as f64 / 5.)).collect();
    assert_eq!(run(&records, 1, &s), run(&records, 100, &s));
}

// Simulate the caller's immutable rendered prefix and replaceable tail. This checks
// commands, not document pixels; the document checkpoint integration is a later stage.
fn run(records: &[TaperRecord], batch: usize, s: &StrokeTaper) -> Vec<TaperedRecord> {
    let mut buffer = TaperBuffer::new(s, MAX_TAPER_RECORDS, 1.).unwrap();
    let program = s.compile(1.).unwrap();
    let mut frozen = Vec::new();
    let mut seen = Vec::new();
    for chunk in records.chunks(batch) {
        let endpoint = chunk.last().unwrap().distance;
        let update = buffer.prepare_append(chunk, endpoint).unwrap();
        assert!(update.replay().len() <= MAX_TAPER_RECORDS + chunk.len());
        seen.extend_from_slice(chunk);
        frozen.extend(update.retired());
        let preview: Vec<_> = frozen.iter().copied().chain(update.pending()).collect();
        let reference: Vec<_> = seen
            .iter()
            .map(|&record| TaperedRecord {
                record,
                factors: program.factors(record.distance, endpoint).unwrap(),
            })
            .collect();
        assert_eq!(preview, reference);
        buffer.commit(update).unwrap();
    }
    let endpoint = records.last().unwrap().distance;
    let finish = buffer.prepare_finish(endpoint).unwrap();
    frozen.extend(finish.retired());
    assert_eq!(finish.pending().len(), 0);
    buffer.commit(finish).unwrap();
    assert!(buffer.finished());
    assert_eq!(buffer.pending_len(), 0);
    assert!(buffer.prepare_append(&[], endpoint).is_err());
    assert!(buffer.prepare_finish(endpoint).is_err());
    frozen
}

#[test]
fn both_streams_channels_and_global_distance_survive_replay_and_batch_splits() {
    // Two records per distance also exercises a zero-distance stream tie.
    let records: Vec<_> = (0..400).map(|i| record(i, (i / 2) as f64 * 0.75)).collect();
    let s = settings(120., 17.);
    let all = run(&records, records.len(), &s);
    assert_eq!(all, run(&records, 1, &s));
    assert_eq!(all, run(&records, 13, &s));
    assert_eq!(all.iter().map(|r| r.record).collect::<Vec<_>>(), records);
}

#[test]
fn long_strokes_keep_only_the_active_tail_and_allow_advancing_a_full_buffer() {
    let mut buffer = TaperBuffer::new(&settings(0., 4.), 4, 1.).unwrap();
    let mut retired = 0;
    for id in 0..2000 {
        let r = record(id, id as f64);
        let update = buffer.prepare_append(&[r], r.distance).unwrap();
        retired += update.retired().len();
        buffer.commit(update).unwrap();
        assert!(buffer.pending_len() <= 4);
    }
    assert_eq!(retired, 1996);
    let noop = buffer.prepare_append(&[], 1999.).unwrap();
    assert_eq!(noop.replay().len(), 0);
    assert_eq!(noop.pending().len(), 4);
    buffer.commit(noop).unwrap();
    let advance = buffer.prepare_append(&[], 2002.).unwrap();
    assert_eq!(advance.retired().len(), 3);
    assert_eq!(advance.pending().len(), 1);
    buffer.commit(advance).unwrap();
    assert_eq!(buffer.endpoint(), 2002.);
    let finish = buffer.prepare_finish(2002.).unwrap();
    assert_eq!(finish.replay().len(), 1);
    assert_eq!(finish.replay().next().unwrap().record, record(1999, 1999.));
    buffer.commit(finish).unwrap();
}

#[test]
fn capacity_and_batch_limits_reject_dense_tails_atomically() {
    let s = settings(0., 1.);
    let mut buffer = TaperBuffer::new(&s, 2, 1.).unwrap();
    buffer
        .commit(
            buffer
                .prepare_append(&[record(1, 0.), record(2, 0.)], 0.)
                .unwrap(),
        )
        .unwrap();
    assert!(buffer.prepare_append(&[record(3, 0.)], 0.).is_err());
    assert_eq!(buffer.pending_len(), 2);
    assert_eq!(buffer.endpoint(), 0.);
    let advance = buffer
        .prepare_append(&[record(3, 2.), record(4, 2.)], 2.)
        .unwrap();
    assert_eq!(advance.retired().len(), 2);
    assert_eq!(advance.pending().len(), 2);
    buffer.commit(advance).unwrap();
    let many: Vec<_> = (0..=MAX_TAPER_RECORDS)
        .map(|i| record(i as u64, 0.))
        .collect();
    let mut large = TaperBuffer::new(&s, MAX_TAPER_RECORDS, 1.).unwrap();
    assert!(large.prepare_append(&many, 0.).is_err());
    assert_eq!(large.pending_len(), 0);
    large
        .commit(
            large
                .prepare_append(&many[..MAX_TAPER_RECORDS], 0.)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(large.pending_len(), MAX_TAPER_RECORDS);
    let finish = large.prepare_finish(0.).unwrap();
    assert_eq!(finish.retired().len(), MAX_TAPER_RECORDS);
    large.commit(finish).unwrap();
}

#[test]
fn invalid_later_records_leave_prior_state_and_existing_plan_valid() {
    let mut buffer = TaperBuffer::new(&settings(0., 10.), 10, 1.).unwrap();
    buffer
        .commit(buffer.prepare_append(&[record(10, 1.)], 1.).unwrap())
        .unwrap();
    let valid = buffer.prepare_append(&[record(11, 2.)], 2.).unwrap();
    let mut variants = vec![
        record(10, 2.),
        record(12, 0.5),
        record(12, 3.),
        record(12, f64::NAN),
    ];
    let mut time = record(12, 2.);
    time.point.timestamp_ns = 1;
    variants.push(time);
    for channel in 0..7 {
        let mut r = record(12, 2.);
        match channel {
            0 => r.point.x = f64::NAN,
            1 => r.point.y = 2_000_001.,
            2 => r.point.pressure = 1.01,
            3 => r.point.tilt_x = 91.,
            4 => r.point.tilt_y = f32::NAN,
            5 => r.point.rotation = 361.,
            _ => r.point.tangential_pressure = -1.01,
        }
        variants.push(r);
    }
    for invalid in variants {
        assert!(buffer
            .prepare_append(&[record(11, 2.), invalid], 2.)
            .is_err());
        assert_eq!(buffer.endpoint(), 1.);
        assert_eq!(buffer.pending_len(), 1);
    }
    assert!(buffer.prepare_append(&[], 0.9).is_err());
    buffer.commit(valid).unwrap();
    assert_eq!(buffer.endpoint(), 2.);
}

#[test]
fn cancelled_stale_and_foreign_updates_never_adopt_state() {
    let s = settings(0., 10.);
    let mut a = TaperBuffer::new(&s, 10, 1.).unwrap();
    let mut b = TaperBuffer::new(&s, 10, 1.).unwrap();
    let cancelled = a.prepare_append(&[record(0, 0.)], 0.).unwrap();
    drop(cancelled);
    assert_eq!(a.pending_len(), 0);
    let first = a.prepare_append(&[record(0, 0.)], 0.).unwrap();
    let stale = a.prepare_append(&[record(0, 0.)], 0.).unwrap();
    let foreign = a.prepare_append(&[record(0, 0.)], 0.).unwrap();
    assert!(b.commit(foreign).is_err());
    assert_eq!(b.pending_len(), 0);
    a.commit(first).unwrap();
    assert!(a.commit(stale).is_err());
    assert_eq!(a.pending_len(), 1);
    let fork = a.clone();
    let pending = fork.prepare_append(&[record(1, 1.)], 1.).unwrap();
    a.commit(pending).unwrap(); // Same immutable base may legitimately be forked.
    assert_eq!(fork.endpoint(), 0.);
    assert_eq!(a.endpoint(), 1.);
}

#[test]
fn disabled_tail_retires_immediately_but_keeps_id_and_time_order() {
    let mut b = TaperBuffer::new(&settings(10., 0.), 1, 1.).unwrap();
    let records: Vec<_> = (0..100).map(|i| record(i, i as f64)).collect();
    let u = b.prepare_append(&records, 99.).unwrap();
    assert_eq!(u.retired().len(), 100);
    assert_eq!(u.pending().len(), 0);
    b.commit(u).unwrap();
    assert_eq!(b.pending_len(), 0);
    assert!(b.prepare_append(&[record(99, 100.)], 100.).is_err());
    let mut backwards = record(100, 100.);
    backwards.point.timestamp_ns = 0;
    assert!(b.prepare_append(&[backwards], 100.).is_err());
}
