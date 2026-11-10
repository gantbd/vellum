//! Installed F14 typed native requests use the original reversal owner.
use super::*;
/// Borrowed original native roles, not a wire decoder or recipient grant.
#[derive(Clone, Copy)]
pub struct NativeDiscoveredReversalRequestV1<'a> {
    pub invocation: &'a InvocationId,
    pub target: &'a HistoryReversalTargetV1,
    pub direction: HistoryDirection,
    pub reads: &'a ReadDependencyRequestV3,
    pub result: &'a TrustedReversalResultAdmissionV1<'a>,
}
impl SourceCoordinator {
    /// Trusted-native embedding only, as with the underlying reversal API.
    /// Discovery/advice and result-disclosure grants are not mutation authority;
    /// the host independently authorizes its caller. Current native frontier,
    /// participants, reads, publication and replay remain with the history owner.
    pub fn submit_discovered_native_reversal_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        request: NativeDiscoveredReversalRequestV1<'_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let recognized = self.discovery_context(route.context).and_then(|c| {
            queries::reversal_selector_matches(c, route.entry, route.variant, request.direction)
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
        self.submit_reversal_target_with_reads_and_result_v3(
            request.invocation,
            request.target,
            request.direction,
            request.reads,
            request.result,
            r,
        )
        .map_err(DiscoveredNativeCommandErrorV1::Invocation)
    }
}
