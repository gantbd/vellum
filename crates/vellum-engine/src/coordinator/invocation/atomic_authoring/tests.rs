use super::*;
use crate::capture::InertValueV1 as D;
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: BOUNDED_ATOMIC_AUTHORING_PROFILE_V1.into(),
        continuity: "c".into(),
    }
}
fn native<'a>(c: &'a SettingsExecutionContextV1) -> TrustedAnnotationInvocationV1<'a> {
    TrustedAnnotationInvocationV1 {
        context: c,
        limits: AnnotationInvocationLimitsV1 {
            max_context_bytes: 2048,
            max_capture_bytes: 32768,
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
fn prepared(
    o: &mut SourceCoordinator,
    source: &DocumentVersion,
    id: &InvocationId,
    c: &SettingsExecutionContextV1,
    r: &mut Recorder,
) {
    use AtomicAuthoringEnvelopeValueV1 as V;
    let mutation = AnnotationMutation::Create {
        target: SourceTarget {
            source: source.clone(),
            target: crate::attachment::AttachmentTarget::Document,
            basis: TargetBasis::Before,
        },
        body: "new".into(),
    };
    let reads = ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 1024,
        },
    };
    let components = [AtomicAnnotationComponentV1 {
        document: &source.document,
        mutation: &mutation,
        dependencies: &reads,
    }];
    let rows = [
        ("contract", V::Data(D::Text("atomic-source-annotation/1"))),
        ("operation", V::Data(D::Text("exact-batch"))),
        ("invocation", V::Invocation(id)),
        (
            "profile",
            V::Data(D::Text(BOUNDED_ATOMIC_AUTHORING_PROFILE_V1)),
        ),
        ("sources", V::Sources(&[])),
        ("annotations", V::Annotations(&components)),
        ("reads", V::Reads(&reads)),
        (
            "scopes",
            V::Scopes(
                &[],
                SourceScopeBudget {
                    max_scopes: 8,
                    max_records: 8,
                    max_work: 1024,
                },
            ),
        ),
    ];
    let q = o
        .normalize_atomic_authoring_arguments_v1(
            V::Object(&rows),
            c,
            CaptureLimitsV1 {
                max_bytes: 32768,
                max_nodes: 1024,
                max_depth: 16,
            },
            r,
        )
        .unwrap();
    let n = native(c);
    let bindings = [TrustedAtomicAnnotationBindingV1 {
        component: 0,
        invocation: id,
        document: &source.document,
        operation: AnnotationResultOperationV1::Create,
        native: &n,
    }];
    o.submit_atomic_authoring_arguments_v1(
        &q,
        &TrustedAtomicAuthoringV1 {
            context: c,
            components: &bindings,
            source_readable: Some(false),
            source_writable: Some(false),
            all_targets_writable: Some(false),
        },
        &TrustedCompoundAnnotationResultAdmissionV1 {
            context: c,
            publication_epoch: 7,
            publication_allowed: Some(true),
            context_current: Some(true),
            result_admission_allowed: Some(true),
            max_result_bytes: 512,
            grants: &[None],
        },
        r,
    )
    .unwrap();
}
fn setup() -> (
    SourceCoordinator,
    DocumentVersion,
    InvocationId,
    SettingsExecutionContextV1,
    Recorder,
) {
    let mut r = Recorder::disabled();
    let (mut o, d, _) = annotation::tests::setup(&mut r);
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    o.configure_retention_budget(8388608, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let c = context();
    prepared(&mut o, &d, &id, &c, &mut r);
    (o, d, id, c, r)
}

#[test]
fn actual_inventory_keeps_both_parent_frames_after_stop_and_forget_until_protected_retirement() {
    use crate::coordinator::shutdown::tests::invocation_funding;
    let (mut o, _, id, c, mut r) = setup();
    let charge = o.retention_usage().reserved;
    invocation_funding(&o, &id, 1, charge, &mut r);
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
    o.run(&id, &mut r).unwrap();
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
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(matches!(
        o.invocation_slot(&id).unwrap().intent,
        Some(BoundIntent::RetiredAtomicAuthoring)
    ));
}

#[test]
fn corrupted_original_funding_seal_faults_without_a_published_prefix_and_keeps_responsibility() {
    let (mut o, d, id, c, mut r) = setup();
    let before = o.retention_usage();
    let generation = o.generation;
    let history = o.history_usage();
    let slot = o.invocations.slots[id.slot].as_mut().unwrap();
    let Some(BoundIntent::AtomicAuthoring(capture)) = &mut slot.intent else {
        panic!()
    };
    let original = capture.base_charge;
    Arc::get_mut(capture).unwrap().base_charge += 1;
    let n = native(&c);
    let bindings = [TrustedAtomicAnnotationBindingV1 {
        component: 0,
        invocation: &id,
        document: &d.document,
        operation: AnnotationResultOperationV1::Create,
        native: &n,
    }];
    assert_eq!(
        o.run_atomic_authoring_v1(
            &id,
            &TrustedAtomicAuthoringV1 {
                context: &c,
                components: &bindings,
                source_readable: Some(false),
                source_writable: Some(false),
                all_targets_writable: Some(false)
            },
            &mut r
        ),
        Err(InvocationError::Faulted)
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation, generation);
    assert_eq!(o.history_usage(), history);
    assert_eq!(o.retention_usage(), before);
    assert_eq!(
        o.documents[d.document.slot].as_ref().unwrap().source.text(),
        "abc"
    );
    assert_eq!(o.annotation_usage().present_records, 1);
    // Restore only the injected fixture corruption, then exercise protected
    // discharge while preserving the engine's unknown/faulted classification.
    let Some(BoundIntent::AtomicAuthoring(capture)) =
        &mut o.invocations.slots[id.slot].as_mut().unwrap().intent
    else {
        panic!()
    };
    Arc::get_mut(capture).unwrap().base_charge = original;
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
    assert!(o.is_faulted());
}
