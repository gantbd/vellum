//! Complete original mixed-projection receipts; control is not disclosure authority.
use super::*;
use std::sync::Weak;
type NativeResult = Result<Arc<CollectionPublicationBaselineV1>, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollectionPublicationReceiptLimitsV1 {
    pub max_contexts: usize,
    pub max_issued: usize,
    pub max_context_bytes: usize,
    pub max_reply_bytes: usize,
    pub max_pool_bytes: usize,
}
impl CollectionPublicationReceiptLimitsV1 {
    pub fn required_pool_bytes(self) -> Result<usize, Error> {
        size_of::<Pool>()
            .checked_add(
                self.max_issued
                    .checked_mul(size_of::<Option<Context>>())
                    .ok_or(Error::Limit)?,
            )
            .and_then(|n| {
                n.checked_add(
                    self.max_contexts
                        .checked_mul(self.max_context_bytes.checked_add(self.max_reply_bytes)?)?,
                )
            })
            .filter(|n| *n <= self.max_pool_bytes)
            .ok_or(Error::Limit)
    }
}
#[derive(Clone)]
pub struct CollectionPublicationReceiptContextIdV1 {
    owner: Arc<()>,
    slot: usize,
}
impl PartialEq for CollectionPublicationReceiptContextIdV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for CollectionPublicationReceiptContextIdV1 {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollectionPublicationReceiptCeilingV1 {
    pub complete_original_result: bool,
}
/// Trusted host's current facts about receipt authority, not document edit liveness.
/// `authorization_current` affirms policy truth for this exact supplied basis at
/// this call; native-root equality alone is not a policy grant or freshness lease.
pub struct TrustedCollectionPublicationReceiptFactsV1<'a> {
    pub scope: &'a CollectionPublicationScopeV1,
    pub authorized: Option<bool>,
    pub scope_current: Option<bool>,
    pub continuous: Option<bool>,
    pub live: Option<bool>,
    pub not_stopped: Option<bool>,
    pub retained: Option<bool>,
    pub receipt_grant_active: Option<bool>,
    pub complete_result_allowed: Option<bool>,
    pub authorization_current: Option<bool>,
    pub resources_ready: Option<bool>,
    pub reply_ready: Option<bool>,
    pub transfer_ready: Option<bool>,
    pub max_reply_bytes: usize,
}
impl TrustedCollectionPublicationReceiptFactsV1<'_> {
    fn authority(&self) -> [Option<bool>; 9] {
        [
            self.authorized,
            self.scope_current,
            self.continuous,
            self.live,
            self.not_stopped,
            self.retained,
            self.receipt_grant_active,
            self.complete_result_allowed,
            self.authorization_current,
        ]
    }
    fn ready(&self) -> bool {
        [self.resources_ready, self.reply_ready, self.transfer_ready]
            .into_iter()
            .all(|v| v == Some(true))
    }
}
enum ResultIdentity {
    Baseline(Weak<CollectionPublicationBaselineV1>),
    NativeError(Error),
}
impl ResultIdentity {
    fn capture(result: &NativeResult) -> Self {
        match result {
            Ok(b) => Self::Baseline(Arc::downgrade(b)),
            Err(e) => Self::NativeError(*e),
        }
    }
    fn matches(&self, result: &NativeResult) -> bool {
        match (self, result) {
            (Self::Baseline(a), Ok(b)) => Weak::ptr_eq(a, &Arc::downgrade(b)),
            (Self::NativeError(a), Err(b)) => a == b,
            _ => false,
        }
    }
}
/// Correlation only, never a grant or managed hold. Weak allocation identity
/// cannot retain the original payload after its real owners discharge it.
pub struct CollectionPublicationReceiptAuthorizationBasisIdV1 {
    context: Option<CollectionPublicationReceiptContextIdV1>,
    command: CollectionPublicationCommandIdV1,
    result: ResultIdentity,
    root: root_identity::NativeRootIdentity,
}
/// Privileged exact historical projection input, not ordinary recipient disclosure.
pub struct CollectionPublicationReceiptAuthorizationBasisV1<'a> {
    pub basis: CollectionPublicationReceiptAuthorizationBasisIdV1,
    pub original_projection: Option<&'a CollectionPublicationProjectionV1>,
    pub native_error: Option<Error>,
}
pub struct TrustedCollectionPublicationReceiptHistoricalFactsV1<'a> {
    pub basis: &'a CollectionPublicationReceiptAuthorizationBasisIdV1,
    pub current: &'a TrustedCollectionPublicationReceiptFactsV1<'a>,
}
pub struct CollectionPublicationReceiptReplyV1 {
    pub command: CollectionPublicationCommandIdV1,
    pub result: NativeResult,
}
impl CollectionPublicationReceiptReplyV1 {
    pub fn status(&self) -> &'static str {
        "operation-receipt"
    }
    pub fn facet(&self) -> &'static str {
        "collection-baseline"
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CollectionPublicationReceiptUsageV1 {
    pub contexts: usize,
    pub stopped: usize,
    pub issued_lifetime: usize,
    pub reserved_pool_bytes: usize,
    pub closed: bool,
}
redacted!(
    CollectionPublicationReceiptContextIdV1,
    TrustedCollectionPublicationReceiptFactsV1<'_>,
    CollectionPublicationReceiptAuthorizationBasisIdV1,
    CollectionPublicationReceiptAuthorizationBasisV1<'_>,
    TrustedCollectionPublicationReceiptHistoricalFactsV1<'_>,
    CollectionPublicationReceiptReplyV1
);
struct Context {
    scope: CollectionPublicationScopeV1,
    command: CollectionPublicationCommandIdV1,
    result: NativeResult,
    ceiling: CollectionPublicationReceiptCeilingV1,
    stopped: bool,
}
pub(super) struct Pool {
    limits: CollectionPublicationReceiptLimitsV1,
    rows: Vec<Option<Context>>,
    reservation: retention::Reservation,
    charge: usize,
}
#[derive(Default)]
pub(super) enum State {
    #[default]
    Unconfigured,
    Active(Box<Pool>),
    Closed {
        issued: usize,
    },
}
impl State {
    pub(super) fn closed(&self) -> bool {
        !matches!(self, Self::Active(_))
    }
    fn pool(&self, ledger: &retention::RetentionState) -> Result<&Pool, Error> {
        let Self::Active(p) = self else {
            return Err(Error::Unavailable);
        };
        ledger
            .validate_exact(&p.reservation, p.charge)
            .map_err(parent)?;
        if p.limits.required_pool_bytes().map_err(|_| Error::Faulted)? != p.charge
            || p.rows.capacity() < p.limits.max_issued
            || p.rows.len() > p.limits.max_issued
            || p.rows.iter().flatten().count() > p.limits.max_contexts
        {
            return Err(Error::Faulted);
        }
        Ok(p)
    }
    pub(super) fn validate_claims(
        &self,
        owner: &Arc<()>,
        commands: &[Option<Command>],
        ledger: &retention::RetentionState,
    ) -> Result<(), Error> {
        if matches!(self, Self::Active(_)) {
            for c in self.pool(ledger)?.rows.iter().flatten() {
                let original = commands
                    .get(c.command.slot)
                    .and_then(Option::as_ref)
                    .ok_or(Error::Faulted)?;
                if !Arc::ptr_eq(owner, &c.command.owner)
                    || original.scope != c.scope
                    || original.life.phase != PhaseV1::Settled
                    || !c.ceiling.complete_original_result
                    || !original
                        .life
                        .result
                        .as_ref()
                        .is_some_and(|r| ResultIdentity::capture(r).matches(&c.result))
                {
                    return Err(Error::Faulted);
                }
            }
        }
        Ok(())
    }
    pub(super) fn command_claimed(&self, id: &CollectionPublicationCommandIdV1) -> bool {
        matches!(self, Self::Active(p) if p.rows.iter().flatten().any(|c| c.command == *id))
    }
    pub(super) fn document_pins(
        &self,
        document: &DocumentId,
        ledger: &retention::RetentionState,
    ) -> Result<usize, Error> {
        match self {
            Self::Active(_) => Ok(self
                .pool(ledger)?
                .rows
                .iter()
                .flatten()
                .filter(|c| c.scope.source.document == *document)
                .count()),
            _ => Ok(0),
        }
    }
}
fn reply_bytes(result: &NativeResult) -> Result<usize, Error> {
    match result {
        Err(_) => Ok(size_of::<CollectionPublicationReceiptReplyV1>()),
        Ok(b) => largest_result(b.projection.bytes()?)?
            .checked_add(size_of::<CollectionPublicationReceiptReplyV1>())
            .ok_or(Error::Limit),
    }
}
impl State {
    pub(super) fn shutdown_summary(
        &self,
        o: &SourceCoordinator,
        b: &mut crate::coordinator::shutdown::inventory::Builder,
    ) -> Result<(usize, usize), CoordinatorError> {
        let Self::Active(_) = self else {
            return Ok((0, 0));
        };
        let p = self
            .pool(&o.retention)
            .map_err(|_| CoordinatorError::Faulted)?;
        let retained = b.slots(&p.rows)?;
        for c in p.rows.iter().flatten() {
            b.step()?;
            o.record_shutdown_pool_document(
                b,
                ShutdownOwnerRoleV1::CollectionPublication,
                &c.scope.source.document,
            )?;
        }
        Ok((retained, p.charge))
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn collection_publication_discovery_source_v1(
        &self,
        authority: &str,
        engine: &str,
    ) -> Result<(), Error> {
        let p = self.collection_publication_pool()?;
        if p.authority != authority || p.engine != engine {
            return Err(Error::Unavailable);
        }
        self.collection_command_table_v1()?;
        self.collection_receipt_pool_v1()?;
        Ok(())
    }
    pub(in crate::coordinator) fn validate_collection_publication_receipt_reply_v1(
        &mut self,
        context: &CollectionPublicationReceiptContextIdV1,
        command: &CollectionPublicationCommandIdV1,
        scope: &CollectionPublicationScopeV1,
        reply: &CollectionPublicationReceiptReplyV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let c = self.collection_receipt_context_v1(context, command, scope)?;
            if c.stopped
                || reply.command != *command
                || !ResultIdentity::capture(&c.result).matches(&reply.result)
                || reply_bytes(&reply.result)?
                    > self.collection_receipt_pool_v1()?.limits.max_reply_bytes
            {
                self.collection_receipt_mut_v1(context).stopped = true;
                return Err(Error::Faulted);
            }
            Ok(())
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationReceiptResultValidation,
            result,
            timing.finish(),
        )
    }
    fn validate_collection_receipt_basis_v1(
        &self,
        basis: &CollectionPublicationReceiptAuthorizationBasisIdV1,
        context: Option<&CollectionPublicationReceiptContextIdV1>,
        command: &CollectionPublicationCommandIdV1,
        original: &NativeResult,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if basis.context.as_ref() != context
                || basis.command != *command
                || !basis.result.matches(original)
                || basis.root != self.native_root_identity().map_err(|_| Error::Faulted)?
            {
                return Err(Error::Unavailable);
            }
            Ok(())
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationReceiptBasisValidation,
            result,
            timing.finish(),
        )
    }
    fn collection_receipt_pool_v1(&self) -> Result<&Pool, Error> {
        self.collection_command_table_v1()?
            .receipts
            .pool(&self.retention)
    }
    fn collection_receipt_context_v1(
        &self,
        id: &CollectionPublicationReceiptContextIdV1,
        command: &CollectionPublicationCommandIdV1,
        scope: &CollectionPublicationScopeV1,
    ) -> Result<&Context, Error> {
        if !Arc::ptr_eq(&self.collection_publication_pool()?.owner, &id.owner) {
            return Err(Error::Unavailable);
        }
        let c = self
            .collection_receipt_pool_v1()?
            .rows
            .get(id.slot)
            .and_then(Option::as_ref)
            .ok_or(Error::Unavailable)?;
        if c.command != *command || c.scope != *scope {
            return Err(Error::Unavailable);
        }
        Ok(c)
    }
    fn collection_receipt_mut_v1(
        &mut self,
        id: &CollectionPublicationReceiptContextIdV1,
    ) -> &mut Context {
        let super::super::State::Active(p) = &mut self.collection_publication_streams else {
            unreachable!()
        };
        let super::State::Active(t) = &mut p.commands else {
            unreachable!()
        };
        let State::Active(p) = &mut t.receipts else {
            unreachable!()
        };
        p.rows[id.slot].as_mut().expect("validated receipt context")
    }
    pub fn configure_collection_publication_receipts_v1(
        &mut self,
        limits: CollectionPublicationReceiptLimitsV1,
        control: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(|_| Error::Faulted)?;
            self.collection_publication_admission_v1()?;
            self.collection_command_ordinary_gate_v1()?;
            if control.owner_control_allowed != Some(true)
                || control.resources_ready != Some(true)
                || !matches!(
                    self.collection_command_table_v1()?.receipts,
                    State::Unconfigured
                )
            {
                return Err(Error::Unavailable);
            }
            let largest = largest_result(
                self.collection_publication_pool()?
                    .limits
                    .max_projection_bytes,
            )?
            .checked_add(size_of::<CollectionPublicationReceiptReplyV1>())
            .ok_or(Error::Limit)?;
            if limits.max_contexts == 0
                || limits.max_contexts > limits.max_issued
                || limits.max_reply_bytes < largest
            {
                return Err(Error::Limit);
            }
            let charge = limits.required_pool_bytes()?;
            self.retention.check(charge).map_err(parent)?;
            let mut rows = Vec::new();
            rows.try_reserve_exact(limits.max_issued)
                .map_err(|_| Error::Capacity)?;
            let reservation = self.retention.reserve(charge).map_err(parent)?;
            let super::super::State::Active(p) = &mut self.collection_publication_streams else {
                unreachable!()
            };
            let super::State::Active(t) = &mut p.commands else {
                unreachable!()
            };
            t.receipts = State::Active(Box::new(Pool {
                limits,
                rows,
                reservation,
                charge,
            }));
            Ok(())
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationReceiptConfiguration,
            result,
            timing.finish(),
        )
    }
    fn collection_receipt_basis_v1(
        &self,
        context: Option<&CollectionPublicationReceiptContextIdV1>,
        command: &CollectionPublicationCommandIdV1,
        scope: &CollectionPublicationScopeV1,
        r: &mut Recorder,
    ) -> Result<CollectionPublicationReceiptAuthorizationBasisV1<'_>, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let c = self.collection_command_v1(command, scope)?;
            if c.life.phase != PhaseV1::Settled {
                return Err(Error::Unavailable);
            }
            let result = c.life.result.as_ref().ok_or(Error::Faulted)?;
            if let Some(id) = context {
                let receipt = self.collection_receipt_context_v1(id, command, scope)?;
                if receipt.stopped {
                    return Err(Error::Unavailable);
                }
            }
            Ok(CollectionPublicationReceiptAuthorizationBasisV1 {
                basis: CollectionPublicationReceiptAuthorizationBasisIdV1 {
                    context: context.cloned(),
                    command: command.clone(),
                    result: ResultIdentity::capture(result),
                    root: self.native_root_identity().map_err(|_| Error::Faulted)?,
                },
                original_projection: result.as_ref().ok().map(|b| b.projection.as_ref()),
                native_error: result.as_ref().err().copied(),
            })
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationReceiptBasis,
            result,
            timing.finish(),
        )
    }
    /// Privileged original-result input for acquisition authorization.
    pub fn collection_publication_command_receipt_basis_v1(
        &self,
        command: &CollectionPublicationCommandIdV1,
        scope: &CollectionPublicationScopeV1,
        r: &mut Recorder,
    ) -> Result<CollectionPublicationReceiptAuthorizationBasisV1<'_>, Error> {
        self.collection_receipt_basis_v1(None, command, scope, r)
    }
    /// Privileged original-result input for this exact receipt's current authorization.
    pub fn collection_publication_receipt_basis_v1(
        &self,
        context: &CollectionPublicationReceiptContextIdV1,
        command: &CollectionPublicationCommandIdV1,
        scope: &CollectionPublicationScopeV1,
        r: &mut Recorder,
    ) -> Result<CollectionPublicationReceiptAuthorizationBasisV1<'_>, Error> {
        self.collection_receipt_basis_v1(Some(context), command, scope, r)
    }
    pub fn open_collection_publication_receipt_v1(
        &mut self,
        command: &CollectionPublicationCommandIdV1,
        scope: &CollectionPublicationScopeV1,
        ceiling: CollectionPublicationReceiptCeilingV1,
        f: &TrustedCollectionPublicationReceiptHistoricalFactsV1<'_>,
        admission_allowed: Option<bool>,
        r: &mut Recorder,
    ) -> Result<CollectionPublicationReceiptContextIdV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.ensure_operational().map_err(|_| Error::Faulted)?;
            self.collection_publication_admission_v1()?;
            self.collection_command_ordinary_gate_v1()?;
            let c = self.collection_command_v1(command, scope)?;
            let original = c
                .life
                .result
                .as_ref()
                .filter(|_| c.life.phase == PhaseV1::Settled)
                .ok_or(Error::Unavailable)?;
            self.validate_collection_receipt_basis_v1(f.basis, None, command, original, r)?;
            if f.current.scope != scope
                || !ceiling.complete_original_result
                || admission_allowed != Some(true)
                || self.source_facade_busy
                || f.current.authority().into_iter().any(|v| v != Some(true))
                || !f.current.ready()
            {
                return Err(Error::Unavailable);
            }
            let p = self.collection_receipt_pool_v1()?;
            if p.rows.len() >= p.limits.max_issued
                || p.rows.iter().flatten().count() >= p.limits.max_contexts
                || scope_bytes(scope)? > p.limits.max_context_bytes
                || reply_bytes(original)? > p.limits.max_reply_bytes.min(f.current.max_reply_bytes)
            {
                return Err(Error::Limit);
            }
            let id = CollectionPublicationReceiptContextIdV1 {
                owner: self.collection_publication_pool()?.owner.clone(),
                slot: p.rows.len(),
            };
            let context = Context {
                command: command.clone(),
                scope: copy_scope(scope)?,
                result: original.clone(),
                ceiling,
                stopped: false,
            };
            let super::super::State::Active(p) = &mut self.collection_publication_streams else {
                unreachable!()
            };
            let super::State::Active(t) = &mut p.commands else {
                unreachable!()
            };
            let State::Active(p) = &mut t.receipts else {
                unreachable!()
            };
            p.rows.push(Some(context));
            Ok(id)
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationReceiptAdmission,
            result,
            timing.finish(),
        )
    }
    pub fn release_collection_publication_receipt_v1(
        &mut self,
        context: &CollectionPublicationReceiptContextIdV1,
        command: &CollectionPublicationCommandIdV1,
        scope: &CollectionPublicationScopeV1,
        f: &TrustedCollectionPublicationReceiptHistoricalFactsV1<'_>,
        r: &mut Recorder,
    ) -> Result<CollectionPublicationReceiptReplyV1, Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let c = self.collection_receipt_context_v1(context, command, scope)?;
            if c.stopped || f.current.scope != scope {
                return Err(Error::Unavailable);
            }
            self.validate_collection_receipt_basis_v1(
                f.basis,
                Some(context),
                command,
                &c.result,
                r,
            )?;
            // Foreign/stale classification cannot apply loss. Missing evidence
            // fails closed; an explicit current loss is absorbing for this context.
            if f.current.authorization_current != Some(true) {
                return Err(Error::Unavailable);
            }
            if f.current.authority()[..8].contains(&Some(false)) {
                self.collection_receipt_mut_v1(context).stopped = true;
                r.record(Observation {
                    boundary: Boundary::CollectionPublicationReceiptClosure,
                    disposition: Disposition::Completed,
                    service_time: None,
                });
                return Err(Error::Unavailable);
            }
            if f.current.authority().into_iter().any(|v| v != Some(true))
                || !f.current.ready()
                || self.source_facade_busy
            {
                return Err(Error::Unavailable);
            }
            self.ensure_operational().map_err(|_| Error::Faulted)?;
            self.collection_command_ordinary_gate_v1()?;
            let c = self.collection_receipt_context_v1(context, command, scope)?;
            if reply_bytes(&c.result)?
                > self
                    .collection_receipt_pool_v1()?
                    .limits
                    .max_reply_bytes
                    .min(f.current.max_reply_bytes)
            {
                return Err(Error::Limit);
            }
            Ok(CollectionPublicationReceiptReplyV1 {
                command: command.clone(),
                result: c.result.clone(),
            })
        })();
        let result = result.and_then(|reply| {
            self.validate_collection_publication_receipt_reply_v1(
                context, command, scope, &reply, r,
            )?;
            Ok(reply)
        });
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationReceiptRelease,
            result,
            timing.finish(),
        )
    }
    pub fn stop_collection_publication_receipt_v1(
        &mut self,
        context: &CollectionPublicationReceiptContextIdV1,
        command: &CollectionPublicationCommandIdV1,
        control: &CollectionPublicationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            self.collection_receipt_context_v1(context, command, control.scope)?;
            if control.owner_control_allowed != Some(true) {
                return Err(Error::Unavailable);
            }
            self.collection_receipt_mut_v1(context).stopped = true;
            Ok(())
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationReceiptStop,
            result,
            timing.finish(),
        )
    }
    pub fn retire_collection_publication_receipt_v1(
        &mut self,
        context: &CollectionPublicationReceiptContextIdV1,
        command: &CollectionPublicationCommandIdV1,
        control: &CollectionPublicationControlV1<'_>,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let c = self.collection_receipt_context_v1(context, command, control.scope)?;
            if !c.stopped
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
            let super::super::State::Active(p) = &mut self.collection_publication_streams else {
                unreachable!()
            };
            let super::State::Active(t) = &mut p.commands else {
                unreachable!()
            };
            let State::Active(p) = &mut t.receipts else {
                unreachable!()
            };
            p.rows[context.slot] = None;
            Ok(())
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationReceiptRetirement,
            result,
            timing.finish(),
        )
    }
    pub fn retire_collection_publication_receipts_v1(
        &mut self,
        control: &SourceReadPoolControlV1,
        r: &mut Recorder,
    ) -> Result<(), Error> {
        let timing = r.begin_service_timing();
        let result = (|| {
            let p = self.collection_receipt_pool_v1()?;
            if p.rows.iter().any(Option::is_some)
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
            let super::super::State::Active(p) = &mut self.collection_publication_streams else {
                unreachable!()
            };
            let super::State::Active(t) = &mut p.commands else {
                unreachable!()
            };
            let State::Active(p) = &mut t.receipts else {
                unreachable!()
            };
            self.retention.release(&mut p.reservation).map_err(parent)?;
            t.receipts = State::Closed {
                issued: p.rows.len(),
            };
            Ok(())
        })();
        Self::finish_collection_publication(
            r,
            Boundary::CollectionPublicationReceiptRetirement,
            result,
            timing.finish(),
        )
    }
    pub fn collection_publication_receipt_usage_v1(
        &self,
    ) -> Result<CollectionPublicationReceiptUsageV1, Error> {
        match &self.collection_command_table_v1()?.receipts {
            State::Unconfigured => Ok(CollectionPublicationReceiptUsageV1::default()),
            State::Closed { issued } => Ok(CollectionPublicationReceiptUsageV1 {
                closed: true,
                issued_lifetime: *issued,
                ..Default::default()
            }),
            State::Active(_) => {
                let p = self.collection_receipt_pool_v1()?;
                Ok(CollectionPublicationReceiptUsageV1 {
                    contexts: p.rows.iter().flatten().count(),
                    stopped: p.rows.iter().flatten().filter(|c| c.stopped).count(),
                    issued_lifetime: p.rows.len(),
                    reserved_pool_bytes: p.charge,
                    closed: false,
                })
            }
        }
    }
}
