//! Sealed mixed command. Metadata is never source or component-write authority.
use super::*;
impl SourceCoordinator {
    pub fn prepare_native_atomic_authoring_discovery_cut_v1(
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
            self.annotation_profile().map_err(|_| Error::Unavailable)?;
            let store = self.discovery_store()?;
            if store.limits.max_entries == 0 || store.limits.max_variants == 0 {
                return Err(Error::Limit);
            }
            let authority = text(&store.authority)?;
            let command = command(
                &store.engine,
                "F11-atomic-authoring-v1",
                store.limits.max_context_bytes,
            )?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            // One eight-field variant/entry/two strings fits the already prepaid
            // owner-exclusive selection frame; no additional registry pool.
            selection::with_variant(
                NormalizedAtomicAuthoringArgumentsV1::ARGUMENT_DECLARATION,
                visibility.variants[0],
                |mut variant| {
                    variant.outcomes = D::Text(
                        "One atomic source/annotation/current-view/history publication, original ordered component receipts, or no publication; original create identities never refreshed by replay",
                    );
                    variant.limits = D::Text(
                        "Explicit native input graph limits; actual parent prepays 3C+2M+2R and original receipt hold; complete bundle/native plan/root bounds and every original component profile",
                    );
                    let variants = [variant];
                    let entries = [DiscoveryEntryInputV1 {
                        command: DiscoveryCommandV1 {
                            authority: &authority,
                            id: &command,
                            incarnation: 1,
                        },
                        contract: "atomic-source-annotation/1",
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
                        Origin::NativeAtomicAuthoringV1,
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
    pub fn submit_discovered_atomic_authoring_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        q: &NormalizedAtomicAuthoringArgumentsV1<'_>,
        f: &TrustedAtomicAuthoringV1<'_>,
        a: &TrustedCompoundAnnotationResultAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        type E = DiscoveredNativeCommandErrorV1;
        let timer = r.begin_service_timing();
        let result = (|| {
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::atomic_authoring_selector_matches(c, route.entry, route.variant)
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
            self.submit_atomic_authoring_arguments_v1(q, f, a, r)
                .map_err(E::Invocation)
        })();
        r.record(Observation {
            boundary: Boundary::DiscoveryAtomicAuthoringDispatch,
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
