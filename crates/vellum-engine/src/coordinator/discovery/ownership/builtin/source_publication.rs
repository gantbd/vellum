//! Installed source publication commands and historical receipts. Metadata is
//! descriptive; original opaque roles and current native authority remain required.
use super::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredSourcePublicationErrorV1 {
    Discovery(DiscoveryErrorV1),
    Request(SourcePublicationIngressErrorV1),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeSourcePublicationDiscoveryVisibilityV1 {
    pub commands: NativeBuiltinEntryVisibilityV1<2>,
    pub receipts: NativeBuiltinEntryVisibilityV1<1>,
}
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(3, maximum)?,
        size_of::<[String; 3]>(),
        // Conservative complete fixed frame, including both schemas and roles.
        size_of::<[[(&str, D<'_>); 5]; 24]>(),
        size_of::<[[D<'_>; 5]; 3]>(),
        size_of::<[DiscoveryVariantInputV1<'_>; 3]>(),
        size_of::<[DiscoveryEntryInputV1<'_>; 2]>(),
        size_of::<DiscoveryCutInputV1<'_>>(),
        size_of::<TrustedDiscoveryFactsV1>(),
    ]
    .into_iter()
    .try_fold(0, add)
}
fn with_input<T>(
    authority: &str,
    commands: &[String; 2],
    visibility: &NativeSourcePublicationDiscoveryVisibilityV1,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    let contracts = [
        SOURCE_PUBLICATION_COMMAND_PROFILE_V1,
        SOURCE_PUBLICATION_COMMAND_PROFILE_V1,
        SOURCE_PUBLICATION_RECEIPT_PROFILE_V1,
    ];
    let names = [
        "establish-bounded-stream",
        "authorized-resynchronization",
        "retained-outcome",
    ];
    let constants = contracts.map(|v| [("const", D::Text(v))]);
    let operations = names.map(|v| [("const", D::Text(v))]);
    let opaque = [("type", D::Text("object"))];
    let null = [("type", D::Text("null"))];
    let facet_kind = [("const", D::Text("source-baseline"))];
    let facet_properties = [("kind", D::Object(&facet_kind))];
    let facet_required = [D::Text("kind")];
    let facet = [
        ("type", D::Text("object")),
        ("properties", D::Object(&facet_properties)),
        ("required", D::Array(&facet_required)),
        ("additionalProperties", D::Boolean(false)),
    ];
    let command_properties: [[(&str, D<'_>); 4]; 2] = std::array::from_fn(|i| {
        [
            ("contract", D::Object(&constants[i])),
            ("operation", D::Object(&operations[i])),
            ("command", D::Object(&opaque)),
            (
                "stream",
                if i == 0 {
                    D::Object(&null)
                } else {
                    D::Object(&opaque)
                },
            ),
        ]
    });
    let receipt_properties = [
        ("contract", D::Object(&constants[2])),
        ("operation", D::Object(&operations[2])),
        ("command", D::Object(&opaque)),
        ("context", D::Object(&opaque)),
        ("facet", D::Object(&facet)),
    ];
    let command_required = ["contract", "operation", "command", "stream"].map(D::Text);
    let receipt_required = ["contract", "operation", "command", "context", "facet"].map(D::Text);
    let schemas: [[(&str, D<'_>); 5]; 3] = std::array::from_fn(|i| {
        [
            (
                "$schema",
                D::Text("https://json-schema.org/draft/2020-12/schema"),
            ),
            ("type", D::Text("object")),
            (
                "properties",
                D::Object(if i < 2 {
                    &command_properties[i]
                } else {
                    &receipt_properties
                }),
            ),
            (
                "required",
                D::Array(if i < 2 {
                    &command_required
                } else {
                    &receipt_required
                }),
            ),
            ("additionalProperties", D::Boolean(false)),
        ]
    });
    let command_roles: [[(&str, D<'_>); 4]; 2] = std::array::from_fn(|i| {
        [
            ("contract", D::Text(contracts[i])),
            ("operation", D::Text(names[i])),
            (
                "command",
                D::Text("OriginalOpaqueSourcePublicationCommandV1"),
            ),
            (
                "stream",
                D::Text(if i == 0 {
                    "Null"
                } else {
                    "OriginalOpaqueSourcePublicationStreamV1"
                }),
            ),
        ]
    });
    let receipt_roles = [
        ("contract", D::Text(contracts[2])),
        ("operation", D::Text(names[2])),
        (
            "command",
            D::Text("OriginalOpaqueSourcePublicationCommandV1"),
        ),
        (
            "context",
            D::Text("OriginalOpaqueSourcePublicationReceiptContextV1"),
        ),
        ("facet", D::Text("SourceBaselineFacetV1")),
    ];
    let inputs: [[(&str, D<'_>); 3]; 3] = std::array::from_fn(|i| {
        [
            ("contract", D::Text(contracts[i])),
            ("variant", D::Text(names[i])),
            (
                "fields",
                D::Object(if i < 2 {
                    &command_roles[i]
                } else {
                    &receipt_roles
                }),
            ),
        ]
    });
    let requirements: [[(&str, D<'_>); 3]; 3] = std::array::from_fn(|i| {
        [
            ("schema", D::Object(&schemas[i])),
            ("coverage", D::Text("outer-structure-only")),
            (
                "runtime",
                D::Text(
                    "Original native command, stream or receipt identities; independently current command and historical result authority; metadata grants nothing",
                ),
            ),
        ]
    });
    let variants: [_; 3] = std::array::from_fn(|i| {
        let v = if i < 2 {
            visibility.commands.variants[i]
        } else {
            visibility.receipts.variants[0]
        };
        DiscoveryVariantInputV1 {
            id: names[i],
            input: D::Object(&inputs[i]),
            outcomes: D::Text(if i < 2 {
                "command-settled or observe-existing, identity only"
            } else {
                "operation-receipt, original complete source-baseline result"
            }),
            requirements: D::Object(&requirements[i]),
            limits: D::Text(
                "Fixed command 2/4/4 or receipt 2/4/5/1 input; prepaid native and discovery budgets",
            ),
            cancellation: D::Text(
                "Stop is not command cancellation; retry never reexecutes a settled original",
            ),
            order: v.order,
            enabled: v.enabled,
        }
    });
    let entries: [_; 2] = std::array::from_fn(|i| {
        let (contract, kind, variants, order, schema, presentation, advice) = if i == 0 {
            let v = &visibility.commands;
            (
                contracts[0],
                DiscoveryExecutionKindV1::LocalCommand,
                &variants[..2],
                v.order,
                v.schema,
                v.presentation,
                v.advice,
            )
        } else {
            let v = &visibility.receipts;
            (
                contracts[2],
                DiscoveryExecutionKindV1::Query,
                &variants[2..],
                v.order,
                v.schema,
                v.presentation,
                v.advice,
            )
        };
        DiscoveryEntryInputV1 {
            command: DiscoveryCommandV1 {
                authority,
                id: &commands[i],
                incarnation: 1,
            },
            contract,
            version: 1,
            kind,
            variants,
            presentation: D::Object(&[]),
            order,
            schema_visible: schema,
            presentation_visible: presentation,
            advice_visible: advice,
        }
    });
    consume(DiscoveryCutInputV1 {
        entries: &entries,
        target: None,
        advice_present: false,
    })
}
impl SourceCoordinator {
    pub fn prepare_native_source_publication_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeSourcePublicationDiscoveryVisibilityV1,
        facts: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V1 {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, facts, r)?;
            self.source_publication_discovery_source_v1(
                &scope.context.authority,
                &scope.context.engine,
            )
            .map_err(|_| Error::Unavailable)?;
            let s = self.discovery_store()?;
            if s.limits.max_entries < 2 || s.limits.max_variants < 2 {
                return Err(Error::Limit);
            }
            let authority = text(&s.authority)?;
            let commands = [
                command(
                    &s.engine,
                    "F19-source-publication",
                    s.limits.max_context_bytes,
                )?,
                command(
                    &s.engine,
                    "F06-source-publication",
                    s.limits.max_context_bytes,
                )?,
            ];
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*facts
            };
            with_input(&authority, &commands, visibility, |input| {
                self.prepare_discovery_cut_from_source(
                    scope,
                    input,
                    &facts,
                    Origin::NativeSourcePublicationV1,
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
    fn recognize_source_publication_route_v1(
        &mut self,
        route: &NativeDiscoveredCommandRouteV1<'_>,
        contract: &str,
        operation: &str,
        kind: DiscoveryExecutionKindV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let recognized = self.discovery_context(route.context).and_then(|c| {
            queries::source_publication_selector_matches(
                c,
                route.entry,
                route.variant,
                contract,
                operation,
                kind,
            )
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
    pub fn request_discovered_source_publication_command_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        envelope: &SourcePublicationEnvelopeValueV1<'_, '_>,
        port: &SourcePublicationCommandPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourcePublicationCommandReplyV1, DiscoveredSourcePublicationErrorV1> {
        source_dispatch(r, |r| {
            let n = normalize_source_publication_command_v1(envelope, r)
                .map_err(DiscoveredSourcePublicationErrorV1::Request)?;
            if !n.matches(port) {
                return Err(DiscoveredSourcePublicationErrorV1::Request(
                    SourcePublicationIngressErrorV1::InvalidRequest,
                ));
            }
            self.recognize_source_publication_route_v1(
                &route,
                SOURCE_PUBLICATION_COMMAND_PROFILE_V1,
                n.operation(),
                DiscoveryExecutionKindV1::LocalCommand,
                r,
            )
            .map_err(DiscoveredSourcePublicationErrorV1::Discovery)?;
            self.request_decoded_source_publication_command_v1(envelope, port, r)
                .map_err(DiscoveredSourcePublicationErrorV1::Request)
        })
    }
    pub fn release_discovered_source_publication_receipt_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        envelope: &SourcePublicationEnvelopeValueV1<'_, '_>,
        port: &SourcePublicationReceiptPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourcePublicationReceiptReplyV1, DiscoveredSourcePublicationErrorV1> {
        source_dispatch(r, |r| {
            let n = normalize_source_publication_receipt_v1(envelope, r)
                .map_err(DiscoveredSourcePublicationErrorV1::Request)?;
            if !n.matches(port) {
                return Err(DiscoveredSourcePublicationErrorV1::Request(
                    SourcePublicationIngressErrorV1::InvalidRequest,
                ));
            }
            self.recognize_source_publication_route_v1(
                &route,
                SOURCE_PUBLICATION_RECEIPT_PROFILE_V1,
                "retained-outcome",
                DiscoveryExecutionKindV1::Query,
                r,
            )
            .map_err(DiscoveredSourcePublicationErrorV1::Discovery)?;
            self.release_decoded_source_publication_receipt_v1(envelope, port, r)
                .map_err(DiscoveredSourcePublicationErrorV1::Request)
        })
    }
}
fn source_dispatch<T>(
    r: &mut Recorder,
    work: impl FnOnce(&mut Recorder) -> Result<T, DiscoveredSourcePublicationErrorV1>,
) -> Result<T, DiscoveredSourcePublicationErrorV1> {
    let timer = r.begin_service_timing();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(r)));
    let disposition = match &caught {
        Ok(Ok(_)) => Disposition::Completed,
        Ok(Err(
            DiscoveredSourcePublicationErrorV1::Discovery(Error::Faulted)
            | DiscoveredSourcePublicationErrorV1::Request(SourcePublicationIngressErrorV1::Native(
                SourcePublicationErrorV1::Faulted,
            )),
        ))
        | Err(_) => Disposition::Unknown,
        _ => Disposition::Refused,
    };
    r.record(Observation {
        boundary: Boundary::DiscoverySourcePublicationDispatch,
        disposition,
        service_time: timer.finish(),
    });
    match caught {
        Ok(result) => result,
        Err(cause) => std::panic::resume_unwind(cause),
    }
}
