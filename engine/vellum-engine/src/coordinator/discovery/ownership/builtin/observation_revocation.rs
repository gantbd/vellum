//! Installed exact-context revocation. Discovery visibility grants no control.
use super::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredObservationRevocationErrorV1 {
    Discovery(DiscoveryErrorV1),
    Request(ObservationRevocationIngressErrorV1),
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
    let contract = [("const", D::Text("read-context/1"))];
    let operation = [("const", D::Text("revoke-observation-grant"))];
    let opaque = [("type", D::Text("object"))];
    let properties = [
        ("contract", D::Object(&contract)),
        ("operation", D::Object(&operation)),
        ("context", D::Object(&opaque)),
    ];
    let required = ["contract", "operation", "context"].map(D::Text);
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
        ("contract", D::Text("read-context/1")),
        ("operation", D::Text("revoke-observation-grant")),
        ("context", D::Text("OriginalOpaqueObservationContextV1")),
    ];
    let input = [
        ("contract", D::Text("read-context/1")),
        ("variant", D::Text("revoke-observation-grant")),
        ("fields", D::Object(&fields)),
    ];
    let requirements = [
        ("schema", D::Object(&schema)),
        ("coverage", D::Text("outer-structure-only")),
        (
            "runtime",
            D::Text(
                "Exact original target/port/authority and separate current revocation permission; native owner scope/control; metadata and target possession grant no authority",
            ),
        ),
    ];
    let variants = [DiscoveryVariantInputV1 {
        id: "revoke-observation-grant",
        input: D::Object(&input),
        outcomes: D::Text(
            "Context authorization withdrawn, including retained-closed retry; unavailable/refused preserves obligations; retired targets are not fabricated success",
        ),
        requirements: D::Object(&requirements),
        limits: D::Text(
            "Fixed 2/4/3 inert input; one existing native control; existing discovery parent/workspace/reply and native retention limits",
        ),
        cancellation: D::Text(
            "Context-local only; does not revoke siblings/future independent admissions, cancel producers, erase evidence or retire holds",
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
        contract: "read-context/1",
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
    /// Registers the executable declaration source, not a target grant. Native
    /// context existence, scope and permission are revalidated at execution.
    pub fn prepare_native_observation_revocation_discovery_cut_v1(
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
                "F19-context-revocation",
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
                    Origin::NativeObservationRevocationV1,
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
    /// Installed discovery release and native revocation authority are separate.
    /// Protected direct revocation remains available without fresh discovery work.
    pub fn revoke_discovered_observation_context_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        envelope: &ObservationRevocationEnvelopeValueV1<'_, '_>,
        port: ObservationStopPortV3<'_>,
        authority: &TrustedObservationRevocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), DiscoveredObservationRevocationErrorV1> {
        type E = DiscoveredObservationRevocationErrorV1;
        let timer = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let n = normalize_observation_revocation_v1(envelope, r).map_err(E::Request)?;
            if !port.matches(n.target()) || authority.original_target != n.target() {
                return Err(E::Request(ObservationRevocationIngressErrorV1::Execution(
                    ObservationRevocationErrorV1::Unavailable,
                )));
            }
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::observation_revocation_selector_matches(c, route.entry, route.variant)
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
            self.revoke_observation_context_v1(n.target(), port, authority, r)
                .map_err(|e| E::Request(ObservationRevocationIngressErrorV1::Execution(e)))
        }));
        let result = match caught {
            Ok(result) => result,
            Err(p) => {
                r.record(Observation {
                    boundary: Boundary::DiscoveryObservationRevocationDispatch,
                    disposition: Disposition::Unknown,
                    service_time: timer.finish(),
                });
                std::panic::resume_unwind(p)
            }
        };
        let disposition = match result {
            Ok(()) => Disposition::Completed,
            Err(E::Discovery(Error::Faulted)) => Disposition::Unknown,
            Err(E::Request(ObservationRevocationIngressErrorV1::Execution(
                ObservationRevocationErrorV1::Native(e),
            ))) if e.faulted() => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        };
        r.record(Observation {
            boundary: Boundary::DiscoveryObservationRevocationDispatch,
            disposition,
            service_time: timer.finish(),
        });
        result
    }
}
