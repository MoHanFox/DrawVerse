use paint_core::{
    Brush, BrushMode, Document, DocumentOptions, Error, InputPoint, LayerProperties, Pixel,
    TileCoord, Tool, MAX_DIMENSION, TILE_BYTES,
};

fn point(x: f64, y: f64) -> InputPoint {
    InputPoint::new(x, y, 1.0)
}
fn brush() -> Brush {
    Brush {
        radius: 4.0,
        ..Brush::default()
    }
}
fn dot(doc: &mut Document, x: f64, y: f64, brush: Brush) {
    doc.begin_stroke(brush, point(x, y)).unwrap();
    doc.end_stroke().unwrap();
}
fn rgba(doc: &Document) -> Vec<Pixel> {
    let (w, h) = doc.dimensions();
    (0..h)
        .flat_map(|y| (0..w).map(move |x| doc.pixel(x, y)))
        .collect()
}

#[test]
fn huge_blank_document_allocates_no_tiles() {
    let doc = Document::new(MAX_DIMENSION, MAX_DIMENSION).unwrap();
    assert_eq!(doc.tile_count(), 0);
    assert_eq!(
        doc.pixel(MAX_DIMENSION - 1, MAX_DIMENSION - 1),
        Pixel::TRANSPARENT
    );
    assert_eq!(doc.layers().len(), 1);
}

#[test]
fn validates_dimensions_budgets_and_last_layer() {
    for (w, h) in [(0, 1), (1, 0), (MAX_DIMENSION + 1, 1)] {
        assert!(Document::new(w, h).is_err());
    }
    assert!(Document::with_options(
        1,
        1,
        DocumentOptions {
            max_document_tiles: 0,
            ..Default::default()
        }
    )
    .is_err());
    let mut doc = Document::new(16, 16).unwrap();
    assert_eq!(doc.remove_layer(1), Err(Error::LastLayer));
    assert_eq!(doc.set_active_layer(42), Err(Error::LayerNotFound(42)));
}

#[test]
fn cross_tile_stroke_is_one_undo_and_redo_restores_exact_pixels() {
    let mut doc = Document::new(160, 96).unwrap();
    doc.take_dirty();
    doc.begin_stroke(brush(), point(20.5, 40.5)).unwrap();
    doc.stroke_to(point(140.5, 40.5)).unwrap();
    assert!(doc.end_stroke().unwrap());
    assert_eq!(doc.tile_count(), 3);
    for x in 21..140 {
        assert!(doc.pixel(x, 40).alpha() > 0.9, "gap at {x}");
    }
    let painted = rgba(&doc);
    assert_eq!(doc.history_depth(), (1, 0));
    assert_eq!(
        doc.take_dirty().tiles,
        [
            TileCoord { x: 0, y: 0 },
            TileCoord { x: 1, y: 0 },
            TileCoord { x: 2, y: 0 }
        ]
        .into()
    );
    assert!(doc.undo().unwrap());
    assert_eq!(doc.tile_count(), 0);
    assert_eq!(doc.history_depth(), (0, 1));
    assert!(doc.redo().unwrap());
    assert_eq!(rgba(&doc), painted);
}

#[test]
fn cancel_restores_existing_tiles_and_does_not_clear_redo() {
    let mut doc = Document::new(96, 64).unwrap();
    dot(&mut doc, 20.5, 20.5, brush());
    dot(&mut doc, 80.5, 20.5, brush());
    doc.undo().unwrap();
    let original = rgba(&doc);
    doc.begin_stroke(
        Brush {
            mode: BrushMode::Erase,
            ..brush()
        },
        point(20.5, 20.5),
    )
    .unwrap();
    doc.stroke_to(point(70.5, 20.5)).unwrap();
    doc.cancel_stroke().unwrap();
    assert_eq!(rgba(&doc), original);
    assert_eq!(doc.history_depth(), (1, 1));
    assert!(doc.redo().unwrap());
    assert!(doc.pixel(80, 20).alpha() > 0.0);
}

#[test]
fn new_committed_branch_discards_redo() {
    let mut doc = Document::new(32, 32).unwrap();
    dot(&mut doc, 8.5, 8.5, brush());
    doc.undo().unwrap();
    dot(&mut doc, 20.5, 20.5, brush());
    assert_eq!(doc.history_depth(), (1, 0));
    assert!(!doc.redo().unwrap());
    assert_eq!(doc.pixel(8, 8).alpha(), 0.0);
}

#[test]
fn zero_pressure_and_outside_points_allocate_nothing_and_create_no_history() {
    let mut doc = Document::new(16, 16).unwrap();
    doc.begin_stroke(brush(), InputPoint::new(8.5, 8.5, 0.0))
        .unwrap();
    assert!(!doc.end_stroke().unwrap());
    dot(&mut doc, -50.5, -50.5, brush());
    assert_eq!(doc.tile_count(), 0);
    assert_eq!(doc.history_depth(), (0, 0));
}

#[test]
fn pressure_controls_radius_and_flow() {
    let mut strong = Document::new(32, 32).unwrap();
    let mut light = Document::new(32, 32).unwrap();
    dot(&mut strong, 16.5, 16.5, brush());
    light
        .begin_stroke(brush(), InputPoint::new(16.5, 16.5, 0.25))
        .unwrap();
    light.end_stroke().unwrap();
    assert_eq!(strong.pixel(16, 16).alpha(), 1.0);
    assert_eq!(light.pixel(16, 16).alpha(), 0.25);
    assert!(strong.pixel(19, 16).alpha() > 0.0);
    assert_eq!(light.pixel(19, 16).alpha(), 0.0);
}

#[test]
fn eraser_tool_uses_destination_out_and_is_undoable() {
    let mut doc = Document::new(32, 32).unwrap();
    dot(&mut doc, 16.5, 16.5, brush());
    let painted = rgba(&doc);
    let mut eraser = InputPoint::new(16.5, 16.5, 0.5);
    eraser.tool = Tool::Eraser;
    doc.begin_stroke(brush(), eraser).unwrap();
    doc.end_stroke().unwrap();
    assert_eq!(doc.pixel(16, 16).alpha(), 0.5);
    doc.undo().unwrap();
    assert_eq!(rgba(&doc), painted);
}

#[test]
fn fully_erased_tile_is_reclaimed_but_redo_keeps_result() {
    let mut doc = Document::new(32, 32).unwrap();
    dot(&mut doc, 16.5, 16.5, brush());
    dot(
        &mut doc,
        16.5,
        16.5,
        Brush {
            radius: 8.0,
            mode: BrushMode::Erase,
            ..brush()
        },
    );
    assert_eq!(doc.tile_count(), 0);
    doc.undo().unwrap();
    assert_eq!(doc.tile_count(), 1);
    doc.redo().unwrap();
    assert_eq!(doc.tile_count(), 0);
}

#[test]
fn layer_composition_properties_and_structure_follow_history() {
    let mut doc = Document::new(32, 32).unwrap();
    dot(
        &mut doc,
        16.5,
        16.5,
        Brush {
            color: Pixel::from_straight([0.0, 0.0, 1.0, 1.0]).unwrap(),
            ..brush()
        },
    );
    let top = doc.add_layer("颜色 / Ink").unwrap();
    dot(
        &mut doc,
        16.5,
        16.5,
        Brush {
            color: Pixel::from_straight([1.0, 0.0, 0.0, 0.5]).unwrap(),
            spacing: 1.0,
            ..brush()
        },
    );
    assert_eq!(doc.pixel(16, 16).components(), [0.5, 0.0, 0.5, 1.0]);
    doc.set_layer_properties(
        top,
        LayerProperties {
            visible: true,
            opacity: 0.5,
        },
    )
    .unwrap();
    assert_eq!(doc.pixel(16, 16).components(), [0.25, 0.0, 0.75, 1.0]);
    doc.set_layer_properties(
        top,
        LayerProperties {
            visible: false,
            opacity: 0.5,
        },
    )
    .unwrap();
    assert_eq!(doc.pixel(16, 16).components(), [0.0, 0.0, 1.0, 1.0]);
    doc.undo().unwrap();
    assert_eq!(doc.pixel(16, 16).components(), [0.25, 0.0, 0.75, 1.0]);
    doc.remove_layer(top).unwrap();
    assert_eq!(doc.active_layer(), 1);
    doc.undo().unwrap();
    assert_eq!(doc.layers()[1].id(), top);
    assert_eq!(doc.layers()[1].name(), "颜色 / Ink");
    assert_eq!(doc.pixel(16, 16).components(), [0.25, 0.0, 0.75, 1.0]);
    doc.redo().unwrap();
    assert!(doc.layer(top).is_none());
}

#[test]
fn layer_ids_do_not_alias_after_undo_branching() {
    let mut doc = Document::new(16, 16).unwrap();
    let first = doc.add_layer("first").unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.active_layer(), 1);
    let second = doc.add_layer("second").unwrap();
    assert!(second > first);
    assert!(doc.layer(first).is_none());
}

#[test]
fn layer_selection_does_not_add_history_and_mutation_is_blocked_during_stroke() {
    let mut doc = Document::new(32, 32).unwrap();
    let top = doc.add_layer("top").unwrap();
    doc.set_active_layer(1).unwrap();
    assert_eq!(doc.history_depth(), (1, 0));
    doc.begin_stroke(brush(), point(12.5, 12.5)).unwrap();
    assert_eq!(doc.set_active_layer(top), Err(Error::Busy));
    assert_eq!(doc.add_layer("blocked"), Err(Error::Busy));
    assert_eq!(doc.undo(), Err(Error::Busy));
    assert_eq!(doc.redo(), Err(Error::Busy));
    assert_eq!(doc.remove_layer(top), Err(Error::Busy));
    assert_eq!(doc.begin_stroke(brush(), point(1.0, 1.0)), Err(Error::Busy));
    doc.cancel_stroke().unwrap();
    assert_eq!(doc.end_stroke(), Err(Error::NoStroke));
}

#[test]
fn invalid_tablet_channels_do_not_corrupt_live_stroke() {
    let mut doc = Document::new(64, 64).unwrap();
    doc.begin_stroke(brush(), point(8.5, 8.5)).unwrap();
    let baseline = rgba(&doc);
    let revision = doc.revision();
    let mut invalid = point(20.5, 20.5);
    invalid.x = f64::NAN;
    assert!(doc.stroke_to(invalid).is_err());
    invalid = point(20.5, 20.5);
    invalid.pressure = -0.1;
    assert!(doc.stroke_to(invalid).is_err());
    invalid = point(20.5, 20.5);
    invalid.tilt_x = f32::INFINITY;
    assert!(doc.stroke_to(invalid).is_err());
    invalid = point(20.5, 20.5);
    invalid.tangential_pressure = 2.0;
    assert!(doc.stroke_to(invalid).is_err());
    assert_eq!(rgba(&doc), baseline);
    assert_eq!(doc.revision(), revision);
    assert!(doc.stroke_active());
    doc.stroke_to(point(20.5, 20.5)).unwrap();
    doc.end_stroke().unwrap();
}

#[test]
fn tablet_metadata_and_time_are_preserved_and_validated() {
    let mut doc = Document::new(32, 32).unwrap();
    let mut first = point(8.5, 8.5);
    first.timestamp_ns = 100;
    doc.begin_stroke(brush(), first).unwrap();
    let mut next = point(20.5, 20.5);
    next.tilt_x = 25.0;
    next.tilt_y = -32.0;
    next.rotation = 75.0;
    next.tangential_pressure = -0.25;
    next.buttons = 3;
    next.capabilities = 0x1f;
    next.timestamp_ns = 200;
    doc.stroke_to(next).unwrap();
    assert_eq!(doc.last_input(), Some(next));
    next.timestamp_ns = 99;
    assert!(doc.stroke_to(next).is_err());
    assert_eq!(doc.last_input().unwrap().timestamp_ns, 200);
}

#[test]
fn invalid_brush_and_layer_properties_are_rejected_before_mutation() {
    let mut doc = Document::new(32, 32).unwrap();
    for b in [
        Brush {
            radius: f32::NAN,
            ..brush()
        },
        Brush {
            spacing: 0.0,
            ..brush()
        },
        Brush {
            opacity: 2.0,
            ..brush()
        },
    ] {
        assert!(doc.begin_stroke(b, point(8.0, 8.0)).is_err());
        assert!(!doc.stroke_active());
    }
    assert!(doc
        .set_layer_properties(
            1,
            LayerProperties {
                visible: true,
                opacity: f32::NAN
            }
        )
        .is_err());
    assert!(doc.add_layer("bad\0name").is_err());
    assert_eq!(doc.revision(), 0);
}

#[test]
fn tile_budget_failure_rolls_back_entire_stroke_including_existing_tile_edits() {
    let options = DocumentOptions {
        max_document_tiles: 1,
        ..Default::default()
    };
    let mut doc = Document::with_options(160, 64, options).unwrap();
    dot(&mut doc, 20.5, 20.5, brush());
    let original = rgba(&doc);
    doc.begin_stroke(brush(), point(30.5, 20.5)).unwrap();
    assert!(matches!(
        doc.stroke_to(point(90.5, 20.5)),
        Err(Error::ResourceLimit(_))
    ));
    assert!(!doc.stroke_active());
    assert_eq!(rgba(&doc), original);
    assert_eq!(doc.tile_count(), 1);
    assert_eq!(doc.history_depth(), (1, 0));
}

#[test]
fn large_transaction_uses_encoded_history_and_remains_one_undo() {
    let options = DocumentOptions {
        max_history_bytes: 2 * TILE_BYTES + 256,
        ..Default::default()
    };
    let mut doc = Document::with_options(160, 64, options).unwrap();
    doc.begin_stroke(brush(), point(20.5, 20.5)).unwrap();
    doc.stroke_to(point(100.5, 20.5)).unwrap();
    doc.end_stroke().unwrap();
    let painted = rgba(&doc);
    assert!(doc.tile_count() > 1);
    assert_eq!(doc.history_depth(), (1, 0));
    assert!(doc.history_bytes() <= options.max_history_bytes);
    assert!(!doc.stroke_active());
    assert!(doc.undo().unwrap());
    assert_eq!(doc.tile_count(), 0);
    assert!(doc.redo().unwrap());
    assert_eq!(rgba(&doc), painted);
}

#[test]
fn failure_during_first_dab_also_rolls_back_partial_tile_writes() {
    let options = DocumentOptions {
        max_document_tiles: 1,
        ..Default::default()
    };
    let mut doc = Document::with_options(128, 128, options).unwrap();
    assert!(doc.begin_stroke(brush(), point(64.0, 64.0)).is_err());
    assert_eq!(doc.tile_count(), 0);
    assert!(!doc.stroke_active());
}

#[test]
fn oversized_segment_rejection_preserves_active_stroke_for_resampling() {
    let mut doc = Document::new(1024, 64).unwrap();
    doc.begin_stroke(brush(), point(10.5, 10.5)).unwrap();
    let original = rgba(&doc);
    assert!(matches!(
        doc.stroke_to(point(100_000.5, 10.5)),
        Err(Error::ResourceLimit(_))
    ));
    assert_eq!(rgba(&doc), original);
    assert!(doc.stroke_active());
    doc.stroke_to(point(30.5, 10.5)).unwrap();
    assert!(doc.end_stroke().unwrap());
}

#[test]
fn history_budget_evicts_oldest_without_losing_current_document() {
    let options = DocumentOptions {
        max_history_bytes: 2 * TILE_BYTES + 256,
        ..Default::default()
    };
    let mut doc = Document::with_options(64, 64, options).unwrap();
    dot(&mut doc, 10.5, 10.5, brush());
    dot(&mut doc, 40.5, 40.5, brush());
    assert_eq!(doc.history_depth(), (1, 0));
    assert!(doc.history_bytes() <= options.max_history_bytes);
    doc.undo().unwrap();
    assert!(doc.pixel(10, 10).alpha() > 0.0);
    assert_eq!(doc.pixel(40, 40).alpha(), 0.0);
    assert!(!doc.undo().unwrap());
}

#[test]
fn command_count_budget_and_noop_properties() {
    let options = DocumentOptions {
        max_history_commands: 2,
        ..Default::default()
    };
    let mut doc = Document::with_options(64, 64, options).unwrap();
    for x in [8.5, 25.5, 45.5] {
        dot(&mut doc, x, 20.5, brush());
    }
    assert_eq!(doc.history_depth(), (2, 0));
    let revision = doc.revision();
    doc.set_layer_properties(1, LayerProperties::default())
        .unwrap();
    assert_eq!(doc.history_depth(), (2, 0));
    assert_eq!(doc.revision(), revision);
}

#[test]
fn sampling_frequency_does_not_change_constant_pressure_straight_stroke() {
    let mut coarse = Document::new(160, 64).unwrap();
    let mut fine = Document::new(160, 64).unwrap();
    let b = Brush {
        radius: 5.0,
        opacity: 0.3,
        spacing: 0.3,
        ..brush()
    };
    for doc in [&mut coarse, &mut fine] {
        doc.begin_stroke(b, point(10.5, 20.5)).unwrap();
    }
    coarse.stroke_to(point(140.5, 20.5)).unwrap();
    for x in 11..=140 {
        fine.stroke_to(point(f64::from(x) + 0.5, 20.5)).unwrap();
    }
    coarse.end_stroke().unwrap();
    fine.end_stroke().unwrap();
    for (a, b) in rgba(&coarse).iter().zip(rgba(&fine)) {
        for (a, b) in a.components().into_iter().zip(b.components()) {
            assert!((a - b).abs() < 1e-5);
        }
    }
}

#[test]
fn snapshots_remain_unchanged_after_edit_undo_and_layer_removal() {
    let mut doc = Document::new(64, 64).unwrap();
    let top = doc.add_layer("snapshot").unwrap();
    dot(&mut doc, 20.5, 20.5, brush());
    let snapshot = doc.snapshot();
    let tile = snapshot.layers()[1].tiles().next().unwrap().1.clone();
    dot(
        &mut doc,
        20.5,
        20.5,
        Brush {
            mode: BrushMode::Erase,
            radius: 8.0,
            ..brush()
        },
    );
    doc.remove_layer(top).unwrap();
    doc.undo().unwrap();
    assert_eq!(snapshot.pixel(20, 20).alpha(), 1.0);
    assert_eq!(tile.pixel(20, 20).alpha(), 1.0);
    assert_eq!(doc.pixel(20, 20).alpha(), 0.0);
    assert!(doc.revision() > snapshot.revision);
}

#[test]
fn dirty_regions_track_preview_cancel_and_structural_changes() {
    let mut doc = Document::new(128, 128).unwrap();
    assert!(doc.take_dirty().all);
    doc.begin_stroke(brush(), point(10.5, 10.5)).unwrap();
    let preview = doc.take_dirty();
    assert!(!preview.all);
    assert_eq!(preview.tiles, [TileCoord { x: 0, y: 0 }].into());
    let revision = doc.revision();
    doc.cancel_stroke().unwrap();
    assert!(doc.revision() > revision);
    assert_eq!(doc.take_dirty().tiles, preview.tiles);
    doc.add_layer("dirty all").unwrap();
    assert!(doc.take_dirty().all);
}

#[test]
fn canvas_edges_clip_and_out_of_bounds_reads_are_transparent() {
    let mut doc = Document::new(17, 19).unwrap();
    dot(&mut doc, 16.5, 18.5, brush());
    assert_eq!(doc.tile_count(), 1);
    assert_eq!(doc.pixel(16, 18).alpha(), 1.0);
    assert_eq!(doc.pixel(17, 18).alpha(), 0.0);
    assert_eq!(doc.snapshot().pixel(16, 19).alpha(), 0.0);
    assert_eq!(
        doc.layer(1)
            .unwrap()
            .tiles()
            .next()
            .unwrap()
            .1
            .pixel(63, 63)
            .alpha(),
        0.0
    );
}

#[test]
fn end_point_budget_failure_rolls_back_preview_and_clears_transaction() {
    let options = DocumentOptions {
        max_document_tiles: 1,
        ..Default::default()
    };
    let mut doc = Document::with_options(128, 32, options).unwrap();
    let b = Brush {
        radius: 1.0,
        spacing: 1.0,
        ..brush()
    };
    doc.begin_stroke(b, point(62.6, 10.5)).unwrap();
    doc.stroke_to(point(63.2, 10.5)).unwrap();
    assert_eq!(doc.tile_count(), 1);
    assert!(matches!(doc.end_stroke(), Err(Error::ResourceLimit(_))));
    assert_eq!(doc.tile_count(), 0);
    assert_eq!(doc.history_depth(), (0, 0));
    assert!(!doc.stroke_active());
}

#[test]
fn deleting_layer_larger_than_memory_history_budget_is_undoable() {
    let options = DocumentOptions {
        max_history_bytes: 2 * TILE_BYTES + 256,
        ..Default::default()
    };
    let mut doc = Document::with_options(192, 32, options).unwrap();
    let top = doc.add_layer("large").unwrap();
    for x in [16.5, 80.5, 144.5] {
        dot(&mut doc, x, 10.5, brush());
    }
    let before = rgba(&doc);
    doc.remove_layer(top).unwrap();
    assert!(doc.layer(top).is_none());
    assert!(doc.history_bytes() <= options.max_history_bytes);
    doc.undo().unwrap();
    assert_eq!(rgba(&doc), before);
    assert!(doc.layer(top).is_some());
    doc.redo().unwrap();
    assert!(doc.layer(top).is_none());
}

#[test]
fn single_stroke_fills_canvas_beyond_old_64mib_history_threshold() {
    let mut doc = Document::new(1536, 1536).unwrap();
    let b = Brush {
        radius: 64.,
        spacing: 1.,
        color: Pixel::from_straight([0.2, 0.4, 0.6, 1.]).unwrap(),
        ..brush()
    };
    doc.begin_stroke(b, point(32., 32.)).unwrap();
    for row in 0..24 {
        for column in 0..24 {
            let column = if row % 2 == 0 { column } else { 23 - column };
            doc.stroke_to(point(
                f64::from(column) * 64. + 32.,
                f64::from(row) * 64. + 32.,
            ))
            .unwrap();
        }
    }
    doc.end_stroke().unwrap();
    assert_eq!(doc.tile_count(), 576);
    assert_eq!(doc.history_depth(), (1, 0));
    assert!(doc.history_bytes() <= DocumentOptions::default().max_history_bytes);
    for y in 0..1536 {
        for x in 0..1536 {
            assert_eq!(doc.pixel(x, y).alpha(), 1.);
        }
    }
    let painted = doc.pixel(1000, 1000).components().map(f32::to_bits);
    doc.undo().unwrap();
    assert_eq!(doc.tile_count(), 0);
    doc.redo().unwrap();
    assert_eq!(doc.tile_count(), 576);
    assert_eq!(
        doc.pixel(1000, 1000).components().map(f32::to_bits),
        painted
    );
}
