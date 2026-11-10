use super::*;
fn scope() -> DiscoveryScopeV1 {
    let mut context = producer::context();
    context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into();
    DiscoveryScopeV1 {
        context,
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
    o.prepare_native_retained_release_discovery_cut_v1(&scope(), &visible(), &discovery::facts(), r)
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
#[test]
fn all_three_installed_retained_releases_keep_native_rights_replay_f06_and_protected_cleanup() {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for capacity in [0, 1, 512] {
        let mut f = producer::disposed_with_retention_budget(33554432);
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
        let c = producer::context();
        let current = producer::facts(&c);
        for operation in [
            RetainedReleaseOperationV1::SourceBinding,
            RetainedReleaseOperationV1::AnnotationBinding,
            RetainedReleaseOperationV1::Provenance,
        ] {
            let q = producer::request(&mut f, operation);
            let id = f.owner.issue_invocation(&mut f.recorder).unwrap();
            let before = f.owner.retention_usage();
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
            let mut denied = producer::admission(&current);
            denied.result_admission_allowed = None;
            let p = prepare(&mut f.owner, &mut f.recorder);
            assert_eq!(
                f.owner.submit_discovered_retained_release_v1(
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
                    &denied,
                    &mut f.recorder
                ),
                Err(DiscoveredNativeCommandErrorV1::Invocation(
                    InvocationError::Unavailable
                ))
            );
            assert_eq!(
                f.owner.invocation_state(&id, &mut f.recorder),
                Ok(InvocationState::Issued)
            );
            let p = prepare(&mut f.owner, &mut f.recorder);
            f.owner
                .submit_discovered_retained_release_v1(
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
                    &producer::admission(&current),
                    &mut f.recorder,
                )
                .unwrap();
            assert_eq!(
                f.owner.run(&id, &mut f.recorder),
                Err(InvocationError::Unavailable)
            );
            let mut changed = q.clone();
            changed.operation = if operation == RetainedReleaseOperationV1::SourceBinding {
                RetainedReleaseOperationV1::Provenance
            } else {
                RetainedReleaseOperationV1::SourceBinding
            };
            let p = prepare(&mut f.owner, &mut f.recorder);
            assert_eq!(
                f.owner.submit_discovered_retained_release_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &scope(),
                        cut: &p,
                        entry: &entry,
                        variant: &variant,
                        facts: &discovery::facts(),
                    },
                    &id,
                    &changed,
                    &producer::admission(&current),
                    &mut f.recorder
                ),
                Err(DiscoveredNativeCommandErrorV1::Invocation(
                    InvocationError::IntentMismatch
                ))
            );
            let mut stale_rights = producer::facts(&c);
            stale_rights.release_allowed = None;
            assert_eq!(
                f.owner
                    .run_retained_release_v1(&id, &stale_rights, &mut f.recorder),
                Err(InvocationError::Unavailable)
            );
            f.owner
                .run_retained_release_v1(&id, &current, &mut f.recorder)
                .unwrap();
            let generation = f.owner.generation();
            let p = prepare(&mut f.owner, &mut f.recorder);
            assert_eq!(
                f.owner.submit_discovered_retained_release_v1(
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
                    &producer::admission(&current),
                    &mut f.recorder
                ),
                Ok(InvocationState::Finished)
            );
            assert_eq!(f.owner.generation(), generation);
            let mut observer = f
                .owner
                .open_invocation_observation_with_receipts_v3(
                    &id,
                    &c,
                    &policy(&c),
                    limits(),
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
            let mut withheld = producer::release(&c);
            withheld.result_allowed = None;
            assert!(
                query_with(
                    &f.owner,
                    &mut observer,
                    &c,
                    &withheld,
                    &policy(&c),
                    Some(true),
                    &mut f.recorder
                )
                .is_err()
            );
            let InvocationReceiptV3::RetainedRelease(result) =
                query(&f.owner, &mut observer, &c, &mut f.recorder).unwrap()
            else {
                panic!()
            };
            assert_eq!(result.invocation, id);
            assert_eq!(result.operation, operation);
            assert_eq!(result.document, f.doc.document);
            assert!(result.changed);
            f.owner
                .forget_invocation_outcome(&id, &mut f.recorder)
                .unwrap();
            assert!(query(&f.owner, &mut observer, &c, &mut f.recorder).is_err());
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
                .retire_retained_release_v1(&id, &control, &mut f.recorder)
                .unwrap();
            let released = f.owner.retention_usage();
            assert_eq!(
                f.owner
                    .retire_retained_release_v1(&id, &control, &mut f.recorder),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(f.owner.retention_usage(), released);
            f.owner
                .retire_invocation_observation_policy_v1(&id, &control, &mut f.recorder)
                .unwrap();
            f.owner.retire_invocation(&id, &mut f.recorder).unwrap();
            assert_eq!(f.owner.retention_usage(), before);
        }
        assert_eq!(
            f.owner
                .lifecycle_state(&f.doc.document, &mut f.recorder)
                .unwrap()
                .phase,
            LifecyclePhase::ProvenanceReleased
        );
        if capacity == 512 {
            let events: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
            for boundary in [
                Boundary::DiscoveryBuiltinPreparation,
                Boundary::RetainedReleaseAdmission,
                Boundary::RetainedReleaseService,
                Boundary::RetainedReleaseResultRelease,
            ] {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == boundary && e.service_time.is_some()),
                    "{boundary:?}"
                );
            }
        }
    }
}
