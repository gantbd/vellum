//! Live read-record authority only; no decode, restoration, grant or payload pin.
use super::*;
use std::sync::Weak;
#[cfg(test)]
mod tests;
pub struct AnnotationLoadAuthorityFactsV1 {
    pub owner_control_allowed: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub resources_ready: Option<bool>,
}
impl AnnotationLoadAuthorityFactsV1 {
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
pub struct AnnotationLoadAuthorityV1 {
    task: AnnotationReadId,
    registration: Weak<Registration>,
    record: Weak<()>,
    capture: Weak<ReadCaptureData>,
}
impl PartialEq for AnnotationLoadAuthorityV1 {
    fn eq(&self, other: &Self) -> bool {
        self.task == other.task
            && Weak::ptr_eq(&self.registration, &other.registration)
            && Weak::ptr_eq(&self.record, &other.record)
            && Weak::ptr_eq(&self.capture, &other.capture)
    }
}
impl Eq for AnnotationLoadAuthorityV1 {}
impl AnnotationLoadAuthorityV1 {
    pub(in crate::coordinator) fn capture_for_invocation(&self) -> Self {
        Self {
            task: self.task.clone(),
            registration: self.registration.clone(),
            record: self.record.clone(),
            capture: self.capture.clone(),
        }
    }
    pub(in crate::coordinator) fn belongs_to(&self, owner: &Arc<()>) -> bool {
        Arc::ptr_eq(&self.task.owner, owner)
    }
    pub(in crate::coordinator) fn names_task(&self, id: &AnnotationReadId) -> bool {
        self.task == *id
    }
    pub(in crate::coordinator) fn names_document(&self, document: &DocumentId) -> bool {
        self.capture
            .upgrade()
            .is_some_and(|c| c.source.document == *document)
    }
}
impl std::fmt::Debug for AnnotationLoadAuthorityV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationLoadAuthorityV1(<opaque>)")
    }
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn annotation_load_task_for_invocation(
        &mut self,
        authority: &AnnotationLoadAuthorityV1,
        facts: &AnnotationLoadAuthorityFactsV1,
        r: &mut Recorder,
    ) -> Result<AnnotationReadId, ResourceError> {
        self.annotation_load_authority_current_v1(authority, facts, r)?;
        Ok(authority.task.clone())
    }
    fn current_annotation_load_task(
        &self,
        id: &AnnotationReadId,
        r: &mut Recorder,
    ) -> Result<&ReadTask, ResourceError> {
        self.ensure_operational().map_err(owner_error)?;
        let task = self.read_task(id)?;
        if task.contested {
            return Err(ResourceError::Contested);
        }
        if task.refused || task.stopped && task.restoration.is_none() {
            return Err(ResourceError::Unavailable);
        }
        let capture = task.capture.as_ref().ok_or({
            if matches!(task.terminal, Some(ReadAtom::End { .. })) {
                ResourceError::Faulted
            } else {
                ResourceError::MissingEvidence
            }
        })?;
        let Some(ReadAtom::End {
            version,
            chunks,
            bytes,
        }) = &task.terminal
        else {
            return Err(ResourceError::Faulted);
        };
        if task.phase != ReadPhase::Submitted
            || task.nonce.is_none()
            || capture.id != *id
            || capture.source != task.source
            || capture.set != task.set
            || capture.strength != task.strength
            || capture.version != *version
            || task.start.as_ref() != Some(version)
            || version.0.is_empty()
            || *chunks != task.chunks.len()
            || *bytes != capture.bytes.0.len()
            || task.identity.domain != task.registration.domain
        {
            return Err(ResourceError::Faulted);
        }
        let mut offset = 0usize;
        for (start, len) in &task.chunks {
            if *start != offset {
                return Err(ResourceError::Faulted);
            }
            offset = offset.checked_add(*len).ok_or(ResourceError::Faulted)?;
        }
        if offset != *bytes {
            return Err(ResourceError::Faulted);
        }
        if let Some(consumed) = &task.restoration
            && (!Arc::ptr_eq(capture, &consumed.package.capture.data)
                || consumed.package.capture.origin() != id
                || consumed.outcome.source != task.source
                || consumed.outcome.context != consumed.package.registration.context)
        {
            return Err(ResourceError::Faulted);
        }
        self.provider_current(&task.registration)?;
        self.annotation_read_catalog_current(
            task.catalog.as_ref(),
            &task.registration,
            &task.identity,
            true,
            r,
        )?;
        Ok(task)
    }
    /// Strict native provenance factory. Restoration/owner/writer permissions
    /// are independent, and actual consumer pins still govern task retirement.
    pub fn borrow_annotation_load_authority_v1(
        &mut self,
        id: &AnnotationReadId,
        facts: &AnnotationLoadAuthorityFactsV1,
        r: &mut Recorder,
    ) -> Result<AnnotationLoadAuthorityV1, ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if !facts.allowed() {
                return Err(ResourceError::Unavailable);
            }
            let task = self.current_annotation_load_task(id, r)?;
            Ok(AnnotationLoadAuthorityV1 {
                task: id.clone(),
                registration: Arc::downgrade(&task.registration),
                record: Arc::downgrade(task.nonce.as_ref().ok_or(ResourceError::Faulted)?),
                capture: Arc::downgrade(task.capture.as_ref().ok_or(ResourceError::Faulted)?),
            })
        })();
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture_service(
            r,
            Boundary::AnnotationLoadAuthorityBorrow,
            &result,
            timing.finish(),
        );
        result
    }
    pub fn annotation_load_authority_current_v1(
        &mut self,
        authority: &AnnotationLoadAuthorityV1,
        facts: &AnnotationLoadAuthorityFactsV1,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if !facts.allowed() {
                return Err(ResourceError::Unavailable);
            }
            let task = self.current_annotation_load_task(&authority.task, r)?;
            if !Weak::ptr_eq(&authority.registration, &Arc::downgrade(&task.registration))
                || !Weak::ptr_eq(
                    &authority.record,
                    &Arc::downgrade(task.nonce.as_ref().ok_or(ResourceError::Faulted)?),
                )
                || !Weak::ptr_eq(
                    &authority.capture,
                    &Arc::downgrade(task.capture.as_ref().ok_or(ResourceError::Faulted)?),
                )
            {
                return Err(ResourceError::Unavailable);
            }
            Ok(())
        })();
        if result == Err(ResourceError::Faulted) {
            self.faulted = true;
        }
        capture_service(
            r,
            Boundary::AnnotationLoadAuthorityValidation,
            &result,
            timing.finish(),
        );
        result
    }
}
