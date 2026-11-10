//! Sealed F12 native selection dispatch; host authorization remains independent.
use super::*;
#[cfg(test)]
mod tests;
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(3, maximum)?,
        size_of::<[String; 3]>(),
        4 * (size_of::<[[(&str, D<'_>); 13]; 2]>()
            + size_of::<[D<'_>; 13]>()
            + size_of::<[(&str, D<'_>); 5]>()
            + size_of::<[[(&str, D<'_>); 3]; 2]>()
            + size_of::<[[(&str, D<'_>); 1]; 2]>()
            + size_of::<DiscoveryVariantInputV1<'_>>()),
        size_of::<[DiscoveryEntryInputV1<'_>; 2]>(),
        size_of::<[DiscoveryVariantInputV1<'_>; 4]>(),
        size_of::<[&NativeArgumentDeclarationV1; 4]>(),
        size_of::<DiscoveryCutInputV1<'_>>(),
        size_of::<TrustedDiscoveryFactsV1>(),
    ]
    .into_iter()
    .try_fold(0, add)
}
pub(super) fn with_variant<T>(
    declaration: &'static NativeArgumentDeclarationV1,
    visibility: NativeBuiltinVariantVisibilityV1,
    consume: impl FnOnce(DiscoveryVariantInputV1<'_>) -> T,
) -> T {
    let contract = [("const", D::Text(declaration.contract))];
    let operation = [("const", D::Text(declaration.variant.operation))];
    let mut properties = [("", D::Null); 13];
    let mut required = [D::Null; 13];
    let mut roles = [("", D::Null); 13];
    for (i, (field, role)) in declaration.variant.fields.iter().enumerate() {
        properties[i] = (
            field,
            match *field {
                "contract" => D::Object(&contract),
                "operation" => D::Object(&operation),
                _ => D::Object(&[]),
            },
        );
        required[i] = D::Text(field);
        roles[i] = (field, D::Text(role));
    }
    let n = declaration.variant.fields.len();
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
        ("contract", D::Text(declaration.contract)),
        ("variant", D::Text(declaration.variant.operation)),
        ("fields", D::Object(&roles[..n])),
    ];
    let requirements = [
        ("schema", D::Object(&schema)),
        ("coverage", D::Text("outer-structure-only")),
        (
            "runtime",
            D::Text(
                "Existing owning normalizer validates closed nested native values, exact execution profile, original opaque role binding and per-form interpretation limits; metadata is not recipient authorization",
            ),
        ),
    ];
    consume(DiscoveryVariantInputV1 {
        id: declaration.variant.operation,
        input: D::Object(&input),
        requirements: D::Object(&requirements),
        outcomes: D::Text(
            "Existing one-shot selection admission, native execution and original retained outcome; no authored text or source history from selection-only work",
        ),
        limits: D::Text(
            "Explicit input-selection/owned-byte/endpoint interpretation limits; existing invocation, view, mapping and boundary owner bounds",
        ),
        cancellation: D::Text(
            "Existing invocation cancellation and protected retirement; matching retry does not reacquire current selections or execute again",
        ),
        order: visibility.order,
        enabled: visibility.enabled,
    })
}
fn with_input<T>(
    authority: &str,
    commands: &[String; 2],
    selection: &NativeBuiltinEntryVisibilityV1<3>,
    boundary: &NativeBuiltinEntryVisibilityV1<1>,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    let d = native_selection_argument_declarations_v1();
    with_variant(d[0], selection.variants[0], |set| {
        with_variant(d[1], selection.variants[1], |movement| {
            with_variant(d[2], selection.variants[2], |occurrence| {
                with_variant(d[3], boundary.variants[0], |unicode| {
                    let selected = [set, movement, occurrence];
                    let bounded = [unicode];
                    let entries = [
                        DiscoveryEntryInputV1 {
                            command: DiscoveryCommandV1 {
                                authority,
                                id: &commands[0],
                                incarnation: 1,
                            },
                            contract: "selection-edit/1",
                            version: 1,
                            kind: DiscoveryExecutionKindV1::LocalCommand,
                            variants: &selected,
                            presentation: D::Object(&[]),
                            order: selection.order,
                            schema_visible: selection.schema,
                            presentation_visible: selection.presentation,
                            advice_visible: selection.advice,
                        },
                        DiscoveryEntryInputV1 {
                            command: DiscoveryCommandV1 {
                                authority,
                                id: &commands[1],
                                incarnation: 1,
                            },
                            contract: "boundary-command/1",
                            version: 1,
                            kind: DiscoveryExecutionKindV1::LocalCommand,
                            variants: &bounded,
                            presentation: D::Object(&[]),
                            order: boundary.order,
                            schema_visible: boundary.schema,
                            presentation_visible: boundary.presentation,
                            advice_visible: boundary.advice,
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
    pub fn prepare_native_selection_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        selection: &NativeBuiltinEntryVisibilityV1<3>,
        boundary: &NativeBuiltinEntryVisibilityV1<1>,
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
            if store.limits.max_entries < 2 || store.limits.max_variants < 3 {
                return Err(Error::Limit);
            }
            let authority = text(&store.authority)?;
            let commands = [
                command(
                    &store.engine,
                    "F12-selection-v1",
                    store.limits.max_context_bytes,
                )?,
                command(
                    &store.engine,
                    "F12-boundary-movement-v1",
                    store.limits.max_context_bytes,
                )?,
            ];
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            with_input(&authority, &commands, selection, boundary, |input| {
                self.prepare_discovery_cut_from_source(
                    scope,
                    input,
                    &facts,
                    Origin::NativeSelectionV1,
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
    /// Trusted native embedding only. The host independently authorizes mutation,
    /// as for submit_selection_arguments_v1. Discovery is not that authority.
    pub fn submit_discovered_native_selection_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        q: &NormalizedSelectionArgumentsV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        type E = DiscoveredNativeCommandErrorV1;
        let timing = r.begin_service_timing();
        let result = (|| {
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::selection_selector_matches(
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
            self.submit_selection_arguments_v1(q, r)
                .map_err(E::Invocation)
        })();
        r.record(Observation {
            boundary: Boundary::DiscoverySelectionDispatch,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(E::Discovery(Error::Faulted))
                | Err(E::Invocation(InvocationError::Faulted)) => Disposition::Unknown,
                _ => Disposition::Refused,
            },
            service_time: timing.finish(),
        });
        result
    }
}
