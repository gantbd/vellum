//! Independent installed F17 container source. Existing /1–/4 inventories stay
//! unchanged; the scoped /1 owner stores this source with a private origin.
use super::*;
#[cfg(test)]
mod tests;

pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<[[(&str, D<'_>); 7]; 4]>(),
        size_of::<[[(&str, D<'_>); 4]; 4]>(),
        size_of::<[D<'_>; 4]>(),
        size_of::<[[(&str, D<'_>); 3]; 2]>(),
        size_of::<[[(&str, D<'_>); 2]; 2]>(),
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
    let forms = native_relation_argument_declarations_v1();
    let fields: [[(&str, D<'_>); 7]; 4] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            forms[i]
                .variant
                .fields
                .get(j)
                .map_or(("", D::Null), |(key, kind)| (*key, D::Text(kind)))
        })
    });
    let descriptions: [[(&str, D<'_>); 4]; 4] = std::array::from_fn(|i| {
        [
            ("contract", D::Text(forms[i].contract)),
            ("operation", D::Text(forms[i].variant.operation)),
            (
                "action",
                D::Text(if i % 2 == 0 { "create" } else { "retire" }),
            ),
            (
                "fields",
                D::Object(&fields[i][..forms[i].variant.fields.len()]),
            ),
        ]
    });
    let values: [D<'_>; 4] = std::array::from_fn(|i| D::Object(&descriptions[i]));
    let names = ["view-container", "workspace-container"];
    let groups: [[(&str, D<'_>); 2]; 2] = std::array::from_fn(|i| {
        [
            ("profile", D::Text(NATIVE_RELATION_LIFECYCLE_PROFILE_V1)),
            ("forms", D::Array(&values[i * 2..i * 2 + 2])),
        ]
    });
    let inputs: [[(&str, D<'_>); 3]; 2] = std::array::from_fn(|i| {
        [
            ("contract", D::Text("relation-invocation/1")),
            ("variant", D::Text(names[i])),
            ("fields", D::Object(&groups[i])),
        ]
    });
    let variants: [DiscoveryVariantInputV1<'_>; 2] = std::array::from_fn(|i| {
        DiscoveryVariantInputV1 {
            id: names[i],
            input: D::Object(&inputs[i]),
            outcomes: D::Text(
                "Original invocation and independent authorized historical relation result",
            ),
            requirements: D::Text(
                "Trusted-host native execution; discovery grants no execution or disclosure rights",
            ),
            limits: D::Text(
                "Explicit native normalization, invocation and result limits; shared parent funding",
            ),
            cancellation: D::Text(
                "Original cooperative invocation control; stopping observation does not cancel work",
            ),
            order: visibility.variants[i].order,
            enabled: visibility.variants[i].enabled,
        }
    });
    let entries = [DiscoveryEntryInputV1 {
        command: DiscoveryCommandV1 {
            authority,
            id: command,
            incarnation: 1,
        },
        contract: "relation-invocation/1",
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
    /// Fixed installed source using the existing scoped discovery /1 owner, not
    /// an extension to the closed `prepare_native_builtin_discovery_cut_v1` list.
    /// All visibility and release facts remain independent of command rights.
    pub fn prepare_native_container_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<2>,
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
            if store.limits.max_entries < 1 || store.limits.max_variants < 2 {
                return Err(Error::Limit);
            }
            let authority = text(&store.authority)?;
            let command = command(
                &store.engine,
                "F17-containers",
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
                    Origin::NativeContainersV1,
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
    /// Trusted-native container execution only. Original selectors and fresh
    /// discovery release precede the existing normalized, one-shot relation owner.
    /// Optional result admission never grants mutation rights to an untrusted caller.
    pub fn submit_discovered_native_container_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        arguments: &NormalizedRelationArgumentsV1<'_>,
        result: Option<&TrustedRelationResultAdmissionV1<'_>>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let recognized = self.discovery_context(route.context).and_then(|c| {
            queries::container_selector_matches(
                c,
                route.entry,
                route.variant,
                arguments.argument_declaration_v1().variant.operation,
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
        match result {
            Some(facts) => self.submit_relation_arguments_with_result_v1(arguments, facts, r),
            None => self.submit_relation_arguments_v1(arguments, r),
        }
        .map_err(DiscoveredNativeCommandErrorV1::Invocation)
    }
}
