//! Explicit sealed F05 query source. Does not append rows to older builtin
//! discovery profiles or turn metadata visibility into source-read authority.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredLiteralSearchErrorV1 {
    Discovery(DiscoveryErrorV1),
    Search(BoundLiteralSearchErrorV1),
}
impl SourceCoordinator {
    pub fn prepare_native_literal_search_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<3>,
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
            if store.limits.max_entries == 0 || store.limits.max_variants < 3 {
                return Err(Error::Limit);
            }
            let authority = text(&store.authority)?;
            let command = command(
                &store.engine,
                "F05-literal-search-v1",
                store.limits.max_context_bytes,
            )?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            // Three <=13-field variants/one entry fit the existing prepaid
            // selection frame (four variants/two entries). No budget increase.
            selection::with_variant(
                LiteralSearchOperationV1::LiteralMatches.argument_declaration_v1(),
                visibility.variants[0],
                |matches| {
                    selection::with_variant(
                        LiteralSearchOperationV1::NextMatch.argument_declaration_v1(),
                        visibility.variants[1],
                        |next| {
                            selection::with_variant(
                                LiteralSearchOperationV1::PreviousMatch.argument_declaration_v1(),
                                visibility.variants[2],
                                |previous| {
                                    let variants=[matches,next,previous].map(|mut v| {
                            v.outcomes=D::Text("Original native retained literal result; separately authorized matches/navigation with logical source reference, matching profile and request generation; no edit authority");
                            v.limits=D::Text("Bounded native input, correspondence capture, request/capture/reply, comparison and match limits; explicit literal-exact/1 only, unsupported profiles do not fall back");
                            v.cancellation=D::Text("Bounded synchronous query quantum, absorbing observed correspondence/native loss and protected original result retirement; no scheduler or provider cancellation");
                            v
                        });
                                    let entries = [DiscoveryEntryInputV1 {
                                        command: DiscoveryCommandV1 {
                                            authority: &authority,
                                            id: &command,
                                            incarnation: 1,
                                        },
                                        contract: "literal-search/1",
                                        version: 1,
                                        kind: DiscoveryExecutionKindV1::Query,
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
                                        Origin::NativeLiteralSearchV1,
                                        r,
                                    )
                                },
                            )
                        },
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
    /// Recognize original installed selectors, release current metadata, then
    /// invoke the same bound native owner with separately supplied read facts.
    #[allow(clippy::too_many_arguments)]
    pub fn query_discovered_bound_literal_search_v1<'a>(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        binding: &'a LogicalSearchBindingV1<'a>,
        q: &'a NormalizedLiteralSearchArgumentsV1<'a>,
        sf: SearchReadFactsV1<'_>,
        facts: &TrustedSourceSearchFactsV1<'_>,
        correspondence_current: Option<bool>,
        r: &mut Recorder,
    ) -> Result<BoundLiteralSearchResultV1<'a>, DiscoveredLiteralSearchErrorV1> {
        type E = DiscoveredLiteralSearchErrorV1;
        let timer = r.begin_service_timing();
        let result = (|| {
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::literal_search_selector_matches(
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
            self.query_bound_literal_search_v1(binding, q, sf, facts, correspondence_current, r)
                .map_err(E::Search)
        })();
        r.record(Observation {
            boundary: Boundary::DiscoveryLiteralSearchDispatch,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(E::Discovery(Error::Faulted))
                | Err(E::Search(BoundLiteralSearchErrorV1::Native(
                    SourceReadContextErrorV1::Faulted,
                ))) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: timer.finish(),
        });
        result
    }
}
