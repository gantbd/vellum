//! Installed F06 historical query; discovery grants no native release rights.
use super::*;
use crate::resource::ResourceError;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredRegistryReceiptErrorV1 {
    Discovery(DiscoveryErrorV1),
    Native(ResourceError),
}
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<[[(&str, D<'_>); 6]; 5]>(),
        size_of::<[[(&str, D<'_>); 4]; 5]>(),
        size_of::<[[D<'_>; 6]; 2]>(),
        size_of::<[DiscoveryVariantInputV1<'_>; 1]>(),
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
    visibility: &NativeBuiltinEntryVisibilityV1<1>,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    let contract = [("const", D::Text(RESOURCE_REGISTRY_RECEIPT_PROFILE_V1))];
    let operation = [("const", D::Text("retained-outcome"))];
    let opaque = [("type", D::Text("object"))];
    let profile = [("const", D::Text(RESOURCE_REGISTRY_OBSERVATION_PROFILE_V1))];
    let kind = [("const", D::Text("registry-baseline"))];
    let facet_properties = [("kind", D::Object(&kind))];
    let facet_required = [D::Text("kind")];
    let facet = [
        ("type", D::Text("object")),
        ("properties", D::Object(&facet_properties)),
        ("required", D::Array(&facet_required)),
        ("additionalProperties", D::Boolean(false)),
    ];
    let properties = [
        ("contract", D::Object(&contract)),
        ("operation", D::Object(&operation)),
        ("command", D::Object(&opaque)),
        ("context", D::Object(&opaque)),
        ("profile", D::Object(&profile)),
        ("facet", D::Object(&facet)),
    ];
    let required = [
        "contract",
        "operation",
        "command",
        "context",
        "profile",
        "facet",
    ]
    .map(D::Text);
    let schema = [
        (
            "$schema",
            D::Text("https://json-schema.org/draft/2020-12/schema"),
        ),
        ("type", D::Text("object")),
        ("properties", D::Object(&properties)),
        ("required", D::Array(&required)),
        ("additionalProperties", D::Boolean(false)),
    ];
    // Existing /1 role declaration plus exact outer schema; opaque identity,
    // permissions and historical currentness are independently checked at runtime.
    let roles = [
        ("contract", D::Text(RESOURCE_REGISTRY_RECEIPT_PROFILE_V1)),
        ("operation", D::Text("retained-outcome")),
        ("command", D::Text("OriginalOpaqueRegistryCommandV1")),
        ("context", D::Text("OriginalOpaqueRegistryReceiptContextV1")),
        ("profile", D::Text(RESOURCE_REGISTRY_OBSERVATION_PROFILE_V1)),
        ("facet", D::Text("ExactObject: kind=registry-baseline")),
    ];
    let input = [
        ("contract", D::Text(RESOURCE_REGISTRY_RECEIPT_PROFILE_V1)),
        ("variant", D::Text("retained-outcome")),
        ("fields", D::Object(&roles)),
    ];
    let requirements = [
        ("schema", D::Object(&schema)),
        ("coverage", D::Text("outer-structure-only")),
        (
            "runtime",
            D::Text(
                "Actual original opaque roles, context/command/result retention, captured and current release authority, exact historical slots and current root certificate; no grants from metadata",
            ),
        ),
    ];
    let variants = [DiscoveryVariantInputV1 {
        id: "retained-outcome",
        input: D::Object(&input),
        outcomes: D::Text(
            "operation-receipt with complete original registry-baseline, or unavailable; no execution, refresh, filtering or stream consumption",
        ),
        requirements: D::Object(&requirements),
        limits: D::Text(
            "Fixed passive 2/4/6/1 input; original registry packet/result/binding/preparation/issuance budgets; existing discovery parent and selector bounds",
        ),
        cancellation: D::Text(
            "Receipt stop is not producer cancellation; command claim remains until protected receipt retirement",
        ),
        order: visibility.variants[0].order,
        enabled: visibility.variants[0].enabled,
    }];
    let entries = [DiscoveryEntryInputV1 {
        command: DiscoveryCommandV1 {
            authority,
            id: command,
            incarnation: 1,
        },
        contract: RESOURCE_REGISTRY_RECEIPT_PROFILE_V1,
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
}
impl SourceCoordinator {
    pub fn prepare_native_registry_receipt_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<1>,
        facts: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V1 {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, facts, r)?;
            self.registry_observation_source_v1(&scope.context.authority, &scope.context.engine)
                .map_err(|_| Error::Unavailable)?;
            let store = self.discovery_store()?;
            if store.limits.max_entries < 1 || store.limits.max_variants < 1 {
                return Err(Error::Limit);
            }
            let authority = text(&store.authority)?;
            let command = command(
                &store.engine,
                "F06-registry-receipt",
                store.limits.max_context_bytes,
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
                    Origin::NativeRegistryReceiptV1,
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
    /// Recognize original request and installed selectors before applying loss.
    /// Discovery release and actual historical receipt release remain independent.
    pub fn release_discovered_registry_receipt_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        envelope: &ResourceRegistryReceiptEnvelopeValueV1<'_, '_>,
        port: &ResourceRegistryReceiptPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<ResourceRegistryReceiptReplyV1, DiscoveredRegistryReceiptErrorV1> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let normalized = normalize_resource_registry_receipt_envelope_v1(envelope, r)
                .map_err(DiscoveredRegistryReceiptErrorV1::Native)?;
            if normalized.command() != port.command
                || normalized.context() != port.context
                || port.scope.context.profile != RESOURCE_REGISTRY_OBSERVATION_PROFILE_V1
            {
                return Err(DiscoveredRegistryReceiptErrorV1::Native(
                    ResourceError::Unavailable,
                ));
            }
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::registry_receipt_selector_matches(c, route.entry, route.variant)
            });
            observe(r, Boundary::DiscoveryRequestRecognition, &recognized);
            recognized.map_err(DiscoveredRegistryReceiptErrorV1::Discovery)?;
            self.query_discovery_v1(
                route.context,
                route.scope,
                route.cut,
                DiscoveryQueryV1::Inspect { entry: route.entry },
                route.facts,
                r,
            )
            .map_err(DiscoveredRegistryReceiptErrorV1::Discovery)?;
            self.release_decoded_resource_registry_receipt_v1(envelope, port, r)
                .map_err(DiscoveredRegistryReceiptErrorV1::Native)
        })();
        r.record(Observation {
            boundary: Boundary::DiscoveryRegistryReceiptDispatch,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(
                    DiscoveredRegistryReceiptErrorV1::Native(ResourceError::Faulted)
                    | DiscoveredRegistryReceiptErrorV1::Discovery(Error::Faulted),
                ) => Disposition::Unknown,
                Err(_) => Disposition::Refused,
            },
            service_time: timing.finish(),
        });
        result
    }
}
