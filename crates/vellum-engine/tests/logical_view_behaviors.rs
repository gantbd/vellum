use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::{BytePosition, CoordinateBudget};
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::selection::*;
use vellum_engine::source::{Limits, Replacement};
fn limits() -> LogicalViewLimits {
    LogicalViewLimits {
        max_live_views: 8,
        max_issued_views: 16,
        max_total_selections: 64,
        normalization: SelectionBudget {
            max_input_selections: 16,
            max_output_selections: 16,
            max_work: 65536,
        },
        max_mapping_steps: 65536,
    }
}
fn owner() -> (SourceCoordinator, Recorder) {
    owner_with(limits())
}
fn owner_with(view_limits: LogicalViewLimits) -> (SourceCoordinator, Recorder) {
    let mut owner = SourceCoordinator::with_history_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 8,
                max_edit_payload_bytes: 4096,
            },
        },
        HistoryLimits {
            max_retained_groups: 16,
            max_journal_bytes: 65536,
            max_pins: 16,
        },
    );
    let mut recorder = Recorder::disabled();
    owner
        .configure_logical_views(view_limits, &mut recorder)
        .unwrap();
    (owner, recorder)
}
fn caret(at: usize, affinity: Affinity) -> Selection {
    Selection {
        anchor: BytePosition(at),
        head: BytePosition(at),
        affinity,
    }
}

#[test]
fn attach_detach_timing_preserves_success_stale_refusal_noop_and_source_peer_history() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, mut setup) = owner();
        let d = o.create(b"private target", &mut setup).unwrap();
        let other = o.create(b"private peer", &mut setup).unwrap();
        let peer = o.create_logical_view(&mut setup).unwrap();
        let ps = o.logical_view(&peer, &mut setup).unwrap().stamp;
        o.attach_logical_view(
            &peer,
            ps,
            &other,
            &[caret(1, Affinity::After)],
            0,
            &mut setup,
        )
        .unwrap();
        let peer_context = o
            .logical_view(&peer, &mut setup)
            .unwrap()
            .context()
            .unwrap();
        let view = o.create_logical_view(&mut setup).unwrap();
        let initial = o.logical_view(&view, &mut setup).unwrap().stamp;
        let generation = o.generation();
        let history = o.history_usage();
        let held = o.retention_usage();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        if mode == 3 {
            r.record(Observation {
                boundary: Boundary::Control,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        let attached = o
            .attach_logical_view(&view, initial, &d, &[caret(0, Affinity::After)], 0, &mut r)
            .unwrap();
        assert!(attached.changed);
        assert_eq!(
            o.attach_logical_view(
                &view,
                attached.stamp,
                &d,
                &[caret(0, Affinity::After)],
                0,
                &mut r
            ),
            Err(SelectionError::Unavailable)
        );
        assert!(!o.is_faulted());
        let detached = o
            .detach_logical_view(&view, attached.stamp, &mut r)
            .unwrap();
        assert!(detached.changed);
        assert!(
            !o.detach_logical_view(&view, detached.stamp, &mut r)
                .unwrap()
                .changed
        );
        assert_eq!(
            o.detach_logical_view(&view, initial, &mut r),
            Err(SelectionError::Stale)
        );
        assert!(!o.is_faulted());
        assert_eq!(o.generation(), generation + 2);
        assert_eq!(
            o.logical_view(&peer, &mut setup)
                .unwrap()
                .context()
                .unwrap(),
            peer_context
        );
        assert_eq!(
            o.read(&d.document, &mut setup).unwrap().source.text(),
            "private target"
        );
        assert_eq!(o.history_usage(), history);
        assert_eq!(o.retention_usage(), held);
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front())
            .filter(|e| e.boundary == Boundary::LogicalViewLifecycle)
            .collect();
        assert_eq!(events.len(), if mode == 1 || mode == 2 { 5 } else { 0 });
        if mode == 1 || mode == 2 {
            assert_eq!(
                events.iter().map(|e| e.disposition).collect::<Vec<_>>(),
                [
                    Disposition::Completed,
                    Disposition::Refused,
                    Disposition::Completed,
                    Disposition::Completed,
                    Disposition::Refused
                ]
            );
            for e in events {
                assert_eq!(e.service_time.is_some(), mode == 2);
                assert!(!format!("{e:?}").contains("private"));
            }
        }
    }
}

#[test]
fn registration_and_selection_caps_are_explicit_and_retired_view_origins_never_recycle() {
    let profile = LogicalViewLimits {
        max_live_views: 1,
        max_issued_views: 2,
        max_total_selections: 1,
        ..limits()
    };
    let (mut owner, mut r) = owner_with(profile);
    let source = owner.create(b"ab", &mut r).unwrap();
    assert!(!owner.configure_logical_views(profile, &mut r).unwrap());
    assert_eq!(
        owner.configure_logical_views(limits(), &mut r),
        Err(SelectionError::Unavailable)
    );
    let view = attach(&mut owner, &source, 0, Affinity::After, &mut r);
    let before = owner.logical_view_usage();
    assert_eq!(
        owner.create_logical_view(&mut r),
        Err(SelectionError::Limit)
    );
    assert_eq!(owner.logical_view_usage(), before);
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner.set_logical_selections(
            &context,
            &[caret(0, Affinity::Before), caret(1, Affinity::After)],
            0,
            &mut r
        ),
        Err(SelectionError::Limit)
    );
    assert_eq!(owner.generation(), generation);
    assert_eq!(owner.logical_view_usage(), before);
    owner
        .retire_logical_view(&view, context.stamp, &mut r)
        .unwrap();
    let fresh = owner.create_logical_view(&mut r).unwrap();
    assert_ne!(fresh, view);
    let stamp = owner.logical_view(&fresh, &mut r).unwrap().stamp;
    owner.retire_logical_view(&fresh, stamp, &mut r).unwrap();
    assert_eq!(
        owner.create_logical_view(&mut r),
        Err(SelectionError::Limit)
    );
    assert_eq!(owner.logical_view_usage().issued_lifetime, 2);
    let (other, mut recorder) = crate::owner();
    assert_eq!(
        other.logical_view(&view, &mut recorder).err(),
        Some(SelectionError::Unavailable)
    );
}

#[test]
fn endpoints_preserve_affinity_and_crlf_interior_or_scan_refusal_does_not_move_any_selection() {
    let (mut owner, mut r) = owner();
    let source = owner.create(b"ab\r\ncd\n", &mut r).unwrap();
    let view = attach(&mut owner, &source, 5, Affinity::After, &mut r);
    for (endpoint, at) in [
        (LogicalEndpointV1::LineStart, 4),
        (LogicalEndpointV1::LineEnd, 6),
        (LogicalEndpointV1::DocumentStart, 0),
        (LogicalEndpointV1::DocumentEnd, 7),
    ] {
        let context = owner
            .logical_view(&view, &mut r)
            .unwrap()
            .context()
            .unwrap();
        owner
            .set_logical_selections(&context, &[caret(5, Affinity::After)], 0, &mut r)
            .unwrap();
        let context = owner
            .logical_view(&view, &mut r)
            .unwrap()
            .context()
            .unwrap();
        owner
            .move_logical_selections(
                &context,
                endpoint,
                false,
                CoordinateBudget { max_scan_bytes: 64 },
                &mut r,
            )
            .unwrap();
        assert_eq!(
            owner
                .logical_view(&view, &mut r)
                .unwrap()
                .selections
                .selections(),
            &[caret(at, Affinity::After)]
        );
    }
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    owner
        .set_logical_selections(&context, &[caret(3, Affinity::Before)], 0, &mut r)
        .unwrap();
    let snapshot = owner.logical_view(&view, &mut r).unwrap();
    let context = snapshot.context().unwrap();
    let generation = owner.generation();
    assert_eq!(
        owner.move_logical_selections(
            &context,
            LogicalEndpointV1::LineStart,
            false,
            CoordinateBudget { max_scan_bytes: 64 },
            &mut r
        ),
        Err(SelectionError::Coordinate(
            vellum_engine::coordinates::CoordinateError::UnavailableProjection
        ))
    );
    assert_eq!(owner.generation(), generation);
    assert_eq!(
        owner.logical_view(&view, &mut r).unwrap().selections,
        snapshot.selections
    );
    assert_eq!(
        owner.move_logical_selections(
            &context,
            LogicalEndpointV1::LineEnd,
            false,
            CoordinateBudget { max_scan_bytes: 0 },
            &mut r
        ),
        Err(SelectionError::Coordinate(
            vellum_engine::coordinates::CoordinateError::Limit
        ))
    );
    owner
        .set_logical_selections(
            &context,
            &[caret(1, Affinity::Before), caret(1, Affinity::After)],
            0,
            &mut r,
        )
        .unwrap();
    let generation = owner.generation();
    let edits = [DocumentEdits {
        expected: source.clone(),
        edits: vec![
            Replacement {
                start: 1,
                end: 1,
                expected: "".into(),
                inserted: "X".into(),
            },
            Replacement {
                start: 1,
                end: 1,
                expected: "".into(),
                inserted: "Y".into(),
            },
        ],
    }];
    assert_eq!(
        owner.prepare(&edits, &[], &mut r).err(),
        Some(CoordinatorError::Source(
            vellum_engine::source::SourceError::AmbiguousEdits
        ))
    );
    assert_eq!(owner.generation(), generation);
}

#[test]
fn diagnostics_off_available_or_full_preserve_view_mapping_and_order_preparation_before_publication()
 {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(256), Some(1)] {
        let (mut owner, _) = owner();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let source = owner.create(b"PRIVATE", &mut r).unwrap();
        let view = attach(&mut owner, &source, 1, Affinity::After, &mut r);
        let context = owner
            .logical_view(&view, &mut r)
            .unwrap()
            .context()
            .unwrap();
        owner
            .move_logical_selections(
                &context,
                LogicalEndpointV1::LineEnd,
                false,
                CoordinateBudget { max_scan_bytes: 64 },
                &mut r,
            )
            .unwrap();
        let plan = edit(&owner, &source, 0, 0, "", "X", &mut r);
        owner.commit(plan, &mut r).unwrap();
        let snapshot = owner.logical_view(&view, &mut r).unwrap();
        assert_eq!(
            snapshot.selections.selections(),
            &[caret(8, Affinity::After)]
        );
        assert_eq!(snapshot.source.unwrap().revision, 1);
        let mut observations = Vec::new();
        while let Some(observation) = r.pop_front() {
            observations.push(observation);
        }
        if capacity == Some(256) {
            let prepared = observations
                .iter()
                .position(|o| o.boundary == Boundary::LogicalSelectionPreparation)
                .unwrap();
            let published = observations
                .iter()
                .position(|o| o.boundary == Boundary::LogicalSelectionPublication)
                .unwrap();
            assert!(prepared < published);
            assert!(
                observations
                    .iter()
                    .any(|o| o.boundary == Boundary::LogicalSelectionMapping)
            );
        }
        assert!(!format!("{observations:?}").contains("PRIVATE"));
        assert!(observations.iter().all(|o| o.service_time.is_none()));
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
fn attach(
    owner: &mut SourceCoordinator,
    source: &DocumentVersion,
    at: usize,
    affinity: Affinity,
    r: &mut Recorder,
) -> LogicalViewId {
    let id = owner.create_logical_view(r).unwrap();
    let born = owner.logical_view(&id, r).unwrap();
    assert!(born.source.is_none());
    assert!(born.selections.selections().is_empty());
    owner
        .attach_logical_view(&id, born.stamp, source, &[caret(at, affinity)], 0, r)
        .unwrap();
    id
}
fn edit(
    owner: &SourceCoordinator,
    source: &DocumentVersion,
    start: usize,
    end: usize,
    expected: &str,
    inserted: &str,
    r: &mut Recorder,
) -> PreparedTransaction {
    owner
        .prepare(
            &[DocumentEdits {
                expected: source.clone(),
                edits: vec![Replacement {
                    start,
                    end,
                    expected: expected.into(),
                    inserted: inserted.into(),
                }],
            }],
            &[],
            r,
        )
        .unwrap()
}

#[test]
fn registered_view_selection_and_endpoint_changes_are_headless_and_not_source_history() {
    let (mut owner, mut r) = owner();
    let source = owner.create(b"ab\r\ncd", &mut r).unwrap();
    owner.set_read_only(&source.document, true, &mut r).unwrap();
    let view = attach(&mut owner, &source, 1, Affinity::After, &mut r);
    let context = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    let update = owner
        .move_logical_selections(
            &context,
            LogicalEndpointV1::LineEnd,
            true,
            CoordinateBudget { max_scan_bytes: 64 },
            &mut r,
        )
        .unwrap();
    assert!(update.changed);
    let selected = owner.logical_view(&view, &mut r).unwrap();
    assert_eq!(
        selected.selections.selections(),
        &[Selection {
            anchor: BytePosition(1),
            head: BytePosition(2),
            affinity: Affinity::After
        }]
    );
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().version,
        source
    );
    assert_eq!(owner.history_usage().retained_groups, 0);
    assert_eq!(
        owner.set_logical_selections(&context, &[caret(0, Affinity::Before)], 0, &mut r),
        Err(SelectionError::Stale)
    );
    let generation = owner.generation();
    let context = selected.context().unwrap();
    assert!(
        !owner
            .set_logical_selections(&context, selected.selections.selections(), 0, &mut r)
            .unwrap()
            .changed
    );
    assert_eq!(owner.generation(), generation);
}

#[test]
fn source_publication_maps_all_views_and_undo_maps_current_not_historical_cursor_state() {
    let (mut owner, mut r) = owner();
    let source = owner.create(b"abcd", &mut r).unwrap();
    let a = attach(&mut owner, &source, 1, Affinity::Before, &mut r);
    let b = attach(&mut owner, &source, 3, Affinity::After, &mut r);
    let prepared = edit(&owner, &source, 2, 2, "", "X", &mut r);
    let outcome = owner.commit(prepared, &mut r).unwrap();
    let current = outcome.changes[0].after.clone();
    assert_eq!(
        owner.logical_view(&a, &mut r).unwrap().source,
        Some(current.clone())
    );
    assert_eq!(
        owner
            .logical_view(&b, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[caret(4, Affinity::After)]
    );
    let group = outcome.history_group.unwrap();
    let old_reversal = owner
        .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
        .unwrap();
    let context = owner.logical_view(&b, &mut r).unwrap().context().unwrap();
    owner
        .set_logical_selections(&context, &[caret(2, Affinity::After)], 0, &mut r)
        .unwrap();
    assert_eq!(
        owner.commit(old_reversal, &mut r),
        Err(CoordinatorError::Stale)
    );
    let reversal = owner
        .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
        .unwrap();
    owner.commit(reversal, &mut r).unwrap();
    let after = owner.logical_view(&b, &mut r).unwrap();
    assert_eq!(after.selections.selections(), &[caret(2, Affinity::After)]);
    assert_eq!(after.source.as_ref().unwrap().revision, 2);
}

#[test]
fn new_or_changed_affected_views_refuse_an_old_source_plan_but_unrelated_views_do_not() {
    let (mut owner, mut r) = owner();
    let source = owner.create(b"abc", &mut r).unwrap();
    let plan = edit(&owner, &source, 0, 0, "", "X", &mut r);
    let view = attach(&mut owner, &source, 1, Affinity::After, &mut r);
    assert_eq!(owner.commit(plan, &mut r), Err(CoordinatorError::Stale));
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().source.text(),
        "abc"
    );
    let peer = owner.create(b"peer", &mut r).unwrap();
    let peer_view = attach(&mut owner, &peer, 0, Affinity::Before, &mut r);
    let plan = edit(&owner, &source, 0, 0, "", "X", &mut r);
    let context = owner
        .logical_view(&peer_view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    owner
        .set_logical_selections(&context, &[caret(1, Affinity::Before)], 0, &mut r)
        .unwrap();
    owner.commit(plan, &mut r).unwrap();
    assert_eq!(
        owner
            .logical_view(&view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[caret(2, Affinity::After)]
    );
}

#[test]
fn equal_byte_replacement_can_publish_selection_mapping_without_a_source_revision_or_undo_group() {
    let (mut owner, mut r) = owner();
    let source = owner.create(b"abc", &mut r).unwrap();
    let view = attach(&mut owner, &source, 1, Affinity::After, &mut r);
    let generation = owner.generation();
    let plan = edit(&owner, &source, 0, 3, "abc", "abc", &mut r);
    let outcome = owner.commit(plan, &mut r).unwrap();
    assert_eq!(outcome.generation, generation + 1);
    assert!(outcome.changes.is_empty());
    assert!(outcome.history_group.is_none());
    assert!(outcome.operation.is_some());
    assert_eq!(
        owner
            .logical_view(&view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[caret(3, Affinity::After)]
    );
    assert_eq!(
        owner.read(&source.document, &mut r).unwrap().version,
        source
    );
}

#[test]
fn attachment_aba_disposal_and_container_retirement_never_resurrect_or_dispose_a_source() {
    let (mut owner, mut r) = owner();
    let source = owner.create(b"abc", &mut r).unwrap();
    let view = attach(&mut owner, &source, 1, Affinity::Before, &mut r);
    let old = owner
        .logical_view(&view, &mut r)
        .unwrap()
        .context()
        .unwrap();
    owner.detach_logical_view(&view, old.stamp, &mut r).unwrap();
    let detached = owner.logical_view(&view, &mut r).unwrap();
    owner
        .attach_logical_view(
            &view,
            detached.stamp,
            &source,
            &[caret(1, Affinity::Before)],
            0,
            &mut r,
        )
        .unwrap();
    assert_eq!(
        owner.set_logical_selections(&old, &[caret(2, Affinity::Before)], 0, &mut r),
        Err(SelectionError::Stale)
    );
    let plan = owner.prepare_retained_disposal(&source, 0, &mut r).unwrap();
    owner.commit_retained_disposal(plan, &mut r).unwrap();
    let detached = owner.logical_view(&view, &mut r).unwrap();
    assert!(detached.source.is_none());
    assert!(detached.selections.selections().is_empty());
    owner
        .retire_logical_view(&view, detached.stamp, &mut r)
        .unwrap();
    assert_eq!(
        owner.logical_view(&view, &mut r).err(),
        Some(SelectionError::Unavailable)
    );
    let fresh = owner.create_logical_view(&mut r).unwrap();
    assert_ne!(fresh, view);
    let peer = owner.create(b"peer", &mut r).unwrap();
    let peer_view = attach(&mut owner, &peer, 0, Affinity::After, &mut r);
    let stamp = owner.logical_view(&peer_view, &mut r).unwrap().stamp;
    owner
        .retire_logical_view(&peer_view, stamp, &mut r)
        .unwrap();
    assert!(owner.read(&peer.document, &mut r).is_ok());
}
