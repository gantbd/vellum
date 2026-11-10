use super::*;

#[test]
fn expanded_observer_fences_only_receipts_after_a_native_disposal_seal_fault() {
    let (mut o, id, p, mut r) = setup(32768);
    let c = context();
    let policy = InvocationObservationPolicyV1 {
        recipient: c.recipient.clone(),
        epoch: 7,
        grant: true,
        publication: true,
        control: true,
        effect: true,
        reasons: true,
        details: false,
        recovery: true,
    };
    let f = TrustedInvocationPolicyFactsV1 {
        context: &c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        resources_ready: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        preparation_reserved: Some(true),
        registry_ready: Some(true),
        retention_ready: Some(true),
    };
    o.bind_invocation_observation_policy_v1(
        &id,
        &c,
        &policy,
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &f,
        &mut r,
    )
    .unwrap();
    o.submit_retained_disposal_v1(&id, &p, &admission(&facts(&c)), &mut r)
        .unwrap();
    let mut limits = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 0,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: size_of::<InvocationObservationReferenceV1>()
            .max(size_of::<InvocationControlReferenceV1>()),
        max_reply_bytes: InvocationReceiptV2::required_reply_bytes(512).unwrap(),
        max_control_reply_bytes: size_of::<InvocationControlObservationV1>(),
        max_held_bytes: 0,
    };
    limits.max_held_bytes = limits.required_hold_bytes().unwrap();
    let mut observer = o
        .open_invocation_observation_with_receipts_v2(
            &id,
            &c,
            &policy,
            limits,
            InvocationReceiptAdmissionV1 {
                observation: &f,
                capabilities: InvocationReceiptCapabilitiesV1 {
                    lifecycle: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    o.run_retained_disposal_v1(&id, &facts(&c), &mut r).unwrap();
    let current = TrustedInvocationObservationReleaseV1 {
        context: &c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        control_ready: Some(true),
    };
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &current,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let receipt = TrustedInvocationReceiptReleaseV1 {
        observation: &ordinary,
        operation_receipt_allowed: Some(true),
    };
    let marker = std::mem::replace(
        &mut o.invocations.slots[id.slot]
            .as_mut()
            .unwrap()
            .disposal_hold
            .as_mut()
            .unwrap()
            .marker,
        Arc::new(()),
    );
    let before = o.retention_usage();
    let generation = o.generation();
    assert!(matches!(
        o.query_invocation_receipt_v2(
            &mut observer,
            InvocationReceiptRequestV2::RetainedDisposal(&release(&c)),
            &policy,
            &receipt,
            &mut r
        ),
        Err(InvocationError::Faulted)
    ));
    assert!(!o.faulted);
    assert_eq!(o.retention_usage(), before);
    assert_eq!(o.generation(), generation);
    // Repair only the injected private corruption; the receipt facility stays closed.
    o.invocations.slots[id.slot]
        .as_mut()
        .unwrap()
        .disposal_hold
        .as_mut()
        .unwrap()
        .marker = marker;
    assert!(matches!(
        o.query_invocation_receipt_v2(
            &mut observer,
            InvocationReceiptRequestV2::RetainedDisposal(&release(&c)),
            &policy,
            &receipt,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    ));
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut observer, &policy, &current, &mut r)
            .unwrap()
            .phase,
        InvocationObservationPhaseV1::Terminal
    );
    o.retire_invocation_observation_v1(&mut observer, &control(&c), &mut r)
        .unwrap();
    o.retire_retained_disposal_v1(&id, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}

pub(super) fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "private authority".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "native".into(),
        continuity: "c".into(),
    }
}
pub(super) fn facts(c: &SettingsExecutionContextV1) -> TrustedRetainedDisposalFactsV1<'_> {
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
pub(super) fn admission<'a>(
    f: &'a TrustedRetainedDisposalFactsV1<'a>,
) -> RetainedDisposalAdmissionV1<'a> {
    RetainedDisposalAdmissionV1 {
        facts: f,
        limits: RetainedDisposalLimitsV1 {
            max_capture_bytes: 2048,
            max_result_bytes: 512,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_allowed: Some(true),
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
fn control(c: &SettingsExecutionContextV1) -> InvocationObservationPolicyControlV1<'_> {
    InvocationObservationPolicyControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(super) fn setup(
    budget: usize,
) -> (SourceCoordinator, InvocationId, PreparedDisposal, Recorder) {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 4,
            max_participants: 4,
            max_staged_bytes: 1024,
            source: Limits {
                max_source_bytes: 128,
                max_edits: 4,
                max_edit_payload_bytes: 128,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 4,
            max_retained: 4,
            max_intent_bytes: 8192,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_retention_budget(budget, &mut r).unwrap();
    let d = o.create(b"private content", &mut r).unwrap();
    let p = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    (o, id, p, r)
}

#[test]
fn publisher_fault_cuts_never_release_staged_candidates_or_infer_safe_retry() {
    for mode in 0..4 {
        for cut in 0..5 {
            let (mut o, id, p, mut setup) = setup(8192);
            let c = context();
            let f = facts(&c);
            let mut r = if mode == 0 {
                Recorder::disabled()
            } else {
                Recorder::try_with_capacity(if mode == 3 { 1 } else { 32 }).unwrap()
            };
            if mode >= 2 {
                r = r.with_native_timing(
                    crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
                );
            }
            if mode == 3 {
                r.record(Observation {
                    boundary: Boundary::Control,
                    disposition: Disposition::Completed,
                    service_time: None,
                });
            }
            o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r)
                .unwrap();
            let unrelated = o.retention.reserve(71).unwrap();
            let before = o.retention_usage();
            o.publication_fault_after = Some(cut);
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || o.run_retained_disposal_v1(&id, &f, &mut r)
                ))
                .is_err()
            );
            assert!(o.faulted);
            assert!(matches!(
                o.invocation_slot(&id).unwrap().phase,
                Phase::Running
            ));
            assert!(
                o.invocation_slot(&id)
                    .unwrap()
                    .disposal_hold
                    .as_ref()
                    .unwrap()
                    .candidate
                    .is_some()
            );
            assert_eq!(
                o.retained_disposal_result_v1(&id, &release(&c), &mut r),
                Err(InvocationError::Faulted)
            );
            assert_eq!(
                o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r),
                Err(InvocationError::Faulted)
            );
            assert_eq!(
                o.create(b"blocked", &mut setup),
                Err(CoordinatorError::Faulted)
            );
            assert_eq!(o.retention_usage(), before);
            o.retire_retained_disposal_v1(&id, &control(&c), &mut r)
                .unwrap();
            assert_eq!(o.retention_usage().reserved, 71);
            assert!(o.retention.validate_exact(&unrelated, 71).is_ok());
            assert_eq!(
                o.invocation_state(&id, &mut r),
                Err(InvocationError::Faulted)
            );
            assert_eq!(o.invocation_usage().faulted_executions, 1);
            assert_eq!(o.invocation_usage().issued_lifetime, 1);
            if mode == 1 || mode == 2 {
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front())
                    .filter(|e| {
                        matches!(
                            e.boundary,
                            Boundary::RetainedDisposal | Boundary::RetainedDisposalService
                        )
                    })
                    .collect();
                assert_eq!(events.len(), 2);
                for e in events {
                    assert_eq!(e.disposition, Disposition::Unknown);
                    assert_eq!(e.service_time.is_some(), mode == 2);
                }
            }
        }
    }
}

#[test]
fn exact_parent_headroom_is_required_before_capture_and_retirement_releases_only_its_hold() {
    for budget in [2047, 2048, 2049] {
        let (mut o, id, p, mut r) = setup(budget);
        let c = context();
        let f = facts(&c);
        let result = o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r);
        if budget < 2048 {
            assert_eq!(result, Err(InvocationError::Limit));
            assert_eq!(o.retention_usage().reserved, 0);
            assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        } else {
            assert_eq!(result, Ok(InvocationState::Pending));
            crate::coordinator::shutdown::tests::invocation_funding(&o, &id, 1, 2048, &mut r);
            o.stop_invocation(&id, &mut r).unwrap();
            o.retire_retained_disposal_v1(&id, &control(&c), &mut r)
                .unwrap();
            assert_eq!(o.retention_usage().reserved, 0);
        }
    }
}

#[test]
fn corrupt_marker_or_wrong_size_parent_hold_fails_closed_before_publication_and_refund() {
    for bad_marker in [false, true] {
        let (mut o, id, p, mut r) = setup(8192);
        let c = context();
        let f = facts(&c);
        o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r)
            .unwrap();
        let generation = o.generation();
        let replacement = o.retention.reserve(73).unwrap();
        let h = o.invocations.slots[id.slot]
            .as_mut()
            .unwrap()
            .disposal_hold
            .as_mut()
            .unwrap();
        let saved_marker = if bad_marker {
            Some(std::mem::replace(&mut h.marker, Arc::new(())))
        } else {
            None
        };
        let saved_reservation = if !bad_marker {
            Some(std::mem::replace(&mut h.reservation, replacement))
        } else {
            None
        };
        let before = o.retention_usage();
        assert_eq!(
            o.run_retained_disposal_v1(&id, &f, &mut r),
            Err(InvocationError::Faulted)
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(
            o.retire_retained_disposal_v1(&id, &control(&c), &mut r),
            Err(InvocationError::Faulted)
        );
        assert_eq!(o.retention_usage(), before);
        // Restore only the deliberately corrupted test seam, then prove cleanup.
        let h = o.invocations.slots[id.slot]
            .as_mut()
            .unwrap()
            .disposal_hold
            .as_mut()
            .unwrap();
        if let Some(m) = saved_marker {
            h.marker = m;
        }
        if let Some(res) = saved_reservation {
            h.reservation = res;
        }
        o.stop_invocation(&id, &mut r).unwrap();
        o.retire_retained_disposal_v1(&id, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 73);
    }
}

#[test]
fn actual_success_paths_are_equivalent_with_disabled_available_timed_or_saturated_capture() {
    for mode in 0..4 {
        let (mut o, id, p, _) = setup(8192);
        let c = context();
        let f = facts(&c);
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 32 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(
                crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
            );
        }
        if mode == 3 {
            r.record(Observation {
                boundary: Boundary::Control,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        o.submit_retained_disposal_v1(&id, &p, &admission(&f), &mut r)
            .unwrap();
        o.run_retained_disposal_v1(&id, &f, &mut r).unwrap();
        assert_eq!(
            o.retained_disposal_result_v1(&id, &release(&c), &mut r)
                .unwrap()
                .document,
            *p.document()
        );
        o.retire_retained_disposal_v1(&id, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        assert!(!o.faulted);
        if mode == 1 || mode == 2 {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for b in [
                Boundary::RetainedDisposalAdmission,
                Boundary::RetainedDisposalService,
                Boundary::RetainedDisposalResultRelease,
                Boundary::RetainedDisposalRetirement,
            ] {
                let selected: Vec<_> = events.iter().filter(|e| e.boundary == b).collect();
                assert_eq!(selected.len(), 1);
                assert_eq!(selected[0].disposition, Disposition::Completed);
                assert_eq!(selected[0].service_time.is_some(), mode == 2);
            }
            let rendered = format!("{events:?}");
            assert!(!rendered.contains("private authority"));
            assert!(!rendered.contains("private content"));
        }
    }
}
