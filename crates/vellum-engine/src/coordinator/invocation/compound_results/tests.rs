use super::*;
use crate::attachment::AttachmentTarget;
use crate::instrumentation::{NativeTimingProfileV1, Observation};
#[test]
fn shutdown_inventory_keeps_compound_results_funded_after_outcome_forgetting() {
    use crate::coordinator::shutdown::tests::invocation_funding;
    let (mut o, _, _, id, intent, c, mut r) = setup();
    let before = o.retention_usage().reserved;
    o.submit_compound_with_annotation_results_v1(&id, &intent, &admission(&c), &mut r)
        .unwrap();
    let charge = o.retention_usage().reserved - before;
    assert!(charge > 0);
    invocation_funding(&o, &id, 1, charge, &mut r);
    o.run(&id, &mut r).unwrap();
    invocation_funding(&o, &id, 1, charge, &mut r);
    assert_eq!(
        o.forget_invocation_outcome(&id, &mut r),
        Err(InvocationError::Unavailable)
    );
    o.forget_compound_annotation_results_v1(
        &id,
        &CompoundAnnotationResultEvictionV1 {
            context: &c,
            retention_released: Some(true),
            delivery_drained: Some(true),
        },
        &mut r,
    )
    .unwrap();
    invocation_funding(&o, &id, 1, charge, &mut r);
    o.retire_compound_annotation_results_v1(
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
    invocation_funding(&o, &id, 0, 0, &mut r);
    assert_eq!(o.retention_usage().reserved, before);
}
#[test]
fn v4_component_corruption_disables_only_receipts_and_preserves_protected_cleanup() {
    let (mut o, d, _, id, intent, c, mut r) = setup();
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
    let facts = TrustedInvocationPolicyFactsV1 {
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
        &facts,
        &mut r,
    )
    .unwrap();
    let grant = TrustedCompoundAnnotationResultGrantV1 {
        invocation: &id,
        authority: &c.authority,
        target: CompoundAnnotationResultGrantTargetV1::CreatedByComponent {
            document: &d.document,
            component: 0,
        },
        capabilities: AnnotationResultCapabilitiesV1::IdentityContentTarget,
    };
    let gs = [Some(grant), None];
    let mut a = admission(&c);
    a.grants = &gs;
    o.submit_compound_with_annotation_results_v1(&id, &intent, &a, &mut r)
        .unwrap();
    let mut l = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 0,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: size_of::<InvocationControlReferenceV1>()
            .max(size_of::<InvocationObservationReferenceV1>()),
        max_reply_bytes: InvocationReceiptV4::required_reply_bytes(512).unwrap(),
        max_control_reply_bytes: size_of::<InvocationControlObservationV1>(),
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    let mut observer = o
        .open_invocation_observation_with_receipts_v4(
            &id,
            &c,
            &p,
            l,
            InvocationReceiptAdmissionV1 {
                observation: &facts,
                capabilities: InvocationReceiptCapabilitiesV1 {
                    atomic_annotation: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    o.run(&id, &mut r).unwrap();
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
    let mut native = release(&c);
    native.grant = Some(grant);
    assert!(
        o.query_invocation_receipt_v4(
            &mut observer,
            InvocationReceiptRequestV4::AtomicAnnotation {
                index: 0,
                native: &native
            },
            &p,
            &common,
            &mut r
        )
        .is_ok()
    );
    let h = o.invocations.slots[id.slot]
        .as_mut()
        .unwrap()
        .compound_results
        .as_mut()
        .unwrap();
    let original = h.receipts.as_ref().unwrap()[0].clone();
    h.receipts.as_mut().unwrap()[0].invocation.slot += 1;
    let generation = o.generation();
    let held = o.retention_usage();
    assert_eq!(
        o.query_invocation_receipt_v4(
            &mut observer,
            InvocationReceiptRequestV4::AtomicAnnotation {
                index: 0,
                native: &native
            },
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
        .compound_results
        .as_mut()
        .unwrap()
        .receipts
        .as_mut()
        .unwrap()[0] = original; // Restore only injected fixture corruption.
    assert!(
        o.retained_compound_annotation_result_v1(&id, 0, &native, &mut r)
            .is_ok()
    );
    assert_eq!(
        o.query_invocation_receipt_v4(
            &mut observer,
            InvocationReceiptRequestV4::AtomicAnnotation {
                index: 0,
                native: &native
            },
            &p,
            &common,
            &mut r
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut observer, &p, &current, &mut r)
            .unwrap()
            .phase,
        InvocationObservationPhaseV1::Terminal
    );
    let control = InvocationObservationPolicyControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    o.retire_invocation_observation_v1(&mut observer, &control, &mut r)
        .unwrap();
    o.retire_compound_annotation_results_v1(&id, &control, &mut r)
        .unwrap();
    o.retire_invocation_observation_policy_v1(&id, &control, &mut r)
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
        profile: "compound-results/1".into(),
        continuity: "c".into(),
    }
}
fn admission(c: &SettingsExecutionContextV1) -> TrustedCompoundAnnotationResultAdmissionV1<'_> {
    TrustedCompoundAnnotationResultAdmissionV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        context_current: Some(true),
        result_admission_allowed: Some(true),
        max_result_bytes: 512,
        grants: &[None, None],
    }
}
fn release(c: &SettingsExecutionContextV1) -> TrustedCompoundAnnotationResultReleaseV1<'_> {
    TrustedCompoundAnnotationResultReleaseV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_context_current: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        max_result_bytes: 512,
        grant: None,
    }
}
fn setup() -> (
    SourceCoordinator,
    DocumentVersion,
    AnnotationId,
    InvocationId,
    CompoundIntent,
    SettingsExecutionContextV1,
    Recorder,
) {
    let mut r = Recorder::disabled();
    let (mut o, d, a) = crate::coordinator::annotation::tests::setup(&mut r);
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    o.configure_retention_budget(8388608, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let intent = CompoundIntent {
        source: SourceIntent {
            writes: vec![DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 0,
                    expected: "".into(),
                    inserted: "X".into(),
                }],
            }],
            reads: vec![],
        },
        annotations: vec![
            AnnotationMutation::Create {
                target: SourceTarget {
                    source: d.clone(),
                    target: AttachmentTarget::ContentRange { start: 0, end: 1 },
                    basis: TargetBasis::Before,
                },
                body: "PRIVATE_NEW".into(),
            },
            AnnotationMutation::UpdateBody {
                annotation: a.clone(),
                expected_content: 1,
                body: "PRIVATE_CHANGED".into(),
            },
        ],
    };
    (o, d, a, id, intent, c, r)
}
fn recorder(mode: usize) -> Recorder {
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
    r
}
#[test]
fn compound_receipt_plan_or_original_component_substitution_faults_before_any_installation() {
    for mode in 0..4 {
        for corruption in 0..4 {
            let (mut o, d, a, id, intent, c, mut setup) = setup();
            o.submit_compound_with_annotation_results_v1(&id, &intent, &admission(&c), &mut setup)
                .unwrap();
            let generation = o.generation();
            let history = o.history_usage();
            let held = o.retention_usage();
            let before = o.annotations.records[a.slot]
                .as_ref()
                .unwrap()
                .value
                .clone();
            let slot = o.invocations.slots[id.slot].as_mut().unwrap();
            match corruption {
                0 => {
                    let Phase::Pending {
                        prepared: Ok(p), ..
                    } = &mut slot.phase
                    else {
                        panic!()
                    };
                    p.annotations.swap(0, 1)
                }
                1 => slot
                    .compound_results
                    .as_mut()
                    .unwrap()
                    .plan
                    .as_mut()
                    .unwrap()
                    .swap(0, 1),
                2 => {
                    let Some(BoundIntent::Compound(i)) = &mut slot.intent else {
                        panic!()
                    };
                    i.annotations.swap(0, 1)
                }
                _ => slot.compound_results.as_mut().unwrap().invocation.slot += 1,
            }
            let mut r = recorder(mode);
            assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
            assert!(o.is_faulted());
            assert_eq!(o.generation(), generation);
            assert_eq!(o.history_usage(), history);
            assert_eq!(o.retention_usage(), held);
            assert_eq!(
                o.documents[d.document.slot].as_ref().unwrap().source.text(),
                "abc"
            );
            assert_eq!(
                o.annotations.records[a.slot].as_ref().unwrap().value,
                before
            );
            assert!(matches!(
                o.invocations.slots[id.slot].as_ref().unwrap().phase,
                Phase::Running
            ));
            assert!(
                o.invocations.slots[id.slot]
                    .as_ref()
                    .unwrap()
                    .compound_results
                    .as_ref()
                    .unwrap()
                    .receipts
                    .as_ref()
                    .unwrap()
                    .iter()
                    .all(|v| v.decision == Decision::NotPublished)
            );
            assert!(
                o.retained_compound_annotation_result_v1(&id, 0, &release(&c), &mut r)
                    .is_err()
            );
            assert_eq!(
                o.stop_invocation(&id, &mut r),
                Err(InvocationError::Faulted)
            );
            let mut saw_unknown = false;
            while let Some(event) = r.pop_front() {
                if event.boundary == Boundary::InvocationExecution {
                    assert_eq!(event.disposition, Disposition::Unknown);
                    saw_unknown = true;
                }
                assert!(!format!("{event:?}").contains("PRIVATE"));
            }
            if mode == 1 || mode == 2 {
                assert!(saw_unknown)
            }
        }
    }
}
#[test]
fn interrupted_compound_installation_never_delivers_a_success_vector_and_cleanup_preserves_unknown()
{
    for mode in 0..4 {
        let (mut o, d, a, id, intent, c, mut setup) = setup();
        o.submit_compound_with_annotation_results_v1(&id, &intent, &admission(&c), &mut setup)
            .unwrap();
        let charge = o.retention_usage().reserved;
        let issued = o.invocation_usage().issued_lifetime;
        let before = o.annotations.records[a.slot]
            .as_ref()
            .unwrap()
            .value
            .clone();
        o.publication_fault_after = Some(1);
        let mut r = recorder(mode);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.run(&id, &mut r))).is_err()
        );
        assert!(o.is_faulted());
        assert_eq!(
            o.documents[d.document.slot].as_ref().unwrap().source.text(),
            "Xabc"
        );
        assert_eq!(
            o.annotations.records[a.slot].as_ref().unwrap().value,
            before
        );
        assert_eq!(
            o.retained_compound_annotation_result_v1(&id, 0, &release(&c), &mut r),
            Err(InvocationError::OutcomeUnavailable)
        );
        assert_eq!(o.retention_usage().reserved, charge);
        o.retire_compound_annotation_results_v1(
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
        assert_eq!(o.retention_usage().reserved, 0);
        assert_eq!(o.invocation_usage().issued_lifetime, issued);
        assert!(matches!(
            o.invocations.slots[id.slot].as_ref().unwrap().phase,
            Phase::Running
        ));
        assert_eq!(o.run(&id, &mut r), Err(InvocationError::Faulted));
        assert_eq!(
            o.retire_invocation(&id, &mut r),
            Err(InvocationError::Faulted)
        );
    }
}
