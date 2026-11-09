//! Installed protected stop/revocation; metadata grants neither control nor disclosure.
use super::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredObservationControlErrorV1 {
    Discovery(DiscoveryErrorV1),
    Stop(ObservationStopErrorV4),
    Revocation(ObservationRevocationIngressErrorV2),
}
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    // This one-entry/two-variant 3-field source is strictly smaller than the
    // existing two-entry/three-variant publication declaration frame.
    source_publication::workspace_bytes(maximum)
}
fn with_input<T>(
    authority: &str,
    command: &str,
    visibility: &NativeBuiltinEntryVisibilityV1<2>,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    let names = ["stop-observation", "revoke-observation-grant"];
    let contract = [("const", D::Text("read-context/1"))];
    let operations = names.map(|n| [("const", D::Text(n))]);
    let opaque = [("type", D::Text("object"))];
    let properties: [[(&str, D<'_>); 3]; 2] = std::array::from_fn(|i| {
        [
            ("contract", D::Object(&contract)),
            ("operation", D::Object(&operations[i])),
            ("context", D::Object(&opaque)),
        ]
    });
    let required = ["contract", "operation", "context"].map(D::Text);
    let schemas: [[(&str, D<'_>); 5]; 2] = std::array::from_fn(|i| {
        [
            (
                "$schema",
                D::Text("https://json-schema.org/draft/2020-12/schema"),
            ),
            ("type", D::Text("object")),
            ("properties", D::Object(&properties[i])),
            ("required", D::Array(&required)),
            ("additionalProperties", D::Boolean(false)),
        ]
    });
    let fields: [[(&str, D<'_>); 3]; 2] = std::array::from_fn(|i| {
        [
            ("contract", D::Text("read-context/1")),
            ("operation", D::Text(names[i])),
            ("context", D::Text("OriginalOpaqueObservationTargetV4")),
        ]
    });
    let inputs: [[(&str, D<'_>); 3]; 2] = std::array::from_fn(|i| {
        [
            ("contract", D::Text("read-context/1")),
            ("variant", D::Text(names[i])),
            ("fields", D::Object(&fields[i])),
        ]
    });
    let requirements: [[(&str, D<'_>); 3]; 2] = std::array::from_fn(|i| {
        [
            ("schema", D::Object(&schemas[i])),
            ("coverage", D::Text("outer-structure-only")),
            (
                "runtime",
                D::Text(
                    "Exact original native target and port, including receipt command/context pair; current native owner/scope control; separately attributable revocation authority; metadata and possession grant no control",
                ),
            ),
        ]
    });
    let variants: [_; 2] = std::array::from_fn(|i| DiscoveryVariantInputV1 {
        id: names[i],
        input: D::Object(&inputs[i]),
        requirements: D::Object(&requirements[i]),
        outcomes: D::Text(
            "Exact context closed, including retained-closed retry; refused or unknown preserves actual obligations",
        ),
        limits: D::Text(
            "Fixed 2/4/3 borrowed native input; prepaid discovery staging and native control; no payload release",
        ),
        cancellation: D::Text(
            "No producer cancellation, implicit retirement, refund, evidence erasure or sibling/grant-cohort closure",
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
    pub fn prepare_native_observation_control_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<2>,
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
            if s.limits.max_entries < 1 || s.limits.max_variants < 2 {
                return Err(Error::Limit);
            }
            let authority = text(&s.authority)?;
            let command = command(
                &s.engine,
                "F19-observation-control-v4",
                s.limits.max_context_bytes,
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
                    Origin::NativeObservationControlV1,
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
    fn recognize_observation_control_v1(
        &mut self,
        route: &NativeDiscoveredCommandRouteV1<'_>,
        operation: &str,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let recognized = self.discovery_context(route.context).and_then(|c| {
            queries::observation_control_selector_matches(c, route.entry, route.variant, operation)
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
        Ok(())
    }
    /// Direct protected control remains available without discovery readiness.
    pub fn stop_discovered_observation_context_v4(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        envelope: &ObservationStopEnvelopeValueV4<'_, '_>,
        port: ObservationStopPortV4<'_>,
        r: &mut Recorder,
    ) -> Result<(), DiscoveredObservationControlErrorV1> {
        type E = DiscoveredObservationControlErrorV1;
        dispatch(r, |r| {
            let n = normalize_observation_stop_envelope_v4(envelope, r).map_err(E::Stop)?;
            if !port.matches(n.target()) {
                return Err(E::Stop(ObservationStopErrorV4::Unavailable));
            }
            self.recognize_observation_control_v1(&route, "stop-observation", r)
                .map_err(E::Discovery)?;
            self.stop_observation_facade_v4(port, r).map_err(E::Stop)
        })
    }
    pub fn revoke_discovered_observation_context_v2(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        envelope: &ObservationRevocationEnvelopeValueV2<'_, '_>,
        port: ObservationStopPortV4<'_>,
        authority: &TrustedObservationRevocationV2<'_>,
        r: &mut Recorder,
    ) -> Result<(), DiscoveredObservationControlErrorV1> {
        type E = DiscoveredObservationControlErrorV1;
        dispatch(r, |r| {
            let n = normalize_observation_revocation_v2(envelope, r).map_err(E::Revocation)?;
            if !port.matches(n.target()) || authority.original_target != n.target() {
                return Err(E::Revocation(
                    ObservationRevocationIngressErrorV2::Execution(
                        ObservationRevocationErrorV2::Unavailable,
                    ),
                ));
            }
            self.recognize_observation_control_v1(&route, "revoke-observation-grant", r)
                .map_err(E::Discovery)?;
            self.revoke_observation_context_v2(n.target(), port, authority, r)
                .map_err(|e| E::Revocation(ObservationRevocationIngressErrorV2::Execution(e)))
        })
    }
}
fn dispatch<T>(
    r: &mut Recorder,
    call: impl FnOnce(&mut Recorder) -> Result<T, DiscoveredObservationControlErrorV1>,
) -> Result<T, DiscoveredObservationControlErrorV1> {
    type E = DiscoveredObservationControlErrorV1;
    let timer = r.begin_service_timing();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| call(r)));
    let disposition = match &caught {
        Ok(Ok(_)) => Disposition::Completed,
        Ok(Err(E::Discovery(Error::Faulted))) | Err(_) => Disposition::Unknown,
        Ok(Err(E::Stop(e))) if e.faulted() => Disposition::Unknown,
        Ok(Err(E::Revocation(ObservationRevocationIngressErrorV2::Execution(
            ObservationRevocationErrorV2::Native(e),
        )))) if e.faulted() => Disposition::Unknown,
        _ => Disposition::Refused,
    };
    r.record(Observation {
        boundary: Boundary::DiscoveryObservationControlDispatch,
        disposition,
        service_time: timer.finish(),
    });
    match caught {
        Ok(result) => result,
        Err(p) => std::panic::resume_unwind(p),
    }
}
