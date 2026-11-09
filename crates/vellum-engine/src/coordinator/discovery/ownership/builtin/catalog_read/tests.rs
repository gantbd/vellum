use super::*;
use crate::coordinator::discovery::ownership::tests::{facts as df, limits as dl};
use crate::coordinator::resource::catalog::read::tests::ingress::{input_limits, request};
use crate::coordinator::resource::catalog::read::tests::{
    acquire, cut, facts, limits, scope, setup_with_parent,
};
use crate::instrumentation::NativeTimingProfileV1;
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
            .prepare_native_catalog_read_discovery_cut_v1(s, &visibility(), &df(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        "F07-resource-catalog-read-v1",
        store.limits.max_context_bytes,
    )
    .unwrap();
    with_input(&authority, &command, &visibility(), |input| {
        o.prepare_discovery_cut_v1(s, input, &df(), r)
    })
    .unwrap()
}
#[test]
fn installed_catalog_capabilities_join_both_routes_all_products_and_independent_release() {
    for maximum in [0, 1, 256, 4096] {
        assert!(
            workspace_bytes(maximum).unwrap()
                <= super::super::context::workspace_bytes(maximum).unwrap()
        );
    }
    for installed in [false, true] {
        for typed in [false, true] {
            for mode in 0..4 {
                let mut discovery = dl();
                discovery.max_context_bytes = 256;
                discovery.max_entries = 1;
                discovery.max_variants = 1;
                discovery.max_row_bytes = 65536;
                discovery.max_capture_bytes = 131072;
                discovery.max_packet_bytes = 131072;
                discovery.max_reply_bytes = 131072;
                discovery.max_selectors = 32;
                discovery.max_issued = 128;
                discovery.metadata.max_bytes = 32768;
                discovery.metadata.max_nodes = 2048;
                discovery.metadata.max_depth = 12;
                discovery.max_held_bytes = discovery.declared_bytes().unwrap();
                let rl = limits();
                let parent = rl.max_pool_bytes
                    + discovery.max_held_bytes
                    + resource_catalog_read_classification_bytes_v1()
                    + 7 * input_limits().max_bytes;
                let (mut o, mut setup) = setup_with_parent(rl, parent);
                o.set_settings_parent_limit_for_test(8192);
                let rs = scope();
                let s = DiscoveryScopeV1 {
                    context: SettingsExecutionContextV1 {
                        profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                        ..rs.context.clone()
                    },
                    policy_epoch: 1,
                };
                o.configure_discovery_v1(&s, discovery, &df(), &mut setup)
                    .unwrap();
                let c = acquire(&mut o, true, &mut setup);
                let p = prepare(&mut o, &s, installed, &mut setup);
                let dc = o
                    .acquire_discovery_v1(&s, &p, &df(), &mut setup)
                    .unwrap()
                    .context;
                let p = prepare(&mut o, &s, installed, &mut setup);
                let DiscoveryReplyV1::Page { mut entries, .. } = o
                    .query_discovery_v1(
                        &dc,
                        &s,
                        &p,
                        DiscoveryQueryV1::List {
                            limit: 1,
                            continuation: None,
                        },
                        &df(),
                        &mut setup,
                    )
                    .unwrap()
                else {
                    panic!()
                };
                let entry = entries.remove(0).reference;
                let p = prepare(&mut o, &s, installed, &mut setup);
                let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
                    .query_discovery_v1(
                        &dc,
                        &s,
                        &p,
                        DiscoveryQueryV1::Inspect { entry: &entry },
                        &df(),
                        &mut setup,
                    )
                    .unwrap()
                else {
                    panic!()
                };
                let variant = descriptor.variants.into_iter().next().unwrap().reference;
                let mut r = if mode == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if mode == 3 { 1 } else { 256 }).unwrap()
                };
                if mode >= 2 {
                    r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                }
                let p = prepare(&mut o, &s, installed, &mut r);
                let current = cut(&mut o, true, false, &mut r);
                let before = (o.retention_usage(), o.generation(), o.resource_usage());
                let f = df();
                let result = request(
                    &c,
                    typed,
                    ResourceCatalogReadQueryV1::List {
                        limit: 1,
                        continuation: None,
                    },
                    |e| {
                        o.query_discovered_resource_catalog_read_v1(
                            NativeDiscoveredCommandRouteV1 {
                                context: &dc,
                                scope: &s,
                                cut: &p,
                                entry: &entry,
                                variant: &variant,
                                facts: &f,
                            },
                            &e,
                            input_limits(),
                            &rs,
                            &current,
                            &facts(),
                            &mut r,
                        )
                    },
                );
                assert_eq!(result.is_ok(), installed);
                assert_eq!(
                    (o.retention_usage(), o.generation(), o.resource_usage()),
                    before
                );
                if let Ok(ResourceCatalogReadReplyV1::Page {
                    context,
                    entries,
                    continuation,
                }) = result
                {
                    assert_eq!(context, c);
                    assert_eq!(entries.len(), 1);
                    assert!(continuation.is_some());
                    for query in [
                        ResourceCatalogReadQueryV1::List {
                            limit: 1,
                            continuation: continuation.as_ref(),
                        },
                        ResourceCatalogReadQueryV1::Inspect {
                            entry: &entries[0].reference,
                        },
                    ] {
                        let p = prepare(&mut o, &s, installed, &mut r);
                        let current = cut(&mut o, true, false, &mut r);
                        let out = request(&c, typed, query, |e| {
                            o.query_discovered_resource_catalog_read_v1(
                                NativeDiscoveredCommandRouteV1 {
                                    context: &dc,
                                    scope: &s,
                                    cut: &p,
                                    entry: &entry,
                                    variant: &variant,
                                    facts: &f,
                                },
                                &e,
                                input_limits(),
                                &rs,
                                &current,
                                &facts(),
                                &mut r,
                            )
                        })
                        .unwrap();
                        assert_eq!(out.coverage(), "captured-permitted");
                        if matches!(query, ResourceCatalogReadQueryV1::Inspect { .. }) {
                            assert!(matches!(
                                out,
                                ResourceCatalogReadReplyV1::Registration {
                                    descriptor: Some(_),
                                    ..
                                }
                            ));
                        }
                    }
                    let p = prepare(&mut o, &s, installed, &mut r);
                    let current = cut(&mut o, true, false, &mut r);
                    let mut denied = f;
                    denied.authorized = Some(false);
                    let mut loss = facts();
                    loss.continuous = Some(false);
                    request(
                        &c,
                        typed,
                        ResourceCatalogReadQueryV1::List {
                            limit: 1,
                            continuation: None,
                        },
                        |e| {
                            assert!(
                                o.query_discovered_resource_catalog_read_v1(
                                    NativeDiscoveredCommandRouteV1 {
                                        context: &dc,
                                        scope: &s,
                                        cut: &p,
                                        entry: &entry,
                                        variant: &variant,
                                        facts: &denied
                                    },
                                    &e,
                                    input_limits(),
                                    &rs,
                                    &current,
                                    &loss,
                                    &mut r
                                )
                                .is_err()
                            )
                        },
                    );
                    o.retire_discovery_cut_v1(&p, &f, &mut r).unwrap();
                    request(
                        &c,
                        typed,
                        ResourceCatalogReadQueryV1::List {
                            limit: 1,
                            continuation: None,
                        },
                        |e| {
                            o.query_decoded_resource_catalog_read_v1(
                                &e,
                                input_limits(),
                                &rs,
                                &current,
                                &facts(),
                                &mut r,
                            )
                        },
                    )
                    .unwrap();
                } else {
                    o.retire_resource_catalog_visibility_v1(&current, &facts(), &mut r)
                        .unwrap();
                    o.retire_discovery_cut_v1(&p, &f, &mut r).unwrap();
                }
                if mode == 3 {
                    assert!(r.dropped() > 0);
                }
                if mode == 1 || mode == 2 {
                    let mut seen = Vec::new();
                    while let Some(record) = r.pop_front() {
                        if [
                            Boundary::ResourceCatalogReadArgumentNormalization,
                            Boundary::ResourceCatalogReadDecodedDispatch,
                            Boundary::DiscoveryCatalogReadDispatch,
                            Boundary::ResourceCatalogReadRelease,
                        ]
                        .contains(&record.boundary)
                        {
                            assert_eq!(record.service_time.is_some(), mode == 2);
                            seen.push(record.boundary);
                        }
                    }
                    assert!(seen.contains(&Boundary::DiscoveryCatalogReadDispatch));
                    if installed {
                        assert!(seen.contains(&Boundary::ResourceCatalogReadDecodedDispatch));
                        assert!(seen.contains(&Boundary::ResourceCatalogReadArgumentNormalization));
                    }
                }
                o.stop_resource_catalog_read_v1(&c, &rs, &facts(), &mut r)
                    .unwrap();
                o.retire_resource_catalog_read_v1(&c, &facts(), &mut r)
                    .unwrap();
                o.retire_resource_catalog_reader_v1(&facts(), &mut r)
                    .unwrap();
                o.stop_discovery_context_v1(&dc, &s, &df(), &mut r).unwrap();
                o.retire_discovery_context_v1(&dc, &df(), &mut r).unwrap();
                o.stop_discovery_admission_v1(&df(), &mut r).unwrap();
                o.retire_discovery_registry_v1(&df(), &mut r).unwrap();
                assert_eq!(o.retention_usage().reserved, 0);
            }
        }
    }
}
