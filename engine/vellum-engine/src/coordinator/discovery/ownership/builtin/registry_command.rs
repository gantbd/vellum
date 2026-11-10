//! Installed F19 registry commands; metadata never grants native admission/control.
use super::*;
use crate::resource::ResourceError;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredRegistryCommandErrorV1 {
    Discovery(DiscoveryErrorV1),
    Native(ResourceError),
}
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<[[(&str, D<'_>); 5]; 12]>(),
        size_of::<[[(&str, D<'_>); 4]; 4]>(),
        size_of::<[[D<'_>; 5]; 2]>(),
        size_of::<[DiscoveryVariantInputV1<'_>; 2]>(),
        size_of::<[DiscoveryEntryInputV1<'_>; 1]>(),
        size_of::<DiscoveryCutInputV1<'_>>(),
        size_of::<TrustedDiscoveryFactsV1>(),
    ]
    .into_iter()
    .try_fold(0, add)
}
fn with_input<T>(
    authority: &str,
    command: &str,
    visibility: &NativeBuiltinEntryVisibilityV1<2>,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    let names = ["establish-bounded-stream", "authorized-resynchronization"];
    let contract = [("const", D::Text(RESOURCE_REGISTRY_COMMAND_PROFILE_V1))];
    let operations = names.map(|s| [("const", D::Text(s))]);
    let opaque = [("type", D::Text("object"))];
    let null = [("type", D::Text("null"))];
    let alternatives = [D::Object(&opaque), D::Object(&null)];
    let preparation = [("oneOf", D::Array(&alternatives))];
    let properties: [[(&str, D<'_>); 5]; 2] = std::array::from_fn(|i| {
        [
            ("contract", D::Object(&contract)),
            ("operation", D::Object(&operations[i])),
            ("command", D::Object(&opaque)),
            (
                "stream",
                if i == 0 {
                    D::Object(&null)
                } else {
                    D::Object(&opaque)
                },
            ),
            ("preparation", D::Object(&preparation)),
        ]
    });
    let required = ["contract", "operation", "command", "stream", "preparation"].map(D::Text);
    let schemas: [[(&str, D<'_>); 5]; 2] = std::array::from_fn(|i| {
        [
            (
                "$schema",
                D::Text("https://json-schema.org/draft/2020-12/schema"),
            ),
            ("type", D::Text("object")),
            ("properties", D::Object(&properties[i])),
            ("required", D::Array(&required)),
            ("additionalProperties", D::Boolean(false)),
        ]
    });
    let roles: [[(&str, D<'_>); 5]; 2] = std::array::from_fn(|i| {
        [
            ("contract", D::Text(RESOURCE_REGISTRY_COMMAND_PROFILE_V1)),
            ("operation", D::Text(names[i])),
            ("command", D::Text("OriginalOpaqueRegistryCommandV1")),
            (
                "stream",
                D::Text(if i == 0 {
                    "Null"
                } else {
                    "OriginalOpaqueRegistryStreamV1"
                }),
            ),
            (
                "preparation",
                D::Text("OriginalOpaqueRegistryPreparationV1OrNull"),
            ),
        ]
    });
    let inputs: [[(&str, D<'_>); 3]; 2] = std::array::from_fn(|i| {
        [
            ("contract", D::Text(RESOURCE_REGISTRY_COMMAND_PROFILE_V1)),
            ("variant", D::Text(names[i])),
            ("fields", D::Object(&roles[i])),
        ]
    });
    let requirements: [[(&str, D<'_>); 3]; 2] = std::array::from_fn(|i| {
        [
            ("schema", D::Object(&schemas[i])),
            ("coverage", D::Text("outer-structure-only")),
            (
                "runtime",
                D::Text(
                    "Actual original command/operation/stream/scope/preparation and current independent admission/control facts; original root/generation on first execution; metadata grants nothing",
                ),
            ),
        ]
    });
    let variants: [_; 2] = std::array::from_fn(|i| DiscoveryVariantInputV1 {
        id: names[i],
        input: D::Object(&inputs[i]),
        outcomes: D::Text(
            "command-settled or observe-existing with original command identity, or unavailable; baseline payload requires separate F06 receipt authority",
        ),
        requirements: D::Object(&requirements[i]),
        limits: D::Text(
            "Fixed 2/4/5 passive input; existing registry command/preparation/reply/binding/lifetime and discovery-parent budgets",
        ),
        cancellation: D::Text(
            "Observation stop is not command cancellation or retirement; matching replay does not reset queues or retry native execution",
        ),
        order: visibility.variants[i].order,
        enabled: visibility.variants[i].enabled,
    });
    let entries = [DiscoveryEntryInputV1 {
        command: DiscoveryCommandV1 {
            authority,
            id: command,
            incarnation: 1,
        },
        contract: RESOURCE_REGISTRY_COMMAND_PROFILE_V1,
        version: 1,
        kind: DiscoveryExecutionKindV1::LocalCommand,
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
}
impl SourceCoordinator {
    pub fn prepare_native_registry_command_discovery_cut_v1(
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
            self.registry_observation_source_v1(&scope.context.authority, &scope.context.engine)
                .map_err(|_| Error::Unavailable)?;
            let s = self.discovery_store()?;
            if s.limits.max_entries < 1 || s.limits.max_variants < 2 {
                return Err(Error::Limit);
            }
            let authority = text(&s.authority)?;
            let command = command(
                &s.engine,
                "F19-registry-command",
                s.limits.max_context_bytes,
            )?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            with_input(&authority, &command, visibility, |i| {
                self.prepare_discovery_cut_from_source(
                    scope,
                    i,
                    &facts,
                    Origin::NativeRegistryCommandV1,
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
    /// Discovery release and native command authority are independent. Protected
    /// identity-only retry also remains directly available without discovery work.
    pub fn request_discovered_registry_command_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        envelope: &ResourceRegistryCommandEnvelopeValueV1<'_, '_>,
        port: &ResourceRegistryCommandPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryCommandReplyV1, DiscoveredRegistryCommandErrorV1> {
        let timer = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let n = normalize_resource_registry_command_envelope_v1(envelope, r)
                .map_err(DiscoveredRegistryCommandErrorV1::Native)?;
            if !n.matches(port) {
                return Err(DiscoveredRegistryCommandErrorV1::Native(
                    ResourceError::Unavailable,
                ));
            }
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::registry_command_selector_matches(
                    c,
                    route.entry,
                    route.variant,
                    n.operation(),
                )
            });
            observe(r, Boundary::DiscoveryRequestRecognition, &recognized);
            recognized.map_err(DiscoveredRegistryCommandErrorV1::Discovery)?;
            self.query_discovery_v1(
                route.context,
                route.scope,
                route.cut,
                DiscoveryQueryV1::Inspect { entry: route.entry },
                route.facts,
                r,
            )
            .map_err(DiscoveredRegistryCommandErrorV1::Discovery)?;
            self.request_decoded_resource_registry_command_v1(envelope, port, r)
                .map_err(DiscoveredRegistryCommandErrorV1::Native)
        }));
        match caught {
            Ok(result) => {
                r.record(Observation {
                    boundary: Boundary::DiscoveryRegistryCommandDispatch,
                    disposition: match &result {
                        Ok(_) => Disposition::Completed,
                        Err(
                            DiscoveredRegistryCommandErrorV1::Native(ResourceError::Faulted)
                            | DiscoveredRegistryCommandErrorV1::Discovery(Error::Faulted),
                        ) => Disposition::Unknown,
                        _ => Disposition::Refused,
                    },
                    service_time: timer.finish(),
                });
                result
            }
            Err(cause) => {
                r.record(Observation {
                    boundary: Boundary::DiscoveryRegistryCommandDispatch,
                    disposition: Disposition::Unknown,
                    service_time: timer.finish(),
                });
                std::panic::resume_unwind(cause)
            }
        }
    }
}
