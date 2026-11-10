use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits};
use crate::coordinator::source_context::facade::acquisition::{
    arguments::tests::with_envelope, tests as acquisition_tests,
};
use crate::coordinator::source_context::search::result_contract::tests::with_facts;
use crate::coordinator::source_query::result_contract::tests::owner;
use crate::instrumentation::NativeTimingProfileV1;

fn visibility<const N: usize>(order: usize) -> NativeBuiltinEntryVisibilityV1<N> {
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
fn prepare(
    o: &mut SourceCoordinator,
    s: &DiscoveryScopeV1,
    installed: bool,
    r: &mut Recorder,
) -> DiscoveryCutIdV1 {
    if installed {
        return o
            .prepare_native_source_read_discovery_cut_v1(
                s,
                &visibility(0),
                &[visibility(1), visibility(2)],
                &facts(),
                r,
            )
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let commands = [
        "F02-source-read-v1",
        "F02-single-acquisition-v1",
        "F02-coherent-acquisition-v1",
    ]
    .map(|name| command(&store.engine, name, store.limits.max_context_bytes).unwrap());
    with_input(
        &authority,
        &commands,
        &[visibility(0)],
        &[visibility(1), visibility(2)],
        |input| o.prepare_discovery_cut_v1(s, input, &facts(), r),
    )
    .unwrap()
}
struct Binding {
    scope: DiscoveryScopeV1,
    context: DiscoveryContextIdV1,
    selectors: Vec<(DiscoveryEntryIdV1, Vec<DiscoveryVariantIdV1>)>,
    installed: bool,
}
impl Binding {
    fn new(
        o: &mut SourceCoordinator,
        actor: &SettingsExecutionContextV1,
        installed: bool,
        r: &mut Recorder,
    ) -> Self {
        let scope = DiscoveryScopeV1 {
            context: SettingsExecutionContextV1 {
                profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                ..actor.clone()
            },
            policy_epoch: 1,
        };
        let mut l = limits();
        l.max_page_rows = 3;
        l.max_selectors = 32;
        l.max_issued = 256;
        l.max_context_bytes = 256;
        l.max_row_bytes = 65536;
        l.max_capture_bytes = 131072;
        l.max_packet_bytes = 131072;
        l.max_reply_bytes = 131072;
        l.metadata.max_bytes = 32768;
        l.metadata.max_nodes = 2048;
        l.metadata.max_depth = 12;
        l.max_held_bytes = l.declared_bytes().unwrap();
        o.configure_discovery_v1(&scope, l, &facts(), r).unwrap();
        let p = prepare(o, &scope, installed, r);
        let context = o
            .acquire_discovery_v1(&scope, &p, &facts(), r)
            .unwrap()
            .context;
        let p = prepare(o, &scope, installed, r);
        let DiscoveryReplyV1::Page { entries, .. } = o
            .query_discovery_v1(
                &context,
                &scope,
                &p,
                DiscoveryQueryV1::List {
                    limit: 3,
                    continuation: None,
                },
                &facts(),
                r,
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(entries.len(), 3);
        let selectors = entries
            .into_iter()
            .map(|entry| {
                let p = prepare(o, &scope, installed, r);
                let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
                    .query_discovery_v1(
                        &context,
                        &scope,
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
        Self {
            scope,
            context,
            selectors,
            installed,
        }
    }
    fn cut(&self, o: &mut SourceCoordinator, r: &mut Recorder) -> DiscoveryCutIdV1 {
        prepare(o, &self.scope, self.installed, r)
    }
    fn route<'a>(
        &'a self,
        cut: &'a DiscoveryCutIdV1,
        entry: usize,
        variant: usize,
        f: &'a TrustedDiscoveryFactsV1,
    ) -> NativeDiscoveredCommandRouteV1<'a> {
        NativeDiscoveredCommandRouteV1 {
            context: &self.context,
            scope: &self.scope,
            cut,
            entry: &self.selectors[entry].0,
            variant: &self.selectors[entry].1[variant],
            facts: f,
        }
    }
}

fn source_envelope<T>(
    source: SearchReadContextV1<'_>,
    scope: &SourceReadScopeV1,
    typed: bool,
    line: bool,
    consume: impl FnOnce(&SourceEnvelopeValueV1<'_, '_>) -> T,
) -> T {
    use SourceEnvelopeValueV1::{Data, Document, Object, Snapshot};
    let operation = if line { "line" } else { "bounded-range" };
    let args = [
        ("contract", Data(D::Text("source-query/1"))),
        ("operation", Data(D::Text(operation))),
        ("snapshot", Snapshot(source)),
        ("document", Document(&scope.document)),
        ("profile", Data(D::Text(&scope.context.profile))),
        (if line { "line" } else { "lo" }, Data(D::Number(0.0))),
        ("hi", Data(D::Number(4.0))),
    ];
    let header = [
        (
            if typed { "family" } else { "name" },
            Data(if typed {
                D::Number(1.0)
            } else {
                D::Text("read-document")
            }),
        ),
        ("variant", Data(D::Text(operation))),
        ("version", Data(D::Number(1.0))),
        ("arguments", Object(&args[..if line { 6 } else { 7 }])),
    ];
    let outer = [
        (
            "route",
            Data(D::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", Object(&header)),
    ];
    consume(&Object(&outer))
}

#[test]
fn installed_source_reads_use_original_snapshot_and_independent_current_authority() {
    for installed in [false, true] {
        for kind in 0..3 {
            for typed in [false, true] {
                for line in [false, true] {
                    for mode in 0..4 {
                        let mut f = owner();
                        f.o.set_settings_parent_limit_for_test(16384);
                        let binding = Binding::new(&mut f.o, &f.s.context, installed, &mut f.r);
                        f.r = if mode == 0 {
                            Recorder::disabled()
                        } else {
                            Recorder::try_with_capacity(if mode == 3 { 1 } else { 256 }).unwrap()
                        };
                        if mode >= 2 {
                            f.r =
                                f.r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                        }
                        with_facts(&mut f, kind, |o, source, scope, native_facts, r| {
                            let cut = binding.cut(o, r);
                            let before = (
                                o.retention_usage(),
                                o.generation(),
                                o.source_read_context_usage(),
                                o.annotation_context_usage(),
                                o.version_set_context_usage(),
                            );
                            source_envelope(source, scope, typed, line, |envelope| {
                                let request = DecodedSourceFacadeRequestV1 {
                                    envelope,
                                    actor: &scope.context,
                                    scope,
                                    facts: native_facts,
                                    limits: crate::capture::CaptureLimitsV1 {
                                        max_bytes: 16384,
                                        max_nodes: 512,
                                        max_depth: 16,
                                    },
                                };
                                let result = o.query_discovered_source_read_v1(
                                    binding.route(&cut, 0, usize::from(line), &facts()),
                                    request,
                                    r,
                                );
                                assert_eq!(
                                    result.is_ok(),
                                    installed,
                                    "kind={kind}/typed={typed}/line={line}"
                                );
                                if let Ok(reply) = result {
                                    let content = match reply {
                                        SourceFacadeReplyV1::Source(v) => v.content,
                                        SourceFacadeReplyV1::Combined(v) => v.content,
                                        SourceFacadeReplyV1::VersionSet(v) => v.content,
                                    };
                                    match (line, content) {
                                        (true, SourceContextContentV1::Line(v)) => {
                                            assert_eq!(v.body(), b"body");
                                            assert_eq!(v.delimiter(), b"\r\n");
                                        }
                                        (false, SourceContextContentV1::Range(v)) => {
                                            assert_eq!(v.bytes(), b"body")
                                        }
                                        _ => panic!("wrong source product"),
                                    }
                                }
                            });
                            assert_eq!(
                                (
                                    o.retention_usage(),
                                    o.generation(),
                                    o.source_read_context_usage(),
                                    o.annotation_context_usage(),
                                    o.version_set_context_usage()
                                ),
                                before
                            );
                            assert_eq!(o.source_facade_usage().workspace_bytes, 0);
                        });
                        let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
                        if mode == 1 || mode == 2 {
                            let event = events
                                .iter()
                                .find(|e| e.boundary == Boundary::DiscoverySourceReadDispatch)
                                .unwrap();
                            assert_eq!(event.service_time.is_some(), mode == 2);
                            assert_eq!(
                                event.disposition,
                                if installed {
                                    Disposition::Completed
                                } else {
                                    Disposition::Refused
                                }
                            );
                        }
                        if mode == 3 {
                            assert!(f.r.dropped() > 0);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn installed_acquisition_reuses_all_six_native_ports_and_both_envelopes() {
    for installed in [false, true] {
        for kind in 0..6 {
            for route in ["typed", "dynamic"] {
                for mode in 0..4 {
                    let mut f = acquisition_tests::owner();
                    let binding =
                        Binding::new(&mut f.base.o, &f.base.s.context, installed, &mut f.base.r);
                    f.base.r = if mode == 0 {
                        Recorder::disabled()
                    } else {
                        Recorder::try_with_capacity(if mode == 3 { 1 } else { 256 }).unwrap()
                    };
                    if mode >= 2 {
                        f.base.r = f
                            .base
                            .r
                            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                    }
                    let result = acquisition_tests::invoke_with(
                        &mut f,
                        kind,
                        acquisition_tests::params(),
                        |o, native, r| {
                            let cut = binding.cut(o, r);
                            with_envelope(native, route, None, |envelope| {
                                let result = o.acquire_discovered_read_v1(
                                    binding.route(&cut, if kind >= 4 { 2 } else { 1 }, 0, &facts()),
                                    DecodedReadAcquisitionFacadeRequestV1 { envelope, native },
                                    r,
                                );
                                assert_eq!(result.is_ok(), installed, "port={kind}/{route}");
                                result.ok()
                            })
                        },
                    );
                    assert_eq!(result.is_some(), installed);
                    assert_eq!(f.base.o.read_facade_usage().workspace_bytes, 0);
                    let events: Vec<_> = std::iter::from_fn(|| f.base.r.pop_front()).collect();
                    if mode == 1 || mode == 2 {
                        let event = events
                            .iter()
                            .find(|e| e.boundary == Boundary::DiscoveryReadAcquisitionDispatch)
                            .unwrap();
                        assert_eq!(event.service_time.is_some(), mode == 2);
                        assert_eq!(
                            event.disposition,
                            if installed {
                                Disposition::Completed
                            } else {
                                Disposition::Refused
                            }
                        );
                    }
                    if mode == 3 {
                        assert!(f.base.r.dropped() > 0);
                    }
                }
            }
        }
    }
}

#[test]
fn installed_source_selector_refusal_cannot_apply_native_loss_or_upgrade_metadata_authority() {
    for typed in [false, true] {
        for metadata_denied in [false, true] {
            let mut f = owner();
            f.o.set_settings_parent_limit_for_test(16384);
            let binding = Binding::new(&mut f.o, &f.s.context, true, &mut f.r);
            let cut = binding.cut(&mut f.o, &mut f.r);
            let before = (
                f.o.retention_usage(),
                f.o.generation(),
                f.o.source_read_context_usage(),
            );
            let mut lost =
                crate::coordinator::source_query::result_contract::tests::source_facts(&f.s);
            lost.source_readable = Some(false);
            let mut metadata = facts();
            if metadata_denied {
                metadata.discovery_allowed = Some(false);
            }
            source_envelope(
                SearchReadContextV1::Source(&f.source),
                &f.s,
                typed,
                false,
                |envelope| {
                    let request = DecodedSourceFacadeRequestV1 {
                        envelope,
                        actor: &f.s.context,
                        scope: &f.s,
                        facts: SearchReadFactsV1::Source(&lost),
                        limits: crate::capture::CaptureLimitsV1 {
                            max_bytes: 16384,
                            max_nodes: 512,
                            max_depth: 16,
                        },
                    };
                    // With a denied metadata grant use the correct selector. Otherwise
                    // use the line selector for a range intent. Neither may enter native.
                    assert!(matches!(
                        f.o.query_discovered_source_read_v1(
                            binding.route(&cut, 0, usize::from(!metadata_denied), &metadata),
                            request,
                            &mut f.r
                        ),
                        Err(DiscoveredSourceReadErrorV1::Discovery(_))
                    ));
                    let current =
                        crate::coordinator::source_query::result_contract::tests::source_facts(
                            &f.s,
                        );
                    assert!(
                        f.o.try_query_decoded_source_facade_v1(
                            DecodedSourceFacadeRequestV1 {
                                facts: SearchReadFactsV1::Source(&current),
                                ..request
                            },
                            &mut f.r
                        )
                        .is_ok()
                    );
                },
            );
            assert_eq!(
                (
                    f.o.retention_usage(),
                    f.o.generation(),
                    f.o.source_read_context_usage()
                ),
                before
            );
        }
    }
}
