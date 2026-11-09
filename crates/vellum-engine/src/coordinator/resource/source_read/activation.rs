//! Transient native preparation, never a queued replacement root or public grant.
use super::super::source_materialization::PreparedSourceMaterialization;
use super::*;
use crate::coordinator::root_identity::NativeRootIdentity;
use std::sync::Weak;
pub(in crate::coordinator) struct PreparedSourceReadActivation {
    id: SourceReadIdV1,
    // Materialization carries its own root guard; repeats need only this one.
    before: Option<NativeRootIdentity>,
    nonce: Weak<()>,
    candidate: Weak<()>,
    expected: Option<(SourceReadActivationV1, BindingKey)>,
    materialization: Option<PreparedSourceMaterialization>,
    marker: Option<(SourceReadActivationV1, BindingKey)>,
    outcome: SourceReadActivationV1,
}
impl PreparedSourceReadActivation {
    pub(in crate::coordinator) fn outcome(&self) -> &SourceReadActivationV1 {
        &self.outcome
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn prepare_source_read_activation_native(
        &mut self,
        id: &SourceReadIdV1,
        r: &mut Recorder,
    ) -> Result<PreparedSourceReadActivation, ResourceError> {
        let task = self.current_source_load_task(id, r)?;
        let nonce = Arc::downgrade(task.nonce.as_ref().ok_or(ResourceError::Faulted)?);
        let candidate = Arc::downgrade(
            task.candidate_marker
                .as_ref()
                .ok_or(ResourceError::Faulted)?,
        );
        let expected = task.activated.clone();
        let (outcome, materialization, marker) = if let Some((original, key)) = &expected {
            let binding = self
                .resources
                .bindings
                .iter()
                .find(|b| b.document == original.document.document)
                .ok_or(ResourceError::Stale)?;
            if binding.identity != key.identity
                || binding.generation != key.generation
                || !Arc::ptr_eq(&binding.registration, &key.registration)
            {
                return Err(ResourceError::Stale);
            }
            let doc = self.document(&binding.document).map_err(owner_error)?;
            (
                SourceReadActivationV1 {
                    disposition: SourceReadActivationDispositionV1::AlreadyActivated,
                    document: DocumentVersion {
                        document: binding.document.clone(),
                        revision: doc.revision,
                    },
                },
                None,
                None,
            )
        } else {
            let version = task.start.as_ref().ok_or(ResourceError::Faulted)?;
            let read = ResourceRead {
                epoch: version.epoch.clone(),
                validator: version.token.clone(),
                registration: task.registration.clone(),
                identity: task.identity.clone(),
                bytes: blob(
                    &task.bytes,
                    self.source_reader()?.limits.max_input_bytes,
                    false,
                )?,
            };
            let policy = task
                .catalog
                .as_ref()
                .map(|c| c.source_insertion_policy())
                .transpose()?
                .unwrap_or(GeneratedNewlineV1::Lf);
            let plan = self.prepare_source_materialization(read, policy, r)?;
            let outcome = SourceReadActivationV1 {
                disposition: if plan.creates() {
                    SourceReadActivationDispositionV1::Created
                } else {
                    SourceReadActivationDispositionV1::Joined
                },
                document: plan.version.clone(),
            };
            let marker = Some((outcome.clone(), plan.key.clone()));
            (outcome, Some(plan), marker)
        };
        let before = if materialization.is_none() {
            Some(
                self.native_root_identity()
                    .map_err(|_| ResourceError::Faulted)?,
            )
        } else {
            None
        };
        Ok(PreparedSourceReadActivation {
            id: id.clone(),
            before,
            nonce,
            candidate,
            expected,
            materialization,
            marker,
            outcome,
        })
    }
    pub(super) fn commit_source_read_activation_native(
        &mut self,
        plan: PreparedSourceReadActivation,
        r: &mut Recorder,
    ) -> Result<SourceReadActivationV1, ResourceError> {
        self.commit_source_read_activation_for_invocation(plan, None, r)
    }
    pub(in crate::coordinator) fn source_load_writer_guard(
        &self,
        authority: &SourceLoadAuthorityV1,
        id: &SourceReadIdV1,
    ) -> Result<(), ResourceError> {
        if !authority.names_task(id) {
            return Err(ResourceError::Unavailable);
        }
        let task = self.source_read_task(id)?;
        if self.resource_contested(&task.identity) {
            return Err(ResourceError::Contested);
        }
        if self
            .resources
            .tasks
            .iter()
            .flatten()
            .any(|t| t.identity == task.identity && !t.safe())
        {
            return Err(ResourceError::Blocked);
        }
        Ok(())
    }
    pub(in crate::coordinator) fn commit_source_read_activation_for_invocation(
        &mut self,
        plan: PreparedSourceReadActivation,
        invocation: Option<&InvocationId>,
        r: &mut Recorder,
    ) -> Result<SourceReadActivationV1, ResourceError> {
        let task = self.current_source_load_task(&plan.id, r)?;
        if !task
            .nonce
            .as_ref()
            .is_some_and(|n| Weak::ptr_eq(&Arc::downgrade(n), &plan.nonce))
            || !task
                .candidate_marker
                .as_ref()
                .is_some_and(|n| Weak::ptr_eq(&Arc::downgrade(n), &plan.candidate))
            || task.activated != plan.expected
        {
            return Err(ResourceError::Stale);
        }
        if let Some(before) = &plan.before
            && &self
                .native_root_identity()
                .map_err(|_| ResourceError::Faulted)?
                != before
        {
            return Err(ResourceError::Stale);
        }
        if let Some(materialization) = &plan.materialization {
            materialization.validate(self, r)?;
        }
        let terminal = invocation
            .map(|id| self.stage_source_load_candidate(id, &plan.id, &plan.outcome))
            .transpose()?;
        if plan.marker.is_none() && invocation.is_none() {
            return Ok(plan.outcome);
        }
        // No callbacks, awaits, new clones or fallible capacity work below.
        self.faulted = true;
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(0),
            "injected activation interruption"
        );
        if let Some(materialization) = plan.materialization {
            materialization.install_fenced(self);
        }
        if plan.marker.is_some() {
            let State::Active(store) = &mut self.resources.source_reader else {
                unreachable!("validated source reader")
            };
            store.rows[plan.id.slot]
                .as_mut()
                .expect("validated source read")
                .activated = plan.marker;
        }
        #[cfg(test)]
        assert_ne!(
            self.publication_fault_after,
            Some(4),
            "injected activation marker interruption"
        );
        if let (Some(id), Some(terminal)) = (invocation, terminal) {
            #[cfg(test)]
            assert_ne!(
                self.publication_fault_after,
                Some(5),
                "injected activation terminal interruption"
            );
            self.install_invocation_terminal(id, terminal);
        }
        self.faulted = false;
        Ok(plan.outcome)
    }
}
