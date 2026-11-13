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
    save_as::{
        SaveAsAuthorityPortV1, SaveAsBindingUnknownReasonV1 as HostBindingUnknownReasonV1,
        SaveAsBindingV1 as HostBindingV1, SaveAsFailureV1 as HostFailureV1,
        SaveAsLifecycleV1 as HostLifecycleV1,
        SaveAsNotActivatedReasonV1 as HostNotActivatedReasonV1,
        SaveAsNotAttemptedReasonV1 as HostNotAttemptedReasonV1,
        SaveAsNotDurableReasonV1 as HostNotDurableReasonV1,
        SaveAsPersistenceV1 as HostPersistenceV1,
        SaveAsPhysicalUnknownReasonV1 as HostPhysicalUnknownReasonV1, SaveAsReceiptV1,
    },
    save_as_journal::{
        DurableOperationRefV1, DurableSaveAsErrorV1, DurableSaveAsStageV1,
        DurableSaveAsTransactionV1,
    },
    workspace::{WorkspaceIdentityLimitsV1, WorkspaceIdentityMetadataPortV1},
};
use vellum_protocol::{
    DurableSaveAsBindingUnknownReasonV1, DurableSaveAsBindingV1, DurableSaveAsFailureV1,
    DurableSaveAsFenceSetV1, DurableSaveAsLifecycleV1, DurableSaveAsNotActivatedReasonV1,
    DurableSaveAsNotAttemptedReasonV1, DurableSaveAsNotDurableReasonV1, DurableSaveAsPersistenceV1,
    DurableSaveAsPhysicalUnknownReasonV1, DurableSaveAsReceiptV1, RefusalV1, WorkspaceDocumentIdV1,
};

#[must_use = "a durable service Save As may own journal, metadata, or native cleanup obligations"]
pub struct DurableServiceSaveAsV1 {
    document: WorkspaceDocumentIdV1,
    operation: ServiceSaveAsIdV1,
    durable: DurableSaveAsTransactionV1,
}

impl std::fmt::Debug for DurableServiceSaveAsV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DurableServiceSaveAsV1")
            .field("document", &self.document)
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
    InvalidReceipt,
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
            Ok(durable) => Ok(DurableServiceSaveAsV1 {
                document,
                operation,
                durable,
            }),
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

    pub fn durable_save_as_protocol_receipt(
        &self,
        operation: &DurableServiceSaveAsV1,
    ) -> Result<Option<DurableSaveAsReceiptV1>, DurableServiceSaveAsErrorV1> {
        self.durable_save_as_receipt(operation)?
            .as_ref()
            .map(|receipt| project_save_as_receipt(operation.document, receipt))
            .transpose()
    }
}

fn project_save_as_receipt(
    document: WorkspaceDocumentIdV1,
    receipt: &SaveAsReceiptV1,
) -> Result<DurableSaveAsReceiptV1, DurableServiceSaveAsErrorV1> {
    let persistence = match receipt.persistence() {
        HostPersistenceV1::Durable => DurableSaveAsPersistenceV1::Durable,
        HostPersistenceV1::NotDurable => DurableSaveAsPersistenceV1::NotDurable,
        HostPersistenceV1::Unknown => DurableSaveAsPersistenceV1::Unknown,
        HostPersistenceV1::NotAttempted => DurableSaveAsPersistenceV1::NotAttempted,
    };
    let binding = match receipt.binding() {
        HostBindingV1::Activated => DurableSaveAsBindingV1::Activated,
        HostBindingV1::NotActivated => DurableSaveAsBindingV1::NotActivated,
        HostBindingV1::Unknown => DurableSaveAsBindingV1::Unknown,
    };
    let failure = receipt.failure().map(|failure| match failure {
        HostFailureV1::NotAttempted(reason) => DurableSaveAsFailureV1::NotAttempted(match reason {
            HostNotAttemptedReasonV1::AuthorityDenied => {
                DurableSaveAsNotAttemptedReasonV1::AuthorityDenied
            }
            HostNotAttemptedReasonV1::AuthorityUnavailable => {
                DurableSaveAsNotAttemptedReasonV1::AuthorityUnavailable
            }
            HostNotAttemptedReasonV1::StaleSource => DurableSaveAsNotAttemptedReasonV1::StaleSource,
            HostNotAttemptedReasonV1::DestinationRefused => {
                DurableSaveAsNotAttemptedReasonV1::DestinationRefused
            }
            HostNotAttemptedReasonV1::Capacity => DurableSaveAsNotAttemptedReasonV1::Capacity,
            HostNotAttemptedReasonV1::Unsupported => DurableSaveAsNotAttemptedReasonV1::Unsupported,
        }),
        HostFailureV1::NotDurable(reason) => DurableSaveAsFailureV1::NotDurable(match reason {
            HostNotDurableReasonV1::Conflict => DurableSaveAsNotDurableReasonV1::Conflict,
            HostNotDurableReasonV1::Unsupported => DurableSaveAsNotDurableReasonV1::Unsupported,
            HostNotDurableReasonV1::Limit => DurableSaveAsNotDurableReasonV1::Limit,
            HostNotDurableReasonV1::Capacity => DurableSaveAsNotDurableReasonV1::Capacity,
            HostNotDurableReasonV1::Io => DurableSaveAsNotDurableReasonV1::Io,
        }),
        HostFailureV1::PhysicalUnknown(reason) => {
            DurableSaveAsFailureV1::PhysicalUnknown(match reason {
                HostPhysicalUnknownReasonV1::Io => DurableSaveAsPhysicalUnknownReasonV1::Io,
                HostPhysicalUnknownReasonV1::Cleanup => {
                    DurableSaveAsPhysicalUnknownReasonV1::Cleanup
                }
                HostPhysicalUnknownReasonV1::EvidenceIncomplete => {
                    DurableSaveAsPhysicalUnknownReasonV1::EvidenceIncomplete
                }
                HostPhysicalUnknownReasonV1::TaskStateUnavailable => {
                    DurableSaveAsPhysicalUnknownReasonV1::TaskStateUnavailable
                }
            })
        }
        HostFailureV1::NotActivated(reason) => DurableSaveAsFailureV1::NotActivated(match reason {
            HostNotActivatedReasonV1::NotAttempted => {
                DurableSaveAsNotActivatedReasonV1::NotAttempted
            }
            HostNotActivatedReasonV1::ObservedNotApplied => {
                DurableSaveAsNotActivatedReasonV1::ObservedNotApplied
            }
            HostNotActivatedReasonV1::Refused => DurableSaveAsNotActivatedReasonV1::Refused,
            HostNotActivatedReasonV1::Unavailable => DurableSaveAsNotActivatedReasonV1::Unavailable,
            HostNotActivatedReasonV1::DestinationChanged => {
                DurableSaveAsNotActivatedReasonV1::DestinationChanged
            }
        }),
        HostFailureV1::BindingUnknown(reason) => {
            DurableSaveAsFailureV1::BindingUnknown(match reason {
                HostBindingUnknownReasonV1::Faulted => DurableSaveAsBindingUnknownReasonV1::Faulted,
                HostBindingUnknownReasonV1::TaskStateUnavailable => {
                    DurableSaveAsBindingUnknownReasonV1::TaskStateUnavailable
                }
            })
        }
    });
    let lifecycle = match receipt.lifecycle() {
        HostLifecycleV1::TerminalReplayable => DurableSaveAsLifecycleV1::TerminalReplayable,
        HostLifecycleV1::Fenced(fence) => DurableSaveAsLifecycleV1::Fenced(
            DurableSaveAsFenceSetV1::try_from_bits(fence.bits())
                .ok_or(DurableServiceSaveAsErrorV1::InvalidReceipt)?,
        ),
    };
    DurableSaveAsReceiptV1::try_new(
        document,
        receipt.evidence().source_revision(),
        persistence,
        binding,
        failure,
        lifecycle,
    )
    .ok_or(DurableServiceSaveAsErrorV1::InvalidReceipt)
}
