use super::*;
use vellum_engine::capture::InertValueV1 as D;
use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1, Observation};

#[test]
fn installed_receipt_query_joins_the_original_native_result_and_rechecks_discovery_release() {
    for typed in [false, true] {
        for mode in 0..4 {
            let mut f = producer::disposed_with_retention_budget(33554432);
            let c = producer::context();
            let p = policy(&c);
            let current = producer::facts(&c);
            let q = producer::request(&mut f, RetainedReleaseOperationV1::SourceBinding);
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            f.owner
                .bind_invocation_observation_policy_v1(
                    &id,
                    &c,
                    &p,
                    InvocationObservationPolicyLimitsV1 {
                        max_context_bytes: 2048,
                        max_result_bytes: 512,
                    },
                    &facts(&c),
                    &mut f.recorder,
                )
                .unwrap();
            f.owner
                .submit_retained_release_v1(
                    &id,
                    &q,
                    &producer::admission(&current),
                    &mut f.recorder,
                )
                .unwrap();
            f.owner
                .run_retained_release_v1(&id, &current, &mut f.recorder)
                .unwrap();
            let mut l = limits();
            l.max_reply_bytes = InvocationReceiptV10::required_reply_bytes(512).unwrap();
            l.max_held_bytes = l.required_hold_bytes().unwrap();
            let mut observer = f
                .owner
                .open_invocation_observation_with_receipts_v10(
                    &id,
                    &c,
                    &p,
                    l,
                    InvocationReceiptAdmissionV1 {
                        observation: &facts(&c),
                        capabilities: InvocationReceiptCapabilitiesV1 {
                            lifecycle: true,
                            ..Default::default()
                        },
                        operation_receipt_admission_allowed: Some(true),
                    },
                    &mut f.recorder,
                )
                .unwrap();
            let context = observer.context_id_v1();
            let ds = DiscoveryScopeV1 {
                context: SettingsExecutionContextV1 {
                    profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                    ..c.clone()
                },
                policy_epoch: 7,
            };
            f.owner
                .configure_discovery_v1(
                    &ds,
                    discovery::limits(),
                    &discovery::facts(),
                    &mut f.recorder,
                )
                .unwrap();
            let visible = NativeBuiltinEntryVisibilityV1 {
                order: Some(0),
                schema: true,
                presentation: false,
                advice: false,
                variants: [NativeBuiltinVariantVisibilityV1 {
                    order: Some(0),
                    enabled: None,
                }],
            };
            let prepare = |o: &mut SourceCoordinator, r: &mut Recorder| {
                o.prepare_native_invocation_receipt_discovery_cut_v1(
                    &ds,
                    &visible,
                    &discovery::facts(),
                    r,
                )
                .unwrap()
            };
            let cut = prepare(&mut f.owner, &mut f.recorder);
            let dc = f
                .owner
                .acquire_discovery_v1(&ds, &cut, &discovery::facts(), &mut f.recorder)
                .unwrap()
                .context;
            let cut = prepare(&mut f.owner, &mut f.recorder);
            let DiscoveryReplyV1::Page { entries, .. } = f
                .owner
                .query_discovery_v1(
                    &dc,
                    &ds,
                    &cut,
                    DiscoveryQueryV1::List {
                        limit: 1,
                        continuation: None,
                    },
                    &discovery::facts(),
                    &mut f.recorder,
                )
                .unwrap()
            else {
                panic!()
            };
            let entry = entries[0].reference.clone();
            let cut = prepare(&mut f.owner, &mut f.recorder);
            let DiscoveryReplyV1::Descriptor { descriptor, .. } = f
                .owner
                .query_discovery_v1(
                    &dc,
                    &ds,
                    &cut,
                    DiscoveryQueryV1::Inspect { entry: &entry },
                    &discovery::facts(),
                    &mut f.recorder,
                )
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(descriptor.variants.len(), 1);
            assert!(descriptor.variants[0].schema.is_some());
            let variant = descriptor.variants[0].reference.clone();
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
            let result = TrustedInvocationReceiptReleaseV1 {
                observation: &ordinary,
                operation_receipt_allowed: Some(true),
            };
            let native_facts = producer::release(&c);
            let mut r = if mode == 0 {
                Recorder::disabled()
            } else {
                Recorder::try_with_capacity(if mode == 3 { 1 } else { 256 }).unwrap()
            };
            if mode >= 2 {
                r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            }
            if mode == 3 {
                r.record(Observation {
                    boundary: Boundary::Query,
                    disposition: Disposition::Completed,
                    service_time: None,
                });
            }
            let before = f.owner.retention_usage();
            let generation = f.owner.generation();
            for denied in [false, false, true] {
                let cut = prepare(&mut f.owner, &mut f.recorder);
                let mut df = discovery::facts();
                if denied {
                    df.reply_ready = Some(false);
                }
                envelope(&id, &id, &context, &c.profile, typed, 99, |e| {
                    let answer = f.owner.query_discovered_invocation_receipt_v10(
                        NativeDiscoveredCommandRouteV1 {
                            context: &dc,
                            scope: &ds,
                            cut: &cut,
                            entry: &entry,
                            variant: &variant,
                            facts: &df,
                        },
                        &mut observer,
                        e,
                        native(&native_facts),
                        &p,
                        &result,
                        &mut r,
                    );
                    if denied {
                        assert!(matches!(
                            answer,
                            Err(DiscoveredInvocationObservationErrorV1::Discovery(
                                DiscoveryErrorV1::Unavailable
                            ))
                        ));
                    } else {
                        assert_eq!(answer.unwrap().invocation(), &id);
                    }
                });
            }
            // Losing discovery delivery does not manufacture native revocation.
            envelope(&id, &id, &context, &c.profile, typed, 99, |e| {
                assert_eq!(
                    f.owner
                        .query_decoded_invocation_receipt_v10(
                            &mut observer,
                            e,
                            native(&native_facts),
                            &p,
                            &result,
                            &mut f.recorder
                        )
                        .unwrap()
                        .invocation(),
                    &id
                )
            });
            assert_eq!(f.owner.generation(), generation);
            assert_eq!(f.owner.retention_usage(), before);
            let mut completed = 0;
            let mut refused = 0;
            while let Some(e) = r.pop_front() {
                if e.boundary == Boundary::DiscoveryInvocationObservationDispatch {
                    assert_eq!(e.service_time.is_some(), mode == 2);
                    completed += usize::from(e.disposition == Disposition::Completed);
                    refused += usize::from(e.disposition == Disposition::Refused);
                }
            }
            if mode == 1 || mode == 2 {
                assert_eq!((completed, refused), (2, 1));
            }
            if mode == 3 {
                assert!(r.dropped() > 0);
            }
            if mode == 0 {
                assert_eq!(r.dropped(), 0);
            }
        }
    }
}

fn native<'a>(f: &'a TrustedRetainedReleaseResultV1<'a>) -> InvocationReceiptRequestV10<'a> {
    InvocationReceiptRequestV10::Existing(InvocationReceiptRequestV9::Existing(
        InvocationReceiptRequestV8::Existing(InvocationReceiptRequestV7::Existing(
            InvocationReceiptRequestV6::Existing(InvocationReceiptRequestV5::Existing(
                InvocationReceiptRequestV4::Existing(InvocationReceiptRequestV3::RetainedRelease(
                    f,
                )),
            )),
        )),
    ))
}

#[test]
fn receipt_ingress_joins_original_result_and_refuses_substitution_before_applying_loss() {
    for typed in [false, true] {
        for mode in 0..4 {
            let mut f = producer::disposed();
            let c = producer::context();
            let current = producer::facts(&c);
            let request = producer::request(&mut f, RetainedReleaseOperationV1::SourceBinding);
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            f.owner
                .bind_invocation_observation_policy_v1(
                    &id,
                    &c,
                    &policy(&c),
                    InvocationObservationPolicyLimitsV1 {
                        max_context_bytes: 2048,
                        max_result_bytes: 512,
                    },
                    &facts(&c),
                    &mut f.recorder,
                )
                .unwrap();
            f.owner
                .submit_retained_release_v1(
                    &id,
                    &request,
                    &producer::admission(&current),
                    &mut f.recorder,
                )
                .unwrap();
            let mut bounds = limits();
            bounds.max_reply_bytes = InvocationReceiptV10::required_reply_bytes(512).unwrap();
            bounds.max_held_bytes = bounds.required_hold_bytes().unwrap();
            let mut observer = f
                .owner
                .open_invocation_observation_with_receipts_v10(
                    &id,
                    &c,
                    &policy(&c),
                    bounds,
                    InvocationReceiptAdmissionV1 {
                        observation: &facts(&c),
                        capabilities: InvocationReceiptCapabilitiesV1 {
                            lifecycle: true,
                            ..Default::default()
                        },
                        operation_receipt_admission_allowed: Some(true),
                    },
                    &mut f.recorder,
                )
                .unwrap();
            let context = observer.context_id_v1();
            let foreign = f.owner.issue_invocation(&mut f.recorder).unwrap();
            f.owner
                .run_retained_release_v1(&id, &current, &mut f.recorder)
                .unwrap();
            let generation = f.owner.generation();
            let debt = f.owner.retention_usage();
            let p = policy(&c);
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
                    boundary: Boundary::Query,
                    disposition: Disposition::Completed,
                    service_time: None,
                });
            }
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
            let result_facts = TrustedInvocationReceiptReleaseV1 {
                observation: &ordinary,
                operation_receipt_allowed: Some(true),
            };
            let native_facts = producer::release(&c);
            let lost_release = TrustedInvocationObservationReleaseV1 {
                authorized: Some(false),
                ..release
            };
            let lost_ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
                current: &lost_release,
                ordinary_ready: Some(true),
                resources_ready: Some(true),
            };
            let lost_facts = TrustedInvocationReceiptReleaseV1 {
                observation: &lost_ordinary,
                operation_receipt_allowed: Some(false),
            };
            // Duplicate/unknown fields, wrong tuple/contract/profile/role/index and
            // foreign original identity must not reach native release or absorb loss.
            for case in 0..11 {
                envelope(&id, &foreign, &context, &c.profile, typed, case, |e| {
                    assert!(
                        f.owner
                            .query_decoded_invocation_receipt_v10(
                                &mut observer,
                                e,
                                native(&native_facts),
                                &p,
                                &lost_facts,
                                &mut r
                            )
                            .is_err()
                    );
                });
            }
            for _ in 0..2 {
                envelope(&id, &foreign, &context, &c.profile, typed, 99, |e| {
                    assert_eq!(
                        f.owner
                            .query_decoded_invocation_receipt_v10(
                                &mut observer,
                                e,
                                native(&native_facts),
                                &p,
                                &result_facts,
                                &mut r
                            )
                            .unwrap()
                            .invocation(),
                        &id
                    );
                });
            }
            // The same loss on an attributed valid request must be enforced.
            envelope(&id, &foreign, &context, &c.profile, typed, 99, |e| {
                assert!(
                    f.owner
                        .query_decoded_invocation_receipt_v10(
                            &mut observer,
                            e,
                            native(&native_facts),
                            &p,
                            &lost_facts,
                            &mut r
                        )
                        .is_err()
                );
            });
            let mut native_calls = 0;
            let mut completed_ingress = 0;
            while let Some(event) = r.pop_front() {
                if event.boundary == Boundary::InvocationReceiptRelease {
                    native_calls += 1;
                }
                if event.boundary == Boundary::InvocationReceiptIngress {
                    assert_eq!(event.service_time.is_some(), mode == 2);
                    completed_ingress += usize::from(event.disposition == Disposition::Completed);
                }
            }
            if mode == 1 || mode == 2 {
                assert_eq!(native_calls, 3);
                assert_eq!(completed_ingress, 2);
            }
            assert_eq!(f.owner.generation(), generation);
            assert_eq!(f.owner.retention_usage(), debt);
            if mode == 3 {
                assert!(r.dropped() > 0);
            }
            if mode == 0 {
                assert_eq!(r.dropped(), 0);
            }
        }
    }
}

fn envelope<T>(
    id: &InvocationId,
    foreign: &InvocationId,
    context: &InvocationObservationContextIdV1,
    profile: &str,
    typed: bool,
    case: usize,
    use_it: impl FnOnce(&InvocationReceiptEnvelopeValueV1<'_, '_>) -> T,
) -> T {
    use InvocationReceiptEnvelopeValueV1 as V;
    let mut facet = vec![(
        "kind",
        V::Data(D::Text(if case == 7 { "annotation" } else { "lifecycle" })),
    )];
    if case == 8 {
        facet.push(("index", V::Data(D::Number(0.0))));
    }
    let mut args = vec![
        (
            "contract",
            V::Data(D::Text(if case == 4 {
                "execution-receipt/2"
            } else {
                "execution-receipt/1"
            })),
        ),
        (
            "operation",
            V::Data(D::Text(if case == 5 {
                "status"
            } else {
                "retained-outcome"
            })),
        ),
        (
            "invocation",
            V::Invocation(if case == 9 { foreign } else { id }),
        ),
        (
            "context",
            if case == 10 {
                V::Data(D::Text("forged"))
            } else {
                V::Context(context)
            },
        ),
        (
            "profile",
            V::Data(D::Text(if case == 6 { "foreign" } else { profile })),
        ),
        ("facet", V::Object(&facet)),
    ];
    if case == 0 {
        args.push(("contract", V::Data(D::Text("execution-receipt/1"))));
    }
    if case == 1 {
        args[0].0 = "unknown";
    }
    let header = [
        (
            if typed { "family" } else { "name" },
            if typed {
                V::Data(D::Number(if case == 2 { 999.0 } else { 5.0 }))
            } else {
                V::Data(D::Text(if case == 2 {
                    "foreign"
                } else {
                    "inspect-execution"
                }))
            },
        ),
        ("variant", V::Data(D::Text("retained-outcome"))),
        (
            "version",
            V::Data(D::Number(if case == 3 { 2.0 } else { 1.0 })),
        ),
        ("arguments", V::Object(&args)),
    ];
    use_it(&V::Object(&[
        (
            "route",
            V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&header)),
    ]))
}
