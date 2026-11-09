use super::invocations::{admission, context, release, request};
use super::*;
use super::{discovery_support as discovery, observation_support as observation};
use std::mem::size_of;
use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
#[path = "../support/scratch_invocation_fixture.rs"]
#[allow(dead_code)]
mod scratch;
fn scope() -> DiscoveryScopeV1 {
    let mut c = context();
    c.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into();
    DiscoveryScopeV1 {
        context: c,
        policy_epoch: 7,
    }
}
fn visible() -> NativeBuiltinEntryVisibilityV1<1> {
    NativeBuiltinEntryVisibilityV1 {
        order: Some(0),
        schema: true,
        presentation: false,
        advice: false,
        variants: [NativeBuiltinVariantVisibilityV1 {
            order: Some(0),
            enabled: None,
        }],
    }
}
fn prepare(o: &mut SourceCoordinator, r: &mut Recorder) -> DiscoveryCutIdV1 {
    o.prepare_native_rebind_discovery_cut_v1(&scope(), &visible(), &discovery::facts(), r)
        .unwrap()
}
fn selectors(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
) -> (
    DiscoveryContextIdV1,
    DiscoveryEntryIdV1,
    DiscoveryVariantIdV1,
) {
    let p = prepare(o, r);
    let context = o
        .acquire_discovery_v1(&scope(), &p, &discovery::facts(), r)
        .unwrap()
        .context;
    let p = prepare(o, r);
    let DiscoveryReplyV1::Page { entries, .. } = o
        .query_discovery_v1(
            &context,
            &scope(),
            &p,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
            &discovery::facts(),
            r,
        )
        .unwrap()
    else {
        panic!()
    };
    let entry = entries[0].reference.clone();
    let p = prepare(o, r);
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
        .query_discovery_v1(
            &context,
            &scope(),
            &p,
            DiscoveryQueryV1::Inspect { entry: &entry },
            &discovery::facts(),
            r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(descriptor.variants.len(), 1);
    (context, entry, descriptor.variants[0].reference.clone())
}
fn observer_limits(maximum: usize) -> InvocationObservationLimitsV1 {
    let mut l = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 0,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: size_of::<InvocationObservationReferenceV1>()
            .max(size_of::<InvocationControlReferenceV1>()),
        max_reply_bytes: InvocationReceiptV7::required_reply_bytes(maximum)
            .unwrap()
            .max(size_of::<InvocationWaitReplyV1>())
            .max(size_of::<InvocationControlObservationV1>()),
        max_control_reply_bytes: size_of::<InvocationControlObservationV1>(),
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    l
}
fn query<'a>(
    o: &'a SourceCoordinator,
    observer: &mut InvocationObservationContextV1,
    c: &SettingsExecutionContextV1,
    native: &TrustedSourceRebindResultV1<'_>,
    permission: Option<bool>,
    r: &mut Recorder,
) -> Result<InvocationReceiptV7<'a>, InvocationError> {
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
    o.query_invocation_receipt_v7(
        observer,
        InvocationReceiptRequestV7::SourceRebind(native),
        &observation::policy(c, 7),
        &TrustedInvocationReceiptReleaseV1 {
            observation: &ordinary,
            operation_receipt_allowed: permission,
        },
        r,
    )
}
#[test]
fn v8_existing_prefix_preserves_actual_v7_rebind_delivery_without_widening_the_v7_entrypoint() {
    let mut f = Fixture::with_invocations();
    f.owner
        .configure_retention_budget(1048576, &mut f.recorder)
        .unwrap();
    let c = context();
    let native = rebind_facts();
    let current = TrustedSourceRebindInvocationFactsV1 {
        context: &c,
        native: &native,
    };
    let q = request(&mut f, b"source");
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    let profile = observation::Profile {
        epoch: 7,
        maximum: 512,
        capabilities: InvocationReceiptCapabilitiesV1 {
            lifecycle: true,
            ..Default::default()
        },
    };
    profile.bind(&mut f.owner, &id, &c, &mut f.recorder);
    f.owner
        .submit_source_rebind_v1(&id, &q, &admission(&current), &mut f.recorder)
        .unwrap();
    f.owner
        .run_source_rebind_v1(&id, &current, &mut f.recorder)
        .unwrap();
    let mut l = observer_limits(512);
    l.max_reply_bytes = InvocationReceiptV8::required_reply_bytes(512).unwrap();
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    let mut observer = f
        .owner
        .open_invocation_observation_with_receipts_v8(
            &id,
            &c,
            &observation::policy(&c, 7),
            l,
            InvocationReceiptAdmissionV1 {
                observation: &observation::facts(&c),
                capabilities: profile.capabilities,
                operation_receipt_admission_allowed: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        query(
            &f.owner,
            &mut observer,
            &c,
            &release(&c),
            Some(true),
            &mut f.recorder
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    let grant = TrustedInvocationObservationReleaseV1 {
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
        current: &grant,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let native_release = release(&c);
    let generation = f.owner.generation();
    let InvocationReceiptV8::Existing(InvocationReceiptV7::SourceRebind(result)) = f
        .owner
        .query_invocation_receipt_v8(
            &mut observer,
            InvocationReceiptRequestV8::Existing(InvocationReceiptRequestV7::SourceRebind(
                &native_release,
            )),
            &observation::policy(&c, 7),
            &TrustedInvocationReceiptReleaseV1 {
                observation: &ordinary,
                operation_receipt_allowed: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(result.invocation, id);
    assert_eq!(result.document, f.doc.document);
    assert!(!result.changed);
    assert_eq!(f.owner.generation(), generation);
}
#[test]
fn rebind_f06_requires_v7_original_capability_and_the_complete_reply_envelope() {
    let mut f = Fixture::with_invocations();
    f.owner
        .configure_retention_budget(1048576, &mut f.recorder)
        .unwrap();
    let c = context();
    let native = rebind_facts();
    let current = TrustedSourceRebindInvocationFactsV1 {
        context: &c,
        native: &native,
    };
    let q = request(&mut f, b"new-source");
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    let before = f.owner.retention_usage();
    let profile = observation::Profile {
        epoch: 7,
        maximum: 512,
        capabilities: InvocationReceiptCapabilitiesV1 {
            lifecycle: true,
            ..Default::default()
        },
    };
    profile.bind(&mut f.owner, &id, &c, &mut f.recorder);
    f.owner
        .submit_source_rebind_v1(&id, &q, &admission(&current), &mut f.recorder)
        .unwrap();
    f.owner
        .run_source_rebind_v1(&id, &current, &mut f.recorder)
        .unwrap();
    let mut short = observer_limits(512);
    short.max_reply_bytes -= 1;
    short.max_held_bytes = short.required_hold_bytes().unwrap();
    let funded = f.owner.retention_usage();
    assert_eq!(
        f.owner
            .open_invocation_observation_with_receipts_v7(
                &id,
                &c,
                &observation::policy(&c, 7),
                short,
                InvocationReceiptAdmissionV1 {
                    observation: &observation::facts(&c),
                    capabilities: profile.capabilities,
                    operation_receipt_admission_allowed: Some(true)
                },
                &mut f.recorder
            )
            .err(),
        Some(InvocationError::Limit)
    );
    assert_eq!(f.owner.retention_usage(), funded);
    let mut old = f
        .owner
        .open_invocation_observation_with_receipts_v6(
            &id,
            &c,
            &observation::policy(&c, 7),
            observer_limits(512),
            InvocationReceiptAdmissionV1 {
                observation: &observation::facts(&c),
                capabilities: profile.capabilities,
                operation_receipt_admission_allowed: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        query(
            &f.owner,
            &mut old,
            &c,
            &release(&c),
            Some(true),
            &mut f.recorder
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    let mut masked = f
        .owner
        .open_invocation_observation_with_receipts_v7(
            &id,
            &c,
            &observation::policy(&c, 7),
            observer_limits(512),
            InvocationReceiptAdmissionV1 {
                observation: &observation::facts(&c),
                capabilities: InvocationReceiptCapabilitiesV1::default(),
                operation_receipt_admission_allowed: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        query(
            &f.owner,
            &mut masked,
            &c,
            &release(&c),
            Some(true),
            &mut f.recorder
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    let mut observer = f
        .owner
        .open_invocation_observation_with_receipts_v7(
            &id,
            &c,
            &observation::policy(&c, 7),
            observer_limits(512),
            InvocationReceiptAdmissionV1 {
                observation: &observation::facts(&c),
                capabilities: profile.capabilities,
                operation_receipt_admission_allowed: Some(true),
            },
            &mut f.recorder,
        )
        .unwrap();
    {
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
        assert_eq!(
            f.owner
                .query_invocation_receipt_v7(
                    &mut observer,
                    InvocationReceiptRequestV7::Existing(InvocationReceiptRequestV6::Scratch(
                        &scratch::release(&c)
                    )),
                    &observation::policy(&c, 7),
                    &TrustedInvocationReceiptReleaseV1 {
                        observation: &ordinary,
                        operation_receipt_allowed: Some(true)
                    },
                    &mut f.recorder
                )
                .err(),
            Some(InvocationError::OutcomeUnavailable)
        );
    }
    assert!(matches!(
        query(
            &f.owner,
            &mut observer,
            &c,
            &release(&c),
            Some(true),
            &mut f.recorder
        )
        .unwrap(),
        InvocationReceiptV7::SourceRebind(_)
    ));
    let control = InvocationObservationPolicyControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    for v in [&mut old, &mut masked, &mut observer] {
        f.owner
            .retire_invocation_observation_v1(v, &control, &mut f.recorder)
            .unwrap();
    }
    f.owner
        .forget_invocation_outcome(&id, &mut f.recorder)
        .unwrap();
    f.owner
        .retire_source_rebind_v1(&id, &control, &mut f.recorder)
        .unwrap();
    f.owner
        .retire_invocation_observation_policy_v1(&id, &control, &mut f.recorder)
        .unwrap();
    f.owner.retire_invocation(&id, &mut f.recorder).unwrap();
    assert_eq!(f.owner.retention_usage(), before);
}
#[test]
fn v7_existing_prefix_delegates_the_original_scratch_result_without_reallocation() {
    let (mut o, mut r) = scratch::owner();
    let c = scratch::context();
    let facts = scratch::facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    let profile = observation::Profile {
        epoch: 7,
        maximum: 512,
        capabilities: InvocationReceiptCapabilitiesV1 {
            lifecycle: true,
            ..Default::default()
        },
    };
    profile.bind(&mut o, &id, &c, &mut r);
    o.submit_scratch_v1(
        &id,
        &scratch::request(b"PRIVATE-SCRATCH"),
        &scratch::admission(&facts),
        &mut r,
    )
    .unwrap();
    o.run_scratch_v1(&id, &facts, &mut r).unwrap();
    let mut observer = o
        .open_invocation_observation_with_receipts_v7(
            &id,
            &c,
            &observation::policy(&c, 7),
            observer_limits(512),
            InvocationReceiptAdmissionV1 {
                observation: &observation::facts(&c),
                capabilities: profile.capabilities,
                operation_receipt_admission_allowed: Some(true),
            },
            &mut r,
        )
        .unwrap();
    let generation = o.generation();
    let issued = o.lifecycle_usage().issued_lifetime;
    for _ in 0..2 {
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
        let reply = o
            .query_invocation_receipt_v7(
                &mut observer,
                InvocationReceiptRequestV7::Existing(InvocationReceiptRequestV6::Scratch(
                    &scratch::release(&c),
                )),
                &observation::policy(&c, 7),
                &TrustedInvocationReceiptReleaseV1 {
                    observation: &ordinary,
                    operation_receipt_allowed: Some(true),
                },
                &mut r,
            )
            .unwrap();
        let InvocationReceiptV7::Existing(InvocationReceiptV6::Scratch(result)) = reply else {
            panic!()
        };
        assert_eq!(result.invocation, id);
    }
    assert_eq!(o.generation(), generation);
    assert_eq!(o.lifecycle_usage().issued_lifetime, issued);
    let control = InvocationObservationPolicyControlV1 {
        context: &c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    o.retire_invocation_observation_v1(&mut observer, &control, &mut r)
        .unwrap();
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    o.retire_scratch_v1(&id, &control, &mut r).unwrap();
    o.retire_invocation_observation_policy_v1(&id, &control, &mut r)
        .unwrap();
    o.retire_invocation(&id, &mut r).unwrap();
}
#[test]
fn discovered_rebind_publishes_once_and_releases_only_the_original_authorized_f06_result() {
    for capacity in [0, 1, 512] {
        for unchanged in [false, true] {
            let mut f = Fixture::with_invocations();
            f.owner
                .configure_retention_budget(33554432, &mut f.recorder)
                .unwrap();
            f.recorder = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            f.owner
                .configure_discovery_v1(
                    &scope(),
                    discovery::limits(),
                    &discovery::facts(),
                    &mut f.recorder,
                )
                .unwrap();
            let (dc, entry, variant) = selectors(&mut f.owner, &mut f.recorder);
            let q = request(&mut f, if unchanged { b"source" } else { b"new-source" });
            let c = context();
            let native = rebind_facts();
            let facts = TrustedSourceRebindInvocationFactsV1 {
                context: &c,
                native: &native,
            };
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            let baseline = f.owner.retention_usage();
            let profile = observation::Profile {
                epoch: 7,
                maximum: 512,
                capabilities: InvocationReceiptCapabilitiesV1 {
                    lifecycle: true,
                    ..Default::default()
                },
            };
            profile.bind(&mut f.owner, &id, &c, &mut f.recorder);
            let p = prepare(&mut f.owner, &mut f.recorder);
            f.owner
                .submit_discovered_source_rebind_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &scope(),
                        cut: &p,
                        entry: &entry,
                        variant: &variant,
                        facts: &discovery::facts(),
                    },
                    &id,
                    &q,
                    &admission(&facts),
                    &mut f.recorder,
                )
                .unwrap();
            let mut observer = f
                .owner
                .open_invocation_observation_with_receipts_v7(
                    &id,
                    &c,
                    &observation::policy(&c, 7),
                    observer_limits(512),
                    InvocationReceiptAdmissionV1 {
                        observation: &observation::facts(&c),
                        capabilities: profile.capabilities,
                        operation_receipt_admission_allowed: Some(true),
                    },
                    &mut f.recorder,
                )
                .unwrap();
            assert!(
                query(
                    &f.owner,
                    &mut observer,
                    &c,
                    &release(&c),
                    Some(true),
                    &mut f.recorder
                )
                .is_err()
            );
            let generation = f.owner.generation();
            f.owner
                .run_source_rebind_v1(&id, &facts, &mut f.recorder)
                .unwrap();
            assert_eq!(f.owner.generation(), generation + u64::from(!unchanged));
            let generation = f.owner.generation();
            let p = prepare(&mut f.owner, &mut f.recorder);
            assert_eq!(
                f.owner.submit_discovered_source_rebind_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &scope(),
                        cut: &p,
                        entry: &entry,
                        variant: &variant,
                        facts: &discovery::facts()
                    },
                    &id,
                    &q,
                    &admission(&facts),
                    &mut f.recorder
                ),
                Ok(InvocationState::Finished)
            );
            assert!(
                query(
                    &f.owner,
                    &mut observer,
                    &c,
                    &release(&c),
                    None,
                    &mut f.recorder
                )
                .is_err()
            );
            let mut denied = release(&c);
            denied.result_allowed = None;
            assert!(
                query(
                    &f.owner,
                    &mut observer,
                    &c,
                    &denied,
                    Some(true),
                    &mut f.recorder
                )
                .is_err()
            );
            let InvocationReceiptV7::SourceRebind(result) = query(
                &f.owner,
                &mut observer,
                &c,
                &release(&c),
                Some(true),
                &mut f.recorder,
            )
            .unwrap() else {
                panic!()
            };
            assert_eq!(result.invocation, id);
            assert_eq!(result.document, f.doc.document);
            assert_eq!(result.changed, !unchanged);
            assert_eq!(f.owner.generation(), generation);
            let control = InvocationObservationPolicyControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            };
            f.owner
                .forget_invocation_outcome(&id, &mut f.recorder)
                .unwrap();
            assert!(
                query(
                    &f.owner,
                    &mut observer,
                    &c,
                    &release(&c),
                    Some(true),
                    &mut f.recorder
                )
                .is_err()
            );
            f.owner
                .retire_invocation_observation_v1(&mut observer, &control, &mut f.recorder)
                .unwrap();
            f.owner
                .retire_source_rebind_v1(&id, &control, &mut f.recorder)
                .unwrap();
            f.owner
                .retire_invocation_observation_policy_v1(&id, &control, &mut f.recorder)
                .unwrap();
            f.owner.retire_invocation(&id, &mut f.recorder).unwrap();
            assert_eq!(f.owner.retention_usage(), baseline);
            if capacity == 512 {
                let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
                for boundary in [
                    Boundary::SourceRebindAdmission,
                    Boundary::SourceRebindService,
                    Boundary::InvocationReceiptRelease,
                ] {
                    assert!(
                        records
                            .iter()
                            .any(|e| e.boundary == boundary && e.service_time.is_some())
                    );
                }
            }
        }
    }
}
