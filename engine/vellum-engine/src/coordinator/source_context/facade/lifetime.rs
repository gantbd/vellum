//! Linear temporary reservations; native retained obligations are never refunded.
use super::*;
pub(super) struct Gateway<'a> {
    pub(super) owner: &'a mut SourceCoordinator,
    pub(super) recorder: &'a mut Recorder,
    classification: Option<retention::Reservation>,
    workspace: Option<retention::Reservation>,
    charge: usize,
    armed: bool,
    family: Family,
}
#[derive(Clone, Copy)]
pub(super) enum Family {
    Source,
    Annotation,
    Acquisition,
    History,
    Search,
}
/// Private first-party dispatch only; never a caller/provider execution hook.
pub(super) fn execute<T>(
    owner: &mut SourceCoordinator,
    family: Family,
    r: &mut Recorder,
    run: impl FnOnce(&mut Gateway<'_>) -> Result<T, Error>,
) -> Result<T, Error> {
    let out = execute_inner(owner, family, r, run);
    owner.release_measurements.record(&out);
    out
}
fn execute_inner<T>(
    owner: &mut SourceCoordinator,
    family: Family,
    r: &mut Recorder,
    run: impl FnOnce(&mut Gateway<'_>) -> Result<T, Error>,
) -> Result<T, Error> {
    let timing = r.begin_service_timing();
    if owner.faulted || owner.source_facade_busy {
        timed_observation(
            r,
            family.admission(),
            if owner.faulted {
                Disposition::Unknown
            } else {
                Disposition::Refused
            },
            timing.finish(),
        );
        return Err(if owner.faulted {
            Error::Faulted
        } else {
            Error::Unavailable
        });
    }
    if owner.source_facade_workspace != 0 {
        owner.faulted = true;
        timed_observation(r, family.admission(), Disposition::Unknown, timing.finish());
        return Err(Error::Faulted); // never erase or adopt someone else's debt
    }
    let out = catch_unwind(AssertUnwindSafe(|| {
        let mut gateway = match family {
            Family::Source => Gateway::new(owner, r),
            Family::Annotation => Gateway::for_annotation(owner, r),
            Family::Acquisition => Gateway::for_acquisition(owner, r),
            Family::History => Gateway::for_history(owner, r),
            Family::Search => Gateway::for_search(owner, r),
        };
        let out = run(&mut gateway);
        gateway.finish(&out);
        out
    }));
    // Gateway Drop owns cleanup and the health fence. Never deliver an already
    // produced value before it has discharged only its own temporary obligations.
    let reply = if owner.faulted {
        Err(Error::Faulted)
    } else {
        out.unwrap_or(Err(Error::Faulted))
    };
    timed_observation(
        r,
        family.release(),
        if owner.faulted {
            Disposition::Unknown
        } else if reply.is_ok() {
            Disposition::Completed
        } else {
            Disposition::Refused
        },
        timing.finish(),
    );
    reply
}
impl Family {
    fn admission(self) -> Boundary {
        match self {
            Self::Source => Boundary::SourceFacadeAdmission,
            Self::Annotation => Boundary::AnnotationFacadeAdmission,
            Self::Acquisition => Boundary::AcquisitionFacadeAdmission,
            Self::History => Boundary::HistoryFacadeAdmission,
            Self::Search => Boundary::SearchFacadeAdmission,
        }
    }
    fn release(self) -> Boundary {
        match self {
            Self::Source => Boundary::SourceFacadeRelease,
            Self::Annotation => Boundary::AnnotationFacadeRelease,
            Self::Acquisition => Boundary::AcquisitionFacadeRelease,
            Self::History => Boundary::HistoryFacadeRelease,
            Self::Search => Boundary::SearchFacadeRelease,
        }
    }
}
fn parent(e: RetentionErrorV1) -> Error {
    match e {
        RetentionErrorV1::Faulted => Error::Faulted,
        RetentionErrorV1::Limit => Error::Limit,
        RetentionErrorV1::Unavailable => Error::Unavailable,
    }
}
impl<'a> Gateway<'a> {
    pub(super) fn new(owner: &'a mut SourceCoordinator, recorder: &'a mut Recorder) -> Self {
        owner.source_facade_busy = true;
        Self {
            owner,
            recorder,
            classification: None,
            workspace: None,
            charge: 0,
            armed: true,
            family: Family::Source,
        }
    }
    pub(super) fn for_annotation(
        owner: &'a mut SourceCoordinator,
        recorder: &'a mut Recorder,
    ) -> Self {
        let mut guard = Self::new(owner, recorder);
        guard.family = Family::Annotation;
        guard
    }
    pub(super) fn for_acquisition(
        owner: &'a mut SourceCoordinator,
        recorder: &'a mut Recorder,
    ) -> Self {
        let mut guard = Self::new(owner, recorder);
        guard.family = Family::Acquisition;
        guard
    }
    pub(super) fn for_search(owner: &'a mut SourceCoordinator, recorder: &'a mut Recorder) -> Self {
        let mut guard = Self::new(owner, recorder);
        guard.family = Family::Search;
        guard
    }
    pub(super) fn admit(&mut self, input: CaptureLimitsV1) -> Result<(), Error> {
        if self.owner.source_facade_workspace != 0 {
            return Err(Error::Faulted);
        }
        let fixed = size_of::<CaptureLimitsV1>();
        if matches!(self.family, Family::History)
            && fixed
                > self
                    .owner
                    .history_facade_headroom()
                    .map_err(history::native_error)?
        {
            return Err(Error::Limit);
        }
        self.classification = Some(self.owner.retention.reserve(fixed).map_err(parent)?);
        self.charge = fixed;
        self.owner.source_facade_workspace = fixed;
        #[cfg(test)]
        assert_ne!(
            self.owner.source_facade_fault,
            Some(Fault::Classification),
            "injected classification fault"
        );
        let maximum = if matches!(self.family, Family::History) {
            input.max_bytes
        } else {
            input.max_bytes.min(self.owner.settings_job_parent_limit())
        };
        let frames = match self.family {
            Family::Acquisition | Family::Search => 8,
            _ => 7,
        };
        let workspace = maximum.checked_mul(frames).ok_or(Error::Limit)?;
        let total = fixed.checked_add(workspace).ok_or(Error::Limit)?;
        if matches!(self.family, Family::History)
            && total
                > self
                    .owner
                    .history_facade_headroom()
                    .map_err(history::native_error)?
        {
            return Err(Error::Limit);
        }
        self.workspace = Some(self.owner.retention.reserve(workspace).map_err(parent)?);
        self.charge = total;
        self.owner.source_facade_workspace = total;
        #[cfg(test)]
        assert_ne!(
            self.owner.source_facade_fault,
            Some(Fault::Workspace),
            "injected workspace fault"
        );
        observation(
            self.recorder,
            match self.family {
                Family::Source => Boundary::SourceFacadeAdmission,
                Family::Annotation => Boundary::AnnotationFacadeAdmission,
                Family::Acquisition => Boundary::AcquisitionFacadeAdmission,
                Family::History => Boundary::HistoryFacadeAdmission,
                Family::Search => Boundary::SearchFacadeAdmission,
            },
            Disposition::Completed,
        );
        Ok(())
    }
    pub(super) fn finish<T>(&mut self, result: &Result<T, Error>) {
        #[cfg(test)]
        if self.owner.source_facade_fault == Some(Fault::RefundDrift) {
            self.owner.source_facade_workspace += 1;
        }
        self.armed = false;
        if matches!(result, Err(Error::Faulted)) {
            self.owner.faulted = true;
        }
    }
    pub(super) fn for_history(
        owner: &'a mut SourceCoordinator,
        recorder: &'a mut Recorder,
    ) -> Self {
        let mut guard = Self::new(owner, recorder);
        guard.family = Family::History;
        guard
    }
}
impl Drop for Gateway<'_> {
    fn drop(&mut self) {
        let mut failed = self.armed || self.owner.source_facade_workspace != self.charge;
        let fixed = size_of::<CaptureLimitsV1>();
        let mut remaining = self.charge;
        // Both are actual private linear reservations; neither aliases a native pool.
        for (reservation, bytes) in [
            (&mut self.workspace, self.charge.saturating_sub(fixed)),
            (&mut self.classification, fixed),
        ] {
            let Some(reservation) = reservation else {
                continue;
            };
            if self.owner.retention.release(reservation).is_err() {
                failed = true;
            } else if let Some(next) = remaining.checked_sub(bytes) {
                remaining = next;
            } else {
                failed = true;
            }
        }
        if failed {
            self.owner.faulted = true;
        }
        self.owner.source_facade_workspace = remaining;
        self.owner.source_facade_busy = false;
        observation(
            self.recorder,
            match self.family {
                Family::Source => Boundary::SourceFacadeRefund,
                Family::Annotation => Boundary::AnnotationFacadeRefund,
                Family::Acquisition => Boundary::AcquisitionFacadeRefund,
                Family::History => Boundary::HistoryFacadeRefund,
                Family::Search => Boundary::SearchFacadeRefund,
            },
            if failed {
                Disposition::Unknown
            } else {
                Disposition::Completed
            },
        );
    }
}
