//! Installed native-operation test route; no second observation owner.
#![allow(dead_code)]
use crate::discovery_support as discovery;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
fn visible<const N: usize>(order: usize) -> NativeBuiltinEntryVisibilityV1<N> {
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
fn cut(o: &mut SourceCoordinator, scope: &DiscoveryScopeV1, r: &mut Recorder) -> DiscoveryCutIdV1 {
    o.prepare_native_operation_discovery_cut_v1(
        scope,
        &visible(0),
        &visible(1),
        &discovery::facts(),
        r,
    )
    .unwrap()
}
pub struct Route {
    scope: DiscoveryScopeV1,
    context: DiscoveryContextIdV1,
    entries: Vec<(DiscoveryEntryIdV1, Vec<DiscoveryVariantIdV1>)>,
    charge: usize,
}
impl Route {
    pub fn install(
        o: &mut SourceCoordinator,
        c: &SettingsExecutionContextV1,
        r: &mut Recorder,
    ) -> Self {
        let scope = DiscoveryScopeV1 {
            context: SettingsExecutionContextV1 {
                profile: OWNED_CONTRACT_DISCOVERY_PROFILE_V1.into(),
                ..c.clone()
            },
            policy_epoch: 1,
        };
        let mut l = discovery::limits();
        l.max_contexts = 1;
        l.max_preparations = 1;
        l.max_entries = 2;
        l.max_variants = 3;
        l.max_selectors = 16;
        l.max_issued = 64;
        l.max_page_rows = 2;
        l.max_context_bytes = 256;
        l.max_capture_bytes = 32768;
        l.max_packet_bytes = 32768;
        l.max_reply_bytes = 32768;
        l.max_row_bytes = 32768;
        l.max_root_bytes = 65536;
        l.metadata.max_bytes = 8192;
        l.metadata.max_nodes = 512;
        l.metadata.max_depth = 12;
        l.max_held_bytes = l.declared_bytes().unwrap();
        let before = o.retention_usage().reserved;
        o.configure_discovery_v1(&scope, l, &discovery::facts(), r)
            .unwrap();
        let p = cut(o, &scope, r);
        let context = o
            .acquire_discovery_v1(&scope, &p, &discovery::facts(), r)
            .unwrap()
            .context;
        let p = cut(o, &scope, r);
        let DiscoveryReplyV1::Page { entries, .. } = o
            .query_discovery_v1(
                &context,
                &scope,
                &p,
                DiscoveryQueryV1::List {
                    limit: 2,
                    continuation: None,
                },
                &discovery::facts(),
                r,
            )
            .unwrap()
        else {
            panic!("page")
        };
        let entries = entries
            .into_iter()
            .map(|e| {
                let p = cut(o, &scope, r);
                let DiscoveryReplyV1::Descriptor { descriptor, .. } = o
                    .query_discovery_v1(
                        &context,
                        &scope,
                        &p,
                        DiscoveryQueryV1::Inspect {
                            entry: &e.reference,
                        },
                        &discovery::facts(),
                        r,
                    )
                    .unwrap()
                else {
                    panic!("descriptor")
                };
                (
                    e.reference,
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
            charge: o.retention_usage().reserved - before,
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn query(
        &self,
        o: &mut SourceCoordinator,
        id: &NativeOperationObservationIdV1,
        envelope: &NativeOperationEnvelopeValueV1<'_, '_>,
        policy: &NativeOperationObservationPolicyV1,
        facts: &NativeOperationObservationFactsV1<'_>,
        clock: Option<NativeOperationWaitClockV1>,
        r: &mut Recorder,
    ) -> Result<NativeOperationDecodedReplyV1, DiscoveredNativeOperationErrorV1> {
        let d = recognize_native_operation_envelope_v1(envelope).unwrap();
        let (entry, variant) = match d.variant.operation {
            "status" => (0, 0),
            "minimal-control-status" => (0, 1),
            "bounded-observation-wait" => (0, 2),
            "retained-outcome" => (1, 0),
            _ => panic!("closed operation"),
        };
        let p = cut(o, &self.scope, r);
        o.query_discovered_native_operation_v1(
            NativeDiscoveredCommandRouteV1 {
                context: &self.context,
                scope: &self.scope,
                cut: &p,
                entry: &self.entries[entry].0,
                variant: &self.entries[entry].1[variant],
                facts: &discovery::facts(),
            },
            id,
            envelope,
            policy,
            facts,
            clock,
            r,
        )
    }
    pub fn retire(self, o: &mut SourceCoordinator, r: &mut Recorder) {
        let before = o.retention_usage().reserved;
        o.stop_discovery_context_v1(&self.context, &self.scope, &discovery::facts(), r)
            .unwrap();
        o.retire_discovery_context_v1(&self.context, &discovery::facts(), r)
            .unwrap();
        o.stop_discovery_admission_v1(&discovery::facts(), r)
            .unwrap();
        o.retire_discovery_registry_v1(&discovery::facts(), r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, before - self.charge);
    }
}
