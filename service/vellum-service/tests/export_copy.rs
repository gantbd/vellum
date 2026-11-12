#![cfg(target_os = "macos")]
#![forbid(unsafe_code)]

use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};
use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
use vellum_editor::Workspace;
use vellum_engine::instrumentation::Recorder;
use vellum_host::{
    authentication::{
        AuthenticationDecisionV1, AuthenticationEvidenceV1, AuthenticationGateV1,
        AuthenticationPortV1, AuthenticationRequestV1,
    },
    authority::{
        AuthorityActionSetV1, AuthorityContextV1, AuthorityDecisionEvidenceV1,
        AuthorityDecisionRefV1, AuthorityDecisionV1, AuthorityPortV1, AuthorityRequestV1,
        AuthorityResourceRefV1, AuthoritySessionRefV1, AuthoritySubjectRefV1,
        SaveAsDestinationDecisionRefV1, SaveAsDestinationEvidenceV1, SaveAsDestinationObjectV1,
        SaveAsDestinationRequestV1,
    },
    export_copy::{
        AuthorizedExportCopyV1, ExportCopyAuthorityDecisionV1, ExportCopyAuthorityPortV1,
        ExportCopyEffectPortV1, ExportCopyFailureV1, ExportCopyReceiptV1,
        ExportCopySourceDecisionEvidenceV1, ExportCopySourceRequestV1,
        FailedExportCopyPersistenceV1,
    },
};
use vellum_protocol::{
    CommandV1, DurableEffectIntentV1, DurableEffectReceiptV1, DurableExecuteOutcomeV1,
    DurableExportCopyFailureV1, DurableExportCopyPersistenceV1, OutcomeV1, RefusalV1, ReplyV1,
    RequestId, RequestV1,
};
use vellum_service::{Service, ServiceLimits};

struct Fixture {
    source: PathBuf,
    destination: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "vellum-service-export-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir(&directory).unwrap();
        let source = directory.join("source-private.txt");
        fs::write(&source, "private-source-content").unwrap();
        Self {
            source,
            destination: directory.join("destination-private.txt"),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(directory) = self.source.parent() {
            let _ = fs::remove_dir_all(directory);
        }
    }
}

struct Authenticator {
    subject: AuthoritySubjectRefV1,
}

impl AuthenticationPortV1 for Authenticator {
    type Context = ();

    fn authenticate(
        &mut self,
        request: AuthenticationRequestV1,
        (): &Self::Context,
    ) -> AuthenticationDecisionV1 {
        AuthenticationDecisionV1::Authenticated(AuthenticationEvidenceV1::from_port(
            request,
            self.subject,
        ))
    }
}

struct ExactOrdinaryAuthority {
    context: AuthorityContextV1,
    resource: AuthorityResourceRefV1,
    observations: Rc<RefCell<ExportObservations>>,
}

impl AuthorityPortV1 for ExactOrdinaryAuthority {
    fn authorize(&mut self, request: AuthorityRequestV1) -> AuthorityDecisionV1 {
        self.observations.borrow_mut().ordinary_authority_calls += 1;
        if request.context != self.context || request.resource != self.resource {
            return AuthorityDecisionV1::Denied;
        }
        AuthorityDecisionV1::Granted(AuthorityDecisionEvidenceV1::new(
            AuthorityDecisionRefV1::new([51; 16]),
            request,
        ))
    }
}

#[derive(Default)]
struct ExportObservations {
    ordinary_authority_calls: usize,
    authority_calls: usize,
    effect_calls: usize,
    revision: Option<u64>,
    bytes: Vec<u8>,
}

struct ExactExportAuthority {
    context: AuthorityContextV1,
    resource: AuthorityResourceRefV1,
    destination: PathBuf,
    observations: Rc<RefCell<ExportObservations>>,
    deny: bool,
}

impl ExportCopyAuthorityPortV1 for ExactExportAuthority {
    fn authorize_export_copy(
        &mut self,
        source: ExportCopySourceRequestV1,
        destination: SaveAsDestinationRequestV1,
    ) -> ExportCopyAuthorityDecisionV1 {
        let mut observations = self.observations.borrow_mut();
        observations.authority_calls += 1;
        observations.revision = Some(source.source().revision);
        if self.deny
            || source.authority().context != self.context
            || source.authority().resource != self.resource
            || destination.context != self.context
            || destination.resource != self.resource
            || destination.destination() != self.destination
        {
            return ExportCopyAuthorityDecisionV1::Denied;
        }
        ExportCopyAuthorityDecisionV1::Granted {
            source: ExportCopySourceDecisionEvidenceV1::new(
                AuthorityDecisionRefV1::new([52; 16]),
                source,
            ),
            destination: SaveAsDestinationEvidenceV1::with_object(
                SaveAsDestinationDecisionRefV1::new([53; 16]),
                destination.clone(),
                SaveAsDestinationObjectV1::install(destination, ()),
            ),
        }
    }
}

/// Truthful non-publishing test effect. It observes the affine carrier and
/// returns the closed "not attempted/unsupported" physical receipt.
struct RefusingEffect(Rc<RefCell<ExportObservations>>);

impl ExportCopyEffectPortV1 for RefusingEffect {
    fn execute_export_copy(
        &mut self,
        authorization: AuthorizedExportCopyV1,
        _recorder: &mut Recorder,
    ) -> ExportCopyReceiptV1 {
        let (evidence, _destination, bytes, _object) = authorization.into_effect_parts();
        let mut observations = self.0.borrow_mut();
        observations.effect_calls += 1;
        observations.bytes = bytes;
        ExportCopyReceiptV1::failed(
            evidence,
            FailedExportCopyPersistenceV1::NotAttempted,
            ExportCopyFailureV1::Unsupported,
            false,
        )
    }
}

fn authenticated_service(
    fixture: &Fixture,
    deny: bool,
) -> (Service, Rc<RefCell<ExportObservations>>) {
    let workspace = Workspace::open(
        std::slice::from_ref(&fixture.source),
        vellum_editor::reference_workspace_profile(
            LocalFilesystemFactory::new(FilesystemLimits {
                max_resources: 8,
                max_bytes: 4096,
                max_stage_attempts: 8,
                max_metadata_bytes: 4096,
            })
            .erase(),
        ),
    )
    .unwrap();
    let subject = AuthoritySubjectRefV1::new([41; 16]);
    let session_ref = AuthoritySessionRefV1::new([42; 16]);
    let request = AuthenticationRequestV1 {
        challenge: vellum_host::authentication::AuthenticationChallengeRefV1::new([43; 16]),
        claim: vellum_host::authentication::AuthenticationClaimRefV1::new([44; 16]),
        session: session_ref,
    };
    let authenticated = AuthenticationGateV1::new(Authenticator { subject }, 1)
        .establish(request, &())
        .unwrap();
    let context = authenticated.authority_context();
    let resource = AuthorityResourceRefV1::new([45; 16]);
    let observations = Rc::new(RefCell::new(ExportObservations::default()));
    let service = Service::new_with_authenticated_export(
        workspace,
        ServiceLimits::reference(),
        ExactOrdinaryAuthority {
            context,
            resource,
            observations: Rc::clone(&observations),
        },
        ExactExportAuthority {
            context,
            resource,
            destination: fixture.destination.clone(),
            observations: Rc::clone(&observations),
            deny,
        },
        RefusingEffect(Rc::clone(&observations)),
        authenticated,
        resource,
    )
    .unwrap();
    (service, observations)
}

fn request(client: vellum_protocol::ClientId, id: u64, command: CommandV1) -> RequestV1 {
    RequestV1 {
        client,
        request: RequestId::new(id).unwrap(),
        command,
    }
}

#[test]
fn authenticated_exact_revision_routes_authority_before_capture_and_projects_closed_receipt() {
    let fixture = Fixture::new();
    let (mut service, observations) = authenticated_service(&fixture, false);
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) =
        service.execute(request(client, 1, CommandV1::WorkspaceInventory))
    else {
        panic!("inventory refused")
    };
    let ReplyV1::Accepted(OutcomeV1::Snapshot(before)) =
        service.execute(request(client, 2, CommandV1::Snapshot))
    else {
        panic!("snapshot refused")
    };
    let intent = DurableEffectIntentV1::ExportDocumentCopyExclusive {
        document: inventory.documents[0],
        expected_revision: before.revision,
        destination: fixture.destination.to_string_lossy().into_owned(),
    };
    let ordinary_before_export = observations.borrow().ordinary_authority_calls;
    let wrong = DurableEffectIntentV1::ExportDocumentCopyExclusive {
        document: inventory.documents[0],
        expected_revision: before.revision,
        destination: fixture
            .destination
            .with_file_name("wrong-destination-private.txt")
            .to_string_lossy()
            .into_owned(),
    };
    assert_eq!(
        service.execute_durable_effect(client, &wrong),
        DurableExecuteOutcomeV1::Refused(RefusalV1::AuthorityDenied)
    );
    assert_eq!(
        observations.borrow().ordinary_authority_calls,
        ordinary_before_export,
        "composite denial must not consult or consume generic authority"
    );
    assert_eq!(observations.borrow().effect_calls, 0);
    let outcome = service.execute_durable_effect(client, &intent);
    let DurableExecuteOutcomeV1::Completed(DurableEffectReceiptV1::DocumentCopyExportedExclusive(
        receipt,
    )) = &outcome
    else {
        panic!("export route did not return its closed receipt: {outcome:?}")
    };
    assert_eq!(receipt.document(), inventory.documents[0]);
    assert_eq!(receipt.source_revision(), before.revision);
    assert_eq!(
        receipt.persistence(),
        DurableExportCopyPersistenceV1::NotAttempted
    );
    assert_eq!(
        receipt.failure(),
        Some(DurableExportCopyFailureV1::Unsupported)
    );
    let observed = observations.borrow();
    assert_eq!(observed.authority_calls, 2);
    assert_eq!(observed.effect_calls, 1);
    assert_eq!(observed.revision, Some(before.revision));
    assert_eq!(observed.bytes, b"private-source-content");
    assert_eq!(observed.ordinary_authority_calls, ordinary_before_export);
    drop(observed);
    assert!(!fixture.destination.exists());

    let ReplyV1::Accepted(OutcomeV1::Snapshot(after)) =
        service.execute(request(client, 3, CommandV1::Snapshot))
    else {
        panic!("post-export snapshot refused")
    };
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.text, before.text);
    let debug = format!("{outcome:?}");
    assert!(!debug.contains(fixture.destination.to_str().unwrap()));
    assert!(!debug.contains("private-source-content"));
}

#[test]
fn stale_and_denied_exports_are_pre_effect_and_preserve_workspace_state() {
    let fixture = Fixture::new();
    let (mut service, observations) = authenticated_service(&fixture, true);
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) =
        service.execute(request(client, 1, CommandV1::WorkspaceInventory))
    else {
        panic!("inventory refused")
    };
    let ReplyV1::Accepted(OutcomeV1::Snapshot(before)) =
        service.execute(request(client, 2, CommandV1::Snapshot))
    else {
        panic!("snapshot refused")
    };
    let destination = fixture.destination.to_string_lossy().into_owned();
    assert_eq!(
        service.execute_durable_effect(
            client,
            &DurableEffectIntentV1::ExportDocumentCopyExclusive {
                document: inventory.documents[0],
                expected_revision: before.revision + 1,
                destination: destination.clone(),
            },
        ),
        DurableExecuteOutcomeV1::Refused(RefusalV1::Stale)
    );
    assert_eq!(observations.borrow().authority_calls, 0);
    assert_eq!(
        service.execute_durable_effect(
            client,
            &DurableEffectIntentV1::ExportDocumentCopyExclusive {
                document: inventory.documents[0],
                expected_revision: before.revision,
                destination,
            },
        ),
        DurableExecuteOutcomeV1::Refused(RefusalV1::AuthorityDenied)
    );
    let observed = observations.borrow();
    assert_eq!(observed.authority_calls, 1);
    assert_eq!(observed.effect_calls, 0);
    drop(observed);
    assert!(!fixture.destination.exists());
    let ReplyV1::Accepted(OutcomeV1::Snapshot(after)) =
        service.execute(request(client, 3, CommandV1::Snapshot))
    else {
        panic!("post-refusal snapshot refused")
    };
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.text, before.text);
}

#[test]
fn unauthenticated_service_keeps_export_protocol_unreachable() {
    let fixture = Fixture::new();
    let workspace = Workspace::open(
        std::slice::from_ref(&fixture.source),
        vellum_editor::reference_workspace_profile(
            LocalFilesystemFactory::new(FilesystemLimits {
                max_resources: 8,
                max_bytes: 4096,
                max_stage_attempts: 8,
                max_metadata_bytes: 4096,
            })
            .erase(),
        ),
    )
    .unwrap();
    let mut service = Service::new_with_action_ceiling(
        workspace,
        ServiceLimits::reference(),
        AuthorityActionSetV1::top(),
    )
    .unwrap();
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) =
        service.execute(request(client, 1, CommandV1::WorkspaceInventory))
    else {
        panic!("inventory refused")
    };
    assert_eq!(
        service.execute_durable_effect(
            client,
            &DurableEffectIntentV1::ExportDocumentCopyExclusive {
                document: inventory.documents[0],
                expected_revision: 1,
                destination: fixture.destination.to_string_lossy().into_owned(),
            },
        ),
        DurableExecuteOutcomeV1::Refused(RefusalV1::AuthorityUnavailable)
    );
    assert!(!fixture.destination.exists());
}
