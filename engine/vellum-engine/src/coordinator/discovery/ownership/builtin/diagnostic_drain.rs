//! One sealed existing F26 hostRequest variant. Never an editor invocation job.
use super::*;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredDiagnosticDrainErrorV1 {
    Discovery(DiscoveryErrorV1),
    Drain(DiagnosticDrainErrorV1),
}
impl SourceCoordinator {
    pub fn prepare_native_diagnostic_drain_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<1>,
        facts: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V1
                || !self.native_diagnostic_drain_profile_present()
            {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, facts, r)?;
            let store = self.discovery_store()?;
            if store.limits.max_entries == 0 || store.limits.max_variants == 0 {
                return Err(Error::Limit);
            }
            let authority = text(&store.authority)?;
            let command = command(
                &store.engine,
                "F26-diagnostic-drain-v1",
                store.limits.max_context_bytes,
            )?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            // One four-field variant fits the existing prepaid builtin frame.
            selection::with_variant(
                native_diagnostic_drain_argument_declaration_v1(),
                visibility.variants[0],
                |mut variant| {
                    variant.outcomes = D::Text(
                        "Original issued task and one-shot local drain; current separately authorized copied result and protected control; performed is not source publication, transport delivery or durable export",
                    );
                    variant.limits = D::Text(
                        "Original generation/limit intent, actual queue reply bound and prepaid native task/batch/result/control capacity; aggregate parent funding, distinct live/lifetime limits and explicit consumer retirement",
                    );
                    let variants = [variant];
                    let entries = [DiscoveryEntryInputV1 {
                        command: DiscoveryCommandV1 {
                            authority: &authority,
                            id: &command,
                            incarnation: 1,
                        },
                        contract: native_diagnostic_drain_argument_declaration_v1().contract,
                        version: 1,
                        kind: DiscoveryExecutionKindV1::EffectRequest,
                        variants: &variants,
                        presentation: D::Object(&[]),
                        order: visibility.order,
                        schema_visible: visibility.schema,
                        presentation_visible: visibility.presentation,
                        advice_visible: visibility.advice,
                    }];
                    self.prepare_discovery_cut_from_source(
                        scope,
                        DiscoveryCutInputV1 {
                            entries: &entries,
                            target: None,
                            advice_present: false,
                        },
                        &facts,
                        Origin::NativeDiagnosticDrainV1,
                        r,
                    )
                },
            )
        })();
        self.discovery_finish_timed(
            r,
            Boundary::DiscoveryBuiltinPreparation,
            &result,
            timer.finish(),
        );
        result
    }
    pub fn submit_discovered_diagnostic_drain_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        q: &NormalizedDiagnosticDrainArgumentsV1<'_>,
        scope: &DiagnosticScopeV1,
        f: &TrustedDiagnosticDrainFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiagnosticDrainPhaseV1, DiscoveredDiagnosticDrainErrorV1> {
        type E = DiscoveredDiagnosticDrainErrorV1;
        let timer = r.begin_service_timing();
        let result = (|| {
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::diagnostic_drain_selector_matches(c, route.entry, route.variant)
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
            self.submit_diagnostic_drain_arguments_v1(q, scope, f, r)
                .map_err(E::Drain)
        })();
        r.record(Observation {
            boundary: Boundary::DiscoveryDiagnosticDrainDispatch,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(E::Discovery(Error::Faulted))
                | Err(E::Drain(DiagnosticDrainErrorV1::Faulted)) => Disposition::Unknown,
                _ => Disposition::Refused,
            },
            service_time: timer.finish(),
        });
        result
    }
}
