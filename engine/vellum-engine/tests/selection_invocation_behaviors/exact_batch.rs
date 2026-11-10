//! Explicit batches retain source intent, not admission-time view selections.
use super::*;
use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
use vellum_engine::source::Replacement;
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "host".into(),
        engine: "native".into(),
        recipient: "agent".into(),
        purpose: "edit".into(),
        destination: "engine".into(),
        profile: BOUNDED_EXPLICIT_SOURCE_BATCH_PROFILE_V1.into(),
        continuity: "session".into(),
    }
}
fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 65536,
        max_nodes: 4096,
        max_depth: 16,
    }
}
fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 16,
            max_work: 65536,
        },
    }
}
fn scope_budget() -> SourceScopeBudget {
    SourceScopeBudget {
        max_scopes: 8,
        max_records: 32,
        max_work: 65536,
    }
}
fn write(
    v: &DocumentVersion,
    start: usize,
    end: usize,
    expected: &str,
    inserted: &str,
) -> DocumentEdits {
    DocumentEdits {
        expected: v.clone(),
        edits: vec![Replacement {
            start,
            end,
            expected: expected.into(),
            inserted: inserted.into(),
        }],
    }
}
fn scope(v: &DocumentVersion, bytes: usize) -> SourceOperationScope {
    SourceOperationScope {
        expected: v.clone(),
        replacement_ranges: vec![ByteRange {
            start: 0,
            end: bytes,
        }],
        insertion_sites: vec![InsertionSiteRange {
            first: 0,
            last: bytes,
        }],
    }
}
fn normalize<'h>(
    owner: &SourceCoordinator,
    id: &'h InvocationId,
    sources: &[DocumentEdits],
    reads: &ReadDependencyRequestV3,
    scopes: &[SourceOperationScope],
    r: &mut Recorder,
) -> NormalizedExactBatchArgumentsV1<'h> {
    with_arguments(id, sources, reads, scopes, |args| {
        owner
            .normalize_exact_batch_arguments_v1(args, &context(), limits(), r)
            .unwrap()
    })
}
fn with_arguments<'h, T>(
    id: &'h InvocationId,
    sources: &[DocumentEdits],
    reads: &ReadDependencyRequestV3,
    scopes: &[SourceOperationScope],
    consume: impl FnOnce(ExactBatchEnvelopeValueV1<'_, 'h>) -> T,
) -> T {
    use ExactBatchEnvelopeValueV1 as V;
    let args = [
        ("contract", V::Data(D::Text("exact-source-batch/1"))),
        ("operation", V::Data(D::Text("exact-batch"))),
        ("invocation", V::Invocation(id)),
        (
            "profile",
            V::Data(D::Text(BOUNDED_EXPLICIT_SOURCE_BATCH_PROFILE_V1)),
        ),
        ("sources", V::Sources(sources)),
        ("reads", V::Reads(reads)),
        ("scopes", V::Scopes(scopes, scope_budget())),
    ];
    consume(V::Object(&args))
}

#[test]
fn two_document_batch_maps_current_views_and_keeps_one_history_group() {
    let (mut owner, view, mut r) = fixture_with_intent_limit(65536);
    let other = owner.create(b"xyz", &mut r).unwrap();
    let id = owner.issue_invocation(&mut r).unwrap();
    let writes = [
        write(&view.source, 0, 1, "a", "XY"),
        write(&other, 1, 2, "y", "Y"),
    ];
    let scopes = [scope(&view.source, 8), scope(&other, 3)];
    let q = normalize(&owner, &id, &writes, &reads(), &scopes, &mut r);
    assert_eq!(
        owner.submit_exact_batch_arguments_v1(&q, &mut r),
        Ok(InvocationState::Pending)
    );
    let move_id = owner.issue_invocation(&mut r).unwrap();
    owner
        .submit_selection_command(
            &move_id,
            &view,
            &SelectionCommandV1::Set {
                selections: vec![caret(7)],
                primary: 0,
            },
            &mut r,
        )
        .unwrap();
    owner.run(&move_id, &mut r).unwrap();
    let peer = owner.create_logical_view(&mut r).unwrap();
    let stamp = owner.logical_view(&peer, &mut r).unwrap().stamp;
    owner
        .attach_logical_view(&peer, stamp, &view.source, &[caret(6)], 0, &mut r)
        .unwrap();
    owner.run(&id, &mut r).unwrap();
    let InvocationOutcome::Committed(outcome) = owner.invocation_outcome(&id, &mut r).unwrap()
    else {
        panic!("batch was not committed")
    };
    assert_eq!(outcome.changes.len(), 2);
    assert!(outcome.history_group.is_some());
    assert_eq!(owner.history_usage().retained_groups, 1);
    assert_eq!(
        owner
            .read(&view.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "XYbc\r\ndef"
    );
    assert_eq!(
        owner.read(&other.document, &mut r).unwrap().source.text(),
        "xYz"
    );
    assert_eq!(
        owner
            .logical_view(&view.view, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[caret(8)]
    );
    assert_eq!(
        owner
            .logical_view(&peer, &mut r)
            .unwrap()
            .selections
            .selections(),
        &[caret(7)]
    );
    assert_eq!(
        owner.submit_exact_batch_arguments_v1(&q, &mut r),
        Ok(InvocationState::Finished)
    );
    owner.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        owner.submit_exact_batch_arguments_v1(&q, &mut r),
        Ok(InvocationState::Finished)
    );
}

#[test]
fn grouped_partitioned_and_permuted_edits_share_only_their_original_exact_batch_ticket() {
    let (mut owner, view, mut r) = fixture_with_intent_limit(65536);
    let id = owner.issue_invocation(&mut r).unwrap();
    let partitioned = [
        write(&view.source, 2, 3, "c", "Z"),
        write(&view.source, 0, 1, "a", "X"),
    ];
    let scopes = [scope(&view.source, 8)];
    let q = normalize(&owner, &id, &partitioned, &reads(), &scopes, &mut r);
    owner.submit_exact_batch_arguments_v1(&q, &mut r).unwrap();
    let grouped = [DocumentEdits {
        expected: view.source.clone(),
        edits: vec![
            partitioned[1].edits[0].clone(),
            partitioned[0].edits[0].clone(),
        ],
    }];
    let same = normalize(&owner, &id, &grouped, &reads(), &scopes, &mut r);
    assert_eq!(
        owner.submit_exact_batch_arguments_v1(&same, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        owner.submit(
            &id,
            &SourceIntent {
                writes: grouped.to_vec(),
                reads: vec![]
            },
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&view.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "XbZ\r\ndef"
    );
    assert_eq!(owner.history_usage().retained_groups, 1);
}

#[test]
fn spent_identity_batch_has_no_source_or_history_publication_and_cannot_refill_after_forgetting() {
    let (mut owner, view, mut r) = fixture_with_intent_limit(65536);
    let id = owner.issue_invocation(&mut r).unwrap();
    let q = normalize(
        &owner,
        &id,
        &[write(&view.source, 0, 1, "a", "a")],
        &reads(),
        &[scope(&view.source, 8)],
        &mut r,
    );
    owner.submit_exact_batch_arguments_v1(&q, &mut r).unwrap();
    owner.run(&id, &mut r).unwrap();
    let InvocationOutcome::Committed(outcome) = owner.invocation_outcome(&id, &mut r).unwrap()
    else {
        panic!()
    };
    assert!(outcome.operation.is_none());
    assert!(outcome.changes.is_empty());
    assert!(outcome.history_group.is_none());
    assert_eq!(owner.history_usage().retained_groups, 0);
    assert_eq!(
        owner.logical_view(&view.view, &mut r).unwrap().stamp,
        view.stamp
    );
    owner.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        owner.submit_exact_batch_arguments_v1(&q, &mut r),
        Ok(InvocationState::Finished)
    );
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner.read(&view.source.document, &mut r).unwrap().version,
        view.source
    );
}

#[test]
fn invalid_scope_or_expected_bytes_refunds_preparation_without_binding_the_ticket() {
    let (mut owner, view, mut r) = fixture_with_intent_limit(65536);
    let id = owner.issue_invocation(&mut r).unwrap();
    let good = [write(&view.source, 0, 1, "a", "X")];
    let mut outside = scope(&view.source, 8);
    outside.replacement_ranges = vec![ByteRange { start: 2, end: 8 }];
    let q = normalize(&owner, &id, &good, &reads(), &[outside], &mut r);
    assert_eq!(
        owner.submit_exact_batch_arguments_v1(&q, &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        owner.invocation_state(&id, &mut r),
        Ok(InvocationState::Issued)
    );
    let q = normalize(
        &owner,
        &id,
        &[write(&view.source, 0, 1, "z", "X")],
        &reads(),
        &[scope(&view.source, 8)],
        &mut r,
    );
    assert_eq!(
        owner.submit_exact_batch_arguments_v1(&q, &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        owner.invocation_state(&id, &mut r),
        Ok(InvocationState::Issued)
    );
    let q = normalize(
        &owner,
        &id,
        &good,
        &reads(),
        &[scope(&view.source, 8)],
        &mut r,
    );
    owner.submit_exact_batch_arguments_v1(&q, &mut r).unwrap();
    owner.run(&id, &mut r).unwrap();
    assert_eq!(
        owner
            .read(&view.source.document, &mut r)
            .unwrap()
            .source
            .text(),
        "Xbc\r\ndef"
    );
}

#[test]
fn changed_read_or_one_participant_policy_refuses_the_whole_batch() {
    for change_read in [false, true] {
        let (mut owner, view, mut r) = fixture_with_intent_limit(65536);
        let other = owner.create(b"xyz", &mut r).unwrap();
        let read_source = owner.create(b"dependency", &mut r).unwrap();
        let id = owner.issue_invocation(&mut r).unwrap();
        let writes = [
            write(&view.source, 0, 1, "a", "X"),
            write(&other, 0, 1, "x", "Y"),
        ];
        let mut explicit_reads = reads();
        explicit_reads
            .predicates
            .push(ReadDependencyV3::Base(ReadDependencyV2::Base(
                ReadDependencyV1::Source(read_source.clone()),
            )));
        let q = normalize(
            &owner,
            &id,
            &writes,
            &explicit_reads,
            &[scope(&view.source, 8), scope(&other, 3)],
            &mut r,
        );
        owner.submit_exact_batch_arguments_v1(&q, &mut r).unwrap();
        if change_read {
            let p = owner
                .prepare(&[write(&read_source, 0, 1, "d", "D")], &[], &mut r)
                .unwrap();
            owner.commit(p, &mut r).unwrap();
        } else {
            owner.set_read_only(&other.document, true, &mut r).unwrap();
        }
        let groups = owner.history_usage().retained_groups;
        owner.run(&id, &mut r).unwrap();
        assert!(matches!(
            owner.invocation_outcome(&id, &mut r).unwrap(),
            InvocationOutcome::Rejected(_)
        ));
        assert_eq!(
            owner
                .read(&view.source.document, &mut r)
                .unwrap()
                .source
                .text(),
            "abc\r\ndef"
        );
        assert_eq!(
            owner.read(&other.document, &mut r).unwrap().source.text(),
            "xyz"
        );
        assert_eq!(owner.history_usage().retained_groups, groups);
    }
}

#[test]
fn closed_fields_and_three_independent_input_bounds_refuse_before_admission() {
    use ExactBatchEnvelopeValueV1 as V;
    let (mut owner, view, mut r) = fixture_with_intent_limit(65536);
    let id = owner.issue_invocation(&mut r).unwrap();
    let sources = [write(&view.source, 0, 1, "a", "X")];
    for bad in 0..4 {
        with_arguments(&id, &sources, &reads(), &[scope(&view.source, 8)], |args| {
            let V::Object(rows) = args else { panic!() };
            let mut rows = rows.to_vec();
            match bad {
                0 => rows.push(("annotations", V::Data(D::Array(&[])))),
                1 => rows[0] = ("operation", V::Data(D::Text("exact-batch"))),
                2 => rows[2].1 = V::Data(D::Text("invented-ticket")),
                _ => rows[4].1 = V::Data(D::Array(&[])),
            }
            assert!(matches!(
                owner.normalize_exact_batch_arguments_v1(
                    V::Object(&rows),
                    &context(),
                    limits(),
                    &mut r
                ),
                Err(InvocationError::MalformedIntent)
            ));
        });
    }
    for limit in [
        CaptureLimitsV1 {
            max_bytes: 1,
            ..limits()
        },
        CaptureLimitsV1 {
            max_nodes: 1,
            ..limits()
        },
        CaptureLimitsV1 {
            max_depth: 1,
            ..limits()
        },
    ] {
        with_arguments(&id, &sources, &reads(), &[scope(&view.source, 8)], |args| {
            assert!(matches!(
                owner.normalize_exact_batch_arguments_v1(args, &context(), limit, &mut r),
                Err(InvocationError::Limit)
            ))
        });
    }
    assert_eq!(
        owner.invocation_state(&id, &mut r),
        Ok(InvocationState::Issued)
    );
    assert_eq!(owner.history_usage().retained_groups, 0);
}

#[test]
fn installed_typed_dynamic_batches_preserve_stop_and_optional_capture_through_cleanup() {
    use ExactBatchEnvelopeValueV1 as V;
    use vellum_engine::instrumentation::{Disposition, NativeTimingProfileV1, Observation};
    for stop in [false, true] {
        for capture in 0..4 {
            let (mut owner, view, _) = fixture_with_intent_limit(65536);
            ordinary_result::fund(&mut owner, &mut Recorder::disabled());
            let mut r = match capture {
                0 => Recorder::disabled(),
                1 => Recorder::try_with_capacity(4096).unwrap(),
                2 => Recorder::try_with_capacity(4096)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
                _ => Recorder::try_with_capacity(1)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
            };
            if capture == 3 {
                r.record(Observation {
                    boundary: Boundary::InvocationAdmission,
                    disposition: Disposition::Completed,
                    service_time: None,
                });
            }
            let route = selection_route::Route::install_exact_batch(&mut owner, &mut r);
            let id = owner.issue_invocation(&mut r).unwrap();
            ordinary_result::bind_in_context(&mut owner, &id, &context(), &mut r);
            let sources = [write(&view.source, 0, 1, "a", "X")];
            with_arguments(&id, &sources, &reads(), &[scope(&view.source, 8)], |args| {
                let typed = capture % 2 == 0;
                let header = [
                    (
                        if typed { "family" } else { "name" },
                        if typed {
                            V::Data(D::Number(10.0))
                        } else {
                            V::Data(D::Text("change-source"))
                        },
                    ),
                    ("variant", V::Data(D::Text("exact-batch"))),
                    ("version", V::Data(D::Number(1.0))),
                    ("arguments", args),
                ];
                let outer = [
                    (
                        "route",
                        V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
                    ),
                    ("request", V::Object(&header)),
                ];
                let q = owner
                    .normalize_exact_batch_envelope_v1(
                        V::Object(&outer),
                        &context(),
                        limits(),
                        &mut r,
                    )
                    .unwrap();
                assert_eq!(
                    route.submit_exact_batch(&mut owner, &q, &mut r),
                    Ok(InvocationState::Pending)
                );
                let mut changed_context = context();
                changed_context.recipient = "another-agent".into();
                let wrong = owner
                    .normalize_exact_batch_envelope_v1(
                        V::Object(&outer),
                        &changed_context,
                        limits(),
                        &mut r,
                    )
                    .unwrap();
                assert_eq!(
                    owner.submit_exact_batch_arguments_v1(&wrong, &mut r),
                    Err(InvocationError::IntentMismatch)
                );
            });
            if stop {
                owner
                    .request_invocation_stop(
                        &id,
                        &TrustedInvocationControlV1 {
                            context: &context(),
                            owner_control_allowed: Some(true),
                            control_context_current: Some(true),
                            control_scope_valid: Some(true),
                            protected_control_ready: Some(true),
                        },
                        &mut r,
                    )
                    .unwrap();
            }
            route.retire(&mut owner, &mut r);
            owner.run(&id, &mut r).unwrap();
            if stop {
                assert!(matches!(
                    owner.invocation_outcome(&id, &mut r).unwrap(),
                    InvocationOutcome::StoppedBeforePublication
                ));
            } else {
                assert!(matches!(
                    owner.invocation_outcome(&id, &mut r).unwrap(),
                    InvocationOutcome::Committed(_)
                ));
            }
            assert_eq!(
                owner
                    .read(&view.source.document, &mut r)
                    .unwrap()
                    .source
                    .text(),
                if stop { "abc\r\ndef" } else { "Xbc\r\ndef" }
            );
            assert_eq!(owner.history_usage().retained_groups, usize::from(!stop));
            ordinary_result::check_in_context(
                &mut owner,
                &id,
                &context(),
                if stop {
                    InvocationPublicationV1::NotApplied
                } else {
                    InvocationPublicationV1::Applied
                },
                &mut r,
            );
            owner.forget_invocation_outcome(&id, &mut r).unwrap();
            let mut found = [false; 4];
            while let Some(record) = r.pop_front() {
                let index = match record.boundary {
                    Boundary::ExactBatchArgumentNormalization => Some(0),
                    Boundary::ExactBatchPreparation => Some(1),
                    Boundary::ExactBatchAdmission => Some(2),
                    Boundary::DiscoveryExactBatchDispatch => Some(3),
                    _ => None,
                };
                if let Some(index) = index {
                    found[index] = true;
                    assert_eq!(record.service_time.is_some(), capture == 2);
                }
            }
            assert_eq!(found, [capture == 1 || capture == 2; 4]);
        }
    }
}
