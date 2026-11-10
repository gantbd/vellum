//! Installed F06 descriptors bind actual native query handlers, never authority.
use super::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredInvocationObservationErrorV1 {
    Discovery(DiscoveryErrorV1),
    Native(InvocationError),
}
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<[(&str, D<'_>); 11]>(),
        size_of::<[&str; 18]>(),
        size_of::<[[(&str, D<'_>); 7]; 4]>() * 2,
        size_of::<[[(&str, D<'_>); 5]; 4]>(),
        size_of::<[[(&str, D<'_>); 3]; 4]>() * 2,
        size_of::<[D<'_>; 7]>(),
        size_of::<[DiscoveryVariantInputV1<'_>; 4]>(),
        size_of::<[DiscoveryEntryInputV1<'_>; 1]>(),
        size_of::<DiscoveryCutInputV1<'_>>(),
        size_of::<TrustedDiscoveryFactsV1>(),
    ]
    .into_iter()
    .try_fold(0, add)
}
fn with_input<T, const N: usize>(
    authority: &str,
    command: &str,
    receipt: bool,
    visibility: &NativeBuiltinEntryVisibilityV1<N>,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    // Only the two fixed public factories call this private constructor.
    assert_eq!(N, if receipt { 1 } else { 4 });
    let contract_name = if receipt {
        "execution-receipt/1"
    } else {
        "execution-observation/1"
    };
    let names = if receipt {
        ["retained-outcome", "", "", ""]
    } else {
        [
            "status",
            "retained-outcome",
            "minimal-control-status",
            "bounded-observation-wait",
        ]
    };
    let keys = [
        "contract",
        "operation",
        "invocation",
        "context",
        "profile",
        if receipt { "facet" } else { "after" },
        "budget",
    ];
    let roles = [
        contract_name,
        "",
        "OriginalOpaqueInvocationV1",
        "OriginalOpaqueInvocationObservationContextV1",
        "AdmittedProfile",
        if receipt {
            "ClosedNativeReceiptFacetAndExactAtomicIndex"
        } else {
            "OriginalOrdinaryObservationReferenceV1"
        },
        "PositiveSafeIntegerWithinNativeWaitLimit",
    ];
    let count = |i| {
        if receipt {
            6
        } else if i == 3 {
            7
        } else {
            5
        }
    };
    let contract = [("const", D::Text(contract_name))];
    let operations = names.map(|n| [("const", D::Text(n))]);
    let opaque = [("type", D::Text("object"))];
    let text = [("type", D::Text("string")), ("minLength", D::Number(1.0))];
    let budget = [
        ("type", D::Text("integer")),
        ("minimum", D::Number(1.0)),
        ("maximum", D::Number(9_007_199_254_740_991.0)),
    ];
    let properties: [[(&str, D<'_>); 7]; 4] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            (
                keys[j],
                match j {
                    0 => D::Object(&contract),
                    1 => D::Object(&operations[i]),
                    4 => D::Object(&text),
                    6 => D::Object(&budget),
                    _ => D::Object(&opaque),
                },
            )
        })
    });
    let required = keys.map(D::Text);
    let schemas: [[(&str, D<'_>); 5]; 4] = std::array::from_fn(|i| {
        [
            (
                "$schema",
                D::Text("https://json-schema.org/draft/2020-12/schema"),
            ),
            ("type", D::Text("object")),
            ("properties", D::Object(&properties[i][..count(i)])),
            ("required", D::Array(&required[..count(i)])),
            ("additionalProperties", D::Boolean(false)),
        ]
    });
    let fields: [[(&str, D<'_>); 7]; 4] = std::array::from_fn(|i| {
        std::array::from_fn(|j| (keys[j], D::Text(if j == 1 { names[i] } else { roles[j] })))
    });
    let inputs: [[(&str, D<'_>); 3]; 4] = std::array::from_fn(|i| {
        [
            ("contract", D::Text(contract_name)),
            ("variant", D::Text(names[i])),
            ("fields", D::Object(&fields[i][..count(i)])),
        ]
    });
    let requirements: [[(&str, D<'_>); 3]; 4] = std::array::from_fn(|i| {
        [
            ("schema", D::Object(&schemas[i])),
            ("coverage", D::Text("outer-structure-only")),
            (
                "runtime",
                D::Text(
                    "Exact original invocation/context/profile; independent current native observation and receipt authority; bounded original ordinary after-reference; complete native reply limits; facet kind/index validated at runtime, not by metadata",
                ),
            ),
        ]
    });
    let variants: [DiscoveryVariantInputV1<'_>; N] = std::array::from_fn(|i| {
        DiscoveryVariantInputV1 {
            id: names[i],
            input: D::Object(&inputs[i]),
            requirements: D::Object(&requirements[i]),
            outcomes: D::Text(
                "Original filtered native observation, wait identity or historical receipt; unavailable/limit/fault remain explicit",
            ),
            limits: D::Text(
                "Fixed borrowed 2/4/5-or-7 observation or 2/4/6 receipt input; existing native context, wait, receipt and reply limits; prepaid discovery frame",
            ),
            cancellation: D::Text(
                "No execution/replay/producer cancellation; wait continuation, context stop and protected retirement retain independent native owners",
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
        contract: contract_name,
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
    fn prepare_native_invocation_observation_source<const N: usize>(
        &mut self,
        scope: &DiscoveryScopeV1,
        receipt: bool,
        visibility: &NativeBuiltinEntryVisibilityV1<N>,
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
            if s.limits.max_entries < 1 || s.limits.max_variants < N {
                return Err(Error::Limit);
            }
            let authority = text(&s.authority)?;
            let command = command(
                &s.engine,
                if receipt {
                    "F06-native-receipt-v10"
                } else {
                    "F06-native-observation-v1"
                },
                s.limits.max_context_bytes,
            )?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            with_input(&authority, &command, receipt, visibility, |i| {
                self.prepare_discovery_cut_from_source(
                    scope,
                    i,
                    &facts,
                    Origin::NativeInvocationObservationV1,
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
    pub fn prepare_native_invocation_observation_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<4>,
        facts: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        self.prepare_native_invocation_observation_source(scope, false, visibility, facts, r)
    }
    pub fn prepare_native_invocation_receipt_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<1>,
        facts: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        self.prepare_native_invocation_observation_source(scope, true, visibility, facts, r)
    }
    fn recognize_invocation_observation_discovery(
        &mut self,
        route: &NativeDiscoveredCommandRouteV1<'_>,
        contract: &str,
        operation: &str,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let result = self.discovery_context(route.context).and_then(|c| {
            queries::invocation_observation_selector_matches(
                c,
                route.entry,
                route.variant,
                contract,
                operation,
            )
        });
        observe(r, Boundary::DiscoveryRequestRecognition, &result);
        result?;
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
    pub fn query_discovered_invocation_observation_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        observer: &mut InvocationObservationContextV1,
        envelope: &InvocationObservationEnvelopeValueV1<'_, '_>,
        port: InvocationObservationReadPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationObservationReadReplyV1, DiscoveredInvocationObservationErrorV1> {
        type E = DiscoveredInvocationObservationErrorV1;
        let timer = r.begin_service_timing();
        let recognized = self
            .recognize_native_invocation_observation_request_v1(observer, envelope, &port)
            .map_err(E::Native)
            .and_then(|op| {
                self.recognize_invocation_observation_discovery(
                    &route,
                    "execution-observation/1",
                    op,
                    r,
                )
                .map_err(E::Discovery)
            });
        let result = match recognized {
            Ok(()) => self
                .query_decoded_invocation_observation_v1(observer, envelope, port, r)
                .map_err(E::Native),
            Err(e) => Err(e),
        };
        record(r, &result, timer.finish());
        result
    }
    #[allow(clippy::too_many_arguments)] // Recipient input, selector and independent native release authority stay separate.
    pub fn query_discovered_invocation_receipt_v10<'s>(
        &'s mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        observer: &mut InvocationObservationContextV1,
        envelope: &InvocationReceiptEnvelopeValueV1<'_, '_>,
        native: InvocationReceiptRequestV10<'_>,
        current: &InvocationObservationPolicyV1,
        facts: &TrustedInvocationReceiptReleaseV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationReceiptV10<'s>, DiscoveredInvocationObservationErrorV1> {
        type E = DiscoveredInvocationObservationErrorV1;
        let timer = r.begin_service_timing();
        let recognized = self
            .recognize_native_invocation_receipt_request_v10(observer, envelope, native)
            .map_err(E::Native)
            .and_then(|()| {
                self.recognize_invocation_observation_discovery(
                    &route,
                    "execution-receipt/1",
                    "retained-outcome",
                    r,
                )
                .map_err(E::Discovery)
            });
        let result = match recognized {
            Ok(()) => self
                .query_decoded_invocation_receipt_v10(observer, envelope, native, current, facts, r)
                .map_err(E::Native),
            Err(e) => Err(e),
        };
        record(r, &result, timer.finish());
        result
    }
}
fn record<T>(
    r: &mut Recorder,
    result: &Result<T, DiscoveredInvocationObservationErrorV1>,
    service_time: Option<std::time::Duration>,
) {
    type E = DiscoveredInvocationObservationErrorV1;
    r.record(Observation {
        boundary: Boundary::DiscoveryInvocationObservationDispatch,
        disposition: match result {
            Ok(_) => Disposition::Completed,
            Err(E::Native(InvocationError::Faulted)) | Err(E::Discovery(Error::Faulted)) => {
                Disposition::Unknown
            }
            _ => Disposition::Refused,
        },
        service_time,
    });
}
