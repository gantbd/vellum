use super::compound_results::{admission, control, grant, record, release, single};
use super::observation_support::{Profile, facts, policy};
use super::scoped::{context, funded};
use super::*;
use AnnotationResultCapabilitiesV1 as C;
use std::mem::size_of;
#[path = "../support/retained_release_fixture.rs"]
mod release_producer;
fn profile() -> Profile {
    Profile {
        epoch: 7,
        maximum: 512,
        capabilities: InvocationReceiptCapabilitiesV1 {
            atomic_annotation: true,
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
        max_reply_bytes: InvocationReceiptV4::required_reply_bytes(512).unwrap(),
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
    o.open_invocation_observation_with_receipts_v4(
        id,
        c,
        &policy(c, 7),
        limits(),
        InvocationReceiptAdmissionV1 {
            observation: &facts(c),
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
    index: usize,
    c: &SettingsExecutionContextV1,
    native: &TrustedCompoundAnnotationResultReleaseV1<'_>,
    r: &mut Recorder,
) -> Result<InvocationReceiptV4<'a>, InvocationError> {
    query_with(o, v, index, c, native, (&policy(c, 7), Some(true)), r)
}
fn query_with<'a>(
    o: &'a SourceCoordinator,
    v: &mut InvocationObservationContextV1,
    index: usize,
    c: &SettingsExecutionContextV1,
    native: &TrustedCompoundAnnotationResultReleaseV1<'_>,
    gate: (&InvocationObservationPolicyV1, Option<bool>),
    r: &mut Recorder,
) -> Result<InvocationReceiptV4<'a>, InvocationError> {
    query_request(
        o,
        v,
        c,
        InvocationReceiptRequestV4::AtomicAnnotation { index, native },
        gate,
        r,
    )
}
fn query_request<'a>(
    o: &'a SourceCoordinator,
    v: &mut InvocationObservationContextV1,
    c: &SettingsExecutionContextV1,
    request: InvocationReceiptRequestV4<'_>,
    gate: (&InvocationObservationPolicyV1, Option<bool>),
    r: &mut Recorder,
) -> Result<InvocationReceiptV4<'a>, InvocationError> {
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
    o.query_invocation_receipt_v4(
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
fn v4_atomic_release_meets_all_common_publication_and_original_native_ceilings() {
    for bits in 0..32 {
        let (mut o, mut r) = funded(8388608);
        let c = context();
        let d = o.create(b"abc", &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let mut original = policy(&c, 7);
        original.publication = bits & 1 != 0;
        o.bind_invocation_observation_policy_v1(
            &id,
            &c,
            &original,
            InvocationObservationPolicyLimitsV1 {
                max_context_bytes: 2048,
                max_result_bytes: 512,
            },
            &facts(&c),
            &mut r,
        )
        .unwrap();
        let gs = [Some(grant(
            &id,
            &c,
            &d.document,
            0,
            if bits & 16 != 0 {
                C::IdentityContentTarget
            } else {
                C::None
            },
        ))];
        let mut native_admit = admission(&c, &gs);
        native_admit.publication_allowed = Some(bits & 8 != 0);
        o.submit_compound_with_annotation_results_v1(&id, &single(&d), &native_admit, &mut r)
            .unwrap();
        let mut captured = policy(&c, 7);
        captured.publication = bits & 2 != 0;
        let mut observer = o
            .open_invocation_observation_with_receipts_v4(
                &id,
                &c,
                &captured,
                limits(),
                InvocationReceiptAdmissionV1 {
                    observation: &facts(&c),
                    capabilities: profile().capabilities,
                    operation_receipt_admission_allowed: Some(true),
                },
                &mut r,
            )
            .unwrap();
        o.run(&id, &mut r).unwrap();
        let generation = o.generation();
        let held = o.retention_usage();
        let mut current = policy(&c, 7);
        current.publication = bits & 4 != 0;
        assert_eq!(
            query_with(
                &o,
                &mut observer,
                0,
                &c,
                &release(&c, gs[0]),
                (&current, Some(true)),
                &mut r
            )
            .is_ok(),
            bits == 31
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
    }
}

#[test]
fn v4_current_native_denials_and_common_receipt_permission_never_rewrite_or_poison_the_result() {
    let (mut o, mut r) = funded(8388608);
    let c = context();
    let mut wrong = c.clone();
    wrong.recipient = "other".into();
    let d = o.create(b"abc", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    profile().bind(&mut o, &id, &c, &mut r);
    let gs = [Some(grant(
        &id,
        &c,
        &d.document,
        0,
        C::IdentityContentTarget,
    ))];
    o.submit_compound_with_annotation_results_v1(&id, &single(&d), &admission(&c, &gs), &mut r)
        .unwrap();
    let mut observer = open(&mut o, &id, &c, &mut r);
    o.run(&id, &mut r).unwrap();
    let generation = o.generation();
    let held = o.retention_usage();
    for bad in 0..18 {
        let mut n = release(&c, gs[0]);
        let mut permission = Some(true);
        match bad {
            0 => n.publication_allowed = None,
            1 => n.publication_allowed = Some(false),
            2 => n.outcome_readable = None,
            3 => n.outcome_readable = Some(false),
            4 => n.result_context_current = None,
            5 => n.result_context_current = Some(false),
            6 => n.reply_ready = None,
            7 => n.reply_ready = Some(false),
            8 => n.resources_ready = None,
            9 => n.resources_ready = Some(false),
            10 => n.context = &wrong,
            11 => n.publication_epoch = 8,
            12 => n.max_result_bytes = AnnotationResultV1::required_bytes() - 1,
            13 => n.grant = None,
            14 => n.grant.as_mut().unwrap().authority = "other",
            15 => permission = None,
            16 => permission = Some(false),
            _ => {
                n.grant.as_mut().unwrap().target =
                    CompoundAnnotationResultGrantTargetV1::CreatedByComponent {
                        document: &d.document,
                        component: 1,
                    }
            }
        }
        assert!(
            query_with(
                &o,
                &mut observer,
                0,
                &c,
                &n,
                (&policy(&c, 7), permission),
                &mut r
            )
            .is_err()
        );
        assert!(query(&o, &mut observer, 0, &c, &release(&c, gs[0]), &mut r).is_ok());
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
    }
}

#[test]
fn v4_requires_explicit_profile_family_and_complete_original_reply_bounds() {
    let (mut o, mut r) = funded(8388608);
    let c = context();
    let d = o.create(b"abc", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    profile().bind(&mut o, &id, &c, &mut r);
    let gs = [Some(grant(
        &id,
        &c,
        &d.document,
        0,
        C::IdentityContentTarget,
    ))];
    o.submit_compound_with_annotation_results_v1(&id, &single(&d), &admission(&c, &gs), &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    let base = o.retention_usage();
    for version in 1..4 {
        let a = InvocationReceiptAdmissionV1 {
            observation: &facts(&c),
            capabilities: profile().capabilities,
            operation_receipt_admission_allowed: Some(true),
        };
        let result = match version {
            1 => o.open_invocation_observation_with_receipts_v1(
                &id,
                &c,
                &policy(&c, 7),
                limits(),
                a,
                &mut r,
            ),
            2 => o.open_invocation_observation_with_receipts_v2(
                &id,
                &c,
                &policy(&c, 7),
                limits(),
                a,
                &mut r,
            ),
            _ => o.open_invocation_observation_with_receipts_v3(
                &id,
                &c,
                &policy(&c, 7),
                limits(),
                a,
                &mut r,
            ),
        };
        assert_eq!(result.err(), Some(InvocationError::Unavailable));
        assert_eq!(o.retention_usage(), base);
        let a = InvocationReceiptAdmissionV1 {
            observation: &facts(&c),
            capabilities: Default::default(),
            operation_receipt_admission_allowed: Some(true),
        };
        let mut old = match version {
            1 => o.open_invocation_observation_with_receipts_v1(
                &id,
                &c,
                &policy(&c, 7),
                limits(),
                a,
                &mut r,
            ),
            2 => o.open_invocation_observation_with_receipts_v2(
                &id,
                &c,
                &policy(&c, 7),
                limits(),
                a,
                &mut r,
            ),
            _ => o.open_invocation_observation_with_receipts_v3(
                &id,
                &c,
                &policy(&c, 7),
                limits(),
                a,
                &mut r,
            ),
        }
        .unwrap();
        assert!(matches!(
            query(&o, &mut old, 0, &c, &release(&c, gs[0]), &mut r),
            Err(InvocationError::Unavailable)
        ));
        o.retire_invocation_observation_v1(&mut old, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage(), base);
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
            o.open_invocation_observation_with_receipts_v4(
                &id,
                &c,
                &policy(&c, 7),
                l,
                InvocationReceiptAdmissionV1 {
                    observation: &facts(&c),
                    capabilities: profile().capabilities,
                    operation_receipt_admission_allowed: permission
                },
                &mut r
            )
            .is_err()
        );
        assert_eq!(o.retention_usage(), base);
    }
    let mut no_family = o
        .open_invocation_observation_with_receipts_v4(
            &id,
            &c,
            &policy(&c, 7),
            limits(),
            InvocationReceiptAdmissionV1 {
                observation: &facts(&c),
                capabilities: InvocationReceiptCapabilitiesV1 {
                    annotation: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    assert!(matches!(
        query(&o, &mut no_family, 0, &c, &release(&c, gs[0]), &mut r),
        Err(InvocationError::Unavailable)
    ));
    assert_eq!(
        InvocationReceiptV4::required_reply_bytes(usize::MAX),
        Err(InvocationError::Limit)
    );
    let mut v4 = open(&mut o, &id, &c, &mut r);
    // The old grant-less selector is not a shortcut to the new product.
    let request = InvocationReceiptRequestV4::Existing(InvocationReceiptRequestV3::Existing(
        InvocationReceiptRequestV2::Existing(InvocationReceiptRequestV1::AtomicAnnotation {
            index: 0,
        }),
    ));
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
    let f = TrustedInvocationReceiptReleaseV1 {
        observation: &ordinary,
        operation_receipt_allowed: Some(true),
    };
    assert!(matches!(
        o.query_invocation_receipt_v4(&mut v4, request, &policy(&c, 7), &f, &mut r),
        Err(InvocationError::Unavailable)
    ));
    assert!(matches!(
        o.query_invocation_receipt_v3(
            &mut v4,
            InvocationReceiptRequestV3::Existing(InvocationReceiptRequestV2::Existing(
                InvocationReceiptRequestV1::AtomicAnnotation { index: 0 }
            )),
            &policy(&c, 7),
            &f,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    ));
    assert!(query(&o, &mut v4, 0, &c, &release(&c, gs[0]), &mut r).is_ok());
}

#[test]
fn v4_short_original_native_frame_refuses_before_observer_funding() {
    let (mut o, mut r) = funded(8388608);
    let c = context();
    let d = o.create(b"abc", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    Profile {
        maximum: AnnotationResultV1::required_bytes() - 1,
        ..profile()
    }
    .bind(&mut o, &id, &c, &mut r);
    o.submit_compound_with_annotation_results_v1(&id, &single(&d), &admission(&c, &[None]), &mut r)
        .unwrap();
    let held = o.retention_usage();
    assert_eq!(
        o.open_invocation_observation_with_receipts_v4(
            &id,
            &c,
            &policy(&c, 7),
            limits(),
            InvocationReceiptAdmissionV1 {
                observation: &facts(&c),
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
fn v4_delegates_actual_v3_release_and_v1_relation_products_without_profile_fallback() {
    let mut f = release_producer::disposed();
    let c = release_producer::context();
    let fresh = release_producer::facts(&c);
    for operation in [
        RetainedReleaseOperationV1::SourceBinding,
        RetainedReleaseOperationV1::SourceBinding,
        RetainedReleaseOperationV1::AnnotationBinding,
        RetainedReleaseOperationV1::AnnotationBinding,
        RetainedReleaseOperationV1::Provenance,
        RetainedReleaseOperationV1::Provenance,
    ] {
        let q = release_producer::request(&mut f, operation);
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        profile().bind(&mut f.owner, &id, &c, &mut f.recorder);
        f.owner
            .submit_retained_release_v1(
                &id,
                &q,
                &release_producer::admission(&fresh),
                &mut f.recorder,
            )
            .unwrap();
        let mut v = f
            .owner
            .open_invocation_observation_with_receipts_v4(
                &id,
                &c,
                &policy(&c, 7),
                limits(),
                InvocationReceiptAdmissionV1 {
                    observation: &facts(&c),
                    capabilities: InvocationReceiptCapabilitiesV1 {
                        lifecycle: true,
                        atomic_annotation: true,
                        ..Default::default()
                    },
                    operation_receipt_admission_allowed: Some(true),
                },
                &mut f.recorder,
            )
            .unwrap();
        f.owner
            .run_retained_release_v1(&id, &fresh, &mut f.recorder)
            .unwrap();
        let generation = f.owner.generation();
        let n = release_producer::release(&c);
        let request =
            InvocationReceiptRequestV4::Existing(InvocationReceiptRequestV3::RetainedRelease(&n));
        let InvocationReceiptV4::Existing(InvocationReceiptV3::RetainedRelease(out)) =
            query_request(
                &f.owner,
                &mut v,
                &c,
                request,
                (&policy(&c, 7), Some(true)),
                &mut f.recorder,
            )
            .unwrap()
        else {
            panic!("wrong delegated family")
        };
        assert_eq!(out.operation, operation);
        assert_eq!(out.invocation, id);
        assert_eq!(f.owner.generation(), generation);
    }
    let (mut o, mut r) = funded(8388608);
    let c = context();
    o.configure_workspaces(
        WorkspaceLimits {
            max_issued: 4,
            max_live: 4,
            max_memberships: 16,
            max_work: 4096,
        },
        &mut r,
    )
    .unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    profile().bind(&mut o, &id, &c, &mut r);
    o.submit_relation_with_result_v1(
        &id,
        &RelationCommandV1::CreateWorkspace,
        &TrustedRelationResultAdmissionV1 {
            context: &c,
            max_result_bytes: 512,
            result_admission_allowed: Some(true),
            context_current: Some(true),
            relation_result_identifiers_allowed: Some(true),
        },
        &mut r,
    )
    .unwrap();
    o.run(&id, &mut r).unwrap();
    let mut v = o
        .open_invocation_observation_with_receipts_v4(
            &id,
            &c,
            &policy(&c, 7),
            limits(),
            InvocationReceiptAdmissionV1 {
                observation: &facts(&c),
                capabilities: InvocationReceiptCapabilitiesV1 {
                    relation: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    let n = TrustedRelationResultReleaseV1 {
        context: &c,
        max_result_bytes: 512,
        outcome_readable: Some(true),
        relation_result_readable: Some(true),
        relation_result_context_current: Some(true),
        historical_target_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    };
    let request = InvocationReceiptRequestV4::Existing(InvocationReceiptRequestV3::Existing(
        InvocationReceiptRequestV2::Existing(InvocationReceiptRequestV1::Relation(&n)),
    ));
    let InvocationReceiptV4::Existing(InvocationReceiptV3::Existing(
        InvocationReceiptV2::Existing(InvocationReceiptV1::Relation(out)),
    )) = query_request(
        &o,
        &mut v,
        &c,
        request,
        (&policy(&c, 7), Some(true)),
        &mut r,
    )
    .unwrap()
    else {
        panic!("wrong delegated product")
    };
    assert_eq!(out.invocation, id);
}

#[test]
fn v4_receipt_timing_is_bounded_optional_redacted_and_one_outer_episode_per_call() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, mut setup) = funded(8388608);
        let c = context();
        let d = o.create(b"abc", &mut setup).unwrap();
        let id = o.issue_invocation(&mut setup).unwrap();
        profile().bind(&mut o, &id, &c, &mut setup);
        let gs = [Some(grant(
            &id,
            &c,
            &d.document,
            0,
            C::IdentityContentTarget,
        ))];
        o.submit_compound_with_annotation_results_v1(
            &id,
            &single(&d),
            &admission(&c, &gs),
            &mut setup,
        )
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
        o.run(&id, &mut setup).unwrap();
        assert!(query(&o, &mut v, 0, &c, &release(&c, gs[0]), &mut r).is_ok());
        let mut counts = [0; 3];
        while let Some(e) = r.pop_front() {
            assert!(!format!("{e:?}").contains("PRIVATE"));
            let i = match e.boundary {
                Boundary::InvocationReceiptAdmission => Some(0),
                Boundary::InvocationReceiptRelease => Some(1),
                Boundary::AnnotationResultRelease => Some(2),
                _ => None,
            };
            if let Some(i) = i {
                counts[i] += 1;
                assert_eq!(e.disposition, Disposition::Completed);
                assert_eq!(e.service_time.is_some(), mode == 2)
            }
        }
        if mode == 1 || mode == 2 {
            assert_eq!(counts, [1, 1, 1])
        } else {
            assert_eq!(counts, [0, 0, 0])
        }
    }
}

#[test]
fn v4_known_stop_or_refusal_releases_only_not_published_component_facts() {
    for stopped in [false, true] {
        let (mut o, mut r) = funded(8388608);
        let c = context();
        let d = o.create(b"abc", &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        profile().bind(&mut o, &id, &c, &mut r);
        o.submit_compound_with_annotation_results_v1(
            &id,
            &single(&d),
            &admission(&c, &[None]),
            &mut r,
        )
        .unwrap();
        let mut v = open(&mut o, &id, &c, &mut r);
        if stopped {
            o.stop_invocation(&id, &mut r).unwrap()
        } else {
            o.set_annotations_read_only(&d.document, true, &mut r)
                .unwrap();
            o.run(&id, &mut r).unwrap();
        }
        let generation = o.generation();
        let InvocationReceiptV4::AtomicAnnotation(out) =
            query(&o, &mut v, 0, &c, &release(&c, None), &mut r).unwrap()
        else {
            panic!("wrong family")
        };
        assert_eq!(out.decision, AnnotationResultDecisionV1::NotPublished);
        assert_eq!(o.generation(), generation);
        assert_eq!(o.annotation_usage().present_records, 0);
    }
}
#[test]
fn v4_atomic_receipts_use_the_actual_component_producer_and_do_not_execute_or_authorize_siblings() {
    let (mut o, mut r) = funded(8388608);
    let c = context();
    let source = o.create(b"abc", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    profile().bind(&mut o, &id, &c, &mut r);
    let mut intent = single(&source);
    intent.annotations.push(AnnotationMutation::Create {
        target: target(&source, 1, 2),
        body: "PRIVATE_SIBLING".into(),
    });
    let gs = [
        Some(grant(
            &id,
            &c,
            &source.document,
            0,
            C::IdentityContentTarget,
        )),
        Some(grant(&id, &c, &source.document, 1, C::Identity)),
    ];
    o.submit_compound_with_annotation_results_v1(&id, &intent, &admission(&c, &gs), &mut r)
        .unwrap();
    let mut observer = open(&mut o, &id, &c, &mut r);
    assert!(matches!(
        query(&o, &mut observer, 0, &c, &release(&c, gs[0]), &mut r),
        Err(InvocationError::Unavailable)
    ));
    assert_eq!(
        o.invocation_state(&id, &mut r).unwrap(),
        InvocationState::Pending
    );
    o.run(&id, &mut r).unwrap();
    let generation = o.generation();
    let InvocationReceiptV4::AtomicAnnotation(a) =
        query(&o, &mut observer, 0, &c, &release(&c, gs[0]), &mut r).unwrap()
    else {
        panic!("wrong family")
    };
    assert_eq!(record(&a).content_version, Some(1));
    assert!(matches!(
        query(&o, &mut observer, 1, &c, &release(&c, gs[0]), &mut r),
        Err(InvocationError::Unavailable)
    ));
    let InvocationReceiptV4::AtomicAnnotation(b) = query(
        &o,
        &mut observer,
        1,
        &c,
        &release(
            &c,
            Some(grant(
                &id,
                &c,
                &source.document,
                1,
                C::IdentityContentTarget,
            )),
        ),
        &mut r,
    )
    .unwrap() else {
        panic!("wrong family")
    };
    assert_ne!(record(&a).annotation, record(&b).annotation);
    assert_eq!(record(&b).content_version, None);
    assert_eq!(o.generation(), generation);
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: record(&a).annotation.clone(),
                expected_content: 1,
                body: "later".into(),
            }],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    let InvocationReceiptV4::AtomicAnnotation(historical) =
        query(&o, &mut observer, 0, &c, &release(&c, gs[0]), &mut r).unwrap()
    else {
        panic!()
    };
    assert_eq!(historical, a);
    let held = o.retention_usage().reserved;
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
    assert!(matches!(
        query(&o, &mut observer, 0, &c, &release(&c, gs[0]), &mut r),
        Err(InvocationError::Unavailable)
    ));
    assert_eq!(o.retention_usage().reserved, held);
    o.retire_invocation_observation_v1(&mut observer, &control(&c), &mut r)
        .unwrap();
    o.retire_compound_annotation_results_v1(&id, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
