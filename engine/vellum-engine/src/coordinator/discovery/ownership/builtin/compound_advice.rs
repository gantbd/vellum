//! Fixed installed F10 query; metadata grants no proposal disclosure.
use super::*;
#[cfg(test)]
mod tests;
fn with_input<T>(
    authority: &str,
    command: &str,
    visibility: &NativeBuiltinEntryVisibilityV1<1>,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    // This one three-field variant/entry and two owned strings fit the already
    // prepaid source_read frame (four seven-field variants/three entries/four
    // strings). Synchronous exclusive-owner preparations cannot coexist.
    let contract = [("const", D::Text("proposal-validation/1"))];
    let operation = [("const", D::Text("advisory-validation"))];
    let properties = [
        ("contract", D::Object(&contract)),
        ("operation", D::Object(&operation)),
        ("plan", D::Object(&[])),
    ];
    let required = [D::Text("contract"), D::Text("operation"), D::Text("plan")];
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
        ("contract", D::Text("proposal-validation/1")),
        ("operation", D::Text("advisory-validation")),
        ("plan", D::Text("OriginalBorrowedNativeCompoundIntent")),
    ];
    let input = [
        ("contract", D::Text("proposal-validation/1")),
        ("variant", D::Text("advisory-validation")),
        ("fields", D::Object(&roles)),
    ];
    let requirements = [
        ("schema", D::Object(&schema)),
        ("coverage", D::Text("outer-structure-only")),
        (
            "runtime",
            D::Text(
                "Closed typed/dynamic header and original plan/proposal pairing; original request/root/context/policy basis; complete current native observability; no logical-plan JSON/wire codec",
            ),
        ),
    ];
    let variants = [DiscoveryVariantInputV1 {
        id: "advisory-validation",
        input: D::Object(&input),
        requirements: D::Object(&requirements),
        outcomes: D::Text(
            "Fixed-profile advice with checksSatisfied Boolean, or status-only unavailable; no prepared transaction, invocation, reservation or publication capability",
        ),
        limits: D::Text(
            "Actual intent ceiling; fixed classification/eight native request frames funded by actual parent; existing candidate/read/scope/root/history/view bounds",
        ),
        cancellation: D::Text(
            "One bounded synchronous query; no invocation, task or retained result; future publication independently revalidates",
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
        contract: "proposal-validation/1",
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
    pub fn prepare_native_compound_advice_discovery_cut_v1(
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
            if store.limits.max_entries == 0 || store.limits.max_variants == 0 {
                return Err(Error::Limit);
            }
            let authority = text(&store.authority)?;
            let command = command(
                &store.engine,
                "F10-compound-advice-v1",
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
                    Origin::NativeCompoundAdviceV1,
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
    pub fn query_discovered_compound_advice_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        envelope: &CompoundAdviceEnvelopeValueV1<'_, '_>,
        facts: &TrustedCompoundAdviceFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<CompoundAdviceReplyV1, DiscoveryErrorV1> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if crate::coordinator::proposal_advice::ingress::recognize(envelope).is_none() {
                return Ok(CompoundAdviceReplyV1::Unavailable);
            }
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::compound_advice_selector_matches(c, route.entry, route.variant)
            });
            observe(r, Boundary::DiscoveryRequestRecognition, &recognized);
            recognized?;
            self.query_discovery_v1(
                route.context,
                route.scope,
                route.cut,
                DiscoveryQueryV1::Inspect { entry: route.entry },
                route.facts,
                r,
            )?;
            Ok(self.query_decoded_compound_advice_v1(envelope, facts, r))
        })();
        if let Ok(out) = &result {
            self.record_compound_advice_dispatch(
                r,
                Boundary::DiscoveryCompoundAdviceDispatch,
                out,
                timing.finish(),
            );
        } else {
            r.record(Observation {
                boundary: Boundary::DiscoveryCompoundAdviceDispatch,
                disposition: if matches!(result, Err(Error::Faulted)) {
                    Disposition::Unknown
                } else {
                    Disposition::Refused
                },
                service_time: timing.finish(),
            });
        }
        result
    }
}
