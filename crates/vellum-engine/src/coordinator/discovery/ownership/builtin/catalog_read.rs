//! Sealed installed F07 resource-catalog query; not configuration capabilities.
use super::*;
use crate::resource::ResourceError;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredCatalogReadErrorV1 {
    Discovery(DiscoveryErrorV1),
    Native(ResourceError),
}
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<[[(&str, D<'_>); 5]; 5]>(),
        size_of::<[[(&str, D<'_>); 3]; 3]>(),
        size_of::<[D<'_>; 5]>(),
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
    let keys = ["contract", "operation", "context", "profile", "request"];
    let contract = [("const", D::Text("resource-catalog-query/1"))];
    let operation = [("const", D::Text("capabilities"))];
    let profile = [("const", D::Text(RESOURCE_CATALOG_READ_PROFILE_V1))];
    let opaque = [("type", D::Text("object"))];
    let properties = [
        ("contract", D::Object(&contract)),
        ("operation", D::Object(&operation)),
        ("context", D::Object(&opaque)),
        ("profile", D::Object(&profile)),
        ("request", D::Object(&opaque)),
    ];
    let required = keys.map(D::Text);
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
    let roles = [
        ("contract", D::Text("resource-catalog-query/1")),
        ("operation", D::Text("capabilities")),
        (
            "context",
            D::Text("OriginalOpaqueResourceCatalogReadContextV1"),
        ),
        ("profile", D::Text(RESOURCE_CATALOG_READ_PROFILE_V1)),
        (
            "request",
            D::Text(
                "ClosedList(operation,positive-safe-limit,null-or-original-continuation)OrInspect(operation,original-entry)",
            ),
        ),
    ];
    let input = [
        ("contract", D::Text("resource-catalog-query/1")),
        ("variant", D::Text("capabilities")),
        ("fields", D::Object(&roles)),
    ];
    let requirements = [
        ("schema", D::Object(&schema)),
        ("coverage", D::Text("outer-structure-only")),
        (
            "runtime",
            D::Text(
                "Closed nested list/inspect grammar; original context and distinct entry/continuation roles; captured/current scope/visibility meet and original page limit; metadata grants no provider, mutation or content authority",
            ),
        ),
    ];
    let variants = [DiscoveryVariantInputV1 {
        id: "capabilities",
        input: D::Object(&input),
        requirements: D::Object(&requirements),
        outcomes: D::Text(
            "Original-context captured-permitted page or inspected registration; original entry/continuation bindings; whitelisted owned descriptors; refusal/limit/fault explicit",
        ),
        limits: D::Text(
            "Actual catalog reader/selector/reply profile; independent native capture bytes/nodes/depth; actual parent intent ceiling and classification plus seven bounded request frames",
        ),
        cancellation: D::Text(
            "Immediate metadata only; no provider task or mutation; protected stop, prepared visibility and retained lifetime discharge remain at their native owner",
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
        contract: "resource-catalog-query/1",
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
    pub fn prepare_native_catalog_read_discovery_cut_v1(
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
            let s = self.discovery_store()?;
            if s.limits.max_entries == 0 || s.limits.max_variants == 0 {
                return Err(Error::Limit);
            }
            let authority = text(&s.authority)?;
            let command = command(
                &s.engine,
                "F07-resource-catalog-read-v1",
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
                    Origin::NativeCatalogReadV1,
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
    #[allow(clippy::too_many_arguments)]
    pub fn query_discovered_resource_catalog_read_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        envelope: &ResourceCatalogReadEnvelopeValueV1<'_, '_>,
        limits: crate::capture::CaptureLimitsV1,
        scope: &ResourceCatalogReadScopeV1,
        cut: &ResourceCatalogVisibilityIdV1,
        facts: &TrustedResourceCatalogReadFactsV1,
        r: &mut Recorder,
    ) -> Result<ResourceCatalogReadReplyV1, DiscoveredCatalogReadErrorV1> {
        type E = DiscoveredCatalogReadErrorV1;
        let timing = r.begin_service_timing();
        let result = (|| {
            self.recognize_resource_catalog_read_request_v1(envelope)
                .map_err(E::Native)?;
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::catalog_read_selector_matches(c, route.entry, route.variant)
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
            self.query_decoded_resource_catalog_read_v1(envelope, limits, scope, cut, facts, r)
                .map_err(E::Native)
        })();
        r.record(Observation {
            boundary: Boundary::DiscoveryCatalogReadDispatch,
            disposition: match &result {
                Ok(_) => Disposition::Completed,
                Err(E::Discovery(Error::Faulted)) | Err(E::Native(ResourceError::Faulted)) => {
                    Disposition::Unknown
                }
                _ => Disposition::Refused,
            },
            service_time: timing.finish(),
        });
        result
    }
}
