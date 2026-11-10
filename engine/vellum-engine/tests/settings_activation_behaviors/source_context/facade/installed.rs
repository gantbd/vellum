use super::*;
use crate::discovered_context::{facts as discovery_facts, limits as discovery_limits};
fn show<const N: usize>(order: usize) -> NativeBuiltinEntryVisibilityV1<N> {
    NativeBuiltinEntryVisibilityV1 {
        order: Some(order),
        schema: true,
        presentation: false,
        advice: false,
        variants: std::array::from_fn(|i| NativeBuiltinVariantVisibilityV1 {
            order: Some(i),
            enabled: None,
        }),
    }
}
fn cut(o: &mut SourceCoordinator, s: &DiscoveryScopeV1, r: &mut Recorder) -> DiscoveryCutIdV1 {
    o.prepare_native_source_read_discovery_cut_v1(
        s,
        &show(0),
        &[show(1), show(2)],
        &discovery_facts(),
        r,
    )
    .unwrap()
}
fn acquisition<T>(
    scope: &SourceReadScopeV1,
    original: Option<&SourceReadContextId>,
    dynamic: bool,
    consume: impl FnOnce(&ReadAcquisitionEnvelopeValueV1<'_, '_>) -> T,
) -> T {
    use InertValueV1 as D;
    use ReadAcquisitionEnvelopeValueV1::{Classes, Data, Document, Object, Reference};
    let selection = [
        (
            "kind",
            Data(D::Text(if original.is_some() {
                "retained"
            } else {
                "capture-current"
            })),
        ),
        (
            "reference",
            Reference(original.map_or(
                ReadAcquisitionSelectionV1::Current,
                ReadAcquisitionSelectionV1::Source,
            )),
        ),
    ];
    let args = [
        ("contract", Data(D::Text("read-context/1"))),
        ("operation", Data(D::Text("snapshot-handoff"))),
        ("document", Document(&scope.document)),
        (
            "selection",
            Object(&selection[..if original.is_some() { 2 } else { 1 }]),
        ),
        ("data", Classes(AnnotationProjectionClassesV1::Source)),
        ("profile", Data(D::Text(&scope.context.profile))),
    ];
    let header = [
        (
            if dynamic { "name" } else { "family" },
            Data(if dynamic {
                D::Text("read-document")
            } else {
                D::Number(1.0)
            }),
        ),
        ("variant", Data(D::Text("snapshot-handoff"))),
        ("version", Data(D::Number(1.0))),
        ("arguments", Object(&args)),
    ];
    let outer = [
        (
            "route",
            Data(D::Text(if dynamic { "dynamic" } else { "typed" })),
        ),
        ("request", Object(&header)),
    ];
    consume(&Object(&outer))
}

#[test]
fn given_discovered_read_facade_when_source_changes_then_retained_reads_preserve_capture_and_cleanup()
 {
    for dynamic in [false, true] {
        let (mut o, mut r) = setup();
        let discovery = discovery_limits();
        let source_limits = super::super::profile();
        o.configure_retention_budget(
            discovery.max_held_bytes + source_limits.max_pool_bytes + 16 * limits().max_bytes,
            &mut r,
        )
        .unwrap();
        o.configure_source_read_contexts("engine", "local", source_limits, &pool_control(), &mut r)
            .unwrap();
        let document = o.create(b"body\r\nlast", &mut r).unwrap();
        let scope = super::super::scope(&document.document);
        let ds = DiscoveryScopeV1 {
            context: SettingsExecutionContextV1 {
                profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                ..scope.context.clone()
            },
            policy_epoch: 1,
        };
        o.configure_discovery_v1(&ds, discovery, &discovery_facts(), &mut r)
            .unwrap();
        let p = cut(&mut o, &ds, &mut r);
        let dc = o
            .acquire_discovery_v1(&ds, &p, &discovery_facts(), &mut r)
            .unwrap()
            .context;
        let p = cut(&mut o, &ds, &mut r);
        let DiscoveryReplyV1::Page { entries, .. } = o
            .query_discovery_v1(
                &dc,
                &ds,
                &p,
                DiscoveryQueryV1::List {
                    limit: 3,
                    continuation: None,
                },
                &discovery_facts(),
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        let selectors: Vec<_> = entries
            .iter()
            .take(2)
            .map(|entry| {
                let p = cut(&mut o, &ds, &mut r);
                let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
                    .query_discovery_v1(
                        &dc,
                        &ds,
                        &p,
                        DiscoveryQueryV1::Inspect {
                            entry: &entry.reference,
                        },
                        &discovery_facts(),
                        &mut r,
                    )
                    .unwrap()
                else {
                    panic!()
                };
                descriptor.variants.into_iter().next().unwrap().reference
            })
            .collect();
        let native_facts = facts(&scope);
        let p = cut(&mut o, &ds, &mut r);
        let before = o.generation();
        let ReadAcquisitionFacadeReplyV1::Source(snapshot) =
            acquisition(&scope, None, dynamic, |envelope| {
                o.acquire_discovered_read_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &ds,
                        cut: &p,
                        entry: &entries[1].reference,
                        variant: &selectors[1],
                        facts: &discovery_facts(),
                    },
                    DecodedReadAcquisitionFacadeRequestV1 {
                        envelope,
                        native: ReadAcquisitionFacadeRequestV1 {
                            port: ReadAcquisitionFacadePortV1::SourceCurrent {
                                scope: &scope,
                                facts: &native_facts,
                            },
                            actor: &scope.context,
                            limits: limits(),
                        },
                    },
                    &mut r,
                )
                .unwrap()
            })
        else {
            panic!("source acquisition")
        };
        assert_eq!(o.generation(), before);
        let prepared = o
            .prepare(
                &[DocumentEdits {
                    expected: document.clone(),
                    edits: vec![vellum_engine::source::Replacement {
                        start: 0,
                        end: 4,
                        expected: "body".into(),
                        inserted: "new".into(),
                    }],
                }],
                &[],
                &mut r,
            )
            .unwrap();
        o.commit(prepared, &mut r).unwrap();
        let p = cut(&mut o, &ds, &mut r);
        let ReadAcquisitionFacadeReplyV1::Source(retained) =
            acquisition(&scope, Some(&snapshot), dynamic, |envelope| {
                o.acquire_discovered_read_v1(
                    NativeDiscoveredCommandRouteV1 {
                        context: &dc,
                        scope: &ds,
                        cut: &p,
                        entry: &entries[1].reference,
                        variant: &selectors[1],
                        facts: &discovery_facts(),
                    },
                    DecodedReadAcquisitionFacadeRequestV1 {
                        envelope,
                        native: ReadAcquisitionFacadeRequestV1 {
                            port: ReadAcquisitionFacadePortV1::SourceRetained {
                                scope: &scope,
                                original: &snapshot,
                                facts: &native_facts,
                            },
                            actor: &scope.context,
                            limits: limits(),
                        },
                    },
                    &mut r,
                )
                .unwrap()
            })
        else {
            panic!("retained acquisition")
        };
        let p = cut(&mut o, &ds, &mut r);
        use InertValueV1 as D;
        use SourceEnvelopeValueV1::{Data, Document, Object, Snapshot};
        let args = [
            ("contract", Data(D::Text("source-query/1"))),
            ("operation", Data(D::Text("bounded-range"))),
            ("snapshot", Snapshot(SearchReadContextV1::Source(&retained))),
            ("document", Document(&scope.document)),
            ("profile", Data(D::Text(&scope.context.profile))),
            ("lo", Data(D::Number(0.0))),
            ("hi", Data(D::Number(4.0))),
        ];
        let header = [
            (
                if dynamic { "name" } else { "family" },
                Data(if dynamic {
                    D::Text("read-document")
                } else {
                    D::Number(1.0)
                }),
            ),
            ("variant", Data(D::Text("bounded-range"))),
            ("version", Data(D::Number(1.0))),
            ("arguments", Object(&args)),
        ];
        let outer = [
            (
                "route",
                Data(D::Text(if dynamic { "dynamic" } else { "typed" })),
            ),
            ("request", Object(&header)),
        ];
        let SourceFacadeReplyV1::Source(reply) = o
            .query_discovered_source_read_v1(
                NativeDiscoveredCommandRouteV1 {
                    context: &dc,
                    scope: &ds,
                    cut: &p,
                    entry: &entries[0].reference,
                    variant: &selectors[0],
                    facts: &discovery_facts(),
                },
                DecodedSourceFacadeRequestV1 {
                    envelope: &Object(&outer),
                    actor: &scope.context,
                    scope: &scope,
                    limits: limits(),
                    facts: SearchReadFactsV1::Source(&native_facts),
                },
                &mut r,
            )
            .unwrap()
        else {
            panic!("source reply")
        };
        let SourceContextContentV1::Range(disclosed) = reply.content else {
            panic!("range")
        };
        assert_eq!(disclosed.bytes(), b"body");
        let mut denied = facts(&scope);
        denied.source_readable = Some(false);
        let p = cut(&mut o, &ds, &mut r);
        assert!(matches!(
            o.query_discovered_source_read_v1(
                NativeDiscoveredCommandRouteV1 {
                    context: &dc,
                    scope: &ds,
                    cut: &p,
                    entry: &entries[0].reference,
                    variant: &selectors[0],
                    facts: &discovery_facts()
                },
                DecodedSourceFacadeRequestV1 {
                    envelope: &Object(&outer),
                    actor: &scope.context,
                    scope: &scope,
                    limits: limits(),
                    facts: SearchReadFactsV1::Source(&denied)
                },
                &mut r
            ),
            Err(DiscoveredSourceReadErrorV1::Native(_))
        ));
        for id in [&snapshot, &retained] {
            assert!(o.stop_source_read_context(id, &controls(&scope), &mut r));
            assert!(o.retire_source_read_context(id, &controls(&scope), &mut r));
        }
        o.retire_source_read_contexts(&pool_control(), &mut r)
            .unwrap();
        o.stop_discovery_context_v1(&dc, &ds, &discovery_facts(), &mut r)
            .unwrap();
        o.retire_discovery_context_v1(&dc, &discovery_facts(), &mut r)
            .unwrap();
        o.stop_discovery_admission_v1(&discovery_facts(), &mut r)
            .unwrap();
        o.retire_discovery_registry_v1(&discovery_facts(), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        assert_eq!(disclosed.bytes(), b"body");
    }
}
