//! Installed continuation of the original accepted shutdown plan, never new consent.
use super::*;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredShutdownDispositionErrorV1 {
    Discovery(DiscoveryErrorV1),
    Request(ShutdownDispositionIngressErrorV1),
}
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<[[(&str, D<'_>); 1]; 3]>(),
        size_of::<[[(&str, D<'_>); 3]; 2]>(),
        size_of::<[[(&str, D<'_>); 4]; 2]>(),
        size_of::<[(&str, D<'_>); 5]>(),
        size_of::<[D<'_>; 4]>(),
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
    let contract = [("const", D::Text(SHUTDOWN_DISPOSITION_PROFILE_V1))];
    let operation = [("const", D::Text("shutdown-disposition"))];
    let opaque = [("type", D::Text("object"))];
    let properties = [
        ("contract", D::Object(&contract)),
        ("operation", D::Object(&operation)),
        ("fence", D::Object(&opaque)),
        ("document", D::Object(&opaque)),
    ];
    let required = ["contract", "operation", "fence", "document"].map(D::Text);
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
        ("contract", D::Text(SHUTDOWN_DISPOSITION_PROFILE_V1)),
        ("operation", D::Text("shutdown-disposition")),
        ("fence", D::Text("OriginalOpaqueShutdownFenceId")),
        ("document", D::Text("OriginalOpaqueDocumentId")),
    ];
    let input = [
        ("contract", D::Text(SHUTDOWN_DISPOSITION_PROFILE_V1)),
        ("variant", D::Text("shutdown-disposition")),
        ("fields", D::Object(&fields)),
    ];
    let requirements = [
        ("schema", D::Object(&schema)),
        ("coverage", D::Text("outer-structure-only")),
        (
            "runtime",
            D::Text(
                "Original native fence and engine-retained accepted document consent/save bindings; independent current owner/disposition rights and complete native coverage; discovery grants no authority",
            ),
        ),
    ];
    let variants = [DiscoveryVariantInputV1 {
        id: "shutdown-disposition",
        input: D::Object(&input),
        outcomes: D::Text(
            "Original native LifecycleState on authorized disposition; refused/stale/unavailable; integrity fault remains unknown",
        ),
        requirements: D::Object(&requirements),
        limits: D::Text(
            "Fixed 2/4/4 borrowed input; bounded accepted plan lookup and existing native disposition bounds; prepaid discovery workspace",
        ),
        cancellation: D::Text(
            "Not new consent, save execution, settlement, quiescence, report retirement, final payload reclamation or process-exit safety; protected native progress stays separate",
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
        contract: SHUTDOWN_DISPOSITION_PROFILE_V1,
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
    pub fn prepare_native_shutdown_disposition_discovery_cut_v1(
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
                "F18-shutdown-disposition",
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
                    Origin::NativeShutdownDispositionV1,
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
    /// Separate native facts are not taken from discovery metadata or arguments.
    /// Direct protected control remains available if this optional discovery owner closes.
    pub fn request_discovered_shutdown_disposition_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        envelope: &ShutdownDispositionEnvelopeValueV1<'_, '_>,
        facts: &TrustedShutdownInventoryFactsV1<'_>,
        coverage: &TrustedDisposalCoverageV1,
        r: &mut Recorder,
    ) -> Result<LifecycleState, DiscoveredShutdownDispositionErrorV1> {
        type E = DiscoveredShutdownDispositionErrorV1;
        let timer = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let n = normalize_shutdown_disposition_v1(envelope, r).map_err(E::Request)?;
            let recognized = self.discovery_context(route.context).and_then(|c| {
                queries::shutdown_disposition_selector_matches(c, route.entry, route.variant)
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
            self.dispose_accepted_shutdown_document_v1(n.fence(), n.document(), facts, coverage, r)
                .map_err(|e| E::Request(ShutdownDispositionIngressErrorV1::Execution(e)))
        }));
        let result = match caught {
            Ok(result) => result,
            Err(p) => {
                r.record(Observation {
                    boundary: Boundary::DiscoveryShutdownDispositionDispatch,
                    disposition: Disposition::Unknown,
                    service_time: timer.finish(),
                });
                std::panic::resume_unwind(p)
            }
        };
        let disposition = match result {
            Ok(_) => Disposition::Completed,
            Err(E::Discovery(Error::Faulted))
            | Err(E::Request(ShutdownDispositionIngressErrorV1::Execution(
                CoordinatorError::Faulted
                | CoordinatorError::Resource(crate::resource::ResourceError::Faulted),
            ))) => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        };
        r.record(Observation {
            boundary: Boundary::DiscoveryShutdownDispositionDispatch,
            disposition,
            service_time: timer.finish(),
        });
        result
    }
}
