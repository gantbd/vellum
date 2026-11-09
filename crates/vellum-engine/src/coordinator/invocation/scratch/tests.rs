use super::*;
use crate::instrumentation::NativeTimingProfileV1;

#[test]
fn shutdown_inventory_refuses_wrong_credit_even_without_any_documents() {
    use crate::coordinator::shutdown::tests::{inspect, inspection_current};
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_scratch_v1(&id, &request(b"pending"), &admission(&f), &mut r)
        .unwrap();
    let original_cut = inspect(&o, &mut r);
    let before = o.retention_usage();
    let zero = o.retention.reserve(0).unwrap();
    let original = std::mem::replace(
        &mut o.invocations.slots[id.slot]
            .as_mut()
            .unwrap()
            .scratch_hold
            .as_mut()
            .unwrap()
            .reservation,
        zero,
    );
    assert_eq!(
        inspection_current(&o, &original_cut, &mut r),
        Err(CoordinatorError::Faulted)
    );
    assert_eq!(o.retention_usage(), before);
    let mut zero = std::mem::replace(
        &mut o.invocations.slots[id.slot]
            .as_mut()
            .unwrap()
            .scratch_hold
            .as_mut()
            .unwrap()
            .reservation,
        original,
    );
    o.retention.release(&mut zero).unwrap();
    assert_eq!(inspection_current(&o, &original_cut, &mut r), Ok(()));
    assert_eq!(o.retention_usage(), before);
}

#[test]
fn shutdown_inventory_preserves_original_hold_funding_until_native_retirement() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read, inventory_owner};
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    let before = o.retention_usage();
    o.submit_scratch_v1(&id, &request(b"prose"), &admission(&f), &mut r)
        .unwrap();
    let charge = admission(&f).limits.max_capture_bytes;
    for phase in 0..3 {
        if phase == 1 {
            o.run_scratch_v1(&id, &f, &mut r).unwrap();
        }
        if phase == 2 {
            o.forget_invocation_outcome(&id, &mut r).unwrap();
        }
        let cut = inspect(&o, &mut r);
        let rows = inspect_read(&o, &cut, &mut r);
        assert_eq!(rows.invocations[0].retained_holds, 1);
        assert_eq!(rows.invocations[0].reserved_bytes, charge);
        let row = inventory_owner(&o, &mut r, ShutdownOwnerRoleV1::Invocations, &[]);
        assert_eq!(row.additional_reserved_bytes, Some(charge));
        assert_eq!(o.retention_usage().reserved, before.reserved + charge);
    }
    let control = InvocationObservationPolicyControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    o.retire_scratch_v1(&id, &control, &mut r).unwrap();
    let cut = inspect(&o, &mut r);
    let rows = inspect_read(&o, &cut, &mut r);
    assert_eq!(rows.invocations[0].retained_holds, 0);
    assert_eq!(rows.invocations[0].reserved_bytes, 0);
    assert_eq!(o.retention_usage(), before);
    o.retire_invocation(&id, &mut r).unwrap();
}

#[test]
fn v6_scratch_receipt_corruption_closes_only_receipts_and_preserves_control_and_cleanup() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
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
    o.submit_scratch_v1(&id, &request(b"PRIVATE"), &admission(&f), &mut r)
        .unwrap();
    let mut l = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 0,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: size_of::<InvocationControlReferenceV1>()
            .max(size_of::<InvocationObservationReferenceV1>()),
        max_reply_bytes: InvocationReceiptV6::required_reply_bytes(512).unwrap(),
        max_control_reply_bytes: size_of::<InvocationControlObservationV1>(),
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    let mut v = o
        .open_invocation_observation_with_receipts_v6(
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
    o.run_scratch_v1(&id, &f, &mut r).unwrap();
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
    let n = TrustedScratchResultV1 {
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
    assert!(
        o.query_invocation_receipt_v6(
            &mut v,
            InvocationReceiptRequestV6::Scratch(&n),
            &p,
            &common,
            &mut r
        )
        .is_ok()
    );
    let generation = o.generation();
    let held = o.retention_usage();
    let original = o.invocations.slots[id.slot]
        .as_mut()
        .unwrap()
        .scratch_hold
        .as_mut()
        .unwrap()
        .candidate
        .take()
        .unwrap();
    assert_eq!(
        o.query_invocation_receipt_v6(
            &mut v,
            InvocationReceiptRequestV6::Scratch(&n),
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
    // Restore only the deliberately corrupted private fixture; the receipt fault is absorbing.
    o.invocations.slots[id.slot]
        .as_mut()
        .unwrap()
        .scratch_hold
        .as_mut()
        .unwrap()
        .candidate = Some(original);
    assert!(o.scratch_result_v1(&id, &n, &mut r).is_ok());
    assert_eq!(
        o.query_invocation_receipt_v6(
            &mut v,
            InvocationReceiptRequestV6::Scratch(&n),
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
    o.retire_scratch_v1(&id, &ctl, &mut r).unwrap();
    o.retire_invocation_observation_policy_v1(&id, &ctl, &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}

fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "scratch/1".into(),
        continuity: "c".into(),
    }
}
fn facts(c: &SettingsExecutionContextV1) -> TrustedScratchFactsV1<'_> {
    TrustedScratchFactsV1 {
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
        creation_allowed: Some(true),
    }
}
fn request(source: &[u8]) -> ScratchDocumentRequestV1<'_> {
    ScratchDocumentRequestV1 {
        source,
        serialization: SourceSerializationV1 {
            profile: SourceSerializationProfileV1::StrictUtf8OneBom,
            bom: false,
        },
        insertion_policy: GeneratedNewlineV1::CrLf,
    }
}
fn admission<'a>(f: &'a TrustedScratchFactsV1<'a>) -> ScratchInvocationAdmissionV1<'a> {
    ScratchInvocationAdmissionV1 {
        facts: f,
        limits: ScratchInvocationLimitsV1 {
            max_capture_bytes: 8192,
            max_result_bytes: 512,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_identifiers_allowed: Some(true),
    }
}
fn owner() -> (SourceCoordinator, Recorder) {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: crate::source::Limits {
                max_source_bytes: 4096,
                max_edits: 16,
                max_edit_payload_bytes: 8192,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 32,
            max_retained: 32,
            max_intent_bytes: 32768,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    (o, r)
}
#[test]
fn document_pin_inventory_refuses_orphaned_funded_capture_after_outcome_forgetting() {
    let (mut o, mut r) = owner();
    let peer = o.create(b"unrelated", &mut r).unwrap();
    let c = context();
    let f = facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_scratch_v1(&id, &request(b"prose"), &admission(&f), &mut r)
        .unwrap();
    o.run_scratch_v1(&id, &f, &mut r).unwrap();
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(o.invocations.document_pins(&peer.document), Ok(0));
    let original = o.invocations.slots[id.slot].as_mut().unwrap().intent.take();
    assert_eq!(
        o.invocations.document_pins(&peer.document),
        Err(CoordinatorError::Faulted)
    );
    o.invocations.slots[id.slot].as_mut().unwrap().intent = original;
    let hold = o.invocations.slots[id.slot]
        .as_mut()
        .unwrap()
        .scratch_hold
        .take();
    assert_eq!(
        o.invocations.document_pins(&peer.document),
        Err(CoordinatorError::Faulted)
    );
    o.invocations.slots[id.slot].as_mut().unwrap().scratch_hold = hold;
    assert_eq!(o.invocations.document_pins(&peer.document), Ok(0));
}
#[test]
fn invocation_document_pins_survive_scratch_hold_retirement_until_terminal_outcome_is_forgotten() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_scratch_v1(&id, &request(b"prose"), &admission(&f), &mut r)
        .unwrap();
    o.run_scratch_v1(&id, &f, &mut r).unwrap();
    let Phase::Terminal(Some(InvocationOutcome::ScratchCommitted(d))) =
        &o.invocation_slot(&id).unwrap().phase
    else {
        panic!("expected native output")
    };
    let d = d.document.clone();
    assert_eq!(o.invocations.document_pins(&d), Ok(1));
    let control = InvocationObservationPolicyControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    o.retire_scratch_v1(&id, &control, &mut r).unwrap();
    assert_eq!(o.invocations.document_pins(&d), Ok(1));
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(o.invocations.document_pins(&d), Ok(0));
    o.retire_invocation(&id, &mut r).unwrap();
}
#[test]
fn scratch_capture_requires_actual_parent_headroom_not_a_claimed_child_allowance() {
    for short in [false, true] {
        let (mut o, mut r) = owner();
        let c = context();
        let f = facts(&c);
        let q = request(b"PRIVATE");
        let mut a = admission(&f);
        a.limits.max_capture_bytes =
            ScratchInvocationLimitsV1::required_capture_bytes(&c, &q).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let _other = o
            .retention
            .reserve(1048576 - a.limits.max_capture_bytes + usize::from(short))
            .unwrap();
        let before = o.retention_usage();
        let generation = o.generation();
        let result = o.submit_scratch_v1(&id, &q, &a, &mut r);
        if short {
            assert_eq!(result, Err(InvocationError::Limit));
            assert_eq!(o.retention_usage(), before);
            assert!(
                o.invocations.slots[id.slot]
                    .as_ref()
                    .unwrap()
                    .scratch_hold
                    .is_none()
            );
        } else {
            assert_eq!(result, Ok(InvocationState::Pending));
            assert_eq!(o.retention_usage().reserved, 1048576);
        }
        assert_eq!(o.generation(), generation);
        assert!(o.documents.is_empty());
    }
}
#[test]
fn scratch_publisher_rejects_substituted_source_or_serialization_before_any_row_write() {
    for field in 0..3 {
        let (mut o, mut r) = owner();
        let c = context();
        let f = facts(&c);
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_scratch_v1(&id, &request(b"original"), &admission(&f), &mut r)
            .unwrap();
        o.invocations.slots[id.slot].as_mut().unwrap().phase = Phase::Running;
        let mut q = request(b"original");
        match field {
            0 => q.source = b"substitute",
            1 => q.serialization.bom = true,
            _ => q.insertion_policy = GeneratedNewlineV1::Lf,
        }
        let generation = o.generation();
        let held = o.retention_usage();
        assert_eq!(
            o.create_scratch_document_for_invocation(&q, Some(&id), &mut r),
            Err(CoordinatorError::Faulted)
        );
        assert!(o.is_faulted());
        assert!(o.documents.is_empty());
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
        assert!(
            o.invocations.slots[id.slot]
                .as_ref()
                .unwrap()
                .scratch_hold
                .as_ref()
                .unwrap()
                .candidate
                .is_none()
        );
    }
}
#[test]
fn interrupted_owned_scratch_is_unknown_and_only_its_hold_can_be_retired() {
    for cut in 0..3 {
        for mode in 0..4 {
            let (mut o, mut setup) = owner();
            let c = context();
            let f = facts(&c);
            let peer = o.create(b"peer", &mut setup).unwrap();
            let _other = o.retention.reserve(71).unwrap();
            let id = o.issue_invocation(&mut setup).unwrap();
            o.submit_scratch_v1(&id, &request(b"PRIVATE"), &admission(&f), &mut setup)
                .unwrap();
            let held = o.retention_usage();
            let generation = o.generation();
            let history = o.history_usage();
            let mut r = if mode == 0 {
                Recorder::disabled()
            } else {
                Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
            };
            if mode >= 2 {
                r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            }
            o.publication_fault_after = Some(cut);
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || o.run_scratch_v1(&id, &f, &mut r)
                ))
                .is_err()
            );
            assert!(o.is_faulted());
            assert!(matches!(
                o.invocations.slots[id.slot].as_ref().unwrap().phase,
                Phase::Running
            ));
            assert_eq!(o.retention_usage(), held);
            assert_eq!(o.history_usage(), history);
            assert_eq!(
                o.documents[peer.document.slot]
                    .as_ref()
                    .unwrap()
                    .source
                    .text(),
                "peer"
            );
            assert_eq!(o.documents.len(), if cut == 0 { 1 } else { 2 });
            assert_eq!(o.generation(), generation + u64::from(cut == 2));
            let release = TrustedScratchResultV1 {
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
            assert_eq!(
                o.scratch_result_v1(&id, &release, &mut r),
                Err(InvocationError::OutcomeUnavailable)
            );
            assert_eq!(
                o.run_scratch_v1(&id, &f, &mut r),
                Err(InvocationError::Faulted)
            );
            let control = InvocationObservationPolicyControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            };
            o.retire_scratch_v1(&id, &control, &mut r).unwrap();
            assert_eq!(o.retention_usage().reserved, 71);
            assert!(matches!(
                o.invocations.slots[id.slot].as_ref().unwrap().phase,
                Phase::Running
            ));
            assert_eq!(
                o.retire_scratch_v1(&id, &control, &mut r),
                Err(InvocationError::Unavailable)
            );
            let mut unknown = false;
            while let Some(e) = r.pop_front() {
                if e.boundary == Boundary::ScratchInvocationService {
                    assert_eq!(e.disposition, Disposition::Unknown);
                    unknown = true;
                }
            }
            if mode == 1 || mode == 2 {
                assert!(unknown);
            }
        }
    }
}
