//! Installed F20 admission/recovery; actual readers and activation stay separate.
use super::*;
use crate::resource::{OrdinaryPairEvidence, ResourceError};
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredReadOperationErrorV1 {
    Discovery(DiscoveryErrorV1),
    Native(ResourceError),
}
/// Original native products, not a fabricated common task or activation result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiscoveredReadOperationAdmissionV1 {
    Source(SourceReadOperationAdmissionV1),
    Annotations(AnnotationReadOperationAdmissionV1),
}
/// Closed native SDK roles. Catalog provenance remains in the original target;
/// metadata visibility does not grant native admission or observation authority.
pub enum NativeDiscoveredReadCommandV1<'a> {
    Source {
        request: &'a OwnedSourceReadRequestV1<'a>,
        facts: &'a TrustedSourceReadOperationFactsV1<'a>,
    },
    Annotations {
        request: &'a OwnedAnnotationReadRequestV1<'a>,
        facts: &'a TrustedAnnotationReadOperationFactsV1<'a>,
        pair: Option<OrdinaryPairEvidence>,
    },
}
impl NativeDiscoveredReadCommandV1<'_> {
    fn operation(&self) -> &'static str {
        match self {
            Self::Source { .. } => "source-load-candidate",
            Self::Annotations { .. } => "annotation-load-candidate",
        }
    }
}
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<[(&str, D<'_>); 2]>(),
        size_of::<[(&str, D<'_>); 3]>(),
        size_of::<[[(&str, D<'_>); 3]; 2]>(),
        size_of::<[DiscoveryVariantInputV1<'_>; 2]>(),
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
    visibility: &NativeBuiltinEntryVisibilityV1<2>,
    consume: impl FnOnce(DiscoveryCutInputV1<'_>) -> T,
) -> T {
    let source = [
        (
            "request",
            D::Text(
                "OriginalOwnedSourceReadRequestV1: origin/target/strength; original catalog capture retained",
            ),
        ),
        (
            "facts",
            D::Text("IndependentCurrentTrustedSourceReadOperationFactsV1"),
        ),
    ];
    let annotations = [
        (
            "request",
            D::Text(
                "OriginalOwnedAnnotationReadRequestV1: origin/source/set/bindings/target/strength; original catalog capture retained",
            ),
        ),
        (
            "facts",
            D::Text("IndependentCurrentTrustedAnnotationReadOperationFactsV1"),
        ),
        ("pair", D::Text("OptionalOriginalOrdinaryPairEvidence")),
    ];
    let inputs = [
        [
            ("contract", D::Text("engine-read-admission-declaration/1")),
            ("variant", D::Text("source-load-candidate")),
            ("fields", D::Object(&source)),
        ],
        [
            ("contract", D::Text("engine-read-admission-declaration/1")),
            ("variant", D::Text("annotation-load-candidate")),
            ("fields", D::Object(&annotations)),
        ],
    ];
    let variants: [_; 2] = std::array::from_fn(|i| DiscoveryVariantInputV1 {
        id: if i == 0 {
            "source-load-candidate"
        } else {
            "annotation-load-candidate"
        },
        input: D::Object(&inputs[i]),
        outcomes: D::Text(
            "Original reader parent/child admission or authorized matching recovery; native operation observation independently released",
        ),
        requirements: D::Text(
            "Independent current native facts and actual reader/target/catalog guards; fresh admission only checks applicable current document/binding/pair",
        ),
        limits: D::Text(
            "Existing reader and separately funded lifetime parent limits; no new task or payload store",
        ),
        cancellation: D::Text(
            "Observation stop is not read cancellation, quiescence or candidate activation",
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
        contract: "engine-read-admission-declaration/1",
        version: 1,
        kind: DiscoveryExecutionKindV1::EffectRequest,
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
    pub fn prepare_native_read_admission_discovery_cut_v1(
        &mut self,
        scope: &DiscoveryScopeV1,
        visibility: &NativeBuiltinEntryVisibilityV1<2>,
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
            if store.limits.max_entries < 1 || store.limits.max_variants < 2 {
                return Err(Error::Limit);
            }
            let authority = text(&store.authority)?;
            let command = command(
                &store.engine,
                "F20-read-admission",
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
                    Origin::NativeReadAdmissionV1,
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
    pub fn request_discovered_read_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        command: NativeDiscoveredReadCommandV1<'_>,
        r: &mut Recorder,
    ) -> Result<DiscoveredReadOperationAdmissionV1, DiscoveredReadOperationErrorV1> {
        let recognized = self.discovery_context(route.context).and_then(|c| {
            queries::read_admission_selector_matches(
                c,
                route.entry,
                route.variant,
                command.operation(),
            )
        });
        observe(r, Boundary::DiscoveryRequestRecognition, &recognized);
        recognized.map_err(DiscoveredReadOperationErrorV1::Discovery)?;
        self.query_discovery_v1(
            route.context,
            route.scope,
            route.cut,
            DiscoveryQueryV1::Inspect { entry: route.entry },
            route.facts,
            r,
        )
        .map_err(DiscoveredReadOperationErrorV1::Discovery)?;
        match command {
            NativeDiscoveredReadCommandV1::Source { request, facts } => self
                .request_owned_source_read_v1(request, facts, r)
                .map(DiscoveredReadOperationAdmissionV1::Source),
            NativeDiscoveredReadCommandV1::Annotations {
                request,
                facts,
                pair,
            } => self
                .request_owned_annotation_read_v1(request, facts, pair, r)
                .map(DiscoveredReadOperationAdmissionV1::Annotations),
        }
        .map_err(DiscoveredReadOperationErrorV1::Native)
    }
}
