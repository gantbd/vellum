use super::*;
use crate::attachment::Affinity;
use crate::coordinates::BytePosition;
use crate::instrumentation::{NativeTimingProfileV1, Observation};
use crate::selection::SelectionBudget;
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "live-relations/1".into(),
        continuity: "c".into(),
    }
}
#[test]
fn v5_live_relation_receipt_fault_is_absorbing_but_does_not_disable_protected_status_or_cleanup() {
    let (mut o, q, id, c, mut r) = setup(2);
    let f = facts(&c);
    let p = InvocationObservationPolicyV1 {
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
    let af = TrustedInvocationPolicyFactsV1 {
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
        &p,
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &af,
        &mut r,
    )
    .unwrap();
    o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    let mut l = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 0,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: size_of::<InvocationControlReferenceV1>()
            .max(size_of::<InvocationObservationReferenceV1>()),
        max_reply_bytes: InvocationReceiptV5::required_reply_bytes(512).unwrap(),
        max_control_reply_bytes: size_of::<InvocationControlObservationV1>(),
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    let mut v = o
        .open_invocation_observation_with_receipts_v5(
            &id,
            &c,
            &p,
            l,
            InvocationReceiptAdmissionV1 {
                observation: &af,
                capabilities: InvocationReceiptCapabilitiesV1 {
                    lifecycle: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    o.run_live_relation_v1(&id, &f, &mut r).unwrap();
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
    let common = TrustedInvocationReceiptReleaseV1 {
        observation: &ordinary,
        operation_receipt_allowed: Some(true),
    };
    let n = release(&c);
    assert!(
        o.query_invocation_receipt_v5(
            &mut v,
            InvocationReceiptRequestV5::LiveRelation(&n),
            &p,
            &common,
            &mut r
        )
        .is_ok()
    );
    let generation = o.generation();
    let held = o.retention_usage();
    o.invocations.slots[id.slot]
        .as_mut()
        .unwrap()
        .live_relation_hold
        .as_mut()
        .unwrap()
        .candidate = Some(false);
    assert_eq!(
        o.query_invocation_receipt_v5(
            &mut v,
            InvocationReceiptRequestV5::LiveRelation(&n),
            &p,
            &common,
            &mut r
        )
        .err(),
        Some(InvocationError::Faulted)
    );
    assert!(!o.is_faulted());
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage(), held);
    o.invocations.slots[id.slot]
        .as_mut()
        .unwrap()
        .live_relation_hold
        .as_mut()
        .unwrap()
        .candidate = Some(true); // Restore only injected private fixture corruption.
    assert!(o.live_relation_result_v1(&id, &n, &mut r).is_ok());
    assert_eq!(
        o.query_invocation_receipt_v5(
            &mut v,
            InvocationReceiptRequestV5::LiveRelation(&n),
            &p,
            &common,
            &mut r
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut v, &p, &current, &mut r)
            .unwrap()
            .phase,
        InvocationObservationPhaseV1::Terminal
    );
    let ctl = InvocationObservationPolicyControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    o.retire_invocation_observation_v1(&mut v, &ctl, &mut r)
        .unwrap();
    o.retire_live_relation_v1(&id, &ctl, &mut r).unwrap();
    o.retire_invocation_observation_policy_v1(&id, &ctl, &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
fn facts(c: &SettingsExecutionContextV1) -> TrustedLiveRelationFactsV1<'_> {
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
fn admission<'a>(f: &'a TrustedLiveRelationFactsV1<'a>) -> LiveRelationAdmissionV1<'a> {
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
fn release(c: &SettingsExecutionContextV1) -> TrustedLiveRelationResultV1<'_> {
    TrustedLiveRelationResultV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_readable: Some(true),
        result_context_current: Some(true),
        historical_document_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        max_result_bytes: 512,
    }
}
fn caret() -> Selection {
    Selection {
        anchor: BytePosition(1),
        head: BytePosition(1),
        affinity: Affinity::After,
    }
}
fn setup(
    kind: usize,
) -> (
    SourceCoordinator,
    LiveRelationRequestV1,
    InvocationId,
    SettingsExecutionContextV1,
    Recorder,
) {
    let mut r = Recorder::disabled();
    let (mut o, d, _) = crate::coordinator::annotation::tests::setup(&mut r);
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.configure_logical_views(
        LogicalViewLimits {
            max_live_views: 8,
            max_issued_views: 16,
            max_total_selections: 32,
            normalization: SelectionBudget {
                max_input_selections: 8,
                max_output_selections: 8,
                max_work: 4096,
            },
            max_mapping_steps: 4096,
        },
        &mut r,
    )
    .unwrap();
    o.configure_workspaces(
        WorkspaceLimits {
            max_issued: 8,
            max_live: 8,
            max_memberships: 16,
            max_work: 4096,
        },
        &mut r,
    )
    .unwrap();
    let operation = if kind == 2 {
        LiveRelationOperationV1::Membership {
            workspace: o.create_workspace(&mut r).unwrap(),
            present: true,
        }
    } else {
        let view = o.create_logical_view(&mut r).unwrap();
        let expected = o.logical_view(&view, &mut r).unwrap().stamp;
        if kind == 0 {
            LiveRelationOperationV1::Attach {
                view,
                expected,
                source: d.clone(),
                selections: vec![caret()],
                primary: 0,
            }
        } else {
            let expected = o
                .attach_logical_view(&view, expected, &d, &[caret()], 0, &mut r)
                .unwrap()
                .stamp;
            LiveRelationOperationV1::Detach { view, expected }
        }
    };
    let q = LiveRelationRequestV1 {
        document: d.document.clone(),
        expected_lifecycle: o.lifecycle_state(&d.document, &mut r).unwrap(),
        operation,
    };
    let id = o.issue_invocation(&mut r).unwrap();
    (o, q, id, context(), r)
}
#[test]
fn live_relation_interruptions_preserve_unknown_and_once_only_independent_cleanup() {
    for kind in 0..3 {
        for cut in 0..if kind == 2 { 4 } else { 3 } {
            for mode in 0..4 {
                let (mut o, q, id, c, mut setup) = setup(kind);
                let f = facts(&c);
                o.submit_live_relation_v1(&id, &q, &admission(&f), &mut setup)
                    .unwrap();
                let mut other = o.retention.reserve(71).unwrap();
                let history = o.history_usage();
                let lifecycle = o.documents[q.document.slot].as_ref().unwrap().lifecycle;
                let source = o.documents[q.document.slot]
                    .as_ref()
                    .unwrap()
                    .source
                    .clone();
                let issued = o.invocation_usage().issued_lifetime;
                let mut r = if mode == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
                };
                if mode >= 2 {
                    r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1)
                }
                if mode == 3 {
                    r.record(Observation {
                        boundary: Boundary::Control,
                        disposition: Disposition::Completed,
                        service_time: None,
                    });
                }
                o.publication_fault_after = Some(cut);
                assert!(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                        || o.run_live_relation_v1(&id, &f, &mut r)
                    ))
                    .is_err()
                );
                assert!(o.is_faulted());
                assert!(matches!(
                    o.invocations.slots[id.slot].as_ref().unwrap().phase,
                    Phase::Running
                ));
                assert_eq!(
                    o.live_relation_result_v1(&id, &release(&c), &mut r),
                    Err(InvocationError::OutcomeUnavailable)
                );
                assert_eq!(
                    o.stop_invocation(&id, &mut r),
                    Err(InvocationError::Faulted)
                );
                assert_eq!(o.history_usage(), history);
                assert_eq!(
                    o.documents[q.document.slot].as_ref().unwrap().source,
                    source
                );
                assert_eq!(
                    o.documents[q.document.slot].as_ref().unwrap().lifecycle,
                    lifecycle
                );
                assert_eq!(o.retention_usage().reserved, 8192 + 71);
                o.retire_live_relation_v1(
                    &id,
                    &InvocationObservationPolicyControlV1 {
                        context: &c,
                        owner_control_allowed: Some(true),
                        retention_released: Some(true),
                        delivery_drained: Some(true),
                    },
                    &mut r,
                )
                .unwrap();
                assert_eq!(o.retention_usage().reserved, 71);
                assert_eq!(o.invocation_usage().issued_lifetime, issued);
                assert!(matches!(
                    o.invocations.slots[id.slot].as_ref().unwrap().phase,
                    Phase::Running
                ));
                assert_eq!(
                    o.run_live_relation_v1(&id, &f, &mut r),
                    Err(InvocationError::Faulted)
                );
                assert_eq!(
                    o.retire_invocation(&id, &mut r),
                    Err(InvocationError::Faulted)
                );
                o.retention.release(&mut other).unwrap();
                let mut unknown = 0;
                while let Some(e) = r.pop_front() {
                    if e.boundary == Boundary::LiveRelationService
                        && e.disposition == Disposition::Unknown
                    {
                        unknown += 1;
                        assert_eq!(e.service_time.is_some(), mode == 2)
                    }
                    assert!(!format!("{e:?}").contains("PRIVATE"));
                }
                if mode == 1 || mode == 2 {
                    assert!(unknown >= 1)
                }
            }
        }
    }
}
#[test]
fn live_relation_publisher_refuses_original_ticket_target_substitution_before_installation() {
    for kind in 0..3 {
        let (mut o, q, id, c, mut r) = setup(kind);
        let f = facts(&c);
        o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
            .unwrap();
        let other = o.create(b"peer", &mut r).unwrap();
        let generation = o.generation();
        let held = o.retention_usage();
        o.invocations.slots[id.slot].as_mut().unwrap().phase = Phase::Running;
        match &q.operation {
            LiveRelationOperationV1::Attach {
                view,
                expected,
                selections,
                primary,
                ..
            } => assert_eq!(
                o.attach_logical_view_for_invocation(
                    view,
                    *expected,
                    &other,
                    selections,
                    *primary,
                    Some(&id),
                    &mut r
                ),
                Err(SelectionError::Faulted)
            ),
            LiveRelationOperationV1::Detach { expected, .. } => {
                let v = o.create_logical_view(&mut r).unwrap();
                let stamp = o.logical_view(&v, &mut r).unwrap().stamp;
                let stamp = o
                    .attach_logical_view(&v, stamp, &other, &[caret()], 0, &mut r)
                    .unwrap()
                    .stamp;
                let before = o.generation();
                assert_eq!(
                    o.detach_logical_view_for_invocation(&v, stamp, Some(&id), &mut r),
                    Err(SelectionError::Faulted)
                );
                assert_eq!(o.generation(), before);
                let _ = expected;
            }
            LiveRelationOperationV1::Membership { workspace, present } => assert_eq!(
                o.set_workspace_membership_for_invocation(
                    workspace,
                    &other.document,
                    *present,
                    Some(&id),
                    &mut r
                ),
                Err(CoordinatorError::Faulted)
            ),
        }
        if kind != 1 {
            assert_eq!(o.generation(), generation)
        }
        assert!(o.is_faulted());
        assert_eq!(o.retention_usage(), held);
        assert!(
            o.invocations.slots[id.slot]
                .as_ref()
                .unwrap()
                .live_relation_hold
                .as_ref()
                .unwrap()
                .candidate
                .is_none()
        );
    }
}
#[test]
fn live_relation_headroom_is_real_parent_capacity_before_copy_or_preparation() {
    for kind in 0..3 {
        for short in [false, true] {
            let (mut o, q, id, c, mut r) = setup(kind);
            let f = facts(&c);
            let mut a = admission(&f);
            a.limits.max_capture_bytes =
                LiveRelationLimitsV1::required_capture_bytes(&c, &q).unwrap();
            let mut other = o
                .retention
                .reserve(1048576 - a.limits.max_capture_bytes + usize::from(short))
                .unwrap();
            let before = o.retention_usage();
            let generation = o.generation();
            let result = o.submit_live_relation_v1(&id, &q, &a, &mut r);
            if short {
                assert_eq!(result, Err(InvocationError::Limit));
                assert_eq!(o.retention_usage(), before);
                assert_eq!(
                    o.invocation_state(&id, &mut r).unwrap(),
                    InvocationState::Issued
                )
            } else {
                result.unwrap();
                crate::coordinator::shutdown::tests::invocation_funding(
                    &o,
                    &id,
                    1,
                    a.limits.max_capture_bytes,
                    &mut r,
                );
                assert_eq!(o.retention_usage().reserved, 1048576)
            }
            assert_eq!(o.generation(), generation);
            o.retention.release(&mut other).unwrap();
        }
    }
}
