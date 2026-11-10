//! Installed F02 adapter over the existing funded read/acquisition facades.
use super::*;
use crate::coordinator::source_context::facade::{acquisition, arguments};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredSourceReadErrorV1 {
    Discovery(DiscoveryErrorV1),
    Native(SourceReadContextErrorV1),
}

pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    // Three command strings and four simultaneously live, fixed declaration
    // frames. Not caller-sized payload capture or a physical stack/RSS claim.
    [
        mul(4, maximum)?,
        size_of::<[String; 4]>(),
        4 * variant_workspace_bytes(),
        size_of::<[DiscoveryEntryInputV1<'_>; 3]>(),
        size_of::<DiscoveryCutInputV1<'_>>(),
        size_of::<TrustedDiscoveryFactsV1>(),
    ]
    .into_iter()
    .try_fold(0, add)
}

pub(super) fn variant_workspace_bytes() -> usize {
    size_of::<[[(&str, D<'_>); 7]; 3]>()
        + size_of::<[(&str, D<'_>); 5]>()
        + size_of::<[(&str, D<'_>); 3]>() * 2
        + size_of::<[D<'_>; 7]>()
        + size_of::<[(&str, D<'_>); 1]>() * 2
        + size_of::<DiscoveryVariantInputV1<'_>>()
}
pub(super) fn with_variant<T>(
    contract: &'static str,
    operation: &'static str,
    keys: &[&'static str],
    visibility: NativeBuiltinVariantVisibilityV1,
    consume: impl FnOnce(DiscoveryVariantInputV1<'_>) -> T,
) -> T {
    let contract_schema = [("const", D::Text(contract))];
    let operation_schema = [("const", D::Text(operation))];
    let mut properties = [("", D::Null); 7];
    let mut required = [D::Null; 7];
    let mut roles = [("", D::Null); 7];
    for (i, key) in keys.iter().enumerate() {
        properties[i] = (
            key,
            match *key {
                "contract" => D::Object(&contract_schema),
                "operation" => D::Object(&operation_schema),
                _ => D::Object(&[]),
            },
        );
        required[i] = D::Text(key);
        roles[i] = (
            key,
            D::Text(match *key {
                "snapshot" => "OriginalOpaqueSourceOrCombinedOrVersionSetMember",
                "document" => "OriginalDocumentIdentity",
                "context" => "OriginalOpaqueReadContext",
                "annotation" => "OriginalOpaqueAnnotationRecord",
                "continuation" => "NullOrOriginalOpaqueContinuation",
                "limit" => "PositiveSafeInteger",
                "selection" => "ClosedCurrentOrOriginalRetainedReference",
                "participants" => "OriginalBoundedParticipantRoles",
                "data" => "ClosedProjectionClasses",
                "lo" | "hi" | "line" => "NonnegativeSafeInteger",
                _ => "OwnedOrdinaryMeaning",
            }),
        );
    }
    let schema = [
        (
            "$schema",
            D::Text("https://json-schema.org/draft/2020-12/schema"),
        ),
        ("type", D::Text("object")),
        ("properties", D::Object(&properties[..keys.len()])),
        ("required", D::Array(&required[..keys.len()])),
        ("additionalProperties", D::Boolean(false)),
    ];
    let input = [
        ("contract", D::Text(contract)),
        ("variant", D::Text(operation)),
        ("fields", D::Object(&roles[..keys.len()])),
    ];
    let requirements = [
        ("schema", D::Object(&schema)),
        ("coverage", D::Text("outer-structure-only")),
        (
            "runtime",
            D::Text(
                "Closed borrowed typed/dynamic grammar; original opaque owner roles; exact actor/scope/profile and current native release remain mandatory",
            ),
        ),
    ];
    consume(DiscoveryVariantInputV1 {
        id: operation,
        input: D::Object(&input),
        requirements: D::Object(&requirements),
        outcomes: D::Text(
            "Original-bound guarded native read product or actual current/retained snapshot acquisition; explicit refusal/limit/fault",
        ),
        limits: D::Text(
            "Existing funded read facade capture/native/result workspace, profile-specific request ceilings and original owner limits",
        ),
        cancellation: D::Text(
            "Immediate query/acquisition; no new task; existing context stop and protected retirement remain native",
        ),
        order: visibility.order,
        enabled: visibility.enabled,
    })
}

fn with_input<T>(
    authority: &str,
    commands: &[String; 3],
    visibility: &[NativeBuiltinEntryVisibilityV1<2>; 1],
    acquisitions: &[NativeBuiltinEntryVisibilityV1<1>; 2],
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    let source = visibility[0];
    with_variant(
        "source-query/1",
        "bounded-range",
        &[
            "contract",
            "operation",
            "snapshot",
            "document",
            "profile",
            "lo",
            "hi",
        ],
        source.variants[0],
        |range| {
            with_variant(
                "source-query/1",
                "line",
                &[
                    "contract",
                    "operation",
                    "snapshot",
                    "document",
                    "profile",
                    "line",
                ],
                source.variants[1],
                |line| {
                    with_variant(
                        "read-context/1",
                        "snapshot-handoff",
                        &[
                            "contract",
                            "operation",
                            "document",
                            "selection",
                            "data",
                            "profile",
                        ],
                        acquisitions[0].variants[0],
                        |single| {
                            with_variant(
                                "read-version-set/1",
                                "snapshot-handoff",
                                &[
                                    "contract",
                                    "operation",
                                    "authority",
                                    "participants",
                                    "selection",
                                    "profile",
                                ],
                                acquisitions[1].variants[0],
                                |coherent| {
                                    let source_variants = [range, line];
                                    let single_variants = [single];
                                    let coherent_variants = [coherent];
                                    let variants = [
                                        &source_variants[..],
                                        &single_variants[..],
                                        &coherent_variants[..],
                                    ];
                                    let entries = std::array::from_fn::<_, 3, _>(|i| {
                                        let (order, schema, presentation, advice) = if i == 0 {
                                            (
                                                source.order,
                                                source.schema,
                                                source.presentation,
                                                source.advice,
                                            )
                                        } else {
                                            let v = acquisitions[i - 1];
                                            (v.order, v.schema, v.presentation, v.advice)
                                        };
                                        DiscoveryEntryInputV1 {
                                            command: DiscoveryCommandV1 {
                                                authority,
                                                id: &commands[i],
                                                incarnation: 1,
                                            },
                                            contract: [
                                                "source-query/1",
                                                "read-context/1",
                                                "read-version-set/1",
                                            ][i],
                                            version: 1,
                                            kind: DiscoveryExecutionKindV1::Query,
                                            variants: variants[i],
                                            presentation: D::Object(&[]),
                                            order,
                                            schema_visible: schema,
                                            presentation_visible: presentation,
                                            advice_visible: advice,
                                        }
                                    });
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
        },
    )
}

impl SourceCoordinator {
    pub fn prepare_native_source_read_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        source: &NativeBuiltinEntryVisibilityV1<2>,
        acquisitions: &[NativeBuiltinEntryVisibilityV1<1>; 2],
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
            if store.limits.max_entries < 3 || store.limits.max_variants < 2 {
                return Err(Error::Limit);
            }
            let authority = text(&store.authority)?;
            let commands = [
                command(
                    &store.engine,
                    "F02-source-read-v1",
                    store.limits.max_context_bytes,
                )?,
                command(
                    &store.engine,
                    "F02-single-acquisition-v1",
                    store.limits.max_context_bytes,
                )?,
                command(
                    &store.engine,
                    "F02-coherent-acquisition-v1",
                    store.limits.max_context_bytes,
                )?,
            ];
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            with_input(&authority, &commands, &[*source], acquisitions, |input| {
                self.prepare_discovery_cut_from_source(
                    scope,
                    input,
                    &facts,
                    Origin::NativeSourceReadV1,
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

    fn authorize_discovered_source_read(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        contract: &str,
        operation: &str,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let recognized = self.discovery_context(route.context).and_then(|c| {
            queries::source_read_selector_matches(
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

    pub fn query_discovered_source_read_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        request: DecodedSourceFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Result<SourceFacadeReplyV1, DiscoveredSourceReadErrorV1> {
        type E = DiscoveredSourceReadErrorV1;
        let timing = r.begin_service_timing();
        let result = (|| {
            let operation = arguments::installed_selector(request.envelope).map_err(E::Native)?;
            self.authorize_discovered_source_read(route, "source-query/1", operation, r)
                .map_err(E::Discovery)?;
            self.try_query_decoded_source_facade_v1(request, r)
                .map_err(E::Native)
        })();
        record_dispatch(
            r,
            Boundary::DiscoverySourceReadDispatch,
            &result,
            timing.finish(),
        );
        result
    }

    pub fn acquire_discovered_read_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        request: DecodedReadAcquisitionFacadeRequestV1<'_, '_>,
        r: &mut Recorder,
    ) -> Result<ReadAcquisitionFacadeReplyV1, DiscoveredSourceReadErrorV1> {
        type E = DiscoveredSourceReadErrorV1;
        let timing = r.begin_service_timing();
        let result = (|| {
            let contract =
                acquisition::arguments::installed_selector(request.envelope).map_err(E::Native)?;
            self.authorize_discovered_source_read(route, contract, "snapshot-handoff", r)
                .map_err(E::Discovery)?;
            self.try_acquire_decoded_read_facade_v1(request, r)
                .map_err(E::Native)
        })();
        record_dispatch(
            r,
            Boundary::DiscoveryReadAcquisitionDispatch,
            &result,
            timing.finish(),
        );
        result
    }
}

pub(super) fn record_dispatch<T>(
    r: &mut Recorder,
    boundary: Boundary,
    result: &Result<T, DiscoveredSourceReadErrorV1>,
    service_time: Option<std::time::Duration>,
) {
    r.record(Observation {
        boundary,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(DiscoveredSourceReadErrorV1::Discovery(Error::Faulted))
            | Err(DiscoveredSourceReadErrorV1::Native(SourceReadContextErrorV1::Faulted)) => {
                Disposition::Unknown
            }
            _ => Disposition::Refused,
        },
        service_time,
    });
}
