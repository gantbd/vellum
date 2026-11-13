use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use vellum_editor::{Workspace, reference_workspace_profile};
use vellum_engine::{
    coordinator::{
        TrustedSourceExportControlV1, TrustedSourceExportFactsV1, TrustedSourceSaveAsFactsV1,
    },
    instrumentation::Recorder,
    resource::{
        Acknowledgment, AnnotationReadDispatch, AnnotationReadTarget, Application,
        OrdinaryPairCheck, OrdinaryPairEvidence, Protection, ProviderPort, ResourceRead,
        ResourceRelation, SaveDispatch, SaveRequirements, Synchronization,
    },
};
use vellum_host::{
    authority::{
        AuthorityActionV1, AuthorityContextV1, AuthorityRequestV1, AuthorityResourceRefV1,
        AuthoritySubjectRefV1, SaveAsDestinationEffectV1, SaveAsDestinationObjectV1,
        SaveAsDestinationRequestV1,
    },
    request_journal::{
        DurableOperationCompletionV1, DurableOperationIntentV1, DurableOperationRecordV1,
        DurableOperationRecoveryV1, DurableOperationRetirementRecoveryV1,
        DurableOperationRetirementResultV1, DurableOperationRetirementV1, DurableOperationSortV1,
        RequestJournalCompletionV1, RequestJournalIntentV1, RequestJournalLimitsV1,
        RequestJournalPortV1, RequestJournalRecoveryV1, RequestJournalRefusalV1,
        RequestJournalScopeV1, decode_durable_operation_envelope_v1,
    },
    resource::{
        AdapterFailure, AnnotationReadCompletion, ErasedPathResourceAdapterFactory,
        ResourceAdapter, ResourceAdapterFactory, WriteCompletion,
    },
    save_as::{FixedSaveAsAuthorityV1, SaveAsAuthorityRequestV1, SaveAsLifecycleV1},
    save_as_journal::{DurableOperationRefV1, DurableSaveAsStageV1},
    workspace::{
        WorkspaceIdentityLimitsV1, WorkspaceIdentityMetadataPortV1, WorkspaceIdentityMetadataV1,
        WorkspaceIdentityRecoveryRefusalV1, WorkspaceIdentityRecoveryV1,
    },
};
use vellum_protocol::{
    CommandV1, DurableSaveAsBindingV1, DurableSaveAsDestinationEffectV1,
    DurableSaveAsFenceReasonV1, DurableSaveAsIntentV1, DurableSaveAsLifecycleV1,
    DurableSaveAsPersistenceV1, DurableSaveAsReceiptV1, OutcomeV1, ReplyV1, RequestId, RequestV1,
    WorkspaceChangeKindV1, WorkspaceChangesV1, WorkspaceDocumentIdV1,
};
use vellum_service::{
    DecodeDurableServiceSaveAsErrorV1, DecodedDurableServiceSaveAsV1,
    DurableServiceSaveAsBeginErrorV1, Service, ServiceLimits, ServiceSaveAsStageV1,
    decode_durable_service_save_as_record_v1,
};
use vellum_wire::{
    WireLimits, decode_durable_save_as_intent_v1, encode_durable_save_as_receipt_v1,
};

type Resources = Arc<Mutex<BTreeMap<PathBuf, (Vec<u8>, u64)>>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DurableEvent {
    Intent,
    BindingCheckpoint,
    Completion,
}

#[derive(Default)]
struct DurablePorts {
    events: Vec<DurableEvent>,
    fail_intent: bool,
    frozen_request: Option<Vec<u8>>,
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
        intent: DurableOperationIntentV1<'_>,
        _limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        if self.fail_intent {
            return Err(RequestJournalRefusalV1::Io);
        }
        self.frozen_request = Some(intent.frozen_request.to_vec());
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
        max_request_bytes: 256,
        max_reply_bytes: 128,
        max_encoded_bytes: 1024,
    }
}

fn wire_limits() -> WireLimits {
    WireLimits {
        max_frame_bytes: 256,
        max_text_bytes: 128,
        max_fields: 16,
    }
}

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
    ) -> Result<AnnotationReadCompletion<Self::Error>, Self::Error> {
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

#[test]
fn service_binding_commit_advances_checkpoint_metadata_before_terminal_receipt() {
    let base = std::env::temp_dir().join("vellum-service-save-as-memory");
    let source = base.join("source");
    let destination = base.join("destination");
    let resources = Arc::new(Mutex::new(BTreeMap::from([
        (source.clone(), (b"alpha".to_vec(), 1)),
        (destination.clone(), (b"old".to_vec(), 1)),
    ])));
    let mut profile = reference_workspace_profile(ErasedPathResourceAdapterFactory::new(
        MemoryFactory(resources),
    ));
    profile.engine.resources.as_mut().unwrap().max_key_bytes = 4096;
    let mut workspace = Workspace::open(std::slice::from_ref(&source), profile).unwrap();
    let native = workspace.inventory().unwrap().documents[0].clone();
    let version = workspace.document_version(&native).unwrap();
    let mut service = Service::new(workspace, ServiceLimits::reference()).unwrap();
    let observer = service.connect().unwrap();
    let limits = WorkspaceIdentityLimitsV1 {
        max_documents: 16,
        max_roots: 16,
        max_encoded_bytes: 8192,
    };
    let before_metadata = service.identity_metadata(limits).unwrap();
    let before_revision = service.identity_revision_evidence();
    let document = WorkspaceDocumentIdV1::new(
        service.workspace_id(),
        before_metadata.document_bindings()[0].ordinal(),
    )
    .unwrap();

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
    let intent = DurableSaveAsIntentV1::try_new(
        document,
        version.revision,
        destination.to_str().unwrap().into(),
        DurableSaveAsDestinationEffectV1::OverwriteExisting,
    )
    .unwrap();
    let mut failed_authority = FixedSaveAsAuthorityV1::install(
        SaveAsAuthorityRequestV1::new(
            persist,
            rebind,
            version.clone(),
            destination_request.clone(),
        )
        .unwrap(),
        SaveAsDestinationObjectV1::install(destination_request.clone(), destination.clone()),
    )
    .unwrap();
    let mut failed_ports = DurablePorts {
        fail_intent: true,
        ..DurablePorts::default()
    };
    let mismatched = DurableSaveAsIntentV1::try_new(
        document,
        version.revision,
        destination
            .with_extension("foreign")
            .to_str()
            .unwrap()
            .into(),
        DurableSaveAsDestinationEffectV1::OverwriteExisting,
    )
    .unwrap();
    assert!(matches!(
        service.begin_durable_save_as(
            &mismatched,
            &mut failed_authority,
            persist,
            rebind,
            destination_request.clone(),
            &mut failed_ports,
            Path::new("journal"),
            journal_limits(),
            wire_limits(),
            limits,
            DurableOperationRefV1::new([2; 16]).unwrap(),
            [5; 16],
        ),
        Err(DurableServiceSaveAsBeginErrorV1::IntentMismatch)
    ));
    assert!(failed_ports.events.is_empty());

    let failed = service.begin_durable_save_as(
        &intent,
        &mut failed_authority,
        persist,
        rebind,
        destination_request.clone(),
        &mut failed_ports,
        Path::new("journal"),
        journal_limits(),
        wire_limits(),
        limits,
        DurableOperationRefV1::new([3; 16]).unwrap(),
        [5; 16],
    );
    let error = failed.unwrap_err();
    assert!(error.retained_operation().is_none());
    assert!(failed_ports.events.is_empty());
    assert_eq!(service.usage().save_as_retained, 0);

    let mut authority = FixedSaveAsAuthorityV1::install(
        SaveAsAuthorityRequestV1::new(
            persist,
            rebind,
            version.clone(),
            destination_request.clone(),
        )
        .unwrap(),
        SaveAsDestinationObjectV1::install(destination_request.clone(), destination.clone()),
    )
    .unwrap();
    let mut durable_ports = DurablePorts::default();
    let mut operation = service
        .begin_durable_save_as(
            &intent,
            &mut authority,
            persist,
            rebind,
            destination_request,
            &mut durable_ports,
            Path::new("journal"),
            journal_limits(),
            wire_limits(),
            limits,
            DurableOperationRefV1::new([4; 16]).unwrap(),
            [5; 16],
        )
        .unwrap();
    assert_eq!(operation.stage(), DurableSaveAsStageV1::IntentDurable);
    assert_eq!(durable_ports.events, vec![DurableEvent::Intent]);
    let frozen = durable_ports.frozen_request.as_deref().unwrap();
    let envelope =
        decode_durable_operation_envelope_v1(frozen, journal_limits().max_request_bytes).unwrap();
    assert_eq!(envelope.sort(), DurableOperationSortV1::SaveAs);
    assert_eq!(
        decode_durable_save_as_intent_v1(envelope.payload(), wire_limits()).unwrap(),
        intent
    );
    let pending_record = DurableOperationRecordV1 {
        scope: RequestJournalScopeV1::new(1, 1).unwrap(),
        operation: DurableOperationRefV1::new([4; 16]).unwrap(),
        authenticated_subject: [5; 16],
        frozen_request: frozen.to_vec(),
        frozen_reply: None,
    };
    assert_eq!(
        decode_durable_service_save_as_record_v1(&pending_record, journal_limits(), wire_limits(),),
        Ok(DecodedDurableServiceSaveAsV1::OutcomeUnknown {
            intent: intent.clone(),
        })
    );
    let terminal = DurableSaveAsReceiptV1::try_new(
        document,
        version.revision,
        DurableSaveAsPersistenceV1::Durable,
        DurableSaveAsBindingV1::Activated,
        None,
        DurableSaveAsLifecycleV1::TerminalReplayable,
    )
    .unwrap();
    let complete_record = DurableOperationRecordV1 {
        frozen_reply: Some(encode_durable_save_as_receipt_v1(terminal, wire_limits()).unwrap()),
        ..pending_record
    };
    assert_eq!(
        decode_durable_service_save_as_record_v1(&complete_record, journal_limits(), wire_limits(),),
        Ok(DecodedDurableServiceSaveAsV1::Complete {
            intent: intent.clone(),
            receipt: terminal,
        })
    );
    let fenced = DurableSaveAsReceiptV1::try_new(
        document,
        version.revision,
        DurableSaveAsPersistenceV1::Durable,
        DurableSaveAsBindingV1::Activated,
        None,
        DurableSaveAsLifecycleV1::Fenced(
            vellum_protocol::DurableSaveAsFenceSetV1::try_from_bits(
                1 << DurableSaveAsFenceReasonV1::RetainedTask as u8,
            )
            .unwrap(),
        ),
    )
    .unwrap();
    let fenced_record = DurableOperationRecordV1 {
        frozen_reply: Some(encode_durable_save_as_receipt_v1(fenced, wire_limits()).unwrap()),
        ..complete_record
    };
    assert_eq!(
        decode_durable_service_save_as_record_v1(&fenced_record, journal_limits(), wire_limits(),),
        Err(DecodeDurableServiceSaveAsErrorV1::ReceiptNotTerminal)
    );
    service
        .admit_durable_save_as(
            &operation,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Synchronized,
            },
            &export_facts(),
        )
        .unwrap();
    service
        .service_durable_save_as(&operation, &export_facts())
        .unwrap();
    service
        .activate_durable_save_as(
            &mut operation,
            1,
            &activation_facts(),
            &mut durable_ports,
            Path::new("metadata"),
            limits,
        )
        .unwrap();
    assert_eq!(operation.stage(), DurableSaveAsStageV1::BindingCheckpointed);
    assert_eq!(
        durable_ports.events,
        vec![DurableEvent::Intent, DurableEvent::BindingCheckpoint]
    );

    let after_revision = service.identity_revision_evidence();
    assert_eq!(after_revision.0, before_revision.0 + 1);
    let ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Page(changes))) = service
        .execute(RequestV1 {
            client: observer,
            request: RequestId::new(1).unwrap(),
            command: CommandV1::WorkspaceChanges {
                after: None,
                page_size: 1,
            },
        })
    else {
        panic!("expected binding change page")
    };
    assert_eq!(changes.observed_head, 1);
    assert_eq!(changes.changes.len(), 1);
    assert!(matches!(
        changes.changes[0].kind,
        WorkspaceChangeKindV1::DocumentBindingChanged(changed) if changed == document
    ));
    let checkpoint = service.identity_metadata(limits).unwrap();
    assert_eq!(checkpoint.document_bindings()[0].locator(), destination);
    let in_flight = service
        .durable_save_as_receipt(&operation)
        .unwrap()
        .unwrap();
    assert!(in_flight.changes_stable_binding());
    assert!(matches!(
        in_flight.lifecycle(),
        SaveAsLifecycleV1::Fenced(_)
    ));
    let projected = service
        .durable_save_as_protocol_receipt(&operation)
        .unwrap()
        .unwrap();
    assert_eq!(projected.document(), document);
    assert_eq!(projected.source_revision(), version.revision);
    assert_eq!(projected.persistence(), DurableSaveAsPersistenceV1::Durable);
    assert_eq!(projected.binding(), DurableSaveAsBindingV1::Activated);
    let DurableSaveAsLifecycleV1::Fenced(fence) = projected.lifecycle() else {
        panic!("retained native task must remain fenced")
    };
    assert!(fence.contains(DurableSaveAsFenceReasonV1::RetainedTask));
    assert!(
        service
            .complete_durable_save_as(
                &mut operation,
                &mut durable_ports,
                Path::new("journal"),
                journal_limits(),
                wire_limits(),
            )
            .is_err()
    );
    assert_eq!(
        durable_ports.events,
        vec![DurableEvent::Intent, DurableEvent::BindingCheckpoint]
    );

    service
        .release_durable_save_as(&operation, &control_facts())
        .unwrap();
    assert!(
        service
            .retire_durable_save_as(&operation, &control_facts())
            .is_err()
    );
    assert!(
        service
            .reconcile_durable_save_as_retirement(&operation, &control_facts())
            .is_err()
    );
    assert!(matches!(
        service
            .durable_save_as_receipt(&operation)
            .unwrap()
            .unwrap()
            .lifecycle(),
        SaveAsLifecycleV1::Fenced(_)
    ));
    let usage = service.usage();
    assert_eq!(usage.save_as_begin_requests, 2);
    assert_eq!(usage.save_as_begun, 2);
    assert_eq!(usage.save_as_binding_commits, 1);
    assert_eq!(usage.save_as_live, 1);
    assert_eq!(usage.save_as_fenced, 1);
    assert_eq!(usage.save_as_retained, 1);
    for stage in [
        ServiceSaveAsStageV1::Persistence,
        ServiceSaveAsStageV1::Activation,
        ServiceSaveAsStageV1::Release,
    ] {
        let observed = usage.save_as_stage(stage);
        assert_eq!(observed.requests, 1);
        assert_eq!(observed.completed, 1);
        assert_eq!(observed.errors, 0);
        assert_eq!(observed.latency_samples, 1);
    }
    for stage in [ServiceSaveAsStageV1::Begin, ServiceSaveAsStageV1::Admission] {
        let observed = usage.save_as_stage(stage);
        assert_eq!(observed.requests, 2);
        assert_eq!(observed.completed, 2);
        assert_eq!(observed.errors, 0);
        assert_eq!(observed.latency_samples, 2);
    }
    let retirement = usage.save_as_stage(ServiceSaveAsStageV1::Retirement);
    assert_eq!(retirement.requests, 2);
    assert_eq!(retirement.completed, 0);
    assert_eq!(retirement.errors, 2);
    assert_eq!(retirement.latency_samples, 2);
}
