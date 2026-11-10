use super::*;
#[test]
fn expanded_receipt_profile_preserves_existing_native_results_and_is_not_a_v1_context() {
    let (mut o, id, c, p, mut r) = relation_pending(true);
    let mut expanded = limits();
    expanded.max_reply_bytes = InvocationReceiptV2::required_reply_bytes(512).unwrap();
    expanded.max_held_bytes = expanded.required_hold_bytes().unwrap();
    let mut observer = o
        .open_invocation_observation_with_receipts_v2(
            &id,
            &c,
            &p,
            expanded,
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
    o.run(&id, &mut r).unwrap();
    let generation = o.generation();
    let current = release(&c);
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
        o.query_invocation_receipt_v1(
            &mut observer,
            InvocationReceiptRequestV1::Relation(&native(&c)),
            &p,
            &f,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    ));
    let InvocationReceiptV2::Existing(InvocationReceiptV1::Relation(result)) = o
        .query_invocation_receipt_v2(
            &mut observer,
            InvocationReceiptRequestV2::Existing(InvocationReceiptRequestV1::Relation(&native(&c))),
            &p,
            &f,
            &mut r,
        )
        .unwrap()
    else {
        panic!("wrong result family")
    };
    assert_eq!(result.invocation, id);
    assert_eq!(o.generation(), generation);
    let mut legacy = o
        .open_invocation_observation_with_receipts_v1(
            &id,
            &c,
            &p,
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
    assert!(matches!(
        o.query_invocation_receipt_v2(
            &mut legacy,
            InvocationReceiptRequestV2::Existing(InvocationReceiptRequestV1::Relation(&native(&c))),
            &p,
            &f,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    ));
    assert!(matches!(
        o.query_invocation_receipt_v1(
            &mut legacy,
            InvocationReceiptRequestV1::Relation(&native(&c)),
            &p,
            &f,
            &mut r
        ),
        Ok(InvocationReceiptV1::Relation(_))
    ));
}
fn relation_pending(
    publication: bool,
) -> (
    SourceCoordinator,
    InvocationId,
    SettingsExecutionContextV1,
    InvocationObservationPolicyV1,
    Recorder,
) {
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.configure_retention_budget(32768, &mut r).unwrap();
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
    let c = context();
    let mut p = policy(&c);
    p.publication = publication;
    o.bind_invocation_observation_policy_v1(
        &id,
        &c,
        &p,
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &facts(&c),
        &mut r,
    )
    .unwrap();
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
    (o, id, c, p, r)
}
fn native(c: &SettingsExecutionContextV1) -> TrustedRelationResultReleaseV1<'_> {
    TrustedRelationResultReleaseV1 {
        context: c,
        max_result_bytes: 512,
        outcome_readable: Some(true),
        relation_result_readable: Some(true),
        relation_result_context_current: Some(true),
        historical_target_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    }
}
#[test]
fn relation_receipt_observation_preserves_native_permissions_and_does_not_execute_or_issue_progress()
 {
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.configure_retention_budget(32768, &mut r).unwrap();
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
    let c = context();
    let p = policy(&c);
    o.bind_invocation_observation_policy_v1(
        &id,
        &c,
        &p,
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &facts(&c),
        &mut r,
    )
    .unwrap();
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
    let af = facts(&c);
    let admission = InvocationReceiptAdmissionV1 {
        observation: &af,
        capabilities: InvocationReceiptCapabilitiesV1 {
            relation: true,
            ..Default::default()
        },
        operation_receipt_admission_allowed: Some(true),
    };
    let mut v = o
        .open_invocation_observation_with_receipts_v1(&id, &c, &p, limits(), admission, &mut r)
        .unwrap();
    let f = release(&c);
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let receipt = TrustedInvocationReceiptReleaseV1 {
        observation: &ordinary,
        operation_receipt_allowed: Some(true),
    };
    let mut native = TrustedRelationResultReleaseV1 {
        context: &c,
        max_result_bytes: 512,
        outcome_readable: Some(true),
        relation_result_readable: Some(true),
        relation_result_context_current: Some(true),
        historical_target_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    };
    assert_eq!(
        o.query_invocation_receipt_v1(
            &mut v,
            InvocationReceiptRequestV1::Relation(&native),
            &p,
            &receipt,
            &mut r
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    assert_eq!(
        o.invocation_state(&id, &mut r),
        Ok(InvocationState::Pending)
    );
    o.run(&id, &mut r).unwrap();
    let generation = o.generation();
    for _ in 0..2 {
        let InvocationReceiptV1::Relation(out) = o
            .query_invocation_receipt_v1(
                &mut v,
                InvocationReceiptRequestV1::Relation(&native),
                &p,
                &receipt,
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(out.invocation, id);
        assert!(matches!(
            out.receipt,
            RelationReceiptV1::WorkspaceCreated(_)
        ));
    }
    assert_eq!(o.generation(), generation);
    native.historical_target_readable = Some(false);
    assert_eq!(
        o.query_invocation_receipt_v1(
            &mut v,
            InvocationReceiptRequestV1::Relation(&native),
            &p,
            &receipt,
            &mut r
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut v, &p, &f, &mut r)
            .unwrap()
            .phase,
        InvocationObservationPhaseV1::Terminal
    );
    o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
}

#[test]
fn captured_family_capabilities_and_current_receipt_permission_are_independent_of_coarse_details() {
    let (mut o, id, c, p, mut r) = relation_pending(true);
    o.run(&id, &mut r).unwrap();
    let af = facts(&c);
    let f = release(&c);
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let n = native(&c);
    for mask in 0..64 {
        let caps = InvocationReceiptCapabilitiesV1 {
            annotation: mask & 1 != 0,
            reversal: mask & 2 != 0,
            relation: mask & 4 != 0,
            host_registration: mask & 8 != 0,
            scoped_registration: mask & 16 != 0,
            created_settings: mask & 32 != 0,
            ..Default::default()
        };
        let mut v = o
            .open_invocation_observation_with_receipts_v1(
                &id,
                &c,
                &p,
                limits(),
                InvocationReceiptAdmissionV1 {
                    observation: &af,
                    capabilities: caps,
                    operation_receipt_admission_allowed: Some(true),
                },
                &mut r,
            )
            .unwrap();
        for allowed in [None, Some(false), Some(true)] {
            let receipt = TrustedInvocationReceiptReleaseV1 {
                observation: &ordinary,
                operation_receipt_allowed: allowed,
            };
            let result = o.query_invocation_receipt_v1(
                &mut v,
                InvocationReceiptRequestV1::Relation(&n),
                &p,
                &receipt,
                &mut r,
            );
            assert_eq!(result.is_ok(), caps.relation && allowed == Some(true));
        }
        o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
            .unwrap();
    }
    let mut plain = o
        .open_invocation_observation_v1(&id, &c, &p, limits(), &af, &mut r)
        .unwrap();
    let receipt = TrustedInvocationReceiptReleaseV1 {
        observation: &ordinary,
        operation_receipt_allowed: Some(true),
    };
    let mut rich = p.clone();
    rich.details = true;
    assert_eq!(
        o.query_invocation_receipt_v1(
            &mut plain,
            InvocationReceiptRequestV1::Relation(&n),
            &rich,
            &receipt,
            &mut r
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
}

#[test]
fn original_captured_and_current_publication_permission_all_gate_receipt_disclosure() {
    for original in [false, true] {
        for captured in [false, true] {
            for current in [false, true] {
                let (mut o, id, c, p, mut r) = relation_pending(original);
                let af = facts(&c);
                let mut ceiling = p.clone();
                ceiling.publication = captured;
                let mut v = o
                    .open_invocation_observation_with_receipts_v1(
                        &id,
                        &c,
                        &ceiling,
                        limits(),
                        InvocationReceiptAdmissionV1 {
                            observation: &af,
                            capabilities: InvocationReceiptCapabilitiesV1 {
                                relation: true,
                                ..Default::default()
                            },
                            operation_receipt_admission_allowed: Some(true),
                        },
                        &mut r,
                    )
                    .unwrap();
                o.run(&id, &mut r).unwrap();
                let f = release(&c);
                let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
                    current: &f,
                    ordinary_ready: Some(true),
                    resources_ready: Some(true),
                };
                let receipt = TrustedInvocationReceiptReleaseV1 {
                    observation: &ordinary,
                    operation_receipt_allowed: Some(true),
                };
                let n = native(&c);
                let mut p = p.clone();
                p.publication = current;
                assert_eq!(
                    o.query_invocation_receipt_v1(
                        &mut v,
                        InvocationReceiptRequestV1::Relation(&n),
                        &p,
                        &receipt,
                        &mut r
                    )
                    .is_ok(),
                    original && captured && current
                );
            }
        }
    }
}

#[test]
fn unsupported_facets_and_combined_frame_bounds_refuse_without_attaching_or_leaking_funding() {
    let (mut o, id, c, p, mut r) = relation_pending(true);
    let af = facts(&c);
    let reserved = o.retention_usage().reserved;
    for case in 0..4 {
        let mut caps = InvocationReceiptCapabilitiesV1::default();
        let mut allowed = Some(true);
        match case {
            0 => allowed = None,
            1 => allowed = Some(false),
            2 => caps.atomic_annotation = true,
            _ => caps.lifecycle = true,
        }
        assert_eq!(
            o.open_invocation_observation_with_receipts_v1(
                &id,
                &c,
                &p,
                limits(),
                InvocationReceiptAdmissionV1 {
                    observation: &af,
                    capabilities: caps,
                    operation_receipt_admission_allowed: allowed
                },
                &mut r
            )
            .unwrap_err(),
            InvocationError::Unavailable
        );
        assert_eq!(o.retention_usage().reserved, reserved);
    }
    for delta in [-1isize, 0, 1] {
        let mut l = limits();
        l.max_reply_bytes = InvocationReceiptCapabilitiesV1::required_reply_bytes(512)
            .unwrap()
            .checked_add_signed(delta)
            .unwrap();
        let v = o.open_invocation_observation_with_receipts_v1(
            &id,
            &c,
            &p,
            l,
            InvocationReceiptAdmissionV1 {
                observation: &af,
                capabilities: InvocationReceiptCapabilitiesV1 {
                    relation: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        );
        if delta < 0 {
            assert_eq!(v.unwrap_err(), InvocationError::Limit);
        } else {
            o.retire_invocation_observation_v1(&mut v.unwrap(), &control(&c), &mut r)
                .unwrap();
        }
        assert_eq!(o.retention_usage().reserved, reserved);
    }
    assert_eq!(
        InvocationReceiptCapabilitiesV1::required_reply_bytes(usize::MAX),
        Err(InvocationError::Limit)
    );
}

#[test]
fn native_release_facts_context_and_budget_are_not_replaced_by_common_observation_permission() {
    let (mut o, id, c, p, mut r) = relation_pending(true);
    let af = facts(&c);
    let mut v = o
        .open_invocation_observation_with_receipts_v1(
            &id,
            &c,
            &p,
            limits(),
            InvocationReceiptAdmissionV1 {
                observation: &af,
                capabilities: InvocationReceiptCapabilitiesV1 {
                    relation: true,
                    ..Default::default()
                },
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    o.run(&id, &mut r).unwrap();
    let f = release(&c);
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let receipt = TrustedInvocationReceiptReleaseV1 {
        observation: &ordinary,
        operation_receipt_allowed: Some(true),
    };
    for gate in 0..6 {
        for value in [None, Some(false)] {
            let mut n = native(&c);
            match gate {
                0 => n.outcome_readable = value,
                1 => n.relation_result_readable = value,
                2 => n.relation_result_context_current = value,
                3 => n.historical_target_readable = value,
                4 => n.reply_ready = value,
                _ => n.resources_ready = value,
            }
            assert_eq!(
                o.query_invocation_receipt_v1(
                    &mut v,
                    InvocationReceiptRequestV1::Relation(&n),
                    &p,
                    &receipt,
                    &mut r
                )
                .err(),
                Some(InvocationError::Unavailable)
            );
        }
    }
    let mut other = c.clone();
    other.purpose.push('!');
    let wrong = native(&other);
    assert_eq!(
        o.query_invocation_receipt_v1(
            &mut v,
            InvocationReceiptRequestV1::Relation(&wrong),
            &p,
            &receipt,
            &mut r
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    let mut n = native(&c);
    n.max_result_bytes = 0;
    assert_eq!(
        o.query_invocation_receipt_v1(
            &mut v,
            InvocationReceiptRequestV1::Relation(&n),
            &p,
            &receipt,
            &mut r
        )
        .err(),
        Some(InvocationError::Limit)
    );
    let n = native(&c);
    assert!(
        o.query_invocation_receipt_v1(
            &mut v,
            InvocationReceiptRequestV1::Relation(&n),
            &p,
            &receipt,
            &mut r
        )
        .is_ok()
    );
}

#[test]
fn receipt_admission_release_timing_is_optional_inclusive_and_preserves_off_full_behavior() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, id, c, p, _) = relation_pending(true);
        let af = facts(&c);
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
        let mut v = o
            .open_invocation_observation_with_receipts_v1(
                &id,
                &c,
                &p,
                limits(),
                InvocationReceiptAdmissionV1 {
                    observation: &af,
                    capabilities: InvocationReceiptCapabilitiesV1 {
                        relation: true,
                        ..Default::default()
                    },
                    operation_receipt_admission_allowed: Some(true),
                },
                &mut r,
            )
            .unwrap();
        let f = release(&c);
        let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
            current: &f,
            ordinary_ready: Some(true),
            resources_ready: Some(true),
        };
        let mut receipt = TrustedInvocationReceiptReleaseV1 {
            observation: &ordinary,
            operation_receipt_allowed: Some(true),
        };
        let n = native(&c);
        assert_eq!(
            o.query_invocation_receipt_v1(
                &mut v,
                InvocationReceiptRequestV1::Relation(&n),
                &p,
                &receipt,
                &mut r
            )
            .err(),
            Some(InvocationError::Unavailable)
        );
        o.run(&id, &mut r).unwrap();
        assert!(
            o.query_invocation_receipt_v1(
                &mut v,
                InvocationReceiptRequestV1::Relation(&n),
                &p,
                &receipt,
                &mut r
            )
            .is_ok()
        );
        receipt.operation_receipt_allowed = Some(false);
        assert_eq!(
            o.query_invocation_receipt_v1(
                &mut v,
                InvocationReceiptRequestV1::Relation(&n),
                &p,
                &receipt,
                &mut r
            )
            .err(),
            Some(InvocationError::Unavailable)
        );
        let mut counts = [0; 3];
        let mut refused = 0;
        while let Some(e) = r.pop_front() {
            let i = match e.boundary {
                Boundary::InvocationReceiptAdmission => Some(0),
                Boundary::InvocationReceiptRelease => Some(1),
                Boundary::RelationResultRelease => Some(2),
                _ => None,
            };
            if let Some(i) = i {
                counts[i] += 1;
                refused += usize::from(e.disposition == Disposition::Refused);
                assert_eq!(e.service_time.is_some(), mode == 2);
            }
            assert!(!format!("{e:?}").contains("native-observation/1"));
        }
        assert_eq!(
            counts,
            if mode == 1 || mode == 2 {
                [1, 3, 1]
            } else {
                [0; 3]
            }
        );
        if mode == 1 || mode == 2 {
            assert_eq!(refused, 2);
        }
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
    }
}
