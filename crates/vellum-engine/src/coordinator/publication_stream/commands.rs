//! Bounded original commands retained inside the actual source publisher.
use super::*;
mod shutdown_inventory;
use crate::coordinator::publication_command::Lifecycle;
type PhaseV1 = PublicationCommandPhaseV1;
mod ingress;
pub use ingress::*;
mod receipts;
#[cfg(test)]
pub(in crate::coordinator) use ingress::tests::{command_envelope, receipt_envelope};
#[cfg(test)]
pub(in crate::coordinator) use receipts::tests::{
    facts as source_receipt_test_facts, limits as source_receipt_test_limits,
    open as source_receipt_test_open, setup_with_budget as source_receipt_test_setup,
};
pub use receipts::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePublicationCommandLimitsV1 {
    /// Concurrent retained original commands, including known failed results.
    pub max_commands: usize,
    /// Lifetime command identities; separate from stream/generation issuance.
    pub max_issued: usize,
    /// Bounded owned scope copy per live command.
    pub max_context_bytes: usize,
    /// Complete retained baseline and projection charge, not a physical RSS bound.
    pub max_result_bytes: usize,
    pub max_pool_bytes: usize,
}
impl SourcePublicationCommandLimitsV1 {
    pub fn required_pool_bytes(self) -> Result<usize, Error> {
        size_of::<Table>()
            .checked_add(
                self.max_issued
                    .checked_mul(size_of::<Option<Command>>())
                    .ok_or(Error::Limit)?,
            )
            .and_then(|n| {
                n.checked_add(
                    self.max_commands
                        .checked_mul(self.max_context_bytes.checked_add(self.max_result_bytes)?)?,
                )
            })
            .filter(|n| *n <= self.max_pool_bytes)
            .ok_or(Error::Limit)
    }
}
#[derive(Clone)]
pub struct SourcePublicationCommandIdV1 {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for SourcePublicationCommandIdV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for SourcePublicationCommandIdV1 {}
#[derive(Clone, PartialEq, Eq)]
pub enum SourcePublicationCommandOperationV1 {
    Establish,
    Resynchronize { stream: SourcePublicationStreamIdV1 },
}
#[derive(Clone, PartialEq, Eq)]
pub enum SourcePublicationCommandReplyV1 {
    Settled {
        command: SourcePublicationCommandIdV1,
    },
    ObserveExisting {
        command: SourcePublicationCommandIdV1,
    },
}
pub struct SourcePublicationCommandRequestV1<'a> {
    pub scope: &'a SourceReadScopeV1,
    pub operation: &'a SourcePublicationCommandOperationV1,
    /// Absent on identity-only protected retry. No ordinary permission is inferred.
    pub current: Option<&'a TrustedSourcePublicationFactsV1<'a>>,
    pub owner_control_allowed: Option<bool>,
    pub max_reply_bytes: usize,
}
/// Privileged host seam, not a recipient F06 payload or disclosure authorization.
pub struct SourcePublicationCommandFactsV1<'a> {
    pub phase: PublicationCommandPhaseV1,
    pub native_started: bool,
    pub operation: &'a SourcePublicationCommandOperationV1,
    pub as_observed: Option<&'a Result<Arc<SourcePublicationBaselineV1>, Error>>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourcePublicationCommandUsageV1 {
    pub commands: usize,
    pub issued_lifetime: usize,
    pub reserved_pool_bytes: usize,
    pub closed: bool,
}
redacted!(
    SourcePublicationCommandIdV1,
    SourcePublicationCommandOperationV1,
    SourcePublicationCommandReplyV1,
    SourcePublicationCommandRequestV1<'_>,
    SourcePublicationCommandFactsV1<'_>
);

struct Command {
    scope: SourceReadScopeV1,
    operation: SourcePublicationCommandOperationV1,
    root: root_identity::NativeRootIdentity,
    generation: Option<SourcePublicationGenerationV1>,
    life: Lifecycle<Result<Arc<SourcePublicationBaselineV1>, Error>>,
}
pub(super) struct Table {
    limits: SourcePublicationCommandLimitsV1,
    rows: Vec<Option<Command>>,
    reservation: retention::Reservation,
    charge: usize,
    receipts: receipts::State,
    #[cfg(test)]
    fault_after_native: bool,
}
#[derive(Default)]
pub(super) enum State {
    #[default]
    Unconfigured,
    Active(Box<Table>),
    Closed {
        issued: usize,
    },
}
impl State {
    pub(super) fn closed(&self) -> bool {
        !matches!(self, Self::Active(_))
    }
    fn table(&self, ledger: &retention::RetentionState) -> Result<&Table, Error> {
        let Self::Active(t) = self else {
            return Err(Error::Unavailable);
        };
        ledger
            .validate_exact(&t.reservation, t.charge)
            .map_err(parent)?;
        if t.limits.required_pool_bytes().map_err(|_| Error::Faulted)? != t.charge
            || t.rows.capacity() < t.limits.max_issued
        {
            return Err(Error::Faulted);
        }
        if t.rows.len() > t.limits.max_issued
            || t.rows.iter().flatten().count() > t.limits.max_commands
            || t.rows.iter().flatten().any(|c| !c.life.valid())
        {
            return Err(Error::Faulted);
        }
        Ok(t)
    }
    pub(super) fn document_pins(
        &self,
        document: &DocumentId,
        ledger: &retention::RetentionState,
        owner: &Arc<()>,
    ) -> Result<usize, Error> {
        match self {
            Self::Unconfigured | Self::Closed { .. } => Ok(0),
            Self::Active(_) => {
                let t = self.table(ledger)?;
                t.receipts.validate_claims(owner, &t.rows, ledger)?;
                t.rows
                    .iter()
                    .flatten()
                    .filter(|c| c.scope.document == *document)
                    .count()
                    .checked_add(t.receipts.document_pins(document, ledger)?)
                    .ok_or(Error::Faulted)
            }
        }
    }
}
fn largest_result(projection_bytes: usize) -> Result<usize, Error> {
    projection_bytes
        .checked_add(size_of::<SourcePublicationBaselineV1>())
        .and_then(|n| n.checked_add(4 * size_of::<usize>()))
        .ok_or(Error::Limit)
}
impl SourceCoordinator {
    pub(super) fn source_command_ordinary_gate_v1(&self) -> Result<(), Error> {
        if let State::Active(_) = &self.publication_pool()?.commands {
            let t = self.source_command_table_v1()?;
            if t.rows
                .iter()
                .flatten()
                .any(|c| c.life.phase == PhaseV1::Faulted)
            {
                return Err(Error::Faulted);
            }
        }
        Ok(())
    }
    fn source_command_table_v1(&self) -> Result<&Table, Error> {
        let p = self.publication_pool()?;
        let t = p.commands.table(&self.retention)?;
        t.receipts
            .validate_claims(&p.owner, &t.rows, &self.retention)?;
        Ok(t)
    }
    fn source_command_v1(
        &self,
        id: &SourcePublicationCommandIdV1,
        scope: &SourceReadScopeV1,
    ) -> Result<&Command, Error> {
        let p = self.publication_pool()?;
        if !Arc::ptr_eq(&p.owner, &id.owner) {
            return Err(Error::Unavailable);
        }
        let c = self
            .source_command_table_v1()?
            .rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        if c.scope != *scope {
            return Err(Error::Unavailable);
        }
        Ok(c)
    }
    fn source_command_mut_v1(&mut self, id: &SourcePublicationCommandIdV1) -> &mut Command {
        let super::State::Active(p) = &mut self.publication_streams else {
            unreachable!()
        };
        let State::Active(t) = &mut p.commands else {
            unreachable!()
        };
        t.rows[id.slot]
            .as_mut()
            .expect("validated original source command")
    }
    pub fn configure_source_publication_commands_v1(
        &mut self,
        limits: SourcePublicationCommandLimitsV1,
        control: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            let p = self.publication_pool()?;
            if !matches!(p.commands, State::Unconfigured)
                || control.owner_control_allowed != Some(true)
                || control.resources_ready != Some(true)
            {
                return Err(Error::Unavailable);
            }
            if limits.max_commands == 0
                || limits.max_commands > limits.max_issued
                || largest_result(p.limits.max_projection_bytes)? > limits.max_result_bytes
                || p.limits.max_reply_bytes < size_of::<SourcePublicationCommandReplyV1>()
            {
                return Err(Error::Limit);
            }
            let charge = limits.required_pool_bytes()?;
            self.retention.check(charge).map_err(parent)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(limits.max_issued)
                .map_err(|_| Error::Capacity)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            let super::State::Active(p) = &mut self.publication_streams else {
                unreachable!()
            };
            p.commands = State::Active(Box::new(Table {
                limits,
                rows,
                reservation,
                charge,
                receipts: receipts::State::default(),
                #[cfg(test)]
                fault_after_native: false,
            }));
            Ok(())
        })();
        Self::finish_publication(
            r,
            Boundary::SourcePublicationCommandConfiguration,
            result,
            timing.finish(),
        )
    }
    pub fn issue_source_publication_command_v1(
        &mut self,
        operation: &SourcePublicationCommandOperationV1,
        scope: &SourceReadScopeV1,
        f: &TrustedSourcePublicationFactsV1<'_>,
        owner_control_allowed: Option<bool>,
        r: &mut Recorder,
    ) -> Result<SourcePublicationCommandIdV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            self.source_command_ordinary_gate_v1()?;
            let p = self.publication_pool()?;
            let t = self.source_command_table_v1()?;
            if self.source_facade_busy
                || owner_control_allowed != Some(true)
                || f.original_scope != scope
                || f.current.scope != scope
                || !ordinary(f)
                || !ready(f)
                || f.baseline_allowed != Some(true)
                || scope.context.authority != p.authority
                || scope.context.engine != p.engine
                || scope.policy.is_empty()
                || settings::basis::labels(&scope.context)
                    .into_iter()
                    .any(str::is_empty)
            {
                return Err(Error::Unavailable);
            }
            self.document(&scope.document)
                .map_err(|_| Error::Unavailable)?;
            if source_context::scope_bytes(scope).map_err(source_error)?
                > t.limits.max_context_bytes.min(p.limits.max_context_bytes)
                || t.rows.len() >= t.limits.max_issued
                || t.rows.iter().flatten().count() >= t.limits.max_commands
            {
                return Err(Error::Limit);
            }
            let generation = match operation {
                SourcePublicationCommandOperationV1::Establish => None,
                SourcePublicationCommandOperationV1::Resynchronize { stream } => {
                    let row = self.publication_record(stream)?;
                    if row.scope != *scope || row.phase == Phase::Stopped {
                        return Err(Error::Unavailable);
                    }
                    Some(row.generation.clone())
                }
            };
            let root = self.native_root_identity().map_err(|_| Error::Faulted)?;
            let scope = SourceReadScopeV1 {
                document: scope.document.clone(),
                policy: owned(&scope.policy)?,
                context: settings::basis::copy_context(&scope.context)
                    .map_err(|_| Error::Capacity)?,
            };
            let id = SourcePublicationCommandIdV1 {
                owner: p.owner.clone(),
                slot: t.rows.len(),
            };
            let c = Command {
                scope,
                operation: operation.clone(),
                root,
                generation,
                life: Lifecycle::issued(),
            };
            let super::State::Active(p) = &mut self.publication_streams else {
                unreachable!()
            };
            let State::Active(t) = &mut p.commands else {
                unreachable!()
            };
            t.rows.push(Some(c));
            Ok(id)
        })();
        Self::finish_publication(
            r,
            Boundary::SourcePublicationCommandIssuance,
            result,
            timing.finish(),
        )
    }
    pub fn request_source_publication_command_v1(
        &mut self,
        id: &SourcePublicationCommandIdV1,
        q: &SourcePublicationCommandRequestV1<'_>,
        r: &mut Recorder,
    ) -> Result<SourcePublicationCommandReplyV1, Error> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let c = self.source_command_v1(id, q.scope)?;
            if c.operation != *q.operation || q.owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            if q.max_reply_bytes
                .min(self.publication_pool()?.limits.max_reply_bytes)
                < size_of::<SourcePublicationCommandReplyV1>()
            {
                return Err(Error::Limit);
            }
            if c.life.phase == PhaseV1::Settled {
                return Ok(SourcePublicationCommandReplyV1::ObserveExisting {
                    command: id.clone(),
                });
            }
            if c.life.phase != PhaseV1::Issued {
                return Err(Error::Unavailable);
            }
            self.ensure_operational().map_err(|_| Error::Faulted)?;
            if matches!(q.operation, SourcePublicationCommandOperationV1::Establish) {
                self.ensure_admission_unfenced().map_err(admission_error)?;
            }
            self.source_command_ordinary_gate_v1()?;
            let f = q.current.ok_or(Error::Unavailable)?;
            if self.source_facade_busy
                || f.original_scope != q.scope
                || f.current.scope != q.scope
                || !ordinary(f)
                || !ready(f)
                || f.baseline_allowed != Some(true)
                || c.root != self.native_root_identity().map_err(|_| Error::Faulted)?
            {
                return Err(Error::Unavailable);
            }
            let expected_stream = match &c.operation {
                SourcePublicationCommandOperationV1::Establish => SourcePublicationStreamIdV1 {
                    owner: self.publication_pool()?.owner.clone(),
                    slot: self.publication_pool()?.rows.len(),
                },
                SourcePublicationCommandOperationV1::Resynchronize { stream } => {
                    let row = self.publication_record(stream)?;
                    if row.phase == Phase::Stopped
                        || row.scope != *q.scope
                        || Some(&row.generation) != c.generation.as_ref()
                    {
                        return Err(Error::Unavailable);
                    }
                    stream.clone()
                }
            };
            let serial = match &c.generation {
                None => 0,
                Some(g) => g.serial.checked_add(1).ok_or(Error::Limit)?,
            };
            let p = self.publication_pool()?;
            if p.issued >= p.limits.max_issued
                || matches!(q.operation, SourcePublicationCommandOperationV1::Establish)
                    && p.rows.iter().flatten().count() >= p.limits.max_streams
            {
                return Err(Error::Limit);
            }
            let projection_bytes = size_of::<SourcePublicationProjectionV1>()
                .checked_add(
                    self.document(&q.scope.document)
                        .map_err(|_| Error::Unavailable)?
                        .source
                        .text()
                        .len(),
                )
                .ok_or(Error::Limit)?;
            if projection_bytes > p.limits.max_projection_bytes
                || projection_bytes
                    .checked_add(size_of::<SourcePublicationDeliveryV1>())
                    .is_none_or(|n| n > p.limits.max_reply_bytes.min(f.max_reply_bytes))
                || largest_result(projection_bytes)?
                    > self.source_command_table_v1()?.limits.max_result_bytes
            {
                return Err(Error::Limit);
            }
            self.source_command_mut_v1(id).life.begin();
            let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let native = match q.operation {
                    SourcePublicationCommandOperationV1::Establish => {
                        self.open_source_publication_stream_v1(q.scope, f, r)
                    }
                    SourcePublicationCommandOperationV1::Resynchronize { stream } => {
                        self.resync_source_publication_stream_v1(stream, q.scope, f, r)
                    }
                };
                #[cfg(test)]
                {
                    assert!(
                        !self.source_command_table_v1().unwrap().fault_after_native,
                        "injected source command acknowledgment interruption"
                    );
                }
                if let Ok(b) = &native {
                    let row = self
                        .publication_record(&b.stream)
                        .expect("native source command result owner");
                    assert!(
                        b.stream == expected_stream
                            && b.generation.stream == expected_stream
                            && b.generation.serial == serial
                            && row.scope == *q.scope
                            && row.generation == b.generation
                            && row
                                .baseline
                                .as_ref()
                                .is_some_and(|p| Arc::ptr_eq(p, &b.projection)),
                        "invalid native source command result"
                    );
                }
                native.map(Arc::new)
            }));
            match caught {
                Ok(native) => {
                    self.source_command_mut_v1(id).life.settle(native);
                    Ok(SourcePublicationCommandReplyV1::Settled {
                        command: id.clone(),
                    })
                }
                Err(p) => {
                    self.source_command_mut_v1(id).life.fault();
                    std::panic::resume_unwind(p)
                }
            }
        }));
        let result = match caught {
            Ok(result) => result,
            Err(p) => {
                r.record(Observation {
                    boundary: Boundary::SourcePublicationCommandRequest,
                    disposition: Disposition::Unknown,
                    service_time: timing.finish(),
                });
                std::panic::resume_unwind(p)
            }
        };
        Self::finish_publication(
            r,
            Boundary::SourcePublicationCommandRequest,
            result,
            timing.finish(),
        )
    }
    pub fn source_publication_command_facts_v1(
        &self,
        id: &SourcePublicationCommandIdV1,
        scope: &SourceReadScopeV1,
        r: &mut Recorder,
    ) -> Result<SourcePublicationCommandFactsV1<'_>, Error> {
        let timing = r.begin_service_timing();
        let result = self
            .source_command_v1(id, scope)
            .map(|c| SourcePublicationCommandFactsV1 {
                phase: c.life.phase,
                native_started: c.life.started,
                operation: &c.operation,
                as_observed: c.life.result.as_ref(),
            });
        Self::finish_publication(
            r,
            Boundary::SourcePublicationCommandInspection,
            result,
            timing.finish(),
        )
    }
    pub fn retire_source_publication_command_v1(
        &mut self,
        id: &SourcePublicationCommandIdV1,
        control: &SourceReadContextControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let c = self.source_command_v1(id, control.scope)?;
            if self.source_command_table_v1()?.receipts.command_claimed(id) {
                return Err(Error::Unavailable);
            }
            if [
                control.owner_control_allowed,
                control.retention_released,
                control.delivery_drained,
            ]
            .into_iter()
            .any(|v| v != Some(true))
                || matches!(c.life.phase, PhaseV1::Executing | PhaseV1::Faulted)
            {
                return Err(Error::Unavailable);
            }
            let super::State::Active(p) = &mut self.publication_streams else {
                unreachable!()
            };
            let State::Active(t) = &mut p.commands else {
                unreachable!()
            };
            t.rows[id.slot] = None;
            Ok(())
        })();
        Self::finish_publication(
            r,
            Boundary::SourcePublicationCommandRetirement,
            result,
            timing.finish(),
        )
    }
    pub fn retire_source_publication_commands_v1(
        &mut self,
        control: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let t = self.source_command_table_v1()?;
            if !t.receipts.closed()
                || t.rows.iter().any(Option::is_some)
                || [
                    control.owner_control_allowed,
                    control.retention_released,
                    control.delivery_drained,
                ]
                .into_iter()
                .any(|v| v != Some(true))
            {
                return Err(Error::Unavailable);
            }
            let super::State::Active(p) = &mut self.publication_streams else {
                unreachable!()
            };
            let State::Active(t) = &mut p.commands else {
                unreachable!()
            };
            self.retention.release(&mut t.reservation).map_err(parent)?;
            p.commands = State::Closed {
                issued: t.rows.len(),
            };
            Ok(())
        })();
        Self::finish_publication(
            r,
            Boundary::SourcePublicationCommandRetirement,
            result,
            timing.finish(),
        )
    }
    pub fn source_publication_command_usage_v1(
        &self,
    ) -> Result<SourcePublicationCommandUsageV1, Error> {
        match &self.publication_pool()?.commands {
            State::Unconfigured => Ok(SourcePublicationCommandUsageV1::default()),
            State::Closed { issued } => Ok(SourcePublicationCommandUsageV1 {
                issued_lifetime: *issued,
                closed: true,
                ..Default::default()
            }),
            State::Active(_) => {
                let t = self.source_command_table_v1()?;
                Ok(SourcePublicationCommandUsageV1 {
                    commands: t.rows.iter().flatten().count(),
                    issued_lifetime: t.rows.len(),
                    reserved_pool_bytes: t.charge,
                    closed: false,
                })
            }
        }
    }
}
