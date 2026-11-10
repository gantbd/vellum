use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits};
use crate::coordinator::resource::catalog::observation::{
    registry_observation_test_facts, registry_receipt_test_certificate,
    registry_receipt_test_facts, registry_receipt_test_setup,
};
fn visibility() -> NativeBuiltinEntryVisibilityV1<2> {
    NativeBuiltinEntryVisibilityV1 {
        order: Some(0),
        schema: true,
        presentation: false,
        advice: false,
        variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
            order: Some(i),
            enabled: None,
        }),
    }
}
fn prepare(
    o: &mut SourceCoordinator,
    s: &DiscoveryScopeV1,
    installed: bool,
    r: &mut Recorder,
) -> DiscoveryCutIdV1 {
    if installed {
        return o
            .prepare_native_registry_command_discovery_cut_v1(s, &visibility(), &facts(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        "F19-registry-command",
        store.limits.max_context_bytes,
    )
    .unwrap();
    with_input(&authority, &command, &visibility(), |i| {
        o.prepare_discovery_cut_v1(s, i, &facts(), r)
    })
    .unwrap()
}
fn install(
    o: &mut SourceCoordinator,
    s: &DiscoveryScopeV1,
    installed: bool,
    r: &mut Recorder,
) -> (
    DiscoveryContextIdV1,
    DiscoveryEntryIdV1,
    Vec<DiscoveryVariantIdV1>,
) {
    let mut l = limits();
    l.max_context_bytes = 256;
    l.max_entries = 1;
    l.max_variants = 2;
    l.max_row_bytes = 32768;
    l.max_capture_bytes = 32768;
    l.max_packet_bytes = 65536;
    l.max_reply_bytes = 65536;
    l.max_selectors = 32;
    l.max_issued = 128;
    l.metadata.max_bytes = 16384;
    l.metadata.max_nodes = 512;
    l.metadata.max_depth = 12;
    l.max_held_bytes = 1048576;
    o.configure_discovery_v1(s, l, &facts(), r).unwrap();
    let p = prepare(o, s, installed, r);
    let context = o.acquire_discovery_v1(s, &p, &facts(), r).unwrap().context;
    let p = prepare(o, s, installed, r);
    let DiscoveryReplyV1::Page { mut entries, .. } = o
        .query_discovery_v1(
            &context,
            s,
            &p,
            DiscoveryQueryV1::List {
                limit: 1,
                continuation: None,
            },
            &facts(),
            r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(entries[0].kind, DiscoveryExecutionKindV1::LocalCommand);
    let entry = entries.remove(0).reference;
    let p = prepare(o, s, installed, r);
    let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
        .query_discovery_v1(
            &context,
            s,
            &p,
            DiscoveryQueryV1::Inspect { entry: &entry },
            &facts(),
            r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(descriptor.variants.len(), 2);
    assert!(descriptor.variants.iter().all(|v| v.schema.is_some()));
    (
        context,
        entry,
        descriptor
            .variants
            .into_iter()
            .map(|v| v.reference)
            .collect(),
    )
}
fn snapshot(
    o: &mut SourceCoordinator,
    id: &ResourceRegistryCommandIdV1,
    original: &ResourceRegistryCommandIdV1,
    s: &ResourceRegistryObservationScopeV1,
    r: &mut Recorder,
) -> ResourceRegistryPreparationIdV1 {
    let addresses: Vec<_> = o
        .resource_registry_command_facts_v1(original, s, r)
        .unwrap()
        .historical_slots
        .iter()
        .map(|slot| slot.address.copy().unwrap())
        .collect();
    let rows: Vec<_> = addresses
        .iter()
        .enumerate()
        .map(|(i, address)| ResourceRegistrySnapshotRowFactsV1 {
            address,
            captured_visible: true,
            current_visible: true,
            captured_descriptor: true,
            current_descriptor: true,
            visible_order: Some(i),
        })
        .collect();
    let f = registry_observation_test_facts(s);
    o.prepare_resource_registry_command_snapshot_v1(
        id,
        s,
        &TrustedResourceRegistrySnapshotFactsV1 {
            current: &f,
            rows: &rows,
            capture_policy_current: Some(true),
            visibility_complete: Some(true),
            order_attested: Some(true),
        },
        r,
    )
    .unwrap()
}
fn envelope<T>(
    typed: bool,
    operation: &ResourceRegistryCommandOperationV1,
    id: &ResourceRegistryCommandIdV1,
    prep: Option<&ResourceRegistryPreparationIdV1>,
    call: impl FnOnce(&ResourceRegistryCommandEnvelopeValueV1<'_, '_>) -> T,
) -> T {
    use ResourceRegistryCommandEnvelopeValueV1 as V;
    let (name, stream) = match operation {
        ResourceRegistryCommandOperationV1::Establish => {
            ("establish-bounded-stream", V::Data(D::Null))
        }
        ResourceRegistryCommandOperationV1::Resynchronize { stream } => {
            ("authorized-resynchronization", V::Stream(stream))
        }
    };
    let args = [
        (
            "contract",
            V::Data(D::Text(RESOURCE_REGISTRY_COMMAND_PROFILE_V1)),
        ),
        ("operation", V::Data(D::Text(name))),
        ("command", V::Command(id)),
        ("stream", stream),
        ("preparation", prep.map_or(V::Data(D::Null), V::Preparation)),
    ];
    let header = [
        (
            if typed { "family" } else { "name" },
            if typed {
                V::Data(D::Number(18.0))
            } else {
                V::Data(D::Text("control-observation"))
            },
        ),
        ("variant", V::Data(D::Text(name))),
        ("version", V::Data(D::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    let outer = [
        (
            "route",
            V::Data(D::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&header)),
    ];
    call(&V::Object(&outer))
}

#[test]
fn discovered_registry_command_both_forms_routes_join_original_results_and_protected_replay() {
    use crate::instrumentation::NativeTimingProfileV1;
    for resync in [false, true] {
        for typed in [false, true] {
            for capacity in [0, 1, 128] {
                let (mut o, mut setup, s, original, first) = registry_receipt_test_setup();
                let ds = DiscoveryScopeV1 {
                    context: SettingsExecutionContextV1 {
                        profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                        ..s.context.clone()
                    },
                    policy_epoch: 1,
                };
                let (context, entry, variants) = install(&mut o, &ds, true, &mut setup);
                let op = if resync {
                    ResourceRegistryCommandOperationV1::Resynchronize {
                        stream: first.stream.clone(),
                    }
                } else {
                    ResourceRegistryCommandOperationV1::Establish
                };
                let f = registry_observation_test_facts(&s);
                let id = o
                    .issue_resource_registry_command_v1(
                        &op,
                        &s,
                        &TrustedResourceRegistryCommandAdmissionV1 {
                            current: &f,
                            owner_control_allowed: Some(true),
                            baseline_allowed: Some(true),
                        },
                        &mut setup,
                    )
                    .unwrap();
                let p = snapshot(&mut o, &id, &original, &s, &mut setup);
                let mut r = Recorder::try_with_capacity(capacity)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                let cut = prepare(&mut o, &ds, true, &mut r);
                let q = ResourceRegistryCommandRequestV1 {
                    operation: &op,
                    scope: &s,
                    preparation: Some(&p),
                    current: Some(&f),
                    owner_control_allowed: Some(true),
                    baseline_allowed: Some(true),
                };
                let result = envelope(typed, &op, &id, Some(&p), |e| {
                    o.request_discovered_registry_command_v1(
                        NativeDiscoveredCommandRouteV1 {
                            context: &context,
                            scope: &ds,
                            cut: &cut,
                            entry: &entry,
                            variant: &variants[usize::from(resync)],
                            facts: &facts(),
                        },
                        e,
                        &ResourceRegistryCommandPortV1 {
                            command: &id,
                            request: &q,
                        },
                        &mut r,
                    )
                })
                .unwrap();
                assert_eq!(
                    result,
                    ResourceRegistryCommandReplyV1::Settled {
                        command: id.clone()
                    }
                );
                let baseline = o
                    .resource_registry_command_facts_v1(&id, &s, &mut setup)
                    .unwrap()
                    .as_observed
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .clone();
                let rf = registry_receipt_test_facts(&f);
                let rc = o
                    .open_resource_registry_receipt_v1(
                        &id,
                        &s,
                        ResourceRegistryReceiptCeilingV1 { allowed: true },
                        &rf,
                        Some(true),
                        &mut setup,
                    )
                    .unwrap();
                let cert = registry_receipt_test_certificate(
                    &mut o,
                    &rc,
                    &id,
                    &s,
                    &baseline,
                    (true, true),
                    &mut setup,
                );
                assert!(Arc::ptr_eq(
                    &baseline,
                    &o.release_resource_registry_receipt_v1(&rc, &id, &s, &cert, &rf, &mut setup)
                        .unwrap()
                        .receipt
                ));
                o.faulted = true;
                let before = o.resource_registry_observation_usage_v1().unwrap();
                let retry = ResourceRegistryCommandRequestV1 {
                    operation: &op,
                    scope: &s,
                    preparation: None,
                    current: None,
                    owner_control_allowed: Some(true),
                    baseline_allowed: None,
                };
                assert_eq!(
                    envelope(!typed, &op, &id, None, |e| o
                        .request_decoded_resource_registry_command_v1(
                            e,
                            &ResourceRegistryCommandPortV1 {
                                command: &id,
                                request: &retry
                            },
                            &mut r
                        ))
                    .unwrap(),
                    ResourceRegistryCommandReplyV1::ObserveExisting {
                        command: id.clone()
                    }
                );
                assert_eq!(o.resource_registry_observation_usage_v1().unwrap(), before);
                if capacity == 128 {
                    let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                    assert!(
                        events
                            .iter()
                            .any(|e| e.boundary == Boundary::DiscoveryRegistryCommandDispatch
                                && e.service_time.is_some())
                    );
                }
            }
        }
    }
}

#[test]
fn discovered_registry_command_copied_metadata_and_cross_variant_cannot_execute_or_apply_loss() {
    for installed in [false, true] {
        let (mut o, mut r, s, original, _) = registry_receipt_test_setup();
        let ds = DiscoveryScopeV1 {
            context: SettingsExecutionContextV1 {
                profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                ..s.context.clone()
            },
            policy_epoch: 1,
        };
        let (context, entry, variants) = install(&mut o, &ds, installed, &mut r);
        let op = ResourceRegistryCommandOperationV1::Establish;
        let f = registry_observation_test_facts(&s);
        let id = o
            .issue_resource_registry_command_v1(
                &op,
                &s,
                &TrustedResourceRegistryCommandAdmissionV1 {
                    current: &f,
                    owner_control_allowed: Some(true),
                    baseline_allowed: Some(true),
                },
                &mut r,
            )
            .unwrap();
        let p = snapshot(&mut o, &id, &original, &s, &mut r);
        let q = ResourceRegistryCommandRequestV1 {
            operation: &op,
            scope: &s,
            preparation: Some(&p),
            current: Some(&f),
            owner_control_allowed: Some(true),
            baseline_allowed: Some(true),
        };
        let cut = prepare(&mut o, &ds, installed, &mut r);
        let mut lost = facts();
        lost.authorized = Some(false);
        let result = envelope(true, &op, &id, Some(&p), |e| {
            o.request_discovered_registry_command_v1(
                NativeDiscoveredCommandRouteV1 {
                    context: &context,
                    scope: &ds,
                    cut: &cut,
                    entry: &entry,
                    variant: &variants[usize::from(installed)],
                    facts: &lost,
                },
                e,
                &ResourceRegistryCommandPortV1 {
                    command: &id,
                    request: &q,
                },
                &mut r,
            )
        });
        assert_eq!(
            result.err(),
            Some(DiscoveredRegistryCommandErrorV1::Discovery(
                Error::Unavailable
            ))
        );
        assert!(!o.discovery_context(&context).unwrap().stopped);
        assert!(
            !o.resource_registry_command_facts_v1(&id, &s, &mut r)
                .unwrap()
                .native_started
        );
    }
}
