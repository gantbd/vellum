use super::*;
use crate::coordinator::discovery::ownership::tests::{facts as df, limits};
use crate::coordinator::proposal_advice::ingress::tests::packet;
use crate::coordinator::proposal_advice::tests::{facts, owner_with_parent, plan, request};
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
            .prepare_native_compound_advice_discovery_cut_v1(s, &visibility(), &df(), r)
            .unwrap();
    }
    let store = o.discovery_store().unwrap();
    let authority = store.authority.clone();
    let command = command(
        &store.engine,
        "F10-compound-advice-v1",
        store.limits.max_context_bytes,
    )
    .unwrap();
    with_input(&authority, &command, &visibility(), |input| {
        o.prepare_discovery_cut_v1(s, input, &df(), r)
    })
    .unwrap()
}
struct Binding {
    scope: DiscoveryScopeV1,
    context: DiscoveryContextIdV1,
    entry: DiscoveryEntryIdV1,
    variant: DiscoveryVariantIdV1,
    installed: bool,
}
impl Binding {
    fn new(
        o: &mut SourceCoordinator,
        context: &SettingsExecutionContextV1,
        installed: bool,
        r: &mut Recorder,
    ) -> Self {
        let scope = DiscoveryScopeV1 {
            context: SettingsExecutionContextV1 {
                profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                ..context.clone()
            },
            policy_epoch: 1,
        };
        let mut l = limits();
        l.max_contexts = 1;
        l.max_preparations = 1;
        l.max_entries = 1;
        l.max_variants = 1;
        l.max_selectors = 8;
        l.max_issued = 64;
        l.max_context_bytes = 256;
        l.max_root_bytes = 16384;
        l.max_packet_bytes = 32768;
        l.max_capture_bytes = 32768;
        l.max_reply_bytes = 32768;
        l.max_row_bytes = 16384;
        l.metadata.max_bytes = 8192;
        l.metadata.max_nodes = 512;
        l.metadata.max_depth = 12;
        l.max_held_bytes = l.declared_bytes().unwrap();
        o.configure_discovery_v1(&scope, l, &df(), r).unwrap();
        let p = prepare(o, &scope, installed, r);
        let context = o
            .acquire_discovery_v1(&scope, &p, &df(), r)
            .unwrap()
            .context;
        let p = prepare(o, &scope, installed, r);
        let DiscoveryReplyV1::Page { mut entries, .. } = o
            .query_discovery_v1(
                &context,
                &scope,
                &p,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None,
                },
                &df(),
                r,
            )
            .unwrap()
        else {
            panic!()
        };
        let entry = entries.remove(0).reference;
        let p = prepare(o, &scope, installed, r);
        let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
            .query_discovery_v1(
                &context,
                &scope,
                &p,
                DiscoveryQueryV1::Inspect { entry: &entry },
                &df(),
                r,
            )
            .unwrap()
        else {
            panic!()
        };
        Self {
            scope,
            context,
            entry,
            variant: descriptor.variants.into_iter().next().unwrap().reference,
            installed,
        }
    }
    fn cut(&self, o: &mut SourceCoordinator, r: &mut Recorder) -> DiscoveryCutIdV1 {
        prepare(o, &self.scope, self.installed, r)
    }
    fn route<'a>(
        &'a self,
        cut: &'a DiscoveryCutIdV1,
        f: &'a TrustedDiscoveryFactsV1,
    ) -> NativeDiscoveredCommandRouteV1<'a> {
        NativeDiscoveredCommandRouteV1 {
            context: &self.context,
            scope: &self.scope,
            cut,
            entry: &self.entry,
            variant: &self.variant,
            facts: f,
        }
    }
    fn retire(&self, o: &mut SourceCoordinator, p: &DiscoveryCutIdV1, r: &mut Recorder) {
        assert!(matches!(
            o.retire_discovery_cut_v1(p, &df(), r),
            Ok(()) | Err(Error::Unavailable)
        ));
        o.stop_discovery_context_v1(&self.context, &self.scope, &df(), r)
            .unwrap();
        o.retire_discovery_context_v1(&self.context, &df(), r)
            .unwrap();
        o.stop_discovery_admission_v1(&df(), r).unwrap();
        o.retire_discovery_registry_v1(&df(), r).unwrap();
    }
}
#[test]
fn installed_advice_routes_preserve_full_native_profiles_and_copied_metadata_has_no_handler() {
    for installed in [false, true] {
        for typed in [false, true] {
            for kind in 0..4 {
                for mode in 0..4 {
                    let (mut o, mut r, d) = owner_with_parent(1048576);
                    let p = plan(&d, 1);
                    let c = crate::coordinator::serialization::tests::context();
                    let binding = Binding::new(&mut o, &c, installed, &mut r);
                    let scopes = [SourceOperationScope {
                        expected: d.clone(),
                        replacement_ranges: vec![ByteRange { start: 0, end: 4 }],
                        insertion_sites: vec![],
                    }];
                    let reads = ReadDependencyRequestV3 {
                        predicates: vec![ReadDependencyV3::Base(ReadDependencyV2::Base(
                            ReadDependencyV1::Source(d.clone()),
                        ))],
                        budget: ReadDependencyBudget {
                            max_predicates: 4,
                            max_work: 128,
                        },
                    };
                    let mut q = request(&p, &c);
                    if kind & 1 != 0 {
                        q.reads = Some(&reads);
                    }
                    if kind & 2 != 0 {
                        q.scopes = Some((
                            &scopes,
                            SourceScopeBudget {
                                max_scopes: 2,
                                max_records: 4,
                                max_work: 128,
                            },
                        ));
                    }
                    let basis = o.capture_compound_advice_basis_v1(&q, &mut r).unwrap();
                    r = if mode == 0 {
                        Recorder::disabled()
                    } else {
                        Recorder::try_with_capacity(if mode == 3 { 1 } else { 256 }).unwrap()
                    };
                    if mode >= 2 {
                        r = r.with_native_timing(
                            crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
                        );
                    }
                    let cut = binding.cut(&mut o, &mut r);
                    let before = (
                        o.generation(),
                        o.annotation_usage(),
                        o.history_usage(),
                        o.invocation_usage(),
                        o.retention_usage(),
                    );
                    let out = packet(&q, &p, typed, 0, |envelope| {
                        o.query_discovered_compound_advice_v1(
                            binding.route(&cut, &df()),
                            envelope,
                            &facts(&q, &basis),
                            &mut r,
                        )
                    });
                    if installed {
                        assert!(matches!(
                            out,
                            Ok(CompoundAdviceReplyV1::Advice {
                                checks_satisfied: true,
                                ..
                            })
                        ));
                    } else {
                        assert!(matches!(out, Err(Error::Unavailable)));
                    }
                    assert_eq!(
                        (
                            o.generation(),
                            o.annotation_usage(),
                            o.history_usage(),
                            o.invocation_usage(),
                            o.retention_usage()
                        ),
                        before
                    );
                    let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                    if mode == 1 || mode == 2 {
                        let event = events
                            .iter()
                            .find(|e| e.boundary == Boundary::DiscoveryCompoundAdviceDispatch)
                            .unwrap();
                        assert_eq!(event.service_time.is_some(), mode == 2);
                        assert_eq!(
                            events
                                .iter()
                                .filter(|e| e.boundary == Boundary::CompoundAdviceDecodedDispatch)
                                .count(),
                            usize::from(installed)
                        );
                        assert_eq!(
                            events
                                .iter()
                                .filter(|e| e.boundary == Boundary::CompoundAdviceAdmission)
                                .count(),
                            usize::from(installed)
                        );
                    }
                    if mode == 3 {
                        assert!(r.dropped() > 0);
                    }
                    binding.retire(&mut o, &cut, &mut r);
                }
            }
        }
    }
}
#[test]
fn installed_advice_does_not_upgrade_native_observability_or_apply_facts_for_malformed_roles() {
    for denied in 0..3 {
        let (mut o, mut r, d) = owner_with_parent(1048576);
        let p = plan(&d, 1);
        let c = crate::coordinator::serialization::tests::context();
        let binding = Binding::new(&mut o, &c, true, &mut r);
        let q = request(&p, &c);
        let b = o.capture_compound_advice_basis_v1(&q, &mut r).unwrap();
        let mut f = facts(&q, &b);
        let mut metadata = df();
        if denied == 0 {
            metadata.discovery_allowed = Some(false);
        } else {
            f.validation_observable = Some(false);
        }
        let cut = binding.cut(&mut o, &mut r);
        let before = (
            o.generation(),
            o.annotation_usage(),
            o.history_usage(),
            o.invocation_usage(),
            o.retention_usage(),
        );
        let out = packet(&q, &p, true, if denied == 2 { 12 } else { 0 }, |e| {
            o.query_discovered_compound_advice_v1(binding.route(&cut, &metadata), e, &f, &mut r)
        });
        if denied == 0 {
            assert!(matches!(out, Err(Error::Unavailable)));
        } else {
            assert_eq!(out.unwrap(), CompoundAdviceReplyV1::Unavailable);
        }
        assert_eq!(
            (
                o.generation(),
                o.annotation_usage(),
                o.history_usage(),
                o.invocation_usage(),
                o.retention_usage()
            ),
            before
        );
        assert!(matches!(
            o.query_compound_advice_v1(&q, &facts(&q, &b), &mut r),
            CompoundAdviceReplyV1::Advice {
                checks_satisfied: true,
                ..
            }
        ));
        binding.retire(&mut o, &cut, &mut r);
    }
}
