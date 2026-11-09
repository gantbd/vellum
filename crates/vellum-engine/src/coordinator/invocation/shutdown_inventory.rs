//! Native local-store obligations; no body copies or invented parent reservations.
use super::*;
use crate::coordinator::shutdown::inventory::{Builder, LocalOwnerSnapshot};
impl Slot {
    fn shutdown_holds(
        &self,
        owner: &Arc<()>,
        id: &InvocationId,
        ledger: &retention::RetentionState,
    ) -> Result<(usize, usize), CoordinatorError> {
        // Exhaustive holder classification: a new slot field needs an explicit
        // accounting decision, rather than disappearing behind a usage getter.
        let Self {
            observation_policy,
            annotation_hold,
            compound_results,
            disposal_hold,
            release_hold,
            rebind_hold,
            load_hold,
            saveas_hold,
            annotation_load_hold,
            live_relation_hold,
            scratch_hold,
            created_results,
            intent,
            phase: _,
            stop_requested: _,
            measurement_cohort: _,
        } = self;
        let (mut count, mut bytes) = (0usize, 0usize);
        let mut add = |charge: Option<usize>| -> Result<(), CoordinatorError> {
            if let Some(charge) = charge {
                count = count.checked_add(1).ok_or(CoordinatorError::Faulted)?;
                bytes = bytes.checked_add(charge).ok_or(CoordinatorError::Faulted)?;
            }
            Ok(())
        };
        macro_rules! pair {
            ($variant:ident, $hold:ident) => {
                match (intent.as_ref(), $hold.as_ref()) {
                    (Some(BoundIntent::$variant(c)), Some(_)) => {
                        add(Some(c.shutdown_reserved_bytes(owner, id, $hold, ledger)?))?
                    }
                    (Some(BoundIntent::$variant(_)), None) | (_, Some(_)) => {
                        return Err(CoordinatorError::Faulted)
                    }
                    (_, None) => {}
                }
            };
        }
        pair!(Annotation, annotation_hold);
        pair!(RetainedDisposal, disposal_hold);
        pair!(RetainedRelease, release_hold);
        pair!(SourceRebind, rebind_hold);
        pair!(SourceLoad, load_hold);
        pair!(SourceSaveAs, saveas_hold);
        pair!(AnnotationLoad, annotation_load_hold);
        pair!(LiveRelation, live_relation_hold);
        pair!(Scratch, scratch_hold);
        if let Some(hold) = compound_results {
            if let Some(BoundIntent::AtomicAuthoring(c)) = intent.as_ref()
                && hold.base_charge() != Some(c.expected_base_charge())
            {
                return Err(CoordinatorError::Faulted);
            }
            if !matches!(
                intent,
                Some(
                    BoundIntent::Compound(_)
                        | BoundIntent::AtomicAuthoring(_)
                        | BoundIntent::ScopedCompound { .. }
                        | BoundIntent::DependentCompound { .. }
                )
            ) {
                return Err(CoordinatorError::Faulted);
            }
            add(Some(hold.shutdown_reserved_bytes(owner, id, ledger)?))?;
        } else if matches!(intent, Some(BoundIntent::AtomicAuthoring(_))) {
            return Err(CoordinatorError::Faulted);
        }
        add(observation_policy.shutdown_reserved_bytes(id, ledger)?)?;
        if let Some(BoundIntent::ReturnedSearchReplacement(c)) = intent {
            if !c.intact(owner, id, ledger) {
                return Err(CoordinatorError::Faulted);
            }
            add(Some(c.reserved_bytes().ok_or(CoordinatorError::Faulted)?))?;
        }
        add(created_results.shutdown_reserved_bytes(id, ledger)?)?;
        Ok((count, bytes))
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn shutdown_cancellation_invocation_end(&self) -> usize {
        self.invocations.slots.len()
    }
    pub(in crate::coordinator) fn shutdown_cancellation_invocation_at(
        &self,
        index: usize,
    ) -> Result<Option<InvocationId>, CoordinatorError> {
        let Some(row) = self.invocations.slots.get(index).and_then(Option::as_ref) else {
            return Ok(None);
        };
        match row.phase.state().map_err(|_| CoordinatorError::Faulted)? {
            InvocationState::Pending => Ok(Some(InvocationId {
                namespace: self.namespace.clone(),
                slot: index,
            })),
            InvocationState::Issued | InvocationState::Finished => Ok(None),
        }
    }
    pub(in crate::coordinator) fn capture_shutdown_invocation_ledger(
        &self,
        b: &mut Builder,
    ) -> Result<(), CoordinatorError> {
        let role = ShutdownOwnerRoleV1::Invocations;
        let table = &self.invocations;
        let retained = b.slots(&table.slots)?;
        if retained != table.retained
            || retained > table.limits.max_retained
            || table.slots.len() > table.limits.max_lifetime_issued
        {
            return Err(CoordinatorError::Faulted);
        }
        let mut reserved = 0usize;
        for (slot, row) in table
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|s| (i, s)))
        {
            let id = InvocationId {
                namespace: self.namespace.clone(),
                slot,
            };
            let (retained_holds, reserved_bytes) =
                row.shutdown_holds(&self.namespace, &id, &self.retention)?;
            for _ in 0..retained_holds {
                b.step()?;
            }
            reserved = reserved
                .checked_add(reserved_bytes)
                .ok_or(CoordinatorError::Faulted)?;
            b.invocation(ShutdownInvocationRecordV1 {
                invocation: id,
                state: row.phase.state(),
                stop_requested: row.stop_requested,
                intent_bound: row.intent.is_some(),
                outcome_available: matches!(row.phase, Phase::Terminal(Some(_))),
                retained_holds,
                reserved_bytes,
            })?;
        }
        for (slot, document) in self.documents.iter().enumerate() {
            b.step()?;
            if document.is_none() {
                continue;
            }
            let id = DocumentId {
                namespace: self.namespace.clone(),
                slot,
            };
            if table.document_pins(&id)? > 0 {
                self.record_shutdown_pool_document(b, role, &id)?;
            }
        }
        if reserved > self.retention_usage().reserved {
            return Err(CoordinatorError::Faulted);
        }
        self.record_shutdown_local_owner(
            b,
            role,
            LocalOwnerSnapshot {
                additional_reserved: Some(reserved),
                configured: true,
                issued: Some(table.slots.len()),
                retained,
                held: None,
                reserved: None,
                limit: None,
            },
        )
    }
}
