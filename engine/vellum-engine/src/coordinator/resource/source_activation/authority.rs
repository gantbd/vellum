//! Actual Save As record identity, never an effect or activation grant.
use super::*;
pub struct SourceSaveAsAuthorityFactsV1 {
    pub owner_control_allowed: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub resources_ready: Option<bool>,
}
impl SourceSaveAsAuthorityFactsV1 {
    fn allowed(&self) -> bool {
        [
            self.owner_control_allowed,
            self.preparation_reserved,
            self.resources_ready,
        ]
        .into_iter()
        .all(|v| v == Some(true))
    }
}
pub struct SourceSaveAsAuthorityV1 {
    task: SaveId,
    registration: Weak<Registration>,
    record: Weak<()>,
}
impl PartialEq for SourceSaveAsAuthorityV1 {
    fn eq(&self, other: &Self) -> bool {
        self.task == other.task
            && Weak::ptr_eq(&self.registration, &other.registration)
            && Weak::ptr_eq(&self.record, &other.record)
    }
}
impl Eq for SourceSaveAsAuthorityV1 {}
impl SourceSaveAsAuthorityV1 {
    pub(in crate::coordinator) fn capture_for_invocation(&self) -> Self {
        Self {
            task: self.task.clone(),
            registration: self.registration.clone(),
            record: self.record.clone(),
        }
    }
    pub(in crate::coordinator) fn belongs_to(&self, owner: &Arc<()>) -> bool {
        Arc::ptr_eq(&self.task.owner, owner)
    }
    pub(in crate::coordinator) fn names_task(&self, id: &SaveId) -> bool {
        self.task == *id
    }
}
impl std::fmt::Debug for SourceSaveAsAuthorityV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceSaveAsAuthorityV1(<opaque>)")
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn source_save_as_identity_for_invocation(
        &mut self,
        authority: &SourceSaveAsAuthorityV1,
        facts: &SourceSaveAsAuthorityFactsV1,
        r: &mut Recorder,
    ) -> Result<(SaveId, DocumentId), ResourceError> {
        self.source_save_as_authority_current_v1(authority, facts, r)?;
        Ok((
            authority.task.clone(),
            self.resource_task(&authority.task)?
                .captured
                .document
                .clone(),
        ))
    }
    fn current_source_save_as_task(&self, id: &SaveId) -> Result<&Task, ResourceError> {
        self.ensure_operational().map_err(owner_error)?;
        let task = self.resource_task(id)?;
        if task.kind != ArtifactKind::Source || task.purpose != SavePurpose::SourceSaveAs {
            return Err(ResourceError::Unsupported);
        }
        if task.source_export.is_none()
            || task.identity.domain != task.registration.domain
            || !Arc::ptr_eq(&task.registration.owner, &self.namespace)
        {
            return Err(ResourceError::Faulted);
        }
        self.provider_current(&task.registration)?;
        Ok(task)
    }
    pub fn borrow_source_save_as_authority_v1(
        &mut self,
        id: &SaveId,
        facts: &SourceSaveAsAuthorityFactsV1,
        r: &mut Recorder,
    ) -> Result<SourceSaveAsAuthorityV1, ResourceError> {
        let timing = r.begin_service_timing();
        let out = (|| {
            if !facts.allowed() {
                return Err(ResourceError::Unavailable);
            }
            let task = self.current_source_save_as_task(id)?;
            Ok(SourceSaveAsAuthorityV1 {
                task: id.clone(),
                registration: Arc::downgrade(&task.registration),
                record: Arc::downgrade(
                    &task
                        .source_export
                        .as_ref()
                        .ok_or(ResourceError::Faulted)?
                        .marker,
                ),
            })
        })();
        if matches!(out, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture_service(
            r,
            Boundary::SourceSaveAsAuthorityBorrow,
            &out,
            timing.finish(),
        );
        out
    }
    pub fn source_save_as_authority_current_v1(
        &mut self,
        authority: &SourceSaveAsAuthorityV1,
        facts: &SourceSaveAsAuthorityFactsV1,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = r.begin_service_timing();
        let out = (|| {
            if !facts.allowed() {
                return Err(ResourceError::Unavailable);
            }
            let task = self.current_source_save_as_task(&authority.task)?;
            if !Weak::ptr_eq(&authority.registration, &Arc::downgrade(&task.registration))
                || !Weak::ptr_eq(
                    &authority.record,
                    &Arc::downgrade(
                        &task
                            .source_export
                            .as_ref()
                            .ok_or(ResourceError::Faulted)?
                            .marker,
                    ),
                )
            {
                return Err(ResourceError::Unavailable);
            }
            Ok(())
        })();
        if out == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            r,
            Boundary::SourceSaveAsAuthorityValidation,
            &out,
            timing.finish(),
        );
        out
    }
}
