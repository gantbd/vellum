//! Authority-local shutdown control in the existing non-cloneable engine owner.
//! The funded admission fence is a prerequisite, not accepted disposition or a
//! claim that effects, observer attachments or retained owners have drained.
use super::*;
use crate::settings::SettingsError;
use std::mem::size_of;
pub(in crate::coordinator) mod inventory;
#[cfg(test)]
pub(super) mod tests;
pub use inventory::*;
mod decisions;
pub use decisions::*;
mod disposition;
pub use disposition::ShutdownDocumentDispositionRequestV1;
mod wait;
pub use wait::*;
mod acceptance;
pub use acceptance::*;
mod cancellation;
pub use cancellation::*;
mod attempt;
pub use attempt::ShutdownAttemptStatusV1;
mod ingress;
pub use ingress::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShutdownFenceLimitsV1 {
    pub max_context_bytes: usize,
    pub max_root_bytes: usize,
    pub max_capture_bytes: usize,
}
impl ShutdownFenceLimitsV1 {
    pub fn required_capture_bytes(self) -> Result<usize, CoordinatorError> {
        size_of::<Fence>()
            .checked_add(size_of::<Basis>())
            .and_then(|n| n.checked_add(self.max_context_bytes))
            .and_then(|n| n.checked_add(self.max_root_bytes))
            .filter(|n| *n <= self.max_capture_bytes)
            .ok_or(CoordinatorError::Limit)
    }
}
pub struct TrustedShutdownFenceFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope_current: Option<bool>,
    pub continuous: Option<bool>,
    pub owner_control_allowed: Option<bool>,
    pub admission_fence_allowed: Option<bool>,
    pub resources_ready: Option<bool>,
}
struct Basis {
    owner: Arc<()>,
    marker: Arc<()>,
    context: SettingsExecutionContextV1,
    root: root_identity::NativeRootIdentity,
    limits: ShutdownFenceLimitsV1,
}
/// Bounded host-held inspection metadata, not a captured document body or grant.
#[derive(Clone)]
pub struct ShutdownFenceBasisV1(Arc<Basis>);
#[derive(Clone)]
pub struct ShutdownFenceIdV1 {
    owner: Arc<()>,
    marker: Arc<()>,
}
impl PartialEq for ShutdownFenceIdV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && Arc::ptr_eq(&self.marker, &other.marker)
    }
}
impl Eq for ShutdownFenceIdV1 {}
impl std::fmt::Debug for ShutdownFenceIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShutdownFenceIdV1(<opaque>)")
    }
}
impl std::fmt::Debug for ShutdownFenceBasisV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShutdownFenceBasisV1(<opaque>)")
    }
}
struct Fence {
    basis: Arc<Basis>,
    id: ShutdownFenceIdV1,
    reservation: retention::Reservation,
    charge: usize,
    wait: wait::State,
    accepted: acceptance::State,
    attempt: attempt::State,
}
#[derive(Default)]
pub(super) struct State {
    fence: Option<Fence>,
    pub(super) retired: bool,
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Shutdown(<owned>)")
    }
}
fn facts(f: &TrustedShutdownFenceFactsV1<'_>) -> bool {
    [
        f.authorized,
        f.scope_current,
        f.continuous,
        f.owner_control_allowed,
        f.admission_fence_allowed,
        f.resources_ready,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
impl SourceCoordinator {
    /// Protected local invocation stop, not provider cancellation or effect
    /// settlement. Original terminal outcomes and retained intent/holds survive.
    pub fn stop_shutdown_invocation_v1(
        &mut self,
        fence: &ShutdownFenceIdV1,
        invocation: &InvocationId,
        control: &TrustedShutdownFenceFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.validate_shutdown_owner_control(fence, control)?;
            self.stop_invocation(invocation, r).map_err(|e| match e {
                InvocationError::Faulted => CoordinatorError::Faulted,
                _ => CoordinatorError::Unavailable,
            })
        })();
        if result == Err(CoordinatorError::Faulted) {
            self.faulted = true;
        }
        observe_service(r, Boundary::ShutdownInvocationStop, &result, timer.finish());
        result
    }
    pub(super) fn validate_shutdown_owner_control(
        &self,
        id: &ShutdownFenceIdV1,
        control: &TrustedShutdownFenceFactsV1<'_>,
    ) -> Result<(), CoordinatorError> {
        self.validate_shutdown_disposition_fence(id)?;
        let fence = self
            .shutdown
            .fence
            .as_ref()
            .ok_or(CoordinatorError::Unavailable)?;
        if fence.basis.context != *control.context || !facts(control) {
            return Err(CoordinatorError::Unavailable);
        }
        Ok(())
    }
    pub(super) fn validate_shutdown_disposition_fence(
        &self,
        id: &ShutdownFenceIdV1,
    ) -> Result<(), CoordinatorError> {
        let fence = self
            .shutdown
            .fence
            .as_ref()
            .ok_or(CoordinatorError::Unavailable)?;
        if fence.id != *id {
            return Err(CoordinatorError::Unavailable);
        }
        self.retention
            .validate_exact(&fence.reservation, fence.charge)
            .map_err(|_| CoordinatorError::Faulted)
    }
    pub(super) fn shutdown_fence_marker(&self) -> Option<Arc<()>> {
        self.shutdown.fence.as_ref().map(|f| f.id.marker.clone())
    }
    /// Ordinary ingress/final-publication guard; never confused with integrity.
    pub(super) fn ensure_ordinary_admission(&self) -> Result<(), CoordinatorError> {
        self.ensure_operational()?;
        self.ensure_admission_unfenced()
    }
    /// Final installation may already carry an integrity latch. This guard never
    /// clears that latch and does not mistake it for the independent fence.
    pub(super) fn ensure_admission_unfenced(&self) -> Result<(), CoordinatorError> {
        if self.shutdown.retired {
            return Err(CoordinatorError::Unavailable);
        }
        if let Some(f) = &self.shutdown.fence {
            self.retention
                .validate_exact(&f.reservation, f.charge)
                .map_err(|_| CoordinatorError::Faulted)?;
            return Err(CoordinatorError::Unavailable);
        }
        Ok(())
    }
    pub fn capture_shutdown_fence_basis_v1(
        &self,
        limits: ShutdownFenceLimitsV1,
        f: &TrustedShutdownFenceFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownFenceBasisV1, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission()?;
            if !facts(f) {
                return Err(CoordinatorError::Unavailable);
            }
            limits.required_capture_bytes()?;
            let bytes = settings::basis::labels(f.context)
                .into_iter()
                .try_fold(0usize, |n, s| n.checked_add(s.len()))
                .ok_or(CoordinatorError::Limit)?;
            if bytes > limits.max_context_bytes
                || self
                    .measure_semantic_root()
                    .map_err(|_| CoordinatorError::Faulted)?
                    .total_bytes
                    > limits.max_root_bytes
            {
                return Err(CoordinatorError::Limit);
            }
            let context = settings::basis::copy_context(f.context).map_err(|e| match e {
                SettingsError::Capacity => CoordinatorError::Capacity,
                _ => CoordinatorError::Limit,
            })?;
            Ok(ShutdownFenceBasisV1(Arc::new(Basis {
                owner: self.namespace.clone(),
                marker: Arc::new(()),
                context,
                root: self
                    .native_root_identity()
                    .map_err(|_| CoordinatorError::Faulted)?,
                limits,
            })))
        })();
        observe_service(r, Boundary::ShutdownFenceCapture, &result, timer.finish());
        result
    }
    /// Once-only actual owner fence. Matching retry is identity-only; there is
    /// deliberately no reopen or retirement route before full shutdown discharge.
    pub fn install_shutdown_fence_v1(
        &mut self,
        basis: &ShutdownFenceBasisV1,
        f: &TrustedShutdownFenceFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownFenceIdV1, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            if !Arc::ptr_eq(&basis.0.owner, &self.namespace)
                || basis.0.context != *f.context
                || !facts(f)
            {
                return Err(CoordinatorError::Unavailable);
            }
            if let Some(old) = &self.shutdown.fence {
                self.retention
                    .validate_exact(&old.reservation, old.charge)
                    .map_err(|_| CoordinatorError::Faulted)?;
                return if Arc::ptr_eq(&basis.0.marker, &old.basis.marker) {
                    Ok(old.id.clone())
                } else {
                    Err(CoordinatorError::Unavailable)
                };
            }
            self.ensure_operational()?;
            if self.source_facade_busy || self.invocations.shutdown_execution_busy() {
                return Err(CoordinatorError::Unavailable);
            }
            if self
                .native_root_identity()
                .map_err(|_| CoordinatorError::Faulted)?
                != basis.0.root
            {
                return Err(CoordinatorError::Stale);
            }
            let charge = basis.0.limits.required_capture_bytes()?;
            // Allocate the last private token before changing the shared ledger.
            // The reserve + Option installation below has no allocator/callback.
            let id = ShutdownFenceIdV1 {
                owner: self.namespace.clone(),
                marker: Arc::new(()),
            };
            let reservation = self.retention.reserve(charge).map_err(|e| match e {
                RetentionErrorV1::Faulted => CoordinatorError::Faulted,
                RetentionErrorV1::Limit => CoordinatorError::Limit,
                RetentionErrorV1::Unavailable => CoordinatorError::Unavailable,
            })?;
            self.shutdown.fence = Some(Fence {
                basis: basis.0.clone(),
                id: id.clone(),
                reservation,
                charge,
                wait: wait::State::Unstarted,
                accepted: acceptance::State::Unrequested,
                attempt: attempt::State::Unreturned,
            });
            Ok(id)
        })();
        if result == Err(CoordinatorError::Faulted) {
            self.faulted = true;
        }
        observe_service(r, Boundary::ShutdownFenceAdmission, &result, timer.finish());
        result
    }
}
