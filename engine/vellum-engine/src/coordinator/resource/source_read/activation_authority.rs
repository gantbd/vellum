//! Read-task/codec authority only. No source buffer, new read, document, producer
//! pin, caller-selected writer or activation permission is supplied by this handle.
use super::*;
use std::sync::Weak;

pub struct SourceLoadAuthorityFactsV1 {
    pub owner_control_allowed: Option<bool>,
    pub preparation_reserved: Option<bool>,
    pub resources_ready: Option<bool>,
}
impl SourceLoadAuthorityFactsV1 {
    fn allowed(&self) -> bool {
        [
            self.owner_control_allowed,
            self.preparation_reserved,
            self.resources_ready,
        ]
        .into_iter()
        .all(|f| f == Some(true))
    }
}
/// Opaque identity of this owner's exact submitted task and live codec witness.
/// Weak markers cannot keep task/decoded payload alive after actual retirement.
/// It is neither an execution grant nor a source/result disclosure capability.
pub struct SourceLoadAuthorityV1 {
    task: SourceReadIdV1,
    registration: Weak<Registration>,
    record: Weak<()>,
    candidate: Weak<()>,
}
impl PartialEq for SourceLoadAuthorityV1 {
    fn eq(&self, other: &Self) -> bool {
        self.task == other.task
            && Weak::ptr_eq(&self.registration, &other.registration)
            && Weak::ptr_eq(&self.record, &other.record)
            && Weak::ptr_eq(&self.candidate, &other.candidate)
    }
}
impl Eq for SourceLoadAuthorityV1 {}
impl SourceLoadAuthorityV1 {
    pub(in crate::coordinator) fn capture_for_invocation(&self) -> Self {
        Self {
            task: self.task.clone(),
            registration: self.registration.clone(),
            record: self.record.clone(),
            candidate: self.candidate.clone(),
        }
    }
    pub(in crate::coordinator) fn belongs_to(&self, owner: &Arc<()>) -> bool {
        Arc::ptr_eq(&self.task.owner, owner)
    }
    pub(in crate::coordinator) fn names_task(&self, id: &SourceReadIdV1) -> bool {
        self.task == *id
    }
}
impl std::fmt::Debug for SourceLoadAuthorityV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceLoadAuthorityV1(<opaque>)")
    }
}
fn complete(task: &Task) -> Result<(), ResourceError> {
    if task.contested {
        return Err(ResourceError::Contested);
    }
    if task.stopped || task.refused {
        return Err(ResourceError::Unavailable);
    }
    let Some(candidate) = &task.candidate else {
        return Err(ResourceError::MissingEvidence);
    };
    let Some(SourceReadAtom::End {
        version,
        chunks,
        bytes,
    }) = &task.terminal
    else {
        return Err(ResourceError::Faulted);
    };
    if task.phase != ReadPhase::Submitted
        || task.nonce.is_none()
        || task.candidate_marker.is_none()
        || task.identity.domain != task.registration.domain
        || task.start.as_ref() != Some(version)
        || *chunks != task.chunks.len()
        || *bytes != task.bytes.len()
        || version.token.0.is_empty()
        || version.epoch.as_ref().is_some_and(|e| e.0.is_empty())
    {
        return Err(ResourceError::Faulted);
    }
    let (bom, body) = task
        .bytes
        .strip_prefix(&[0xef, 0xbb, 0xbf])
        .map_or((false, task.bytes.as_slice()), |body| (true, body));
    if candidate.initial_bom() != bom || candidate.text().as_bytes() != body {
        return Err(ResourceError::Faulted);
    }
    Ok(())
}
impl SourceCoordinator {
    pub(in crate::coordinator) fn source_load_task_for_invocation(
        &mut self,
        authority: &SourceLoadAuthorityV1,
        facts: &SourceLoadAuthorityFactsV1,
        r: &mut Recorder,
    ) -> Result<SourceReadIdV1, ResourceError> {
        self.source_load_authority_current_v1(authority, facts, r)?;
        Ok(authority.task.clone())
    }
    pub(super) fn current_source_load_task(
        &self,
        id: &SourceReadIdV1,
        r: &mut Recorder,
    ) -> Result<&Task, ResourceError> {
        if !Arc::ptr_eq(&id.owner, &self.namespace) {
            return Err(ResourceError::Unavailable);
        }
        self.ensure_operational().map_err(owner_error)?;
        self.validate_source_reader()?;
        let task = self.source_read_task(id)?;
        self.provider_current(&task.registration)?;
        self.source_read_catalog_current(
            task.catalog.as_ref(),
            &task.registration,
            &task.identity,
            true,
            r,
        )?;
        complete(task)?;
        Ok(task)
    }
    /// Privileged native factory, not activation or northbound source disclosure.
    /// There is one actual source-read owner and paired destination writer in
    /// this coordinator; a caller cannot nominate another inspection or owner.
    pub fn borrow_source_load_authority_v1(
        &mut self,
        id: &SourceReadIdV1,
        facts: &SourceLoadAuthorityFactsV1,
        r: &mut Recorder,
    ) -> Result<SourceLoadAuthorityV1, ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if !facts.allowed() {
                return Err(ResourceError::Unavailable);
            }
            let task = self.current_source_load_task(id, r)?;
            Ok(SourceLoadAuthorityV1 {
                task: id.clone(),
                registration: Arc::downgrade(&task.registration),
                record: Arc::downgrade(task.nonce.as_ref().ok_or(ResourceError::Faulted)?),
                candidate: Arc::downgrade(
                    task.candidate_marker
                        .as_ref()
                        .ok_or(ResourceError::Faulted)?,
                ),
            })
        })();
        if matches!(result, Err(ResourceError::Faulted)) {
            self.faulted = true;
        }
        capture_service(
            r,
            Boundary::SourceLoadAuthorityBorrow,
            &result,
            timing.finish(),
        );
        result
    }
    /// Read-only currentness; no activation or lifetime extension is implied.
    pub fn source_load_authority_current_v1(
        &mut self,
        authority: &SourceLoadAuthorityV1,
        facts: &SourceLoadAuthorityFactsV1,
        r: &mut Recorder,
    ) -> Result<(), ResourceError> {
        let timing = r.begin_service_timing();
        let result = (|| {
            if !facts.allowed() {
                return Err(ResourceError::Unavailable);
            }
            let task = self.current_source_load_task(&authority.task, r)?;
            if !Weak::ptr_eq(&authority.registration, &Arc::downgrade(&task.registration))
                || !Weak::ptr_eq(
                    &authority.record,
                    &Arc::downgrade(task.nonce.as_ref().ok_or(ResourceError::Faulted)?),
                )
                || !Weak::ptr_eq(
                    &authority.candidate,
                    &Arc::downgrade(
                        task.candidate_marker
                            .as_ref()
                            .ok_or(ResourceError::Faulted)?,
                    ),
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
            Boundary::SourceLoadAuthorityValidation,
            &result,
            timing.finish(),
        );
        result
    }
}
