use super::saveas_invocations::{admission, authority_facts, context, ready, release};
use super::*;
use super::{discovery_support as discovery, observation_support as observation};
use std::mem::size_of;
use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
fn scope() -> DiscoveryScopeV1 {
    let mut context = context();
    context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into();
    DiscoveryScopeV1 {
        context,
        policy_epoch: 7,
    }
}
fn prepare(o: &mut SourceCoordinator, r: &mut Recorder) -> DiscoveryCutIdV1 {
    o.prepare_native_save_as_discovery_cut_v1(
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
    let c = o
        .acquire_discovery_v1(&scope(), &p, &discovery::facts(), r)
        .unwrap()
        .context;
    let p = prepare(o, r);
    let DiscoveryReplyV1::Page { entries, .. } = o
        .query_discovery_v1(
            &c,
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
            &c,
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
    (c, entry, descriptor.variants[0].reference.clone())
}
fn limits() -> InvocationObservationLimitsV1 {
    let mut l = InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 0,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: InvocationObservationLimitsV1::required_wait_bytes(),
        max_reference_bytes: size_of::<InvocationObservationReferenceV1>()
            .max(size_of::<InvocationControlReferenceV1>()),
        max_reply_bytes: InvocationReceiptV10::required_reply_bytes(256)
            .unwrap()
            .max(size_of::<InvocationWaitReplyV1>())
            .max(size_of::<InvocationControlObservationV1>()),
        max_control_reply_bytes: size_of::<InvocationControlObservationV1>(),
        max_held_bytes: 0,
    };
    l.max_held_bytes = l.required_hold_bytes().unwrap();
    l
}
fn caps() -> InvocationReceiptCapabilitiesV1 {
    InvocationReceiptCapabilitiesV1 {
        lifecycle: true,
        ..Default::default()
    }
}
fn query<'a>(
    o: &'a SourceCoordinator,
    observer: &mut InvocationObservationContextV1,
    c: &SettingsExecutionContextV1,
    request: InvocationReceiptRequestV10<'_>,
    permission: Option<bool>,
    r: &mut Recorder,
) -> Result<InvocationReceiptV10<'a>, InvocationError> {
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
    o.query_invocation_receipt_v10(
        observer,
        request,
        &observation::policy(c, 7),
        &TrustedInvocationReceiptReleaseV1 {
            observation: &ordinary,
            operation_receipt_allowed: permission,
        },
        r,
    )
}
#[test]
fn installed_saveas_routes_original_activation_and_f06_release_without_another_write() {
    for capacity in [0, 1, 256] {
        let (mut f, d, _, _, auth, request) =
            super::saveas_invocations::ready_with_retention(33554432);
        f.owner
            .configure_discovery_v1(
                &scope(),
                discovery::limits(),
                &discovery::facts(),
                &mut f.recorder,
            )
            .unwrap();
        let (dc, entry, variant) = selectors(&mut f.owner, &mut f.recorder);
        let c = context();
        let native = activation_facts();
        let grant = authority_facts();
        let current = TrustedSourceSaveAsInvocationFactsV1 {
            context: &c,
            native: &native,
            authority: &grant,
            interpretation_current: Some(true),
        };
        let baseline = f.owner.retention_usage();
        let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
        observation::Profile {
            epoch: 7,
            maximum: 256,
            capabilities: caps(),
        }
        .bind(&mut f.owner, &id, &c, &mut f.recorder);
        f.recorder = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let before = f.owner.resource_usage();
        let p = prepare(&mut f.owner, &mut f.recorder);
        let denied_current = TrustedSourceSaveAsInvocationFactsV1 {
            interpretation_current: None,
            ..current
        };
        let denied = admission(&denied_current);
        let ds = scope();
        let df = discovery::facts();
        let route = |cut| NativeDiscoveredCommandRouteV1 {
            context: &dc,
            scope: &ds,
            cut,
            entry: &entry,
            variant: &variant,
            facts: &df,
        };
        assert_eq!(
            f.owner
                .submit_discovered_source_save_as_v1(
                    route(&p),
                    &id,
                    &auth,
                    &request,
                    &denied,
                    &mut f.recorder
                )
                .err(),
            Some(DiscoveredNativeCommandErrorV1::Invocation(
                InvocationError::Unavailable
            ))
        );
        let p = prepare(&mut f.owner, &mut f.recorder);
        f.owner
            .submit_discovered_source_save_as_v1(
                route(&p),
                &id,
                &auth,
                &request,
                &admission(&current),
                &mut f.recorder,
            )
            .unwrap();
        f.owner
            .run_source_save_as_v1(&id, &current, &mut f.recorder)
            .unwrap();
        let terminal = f.owner.invocation_state(&id, &mut f.recorder).unwrap();
        let p = prepare(&mut f.owner, &mut f.recorder);
        assert_eq!(
            f.owner
                .submit_discovered_source_save_as_v1(
                    route(&p),
                    &id,
                    &auth,
                    &request,
                    &admission(&current),
                    &mut f.recorder
                )
                .unwrap(),
            terminal
        );
        assert_eq!(
            f.owner.resource_usage().issued_lifetime,
            before.issued_lifetime
        );
        let edit = f
            .owner
            .prepare(
                &[DocumentEdits {
                    expected: d.clone(),
                    edits: vec![vellum_engine::source::Replacement {
                        start: 0,
                        end: 0,
                        expected: "".into(),
                        inserted: "later ".into(),
                    }],
                }],
                &[],
                &mut f.recorder,
            )
            .unwrap();
        f.owner.commit(edit, &mut f.recorder).unwrap();
        let mut observer = f
            .owner
            .open_invocation_observation_with_receipts_v10(
                &id,
                &c,
                &observation::policy(&c, 7),
                limits(),
                InvocationReceiptAdmissionV1 {
                    observation: &observation::facts(&c),
                    capabilities: caps(),
                    operation_receipt_admission_allowed: Some(true),
                },
                &mut f.recorder,
            )
            .unwrap();
        for permission in [Some(false), None] {
            assert_eq!(
                query(
                    &f.owner,
                    &mut observer,
                    &c,
                    InvocationReceiptRequestV10::SourceSaveAs(&release(&c)),
                    permission,
                    &mut f.recorder
                )
                .err(),
                Some(InvocationError::Unavailable)
            );
        }
        let mut denied = release(&c);
        denied.historical_document_readable = None;
        assert_eq!(
            query(
                &f.owner,
                &mut observer,
                &c,
                InvocationReceiptRequestV10::SourceSaveAs(&denied),
                Some(true),
                &mut f.recorder
            )
            .err(),
            Some(InvocationError::Unavailable)
        );
        let InvocationReceiptV10::SourceSaveAs(result) = query(
            &f.owner,
            &mut observer,
            &c,
            InvocationReceiptRequestV10::SourceSaveAs(&release(&c)),
            Some(true),
            &mut f.recorder,
        )
        .unwrap() else {
            panic!()
        };
        assert_eq!(result.invocation, id);
        assert_eq!(result.document, d.document);
        assert!(result.changed);
        let control = super::saveas_invocations::release_control(&c);
        f.owner
            .retire_invocation_observation_v1(&mut observer, &control, &mut f.recorder)
            .unwrap();
        f.owner
            .forget_invocation_outcome(&id, &mut f.recorder)
            .unwrap();
        f.owner
            .retire_source_save_as_v1(&id, &control, &mut f.recorder)
            .unwrap();
        f.owner
            .retire_invocation_observation_policy_v1(&id, &control, &mut f.recorder)
            .unwrap();
        f.owner.retire_invocation(&id, &mut f.recorder).unwrap();
        assert_eq!(f.owner.retention_usage(), baseline);
        if capacity == 256 {
            let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
            assert!(
                records
                    .iter()
                    .any(|v| v.boundary == Boundary::InvocationReceiptRelease
                        && v.service_time.is_some())
            );
            assert!(!format!("{records:?}").contains("PRIVATE-OUT"));
        }
    }
}
#[test]
fn saveas_f06_requires_v10_original_capability_and_full_envelope() {
    let (mut f, _, _, _, auth, request) = ready();
    let c = context();
    let native = activation_facts();
    let grant = authority_facts();
    let current = TrustedSourceSaveAsInvocationFactsV1 {
        context: &c,
        native: &native,
        authority: &grant,
        interpretation_current: Some(true),
    };
    let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
    observation::Profile {
        epoch: 7,
        maximum: 256,
        capabilities: caps(),
    }
    .bind(&mut f.owner, &id, &c, &mut f.recorder);
    f.owner
        .submit_source_save_as_v1(&id, &auth, &request, &admission(&current), &mut f.recorder)
        .unwrap();
    f.owner
        .run_source_save_as_v1(&id, &current, &mut f.recorder)
        .unwrap();
    let mut short = limits();
    short.max_reply_bytes -= 1;
    short.max_held_bytes = short.required_hold_bytes().unwrap();
    let before = f.owner.retention_usage();
    assert_eq!(
        f.owner
            .open_invocation_observation_with_receipts_v10(
                &id,
                &c,
                &observation::policy(&c, 7),
                short,
                InvocationReceiptAdmissionV1 {
                    observation: &observation::facts(&c),
                    capabilities: caps(),
                    operation_receipt_admission_allowed: Some(true)
                },
                &mut f.recorder
            )
            .err(),
        Some(InvocationError::Limit)
    );
    assert_eq!(before, f.owner.retention_usage());
    assert_eq!(
        InvocationReceiptV10::required_reply_bytes(usize::MAX),
        Err(InvocationError::Limit)
    );
    let mut old = f
        .owner
        .open_invocation_observation_with_receipts_v9(
            &id,
            &c,
            &observation::policy(&c, 7),
            limits(),
            InvocationReceiptAdmissionV1 {
                observation: &observation::facts(&c),
                capabilities: caps(),
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
            InvocationReceiptRequestV10::SourceSaveAs(&release(&c)),
            Some(true),
            &mut f.recorder
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
    let mut masked = f
        .owner
        .open_invocation_observation_with_receipts_v10(
            &id,
            &c,
            &observation::policy(&c, 7),
            limits(),
            InvocationReceiptAdmissionV1 {
                observation: &observation::facts(&c),
                capabilities: Default::default(),
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
            InvocationReceiptRequestV10::SourceSaveAs(&release(&c)),
            Some(true),
            &mut f.recorder
        )
        .err(),
        Some(InvocationError::Unavailable)
    );
}
