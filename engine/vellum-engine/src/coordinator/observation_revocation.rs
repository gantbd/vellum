//! Exact-context authorization withdrawal, not grant-wide policy mutation.
use super::*;
mod v2;
pub use v2::*;

/// Reuses existing opaque target roles without widening any executable enum.
pub type ObservationRevocationTargetV1<'a> = ObservationStopTargetV3<'a>;

/// Independently attributable host authorization, not recipient-supplied data.
/// Target possession and ordinary stop permission do not imply this permission.
pub struct TrustedObservationRevocationV1<'a> {
    pub original_target: ObservationRevocationTargetV1<'a>,
    pub revocation_allowed: Option<bool>,
}
impl std::fmt::Debug for TrustedObservationRevocationV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<observation-revocation-authority:redacted>")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationRevocationErrorV1 {
    Unavailable,
    Native(ObservationStopErrorV3),
}
/// Inert data/opaque-role shell only; sharing it does not widen stop's grammar.
pub type ObservationRevocationEnvelopeValueV1<'a, 'h> = ObservationStopEnvelopeValueV3<'a, 'h>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationRevocationIngressErrorV1 {
    InvalidRequest,
    Execution(ObservationRevocationErrorV1),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NormalizedObservationRevocationV1<'h> {
    target: ObservationRevocationTargetV1<'h>,
}
impl<'h> NormalizedObservationRevocationV1<'h> {
    pub fn target(self) -> ObservationRevocationTargetV1<'h> {
        self.target
    }
}
impl std::fmt::Debug for NormalizedObservationRevocationV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<observation-revocation-arguments:redacted>")
    }
}
/// Exactly the existing F19 2/4/3-field native envelope, with a separately closed
/// revocation operation. No callbacks, identity parsing, allocation or transport.
pub fn normalize_observation_revocation_v1<'h>(
    envelope: &ObservationRevocationEnvelopeValueV1<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedObservationRevocationV1<'h>, ObservationRevocationIngressErrorV1> {
    let timing = r.begin_service_timing();
    let result =
        observation_control::normalize_context_operation(envelope, "revoke-observation-grant")
            .map(|n| NormalizedObservationRevocationV1 { target: n.target() })
            .map_err(|_| ObservationRevocationIngressErrorV1::InvalidRequest);
    r.record(Observation {
        boundary: Boundary::ObservationRevocationArgumentNormalization,
        disposition: if result.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        service_time: timing.finish(),
    });
    result
}
impl SourceCoordinator {
    /// Decode inert arguments first, then validate original target and separately
    /// attributable authority through the same native operation as typed callers.
    pub fn revoke_decoded_observation_context_v1(
        &mut self,
        envelope: &ObservationRevocationEnvelopeValueV1<'_, '_>,
        port: ObservationStopPortV3<'_>,
        authority: &TrustedObservationRevocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), ObservationRevocationIngressErrorV1> {
        match normalize_observation_revocation_v1(envelope, r) {
            Ok(n) => self
                .revoke_observation_context_v1(n.target(), port, authority, r)
                .map_err(ObservationRevocationIngressErrorV1::Execution),
            Err(e) => {
                // No native work ran: do not fabricate a native service sample.
                r.record(Observation {
                    boundary: Boundary::ObservationContextRevocation,
                    disposition: Disposition::Refused,
                    service_time: None,
                });
                Err(e)
            }
        }
    }
    /// Withdraws authorization of exactly one original context under exclusive
    /// owner access. Native absorbing closure denies later mediated delivery.
    /// Siblings/new admissions retain their independent authorization. No
    /// producer cancellation, evidence deletion, implicit retirement or refund.
    pub fn revoke_observation_context_v1(
        &mut self,
        target: ObservationRevocationTargetV1<'_>,
        port: ObservationStopPortV3<'_>,
        authority: &TrustedObservationRevocationV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), ObservationRevocationErrorV1> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Validate every original role before inspecting or closing a native
            // context. Never synthesize native owner-control facts from this bit.
            if authority.revocation_allowed != Some(true)
                || authority.original_target != target
                || !port.matches(target)
            {
                return Err(ObservationRevocationErrorV1::Unavailable);
            }
            self.stop_observation_facade_v3(port, r)
                .map_err(ObservationRevocationErrorV1::Native)
        }));
        let result = match caught {
            Ok(result) => result,
            Err(p) => {
                self.faulted = true;
                r.record(Observation {
                    boundary: Boundary::ObservationContextRevocation,
                    disposition: Disposition::Unknown,
                    service_time: timing.finish(),
                });
                std::panic::resume_unwind(p)
            }
        };
        let disposition = match result {
            Ok(()) => Disposition::Completed,
            Err(ObservationRevocationErrorV1::Native(e)) if e.faulted() => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        };
        r.record(Observation {
            boundary: Boundary::ObservationContextRevocation,
            disposition,
            service_time: timing.finish(),
        });
        result
    }
}

#[cfg(test)]
mod tests;
