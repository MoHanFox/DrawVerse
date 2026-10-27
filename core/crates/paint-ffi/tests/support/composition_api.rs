use super::*;

fn caps(h: &Harness) -> PaintCompositionCapabilities {
    let mut caps = dto!(PaintCompositionCapabilities);
    assert_eq!(
        unsafe { paint_core_composition_capabilities(h.core, 1, &mut caps) },
        PAINT_OK
    );
    caps
}
fn request(h: &Harness, id: u64, epoch: u64) -> PaintCompositionRequest {
    let c = caps(h);
    let info = h.info();
    PaintCompositionRequest {
        version: 1,
        features: c.features,
        revision: info.revision,
        request_id: id,
        device_epoch: epoch,
        max_pinned_bytes: c.max_pinned_bytes,
        max_metadata_bytes: c.max_metadata_bytes,
        max_nodes: c.max_nodes,
        max_tiles: c.max_tiles,
        viewport: PaintViewport {
            view_id: 0,
            enabled: 1,
            width: 64.,
            height: 64.,
            pixel_width: 64,
            pixel_height: 64,
            document_generation: info.document_generation,
            ..dto!(PaintViewport)
        },
        ..dto!(PaintCompositionRequest)
    }
}
fn submit(h: &Harness, desc: &PaintCompositionRequest) -> (PaintStatus, u64) {
    let mut id = 99;
    let status = unsafe { paint_session_request_composition(h.core, h.session, desc, &mut id) };
    (status, id)
}
fn ready(h: &Harness, id: u64) -> PaintCompositionInfo {
    wait(|| unsafe {
        let mut info = dto!(PaintCompositionInfo);
        assert_eq!(
            paint_session_composition_info(h.core, h.session, id, &mut info),
            PAINT_OK
        );
        (info.state != PAINT_COMPOSITION_PENDING).then_some(info)
    })
}
fn acquire(h: &Harness, id: u64, epoch: u64) -> PaintCompositionInfo {
    let (status, lease) = submit(h, &request(h, id, epoch));
    assert_eq!(status, PAINT_OK);
    let info = ready(h, lease);
    assert_eq!(info.state, PAINT_COMPOSITION_READY);
    assert_eq!(info.status, PAINT_OK);
    info
}
fn release(h: &Harness, info: PaintCompositionInfo) {
    assert_eq!(
        unsafe { paint_session_release_composition(h.core, h.session, info.key.lease_id) },
        PAINT_OK
    );
}
fn tile(h: &Harness, key: PaintCompositionKey, index: u32) -> PaintCompositionTile {
    let mut tile = dto!(PaintCompositionTile);
    assert_eq!(
        unsafe { paint_session_composition_tile(h.core, h.session, &key, index, &mut tile) },
        PAINT_OK
    );
    tile
}
fn bytes(h: &Harness, key: PaintCompositionKey, index: u32) -> Vec<u8> {
    let mut bytes = vec![0; 65536];
    let mut required = 0;
    assert_eq!(
        unsafe {
            paint_session_read_composition_tile(
                h.core,
                h.session,
                &key,
                index,
                bytes.as_mut_ptr(),
                bytes.len() as u64,
                &mut required,
            )
        },
        PAINT_OK
    );
    assert_eq!(required, bytes.len() as u64);
    bytes
}

#[test]
fn composition_protocol_capabilities_validation_and_capture_never_change_document() {
    let h = Harness::new(64, 64);
    let before = h.wait(h.stroke(24., 24.));
    let c = caps(&h);
    assert_eq!(c.tile_edge, 64);
    assert_eq!(c.max_read_bytes, 65536);
    assert_eq!(c.max_leases_per_session, 2);
    unsafe {
        let mut out = dto!(PaintCompositionCapabilities);
        assert_eq!(
            paint_core_composition_capabilities(h.core, 2, &mut out),
            PAINT_UNSUPPORTED
        );
        assert_eq!(out.features, 0);
    }
    let desc = request(&h, 1, 1);
    for bad in [
        PaintCompositionRequest {
            struct_size: 0,
            ..desc
        },
        PaintCompositionRequest {
            device_epoch: 0,
            ..desc
        },
        PaintCompositionRequest {
            reserved: [1, 0],
            ..desc
        },
        PaintCompositionRequest {
            max_tiles: c.max_tiles + 1,
            ..desc
        },
        PaintCompositionRequest {
            viewport: PaintViewport {
                struct_size: 0,
                ..desc.viewport
            },
            ..desc
        },
    ] {
        assert_eq!(submit(&h, &bad), (PAINT_INVALID_ARGUMENT, 0));
    }
    for bad in [
        PaintCompositionRequest { version: 2, ..desc },
        PaintCompositionRequest {
            features: desc.features | 8,
            ..desc
        },
    ] {
        assert_eq!(submit(&h, &bad), (PAINT_UNSUPPORTED, 0));
    }
    assert_eq!(
        submit(
            &h,
            &PaintCompositionRequest {
                revision: before.revision - 1,
                ..desc
            }
        ),
        (PAINT_BUSY, 0)
    );
    let p = acquire(&h, 1, 1);
    assert_eq!(p.key.revision, before.revision);
    assert_eq!(p.key.document_generation, before.document_generation);
    assert_eq!(p.key.publication, before.publication);
    assert!(p.budget_bytes >= p.pinned_bytes + p.metadata_bytes);
    assert_eq!(submit(&h, &desc), (PAINT_BUSY, 0));
    let after = h.info();
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.completed_sequence, before.completed_sequence);
    assert_eq!(
        (after.undo_depth, after.redo_depth),
        (before.undo_depth, before.redo_depth)
    );
    assert_eq!(after.last_error_status, PAINT_OK);
    release(&h, p);
}

#[test]
fn composition_tile_buffers_are_atomic_and_old_leases_keep_immutable_pixels() {
    let h = Harness::new(64, 64);
    h.wait(h.stroke(24., 24.));
    let a = acquire(&h, 1, 1);
    assert_eq!(a.tile_count, 1);
    let content = tile(&h, a.key, 0);
    let original = bytes(&h, a.key, 0);
    assert_eq!(content.required_bytes, 65536);
    assert_eq!(content.format, PAINT_COMPOSITION_LINEAR_RGBA32F as u32);
    let at = (24 * 64 + 24) * 16;
    assert_eq!(
        f32::from_le_bytes(original[at..at + 4].try_into().unwrap()),
        1.
    );
    unsafe {
        let mut required = 99;
        assert_eq!(
            paint_session_read_composition_tile(
                h.core,
                h.session,
                &a.key,
                0,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(required, 65536);
        let mut sentinel = [0xCDu8; 32];
        assert_eq!(
            paint_session_read_composition_tile(
                h.core,
                h.session,
                &a.key,
                0,
                sentinel.as_mut_ptr(),
                32,
                &mut required
            ),
            PAINT_BUFFER_TOO_SMALL
        );
        assert_eq!(sentinel, [0xCD; 32]);
        assert_eq!(
            paint_session_read_composition_tile(
                h.core,
                h.session,
                &a.key,
                99,
                ptr::null_mut(),
                0,
                &mut required
            ),
            PAINT_NOT_FOUND
        );
        assert_eq!(required, 0);
        assert_eq!(
            paint_session_read_composition_tile(
                h.core,
                h.session,
                &a.key,
                0,
                sentinel.as_mut_ptr(),
                u64::MAX,
                &mut required
            ),
            PAINT_INVALID_ARGUMENT
        );
        assert_eq!(required, 0);
        let mut node = dto!(PaintCompositionNode);
        assert_eq!(
            paint_session_composition_node(h.core, h.session, &a.key, 0, &mut node),
            PAINT_OK
        );
        assert_eq!(node.mask_index, u32::MAX);
        assert_eq!(node.first_tile, 0);
        assert_eq!(node.tile_count, 1);
        assert_eq!(
            paint_session_composition_node(h.core, h.session, &a.key, 1, &mut node),
            PAINT_NOT_FOUND
        );
    }
    let b = acquire(&h, 2, 1);
    assert_eq!(tile(&h, b.key, 0).content_id, content.content_id);
    release(&h, b);
    h.wait(h.stroke(40., 24.));
    let c = acquire(&h, 3, 1);
    assert_ne!(tile(&h, c.key, 0).content_id, content.content_id);
    assert_eq!(bytes(&h, a.key, 0), original);
    assert_ne!(bytes(&h, c.key, 0), original);
    release(&h, a);
    release(&h, c);
}

#[test]
fn composition_budget_and_unsupported_failures_are_explicit_without_document_error() {
    let h = Harness::new(64, 64);
    let before = h.wait(h.stroke(24., 24.));
    let (status, id) = submit(
        &h,
        &PaintCompositionRequest {
            max_pinned_bytes: 0,
            ..request(&h, 1, 1)
        },
    );
    assert_eq!(status, PAINT_OK);
    let failed = ready(&h, id);
    assert_eq!(failed.state, PAINT_COMPOSITION_FAILED);
    assert_eq!(failed.status, PAINT_LIMIT_EXCEEDED);
    assert_eq!(failed.budget_bytes, 0);
    assert_eq!(h.info().last_error_status, PAINT_OK);
    let good = acquire(&h, 2, 1);
    assert_eq!(submit(&h, &request(&h, 3, 1)), (PAINT_LIMIT_EXCEEDED, 0));
    release(&h, failed);
    release(&h, good);
    unsafe {
        let appearance = PaintLayerAppearance {
            fill: 1.,
            blend_mode: paint_core::BlendMode::Multiply as u32,
            ..dto!(PaintLayerAppearance)
        };
        let mut sequence = 0;
        assert_eq!(
            paint_session_set_layer_appearance(
                h.core,
                h.session,
                before.active_layer_id,
                &appearance,
                &mut sequence
            ),
            PAINT_OK
        );
        h.wait(sequence);
    }
    let (status, id) = submit(&h, &request(&h, 4, 1));
    assert_eq!(status, PAINT_OK);
    let failed = ready(&h, id);
    assert_eq!(failed.state, PAINT_COMPOSITION_FAILED);
    assert_eq!(failed.status, PAINT_UNSUPPORTED);
    assert_eq!(h.info().last_error_status, PAINT_OK);
    release(&h, failed);
}

#[test]
fn composition_new_device_new_document_cancel_and_cross_core_reject_stale_keys() {
    let h = Harness::new(64, 64);
    h.wait(h.stroke(24., 24.));
    let old = acquire(&h, 1, 1);
    let new = acquire(&h, 2, 2);
    let mut out = dto!(PaintCompositionInfo);
    unsafe {
        assert_eq!(
            paint_session_composition_info(h.core, h.session, old.key.lease_id, &mut out),
            PAINT_CANCELLED
        );
        assert_eq!(submit(&h, &request(&h, 3, 1)), (PAINT_BUSY, 0));
        let mut node = dto!(PaintCompositionNode);
        for changed in [
            PaintCompositionKey {
                request_id: 99,
                ..new.key
            },
            PaintCompositionKey {
                revision: new.key.revision + 1,
                ..new.key
            },
            PaintCompositionKey {
                device_epoch: 1,
                ..new.key
            },
            PaintCompositionKey {
                view_id: 1,
                ..new.key
            },
        ] {
            assert_eq!(
                paint_session_composition_node(h.core, h.session, &changed, 0, &mut node),
                PAINT_BUSY
            );
        }
        let other = Harness::new(64, 64);
        assert_eq!(
            paint_session_composition_node(other.core, h.session, &new.key, 0, &mut node),
            PAINT_INVALID_HANDLE
        );
        assert_eq!(
            paint_session_composition_node(other.core, other.session, &new.key, 0, &mut node),
            PAINT_CANCELLED
        );
    }
    h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_NEW_DOCUMENT,
        width: 64,
        height: 64,
        ..dto!(PaintCommand)
    }));
    unsafe {
        let mut sentinel = vec![0xAD; 65536];
        let mut required = 99;
        assert_eq!(
            paint_session_read_composition_tile(
                h.core,
                h.session,
                &new.key,
                0,
                sentinel.as_mut_ptr(),
                65536,
                &mut required
            ),
            PAINT_CANCELLED
        );
        assert!(sentinel.iter().all(|b| *b == 0xAD));
        assert_eq!(required, 0);
    }
    let final_plan = acquire(&h, 4, 2);
    release(&h, final_plan);
}

#[test]
fn composition_display_flood_never_consumes_pen_sequences_or_discards_strokes() {
    let h = Harness::new(192, 48);
    let mut leases = Vec::new();
    let mut ends = Vec::new();
    let mut id = 0;
    for stroke in 0..24 {
        h.submit(PaintCommand {
            kind: PAINT_COMMAND_BEGIN_STROKE,
            stroke: PaintStrokeDesc {
                mode: PAINT_MODE_PAINT,
                radius: 2.,
                opacity: 1.,
                spacing: 0.15,
                linear_rgba: [1., 0., 0., 1.],
                ..dto!(PaintStrokeDesc)
            },
            point: PaintPoint {
                x: 4. + stroke as f64 * 7.,
                y: 24.,
                pressure: 1.,
                tool: PAINT_TOOL_MOUSE,
                ..dto!(PaintPoint)
            },
            ..dto!(PaintCommand)
        });
        for _ in 0..12 {
            id += 1;
            for lease in leases.drain(..) {
                let status = unsafe { paint_session_release_composition(h.core, h.session, lease) };
                assert!(status == PAINT_OK || status == PAINT_CANCELLED);
            }
            let (status, lease) = submit(&h, &request(&h, id, 1));
            assert!(status == PAINT_OK || status == PAINT_BUSY || status == PAINT_LIMIT_EXCEEDED);
            if status == PAINT_OK {
                leases.push(lease);
            }
        }
        ends.push(h.submit(PaintCommand {
            kind: PAINT_COMMAND_END_STROKE,
            ..dto!(PaintCommand)
        }));
    }
    let info = h.wait(*ends.last().unwrap());
    assert_eq!(info.undo_depth, 24);
    assert_eq!(info.last_error_status, PAINT_OK);
    assert_eq!(ends, (1..=24).map(|n| n * 2).collect::<Vec<_>>());
    for lease in leases {
        let status = unsafe { paint_session_release_composition(h.core, h.session, lease) };
        assert!(status == PAINT_OK || status == PAINT_CANCELLED);
    }
    let mut count = 0;
    let mut receipts = [0; 24];
    assert_eq!(
        unsafe {
            paint_session_committed_strokes(
                h.core,
                h.session,
                info.publication,
                0,
                receipts.as_mut_ptr(),
                24,
                &mut count,
            )
        },
        PAINT_OK
    );
    assert_eq!(count, 24);
    assert_eq!(receipts.as_slice(), ends.as_slice());
}

#[test]
fn composition_parallel_tile_reads_and_cancel_publish_only_complete_or_untouched_buffers() {
    let h = Harness::new(64, 64);
    h.wait(h.stroke(24., 24.));
    let info = acquire(&h, 1, 1);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(9));
    thread::scope(|scope| {
        let mut readers = Vec::new();
        for _ in 0..8 {
            let barrier = barrier.clone();
            let core = h.core as usize;
            let session = h.session as usize;
            let key = info.key;
            readers.push(scope.spawn(move || {
                let mut pixels = vec![0xAB; 65536];
                let mut required = 0;
                barrier.wait();
                let status = unsafe {
                    paint_session_read_composition_tile(
                        core as *mut PaintCore,
                        session as *mut PaintSession,
                        &key,
                        0,
                        pixels.as_mut_ptr(),
                        65536,
                        &mut required,
                    )
                };
                if status == PAINT_OK {
                    assert_eq!(required, 65536);
                    assert_eq!(
                        &pixels[(24 * 64 + 24) * 16..(24 * 64 + 24) * 16 + 4],
                        &1f32.to_le_bytes()
                    );
                } else {
                    assert_eq!(status, PAINT_CANCELLED);
                    assert!(pixels.iter().all(|b| *b == 0xAB));
                }
            }));
        }
        barrier.wait();
        release(&h, info);
        for reader in readers {
            reader.join().unwrap();
        }
    });
    let mut usage = dto!(PaintCompositionUsage);
    assert_eq!(
        unsafe { paint_core_composition_usage(h.core, &mut usage) },
        PAINT_OK
    );
    let limits = caps(&h);
    assert!(usage.plan_bytes <= limits.process_plan_bytes);
    assert!(usage.peak_read_bytes <= limits.process_read_bytes);
}

#[test]
fn composition_active_brush_capture_is_immutable_and_never_commits_preview_history() {
    let h = Harness::new(64, 64);
    let mut brush = dto!(PaintStrokeDesc);
    brush.mode = PAINT_MODE_PAINT;
    brush.radius = 8.;
    brush.opacity = 1.;
    brush.spacing = 0.15;
    brush.linear_rgba = [1., 0., 0., 1.];
    let begin = h.submit(PaintCommand {
        kind: PAINT_COMMAND_BEGIN_STROKE,
        stroke: brush,
        point: PaintPoint {
            x: 16.,
            y: 16.,
            pressure: 1.,
            tool: PAINT_TOOL_MOUSE,
            ..dto!(PaintPoint)
        },
        ..dto!(PaintCommand)
    });
    let active = h.wait(begin);
    assert_eq!(active.stroke_active, 1);
    let plan = acquire(&h, 1, 1);
    let original = bytes(&h, plan.key, 0);
    assert_eq!(h.info().undo_depth, active.undo_depth);
    h.wait(h.submit(PaintCommand {
        kind: PAINT_COMMAND_STROKE_TO,
        point: PaintPoint {
            x: 48.,
            y: 48.,
            pressure: 1.,
            tool: PAINT_TOOL_MOUSE,
            ..dto!(PaintPoint)
        },
        ..dto!(PaintCommand)
    }));
    assert_eq!(bytes(&h, plan.key, 0), original);
    let advanced = acquire(&h, 2, 1);
    assert_ne!(bytes(&h, advanced.key, 0), original);
    assert_eq!(h.info().undo_depth, active.undo_depth);
    let end = h.submit(PaintCommand {
        kind: PAINT_COMMAND_END_STROKE,
        ..dto!(PaintCommand)
    });
    let final_state = h.wait(end);
    assert_eq!(final_state.undo_depth, active.undo_depth + 1);
    assert_eq!(final_state.completed_sequence, begin + 2);
    assert_eq!(bytes(&h, plan.key, 0), original);
    release(&h, plan);
    release(&h, advanced);
}
