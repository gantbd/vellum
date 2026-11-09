use super::invocations::{admission, authority, context, facts, release, setup_owned};
use super::*;
use crate::discovery_support as discovery;
use std::mem::size_of;
use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
#[path = "../support/invocation_observation_fixture.rs"]
#[allow(dead_code)]
mod observation;
fn scope() -> DiscoveryScopeV1 {
    let mut c = context();
    c.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into();
    DiscoveryScopeV1 {
        context: c,
        policy_epoch: 7,
    }
}
fn prepare(o: &mut SourceCoordinator, r: &mut Recorder) -> DiscoveryCutIdV1 {
    o.prepare_native_source_load_discovery_cut_v1(
        &scope(),
        &NativeBuiltinEntryVisibilityV1 {
            order: Some(0),
            schema: true,
            presentation: false,
            advice: false,
            variants: [NativeBuiltinVariantVisibilityV1 {
                order: Some(0),
                enabled: None,
            }],
        },
        &discovery::facts(),
        r,
    )
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
fn limits(maximum: usize) -> InvocationObservationLimitsV1 {
    let mut l = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 0,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: size_of::<InvocationObservationReferenceV1>()
            .max(size_of::<InvocationControlReferenceV1>()),
        max_reply_bytes: InvocationReceiptV8::required_reply_bytes(maximum)
            .unwrap()
            .max(size_of::<InvocationWaitReplyV1>())
            .max(size_of::<InvocationControlObservationV1>()),
        max_control_reply_bytes: size_of::<InvocationControlObservationV1>(),
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    l
}
fn profile() -> observation::Profile {
    observation::Profile {
        epoch: 7,
        maximum: 256,
        capabilities: InvocationReceiptCapabilitiesV1 {
            lifecycle: true,
            ..Default::default()
        },
    }
}
fn open(
    o: &mut SourceCoordinator,
    id: &InvocationId,
    c: &SettingsExecutionContextV1,
    r: &mut Recorder,
) -> InvocationObservationContextV1 {
    o.open_invocation_observation_with_receipts_v8(
        id,
        c,
        &observation::policy(c, 7),
        limits(256),
        InvocationReceiptAdmissionV1 {
            observation: &observation::facts(c),
            capabilities: profile().capabilities,
            operation_receipt_admission_allowed: Some(true),
        },
        r,
    )
    .unwrap()
}
fn query<'a>(
    o: &'a SourceCoordinator,
    observer: &mut InvocationObservationContextV1,
    c: &SettingsExecutionContextV1,
    native: &TrustedSourceLoadResultV1<'_>,
    permission: Option<bool>,
    r: &mut Recorder,
) -> Result<InvocationReceiptV8<'a>, InvocationError> {
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
    o.query_invocation_receipt_v8(
        observer,
        InvocationReceiptRequestV8::SourceLoad(native),
        &observation::policy(c, 7),
        &TrustedInvocationReceiptReleaseV1 {
            observation: &ordinary,
            operation_receipt_allowed: permission,
        },
        r,
    )
}
#[test]
fn v9_existing_prefix_preserves_actual_v8_source_delivery_without_widening_v8_entrypoint() {
    let mut f = setup_owned();
    let (task, dispatch) = submitted(&mut f, b"new");
    complete(&mut f, &dispatch, b"candidate");
    let auth = authority(&mut f, &task);
    let c = context();
    let current = facts(&c);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    profile().bind(&mut f.owner, &id, &c, &mut f.recorder);
    f.owner
        .submit_source_load_v1(&id, &auth, &admission(&current), &mut f.recorder)
        .unwrap();
    f.owner
        .run_source_load_v1(&id, &current, &mut f.recorder)
        .unwrap();
    let mut l = limits(256);
    l.max_reply_bytes = InvocationReceiptV9::required_reply_bytes(256).unwrap();
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    let mut observer = f
        .owner
        .open_invocation_observation_with_receipts_v9(
            &id,
            &c,
            &observation::policy(&c, 7),
            l,
            InvocationReceiptAdmissionV1 {
                observation: &observation::facts(&c),
                capabilities: profile().capabilities,
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
    let native = release(&c);
    let generation = f.owner.generation();
    let InvocationReceiptV9::Existing(InvocationReceiptV8::SourceLoad(result)) = f
        .owner
        .query_invocation_receipt_v9(
            &mut observer,
            InvocationReceiptRequestV9::Existing(InvocationReceiptRequestV8::SourceLoad(&native)),
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
    assert_eq!(
        result.disposition,
        SourceReadActivationDispositionV1::Created
    );
    assert_eq!(f.owner.generation(), generation);
}
#[test]
fn source_load_f06_requires_v8_original_lifecycle_capability_and_the_complete_envelope() {
    let mut f = setup_owned();
    let (task, dispatch) = submitted(&mut f, b"new");
    complete(&mut f, &dispatch, b"candidate");
    let auth = authority(&mut f, &task);
    let c = context();
    let current = facts(&c);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    profile().bind(&mut f.owner, &id, &c, &mut f.recorder);
    f.owner
        .submit_source_load_v1(&id, &auth, &admission(&current), &mut f.recorder)
        .unwrap();
    f.owner
        .run_source_load_v1(&id, &current, &mut f.recorder)
        .unwrap();
    let mut short = limits(256);
    short.max_reply_bytes -= 1;
    short.max_held_bytes = short.required_hold_bytes().unwrap();
    let funded = f.owner.retention_usage();
    assert_eq!(
        f.owner
            .open_invocation_observation_with_receipts_v8(
                &id,
                &c,
                &observation::policy(&c, 7),
                short,
                InvocationReceiptAdmissionV1 {
                    observation: &observation::facts(&c),
                    capabilities: profile().capabilities,
                    operation_receipt_admission_allowed: Some(true),
                },
                &mut f.recorder
            )
            .err(),
        Some(InvocationError::Limit)
    );
    assert_eq!(f.owner.retention_usage(), funded);
    assert_eq!(
        InvocationReceiptV8::required_reply_bytes(usize::MAX),
        Err(InvocationError::Limit)
    );
    let mut old = f
        .owner
        .open_invocation_observation_with_receipts_v7(
            &id,
            &c,
            &observation::policy(&c, 7),
            limits(256),
            InvocationReceiptAdmissionV1 {
                observation: &observation::facts(&c),
                capabilities: profile().capabilities,
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
        .open_invocation_observation_with_receipts_v8(
            &id,
            &c,
            &observation::policy(&c, 7),
            limits(256),
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
    let mut observer = open(&mut f.owner, &id, &c, &mut f.recorder);
    assert!(matches!(
        query(
            &f.owner,
            &mut observer,
            &c,
            &release(&c),
            Some(true),
            &mut f.recorder
        ),
        Ok(InvocationReceiptV8::SourceLoad(_))
    ));
}
#[test]
fn installed_source_activation_replays_once_and_f06_retains_the_exact_historical_disposition() {
    for capacity in [0, 1, 512] {
        for mode in 0..3 {
            let mut f = Fixture::with_invocations();
            f.owner
                .configure_retention_budget(33554432, &mut f.recorder)
                .unwrap();
            f.owner
                .configure_source_reader_v1(super::limits(), &mut f.recorder)
                .unwrap();
            let (task, dispatch) =
                submitted(&mut f, if mode == 1 { b"source" } else { b"PRIVATE-NEW" });
            complete(&mut f, &dispatch, b"PRIVATE-CONTENT");
            if mode == 2 {
                f.owner
                    .activate_source_read_v1(&task, &mut f.recorder)
                    .unwrap();
            }
            let auth = authority(&mut f, &task);
            let c = context();
            let current = facts(&c);
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
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            let baseline = f.owner.retention_usage();
            profile().bind(&mut f.owner, &id, &c, &mut f.recorder);
            let p = prepare(&mut f.owner, &mut f.recorder);
            let mut denied = facts(&c);
            denied.activation_allowed = None;
            assert_eq!(
                f.owner.submit_discovered_source_load_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &scope(),
                        cut: &p,
                        entry: &entry,
                        variant: &variant,
                        facts: &discovery::facts()
                    },
                    &id,
                    &auth,
                    &admission(&denied),
                    &mut f.recorder
                ),
                Err(DiscoveredNativeCommandErrorV1::Invocation(
                    InvocationError::Unavailable
                ))
            );
            let p = prepare(&mut f.owner, &mut f.recorder);
            f.owner
                .submit_discovered_source_load_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &scope(),
                        cut: &p,
                        entry: &entry,
                        variant: &variant,
                        facts: &discovery::facts(),
                    },
                    &id,
                    &auth,
                    &admission(&current),
                    &mut f.recorder,
                )
                .unwrap();
            let mut observer = open(&mut f.owner, &id, &c, &mut f.recorder);
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
                .run_source_load_v1(&id, &current, &mut f.recorder)
                .unwrap();
            assert_eq!(f.owner.generation(), generation + u64::from(mode == 0));
            let p = prepare(&mut f.owner, &mut f.recorder);
            assert_eq!(
                f.owner.submit_discovered_source_load_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &scope(),
                        cut: &p,
                        entry: &entry,
                        variant: &variant,
                        facts: &discovery::facts()
                    },
                    &id,
                    &auth,
                    &admission(&current),
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
            let InvocationReceiptV8::SourceLoad(result) = query(
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
            assert_eq!(
                result.disposition,
                [
                    SourceReadActivationDispositionV1::Created,
                    SourceReadActivationDispositionV1::Joined,
                    SourceReadActivationDispositionV1::AlreadyActivated
                ][mode]
            );
            let d = f
                .owner
                .read(&result.document, &mut f.recorder)
                .unwrap()
                .version;
            let edit = f
                .owner
                .prepare(
                    &[DocumentEdits {
                        expected: d,
                        edits: vec![Replacement {
                            start: 0,
                            end: 0,
                            expected: "".into(),
                            inserted: "later".into(),
                        }],
                    }],
                    &[],
                    &mut f.recorder,
                )
                .unwrap();
            f.owner.commit(edit, &mut f.recorder).unwrap();
            let InvocationReceiptV8::SourceLoad(again) = query(
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
            assert_eq!(again, result);
            let control = InvocationObservationPolicyControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                retention_released: Some(true),
                delivery_drained: Some(true),
            };
            f.owner
                .retire_invocation_observation_v1(&mut observer, &control, &mut f.recorder)
                .unwrap();
            f.owner
                .forget_invocation_outcome(&id, &mut f.recorder)
                .unwrap();
            f.owner
                .retire_source_load_v1(&id, &control, &mut f.recorder)
                .unwrap();
            f.owner
                .retire_invocation_observation_policy_v1(&id, &control, &mut f.recorder)
                .unwrap();
            f.owner.retire_invocation(&id, &mut f.recorder).unwrap();
            assert_eq!(f.owner.retention_usage(), baseline);
            let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
            assert!(records.len() <= capacity);
            assert!(!format!("{records:?}").contains("PRIVATE"));
            if capacity == 512 {
                for boundary in [
                    Boundary::DiscoveryBuiltinPreparation,
                    Boundary::DiscoveryRequestRecognition,
                    Boundary::SourceLoadAdmission,
                    Boundary::SourceLoadService,
                    Boundary::InvocationReceiptRelease,
                ] {
                    assert!(
                        records.iter().any(|e| e.boundary == boundary
                            && (boundary == Boundary::DiscoveryRequestRecognition
                                || e.service_time.is_some())),
                        "{boundary:?}"
                    );
                }
            }
        }
    }
}
