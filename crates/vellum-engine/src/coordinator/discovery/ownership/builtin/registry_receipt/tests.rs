use super::*;
use crate::capture::{CapturedValueV1 as C, InertCaptureV1};
use crate::coordinator::discovery::ownership::tests::{facts, limits};
use crate::coordinator::resource::catalog::observation::{
    registry_receipt_test_certificate, registry_receipt_test_facts, registry_receipt_test_setup,
};

fn visibility() -> NativeBuiltinEntryVisibilityV1<1> {
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
fn prepare(
    o: &mut SourceCoordinator,
    s: &DiscoveryScopeV1,
    installed: bool,
    r: &mut Recorder,
) -> DiscoveryCutIdV1 {
    if installed {
        return o
            .prepare_native_registry_receipt_discovery_cut_v1(s, &visibility(), &facts(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        "F06-registry-receipt",
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
    DiscoveryVariantIdV1,
    Arc<InertCaptureV1>,
) {
    let mut l = limits();
    l.max_context_bytes = 256;
    l.max_row_bytes = 16384;
    l.max_capture_bytes = 16384;
    l.max_packet_bytes = 32768;
    l.max_reply_bytes = 32768;
    l.max_issued = 64;
    l.max_held_bytes = 1048576;
    l.metadata.max_bytes = 16384;
    l.metadata.max_nodes = 512;
    l.metadata.max_depth = 12;
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
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, DiscoveryExecutionKindV1::Query);
    let entry = entries.remove(0).reference;
    let p = prepare(o, s, installed, r);
    let DiscoveryReplyV1::Descriptor { mut descriptor, .. } = o
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
    let variant = descriptor.variants.remove(0);
    (context, entry, variant.reference, variant.schema.unwrap())
}
fn envelope<T>(
    route: &str,
    command: &ResourceRegistryCommandIdV1,
    context: &ResourceRegistryReceiptContextIdV1,
    run: impl FnOnce(&ResourceRegistryReceiptEnvelopeValueV1<'_, '_>) -> T,
) -> T {
    use ResourceRegistryReceiptEnvelopeValueV1 as V;
    let facet = [("kind", V::Data(D::Text("registry-baseline")))];
    let args = [
        (
            "contract",
            V::Data(D::Text(RESOURCE_REGISTRY_RECEIPT_PROFILE_V1)),
        ),
        ("operation", V::Data(D::Text("retained-outcome"))),
        ("command", V::Command(command)),
        ("context", V::Context(context)),
        (
            "profile",
            V::Data(D::Text(RESOURCE_REGISTRY_OBSERVATION_PROFILE_V1)),
        ),
        ("facet", V::Object(&facet)),
    ];
    let header = [
        (
            if route == "typed" { "family" } else { "name" },
            if route == "typed" {
                V::Data(D::Number(5.0))
            } else {
                V::Data(D::Text("inspect-execution"))
            },
        ),
        ("variant", V::Data(D::Text("retained-outcome"))),
        ("version", V::Data(D::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    let outer = [
        ("route", V::Data(D::Text(route))),
        ("request", V::Object(&header)),
    ];
    run(&V::Object(&outer))
}
fn object_field<'a>(c: &'a InertCaptureV1, node: &'a C, key: &str) -> &'a C {
    let C::Object(rows) = node else {
        panic!("expected metadata object")
    };
    c.node(rows.iter().find(|(k, _)| k == key).unwrap().1)
        .unwrap()
}

#[test]
fn discovered_registry_receipt_executes_both_routes_with_exact_schema_and_optional_capture() {
    use crate::instrumentation::NativeTimingProfileV1;
    for capacity in [0, 1, 128] {
        let (mut o, mut setup_r, s, cmd, result) = registry_receipt_test_setup();
        let ds = DiscoveryScopeV1 {
            context: SettingsExecutionContextV1 {
                profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                ..s.context.clone()
            },
            policy_epoch: 1,
        };
        let (context, entry, variant, schema) = install(&mut o, &ds, true, &mut setup_r);
        let requirements = object_field(&schema, schema.root(), "requirements");
        let json = object_field(&schema, requirements, "schema");
        assert!(matches!(
            object_field(&schema, json, "additionalProperties"),
            C::Boolean(false)
        ));
        let C::Array(required) = object_field(&schema, json, "required") else {
            panic!()
        };
        assert_eq!(required.len(), 6);
        let props = object_field(&schema, json, "properties");
        assert!(
            matches!(object_field(&schema, object_field(&schema, props, "command"), "type"), C::Text(s) if s == "object")
        );
        let facet = object_field(&schema, props, "facet");
        assert!(matches!(
            object_field(&schema, facet, "additionalProperties"),
            C::Boolean(false)
        ));
        let f =
            crate::coordinator::resource::catalog::observation::registry_observation_test_facts(&s);
        let rf = registry_receipt_test_facts(&f);
        let receipt_context = o
            .open_resource_registry_receipt_v1(
                &cmd,
                &s,
                ResourceRegistryReceiptCeilingV1 { allowed: true },
                &rf,
                Some(true),
                &mut setup_r,
            )
            .unwrap();
        let root = o.native_root_identity().unwrap();
        let debt = o.retention_usage();
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        for route in ["typed", "dynamic"] {
            let packet = registry_receipt_test_certificate(
                &mut o,
                &receipt_context,
                &cmd,
                &s,
                &result,
                (true, true),
                &mut r,
            );
            let cut = prepare(&mut o, &ds, true, &mut r);
            let port = ResourceRegistryReceiptPortV1 {
                command: &cmd,
                context: &receipt_context,
                scope: &s,
                preparation: &packet,
                current: &rf,
            };
            let reply = envelope(route, &cmd, &receipt_context, |envelope| {
                o.release_discovered_registry_receipt_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &context,
                        scope: &ds,
                        cut: &cut,
                        entry: &entry,
                        variant: &variant,
                        facts: &facts(),
                    },
                    envelope,
                    &port,
                    &mut r,
                )
            })
            .unwrap();
            assert!(Arc::ptr_eq(&reply.receipt, &result));
        }
        assert_eq!(o.native_root_identity().unwrap(), root);
        assert_eq!(o.retention_usage(), debt);
        assert_eq!(
            o.resource_registry_receipt_usage_v1().unwrap().certificates,
            0
        );
        if capacity == 128 {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::DiscoveryRegistryReceiptDispatch,
                Boundary::DiscoveryRequestRecognition,
                Boundary::DiscoveryQuery,
                Boundary::RegistryReceiptArgumentNormalization,
                Boundary::RegistryReceiptRelease,
                Boundary::RegistryReceiptFacade,
            ] {
                assert!(events.iter().any(|e| e.boundary == boundary));
            }
        }
    }
}

#[test]
fn discovered_registry_receipt_preserves_native_certificate_across_independent_denials() {
    let (mut o, mut r, s, cmd, result) = registry_receipt_test_setup();
    let ds = DiscoveryScopeV1 {
        context: SettingsExecutionContextV1 {
            profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
            ..s.context.clone()
        },
        policy_epoch: 1,
    };
    let (context, entry, variant, _) = install(&mut o, &ds, true, &mut r);
    let f = crate::coordinator::resource::catalog::observation::registry_observation_test_facts(&s);
    let rf = registry_receipt_test_facts(&f);
    let rc = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let other = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let packet =
        registry_receipt_test_certificate(&mut o, &rc, &cmd, &s, &result, (true, true), &mut r);
    let mut lost = facts();
    lost.authorized = Some(false);
    let mut denied = registry_receipt_test_facts(&f);
    denied.receipt_allowed = Some(false);
    let port = ResourceRegistryReceiptPortV1 {
        command: &cmd,
        context: &rc,
        scope: &s,
        preparation: &packet,
        current: &denied,
    };
    let p = prepare(&mut o, &ds, true, &mut r);
    assert!(matches!(
        envelope("typed", &cmd, &other, |e| o
            .release_discovered_registry_receipt_v1(
                NativeDiscoveredCommandRouteV1 {
                    context: &context,
                    scope: &ds,
                    cut: &p,
                    entry: &entry,
                    variant: &variant,
                    facts: &lost
                },
                e,
                &port,
                &mut r
            )),
        Err(DiscoveredRegistryReceiptErrorV1::Native(
            ResourceError::Unavailable
        ))
    ));
    assert!(!o.discovery_context(&context).unwrap().stopped);
    assert!(matches!(
        envelope("typed", &cmd, &rc, |e| o
            .release_discovered_registry_receipt_v1(
                NativeDiscoveredCommandRouteV1 {
                    context: &context,
                    scope: &ds,
                    cut: &p,
                    entry: &entry,
                    variant: &variant,
                    facts: &facts()
                },
                e,
                &port,
                &mut r
            )),
        Err(DiscoveredRegistryReceiptErrorV1::Native(
            ResourceError::Unavailable
        ))
    ));
    assert_eq!(
        o.resource_registry_receipt_usage_v1().unwrap().certificates,
        1
    );
    assert_eq!(o.resource_registry_receipt_usage_v1().unwrap().stopped, 0);
    let p = prepare(&mut o, &ds, true, &mut r);
    assert!(matches!(
        envelope("dynamic", &cmd, &rc, |e| o
            .release_discovered_registry_receipt_v1(
                NativeDiscoveredCommandRouteV1 {
                    context: &context,
                    scope: &ds,
                    cut: &p,
                    entry: &entry,
                    variant: &variant,
                    facts: &lost
                },
                e,
                &port,
                &mut r
            )),
        Err(DiscoveredRegistryReceiptErrorV1::Discovery(
            Error::Unavailable
        ))
    ));
    assert!(o.discovery_context(&context).unwrap().stopped);
    assert_eq!(o.resource_registry_receipt_usage_v1().unwrap().stopped, 0);
    assert!(
        o.release_resource_registry_receipt_v1(&rc, &cmd, &s, &packet, &rf, &mut r)
            .is_ok()
    );
}

#[test]
fn discovered_registry_receipt_source_requires_the_actual_native_parent_correspondence() {
    let (mut o, mut r, s, _, _) = registry_receipt_test_setup();
    let ds = DiscoveryScopeV1 {
        context: SettingsExecutionContextV1 {
            authority: "other actual host".into(),
            profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
            ..s.context.clone()
        },
        policy_epoch: 1,
    };
    o.configure_discovery_v1(&ds, limits(), &facts(), &mut r)
        .unwrap();
    let before = o.discovery_usage_v1().unwrap();
    assert_eq!(
        o.prepare_native_registry_receipt_discovery_cut_v1(&ds, &visibility(), &facts(), &mut r)
            .err(),
        Some(Error::Unavailable)
    );
    assert_eq!(o.discovery_usage_v1().unwrap(), before);
}

#[test]
fn copied_registry_receipt_metadata_does_not_gain_installed_origin_or_apply_loss() {
    let (mut o, mut r, s, cmd, result) = registry_receipt_test_setup();
    let ds = DiscoveryScopeV1 {
        context: SettingsExecutionContextV1 {
            profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
            ..s.context.clone()
        },
        policy_epoch: 1,
    };
    let (context, entry, variant, _) = install(&mut o, &ds, false, &mut r);
    let f = crate::coordinator::resource::catalog::observation::registry_observation_test_facts(&s);
    let rf = registry_receipt_test_facts(&f);
    let rc = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let packet =
        registry_receipt_test_certificate(&mut o, &rc, &cmd, &s, &result, (true, true), &mut r);
    let p = prepare(&mut o, &ds, false, &mut r);
    let mut lost = facts();
    lost.authorized = Some(false);
    let port = ResourceRegistryReceiptPortV1 {
        command: &cmd,
        context: &rc,
        scope: &s,
        preparation: &packet,
        current: &rf,
    };
    assert!(matches!(
        envelope("typed", &cmd, &rc, |e| o
            .release_discovered_registry_receipt_v1(
                NativeDiscoveredCommandRouteV1 {
                    context: &context,
                    scope: &ds,
                    cut: &p,
                    entry: &entry,
                    variant: &variant,
                    facts: &lost
                },
                e,
                &port,
                &mut r
            )),
        Err(DiscoveredRegistryReceiptErrorV1::Discovery(
            Error::Unavailable
        ))
    ));
    assert!(!o.discovery_context(&context).unwrap().stopped);
    assert_eq!(
        o.resource_registry_receipt_usage_v1().unwrap().certificates,
        1
    );
    assert!(
        o.release_resource_registry_receipt_v1(&rc, &cmd, &s, &packet, &rf, &mut r)
            .is_ok()
    );
}
