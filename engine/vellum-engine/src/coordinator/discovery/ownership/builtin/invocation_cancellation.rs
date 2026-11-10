//! Installed scoped producer cancellation, separate from observation control.
use super::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredInvocationCancellationErrorV1 {
    Discovery(DiscoveryErrorV1),
    Request(InvocationCancellationIngressErrorV1),
}
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<[[(&str, D<'_>); 1]; 3]>(),
        size_of::<[[(&str, D<'_>); 3]; 4]>(),
        size_of::<[(&str, D<'_>); 5]>(),
        size_of::<[D<'_>; 3]>(),
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
    let contract = [("const", D::Text(INVOCATION_CONTROL_PROFILE_V1))];
    let operation = [("const", D::Text("request-cancellation"))];
    let opaque = [("type", D::Text("object"))];
    let properties = [
        ("contract", D::Object(&contract)),
        ("operation", D::Object(&operation)),
        ("invocation", D::Object(&opaque)),
    ];
    let required = ["contract", "operation", "invocation"].map(D::Text);
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
    let fields = [
        ("contract", D::Text(INVOCATION_CONTROL_PROFILE_V1)),
        ("operation", D::Text("request-cancellation")),
        ("invocation", D::Text("OriginalOpaqueInvocationId")),
    ];
    let input = [
        ("contract", D::Text(INVOCATION_CONTROL_PROFILE_V1)),
        ("variant", D::Text("request-cancellation")),
        ("fields", D::Object(&fields)),
    ];
    let requirements = [
        ("schema", D::Object(&schema)),
        ("coverage", D::Text("outer-structure-only")),
        (
            "runtime",
            D::Text(
                "Original native ticket/owner and complete original context; independently trusted current owner-control, context, scope and protected readiness; discovery grants no authority",
            ),
        ),
    ];
    let variants = [DiscoveryVariantInputV1 {
        id: "request-cancellation",
        input: D::Object(&input),
        outcomes: D::Text(
            "Accepted monotone pending stop intent; refused/unavailable; fault remains unknown. No settlement result or disclosure",
        ),
        requirements: D::Object(&requirements),
        limits: D::Text(
            "Fixed 2/4/3-field borrowed input; one native pending ticket lookup; prepaid discovery workspace/reply limits",
        ),
        cancellation: D::Text(
            "Does not settle, publish, refund, retire, stop observation or assert external quiescence; real scheduler settlement is separate",
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
        contract: INVOCATION_CONTROL_PROFILE_V1,
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
    pub fn prepare_native_invocation_cancellation_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<1>,
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
            if s.limits.max_entries < 1 || s.limits.max_variants < 1 {
                return Err(Error::Limit);
            }
            let authority = text(&s.authority)?;
            let command = command(
                &s.engine,
                "F18-invocation-cancellation",
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
                    Origin::NativeInvocationCancellationV1,
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
    /// Visibility/current discovery release never replaces original native control
    /// authority. Direct protected ingress remains usable without discovery work.
    pub fn request_discovered_invocation_cancellation_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        envelope: &InvocationCancellationEnvelopeValueV1<'_, '_>,
        control: &TrustedInvocationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), DiscoveredInvocationCancellationErrorV1> {
        type E = DiscoveredInvocationCancellationErrorV1;
        let timer = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let n = normalize_invocation_cancellation_v1(envelope, r).map_err(E::Request)?;
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::invocation_cancellation_selector_matches(c, route.entry, route.variant)
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
            self.request_invocation_stop(n.invocation(), control, r)
                .map_err(|e| E::Request(InvocationCancellationIngressErrorV1::Execution(e)))
        }));
        let result = match caught {
            Ok(result) => result,
            Err(p) => {
                r.record(Observation {
                    boundary: Boundary::DiscoveryInvocationCancellationDispatch,
                    disposition: Disposition::Unknown,
                    service_time: timer.finish(),
                });
                std::panic::resume_unwind(p)
            }
        };
        let disposition = match result {
            Ok(()) => Disposition::Completed,
            Err(E::Discovery(Error::Faulted))
            | Err(E::Request(InvocationCancellationIngressErrorV1::Execution(
                InvocationError::Faulted,
            ))) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        };
        r.record(Observation {
            boundary: Boundary::DiscoveryInvocationCancellationDispatch,
            disposition,
            service_time: timer.finish(),
        });
        result
    }
}
