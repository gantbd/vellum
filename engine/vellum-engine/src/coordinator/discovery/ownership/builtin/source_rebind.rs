//! Fixed native F17 rebind source and existing-owner dispatch, not a new writer.
use super::*;
#[cfg(test)]
mod tests;
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<NativeArgumentDeclarationV1>(),
        size_of::<[(&str, D<'_>); 3]>() * 3,
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
    let declaration = SourceRebindRequestV1::argument_declaration_v1();
    let fields: [(&str, D<'_>); 3] = std::array::from_fn(|i| {
        let (key, value) = declaration.variant.fields[i];
        (key, D::Text(value))
    });
    let roles = [
        ("invocation", D::Text("OriginalOpaqueInvocation")),
        ("request", D::Object(&fields)),
        ("admission", D::Text("IndependentSourceRebindAdmissionV1")),
    ];
    let input = [
        ("contract", D::Text(declaration.contract)),
        ("variant", D::Text("rebind")),
        ("fields", D::Object(&roles)),
    ];
    let variants = [DiscoveryVariantInputV1 {
        id: "rebind",
        input: D::Object(&input),
        outcomes: D::Text(
            "Original native invocation and independent authorized F06 V7 historical rebind result",
        ),
        requirements: D::Text(
            "Original native admission and fresh source-rebind service authority; metadata grants nothing",
        ),
        limits: D::Text("Explicit bounded native roles, parent-funded capture and result ceiling"),
        cancellation: D::Text(
            "Original cooperative invocation stop; no filesystem rename, write or effect cancellation",
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
    /// Separate installed source on the existing scoped /1 owner. No widening
    /// of the earlier fixed inventories, or new registry/profile/permission.
    pub fn prepare_native_rebind_discovery_cut_v1(
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
                "F17-source-rebind",
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
                    Origin::NativeRebindV1,
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
    /// Original selector recognition precedes observed loss. Discovery release
    /// is separate from the owning native admission and fresh service authority.
    pub fn submit_discovered_source_rebind_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        invocation: &InvocationId,
        request: &SourceRebindRequestV1,
        admission: &SourceRebindAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let recognized = self
            .discovery_context(route.context)
            .and_then(|c| queries::rebind_selector_matches(c, route.entry, route.variant));
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
        self.submit_source_rebind_v1(invocation, request, admission, r)
            .map_err(DiscoveredNativeCommandErrorV1::Invocation)
    }
}
