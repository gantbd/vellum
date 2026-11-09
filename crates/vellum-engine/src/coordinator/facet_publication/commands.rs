//! Original named-facet commands, funded and retained by the native publisher.
use super::*;
mod receipts;
use crate::coordinator::publication_command::Lifecycle;
pub use receipts::*;
mod ingress;
#[cfg(test)]
pub(in crate::coordinator) use ingress::tests::{command_envelope, receipt_envelope};
pub use ingress::*;
#[cfg(test)]
pub(in crate::coordinator) use receipts::tests::{
    facts as facet_receipt_test_facts, open as facet_receipt_test_open,
    setup as facet_receipt_test_setup, setup_with_budget as facet_receipt_test_setup_with_budget,
};
type PhaseV1 = PublicationCommandPhaseV1;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FacetPublicationCommandLimitsV1 {
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
impl FacetPublicationCommandLimitsV1 {
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
pub struct FacetPublicationCommandIdV1 {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for FacetPublicationCommandIdV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for FacetPublicationCommandIdV1 {}
#[derive(Clone, PartialEq, Eq)]
pub enum FacetPublicationCommandOperationV1 {
    Establish,
    Resynchronize { stream: FacetPublicationStreamIdV1 },
}
#[derive(Clone, PartialEq, Eq)]
pub enum FacetPublicationCommandReplyV1 {
    Settled {
        command: FacetPublicationCommandIdV1,
    },
    ObserveExisting {
        command: FacetPublicationCommandIdV1,
    },
}
pub struct FacetPublicationCommandRequestV1<'a> {
    pub scope: &'a FacetPublicationScopeV1,
    pub operation: &'a FacetPublicationCommandOperationV1,
    /// Absent on identity-only protected retry. No ordinary permission is inferred.
    pub current: Option<&'a TrustedFacetPublicationFactsV1<'a>>,
    pub owner_control_allowed: Option<bool>,
    pub max_reply_bytes: usize,
}
/// Privileged host seam, not a recipient F06 payload or disclosure authorization.
pub struct FacetPublicationCommandFactsV1<'a> {
    pub phase: PublicationCommandPhaseV1,
    pub native_started: bool,
    pub operation: &'a FacetPublicationCommandOperationV1,
    pub as_observed: Option<&'a Result<Arc<FacetPublicationBaselineV1>, Error>>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FacetPublicationCommandUsageV1 {
    pub commands: usize,
    pub issued_lifetime: usize,
    pub reserved_pool_bytes: usize,
    pub closed: bool,
}
redacted!(
    FacetPublicationCommandIdV1,
    FacetPublicationCommandOperationV1,
    FacetPublicationCommandReplyV1,
    FacetPublicationCommandRequestV1<'_>,
    FacetPublicationCommandFactsV1<'_>
);

struct Command {
    scope: FacetPublicationScopeV1,
    operation: FacetPublicationCommandOperationV1,
    root: root_identity::NativeRootIdentity,
    generation: Option<FacetPublicationGenerationV1>,
    life: Lifecycle<Result<Arc<FacetPublicationBaselineV1>, Error>>,
}
pub(super) struct Table {
    limits: FacetPublicationCommandLimitsV1,
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
                    .filter(|c| c.scope.source.document == *document)
                    .count()
                    .checked_add(t.receipts.document_pins(document, ledger)?)
                    .ok_or(Error::Faulted)
            }
        }
    }
}
impl State {
    pub(super) fn shutdown_summary(
        &self,
        o: &SourceCoordinator,
        b: &mut crate::coordinator::shutdown::inventory::Builder,
        owner: &Arc<()>,
    ) -> Result<(usize, usize), CoordinatorError> {
        let Self::Active(_) = self else {
            return Ok((0, 0));
        };
        let t = self
            .table(&o.retention)
            .map_err(|_| CoordinatorError::Faulted)?;
        t.receipts
            .validate_claims(owner, &t.rows, &o.retention)
            .map_err(|_| CoordinatorError::Faulted)?;
        let retained = b.slots(&t.rows)?;
        for c in t.rows.iter().flatten() {
            b.step()?;
            o.record_shutdown_pool_document(
                b,
                ShutdownOwnerRoleV1::FacetPublication,
                &c.scope.source.document,
            )?;
        }
        let (contexts, charge) = t.receipts.shutdown_summary(o, b)?;
        Ok((
            retained
                .checked_add(contexts)
                .ok_or(CoordinatorError::Faulted)?,
            t.charge
                .checked_add(charge)
                .ok_or(CoordinatorError::Faulted)?,
        ))
    }
}
fn largest_result(projection_bytes: usize) -> Result<usize, Error> {
    projection_bytes
        .checked_add(size_of::<FacetPublicationBaselineV1>())
        .and_then(|n| n.checked_add(4 * size_of::<usize>()))
        .ok_or(Error::Limit)
}
impl SourceCoordinator {
    pub(super) fn facet_command_ordinary_gate_v1(&self) -> Result<(), Error> {
        if let State::Active(_) = &self.facet_publication_pool()?.commands {
            let t = self.facet_command_table_v1()?;
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
    fn facet_command_table_v1(&self) -> Result<&Table, Error> {
        let p = self.facet_publication_pool()?;
        let t = p.commands.table(&self.retention)?;
        t.receipts
            .validate_claims(&p.owner, &t.rows, &self.retention)?;
        Ok(t)
    }
    fn facet_command_v1(
        &self,
        id: &FacetPublicationCommandIdV1,
        scope: &FacetPublicationScopeV1,
    ) -> Result<&Command, Error> {
        let p = self.facet_publication_pool()?;
        if !Arc::ptr_eq(&p.owner, &id.owner) {
            return Err(Error::Unavailable);
        }
        let c = self
            .facet_command_table_v1()?
            .rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        if c.scope != *scope {
            return Err(Error::Unavailable);
        }
        Ok(c)
    }
    fn facet_command_mut_v1(&mut self, id: &FacetPublicationCommandIdV1) -> &mut Command {
        let super::State::Active(p) = &mut self.facet_publication_streams else {
            unreachable!()
        };
        let State::Active(t) = &mut p.commands else {
            unreachable!()
        };
        t.rows[id.slot]
            .as_mut()
            .expect("validated original source command")
    }
    pub fn configure_facet_publication_commands_v1(
        &mut self,
        limits: FacetPublicationCommandLimitsV1,
        control: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            let p = self.facet_publication_pool()?;
            if !matches!(p.commands, State::Unconfigured)
                || control.owner_control_allowed != Some(true)
                || control.resources_ready != Some(true)
            {
                return Err(Error::Unavailable);
            }
            if limits.max_commands == 0
                || limits.max_commands > limits.max_issued
                || largest_result(p.limits.max_projection_bytes)? > limits.max_result_bytes
                || p.limits.max_reply_bytes < size_of::<FacetPublicationCommandReplyV1>()
            {
                return Err(Error::Limit);
            }
            let charge = limits.required_pool_bytes()?;
            self.retention.check(charge).map_err(parent)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(limits.max_issued)
                .map_err(|_| Error::Capacity)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            let super::State::Active(p) = &mut self.facet_publication_streams else {
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
        Self::finish_facet_publication(
            r,
            Boundary::FacetPublicationCommandConfiguration,
            result,
            timing.finish(),
        )
    }
    pub fn issue_facet_publication_command_v1(
        &mut self,
        operation: &FacetPublicationCommandOperationV1,
        scope: &FacetPublicationScopeV1,
        f: &TrustedFacetPublicationFactsV1<'_>,
        owner_control_allowed: Option<bool>,
        r: &mut Recorder,
    ) -> Result<FacetPublicationCommandIdV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_ordinary_admission().map_err(admission_error)?;
            self.facet_command_ordinary_gate_v1()?;
            let p = self.facet_publication_pool()?;
            let t = self.facet_command_table_v1()?;
            if self.source_facade_busy
                || owner_control_allowed != Some(true)
                || f.original_scope != scope
                || f.current.scope != &scope.source
                || !ordinary(f)
                || facet_rights(scope, f) != Some(true)
                || !ready(f)
                || f.baseline_allowed != Some(true)
                || scope.source.context.authority != p.authority
                || scope.source.context.engine != p.engine
                || scope.source.policy.is_empty()
                || settings::basis::labels(&scope.source.context)
                    .into_iter()
                    .any(str::is_empty)
            {
                return Err(Error::Unavailable);
            }
            self.document(&scope.source.document)
                .map_err(|_| Error::Unavailable)?;
            if scope_bytes(scope, p.limits)?
                > t.limits.max_context_bytes.min(p.limits.max_context_bytes)
                || t.rows.len() >= t.limits.max_issued
                || t.rows.iter().flatten().count() >= t.limits.max_commands
            {
                return Err(Error::Limit);
            }
            let generation = match operation {
                FacetPublicationCommandOperationV1::Establish => None,
                FacetPublicationCommandOperationV1::Resynchronize { stream } => {
                    let row = self.facet_publication_record(stream)?;
                    if row.scope != *scope || row.phase == Phase::Stopped {
                        return Err(Error::Unavailable);
                    }
                    Some(row.generation.clone())
                }
            };
            let root = self.native_root_identity().map_err(|_| Error::Faulted)?;
            let scope = copy_scope(scope, p.limits)?;
            let id = FacetPublicationCommandIdV1 {
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
            let super::State::Active(p) = &mut self.facet_publication_streams else {
                unreachable!()
            };
            let State::Active(t) = &mut p.commands else {
                unreachable!()
            };
            t.rows.push(Some(c));
            Ok(id)
        })();
        Self::finish_facet_publication(
            r,
            Boundary::FacetPublicationCommandIssuance,
            result,
            timing.finish(),
        )
    }
    pub fn request_facet_publication_command_v1(
        &mut self,
        id: &FacetPublicationCommandIdV1,
        q: &FacetPublicationCommandRequestV1<'_>,
        r: &mut Recorder,
    ) -> Result<FacetPublicationCommandReplyV1, Error> {
        let timing = r.begin_service_timing();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let c = self.facet_command_v1(id, q.scope)?;
            if c.operation != *q.operation || q.owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            if q.max_reply_bytes
                .min(self.facet_publication_pool()?.limits.max_reply_bytes)
                < size_of::<FacetPublicationCommandReplyV1>()
            {
                return Err(Error::Limit);
            }
            if c.life.phase == PhaseV1::Settled {
                return Ok(FacetPublicationCommandReplyV1::ObserveExisting {
                    command: id.clone(),
                });
            }
            if c.life.phase != PhaseV1::Issued {
                return Err(Error::Unavailable);
            }
            self.ensure_operational().map_err(|_| Error::Faulted)?;
            if matches!(q.operation, FacetPublicationCommandOperationV1::Establish) {
                self.ensure_admission_unfenced().map_err(admission_error)?;
            }
            self.facet_command_ordinary_gate_v1()?;
            let f = q.current.ok_or(Error::Unavailable)?;
            if self.source_facade_busy
                || f.original_scope != q.scope
                || f.current.scope != &q.scope.source
                || !ordinary(f)
                || facet_rights(q.scope, f) != Some(true)
                || !ready(f)
                || f.baseline_allowed != Some(true)
                || c.root != self.native_root_identity().map_err(|_| Error::Faulted)?
            {
                return Err(Error::Unavailable);
            }
            let expected_stream = match &c.operation {
                FacetPublicationCommandOperationV1::Establish => FacetPublicationStreamIdV1 {
                    owner: self.facet_publication_pool()?.owner.clone(),
                    slot: self.facet_publication_pool()?.rows.len(),
                },
                FacetPublicationCommandOperationV1::Resynchronize { stream } => {
                    let row = self.facet_publication_record(stream)?;
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
            let p = self.facet_publication_pool()?;
            if p.issued >= p.limits.max_issued
                || matches!(q.operation, FacetPublicationCommandOperationV1::Establish)
                    && p.rows.iter().flatten().count() >= p.limits.max_streams
            {
                return Err(Error::Limit);
            }
            // Configuration prepays the publisher's largest possible baseline.
            // Let the native kernel build/check it once, not a throwaway preview
            // followed by a second projection allocation/copy.
            self.facet_command_mut_v1(id).life.begin();
            let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let native = match q.operation {
                    FacetPublicationCommandOperationV1::Establish => {
                        self.open_facet_publication_stream_v1(q.scope, f, r)
                    }
                    FacetPublicationCommandOperationV1::Resynchronize { stream } => {
                        self.resync_facet_publication_stream_v1(stream, q.scope, f, r)
                    }
                };
                #[cfg(test)]
                {
                    assert!(
                        !self.facet_command_table_v1().unwrap().fault_after_native,
                        "injected facet command acknowledgment interruption"
                    );
                }
                if let Ok(b) = &native {
                    let row = self
                        .facet_publication_record(&b.stream)
                        .expect("native facet command result owner");
                    assert!(
                        b.stream == expected_stream
                            && b.generation.stream == expected_stream
                            && b.generation.serial == serial
                            && row.scope == *q.scope
                            && row.generation == b.generation
                            && largest_result(
                                b.projection.bytes().expect("native projection bytes")
                            )
                            .is_ok_and(|n| n
                                <= self
                                    .facet_command_table_v1()
                                    .unwrap()
                                    .limits
                                    .max_result_bytes)
                            && row
                                .baseline
                                .as_ref()
                                .is_some_and(|p| Arc::ptr_eq(p, &b.projection)),
                        "invalid native facet command result"
                    );
                }
                native.map(Arc::new)
            }));
            match caught {
                Ok(native) => {
                    self.facet_command_mut_v1(id).life.settle(native);
                    Ok(FacetPublicationCommandReplyV1::Settled {
                        command: id.clone(),
                    })
                }
                Err(p) => {
                    self.facet_command_mut_v1(id).life.fault();
                    std::panic::resume_unwind(p)
                }
            }
        }));
        let result = match caught {
            Ok(result) => result,
            Err(p) => {
                r.record(Observation {
                    boundary: Boundary::FacetPublicationCommandRequest,
                    disposition: Disposition::Unknown,
                    service_time: timing.finish(),
                });
                std::panic::resume_unwind(p)
            }
        };
        Self::finish_facet_publication(
            r,
            Boundary::FacetPublicationCommandRequest,
            result,
            timing.finish(),
        )
    }
    pub fn facet_publication_command_facts_v1(
        &self,
        id: &FacetPublicationCommandIdV1,
        scope: &FacetPublicationScopeV1,
        r: &mut Recorder,
    ) -> Result<FacetPublicationCommandFactsV1<'_>, Error> {
        let timing = r.begin_service_timing();
        let result = self
            .facet_command_v1(id, scope)
            .map(|c| FacetPublicationCommandFactsV1 {
                phase: c.life.phase,
                native_started: c.life.started,
                operation: &c.operation,
                as_observed: c.life.result.as_ref(),
            });
        Self::finish_facet_publication(
            r,
            Boundary::FacetPublicationCommandInspection,
            result,
            timing.finish(),
        )
    }
    pub fn retire_facet_publication_command_v1(
        &mut self,
        id: &FacetPublicationCommandIdV1,
        control: &FacetPublicationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let c = self.facet_command_v1(id, control.scope)?;
            if self.facet_command_table_v1()?.receipts.command_claimed(id) {
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
            let super::State::Active(p) = &mut self.facet_publication_streams else {
                unreachable!()
            };
            let State::Active(t) = &mut p.commands else {
                unreachable!()
            };
            t.rows[id.slot] = None;
            Ok(())
        })();
        Self::finish_facet_publication(
            r,
            Boundary::FacetPublicationCommandRetirement,
            result,
            timing.finish(),
        )
    }
    pub fn retire_facet_publication_commands_v1(
        &mut self,
        control: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let t = self.facet_command_table_v1()?;
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
            let super::State::Active(p) = &mut self.facet_publication_streams else {
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
        Self::finish_facet_publication(
            r,
            Boundary::FacetPublicationCommandRetirement,
            result,
            timing.finish(),
        )
    }
    pub fn facet_publication_command_usage_v1(
        &self,
    ) -> Result<FacetPublicationCommandUsageV1, Error> {
        match &self.facet_publication_pool()?.commands {
            State::Unconfigured => Ok(FacetPublicationCommandUsageV1::default()),
            State::Closed { issued } => Ok(FacetPublicationCommandUsageV1 {
                issued_lifetime: *issued,
                closed: true,
                ..Default::default()
            }),
            State::Active(_) => {
                let t = self.facet_command_table_v1()?;
                Ok(FacetPublicationCommandUsageV1 {
                    commands: t.rows.iter().flatten().count(),
                    issued_lifetime: t.rows.len(),
                    reserved_pool_bytes: t.charge,
                    closed: false,
                })
            }
        }
    }
}
