use super::*;
fn start() -> InputScheduleStart {
    InputScheduleStart {
        point: InputPoint::new(0., 0., 0.2),
        primary_next: 0.6,
        secondary_next: 1.7,
        path_distance: 19.25,
        direction: Some(0.3),
    }
}
fn steps(secondary: bool) -> InputScheduleSteps {
    InputScheduleSteps {
        primary: 0.6,
        secondary: secondary.then_some(1.7),
        primary_work: InputScheduleWork {
            dabs: 2,
            visits: 250_000,
        },
        secondary_work: InputScheduleWork {
            dabs: 3,
            visits: 400_000,
        },
    }
}
fn segment(x: f64, y: f64, p: f32) -> InputScheduleSegment {
    InputScheduleSegment {
        point: InputPoint::new(x, y, p),
        stationary_primary: false,
        stationary_secondary: false,
    }
}
fn collect(
    mut schedule: BrushInputSchedule,
    budget: InputScheduleBudget,
) -> Vec<InputScheduleEvent> {
    let mut events = Vec::new();
    let mut pages = 0;
    while !schedule.complete() {
        let page = schedule.prepare_page(budget).unwrap();
        assert!(!page.events().is_empty());
        assert!(budget.fits(page.work()));
        events.extend_from_slice(page.events());
        let complete = page.complete();
        schedule.adopt(page).unwrap();
        assert_eq!(schedule.complete(), complete);
        pages += 1;
        assert!(pages < 20000);
    }
    events
}
// Independent copy of the prior Document loop: no calls to the scheduler's cursor/count.
fn legacy(
    start: InputScheduleStart,
    steps: InputScheduleSteps,
    segments: &[InputScheduleSegment],
) -> Vec<InputScheduleEvent> {
    let mut last = start.point;
    let mut next = start.primary_next;
    let mut child_next = start.secondary_next;
    let mut path = start.path_distance;
    let mut direction = start.direction;
    let mut result = Vec::new();
    for segment in segments {
        let point = segment.point;
        let distance = (point.x - last.x).hypot(point.y - last.y);
        if distance > 1e-9 {
            direction = Some((point.y - last.y).atan2(point.x - last.x));
        }
        let mut main = if distance + 1e-9 < next {
            0
        } else {
            ((distance - next + 1e-9) / steps.primary).floor() as usize + 1
        };
        let mut child = steps.secondary.map_or(0, |s| {
            if distance + 1e-9 < child_next {
                0
            } else {
                ((distance - child_next + 1e-9) / s).floor() as usize + 1
            }
        });
        let mut position = next;
        let mut secondary_position = child_next;
        while main > 0 || child > 0 {
            let (stream, sample, at) =
                if child > 0 && (main == 0 || secondary_position <= position + 1e-9) {
                    let p = last.interpolate(point, (secondary_position / distance).min(1.));
                    let at = secondary_position.min(distance);
                    secondary_position += steps.secondary.unwrap();
                    child -= 1;
                    (InputScheduleStream::Secondary, p, at)
                } else {
                    let p = last.interpolate(point, (position / distance).min(1.));
                    let at = position.min(distance);
                    position += steps.primary;
                    main -= 1;
                    (InputScheduleStream::Primary, p, at)
                };
            result.push(InputScheduleEvent::Dab {
                stream,
                point: sample,
                path_distance: path + at,
                direction,
            });
        }
        next = (position - distance).max(steps.primary * 1e-9);
        if let Some(step) = steps.secondary {
            child_next = (secondary_position - distance).max(step * 1e-9);
        }
        path += distance;
        if segment.stationary_secondary {
            result.push(InputScheduleEvent::Dab {
                stream: InputScheduleStream::Secondary,
                point,
                path_distance: path,
                direction,
            });
        }
        if segment.stationary_primary {
            result.push(InputScheduleEvent::Dab {
                stream: InputScheduleStream::Primary,
                point,
                path_distance: path,
                direction,
            });
        }
        result.push(InputScheduleEvent::SegmentEnd {
            point,
            path_distance: path,
            primary_next: next,
            secondary_next: child_next,
            direction,
        });
        last = point;
    }
    result
}
#[test]
fn input_pages_match_legacy_original_segments_exactly_for_every_page_size_and_device_channel() {
    for independent in [false, true] {
        for stride in [1, 2, 7, 31, 8192] {
            let mut first = start();
            first.point.capabilities = 7;
            first.point.rotation = 350.;
            first.point.tilt_x = -30.;
            first.point.timestamp_ns = 10;
            let mut segments = vec![
                segment(0.2, 0.1, 0.4),
                segment(0.4, 0.1, 0.8),
                segment(64.5, 7.5, 0.9),
                segment(0.2, -8.2, 0.1),
                segment(0.2, -8.2, 0.7),
            ];
            for (i, s) in segments.iter_mut().enumerate() {
                s.point.capabilities = if i == 2 { 1 } else { 7 };
                s.point.rotation = -170. + i as f32 * 30.;
                s.point.tilt_x = 20.;
                s.point.tilt_y = 30.;
                s.point.tangential_pressure = -0.7 + i as f32 * 0.1;
                s.point.buttons = i as u32;
                s.point.timestamp_ns = 100 + i as u64 * 100;
            }
            segments[4].stationary_primary = true;
            segments[4].stationary_secondary = independent;
            let expected = legacy(first, steps(independent), &segments);
            let schedule = BrushInputSchedule::new(first, steps(independent), segments).unwrap();
            let budget = InputScheduleBudget::new(8192, 8_000_000, stride).unwrap();
            assert_eq!(collect(schedule, budget), expected);
        }
    }
}
#[test]
fn input_pages_do_not_insert_endpoints_and_keep_secondary_first_at_ties() {
    let s = InputScheduleSteps {
        primary: 1.,
        secondary: Some(1.),
        ..steps(true)
    };
    let first = InputScheduleStart {
        primary_next: 1.,
        secondary_next: 1.,
        ..start()
    };
    let segments = vec![segment(4.2, 0., 1.)];
    let events = collect(
        BrushInputSchedule::new(first, s, segments).unwrap(),
        InputScheduleBudget::new(3, 400_000, 2).unwrap(),
    );
    let dabs: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            InputScheduleEvent::Dab { stream, point, .. } => Some((*stream, point.x)),
            _ => None,
        })
        .collect();
    assert_eq!(dabs.len(), 8);
    for i in 0..4 {
        assert_eq!(
            dabs[i * 2],
            (InputScheduleStream::Secondary, (i + 1) as f64)
        );
        assert_eq!(
            dabs[i * 2 + 1],
            (InputScheduleStream::Primary, (i + 1) as f64)
        );
    }
    assert!(!dabs.iter().any(|(_, x)| *x == 4.2));
}
#[test]
fn input_pages_prepare_is_read_only_and_adoption_rejects_stale_and_foreign_receipts() {
    let mut schedule =
        BrushInputSchedule::new(start(), steps(true), vec![segment(30., 4., 1.)]).unwrap();
    let budget = InputScheduleBudget::new(5, 650_000, 4).unwrap();
    let a = schedule.prepare_page(budget).unwrap();
    let old = schedule.prepare_page(budget).unwrap();
    assert_eq!(a.events(), old.events());
    schedule.adopt(a).unwrap();
    let before = schedule.prepare_page(budget).unwrap().events().to_vec();
    assert!(schedule.adopt(old).is_err());
    assert_eq!(schedule.prepare_page(budget).unwrap().events(), before);
    let other = BrushInputSchedule::new(start(), steps(true), vec![segment(30., 4., 1.)]).unwrap();
    assert!(schedule.adopt(other.prepare_page(budget).unwrap()).is_err());
    assert_eq!(schedule.prepare_page(budget).unwrap().events(), before);
}
#[test]
fn input_pages_zero_dab_segments_are_bounded_and_commit_their_state() {
    let mut first = start();
    first.primary_next = 100.;
    let s = InputScheduleSteps {
        primary: 100.,
        secondary: None,
        ..steps(false)
    };
    let segments: Vec<_> = (0..4096)
        .map(|i| segment((i % 2) as f64 * 0.001, 0., 0.5))
        .collect();
    let expected = legacy(first, s, &segments);
    let schedule = BrushInputSchedule::new(first, s, segments).unwrap();
    let events = collect(schedule, InputScheduleBudget::new(1, 1, 3).unwrap());
    assert_eq!(events.len(), 4096);
    assert_eq!(events, expected);
    assert!(events
        .iter()
        .all(|e| matches!(e, InputScheduleEvent::SegmentEnd { .. })));
}
#[test]
fn input_pages_reject_geometry_time_segment_and_work_limits_before_allocation_of_events() {
    let bad = [f64::NAN, f64::INFINITY, 0., -1.];
    for value in bad {
        let s = InputScheduleSteps {
            primary: value,
            ..steps(false)
        };
        assert!(BrushInputSchedule::new(start(), s, vec![]).is_err());
    }
    let mut invalid = segment(1., 0., 1.);
    invalid.point.pressure = f32::NAN;
    assert!(BrushInputSchedule::new(start(), steps(false), vec![invalid]).is_err());
    let mut first = start();
    first.point.timestamp_ns = 100;
    assert!(BrushInputSchedule::new(first, steps(false), vec![segment(1., 0., 1.)]).is_err());
    invalid = segment(1., 0., 1.);
    invalid.stationary_primary = true;
    assert!(BrushInputSchedule::new(start(), steps(false), vec![invalid]).is_err());
    assert!(
        BrushInputSchedule::new(start(), steps(false), vec![segment(0., 0., 1.); 4097]).is_err()
    );
    let s = InputScheduleSteps {
        primary: 1e-300,
        ..steps(false)
    };
    let first = InputScheduleStart {
        primary_next: 1e-300,
        ..start()
    };
    assert!(BrushInputSchedule::new(first, s, vec![segment(1., 0., 1.)]).is_err());
    assert!(BrushInputSchedule::new(start(), steps(false), vec![segment(5000., 0., 1.)]).is_err());
    let s = InputScheduleSteps {
        primary_work: InputScheduleWork {
            dabs: 1,
            visits: 8_000_001,
        },
        ..steps(false)
    };
    assert!(BrushInputSchedule::new(start(), s, vec![segment(1., 0., 1.)]).is_err());
}
#[test]
fn input_pages_smaller_budget_failure_preserves_cursor_and_cannot_raise_old_limits() {
    for values in [
        (0, 1, 1),
        (1, 0, 1),
        (1, 1, 0),
        (8193, 1, 1),
        (1, 8_000_001, 1),
        (1, 1, 8193),
    ] {
        assert!(InputScheduleBudget::new(values.0, values.1, values.2).is_err());
    }
    let schedule =
        BrushInputSchedule::new(start(), steps(false), vec![segment(10., 0., 1.)]).unwrap();
    let before = schedule
        .prepare_page(InputScheduleBudget::default())
        .unwrap()
        .events()
        .to_vec();
    assert!(schedule
        .prepare_page(InputScheduleBudget::new(1, 1, 1).unwrap())
        .is_err());
    assert_eq!(
        schedule
            .prepare_page(InputScheduleBudget::default())
            .unwrap()
            .events(),
        before
    );
}
#[test]
fn input_pages_dense_large_segment_is_paged_without_skipping_or_holding_a_dab_array() {
    let segments = vec![segment(64., 0., 1.)];
    let expected = legacy(start(), steps(true), &segments);
    let schedule = BrushInputSchedule::new(start(), steps(true), segments).unwrap();
    let total: usize = expected
        .iter()
        .map(|e| match e {
            InputScheduleEvent::Dab {
                stream: InputScheduleStream::Primary,
                ..
            } => 250_000,
            InputScheduleEvent::Dab { .. } => 400_000,
            _ => 0,
        })
        .sum();
    assert!(total > MAX_INPUT_PAGE_VISITS);
    assert_eq!(collect(schedule, InputScheduleBudget::default()), expected);
}
#[test]
fn input_pages_empty_plan_and_oversized_actual_scatter_work_are_explicit() {
    let schedule = BrushInputSchedule::new(start(), steps(false), vec![]).unwrap();
    assert!(schedule.complete());
    let page = schedule
        .prepare_page(InputScheduleBudget::default())
        .unwrap();
    assert!(page.complete());
    assert!(page.events().is_empty());
    let s = InputScheduleSteps {
        primary_work: InputScheduleWork {
            dabs: 8193,
            visits: 1,
        },
        ..steps(false)
    };
    assert!(BrushInputSchedule::new(start(), s, vec![segment(1., 0., 1.)]).is_err());
}

#[test]
fn input_pages_preserve_the_existing_disabled_secondary_infinity_sentinel() {
    let first = InputScheduleStart {
        secondary_next: f64::INFINITY,
        ..start()
    };
    let segments = vec![segment(8., 0., 1.)];
    let schedule = BrushInputSchedule::new(first, steps(false), segments.clone()).unwrap();
    assert_eq!(
        collect(schedule, InputScheduleBudget::default()),
        legacy(first, steps(false), &segments)
    );
    assert!(BrushInputSchedule::new(first, steps(true), segments).is_err());
}
