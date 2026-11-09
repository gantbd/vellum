use super::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
fn preparation_count(r: &mut Recorder) -> usize {
    let mut count = 0;
    while let Some(e) = r.pop_front() {
        if e.boundary == Boundary::ExplicitDisposalPreparation {
            count += 1;
        }
    }
    count
}
#[test]
fn data_disposal_retry_after_publication_and_forgetting_never_requires_a_caller_plan() {
    let (mut o, mut r) = fixture::owner();
    let c = fixture::context();
    let f = facts(&c);
    let a = admission(&f);
    let g = coverage();
    let doc = o.create(b"original", &mut r).unwrap();
    let q = request(&doc, o.lifecycle_state(&doc.document, &mut r).unwrap());
    let id = o.issue_invocation(&mut r).unwrap();
    assert_eq!(
        o.submit_explicit_disposal_request_v1(&id, &q, &a, &g, &mut r),
        Ok(InvocationState::Pending)
    );
    o.run_explicit_disposal_v1(&id, &f, &g, &mut r).unwrap();
    let original = o
        .retained_disposal_result_v1(&id, &release(&c), &mut r)
        .unwrap();
    let generation = o.generation();
    let held = o.retention_usage();
    assert_eq!(
        o.submit_explicit_disposal_request_v1(&id, &q, &a, &g, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        o.retained_disposal_result_v1(&id, &release(&c), &mut r)
            .unwrap(),
        original
    );
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        o.submit_explicit_disposal_request_v1(&id, &q, &a, &g, &mut r),
        Ok(InvocationState::Finished)
    );
    o.run_explicit_disposal_v1(&id, &f, &g, &mut r).unwrap();
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage(), held);
    o.retire_retained_disposal_v1(&id, &fixture::control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}

#[test]
fn data_disposal_pending_retry_never_refreshes_original_root_or_rechecks_later_metadata() {
    let (mut o, mut setup) = fixture::owner();
    let c = fixture::context();
    let f = facts(&c);
    let a = admission(&f);
    let g = coverage();
    let doc = o.create(b"original", &mut setup).unwrap();
    let q = request(&doc, o.lifecycle_state(&doc.document, &mut setup).unwrap());
    let id = o.issue_invocation(&mut setup).unwrap();
    let mut r = Recorder::try_with_capacity(128).unwrap();
    o.submit_explicit_disposal_request_v1(&id, &q, &a, &g, &mut r)
        .unwrap();
    assert_eq!(preparation_count(&mut r), 1);
    o.create(b"intervening publication", &mut setup).unwrap();
    let generation = o.generation();
    let held = o.retention_usage();
    assert_eq!(
        o.submit_explicit_disposal_request_v1(&id, &q, &a, &g, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(preparation_count(&mut r), 0);
    assert_eq!(o.retention_usage(), held);
    o.run_explicit_disposal_v1(&id, &f, &g, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::RetainedDisposalRejected(CoordinatorError::Stale)
    ));
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.lifecycle_state(&doc.document, &mut r).unwrap().phase,
        LifecyclePhase::Live
    );
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    o.submit_explicit_disposal_request_v1(&id, &q, &a, &g, &mut r)
        .unwrap();
    assert_eq!(preparation_count(&mut r), 0);
}
#[test]
fn data_disposal_complete_intent_changes_refuse_without_preparing_or_replacing_the_original() {
    let (mut o, mut setup) = fixture::owner();
    let c = fixture::context();
    let f = facts(&c);
    let a = admission(&f);
    let g = coverage();
    let doc = o.create(b"original", &mut setup).unwrap();
    let q = request(&doc, o.lifecycle_state(&doc.document, &mut setup).unwrap());
    let id = o.issue_invocation(&mut setup).unwrap();
    o.submit_explicit_disposal_request_v1(&id, &q, &a, &g, &mut setup)
        .unwrap();
    let held = o.retention_usage();
    let generation = o.generation();
    let mut r = Recorder::try_with_capacity(128).unwrap();
    for field in 0..9 {
        let mut changed = q.clone();
        match field {
            0 => changed.source.revision += 1,
            1 => changed.annotation_generation += 1,
            2 => changed.lifecycle.generation += 1,
            3 => changed.lifecycle.pair_generation += 1,
            4 => changed.lifecycle.phase = LifecyclePhase::DisposedRetained,
            5 => changed.serialization_generation += 1,
            6 => changed.binding_generation = Some(0),
            7 => changed.source_disposition = DisposalDispositionV1::Discard,
            _ => changed.annotation_disposition = DisposalDispositionV1::Discard,
        }
        assert_eq!(
            o.submit_explicit_disposal_request_v1(&id, &changed, &a, &g, &mut r),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(preparation_count(&mut r), 0);
        assert_eq!(
            o.invocation_state(&id, &mut r),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.retention_usage(), held);
        assert_eq!(o.generation(), generation);
    }
    o.run_explicit_disposal_v1(&id, &f, &g, &mut r).unwrap();
    assert_eq!(o.generation(), generation + 1);
}
#[test]
fn data_disposal_authority_ceiling_and_limit_refusals_precede_fresh_preparation() {
    let (mut o, mut setup) = fixture::owner();
    let c = fixture::context();
    let doc = o.create(b"original", &mut setup).unwrap();
    let mut q = request(&doc, o.lifecycle_state(&doc.document, &mut setup).unwrap());
    q.serialization_generation += 1;
    for bad in 0..5 {
        let id = o.issue_invocation(&mut setup).unwrap();
        let mut f = facts(&c);
        let mut g = coverage();
        if bad == 0 {
            f.authorized = None;
        }
        if bad == 1 {
            g.binding_coverage_complete = Some(false);
        }
        let mut a = admission(&f);
        if bad == 2 {
            a.publication_allowed = None;
        }
        if bad == 3 {
            a.limits.max_capture_bytes = 0;
        }
        if bad == 4 {
            a.limits.max_result_bytes = 0;
        }
        let mut r = Recorder::try_with_capacity(128).unwrap();
        let held = o.retention_usage();
        let generation = o.generation();
        assert!(
            o.submit_explicit_disposal_request_v1(&id, &q, &a, &g, &mut r)
                .is_err()
        );
        assert_eq!(preparation_count(&mut r), 0);
        assert_eq!(o.retention_usage(), held);
        assert_eq!(o.generation(), generation);
        assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        assert!(!o.is_faulted());
    }
    let id = o.issue_invocation(&mut setup).unwrap();
    let f = facts(&c);
    let held = o.retention_usage();
    let mut r = Recorder::try_with_capacity(128).unwrap();
    assert_eq!(
        o.submit_explicit_disposal_request_v1(&id, &q, &admission(&f), &coverage(), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(preparation_count(&mut r), 1);
    assert_eq!(o.retention_usage(), held);
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}
#[test]
fn data_facade_disposal_is_one_native_admission_with_separate_execution_and_historical_release() {
    use vellum_engine::instrumentation::{Disposition, NativeTimingProfileV1, Observation};
    for mode in 0..4 {
        let (mut o, mut setup) = fixture::owner();
        let c = fixture::context();
        let f = facts(&c);
        let a = admission(&f);
        let g = coverage();
        let doc = o.create(b"PRIVATE", &mut setup).unwrap();
        let q = request(&doc, o.lifecycle_state(&doc.document, &mut setup).unwrap());
        let id = o.issue_invocation(&mut setup).unwrap();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
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
        let generation = o.generation();
        o.submit_native_document_lifecycle_request_v1(
            &id,
            NativeDocumentLifecycleRequestV1::ExplicitDisposal(&q),
            NativeDocumentLifecycleAdmissionV2::ExplicitDisposal {
                native: &a,
                coverage: &g,
            },
            &mut r,
        )
        .unwrap();
        assert_eq!(o.generation(), generation);
        o.run_native_document_lifecycle_v2(
            &id,
            NativeDocumentLifecycleServiceV2::ExplicitDisposal {
                native: &f,
                coverage: &g,
            },
            &mut r,
        )
        .unwrap();
        assert_eq!(
            o.retained_disposal_result_v1(&id, &release(&c), &mut setup)
                .unwrap()
                .document,
            doc.document
        );
        let mut counts = [0; 5];
        while let Some(e) = r.pop_front() {
            let i = match e.boundary {
                Boundary::DocumentLifecycleFacadeAdmission => Some(0),
                Boundary::RetainedDisposalAdmission => Some(1),
                Boundary::ExplicitDisposalPreparation => Some(2),
                Boundary::DocumentLifecycleFacadeService => Some(3),
                Boundary::RetainedDisposalService => Some(4),
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
                [1, 1, 1, 1, 1]
            } else {
                [0, 0, 0, 0, 0]
            }
        );
        o.forget_invocation_outcome(&id, &mut setup).unwrap();
        o.submit_native_document_lifecycle_request_v1(
            &id,
            NativeDocumentLifecycleRequestV1::ExplicitDisposal(&q),
            NativeDocumentLifecycleAdmissionV2::ExplicitDisposal {
                native: &a,
                coverage: &g,
            },
            &mut r,
        )
        .unwrap();
        assert_eq!(preparation_count(&mut r), 0);
        assert_eq!(o.invocation_usage().issued_lifetime, 1);
        o.retire_retained_disposal_v1(&id, &fixture::control(&c), &mut setup)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
