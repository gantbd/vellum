//! Installed native Save As activation, not write admission or dispatch.
use super::*;
#[cfg(test)]
mod tests;
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<NativeArgumentDeclarationV1>(),
        size_of::<[(&str, D<'_>); 4]>(),
        size_of::<[(&str, D<'_>); 3]>() * 2,
        size_of::<[DiscoveryVariantInputV1<'_>; 1]>(),
        size_of::<[DiscoveryEntryInputV1<'_>; 1]>(),
        size_of::<TrustedDiscoveryFactsV1>(),
        size_of::<DiscoveryCutInputV1<'_>>(),
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
    let declaration = SourceSaveAsRequestV1::argument_declaration_v1();
    let fields: [(&str, D<'_>); 3] = std::array::from_fn(|i| {
        let (key, value) = declaration.variant.fields[i];
        (key, D::Text(value))
    });
    let roles = [
        ("invocation", D::Text("OriginalOpaqueInvocation")),
        ("authority", D::Text("OriginalSourceSaveAsAuthorityV1")),
        ("request", D::Object(&fields)),
        ("admission", D::Text("IndependentSourceSaveAsAdmissionV1")),
    ];
    let input = [
        ("contract", D::Text(declaration.contract)),
        ("variant", D::Text("activate")),
        ("fields", D::Object(&roles)),
    ];
    let variants = [DiscoveryVariantInputV1 {
        id: "activate",
        input: D::Object(&input),
        outcomes: D::Text(
            "Original invocation and independently released F06 V10 historical activation result",
        ),
        requirements: D::Text(
            "Original task authority and native admission; fresh service facts; metadata grants nothing",
        ),
        limits: D::Text("Explicit native capture, history and result bounds"),
        cancellation: D::Text(
            "Cooperative invocation stop is not producer cancellation or rollback",
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
        contract: declaration.contract,
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
    pub fn prepare_native_save_as_discovery_cut_v1(
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
            let store = self.discovery_store()?;
            if store.limits.max_entries < 1 || store.limits.max_variants < 1 {
                return Err(Error::Limit);
            }
            let authority = text(&store.authority)?;
            let command = command(
                &store.engine,
                "F17-source-saveas",
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
                    Origin::NativeSaveAsV1,
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
    pub fn submit_discovered_source_save_as_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        invocation: &InvocationId,
        authority: &SourceSaveAsAuthorityV1,
        request: &SourceSaveAsRequestV1,
        admission: &SourceSaveAsAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let recognized = self
            .discovery_context(route.context)
            .and_then(|c| queries::source_save_as_selector_matches(c, route.entry, route.variant));
        observe(r, Boundary::DiscoveryRequestRecognition, &recognized);
        recognized.map_err(DiscoveredNativeCommandErrorV1::Discovery)?;
        self.query_discovery_v1(
            route.context,
            route.scope,
            route.cut,
            DiscoveryQueryV1::Inspect { entry: route.entry },
            route.facts,
            r,
        )
        .map_err(DiscoveredNativeCommandErrorV1::Discovery)?;
        self.submit_source_save_as_v1(invocation, authority, request, admission, r)
            .map_err(DiscoveredNativeCommandErrorV1::Invocation)
    }
}
