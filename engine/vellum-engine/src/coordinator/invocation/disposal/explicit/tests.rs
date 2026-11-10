use super::super::tests::{admission, context, facts, setup};
use super::*;
fn coverage() -> TrustedDisposalCoverageV1 {
    TrustedDisposalCoverageV1 {
        not_stopped: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        lifecycle_allowed: Some(true),
        coverage_complete: Some(true),
        owner_control_allowed: Some(true),
        source_retention_ready: Some(true),
        source_discard_allowed: Some(true),
        annotation_retention_ready: Some(true),
        annotation_discard_allowed: Some(true),
        retention_complete: Some(true),
        effects_retained: Some(true),
        history_retained: Some(true),
        snapshots_retained: Some(true),
        providers_retained: Some(true),
        views_detachable: Some(true),
        memberships_detachable: Some(true),
        observer_scopes_fenced: Some(true),
        binding_coverage_complete: Some(true),
    }
}
fn prepare(
    o: &SourceCoordinator,
    p: &PreparedDisposal,
    r: &mut Recorder,
) -> PreparedExplicitDisposalV1 {
    let d = o.read(p.document(), r).unwrap().version;
    o.prepare_explicit_disposal_v1(
        &ExplicitDisposalRequestV1 {
            source: d,
            annotation_generation: 0,
            lifecycle: o.lifecycle_state(p.document(), r).unwrap(),
            serialization_generation: 1,
            binding_generation: None,
            source_disposition: DisposalDispositionV1::Discard,
            annotation_disposition: DisposalDispositionV1::Discard,
        },
        r,
    )
    .unwrap()
}
#[test]
fn explicit_capture_is_funded_by_exact_actual_parent_headroom_before_copy() {
    let c = context();
    let f = facts(&c);
    let mut a = admission(&f);
    a.limits.max_capture_bytes = RetainedDisposalLimitsV1::required_capture_bytes(&c).unwrap();
    for short in [false, true] {
        let (mut o, id, legacy, mut r) =
            setup(a.limits.max_capture_bytes + 71 - usize::from(short));
        let p = prepare(&o, &legacy, &mut r);
        let _other = o.retention.reserve(71).unwrap();
        let held = o.retention_usage();
        let result = o.submit_explicit_disposal_v1(&id, &p, &a, &coverage(), &mut r);
        if short {
            assert_eq!(result, Err(InvocationError::Limit));
            assert_eq!(o.retention_usage(), held);
        } else {
            assert_eq!(result, Ok(InvocationState::Pending));
            assert_eq!(
                o.retention_usage().reserved,
                71 + a.limits.max_capture_bytes
            );
        }
    }
}
#[test]
fn data_request_reserves_actual_parent_before_metadata_validation_and_refunds_failed_preparation() {
    let c = context();
    let f = facts(&c);
    let a = admission(&f);
    for short in [false, true] {
        let (mut o, id, legacy, mut setup) = setup(a.limits.max_capture_bytes - usize::from(short));
        let mut q = prepare(&o, &legacy, &mut setup).request;
        q.serialization_generation += 1;
        let mut r = Recorder::try_with_capacity(64).unwrap();
        let held = o.retention_usage();
        let generation = o.generation();
        assert_eq!(
            o.submit_explicit_disposal_request_v1(&id, &q, &a, &coverage(), &mut r),
            Err(if short {
                InvocationError::Limit
            } else {
                InvocationError::Unavailable
            })
        );
        assert_eq!(o.retention_usage(), held);
        assert_eq!(o.generation(), generation);
        assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        assert!(!o.is_faulted());
        let mut count = 0;
        while let Some(e) = r.pop_front() {
            if e.boundary == Boundary::ExplicitDisposalPreparation {
                count += 1;
            }
        }
        assert_eq!(count, usize::from(!short));
    }
}
#[test]
fn data_preparation_maps_only_declared_refusals_and_fences_surprise_products() {
    use crate::resource::ResourceError;
    for e in [
        CoordinatorError::Unavailable,
        CoordinatorError::Stale,
        CoordinatorError::Resource(ResourceError::Retained),
    ] {
        assert_eq!(
            super::super::preparation_error(e),
            InvocationError::Unavailable
        );
    }
    assert_eq!(
        super::super::preparation_error(CoordinatorError::Limit),
        InvocationError::Limit
    );
    assert_eq!(
        super::super::preparation_error(CoordinatorError::Capacity),
        InvocationError::Capacity
    );
    for e in [
        CoordinatorError::Faulted,
        CoordinatorError::Resource(ResourceError::Faulted),
        CoordinatorError::History(HistoryError::Faulted),
        CoordinatorError::DuplicateParticipant,
        CoordinatorError::Source(SourceError::AmbiguousBom),
    ] {
        assert_eq!(super::super::preparation_error(e), InvocationError::Faulted);
    }
}
#[test]
fn actual_publisher_checks_explicit_serialization_before_any_lifecycle_installation() {
    let (mut o, id, legacy, mut r) = setup(32768);
    let p = prepare(&o, &legacy, &mut r);
    let c = context();
    let f = facts(&c);
    o.submit_explicit_disposal_v1(&id, &p, &admission(&f), &coverage(), &mut r)
        .unwrap();
    o.invocations.slots[id.slot].as_mut().unwrap().phase = Phase::Running;
    // Inject untracked private metadata damage to exercise the actual publisher,
    // not the earlier global-publication stale check.
    o.documents[p.request.source.document.slot]
        .as_mut()
        .unwrap()
        .serialization
        .generation += 1;
    let generation = o.generation();
    let held = o.retention_usage();
    assert_eq!(
        o.commit_retained_disposal_for_invocation(p.plan.clone(), Some(&id), &mut r),
        Err(CoordinatorError::Faulted)
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage(), held);
    assert_eq!(
        o.retained_document(&p.request.source.document)
            .unwrap()
            .lifecycle
            .phase,
        LifecyclePhase::Live
    );
    assert!(
        o.invocations.slots[id.slot]
            .as_ref()
            .unwrap()
            .disposal_hold
            .as_ref()
            .unwrap()
            .candidate
            .is_none()
    );
}
#[test]
fn independent_explicit_seal_detects_hold_expectation_corruption_before_publication() {
    let (mut o, id, legacy, mut r) = setup(32768);
    let p = prepare(&o, &legacy, &mut r);
    let c = context();
    let f = facts(&c);
    o.submit_explicit_disposal_v1(&id, &p, &admission(&f), &coverage(), &mut r)
        .unwrap();
    o.invocations.slots[id.slot]
        .as_mut()
        .unwrap()
        .disposal_hold
        .as_mut()
        .unwrap()
        .explicit
        .as_mut()
        .unwrap()
        .binding_generation = Some(0);
    let generation = o.generation();
    let held = o.retention_usage();
    assert_eq!(
        o.run_explicit_disposal_v1(&id, &f, &coverage(), &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage(), held);
}
#[test]
fn explicit_discard_retains_evidence_and_interrupted_publication_never_claims_erasure_or_retry() {
    for cut in 0..=5 {
        let (mut o, id, legacy, mut r) = setup(32768);
        let p = prepare(&o, &legacy, &mut r);
        let c = context();
        let f = facts(&c);
        let doc = p.request.source.document.clone();
        let _other = o.retention.reserve(71).unwrap();
        o.submit_explicit_disposal_v1(&id, &p, &admission(&f), &coverage(), &mut r)
            .unwrap();
        let held = o.retention_usage();
        let mut r = Recorder::try_with_capacity(128).unwrap();
        if cut < 5 {
            o.publication_fault_after = Some(cut);
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o
                    .run_native_document_lifecycle_v2(
                        &id,
                        NativeDocumentLifecycleServiceV2::ExplicitDisposal {
                            native: &f,
                            coverage: &coverage()
                        },
                        &mut r
                    )))
                .is_err()
            );
            assert!(o.is_faulted());
            assert!(matches!(
                o.invocations.slots[id.slot].as_ref().unwrap().phase,
                Phase::Running
            ));
        } else {
            o.run_native_document_lifecycle_v2(
                &id,
                NativeDocumentLifecycleServiceV2::ExplicitDisposal {
                    native: &f,
                    coverage: &coverage(),
                },
                &mut r,
            )
            .unwrap();
        }
        assert_eq!(o.retention_usage(), held);
        assert_eq!(
            o.documents[doc.slot].as_ref().unwrap().source.text(),
            "private content"
        );
        let ctl = InvocationObservationPolicyControlV1 {
            context: &c,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        o.retire_retained_disposal_v1(&id, &ctl, &mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, 71);
        assert_eq!(
            o.retire_retained_disposal_v1(&id, &ctl, &mut r),
            Err(InvocationError::Unavailable)
        );
        let mut counts = [0; 2];
        while let Some(e) = r.pop_front() {
            let i = match e.boundary {
                Boundary::DocumentLifecycleFacadeService => Some(0),
                Boundary::RetainedDisposalService => Some(1),
                _ => None,
            };
            if let Some(i) = i {
                counts[i] += 1;
                assert_eq!(
                    e.disposition,
                    if cut < 5 {
                        crate::instrumentation::Disposition::Unknown
                    } else {
                        crate::instrumentation::Disposition::Completed
                    }
                );
            }
        }
        assert_eq!(counts, [1, 1]);
    }
}
