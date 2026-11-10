#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)] // This consumer only needs the legacy loaded binding fixture.
mod bound;
#[path = "support/scratch_invocation_fixture.rs"]
#[allow(dead_code)] // Shared fixture also supplies independent scratch consumers.
mod fixture;
use vellum_engine::coordinator::*;
#[path = "explicit_disposal_behaviors/facade.rs"]
mod facade;
#[path = "explicit_disposal_behaviors/requests.rs"]
mod requests;
fn facts(c: &SettingsExecutionContextV1) -> TrustedRetainedDisposalFactsV1<'_> {
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
fn admission<'a>(f: &'a TrustedRetainedDisposalFactsV1<'a>) -> RetainedDisposalAdmissionV1<'a> {
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
fn request(source: &DocumentVersion, lifecycle: LifecycleState) -> ExplicitDisposalRequestV1 {
    ExplicitDisposalRequestV1 {
        source: source.clone(),
        annotation_generation: 0,
        lifecycle,
        serialization_generation: 1,
        binding_generation: None,
        source_disposition: DisposalDispositionV1::Retain,
        annotation_disposition: DisposalDispositionV1::Retain,
    }
}
fn release(c: &SettingsExecutionContextV1) -> TrustedRetainedDisposalResultV1<'_> {
    TrustedRetainedDisposalResultV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_allowed: Some(true),
        result_context_current: Some(true),
        historical_document_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    }
}
#[test]
fn explicit_disposal_seals_all_expectations_and_dispositions_without_upgrading_legacy_service() {
    for bits in 0..4 {
        let (mut o, mut r) = fixture::owner();
        let c = fixture::context();
        let f = facts(&c);
        let a = admission(&f);
        let gates = coverage();
        let doc = o.create(b"original", &mut r).unwrap();
        let life = o.lifecycle_state(&doc.document, &mut r).unwrap();
        let mut q = request(&doc, life);
        if bits & 1 != 0 {
            q.source_disposition = DisposalDispositionV1::Discard;
        }
        if bits & 2 != 0 {
            q.annotation_disposition = DisposalDispositionV1::Discard;
        }
        let legacy = o.prepare_retained_disposal(&doc, 0, &mut r).unwrap();
        let prepared = o.prepare_explicit_disposal_v1(&q, &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_explicit_disposal_v1(&id, &prepared, &a, &gates, &mut r)
            .unwrap();
        assert_eq!(
            o.submit_retained_disposal_v1(&id, &legacy, &a, &mut r),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(
            o.run_retained_disposal_v1(&id, &f, &mut r),
            Err(InvocationError::Unavailable)
        );
        o.run_explicit_disposal_v1(&id, &f, &gates, &mut r).unwrap();
        assert_eq!(
            o.lifecycle_state(&doc.document, &mut r).unwrap().phase,
            LifecyclePhase::DisposedRetained
        );
        let generation = o.generation();
        let result = o
            .retained_disposal_result_v1(&id, &release(&c), &mut r)
            .unwrap();
        assert_eq!(result.document, doc.document);
        assert_eq!(result.invocation, id);
        o.submit_explicit_disposal_v1(&id, &prepared, &a, &gates, &mut r)
            .unwrap();
        o.run_explicit_disposal_v1(&id, &f, &gates, &mut r).unwrap();
        assert_eq!(o.generation(), generation);
        o.retire_retained_disposal_v1(&id, &fixture::control(&c), &mut r)
            .unwrap();
    }
}

#[test]
fn explicit_native_binding_expectation_distinguishes_legacy_bound_zero_from_unbound() {
    let mut f = bound::Fixture::new();
    let life = f
        .owner
        .lifecycle_state(&f.doc.document, &mut f.recorder)
        .unwrap();
    let mut q = request(&f.doc, life);
    assert_eq!(
        f.owner
            .prepare_explicit_disposal_v1(&q, &mut f.recorder)
            .err(),
        Some(CoordinatorError::Stale)
    );
    q.binding_generation = Some(0);
    f.owner
        .prepare_explicit_disposal_v1(&q, &mut f.recorder)
        .unwrap();
    q.binding_generation = Some(1);
    assert_eq!(
        f.owner
            .prepare_explicit_disposal_v1(&q, &mut f.recorder)
            .err(),
        Some(CoordinatorError::Stale)
    );
    let (mut o, mut r) = fixture::owner();
    let doc = o.create(b"unbound", &mut r).unwrap();
    let mut q = request(&doc, o.lifecycle_state(&doc.document, &mut r).unwrap());
    o.prepare_explicit_disposal_v1(&q, &mut r).unwrap();
    q.binding_generation = Some(0);
    assert_eq!(
        o.prepare_explicit_disposal_v1(&q, &mut r).err(),
        Some(CoordinatorError::Stale)
    );
}

fn deny(g: &mut TrustedDisposalCoverageV1, index: usize, value: Option<bool>) {
    match index {
        0 => g.not_stopped = value,
        1 => g.companions_ready = value,
        2 => g.versions_available = value,
        3 => g.reply_ready = value,
        4 => g.lifecycle_allowed = value,
        5 => g.coverage_complete = value,
        6 => g.owner_control_allowed = value,
        7 => g.source_retention_ready = value,
        8 => g.source_discard_allowed = value,
        9 => g.annotation_retention_ready = value,
        10 => g.annotation_discard_allowed = value,
        11 => g.retention_complete = value,
        12 => g.effects_retained = value,
        13 => g.history_retained = value,
        14 => g.snapshots_retained = value,
        15 => g.providers_retained = value,
        16 => g.views_detachable = value,
        17 => g.memberships_detachable = value,
        18 => g.observer_scopes_fenced = value,
        _ => g.binding_coverage_complete = value,
    }
}
#[test]
fn explicit_disposal_requires_every_applicable_coverage_premise_at_admission_and_fresh_service() {
    for bits in 0..4 {
        for gate in 0..20 {
            for value in [None, Some(false)] {
                let (mut o, mut r) = fixture::owner();
                let c = fixture::context();
                let f = facts(&c);
                let a = admission(&f);
                let doc = o.create(b"original", &mut r).unwrap();
                let mut q = request(&doc, o.lifecycle_state(&doc.document, &mut r).unwrap());
                if bits & 1 != 0 {
                    q.source_disposition = DisposalDispositionV1::Discard;
                }
                if bits & 2 != 0 {
                    q.annotation_disposition = DisposalDispositionV1::Discard;
                }
                let applicable = match gate {
                    7 => bits & 1 == 0,
                    8 => bits & 1 != 0,
                    9 => bits & 2 == 0,
                    10 => bits & 2 != 0,
                    _ => true,
                };
                let p = o.prepare_explicit_disposal_v1(&q, &mut r).unwrap();
                let id = o.issue_invocation(&mut r).unwrap();
                let mut bad = coverage();
                deny(&mut bad, gate, value);
                let held = o.retention_usage();
                let generation = o.generation();
                let result = o.submit_explicit_disposal_v1(&id, &p, &a, &bad, &mut r);
                if applicable {
                    assert_eq!(result, Err(InvocationError::Unavailable));
                    assert_eq!(o.retention_usage(), held);
                    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
                } else {
                    assert_eq!(result, Ok(InvocationState::Pending));
                }
                o.submit_explicit_disposal_v1(&id, &p, &a, &coverage(), &mut r)
                    .unwrap();
                let held = o.retention_usage();
                let result = o.run_explicit_disposal_v1(&id, &f, &bad, &mut r);
                if applicable {
                    assert_eq!(result, Err(InvocationError::Unavailable));
                    assert_eq!(o.retention_usage(), held);
                    assert_eq!(o.generation(), generation);
                    assert_eq!(
                        o.invocation_state(&id, &mut r),
                        Ok(InvocationState::Pending)
                    );
                } else {
                    assert_eq!(result, Ok(InvocationState::Finished));
                }
                o.run_explicit_disposal_v1(&id, &f, &coverage(), &mut r)
                    .unwrap();
                assert_eq!(o.generation(), generation + 1);
                assert!(!o.is_faulted());
            }
        }
    }
}
#[test]
fn explicit_expectation_failures_do_not_prepare_or_refresh_any_native_owner() {
    let (mut o, mut r) = fixture::owner();
    let doc = o.create(b"original", &mut r).unwrap();
    let life = o.lifecycle_state(&doc.document, &mut r).unwrap();
    let generation = o.generation();
    let held = o.retention_usage();
    for field in 0..7 {
        let mut q = request(&doc, life);
        match field {
            0 => q.source.revision += 1,
            1 => q.annotation_generation += 1,
            2 => q.lifecycle.generation += 1,
            3 => q.lifecycle.pair_generation += 1,
            4 => q.lifecycle.phase = LifecyclePhase::DisposedRetained,
            5 => q.serialization_generation += 1,
            _ => q.binding_generation = Some(0),
        }
        assert_eq!(
            o.prepare_explicit_disposal_v1(&q, &mut r).err(),
            Some(CoordinatorError::Stale)
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
    }
    let (other, _) = fixture::owner();
    assert_eq!(
        other
            .prepare_explicit_disposal_v1(&request(&doc, life), &mut r)
            .err(),
        Some(CoordinatorError::Unavailable)
    );
}
#[test]
fn changed_disposition_cannot_replace_original_intent_and_stale_service_never_reprepares() {
    let (mut o, mut r) = fixture::owner();
    let c = fixture::context();
    let f = facts(&c);
    let a = admission(&f);
    let doc = o.create(b"original", &mut r).unwrap();
    let mut q = request(&doc, o.lifecycle_state(&doc.document, &mut r).unwrap());
    let p = o.prepare_explicit_disposal_v1(&q, &mut r).unwrap();
    q.source_disposition = DisposalDispositionV1::Discard;
    let changed = o.prepare_explicit_disposal_v1(&q, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_explicit_disposal_v1(&id, &p, &a, &coverage(), &mut r)
        .unwrap();
    assert_eq!(
        o.submit_explicit_disposal_v1(&id, &changed, &a, &coverage(), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.create(b"intervening publication", &mut r).unwrap();
    let generation = o.generation();
    o.run_explicit_disposal_v1(&id, &f, &coverage(), &mut r)
        .unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::RetainedDisposalRejected(CoordinatorError::Stale)
    ));
    assert_eq!(
        o.lifecycle_state(&doc.document, &mut r).unwrap().phase,
        LifecyclePhase::Live
    );
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    o.submit_explicit_disposal_v1(&id, &p, &a, &coverage(), &mut r)
        .unwrap();
    o.run_explicit_disposal_v1(&id, &f, &coverage(), &mut r)
        .unwrap();
    assert_eq!(o.generation(), generation);
    o.retire_retained_disposal_v1(&id, &fixture::control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
#[test]
fn legacy_admission_cannot_be_upgraded_and_protected_stop_does_not_need_discard_rights() {
    for explicit in [false, true] {
        let (mut o, mut r) = fixture::owner();
        let c = fixture::context();
        let f = facts(&c);
        let a = admission(&f);
        let doc = o.create(b"original", &mut r).unwrap();
        let life = o.lifecycle_state(&doc.document, &mut r).unwrap();
        let legacy = o.prepare_retained_disposal(&doc, 0, &mut r).unwrap();
        let p = o
            .prepare_explicit_disposal_v1(&request(&doc, life), &mut r)
            .unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        if explicit {
            o.submit_explicit_disposal_v1(&id, &p, &a, &coverage(), &mut r)
                .unwrap();
        } else {
            o.submit_retained_disposal_v1(&id, &legacy, &a, &mut r)
                .unwrap();
            assert_eq!(
                o.submit_explicit_disposal_v1(&id, &p, &a, &coverage(), &mut r),
                Err(InvocationError::IntentMismatch)
            );
            assert_eq!(
                o.run_explicit_disposal_v1(&id, &f, &coverage(), &mut r),
                Err(InvocationError::Unavailable)
            );
        }
        o.request_invocation_stop(
            &id,
            &TrustedInvocationControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                control_context_current: Some(true),
                control_scope_valid: Some(true),
                protected_control_ready: Some(true),
            },
            &mut r,
        )
        .unwrap();
        if explicit {
            let mut denied = coverage();
            denied.owner_control_allowed = Some(false);
            denied.source_retention_ready = None;
            o.run_explicit_disposal_v1(&id, &f, &denied, &mut r)
                .unwrap();
        } else {
            o.run_retained_disposal_v1(&id, &f, &mut r).unwrap();
        }
        assert!(matches!(
            o.invocation_outcome(&id, &mut r).unwrap(),
            InvocationOutcome::StoppedBeforePublication
        ));
        assert_eq!(o.lifecycle_state(&doc.document, &mut r).unwrap(), life);
        o.retire_retained_disposal_v1(&id, &fixture::control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
#[test]
fn explicit_disposal_instrumentation_is_optional_bounded_and_content_free() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation, Recorder,
    };
    for mode in 0..4 {
        let (mut o, mut setup) = fixture::owner();
        let c = fixture::context();
        let f = facts(&c);
        let a = admission(&f);
        let doc = o.create(b"PRIVATE", &mut setup).unwrap();
        let life = o.lifecycle_state(&doc.document, &mut setup).unwrap();
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
        let p = o
            .prepare_explicit_disposal_v1(&request(&doc, life), &mut r)
            .unwrap();
        o.submit_explicit_disposal_v1(&id, &p, &a, &coverage(), &mut r)
            .unwrap();
        o.run_explicit_disposal_v1(&id, &f, &coverage(), &mut r)
            .unwrap();
        let mut counts = [0; 3];
        while let Some(e) = r.pop_front() {
            let i = match e.boundary {
                Boundary::ExplicitDisposalPreparation => Some(0),
                Boundary::RetainedDisposalAdmission => Some(1),
                Boundary::RetainedDisposalService => Some(2),
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
                [1, 1, 1]
            } else {
                [0, 0, 0]
            }
        );
    }
}
