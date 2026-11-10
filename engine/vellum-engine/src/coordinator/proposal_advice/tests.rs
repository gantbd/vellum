use super::*;
use crate::attachment::AttachmentTarget;
fn owner() -> (SourceCoordinator, Recorder, DocumentVersion) {
    owner_with_parent(8 * 16384 + size_of::<CompoundAdviceLimitsV1>())
}
pub(in crate::coordinator) fn owner_with_parent(
    parent: usize,
) -> (SourceCoordinator, Recorder, DocumentVersion) {
    let mut o = SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 16,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 16,
                max_edit_payload_bytes: 4096,
            },
        },
        invocation: Some(InvocationLimits {
            max_lifetime_issued: 8,
            max_retained: 8,
            max_intent_bytes: 16384,
        }),
        annotations: Some(AnnotationLimits {
            max_issued: 16,
            max_records: 16,
            max_body_bytes: 1024,
            max_retained_body_bytes: 8192,
            max_mapping_steps: 8192,
            max_live_snapshots: 8,
            max_snapshot_bytes: 65536,
        }),
        history: Some(HistoryLimits {
            max_retained_groups: 16,
            max_journal_bytes: 65536,
            max_pins: 8,
        }),
        resources: None,
        search: None,
    });
    let mut r = Recorder::disabled();
    o.configure_retention_budget(parent, &mut r).unwrap();
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    let d = o.create(b"body", &mut r).unwrap();
    (o, r, d)
}
pub(in crate::coordinator) fn plan(d: &DocumentVersion, creates: usize) -> CompoundIntent {
    CompoundIntent {
        source: SourceIntent {
            writes: vec![DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 4,
                    expected: "body".into(),
                    inserted: "new".into(),
                }],
            }],
            reads: vec![],
        },
        annotations: (0..creates)
            .map(|_| AnnotationMutation::Create {
                target: SourceTarget {
                    source: d.clone(),
                    target: AttachmentTarget::Document,
                    basis: TargetBasis::Before,
                },
                body: "private comment".into(),
            })
            .collect(),
    }
}
pub(in crate::coordinator) fn request<'a>(
    plan: &'a CompoundIntent,
    context: &'a SettingsExecutionContextV1,
) -> CompoundAdviceRequestV1<'a> {
    CompoundAdviceRequestV1 {
        context,
        policy: "approved",
        profile: NATIVE_COMPOUND_ADVICE_PROFILE_V1,
        plan,
        reads: None,
        scopes: None,
        limits: CompoundAdviceLimitsV1 {
            max_request_bytes: 16384,
            max_label_bytes: 128,
            max_reply_bytes: size_of::<CompoundAdviceReplyV1>(),
        },
    }
}
pub(in crate::coordinator) fn facts<'a>(
    q: &'a CompoundAdviceRequestV1<'a>,
    basis: &'a CompoundAdviceBasisV1<'a>,
) -> TrustedCompoundAdviceFactsV1<'a> {
    TrustedCompoundAdviceFactsV1 {
        request: q,
        basis,
        context: q.context,
        policy: q.policy,
        domain_bound: Some(true),
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        proposal_readable: Some(true),
        validation_observable: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
    }
}
fn advice(value: bool) -> CompoundAdviceReplyV1 {
    CompoundAdviceReplyV1::Advice {
        profile: NATIVE_COMPOUND_ADVICE_PROFILE_V1,
        checks_satisfied: value,
    }
}
#[test]
fn private_mixed_candidates_spend_no_identity_history_storage_or_invocation_and_match_publication_root()
 {
    let (mut o, mut r, d) = owner();
    let plan = plan(&d, 2);
    let context = crate::coordinator::serialization::tests::context();
    let q = request(&plan, &context);
    let basis = o.capture_compound_advice_basis_v1(&q, &mut r).unwrap();
    let before = (
        o.generation(),
        o.annotation_usage(),
        o.history_usage(),
        o.invocation_usage(),
        o.retention_usage(),
        o.semantic_root_usage(&mut r).unwrap(),
    );
    let capacities = (
        o.annotations.records.capacity(),
        o.advice_history_capacity_for_test(),
    );
    for _ in 0..3 {
        assert_eq!(
            o.query_compound_advice_v1(&q, &facts(&q, &basis), &mut r),
            advice(true)
        );
    }
    assert_eq!(
        (
            o.annotations.records.capacity(),
            o.advice_history_capacity_for_test()
        ),
        capacities
    );
    assert_eq!(
        (
            o.generation(),
            o.annotation_usage(),
            o.history_usage(),
            o.invocation_usage(),
            o.retention_usage(),
            o.semantic_root_usage(&mut r).unwrap()
        ),
        before
    );
    let private = o
        .prepare_compound_advice_candidate(
            &plan.source.writes,
            &[],
            &plan.annotations,
            None,
            None,
            &mut r,
        )
        .unwrap();
    let decision = o
        .decide_transaction_candidate(private.prepared(), true, &mut r)
        .unwrap();
    let history = o
        .decide_history(
            private.prepared(),
            o.generation() + 1,
            &decision.annotations.journals,
        )
        .unwrap();
    let projected = o
        .annotations
        .root_bytes_after_advice(&decision.annotations, &history, &o.documents, &o.namespace)
        .unwrap();
    let p = o
        .prepare_compound(&plan.source.writes, &[], &plan.annotations, &mut r)
        .unwrap();
    o.commit(p, &mut r).unwrap();
    assert_eq!(o.annotation_usage().issued_lifetime, 2);
    assert_eq!(o.history_usage().retained_groups, 1);
    assert_eq!(
        projected,
        o.semantic_root_usage(&mut r).unwrap().annotations_bytes
    );
    assert_eq!(
        o.query_compound_advice_v1(&q, &facts(&q, &basis), &mut r),
        CompoundAdviceReplyV1::Unavailable
    );
}
#[test]
fn invalid_candidate_is_false_only_after_complete_permission_and_has_no_partial_identity_claims() {
    let (mut o, mut r, d) = owner();
    let mut p = plan(&d, 2);
    if let AnnotationMutation::Create { body, .. } = &mut p.annotations[1] {
        *body = "x".repeat(1025);
    }
    let context = crate::coordinator::serialization::tests::context();
    let q = request(&p, &context);
    let basis = o.capture_compound_advice_basis_v1(&q, &mut r).unwrap();
    let before = (
        o.annotation_usage(),
        o.history_usage(),
        o.generation(),
        o.retention_usage(),
    );
    for i in 0..11 {
        for value in [None, Some(false)] {
            let mut f = facts(&q, &basis);
            let slots = [
                &mut f.domain_bound,
                &mut f.authorized,
                &mut f.scope_allowed,
                &mut f.continuous,
                &mut f.live,
                &mut f.not_stopped,
                &mut f.interpretation_current,
                &mut f.proposal_readable,
                &mut f.validation_observable,
                &mut f.resources_ready,
                &mut f.reply_ready,
            ];
            *slots.into_iter().nth(i).unwrap() = value;
            assert_eq!(
                o.query_compound_advice_v1(&q, &f, &mut r),
                CompoundAdviceReplyV1::Unavailable
            );
        }
    }
    assert_eq!(
        o.query_compound_advice_v1(&q, &facts(&q, &basis), &mut r),
        advice(false)
    );
    assert_eq!(
        (
            o.annotation_usage(),
            o.history_usage(),
            o.generation(),
            o.retention_usage()
        ),
        before
    );
}
#[test]
fn original_request_root_and_owner_basis_are_not_interchangeable_or_refreshable() {
    let (mut o, mut r, d) = owner();
    let p = plan(&d, 0);
    let context = crate::coordinator::serialization::tests::context();
    let q = request(&p, &context);
    let other = request(&p, &context);
    let b = o.capture_compound_advice_basis_v1(&q, &mut r).unwrap();
    assert_eq!(
        o.query_compound_advice_v1(&other, &facts(&q, &b), &mut r),
        CompoundAdviceReplyV1::Unavailable
    );
    let (peer, _, _) = owner();
    let foreign = peer.capture_compound_advice_basis_v1(&q, &mut r).unwrap();
    assert_eq!(
        o.query_compound_advice_v1(&q, &facts(&q, &foreign), &mut r),
        CompoundAdviceReplyV1::Unavailable
    );
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                target: SourceTarget {
                    source: d,
                    target: AttachmentTarget::Document,
                    basis: TargetBasis::Before,
                },
                body: "claim".into(),
            }],
            &mut r,
        )
        .unwrap();
    drop(p);
    assert_eq!(
        o.query_compound_advice_v1(&q, &facts(&q, &b), &mut r),
        CompoundAdviceReplyV1::Unavailable
    );
}
#[test]
fn explicit_reads_and_scopes_are_current_predicates_not_publication_certificates() {
    let (mut o, mut r, d) = owner();
    let p = plan(&d, 0);
    let context = crate::coordinator::serialization::tests::context();
    let scopes = [SourceOperationScope {
        expected: d.clone(),
        replacement_ranges: vec![ByteRange { start: 0, end: 4 }],
        insertion_sites: vec![],
    }];
    let reads = ReadDependencyRequestV3 {
        predicates: vec![ReadDependencyV3::Base(ReadDependencyV2::Base(
            ReadDependencyV1::Source(d.clone()),
        ))],
        budget: ReadDependencyBudget {
            max_predicates: 4,
            max_work: 128,
        },
    };
    let mut q = request(&p, &context);
    q.reads = Some(&reads);
    q.scopes = Some((
        &scopes,
        SourceScopeBudget {
            max_scopes: 2,
            max_records: 4,
            max_work: 128,
        },
    ));
    let b = o.capture_compound_advice_basis_v1(&q, &mut r).unwrap();
    assert_eq!(
        o.query_compound_advice_v1(&q, &facts(&q, &b), &mut r),
        advice(true)
    );
    o.set_read_only(&d.document, true, &mut r).unwrap();
    let b = o.capture_compound_advice_basis_v1(&q, &mut r).unwrap();
    assert_eq!(
        o.query_compound_advice_v1(&q, &facts(&q, &b), &mut r),
        advice(false)
    );
}
#[test]
fn capacity_refusal_and_capture_modes_do_not_change_advice_or_lease_accounting() {
    for mode in 0..4 {
        let (mut o, _, d) = owner();
        let p = plan(&d, 1);
        let context = crate::coordinator::serialization::tests::context();
        let q = request(&p, &context);
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(
                crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
            );
        }
        let b = o.capture_compound_advice_basis_v1(&q, &mut r).unwrap();
        let debt = o.retention_usage();
        assert_eq!(
            o.query_compound_advice_v1(&q, &facts(&q, &b), &mut r),
            advice(true)
        );
        assert_eq!(o.retention_usage(), debt);
        let mut short = request(&p, &context);
        short.limits.max_request_bytes = 1;
        let b = o.capture_compound_advice_basis_v1(&short, &mut r).unwrap();
        assert_eq!(
            o.query_compound_advice_v1(&short, &facts(&short, &b), &mut r),
            CompoundAdviceReplyV1::Unavailable
        );
        assert_eq!(o.retention_usage(), debt);
        assert!(!o.is_faulted());
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if mode == 1 || mode == 2 {
            let releases: Vec<_> = events
                .iter()
                .filter(|e| e.boundary == Boundary::CompoundAdviceRelease)
                .collect();
            assert_eq!(releases.len(), 2);
            assert!(
                releases
                    .iter()
                    .all(|e| e.service_time.is_some() == (mode == 2))
            );
        }
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
    }
}
#[test]
fn unexpected_private_annotation_invariant_failure_is_not_a_negative_verdict() {
    let (mut o, mut r, d) = owner();
    let p = o
        .prepare_compound(&[], &[], &plan(&d, 1).annotations, &mut r)
        .unwrap();
    o.commit(p, &mut r).unwrap();
    let d = o.read(&d.document, &mut r).unwrap().version;
    let id = o.annotations.records[0].as_ref().unwrap().value.id.clone();
    let mut p = plan(&d, 0);
    p.source.writes.clear();
    p.annotations.push(AnnotationMutation::UpdateBody {
        annotation: id,
        expected_content: 1,
        body: "updated".into(),
    });
    o.corrupt_advice_annotation_registry_for_test();
    let context = crate::coordinator::serialization::tests::context();
    let q = request(&p, &context);
    let b = o.capture_compound_advice_basis_v1(&q, &mut r).unwrap();
    let debt = o.retention_usage();
    let generation = o.generation();
    assert_eq!(
        o.query_compound_advice_v1(&q, &facts(&q, &b), &mut r),
        CompoundAdviceReplyV1::Unavailable
    );
    assert!(o.is_faulted());
    assert_eq!(o.retention_usage(), debt);
    assert_eq!(o.generation(), generation);
}
