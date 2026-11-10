//! Installed native F17 routes; descriptive fields never resolve opaque roles.
use super::context::entry;
use super::*;
#[cfg(test)]
mod tests;

pub const NATIVE_LIFECYCLE_DISCOVERY_CONTRACT_V1: &str = "engine-document-lifecycle-declaration/1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeBuiltinDiscoveryVisibilityV4 {
    pub previous: NativeBuiltinDiscoveryVisibilityV3,
    pub lifecycle: NativeBuiltinEntryVisibilityV1<4>,
}
pub type NativeBuiltinDiscoveryRowV4 = NativeBuiltinDiscoveryRowV2;

pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    // Simultaneous /3 frame is charged separately. No caller-sized source graph.
    [
        mul(10, maximum)?,
        size_of::<[String; 5]>(),
        size_of::<[NativeBuiltinDiscoveryRowV4; 5]>(),
        size_of::<[[(&str, D<'_>); 10]; 5]>(),
        size_of::<[[(&str, D<'_>); 3]; 5]>(),
        size_of::<[D<'_>; 5]>(),
        size_of::<[[(&str, D<'_>); 5]; 4]>(),
        size_of::<[[(&str, D<'_>); 3]; 4]>(),
        size_of::<[std::ops::Range<usize>; 4]>(),
        size_of::<[&str; 4]>(),
        size_of::<[DiscoveryVariantInputV1<'_>; 4]>(),
        size_of::<[DiscoveryEntryInputV1<'_>; 5]>(),
        size_of::<NativeBuiltinDiscoveryVisibilityV4>(),
        size_of::<TrustedDiscoveryFactsV1>(),
        size_of::<DiscoveryCutInputV1<'_>>(),
    ]
    .into_iter()
    .try_fold(0, add)
}
fn capacity(l: DiscoveryLimitsV1) -> Result<(), Error> {
    catalog_capacity(l)?;
    if l.max_entries < 5 {
        return Err(Error::Limit);
    }
    Ok(())
}
impl SourceCoordinator {
    pub fn native_builtin_discovery_rows_v4(
        &mut self,
        scope: &DiscoveryScopeV1,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<[NativeBuiltinDiscoveryRowV4; 5], Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V4 {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, f, r)?;
            if f.owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            let s = self.discovery_store()?;
            capacity(s.limits)?;
            let identity = |family, contract| {
                Ok(NativeBuiltinDiscoveryRowV4 {
                    authority: text(&s.authority)?,
                    command: command(&s.engine, family, s.limits.max_context_bytes)?,
                    incarnation: 1,
                    family,
                    contract,
                })
            };
            let [a, h] = declarations();
            Ok([
                identity(a.family, a.contract)?,
                identity(h.family, h.contract)?,
                identity("F16", "engine-context-declaration/1")?,
                identity(
                    "F11",
                    crate::coordinator::handler_declaration::ORDINARY_EDIT_DESCRIPTOR_CONTRACT_V1,
                )?,
                identity("F17", NATIVE_LIFECYCLE_DISCOVERY_CONTRACT_V1)?,
            ])
        })();
        self.discovery_finish_timed(
            r,
            Boundary::DiscoveryBuiltinInspection,
            &result,
            timing.finish(),
        );
        result
    }
    /// /4 preserves /1–/3's closed inventories and shares their actual owner,
    /// issuance, retention and current-release rules. Only source truth is supplied.
    pub fn prepare_native_builtin_discovery_cut_v4(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinDiscoveryVisibilityV4,
        target: Option<DiscoveryTargetInputV1<'_>>,
        advice_present: bool,
        f: &TrustedDiscoveryFactsV1,
        r: &mut Recorder,
    ) -> Result<DiscoveryCutIdV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if scope.context.profile != OWNED_CONTRACT_DISCOVERY_PROFILE_V4 {
                return Err(Error::Unavailable);
            }
            self.discovery_current(scope, f, r)?;
            let s = self.discovery_store()?;
            capacity(s.limits)?;
            let commands = [
                command(&s.engine, "F13", s.limits.max_context_bytes)?,
                command(&s.engine, "F14", s.limits.max_context_bytes)?,
                command(&s.engine, "F16", s.limits.max_context_bytes)?,
                command(&s.engine, "F11", s.limits.max_context_bytes)?,
            ];
            let lifecycle = command(&s.engine, "F17", s.limits.max_context_bytes)?;
            let authority = text(&s.authority)?;
            let facts = TrustedDiscoveryFactsV1 {
                accepted_registry_attested: Some(true),
                schema_descriptions_attested: Some(true),
                ..*f
            };
            editing::with_inputs_v3(
                &authority,
                &commands,
                &visibility.previous,
                target,
                advice_present,
                |prior| {
                    with_lifecycle_inputs(&authority, &lifecycle, visibility, prior, |input| {
                        self.prepare_discovery_cut_from_source(
                            scope,
                            input,
                            &facts,
                            Origin::BuiltinV4,
                            r,
                        )
                    })
                },
            )??
        })();
        self.discovery_finish_timed(
            r,
            Boundary::DiscoveryBuiltinPreparation,
            &result,
            timing.finish(),
        );
        result
    }
}
fn with_lifecycle_inputs<T>(
    authority: &str,
    command: &str,
    visibility: &NativeBuiltinDiscoveryVisibilityV4,
    previous: DiscoveryCutInputV1<'_>,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> Result<T, Error> {
    let forms = NormalizedDocumentLifecycleArgumentsV1::REQUEST_DECLARATIONS;
    let header = NormalizedDocumentLifecycleArgumentsV1::ARGUMENT_DECLARATION;
    if header.variant.fields.len() != 5 || forms.iter().any(|f| f.variant.fields.len() > 10) {
        return Err(Error::Faulted);
    }
    let fields: [[(&str, D<'_>); 10]; 5] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            forms[i]
                .variant
                .fields
                .get(j)
                .map_or(("", D::Null), |(k, v)| (*k, D::Text(v)))
        })
    });
    let descriptions: [[(&str, D<'_>); 3]; 5] = std::array::from_fn(|i| {
        [
            ("contract", D::Text(forms[i].contract)),
            ("operation", D::Text(forms[i].variant.operation)),
            (
                "fields",
                D::Object(&fields[i][..forms[i].variant.fields.len()]),
            ),
        ]
    });
    let values: [D<'_>; 5] = std::array::from_fn(|i| D::Object(&descriptions[i]));
    let ranges = [0..1, 1..3, 3..4, 4..5];
    let outer: [[(&str, D<'_>); 5]; 4] = std::array::from_fn(|variant| {
        std::array::from_fn(|i| {
            let (k, v) = header.variant.fields[i];
            (
                k,
                match k {
                    "request" => D::Array(&values[ranges[variant].clone()]),
                    "contract" => D::Text(header.contract),
                    "operation" => D::Text(header.variant.operation),
                    "profile" => D::Text(NATIVE_DOCUMENT_LIFECYCLE_PROFILE_V1),
                    _ => D::Text(v),
                },
            )
        })
    });
    let names = [
        "create-scratch",
        "attach-detach-view",
        "membership",
        "dispose",
    ];
    let input: [[(&str, D<'_>); 3]; 4] = std::array::from_fn(|i| {
        [
            ("contract", D::Text(NATIVE_LIFECYCLE_DISCOVERY_CONTRACT_V1)),
            ("variant", D::Text(names[i])),
            ("fields", D::Object(&outer[i])),
        ]
    });
    let variants: [DiscoveryVariantInputV1<'_>; 4] = std::array::from_fn(|i| {
        DiscoveryVariantInputV1 {
            id: names[i],
            input: D::Object(&input[i]),
            outcomes: D::Text(
                "Original invocation; independent authorized retained outcome; native opaque roles only",
            ),
            requirements: D::Text(
                "Owning native admission and fresh service authorization; metadata grants nothing",
            ),
            limits: D::Text(
                "Caller-funded bounded normalization; original parent-funded intent and result",
            ),
            cancellation: D::Text(
                "Cooperative original invocation control; stop is not quiescence",
            ),
            order: visibility.lifecycle.variants[i].order,
            enabled: visibility.lifecycle.variants[i].enabled,
        }
    });
    let rows = [
        entry(&previous.entries[0]),
        entry(&previous.entries[1]),
        entry(&previous.entries[2]),
        entry(&previous.entries[3]),
        DiscoveryEntryInputV1 {
            command: DiscoveryCommandV1 {
                authority,
                id: command,
                incarnation: 1,
            },
            contract: NATIVE_LIFECYCLE_DISCOVERY_CONTRACT_V1,
            version: 1,
            kind: DiscoveryExecutionKindV1::LocalCommand,
            variants: &variants,
            presentation: D::Object(&[]),
            order: visibility.lifecycle.order,
            schema_visible: visibility.lifecycle.schema,
            presentation_visible: visibility.lifecycle.presentation,
            advice_visible: visibility.lifecycle.advice,
        },
    ];
    Ok(consume(DiscoveryCutInputV1 {
        entries: &rows,
        target: previous.target,
        advice_present: previous.advice_present,
    }))
}

/// Original discovery roles, not strings, executable metadata or command rights.
pub struct NativeDiscoveredLifecycleRouteV1<'a> {
    pub context: &'a DiscoveryContextIdV1,
    pub scope: &'a DiscoveryScopeV1,
    pub cut: &'a DiscoveryCutIdV1,
    pub entry: &'a DiscoveryEntryIdV1,
    pub variant: &'a DiscoveryVariantIdV1,
    pub facts: &'a TrustedDiscoveryFactsV1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredLifecycleErrorV1 {
    Discovery(DiscoveryErrorV1),
    Invocation(InvocationError),
}
impl SourceCoordinator {
    /// Select only a privately installed F17 handler, revalidate current
    /// discovery, then delegate unchanged native authorization and replay.
    /// Discovery and command grants remain independent; advice is never read.
    pub fn submit_discovered_native_document_lifecycle_v1(
        &mut self,
        route: NativeDiscoveredLifecycleRouteV1<'_>,
        arguments: &NormalizedDocumentLifecycleArgumentsV1<'_>,
        admission: NativeDocumentLifecycleAdmissionV2<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredLifecycleErrorV1> {
        let recognition = self.discovery_context(route.context).and_then(|c| {
            queries::lifecycle_selector_matches(
                c,
                route.entry,
                route.variant,
                arguments.operation(),
            )
        });
        observe(r, Boundary::DiscoveryRequestRecognition, &recognition);
        recognition.map_err(DiscoveredLifecycleErrorV1::Discovery)?;
        self.query_discovery_v1(
            route.context,
            route.scope,
            route.cut,
            DiscoveryQueryV1::Inspect { entry: route.entry },
            route.facts,
            r,
        )
        .map_err(DiscoveredLifecycleErrorV1::Discovery)?;
        self.submit_native_document_lifecycle_arguments_v1(arguments, admission, r)
            .map_err(DiscoveredLifecycleErrorV1::Invocation)
    }
}
