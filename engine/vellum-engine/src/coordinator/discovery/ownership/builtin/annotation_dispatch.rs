//! F13 installed routing composes existing admission and result rights unchanged.
use super::*;
/// Independent trusted premises, not rights inferred from discovery metadata.
pub struct NativeDiscoveredAnnotationAdmissionV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub facts: &'a TrustedAnnotationInvocationV1<'a>,
    /// None uses the existing no-result admission; it cannot gain a result later.
    pub result: Option<&'a TrustedAnnotationResultAdmissionV1<'a>>,
}
impl SourceCoordinator {
    /// Recognize the original installed F13 variant before any observed loss,
    /// revalidate current discovery, then delegate original authorization,
    /// funding, result ceiling and replay to the annotation owner.
    pub fn submit_discovered_annotation_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        arguments: &NormalizedAnnotationAuthoringArgumentsV1<'_>,
        admission: NativeDiscoveredAnnotationAdmissionV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let recognized = self.discovery_context(route.context).and_then(|c| {
            queries::annotation_selector_matches(
                c,
                route.entry,
                route.variant,
                arguments.argument_declaration_v1().variant.operation,
            )
        });
        observe(r, Boundary::DiscoveryRequestRecognition, &recognized);
        recognized.map_err(DiscoveredNativeCommandErrorV1::Discovery)?;
        self.query_discovery_v1(
            route.context,
            route.scope,
            route.cut,
            DiscoveryQueryV1::Inspect { entry: route.entry },
            route.facts,
            r,
        )
        .map_err(DiscoveredNativeCommandErrorV1::Discovery)?;
        match admission.result {
            Some(result) => self.submit_scoped_annotation_with_result_v1(
                arguments,
                admission.context,
                admission.facts,
                result,
                r,
            ),
            None => {
                self.submit_scoped_annotation_v1(arguments, admission.context, admission.facts, r)
            }
        }
        .map_err(DiscoveredNativeCommandErrorV1::Invocation)
    }
}
