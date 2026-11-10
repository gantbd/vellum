//! Once-only returned shutdown report at the existing fenced owner. Returning
//! does not discard content, settle effects, stop a host worker or release a root.
use super::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShutdownAttemptStatusV1 {
    /// The captured native work/ownership census is discharged, excluding the
    /// shutdown controller's own metadata. Not permission for final root release.
    Drained,
    UnresolvedRetained,
}
#[derive(Default)]
pub(super) enum State {
    #[default]
    Unreturned,
    Returned(Box<Returned>),
    Retired,
}
pub(super) struct Returned {
    limits: ShutdownInventoryLimitsV1,
    status: ShutdownAttemptStatusV1,
    report: ShutdownProgressV1,
    reservation: retention::Reservation,
}
fn charge(limits: ShutdownInventoryLimitsV1) -> Result<usize, CoordinatorError> {
    size_of::<Returned>()
        .checked_add(limits.max_capture_bytes)
        .ok_or(CoordinatorError::Limit)
}
fn parent(e: RetentionErrorV1) -> CoordinatorError {
    match e {
        RetentionErrorV1::Limit => CoordinatorError::Limit,
        RetentionErrorV1::Unavailable => CoordinatorError::Unavailable,
        RetentionErrorV1::Faulted => CoordinatorError::Faulted,
    }
}
impl State {
    pub(super) fn reservation(
        &self,
        ledger: &retention::RetentionState,
    ) -> Result<Option<usize>, CoordinatorError> {
        let Self::Returned(control) = self else {
            return Ok(None);
        };
        let bytes = charge(control.limits)?;
        ledger
            .validate_exact(&control.reservation, bytes)
            .map_err(parent)?;
        Ok(Some(bytes))
    }
}
fn work_discharged(cut: &ShutdownInventoryReadV1<'_>, resource_metadata_idle: bool) -> bool {
    let coverage = cut.coverage;
    coverage.documents
        && coverage.writer_tasks
        && coverage.artifacts
        && coverage.bindings
        && coverage.reader_tasks
        && coverage.all_owner_obligations
        && cut.funding.unrepresented_parent_bytes == 0
        && cut.documents.is_empty()
        && cut.writes.is_empty()
        && cut.artifacts.is_empty()
        && cut.bindings.is_empty()
        && cut.reads.is_empty()
        && cut.invocations.is_empty()
        && cut.observation_contexts.is_empty()
        && cut.read_writer_attachments.is_empty()
        && cut.dependencies.is_empty()
        && cut.owners.iter().all(|row| {
            if row.owner.role() == ShutdownOwnerRoleV1::ShutdownControl {
                return true;
            }
            if row.owner.role() == ShutdownOwnerRoleV1::ResourceMetadata {
                return resource_metadata_idle;
            }
            if row.owner.role() == ShutdownOwnerRoleV1::SemanticRoot {
                // This projector always counts the engine's one structural
                // root, even for a never-used empty engine. Live/disposed
                // document payloads and other owner holds are checked above
                // and by the complete owner census; the root header itself is
                // not outstanding work and cannot become zero before Drop.
                return row.state == ShutdownOwnerStateV1::Active
                    && row.retained_records == Some(1);
            }
            match row.state {
                ShutdownOwnerStateV1::Unconfigured | ShutdownOwnerStateV1::Closed => true,
                ShutdownOwnerStateV1::Active => row.retained_records == Some(0),
            }
        })
}
impl SourceCoordinator {
    /// Final native control retirement after fresh discharge. Removes only the
    /// controller's original reservation; an absorbing inline tombstone keeps
    /// all ordinary engine use closed. Inert configuration is freed by host Drop.
    /// This cannot force host/provider termination or establish crash durability.
    pub fn retire_shutdown_control_v1(
        &mut self,
        fence: &ShutdownFenceIdV1,
        limits: ShutdownInventoryLimitsV1,
        facts: &TrustedShutdownInventoryFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.check_shutdown_root_release_v1(fence, limits, facts, r)?;
            let was_faulted = self.faulted;
            self.faulted = true;
            let installed = self
                .shutdown
                .fence
                .as_mut()
                .expect("validated original control");
            self.retention
                .release(&mut installed.reservation)
                .map_err(parent)?;
            self.shutdown.retired = true;
            self.shutdown.fence = None;
            self.faulted = was_faulted;
            Ok(())
        })();
        if result == Err(CoordinatorError::Faulted) {
            self.faulted = true;
        }
        observe_service(
            r,
            Boundary::ShutdownControlRetirement,
            &result,
            timer.finish(),
        );
        result
    }
    /// Returns one historical as-of report. Identical retry reauthorizes and
    /// returns its original status; it never recaptures, renews funding or clears
    /// the admission fence. Original disposition and cleanup APIs remain usable.
    pub fn finish_shutdown_attempt_v1(
        &mut self,
        fence: &ShutdownFenceIdV1,
        limits: ShutdownInventoryLimitsV1,
        facts: &TrustedShutdownInventoryFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<ShutdownAttemptStatusV1, CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.validate_shutdown_owner_control(fence, facts.control)?;
            let installed = self
                .shutdown
                .fence
                .as_ref()
                .ok_or(CoordinatorError::Unavailable)?;
            self.shutdown_inventory_roles(&installed.basis, facts)?;
            installed.attempt.reservation(&self.retention)?;
            match &installed.attempt {
                State::Returned(control) => {
                    return if control.limits == limits {
                        Ok(control.status)
                    } else {
                        Err(CoordinatorError::Unavailable)
                    };
                }
                State::Retired => return Err(CoordinatorError::Unavailable),
                State::Unreturned => {}
            }
            let bytes = charge(limits)?;
            self.retention.check(bytes).map_err(parent)?;
            // Capture precedes this report's reservation/install. The historical
            // cut does not pretend to include its own later metadata allocation.
            let report = self.capture_shutdown_progress_v1(fence, limits, facts, r)?;
            let cut = self.read_shutdown_progress_v1(&report, facts, r)?;
            let status = if work_discharged(&cut, self.shutdown_resource_metadata_idle()) {
                ShutdownAttemptStatusV1::Drained
            } else {
                ShutdownAttemptStatusV1::UnresolvedRetained
            };
            let reservation = self.retention.reserve(bytes).map_err(parent)?;
            self.shutdown
                .fence
                .as_mut()
                .expect("validated original fence")
                .attempt = State::Returned(Box::new(Returned {
                limits,
                status,
                report,
                reservation,
            }));
            Ok(status)
        })();
        observe_service(r, Boundary::ShutdownAttemptReturn, &result, timer.finish());
        result
    }
    /// Current inventory disclosure is independent from owner stop/control.
    /// Late evidence cannot rewrite this original report or its as-of status.
    pub fn read_shutdown_attempt_report_v1<'a>(
        &'a self,
        fence: &ShutdownFenceIdV1,
        facts: &TrustedShutdownInventoryFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(ShutdownAttemptStatusV1, ShutdownInventoryReadV1<'a>), CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.validate_shutdown_owner_control(fence, facts.control)?;
            let installed = self
                .shutdown
                .fence
                .as_ref()
                .ok_or(CoordinatorError::Unavailable)?;
            self.shutdown_inventory_roles(&installed.basis, facts)?;
            installed.attempt.reservation(&self.retention)?;
            let State::Returned(control) = &installed.attempt else {
                return Err(CoordinatorError::Unavailable);
            };
            Ok((
                control.status,
                self.read_shutdown_progress_v1(&control.report, facts, r)?,
            ))
        })();
        observe_service(r, Boundary::ShutdownAttemptRelease, &result, timer.finish());
        result
    }
    /// Protected once-only metadata release, not producer/effect discharge.
    pub fn retire_shutdown_attempt_report_v1(
        &mut self,
        fence: &ShutdownFenceIdV1,
        facts: &TrustedShutdownFenceFactsV1<'_>,
        retention_released: Option<bool>,
        delivery_drained: Option<bool>,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.validate_shutdown_owner_control(fence, facts)?;
            self.shutdown
                .fence
                .as_ref()
                .ok_or(CoordinatorError::Unavailable)?
                .attempt
                .reservation(&self.retention)?;
            if retention_released != Some(true) || delivery_drained != Some(true) {
                return Err(CoordinatorError::Unavailable);
            }
            let State::Returned(control) = &mut self
                .shutdown
                .fence
                .as_mut()
                .expect("validated fence")
                .attempt
            else {
                return Err(CoordinatorError::Unavailable);
            };
            self.retention
                .release(&mut control.reservation)
                .map_err(parent)?;
            self.shutdown
                .fence
                .as_mut()
                .expect("validated fence")
                .attempt = State::Retired;
            Ok(())
        })();
        observe_service(
            r,
            Boundary::ShutdownAttemptRetirement,
            &result,
            timer.finish(),
        );
        result
    }
    /// Fresh owner-wide discharge check, not a historical report or a Rust Drop
    /// interceptor. The host controls process exit and must not infer persistence
    /// from this result. The admission fence remains installed after success.
    pub fn check_shutdown_root_release_v1(
        &self,
        fence: &ShutdownFenceIdV1,
        limits: ShutdownInventoryLimitsV1,
        facts: &TrustedShutdownInventoryFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), CoordinatorError> {
        let timer = r.begin_service_timing();
        let result = (|| {
            self.validate_shutdown_owner_control(fence, facts.control)?;
            let installed = self
                .shutdown
                .fence
                .as_ref()
                .ok_or(CoordinatorError::Unavailable)?;
            if !matches!(installed.attempt, State::Retired) {
                return Err(CoordinatorError::Unavailable);
            }
            let progress = self.capture_shutdown_progress_v1(fence, limits, facts, r)?;
            let cut = self.read_shutdown_progress_v1(&progress, facts, r)?;
            if !work_discharged(&cut, self.shutdown_resource_metadata_idle()) {
                return Err(CoordinatorError::Unavailable);
            }
            let own = cut
                .owners
                .iter()
                .find(|row| row.owner.role() == ShutdownOwnerRoleV1::ShutdownControl)
                .ok_or(CoordinatorError::Faulted)?;
            // Fence only. Accepted-disposition, wait and returned report holds
            // must be actually retired, not ignored as controller metadata.
            if own.retained_records != Some(1) || own.additional_reserved_bytes != Some(0) {
                return Err(CoordinatorError::Unavailable);
            }
            Ok(())
        })();
        observe_service(
            r,
            Boundary::ShutdownRootReleaseCheck,
            &result,
            timer.finish(),
        );
        result
    }
}
