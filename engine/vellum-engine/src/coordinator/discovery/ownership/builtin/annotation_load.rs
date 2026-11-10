//! Fixed native F17 annotation restoration/adoption. No decoder, I/O or new registry.
use super::*;
#[cfg(test)]
mod tests;
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<[NativeArgumentDeclarationV1; 2]>(),
        size_of::<[(&str, D<'_>); 2]>(),
        size_of::<[(&str, D<'_>); 3]>() * 3,
        size_of::<[(&str, D<'_>); 4]>() * 2,
        size_of::<[DiscoveryVariantInputV1<'_>; 2]>(),
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
    visibility: &NativeBuiltinEntryVisibilityV1<2>,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    let decl = native_annotation_load_argument_declarations_v1();
    let restore: [(&str, D<'_>); 2] = std::array::from_fn(|i| {
        let (k, v) = decl[0].variant.fields[i];
        (k, D::Text(v))
    });
    let baseline: [(&str, D<'_>); 3] = std::array::from_fn(|i| {
        let (k, v) = decl[1].variant.fields[i];
        (k, D::Text(v))
    });
    let requests = [D::Object(&restore), D::Object(&baseline)];
    let roles: [[(&str, D<'_>); 4]; 2] = std::array::from_fn(|i| {
        [
            ("invocation", D::Text("OriginalOpaqueInvocation")),
            (
                "authority",
                D::Text("OriginalOpaqueAnnotationLoadAuthorityV1"),
            ),
            ("request", requests[i]),
            ("admission", D::Text("IndependentAnnotationLoadAdmissionV1")),
        ]
    });
    let inputs: [[(&str, D<'_>); 3]; 2] = std::array::from_fn(|i| {
        [
            ("contract", D::Text(decl[i].contract)),
            ("variant", D::Text(decl[i].variant.operation)),
            ("fields", D::Object(&roles[i])),
        ]
    });
    let variants = std::array::from_fn::<_, 2, _>(|i| DiscoveryVariantInputV1 {
        id: decl[i].variant.operation,
        input: D::Object(&inputs[i]),
        outcomes: D::Text(
            "Original native invocation and independently authorized F06 V9 historical annotation-load identity and disposition",
        ),
        requirements: D::Text(
            "Original sealed proof or consumed read, independent current phase facts and actual native guards; metadata grants nothing",
        ),
        limits: D::Text("Explicit bounded native roles and parent-funded capture/result ceilings"),
        cancellation: D::Text(
            "Cooperative invocation stop is not read cancellation, producer discharge or rollback",
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
        contract: decl[0].contract,
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
    pub fn prepare_native_annotation_load_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<2>,
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
            if store.limits.max_entries < 1 || store.limits.max_variants < 2 {
                return Err(Error::Limit);
            }
            let authority = text(&store.authority)?;
            let command = command(
                &store.engine,
                "F17-annotation-load",
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
                    Origin::NativeAnnotationLoadV1,
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
    pub fn submit_discovered_annotation_load_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        invocation: &InvocationId,
        authority: &AnnotationLoadAuthorityV1,
        request: &AnnotationLoadRequestV1,
        admission: &AnnotationLoadAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let recognized = self.discovery_context(route.context).and_then(|c| {
            queries::annotation_load_selector_matches(
                c,
                route.entry,
                route.variant,
                request.argument_declaration_v1().variant.operation,
            )
        });
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
        self.submit_annotation_load_v1(invocation, authority, request, admission, r)
            .map_err(DiscoveredNativeCommandErrorV1::Invocation)
    }
}
