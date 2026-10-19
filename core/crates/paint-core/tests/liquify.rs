use paint_core::*;
fn seed() -> Document {
    let mut d = Document::new(96, 96).unwrap();
    for (x, y, c) in [
        (35., 40., [1., 0., 0., 1.]),
        (48., 48., [0., 1., 0., 0.7]),
        (56., 35., [0., 0., 1., 1.]),
    ] {
        d.begin_stroke(
            Brush {
                radius: 9.,
                color: Pixel::from_straight(c).unwrap(),
                ..Brush::default()
            },
            InputPoint::new(x, y, 1.),
        )
        .unwrap();
        d.end_stroke().unwrap();
    }
    d
}
fn pixels(d: &Document) -> Vec<Pixel> {
    (0..96)
        .flat_map(|y| (0..96).map(move |x| d.pixel(x, y)))
        .collect()
}
fn dab(tool: u32) -> LiquifyDab {
    LiquifyDab {
        tool,
        radius: 24.,
        strength: 0.5,
        density: 1.,
        speed: 1.,
        pressure_enabled: false,
        from: InputPoint::new(40., 40., 1.),
        to: InputPoint::new(48., 44., 1.),
        elapsed_ms: 100.,
    }
}
#[test]
fn transaction_reset_cancel_commit_and_exact_history() {
    let mut d = seed();
    let before = pixels(&d);
    let depth = d.history_depth();
    d.begin_liquify().unwrap();
    assert!(d.liquify_dab(dab(0)).unwrap());
    assert_ne!(pixels(&d), before);
    assert_eq!(d.history_depth(), depth);
    assert!(d.undo().is_err());
    assert!(d.add_layer("blocked").is_err());
    d.reset_liquify().unwrap();
    assert_eq!(pixels(&d), before);
    assert!(d.liquify_active());
    d.liquify_dab(dab(2)).unwrap();
    d.cancel_liquify().unwrap();
    assert_eq!(pixels(&d), before);
    assert_eq!(d.history_depth(), depth);
    d.begin_liquify().unwrap();
    d.liquify_dab(dab(0)).unwrap();
    d.liquify_dab(dab(1)).unwrap();
    let preview = pixels(&d);
    assert!(d.commit_liquify().unwrap());
    let after = pixels(&d);
    assert_ne!(after, preview, "final uses original-source cubic sampling");
    assert_eq!(d.history_depth().0, depth.0 + 1);
    assert_eq!(d.history_actions().last(), Some(&HistoryAction::Liquify));
    d.undo().unwrap();
    assert_eq!(pixels(&d), before);
    d.redo().unwrap();
    assert_eq!(pixels(&d), after);
    d.undo().unwrap();
    let depth = d.history_depth();
    d.begin_liquify().unwrap();
    d.liquify_dab(dab(0)).unwrap();
    d.cancel_liquify().unwrap();
    assert_eq!(d.history_depth(), depth);
    d.redo().unwrap();
    assert_eq!(pixels(&d), after);
}
#[test]
fn pressure_strength_density_speed_and_elapsed_are_real() {
    let run = |v: LiquifyDab| {
        let mut d = seed();
        d.begin_liquify().unwrap();
        d.liquify_dab(v).unwrap();
        pixels(&d)
    };
    for tool in 0..3 {
        let v = dab(tool);
        let full = run(v);
        let mut low = v;
        low.strength = 0.1;
        assert_ne!(full, run(low));
        low = v;
        low.density = 0.1;
        assert_ne!(full, run(low));
        low = v;
        low.from.pressure = 0.;
        low.to.pressure = 0.;
        assert_eq!(full, run(low));
        low.pressure_enabled = true;
        assert_eq!(run(low), pixels(&seed()));
        if tool > 0 {
            low = v;
            low.speed = 0.1;
            assert_ne!(full, run(low));
            low = v;
            low.elapsed_ms = 0.;
            assert_eq!(run(low), pixels(&seed()));
        }
    }
}
#[test]
fn reconstruct_and_uniform_noop_preserve_original() {
    let mut d = seed();
    let before = pixels(&d);
    let depth = d.history_depth();
    d.begin_liquify().unwrap();
    d.liquify_dab(dab(0)).unwrap();
    let mut v = dab(3);
    v.radius = 60.;
    v.strength = 1.;
    v.from = v.to;
    d.liquify_dab(v).unwrap();
    assert_eq!(pixels(&d), before);
    assert!(!d.commit_liquify().unwrap());
    assert_eq!(d.history_depth(), depth);
    let mut white = Document::new(96, 96).unwrap();
    white.initialize_white_background().unwrap();
    let before = pixels(&white);
    white.begin_liquify().unwrap();
    white.liquify_dab(dab(0)).unwrap();
    assert_eq!(pixels(&white), before);
    assert!(!white.commit_liquify().unwrap());
    assert_eq!(white.tile_count(), 0);
}
#[test]
fn invalid_input_is_atomic_and_no_active_transaction_rejected() {
    let mut d = seed();
    assert!(d.liquify_dab(dab(0)).is_err());
    d.begin_liquify().unwrap();
    d.liquify_dab(dab(0)).unwrap();
    let before = pixels(&d);
    let depth = d.history_depth();
    for field in 0..7 {
        let mut v = dab(0);
        match field {
            0 => v.radius = f32::NAN,
            1 => v.tool = 4,
            2 => v.strength = 0.,
            3 => v.density = 2.,
            4 => v.speed = 0.,
            5 => v.elapsed_ms = 251.,
            _ => v.to.x = 1000000.,
        };
        assert!(d.liquify_dab(v).is_err());
        assert_eq!(pixels(&d), before);
        assert_eq!(d.history_depth(), depth);
    }
    d.cancel_liquify().unwrap();
}

#[test]
fn selection_offset_masks_and_inherited_locks() {
    let mut d = seed();
    let id = d.active_layer();
    let mut appearance = d.layer(id).unwrap().appearance();
    appearance.offset_x = 9;
    appearance.offset_y = 3;
    d.set_layer_appearance(id, appearance).unwrap();
    let selection = Selection::default()
        .apply(
            SelectionShape::geometry(SelectionKind::Rectangle, 40., 32., 22., 20., false),
            SelectionOperation::Replace,
            96,
            96,
        )
        .unwrap();
    d.set_selection(selection).unwrap();
    let before = pixels(&d);
    d.begin_liquify().unwrap();
    d.liquify_dab(dab(0)).unwrap();
    assert_ne!(pixels(&d), before);
    for y in 0..96 {
        for x in 0..96 {
            if !(40..62).contains(&x) || !(32..52).contains(&y) {
                assert_eq!(d.pixel(x, y), before[(y * 96 + x) as usize]);
            }
        }
    }
    d.cancel_liquify().unwrap();
    for lock in [LOCK_ALL, LOCK_POSITION, LOCK_TRANSPARENCY] {
        appearance.locks = lock;
        d.set_layer_appearance(id, appearance).unwrap();
        assert!(d.begin_liquify().is_err());
    }
    appearance.locks = 0;
    d.set_layer_appearance(id, appearance).unwrap();
    let group = d.group_layer(id, "group").unwrap();
    assert!(d.begin_liquify().is_err());
    let mut ga = d.layer(group).unwrap().appearance();
    ga.locks = LOCK_POSITION;
    d.set_layer_appearance(group, ga).unwrap();
    d.set_active_layer(id).unwrap();
    assert!(d.begin_liquify().is_err());
    ga.locks = 0;
    d.set_layer_appearance(group, ga).unwrap();
    let mask = d.add_mask(id).unwrap();
    d.set_active_layer(mask).unwrap();
    d.begin_liquify().unwrap();
    d.liquify_dab(dab(0)).unwrap();
    d.cancel_liquify().unwrap();
}

#[test]
fn focus_is_display_only_and_exit_restores_visibility() {
    let mut d = seed();
    let target = d.active_layer();
    let other = d.add_layer("other").unwrap();
    let group = d.group_layer(target, "group").unwrap();
    d.set_active_layer(target).unwrap();
    let depth = d.history_depth();
    let before = pixels(&d);
    d.begin_liquify().unwrap();
    d.set_liquify_focus(true).unwrap();
    let focus = d.liquify_snapshot();
    assert!(
        focus
            .layers()
            .iter()
            .find(|l| l.id() == target)
            .unwrap()
            .properties()
            .visible
    );
    assert!(
        focus
            .layers()
            .iter()
            .find(|l| l.id() == group)
            .unwrap()
            .properties()
            .visible
    );
    assert!(
        !focus
            .layers()
            .iter()
            .find(|l| l.id() == other)
            .unwrap()
            .properties()
            .visible
    );
    assert!(d.layer(other).unwrap().properties().visible);
    d.reset_liquify().unwrap();
    assert!(
        !d.liquify_snapshot()
            .layers()
            .iter()
            .find(|l| l.id() == other)
            .unwrap()
            .properties()
            .visible
    );
    assert!(!d.commit_liquify().unwrap());
    assert!(
        d.liquify_snapshot()
            .layers()
            .iter()
            .find(|l| l.id() == other)
            .unwrap()
            .properties()
            .visible
    );
    assert_eq!(d.history_depth(), depth);
    assert_eq!(pixels(&d), before);
}
#[test]
fn resource_failure_preserves_preview() {
    let mut d = Document::with_options(
        96,
        96,
        DocumentOptions {
            max_document_tiles: 1,
            ..Default::default()
        },
    )
    .unwrap();
    d.begin_stroke(
        Brush {
            radius: 6.,
            ..Default::default()
        },
        InputPoint::new(57., 32., 1.),
    )
    .unwrap();
    d.end_stroke().unwrap();
    let before = pixels(&d);
    let depth = d.history_depth();
    d.begin_liquify().unwrap();
    let mut v = dab(0);
    v.from = InputPoint::new(57., 32., 1.);
    v.to = InputPoint::new(70., 32., 1.);
    assert!(matches!(d.liquify_dab(v), Err(Error::ResourceLimit(_))));
    assert_eq!(pixels(&d), before);
    assert_eq!(d.history_depth(), depth);
    assert!(d.liquify_active());
    d.cancel_liquify().unwrap();
}

fn fingerprint(d: &Document, size: u32) -> u64 {
    let mut hash = 14695981039346656037u64;
    for y in 0..size {
        for x in 0..size {
            for value in d.pixel(x, y).components() {
                hash ^= u64::from(value.to_bits());
                hash = hash.wrapping_mul(1099511628211);
            }
        }
    }
    hash
}
#[test]
fn elastic_liquify_cpu_fingerprint_stays_exact() {
    let mut d = seed();
    d.begin_liquify().unwrap();
    for tool in [0, 2, 1, 0, 3] {
        d.liquify_dab(dab(tool)).unwrap();
    }
    // Fixed elastic operator plan, including the bounded inverse-step mesh halo.
    assert_eq!(fingerprint(&d, 96), 4348800649998126968);
}
#[test]
#[ignore = "manual release performance comparison"]
fn liquify_release_workload() {
    let mut d = Document::new(512, 512).unwrap();
    for (x, y) in [(180., 230.), (290., 230.), (240., 320.)] {
        d.begin_stroke(
            Brush {
                radius: 100.,
                color: Pixel::WHITE,
                ..Default::default()
            },
            InputPoint::new(x, y, 1.),
        )
        .unwrap();
        d.end_stroke().unwrap();
    }
    d.begin_liquify().unwrap();
    let start = std::time::Instant::now();
    for i in 0..24 {
        let mut v = dab(i % 3);
        v.radius = 128.;
        v.from = InputPoint::new(240. + f64::from(i % 4) * 3., 256., 1.);
        v.to = InputPoint::new(v.from.x + 8., 260., 1.);
        d.liquify_dab(v).unwrap();
    }
    println!(
        "liquify 24 dabs {:?}, fingerprint {}",
        start.elapsed(),
        fingerprint(&d, 512)
    );
    assert_ne!(fingerprint(&d, 512), 0);
    assert!(d.commit_liquify().unwrap());
}

#[test]
fn off_canvas_mesh_is_editable_without_changing_unaffected_pixels() {
    let mut d = seed();
    let before = pixels(&d);
    d.begin_liquify().unwrap();
    let mut v = dab(0);
    v.from = InputPoint::new(150., 48., 1.);
    v.to = InputPoint::new(155., 48., 1.);
    assert!(d.liquify_dab(v).unwrap());
    assert_eq!(pixels(&d), before);
    assert!(!d.commit_liquify().unwrap());
}

#[test]
fn session_history_groups_strokes_restores_fields_and_preserves_document_history() {
    let mut d = seed();
    let original = pixels(&d);
    let history = d.history_actions();
    let depth = d.history_depth();
    d.begin_liquify().unwrap();
    assert_eq!(d.liquify_history_depth(), Some((0, 0)));
    d.begin_liquify_stroke().unwrap();
    d.liquify_dab(dab(0)).unwrap();
    d.liquify_dab(dab(2)).unwrap();
    assert!(d.end_liquify_stroke().unwrap());
    let first = pixels(&d);
    assert_eq!(d.liquify_history_depth(), Some((1, 0)));
    assert_eq!(d.history_actions(), history);
    d.begin_liquify_stroke().unwrap();
    d.liquify_dab(dab(1)).unwrap();
    d.end_liquify_stroke().unwrap();
    let second = pixels(&d);
    assert!(d.undo_liquify().unwrap());
    assert_eq!(pixels(&d), first);
    assert!(d.undo_liquify().unwrap());
    assert_eq!(pixels(&d), original);
    assert!(!d.undo_liquify().unwrap());
    assert!(d.redo_liquify().unwrap());
    assert!(d.redo_liquify().unwrap());
    assert_eq!(pixels(&d), second);
    d.reset_liquify().unwrap();
    assert_eq!(pixels(&d), original);
    d.undo_liquify().unwrap();
    assert_eq!(pixels(&d), second);
    d.undo_liquify().unwrap();
    assert_eq!(pixels(&d), first);
    d.begin_liquify_stroke().unwrap();
    d.liquify_dab(dab(1)).unwrap();
    d.end_liquify_stroke().unwrap();
    assert_eq!(pixels(&d), second);
    assert_eq!(d.liquify_history_depth(), Some((2, 0)));
    assert_eq!(d.history_depth(), depth);
    d.cancel_liquify().unwrap();
    assert_eq!(pixels(&d), original);
    assert_eq!(d.history_actions(), history);
    assert_eq!(d.liquify_history_depth(), None);
    d.begin_liquify().unwrap();
    d.begin_liquify_stroke().unwrap();
    d.liquify_dab(dab(0)).unwrap();
    d.end_liquify_stroke().unwrap();
    d.commit_liquify().unwrap();
    let after = pixels(&d);
    assert_eq!(d.history_depth().0, depth.0 + 1);
    d.undo().unwrap();
    assert_eq!(pixels(&d), original);
    d.redo().unwrap();
    assert_eq!(pixels(&d), after);
}
#[test]
fn session_history_limit_and_noop_strokes() {
    let mut d = seed();
    d.set_max_history_commands(2).unwrap();
    d.begin_liquify().unwrap();
    d.begin_liquify_stroke().unwrap();
    assert!(!d.end_liquify_stroke().unwrap());
    let mut boundary = Vec::new();
    for _ in 0..4 {
        d.begin_liquify_stroke().unwrap();
        d.liquify_dab(dab(0)).unwrap();
        d.end_liquify_stroke().unwrap();
        boundary.push(pixels(&d));
    }
    assert_eq!(d.liquify_history_depth(), Some((2, 0)));
    assert_eq!(
        d.liquify_history_actions().unwrap()[0],
        HistoryAction::Truncated
    );
    d.undo_liquify().unwrap();
    d.undo_liquify().unwrap();
    assert_eq!(pixels(&d), boundary[1]);
    assert!(!d.undo_liquify().unwrap());
    d.cancel_liquify().unwrap();
}
