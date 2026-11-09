//! F21 installed admission/recovery over the existing one-attempt shared writer.
use super::*;
use crate::resource::{EncodedAnnotations, OrdinaryPairEvidence, ResourceError};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscoveredSaveOperationErrorV1 {
    Discovery(DiscoveryErrorV1),
    Native(ResourceError),
}
/// Trusted native SDK roles, not a wire grammar, executor or execution grant.
/// A catalog-bound request retains its original binding on matching lookup.
// This bounded native stack envelope consumes existing issuer inputs. Boxing
// merely to equalize variants would add an allocation to every annotation call.
#[allow(clippy::large_enum_variant)]
pub enum NativeDiscoveredOrdinarySaveCommandV1<'a> {
    Source {
        request: &'a OwnedSourceSaveRequestV1<'a>,
        writer: Option<&'a CatalogWriterBindingV1>,
        release: &'a TrustedSaveOperationReleaseV1<'a>,
        admission: Option<&'a TrustedOrdinarySourceSaveAdmissionV1<'a>>,
        pair: Option<&'a OrdinaryPairEvidence>,
    },
    Annotations {
        request: &'a OwnedAnnotationSaveRequestV1<'a>,
        writer: Option<&'a CatalogWriterBindingV1>,
        release: &'a TrustedAnnotationSaveOperationReleaseV1<'a>,
        admission: Option<&'a TrustedOrdinaryAnnotationSaveAdmissionV1>,
        candidate: Option<EncodedAnnotations>,
        pair: Option<OrdinaryPairEvidence>,
    },
}
impl NativeDiscoveredOrdinarySaveCommandV1<'_> {
    fn operation(&self) -> &'static str {
        match self {
            Self::Source { .. } => "captured-source-save",
            Self::Annotations { .. } => "captured-annotation-save",
        }
    }
}
pub(super) fn workspace_bytes(maximum: usize) -> Result<usize, Error> {
    [
        mul(2, maximum)?,
        size_of::<[String; 2]>(),
        size_of::<[(&str, D<'_>); 5]>(),
        size_of::<[(&str, D<'_>); 6]>(),
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
                "OriginalOwnedSourceSaveRequestV1: origin/source/serialization/binding/target/requirements/condition",
            ),
        ),
        ("writer", D::Text("OptionalOriginalCatalogWriterBindingV1")),
        (
            "release",
            D::Text("IndependentCurrentTrustedSaveOperationReleaseV1"),
        ),
        (
            "admission",
            D::Text("OptionalFreshTrustedOrdinarySourceSaveAdmissionV1"),
        ),
        ("pair", D::Text("OptionalOriginalOrdinaryPairEvidence")),
    ];
    let annotations = [
        (
            "request",
            D::Text(
                "OriginalOwnedAnnotationSaveRequestV1: origin/source/set/bindings/encoding/destination/requirements/condition",
            ),
        ),
        ("writer", D::Text("OptionalOriginalCatalogWriterBindingV1")),
        (
            "release",
            D::Text("IndependentCurrentTrustedAnnotationSaveOperationReleaseV1"),
        ),
        (
            "admission",
            D::Text("OptionalFreshTrustedOrdinaryAnnotationSaveAdmissionV1"),
        ),
        ("pair", D::Text("OptionalOriginalOrdinaryPairEvidence")),
        (
            "candidate",
            D::Text("OptionalOriginalEncodedAnnotations: first admission only"),
        ),
    ];
    let inputs = [
        [
            ("contract", D::Text("engine-ordinary-save-declaration/1")),
            ("variant", D::Text("captured-source-save")),
            ("fields", D::Object(&source)),
        ],
        [
            ("contract", D::Text("engine-ordinary-save-declaration/1")),
            ("variant", D::Text("captured-annotation-save")),
            ("fields", D::Object(&annotations)),
        ],
    ];
    let variants: [_; 2] = std::array::from_fn(|i| DiscoveryVariantInputV1 {
        id: if i == 0 {
            "captured-source-save"
        } else {
            "captured-annotation-save"
        },
        input: D::Object(&inputs[i]),
        outcomes: D::Text(
            "Original one-attempt writer parent and actual child; matching recovery never dispatches or recaptures",
        ),
        requirements: D::Text(
            "Independent current native release, first-admission permissions and actual codec/binding/writer guards; metadata grants nothing",
        ),
        limits: D::Text(
            "Existing bounded parent/task/artifact/claim envelopes; no second task store",
        ),
        cancellation: D::Text(
            "Native producer stop is separate from observation stop, external effect disposition and baseline adoption",
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
        contract: "engine-ordinary-save-declaration/1",
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
    pub fn prepare_native_ordinary_save_discovery_cut_v1(
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
                "F21-ordinary-save",
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
                    Origin::NativeOrdinarySaveV1,
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
    pub fn request_discovered_ordinary_save_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        command: NativeDiscoveredOrdinarySaveCommandV1<'_>,
        r: &mut Recorder,
    ) -> Result<SaveOperationAdmissionV1, DiscoveredSaveOperationErrorV1> {
        let recognized = self.discovery_context(route.context).and_then(|c| {
            queries::ordinary_save_selector_matches(
                c,
                route.entry,
                route.variant,
                command.operation(),
            )
        });
        observe(r, Boundary::DiscoveryRequestRecognition, &recognized);
        recognized.map_err(DiscoveredSaveOperationErrorV1::Discovery)?;
        self.query_discovery_v1(
            route.context,
            route.scope,
            route.cut,
            DiscoveryQueryV1::Inspect { entry: route.entry },
            route.facts,
            r,
        )
        .map_err(DiscoveredSaveOperationErrorV1::Discovery)?;
        match command {
            NativeDiscoveredOrdinarySaveCommandV1::Source {
                request,
                writer,
                release,
                admission,
                pair,
            } => self.request_owned_source_save_catalog(
                (request, writer),
                release,
                admission,
                pair,
                r,
            ),
            NativeDiscoveredOrdinarySaveCommandV1::Annotations {
                request,
                writer,
                release,
                admission,
                candidate,
                pair,
            } => self.request_owned_annotation_save_catalog(
                (request, writer),
                release,
                admission,
                candidate,
                pair,
                r,
            ),
        }
        .map_err(DiscoveredSaveOperationErrorV1::Native)
    }
}
