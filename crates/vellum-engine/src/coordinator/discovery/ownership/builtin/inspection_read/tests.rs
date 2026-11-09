use super::*;
use crate::coordinator::discovery::ownership::tests::{facts, limits};
use crate::coordinator::source_context::facade::annotations::arguments::tests::with_decoded_request as annotation_request;
use crate::coordinator::source_context::facade::history::arguments::tests::with_decoded_request as history_request;
use crate::coordinator::source_context::facade::{
    annotations::tests as annotation, history::tests as historical,
};
use crate::instrumentation::NativeTimingProfileV1;
fn visibility() -> NativeBuiltinEntryVisibilityV1<3> {
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
    scope: &DiscoveryScopeV1,
    history: bool,
    installed: bool,
    r: &mut Recorder,
) -> DiscoveryCutIdV1 {
    if installed {
        return if history {
            o.prepare_native_history_read_discovery_cut_v1(scope, &visibility(), &facts(), r)
        } else {
            o.prepare_native_annotation_read_discovery_cut_v1(scope, &visibility(), &facts(), r)
        }
        .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        if history {
            "F04-history-read-v1"
        } else {
            "F03-annotation-read-v1"
        },
        store.limits.max_context_bytes,
    )
    .unwrap();
    with_input(&authority, &command, history, &visibility(), |input| {
        o.prepare_discovery_cut_v1(scope, input, &facts(), r)
    })
    .unwrap()
}
struct Binding {
    scope: DiscoveryScopeV1,
    context: DiscoveryContextIdV1,
    entry: DiscoveryEntryIdV1,
    variants: Vec<DiscoveryVariantIdV1>,
    history: bool,
    installed: bool,
}
impl Binding {
    fn new(
        o: &mut SourceCoordinator,
        actor: &SettingsExecutionContextV1,
        history: bool,
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
        l.max_contexts = 1;
        l.max_preparations = 1;
        l.max_entries = 1;
        l.max_variants = 3;
        l.max_selectors = 8;
        l.max_issued = 64;
        l.max_context_bytes = 256;
        l.max_packet_bytes = 32768;
        l.max_capture_bytes = 32768;
        l.max_reply_bytes = 32768;
        l.max_row_bytes = 16384;
        l.max_root_bytes = 32768;
        l.metadata.max_bytes = 8192;
        l.metadata.max_nodes = 512;
        l.metadata.max_depth = 12;
        l.max_held_bytes = l.declared_bytes().unwrap();
        o.configure_discovery_v1(&scope, l, &facts(), r).unwrap();
        let p = prepare(o, &scope, history, installed, r);
        let context = o
            .acquire_discovery_v1(&scope, &p, &facts(), r)
            .unwrap()
            .context;
        let p = prepare(o, &scope, history, installed, r);
        let DiscoveryReplyV1::Page { mut entries, .. } = o
            .query_discovery_v1(
                &context,
                &scope,
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
        let p = prepare(o, &scope, history, installed, r);
        let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
            .query_discovery_v1(
                &context,
                &scope,
                &p,
                DiscoveryQueryV1::Inspect { entry: &entry },
                &facts(),
                r,
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(descriptor.variants.len(), 3);
        Self {
            scope,
            context,
            entry,
            variants: descriptor
                .variants
                .into_iter()
                .map(|v| v.reference)
                .collect(),
            history,
            installed,
        }
    }
    fn cut(&self, o: &mut SourceCoordinator, r: &mut Recorder) -> DiscoveryCutIdV1 {
        prepare(o, &self.scope, self.history, self.installed, r)
    }
    fn route<'a>(
        &'a self,
        cut: &'a DiscoveryCutIdV1,
        index: usize,
        facts: &'a TrustedDiscoveryFactsV1,
    ) -> NativeDiscoveredCommandRouteV1<'a> {
        NativeDiscoveredCommandRouteV1 {
            context: &self.context,
            scope: &self.scope,
            cut,
            entry: &self.entry,
            variant: &self.variants[index],
            facts,
        }
    }
    fn discharge_cut(&self, o: &mut SourceCoordinator, p: &DiscoveryCutIdV1, r: &mut Recorder) {
        assert!(matches!(
            o.retire_discovery_cut_v1(p, &facts(), r),
            Ok(()) | Err(Error::Unavailable)
        ));
    }
    fn retire(&self, o: &mut SourceCoordinator, r: &mut Recorder) {
        o.stop_discovery_context_v1(&self.context, &self.scope, &facts(), r)
            .unwrap();
        o.retire_discovery_context_v1(&self.context, &facts(), r)
            .unwrap();
        o.stop_discovery_admission_v1(&facts(), r).unwrap();
        o.retire_discovery_registry_v1(&facts(), r).unwrap();
    }
}
fn recorder(mode: usize) -> Recorder {
    let r = if mode == 0 {
        Recorder::disabled()
    } else {
        Recorder::try_with_capacity(if mode == 3 { 1 } else { 256 }).unwrap()
    };
    if mode >= 2 {
        r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1)
    } else {
        r
    }
}
fn verify_capture(r: &mut Recorder, mode: usize, boundary: Boundary, installed: bool) {
    let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
    if mode == 1 || mode == 2 {
        let events: Vec<_> = events.iter().filter(|e| e.boundary == boundary).collect();
        assert_eq!(events.len(), 3);
        assert!(
            events
                .iter()
                .all(|e| e.service_time.is_some() == (mode == 2)
                    && e.disposition
                        == if installed {
                            Disposition::Completed
                        } else {
                            Disposition::Refused
                        })
        );
    } else if mode == 3 {
        assert!(r.dropped() > 0);
    } else {
        assert!(events.is_empty());
    }
}
fn annotation_query(
    f: &mut annotation::Fixture,
    q: &annotation::Query,
    b: &Binding,
    route: &str,
    index: usize,
) -> Option<AnnotationFacadeReplyV1> {
    let p = b.cut(&mut f.o, &mut f.r);
    let result = annotation::invoke_with(
        f,
        q,
        Some(true),
        Some(true),
        Some(true),
        annotation::limits(),
        |o, native, r| {
            annotation_request(native, route, |request| {
                let result =
                    o.query_discovered_annotations_v1(b.route(&p, index, &facts()), request, r);
                assert_eq!(result.is_ok(), b.installed);
                result.ok()
            })
        },
    );
    b.discharge_cut(&mut f.o, &p, &mut f.r);
    result
}
#[test]
fn installed_annotation_queries_join_both_snapshot_owners_and_all_three_products() {
    for coherent in [false, true] {
        for installed in [false, true] {
            for route in ["typed", "dynamic"] {
                for mode in 0..4 {
                    let mut f = annotation::owner(coherent);
                    let b = Binding::new(&mut f.o, &f.scope.context, false, installed, &mut f.r);
                    f.r = recorder(mode);
                    let generation = f.o.generation();
                    let debt = f.o.retention_usage();
                    let q = annotation::enumerate(&f, 1);
                    let page = annotation_query(&mut f, &q, &b, route, 0);
                    let page = page.unwrap_or_else(|| {
                        annotation::invoke_with(
                            &mut f,
                            &q,
                            Some(true),
                            Some(true),
                            Some(true),
                            annotation::limits(),
                            |o, native, r| o.query_annotation_facade_v1(native, r),
                        )
                        .unwrap()
                    });
                    let (inspect, location) = annotation::record_queries(page);
                    annotation_query(&mut f, &inspect, &b, route, 1);
                    annotation_query(&mut f, &location, &b, route, 2);
                    assert_eq!(f.o.generation(), generation);
                    assert_eq!(f.o.retention_usage(), debt);
                    assert_eq!(f.o.read_facade_usage().workspace_bytes, 0);
                    verify_capture(
                        &mut f.r,
                        mode,
                        Boundary::DiscoveryAnnotationReadDispatch,
                        installed,
                    );
                    b.retire(&mut f.o, &mut f.r);
                    annotation::retire(&mut f);
                }
            }
        }
    }
}
#[test]
fn installed_history_queries_join_existing_frontier_availability_and_filtered_detail() {
    for installed in [false, true] {
        for route in ["typed", "dynamic"] {
            for mode in 0..4 {
                let mut f = historical::owner();
                let b = Binding::new(&mut f.o, &f.s.context, true, installed, &mut f.r);
                f.r = recorder(mode);
                let before = (
                    f.o.retention_usage(),
                    f.o.generation(),
                    f.o.history_inspection_usage_v1(),
                );
                for (index, op) in ["frontier", "availability", "permitted-blocking-detail"]
                    .into_iter()
                    .enumerate()
                {
                    let p = b.cut(&mut f.o, &mut f.r);
                    let result = history_request(&f.id, &f.s, route, op, |request| {
                        f.o.query_discovered_history_v1(
                            b.route(&p, index, &facts()),
                            request,
                            &mut f.r,
                        )
                    });
                    assert_eq!(result.is_ok(), installed);
                    if let Ok(result) = result {
                        assert!(matches!(
                            (index, result),
                            (0, HistoryInspectionReplyV1::Frontier { .. })
                                | (1, HistoryInspectionReplyV1::Availability { .. })
                                | (2, HistoryInspectionReplyV1::PermittedBlockingDetail { .. })
                        ));
                    }
                    b.discharge_cut(&mut f.o, &p, &mut f.r);
                }
                assert_eq!(
                    (
                        f.o.retention_usage(),
                        f.o.generation(),
                        f.o.history_inspection_usage_v1()
                    ),
                    before
                );
                assert_eq!(f.o.read_facade_usage().workspace_bytes, 0);
                verify_capture(
                    &mut f.r,
                    mode,
                    Boundary::DiscoveryHistoryReadDispatch,
                    installed,
                );
                b.retire(&mut f.o, &mut f.r);
                assert!(f.o.stop_history_inspection_v1(
                    &f.id,
                    &historical::control(&f.s),
                    &mut f.r
                ));
                assert!(f.o.retire_history_inspection_v1(
                    &f.id,
                    &historical::control(&f.s),
                    &mut f.r
                ));
            }
        }
    }
}

#[test]
fn inspection_dispatch_keeps_selector_metadata_and_native_authority_independent() {
    for typed in ["typed", "dynamic"] {
        for denied in 0..3 {
            let mut f = historical::owner();
            let b = Binding::new(&mut f.o, &f.s.context, true, true, &mut f.r);
            let p = b.cut(&mut f.o, &mut f.r);
            let mut metadata = facts();
            if denied == 1 {
                metadata.discovery_allowed = Some(false);
            }
            let before = (
                f.o.generation(),
                f.o.history_inspection_usage_v1(),
                f.o.retention_usage(),
            );
            history_request(&f.id, &f.s, typed, "frontier", |request| {
                let mut lost = historical::facts(&f.s);
                lost.granted = Some(false);
                let result = f.o.query_discovered_history_v1(
                    b.route(&p, usize::from(denied == 0), &metadata),
                    DecodedHistoryFacadeRequestV1 {
                        facts: &lost,
                        ..request
                    },
                    &mut f.r,
                );
                if denied == 2 {
                    assert!(matches!(
                        result,
                        Err(DiscoveredSourceReadErrorV1::Native(_))
                    ));
                } else {
                    assert!(matches!(
                        result,
                        Err(DiscoveredSourceReadErrorV1::Discovery(_))
                    ));
                }
            });
            if denied < 2 {
                assert_eq!(
                    (
                        f.o.generation(),
                        f.o.history_inspection_usage_v1(),
                        f.o.retention_usage()
                    ),
                    before
                );
                history_request(&f.id, &f.s, typed, "frontier", |request| {
                    assert!(
                        f.o.try_query_decoded_history_facade_v1(request, &mut f.r)
                            .is_ok()
                    )
                });
            }
            b.discharge_cut(&mut f.o, &p, &mut f.r);
            b.retire(&mut f.o, &mut f.r);
            assert!(f.o.retire_history_inspection_v1(&f.id, &historical::control(&f.s), &mut f.r));

            let mut f = annotation::owner(false);
            let b = Binding::new(&mut f.o, &f.scope.context, false, true, &mut f.r);
            let p = b.cut(&mut f.o, &mut f.r);
            let q = annotation::enumerate(&f, 1);
            let mut metadata = facts();
            if denied == 1 {
                metadata.discovery_allowed = Some(false);
            }
            annotation::invoke_with(
                &mut f,
                &q,
                Some(false),
                Some(true),
                Some(true),
                annotation::limits(),
                |o, native, r| {
                    annotation_request(native, typed, |request| {
                        let result = o.query_discovered_annotations_v1(
                            b.route(&p, usize::from(denied == 0), &metadata),
                            request,
                            r,
                        );
                        if denied == 2 {
                            assert!(matches!(
                                result,
                                Err(DiscoveredSourceReadErrorV1::Native(_))
                            ));
                        } else {
                            assert!(matches!(
                                result,
                                Err(DiscoveredSourceReadErrorV1::Discovery(_))
                            ));
                        }
                        None
                    })
                },
            );
            if denied < 2 {
                assert!(
                    annotation::invoke_with(
                        &mut f,
                        &q,
                        Some(true),
                        Some(true),
                        Some(true),
                        annotation::limits(),
                        |o, native, r| o.query_annotation_facade_v1(native, r)
                    )
                    .is_some()
                );
            }
            b.discharge_cut(&mut f.o, &p, &mut f.r);
            b.retire(&mut f.o, &mut f.r);
            annotation::retire(&mut f);
        }
    }
}
