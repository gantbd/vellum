#[path = "support/invocation_observation_fixture.rs"]
mod common;
#[path = "support/retained_release_fixture.rs"]
mod prior;
#[path = "support/scratch_invocation_fixture.rs"]
mod support;
use std::mem::size_of;
use support::*;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;

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
        max_reply_bytes: InvocationReceiptV6::required_reply_bytes(512).unwrap(),
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
    o.open_invocation_observation_with_receipts_v6(
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
    request: InvocationReceiptRequestV6<'_>,
    gate: (&InvocationObservationPolicyV1, Option<bool>),
    r: &mut Recorder,
) -> Result<InvocationReceiptV6<'a>, InvocationError> {
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
    o.query_invocation_receipt_v6(
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
fn v6_scratch_receipt_delivers_only_the_original_historical_allocation_without_executing() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    profile().bind(&mut o, &id, &c, &mut r);
    o.submit_scratch_v1(&id, &request(b"original"), &admission(&f), &mut r)
        .unwrap();
    let mut v = open(&mut o, &id, &c, &mut r);
    let p = common::policy(&c, 7);
    let n = release(&c);
    let generation = o.generation();
    assert_eq!(
        query(
            &o,
            &mut v,
            &c,
            InvocationReceiptRequestV6::Scratch(&n),
            (&p, Some(true)),
            &mut r
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    assert_eq!(o.generation(), generation);
    o.run_scratch_v1(&id, &f, &mut r).unwrap();
    let original = o.scratch_result_v1(&id, &n, &mut r).unwrap();
    o.dispose(&original.document, &mut r).unwrap();
    let generation = o.generation();
    let held = o.retention_usage();
    for _ in 0..2 {
        match query(
            &o,
            &mut v,
            &c,
            InvocationReceiptRequestV6::Scratch(&n),
            (&p, Some(true)),
            &mut r,
        )
        .unwrap()
        {
            InvocationReceiptV6::Scratch(result) => assert_eq!(result, original),
            _ => panic!("wrong facet"),
        }
    }
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage(), held);
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        query(
            &o,
            &mut v,
            &c,
            InvocationReceiptRequestV6::Scratch(&n),
            (&p, Some(true)),
            &mut r
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    o.retire_scratch_v1(&id, &control(&c), &mut r).unwrap();
}

#[test]
fn v6_six_original_captured_current_disclosure_ceilings_are_independent() {
    for bits in 0..64 {
        let (mut o, mut r) = owner();
        let c = context();
        let f = facts(&c);
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
        o.submit_scratch_v1(&id, &request(b"private"), &a, &mut r)
            .unwrap();
        let mut captured = common::policy(&c, 7);
        captured.publication = bits & 2 != 0;
        let mut v = o
            .open_invocation_observation_with_receipts_v6(
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
        o.run_scratch_v1(&id, &f, &mut r).unwrap();
        let mut current = common::policy(&c, 7);
        current.publication = bits & 4 != 0;
        let generation = o.generation();
        let held = o.retention_usage();
        assert_eq!(
            query(
                &o,
                &mut v,
                &c,
                InvocationReceiptRequestV6::Scratch(&release(&c)),
                (&current, Some(true)),
                &mut r
            )
            .is_ok(),
            bits == 63
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
    }
}
#[test]
fn v6_native_denials_and_common_receipt_permission_never_poison_later_valid_delivery() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let mut wrong = c.clone();
    wrong.recipient = "other".into();
    let id = o.issue_invocation(&mut r).unwrap();
    profile().bind(&mut o, &id, &c, &mut r);
    o.submit_scratch_v1(&id, &request(b"original"), &admission(&f), &mut r)
        .unwrap();
    let mut v = open(&mut o, &id, &c, &mut r);
    o.run_scratch_v1(&id, &f, &mut r).unwrap();
    let p = common::policy(&c, 7);
    let generation = o.generation();
    let held = o.retention_usage();
    for bad in 0..19 {
        let mut n = release(&c);
        let mut gate = Some(true);
        let flag = if bad % 2 == 0 { None } else { Some(false) };
        match bad {
            0..=1 => n.publication_allowed = flag,
            2..=3 => n.outcome_readable = flag,
            4..=5 => n.result_readable = flag,
            6..=7 => n.result_context_current = flag,
            8..=9 => n.historical_document_readable = flag,
            10..=11 => n.reply_ready = flag,
            12..=13 => n.resources_ready = flag,
            14 => n.context = &wrong,
            15 => n.publication_epoch += 1,
            16 => n.max_result_bytes = 0,
            17 => gate = None,
            _ => gate = Some(false),
        }
        assert!(
            query(
                &o,
                &mut v,
                &c,
                InvocationReceiptRequestV6::Scratch(&n),
                (&p, gate),
                &mut r
            )
            .is_err()
        );
        assert!(
            query(
                &o,
                &mut v,
                &c,
                InvocationReceiptRequestV6::Scratch(&release(&c)),
                (&p, Some(true)),
                &mut r
            )
            .is_ok()
        );
    }
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage(), held);
}
#[test]
fn v6_requires_original_version_family_and_complete_parent_funded_wrapper() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    profile().bind(&mut o, &id, &c, &mut r);
    o.submit_scratch_v1(&id, &request(b"original"), &admission(&f), &mut r)
        .unwrap();
    o.run_scratch_v1(&id, &f, &mut r).unwrap();
    let p = common::policy(&c, 7);
    for bad in 0..4 {
        let mut l = limits();
        let mut permission = Some(true);
        match bad {
            0 => {
                l.max_reply_bytes -= 1;
                l.max_held_bytes = l.required_hold_bytes().unwrap();
            }
            1 => l.max_held_bytes -= 1,
            2 => permission = None,
            _ => permission = Some(false),
        }
        let held = o.retention_usage();
        assert!(
            o.open_invocation_observation_with_receipts_v6(
                &id,
                &c,
                &p,
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
    for version in 1..=6 {
        let a = InvocationReceiptAdmissionV1 {
            observation: &common::facts(&c),
            capabilities: InvocationReceiptCapabilitiesV1::default(),
            operation_receipt_admission_allowed: Some(true),
        };
        let mut v = match version {
            1 => Ok(common::Profile {
                capabilities: InvocationReceiptCapabilitiesV1::default(),
                ..profile()
            }
            .open(&mut o, &id, &c, &mut r)),
            2 => o.open_invocation_observation_with_receipts_v2(&id, &c, &p, limits(), a, &mut r),
            3 => o.open_invocation_observation_with_receipts_v3(&id, &c, &p, limits(), a, &mut r),
            4 => o.open_invocation_observation_with_receipts_v4(&id, &c, &p, limits(), a, &mut r),
            5 => o.open_invocation_observation_with_receipts_v5(&id, &c, &p, limits(), a, &mut r),
            _ => o.open_invocation_observation_with_receipts_v6(&id, &c, &p, limits(), a, &mut r),
        }
        .unwrap();
        assert_eq!(
            query(
                &o,
                &mut v,
                &c,
                InvocationReceiptRequestV6::Scratch(&release(&c)),
                (&p, Some(true)),
                &mut r
            )
            .err(),
            Some(InvocationError::Unavailable)
        );
    }
    let mut v = open(&mut o, &id, &c, &mut r);
    assert_eq!(
        profile()
            .query(
                &o,
                &mut v,
                InvocationReceiptRequestV1::Lifecycle,
                &c,
                &mut r
            )
            .err(),
        Some(InvocationError::Unavailable)
    );
    let legacy = InvocationReceiptRequestV6::Existing(InvocationReceiptRequestV5::Existing(
        InvocationReceiptRequestV4::Existing(InvocationReceiptRequestV3::Existing(
            InvocationReceiptRequestV2::Existing(InvocationReceiptRequestV1::Lifecycle),
        )),
    ));
    assert_eq!(
        query(&o, &mut v, &c, legacy, (&p, Some(true)), &mut r).err(),
        Some(InvocationError::Unavailable)
    );
    assert!(
        query(
            &o,
            &mut v,
            &c,
            InvocationReceiptRequestV6::Scratch(&release(&c)),
            (&p, Some(true)),
            &mut r
        )
        .is_ok()
    );
    assert_eq!(
        InvocationReceiptV6::required_reply_bytes(usize::MAX),
        Err(InvocationError::Limit)
    );
}

#[test]
fn v6_short_original_result_bound_refuses_before_observer_funding() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    common::Profile {
        maximum: 1,
        ..profile()
    }
    .bind(&mut o, &id, &c, &mut r);
    o.submit_scratch_v1(&id, &request(b"text"), &admission(&f), &mut r)
        .unwrap();
    let held = o.retention_usage();
    assert_eq!(
        o.open_invocation_observation_with_receipts_v6(
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
fn v6_stop_and_late_refusal_disclose_no_created_identity_and_preserve_protected_status() {
    for stopped in [true, false] {
        let (mut o, mut r) = owner();
        let c = context();
        let f = facts(&c);
        let id = o.issue_invocation(&mut r).unwrap();
        profile().bind(&mut o, &id, &c, &mut r);
        o.submit_scratch_v1(&id, &request(b"text"), &admission(&f), &mut r)
            .unwrap();
        let mut v = open(&mut o, &id, &c, &mut r);
        if stopped {
            o.stop_invocation(&id, &mut r).unwrap();
        } else {
            for _ in 0..8 {
                o.create(b"peer", &mut r).unwrap();
            }
            o.run_scratch_v1(&id, &f, &mut r).unwrap();
        }
        let p = common::policy(&c, 7);
        assert_eq!(
            query(
                &o,
                &mut v,
                &c,
                InvocationReceiptRequestV6::Scratch(&release(&c)),
                (&p, Some(true)),
                &mut r
            )
            .err(),
            Some(InvocationError::OutcomeUnavailable)
        );
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
        assert_eq!(
            o.minimal_invocation_control_status_v1(&mut v, &p, &current, &mut r)
                .unwrap()
                .phase,
            InvocationObservationPhaseV1::Terminal
        );
        let ctl = control(&c);
        o.retire_invocation_observation_v1(&mut v, &ctl, &mut r)
            .unwrap();
        o.retire_scratch_v1(&id, &ctl, &mut r).unwrap();
        o.retire_invocation_observation_policy_v1(&id, &ctl, &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}

#[test]
fn v6_delegates_actual_live_relation_and_retained_release_without_reconstructing_results() {
    let (mut o, mut r) = owner();
    let c = context();
    o.configure_workspaces(
        WorkspaceLimits {
            max_issued: 4,
            max_live: 4,
            max_memberships: 8,
            max_work: 4096,
        },
        &mut r,
    )
    .unwrap();
    let doc = o.create(b"text", &mut r).unwrap();
    let w = o.create_workspace(&mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    profile().bind(&mut o, &id, &c, &mut r);
    let f = TrustedLiveRelationFactsV1 {
        context: &c,
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
        view_writable: None,
        source_readable: None,
        membership_writable: Some(true),
        workspace_readable: Some(true),
    };
    let q = LiveRelationRequestV1 {
        document: doc.document.clone(),
        expected_lifecycle: o.lifecycle_state(&doc.document, &mut r).unwrap(),
        operation: LiveRelationOperationV1::Membership {
            workspace: w.clone(),
            present: true,
        },
    };
    o.submit_live_relation_v1(
        &id,
        &q,
        &LiveRelationAdmissionV1 {
            facts: &f,
            limits: LiveRelationLimitsV1 {
                max_capture_bytes: 8192,
                max_result_bytes: 512,
            },
            publication_epoch: 7,
            publication_allowed: Some(true),
            result_admission_allowed: Some(true),
            result_identifiers_allowed: Some(true),
        },
        &mut r,
    )
    .unwrap();
    let mut v = open(&mut o, &id, &c, &mut r);
    o.run_live_relation_v1(&id, &f, &mut r).unwrap();
    let n = TrustedLiveRelationResultV1 {
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
    let expected = o.live_relation_result_v1(&id, &n, &mut r).unwrap();
    let now = o.workspace(&w.workspace, &mut r).unwrap().context;
    o.set_workspace_membership(&now, &doc.document, false, &mut r)
        .unwrap();
    let generation = o.generation();
    match query(
        &o,
        &mut v,
        &c,
        InvocationReceiptRequestV6::Existing(InvocationReceiptRequestV5::LiveRelation(&n)),
        (&common::policy(&c, 7), Some(true)),
        &mut r,
    )
    .unwrap()
    {
        InvocationReceiptV6::Existing(InvocationReceiptV5::LiveRelation(v)) => {
            assert_eq!(v, expected)
        }
        _ => panic!("wrong delegated result"),
    }
    assert_eq!(o.generation(), generation);
    let mut f = prior::disposed();
    let c = prior::context();
    let n = prior::facts(&c);
    let q = prior::request(&mut f, RetainedReleaseOperationV1::SourceBinding);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    profile().bind(&mut f.owner, &id, &c, &mut f.recorder);
    f.owner
        .submit_retained_release_v1(&id, &q, &prior::admission(&n), &mut f.recorder)
        .unwrap();
    let mut v = open(&mut f.owner, &id, &c, &mut f.recorder);
    f.owner
        .run_retained_release_v1(&id, &n, &mut f.recorder)
        .unwrap();
    let n = prior::release(&c);
    let expected = f
        .owner
        .retained_release_result_v1(&id, &n, &mut f.recorder)
        .unwrap();
    let req = InvocationReceiptRequestV6::Existing(InvocationReceiptRequestV5::Existing(
        InvocationReceiptRequestV4::Existing(InvocationReceiptRequestV3::RetainedRelease(&n)),
    ));
    match query(
        &f.owner,
        &mut v,
        &c,
        req,
        (&common::policy(&c, 7), Some(true)),
        &mut f.recorder,
    )
    .unwrap()
    {
        InvocationReceiptV6::Existing(InvocationReceiptV5::Existing(
            InvocationReceiptV4::Existing(InvocationReceiptV3::RetainedRelease(v)),
        )) => assert_eq!(v, expected),
        _ => panic!("wrong delegated result"),
    }
}

#[test]
fn v6_actual_receipt_capture_is_optional_bounded_redacted_and_not_double_counted() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, mut setup) = owner();
        let c = context();
        let f = facts(&c);
        let id = o.issue_invocation(&mut setup).unwrap();
        profile().bind(&mut o, &id, &c, &mut setup);
        o.submit_scratch_v1(&id, &request(b"PRIVATE"), &admission(&f), &mut setup)
            .unwrap();
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
        let mut v = open(&mut o, &id, &c, &mut r);
        o.run_scratch_v1(&id, &f, &mut setup).unwrap();
        assert!(
            query(
                &o,
                &mut v,
                &c,
                InvocationReceiptRequestV6::Scratch(&release(&c)),
                (&common::policy(&c, 7), Some(true)),
                &mut r
            )
            .is_ok()
        );
        let mut counts = [0; 3];
        while let Some(e) = r.pop_front() {
            let i = match e.boundary {
                Boundary::InvocationReceiptAdmission => Some(0),
                Boundary::InvocationReceiptRelease => Some(1),
                Boundary::ScratchResultRelease => Some(2),
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
