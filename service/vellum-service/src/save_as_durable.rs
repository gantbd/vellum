//! Durable, adapter-neutral composition for the in-process Save As lifecycle.

use super::{Service, ServiceSaveAsErrorV1, ServiceSaveAsIdV1};
use std::path::Path;
use vellum_engine::{
    coordinator::{
        TrustedSourceExportControlV1, TrustedSourceExportFactsV1, TrustedSourceSaveAsFactsV1,
    },
    resource::SaveRequirements,
};
use vellum_host::{
    authority::{AuthorityRequestV1, SaveAsDestinationRequestV1},
    request_journal::{
        DurableOperationEnvelopeRefusalV1, DurableOperationSortV1, RequestJournalLimitsV1,
        RequestJournalPortV1, RequestJournalScopeV1, encode_durable_operation_envelope_v1,
    },
    save_as::{SaveAsAuthorityPortV1, SaveAsReceiptV1},
    save_as_journal::{
        DurableOperationRefV1, DurableSaveAsErrorV1, DurableSaveAsStageV1,
        DurableSaveAsTransactionV1,
    },
    workspace::{WorkspaceIdentityLimitsV1, WorkspaceIdentityMetadataPortV1},
};
use vellum_protocol::{RefusalV1, WorkspaceDocumentIdV1};

#[must_use = "a durable service Save As may own journal, metadata, or native cleanup obligations"]
pub struct DurableServiceSaveAsV1 {
    operation: ServiceSaveAsIdV1,
    durable: DurableSaveAsTransactionV1,
}

impl std::fmt::Debug for DurableServiceSaveAsV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DurableServiceSaveAsV1")
            .field("operation", &self.operation)
            .field("durable", &self.durable)
            .finish()
    }
}

impl DurableServiceSaveAsV1 {
    pub const fn stage(&self) -> DurableSaveAsStageV1 {
        self.durable.stage()
    }
}

#[derive(Debug)]
pub enum DurableServiceSaveAsBeginErrorV1 {
    Envelope(DurableOperationEnvelopeRefusalV1),
    Identity(RefusalV1),
    Service(ServiceSaveAsErrorV1),
    JournalClosed(DurableSaveAsErrorV1),
    JournalFenced {
        journal: DurableSaveAsErrorV1,
        operation: ServiceSaveAsIdV1,
        cleanup: Option<ServiceSaveAsErrorV1>,
    },
}

impl DurableServiceSaveAsBeginErrorV1 {
    pub const fn retained_operation(&self) -> Option<&ServiceSaveAsIdV1> {
        match self {
            Self::JournalFenced { operation, .. } => Some(operation),
            Self::Envelope(_) | Self::Identity(_) | Self::Service(_) | Self::JournalClosed(_) => {
                None
            }
        }
    }
}

#[derive(Debug)]
pub enum DurableServiceSaveAsErrorV1 {
    Service(ServiceSaveAsErrorV1),
    Identity(RefusalV1),
    Durable(DurableSaveAsErrorV1),
    MissingReceipt,
}

impl From<ServiceSaveAsErrorV1> for DurableServiceSaveAsErrorV1 {
    fn from(value: ServiceSaveAsErrorV1) -> Self {
        Self::Service(value)
    }
}

impl Service {
    #[allow(
        clippy::too_many_arguments,
        reason = "the exact durable Save As authority and journal product is intentionally explicit"
    )]
    pub fn begin_durable_save_as<
        A: SaveAsAuthorityPortV1 + ?Sized,
        J: RequestJournalPortV1 + ?Sized,
    >(
        &mut self,
        document: WorkspaceDocumentIdV1,
        expected_revision: u64,
        authority: &mut A,
        persist: AuthorityRequestV1,
        rebind: AuthorityRequestV1,
        destination: SaveAsDestinationRequestV1,
        journal: &mut J,
        journal_directory: &Path,
        journal_limits: RequestJournalLimitsV1,
        identity_limits: WorkspaceIdentityLimitsV1,
        durable_operation: DurableOperationRefV1,
        authenticated_subject: [u8; 16],
        frozen_request: &[u8],
    ) -> Result<DurableServiceSaveAsV1, DurableServiceSaveAsBeginErrorV1> {
        let frozen_request = encode_durable_operation_envelope_v1(
            DurableOperationSortV1::SaveAs,
            frozen_request,
            journal_limits.max_request_bytes,
        )
        .map_err(DurableServiceSaveAsBeginErrorV1::Envelope)?;
        let identity = self
            .identity_metadata(identity_limits)
            .map_err(DurableServiceSaveAsBeginErrorV1::Identity)?;
        let scope = RequestJournalScopeV1::new(identity.workspace(), identity.generation()).ok_or(
            DurableServiceSaveAsBeginErrorV1::Identity(RefusalV1::CounterExhausted),
        )?;
        let operation = self
            .begin_save_as(
                document,
                expected_revision,
                authority,
                persist,
                rebind,
                destination,
            )
            .map_err(DurableServiceSaveAsBeginErrorV1::Service)?;
        match DurableSaveAsTransactionV1::begin(
            journal,
            journal_directory,
            journal_limits,
            scope,
            durable_operation,
            authenticated_subject,
            &frozen_request,
            document.get(),
            &identity,
        ) {
            Ok(durable) => Ok(DurableServiceSaveAsV1 { operation, durable }),
            Err(journal_error) => match self.discard_save_as_preparation(&operation) {
                Ok(true) => Err(DurableServiceSaveAsBeginErrorV1::JournalClosed(
                    journal_error,
                )),
                Ok(false) => Err(DurableServiceSaveAsBeginErrorV1::JournalFenced {
                    journal: journal_error,
                    operation,
                    cleanup: None,
                }),
                Err(cleanup) => Err(DurableServiceSaveAsBeginErrorV1::JournalFenced {
                    journal: journal_error,
                    operation,
                    cleanup: Some(cleanup),
                }),
            },
        }
    }

    pub fn admit_durable_save_as(
        &mut self,
        operation: &DurableServiceSaveAsV1,
        requirements: SaveRequirements,
        facts: &TrustedSourceExportFactsV1,
    ) -> Result<(), DurableServiceSaveAsErrorV1> {
        self.admit_save_as(&operation.operation, requirements, facts)
            .map_err(Into::into)
    }

    pub fn service_durable_save_as(
        &mut self,
        operation: &DurableServiceSaveAsV1,
        facts: &TrustedSourceExportFactsV1,
    ) -> Result<(), DurableServiceSaveAsErrorV1> {
        self.service_save_as(&operation.operation, facts)
            .map_err(Into::into)
    }

    pub fn reconcile_durable_save_as_write(
        &mut self,
        operation: &DurableServiceSaveAsV1,
        facts: &TrustedSourceExportFactsV1,
    ) -> Result<bool, DurableServiceSaveAsErrorV1> {
        self.reconcile_save_as_write(&operation.operation, facts)
            .map_err(Into::into)
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "activation and its exact metadata checkpoint are one explicit durable transition"
    )]
    pub fn activate_durable_save_as<M: WorkspaceIdentityMetadataPortV1 + ?Sized>(
        &mut self,
        operation: &mut DurableServiceSaveAsV1,
        maximum_binding_history: usize,
        facts: &TrustedSourceSaveAsFactsV1,
        metadata: &mut M,
        metadata_directory: &Path,
        identity_limits: WorkspaceIdentityLimitsV1,
    ) -> Result<(), DurableServiceSaveAsErrorV1> {
        self.activate_save_as(&operation.operation, maximum_binding_history, facts)?;
        self.checkpoint_durable_save_as_binding(
            operation,
            metadata,
            metadata_directory,
            identity_limits,
        )
    }

    pub fn reconcile_durable_save_as_activation<M: WorkspaceIdentityMetadataPortV1 + ?Sized>(
        &mut self,
        operation: &mut DurableServiceSaveAsV1,
        metadata: &mut M,
        metadata_directory: &Path,
        identity_limits: WorkspaceIdentityLimitsV1,
    ) -> Result<bool, DurableServiceSaveAsErrorV1> {
        let closed = self.reconcile_save_as_activation(&operation.operation)?;
        if self
            .save_as_receipt(&operation.operation)?
            .is_some_and(|receipt| receipt.changes_stable_binding())
        {
            self.checkpoint_durable_save_as_binding(
                operation,
                metadata,
                metadata_directory,
                identity_limits,
            )?;
        }
        Ok(closed)
    }

    pub fn checkpoint_durable_save_as_binding<M: WorkspaceIdentityMetadataPortV1 + ?Sized>(
        &mut self,
        operation: &mut DurableServiceSaveAsV1,
        metadata: &mut M,
        metadata_directory: &Path,
        identity_limits: WorkspaceIdentityLimitsV1,
    ) -> Result<(), DurableServiceSaveAsErrorV1> {
        let candidate = self
            .identity_metadata(identity_limits)
            .map_err(DurableServiceSaveAsErrorV1::Identity)?;
        operation
            .durable
            .checkpoint_binding(metadata, metadata_directory, identity_limits, &candidate)
            .map_err(DurableServiceSaveAsErrorV1::Durable)
    }

    pub fn release_durable_save_as(
        &mut self,
        operation: &DurableServiceSaveAsV1,
        facts: &TrustedSourceExportControlV1,
    ) -> Result<(), DurableServiceSaveAsErrorV1> {
        self.release_save_as(&operation.operation, facts)
            .map_err(Into::into)
    }

    pub fn reconcile_durable_save_as_release(
        &mut self,
        operation: &DurableServiceSaveAsV1,
        facts: &TrustedSourceExportControlV1,
    ) -> Result<bool, DurableServiceSaveAsErrorV1> {
        self.reconcile_save_as_release(&operation.operation, facts)
            .map_err(Into::into)
    }

    pub fn retire_durable_save_as(
        &mut self,
        operation: &DurableServiceSaveAsV1,
        facts: &TrustedSourceExportControlV1,
    ) -> Result<(), DurableServiceSaveAsErrorV1> {
        self.retire_save_as(&operation.operation, facts)
            .map_err(Into::into)
    }

    pub fn reconcile_durable_save_as_retirement(
        &mut self,
        operation: &DurableServiceSaveAsV1,
        facts: &TrustedSourceExportControlV1,
    ) -> Result<bool, DurableServiceSaveAsErrorV1> {
        self.reconcile_save_as_retirement(&operation.operation, facts)
            .map_err(Into::into)
    }

    pub fn complete_durable_save_as<J: RequestJournalPortV1 + ?Sized>(
        &mut self,
        operation: &mut DurableServiceSaveAsV1,
        journal: &mut J,
        journal_directory: &Path,
        journal_limits: RequestJournalLimitsV1,
        frozen_reply: &[u8],
    ) -> Result<(), DurableServiceSaveAsErrorV1> {
        let receipt = self
            .save_as_receipt(&operation.operation)?
            .ok_or(DurableServiceSaveAsErrorV1::MissingReceipt)?;
        operation
            .durable
            .complete(
                journal,
                journal_directory,
                journal_limits,
                &receipt,
                frozen_reply,
            )
            .map_err(DurableServiceSaveAsErrorV1::Durable)
    }

    pub fn durable_save_as_receipt(
        &self,
        operation: &DurableServiceSaveAsV1,
    ) -> Result<Option<SaveAsReceiptV1>, DurableServiceSaveAsErrorV1> {
        self.save_as_receipt(&operation.operation)
            .map_err(Into::into)
    }
}
