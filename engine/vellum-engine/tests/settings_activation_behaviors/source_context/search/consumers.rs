use super::*;
#[path = "consumers/returned_arguments.rs"]
mod returned_arguments;
fn consumer_facts<'a>(
    command: &'a SourceSearchConsumerV1,
    s: &'a SourceReadScopeV1,
) -> TrustedSourceSearchConsumerFactsV1<'a> {
    TrustedSourceSearchConsumerFactsV1 {
        scope: s,
        command,
        mutation_allowed: Some(true),
        resources_ready: Some(true),
    }
}
#[test]
fn returned_search_replacement_runs_current_and_all_with_capture_modes_and_complete_cleanup() {
    use vellum_engine::capture::CaptureLimitsV1;
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for kind in 0..2 {
        for mode in 0..4 {
            let mut f = fixture(kind);
            f.r = match mode {
                0 => Recorder::disabled(),
                1 => Recorder::try_with_capacity(128).unwrap(),
                2 => Recorder::try_with_capacity(128)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
                _ => Recorder::try_with_capacity(0)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
            };
            let q = request("a");
            let baseline = f.o.retention_usage().reserved;
            f.o.release_source_search(&f.result, &f.s, &facts(&f.s), &qfacts(&q), &mut f.r)
                .unwrap();
            let id = f.o.issue_invocation(&mut f.r).unwrap();
            let limits = CaptureLimitsV1 {
                max_bytes: 16384,
                max_nodes: 128,
                max_depth: 8,
            };
            let submit = |f: &mut Fixture| {
                f.o.submit_returned_search_replacement_v1(
                    &id,
                    &f.command,
                    &f.s,
                    SearchReadFactsV1::Source(&facts(&f.s)),
                    &qfacts(&q),
                    &consumer_facts(&f.command, &f.s),
                    limits,
                    &mut f.r,
                )
            };
            assert_eq!(submit(&mut f), Ok(InvocationState::Pending));
            let funded = f.o.retention_usage().reserved;
            assert!(funded > baseline);
            assert_eq!(f.o.run(&id, &mut f.r), Err(InvocationError::Unavailable));
            assert_eq!(
                f.o.run_source_search_consumer(
                    &id,
                    &f.s,
                    &facts(&f.s),
                    &qfacts(&q),
                    &consumer_facts(&f.command, &f.s),
                    &mut f.r
                ),
                Ok(InvocationState::Finished)
            );
            assert_eq!(
                f.o.read(&f.d.document, &mut f.r).unwrap().source.text(),
                if kind == 0 { "ZbZ" } else { "Zba" }
            );
            assert_eq!(submit(&mut f), Ok(InvocationState::Finished));
            assert_eq!(f.o.retention_usage().reserved, funded);
            assert!(matches!(
                f.o.invocation_outcome(&id, &mut f.r).unwrap(),
                InvocationOutcome::Committed(_)
            ));
            let control = InvocationObservationPolicyControlV1 {
                context: &f.s.context,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            };
            f.o.retire_returned_search_replacement_v1(&id, &control, &mut f.r)
                .unwrap();
            assert_eq!(f.o.retention_usage().reserved, baseline);
            f.o.retire_invocation(&id, &mut f.r).unwrap();
            assert_eq!(f.o.source_search_usage().results, 1);
            if mode == 1 || mode == 2 {
                let mut boundaries = Vec::new();
                while let Some(record) = f.r.pop_front() {
                    if matches!(
                        record.boundary,
                        Boundary::ReturnedSearchReplacementAdmission
                            | Boundary::ReturnedSearchReplacementRetirement
                    ) {
                        assert_eq!(record.service_time.is_some(), mode == 2);
                        boundaries.push(record.boundary);
                    }
                }
                assert_eq!(
                    boundaries,
                    [
                        Boundary::ReturnedSearchReplacementAdmission,
                        Boundary::ReturnedSearchReplacementRetirement
                    ]
                );
            }
        }
    }
}
#[test]
fn managed_replacement_rechecks_original_result_at_service_and_generic_runner_cannot_bypass_it() {
    for lost in [false, true] {
        let (mut o, mut r, d, s) = ready(b"aba");
        o.configure_source_search(search_profile(), &pool_control(), &mut r)
            .unwrap();
        let source = o
            .capture_source_read_context(&s, &facts(&s), &mut r)
            .unwrap();
        let q = request("a");
        let result = o
            .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
            .unwrap();
        let command = SourceSearchConsumerV1::ReplaceAll {
            result,
            replacement: "Z".into(),
        };
        let id = o.issue_invocation(&mut r).unwrap();
        assert_eq!(
            o.submit_source_search_consumer(
                &id,
                &command,
                &s,
                &facts(&s),
                &qfacts(&q),
                &consumer_facts(&command, &s),
                &mut r
            ),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.run(&id, &mut r), Err(InvocationError::Unavailable));
        let mut f = qfacts(&q);
        if lost {
            f.grant = Some(false);
        }
        assert_eq!(
            o.run_source_search_consumer(
                &id,
                &s,
                &facts(&s),
                &f,
                &consumer_facts(&command, &s),
                &mut r
            ),
            Ok(InvocationState::Finished)
        );
        match o.invocation_outcome(&id, &mut r).unwrap() {
            InvocationOutcome::Committed(_) if !lost => {}
            InvocationOutcome::Rejected(CoordinatorError::Unavailable) if lost => {}
            other => panic!("unexpected outcome {other:?}"),
        }
        assert_eq!(
            o.read(&d.document, &mut r).unwrap().source.text(),
            if lost { "aba" } else { "ZbZ" }
        );
        assert_eq!(o.source_read_context_usage().holds, 1);
    }
}

struct Fixture {
    o: SourceCoordinator,
    r: Recorder,
    d: DocumentVersion,
    s: SourceReadScopeV1,
    source: SourceReadContextId,
    result: SourceSearchResultId,
    command: SourceSearchConsumerV1,
    view: LogicalViewContext,
}
fn fixture(kind: usize) -> Fixture {
    use vellum_engine::{
        attachment::Affinity,
        selection::{Selection, SelectionBudget},
    };
    let (mut o, mut r, d, s) = ready(b"aba");
    o.configure_logical_views(
        LogicalViewLimits {
            max_live_views: 8,
            max_issued_views: 8,
            max_total_selections: 16,
            normalization: SelectionBudget {
                max_input_selections: 16,
                max_output_selections: 16,
                max_work: 4096,
            },
            max_mapping_steps: 4096,
        },
        &mut r,
    )
    .unwrap();
    let id = o.create_logical_view(&mut r).unwrap();
    let stamp = o.logical_view(&id, &mut r).unwrap().stamp;
    o.attach_logical_view(
        &id,
        stamp,
        &d,
        &[Selection {
            anchor: BytePosition(3),
            head: BytePosition(3),
            affinity: Affinity::After,
        }],
        0,
        &mut r,
    )
    .unwrap();
    let view = o.logical_view(&id, &mut r).unwrap().context().unwrap();
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request("a");
    let result = o
        .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    let occurrence = o
        .navigate_source_search(
            &result,
            &s,
            SourceSearchNavigationV1 {
                direction: SearchDirection::Next,
                start: 0,
                current: None,
                wrap: false,
            },
            &facts(&s),
            &qfacts(&q),
            &mut r,
        )
        .unwrap()
        .found
        .unwrap()
        .occurrence;
    let command = match kind {
        0 => SourceSearchConsumerV1::ReplaceAll {
            result: result.clone(),
            replacement: "Z".into(),
        },
        1 => SourceSearchConsumerV1::ReplaceCurrent {
            occurrence,
            replacement: "Z".into(),
        },
        _ => SourceSearchConsumerV1::SelectMatch {
            occurrence,
            view: view.clone(),
            direction: SelectionDirectionV1::Backward,
            affinity: Affinity::Before,
            mode: SearchSelectionModeV1::ReplaceAll,
        },
    };
    Fixture {
        o,
        r,
        d,
        s,
        source,
        result,
        command,
        view,
    }
}
fn submit(f: &mut Fixture) -> InvocationId {
    let id = f.o.issue_invocation(&mut f.r).unwrap();
    let q = request("a");
    assert_eq!(
        f.o.submit_source_search_consumer(
            &id,
            &f.command,
            &f.s,
            &facts(&f.s),
            &qfacts(&q),
            &consumer_facts(&f.command, &f.s),
            &mut f.r
        ),
        Ok(InvocationState::Pending)
    );
    id
}
fn rejected(f: &mut Fixture, id: &InvocationId, kind: usize) {
    match f.o.invocation_outcome(id, &mut f.r).unwrap() {
        InvocationOutcome::Rejected(CoordinatorError::Unavailable) if kind < 2 => {}
        InvocationOutcome::SelectionRejected(
            vellum_engine::selection::SelectionError::Unavailable,
        ) if kind == 2 => {}
        other => panic!("unexpected outcome {other:?}"),
    }
}

#[test]
fn all_managed_consumers_publish_through_existing_source_and_selection_contracts_once() {
    for kind in 0..3 {
        let mut f = fixture(kind);
        let id = submit(&mut f);
        let q = request("a");
        assert_eq!(f.o.run(&id, &mut f.r), Err(InvocationError::Unavailable));
        assert_eq!(
            f.o.run_source_search_consumer(
                &id,
                &f.s,
                &facts(&f.s),
                &qfacts(&q),
                &consumer_facts(&f.command, &f.s),
                &mut f.r
            ),
            Ok(InvocationState::Finished)
        );
        assert_eq!(
            f.o.read(&f.d.document, &mut f.r).unwrap().source.text(),
            match kind {
                0 => "ZbZ",
                1 => "Zba",
                _ => "aba",
            }
        );
        if kind == 2 {
            let view = f.o.logical_view(&f.view.view, &mut f.r).unwrap();
            assert_eq!(view.selections.selections()[0].anchor, BytePosition(1));
            assert_eq!(view.selections.selections()[0].head, BytePosition(0));
        }
        let generation = f.o.generation();
        let mut denied = consumer_facts(&f.command, &f.s);
        denied.mutation_allowed = Some(false);
        assert_eq!(
            f.o.run_source_search_consumer(&id, &f.s, &facts(&f.s), &qfacts(&q), &denied, &mut f.r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(f.o.generation(), generation);
        assert_eq!(f.o.source_read_context_usage().holds, 1);
        assert_eq!(f.o.source_read_context_usage().issued_lifetime, 2);
    }
}

#[test]
fn every_source_and_query_fact_is_fresh_at_consumer_service_without_new_leases_or_refunds() {
    for kind in 0..3 {
        for index in 0..16 {
            let mut f = fixture(kind);
            let id = submit(&mut f);
            let q = request("a");
            let reserved = f.o.retention_usage().reserved;
            let generation = f.o.generation();
            let mut sf = facts(&f.s);
            let mut qf = qfacts(&q);
            if index < 10 {
                set_fact(&mut sf, index, Some(false));
            } else {
                match index {
                    10 => qf.grant = Some(false),
                    11 => qf.scope = Some(false),
                    12 => qf.continuity = Some(false),
                    13 => qf.capacity = Some(false),
                    14 => qf.retained = Some(false),
                    _ => qf.not_stopped = Some(false),
                }
            }
            assert_eq!(
                f.o.run_source_search_consumer(
                    &id,
                    &f.s,
                    &sf,
                    &qf,
                    &consumer_facts(&f.command, &f.s),
                    &mut f.r
                ),
                Ok(InvocationState::Finished)
            );
            rejected(&mut f, &id, kind);
            assert_eq!(f.o.generation(), generation);
            assert_eq!(f.o.retention_usage().reserved, reserved);
            assert_eq!(f.o.source_read_context_usage().holds, 1);
            assert_eq!(
                f.o.read(&f.d.document, &mut f.r).unwrap().source.text(),
                "aba"
            );
            assert!(!f.o.is_faulted());
        }
    }
}

#[test]
fn mutation_authority_is_independent_from_search_read_authority_and_never_revokes_reading() {
    for kind in 0..3 {
        for value in [None, Some(false)] {
            let mut f = fixture(kind);
            let q = request("a");
            let id = f.o.issue_invocation(&mut f.r).unwrap();
            let mut cf = consumer_facts(&f.command, &f.s);
            cf.mutation_allowed = value;
            assert_eq!(
                f.o.submit_source_search_consumer(
                    &id,
                    &f.command,
                    &f.s,
                    &facts(&f.s),
                    &qfacts(&q),
                    &cf,
                    &mut f.r
                ),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(
                f.o.invocation_state(&id, &mut f.r),
                Ok(InvocationState::Issued)
            );
            assert_eq!(
                f.o.submit_source_search_consumer(
                    &id,
                    &f.command,
                    &f.s,
                    &facts(&f.s),
                    &qfacts(&q),
                    &consumer_facts(&f.command, &f.s),
                    &mut f.r
                ),
                Ok(InvocationState::Pending)
            );
            assert_eq!(
                f.o.run_source_search_consumer(&id, &f.s, &facts(&f.s), &qfacts(&q), &cf, &mut f.r),
                Ok(InvocationState::Finished)
            );
            rejected(&mut f, &id, kind);
            assert!(
                f.o.release_source_search(&f.result, &f.s, &facts(&f.s), &qfacts(&q), &mut f.r)
                    .is_some()
            );
            assert!(
                f.o.query_source_read_context(&f.source, &f.s, one_byte(), &facts(&f.s), &mut f.r)
                    .is_some()
            );
        }
    }
}

#[test]
fn protected_result_retirement_does_not_cancel_pending_consumers_or_erase_their_original_intent() {
    for kind in 0..3 {
        let mut f = fixture(kind);
        let id = submit(&mut f);
        let q = request("a");
        assert!(f.o.retire_source_search_result(&f.result, &controls(&f.s), &mut f.r));
        assert_eq!(f.o.source_read_context_usage().holds, 0);
        assert_eq!(
            f.o.invocation_state(&id, &mut f.r),
            Ok(InvocationState::Pending)
        );
        assert_eq!(
            f.o.submit_source_search_consumer(
                &id,
                &f.command,
                &f.s,
                &facts(&f.s),
                &qfacts(&q),
                &consumer_facts(&f.command, &f.s),
                &mut f.r
            ),
            Ok(InvocationState::Pending)
        );
        assert_eq!(
            f.o.run_source_search_consumer(
                &id,
                &f.s,
                &facts(&f.s),
                &qfacts(&q),
                &consumer_facts(&f.command, &f.s),
                &mut f.r
            ),
            Ok(InvocationState::Finished)
        );
        rejected(&mut f, &id, kind);
        assert_eq!(f.o.source_search_usage().issued_lifetime, 1);
        assert_eq!(
            f.o.read(&f.d.document, &mut f.r).unwrap().source.text(),
            "aba"
        );
        assert!(
            f.o.query_source_read_context(&f.source, &f.s, one_byte(), &facts(&f.s), &mut f.r)
                .is_some()
        );
    }
}

#[test]
fn changed_service_packets_and_replay_intents_cannot_poison_or_refresh_pending_work() {
    for kind in 0..3 {
        let mut f = fixture(kind);
        let id = submit(&mut f);
        let q = request("a");
        let mut wrong = f.s.clone();
        wrong.policy.push('x');
        let mut loss = qfacts(&q);
        loss.grant = Some(false);
        assert_eq!(
            f.o.run_source_search_consumer(
                &id,
                &wrong,
                &facts(&wrong),
                &loss,
                &consumer_facts(&f.command, &wrong),
                &mut f.r
            ),
            Err(InvocationError::Unavailable)
        );
        let different = SourceSearchConsumerV1::ReplaceAll {
            result: f.result.clone(),
            replacement: "different".into(),
        };
        assert_eq!(
            f.o.run_source_search_consumer(
                &id,
                &f.s,
                &facts(&f.s),
                &loss,
                &consumer_facts(&different, &f.s),
                &mut f.r
            ),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            f.o.submit_source_search_consumer(
                &id,
                &different,
                &f.s,
                &facts(&f.s),
                &qfacts(&q),
                &consumer_facts(&different, &f.s),
                &mut f.r
            ),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(
            f.o.invocation_state(&id, &mut f.r),
            Ok(InvocationState::Pending)
        );
        assert_eq!(f.o.source_search_usage().invalidated, 0);
        assert_eq!(
            f.o.run_source_search_consumer(
                &id,
                &f.s,
                &facts(&f.s),
                &qfacts(&q),
                &consumer_facts(&f.command, &f.s),
                &mut f.r
            ),
            Ok(InvocationState::Finished)
        );
        assert!(!f.o.is_faulted());
    }
}

#[test]
fn intrinsic_read_only_and_stale_source_or_view_checks_remain_required_after_read_eligibility() {
    use vellum_engine::{
        attachment::Affinity,
        selection::{Selection, SelectionError},
    };
    for mode in 0..3 {
        let mut f = fixture(if mode == 2 { 2 } else { 0 });
        let id = submit(&mut f);
        let q = request("a");
        if mode == 0 {
            f.o.set_read_only(&f.d.document, true, &mut f.r).unwrap();
        } else if mode == 1 {
            let p =
                f.o.prepare(
                    &[DocumentEdits {
                        expected: f.d.clone(),
                        edits: vec![vellum_engine::source::Replacement {
                            start: 0,
                            end: 1,
                            expected: "a".into(),
                            inserted: "B".into(),
                        }],
                    }],
                    &[],
                    &mut f.r,
                )
                .unwrap();
            f.o.commit(p, &mut f.r).unwrap();
        } else {
            f.o.set_logical_selections(
                &f.view,
                &[Selection {
                    anchor: BytePosition(2),
                    head: BytePosition(2),
                    affinity: Affinity::After,
                }],
                0,
                &mut f.r,
            )
            .unwrap();
        }
        let generation = f.o.generation();
        assert_eq!(
            f.o.run_source_search_consumer(
                &id,
                &f.s,
                &facts(&f.s),
                &qfacts(&q),
                &consumer_facts(&f.command, &f.s),
                &mut f.r
            ),
            Ok(InvocationState::Finished)
        );
        match f.o.invocation_outcome(&id, &mut f.r).unwrap() {
            InvocationOutcome::Rejected(_) if mode < 2 => {}
            InvocationOutcome::SelectionRejected(SelectionError::Stale) if mode == 2 => {}
            other => panic!("unexpected outcome {other:?}"),
        }
        assert_eq!(f.o.generation(), generation);
        assert!(!f.o.is_faulted());
    }
}

#[test]
fn partial_owned_occurrence_supports_current_replacement_but_not_exact_navigation_or_replace_all() {
    let (mut o, mut r, d, s) = ready(b"aba");
    let mut p = search_profile();
    p.max_matches = 1;
    o.configure_source_search(p, &pool_control(), &mut r)
        .unwrap();
    let source = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let q = request("a");
    let result = o
        .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    assert_eq!(
        o.release_source_search(&result, &s, &facts(&s), &qfacts(&q), &mut r)
            .unwrap()
            .status,
        SearchStatus::Exhausted(SearchBudget::Matches)
    );
    let mut loss = qfacts(&q);
    loss.grant = Some(false);
    assert!(
        o.source_search_occurrence(&result, 1, &s, &facts(&s), &loss, &mut r)
            .is_none()
    );
    assert_eq!(o.source_search_usage().invalidated, 0);
    let occurrence = o
        .source_search_occurrence(&result, 0, &s, &facts(&s), &qfacts(&q), &mut r)
        .unwrap();
    let all = SourceSearchConsumerV1::ReplaceAll {
        result: result.clone(),
        replacement: "Z".into(),
    };
    let id = o.issue_invocation(&mut r).unwrap();
    assert_eq!(
        o.submit_source_search_consumer(
            &id,
            &all,
            &s,
            &facts(&s),
            &qfacts(&q),
            &consumer_facts(&all, &s),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    let current = SourceSearchConsumerV1::ReplaceCurrent {
        occurrence,
        replacement: "Z".into(),
    };
    assert_eq!(
        o.submit_source_search_consumer(
            &id,
            &current,
            &s,
            &facts(&s),
            &qfacts(&q),
            &consumer_facts(&current, &s),
            &mut r
        ),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        o.run_source_search_consumer(
            &id,
            &s,
            &facts(&s),
            &qfacts(&q),
            &consumer_facts(&current, &s),
            &mut r
        ),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "Zba");
}

#[test]
fn protected_stop_is_intent_only_and_does_not_require_mutation_permission_to_settle() {
    for kind in 0..3 {
        let mut f = fixture(kind);
        let id = submit(&mut f);
        let q = request("a");
        let generation = f.o.generation();
        f.o.request_invocation_stop(
            &id,
            &TrustedInvocationControlV1 {
                context: &f.s.context,
                owner_control_allowed: Some(true),
                control_context_current: Some(true),
                control_scope_valid: Some(true),
                protected_control_ready: Some(true),
            },
            &mut f.r,
        )
        .unwrap();
        assert_eq!(
            f.o.invocation_state(&id, &mut f.r),
            Ok(InvocationState::Pending)
        );
        let mut cf = consumer_facts(&f.command, &f.s);
        cf.mutation_allowed = Some(false);
        assert_eq!(
            f.o.run_source_search_consumer(&id, &f.s, &facts(&f.s), &qfacts(&q), &cf, &mut f.r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(
            f.o.invocation_outcome(&id, &mut f.r),
            Ok(&InvocationOutcome::StoppedBeforePublication)
        );
        assert_eq!(f.o.generation(), generation);
        assert_eq!(f.o.source_read_context_usage().holds, 1);
    }
}

#[test]
fn empty_or_equal_replacement_uses_common_validated_unchanged_contract_and_keeps_literal_intent() {
    for query in ["z", "a"] {
        let (mut o, mut r, d, s) = ready(b"aba");
        o.configure_source_search(search_profile(), &pool_control(), &mut r)
            .unwrap();
        let source = o
            .capture_source_read_context(&s, &facts(&s), &mut r)
            .unwrap();
        let q = request(query);
        let result = o
            .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
            .unwrap();
        let c = SourceSearchConsumerV1::ReplaceAll {
            result,
            replacement: query.into(),
        };
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_source_search_consumer(
            &id,
            &c,
            &s,
            &facts(&s),
            &qfacts(&q),
            &consumer_facts(&c, &s),
            &mut r,
        )
        .unwrap();
        let generation = o.generation();
        o.run_source_search_consumer(
            &id,
            &s,
            &facts(&s),
            &qfacts(&q),
            &consumer_facts(&c, &s),
            &mut r,
        )
        .unwrap();
        let InvocationOutcome::Committed(v) = o.invocation_outcome(&id, &mut r).unwrap() else {
            panic!("not committed")
        };
        assert!(v.operation.is_none());
        assert_eq!(o.generation(), generation);
        assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "aba");
    }
}

#[test]
fn consumer_instrumentation_is_optional_bounded_redacted_and_does_not_invent_publication() {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for kind in 0..3 {
        for lost in [false, true] {
            for capacity in [None, Some(128), Some(1)] {
                let mut f = fixture(kind);
                if let SourceSearchConsumerV1::ReplaceAll { replacement, .. }
                | SourceSearchConsumerV1::ReplaceCurrent { replacement, .. } = &mut f.command
                {
                    *replacement = "PRIVATE_REPLACEMENT_SENTINEL".into();
                }
                assert_eq!(
                    format!("{:?}", f.command),
                    "SourceSearchConsumerV1(<redacted>)"
                );
                f.r = capacity.map_or_else(Recorder::disabled, |n| {
                    Recorder::try_with_capacity(n).unwrap()
                });
                let id = submit(&mut f);
                let q = request("a");
                let mut cf = consumer_facts(&f.command, &f.s);
                if lost {
                    cf.mutation_allowed = Some(false);
                }
                f.o.run_source_search_consumer(&id, &f.s, &facts(&f.s), &qfacts(&q), &cf, &mut f.r)
                    .unwrap();
                if lost {
                    rejected(&mut f, &id, kind);
                }
                if capacity == Some(128) {
                    let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
                    for boundary in [
                        Boundary::SourceSearchConsumerPreparation,
                        Boundary::SourceSearchConsumerValidation,
                        Boundary::SourceSearchConsumerPublication,
                    ] {
                        assert!(events.iter().any(|e| e.boundary == boundary));
                    }
                    assert!(events.iter().any(|e| e.boundary
                        == Boundary::SourceSearchConsumerPublication
                        && e.disposition
                            == if lost {
                                Disposition::Refused
                            } else {
                                Disposition::Completed
                            }));
                    assert_eq!(
                        events
                            .iter()
                            .filter(|e| e.boundary == Boundary::Publication)
                            .count(),
                        usize::from(!lost)
                    );
                    assert!(events.iter().all(|e| e.service_time.is_none()));
                    assert!(!format!("{events:?}").contains("PRIVATE_REPLACEMENT_SENTINEL"));
                } else if capacity == Some(1) {
                    assert!(f.r.dropped() > 0);
                } else {
                    assert_eq!(f.r.queued(), 0);
                }
                assert_eq!(f.o.source_read_context_usage().holds, 1);
            }
        }
    }
}

#[test]
fn managed_match_selection_preserves_every_closed_direction_affinity_and_mode() {
    use vellum_engine::attachment::Affinity;
    for direction in [
        SelectionDirectionV1::Forward,
        SelectionDirectionV1::Backward,
    ] {
        for affinity in [Affinity::Before, Affinity::After] {
            for mode in [
                SearchSelectionModeV1::ReplaceAll,
                SearchSelectionModeV1::ReplacePrimary,
                SearchSelectionModeV1::AddPrimary,
            ] {
                let mut f = fixture(2);
                let SourceSearchConsumerV1::SelectMatch {
                    direction: d,
                    affinity: a,
                    mode: m,
                    ..
                } = &mut f.command
                else {
                    unreachable!()
                };
                *d = direction;
                *a = affinity;
                *m = mode;
                let id = submit(&mut f);
                let q = request("a");
                f.o.run_source_search_consumer(
                    &id,
                    &f.s,
                    &facts(&f.s),
                    &qfacts(&q),
                    &consumer_facts(&f.command, &f.s),
                    &mut f.r,
                )
                .unwrap();
                let view = f.o.logical_view(&f.view.view, &mut f.r).unwrap();
                assert_eq!(
                    view.selections.selections().len(),
                    if mode == SearchSelectionModeV1::AddPrimary {
                        2
                    } else {
                        1
                    }
                );
                let selected = view.selections.selections()[view.selections.primary()];
                let (anchor, head) = if direction == SelectionDirectionV1::Forward {
                    (0, 1)
                } else {
                    (1, 0)
                };
                assert_eq!(selected.anchor, BytePosition(anchor));
                assert_eq!(selected.head, BytePosition(head));
                assert_eq!(selected.affinity, affinity);
                assert_eq!(
                    f.o.read(&f.d.document, &mut f.r).unwrap().source.text(),
                    "aba"
                );
            }
        }
    }
}
