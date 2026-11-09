use super::*;
use crate::capture::InertValueV1 as D;
use AnnotationAuthoringEnvelopeValueV1 as V;
#[test]
fn receipt_is_captured_with_publication_and_not_reconstructed_from_later_rows() {
    let (mut o, ticket, c, annotation, mut r) = setup(false);
    let Some(BoundIntent::Annotation(capture)) =
        &o.invocations.slots[ticket.slot].as_ref().unwrap().intent
    else {
        panic!()
    };
    let charge = capture
        .limits
        .required_hold_bytes(capture.native.parent_intent, capture.native.root)
        .unwrap();
    crate::coordinator::shutdown::tests::invocation_funding(&o, &ticket, 1, charge, &mut r);
    o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
        .unwrap();
    crate::coordinator::shutdown::tests::invocation_funding(&o, &ticket, 1, charge, &mut r);
    let receipt = o.invocations.slots[ticket.slot]
        .as_ref()
        .unwrap()
        .annotation_hold
        .as_ref()
        .unwrap()
        .receipt
        .as_ref()
        .unwrap()
        .clone();
    assert_eq!(receipt.invocation, ticket);
    assert_eq!(receipt.operation, receipt::Operation::Body);
    let receipt::Decision::Applied {
        publication,
        record,
    } = &receipt.decision
    else {
        panic!("expected applied historical receipt")
    };
    assert_eq!(publication, &ticket);
    assert_eq!(record.annotation, annotation);
    assert!(record.present);
    assert_eq!(record.content_version, 2);
    assert_eq!(record.target_generation, 1);
    let later = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: annotation.clone(),
                expected_content: 2,
                body: "LATER_PRIVATE".into(),
            }],
            &mut r,
        )
        .unwrap();
    o.commit(later, &mut r).unwrap();
    assert_eq!(
        o.invocations.slots[ticket.slot]
            .as_ref()
            .unwrap()
            .annotation_hold
            .as_ref()
            .unwrap()
            .receipt
            .as_ref(),
        Some(&receipt)
    );
    o.forget_invocation_outcome(&ticket, &mut r).unwrap();
    crate::coordinator::shutdown::tests::invocation_funding(&o, &ticket, 1, charge, &mut r);
    assert!(
        o.invocations.slots[ticket.slot]
            .as_ref()
            .unwrap()
            .annotation_hold
            .as_ref()
            .unwrap()
            .receipt
            .is_none()
    );
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "annotation-test".into(),
        continuity: "c".into(),
    }
}
fn retained(o: &SourceCoordinator, id: &InvocationId) -> Option<receipt::Receipt> {
    o.invocations.slots[id.slot]
        .as_ref()
        .unwrap()
        .annotation_hold
        .as_ref()
        .unwrap()
        .receipt
        .clone()
}
#[test]
fn receipt_staging_rejects_another_ticket_before_any_root_installation() {
    let (mut o, ticket, c, annotation, mut r) = setup(false);
    let other = o.issue_invocation(&mut r).unwrap();
    let generation = o.generation();
    let value = o.annotations.records[annotation.slot]
        .as_ref()
        .unwrap()
        .value
        .clone();
    let Phase::AnnotationPending(p) = std::mem::replace(
        &mut o.invocations.slots[ticket.slot].as_mut().unwrap().phase,
        Phase::Running,
    ) else {
        panic!()
    };
    o.faulted = true;
    let mut receipt = None;
    assert_eq!(
        o.commit_with_annotation_storage(p.plan.unwrap(), &other, &mut receipt, &mut r),
        Err(CoordinatorError::Faulted)
    );
    assert!(receipt.is_none());
    let mut fault_events = 0;
    while let Some(event) = r.pop_front() {
        if event.boundary == Boundary::AnnotationDecisionStaging {
            assert_eq!(
                event.disposition,
                crate::instrumentation::Disposition::Unknown
            );
            assert!(event.service_time.is_some());
            fault_events += 1;
        }
    }
    assert_eq!(fault_events, 1);
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.annotations.records[annotation.slot]
            .as_ref()
            .unwrap()
            .value,
        value
    );
    assert!(retained(&o, &ticket).is_none());
    assert_eq!(
        o.invocation_outcome(&ticket, &mut r),
        Err(InvocationError::Faulted)
    );
    o.retire_scoped_annotation_v1(&ticket, &c, &retire(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(
        o.invocation_outcome(&ticket, &mut r),
        Err(InvocationError::Faulted)
    );
}

#[test]
fn receipt_staging_is_content_free_and_capture_equivalent_with_disabled_or_full_recorders() {
    use crate::instrumentation::{Disposition, NativeTimingProfileV1, Observation};
    for mode in 0..4 {
        let (mut o, ticket, c, _, _) = setup(false);
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
        o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
            .unwrap();
        let receipt = retained(&o, &ticket).unwrap();
        let receipt::Decision::Applied { record, .. } = &receipt.decision else {
            panic!()
        };
        assert_eq!((record.content_version, record.target_generation), (2, 1));
        assert!(!format!("{receipt:?}").contains("PRIVATE"));
        let mut events = 0;
        while let Some(event) = r.pop_front() {
            if event.boundary == Boundary::AnnotationDecisionStaging {
                events += 1;
                assert_eq!(event.disposition, Disposition::Completed);
                assert_eq!(event.service_time.is_some(), mode == 2);
            }
            assert!(!format!("{event:?}").contains("PRIVATE"));
        }
        assert_eq!(events, usize::from(mode == 1 || mode == 2));
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
    }
}
fn submit_mutation(
    o: &mut SourceCoordinator,
    c: &SettingsExecutionContextV1,
    m: &AnnotationMutation,
    r: &mut Recorder,
) -> InvocationId {
    let id = o.issue_invocation(r).unwrap();
    let (doc, op) = match m {
        AnnotationMutation::Create { target, .. } => (&target.source.document, "create"),
        AnnotationMutation::UpdateBody { annotation, .. } => (&annotation.document, "update-body"),
        AnnotationMutation::Reattach { annotation, .. } => (&annotation.document, "reattach"),
        AnnotationMutation::Delete { annotation, .. } => (&annotation.document, "delete"),
    };
    let reads = ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 16,
            max_work: 1024,
        },
    };
    let mut raw = vec![
        ("contract", V::Data(D::Text("annotation-authoring/1"))),
        ("operation", V::Data(D::Text(op))),
        ("invocation", V::Invocation(&id)),
        ("document", V::Document(doc)),
        ("profile", V::Data(D::Text(&c.profile))),
        ("dependencies", V::Dependencies(&reads)),
    ];
    match m {
        AnnotationMutation::Create { target, body } => raw.extend([
            ("target", V::Target(target)),
            ("body", V::Data(D::Text(body))),
        ]),
        AnnotationMutation::UpdateBody {
            annotation,
            expected_content,
            body,
        } => raw.extend([
            ("annotation", V::Annotation(annotation)),
            ("expectedContent", V::Version(*expected_content)),
            ("body", V::Data(D::Text(body))),
        ]),
        AnnotationMutation::Reattach {
            annotation,
            expected_target,
            target,
        } => raw.extend([
            ("annotation", V::Annotation(annotation)),
            ("expectedTarget", V::Version(*expected_target)),
            ("target", V::Target(target)),
        ]),
        AnnotationMutation::Delete {
            annotation,
            expected_content,
            expected_target,
        } => raw.extend([
            ("annotation", V::Annotation(annotation)),
            ("expectedContent", V::Version(*expected_content)),
            ("expectedTarget", V::Version(*expected_target)),
        ]),
    }
    let q = normalize_annotation_authoring_arguments_v1(
        V::Object(&raw),
        AnnotationAuthoringArgumentLimitsV1 {
            max_body_bytes: 1024,
            max_profile_bytes: 128,
            max_dependencies: 16,
            max_owned_bytes: 16384,
        },
        r,
    )
    .unwrap();
    o.submit_scoped_annotation_v1(&q, c, &facts(c), r).unwrap();
    id
}

#[test]
fn receipt_captures_merged_target_then_create_reattach_and_retained_deletion() {
    let (mut o, ticket, c, annotation, mut r) = setup(false);
    let mut target = o.annotation(&annotation, &mut r).unwrap().authored.clone();
    target.target = crate::attachment::AttachmentTarget::ContentRange { start: 1, end: 2 };
    let independent = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Reattach {
                annotation: annotation.clone(),
                expected_target: 1,
                target: target.clone(),
            }],
            &mut r,
        )
        .unwrap();
    o.commit(independent, &mut r).unwrap();
    o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
        .unwrap();
    let receipt::Decision::Applied { record, .. } = retained(&o, &ticket).unwrap().decision else {
        panic!()
    };
    assert_eq!((record.content_version, record.target_generation), (2, 2));
    o.retire_scoped_annotation_v1(&ticket, &c, &retire(), &mut r)
        .unwrap();
    let create = submit_mutation(
        &mut o,
        &c,
        &AnnotationMutation::Create {
            body: "SECRET".into(),
            target: target.clone(),
        },
        &mut r,
    );
    assert!(retained(&o, &create).is_none());
    o.run_scoped_annotation_v1(&create, &c, &facts(&c), &mut r)
        .unwrap();
    let result = retained(&o, &create).unwrap();
    assert_eq!(result.operation, receipt::Operation::Create);
    let receipt::Decision::Applied { record, .. } = result.decision else {
        panic!()
    };
    let created = record.annotation;
    assert!(record.present);
    assert_eq!((record.content_version, record.target_generation), (1, 1));
    o.retire_scoped_annotation_v1(&create, &c, &retire(), &mut r)
        .unwrap();
    target.target = crate::attachment::AttachmentTarget::ContentRange { start: 0, end: 1 };
    let reattach = submit_mutation(
        &mut o,
        &c,
        &AnnotationMutation::Reattach {
            annotation: created.clone(),
            expected_target: 1,
            target,
        },
        &mut r,
    );
    o.run_scoped_annotation_v1(&reattach, &c, &facts(&c), &mut r)
        .unwrap();
    assert_eq!(
        retained(&o, &reattach).unwrap().operation,
        receipt::Operation::Target
    );
    o.retire_scoped_annotation_v1(&reattach, &c, &retire(), &mut r)
        .unwrap();
    let delete = submit_mutation(
        &mut o,
        &c,
        &AnnotationMutation::Delete {
            annotation: created.clone(),
            expected_content: 1,
            expected_target: 2,
        },
        &mut r,
    );
    o.run_scoped_annotation_v1(&delete, &c, &facts(&c), &mut r)
        .unwrap();
    let result = retained(&o, &delete).unwrap();
    assert_eq!(result.operation, receipt::Operation::Delete);
    let receipt::Decision::Applied { record, .. } = result.decision else {
        panic!()
    };
    assert_eq!(record.annotation, created);
    assert!(!record.present);
    assert_eq!((record.content_version, record.target_generation), (1, 2));
}

#[test]
fn unchanged_receipt_needs_no_new_publication_or_unused_version_capacity() {
    let (mut o, ticket, c, annotation, mut r) = setup(true);
    o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
        .unwrap();
    let result = retained(&o, &ticket).unwrap();
    let receipt::Decision::Unchanged { record } = result.decision else {
        panic!()
    };
    assert_eq!(record.annotation, annotation);
    assert_eq!(record.content_version, u64::MAX);
    assert_eq!(o.generation(), u64::MAX);
}

#[test]
fn known_stop_or_refusal_receipts_have_no_candidate_record_facts() {
    for stopped in [false, true] {
        let (mut o, ticket, c, _, mut r) = setup(false);
        let generation = o.generation();
        if stopped {
            o.stop_invocation(&ticket, &mut r).unwrap();
        } else {
            let mut denied = facts(&c);
            denied.authorized = None;
            o.run_scoped_annotation_v1(&ticket, &c, &denied, &mut r)
                .unwrap();
        }
        assert_eq!(
            retained(&o, &ticket).unwrap().decision,
            receipt::Decision::NotPublished
        );
        assert_eq!(o.generation(), generation);
        o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
            .unwrap();
        assert_eq!(
            retained(&o, &ticket).unwrap().decision,
            receipt::Decision::NotPublished
        );
        o.retire_scoped_annotation_v1(&ticket, &c, &retire(), &mut r)
            .unwrap();
        assert!(
            o.invocations.slots[ticket.slot]
                .as_ref()
                .unwrap()
                .annotation_hold
                .is_none()
        );
    }
}
fn facts(c: &SettingsExecutionContextV1) -> TrustedAnnotationInvocationV1<'_> {
    TrustedAnnotationInvocationV1 {
        context: c,
        limits: AnnotationInvocationLimitsV1 {
            max_context_bytes: 1024,
            max_capture_bytes: 8192,
            max_plan_bytes: 32768,
            max_construction_bytes: 65536,
        },
        required_reads: &[],
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        companions_ready: Some(true),
        history_ready: Some(true),
        annotation_writable: Some(true),
        resources_ready: Some(true),
    }
}
fn setup(
    exhausted_noop: bool,
) -> (
    SourceCoordinator,
    InvocationId,
    SettingsExecutionContextV1,
    AnnotationId,
    Recorder,
) {
    setup_profile(exhausted_noop, false)
}
fn setup_profile(
    exhausted_noop: bool,
    with_result: bool,
) -> (
    SourceCoordinator,
    InvocationId,
    SettingsExecutionContextV1,
    AnnotationId,
    Recorder,
) {
    let mut r = Recorder::try_with_capacity(256)
        .unwrap()
        .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
    let (mut o, source, annotation) = crate::coordinator::annotation::tests::setup(&mut r);
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    o.configure_retention_budget(8388608, &mut r).unwrap();
    let expected = if exhausted_noop {
        crate::coordinator::annotation::tests::exhaust_content_for_test(&mut o, &annotation);
        o.generation = u64::MAX;
        u64::MAX
    } else {
        1
    };
    let ticket = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let reads = ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 16,
            max_work: 1024,
        },
    };
    let raw = [
        ("contract", V::Data(D::Text("annotation-authoring/1"))),
        ("operation", V::Data(D::Text("update-body"))),
        ("invocation", V::Invocation(&ticket)),
        ("document", V::Document(&source.document)),
        ("profile", V::Data(D::Text(&c.profile))),
        ("dependencies", V::Dependencies(&reads)),
        ("annotation", V::Annotation(&annotation)),
        ("expectedContent", V::Version(expected)),
        (
            "body",
            V::Data(D::Text(if exhausted_noop {
                "PRIVATE_OLD"
            } else {
                "PRIVATE_NEW"
            })),
        ),
    ];
    let q = normalize_annotation_authoring_arguments_v1(
        V::Object(&raw),
        AnnotationAuthoringArgumentLimitsV1 {
            max_body_bytes: 1024,
            max_profile_bytes: 128,
            max_dependencies: 16,
            max_owned_bytes: 16384,
        },
        &mut r,
    )
    .unwrap();
    if with_result {
        let a = TrustedAnnotationResultAdmissionV1 {
            context: &c,
            publication_epoch: 7,
            publication_allowed: Some(true),
            context_current: Some(true),
            result_admission_allowed: Some(true),
            max_result_bytes: AnnotationResultV1::required_bytes(),
            grant: Some(TrustedAnnotationResultGrantV1 {
                invocation: &ticket,
                authority: &c.authority,
                target: AnnotationResultGrantTargetV1::Annotation(&annotation),
                capabilities: AnnotationResultCapabilitiesV1::IdentityContentTarget,
            }),
        };
        o.submit_scoped_annotation_with_result_v1(&q, &c, &facts(&c), &a, &mut r)
            .unwrap();
    } else {
        o.submit_scoped_annotation_v1(&q, &c, &facts(&c), &mut r)
            .unwrap();
    }
    while r.pop_front().is_some() {}
    (o, ticket, c, annotation, r)
}
fn result_release<'a>(
    ticket: &'a InvocationId,
    c: &'a SettingsExecutionContextV1,
    annotation: &'a AnnotationId,
) -> TrustedAnnotationResultReleaseV1<'a> {
    TrustedAnnotationResultReleaseV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_context_current: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        max_result_bytes: AnnotationResultV1::required_bytes(),
        grant: Some(TrustedAnnotationResultGrantV1 {
            invocation: ticket,
            authority: &c.authority,
            target: AnnotationResultGrantTargetV1::Annotation(annotation),
            capabilities: AnnotationResultCapabilitiesV1::IdentityContentTarget,
        }),
    }
}
#[test]
fn filtered_noop_preserves_full_width_versions_without_unused_publication_capacity() {
    let (mut o, ticket, c, annotation, mut r) = setup_profile(true, true);
    o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
        .unwrap();
    let result = o
        .retained_annotation_result_v1(&ticket, &result_release(&ticket, &c, &annotation), &mut r)
        .unwrap();
    let AnnotationResultDecisionV1::Unchanged { record } = result.decision else {
        panic!()
    };
    assert_eq!(record.content_version, Some(u64::MAX));
    assert_eq!(record.target_generation, Some(1));
    assert_eq!(o.generation(), u64::MAX);
}
#[test]
fn filtered_receipt_checks_original_correlation_and_preserves_valid_history_after_later_domain_fault()
 {
    for corruption in 0..5 {
        let (mut o, ticket, c, annotation, mut r) = setup_profile(false, true);
        let other = o.issue_invocation(&mut r).unwrap();
        o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r)
            .unwrap();
        let original = o.invocations.slots[ticket.slot]
            .as_ref()
            .unwrap()
            .annotation_hold
            .as_ref()
            .unwrap()
            .receipt
            .clone();
        let published = o
            .retained_annotation_result_v1(
                &ticket,
                &result_release(&ticket, &c, &annotation),
                &mut r,
            )
            .unwrap();
        let generation = o.generation();
        let held = o.retention_usage();
        let receipt = o.invocations.slots[ticket.slot]
            .as_mut()
            .unwrap()
            .annotation_hold
            .as_mut()
            .unwrap()
            .receipt
            .as_mut()
            .unwrap();
        match corruption {
            0 => receipt.invocation = other.clone(),
            1 => receipt.operation = receipt::Operation::Target,
            _ => {
                let receipt::Decision::Applied {
                    publication,
                    record,
                } = &mut receipt.decision
                else {
                    panic!()
                };
                match corruption {
                    2 => *publication = other.clone(),
                    3 => record.content_version += 1,
                    _ => record.annotation.namespace = Arc::new(()),
                }
            }
        }
        assert_eq!(
            o.retained_annotation_result_v1(
                &ticket,
                &result_release(&ticket, &c, &annotation),
                &mut r
            ),
            Err(InvocationError::Faulted)
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
        // Fixture repair only; no public engine operation repairs corrupt receipts.
        o.invocations.slots[ticket.slot]
            .as_mut()
            .unwrap()
            .annotation_hold
            .as_mut()
            .unwrap()
            .receipt = original;
        o.faulted = true;
        assert_eq!(
            o.retained_annotation_result_v1(
                &ticket,
                &result_release(&ticket, &c, &annotation),
                &mut r
            )
            .unwrap(),
            published
        );
        assert!(o.is_faulted());
        assert_eq!(o.generation(), generation);
    }
}
#[test]
fn unpublished_protocol_fault_has_no_deliverable_receipt_even_after_protected_discharge() {
    let (mut o, ticket, c, annotation, mut r) = setup_profile(false, true);
    let Phase::AnnotationPending(p) = &mut o.invocations.slots[ticket.slot].as_mut().unwrap().phase
    else {
        panic!()
    };
    p.plan.as_mut().unwrap().annotations[0].id.namespace = Arc::new(());
    let generation = o.generation();
    assert_eq!(
        o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.retained_annotation_result_v1(&ticket, &result_release(&ticket, &c, &annotation), &mut r),
        Err(InvocationError::OutcomeUnavailable)
    );
    assert_eq!(o.generation(), generation);
    o.retire_scoped_annotation_v1(&ticket, &c, &retire(), &mut r)
        .unwrap();
    assert!(
        o.retained_annotation_result_v1(&ticket, &result_release(&ticket, &c, &annotation), &mut r)
            .is_err()
    );
    assert_eq!(o.retention_usage().reserved, 0);
}
fn control(c: &SettingsExecutionContextV1) -> TrustedInvocationControlV1<'_> {
    TrustedInvocationControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        control_context_current: Some(true),
        control_scope_valid: Some(true),
        protected_control_ready: Some(true),
    }
}
fn retire() -> AnnotationInvocationRetirementV1 {
    AnnotationInvocationRetirementV1 {
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
#[test]
fn altered_private_annotation_preparation_faults_before_publication_and_allows_protected_discharge()
{
    let (mut o, ticket, c, annotation, mut r) = setup(false);
    let generation = o.generation();
    let held = o.retention_usage().reserved;
    let value = o.annotations.records[annotation.slot]
        .as_ref()
        .unwrap()
        .value
        .clone();
    let Phase::AnnotationPending(p) = &mut o.invocations.slots[ticket.slot].as_mut().unwrap().phase
    else {
        unreachable!()
    };
    p.plan.as_mut().unwrap().annotations[0].id.namespace = Arc::new(());
    assert_eq!(
        o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.annotations.records[annotation.slot]
            .as_ref()
            .unwrap()
            .value,
        value
    );
    assert_eq!(o.retention_usage().reserved, held);
    o.retire_scoped_annotation_v1(&ticket, &c, &retire(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(
        o.invocation_outcome(&ticket, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.stop_invocation(&ticket, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.retire_invocation(&ticket, &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(
        std::iter::from_fn(|| r.pop_front())
            .any(|v| v.boundary == Boundary::AnnotationInvocationService
                && v.disposition == Disposition::Unknown
                && v.service_time.is_some())
    );
}
#[test]
fn a_broken_pending_seal_cannot_be_settled_as_stopped_or_block_faulted_capsule_discharge() {
    let (mut o, ticket, c, _, mut r) = setup(false);
    let (foreign, foreign_ticket, _, _, _) = setup(false);
    let Some(BoundIntent::Annotation(other)) = &foreign.invocations.slots[foreign_ticket.slot]
        .as_ref()
        .unwrap()
        .intent
    else {
        unreachable!()
    };
    let Phase::AnnotationPending(p) = &mut o.invocations.slots[ticket.slot].as_mut().unwrap().phase
    else {
        unreachable!()
    };
    p.capture = other.clone();
    assert_eq!(
        o.request_invocation_stop(&ticket, &control(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(
        !o.invocations.slots[ticket.slot]
            .as_ref()
            .unwrap()
            .stop_requested
    );
    o.retire_scoped_annotation_v1(&ticket, &c, &retire(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(matches!(
        o.invocations.slots[ticket.slot].as_ref().unwrap().phase,
        Phase::AnnotationFaulted
    ));
    assert_eq!(
        o.invocation_state(&ticket, &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.invocation_outcome(&ticket, &mut r),
        Err(InvocationError::Faulted)
    );
}
#[test]
fn swapped_same_parent_reservation_amount_is_not_valid_funding_or_refundable_as_the_capsule() {
    let (mut o, ticket, c, _, mut r) = setup(false);
    let before = o.retention_usage().reserved;
    let smaller = o.retention.reserve(1).unwrap();
    let hold = o.invocations.slots[ticket.slot]
        .as_mut()
        .unwrap()
        .annotation_hold
        .as_mut()
        .unwrap();
    let original = std::mem::replace(&mut hold.reservation, smaller);
    assert_eq!(
        o.request_invocation_stop(&ticket, &control(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(
        o.retire_scoped_annotation_v1(&ticket, &c, &retire(), &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(o.retention_usage().reserved, before + 1);
    // Restore only the test-corrupted witness; a real caller cannot swap either
    // private reservation. The actual hold then discharges without a clean outcome.
    let hold = o.invocations.slots[ticket.slot]
        .as_mut()
        .unwrap()
        .annotation_hold
        .as_mut()
        .unwrap();
    let mut smaller = std::mem::replace(&mut hold.reservation, original);
    o.retention.release(&mut smaller).unwrap();
    o.retire_scoped_annotation_v1(&ticket, &c, &retire(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(
        o.invocation_outcome(&ticket, &mut r),
        Err(InvocationError::Faulted)
    );
}
#[test]
fn scoped_noop_body_needs_no_unused_publication_or_content_counter() {
    let (mut o, ticket, c, annotation, mut r) = setup(true);
    assert_eq!(
        o.run_scoped_annotation_v1(&ticket, &c, &facts(&c), &mut r),
        Ok(InvocationState::Finished)
    );
    let InvocationOutcome::Committed(result) = o.invocation_outcome(&ticket, &mut r).unwrap()
    else {
        panic!("expected validated noop")
    };
    assert!(result.operation.is_none());
    assert!(result.annotation_changes.is_empty());
    assert_eq!(o.generation(), u64::MAX);
    assert_eq!(
        o.annotation(&annotation, &mut r).unwrap().content_version,
        u64::MAX
    );
}
