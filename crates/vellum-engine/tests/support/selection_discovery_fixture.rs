//! Test-only installed selection route; no host policy or second invocation owner.
#![allow(dead_code)]
use crate::discovered_context as declaration;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Family {
    Selection,
    Composition,
    ExactBatch,
    AtomicAuthoring,
    Replacement,
    DiagnosticDrain,
}
pub struct Route {
    scope: DiscoveryScopeV1,
    context: DiscoveryContextIdV1,
    entries: Vec<(DiscoveryEntryIdV1, Vec<DiscoveryVariantIdV1>)>,
    family: Family,
    discovery_charge: usize,
}
fn prepare(
    o: &mut SourceCoordinator,
    scope: &DiscoveryScopeV1,
    family: Family,
    r: &mut Recorder,
) -> DiscoveryCutIdV1 {
    if family == Family::DiagnosticDrain {
        return o
            .prepare_native_diagnostic_drain_discovery_cut_v1(
                scope,
                &show(0),
                &declaration::facts(),
                r,
            )
            .unwrap();
    }
    if family == Family::Replacement {
        return o
            .prepare_native_replacement_discovery_cut_v1(scope, &show(0), &declaration::facts(), r)
            .unwrap();
    }
    if family == Family::Composition {
        return o
            .prepare_native_composition_discovery_cut_v1(scope, &show(0), &declaration::facts(), r)
            .unwrap();
    }
    if family == Family::ExactBatch {
        return o
            .prepare_native_exact_batch_discovery_cut_v1(scope, &show(0), &declaration::facts(), r)
            .unwrap();
    }
    if family == Family::AtomicAuthoring {
        return o
            .prepare_native_atomic_authoring_discovery_cut_v1(
                scope,
                &show(0),
                &declaration::facts(),
                r,
            )
            .unwrap();
    }
    o.prepare_native_selection_discovery_cut_v1(scope, &show(0), &show(1), &declaration::facts(), r)
        .unwrap()
}
impl Route {
    pub fn install_diagnostic_drain(o: &mut SourceCoordinator, r: &mut Recorder) -> Self {
        Self::install_family(o, Family::DiagnosticDrain, r)
    }
    pub fn submit_diagnostic_drain(
        &self,
        o: &mut SourceCoordinator,
        q: &NormalizedDiagnosticDrainArgumentsV1<'_>,
        scope: &DiagnosticScopeV1,
        f: &TrustedDiagnosticDrainFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticDrainPhaseV1, DiscoveredDiagnosticDrainErrorV1> {
        let cut = prepare(o, &self.scope, self.family, r);
        let result = o.submit_discovered_diagnostic_drain_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &self.context,
                scope: &self.scope,
                cut: &cut,
                entry: &self.entries[0].0,
                variant: &self.entries[0].1[0],
                facts: &declaration::facts(),
            },
            q,
            scope,
            f,
            r,
        );
        if matches!(result, Err(DiscoveredDiagnosticDrainErrorV1::Discovery(_))) {
            o.retire_discovery_cut_v1(&cut, &declaration::facts(), r)
                .unwrap();
        }
        result
    }
    pub fn install_replacement(o: &mut SourceCoordinator, r: &mut Recorder) -> Self {
        Self::install_family(o, Family::Replacement, r)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn submit_replacement(
        &self,
        o: &mut SourceCoordinator,
        q: &NormalizedReplacementArgumentsV1<'_>,
        scope: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        cf: &TrustedSourceSearchConsumerFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let variant = usize::from(q.argument_declaration_v1().variant.operation == "replace-all");
        self.submit_replacement_as_variant(o, q, variant, scope, sf, f, cf, r)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn submit_replacement_as_variant(
        &self,
        o: &mut SourceCoordinator,
        q: &NormalizedReplacementArgumentsV1<'_>,
        variant: usize,
        scope: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        cf: &TrustedSourceSearchConsumerFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let cut = prepare(o, &self.scope, self.family, r);
        let result = o.submit_discovered_native_replacement_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &self.context,
                scope: &self.scope,
                cut: &cut,
                entry: &self.entries[0].0,
                variant: &self.entries[0].1[variant],
                facts: &declaration::facts(),
            },
            q,
            scope,
            sf,
            f,
            cf,
            r,
        );
        // Recognition refusal intentionally leaves the supplied cut untouched.
        // The caller owns its protected cleanup before preparing another cut.
        if matches!(result, Err(DiscoveredNativeCommandErrorV1::Discovery(_))) {
            o.retire_discovery_cut_v1(&cut, &declaration::facts(), r)
                .unwrap();
        }
        result
    }
    pub fn install(o: &mut SourceCoordinator, r: &mut Recorder) -> Self {
        Self::install_family(o, Family::Selection, r)
    }
    pub fn install_composition(o: &mut SourceCoordinator, r: &mut Recorder) -> Self {
        Self::install_family(o, Family::Composition, r)
    }
    pub fn install_exact_batch(o: &mut SourceCoordinator, r: &mut Recorder) -> Self {
        Self::install_family(o, Family::ExactBatch, r)
    }
    pub fn install_atomic_authoring(o: &mut SourceCoordinator, r: &mut Recorder) -> Self {
        Self::install_family(o, Family::AtomicAuthoring, r)
    }
    fn install_family(o: &mut SourceCoordinator, family: Family, r: &mut Recorder) -> Self {
        let scope = DiscoveryScopeV1 {
            context: SettingsExecutionContextV1 {
                authority: "engine".into(),
                engine: "local".into(),
                recipient: "host".into(),
                purpose: "selection".into(),
                destination: "native".into(),
                profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                continuity: "session".into(),
            },
            policy_epoch: 1,
        };
        let mut l = declaration::limits();
        l.max_contexts = 1;
        l.max_preparations = 1;
        l.max_entries = 2;
        l.max_variants = 3;
        l.max_selectors = 16;
        l.max_issued = 64;
        l.max_page_rows = 2;
        l.max_context_bytes = 256;
        l.max_packet_bytes = 65536;
        l.max_capture_bytes = 65536;
        l.max_reply_bytes = 65536;
        l.max_row_bytes = 32768;
        l.max_root_bytes = 65536;
        l.metadata.max_bytes = 8192;
        l.metadata.max_nodes = 512;
        l.metadata.max_depth = 12;
        l.max_held_bytes = l.declared_bytes().unwrap();
        let before = o.retention_usage().reserved;
        if o.retention_usage().maximum.is_none() {
            o.configure_retention_budget(l.max_held_bytes, r).unwrap();
        }
        o.configure_discovery_v1(&scope, l, &declaration::facts(), r)
            .unwrap();
        let p = prepare(o, &scope, family, r);
        let context = o
            .acquire_discovery_v1(&scope, &p, &declaration::facts(), r)
            .unwrap()
            .context;
        let p = prepare(o, &scope, family, r);
        let DiscoveryReplyV1::Page { entries, .. } = o
            .query_discovery_v1(
                &context,
                &scope,
                &p,
                DiscoveryQueryV1::List {
                    limit: 2,
                    continuation: None,
                },
                &declaration::facts(),
                r,
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(
            entries.len(),
            if family == Family::Selection { 2 } else { 1 }
        );
        let entries = entries
            .into_iter()
            .map(|entry| {
                let p = prepare(o, &scope, family, r);
                let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
                    .query_discovery_v1(
                        &context,
                        &scope,
                        &p,
                        DiscoveryQueryV1::Inspect {
                            entry: &entry.reference,
                        },
                        &declaration::facts(),
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
            entries,
            family,
            discovery_charge: o.retention_usage().reserved.checked_sub(before).unwrap(),
        }
    }
    pub fn submit(
        &self,
        o: &mut SourceCoordinator,
        q: &NormalizedSelectionArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let d = q.argument_declaration_v1();
        let (entry, variant) = match (d.contract, d.variant.operation) {
            ("selection-edit/1", "set-selection") => (0, 0),
            ("selection-edit/1", "logical-movement") => (0, 1),
            ("selection-edit/1", "select-search-match") => (0, 2),
            ("boundary-command/1", "logical-movement") => (1, 0),
            _ => panic!("unknown native selection form"),
        };
        let p = prepare(o, &self.scope, self.family, r);
        o.submit_discovered_native_selection_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &self.context,
                scope: &self.scope,
                cut: &p,
                entry: &self.entries[entry].0,
                variant: &self.entries[entry].1[variant],
                facts: &declaration::facts(),
            },
            q,
            r,
        )
    }
    pub fn submit_composition(
        &self,
        o: &mut SourceCoordinator,
        q: &NormalizedCompositionArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        assert_eq!(self.family, Family::Composition);
        let p = prepare(o, &self.scope, self.family, r);
        o.submit_discovered_native_composition_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &self.context,
                scope: &self.scope,
                cut: &p,
                entry: &self.entries[0].0,
                variant: &self.entries[0].1[0],
                facts: &declaration::facts(),
            },
            q,
            r,
        )
    }
    pub fn submit_exact_batch(
        &self,
        o: &mut SourceCoordinator,
        q: &NormalizedExactBatchArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        assert_eq!(self.family, Family::ExactBatch);
        let p = prepare(o, &self.scope, self.family, r);
        o.submit_discovered_native_exact_batch_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &self.context,
                scope: &self.scope,
                cut: &p,
                entry: &self.entries[0].0,
                variant: &self.entries[0].1[0],
                facts: &declaration::facts(),
            },
            q,
            r,
        )
    }
    pub fn retire(self, o: &mut SourceCoordinator, r: &mut Recorder) {
        let expected = o
            .retention_usage()
            .reserved
            .checked_sub(self.discovery_charge)
            .unwrap();
        o.stop_discovery_context_v1(&self.context, &self.scope, &declaration::facts(), r)
            .unwrap();
        o.retire_discovery_context_v1(&self.context, &declaration::facts(), r)
            .unwrap();
        o.stop_discovery_admission_v1(&declaration::facts(), r)
            .unwrap();
        o.retire_discovery_registry_v1(&declaration::facts(), r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, expected);
    }
    pub fn submit_atomic_authoring(
        &self,
        o: &mut SourceCoordinator,
        q: &NormalizedAtomicAuthoringArgumentsV1<'_>,
        f: &TrustedAtomicAuthoringV1<'_>,
        a: &TrustedCompoundAnnotationResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        assert_eq!(self.family, Family::AtomicAuthoring);
        let p = prepare(o, &self.scope, self.family, r);
        o.submit_discovered_atomic_authoring_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &self.context,
                scope: &self.scope,
                cut: &p,
                entry: &self.entries[0].0,
                variant: &self.entries[0].1[0],
                facts: &declaration::facts(),
            },
            q,
            f,
            a,
            r,
        )
    }
}
pub fn submit(
    route: Option<&Route>,
    o: &mut SourceCoordinator,
    q: &NormalizedSelectionArgumentsV1<'_>,
    r: &mut Recorder,
) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
    match route {
        Some(route) => route.submit(o, q, r),
        None => o
            .submit_selection_arguments_v1(q, r)
            .map_err(DiscoveredNativeCommandErrorV1::Invocation),
    }
}
