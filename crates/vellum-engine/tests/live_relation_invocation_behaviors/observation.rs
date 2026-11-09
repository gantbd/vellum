use super::*;
use std::mem::size_of;
#[path = "../support/invocation_observation_fixture.rs"]
mod common;
#[path = "../support/retained_release_fixture.rs"]
mod prior;
fn profile() -> common::Profile {
    common::Profile {
        epoch: 7,
        maximum: 512,
        capabilities: InvocationReceiptCapabilitiesV1 {
            lifecycle: true,
            ..Default::default()
        },
    }
}
fn limits() -> InvocationObservationLimitsV1 {
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
    l
}
fn open(
    o: &mut SourceCoordinator,
    id: &InvocationId,
    c: &SettingsExecutionContextV1,
    r: &mut Recorder,
) -> InvocationObservationContextV1 {
    o.open_invocation_observation_with_receipts_v5(
        id,
        c,
        &common::policy(c, 7),
        limits(),
        InvocationReceiptAdmissionV1 {
            observation: &common::facts(c),
            capabilities: profile().capabilities,
            operation_receipt_admission_allowed: Some(true),
        },
        r,
    )
    .unwrap()
}
fn query<'a>(
    o: &'a SourceCoordinator,
    v: &mut InvocationObservationContextV1,
    c: &SettingsExecutionContextV1,
    n: &TrustedLiveRelationResultV1<'_>,
    r: &mut Recorder,
) -> Result<InvocationReceiptV5<'a>, InvocationError> {
    query_with(o, v, c, n, (&common::policy(c, 7), Some(true)), r)
}
fn query_with<'a>(
    o: &'a SourceCoordinator,
    v: &mut InvocationObservationContextV1,
    c: &SettingsExecutionContextV1,
    n: &TrustedLiveRelationResultV1<'_>,
    gate: (&InvocationObservationPolicyV1, Option<bool>),
    r: &mut Recorder,
) -> Result<InvocationReceiptV5<'a>, InvocationError> {
    query_request(
        o,
        v,
        c,
        InvocationReceiptRequestV5::LiveRelation(n),
        gate,
        r,
    )
}
fn query_request<'a>(
    o: &'a SourceCoordinator,
    v: &mut InvocationObservationContextV1,
    c: &SettingsExecutionContextV1,
    request: InvocationReceiptRequestV5<'_>,
    gate: (&InvocationObservationPolicyV1, Option<bool>),
    r: &mut Recorder,
) -> Result<InvocationReceiptV5<'a>, InvocationError> {
    let current = TrustedInvocationObservationReleaseV1 {
        context: c,
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
    o.query_invocation_receipt_v5(
        v,
        request,
        gate.0,
        &TrustedInvocationReceiptReleaseV1 {
            observation: &ordinary,
            operation_receipt_allowed: gate.1,
        },
        r,
    )
}

#[test]
fn v5_common_and_native_original_disclosure_ceilings_are_six_independent_meets() {
    for bits in 0..64 {
        let (mut o, mut r) = owner();
        let c = context();
        let f = facts(&c);
        let q = request(&mut o, &mut r, 2);
        let id = o.issue_invocation(&mut r).unwrap();
        let mut original = common::policy(&c, 7);
        original.publication = bits & 1 != 0;
        o.bind_invocation_observation_policy_v1(
            &id,
            &c,
            &original,
            InvocationObservationPolicyLimitsV1 {
                max_context_bytes: 2048,
                max_result_bytes: 512,
            },
            &common::facts(&c),
            &mut r,
        )
        .unwrap();
        let mut a = admission(&f);
        a.publication_allowed = Some(bits & 8 != 0);
        a.result_admission_allowed = Some(bits & 16 != 0);
        a.result_identifiers_allowed = Some(bits & 32 != 0);
        o.submit_live_relation_v1(&id, &q, &a, &mut r).unwrap();
        let mut captured = common::policy(&c, 7);
        captured.publication = bits & 2 != 0;
        let mut v = o
            .open_invocation_observation_with_receipts_v5(
                &id,
                &c,
                &captured,
                limits(),
                InvocationReceiptAdmissionV1 {
                    observation: &common::facts(&c),
                    capabilities: profile().capabilities,
                    operation_receipt_admission_allowed: Some(true),
                },
                &mut r,
            )
            .unwrap();
        o.run_live_relation_v1(&id, &f, &mut r).unwrap();
        let generation = o.generation();
        let held = o.retention_usage();
        let mut current = common::policy(&c, 7);
        current.publication = bits & 4 != 0;
        assert_eq!(
            query_with(&o, &mut v, &c, &release(&c), (&current, Some(true)), &mut r).is_ok(),
            bits == 63
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
    }
}

#[test]
fn v5_current_native_and_common_result_denials_preserve_later_valid_delivery() {
    let (mut o, mut r) = owner();
    let c = context();
    let mut wrong = c.clone();
    wrong.recipient = "other".into();
    let f = facts(&c);
    let q = request(&mut o, &mut r, 0);
    let id = o.issue_invocation(&mut r).unwrap();
    profile().bind(&mut o, &id, &c, &mut r);
    o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    let mut v = open(&mut o, &id, &c, &mut r);
    o.run_live_relation_v1(&id, &f, &mut r).unwrap();
    let generation = o.generation();
    let held = o.retention_usage();
    for bad in 0..19 {
        let mut n = release(&c);
        let mut permission = Some(true);
        match bad {
            0 => n.publication_allowed = None,
            1 => n.publication_allowed = Some(false),
            2 => n.outcome_readable = None,
            3 => n.outcome_readable = Some(false),
            4 => n.result_readable = None,
            5 => n.result_readable = Some(false),
            6 => n.result_context_current = None,
            7 => n.result_context_current = Some(false),
            8 => n.historical_document_readable = None,
            9 => n.historical_document_readable = Some(false),
            10 => n.reply_ready = None,
            11 => n.reply_ready = Some(false),
            12 => n.resources_ready = None,
            13 => n.resources_ready = Some(false),
            14 => n.context = &wrong,
            15 => n.publication_epoch = 8,
            16 => n.max_result_bytes = 0,
            17 => permission = None,
            _ => permission = Some(false),
        }
        assert!(
            query_with(
                &o,
                &mut v,
                &c,
                &n,
                (&common::policy(&c, 7), permission),
                &mut r
            )
            .is_err()
        );
        assert!(query(&o, &mut v, &c, &release(&c), &mut r).is_ok());
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
    }
}

#[test]
fn v5_requires_original_profile_family_and_complete_reply_frame_without_refunding_or_upgrading() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let q = request(&mut o, &mut r, 2);
    let id = o.issue_invocation(&mut r).unwrap();
    profile().bind(&mut o, &id, &c, &mut r);
    o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    o.run_live_relation_v1(&id, &f, &mut r).unwrap();
    let held = o.retention_usage();
    for version in 1..5 {
        let a = InvocationReceiptAdmissionV1 {
            observation: &common::facts(&c),
            capabilities: profile().capabilities,
            operation_receipt_admission_allowed: Some(true),
        };
        let mut old = if version == 1 {
            common::Profile {
                capabilities: Default::default(),
                ..profile()
            }
            .open(&mut o, &id, &c, &mut r)
        } else {
            match version {
                2 => o.open_invocation_observation_with_receipts_v2(
                    &id,
                    &c,
                    &common::policy(&c, 7),
                    limits(),
                    a,
                    &mut r,
                ),
                3 => o.open_invocation_observation_with_receipts_v3(
                    &id,
                    &c,
                    &common::policy(&c, 7),
                    limits(),
                    a,
                    &mut r,
                ),
                _ => o.open_invocation_observation_with_receipts_v4(
                    &id,
                    &c,
                    &common::policy(&c, 7),
                    limits(),
                    a,
                    &mut r,
                ),
            }
            .unwrap()
        };
        assert!(matches!(
            query(&o, &mut old, &c, &release(&c), &mut r),
            Err(InvocationError::Unavailable)
        ));
        o.retire_invocation_observation_v1(&mut old, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage(), held);
    }
    for bad in 0..3 {
        let mut l = limits();
        let mut permission = Some(true);
        if bad == 0 {
            l.max_reply_bytes -= 1;
            l.max_held_bytes = l.required_hold_bytes().unwrap()
        } else if bad == 1 {
            permission = None
        } else {
            permission = Some(false)
        }
        assert!(
            o.open_invocation_observation_with_receipts_v5(
                &id,
                &c,
                &common::policy(&c, 7),
                l,
                InvocationReceiptAdmissionV1 {
                    observation: &common::facts(&c),
                    capabilities: profile().capabilities,
                    operation_receipt_admission_allowed: permission
                },
                &mut r
            )
            .is_err()
        );
        assert_eq!(o.retention_usage(), held);
    }
    let mut no_family = o
        .open_invocation_observation_with_receipts_v5(
            &id,
            &c,
            &common::policy(&c, 7),
            limits(),
            InvocationReceiptAdmissionV1 {
                observation: &common::facts(&c),
                capabilities: InvocationReceiptCapabilitiesV1 {
                    relation: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    assert!(matches!(
        query(&o, &mut no_family, &c, &release(&c), &mut r),
        Err(InvocationError::Unavailable)
    ));
    let mut v = open(&mut o, &id, &c, &mut r);
    assert!(matches!(
        profile().query(
            &o,
            &mut v,
            InvocationReceiptRequestV1::Lifecycle,
            &c,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    ));
    let request = InvocationReceiptRequestV5::Existing(InvocationReceiptRequestV4::Existing(
        InvocationReceiptRequestV3::Existing(InvocationReceiptRequestV2::Existing(
            InvocationReceiptRequestV1::Lifecycle,
        )),
    ));
    assert!(matches!(
        query_request(
            &o,
            &mut v,
            &c,
            request,
            (&common::policy(&c, 7), Some(true)),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    ));
    assert!(query(&o, &mut v, &c, &release(&c), &mut r).is_ok());
    assert_eq!(
        InvocationReceiptV5::required_reply_bytes(usize::MAX),
        Err(InvocationError::Limit)
    );
}

#[test]
fn v5_short_original_native_bound_refuses_before_observer_funding() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let q = request(&mut o, &mut r, 0);
    let id = o.issue_invocation(&mut r).unwrap();
    common::Profile {
        maximum: 1,
        ..profile()
    }
    .bind(&mut o, &id, &c, &mut r);
    o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    let held = o.retention_usage();
    assert_eq!(
        o.open_invocation_observation_with_receipts_v5(
            &id,
            &c,
            &common::policy(&c, 7),
            limits(),
            InvocationReceiptAdmissionV1 {
                observation: &common::facts(&c),
                capabilities: profile().capabilities,
                operation_receipt_admission_allowed: Some(true)
            },
            &mut r
        )
        .err(),
        Some(InvocationError::Limit)
    );
    assert_eq!(o.retention_usage(), held);
}

#[test]
fn v5_known_stop_refusal_and_unchanged_membership_are_not_confused() {
    for case in 0..3 {
        let (mut o, mut r) = owner();
        let c = context();
        let f = facts(&c);
        let mut q = request(&mut o, &mut r, 2);
        if case == 2 {
            let LiveRelationOperationV1::Membership { workspace, present } = &mut q.operation
            else {
                panic!()
            };
            *workspace = o
                .set_workspace_membership(workspace, &q.document, *present, &mut r)
                .unwrap()
                .context;
        }
        let id = o.issue_invocation(&mut r).unwrap();
        profile().bind(&mut o, &id, &c, &mut r);
        o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
            .unwrap();
        let mut v = open(&mut o, &id, &c, &mut r);
        if case == 0 {
            o.stop_invocation(&id, &mut r).unwrap()
        } else {
            if case == 1 {
                let LiveRelationOperationV1::Membership { workspace, present } = &q.operation
                else {
                    panic!()
                };
                o.set_workspace_membership(workspace, &q.document, *present, &mut r)
                    .unwrap();
            }
            let generation = o.generation();
            o.run_live_relation_v1(&id, &f, &mut r).unwrap();
            assert_eq!(o.generation(), generation);
        }
        let out = query(&o, &mut v, &c, &release(&c), &mut r);
        if case == 2 {
            let InvocationReceiptV5::LiveRelation(out) = out.unwrap() else {
                panic!()
            };
            assert!(!out.changed)
        } else {
            assert!(matches!(out, Err(InvocationError::OutcomeUnavailable)))
        }
    }
}

#[test]
fn v5_delegates_actual_atomic_and_retained_release_results_without_losing_native_guards() {
    use vellum_engine::attachment::AttachmentTarget;
    let mut f = prior::Fixture::with_invocations();
    let c = context();
    f.owner
        .configure_semantic_root_budget(1048576, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_retention_budget(1048576, &mut f.recorder)
        .unwrap();
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    profile().bind(&mut f.owner, &id, &c, &mut f.recorder);
    let intent = CompoundIntent {
        source: SourceIntent {
            writes: vec![],
            reads: vec![],
        },
        annotations: vec![AnnotationMutation::Create {
            target: SourceTarget {
                source: f.doc.clone(),
                target: AttachmentTarget::ContentRange { start: 0, end: 1 },
                basis: TargetBasis::Before,
            },
            body: "PRIVATE_ATOMIC".into(),
        }],
    };
    let gs = [Some(TrustedCompoundAnnotationResultGrantV1 {
        invocation: &id,
        authority: &c.authority,
        target: CompoundAnnotationResultGrantTargetV1::CreatedByComponent {
            document: &f.doc.document,
            component: 0,
        },
        capabilities: AnnotationResultCapabilitiesV1::IdentityContent,
    })];
    f.owner
        .submit_compound_with_annotation_results_v1(
            &id,
            &intent,
            &TrustedCompoundAnnotationResultAdmissionV1 {
                context: &c,
                publication_epoch: 7,
                publication_allowed: Some(true),
                context_current: Some(true),
                result_admission_allowed: Some(true),
                max_result_bytes: 512,
                grants: &gs,
            },
            &mut f.recorder,
        )
        .unwrap();
    let mut v = f
        .owner
        .open_invocation_observation_with_receipts_v5(
            &id,
            &c,
            &common::policy(&c, 7),
            limits(),
            InvocationReceiptAdmissionV1 {
                observation: &common::facts(&c),
                capabilities: InvocationReceiptCapabilitiesV1 {
                    atomic_annotation: true,
                    lifecycle: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    f.owner.run(&id, &mut f.recorder).unwrap();
    let mut n = TrustedCompoundAnnotationResultReleaseV1 {
        context: &c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_context_current: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        max_result_bytes: 512,
        grant: gs[0],
    };
    let request =
        InvocationReceiptRequestV5::Existing(InvocationReceiptRequestV4::AtomicAnnotation {
            index: 0,
            native: &n,
        });
    let InvocationReceiptV5::Existing(InvocationReceiptV4::AtomicAnnotation(out)) = query_request(
        &f.owner,
        &mut v,
        &c,
        request,
        (&common::policy(&c, 7), Some(true)),
        &mut f.recorder,
    )
    .unwrap() else {
        panic!("wrong delegated result")
    };
    let AnnotationResultDecisionV1::Applied { record, .. } = out.decision else {
        panic!()
    };
    assert_eq!(record.content_version, Some(1));
    assert_eq!(record.target_generation, None);
    n.grant = None;
    assert!(matches!(
        query_request(
            &f.owner,
            &mut v,
            &c,
            InvocationReceiptRequestV5::Existing(InvocationReceiptRequestV4::AtomicAnnotation {
                index: 0,
                native: &n
            }),
            (&common::policy(&c, 7), Some(true)),
            &mut f.recorder
        ),
        Err(InvocationError::Unavailable)
    ));
    let mut f = prior::disposed();
    let c = prior::context();
    let fresh = prior::facts(&c);
    let q = prior::request(&mut f, RetainedReleaseOperationV1::SourceBinding);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    profile().bind(&mut f.owner, &id, &c, &mut f.recorder);
    f.owner
        .submit_retained_release_v1(&id, &q, &prior::admission(&fresh), &mut f.recorder)
        .unwrap();
    let mut v = open(&mut f.owner, &id, &c, &mut f.recorder);
    f.owner
        .run_retained_release_v1(&id, &fresh, &mut f.recorder)
        .unwrap();
    let n = prior::release(&c);
    let request = InvocationReceiptRequestV5::Existing(InvocationReceiptRequestV4::Existing(
        InvocationReceiptRequestV3::RetainedRelease(&n),
    ));
    let InvocationReceiptV5::Existing(InvocationReceiptV4::Existing(
        InvocationReceiptV3::RetainedRelease(out),
    )) = query_request(
        &f.owner,
        &mut v,
        &c,
        request,
        (&common::policy(&c, 7), Some(true)),
        &mut f.recorder,
    )
    .unwrap()
    else {
        panic!("wrong delegated retained result")
    };
    assert!(out.changed);
    assert_eq!(out.operation, RetainedReleaseOperationV1::SourceBinding);
}

#[test]
fn v5_actual_receipt_timing_is_optional_bounded_and_content_free() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, mut setup) = owner();
        let c = context();
        let f = facts(&c);
        let q = request(&mut o, &mut setup, 0);
        let id = o.issue_invocation(&mut setup).unwrap();
        profile().bind(&mut o, &id, &c, &mut setup);
        o.submit_live_relation_v1(&id, &q, &admission(&f), &mut setup)
            .unwrap();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
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
        let mut v = open(&mut o, &id, &c, &mut r);
        o.run_live_relation_v1(&id, &f, &mut setup).unwrap();
        assert!(query(&o, &mut v, &c, &release(&c), &mut r).is_ok());
        let mut counts = [0; 3];
        while let Some(e) = r.pop_front() {
            let i = match e.boundary {
                Boundary::InvocationReceiptAdmission => Some(0),
                Boundary::InvocationReceiptRelease => Some(1),
                Boundary::LiveRelationResultRelease => Some(2),
                _ => None,
            };
            if let Some(i) = i {
                counts[i] += 1;
                assert_eq!(e.disposition, Disposition::Completed);
                assert_eq!(e.service_time.is_some(), mode == 2)
            }
            assert!(!format!("{e:?}").contains("PRIVATE"));
        }
        if mode == 1 || mode == 2 {
            assert_eq!(counts, [1, 1, 1])
        } else {
            assert_eq!(counts, [0, 0, 0])
        }
    }
}
#[test]
fn v5_delivers_all_three_actual_live_relation_products_without_execution_or_later_relation_lookup()
{
    for kind in 0..3 {
        let (mut o, mut r) = owner();
        let c = context();
        let f = facts(&c);
        let q = request(&mut o, &mut r, kind);
        let id = o.issue_invocation(&mut r).unwrap();
        profile().bind(&mut o, &id, &c, &mut r);
        o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
            .unwrap();
        let mut v = open(&mut o, &id, &c, &mut r);
        assert!(matches!(
            query(&o, &mut v, &c, &release(&c), &mut r),
            Err(InvocationError::Unavailable)
        ));
        assert_eq!(
            o.invocation_state(&id, &mut r).unwrap(),
            InvocationState::Pending
        );
        o.run_live_relation_v1(&id, &f, &mut r).unwrap();
        let generation = o.generation();
        let InvocationReceiptV5::LiveRelation(out) =
            query(&o, &mut v, &c, &release(&c), &mut r).unwrap()
        else {
            panic!("wrong native family")
        };
        assert!(out.changed);
        assert_eq!(out.invocation, id);
        assert_eq!(out.document, q.document);
        assert_eq!(o.generation(), generation);
        match &q.operation {
            LiveRelationOperationV1::Attach { view, .. } => {
                let s = o.logical_view(view, &mut r).unwrap().stamp;
                o.detach_logical_view(view, s, &mut r).unwrap();
            }
            LiveRelationOperationV1::Detach { view, .. } => {
                let source = o.read(&q.document, &mut r).unwrap().version;
                let s = o.logical_view(view, &mut r).unwrap().stamp;
                o.attach_logical_view(view, s, &source, &[caret(2)], 0, &mut r)
                    .unwrap();
            }
            LiveRelationOperationV1::Membership { workspace, .. } => {
                let w = o.workspace(&workspace.workspace, &mut r).unwrap().context;
                o.set_workspace_membership(&w, &q.document, false, &mut r)
                    .unwrap();
            }
        }
        let InvocationReceiptV5::LiveRelation(historical) =
            query(&o, &mut v, &c, &release(&c), &mut r).unwrap()
        else {
            panic!()
        };
        assert_eq!(historical, out);
        let held = o.retention_usage();
        o.forget_invocation_outcome(&id, &mut r).unwrap();
        assert!(matches!(
            query(&o, &mut v, &c, &release(&c), &mut r),
            Err(InvocationError::Unavailable)
        ));
        assert_eq!(o.retention_usage(), held);
        o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
            .unwrap();
        o.retire_live_relation_v1(&id, &control(&c), &mut r)
            .unwrap();
        o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
            .unwrap();
        o.retire_invocation(&id, &mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
