#[path = "support/scratch_invocation_fixture.rs"]
mod scratch;
use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::selection::{Selection, SelectionBudget};
#[path = "native_lifecycle_facade_behaviors/requests.rs"]
mod requests;

fn relation_facts(c: &SettingsExecutionContextV1) -> TrustedLiveRelationFactsV1<'_> {
    TrustedLiveRelationFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        resources_ready: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        lifecycle_allowed: Some(true),
        coverage_complete: Some(true),
        view_writable: Some(true),
        source_readable: Some(true),
        membership_writable: Some(true),
        workspace_readable: Some(true),
    }
}
fn relation_admission<'a>(f: &'a TrustedLiveRelationFactsV1<'a>) -> LiveRelationAdmissionV1<'a> {
    LiveRelationAdmissionV1 {
        facts: f,
        limits: LiveRelationLimitsV1 {
            max_capture_bytes: 8192,
            max_result_bytes: 512,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_identifiers_allowed: Some(true),
    }
}
fn disposal_facts(c: &SettingsExecutionContextV1) -> TrustedRetainedDisposalFactsV1<'_> {
    TrustedRetainedDisposalFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        disposal_allowed: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
    }
}
fn disposal_admission<'a>(
    f: &'a TrustedRetainedDisposalFactsV1<'a>,
) -> RetainedDisposalAdmissionV1<'a> {
    RetainedDisposalAdmissionV1 {
        facts: f,
        limits: RetainedDisposalLimitsV1 {
            max_capture_bytes: 8192,
            max_result_bytes: 512,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_allowed: Some(true),
    }
}
fn relation_request(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
    kind: usize,
) -> LiveRelationRequestV1 {
    o.configure_workspaces(
        WorkspaceLimits {
            max_issued: 4,
            max_live: 4,
            max_memberships: 8,
            max_work: 4096,
        },
        r,
    )
    .unwrap();
    o.configure_logical_views(
        LogicalViewLimits {
            max_live_views: 4,
            max_issued_views: 8,
            max_total_selections: 16,
            normalization: SelectionBudget {
                max_input_selections: 8,
                max_output_selections: 8,
                max_work: 4096,
            },
            max_mapping_steps: 4096,
        },
        r,
    )
    .unwrap();
    let d = o.create(b"abc", r).unwrap();
    let lifecycle = o.lifecycle_state(&d.document, r).unwrap();
    let operation = if kind == 2 {
        LiveRelationOperationV1::Membership {
            workspace: o.create_workspace(r).unwrap(),
            present: true,
        }
    } else {
        let view = o.create_logical_view(r).unwrap();
        let mut expected = o.logical_view(&view, r).unwrap().stamp;
        let selections = vec![Selection {
            anchor: BytePosition(1),
            head: BytePosition(1),
            affinity: Affinity::After,
        }];
        if kind == 1 {
            expected = o
                .attach_logical_view(&view, expected, &d, &selections, 0, r)
                .unwrap()
                .stamp;
            LiveRelationOperationV1::Detach { view, expected }
        } else {
            LiveRelationOperationV1::Attach {
                view,
                expected,
                source: d.clone(),
                selections,
                primary: 0,
            }
        }
    };
    LiveRelationRequestV1 {
        document: d.document,
        expected_lifecycle: lifecycle,
        operation,
    }
}

#[test]
fn native_facade_keeps_the_original_scratch_owner_and_replay_without_new_jobs() {
    let (mut o, mut r) = scratch::owner();
    let c = scratch::context();
    let f = scratch::facts(&c);
    let q = scratch::request(b"original");
    let a = scratch::admission(&f);
    let id = o.issue_invocation(&mut r).unwrap();
    assert_eq!(
        o.submit_native_document_lifecycle_v1(
            &id,
            NativeDocumentLifecycleCommandV1::Scratch(&q),
            NativeDocumentLifecycleAdmissionV1::Scratch(&a),
            &mut r
        ),
        Ok(InvocationState::Pending)
    );
    assert_eq!(o.lifecycle_usage().live, 0);
    o.run_native_document_lifecycle_v1(&id, NativeDocumentLifecycleServiceV1::Scratch(&f), &mut r)
        .unwrap();
    let original = o
        .scratch_result_v1(&id, &scratch::release(&c), &mut r)
        .unwrap();
    let generation = o.generation();
    o.submit_native_document_lifecycle_v1(
        &id,
        NativeDocumentLifecycleCommandV1::Scratch(&q),
        NativeDocumentLifecycleAdmissionV1::Scratch(&a),
        &mut r,
    )
    .unwrap();
    o.run_scratch_v1(&id, &f, &mut r).unwrap();
    assert_eq!(
        o.scratch_result_v1(&id, &scratch::release(&c), &mut r)
            .unwrap(),
        original
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.invocation_usage().issued_lifetime, 1);
    o.retire_scratch_v1(&id, &scratch::control(&c), &mut r)
        .unwrap();
}

#[test]
fn native_facade_preserves_all_three_live_relation_services_and_actual_results() {
    for kind in 0..3 {
        let (mut o, mut r) = scratch::owner();
        let c = scratch::context();
        let f = relation_facts(&c);
        let q = relation_request(&mut o, &mut r, kind);
        let a = relation_admission(&f);
        let id = o.issue_invocation(&mut r).unwrap();
        let generation = o.generation();
        o.submit_native_document_lifecycle_v1(
            &id,
            NativeDocumentLifecycleCommandV1::LiveRelation(&q),
            NativeDocumentLifecycleAdmissionV1::LiveRelation(&a),
            &mut r,
        )
        .unwrap();
        assert_eq!(o.generation(), generation);
        o.run_native_document_lifecycle_v1(
            &id,
            NativeDocumentLifecycleServiceV1::LiveRelation(&f),
            &mut r,
        )
        .unwrap();
        assert_eq!(o.generation(), generation + 1);
        let n = TrustedLiveRelationResultV1 {
            context: &c,
            publication_epoch: 7,
            publication_allowed: Some(true),
            outcome_readable: Some(true),
            result_readable: Some(true),
            result_context_current: Some(true),
            historical_document_readable: Some(true),
            reply_ready: Some(true),
            resources_ready: Some(true),
            max_result_bytes: 512,
        };
        let original = o.live_relation_result_v1(&id, &n, &mut r).unwrap();
        assert_eq!(original.document, q.document);
        assert!(original.changed);
        o.submit_live_relation_v1(&id, &q, &a, &mut r).unwrap();
        o.run_native_document_lifecycle_v1(
            &id,
            NativeDocumentLifecycleServiceV1::LiveRelation(&f),
            &mut r,
        )
        .unwrap();
        assert_eq!(
            o.live_relation_result_v1(&id, &n, &mut r).unwrap(),
            original
        );
        assert_eq!(o.generation(), generation + 1);
        o.retire_live_relation_v1(&id, &scratch::control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
#[test]
fn native_facade_uses_exact_prepared_disposal_not_a_retention_discard_data_decoder() {
    let (mut o, mut r) = scratch::owner();
    let c = scratch::context();
    let f = disposal_facts(&c);
    let doc = o.create(b"retained", &mut r).unwrap();
    let plan = o.prepare_retained_disposal(&doc, 0, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let a = disposal_admission(&f);
    let generation = o.generation();
    o.submit_native_document_lifecycle_v1(
        &id,
        NativeDocumentLifecycleCommandV1::PreparedDisposal(&plan),
        NativeDocumentLifecycleAdmissionV1::PreparedDisposal(&a),
        &mut r,
    )
    .unwrap();
    assert_eq!(o.generation(), generation);
    o.run_native_document_lifecycle_v1(
        &id,
        NativeDocumentLifecycleServiceV1::PreparedDisposal(&f),
        &mut r,
    )
    .unwrap();
    assert_eq!(
        o.lifecycle_state(&doc.document, &mut r).unwrap().phase,
        LifecyclePhase::DisposedRetained
    );
    assert_eq!(o.generation(), generation + 1);
    let n = TrustedRetainedDisposalResultV1 {
        context: &c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_allowed: Some(true),
        result_context_current: Some(true),
        historical_document_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    };
    let original = o.retained_disposal_result_v1(&id, &n, &mut r).unwrap();
    assert_eq!(original.document, doc.document);
    o.submit_retained_disposal_v1(&id, &plan, &a, &mut r)
        .unwrap();
    o.run_native_document_lifecycle_v1(
        &id,
        NativeDocumentLifecycleServiceV1::PreparedDisposal(&f),
        &mut r,
    )
    .unwrap();
    assert_eq!(
        o.retained_disposal_result_v1(&id, &n, &mut r).unwrap(),
        original
    );
    assert_eq!(o.generation(), generation + 1);
    o.retire_retained_disposal_v1(&id, &scratch::control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
#[test]
fn mismatched_native_facade_families_never_admit_execute_or_refund_work() {
    let (mut o, mut r) = scratch::owner();
    let c = scratch::context();
    let s = scratch::facts(&c);
    let l = relation_facts(&c);
    let d = disposal_facts(&c);
    let q = scratch::request(b"new");
    let live = relation_request(&mut o, &mut r, 0);
    let doc = o.read(&live.document, &mut r).unwrap().version;
    let plan = o.prepare_retained_disposal(&doc, 0, &mut r).unwrap();
    let sa = scratch::admission(&s);
    let la = relation_admission(&l);
    let da = disposal_admission(&d);
    let commands = [
        NativeDocumentLifecycleCommandV1::Scratch(&q),
        NativeDocumentLifecycleCommandV1::LiveRelation(&live),
        NativeDocumentLifecycleCommandV1::PreparedDisposal(&plan),
    ];
    let admissions = [
        NativeDocumentLifecycleAdmissionV1::Scratch(&sa),
        NativeDocumentLifecycleAdmissionV1::LiveRelation(&la),
        NativeDocumentLifecycleAdmissionV1::PreparedDisposal(&da),
    ];
    for (i, command) in commands.into_iter().enumerate() {
        for (j, admission) in admissions.into_iter().enumerate() {
            if i != j {
                let id = o.issue_invocation(&mut r).unwrap();
                let held = o.retention_usage();
                let generation = o.generation();
                assert_eq!(
                    o.submit_native_document_lifecycle_v1(&id, command, admission, &mut r),
                    Err(InvocationError::Unavailable)
                );
                assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
                assert_eq!(o.retention_usage(), held);
                assert_eq!(o.generation(), generation);
            }
        }
    }
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_scratch_v1(&id, &q, &sa, &mut r).unwrap();
    let held = o.retention_usage();
    let generation = o.generation();
    for service in [
        NativeDocumentLifecycleServiceV1::LiveRelation(&l),
        NativeDocumentLifecycleServiceV1::PreparedDisposal(&d),
    ] {
        assert_eq!(
            o.run_native_document_lifecycle_v1(&id, service, &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            o.invocation_state(&id, &mut r),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.retention_usage(), held);
        assert_eq!(o.generation(), generation);
    }
    let mut denied = scratch::facts(&c);
    denied.creation_allowed = Some(false);
    assert_eq!(
        o.run_native_document_lifecycle_v1(
            &id,
            NativeDocumentLifecycleServiceV1::Scratch(&denied),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    o.run_native_document_lifecycle_v1(&id, NativeDocumentLifecycleServiceV1::Scratch(&s), &mut r)
        .unwrap();
}
#[test]
fn native_facade_capture_is_optional_bounded_and_separate_from_inner_service() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, mut setup) = scratch::owner();
        let c = scratch::context();
        let f = scratch::facts(&c);
        let id = o.issue_invocation(&mut setup).unwrap();
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
        o.submit_native_document_lifecycle_v1(
            &id,
            NativeDocumentLifecycleCommandV1::Scratch(&scratch::request(b"PRIVATE")),
            NativeDocumentLifecycleAdmissionV1::Scratch(&scratch::admission(&f)),
            &mut r,
        )
        .unwrap();
        o.run_native_document_lifecycle_v1(
            &id,
            NativeDocumentLifecycleServiceV1::Scratch(&f),
            &mut r,
        )
        .unwrap();
        assert_eq!(o.lifecycle_usage().live, 1);
        assert_eq!(o.invocation_usage().issued_lifetime, 1);
        let mut counts = [0; 4];
        while let Some(e) = r.pop_front() {
            let i = match e.boundary {
                Boundary::DocumentLifecycleFacadeAdmission => Some(0),
                Boundary::DocumentLifecycleFacadeService => Some(1),
                Boundary::ScratchInvocationAdmission => Some(2),
                Boundary::ScratchInvocationService => Some(3),
                _ => None,
            };
            if let Some(i) = i {
                counts[i] += 1;
                assert_eq!(e.disposition, Disposition::Completed);
                assert_eq!(e.service_time.is_some(), mode == 2);
            }
            assert!(!format!("{e:?}").contains("PRIVATE"));
        }
        assert_eq!(
            counts,
            if mode == 1 || mode == 2 {
                [1, 1, 1, 1]
            } else {
                [0, 0, 0, 0]
            }
        );
    }
}
