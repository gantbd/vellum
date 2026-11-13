use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use vellum_editor::{Workspace, reference_workspace_profile};
use vellum_engine::{
    coordinator::{
        DocumentVersion, TrustedSourceExportControlV1, TrustedSourceExportFactsV1,
        TrustedSourceSaveAsFactsV1,
    },
    instrumentation::Recorder,
    resource::{
        Acknowledgment, AnnotationReadDispatch, AnnotationReadTarget, Application,
        OrdinaryPairCheck, OrdinaryPairEvidence, Protection, ProviderPort, ResourceRead,
        ResourceRelation, SaveDispatch, SaveRequirements, Synchronization,
    },
    source::Replacement,
};
use vellum_host::{
    authority::{
        AuthorityActionV1, AuthorityContextV1, AuthorityRequestV1, AuthorityResourceRefV1,
        AuthoritySubjectRefV1, SaveAsDestinationEffectV1, SaveAsDestinationObjectV1,
        SaveAsDestinationRequestV1,
    },
    request_journal::{
        DurableOperationCompletionV1, DurableOperationIntentV1, DurableOperationRecoveryV1,
        DurableOperationRetirementRecoveryV1, DurableOperationRetirementResultV1,
        DurableOperationRetirementV1, RequestJournalCompletionV1, RequestJournalIntentV1,
        RequestJournalLimitsV1, RequestJournalPortV1, RequestJournalRecoveryV1,
        RequestJournalRefusalV1, RequestJournalScopeV1,
    },
    resource::{
        AdapterFailure, AnnotationReadCompletion as HostAnnotationReadCompletion,
        ErasedPathResourceAdapterFactory, ResourceAdapter, ResourceAdapterFactory, WriteCompletion,
    },
    save_as::{
        FixedSaveAsAuthorityV1, SaveAsAuthorityRequestV1, SaveAsFailureV1, SaveAsFenceReasonV1,
        SaveAsLifecycleV1, SaveAsNotActivatedReasonV1, SaveAsPhysicalUnknownReasonV1,
    },
    save_as_journal::{
        DurableOperationRefV1, DurableSaveAsErrorV1, DurableSaveAsStageV1,
        DurableSaveAsTransactionV1,
    },
    workspace::{
        WorkspaceDocumentBindingV1, WorkspaceIdentityAllocationV1, WorkspaceIdentityLimitsV1,
        WorkspaceIdentityMetadataPortV1, WorkspaceIdentityMetadataV1,
        WorkspaceIdentityRecoveryRefusalV1, WorkspaceIdentityRecoveryV1,
        WorkspaceRootBindingKindV1, WorkspaceRootBindingV1,
    },
};

type Resources = Arc<Mutex<BTreeMap<PathBuf, (Vec<u8>, u64)>>>;

struct MemoryFactory(Resources);

impl ResourceAdapterFactory for MemoryFactory {
    type Adapter = MemoryAdapter;
    type Error = AdapterFailure;

    fn build(self, port: ProviderPort) -> Result<Self::Adapter, Self::Error> {
        Ok(MemoryAdapter {
            resources: self.0,
            port,
        })
    }
}

struct MemoryAdapter {
    resources: Resources,
    port: ProviderPort,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DurableEvent {
    Intent,
    BindingCheckpoint,
    Completion,
}

#[derive(Default)]
struct DurablePorts {
    events: Vec<DurableEvent>,
}

impl RequestJournalPortV1 for DurablePorts {
    fn recover(
        &mut self,
        _directory: &Path,
        _scope: RequestJournalScopeV1,
        _limits: RequestJournalLimitsV1,
    ) -> RequestJournalRecoveryV1 {
        RequestJournalRecoveryV1::Empty
    }

    fn begin(
        &mut self,
        _directory: &Path,
        _intent: RequestJournalIntentV1<'_>,
        _limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        Err(RequestJournalRefusalV1::UnsupportedResource)
    }

    fn complete(
        &mut self,
        _directory: &Path,
        _completion: RequestJournalCompletionV1<'_>,
        _limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        Err(RequestJournalRefusalV1::UnsupportedResource)
    }

    fn recover_operations(
        &mut self,
        _directory: &Path,
        _scope: RequestJournalScopeV1,
        _limits: RequestJournalLimitsV1,
    ) -> DurableOperationRecoveryV1 {
        DurableOperationRecoveryV1::Empty
    }

    fn begin_operation(
        &mut self,
        _directory: &Path,
        _intent: DurableOperationIntentV1<'_>,
        _limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        self.events.push(DurableEvent::Intent);
        Ok(())
    }

    fn complete_operation(
        &mut self,
        _directory: &Path,
        _completion: DurableOperationCompletionV1<'_>,
        _limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        self.events.push(DurableEvent::Completion);
        Ok(())
    }

    fn recover_operation_retirements(
        &mut self,
        _directory: &Path,
        _scope: RequestJournalScopeV1,
        _limits: RequestJournalLimitsV1,
    ) -> DurableOperationRetirementRecoveryV1 {
        DurableOperationRetirementRecoveryV1::Empty
    }

    fn retire_operation(
        &mut self,
        _directory: &Path,
        _retirement: DurableOperationRetirementV1<'_>,
        _limits: RequestJournalLimitsV1,
    ) -> Result<DurableOperationRetirementResultV1, RequestJournalRefusalV1> {
        Err(RequestJournalRefusalV1::UnsupportedResource)
    }
}

impl WorkspaceIdentityMetadataPortV1 for DurablePorts {
    fn load(
        &mut self,
        _directory: &Path,
        _limits: WorkspaceIdentityLimitsV1,
    ) -> WorkspaceIdentityRecoveryV1 {
        WorkspaceIdentityRecoveryV1::Refused(WorkspaceIdentityRecoveryRefusalV1::Missing)
    }

    fn store(
        &mut self,
        _directory: &Path,
        _metadata: &WorkspaceIdentityMetadataV1,
        _limits: WorkspaceIdentityLimitsV1,
    ) -> Result<(), WorkspaceIdentityRecoveryRefusalV1> {
        self.events.push(DurableEvent::BindingCheckpoint);
        Ok(())
    }
}

fn journal_limits() -> RequestJournalLimitsV1 {
    RequestJournalLimitsV1 {
        max_entries: 4,
        max_retired_operations: 4,
        max_request_bytes: 128,
        max_reply_bytes: 128,
        max_encoded_bytes: 1024,
    }
}

fn identity_limits() -> WorkspaceIdentityLimitsV1 {
    WorkspaceIdentityLimitsV1 {
        max_documents: 4,
        max_roots: 4,
        max_encoded_bytes: 1024,
    }
}

fn identity(generation: u64, document: &Path, root: &Path) -> WorkspaceIdentityMetadataV1 {
    WorkspaceIdentityMetadataV1::try_new_with_bindings(
        1,
        generation,
        WorkspaceIdentityAllocationV1::try_new(1, 4, 1, 4).unwrap(),
        vec![WorkspaceDocumentBindingV1::try_new(1, document.to_path_buf()).unwrap()],
        vec![
            WorkspaceRootBindingV1::try_new(
                1,
                WorkspaceRootBindingKindV1::File,
                root.to_path_buf(),
            )
            .unwrap(),
        ],
        identity_limits(),
    )
    .unwrap()
}

impl MemoryAdapter {
    fn observe(&self, path: &Path) -> Result<(ResourceRead, u64), AdapterFailure> {
        let resources = self.resources.lock().map_err(|_| AdapterFailure::Io)?;
        let (bytes, version) = resources.get(path).ok_or(AdapterFailure::Unavailable)?;
        let version_bytes = version.to_be_bytes();
        let read = self
            .port
            .read_result(path_key(path).as_bytes(), &version_bytes, bytes)
            .map_err(|_| AdapterFailure::Provider)?;
        Ok((read, *version))
    }

    fn write_inner(
        &mut self,
        dispatch: SaveDispatch,
        probe: (PathBuf, u64),
    ) -> Result<WriteCompletion<AdapterFailure>, AdapterFailure> {
        let (path, probed_version) = probe;
        if dispatch.resource_key() != path_key(&path).as_bytes() {
            return Err(AdapterFailure::Conflict);
        }
        let mut resources = self.resources.lock().map_err(|_| AdapterFailure::Io)?;
        let (current, version) = resources
            .get_mut(&path)
            .ok_or(AdapterFailure::Unavailable)?;
        if *version != probed_version
            || dispatch.expected_bytes() != Some(current.as_slice())
            || dispatch.expected_validator() != Some(version.to_be_bytes().as_slice())
        {
            return Err(AdapterFailure::Conflict);
        }
        let next_version = version.checked_add(1).ok_or(AdapterFailure::Capacity)?;
        current.clear();
        current
            .try_reserve_exact(dispatch.bytes().len())
            .map_err(|_| AdapterFailure::Capacity)?;
        current.extend_from_slice(dispatch.bytes());
        *version = next_version;
        let version_bytes = next_version.to_be_bytes();
        let written_observation = self
            .port
            .read_result(path_key(&path).as_bytes(), &version_bytes, current)
            .map_err(|_| AdapterFailure::Provider)?;
        let result = self
            .port
            .result(
                &dispatch,
                Application::Written,
                dispatch.bytes().len(),
                Synchronization::Succeeded,
                Some(&version_bytes),
            )
            .map_err(|_| AdapterFailure::Provider)?;
        let no_future_effects = self
            .port
            .no_future_effects(&dispatch)
            .map_err(|_| AdapterFailure::Provider)?;
        let quiescence = self
            .port
            .quiescence(&dispatch)
            .map_err(|_| AdapterFailure::Provider)?;
        Ok(WriteCompletion {
            written_observation: Some(written_observation),
            result,
            no_future_effects,
            quiescence,
            failure: None,
            cleanup_failed: false,
        })
    }
}

impl ResourceAdapter for MemoryAdapter {
    type Locator = Path;
    type Handle = PathBuf;
    type Probe = (PathBuf, u64);
    type Error = AdapterFailure;

    fn open(
        &mut self,
        locator: &Path,
        _recorder: &mut Recorder,
    ) -> Result<(Self::Handle, ResourceRead), Self::Error> {
        let (read, _) = self.observe(locator)?;
        Ok((locator.to_path_buf(), read))
    }

    fn probe(
        &self,
        handle: &Self::Handle,
        _recorder: &mut Recorder,
    ) -> Result<(ResourceRead, Self::Probe), Self::Error> {
        let (read, version) = self.observe(handle)?;
        Ok((read, (handle.clone(), version)))
    }

    fn compare_ordinary_pair(
        &self,
        check: &OrdinaryPairCheck,
        _recorder: &mut Recorder,
    ) -> Result<OrdinaryPairEvidence, Self::Error> {
        self.port
            .attest_ordinary_pair(check, ResourceRelation::Distinct)
            .map_err(|_| AdapterFailure::Provider)
    }

    fn write(
        &mut self,
        dispatch: SaveDispatch,
        probe: Self::Probe,
        _recorder: &mut Recorder,
    ) -> Result<WriteCompletion<Self::Error>, Self::Error> {
        self.write_inner(dispatch, probe)
    }

    fn write_observed(
        &mut self,
        dispatch: SaveDispatch,
        probe: Self::Probe,
        _recorder: &mut Recorder,
    ) -> Result<WriteCompletion<Self::Error>, Self::Error> {
        self.write_inner(dispatch, probe)
    }

    fn annotation_read_target(
        &self,
        _handle: &Self::Handle,
    ) -> Result<AnnotationReadTarget, Self::Error> {
        Err(AdapterFailure::UnsupportedResource)
    }

    fn read_annotation(
        &self,
        _dispatch: &AnnotationReadDispatch,
        _recorder: &mut Recorder,
    ) -> Result<HostAnnotationReadCompletion<Self::Error>, Self::Error> {
        Err(AdapterFailure::UnsupportedResource)
    }
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn export_facts() -> TrustedSourceExportFactsV1 {
    TrustedSourceExportFactsV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        source_readable: Some(true),
        write_allowed: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        provider_available: Some(true),
        resources_ready: Some(true),
        launch_allowed: Some(true),
        destination_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        create_allowed: None,
        overwrite_allowed: Some(true),
    }
}

fn control_facts() -> TrustedSourceExportControlV1 {
    TrustedSourceExportControlV1 {
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        binding_coverage_complete: Some(true),
        writer_coverage_complete: Some(true),
        binding_activation_allowed: Some(true),
        identity_allowed: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        disposition_readable: Some(true),
        reservation_release_allowed: Some(true),
    }
}

fn activation_facts() -> TrustedSourceSaveAsFactsV1 {
    TrustedSourceSaveAsFactsV1 {
        control: control_facts(),
        probe_attested: Some(true),
        resource_continuity_attested: Some(true),
        create_allowed: None,
        overwrite_allowed: Some(true),
    }
}

fn authority_for(
    source: &DocumentVersion,
    destination: PathBuf,
) -> (
    AuthorityRequestV1,
    AuthorityRequestV1,
    SaveAsDestinationRequestV1,
    FixedSaveAsAuthorityV1,
) {
    let context = AuthorityContextV1 {
        subject: AuthoritySubjectRefV1::new([7; 16]),
        session: None,
    };
    let resource = AuthorityResourceRefV1::new([9; 16]);
    let persist = AuthorityRequestV1 {
        context,
        resource,
        action: AuthorityActionV1::PersistSource,
    };
    let rebind = AuthorityRequestV1 {
        context,
        resource,
        action: AuthorityActionV1::RebindSource,
    };
    let destination_request = SaveAsDestinationRequestV1::new(
        context,
        resource,
        destination.clone(),
        SaveAsDestinationEffectV1::OverwriteExisting,
    );
    let authority_request =
        SaveAsAuthorityRequestV1::new(persist, rebind, source.clone(), destination_request.clone())
            .unwrap();
    let authority = FixedSaveAsAuthorityV1::install(
        authority_request,
        SaveAsDestinationObjectV1::install(destination_request.clone(), destination),
    )
    .unwrap();
    (persist, rebind, destination_request, authority)
}

#[test]
fn prepared_save_as_can_be_discarded_without_admission_or_write() {
    let base = std::env::temp_dir().join("vellum-portable-discard-memory");
    let source = base.join("source");
    let destination = base.join("destination");
    let resources = Arc::new(Mutex::new(BTreeMap::from([
        (source.clone(), (b"alpha".to_vec(), 1)),
        (destination.clone(), (b"old".to_vec(), 1)),
    ])));
    let mut profile = reference_workspace_profile(ErasedPathResourceAdapterFactory::new(
        MemoryFactory(resources.clone()),
    ));
    profile.engine.resources.as_mut().unwrap().max_key_bytes = 4096;
    let mut workspace = Workspace::open(std::slice::from_ref(&source), profile).unwrap();
    let document = workspace.inventory().unwrap().documents[0].clone();
    let version = workspace.document_version(&document).unwrap();
    let (persist, rebind, destination_request, mut authority) =
        authority_for(&version, destination.clone());
    let operation = workspace
        .begin_document_save_as(
            &document,
            version.revision,
            &mut authority,
            persist,
            rebind,
            destination_request,
        )
        .unwrap();
    assert_eq!(workspace.save_as_usage().retained, 1);

    assert!(
        workspace
            .discard_document_save_as_preparation(&operation)
            .unwrap()
    );
    assert_eq!(workspace.save_as_usage().retained, 0);
    assert!(
        workspace
            .discard_document_save_as_preparation(&operation)
            .is_err()
    );
    let resources = resources.lock().unwrap();
    assert_eq!(resources.get(&source).unwrap(), &(b"alpha".to_vec(), 1));
    assert_eq!(resources.get(&destination).unwrap(), &(b"old".to_vec(), 1));
}

#[test]
fn save_as_preserves_workspace_identity_and_routes_later_saves_to_the_new_binding() {
    let base = std::env::temp_dir().join("vellum-portable-memory");
    let source = base.join("source");
    let destination = base.join("destination");
    let resources = Arc::new(Mutex::new(BTreeMap::from([
        (source.clone(), (b"alpha".to_vec(), 1)),
        (destination.clone(), (b"old".to_vec(), 1)),
    ])));
    let mut profile = reference_workspace_profile(ErasedPathResourceAdapterFactory::new(
        MemoryFactory(resources.clone()),
    ));
    profile.engine.resources.as_mut().unwrap().max_key_bytes = 4096;
    let mut workspace = Workspace::open(std::slice::from_ref(&source), profile).unwrap();
    let view = workspace.open_view().unwrap();
    let before_inventory = workspace.inventory().unwrap();
    let document = before_inventory.documents[0].clone();
    let before_generation = workspace.inventory_generation();
    let source_version = workspace.document_version(&document).unwrap();
    let mut durable_ports = DurablePorts::default();
    let mut durable = DurableSaveAsTransactionV1::begin(
        &mut durable_ports,
        Path::new("journal"),
        journal_limits(),
        RequestJournalScopeV1::new(1, 1).unwrap(),
        DurableOperationRefV1::new([4; 16]).unwrap(),
        [5; 16],
        b"canonical save as request",
        1,
        &identity(1, &source, &source),
    )
    .unwrap();
    assert_eq!(durable.stage(), DurableSaveAsStageV1::IntentDurable);

    let (persist, rebind, destination_request, mut authority) =
        authority_for(&source_version, destination.clone());

    let operation = workspace
        .begin_document_save_as(
            &document,
            source_version.revision,
            &mut authority,
            persist,
            rebind,
            destination_request,
        )
        .unwrap();
    workspace
        .admit_document_save_as(
            &operation,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Synchronized,
            },
            &export_facts(),
        )
        .unwrap();
    workspace
        .service_document_save_as(&operation, &export_facts())
        .unwrap();
    let written = workspace
        .document_save_as_receipt(&operation)
        .unwrap()
        .unwrap();
    assert!(!written.changes_stable_binding());
    assert_eq!(
        written.failure(),
        Some(SaveAsFailureV1::NotActivated(
            SaveAsNotActivatedReasonV1::NotAttempted
        ))
    );
    assert!(matches!(
        written.lifecycle(),
        SaveAsLifecycleV1::Fenced(fence)
            if fence.contains(SaveAsFenceReasonV1::RetainedTask)
    ));
    workspace
        .activate_document_save_as(&operation, 1, &activation_facts())
        .unwrap();

    let status = workspace.document_save_as_status(&operation).unwrap();
    assert!(status.binding_committed);
    let activated = workspace
        .document_save_as_receipt(&operation)
        .unwrap()
        .unwrap();
    assert!(activated.changes_stable_binding());
    assert!(matches!(
        activated.lifecycle(),
        SaveAsLifecycleV1::Fenced(fence)
            if fence.contains(SaveAsFenceReasonV1::RetainedTask)
    ));
    assert_eq!(
        durable.checkpoint_binding(
            &mut durable_ports,
            Path::new("metadata"),
            identity_limits(),
            &identity(2, &source, &source),
        ),
        Err(DurableSaveAsErrorV1::IdentityConflict)
    );
    assert_eq!(durable.stage(), DurableSaveAsStageV1::IntentDurable);
    assert_eq!(durable_ports.events, vec![DurableEvent::Intent]);
    durable
        .checkpoint_binding(
            &mut durable_ports,
            Path::new("metadata"),
            identity_limits(),
            &identity(2, &destination, &source),
        )
        .unwrap();
    assert_eq!(durable.stage(), DurableSaveAsStageV1::BindingCheckpointed);
    assert_eq!(
        durable.complete(
            &mut durable_ports,
            Path::new("journal"),
            journal_limits(),
            &activated,
            b"canonical save as receipt",
        ),
        Err(DurableSaveAsErrorV1::ReceiptNotTerminal)
    );
    assert_eq!(workspace.inventory_generation(), before_generation);
    assert_eq!(workspace.inventory().unwrap(), before_inventory);
    assert_eq!(view.active_document_id(), &document);
    let bindings = workspace.binding_inventory().unwrap();
    assert_eq!(bindings.documents[0].document, document);
    assert_eq!(bindings.documents[0].locator, destination);
    {
        let state = resources.lock().unwrap();
        assert_eq!(state[&source].0, b"alpha");
        assert_eq!(state[&destination].0, b"alpha");
    }

    let current = workspace.snapshot(&view).unwrap();
    workspace
        .edit_snapshot(
            &view,
            &current.version,
            vec![Replacement {
                start: 0,
                end: 5,
                expected: "alpha".into(),
                inserted: "later".into(),
            }],
        )
        .unwrap();
    let edited = workspace.snapshot(&view).unwrap();
    workspace.save_snapshot(&view, &edited.version).unwrap();
    {
        let state = resources.lock().unwrap();
        assert_eq!(state[&source].0, b"alpha");
        assert_eq!(state[&destination].0, b"later");
    }

    workspace
        .release_document_save_as(&operation, &control_facts())
        .unwrap();
    workspace
        .retire_document_save_as(&operation, &control_facts())
        .unwrap();
    let complete = workspace
        .document_save_as_receipt(&operation)
        .unwrap()
        .unwrap();
    assert!(complete.changes_stable_binding());
    assert_eq!(complete.lifecycle(), SaveAsLifecycleV1::TerminalReplayable);
    durable
        .complete(
            &mut durable_ports,
            Path::new("journal"),
            journal_limits(),
            &complete,
            b"canonical save as receipt",
        )
        .unwrap();
    assert_eq!(durable.stage(), DurableSaveAsStageV1::Complete);
    assert_eq!(
        durable_ports.events,
        vec![
            DurableEvent::Intent,
            DurableEvent::BindingCheckpoint,
            DurableEvent::Completion,
        ]
    );
    let usage = workspace.save_as_usage();
    assert_eq!(usage.issued_lifetime, 1);
    assert_eq!(usage.retained, 1);
    assert_eq!(usage.live, 0);
    assert_eq!(usage.durable, 1);
    assert_eq!(usage.activated, 1);
    assert_eq!(usage.complete, 1);
}

#[test]
fn changed_destination_fences_the_original_attempt_without_retry_or_binding_change() {
    let base = std::env::temp_dir().join("vellum-portable-memory-conflict");
    let source = base.join("source");
    let destination = base.join("destination");
    let resources = Arc::new(Mutex::new(BTreeMap::from([
        (source.clone(), (b"alpha".to_vec(), 1)),
        (destination.clone(), (b"old".to_vec(), 1)),
    ])));
    let mut profile = reference_workspace_profile(ErasedPathResourceAdapterFactory::new(
        MemoryFactory(resources.clone()),
    ));
    profile.engine.resources.as_mut().unwrap().max_key_bytes = 4096;
    let mut workspace = Workspace::open(std::slice::from_ref(&source), profile).unwrap();
    let document = workspace.inventory().unwrap().documents[0].clone();
    let source_version = workspace.document_version(&document).unwrap();
    let (persist, rebind, destination_request, mut authority) =
        authority_for(&source_version, destination.clone());
    let operation = workspace
        .begin_document_save_as(
            &document,
            source_version.revision,
            &mut authority,
            persist,
            rebind,
            destination_request,
        )
        .unwrap();
    workspace
        .admit_document_save_as(
            &operation,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Synchronized,
            },
            &export_facts(),
        )
        .unwrap();
    resources
        .lock()
        .unwrap()
        .insert(destination.clone(), (b"external".to_vec(), 2));

    assert!(
        workspace
            .service_document_save_as(&operation, &export_facts())
            .is_err()
    );
    let receipt = workspace
        .document_save_as_receipt(&operation)
        .unwrap()
        .unwrap();
    assert_eq!(
        receipt.failure(),
        Some(SaveAsFailureV1::PhysicalUnknown(
            SaveAsPhysicalUnknownReasonV1::EvidenceIncomplete
        ))
    );
    assert!(matches!(
        receipt.lifecycle(),
        SaveAsLifecycleV1::Fenced(fence)
            if fence.contains(SaveAsFenceReasonV1::PhysicalUncertainty)
                && fence.contains(SaveAsFenceReasonV1::RetainedTask)
    ));
    assert!(
        !workspace
            .reconcile_document_save_as_write(&operation, &export_facts())
            .unwrap()
    );
    assert_eq!(
        workspace.binding_inventory().unwrap().documents[0].locator,
        source
    );
    let state = resources.lock().unwrap();
    assert_eq!(state[&source].0, b"alpha");
    assert_eq!(state[&destination].0, b"external");
    drop(state);
    let usage = workspace.save_as_usage();
    assert_eq!(usage.live, 1);
    assert_eq!(usage.fenced, 1);
    assert_eq!(usage.physical_unknown, 1);
    assert_eq!(usage.activated, 0);
}
