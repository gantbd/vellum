//! Installed native F17 retained-disposal substeps, not a new public verb,
//! normalized envelope, publisher or erasure/effect-cancellation facility.
use super::*;
#[cfg(test)]
mod tests;
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<[NativeArgumentDeclarationV1; 3]>(),
        size_of::<[[(&str, D<'_>); 3]; 3]>() * 2,
        size_of::<[D<'_>; 3]>(),
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
    let forms = [
        RetainedReleaseOperationV1::SourceBinding,
        RetainedReleaseOperationV1::AnnotationBinding,
        RetainedReleaseOperationV1::Provenance,
    ]
    .map(|op| op.argument_declaration_v1());
    let fields: [[(&str, D<'_>); 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let (key, value) = forms[i].variant.fields[j];
            (key, D::Text(value))
        })
    });
    let descriptions: [[(&str, D<'_>); 3]; 3] = std::array::from_fn(|i| {
        [
            ("contract", D::Text(forms[i].contract)),
            ("operation", D::Text(forms[i].variant.operation)),
            ("fields", D::Object(&fields[i])),
        ]
    });
    let values: [D<'_>; 3] = std::array::from_fn(|i| D::Object(&descriptions[i]));
    let roles = [
        ("invocation", D::Text("OriginalOpaqueInvocation")),
        ("request", D::Array(&values)),
        (
            "admission",
            D::Text("IndependentRetainedReleaseAdmissionV1"),
        ),
    ];
    let input = [
        ("contract", D::Text(forms[0].contract)),
        ("variant", D::Text("dispose")),
        ("fields", D::Object(&roles)),
    ];
    let variants = [DiscoveryVariantInputV1 {
        id: "dispose",
        input: D::Object(&input),
        outcomes: D::Text(
            "Original retained-release invocation; separately authorized F06 historical result",
        ),
        requirements: D::Text(
            "Independent original admission and fresh native release service facts; no grants from discovery",
        ),
        limits: D::Text("Explicit native capture/result bounds and actual shared-parent retention"),
        cancellation: D::Text(
            "Original cooperative stop; release is not payload erasure or effect cancellation",
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
        contract: forms[0].contract,
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
    /// Fixed native source on the existing scoped /1 owner. Earlier builtin
    /// inventories and exhaustive request/result profiles remain unchanged.
    pub fn prepare_native_retained_release_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<1>,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V1 {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, f, r)?;
            let store = self.discovery_store()?;
            if store.limits.max_entries < 1 || store.limits.max_variants < 1 {
                return Err(Error::Limit);
            }
            let authority = text(&store.authority)?;
            let command = command(
                &store.engine,
                "F17-retained-release",
                store.limits.max_context_bytes,
            )?;
            let f = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*f
            };
            with_input(&authority, &command, visibility, |input| {
                self.prepare_discovery_cut_from_source(
                    scope,
                    input,
                    &f,
                    Origin::NativeRetainedReleaseV1,
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
    /// Recognizes original source/roles before observed loss, then delegates the
    /// unchanged typed request to its existing independently authorized owner.
    pub fn submit_discovered_retained_release_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        invocation: &InvocationId,
        request: &RetainedReleaseRequestV1,
        admission: &RetainedReleaseAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let recognized = self.discovery_context(route.context).and_then(|c| {
            queries::retained_release_selector_matches(c, route.entry, route.variant)
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
        self.submit_retained_release_v1(invocation, request, admission, r)
            .map_err(DiscoveredNativeCommandErrorV1::Invocation)
    }
}
