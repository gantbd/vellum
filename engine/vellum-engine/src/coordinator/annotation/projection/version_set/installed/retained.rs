//! Independent ordered narrowing of an original available coherent parent.
use super::*;
#[cfg(test)]
mod tests;
impl SourceCoordinator {
    pub fn capture_retained_version_set(
        &mut self,
        original: &VersionSetContextId,
        participants: &[VersionSetParticipantV1<'_>],
        f: &TrustedVersionSetContextFactsV1<'_>,
        preparation_ready: Option<bool>,
        r: &mut Recorder,
    ) -> Option<VersionSetAcquisitionV1> {
        self.capture_retained_version_set_result(original, participants, f, preparation_ready, r)
            .ok()
    }
    #[allow(clippy::too_many_arguments)]
    pub(in crate::coordinator) fn capture_retained_version_set_result(
        &mut self,
        original: &VersionSetContextId,
        participants: &[VersionSetParticipantV1<'_>],
        f: &TrustedVersionSetContextFactsV1<'_>,
        preparation_ready: Option<bool>,
        r: &mut Recorder,
    ) -> Result<VersionSetAcquisitionV1, Error> {
        let result = (|| {
            if original != f.context {
                return Err(Error::Unavailable);
            }
            let slot = self.version_parent_slot(VersionSetControlTargetV1::Context(original))?;
            let p = self.version_context_pool()?;
            let row = p.rows[slot].as_ref().ok_or(Error::Faulted)?;
            if participants.is_empty() || participants.len() > p.limits.max_members {
                return Err(Error::InvalidRequest);
            }
            if f.members.len() != row.value.members.len()
                || f.members
                    .iter()
                    .zip(&row.value.members)
                    .any(|(f, m)| f.scope != &m.scope)
            {
                return Err(Error::Unavailable);
            }
            let mut request = row
                .value
                .context_facts_request_bytes()?
                .checked_add(std::mem::size_of::<Option<bool>>())
                .ok_or(Error::Limit)?;
            for (i, part) in participants.iter().enumerate() {
                if participants[..i]
                    .iter()
                    .any(|m| m.scope.document == part.scope.document)
                {
                    return Err(Error::InvalidRequest);
                }
                let old = row
                    .value
                    .members
                    .iter()
                    .find(|m| m.scope.document == part.scope.document)
                    .ok_or(Error::Unavailable)?;
                if &old.scope != part.scope || !part.data.subset_of(old.value.classes) {
                    return Err(Error::InvalidRequest);
                }
                request = request
                    .checked_add(scope_bytes(part.scope)?)
                    .and_then(|n| {
                        n.checked_add(std::mem::size_of::<AnnotationProjectionClassesV1>())
                    })
                    .ok_or(Error::Limit)?;
            }
            if request > p.limits.max_request_bytes {
                return Err(Error::Limit);
            }
            // Narrowing/capacity cannot conceal loss in an omitted original peer.
            self.check_version_context(VersionSetControlTargetV1::Context(original), f, r)?;
            self.ensure_admission_unfenced()
                .map_err(source_context::admission_error)?;
            if preparation_ready != Some(true) {
                return Err(Error::Unavailable);
            }
            let p = self.version_context_pool()?;
            let row = p.rows[slot].as_ref().ok_or(Error::Faulted)?;
            let pending = participants.len().checked_add(1).ok_or(Error::Limit)?;
            if p.rows.iter().flatten().count() >= p.limits.max_contexts
                || p.issued()?
                    .checked_add(pending)
                    .is_none_or(|n| n > p.limits.max_issued)
                || p.live_selectors()
                    .checked_add(pending)
                    .is_none_or(|n| n > p.limits.max_selectors)
            {
                return Err(Error::Limit);
            }
            let mut capture = std::mem::size_of::<PreparedSet>();
            let mut scopes = 0usize;
            for part in participants {
                let old = row
                    .value
                    .members
                    .iter()
                    .find(|m| &m.scope == part.scope)
                    .ok_or(Error::Faulted)?;
                capture = capture
                    .checked_add(std::mem::size_of::<Member>())
                    .and_then(|n| n.checked_add(old.value.narrowed_bytes(part.data).ok()?))
                    .ok_or(Error::Limit)?;
                scopes = scopes
                    .checked_add(scope_bytes(part.scope)?)
                    .ok_or(Error::Limit)?;
            }
            if capture > p.limits.max_capture_bytes || scopes > p.limits.max_context_bytes {
                return Err(Error::Limit);
            }
            let mut members = Vec::new();
            members
                .try_reserve_exact(participants.len())
                .map_err(|_| Error::Capacity)?;
            for part in participants {
                let old = row
                    .value
                    .members
                    .iter()
                    .find(|m| &m.scope == part.scope)
                    .ok_or(Error::Faulted)?;
                members.push(Member {
                    scope: SourceReadScopeV1 {
                        document: old.scope.document.clone(),
                        context: settings::basis::copy_context(&old.scope.context)
                            .map_err(|_| Error::Capacity)?,
                        policy: owned(&old.scope.policy)?,
                    },
                    value: old.value.copy_narrowed(part.data)?,
                });
            }
            let value = PreparedSet {
                cut: row.value.cut,
                members,
                invalidated: false,
            };
            let (aliases, reply) = self.stage_version_install(&value, Some(original))?;
            let accepted =
                self.valid_version_acquisition(&value, &aliases, &reply, Some(original))?
                    && reply.participants.len() == participants.len()
                    && reply
                        .participants
                        .iter()
                        .zip(participants)
                        .all(|(result, requested)| {
                            result.document == requested.scope.document
                                && result.data == requested.data
                        });
            if !self.finish_read_result(
                accepted,
                ReadResultInvariantV1::NativeReadAcquisitionResultInvariant,
                Boundary::ReadAcquisitionResultValidation,
                r,
            ) {
                return Err(Error::Faulted);
            }
            // No registry mutation or issued identity exists before this guard.
            self.faulted = true;
            #[cfg(test)]
            assert!(
                !self.version_set_install_fault,
                "injected retained set staging fault"
            );
            let VersionSetContextState::Active(p) = &mut self.version_set_contexts else {
                return Err(Error::Faulted);
            };
            p.rows.push(Some(Context {
                value,
                stopped: false,
                holds: 0,
            }));
            p.aliases.extend(aliases);
            self.faulted = false;
            Ok(reply)
        })();
        self.finish_annotation_preparation(Boundary::VersionSetRetainedCapture, &result, r);
        result
    }
}
