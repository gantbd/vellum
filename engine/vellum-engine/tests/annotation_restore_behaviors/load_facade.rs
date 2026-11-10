use super::owned::{
    admission, baseline_request, context, control, facts, ready, ready_with_retention, release,
};
use super::*;
use std::mem::size_of;
use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
#[path = "../support/discovered_context_fixture.rs"]
mod discovery;
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
    o.prepare_native_annotation_load_discovery_cut_v1(
        &scope(),
        &NativeBuiltinEntryVisibilityV1 {
            order: Some(0),
            schema: true,
            presentation: false,
            advice: false,
            variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
                order: Some(i),
                enabled: None,
            }),
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
    [DiscoveryVariantIdV1; 2],
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
    assert_eq!(descriptor.variants.len(), 2);
    (
        context,
        entry,
        std::array::from_fn(|i| descriptor.variants[i].reference.clone()),
    )
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
        max_reply_bytes: InvocationReceiptV9::required_reply_bytes(maximum)
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
    o.open_invocation_observation_with_receipts_v9(
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
    native: &TrustedAnnotationLoadResultV1<'_>,
    permission: Option<bool>,
    r: &mut Recorder,
) -> Result<InvocationReceiptV9<'a>, InvocationError> {
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
    o.query_invocation_receipt_v9(
        observer,
        InvocationReceiptRequestV9::AnnotationLoad(native),
        &observation::policy(c, 7),
        &TrustedInvocationReceiptReleaseV1 {
            observation: &ordinary,
            operation_receipt_allowed: permission,
        },
        r,
    )
}
#[test]
fn v10_existing_delivers_actual_v9_annotation_result_without_reexecution() {
    let (mut f, _, _, auth, request) = ready(0);
    let c = context();
    let current = facts(&c);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    profile().bind(&mut f.owner, &id, &c, &mut f.recorder);
    f.owner
        .submit_annotation_load_v1(&id, &auth, &request, &admission(&current), &mut f.recorder)
        .unwrap();
    f.owner
        .run_annotation_load_v1(&id, &current, &mut f.recorder)
        .unwrap();
    let generation = f.owner.generation();
    let mut l = limits(256);
    l.max_reply_bytes = l
        .max_reply_bytes
        .max(InvocationReceiptV10::required_reply_bytes(256).unwrap());
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    let mut observer = f
        .owner
        .open_invocation_observation_with_receipts_v10(
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
    let common = TrustedInvocationObservationReleaseV1 {
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
        current: &common,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let InvocationReceiptV10::Existing(InvocationReceiptV9::AnnotationLoad(value)) = f
        .owner
        .query_invocation_receipt_v10(
            &mut observer,
            InvocationReceiptRequestV10::Existing(InvocationReceiptRequestV9::AnnotationLoad(
                &release(&c),
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
    assert_eq!(value.invocation, id);
    assert_eq!(
        value.outcome,
        f.owner
            .annotation_load_result_v1(&id, &release(&c), &mut f.recorder)
            .unwrap()
            .outcome
    );
    assert_eq!(f.owner.generation(), generation);
}
#[test]
fn annotation_f06_requires_v9_original_capability_and_complete_envelope_without_weakening_native_release()
 {
    let (mut f, _, _, auth, request) = ready(0);
    let c = context();
    let current = facts(&c);
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    profile().bind(&mut f.owner, &id, &c, &mut f.recorder);
    f.owner
        .submit_annotation_load_v1(&id, &auth, &request, &admission(&current), &mut f.recorder)
        .unwrap();
    f.owner
        .run_annotation_load_v1(&id, &current, &mut f.recorder)
        .unwrap();
    let mut short = limits(256);
    short.max_reply_bytes -= 1;
    short.max_held_bytes = short.required_hold_bytes().unwrap();
    let funded = f.owner.retention_usage();
    assert_eq!(
        f.owner
            .open_invocation_observation_with_receipts_v9(
                &id,
                &c,
                &observation::policy(&c, 7),
                short,
                InvocationReceiptAdmissionV1 {
                    observation: &observation::facts(&c),
                    capabilities: profile().capabilities,
                    operation_receipt_admission_allowed: Some(true)
                },
                &mut f.recorder
            )
            .err(),
        Some(InvocationError::Limit)
    );
    assert_eq!(f.owner.retention_usage(), funded);
    assert_eq!(
        InvocationReceiptV9::required_reply_bytes(usize::MAX),
        Err(InvocationError::Limit)
    );
    let mut old = f
        .owner
        .open_invocation_observation_with_receipts_v8(
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
        .open_invocation_observation_with_receipts_v9(
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
    assert_eq!(
        query(
            &f.owner,
            &mut observer,
            &c,
            &release(&c),
            None,
            &mut f.recorder
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    let mut denied = release(&c);
    denied.historical_document_readable = None;
    assert_eq!(
        query(
            &f.owner,
            &mut observer,
            &c,
            &denied,
            Some(true),
            &mut f.recorder
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    assert!(matches!(
        query(
            &f.owner,
            &mut observer,
            &c,
            &release(&c),
            Some(true),
            &mut f.recorder
        ),
        Ok(InvocationReceiptV9::AnnotationLoad(_))
    ));
}
#[test]
fn installed_annotation_phases_keep_selectors_execution_and_historical_f06_delivery_separate() {
    for capacity in [0, 1, 512] {
        for mode in 0..4 {
            let (mut f, read, _, auth, request) =
                ready_with_retention(usize::from(mode != 0), 33554432);
            let request = if mode >= 2 {
                baseline_request(&mut f, &read, request)
            } else {
                request
            };
            if mode == 3 {
                let pair = f.pair();
                f.owner
                    .adopt_loaded_annotation_baseline(&read, pair, 65536, &mut f.recorder)
                    .unwrap();
            }
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
            let (dc, entry, variants) = selectors(&mut f.owner, &mut f.recorder);
            let index = usize::from(mode >= 2);
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            let baseline = f.owner.retention_usage();
            profile().bind(&mut f.owner, &id, &c, &mut f.recorder);
            let p = prepare(&mut f.owner, &mut f.recorder);
            assert_eq!(
                f.owner.submit_discovered_annotation_load_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &scope(),
                        cut: &p,
                        entry: &entry,
                        variant: &variants[1 - index],
                        facts: &discovery::facts()
                    },
                    &id,
                    &auth,
                    &request,
                    &admission(&current),
                    &mut f.recorder
                ),
                Err(DiscoveredNativeCommandErrorV1::Discovery(
                    DiscoveryErrorV1::Unavailable
                ))
            );
            f.owner
                .retire_discovery_cut_v1(&p, &discovery::facts(), &mut f.recorder)
                .unwrap();
            let p = prepare(&mut f.owner, &mut f.recorder);
            let mut denied = facts(&c);
            denied.authorized = None;
            assert_eq!(
                f.owner.submit_discovered_annotation_load_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &scope(),
                        cut: &p,
                        entry: &entry,
                        variant: &variants[index],
                        facts: &discovery::facts()
                    },
                    &id,
                    &auth,
                    &request,
                    &admission(&denied),
                    &mut f.recorder
                ),
                Err(DiscoveredNativeCommandErrorV1::Invocation(
                    InvocationError::Unavailable
                ))
            );
            let p = prepare(&mut f.owner, &mut f.recorder);
            f.owner
                .submit_discovered_annotation_load_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &scope(),
                        cut: &p,
                        entry: &entry,
                        variant: &variants[index],
                        facts: &discovery::facts(),
                    },
                    &id,
                    &auth,
                    &request,
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
                .run_annotation_load_v1(&id, &current, &mut f.recorder)
                .unwrap();
            assert_eq!(
                f.owner.generation(),
                generation + u64::from(mode == 1 || mode == 2)
            );
            let InvocationReceiptV9::AnnotationLoad(result) = query(
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
            let expected = if mode < 2 {
                AnnotationLoadOutcomeV1::Restoration {
                    document: f.doc.document.clone(),
                    changed: mode == 1,
                }
            } else {
                AnnotationLoadOutcomeV1::Baseline {
                    document: f.doc.document.clone(),
                    changed: mode == 2,
                }
            };
            assert_eq!(result.outcome, expected);
            let p = prepare(&mut f.owner, &mut f.recorder);
            assert_eq!(
                f.owner.submit_discovered_annotation_load_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &scope(),
                        cut: &p,
                        entry: &entry,
                        variant: &variants[index],
                        facts: &discovery::facts()
                    },
                    &id,
                    &auth,
                    &request,
                    &admission(&current),
                    &mut f.recorder
                ),
                Ok(InvocationState::Finished)
            );
            let edit = f
                .owner
                .prepare(
                    &[DocumentEdits {
                        expected: f.doc.clone(),
                        edits: vec![vellum_engine::source::Replacement {
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
            let InvocationReceiptV9::AnnotationLoad(again) = query(
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
            f.owner
                .retire_invocation_observation_v1(&mut observer, &control(&c), &mut f.recorder)
                .unwrap();
            f.owner
                .forget_invocation_outcome(&id, &mut f.recorder)
                .unwrap();
            f.owner
                .retire_annotation_load_v1(&id, &control(&c), &mut f.recorder)
                .unwrap();
            f.owner
                .retire_invocation_observation_policy_v1(&id, &control(&c), &mut f.recorder)
                .unwrap();
            f.owner.retire_invocation(&id, &mut f.recorder).unwrap();
            assert_eq!(f.owner.retention_usage(), baseline);
            let events: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
            assert!(events.len() <= capacity);
            assert!(!format!("{events:?}").contains("loaded body"));
            if capacity == 512 {
                for boundary in [
                    Boundary::DiscoveryBuiltinPreparation,
                    Boundary::DiscoveryRequestRecognition,
                    Boundary::AnnotationLoadAdmission,
                    Boundary::AnnotationLoadService,
                    Boundary::InvocationReceiptRelease,
                ] {
                    assert!(
                        events.iter().any(|e| e.boundary == boundary
                            && (boundary == Boundary::DiscoveryRequestRecognition
                                || e.service_time.is_some())),
                        "{boundary:?}"
                    );
                }
            }
        }
    }
}
