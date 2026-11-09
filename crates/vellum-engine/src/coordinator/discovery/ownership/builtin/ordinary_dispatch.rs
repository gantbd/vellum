//! Installed ordinary-edit dispatch for trusted native embeddings only.
//! Neither discovery grants nor logical view contexts authenticate a recipient.
use super::*;

/// Identical opaque route roles; no new issuer, reservation or execution mode.
pub type NativeDiscoveredCommandRouteV1<'a> = NativeDiscoveredLifecycleRouteV1<'a>;
pub type DiscoveredNativeCommandErrorV1 = DiscoveredLifecycleErrorV1;

/// The four existing, owning-guard-normalized forms of ordinary-edit. Exhaustive
/// within V1. Adding a form requires an explicit new executable profile.
#[derive(Clone, Copy)]
pub enum NativeDiscoveredOrdinaryEditV1<'a, 'h> {
    Literal(&'a NormalizedLiteralEditArgumentsV1<'h>),
    Line(&'a NormalizedLineEditArgumentsV1<'h>),
    Boundary(&'a NormalizedBoundaryEditArgumentsV1<'h>),
    ConfiguredLine(&'a NormalizedConfiguredLineArgumentsV1<'h>),
}
impl SourceCoordinator {
    /// Revalidate the privately installed F11 route, then use the original
    /// trusted-owner admission. This is NOT a recipient authorization boundary:
    /// a trusted embedding must independently authorize its caller, exactly as
    /// for the underlying native APIs. Metadata/advice never supplies that grant.
    /// Replay, current view/source guards, preparation and execution are unchanged.
    pub fn submit_discovered_native_ordinary_edit_v1(
        &mut self,
        route: NativeDiscoveredCommandRouteV1<'_>,
        command: NativeDiscoveredOrdinaryEditV1<'_, '_>,
        r: &mut Recorder,
    ) -> Result<InvocationState, DiscoveredNativeCommandErrorV1> {
        let recognized = self
            .discovery_context(route.context)
            .and_then(|c| queries::ordinary_selector_matches(c, route.entry, route.variant));
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
        match command {
            NativeDiscoveredOrdinaryEditV1::Literal(q) => {
                self.submit_literal_edit_arguments_v1(q, r)
            }
            NativeDiscoveredOrdinaryEditV1::Line(q) => self.submit_line_edit_arguments_v1(q, r),
            NativeDiscoveredOrdinaryEditV1::Boundary(q) => {
                self.submit_boundary_edit_arguments_v1(q, r)
            }
            NativeDiscoveredOrdinaryEditV1::ConfiguredLine(q) => {
                self.submit_configured_line_arguments_v1(q, r)
            }
        }
        .map_err(DiscoveredNativeCommandErrorV1::Invocation)
    }
}
