//! Explicit existing-file source destinations over the same native writer.
use super::*;
use vellum_engine::coordinator::{
    TrustedSourceExportControlV1, TrustedSourceExportFactsV1, TrustedSourceSaveAsFactsV1,
};
use vellum_engine::resource::{
    PreparedSourceArtifactIdV1, SourceExportDestinationV1, SourceExportOptionsV1,
    SourceExportPurposeV1,
};
use vellum_host::authority::{AuthorityRequestV1, SaveAsDestinationRequestV1};
use vellum_host::export_copy::{
    AuthorizedExportCopyPlanV1, AuthorizedExportCopyV1, ExportCopyAttachmentRefusalV1,
    ExportCopyAuthorityPortV1, ExportCopyAuthorizationRefusalV1,
};

#[derive(Debug)]
pub enum SourceExportCopyPreparationErrorV1 {
    Authority(ExportCopyAuthorizationRefusalV1),
    Preparation(SessionError),
    Capture(vellum_engine::resource::ResourceError),
    Attachment(ExportCopyAttachmentRefusalV1),
}

/// Original bounded codec preparation and exact native source capture. Admission
/// consumes the native preparation; failure never implies it was discarded.
pub struct SourceExportPreparation {
    native: PreparedSourceArtifactIdV1,
    source: DocumentVersion,
    serialization: vellum_engine::coordinator::SourceSerializationStateV1,
}
/// Retain before service. The original task and destination are not refreshed
/// from a newer document binding, caller path or source revision.
pub struct SourceExportAttempt<H> {
    task: SaveId,
    destination: H,
    source: DocumentVersion,
    purpose: SourceExportPurposeV1,
}
impl std::fmt::Debug for SourceExportPreparation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceExportPreparation(<opaque>)")
    }
}
impl<H> std::fmt::Debug for SourceExportAttempt<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceExportAttempt(<opaque>)")
    }
}
impl<H> SourceExportAttempt<H> {
    pub fn native_id(&self) -> &SaveId {
        &self.task
    }
}
pub struct SaveAsActivation<H> {
    pub document: LocalDocument<H>,
    pub changed: bool,
}
impl<A: ResourceAdapter> LocalHost<A> {
    /// Authorize the exact source/destination product before the codec
    /// allocates or encodes a source artifact. Only the content-free
    /// serialization state is inspected before authority; a denial therefore
    /// leaves no preparation to discharge and performs no source capture.
    pub fn authorize_source_export_copy<P: ExportCopyAuthorityPortV1 + ?Sized>(
        &mut self,
        expected: &DocumentVersion,
        authority: &mut P,
        authority_request: AuthorityRequestV1,
        destination_request: SaveAsDestinationRequestV1,
    ) -> Result<AuthorizedExportCopyV1, SourceExportCopyPreparationErrorV1> {
        let serialization = self
            .owner
            .source_serialization(&expected.document, &mut self.diagnostics)
            .map_err(|error| {
                SourceExportCopyPreparationErrorV1::Preparation(SessionError::Serialization(error))
            })?;
        let plan = AuthorizedExportCopyPlanV1::authorize(
            authority,
            authority_request,
            destination_request,
            expected.clone(),
            serialization,
        )
        .map_err(SourceExportCopyPreparationErrorV1::Authority)?;
        let prepared = self
            .owner
            .prepare_source_artifact(expected, serialization, &mut self.diagnostics)
            .map_err(|error| {
                SourceExportCopyPreparationErrorV1::Preparation(SessionError::from(error))
            })?;
        let capture = self
            .owner
            .capture_prepared_source_serialization(&prepared, &mut self.diagnostics)
            .map_err(SourceExportCopyPreparationErrorV1::Capture)?;
        plan.attach_source_capture(capture)
            .map_err(SourceExportCopyPreparationErrorV1::Attachment)
    }

    /// Preserve the exact current native serialization profile. This is source
    /// capture, not a destination grant or an implicit file creation request.
    pub fn prepare_source_export(
        &mut self,
        expected: &DocumentVersion,
    ) -> Result<SourceExportPreparation, SessionError> {
        let serialization = self
            .owner
            .source_serialization(&expected.document, &mut self.diagnostics)
            .map_err(SessionError::Serialization)?;
        let native =
            self.owner
                .prepare_source_artifact(expected, serialization, &mut self.diagnostics)?;
        Ok(SourceExportPreparation {
            native,
            source: expected.clone(),
            serialization,
        })
    }
    pub fn discard_source_export_preparation(
        &mut self,
        prepared: &SourceExportPreparation,
    ) -> Result<(), SessionError> {
        Ok(self
            .owner
            .discard_source_artifact_preparation(&prepared.native, &mut self.diagnostics)?)
    }

    /// Acquire both independent effect decisions before disclosing source bytes,
    /// then consume the exact codec preparation into that one-use plan. This
    /// does not access the filesystem, publish a copy, or rebind the document.
    pub fn authorize_prepared_source_export_copy<P: ExportCopyAuthorityPortV1 + ?Sized>(
        &mut self,
        prepared: &SourceExportPreparation,
        authority: &mut P,
        authority_request: AuthorityRequestV1,
        destination_request: SaveAsDestinationRequestV1,
    ) -> Result<AuthorizedExportCopyV1, SourceExportCopyPreparationErrorV1> {
        let plan = AuthorizedExportCopyPlanV1::authorize(
            authority,
            authority_request,
            destination_request,
            prepared.source.clone(),
            prepared.serialization,
        )
        .map_err(SourceExportCopyPreparationErrorV1::Authority)?;
        let capture = self
            .owner
            .capture_prepared_source_serialization(&prepared.native, &mut self.diagnostics)
            .map_err(SourceExportCopyPreparationErrorV1::Capture)?;
        plan.attach_source_capture(capture)
            .map_err(SourceExportCopyPreparationErrorV1::Attachment)
    }
    fn export_pair(
        &mut self,
        document: &DocumentId,
        destination: &SourceExportDestinationV1,
        purpose: SourceExportPurposeV1,
    ) -> Result<Option<OrdinaryPairEvidence>, SessionError> {
        if purpose == SourceExportPurposeV1::CopyExport {
            return Ok(None);
        }
        self.owner
            .source_export_pair_check(document, destination, &mut self.diagnostics)?
            .map(|check| {
                self.resource_adapter
                    .compare_ordinary_pair(&check, &mut self.diagnostics)
                    .map_err(|error| SessionError::Adapter(error.classification()))
            })
            .transpose()
    }
    /// Explicit replacement of an existing supported local file. Missing files
    /// refuse: exclusive create is not established by this substrate profile.
    /// Facts must come from the embedding's actual authority boundary.
    /// Filesystem preflight can leave preparation pending; native admission may
    /// spend it even on refusal. Do not infer either state from an error alone.
    pub fn queue_source_export(
        &mut self,
        prepared: &SourceExportPreparation,
        locator: &A::Locator,
        purpose: SourceExportPurposeV1,
        requirements: SaveRequirements,
        facts: &TrustedSourceExportFactsV1,
    ) -> Result<SourceExportAttempt<A::Handle>, SessionError> {
        let (destination, read) = self
            .resource_adapter
            .open(locator, &mut self.diagnostics)
            .map_err(|error| SessionError::Adapter(error.classification()))?;
        let read = read.into_source_export_destination();
        let pair = self.export_pair(&prepared.source.document, &read, purpose)?;
        let task = self.owner.queue_source_export(
            &prepared.native,
            read,
            SourceExportOptionsV1 {
                purpose,
                requirements,
                pair,
            },
            facts,
            &mut self.diagnostics,
        )?;
        Ok(SourceExportAttempt {
            task,
            destination,
            source: prepared.source.clone(),
            purpose,
        })
    }
    /// Fresh launch facts, exact original destination probe and separate receipts.
    /// A successful write alone never activates a Save As binding.
    pub fn service_source_export(
        &mut self,
        attempt: &SourceExportAttempt<A::Handle>,
        facts: &TrustedSourceExportFactsV1,
    ) -> Result<SaveReport, SessionError> {
        self.owner
            .save_outcome(&attempt.task, &mut self.diagnostics)?;
        let (read, probe) = self
            .resource_adapter
            .probe(&attempt.destination, &mut self.diagnostics)
            .map_err(|error| SessionError::Adapter(error.classification()))?;
        let read = read.into_source_export_destination();
        let pair = self.export_pair(&attempt.source.document, &read, attempt.purpose)?;
        let dispatch = self.owner.dispatch_source_export(
            &attempt.task,
            read,
            pair,
            facts,
            &mut self.diagnostics,
        )?;
        let completion = self
            .resource_adapter
            .write(dispatch, probe, &mut self.diagnostics)
            .map_err(|error| SessionError::Adapter(error.classification()))?;
        self.finish_write(&attempt.task, completion)
    }
    /// Local activation after acknowledged write and fresh native destination
    /// observation. Returns the new source-binding handle; old handles are not
    /// silently retargeted. Activation failure does not roll back an external write.
    pub fn activate_source_save_as(
        &mut self,
        attempt: &SourceExportAttempt<A::Handle>,
        max_binding_history: usize,
        facts: &TrustedSourceSaveAsFactsV1,
    ) -> Result<SaveAsActivation<A::Handle>, SessionError> {
        if attempt.purpose != SourceExportPurposeV1::SaveAs {
            return Err(ResourceError::Unsupported.into());
        }
        self.owner
            .save_outcome(&attempt.task, &mut self.diagnostics)?;
        let (read, probe) = self
            .resource_adapter
            .probe(&attempt.destination, &mut self.diagnostics)
            .map_err(|error| SessionError::Adapter(error.classification()))?;
        drop(probe);
        let pair = self.export_pair(
            &attempt.source.document,
            &read.into_source_export_destination(),
            attempt.purpose,
        )?;
        let (fresh, probe) = self
            .resource_adapter
            .probe(&attempt.destination, &mut self.diagnostics)
            .map_err(|error| SessionError::Adapter(error.classification()))?;
        drop(probe);
        let changed = self.owner.activate_source_save_as(
            &attempt.task,
            fresh.into_source_save_as_probe(),
            pair,
            max_binding_history,
            facts,
            &mut self.diagnostics,
        )?;
        Ok(SaveAsActivation {
            changed,
            document: LocalDocument {
                resource: attempt.destination.clone(),
                loaded: attempt.source.clone(),
            },
        })
    }
    pub fn release_source_export(
        &mut self,
        attempt: &SourceExportAttempt<A::Handle>,
        facts: &TrustedSourceExportControlV1,
    ) -> Result<bool, SessionError> {
        Ok(self.owner.release_source_export_reservation(
            &attempt.task,
            facts,
            &mut self.diagnostics,
        )?)
    }
}
