//! Co-owned source-load identity, not dispatch, activation or result replay.
use super::*;
mod shutdown_inventory;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceReadOperationLimitsV1 {
    pub max_capture_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_label_bytes: usize,
    pub max_store_bytes: usize,
}
#[derive(Clone, Copy)]
pub struct SourceReadOperationOriginV1<'a> {
    pub operation: &'a [u8],
    pub effect: &'a [u8],
    pub attempt: &'a [u8],
}
pub struct OwnedSourceReadRequestV1<'a> {
    pub origin: SourceReadOperationOriginV1<'a>,
    pub target: &'a SourceReadTargetV1,
    pub strength: ReadStrength,
}
pub struct TrustedSourceReadOperationFactsV1<'a> {
    pub context: &'a SettingsExecutionContextV1,
    pub authorized: Option<bool>,
    pub scope: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub read_allowed: Option<bool>,
    pub retained: Option<bool>,
    pub reply_ready: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub resources_ready: Option<bool>,
    pub identity_allowed: Option<bool>,
    pub binding_coverage_complete: Option<bool>,
    pub provider_available: Option<bool>,
    pub launch_allowed: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceReadOperationAdmissionStatusV1 {
    Admitted,
    ObserveExisting,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceReadOperationAdmissionV1 {
    pub status: SourceReadOperationAdmissionStatusV1,
    pub task: SourceReadIdV1,
}
#[derive(Default)]
pub(super) enum State {
    #[default]
    Unconfigured,
    Active(Box<Registry>),
    Closed,
}
pub(super) struct Registry {
    limits: SourceReadOperationLimitsV1,
    max_issued: usize,
    rows: Vec<Entry>,
    reservation: retention::Reservation,
}
struct Origin {
    operation: Blob,
    effect: Blob,
    attempt: Blob,
}
struct Capture {
    catalog: Option<Arc<ReadCatalogCaptureV1>>,
    registration: Arc<Registration>,
    identity: Identity,
    strength: ReadStrength,
    context: SettingsExecutionContextV1,
}
struct Entry {
    origin: Origin,
    task: SourceReadIdV1,
    owned: bool,
    live: bool,
    capture: Option<Capture>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceReadOperationUsageV1 {
    pub issued: usize,
    pub owned_live: usize,
    pub raw_labeled: usize,
    pub retired: usize,
    pub spent_origins: usize,
    pub reserved_bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedSourceReadVersionV1(SourceReadVersion);
impl ObservedSourceReadVersionV1 {
    pub fn epoch(&self) -> Option<&[u8]> {
        self.0.epoch.as_ref().map(|v| v.0.as_ref())
    }
    pub fn token(&self) -> &[u8] {
        &self.0.token.0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeSourceReadTerminalWitnessV1 {
    End {
        version: ObservedSourceReadVersionV1,
        coherence: ReadStrength,
        chunks: usize,
        bytes: usize,
    },
    Failure(SourceReadFailureV1),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeSourceReadReceiptV1 {
    pub phase: ReadPhase,
    pub stopped: bool,
    pub quiescent: bool,
    pub contested: bool,
    pub decode_refused: bool,
    pub received_chunks: usize,
    pub received_bytes: usize,
    pub witnesses: [Option<NativeSourceReadTerminalWitnessV1>; 2],
}
fn witness(
    atom: &SourceReadAtom,
    max_version: usize,
) -> Result<Option<NativeSourceReadTerminalWitnessV1>, ResourceError> {
    match atom {
        SourceReadAtom::End {
            version,
            chunks,
            bytes,
        } => {
            if version.token.0.is_empty()
                || version.token.0.len() > max_version
                || version
                    .epoch
                    .as_ref()
                    .is_some_and(|v| v.0.is_empty() || v.0.len() > max_version)
            {
                return Err(ResourceError::Faulted);
            }
            Ok(Some(NativeSourceReadTerminalWitnessV1::End {
                version: ObservedSourceReadVersionV1(version.clone()),
                coherence: ReadStrength::CapturedRead,
                chunks: *chunks,
                bytes: *bytes,
            }))
        }
        SourceReadAtom::Failure(reason) => {
            Ok(Some(NativeSourceReadTerminalWitnessV1::Failure(*reason)))
        }
        _ => Ok(None),
    }
}
fn reserved(l: SourceReadOperationLimitsV1, count: usize) -> Result<usize, ResourceError> {
    let inventory = count
        .checked_mul(l.max_capture_bytes)
        .and_then(|n| n.checked_mul(2))
        .ok_or(ResourceError::Limit)?;
    add(inventory, add(size_of::<Registry>(), size_of::<State>())?)
}
fn origin_bytes(origin: SourceReadOperationOriginV1<'_>) -> Result<usize, ResourceError> {
    [
        origin.operation.len(),
        origin.effect.len(),
        origin.attempt.len(),
    ]
    .into_iter()
    .try_fold(size_of::<Origin>(), add)
}
fn capsule_bytes(
    request: &OwnedSourceReadRequestV1<'_>,
    context: Option<&SettingsExecutionContextV1>,
) -> Result<usize, ResourceError> {
    [
        request.origin.operation.len(),
        request.origin.effect.len(),
        request.origin.attempt.len(),
        request.target.identity.key.0.len(),
    ]
    .into_iter()
    .chain(
        context
            .into_iter()
            .flat_map(|c| settings::basis::labels(c).map(str::len)),
    )
    .try_fold(size_of::<Entry>(), add)
    .and_then(|n| {
        add(
            n,
            request
                .target
                .catalog
                .as_ref()
                .map(|c| c.native_bytes())
                .transpose()?
                .unwrap_or(0),
        )
    })
}
fn input(
    l: SourceReadOperationLimitsV1,
    request: &OwnedSourceReadRequestV1<'_>,
    context: Option<&SettingsExecutionContextV1>,
) -> Result<(), ResourceError> {
    for label in [
        request.origin.operation,
        request.origin.effect,
        request.origin.attempt,
    ] {
        if std::str::from_utf8(label).is_err() {
            return Err(ResourceError::Malformed);
        }
        if label.is_empty() || label.len() > l.max_label_bytes {
            return Err(ResourceError::Limit);
        }
    }
    if context.is_some_and(|c| {
        settings::basis::labels(c)
            .iter()
            .any(|s| s.is_empty() || s.len() > l.max_label_bytes)
    }) || origin_bytes(request.origin)? > l.max_capture_bytes
        || capsule_bytes(request, context)? > l.max_capture_bytes
        || size_of::<SourceReadOperationAdmissionV1>() > l.max_reply_bytes
    {
        return Err(ResourceError::Limit);
    }
    Ok(())
}
fn release(f: &TrustedSourceReadOperationFactsV1<'_>) -> bool {
    [
        f.authorized,
        f.scope,
        f.continuous,
        f.live,
        f.not_stopped,
        f.read_allowed,
        f.retained,
        f.reply_ready,
        f.transfer_ready,
        f.resources_ready,
    ]
    .into_iter()
    .all(|v| v == Some(true))
}
impl SourceCoordinator {
    pub(in crate::coordinator::resource) fn owned_source_read_control(
        &self,
        id: &SourceReadIdV1,
        context: &SettingsExecutionContextV1,
    ) -> Result<(bool, bool), ResourceError> {
        self.validate_source_reader()?;
        let task = self.source_read_task(id)?;
        let index = task.operation.ok_or(ResourceError::Unavailable)?;
        let row = self
            .source_read_operations()?
            .rows
            .get(index)
            .ok_or(ResourceError::Faulted)?;
        if !row.owned
            || !row.live
            || row.task != *id
            || row.capture.as_ref().is_none_or(|c| c.context != *context)
        {
            return Err(ResourceError::Unavailable);
        }
        Ok((task.stopped, task.quiescent))
    }
    pub(in crate::coordinator::resource) fn owned_source_read_receipt(
        &self,
        id: &SourceReadIdV1,
        context: &SettingsExecutionContextV1,
    ) -> Result<NativeSourceReadReceiptV1, ResourceError> {
        let (stopped, quiescent) = self.owned_source_read_control(id, context)?;
        let task = self.source_read_task(id)?;
        let mut witnesses = [None, None];
        let mut count = 0;
        for atom in task.terminal.iter().chain(task.witnesses.iter().flatten()) {
            if let Some(value) = witness(atom, task.registration.limits.max_validator_bytes)?
                && !witnesses.iter().flatten().any(|w| w == &value)
            {
                if count >= witnesses.len() {
                    return Err(ResourceError::Faulted);
                }
                witnesses[count] = Some(value);
                count += 1;
            }
        }
        Ok(NativeSourceReadReceiptV1 {
            phase: task.phase,
            stopped,
            quiescent,
            contested: task.contested,
            decode_refused: task.refused,
            received_chunks: task.chunks.len(),
            received_bytes: task.bytes.len(),
            witnesses,
        })
    }
    fn source_read_operations(&self) -> Result<&Registry, ResourceError> {
        match &self.source_reader()?.operations {
            State::Active(s) => Ok(s),
            _ => Err(ResourceError::Unavailable),
        }
    }
    pub(super) fn validate_source_read_operations(&self) -> Result<(), ResourceError> {
        let reader = self.source_reader()?;
        if matches!(reader.operations, State::Closed)
            && (!reader.stopped || reader.rows.iter().any(Option::is_some) || reader.held != 0)
        {
            return Err(ResourceError::Faulted);
        }
        let State::Active(s) = &reader.operations else {
            if reader.rows.iter().flatten().any(|t| t.operation.is_some()) {
                return Err(ResourceError::Faulted);
            }
            return Ok(());
        };
        self.retention
            .validate_exact(
                &s.reservation,
                reserved(s.limits, s.max_issued).map_err(|_| ResourceError::Faulted)?,
            )
            .map_err(parent)?;
        if s.max_issued != reader.limits.max_issued
            || s.rows.len() > s.max_issued
            || s.rows.len() > reader.rows.len()
            || reserved(s.limits, s.max_issued)
                .ok()
                .is_none_or(|n| n > s.limits.max_store_bytes)
        {
            return Err(ResourceError::Faulted);
        }
        for (index, row) in s.rows.iter().enumerate() {
            let origin = SourceReadOperationOriginV1 {
                operation: &row.origin.operation.0,
                effect: &row.origin.effect.0,
                attempt: &row.origin.attempt.0,
            };
            if !Arc::ptr_eq(&row.task.owner, &self.namespace)
                || [
                    &row.origin.operation,
                    &row.origin.effect,
                    &row.origin.attempt,
                ]
                .iter()
                .any(|v| {
                    v.0.is_empty()
                        || v.0.len() > s.limits.max_label_bytes
                        || std::str::from_utf8(&v.0).is_err()
                })
                || origin_bytes(origin)
                    .ok()
                    .is_none_or(|n| n > s.limits.max_capture_bytes)
                || [
                    origin.operation.len(),
                    origin.effect.len(),
                    origin.attempt.len(),
                ]
                .into_iter()
                .try_fold(size_of::<Entry>(), usize::checked_add)
                .is_none_or(|n| n > s.limits.max_capture_bytes)
                || s.rows[..index].iter().any(|p| {
                    p.origin.operation == row.origin.operation
                        && (p.owned
                            || row.owned
                            || (p.origin.effect == row.origin.effect
                                && p.origin.attempt == row.origin.attempt))
                })
                || row.capture.is_some() != (row.owned && row.live)
            {
                return Err(ResourceError::Faulted);
            }
            if row.live {
                let task = self
                    .source_read_task(&row.task)
                    .map_err(|_| ResourceError::Faulted)?;
                if task.operation != Some(index) {
                    return Err(ResourceError::Faulted);
                }
                if let Some(c) = &row.capture {
                    if !Arc::ptr_eq(&c.registration, &task.registration)
                        || c.identity != task.identity
                        || !ReadCatalogCaptureV1::same(c.catalog.as_ref(), task.catalog.as_ref())
                        || c.strength != ReadStrength::CapturedRead
                    {
                        return Err(ResourceError::Faulted);
                    }
                    let target = SourceReadTargetV1 {
                        catalog: c.catalog.clone(),
                        registration: c.registration.clone(),
                        identity: c.identity.clone(),
                    };
                    let req = OwnedSourceReadRequestV1 {
                        origin,
                        target: &target,
                        strength: c.strength,
                    };
                    input(s.limits, &req, Some(&c.context)).map_err(|_| ResourceError::Faulted)?;
                }
            } else if reader.rows.get(row.task.slot).is_none_or(Option::is_some) {
                return Err(ResourceError::Faulted);
            }
        }
        for (slot, task) in reader.rows.iter().enumerate() {
            if let Some(index) = task.as_ref().and_then(|t| t.operation)
                && s.rows
                    .get(index)
                    .is_none_or(|row| !row.live || row.task.slot != slot)
            {
                return Err(ResourceError::Faulted);
            }
        }
        Ok(())
    }
    pub fn configure_source_read_operations_v1(
        &mut self,
        l: SourceReadOperationLimitsV1,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.validate_source_reader()?;
            let reader = self.source_reader()?;
            match &reader.operations {
                State::Active(s) => {
                    return if s.limits == l {
                        Ok(())
                    } else {
                        Err(ResourceError::Unavailable)
                    };
                }
                State::Closed => return Err(ResourceError::Unavailable),
                State::Unconfigured => {}
            }
            self.ensure_admission_unfenced().map_err(owner_error)?;
            if reader.stopped {
                return Err(ResourceError::Unavailable);
            }
            let max_issued = reader.limits.max_issued;
            let total = reserved(l, max_issued)?;
            if l.max_capture_bytes < size_of::<Entry>().max(size_of::<Origin>())
                || total > l.max_store_bytes
            {
                return Err(ResourceError::Limit);
            }
            self.retention.check(total).map_err(parent)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(max_issued)
                .map_err(|_| ResourceError::Capacity)?;
            let reservation = self.retention.reserve(total).map_err(parent)?;
            self.source_reader_mut()?.operations = State::Active(Box::new(Registry {
                limits: l,
                max_issued,
                rows,
                reservation,
            }));
            Ok(())
        })();
        if matches!(&result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture_service(
            r,
            Boundary::SourceReadOperationConfiguration,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn request_owned_source_read_v1(
        &mut self,
        request: &OwnedSourceReadRequestV1<'_>,
        f: &TrustedSourceReadOperationFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourceReadOperationAdmissionV1, ResourceError> {
        let timing = r.begin_service_timing();
        let existing = self.source_read_operations().is_ok_and(|s| {
            s.rows
                .iter()
                .any(|p| p.origin.operation.0.as_ref() == request.origin.operation)
        });
        let result = (|| {
            input(
                self.source_read_operations()?.limits,
                request,
                Some(f.context),
            )?;
            if !release(f) || !Arc::ptr_eq(&request.target.registration.owner, &self.namespace) {
                return Err(ResourceError::Unavailable);
            }
            self.validate_source_reader()?;
            if let Some(row) = self
                .source_read_operations()?
                .rows
                .iter()
                .find(|p| p.origin.operation.0.as_ref() == request.origin.operation)
            {
                let c = row.capture.as_ref().ok_or(ResourceError::Unavailable)?;
                if !row.owned
                    || !row.live
                    || row.origin.effect.0.as_ref() != request.origin.effect
                    || row.origin.attempt.0.as_ref() != request.origin.attempt
                    || c.context != *f.context
                    || c.strength != request.strength
                    || c.identity != request.target.identity
                    || !ReadCatalogCaptureV1::same(
                        c.catalog.as_ref(),
                        request.target.catalog.as_ref(),
                    )
                    || !Arc::ptr_eq(&c.registration, &request.target.registration)
                {
                    return Err(ResourceError::Unavailable);
                }
                return Ok(SourceReadOperationAdmissionV1 {
                    status: SourceReadOperationAdmissionStatusV1::ObserveExisting,
                    task: row.task.clone(),
                });
            }
            self.ensure_operational().map_err(owner_error)?;
            if [
                f.identity_allowed,
                f.binding_coverage_complete,
                f.provider_available,
                f.launch_allowed,
            ]
            .into_iter()
            .any(|v| v != Some(true))
            {
                return Err(ResourceError::Unavailable);
            }
            let task = self.install_source_read_operation(request, Some(f.context), r)?;
            Ok(SourceReadOperationAdmissionV1 {
                status: SourceReadOperationAdmissionStatusV1::Admitted,
                task,
            })
        })();
        if matches!(&result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture_service(
            r,
            if existing {
                Boundary::SourceReadOperationRecovery
            } else {
                Boundary::SourceReadOperationAdmission
            },
            &result,
            timing.finish(),
        );
        result
    }
    fn install_source_read_operation(
        &mut self,
        request: &OwnedSourceReadRequestV1<'_>,
        context: Option<&SettingsExecutionContextV1>,
        r: &mut Recorder,
    ) -> Result<SourceReadIdV1, ResourceError> {
        let s = self.source_read_operations()?;
        input(s.limits, request, context)?;
        if s.rows.len() >= s.max_issued {
            return Err(ResourceError::Limit);
        }
        if s.rows.len() >= s.rows.capacity() {
            return Err(ResourceError::Faulted);
        }
        let index = s.rows.len();
        let origin = Origin {
            operation: blob(request.origin.operation, s.limits.max_label_bytes, true)?,
            effect: blob(request.origin.effect, s.limits.max_label_bytes, true)?,
            attempt: blob(request.origin.attempt, s.limits.max_label_bytes, true)?,
        };
        let original = context
            .map(|c| settings::basis::copy_context(c).map_err(|_| ResourceError::Capacity))
            .transpose()?;
        let registration = request.target.registration.clone();
        let identity = request.target.identity.clone();
        let target = SourceReadTargetV1 {
            catalog: request.target.catalog.clone(),
            registration: registration.clone(),
            identity: identity.clone(),
        };
        let task = self.admit_source_read_v1(target, request.strength, r)?;
        self.faulted = true;
        let reader = match &mut self.resources.source_reader {
            super::State::Active(s) => s,
            _ => unreachable!("validated source reader"),
        };
        reader.rows[task.slot]
            .as_mut()
            .expect("new source read")
            .operation = Some(index);
        let State::Active(s) = &mut reader.operations else {
            unreachable!("validated operation registry")
        };
        s.rows.push(Entry {
            origin,
            task: task.clone(),
            owned: original.is_some(),
            live: true,
            capture: original.map(|context| Capture {
                catalog: request.target.catalog.clone(),
                registration,
                identity,
                strength: request.strength,
                context,
            }),
        });
        self.faulted = false;
        Ok(task)
    }
    pub fn admit_labeled_source_read_v1(
        &mut self,
        origin: SourceReadOperationOriginV1<'_>,
        target: SourceReadTargetV1,
        strength: ReadStrength,
        r: &mut Recorder,
    ) -> Result<SourceReadIdV1, ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(owner_error)?;
            self.validate_source_reader()?;
            let request = OwnedSourceReadRequestV1 {
                origin,
                target: &target,
                strength,
            };
            let s = self.source_read_operations()?;
            input(s.limits, &request, None)?;
            if s.rows.iter().any(|p| {
                p.origin.operation.0.as_ref() == origin.operation
                    && (p.owned
                        || (p.origin.effect.0.as_ref() == origin.effect
                            && p.origin.attempt.0.as_ref() == origin.attempt))
            }) {
                return Err(ResourceError::Unavailable);
            }
            self.install_source_read_operation(&request, None, r)
        })();
        if matches!(&result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture_service(
            r,
            Boundary::SourceReadOperationAdmission,
            &result,
            timing.finish(),
        );
        result
    }
    pub(super) fn spend_source_read_operation(&mut self, index: usize) {
        let super::State::Active(reader) = &mut self.resources.source_reader else {
            unreachable!("validated source reader")
        };
        let State::Active(s) = &mut reader.operations else {
            unreachable!("validated source operations")
        };
        s.rows[index].capture = None;
        s.rows[index].live = false;
    }
    pub fn source_read_operation_usage_v1(
        &self,
    ) -> Result<SourceReadOperationUsageV1, ResourceError> {
        self.validate_source_reader()?;
        let s = self.source_read_operations()?;
        Ok(SourceReadOperationUsageV1 {
            issued: s.rows.len(),
            owned_live: s.rows.iter().filter(|p| p.owned && p.live).count(),
            raw_labeled: s.rows.iter().filter(|p| !p.owned).count(),
            retired: s.rows.iter().filter(|p| !p.live).count(),
            spent_origins: s.rows.len(),
            reserved_bytes: reserved(s.limits, s.max_issued)?,
        })
    }
    pub fn retire_source_read_operations_v1(
        &mut self,
        owner_control: Option<bool>,
        retention_released: Option<bool>,
        delivery_drained: Option<bool>,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.validate_source_reader()?;
            let reader = self.source_reader()?;
            let s = self.source_read_operations()?;
            if !reader.stopped
                || reader.rows.iter().any(Option::is_some)
                || reader.held != 0
                || s.rows.iter().any(|p| p.live || p.capture.is_some())
                || [owner_control, retention_released, delivery_drained]
                    .into_iter()
                    .any(|v| v != Some(true))
            {
                return Err(ResourceError::Retained);
            }
            self.retention.validate(&s.reservation).map_err(parent)?;
            let State::Active(mut s) =
                std::mem::replace(&mut self.source_reader_mut()?.operations, State::Closed)
            else {
                unreachable!("validated source operations")
            };
            self.retention.release(&mut s.reservation).map_err(parent)
        })();
        if matches!(&result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture_service(
            r,
            Boundary::SourceReadOperationRetirement,
            &result,
            timing.finish(),
        );
        result
    }
}
