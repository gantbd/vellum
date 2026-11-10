//! Installed F06 read/save observation, not a new task or invocation owner.
use super::*;
use crate::coordinator::resource::{
    native_operation_argument_declarations_v1, recognize_native_operation_envelope_v1,
};
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredNativeOperationErrorV1 {
    Discovery(DiscoveryErrorV1),
    Native(NativeOperationObservationErrorV1),
}
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    add(
        selection::workspace_bytes(maximum)?,
        mul(2, size_of::<DiscoveryVariantInputV1<'_>>())?,
    )
}
fn with_input<T>(
    authority: &str,
    commands: &[String; 2],
    observation: &NativeBuiltinEntryVisibilityV1<3>,
    receipt: &NativeBuiltinEntryVisibilityV1<1>,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    // Reuse the four fixed selection builders, with separately prepaid local
    // variant-enrichment frames. This is logical staging, not stack/RSS accounting.
    let d = native_operation_argument_declarations_v1();
    selection::with_variant(d[0], observation.variants[0], |status| {
        selection::with_variant(d[2], observation.variants[1], |control| {
            selection::with_variant(d[3], observation.variants[2], |wait| {
                selection::with_variant(d[1], receipt.variants[0], |result| {
                    fn enrich(mut v: DiscoveryVariantInputV1<'_>) -> DiscoveryVariantInputV1<'_> {
                        v.outcomes = D::Text(
                            "Original native source/annotation read or save metadata, fixed progress/control reference or bounded waiter; no bytes, new dispatch, activation or baseline adoption",
                        );
                        v.limits = D::Text(
                            "Original observation/decoder/parent bounds, current class-specific release and closed receipt facet; independent control and nonrenewable wait rank",
                        );
                        v.cancellation = D::Text(
                            "Observation stop never cancels producer work; original protected wait/context/pool retirement remains mandatory",
                        );
                        v
                    }
                    let observations = [enrich(status), enrich(control), enrich(wait)];
                    let receipts = [enrich(result)];
                    let entries = [
                        DiscoveryEntryInputV1 {
                            command: DiscoveryCommandV1 {
                                authority,
                                id: &commands[0],
                                incarnation: 1,
                            },
                            contract: "native-operation-observation/1",
                            version: 1,
                            kind: DiscoveryExecutionKindV1::Query,
                            variants: &observations,
                            presentation: D::Object(&[]),
                            order: observation.order,
                            schema_visible: observation.schema,
                            presentation_visible: observation.presentation,
                            advice_visible: observation.advice,
                        },
                        DiscoveryEntryInputV1 {
                            command: DiscoveryCommandV1 {
                                authority,
                                id: &commands[1],
                                incarnation: 1,
                            },
                            contract: "native-operation-receipt/1",
                            version: 1,
                            kind: DiscoveryExecutionKindV1::Query,
                            variants: &receipts,
                            presentation: D::Object(&[]),
                            order: receipt.order,
                            schema_visible: receipt.schema,
                            presentation_visible: receipt.presentation,
                            advice_visible: receipt.advice,
                        },
                    ];
                    consume(DiscoveryCutInputV1 {
                        entries: &entries,
                        target: None,
                        advice_present: false,
                    })
                })
            })
        })
    })
}
impl SourceCoordinator {
    pub fn prepare_native_operation_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        observation: &NativeBuiltinEntryVisibilityV1<3>,
        receipt: &NativeBuiltinEntryVisibilityV1<1>,
        facts: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V1
                || self.native_operation_observation_pool_usage_v1().is_err()
            {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, facts, r)?;
            let s = self.discovery_store()?;
            if s.limits.max_entries < 2 || s.limits.max_variants < 3 {
                return Err(Error::Limit);
            }
            let authority = text(&s.authority)?;
            let commands = ["F06-native-operation-v1", "F06-native-operation-receipt-v1"]
                .map(|n| command(&s.engine, n, s.limits.max_context_bytes));
            let [a, b] = commands;
            let commands = [a?, b?];
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            with_input(&authority, &commands, observation, receipt, |input| {
                self.prepare_discovery_cut_from_source(
                    scope,
                    input,
                    &facts,
                    Origin::NativeOperationObservationV1,
                    r,
                )
            })
        })();
        self.discovery_finish_timed(
            r,
            Boundary::DiscoveryBuiltinPreparation,
            &result,
            timer.finish(),
        );
        result
    }
    #[allow(clippy::too_many_arguments)]
    pub fn query_discovered_native_operation_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        id: &NativeOperationObservationIdV1,
        envelope: &NativeOperationEnvelopeValueV1<'_, '_>,
        policy: &NativeOperationObservationPolicyV1,
        facts: &NativeOperationObservationFactsV1<'_>,
        clock: Option<NativeOperationWaitClockV1>,
        r: &mut Recorder,
    ) -> Result<NativeOperationDecodedReplyV1, DiscoveredNativeOperationErrorV1> {
        type E = DiscoveredNativeOperationErrorV1;
        let timer = r.begin_service_timing();
        let result = (|| {
            let d = recognize_native_operation_envelope_v1(envelope).map_err(E::Native)?;
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::native_operation_selector_matches(c, route.entry, route.variant, d)
            });
            observe(r, Boundary::DiscoveryRequestRecognition, &recognized);
            recognized.map_err(E::Discovery)?;
            self.query_discovery_v1(
                route.context,
                route.scope,
                route.cut,
                DiscoveryQueryV1::Inspect { entry: route.entry },
                route.facts,
                r,
            )
            .map_err(E::Discovery)?;
            self.query_decoded_native_operation_v1(id, envelope, policy, facts, clock, r)
                .map_err(E::Native)
        })();
        r.record(Observation {
            boundary: Boundary::DiscoveryNativeOperationDispatch,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(E::Discovery(Error::Faulted))
                | Err(E::Native(NativeOperationObservationErrorV1::Faulted)) => {
                    Disposition::Unknown
                }
                _ => Disposition::Refused,
            },
            service_time: timer.finish(),
        });
        result
    }
}
