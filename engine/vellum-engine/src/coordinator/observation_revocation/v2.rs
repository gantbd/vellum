//! Exact-context authorization withdrawal, not grant-wide policy mutation.
use super::*;

/// Reuses existing opaque target roles without widening any executable enum.
pub type ObservationRevocationTargetV2<'a> = ObservationStopTargetV4<'a>;

/// Independently attributable host authorization, not recipient-supplied data.
/// Target possession and ordinary stop permission do not imply this permission.
pub struct TrustedObservationRevocationV2<'a> {
    pub original_target: ObservationRevocationTargetV2<'a>,
    pub revocation_allowed: Option<bool>,
}
impl std::fmt::Debug for TrustedObservationRevocationV2<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<observation-revocation-authority:redacted>")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationRevocationErrorV2 {
    Unavailable,
    Native(ObservationStopErrorV4),
}
/// Inert data/opaque-role shell only; sharing it does not widen stop's grammar.
pub type ObservationRevocationEnvelopeValueV2<'a, 'h> = ObservationStopEnvelopeValueV4<'a, 'h>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationRevocationIngressErrorV2 {
    InvalidRequest,
    Execution(ObservationRevocationErrorV2),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct NormalizedObservationRevocationV2<'h> {
    target: ObservationRevocationTargetV2<'h>,
}
impl<'h> NormalizedObservationRevocationV2<'h> {
    pub fn target(self) -> ObservationRevocationTargetV2<'h> {
        self.target
    }
}
impl std::fmt::Debug for NormalizedObservationRevocationV2<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<observation-revocation-arguments:redacted>")
    }
}
/// Exactly the existing F19 2/4/3-field native envelope, with a separately closed
/// revocation operation. No callbacks, identity parsing, allocation or transport.
pub fn normalize_observation_revocation_v2<'h>(
    envelope: &ObservationRevocationEnvelopeValueV2<'_, 'h>,
    r: &mut Recorder,
) -> Result<NormalizedObservationRevocationV2<'h>, ObservationRevocationIngressErrorV2> {
    let timing = r.begin_service_timing();
    let result =
        observation_control::normalize_context_operation_v4(envelope, "revoke-observation-grant")
            .map(|n| NormalizedObservationRevocationV2 { target: n.target() })
            .map_err(|_| ObservationRevocationIngressErrorV2::InvalidRequest);
    r.record(Observation {
        boundary: Boundary::ObservationRevocationArgumentNormalizationV2,
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
    pub fn revoke_decoded_observation_context_v2(
        &mut self,
        envelope: &ObservationRevocationEnvelopeValueV2<'_, '_>,
        port: ObservationStopPortV4<'_>,
        authority: &TrustedObservationRevocationV2<'_>,
        r: &mut Recorder,
    ) -> Result<(), ObservationRevocationIngressErrorV2> {
        match normalize_observation_revocation_v2(envelope, r) {
            Ok(n) => self
                .revoke_observation_context_v2(n.target(), port, authority, r)
                .map_err(ObservationRevocationIngressErrorV2::Execution),
            Err(e) => {
                // No native work ran: do not fabricate a native service sample.
                r.record(Observation {
                    boundary: Boundary::ObservationContextRevocationV2,
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
    pub fn revoke_observation_context_v2(
        &mut self,
        target: ObservationRevocationTargetV2<'_>,
        port: ObservationStopPortV4<'_>,
        authority: &TrustedObservationRevocationV2<'_>,
        r: &mut Recorder,
    ) -> Result<(), ObservationRevocationErrorV2> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Validate every original role before inspecting or closing a native
            // context. Never synthesize native owner-control facts from this bit.
            if authority.revocation_allowed != Some(true)
                || authority.original_target != target
                || !port.matches(target)
            {
                return Err(ObservationRevocationErrorV2::Unavailable);
            }
            self.stop_observation_facade_v4(port, r)
                .map_err(ObservationRevocationErrorV2::Native)
        }));
        let result = match caught {
            Ok(result) => result,
            Err(p) => {
                self.faulted = true;
                r.record(Observation {
                    boundary: Boundary::ObservationContextRevocationV2,
                    disposition: Disposition::Unknown,
                    service_time: timing.finish(),
                });
                std::panic::resume_unwind(p)
            }
        };
        let disposition = match result {
            Ok(()) => Disposition::Completed,
            Err(ObservationRevocationErrorV2::Native(e)) if e.faulted() => Disposition::Unknown,
            Err(_) => Disposition::Refused,
        };
        r.record(Observation {
            boundary: Boundary::ObservationContextRevocationV2,
            disposition,
            service_time: timing.finish(),
        });
        result
    }
}
