//! Sealed source-only F11 adapter; never an annotation-authoring grant.
use super::*;
impl SourceCoordinator {
    pub fn prepare_native_exact_batch_discovery_cut_v1(
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
                "F11-exact-batch-v1",
                store.limits.max_context_bytes,
            )?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            // One seven-field variant/entry and two strings fit the existing
            // prepaid selection frame (four <=13-field variants/two entries).
            selection::with_variant(
                NormalizedExactBatchArgumentsV1::ARGUMENT_DECLARATION,
                visibility.variants[0],
                |mut variant| {
                    variant.outcomes = D::Text(
                        "One atomic explicit source batch with automatic annotation placement, linked history and current-service views; original retained invocation outcome, no annotation-authoring or created-result grant",
                    );
                    variant.limits = D::Text(
                        "Explicit native request/context/capture limits; native read/scope interpretation budgets; complete retained preparation and current-service view envelope; existing source, annotation, history and root bounds",
                    );
                    let variants = [variant];
                    let entries = [DiscoveryEntryInputV1 {
                        command: DiscoveryCommandV1 {
                            authority: &authority,
                            id: &command,
                            incarnation: 1,
                        },
                        contract: "exact-source-batch/1",
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
                        Origin::NativeExactBatchV1,
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
    /// Trusted host authorizes admission/service separately. Metadata visibility
    /// cannot grant edit rights or disclose engine-private annotation stamps.
    pub fn submit_discovered_native_exact_batch_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        q: &NormalizedExactBatchArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        type E = DiscoveredNativeCommandErrorV1;
        let timer = r.begin_service_timing();
        let result = (|| {
            let recognized = self
                .discovery_context(route.context)
                .and_then(|c| queries::exact_batch_selector_matches(c, route.entry, route.variant));
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
            self.submit_exact_batch_arguments_v1(q, r)
                .map_err(E::Invocation)
        })();
        r.record(Observation {
            boundary: Boundary::DiscoveryExactBatchDispatch,
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
