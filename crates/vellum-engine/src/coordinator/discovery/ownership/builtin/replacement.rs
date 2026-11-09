//! Sealed original-returned-result replacement source, not a logical F05 resolver.
use super::*;
#[cfg(test)]
mod tests;
fn describe(mut v: DiscoveryVariantInputV1<'_>) -> DiscoveryVariantInputV1<'_> {
    v.outcomes = D::Text(
        "One original returned-result replacement through the existing managed-search publisher; retained invocation outcome, no new operation-specific result reference",
    );
    v.limits = D::Text(
        "Original canonical request/context/three-limit capture plus actual native returned reply frame, separately reserved under actual remaining parent capacity; complete native plan and existing source/annotation/view/history bounds",
    );
    v
}
impl SourceCoordinator {
    pub fn prepare_native_replacement_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<2>,
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
            if store.limits.max_entries == 0 || store.limits.max_variants < 2 {
                return Err(Error::Limit);
            }
            let authority = text(&store.authority)?;
            let command = command(
                &store.engine,
                "F11-replacement-v1",
                store.limits.max_context_bytes,
            )?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            let declarations = native_replacement_argument_declarations_v1();
            // Two four-field variants and one entry fit the existing prepaid
            // synchronous selection declaration maximum. No new holding pool.
            selection::with_variant(&declarations[0], visibility.variants[0], |current| {
                selection::with_variant(&declarations[1], visibility.variants[1], |all| {
                    let variants = [describe(current), describe(all)];
                    let entries = [DiscoveryEntryInputV1 {
                        command: DiscoveryCommandV1 {
                            authority: &authority,
                            id: &command,
                            incarnation: 1,
                        },
                        contract: declarations[0].contract,
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
                        Origin::NativeReplacementV1,
                        r,
                    )
                })
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
    /// Trusted embedding dispatch: metadata cannot supply search/source/mutation
    /// permission, replace a returned result, or refresh the original plan.
    #[allow(clippy::too_many_arguments)]
    pub fn submit_discovered_native_replacement_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        q: &NormalizedReplacementArgumentsV1<'_>,
        scope: &SourceReadScopeV1,
        sf: SearchReadFactsV1<'_>,
        f: &TrustedSourceSearchFactsV1<'_>,
        cf: &TrustedSourceSearchConsumerFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        type E = DiscoveredNativeCommandErrorV1;
        let timer = r.begin_service_timing();
        let result = (|| {
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::replacement_selector_matches(
                    c,
                    route.entry,
                    route.variant,
                    q.argument_declaration_v1(),
                )
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
            self.submit_replacement_arguments_v1(q, scope, sf, f, cf, r)
                .map_err(E::Invocation)
        })();
        r.record(Observation {
            boundary: Boundary::DiscoveryReplacementDispatch,
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
