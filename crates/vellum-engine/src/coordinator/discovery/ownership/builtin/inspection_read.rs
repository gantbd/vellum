//! Installed F03/F04 queries delegate to the existing funded read owners.
use super::*;
use crate::coordinator::source_context::facade::{annotations, history};
use source_read::{record_dispatch, with_variant};
#[cfg(test)]
mod tests;

fn with_input<T>(
    authority: &str,
    command: &str,
    history: bool,
    visibility: &NativeBuiltinEntryVisibilityV1<3>,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    let (contract, operations, keys) = if history {
        (
            "history-query/1",
            ["frontier", "availability", "permitted-blocking-detail"],
            [
                &[
                    "contract",
                    "operation",
                    "context",
                    "scope",
                    "direction",
                    "profile",
                ][..],
                &[
                    "contract",
                    "operation",
                    "context",
                    "scope",
                    "direction",
                    "profile",
                    "group",
                ][..],
                &[
                    "contract",
                    "operation",
                    "context",
                    "scope",
                    "direction",
                    "profile",
                    "group",
                ][..],
            ],
        )
    } else {
        (
            "annotation-query/1",
            ["enumerate", "inspect", "target-location"],
            [
                &[
                    "contract",
                    "operation",
                    "snapshot",
                    "document",
                    "profile",
                    "limit",
                    "continuation",
                ][..],
                &[
                    "contract",
                    "operation",
                    "snapshot",
                    "document",
                    "profile",
                    "annotation",
                ][..],
                &[
                    "contract",
                    "operation",
                    "snapshot",
                    "document",
                    "profile",
                    "annotation",
                ][..],
            ],
        )
    };
    // Three live declaration frames + one entry/two strings fit the prepaid
    // source_read frame (four variants + three entries/four strings). These
    // synchronous exclusive-owner preparations cannot coexist.
    with_variant(
        contract,
        operations[0],
        keys[0],
        visibility.variants[0],
        |first| {
            with_variant(
                contract,
                operations[1],
                keys[1],
                visibility.variants[1],
                |second| {
                    with_variant(
                        contract,
                        operations[2],
                        keys[2],
                        visibility.variants[2],
                        |third| {
                            let variants = [first, second, third];
                            let entries = [DiscoveryEntryInputV1 {
                                command: DiscoveryCommandV1 {
                                    authority,
                                    id: command,
                                    incarnation: 1,
                                },
                                contract,
                                version: 1,
                                kind: DiscoveryExecutionKindV1::Query,
                                variants: &variants,
                                presentation: D::Object(&[]),
                                order: visibility.order,
                                schema_visible: visibility.schema,
                                presentation_visible: visibility.presentation,
                                advice_visible: visibility.advice,
                            }];
                            consume(DiscoveryCutInputV1 {
                                entries: &entries,
                                target: None,
                                advice_present: false,
                            })
                        },
                    )
                },
            )
        },
    )
}
impl SourceCoordinator {
    pub fn prepare_native_annotation_read_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<3>,
        facts: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        self.prepare_inspection_read_discovery(scope, visibility, false, facts, r)
    }
    pub fn prepare_native_history_read_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<3>,
        facts: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        self.prepare_inspection_read_discovery(scope, visibility, true, facts, r)
    }
    fn prepare_inspection_read_discovery(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<3>,
        history: bool,
        facts: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        let timing = r.begin_service_timing();
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
                if history {
                    "F04-history-read-v1"
                } else {
                    "F03-annotation-read-v1"
                },
                store.limits.max_context_bytes,
            )?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            with_input(&authority, &command, history, visibility, |input| {
                self.prepare_discovery_cut_from_source(
                    scope,
                    input,
                    &facts,
                    Origin::NativeInspectionReadV1,
                    r,
                )
            })
        })();
        self.discovery_finish_timed(
            r,
            Boundary::DiscoveryBuiltinPreparation,
            &result,
            timing.finish(),
        );
        result
    }
    fn authorize_discovered_inspection_read(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        contract: &str,
        operation: &str,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let recognized = self.discovery_context(route.context).and_then(|c| {
            queries::inspection_read_selector_matches(
                c,
                route.entry,
                route.variant,
                contract,
                operation,
            )
        });
        observe(r, Boundary::DiscoveryRequestRecognition, &recognized);
        recognized?;
        self.query_discovery_v1(
            route.context,
            route.scope,
            route.cut,
            DiscoveryQueryV1::Inspect { entry: route.entry },
            route.facts,
            r,
        )?;
        Ok(())
    }
    pub fn query_discovered_annotations_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        request: DecodedAnnotationFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Result<AnnotationFacadeReplyV1, DiscoveredSourceReadErrorV1> {
        type E = DiscoveredSourceReadErrorV1;
        let timing = r.begin_service_timing();
        let result = (|| {
            let operation =
                annotations::arguments::installed_selector(request.envelope).map_err(E::Native)?;
            self.authorize_discovered_inspection_read(route, "annotation-query/1", operation, r)
                .map_err(E::Discovery)?;
            self.try_query_decoded_annotation_facade_v1(request, r)
                .map_err(E::Native)
        })();
        record_dispatch(
            r,
            Boundary::DiscoveryAnnotationReadDispatch,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn query_discovered_history_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        request: DecodedHistoryFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Result<HistoryInspectionReplyV1, DiscoveredSourceReadErrorV1> {
        type E = DiscoveredSourceReadErrorV1;
        let timing = r.begin_service_timing();
        let result = (|| {
            let operation =
                history::arguments::installed_selector(request.envelope).map_err(E::Native)?;
            self.authorize_discovered_inspection_read(route, "history-query/1", operation, r)
                .map_err(E::Discovery)?;
            self.try_query_decoded_history_facade_v1(request, r)
                .map_err(E::Native)
        })();
        record_dispatch(
            r,
            Boundary::DiscoveryHistoryReadDispatch,
            &result,
            timing.finish(),
        );
        result
    }
}
