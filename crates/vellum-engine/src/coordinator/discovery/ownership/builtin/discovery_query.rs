//! F01 self-description and sealed dispatch over the existing native decoder.
use super::*;
#[cfg(test)]
mod tests;

pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        // Three synchronous nested fixed variant-construction frames. No
        // external callbacks, schema traversal or recursive self-expansion.
        mul(
            3,
            size_of::<[[(&str, D<'_>); 7]; 4]>()
                + size_of::<[D<'_>; 7]>()
                + size_of::<[[D<'_>; 5]; 2]>()
                + size_of::<[[(&str, D<'_>); 3]; 3]>()
                + size_of::<[(&str, D<'_>); 5]>()
                + size_of::<[[(&str, D<'_>); 1]; 2]>()
                + size_of::<NativeDiscoveryInputFormV1>()
                + size_of::<NativeDiscoveryResultFormV1>()
                + size_of::<DiscoveryVariantInputV1<'_>>(),
        )?,
        size_of::<[DiscoveryVariantInputV1<'_>; 3]>(),
        size_of::<[DiscoveryEntryInputV1<'_>; 1]>(),
        size_of::<DiscoveryCutInputV1<'_>>(),
        size_of::<TrustedDiscoveryFactsV1>(),
    ]
    .into_iter()
    .try_fold(0, add)
}
fn with_variant<T>(
    operation: DiscoveryOperationV1,
    visibility: NativeBuiltinVariantVisibilityV1,
    consume: impl FnOnce(DiscoveryVariantInputV1<'_>) -> T,
) -> T {
    let d = operation.declaration();
    let result = operation.result_declaration();
    let contract = [("const", D::Text(d.contract))];
    let variant = [("const", D::Text(operation.name()))];
    let mut properties = [("", D::Null); 7];
    let mut roles = [("", D::Null); 7];
    let mut required = [D::Null; 7];
    for (i, key) in d.fields.iter().enumerate() {
        properties[i] = (
            key,
            match *key {
                "contract" => D::Object(&contract),
                "operation" => D::Object(&variant),
                _ => D::Object(&[]),
            },
        );
        roles[i] = (
            key,
            D::Text(if d.opaque_roles.contains(key) {
                match *key {
                    "context" => "OriginalDiscoveryContext",
                    "entry" => "OriginalEntryOfContext",
                    "variant" => "OriginalVariantOfEntry",
                    "target" => "OriginalCapturedTarget",
                    "continuation" => "NullOrOriginalCursorBoundToPageLimit",
                    _ => unreachable!("closed owning opaque role"),
                }
            } else {
                match *key {
                    "contract" => d.contract,
                    "operation" => operation.name(),
                    "profile" => "ExactConfiguredDiscoveryProfile",
                    "limit" => "PositiveSafeIntegerWithinConfiguredPageLimit",
                    _ => unreachable!("closed owning discovery declaration"),
                }
            }),
        );
        required[i] = D::Text(key);
    }
    let n = d.fields.len();
    let schema = [
        (
            "$schema",
            D::Text("https://json-schema.org/draft/2020-12/schema"),
        ),
        ("type", D::Text("object")),
        ("properties", D::Object(&properties[..n])),
        ("required", D::Array(&required[..n])),
        ("additionalProperties", D::Boolean(false)),
    ];
    let input = [
        ("contract", D::Text(d.contract)),
        ("variant", D::Text(operation.name())),
        ("fields", D::Object(&roles[..n])),
    ];
    let requirements = [
        ("schema", D::Object(&schema)),
        ("coverage", D::Text("outer-structure-only")),
        (
            "runtime",
            D::Text(
                "Native decoder validates closed typed/dynamic grammar, original context/entry/variant/target/cursor, exact profile and page limit; current recipient authority and captured disclosure ceiling remain independent",
            ),
        ),
    ];
    let mut result_fields = [("", D::Null); 7];
    for (i, (key, role)) in result.fields.iter().enumerate() {
        result_fields[i] = (key, D::Text(role));
    }
    let schema_facets: [_; 5] = std::array::from_fn(|i| D::Text(arguments::SCHEMA_FACETS[i]));
    let presentation_fields: [_; 5] =
        std::array::from_fn(|i| D::Text(arguments::PRESENTATION_FIELDS[i]));
    let outcomes = [
        ("nativeForm", D::Text(result.name)),
        ("fields", D::Object(&result_fields[..result.fields.len()])),
        ("schemaFacets", D::Array(&schema_facets)),
        ("presentationFields", D::Array(&presentation_fields)),
        (
            "release",
            D::Text(
                "Native Result<DiscoveryReplyV1,DiscoveryErrorV1>; errors release no product. Native structure/original-binding guard, not an opaque-handle wire schema; no recursive contained-schema expansion",
            ),
        ),
    ];
    consume(DiscoveryVariantInputV1 {
        id: operation.name(),
        input: D::Object(&input),
        requirements: D::Object(&requirements),
        outcomes: D::Object(&outcomes),
        limits: D::Text(
            "Independent configured entry, variant, selector, issuance, page, capture UTF-8 bytes/nodes/depth, row, packet, reply and parent-retention bounds; no capacity growth on refusal",
        ),
        cancellation: D::Text(
            "Immediate query, not an invocation or task; protected context/cut retirement. Advice never invokes its described operation; retirement does not refund lifetime issuance",
        ),
        order: visibility.order,
        enabled: visibility.enabled,
    })
}
pub(super) fn with_input<T>(
    authority: &str,
    command: &str,
    visibility: &NativeBuiltinEntryVisibilityV1<3>,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    with_variant(DiscoveryOperationV1::List, visibility.variants[0], |list| {
        with_variant(
            DiscoveryOperationV1::Inspect,
            visibility.variants[1],
            |inspect| {
                with_variant(
                    DiscoveryOperationV1::AdvisoryEnablement,
                    visibility.variants[2],
                    |advice| {
                        let variants = [list, inspect, advice];
                        let entries = [DiscoveryEntryInputV1 {
                            command: DiscoveryCommandV1 {
                                authority,
                                id: command,
                                incarnation: 1,
                            },
                            contract: DiscoveryOperationV1::List.declaration().contract,
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
    })
}
impl SourceCoordinator {
    /// Explicit native self-description source. Does not acquire a context,
    /// enable a decoder, invoke queries, grow capacities or replace older catalogs.
    pub fn prepare_native_discovery_query_cut_v1(
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
            let s = self.discovery_store()?;
            if s.limits.max_entries == 0 || s.limits.max_variants < 3 {
                return Err(Error::Limit);
            }
            let authority = text(&s.authority)?;
            let command = command(
                &s.engine,
                "F01-discovery-query-v1",
                s.limits.max_context_bytes,
            )?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            with_input(&authority, &command, visibility, |input| {
                self.prepare_discovery_cut_from_source(
                    scope,
                    input,
                    &facts,
                    Origin::NativeDiscoveryQueryV1,
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
    /// Installed query over independently authorized native context/cut. The
    /// description's authority cannot grant target-query access or enable decoding.
    #[allow(clippy::too_many_arguments)]
    pub fn query_discovered_discovery_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        context: &DiscoveryContextIdV1,
        scope: &DiscoveryScopeV1,
        cut: &DiscoveryCutIdV1,
        envelope: &DiscoveryEnvelopeValueV1<'_, '_>,
        facts: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryReplyV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            let operation = arguments::recognize_operation(envelope)?;
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::discovery_query_selector_matches(c, route.entry, route.variant, operation)
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
            self.query_decoded_discovery_v1(context, scope, cut, envelope, facts, r)
        })();
        self.discovery_finish_timed(r, Boundary::DiscoveryQueryDispatch, &result, timer.finish());
        result
    }
}
