//! First-party F19 stop composition. No lookup by trial calls, producer control,
//! new observer registry, payload release, permission grant or implicit retirement.
use super::*;
use crate::instrumentation::{Boundary, Disposition, Observation, Recorder};
#[cfg(test)]
mod tests;
mod v2;
mod v3;
pub(in crate::coordinator) mod v4;
#[cfg(test)]
pub(in crate::coordinator) use v2::tests::request;
pub use v2::*;
pub(in crate::coordinator) use v3::normalize_context_operation;
#[cfg(test)]
pub(in crate::coordinator) use v3::tests::request as request_v3;
pub use v3::*;
pub(in crate::coordinator) use v4::normalize_context_operation_v4;
pub use v4::*;

/// Original capabilities from the actual native owners. This closed profile
/// advertises only implemented owners; it is not the full F19 stream contract.
pub enum ObservationStopPortV1<'a> {
    Read(ReadObservationStopPortV1<'a>),
    Invocation {
        observer: &'a mut InvocationObservationContextV1,
        control: &'a InvocationObservationPolicyControlV1<'a>,
    },
    NativeOperation {
        observer: &'a NativeOperationObservationIdV1,
        control: &'a NativeOperationObservationControlV1<'a>,
    },
    Discovery {
        context: &'a DiscoveryContextIdV1,
        scope: &'a DiscoveryScopeV1,
        facts: &'a TrustedDiscoveryFactsV1,
    },
}
impl std::fmt::Debug for ObservationStopPortV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ObservationStopPortV1(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationStopErrorV1 {
    Read(SourceReadContextErrorV1),
    Invocation(InvocationError),
    NativeOperation(NativeOperationObservationErrorV1),
    Discovery(DiscoveryErrorV1),
}
impl ObservationStopErrorV1 {
    fn faulted(self) -> bool {
        matches!(
            self,
            Self::Read(SourceReadContextErrorV1::Faulted)
                | Self::Invocation(InvocationError::Faulted)
                | Self::NativeOperation(NativeOperationObservationErrorV1::Faulted)
                | Self::Discovery(DiscoveryErrorV1::Faulted)
        )
    }
}
impl SourceCoordinator {
    /// One direct native control call with exact original ownership/scope checks.
    /// Protected control can run under an editing fault and never heals it.
    /// Stop is not discharge: original observer/producer obligations remain.
    pub fn stop_observation_facade_v1(
        &mut self,
        port: ObservationStopPortV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), ObservationStopErrorV1> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match port {
            ObservationStopPortV1::Read(port) => self
                .stop_read_observation_facade_v1(port, r)
                .map_err(ObservationStopErrorV1::Read),
            ObservationStopPortV1::Invocation { observer, control } => self
                .close_invocation_observation_v1(observer, control, r)
                .map_err(ObservationStopErrorV1::Invocation),
            ObservationStopPortV1::NativeOperation { observer, control } => self
                .stop_native_operation_observation_v1(observer, control, r)
                .map_err(ObservationStopErrorV1::NativeOperation),
            ObservationStopPortV1::Discovery {
                context,
                scope,
                facts,
            } => self
                .stop_discovery_context_v1(context, scope, facts, r)
                .map_err(ObservationStopErrorV1::Discovery),
        }));
        let result = match caught {
            Ok(v) => v,
            Err(p) => {
                self.faulted = true;
                r.record(Observation {
                    boundary: Boundary::ObservationStopFacade,
                    disposition: Disposition::Unknown,
                    service_time: timing.finish(),
                });
                std::panic::resume_unwind(p)
            }
        };
        let disposition = match result {
            Ok(()) => Disposition::Completed,
            // Native owners decide containment. An observer-local fault must
            // not become a new engine-wide editing fault merely by dispatch.
            Err(e) if e.faulted() => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        };
        r.record(Observation {
            boundary: Boundary::ObservationStopFacade,
            disposition,
            service_time: timing.finish(),
        });
        result
    }
}
