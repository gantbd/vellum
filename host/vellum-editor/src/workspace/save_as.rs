//! Workspace-owned Save As transactions under stable document identity.

use super::*;
use vellum_engine::{
    coordinator::{
        TrustedSourceExportControlV1, TrustedSourceExportFactsV1, TrustedSourceSaveAsFactsV1,
    },
    resource::{Application, SaveRequirements, Synchronization},
};
use vellum_host::{
    authority::{AuthorityRequestV1, SaveAsDestinationRequestV1},
    save_as::{
        AuthorizedSaveAsV1, SaveAsAttachmentRefusalV1, SaveAsAuthorityPortV1, SaveAsEvidenceV1,
        SaveAsPersistenceV1,
    },
};
use vellum_session::host::{
    AuthorizedSourceSaveAsPreparationV1, PreparationDischargeV1, RetainedSourceSaveAsErrorV1,
    RetainedSourceSaveAsStageV1, RetainedSourceSaveAsV1, SaveAsActivation, SourceExportAttempt,
    SourceSaveAsActivationPortV1, SourceSaveAsAdmissionPortV1, SourceSaveAsCompletionPortV1,
    SourceSaveAsObservationV1, SourceSaveAsPreparationErrorV1, SourceSaveAsPreparePortV1,
    SourceSaveAsServicePortV1, SourceSaveAsTokenSetV1,
};

#[derive(Clone)]
pub struct WorkspaceSaveAsIdV1 {
    owner: Arc<()>,
    slot: usize,
    serial: u64,
}

impl PartialEq for WorkspaceSaveAsIdV1 {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
            && self.slot == other.slot
            && self.serial == other.serial
    }
}

impl Eq for WorkspaceSaveAsIdV1 {}

impl std::fmt::Debug for WorkspaceSaveAsIdV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("WorkspaceSaveAsIdV1(<opaque>)")
    }
}

#[derive(Debug)]
pub enum WorkspaceSaveAsErrorV1 {
    Workspace(WorkspaceError),
    Stale,
    DestinationConflict,
    InvalidTransaction,
    InvalidTransition,
    Preparation(SourceSaveAsPreparationErrorV1),
    Session(SessionError),
    IncompleteAuthority,
    InvalidDestinationObject,
    Attachment(SaveAsAttachmentRefusalV1),
}

impl From<WorkspaceError> for WorkspaceSaveAsErrorV1 {
    fn from(value: WorkspaceError) -> Self {
        Self::Workspace(value)
    }
}

impl From<SessionError> for WorkspaceSaveAsErrorV1 {
    fn from(value: SessionError) -> Self {
        Self::Session(value)
    }
}

struct WorkspaceSaveAsTokens;

impl SourceSaveAsTokenSetV1 for WorkspaceSaveAsTokens {
    type Preparation = AuthorizedSourceSaveAsPreparationV1;
    type Attempt = WorkspaceSaveAsAttempt;
    type Activation = SaveAsActivation<ErasedResourceHandle>;
}

struct WorkspaceSaveAsAttempt {
    native: SourceExportAttempt<ErasedResourceHandle>,
    authorization: Option<AuthorizedSaveAsV1>,
    evidence: Option<SaveAsEvidenceV1>,
}

pub(super) struct RetainedWorkspaceSaveAs {
    id: WorkspaceSaveAsIdV1,
    document: WorkspaceDocumentId,
    source: DocumentVersion,
    destination: PathBuf,
    binding_destination: Option<PathBuf>,
    operation: RetainedSourceSaveAsV1<WorkspaceSaveAsTokens>,
    binding_committed: bool,
}

impl std::fmt::Debug for RetainedWorkspaceSaveAs {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RetainedWorkspaceSaveAs")
            .field("id", &self.id)
            .field("document", &self.document)
            .field("stage", &self.operation.stage())
            .field("binding_committed", &self.binding_committed)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkspaceSaveAsStatusV1 {
    pub stage: RetainedSourceSaveAsStageV1,
    pub persistence: SaveAsPersistenceV1,
    pub binding: vellum_host::save_as::SaveAsBindingV1,
    pub binding_committed: bool,
}

struct PreparePort<'a, A: SaveAsAuthorityPortV1 + ?Sized> {
    host: &'a mut LocalHost,
    expected: &'a DocumentVersion,
    authority: &'a mut A,
    persist: AuthorityRequestV1,
    rebind: AuthorityRequestV1,
    destination: SaveAsDestinationRequestV1,
}

impl<A: SaveAsAuthorityPortV1 + ?Sized> SourceSaveAsPreparePortV1<WorkspaceSaveAsTokens>
    for PreparePort<'_, A>
{
    type Error = SourceSaveAsPreparationErrorV1;

    fn prepare(&mut self) -> Result<AuthorizedSourceSaveAsPreparationV1, Self::Error> {
        self.host.authorize_source_save_as_preparation(
            self.expected,
            self.authority,
            self.persist,
            self.rebind,
            self.destination.clone(),
        )
    }
}

#[derive(Debug)]
enum PortError {
    Session(SessionError),
    IncompleteAuthority,
    InvalidDestinationObject,
    Attachment(SaveAsAttachmentRefusalV1),
}

struct AdmissionPort<'a> {
    host: &'a mut LocalHost,
    requirements: SaveRequirements,
    facts: &'a TrustedSourceExportFactsV1,
}

impl SourceSaveAsAdmissionPortV1<WorkspaceSaveAsTokens> for AdmissionPort<'_> {
    type Error = PortError;

    fn admit(
        &mut self,
        preparation: &mut AuthorizedSourceSaveAsPreparationV1,
    ) -> Result<WorkspaceSaveAsAttempt, Self::Error> {
        let destination = preparation
            .destination()
            .ok_or(PortError::IncompleteAuthority)?;
        let native = self
            .host
            .queue_source_export(
                preparation.native(),
                destination,
                vellum_engine::resource::SourceExportPurposeV1::SaveAs,
                self.requirements,
                self.facts,
            )
            .map_err(PortError::Session)?;
        let authorization = preparation
            .take_authorization()
            .ok_or(PortError::IncompleteAuthority)?;
        Ok(WorkspaceSaveAsAttempt {
            native,
            authorization: Some(authorization),
            evidence: None,
        })
    }

    fn discard_preparation(
        &mut self,
        preparation: &AuthorizedSourceSaveAsPreparationV1,
    ) -> Result<PreparationDischargeV1, Self::Error> {
        self.host
            .discard_source_export_preparation(preparation.native())
            .map(|()| PreparationDischargeV1::Discarded)
            .map_err(PortError::Session)
    }
}

struct ServicePort<'a> {
    host: &'a mut LocalHost,
    expected_destination: &'a Path,
    facts: &'a TrustedSourceExportFactsV1,
}

impl SourceSaveAsServicePortV1<WorkspaceSaveAsTokens> for ServicePort<'_> {
    type Error = PortError;

    fn service(
        &mut self,
        attempt: &mut WorkspaceSaveAsAttempt,
    ) -> Result<SourceSaveAsObservationV1, Self::Error> {
        let authorization = attempt
            .authorization
            .take()
            .ok_or(PortError::IncompleteAuthority)?;
        let (evidence, destination, bytes, object) = authorization
            .into_effect_parts()
            .map_err(PortError::Attachment)?;
        let object = object
            .downcast::<PathBuf>()
            .map_err(|_| PortError::InvalidDestinationObject)?;
        if object != destination.destination() || object != self.expected_destination {
            return Err(PortError::InvalidDestinationObject);
        }
        drop(bytes);
        attempt.evidence = Some(evidence);
        let report = self
            .host
            .service_source_export(&attempt.native, self.facts)
            .map_err(PortError::Session)?;
        Ok(observe_outcome(&report.outcome))
    }

    fn observe(
        &mut self,
        attempt: &WorkspaceSaveAsAttempt,
    ) -> Result<SourceSaveAsObservationV1, Self::Error> {
        self.host
            .save_outcome(attempt.native.native_id())
            .map(|outcome| observe_outcome(&outcome))
            .map_err(PortError::Session)
    }
}

struct ActivationPort<'a> {
    host: &'a mut LocalHost,
    maximum_binding_history: usize,
    facts: &'a TrustedSourceSaveAsFactsV1,
}

impl SourceSaveAsActivationPortV1<WorkspaceSaveAsTokens> for ActivationPort<'_> {
    type Error = PortError;

    fn activate(
        &mut self,
        attempt: &WorkspaceSaveAsAttempt,
    ) -> Result<SaveAsActivation<ErasedResourceHandle>, Self::Error> {
        self.host
            .activate_source_save_as(&attempt.native, self.maximum_binding_history, self.facts)
            .map_err(PortError::Session)
    }

    fn recover_activation(
        &mut self,
        attempt: &WorkspaceSaveAsAttempt,
    ) -> Result<SaveAsActivation<ErasedResourceHandle>, Self::Error> {
        self.host
            .recover_source_save_as_activation(&attempt.native)
            .map_err(PortError::Session)
    }
}

struct CompletionPort<'a> {
    host: &'a mut LocalHost,
    facts: &'a TrustedSourceExportControlV1,
}

impl SourceSaveAsCompletionPortV1<WorkspaceSaveAsTokens> for CompletionPort<'_> {
    type Error = PortError;

    fn release(&mut self, attempt: &WorkspaceSaveAsAttempt) -> Result<bool, Self::Error> {
        self.host
            .release_source_export(&attempt.native, self.facts)
            .map_err(PortError::Session)
    }

    fn retire(&mut self, attempt: &WorkspaceSaveAsAttempt) -> Result<(), Self::Error> {
        self.host
            .retire_save(attempt.native.native_id())
            .map_err(PortError::Session)
    }
}

fn observe_outcome(outcome: &vellum_engine::resource::SaveOutcome) -> SourceSaveAsObservationV1 {
    let persistence = if outcome.application == Application::Written
        && outcome.synchronization == Synchronization::Succeeded
        && outcome.no_future_effects
        && outcome.quiescent
    {
        SaveAsPersistenceV1::Durable
    } else if outcome.application == Application::NotWritten
        && outcome.no_future_effects
        && outcome.quiescent
    {
        SaveAsPersistenceV1::NotDurable
    } else {
        SaveAsPersistenceV1::Unknown
    };
    SourceSaveAsObservationV1::new(persistence, outcome.source_save_as_binding_applied)
}

fn map_port(error: RetainedSourceSaveAsErrorV1<PortError>) -> WorkspaceSaveAsErrorV1 {
    match error {
        RetainedSourceSaveAsErrorV1::InvalidTransition => WorkspaceSaveAsErrorV1::InvalidTransition,
        RetainedSourceSaveAsErrorV1::Port(PortError::Session(error)) => {
            WorkspaceSaveAsErrorV1::Session(error)
        }
        RetainedSourceSaveAsErrorV1::Port(PortError::IncompleteAuthority) => {
            WorkspaceSaveAsErrorV1::IncompleteAuthority
        }
        RetainedSourceSaveAsErrorV1::Port(PortError::InvalidDestinationObject) => {
            WorkspaceSaveAsErrorV1::InvalidDestinationObject
        }
        RetainedSourceSaveAsErrorV1::Port(PortError::Attachment(error)) => {
            WorkspaceSaveAsErrorV1::Attachment(error)
        }
    }
}

impl Workspace {
    pub fn begin_document_save_as<A: SaveAsAuthorityPortV1 + ?Sized>(
        &mut self,
        document: &WorkspaceDocumentId,
        expected_revision: u64,
        authority: &mut A,
        persist: AuthorityRequestV1,
        rebind: AuthorityRequestV1,
        destination_request: SaveAsDestinationRequestV1,
    ) -> Result<WorkspaceSaveAsIdV1, WorkspaceSaveAsErrorV1> {
        if self.document_has_live_save_as(document) {
            return Err(WorkspaceError::SaveAsInProgress.into());
        }
        if self
            .save_as_transactions
            .iter()
            .filter(|transaction| transaction.is_some())
            .count()
            >= self.max_save_as_transactions
        {
            return Err(WorkspaceError::Capacity.into());
        }
        let retained = self.entry(document)?.document.clone();
        let source = self.host.read(&retained)?.version;
        if source.revision != expected_revision {
            return Err(WorkspaceSaveAsErrorV1::Stale);
        }
        let destination_path = destination_request.destination();
        if self
            .documents
            .iter()
            .any(|entry| entry.member && entry.locator == destination_path)
        {
            return Err(WorkspaceSaveAsErrorV1::DestinationConflict);
        }
        let mut destination = PathBuf::new();
        destination
            .try_reserve(destination_path.as_os_str().len())
            .map_err(|_| WorkspaceError::Capacity)?;
        destination.push(destination_path);
        let mut binding_destination = PathBuf::new();
        binding_destination
            .try_reserve(destination_path.as_os_str().len())
            .map_err(|_| WorkspaceError::Capacity)?;
        binding_destination.push(destination_path);
        let serial = self.next_save_as_transaction;
        let next = serial.checked_add(1).ok_or(WorkspaceError::Capacity)?;
        let slot = self
            .save_as_transactions
            .iter()
            .position(Option::is_none)
            .unwrap_or(self.save_as_transactions.len());
        if slot == self.save_as_transactions.len()
            && self.save_as_transactions.len() >= self.max_save_as_transactions
        {
            return Err(WorkspaceError::Capacity.into());
        }
        let id = WorkspaceSaveAsIdV1 {
            owner: self.id.0.clone(),
            slot,
            serial,
        };
        let mut operation = RetainedSourceSaveAsV1::new();
        let mut port = PreparePort {
            host: &mut self.host,
            expected: &source,
            authority,
            persist,
            rebind,
            destination: destination_request,
        };
        operation.prepare(&mut port).map_err(|error| match error {
            RetainedSourceSaveAsErrorV1::InvalidTransition => {
                WorkspaceSaveAsErrorV1::InvalidTransition
            }
            RetainedSourceSaveAsErrorV1::Port(error) => WorkspaceSaveAsErrorV1::Preparation(error),
        })?;
        let transaction = RetainedWorkspaceSaveAs {
            id: id.clone(),
            document: document.clone(),
            source,
            destination,
            binding_destination: Some(binding_destination),
            operation,
            binding_committed: false,
        };
        if slot == self.save_as_transactions.len() {
            self.save_as_transactions.push(Some(transaction));
        } else {
            self.save_as_transactions[slot] = Some(transaction);
        }
        self.next_save_as_transaction = next;
        Ok(id)
    }

    pub fn admit_document_save_as(
        &mut self,
        id: &WorkspaceSaveAsIdV1,
        requirements: SaveRequirements,
        facts: &TrustedSourceExportFactsV1,
    ) -> Result<(), WorkspaceSaveAsErrorV1> {
        let index = self.save_as_index(id)?;
        let (host, transactions) = (&mut self.host, &mut self.save_as_transactions);
        let transaction = transactions[index]
            .as_mut()
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)?;
        transaction
            .operation
            .admit(&mut AdmissionPort {
                host,
                requirements,
                facts,
            })
            .map_err(map_port)
    }

    pub fn service_document_save_as(
        &mut self,
        id: &WorkspaceSaveAsIdV1,
        facts: &TrustedSourceExportFactsV1,
    ) -> Result<(), WorkspaceSaveAsErrorV1> {
        let index = self.save_as_index(id)?;
        let (host, transactions) = (&mut self.host, &mut self.save_as_transactions);
        let transaction = transactions[index]
            .as_mut()
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)?;
        transaction
            .operation
            .service(&mut ServicePort {
                host,
                expected_destination: &transaction.destination,
                facts,
            })
            .map_err(map_port)
    }

    pub fn reconcile_document_save_as_write(
        &mut self,
        id: &WorkspaceSaveAsIdV1,
        facts: &TrustedSourceExportFactsV1,
    ) -> Result<bool, WorkspaceSaveAsErrorV1> {
        let index = self.save_as_index(id)?;
        let (host, transactions) = (&mut self.host, &mut self.save_as_transactions);
        let transaction = transactions[index]
            .as_mut()
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)?;
        transaction
            .operation
            .reconcile_write(&mut ServicePort {
                host,
                expected_destination: &transaction.destination,
                facts,
            })
            .map_err(map_port)
    }

    pub fn activate_document_save_as(
        &mut self,
        id: &WorkspaceSaveAsIdV1,
        maximum_binding_history: usize,
        facts: &TrustedSourceSaveAsFactsV1,
    ) -> Result<(), WorkspaceSaveAsErrorV1> {
        let index = self.preflight_binding_commit(id)?;
        let (host, transactions) = (&mut self.host, &mut self.save_as_transactions);
        let transaction = transactions[index]
            .as_mut()
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)?;
        transaction
            .operation
            .activate(&mut ActivationPort {
                host,
                maximum_binding_history,
                facts,
            })
            .map_err(map_port)?;
        self.commit_activated_binding(index)
    }

    pub fn reconcile_document_save_as_activation(
        &mut self,
        id: &WorkspaceSaveAsIdV1,
    ) -> Result<bool, WorkspaceSaveAsErrorV1> {
        let index = self.preflight_binding_commit(id)?;
        let (host, transactions) = (&mut self.host, &mut self.save_as_transactions);
        let transaction = transactions[index]
            .as_mut()
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)?;
        let mut port = ReconcileActivationPort { host };
        let closed = transaction
            .operation
            .reconcile_activation(&mut port)
            .map_err(map_port)?;
        if transaction.operation.binding() == vellum_host::save_as::SaveAsBindingV1::Activated {
            self.commit_activated_binding(index)?;
        }
        Ok(closed)
    }

    pub fn release_document_save_as(
        &mut self,
        id: &WorkspaceSaveAsIdV1,
        facts: &TrustedSourceExportControlV1,
    ) -> Result<(), WorkspaceSaveAsErrorV1> {
        let index = self.save_as_index(id)?;
        let (host, transactions) = (&mut self.host, &mut self.save_as_transactions);
        transactions[index]
            .as_mut()
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)?
            .operation
            .release(&mut CompletionPort { host, facts })
            .map_err(map_port)
    }

    pub fn retire_document_save_as(
        &mut self,
        id: &WorkspaceSaveAsIdV1,
        facts: &TrustedSourceExportControlV1,
    ) -> Result<(), WorkspaceSaveAsErrorV1> {
        let index = self.save_as_index(id)?;
        let (host, transactions) = (&mut self.host, &mut self.save_as_transactions);
        transactions[index]
            .as_mut()
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)?
            .operation
            .retire(&mut CompletionPort { host, facts })
            .map_err(map_port)
    }

    pub fn document_save_as_status(
        &self,
        id: &WorkspaceSaveAsIdV1,
    ) -> Result<WorkspaceSaveAsStatusV1, WorkspaceSaveAsErrorV1> {
        let index = self.save_as_index(id)?;
        let transaction = self.save_as_transactions[index]
            .as_ref()
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)?;
        Ok(WorkspaceSaveAsStatusV1 {
            stage: transaction.operation.stage(),
            persistence: transaction.operation.persistence(),
            binding: transaction.operation.binding(),
            binding_committed: transaction.binding_committed,
        })
    }

    pub(super) fn document_has_live_save_as(&self, document: &WorkspaceDocumentId) -> bool {
        self.save_as_transactions
            .iter()
            .flatten()
            .any(|transaction| {
                transaction.document == *document
                    && !transaction.binding_committed
                    && transaction.operation.stage() != RetainedSourceSaveAsStageV1::Complete
            })
    }

    fn save_as_index(&self, id: &WorkspaceSaveAsIdV1) -> Result<usize, WorkspaceSaveAsErrorV1> {
        if !Arc::ptr_eq(&self.id.0, &id.owner) {
            return Err(WorkspaceSaveAsErrorV1::InvalidTransaction);
        }
        self.save_as_transactions
            .get(id.slot)
            .and_then(Option::as_ref)
            .filter(|transaction| transaction.id == *id)
            .map(|_| id.slot)
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)
    }

    fn preflight_binding_commit(
        &self,
        id: &WorkspaceSaveAsIdV1,
    ) -> Result<usize, WorkspaceSaveAsErrorV1> {
        let index = self.save_as_index(id)?;
        let transaction = self.save_as_transactions[index]
            .as_ref()
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)?;
        let entry = self
            .documents
            .get(transaction.document.slot)
            .filter(|entry| entry.member && entry.id == transaction.document)
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)?;
        if entry.locator == transaction.destination || transaction.binding_destination.is_none() {
            return Err(WorkspaceSaveAsErrorV1::InvalidTransaction);
        }
        Ok(index)
    }

    fn commit_activated_binding(&mut self, index: usize) -> Result<(), WorkspaceSaveAsErrorV1> {
        let (documents, transactions) = (&mut self.documents, &mut self.save_as_transactions);
        let transaction = transactions[index]
            .as_mut()
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)?;
        let evidence = transaction
            .operation
            .attempt()
            .and_then(|attempt| attempt.evidence.as_ref())
            .ok_or(WorkspaceSaveAsErrorV1::IncompleteAuthority)?;
        if evidence.source_version() != &transaction.source {
            return Err(WorkspaceSaveAsErrorV1::InvalidTransaction);
        }
        if transaction
            .operation
            .activation()
            .is_none_or(|activation| activation.source_version() != &transaction.source)
        {
            return Err(WorkspaceSaveAsErrorV1::InvalidTransaction);
        }
        let entry = documents
            .get_mut(transaction.document.slot)
            .filter(|entry| entry.member && entry.id == transaction.document)
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)?;
        if transaction.binding_destination.is_none() {
            return Err(WorkspaceSaveAsErrorV1::InvalidTransaction);
        }
        let activation = transaction
            .operation
            .take_activation()
            .map_err(|_| WorkspaceSaveAsErrorV1::InvalidTransition)?;
        let destination = transaction
            .binding_destination
            .take()
            .ok_or(WorkspaceSaveAsErrorV1::InvalidTransaction)?;
        entry.locator = destination;
        entry.document = activation.document;
        transaction.binding_committed = true;
        Ok(())
    }
}

struct ReconcileActivationPort<'a> {
    host: &'a mut LocalHost,
}

impl SourceSaveAsServicePortV1<WorkspaceSaveAsTokens> for ReconcileActivationPort<'_> {
    type Error = PortError;

    fn service(
        &mut self,
        _attempt: &mut WorkspaceSaveAsAttempt,
    ) -> Result<SourceSaveAsObservationV1, Self::Error> {
        Err(PortError::IncompleteAuthority)
    }

    fn observe(
        &mut self,
        attempt: &WorkspaceSaveAsAttempt,
    ) -> Result<SourceSaveAsObservationV1, Self::Error> {
        self.host
            .save_outcome(attempt.native.native_id())
            .map(|outcome| observe_outcome(&outcome))
            .map_err(PortError::Session)
    }
}

impl SourceSaveAsActivationPortV1<WorkspaceSaveAsTokens> for ReconcileActivationPort<'_> {
    type Error = PortError;

    fn activate(
        &mut self,
        _attempt: &WorkspaceSaveAsAttempt,
    ) -> Result<SaveAsActivation<ErasedResourceHandle>, Self::Error> {
        Err(PortError::IncompleteAuthority)
    }

    fn recover_activation(
        &mut self,
        attempt: &WorkspaceSaveAsAttempt,
    ) -> Result<SaveAsActivation<ErasedResourceHandle>, Self::Error> {
        self.host
            .recover_source_save_as_activation(&attempt.native)
            .map_err(PortError::Session)
    }
}
