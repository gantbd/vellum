use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits};
use crate::coordinator::observation_control::v4::tests::{collection, facet, request, source};
use crate::instrumentation::NativeTimingProfileV1;

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
            .prepare_native_observation_control_discovery_cut_v1(s, &visibility(), &facts(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        "F19-observation-control-v4",
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
    l.max_capture_bytes = 65536;
    l.max_packet_bytes = 65536;
    l.max_reply_bytes = 65536;
    l.max_selectors = 64;
    l.max_issued = 256;
    l.metadata.max_bytes = 16384;
    l.metadata.max_nodes = 1024;
    l.metadata.max_depth = 12;
    l.max_held_bytes = l.declared_bytes().unwrap();
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

macro_rules! installed_receipt_control_case {
    ($name:ident, $setup:ident, $variant:ident, $control:ident, $scope:expr, $usage:ident, $stream_usage:ident) => {
        #[test]
        fn $name() {
            for installed in [false, true] {
                for revoke in [false, true] {
                    for typed in [false, true] {
                        for mode in 0..4 {
                            let (mut o, mut setup, s, command, one, _two) = $setup();
                            let ds = DiscoveryScopeV1 {
                                context: SettingsExecutionContextV1 {
                                    profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                                    ..($scope)(&s)
                                },
                                policy_epoch: 1,
                            };
                            let (context, entry, variants) =
                                install(&mut o, &ds, installed, &mut setup);
                            let target = ObservationStopTargetV4::$variant {
                                command: &command,
                                context: &one,
                            };
                            let before = o.$stream_usage().unwrap();
                            let debt = o.retention_usage();
                            let operation = if revoke {
                                "revoke-observation-grant"
                            } else {
                                "stop-observation"
                            };
                            let mut r = if mode == 0 {
                                Recorder::disabled()
                            } else {
                                Recorder::try_with_capacity(if mode == 3 { 1 } else { 256 })
                                    .unwrap()
                            };
                            if mode >= 2 {
                                r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                            }
                            let sentinel = Observation {
                                boundary: Boundary::Control,
                                disposition: Disposition::Completed,
                                service_time: None,
                            };
                            if mode == 3 {
                                r.record(sentinel);
                            }
                            for allowed in [false, true, true] {
                                let cut = prepare(&mut o, &ds, installed, &mut setup);
                                let f = facts();
                                let control = $control {
                                    scope: &s,
                                    owner_control_allowed: Some(if revoke {
                                        true
                                    } else {
                                        allowed
                                    }),
                                    retention_released: None,
                                    delivery_drained: None,
                                };
                                let authority = TrustedObservationRevocationV2 {
                                    original_target: target,
                                    revocation_allowed: Some(allowed),
                                };
                                let result = request(target, typed, operation, |e| {
                                    let route = NativeDiscoveredCommandRouteV1 {
                                        context: &context,
                                        scope: &ds,
                                        cut: &cut,
                                        entry: &entry,
                                        variant: &variants[usize::from(revoke)],
                                        facts: &f,
                                    };
                                    let port = ObservationStopPortV4::$variant {
                                        command: &command,
                                        context: &one,
                                        control: &control,
                                    };
                                    if revoke {
                                        o.revoke_discovered_observation_context_v2(
                                            route, e, port, &authority, &mut r,
                                        )
                                    } else {
                                        o.stop_discovered_observation_context_v4(
                                            route, e, port, &mut r,
                                        )
                                    }
                                });
                                if !installed {
                                    assert_eq!(
                                        result,
                                        Err(DiscoveredObservationControlErrorV1::Discovery(
                                            Error::Unavailable
                                        ))
                                    );
                                    o.retire_discovery_cut_v1(&cut, &f, &mut setup).unwrap();
                                } else {
                                    assert_eq!(result.is_ok(), allowed);
                                }
                                assert_eq!(
                                    o.$usage().unwrap().stopped,
                                    usize::from(installed && allowed)
                                );
                                assert_eq!(o.$stream_usage().unwrap(), before);
                                assert_eq!(o.retention_usage(), debt);
                            }
                            // A failed ordinary discovery release cannot disable direct control.
                            let cut = prepare(&mut o, &ds, installed, &mut setup);
                            let mut f = facts();
                            f.reply_ready = Some(false);
                            let control = $control {
                                scope: &s,
                                owner_control_allowed: Some(true),
                                retention_released: None,
                                delivery_drained: None,
                            };
                            request(target, typed, "stop-observation", |e| {
                                assert!(
                                    o.stop_discovered_observation_context_v4(
                                        NativeDiscoveredCommandRouteV1 {
                                            context: &context,
                                            scope: &ds,
                                            cut: &cut,
                                            entry: &entry,
                                            variant: &variants[0],
                                            facts: &f
                                        },
                                        e,
                                        ObservationStopPortV4::$variant {
                                            command: &command,
                                            context: &one,
                                            control: &control
                                        },
                                        &mut r
                                    )
                                    .is_err()
                                )
                            });
                            assert_eq!(o.$usage().unwrap().stopped, usize::from(installed));
                            o.stop_observation_facade_v4(
                                ObservationStopPortV4::$variant {
                                    command: &command,
                                    context: &one,
                                    control: &control,
                                },
                                &mut setup,
                            )
                            .unwrap();
                            assert_eq!(o.$usage().unwrap().stopped, 1);
                            if mode == 0 {
                                assert_eq!(r.queued(), 0);
                                assert_eq!(r.dropped(), 0);
                            } else if mode == 3 {
                                assert_eq!(r.pop_front(), Some(sentinel));
                                assert!(r.dropped() > 0);
                            } else {
                                let mut ends = Vec::new();
                                while let Some(e) = r.pop_front() {
                                    if e.boundary == Boundary::DiscoveryObservationControlDispatch {
                                        assert_eq!(e.service_time.is_some(), mode == 2);
                                        ends.push(e.disposition);
                                    }
                                }
                                assert_eq!(
                                    ends,
                                    if installed {
                                        vec![
                                            Disposition::Refused,
                                            Disposition::Completed,
                                            Disposition::Completed,
                                            Disposition::Refused,
                                        ]
                                    } else {
                                        vec![Disposition::Refused; 4]
                                    }
                                );
                                assert_eq!(r.dropped(), 0);
                            }
                        }
                    }
                }
            }
        }
    };
}
installed_receipt_control_case!(
    installed_source_receipt_control,
    source,
    SourceReceipt,
    SourceReadContextControlV1,
    |s: &SourceReadScopeV1| s.context.clone(),
    source_publication_receipt_usage_v1,
    source_publication_usage_v1
);
installed_receipt_control_case!(
    installed_facet_receipt_control,
    facet,
    FacetReceipt,
    FacetPublicationControlV1,
    |s: &FacetPublicationScopeV1| s.source.context.clone(),
    facet_publication_receipt_usage_v1,
    facet_publication_usage_v1
);
installed_receipt_control_case!(
    installed_collection_receipt_control,
    collection,
    CollectionReceipt,
    CollectionPublicationControlV1,
    |s: &CollectionPublicationScopeV1| s.source.context.clone(),
    collection_publication_receipt_usage_v1,
    collection_publication_usage_v1
);
