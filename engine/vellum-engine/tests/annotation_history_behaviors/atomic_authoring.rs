use super::*;
use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as D};
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "host".into(),
        engine: "engine".into(),
        recipient: "agent".into(),
        purpose: "author".into(),
        destination: "engine".into(),
        profile: BOUNDED_ATOMIC_AUTHORING_PROFILE_V1.into(),
        continuity: "session".into(),
    }
}
fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 16,
            max_work: 65536,
        },
    }
}
fn native<'a>(context: &'a SettingsExecutionContextV1) -> TrustedAnnotationInvocationV1<'a> {
    TrustedAnnotationInvocationV1 {
        context,
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
fn result_admission<'a>(
    context: &'a SettingsExecutionContextV1,
    grants: &'a [Option<TrustedCompoundAnnotationResultGrantV1<'a>>],
) -> TrustedCompoundAnnotationResultAdmissionV1<'a> {
    TrustedCompoundAnnotationResultAdmissionV1 {
        context,
        publication_epoch: 7,
        publication_allowed: Some(true),
        context_current: Some(true),
        result_admission_allowed: Some(true),
        max_result_bytes: 512,
        grants,
    }
}
fn normalize<'h>(
    owner: &SourceCoordinator,
    id: &'h InvocationId,
    source: &DocumentVersion,
    writes: &[DocumentEdits],
    mutations: &[AnnotationMutation],
    r: &mut Recorder,
) -> NormalizedAtomicAuthoringArgumentsV1<'h> {
    with_arguments(id, source, writes, mutations, |args| {
        owner
            .normalize_atomic_authoring_arguments_v1(
                args,
                &context(),
                CaptureLimitsV1 {
                    max_bytes: 32768,
                    max_nodes: 4096,
                    max_depth: 16,
                },
                r,
            )
            .unwrap()
    })
}
fn with_arguments<'h, T>(
    id: &'h InvocationId,
    source: &DocumentVersion,
    writes: &[DocumentEdits],
    mutations: &[AnnotationMutation],
    consume: impl FnOnce(AtomicAuthoringEnvelopeValueV1<'_, 'h>) -> T,
) -> T {
    use AtomicAuthoringEnvelopeValueV1 as V;
    let reads = reads();
    let components: Vec<_> = mutations
        .iter()
        .map(|mutation| AtomicAnnotationComponentV1 {
            document: &source.document,
            mutation,
            dependencies: &reads,
        })
        .collect();
    let scopes = if writes.is_empty() {
        vec![]
    } else {
        vec![SourceOperationScope {
            expected: source.clone(),
            replacement_ranges: vec![ByteRange { start: 0, end: 3 }],
            insertion_sites: vec![InsertionSiteRange { first: 0, last: 3 }],
        }]
    };
    let rows = [
        ("contract", V::Data(D::Text("atomic-source-annotation/1"))),
        ("operation", V::Data(D::Text("exact-batch"))),
        ("invocation", V::Invocation(id)),
        (
            "profile",
            V::Data(D::Text(BOUNDED_ATOMIC_AUTHORING_PROFILE_V1)),
        ),
        ("sources", V::Sources(writes)),
        ("annotations", V::Annotations(&components)),
        ("reads", V::Reads(&reads)),
        (
            "scopes",
            V::Scopes(
                &scopes,
                SourceScopeBudget {
                    max_scopes: 8,
                    max_records: 16,
                    max_work: 65536,
                },
            ),
        ),
    ];
    consume(V::Object(&rows))
}
#[test]
fn all_four_native_components_share_one_publication_receipt_order_and_linked_undo() {
    let mut r = Recorder::disabled();
    let mut o = configured(&mut r);
    let source = o.create(b"abc", &mut r).unwrap();
    let (body, _) = create(&mut o, target(&source, 0, 1), "old body", &mut r);
    let (position, _) = create(&mut o, target(&source, 1, 2), "move me", &mut r);
    let (deleted, _) = create(&mut o, target(&source, 2, 3), "delete me", &mut r);
    let history = o.history_usage().retained_groups;
    let id = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let mutations = [
        AnnotationMutation::Create {
            target: SourceTarget {
                source: source.clone(),
                target: AttachmentTarget::ContentRange { start: 3, end: 4 },
                basis: TargetBasis::PreparedSource,
            },
            body: "new".into(),
        },
        AnnotationMutation::UpdateBody {
            annotation: body.clone(),
            expected_content: 1,
            body: "new body".into(),
        },
        AnnotationMutation::Reattach {
            annotation: position.clone(),
            expected_target: 1,
            target: SourceTarget {
                source: source.clone(),
                target: AttachmentTarget::ContentRange { start: 0, end: 2 },
                basis: TargetBasis::PreparedSource,
            },
        },
        AnnotationMutation::Delete {
            annotation: deleted.clone(),
            expected_content: 1,
            expected_target: 1,
        },
    ];
    let q = normalize(
        &o,
        &id,
        &source,
        &[write(source.clone(), 3, 3, "", "!")],
        &mutations,
        &mut r,
    );
    let n = native(&c);
    let operations = [
        AnnotationResultOperationV1::Create,
        AnnotationResultOperationV1::UpdateBody,
        AnnotationResultOperationV1::Reattach,
        AnnotationResultOperationV1::Delete,
    ];
    let bindings: Vec<_> = operations
        .iter()
        .enumerate()
        .map(|(component, operation)| TrustedAtomicAnnotationBindingV1 {
            component,
            invocation: &id,
            document: &source.document,
            operation: *operation,
            native: &n,
        })
        .collect();
    let facts = TrustedAtomicAuthoringV1 {
        context: &c,
        components: &bindings,
        source_readable: Some(true),
        source_writable: Some(true),
        all_targets_writable: Some(true),
    };
    let targets = [
        CompoundAnnotationResultGrantTargetV1::CreatedByComponent {
            document: &source.document,
            component: 0,
        },
        CompoundAnnotationResultGrantTargetV1::Annotation(&body),
        CompoundAnnotationResultGrantTargetV1::Annotation(&position),
        CompoundAnnotationResultGrantTargetV1::Annotation(&deleted),
    ];
    let grants: Vec<_> = targets
        .iter()
        .map(|target| {
            Some(TrustedCompoundAnnotationResultGrantV1 {
                invocation: &id,
                authority: &c.authority,
                target: *target,
                capabilities: AnnotationResultCapabilitiesV1::IdentityContentTarget,
            })
        })
        .collect();
    o.submit_atomic_authoring_arguments_v1(&q, &facts, &result_admission(&c, &grants), &mut r)
        .unwrap();
    o.run_atomic_authoring_v1(&id, &facts, &mut r).unwrap();
    assert_eq!(
        o.read(&source.document, &mut r).unwrap().source.text(),
        "abc!"
    );
    assert_eq!(o.annotation(&body, &mut r).unwrap().body, "new body");
    assert!(matches!(
        o.annotation(&position, &mut r).unwrap().placement,
        Placement::Located {
            target: AttachmentTarget::ContentRange { start: 0, end: 2 },
            ..
        }
    ));
    assert!(o.annotation(&deleted, &mut r).is_err());
    assert_eq!(o.history_usage().retained_groups, history + 1);
    let group = match o.invocation_outcome(&id, &mut r).unwrap() {
        InvocationOutcome::Committed(v) => v.history_group.clone().unwrap(),
        _ => panic!(),
    };
    for index in 0..4 {
        let result = o
            .retained_compound_annotation_result_v1(
                &id,
                index,
                &TrustedCompoundAnnotationResultReleaseV1 {
                    context: &c,
                    publication_epoch: 7,
                    publication_allowed: Some(true),
                    outcome_readable: Some(true),
                    result_context_current: Some(true),
                    reply_ready: Some(true),
                    resources_ready: Some(true),
                    max_result_bytes: 512,
                    grant: grants[index],
                },
                &mut r,
            )
            .unwrap();
        assert_eq!(result.operation, operations[index]);
        assert!(matches!(
            result.decision,
            AnnotationResultDecisionV1::Applied { .. }
        ));
    }
    let undo = o
        .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
        .unwrap();
    o.commit(undo, &mut r).unwrap();
    assert_eq!(
        o.read(&source.document, &mut r).unwrap().source.text(),
        "abc"
    );
    assert_eq!(o.annotation(&body, &mut r).unwrap().body, "old body");
    assert!(o.annotation(&deleted, &mut r).is_ok());
}

#[test]
fn current_component_authority_cannot_be_replaced_by_source_permission() {
    for admission_denial in [false, true] {
        let mut r = Recorder::disabled();
        let mut o = configured(&mut r);
        let source = o.create(b"abc", &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let c = context();
        let mutations = [AnnotationMutation::Create {
            target: target(&source, 0, 1),
            body: "comment".into(),
        }];
        let q = normalize(
            &o,
            &id,
            &source,
            &[write(source.clone(), 0, 1, "a", "X")],
            &mutations,
            &mut r,
        );
        let mut n = native(&c);
        n.annotation_writable = Some(!admission_denial);
        let bindings = [TrustedAtomicAnnotationBindingV1 {
            component: 0,
            invocation: &id,
            document: &source.document,
            operation: AnnotationResultOperationV1::Create,
            native: &n,
        }];
        let facts = TrustedAtomicAuthoringV1 {
            context: &c,
            components: &bindings,
            source_readable: Some(true),
            source_writable: Some(true),
            all_targets_writable: Some(true),
        };
        let grants = [None];
        if admission_denial {
            assert_eq!(
                o.submit_atomic_authoring_arguments_v1(
                    &q,
                    &facts,
                    &result_admission(&c, &grants),
                    &mut r
                ),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
            assert_eq!(o.retention_usage().reserved, 0);
            assert_eq!(o.annotation_usage().issued_lifetime, 0);
        } else {
            o.submit_atomic_authoring_arguments_v1(
                &q,
                &facts,
                &result_admission(&c, &grants),
                &mut r,
            )
            .unwrap();
            let mut revoked = native(&c);
            revoked.annotation_writable = Some(false);
            let changed_binding = [TrustedAtomicAnnotationBindingV1 {
                component: 0,
                invocation: &id,
                document: &source.document,
                operation: AnnotationResultOperationV1::Create,
                native: &revoked,
            }];
            let revoked = TrustedAtomicAuthoringV1 {
                context: &c,
                components: &changed_binding,
                source_readable: Some(true),
                source_writable: Some(true),
                all_targets_writable: Some(true),
            };
            o.run_atomic_authoring_v1(&id, &revoked, &mut r).unwrap();
            assert!(matches!(
                o.invocation_outcome(&id, &mut r).unwrap(),
                InvocationOutcome::Rejected(_)
            ));
        }
        assert_eq!(
            o.read(&source.document, &mut r).unwrap().source.text(),
            "abc"
        );
        assert_eq!(o.annotation_usage().present_records, 0);
        assert_eq!(o.history_usage().retained_groups, 0);
    }
}

#[test]
fn body_only_component_survives_compatible_mapping_but_unwritten_candidate_keeps_its_old_source() {
    for body_only in [false, true] {
        let mut r = Recorder::disabled();
        let mut o = configured(&mut r);
        let source = o.create(b"abc", &mut r).unwrap();
        let c = context();
        let (existing, _) = create(&mut o, target(&source, 1, 2), "old", &mut r);
        let id = o.issue_invocation(&mut r).unwrap();
        let m = if body_only {
            AnnotationMutation::UpdateBody {
                annotation: existing.clone(),
                expected_content: 1,
                body: "new".into(),
            }
        } else {
            AnnotationMutation::Create {
                target: SourceTarget {
                    source: source.clone(),
                    target: AttachmentTarget::ContentRange { start: 1, end: 2 },
                    basis: TargetBasis::PreparedSource,
                },
                body: "note".into(),
            }
        };
        let q = normalize(&o, &id, &source, &[], std::slice::from_ref(&m), &mut r);
        let n = native(&c);
        let bindings = [TrustedAtomicAnnotationBindingV1 {
            component: 0,
            invocation: &id,
            document: &source.document,
            operation: if body_only {
                AnnotationResultOperationV1::UpdateBody
            } else {
                AnnotationResultOperationV1::Create
            },
            native: &n,
        }];
        let f = TrustedAtomicAuthoringV1 {
            context: &c,
            components: &bindings,
            source_readable: Some(false),
            source_writable: Some(false),
            all_targets_writable: Some(false),
        };
        o.submit_atomic_authoring_arguments_v1(&q, &f, &result_admission(&c, &[None]), &mut r)
            .unwrap();
        let p = o
            .prepare(&[write(source.clone(), 0, 0, "", "X")], &[], &mut r)
            .unwrap();
        o.commit(p, &mut r).unwrap();
        o.run_atomic_authoring_v1(&id, &f, &mut r).unwrap();
        assert_eq!(
            o.read(&source.document, &mut r).unwrap().source.text(),
            "Xabc"
        );
        if body_only {
            assert_eq!(o.annotation(&existing, &mut r).unwrap().body, "new");
        } else {
            assert!(matches!(
                o.invocation_outcome(&id, &mut r).unwrap(),
                InvocationOutcome::Rejected(_)
            ));
        }
    }
}

fn configured(r: &mut Recorder) -> SourceCoordinator {
    let mut o = owner();
    o.configure_semantic_root_budget(1048576, r).unwrap();
    o.configure_retention_budget(8388608, r).unwrap();
    o
}

#[test]
fn canonical_source_partitions_do_not_multiply_participants_or_the_original_parent_charge() {
    let mut r = Recorder::disabled();
    let mut o = configured(&mut r);
    let c = context();
    let source = o.create(b"abc", &mut r).unwrap();
    let edit = write(source.clone(), 3, 3, "", "!");
    let m = AnnotationMutation::Create {
        target: target(&source, 0, 1),
        body: "note".into(),
    };
    let partitioned: Vec<_> = (0..16)
        .map(|i| {
            if i == 0 {
                edit.clone()
            } else {
                DocumentEdits {
                    expected: source.clone(),
                    edits: vec![],
                }
            }
        })
        .collect();
    let mut previous_charge = None;
    for writes in [std::slice::from_ref(&edit), partitioned.as_slice()] {
        let id = o.issue_invocation(&mut r).unwrap();
        let q = normalize(&o, &id, &source, writes, std::slice::from_ref(&m), &mut r);
        let n = native(&c);
        let bindings = [TrustedAtomicAnnotationBindingV1 {
            component: 0,
            invocation: &id,
            document: &source.document,
            operation: AnnotationResultOperationV1::Create,
            native: &n,
        }];
        let f = TrustedAtomicAuthoringV1 {
            context: &c,
            components: &bindings,
            source_readable: Some(true),
            source_writable: Some(true),
            all_targets_writable: Some(true),
        };
        let before = o.retention_usage().reserved;
        o.submit_atomic_authoring_arguments_v1(&q, &f, &result_admission(&c, &[None]), &mut r)
            .unwrap();
        let charge = o.retention_usage().reserved - before;
        if let Some(previous) = previous_charge {
            assert_eq!(charge, previous);
        }
        previous_charge = Some(charge);
        let same = normalize(
            &o,
            &id,
            &source,
            std::slice::from_ref(&edit),
            std::slice::from_ref(&m),
            &mut r,
        );
        assert_eq!(
            o.submit_atomic_authoring_arguments_v1(
                &same,
                &f,
                &result_admission(&c, &[None]),
                &mut r
            ),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.retention_usage().reserved, before + charge);
    }
}

#[test]
fn mixed_create_reaches_existing_typed_retained_outcome_facet_with_independent_release_rights() {
    let mut r = Recorder::disabled();
    let mut o = configured(&mut r);
    let source = o.create(b"abc", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
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
    let pf = TrustedInvocationPolicyFactsV1 {
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
        &pf,
        &mut r,
    )
    .unwrap();
    let mutation = AnnotationMutation::Create {
        target: target(&source, 0, 1),
        body: "comment".into(),
    };
    let q = normalize(&o, &id, &source, &[], &[mutation], &mut r);
    let n = native(&c);
    let bindings = [TrustedAtomicAnnotationBindingV1 {
        component: 0,
        invocation: &id,
        document: &source.document,
        operation: AnnotationResultOperationV1::Create,
        native: &n,
    }];
    let f = TrustedAtomicAuthoringV1 {
        context: &c,
        components: &bindings,
        source_readable: Some(false),
        source_writable: Some(false),
        all_targets_writable: Some(false),
    };
    let grant = TrustedCompoundAnnotationResultGrantV1 {
        invocation: &id,
        authority: &c.authority,
        target: CompoundAnnotationResultGrantTargetV1::CreatedByComponent {
            document: &source.document,
            component: 0,
        },
        capabilities: AnnotationResultCapabilitiesV1::IdentityContentTarget,
    };
    o.submit_atomic_authoring_arguments_v1(&q, &f, &result_admission(&c, &[Some(grant)]), &mut r)
        .unwrap();
    let mut l = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 0,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: std::mem::size_of::<InvocationControlReferenceV1>()
            .max(std::mem::size_of::<InvocationObservationReferenceV1>()),
        max_reply_bytes: InvocationReceiptV4::required_reply_bytes(512).unwrap(),
        max_control_reply_bytes: std::mem::size_of::<InvocationControlObservationV1>(),
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    let mut observer = o
        .open_invocation_observation_with_receipts_v4(
            &id,
            &c,
            &policy,
            l,
            InvocationReceiptAdmissionV1 {
                observation: &pf,
                capabilities: InvocationReceiptCapabilitiesV1 {
                    atomic_annotation: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    o.run_atomic_authoring_v1(&id, &f, &mut r).unwrap();
    let release = TrustedInvocationObservationReleaseV1 {
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
        current: &release,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let mut common = TrustedInvocationReceiptReleaseV1 {
        observation: &ordinary,
        operation_receipt_allowed: Some(false),
    };
    let mut native = TrustedCompoundAnnotationResultReleaseV1 {
        context: &c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_context_current: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        max_result_bytes: 512,
        grant: Some(grant),
    };
    assert!(matches!(
        o.query_invocation_receipt_v4(
            &mut observer,
            InvocationReceiptRequestV4::AtomicAnnotation {
                index: 0,
                native: &native
            },
            &policy,
            &common,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    ));
    common.operation_receipt_allowed = Some(true);
    native.outcome_readable = Some(false);
    assert!(matches!(
        o.query_invocation_receipt_v4(
            &mut observer,
            InvocationReceiptRequestV4::AtomicAnnotation {
                index: 0,
                native: &native
            },
            &policy,
            &common,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    ));
    native.outcome_readable = Some(true);
    assert!(matches!(
        o.query_invocation_receipt_v4(
            &mut observer,
            InvocationReceiptRequestV4::AtomicAnnotation {
                index: 0,
                native: &native
            },
            &policy,
            &common,
            &mut r
        )
        .unwrap(),
        InvocationReceiptV4::AtomicAnnotation(AnnotationResultV1 {
            decision: AnnotationResultDecisionV1::Applied { .. },
            ..
        })
    ));
    let ctl = InvocationObservationPolicyControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    o.retire_invocation_observation_v1(&mut observer, &ctl, &mut r)
        .unwrap();
    o.retire_invocation_observation_policy_v1(&id, &ctl, &mut r)
        .unwrap();
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
    o.retire_compound_annotation_results_v1(&id, &ctl, &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}

#[test]
fn installed_typed_dynamic_commands_preserve_stop_receipts_and_capture_equivalence() {
    use AtomicAuthoringEnvelopeValueV1 as V;
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for stop in [false, true] {
        for mode in 0..4 {
            let mut r = match mode {
                0 => Recorder::disabled(),
                1 => Recorder::try_with_capacity(4096).unwrap(),
                2 => Recorder::try_with_capacity(4096)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
                _ => Recorder::try_with_capacity(1)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
            };
            if mode == 3 {
                r.record(Observation {
                    boundary: Boundary::InvocationAdmission,
                    disposition: Disposition::Completed,
                    service_time: None,
                });
            }
            let mut o = configured(&mut r);
            let source = o.create(b"abc", &mut r).unwrap();
            let c = context();
            let route = selection_route::Route::install_atomic_authoring(&mut o, &mut r);
            let id = o.issue_invocation(&mut r).unwrap();
            let m = AnnotationMutation::Create {
                target: target(&source, 0, 1),
                body: "note".into(),
            };
            let q = with_arguments(
                &id,
                &source,
                &[write(source.clone(), 0, 1, "a", "X")],
                std::slice::from_ref(&m),
                |args| {
                    let typed = mode % 2 == 0;
                    let h = [
                        (
                            if typed { "family" } else { "name" },
                            if typed {
                                V::Data(D::Number(10.0))
                            } else {
                                V::Data(D::Text("change-source"))
                            },
                        ),
                        ("variant", V::Data(D::Text("exact-batch"))),
                        ("version", V::Data(D::Number(1.0))),
                        ("arguments", args),
                    ];
                    let outer = [
                        (
                            "route",
                            V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
                        ),
                        ("request", V::Object(&h)),
                    ];
                    o.normalize_atomic_authoring_envelope_v1(
                        V::Object(&outer),
                        &c,
                        CaptureLimitsV1 {
                            max_bytes: 32768,
                            max_nodes: 4096,
                            max_depth: 16,
                        },
                        &mut r,
                    )
                    .unwrap()
                },
            );
            let n = native(&c);
            let bindings = [TrustedAtomicAnnotationBindingV1 {
                component: 0,
                invocation: &id,
                document: &source.document,
                operation: AnnotationResultOperationV1::Create,
                native: &n,
            }];
            let f = TrustedAtomicAuthoringV1 {
                context: &c,
                components: &bindings,
                source_readable: Some(true),
                source_writable: Some(true),
                all_targets_writable: Some(true),
            };
            let grants = [None];
            let a = result_admission(&c, &grants);
            route
                .submit_atomic_authoring(&mut o, &q, &f, &a, &mut r)
                .unwrap();
            let funded = o.retention_usage().reserved;
            assert_eq!(
                o.submit_atomic_authoring_arguments_v1(&q, &f, &a, &mut r),
                Ok(InvocationState::Pending)
            );
            assert_eq!(o.retention_usage().reserved, funded);
            if stop {
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
            }
            route.retire(&mut o, &mut r);
            assert!(o.retention_usage().reserved > 0);
            o.run_atomic_authoring_v1(&id, &f, &mut r).unwrap();
            if stop {
                assert!(matches!(
                    o.invocation_outcome(&id, &mut r).unwrap(),
                    InvocationOutcome::StoppedBeforePublication
                ));
            } else {
                assert!(matches!(
                    o.invocation_outcome(&id, &mut r).unwrap(),
                    InvocationOutcome::Committed(_)
                ));
            }
            assert_eq!(
                o.read(&source.document, &mut r).unwrap().source.text(),
                if stop { "abc" } else { "Xbc" }
            );
            assert_eq!(o.annotation_usage().present_records, usize::from(!stop));
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
            let mut found = [false; 4];
            while let Some(event) = r.pop_front() {
                let i = match event.boundary {
                    Boundary::AtomicAuthoringArgumentNormalization => Some(0),
                    Boundary::AtomicAuthoringAdmission => Some(1),
                    Boundary::AtomicAuthoringService => Some(2),
                    Boundary::DiscoveryAtomicAuthoringDispatch => Some(3),
                    _ => None,
                };
                if let Some(i) = i {
                    found[i] = true;
                    assert_eq!(event.service_time.is_some(), mode == 2);
                }
            }
            assert_eq!(found, [mode == 1 || mode == 2; 4]);
        }
    }
}

#[test]
fn closed_request_bounds_and_actual_parent_refuse_without_claiming_the_ticket() {
    use AtomicAuthoringEnvelopeValueV1 as V;
    let mut r = Recorder::disabled();
    let mut o = owner();
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    o.configure_retention_budget(1, &mut r).unwrap();
    let source = o.create(b"abc", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let mutations = [AnnotationMutation::Create {
        target: target(&source, 0, 1),
        body: "note".into(),
    }];
    for bad in 0..4 {
        with_arguments(&id, &source, &[], &mutations, |args| {
            let V::Object(rows) = args else { panic!() };
            let mut rows = rows.to_vec();
            match bad {
                0 => rows.push(("componentBindings", V::Data(D::Array(&[])))),
                1 => rows[0] = ("operation", V::Data(D::Text("exact-batch"))),
                2 => rows[2].1 = V::Data(D::Text("fake-id")),
                _ => rows[5].1 = V::Data(D::Array(&[])),
            }
            assert!(matches!(
                o.normalize_atomic_authoring_arguments_v1(
                    V::Object(&rows),
                    &c,
                    CaptureLimitsV1 {
                        max_bytes: 32768,
                        max_nodes: 4096,
                        max_depth: 16
                    },
                    &mut r
                ),
                Err(InvocationError::MalformedIntent)
            ));
        });
    }
    for limits in [
        CaptureLimitsV1 {
            max_bytes: 1,
            max_nodes: 4096,
            max_depth: 16,
        },
        CaptureLimitsV1 {
            max_bytes: 32768,
            max_nodes: 1,
            max_depth: 16,
        },
        CaptureLimitsV1 {
            max_bytes: 32768,
            max_nodes: 4096,
            max_depth: 1,
        },
    ] {
        with_arguments(&id, &source, &[], &mutations, |args| {
            assert!(matches!(
                o.normalize_atomic_authoring_arguments_v1(args, &c, limits, &mut r),
                Err(InvocationError::Limit)
            ))
        });
    }
    let q = normalize(&o, &id, &source, &[], &mutations, &mut r);
    let n = native(&c);
    let bindings = [TrustedAtomicAnnotationBindingV1 {
        component: 0,
        invocation: &id,
        document: &source.document,
        operation: AnnotationResultOperationV1::Create,
        native: &n,
    }];
    let f = TrustedAtomicAuthoringV1 {
        context: &c,
        components: &bindings,
        source_readable: Some(false),
        source_writable: Some(false),
        all_targets_writable: Some(false),
    };
    assert_eq!(
        o.submit_atomic_authoring_arguments_v1(&q, &f, &result_admission(&c, &[None]), &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(o.annotation_usage().issued_lifetime, 0);
}

#[test]
fn source_and_candidate_annotation_publish_once_with_original_receipt_and_funded_cleanup() {
    let mut r = Recorder::disabled();
    let mut o = configured(&mut r);
    let source = o.create(b"abc", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let mutations = [AnnotationMutation::Create {
        target: SourceTarget {
            source: source.clone(),
            target: AttachmentTarget::ContentRange { start: 3, end: 4 },
            basis: TargetBasis::PreparedSource,
        },
        body: "comment on new content".into(),
    }];
    let q = normalize(
        &o,
        &id,
        &source,
        &[write(source.clone(), 3, 3, "", "!")],
        &mutations,
        &mut r,
    );
    let native = native(&c);
    let bindings = [TrustedAtomicAnnotationBindingV1 {
        component: 0,
        invocation: &id,
        document: &source.document,
        operation: AnnotationResultOperationV1::Create,
        native: &native,
    }];
    let facts = TrustedAtomicAuthoringV1 {
        context: &c,
        components: &bindings,
        source_readable: Some(true),
        source_writable: Some(true),
        all_targets_writable: Some(true),
    };
    let grant = TrustedCompoundAnnotationResultGrantV1 {
        invocation: &id,
        authority: &c.authority,
        target: CompoundAnnotationResultGrantTargetV1::CreatedByComponent {
            document: &source.document,
            component: 0,
        },
        capabilities: AnnotationResultCapabilitiesV1::IdentityContentTarget,
    };
    let grants = [Some(grant)];
    let a = result_admission(&c, &grants);
    o.submit_atomic_authoring_arguments_v1(&q, &facts, &a, &mut r)
        .unwrap();
    let funded = o.retention_usage();
    let issued = o.annotation_usage().issued_lifetime;
    assert!(funded.reserved > 0);
    assert_eq!(
        o.submit_atomic_authoring_arguments_v1(&q, &facts, &a, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(o.annotation_usage().issued_lifetime, issued);
    assert_eq!(o.retention_usage(), funded);
    assert_eq!(o.run(&id, &mut r), Err(InvocationError::Unavailable));
    o.run_atomic_authoring_v1(&id, &facts, &mut r).unwrap();
    assert_eq!(
        o.read(&source.document, &mut r).unwrap().source.text(),
        "abc!"
    );
    assert_eq!(o.history_usage().retained_groups, 1);
    let released = o
        .retained_compound_annotation_result_v1(
            &id,
            0,
            &TrustedCompoundAnnotationResultReleaseV1 {
                context: &c,
                publication_epoch: 7,
                publication_allowed: Some(true),
                outcome_readable: Some(true),
                result_context_current: Some(true),
                reply_ready: Some(true),
                resources_ready: Some(true),
                max_result_bytes: 512,
                grant: Some(grant),
            },
            &mut r,
        )
        .unwrap();
    let AnnotationResultDecisionV1::Applied { record, .. } = released.decision else {
        panic!()
    };
    let record = o.annotation(&record.annotation, &mut r).unwrap();
    assert_eq!(record.body, "comment on new content");
    assert!(matches!(
        record.placement,
        Placement::Located {
            target: AttachmentTarget::ContentRange { start: 3, end: 4 },
            ..
        }
    ));
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
    assert_eq!(o.retention_usage(), funded);
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
    assert!(
        o.retire_compound_annotation_results_v1(
            &id,
            &InvocationObservationPolicyControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true)
            },
            &mut r
        )
        .is_err()
    );
    o.retire_invocation(&id, &mut r).unwrap();
}

#[test]
fn annotation_only_candidate_without_source_write_keeps_original_meaning_and_needs_no_source_grant()
{
    let mut r = Recorder::disabled();
    let mut o = configured(&mut r);
    let source = o.create(b"abc", &mut r).unwrap();
    o.set_read_only(&source.document, true, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let mutations = [AnnotationMutation::Create {
        target: SourceTarget {
            source: source.clone(),
            target: AttachmentTarget::Point {
                offset: 1,
                affinity: vellum_engine::attachment::Affinity::Before,
                profile: vellum_engine::attachment::PointProfile::Position,
            },
            basis: TargetBasis::PreparedSource,
        },
        body: "note".into(),
    }];
    let q = normalize(&o, &id, &source, &[], &mutations, &mut r);
    let native = native(&c);
    let bindings = [TrustedAtomicAnnotationBindingV1 {
        component: 0,
        invocation: &id,
        document: &source.document,
        operation: AnnotationResultOperationV1::Create,
        native: &native,
    }];
    let facts = TrustedAtomicAuthoringV1 {
        context: &c,
        components: &bindings,
        source_readable: Some(false),
        source_writable: Some(false),
        all_targets_writable: Some(false),
    };
    let grants = [None];
    o.submit_atomic_authoring_arguments_v1(&q, &facts, &result_admission(&c, &grants), &mut r)
        .unwrap();
    o.run_atomic_authoring_v1(&id, &facts, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::Committed(_)
    ));
    assert_eq!(o.read(&source.document, &mut r).unwrap().version, source);
}
