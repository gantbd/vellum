//! Test-only installed read binding. No SDK policy or engine owner.
#![allow(dead_code)]
use crate::discovered_context as declaration;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
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
#[derive(Clone, Copy)]
enum Family {
    Annotation,
    History,
    Advice,
}
pub struct Reader {
    scope: DiscoveryScopeV1,
    context: DiscoveryContextIdV1,
    entry: DiscoveryEntryIdV1,
    variants: Vec<DiscoveryVariantIdV1>,
    family: Family,
}
impl Reader {
    fn prepare(
        o: &mut SourceCoordinator,
        scope: &DiscoveryScopeV1,
        family: Family,
        r: &mut Recorder,
    ) -> DiscoveryCutIdV1 {
        if matches!(family, Family::Advice) {
            let v = visibility();
            let advice = NativeBuiltinEntryVisibilityV1 {
                order: v.order,
                schema: v.schema,
                presentation: v.presentation,
                advice: v.advice,
                variants: [v.variants[0]],
            };
            return o
                .prepare_native_compound_advice_discovery_cut_v1(
                    scope,
                    &advice,
                    &declaration::facts(),
                    r,
                )
                .unwrap();
        }
        if matches!(family, Family::History) {
            o.prepare_native_history_read_discovery_cut_v1(
                scope,
                &visibility(),
                &declaration::facts(),
                r,
            )
        } else {
            o.prepare_native_annotation_read_discovery_cut_v1(
                scope,
                &visibility(),
                &declaration::facts(),
                r,
            )
        }
        .unwrap()
    }
    pub fn install(
        o: &mut SourceCoordinator,
        actor: &SettingsExecutionContextV1,
        history: bool,
        r: &mut Recorder,
    ) -> Self {
        Self::install_family(
            o,
            actor,
            if history {
                Family::History
            } else {
                Family::Annotation
            },
            r,
        )
    }
    pub fn install_advice(
        o: &mut SourceCoordinator,
        actor: &SettingsExecutionContextV1,
        r: &mut Recorder,
    ) -> Self {
        Self::install_family(o, actor, Family::Advice, r)
    }
    fn install_family(
        o: &mut SourceCoordinator,
        actor: &SettingsExecutionContextV1,
        family: Family,
        r: &mut Recorder,
    ) -> Self {
        let scope = DiscoveryScopeV1 {
            context: SettingsExecutionContextV1 {
                profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                ..actor.clone()
            },
            policy_epoch: 1,
        };
        let mut l = declaration::limits();
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
        o.configure_discovery_v1(&scope, l, &declaration::facts(), r)
            .unwrap();
        let p = Self::prepare(o, &scope, family, r);
        let context = o
            .acquire_discovery_v1(&scope, &p, &declaration::facts(), r)
            .unwrap()
            .context;
        let p = Self::prepare(o, &scope, family, r);
        let DiscoveryReplyV1::Page { mut entries, .. } = o
            .query_discovery_v1(
                &context,
                &scope,
                &p,
                DiscoveryQueryV1::List {
                    limit: 1,
                    continuation: None,
                },
                &declaration::facts(),
                r,
            )
            .unwrap()
        else {
            panic!()
        };
        let entry = entries.remove(0).reference;
        let p = Self::prepare(o, &scope, family, r);
        let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
            .query_discovery_v1(
                &context,
                &scope,
                &p,
                DiscoveryQueryV1::Inspect { entry: &entry },
                &declaration::facts(),
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
            variants: descriptor
                .variants
                .into_iter()
                .map(|v| v.reference)
                .collect(),
            family,
        }
    }
    fn route<'a>(
        &'a self,
        p: &'a DiscoveryCutIdV1,
        index: usize,
        facts: &'a TrustedDiscoveryFactsV1,
    ) -> NativeDiscoveredCommandRouteV1<'a> {
        NativeDiscoveredCommandRouteV1 {
            context: &self.context,
            scope: &self.scope,
            cut: p,
            entry: &self.entry,
            variant: &self.variants[index],
            facts,
        }
    }
    pub fn annotations(
        &self,
        o: &mut SourceCoordinator,
        index: usize,
        request: DecodedAnnotationFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Option<AnnotationFacadeReplyV1> {
        let p = Self::prepare(o, &self.scope, self.family, r);
        o.query_discovered_annotations_v1(self.route(&p, index, &declaration::facts()), request, r)
            .ok()
    }
    pub fn history(
        &self,
        o: &mut SourceCoordinator,
        index: usize,
        request: DecodedHistoryFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Option<HistoryInspectionReplyV1> {
        let p = Self::prepare(o, &self.scope, self.family, r);
        o.query_discovered_history_v1(self.route(&p, index, &declaration::facts()), request, r)
            .ok()
    }
    pub fn advice(
        &self,
        o: &mut SourceCoordinator,
        envelope: &CompoundAdviceEnvelopeValueV1<'_, '_>,
        facts: &TrustedCompoundAdviceFactsV1<'_>,
        r: &mut Recorder,
    ) -> CompoundAdviceReplyV1 {
        let p = Self::prepare(o, &self.scope, self.family, r);
        o.query_discovered_compound_advice_v1(
            self.route(&p, 0, &declaration::facts()),
            envelope,
            facts,
            r,
        )
        .unwrap()
    }
    pub fn retire(self, o: &mut SourceCoordinator, r: &mut Recorder) {
        o.stop_discovery_context_v1(&self.context, &self.scope, &declaration::facts(), r)
            .unwrap();
        o.retire_discovery_context_v1(&self.context, &declaration::facts(), r)
            .unwrap();
        o.stop_discovery_admission_v1(&declaration::facts(), r)
            .unwrap();
        o.retire_discovery_registry_v1(&declaration::facts(), r)
            .unwrap();
    }
}
