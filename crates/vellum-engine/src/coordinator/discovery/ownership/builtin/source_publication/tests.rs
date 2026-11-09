use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits};
use crate::coordinator::publication_stream::commands::{
    command_envelope, receipt_envelope, source_receipt_test_facts, source_receipt_test_limits,
    source_receipt_test_open, source_receipt_test_setup,
};
use crate::coordinator::publication_stream::tests::{facts as stream_facts, native};

fn visibility() -> NativeSourcePublicationDiscoveryVisibilityV1 {
    NativeSourcePublicationDiscoveryVisibilityV1 {
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
            .prepare_native_source_publication_discovery_cut_v1(s, &visibility(), &facts(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let commands = [
        command(&store.engine, "F19-source-publication", 256).unwrap(),
        command(&store.engine, "F06-source-publication", 256).unwrap(),
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
fn installed_source_executes_and_replays_both_commands_then_releases_original_result() {
    for typed in [false, true] {
        for capacity in [0, 1, 128] {
            let (mut o, mut setup, s, original, _) =
                source_receipt_test_setup(source_receipt_test_limits(), 8 * 1048576);
            let receipt = source_receipt_test_open(&mut o, &original, &s, &mut setup);
            let (ds, dc, rows) = install(&mut o, &s, true, &mut setup);
            let mut r = Recorder::try_with_capacity(capacity).unwrap();
            let n = native(&s);
            let f = stream_facts(&s, &n);
            let op = SourcePublicationCommandOperationV1::Establish;
            let id = o
                .issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r)
                .unwrap();
            for replay in [false, true] {
                let cut = prepare(&mut o, &ds, true, &mut r);
                let request = SourcePublicationCommandRequestV1 {
                    scope: &s,
                    operation: &op,
                    current: if replay { None } else { Some(&f) },
                    owner_control_allowed: Some(true),
                    max_reply_bytes: 16384,
                };
                command_envelope(typed, &id, None, |e| {
                    o.request_discovered_source_publication_command_v1(
                        NativeDiscoveredCommandRouteV1 {
                            context: &dc,
                            scope: &ds,
                            cut: &cut,
                            entry: &rows[0].0,
                            variant: &rows[0].1[0],
                            facts: &facts(),
                        },
                        e,
                        &SourcePublicationCommandPortV1 {
                            command: &id,
                            request: &request,
                        },
                        &mut r,
                    )
                })
                .unwrap();
            }
            let stream = o
                .source_publication_command_facts_v1(&id, &s, &mut r)
                .unwrap()
                .as_observed
                .unwrap()
                .as_ref()
                .unwrap()
                .stream
                .clone();
            let old = o.read(&s.document, &mut r).unwrap();
            let p = o
                .prepare(
                    &[DocumentEdits {
                        expected: old.version,
                        edits: vec![Replacement {
                            start: 0,
                            end: 1,
                            expected: "A".into(),
                            inserted: "B".into(),
                        }],
                    }],
                    &[],
                    &mut r,
                )
                .unwrap();
            o.commit(p, &mut r).unwrap();
            let op = SourcePublicationCommandOperationV1::Resynchronize {
                stream: stream.clone(),
            };
            let id = o
                .issue_source_publication_command_v1(&op, &s, &f, Some(true), &mut r)
                .unwrap();
            let cut = prepare(&mut o, &ds, true, &mut r);
            let request = SourcePublicationCommandRequestV1 {
                scope: &s,
                operation: &op,
                current: Some(&f),
                owner_control_allowed: Some(true),
                max_reply_bytes: 16384,
            };
            command_envelope(typed, &id, Some(&stream), |e| {
                o.request_discovered_source_publication_command_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &ds,
                        cut: &cut,
                        entry: &rows[0].0,
                        variant: &rows[0].1[1],
                        facts: &facts(),
                    },
                    e,
                    &SourcePublicationCommandPortV1 {
                        command: &id,
                        request: &request,
                    },
                    &mut r,
                )
            })
            .unwrap();
            let basis = o
                .source_publication_receipt_basis_v1(&receipt, &original, &s, &mut r)
                .unwrap()
                .basis;
            let current = source_receipt_test_facts(&s);
            let historical = TrustedSourcePublicationReceiptHistoricalFactsV1 {
                basis: &basis,
                current: &current,
            };
            let port = SourcePublicationReceiptPortV1 {
                command: &original,
                context: &receipt,
                scope: &s,
                current: &historical,
            };
            let cut = prepare(&mut o, &ds, true, &mut r);
            let reply = receipt_envelope(typed, &original, &receipt, |e| {
                o.release_discovered_source_publication_receipt_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &ds,
                        cut: &cut,
                        entry: &rows[1].0,
                        variant: &rows[1].1[0],
                        facts: &facts(),
                    },
                    e,
                    &port,
                    &mut r,
                )
            })
            .unwrap();
            let original_result = o
                .source_publication_command_facts_v1(&original, &s, &mut r)
                .unwrap()
                .as_observed
                .unwrap()
                .as_ref()
                .unwrap()
                .clone();
            let released = reply.result.unwrap();
            assert!(Arc::ptr_eq(&released, &original_result));
            assert_eq!(released.projection.text(), "A");
            assert_eq!(o.read(&s.document, &mut r).unwrap().source.text(), "B");
        }
    }
}
#[test]
fn copied_metadata_and_cross_kind_selectors_cannot_invoke_or_apply_permission_loss() {
    for installed in [false, true] {
        let (mut o, mut r, s, id, _) =
            source_receipt_test_setup(source_receipt_test_limits(), 8 * 1048576);
        let receipt = source_receipt_test_open(&mut o, &id, &s, &mut r);
        let (ds, dc, rows) = install(&mut o, &s, installed, &mut r);
        let basis = o
            .source_publication_receipt_basis_v1(&receipt, &id, &s, &mut r)
            .unwrap()
            .basis;
        let mut current = source_receipt_test_facts(&s);
        current.authorized = Some(false);
        let historical = TrustedSourcePublicationReceiptHistoricalFactsV1 {
            basis: &basis,
            current: &current,
        };
        let port = SourcePublicationReceiptPortV1 {
            command: &id,
            context: &receipt,
            scope: &s,
            current: &historical,
        };
        let cut = prepare(&mut o, &ds, installed, &mut r);
        let mut denied = facts();
        denied.authorized = Some(false);
        let index = if installed { 0 } else { 1 };
        assert!(
            receipt_envelope(true, &id, &receipt, |e| o
                .release_discovered_source_publication_receipt_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &ds,
                        cut: &cut,
                        entry: &rows[index].0,
                        variant: &rows[index].1[0],
                        facts: &denied
                    },
                    e,
                    &port,
                    &mut r
                ))
            .is_err()
        );
        assert!(!o.discovery_context(&dc).unwrap().stopped);
        // Recognizer refusal did not consume native authority loss or stop receipt.
        o.source_publication_receipt_basis_v1(&receipt, &id, &s, &mut r)
            .unwrap();
    }
}
