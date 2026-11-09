//! Sealed accepted-composition adapter. Discovery does not authorize mutation.
use super::*;
impl SourceCoordinator {
    pub fn prepare_native_composition_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<1>,
        facts: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V1 {
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
                "F11-composition-v1",
                store.limits.max_context_bytes,
            )?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            // One ten-field variant, one entry and two strings fit the existing
            // prepaid selection frame (four <=13-field variants/two entries).
            selection::with_variant(
                NormalizedCompositionArgumentsV1::ARGUMENT_DECLARATION,
                visibility.variants[0],
                |mut variant| {
                    variant.outcomes = D::Text(
                        "One exact source replacement and original-primary collapse through the shared atomic publisher; ordinary retained invocation outcome, no operation-specific result reference",
                    );
                    variant.limits = D::Text(
                        "Original request/context/three-limit capture; actual invocation intent ceiling including complete retained native preparation; existing source, annotation, view, history and root budgets",
                    );
                    let variants = [variant];
                    let entries = [DiscoveryEntryInputV1 {
                        command: DiscoveryCommandV1 {
                            authority: &authority,
                            id: &command,
                            incarnation: 1,
                        },
                        contract: "composition-commit/1",
                        version: 1,
                        kind: DiscoveryExecutionKindV1::LocalCommand,
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
                        Origin::NativeCompositionV1,
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
    /// Trusted-host port. Host permission to edit remains independent of schema
    /// visibility and current recipient result disclosure permission.
    pub fn submit_discovered_native_composition_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        q: &NormalizedCompositionArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        type E = DiscoveredNativeCommandErrorV1;
        let timer = r.begin_service_timing();
        let result = (|| {
            let recognized = self
                .discovery_context(route.context)
                .and_then(|c| queries::composition_selector_matches(c, route.entry, route.variant));
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
            self.submit_composition_arguments_v1(q, r)
                .map_err(E::Invocation)
        })();
        r.record(Observation {
            boundary: Boundary::DiscoveryCompositionDispatch,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(E::Discovery(Error::Faulted))
                | Err(E::Invocation(InvocationError::Faulted)) => Disposition::Unknown,
                _ => Disposition::Refused,
            },
            service_time: timer.finish(),
        });
        result
    }
}
