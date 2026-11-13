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
    resource::{
        AdapterFailure, AnnotationReadCompletion, ErasedPathResourceAdapterFactory,
        ResourceAdapter, ResourceAdapterFactory, WriteCompletion,
    },
    save_as::{FixedSaveAsAuthorityV1, SaveAsAuthorityRequestV1, SaveAsLifecycleV1},
    workspace::WorkspaceIdentityLimitsV1,
};
use vellum_protocol::WorkspaceDocumentIdV1;
use vellum_service::{Service, ServiceLimits, ServiceSaveAsStageV1};

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
    let operation = service
        .begin_save_as(
            document,
            version.revision,
            &mut authority,
            persist,
            rebind,
            destination_request,
        )
        .unwrap();
    service
        .admit_save_as(
            &operation,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Synchronized,
            },
            &export_facts(),
        )
        .unwrap();
    service
        .service_save_as(&operation, &export_facts())
        .unwrap();
    service
        .activate_save_as(&operation, 1, &activation_facts())
        .unwrap();

    let after_revision = service.identity_revision_evidence();
    assert_eq!(after_revision.0, before_revision.0 + 1);
    let checkpoint = service.identity_metadata(limits).unwrap();
    assert_eq!(checkpoint.document_bindings()[0].locator(), destination);
    let in_flight = service.save_as_receipt(&operation).unwrap().unwrap();
    assert!(in_flight.changes_stable_binding());
    assert!(matches!(
        in_flight.lifecycle(),
        SaveAsLifecycleV1::Fenced(_)
    ));

    service
        .release_save_as(&operation, &control_facts())
        .unwrap();
    assert!(
        service
            .retire_save_as(&operation, &control_facts())
            .is_err()
    );
    assert!(
        service
            .reconcile_save_as_retirement(&operation, &control_facts())
            .is_err()
    );
    assert!(matches!(
        service
            .save_as_receipt(&operation)
            .unwrap()
            .unwrap()
            .lifecycle(),
        SaveAsLifecycleV1::Fenced(_)
    ));
    let usage = service.usage();
    assert_eq!(usage.save_as_begin_requests, 1);
    assert_eq!(usage.save_as_begun, 1);
    assert_eq!(usage.save_as_binding_commits, 1);
    assert_eq!(usage.save_as_live, 1);
    assert_eq!(usage.save_as_fenced, 1);
    assert_eq!(usage.save_as_retained, 1);
    for stage in [
        ServiceSaveAsStageV1::Begin,
        ServiceSaveAsStageV1::Admission,
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
    let retirement = usage.save_as_stage(ServiceSaveAsStageV1::Retirement);
    assert_eq!(retirement.requests, 2);
    assert_eq!(retirement.completed, 0);
    assert_eq!(retirement.errors, 2);
    assert_eq!(retirement.latency_samples, 2);
}
