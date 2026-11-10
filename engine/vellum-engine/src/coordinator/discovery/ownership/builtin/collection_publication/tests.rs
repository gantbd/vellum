use super::*;
use crate::coordinator::collection_publication::commands::tests::{
    command_envelope, open_receipt, receipt_envelope, receipt_facts, setup,
};
use crate::coordinator::collection_publication::tests::{facts as stream_facts, packet};
use crate::coordinator::discovery::ownership::tests::{facts, limits};
use crate::instrumentation::NativeTimingProfileV1;
fn visibility() -> NativeCollectionPublicationDiscoveryVisibilityV1 {
    NativeCollectionPublicationDiscoveryVisibilityV1 {
        commands: NativeBuiltinEntryVisibilityV1 {
            order: Some(0),
            schema: true,
            presentation: false,
            advice: false,
            variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
                order: Some(i),
                enabled: None,
            }),
        },
        receipts: NativeBuiltinEntryVisibilityV1 {
            order: Some(1),
            schema: true,
            presentation: false,
            advice: false,
            variants: [NativeBuiltinVariantVisibilityV1 {
                order: Some(0),
                enabled: None,
            }],
        },
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
            .prepare_native_collection_publication_discovery_cut_v1(s, &visibility(), &facts(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let commands = [
        command(&store.engine, "F19-collection-publication", 256).unwrap(),
        command(&store.engine, "F06-collection-publication", 256).unwrap(),
    ];
    with_input(&authority, &commands, &visibility(), |input| {
        o.prepare_discovery_cut_v1(s, input, &facts(), r)
    })
    .unwrap()
}
fn install(
    o: &mut SourceCoordinator,
    source: &SourceReadScopeV1,
    installed: bool,
    r: &mut Recorder,
) -> (
    DiscoveryScopeV1,
    DiscoveryContextIdV1,
    Vec<(DiscoveryEntryIdV1, Vec<DiscoveryVariantIdV1>)>,
) {
    let mut s = DiscoveryScopeV1 {
        context: source.context.clone(),
        policy_epoch: 1,
    };
    s.context.profile = OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into();
    let mut l = limits();
    l.max_context_bytes = 256;
    l.max_entries = 2;
    l.max_variants = 2;
    l.max_capture_bytes = 65536;
    l.max_row_bytes = 32768;
    l.max_packet_bytes = 131072;
    l.max_reply_bytes = 131072;
    l.max_selectors = 64;
    l.max_issued = 256;
    l.metadata.max_bytes = 32768;
    l.metadata.max_nodes = 1024;
    l.metadata.max_depth = 12;
    l.max_held_bytes = l.declared_bytes().unwrap();
    o.configure_discovery_v1(&s, l, &facts(), r).unwrap();
    let p = prepare(o, &s, installed, r);
    let c = o.acquire_discovery_v1(&s, &p, &facts(), r).unwrap().context;
    let p = prepare(o, &s, installed, r);
    let DiscoveryReplyV1::Page { entries, .. } = o
        .query_discovery_v1(
            &c,
            &s,
            &p,
            DiscoveryQueryV1::List {
                limit: 2,
                continuation: None,
            },
            &facts(),
            r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].kind, DiscoveryExecutionKindV1::LocalCommand);
    assert_eq!(entries[1].kind, DiscoveryExecutionKindV1::Query);
    let rows = entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| {
            let p = prepare(o, &s, installed, r);
            let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
                .query_discovery_v1(
                    &c,
                    &s,
                    &p,
                    DiscoveryQueryV1::Inspect {
                        entry: &entry.reference,
                    },
                    &facts(),
                    r,
                )
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(descriptor.variants.len(), if index == 0 { 2 } else { 1 });
            assert!(descriptor.variants.iter().all(|v| v.schema.is_some()));
            (
                entry.reference,
                descriptor
                    .variants
                    .into_iter()
                    .map(|v| v.reference)
                    .collect(),
            )
        })
        .collect();
    (s, c, rows)
}

#[test]
fn installed_collection_commands_consume_only_original_packets_replay_and_deliver_original_receipts()
 {
    for installed in [false, true] {
        for typed in [false, true] {
            for mode in 0..4 {
                let (mut o, mut setup, s, g) = setup();
                let (ds, context, rows) = install(&mut o, &s.source, installed, &mut setup);
                let n = crate::coordinator::publication_stream::tests::native(&s.source);
                let f = stream_facts(&s, &n);
                let mut stream: Option<CollectionPublicationStreamIdV1> = None;
                let debt = o.retention_usage();
                let mut r = if mode == 0 {
                    Recorder::disabled()
                } else {
                    Recorder::try_with_capacity(if mode == 3 { 1 } else { 512 }).unwrap()
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
                for variant in 0..2 {
                    let p = packet(&mut o, &s, &g, &mut setup);
                    let op = match &stream {
                        None => CollectionPublicationCommandOperationV1::Establish,
                        Some(s) => CollectionPublicationCommandOperationV1::Resynchronize {
                            stream: s.clone(),
                        },
                    };
                    let command = o
                        .issue_collection_publication_command_v1(
                            &op,
                            &p,
                            &s,
                            &f,
                            Some(true),
                            &mut setup,
                        )
                        .unwrap();
                    assert_eq!(o.annotation_preparation_usage().prepared, 1);
                    let mut q = CollectionPublicationCommandRequestV1 {
                        scope: &s,
                        operation: &op,
                        preparation: &p,
                        current: Some(&f),
                        owner_control_allowed: Some(true),
                        max_reply_bytes: 16384,
                    };
                    let cut = prepare(&mut o, &ds, installed, &mut setup);
                    let facts = facts();
                    let result = command_envelope(typed, &command, stream.as_ref(), &p, |e| {
                        o.request_discovered_collection_publication_command_v1(
                            NativeDiscoveredCommandRouteV1 {
                                context: &context,
                                scope: &ds,
                                cut: &cut,
                                entry: &rows[0].0,
                                variant: &rows[0].1[variant],
                                facts: &facts,
                            },
                            e,
                            &CollectionPublicationCommandPortV1 {
                                command: &command,
                                request: &q,
                            },
                            &mut r,
                        )
                    });
                    if !installed {
                        assert_eq!(
                            result,
                            Err(DiscoveredCollectionPublicationErrorV1::Discovery(
                                Error::Unavailable
                            ))
                        );
                        assert_eq!(o.annotation_preparation_usage().prepared, 1);
                        assert!(
                            !o.collection_publication_command_facts_v1(&command, &s, &mut setup)
                                .unwrap()
                                .native_started
                        );
                        o.retire_discovery_cut_v1(&cut, &facts, &mut setup).unwrap();
                        command_envelope(typed, &command, stream.as_ref(), &p, |e| {
                            o.request_decoded_collection_publication_command_v1(
                                e,
                                &CollectionPublicationCommandPortV1 {
                                    command: &command,
                                    request: &q,
                                },
                                &mut r,
                            )
                        })
                        .unwrap();
                    } else {
                        assert!(matches!(
                            result,
                            Ok(CollectionPublicationCommandReplyV1::Settled { .. })
                        ));
                    }
                    assert_eq!(o.annotation_preparation_usage().prepared, 0);
                    let original = o
                        .collection_publication_command_facts_v1(&command, &s, &mut setup)
                        .unwrap()
                        .as_observed
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .clone();
                    let before = (
                        o.collection_publication_usage_v1().unwrap(),
                        o.annotation_preparation_usage(),
                    );
                    q.current = None;
                    if installed {
                        let cut = prepare(&mut o, &ds, true, &mut setup);
                        assert!(matches!(
                            command_envelope(typed, &command, stream.as_ref(), &p, |e| o
                                .request_discovered_collection_publication_command_v1(
                                    NativeDiscoveredCommandRouteV1 {
                                        context: &context,
                                        scope: &ds,
                                        cut: &cut,
                                        entry: &rows[0].0,
                                        variant: &rows[0].1[variant],
                                        facts: &facts
                                    },
                                    e,
                                    &CollectionPublicationCommandPortV1 {
                                        command: &command,
                                        request: &q
                                    },
                                    &mut r
                                )),
                            Ok(CollectionPublicationCommandReplyV1::ObserveExisting { .. })
                        ));
                    }
                    assert_eq!(
                        (
                            o.collection_publication_usage_v1().unwrap(),
                            o.annotation_preparation_usage()
                        ),
                        before
                    );
                    let receipt = open_receipt(&mut o, &command, &s, &mut setup);
                    let basis = o
                        .collection_publication_receipt_basis_v1(&receipt, &command, &s, &mut setup)
                        .unwrap()
                        .basis;
                    let f = receipt_facts(&s);
                    let historical = TrustedCollectionPublicationReceiptHistoricalFactsV1 {
                        basis: &basis,
                        current: &f,
                    };
                    let port = CollectionPublicationReceiptPortV1 {
                        command: &command,
                        context: &receipt,
                        scope: &s,
                        current: &historical,
                    };
                    let cut = prepare(&mut o, &ds, installed, &mut setup);
                    let result = receipt_envelope(typed, &command, &receipt, |e| {
                        o.release_discovered_collection_publication_receipt_v1(
                            NativeDiscoveredCommandRouteV1 {
                                context: &context,
                                scope: &ds,
                                cut: &cut,
                                entry: &rows[1].0,
                                variant: &rows[1].1[0],
                                facts: &facts,
                            },
                            e,
                            &port,
                            &mut r,
                        )
                    });
                    if installed {
                        let reply = result.unwrap();
                        assert_eq!(reply.command, command);
                        assert!(Arc::ptr_eq(&reply.result.unwrap(), &original));
                    } else {
                        assert!(matches!(
                            result,
                            Err(DiscoveredCollectionPublicationErrorV1::Discovery(
                                Error::Unavailable
                            ))
                        ));
                        o.retire_discovery_cut_v1(&cut, &facts, &mut setup).unwrap();
                    }
                    stream = Some(original.stream.clone());
                    assert_eq!(o.retention_usage(), debt);
                }
                if mode == 0 {
                    assert_eq!(r.queued(), 0);
                    assert_eq!(r.dropped(), 0);
                } else if mode == 3 {
                    assert_eq!(r.pop_front(), Some(sentinel));
                    assert!(r.dropped() > 0);
                } else {
                    let mut dispatches = 0;
                    while let Some(e) = r.pop_front() {
                        if e.boundary == Boundary::DiscoveryCollectionPublicationDispatch {
                            dispatches += 1;
                            assert_eq!(e.service_time.is_some(), mode == 2);
                            assert_eq!(
                                e.disposition,
                                if installed {
                                    Disposition::Completed
                                } else {
                                    Disposition::Refused
                                }
                            );
                        }
                    }
                    assert_eq!(dispatches, if installed { 6 } else { 4 });
                    assert_eq!(r.dropped(), 0);
                }
            }
        }
    }
}
