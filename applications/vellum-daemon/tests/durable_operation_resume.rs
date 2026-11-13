#![cfg(target_os = "macos")]

use std::{
    fs,
    num::NonZeroU64,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use vellum_adapter_local_fs::{
    FilesystemLimits, LocalFilesystemFactory, LocalRequestJournalV1,
    LocalWorkspaceIdentityMetadataV1,
};
use vellum_daemon::{
    AuthenticatedUnixConnector, DaemonBuildError, DaemonExportPolicyV1, DaemonHealth, DaemonLimits,
    DaemonReadiness, DaemonServeError, DaemonUsage, LocalAuthenticationClaimV1, LocalDaemon,
    RequestLatencyProfile,
};
use vellum_editor::Workspace;
use vellum_host::{
    authentication::{
        AuthenticationDecisionV1, AuthenticationEvidenceV1, AuthenticationPortV1,
        AuthenticationRequestV1,
    },
    authority::{AuthorityActionSetV1, AuthorityActionV1, AuthoritySubjectRefV1},
    request_journal::{
        DurableOperationCompletionV1, DurableOperationIntentV1, DurableOperationRecordV1,
        DurableOperationRecoveryV1, DurableOperationRetirementRecoveryV1,
        DurableOperationRetirementResultV1, DurableOperationRetirementV1, DurableOperationSortV1,
        RequestJournalCompletionV1, RequestJournalIntentV1, RequestJournalLimitsV1,
        RequestJournalPortV1, RequestJournalRecoveryV1, RequestJournalRefusalV1,
        RequestJournalScopeV1, encode_durable_operation_envelope_v1,
    },
    workspace::{
        WorkspaceIdentityLimitsV1, WorkspaceIdentityMetadataPortV1, WorkspaceIdentityRecoveryV1,
    },
};
use vellum_protocol::{
    ClientId, CommandV1, DurableEffectIntentV1, DurableEffectReceiptV1, DurableExecuteOutcomeV1,
    DurableExecuteRequestV1, DurableOperationRefV1, DurableReceiptLookupOutcomeV1,
    DurableReceiptLookupV1, DurableReceiptRetirementOutcomeV1, DurableReceiptRetirementV1,
    DurableSaveAsBindingV1, DurableSaveAsDestinationEffectV1, DurableSaveAsFailureV1,
    DurableSaveAsIntentV1, DurableSaveAsLifecycleV1, DurableSaveAsLookupOutcomeV1,
    DurableSaveAsLookupV1, DurableSaveAsNotAttemptedReasonV1, DurableSaveAsPersistenceV1,
    DurableSaveAsReceiptV1, DurableSaveAsRetirementV1, DurableSaveAsStateV1,
    FileAdmissionReceiptV1, FileAdmissionStatusV1, RefusalV1, ReplyV1, RequestId, RequestV1,
    ServicePort, VersionWitness, WorkspaceDocumentIdV1, WorkspaceIdV1,
};
use vellum_service::ServiceLimits;
use vellum_transport::{FrameExchange, RemoteTransport, TransportError, TransportLimits};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct LoseDurableExecuteReply<X> {
    inner: X,
}

impl<X: FrameExchange> FrameExchange for LoseDurableExecuteReply<X> {
    fn exchange(
        &mut self,
        request: &[u8],
        max_reply_bytes: usize,
    ) -> Result<Vec<u8>, TransportError> {
        let lose = matches!(
            vellum_wire::decode_request_v1(request, vellum_wire::WireLimits::reference()),
            Ok(vellum_wire::WireRequestV1::DurableExecute(_))
        );
        let reply = self.inner.exchange(request, max_reply_bytes)?;
        if lose {
            Err(TransportError::OutcomeUnknown)
        } else {
            Ok(reply)
        }
    }
}

struct Fixture {
    directory: PathBuf,
    source: PathBuf,
    socket: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "vellum-durable-resume-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let source = directory.join("source.txt");
        fs::write(&source, "alpha").unwrap();
        let socket = directory.join("service.sock");
        Self {
            directory,
            source,
            socket,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

struct SubjectAuthentication;

#[derive(Default)]
struct CleanupOutcomeUnknownJournal(LocalRequestJournalV1);

impl RequestJournalPortV1 for CleanupOutcomeUnknownJournal {
    fn recover(
        &mut self,
        directory: &std::path::Path,
        scope: RequestJournalScopeV1,
        limits: RequestJournalLimitsV1,
    ) -> RequestJournalRecoveryV1 {
        self.0.recover(directory, scope, limits)
    }

    fn begin(
        &mut self,
        directory: &std::path::Path,
        intent: RequestJournalIntentV1<'_>,
        limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        self.0.begin(directory, intent, limits)
    }

    fn complete(
        &mut self,
        directory: &std::path::Path,
        completion: RequestJournalCompletionV1<'_>,
        limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        self.0.complete(directory, completion, limits)
    }

    fn recover_operations(
        &mut self,
        directory: &std::path::Path,
        scope: RequestJournalScopeV1,
        limits: RequestJournalLimitsV1,
    ) -> DurableOperationRecoveryV1 {
        self.0.recover_operations(directory, scope, limits)
    }

    fn begin_operation(
        &mut self,
        directory: &std::path::Path,
        intent: DurableOperationIntentV1<'_>,
        limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        self.0.begin_operation(directory, intent, limits)
    }

    fn complete_operation(
        &mut self,
        directory: &std::path::Path,
        completion: DurableOperationCompletionV1<'_>,
        limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        self.0.complete_operation(directory, completion, limits)
    }

    fn recover_operation_retirements(
        &mut self,
        directory: &std::path::Path,
        scope: RequestJournalScopeV1,
        limits: RequestJournalLimitsV1,
    ) -> DurableOperationRetirementRecoveryV1 {
        self.0
            .recover_operation_retirements(directory, scope, limits)
    }

    fn retire_operation(
        &mut self,
        directory: &std::path::Path,
        retirement: DurableOperationRetirementV1<'_>,
        limits: RequestJournalLimitsV1,
    ) -> Result<DurableOperationRetirementResultV1, RequestJournalRefusalV1> {
        match self.0.retire_operation(directory, retirement, limits)? {
            DurableOperationRetirementResultV1::Retired => {
                Err(RequestJournalRefusalV1::OutcomeUnknown)
            }
            other => Ok(other),
        }
    }
}

impl AuthenticationPortV1 for SubjectAuthentication {
    type Context = ([u8; 16], [u8; 16]);

    fn authenticate(
        &mut self,
        request: AuthenticationRequestV1,
        context: &Self::Context,
    ) -> AuthenticationDecisionV1 {
        if request.claim != vellum_host::authentication::AuthenticationClaimRefV1::new(context.0) {
            return AuthenticationDecisionV1::Rejected;
        }
        AuthenticationDecisionV1::Authenticated(AuthenticationEvidenceV1::from_port(
            request,
            AuthoritySubjectRefV1::new(context.1),
        ))
    }
}

fn workspace(path: &PathBuf) -> Workspace {
    Workspace::open(
        std::slice::from_ref(path),
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
    .unwrap()
}

fn empty_workspace() -> Workspace {
    Workspace::empty(vellum_editor::reference_workspace_profile(
        LocalFilesystemFactory::new(FilesystemLimits {
            max_resources: 8,
            max_bytes: 4096,
            max_stage_attempts: 8,
            max_metadata_bytes: 4096,
        })
        .erase(),
    ))
    .unwrap()
}

fn limits() -> DaemonLimits {
    DaemonLimits {
        service: ServiceLimits::reference(),
        transport: TransportLimits::reference(),
        io_timeout: Duration::from_secs(2),
        max_in_flight_connections: 1,
        max_startup_roots: 8,
        max_authentication_attempts: 32,
        workspace_identity: WorkspaceIdentityLimitsV1 {
            max_documents: 256,
            max_roots: 256,
            max_encoded_bytes: 1024,
        },
        request_journal: RequestJournalLimitsV1 {
            max_entries: 8,
            max_retired_operations: 32,
            max_request_bytes: 65_600,
            max_reply_bytes: 65_600,
            max_encoded_bytes: 131_264,
        },
        request_latency: Some(RequestLatencyProfile::reference()),
    }
}

fn actions() -> AuthorityActionSetV1 {
    AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::PersistSource,
    ))
}

fn recovered_scope_and_document(
    fixture: &Fixture,
) -> (RequestJournalScopeV1, WorkspaceDocumentIdV1) {
    let mut metadata = LocalWorkspaceIdentityMetadataV1::default();
    let WorkspaceIdentityRecoveryV1::Recovered(identity) =
        metadata.load(&fixture.directory, limits().workspace_identity)
    else {
        panic!("expected recovered workspace identity")
    };
    let scope = RequestJournalScopeV1::new(identity.workspace(), identity.generation()).unwrap();
    let document = WorkspaceDocumentIdV1::new(
        WorkspaceIdV1::new(identity.workspace()).unwrap(),
        identity.document_bindings()[0].ordinal(),
    )
    .unwrap();
    (scope, document)
}

fn inject_operation(
    fixture: &Fixture,
    operation: DurableOperationRefV1,
    subject: [u8; 16],
    intent: &DurableEffectIntentV1,
    completion: Option<&DurableExecuteOutcomeV1>,
) {
    let (scope, _) = recovered_scope_and_document(fixture);
    let frozen_request =
        vellum_wire::encode_durable_intent_v1(intent, vellum_wire::WireLimits::reference())
            .unwrap();
    let durable_intent = DurableOperationIntentV1 {
        scope,
        operation,
        authenticated_subject: subject,
        frozen_request: &frozen_request,
    };
    let mut journal = LocalRequestJournalV1::default();
    journal
        .begin_operation(&fixture.directory, durable_intent, limits().request_journal)
        .unwrap();
    if let Some(completion) = completion {
        let frozen_reply = vellum_wire::encode_durable_outcome_v1(
            completion,
            vellum_wire::WireLimits::reference(),
        )
        .unwrap();
        journal
            .complete_operation(
                &fixture.directory,
                DurableOperationCompletionV1 {
                    intent: durable_intent,
                    frozen_reply: &frozen_reply,
                },
                limits().request_journal,
            )
            .unwrap();
    }
}

fn inject_save_as_operation(
    fixture: &Fixture,
    operation: DurableOperationRefV1,
    subject: [u8; 16],
    intent: &DurableSaveAsIntentV1,
    completion: Option<DurableSaveAsReceiptV1>,
) {
    let (scope, _) = recovered_scope_and_document(fixture);
    let payload =
        vellum_wire::encode_durable_save_as_intent_v1(intent, vellum_wire::WireLimits::reference())
            .unwrap();
    let frozen_request = encode_durable_operation_envelope_v1(
        DurableOperationSortV1::SaveAs,
        &payload,
        limits().request_journal.max_request_bytes,
    )
    .unwrap();
    let durable_intent = DurableOperationIntentV1 {
        scope,
        operation,
        authenticated_subject: subject,
        frozen_request: &frozen_request,
    };
    let mut journal = LocalRequestJournalV1::default();
    journal
        .begin_operation(&fixture.directory, durable_intent, limits().request_journal)
        .unwrap();
    if let Some(receipt) = completion {
        let frozen_reply = vellum_wire::encode_durable_save_as_receipt_v1(
            receipt,
            vellum_wire::WireLimits::reference(),
        )
        .unwrap();
        journal
            .complete_operation(
                &fixture.directory,
                DurableOperationCompletionV1 {
                    intent: durable_intent,
                    frozen_reply: &frozen_reply,
                },
                limits().request_journal,
            )
            .unwrap();
    }
}

fn inject_legacy_receipt(fixture: &Fixture, subject: [u8; 16]) {
    let (scope, _) = recovered_scope_and_document(fixture);
    let client = NonZeroU64::new(1).unwrap();
    let request_id = NonZeroU64::new(1).unwrap();
    let request = RequestV1 {
        client: ClientId::new(client.get()).unwrap(),
        request: RequestId::new(request_id.get()).unwrap(),
        command: CommandV1::Save {
            expected: VersionWitness::new(1).unwrap(),
        },
    };
    let frozen_request = vellum_wire::encode_request_v1(
        &vellum_wire::WireRequestV1::Execute(request),
        vellum_wire::WireLimits::reference(),
    )
    .unwrap();
    let frozen_reply = vellum_wire::encode_reply_v1(
        &vellum_wire::WireReplyV1::Executed(ReplyV1::Refused(RefusalV1::UnknownWitness)),
        vellum_wire::WireLimits::reference(),
    )
    .unwrap();
    let intent = RequestJournalIntentV1 {
        scope,
        client,
        request: request_id,
        authenticated_subject: subject,
        frozen_request: &frozen_request,
    };
    let mut journal = LocalRequestJournalV1::default();
    journal
        .begin(&fixture.directory, intent, limits().request_journal)
        .unwrap();
    journal
        .complete(
            &fixture.directory,
            RequestJournalCompletionV1 {
                intent,
                frozen_reply: &frozen_reply,
            },
            limits().request_journal,
        )
        .unwrap();
}

fn authenticated_transport(
    socket: PathBuf,
    claim: [u8; 16],
) -> RemoteTransport<AuthenticatedUnixConnector> {
    let connector = AuthenticatedUnixConnector::with_timeout(
        socket,
        LocalAuthenticationClaimV1::new(claim),
        Duration::from_secs(2),
    )
    .unwrap();
    RemoteTransport::new(connector, TransportLimits::reference()).unwrap()
}

fn exact_export_policy(
    operation: DurableOperationRefV1,
    destination: PathBuf,
) -> DaemonExportPolicyV1 {
    DaemonExportPolicyV1::exact_create_exclusive(
        operation,
        1,
        0,
        destination,
        FilesystemLimits {
            max_resources: 1,
            max_bytes: 4096,
            max_stage_attempts: 8,
            max_metadata_bytes: 4096,
        },
        8,
    )
    .unwrap()
}

#[test]
fn authenticated_exact_export_is_operation_bound_and_restart_replays_without_republication() {
    let fixture = Fixture::new();
    let destination = fs::canonicalize(&fixture.directory)
        .unwrap()
        .join("exported-copy.txt");
    let claim = [0xa1; 16];
    let subject = [0xa2; 16];
    let operation = DurableOperationRefV1::new([0xa3; 16]).unwrap();
    let wrong_operation = DurableOperationRefV1::new([0xa4; 16]).unwrap();

    let mut daemon = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        AuthorityActionSetV1::read_only(),
    )
    .unwrap()
    .with_exact_export_policy(exact_export_policy(operation, destination.clone()))
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 8);
    let (_, document) = recovered_scope_and_document(&fixture);
    let intent = DurableEffectIntentV1::ExportDocumentCopyExclusive {
        document,
        expected_revision: 0,
        destination: destination.to_str().unwrap().to_owned(),
    };
    let socket = fixture.socket.clone();
    let first_intent = intent.clone();
    let first = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .execute_durable(DurableExecuteRequestV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: first_intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        daemon.serve_next().unwrap();
    }
    let first = first.join().unwrap();
    assert!(
        matches!(first, DurableExecuteOutcomeV1::Completed(_)),
        "unexpected first export outcome: {first:?}"
    );
    assert_eq!(fs::read_to_string(&destination).unwrap(), "alpha");
    let usage = daemon.service_usage();
    assert!(usage.export_validation_observations >= 1);
    assert!(usage.export_staging_observations >= 1);
    assert!(usage.export_publication_observations >= 1);
    assert!(usage.export_synchronization_observations >= 1);
    assert!(usage.export_observation_latency_samples >= 4);

    let socket = fixture.socket.clone();
    let wrong_intent = intent.clone();
    let wrong = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .execute_durable(DurableExecuteRequestV1 {
                operation: wrong_operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: wrong_intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        daemon.serve_next().unwrap();
    }
    assert_eq!(
        wrong.join().unwrap(),
        DurableExecuteOutcomeV1::Refused(RefusalV1::AuthorityDenied)
    );
    assert_eq!(fs::read_to_string(&destination).unwrap(), "alpha");
    drop(daemon);

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        AuthorityActionSetV1::read_only(),
    )
    .unwrap()
    .with_exact_export_policy(exact_export_policy(operation, destination.clone()))
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 4);
    let socket = fixture.socket.clone();
    let replay = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .execute_durable(DurableExecuteRequestV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        restarted.serve_next().unwrap();
    }
    assert_eq!(replay.join().unwrap(), first);
    assert_eq!(restarted.service_usage().durable_effect_requests, 0);
    assert_eq!(fs::read_to_string(&destination).unwrap(), "alpha");
}

#[test]
fn lost_export_reply_is_recovered_after_restart_without_second_publication_or_state_change() {
    let fixture = Fixture::new();
    let destination = fs::canonicalize(&fixture.directory)
        .unwrap()
        .join("lost-reply-export.txt");
    let claim = [0xd1; 16];
    let subject = [0xd2; 16];
    let operation = DurableOperationRefV1::new([0xd3; 16]).unwrap();

    let mut first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        AuthorityActionSetV1::read_only(),
    )
    .unwrap()
    .with_exact_export_policy(exact_export_policy(operation, destination.clone()))
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 8);
    let (_, document) = recovered_scope_and_document(&fixture);
    let intent = DurableEffectIntentV1::ExportDocumentCopyExclusive {
        document,
        expected_revision: 0,
        destination: destination.to_str().unwrap().to_owned(),
    };
    let socket = fixture.socket.clone();
    let first_intent = intent.clone();
    let first_client = std::thread::spawn(move || {
        let connector = AuthenticatedUnixConnector::with_timeout(
            socket,
            LocalAuthenticationClaimV1::new(claim),
            Duration::from_secs(2),
        )
        .unwrap();
        let mut transport = RemoteTransport::new(
            LoseDurableExecuteReply { inner: connector },
            TransportLimits::reference(),
        )
        .unwrap();
        let client = transport.connect().unwrap();
        let ReplyV1::Accepted(vellum_protocol::OutcomeV1::WorkspaceInventory(inventory)) =
            transport
                .execute(RequestV1 {
                    client,
                    request: RequestId::new(1).unwrap(),
                    command: CommandV1::WorkspaceInventory,
                })
                .unwrap()
        else {
            panic!("expected pre-export inventory")
        };
        let ReplyV1::Accepted(vellum_protocol::OutcomeV1::Snapshot(snapshot)) = transport
            .execute(RequestV1 {
                client,
                request: RequestId::new(2).unwrap(),
                command: CommandV1::Snapshot,
            })
            .unwrap()
        else {
            panic!("expected pre-export snapshot")
        };
        assert_eq!(
            transport.execute_durable(DurableExecuteRequestV1 {
                operation,
                client,
                request: RequestId::new(3).unwrap(),
                intent: first_intent,
            }),
            Err(vellum_protocol::ServiceAccessErrorV1::OutcomeUnknown)
        );
        (inventory, snapshot)
    });
    for _ in 0..4 {
        first.serve_next().unwrap();
    }
    let (inventory_before, snapshot_before) = first_client.join().unwrap();
    assert_eq!(fs::read_to_string(&destination).unwrap(), "alpha");
    assert_eq!(fs::read_to_string(&fixture.source).unwrap(), "alpha");
    assert_eq!(first.service_usage().durable_effect_requests, 1);
    assert_eq!(first.operational_snapshot().utilization.durable_receipts, 1);
    drop(first);

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        AuthorityActionSetV1::read_only(),
    )
    .unwrap()
    .with_exact_export_policy(exact_export_policy(operation, destination.clone()))
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 8);
    let socket = fixture.socket.clone();
    let lookup_intent = intent.clone();
    let recovery_client = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        let recovered = transport
            .lookup_durable_receipt(DurableReceiptLookupV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: lookup_intent,
            })
            .unwrap();
        let ReplyV1::Accepted(vellum_protocol::OutcomeV1::WorkspaceInventory(inventory)) =
            transport
                .execute(RequestV1 {
                    client,
                    request: RequestId::new(2).unwrap(),
                    command: CommandV1::WorkspaceInventory,
                })
                .unwrap()
        else {
            panic!("expected recovered inventory")
        };
        let ReplyV1::Accepted(vellum_protocol::OutcomeV1::Snapshot(snapshot)) = transport
            .execute(RequestV1 {
                client,
                request: RequestId::new(3).unwrap(),
                command: CommandV1::Snapshot,
            })
            .unwrap()
        else {
            panic!("expected recovered snapshot")
        };
        (recovered, inventory, snapshot)
    });
    for _ in 0..4 {
        restarted.serve_next().unwrap();
    }
    let (recovered, inventory_after, snapshot_after) = recovery_client.join().unwrap();
    assert!(matches!(
        recovered,
        DurableReceiptLookupOutcomeV1::Complete(DurableExecuteOutcomeV1::Completed(
            DurableEffectReceiptV1::DocumentCopyExportedExclusive(_)
        ))
    ));
    assert_eq!(inventory_after, inventory_before);
    assert_eq!(snapshot_after.revision, snapshot_before.revision);
    assert_eq!(snapshot_after.text, snapshot_before.text);
    assert_eq!(fs::read_to_string(&fixture.source).unwrap(), "alpha");
    assert_eq!(fs::read_to_string(&destination).unwrap(), "alpha");
    assert_eq!(restarted.service_usage().durable_effect_requests, 0);
    assert_eq!(restarted.usage().durable_operation_lookups, 1);
    assert_eq!(restarted.usage().durable_lookup_complete, 1);
}

#[test]
fn pending_export_is_outcome_unknown_after_restart_and_never_publishes() {
    let fixture = Fixture::new();
    let destination = fs::canonicalize(&fixture.directory)
        .unwrap()
        .join("pending-export.txt");
    let claim = [0xe1; 16];
    let subject = [0xe2; 16];
    let operation = DurableOperationRefV1::new([0xe3; 16]).unwrap();

    let initial = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        AuthorityActionSetV1::read_only(),
    )
    .unwrap();
    drop(initial);
    let (_, document) = recovered_scope_and_document(&fixture);
    let intent = DurableEffectIntentV1::ExportDocumentCopyExclusive {
        document,
        expected_revision: 0,
        destination: destination.to_str().unwrap().to_owned(),
    };
    inject_operation(&fixture, operation, subject, &intent, None);

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        AuthorityActionSetV1::read_only(),
    )
    .unwrap()
    .with_exact_export_policy(exact_export_policy(operation, destination.clone()))
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 4);
    let socket = fixture.socket.clone();
    let lookup = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .lookup_durable_receipt(DurableReceiptLookupV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        restarted.serve_next().unwrap();
    }
    assert_eq!(
        lookup.join().unwrap(),
        DurableReceiptLookupOutcomeV1::OutcomeUnknown
    );
    assert!(!destination.exists());
    assert_eq!(fs::read_to_string(&fixture.source).unwrap(), "alpha");
    assert_eq!(restarted.service_usage().durable_effect_requests, 0);
    assert_eq!(restarted.usage().durable_lookup_pending, 1);
}

#[test]
fn recovered_save_as_pending_is_unknown_and_changed_input_conflicts() {
    let fixture = Fixture::new();
    let claim = [0x31; 16];
    let subject = [0x32; 16];
    let operation = DurableOperationRefV1::new([0x33; 16]).unwrap();
    let initial = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        AuthorityActionSetV1::read_only(),
    )
    .unwrap();
    drop(initial);
    let (_, document) = recovered_scope_and_document(&fixture);
    let intent = DurableSaveAsIntentV1::try_new(
        document,
        0,
        fixture
            .directory
            .join("save-as-pending.txt")
            .to_str()
            .unwrap()
            .to_owned(),
        DurableSaveAsDestinationEffectV1::CreateExclusive,
    )
    .unwrap();
    inject_save_as_operation(&fixture, operation, subject, &intent, None);
    inject_operation(
        &fixture,
        DurableOperationRefV1::new([0x34; 16]).unwrap(),
        subject,
        &DurableEffectIntentV1::SaveDocument {
            document,
            expected_revision: 0,
        },
        None,
    );

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        AuthorityActionSetV1::read_only(),
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 4);
    let socket = fixture.socket.clone();
    let exact = intent.clone();
    let changed = DurableSaveAsIntentV1::try_new(
        document,
        0,
        fixture
            .directory
            .join("changed-destination.txt")
            .to_str()
            .unwrap()
            .to_owned(),
        DurableSaveAsDestinationEffectV1::CreateExclusive,
    )
    .unwrap();
    let lookup = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        let pending = transport
            .lookup_durable_save_as(DurableSaveAsLookupV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: exact,
            })
            .unwrap();
        let conflict = transport
            .lookup_durable_save_as(DurableSaveAsLookupV1 {
                operation,
                client,
                request: RequestId::new(2).unwrap(),
                intent: changed,
            })
            .unwrap();
        (pending, conflict)
    });
    for _ in 0..3 {
        restarted.serve_next().unwrap();
    }
    assert_eq!(
        lookup.join().unwrap(),
        (
            DurableSaveAsLookupOutcomeV1::OutcomeUnknown,
            DurableSaveAsLookupOutcomeV1::RequestConflict,
        )
    );
    assert_eq!(restarted.usage().durable_lookup_pending, 1);
    assert_eq!(restarted.usage().durable_lookup_conflict, 1);
}

#[test]
fn completed_save_as_receipt_retires_once_and_tombstone_survives_restart() {
    let fixture = Fixture::new();
    let claim = [0x41; 16];
    let subject = [0x42; 16];
    let operation = DurableOperationRefV1::new([0x43; 16]).unwrap();
    let authority = AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::RetireReceipt,
    ));
    let initial = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap();
    drop(initial);
    let (_, document) = recovered_scope_and_document(&fixture);
    let intent = DurableSaveAsIntentV1::try_new(
        document,
        0,
        fixture
            .directory
            .join("denied-save-as.txt")
            .to_str()
            .unwrap()
            .to_owned(),
        DurableSaveAsDestinationEffectV1::OverwriteExisting,
    )
    .unwrap();
    let receipt = DurableSaveAsReceiptV1::try_new(
        document,
        0,
        DurableSaveAsPersistenceV1::NotAttempted,
        DurableSaveAsBindingV1::NotActivated,
        Some(DurableSaveAsFailureV1::NotAttempted(
            DurableSaveAsNotAttemptedReasonV1::AuthorityDenied,
        )),
        DurableSaveAsLifecycleV1::TerminalReplayable,
    )
    .unwrap();
    inject_save_as_operation(&fixture, operation, subject, &intent, Some(receipt));

    let mut first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 8);
    let socket = fixture.socket.clone();
    let first_intent = intent.clone();
    let exercise = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        let before = transport
            .lookup_durable_save_as(DurableSaveAsLookupV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: first_intent.clone(),
            })
            .unwrap();
        let retired = transport
            .retire_durable_save_as(DurableSaveAsRetirementV1 {
                operation,
                client,
                request: RequestId::new(2).unwrap(),
                intent: first_intent.clone(),
            })
            .unwrap();
        let after = transport
            .lookup_durable_save_as(DurableSaveAsLookupV1 {
                operation,
                client,
                request: RequestId::new(3).unwrap(),
                intent: first_intent.clone(),
            })
            .unwrap();
        let repeated = transport
            .retire_durable_save_as(DurableSaveAsRetirementV1 {
                operation,
                client,
                request: RequestId::new(4).unwrap(),
                intent: first_intent,
            })
            .unwrap();
        (before, retired, after, repeated)
    });
    for _ in 0..5 {
        first.serve_next().unwrap();
    }
    assert_eq!(
        exercise.join().unwrap(),
        (
            DurableSaveAsLookupOutcomeV1::Observed(DurableSaveAsStateV1::Complete(receipt)),
            DurableReceiptRetirementOutcomeV1::Retired,
            DurableSaveAsLookupOutcomeV1::Observed(DurableSaveAsStateV1::Retired),
            DurableReceiptRetirementOutcomeV1::AbsentOrForeign,
        )
    );
    assert_eq!(first.usage().durable_retirement_retired, 1);
    drop(first);

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 4);
    let socket = fixture.socket.clone();
    let replay = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .lookup_durable_save_as(DurableSaveAsLookupV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        restarted.serve_next().unwrap();
    }
    assert_eq!(
        replay.join().unwrap(),
        DurableSaveAsLookupOutcomeV1::Observed(DurableSaveAsStateV1::Retired)
    );
}

struct DuplicateOperationJournal {
    records: Vec<DurableOperationRecordV1>,
}

impl RequestJournalPortV1 for DuplicateOperationJournal {
    fn recover(
        &mut self,
        _directory: &std::path::Path,
        _scope: RequestJournalScopeV1,
        _limits: RequestJournalLimitsV1,
    ) -> RequestJournalRecoveryV1 {
        RequestJournalRecoveryV1::Empty
    }

    fn begin(
        &mut self,
        _directory: &std::path::Path,
        _intent: RequestJournalIntentV1<'_>,
        _limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        Err(RequestJournalRefusalV1::UnsupportedResource)
    }

    fn complete(
        &mut self,
        _directory: &std::path::Path,
        _completion: RequestJournalCompletionV1<'_>,
        _limits: RequestJournalLimitsV1,
    ) -> Result<(), RequestJournalRefusalV1> {
        Err(RequestJournalRefusalV1::UnsupportedResource)
    }

    fn recover_operations(
        &mut self,
        _directory: &std::path::Path,
        _scope: RequestJournalScopeV1,
        _limits: RequestJournalLimitsV1,
    ) -> DurableOperationRecoveryV1 {
        DurableOperationRecoveryV1::Recovered(self.records.clone())
    }

    fn recover_operation_retirements(
        &mut self,
        _directory: &std::path::Path,
        _scope: RequestJournalScopeV1,
        _limits: RequestJournalLimitsV1,
    ) -> DurableOperationRetirementRecoveryV1 {
        DurableOperationRetirementRecoveryV1::Empty
    }
}

#[test]
fn pending_operation_survives_restart_and_lookup_never_executes_it() {
    let fixture = Fixture::new();
    let initial = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions(),
    )
    .unwrap();
    drop(initial);

    let (_, document) = recovered_scope_and_document(&fixture);
    let operation = DurableOperationRefV1::new([0x31; 16]).unwrap();
    let absent = DurableOperationRefV1::new([0x32; 16]).unwrap();
    let owner = [0x41; 16];
    let foreign = [0x42; 16];
    let claim = [0x51; 16];
    let intent = DurableEffectIntentV1::SaveDocument {
        document,
        expected_revision: 0,
    };
    inject_operation(&fixture, operation, owner, &intent, None);

    let mut foreign_daemon = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions(),
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, foreign), 3);
    let socket = fixture.socket.clone();
    let foreign_intent = intent.clone();
    let foreign_client = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        let hidden = transport
            .lookup_durable_receipt(DurableReceiptLookupV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: foreign_intent.clone(),
            })
            .unwrap();
        let truly_absent = transport
            .lookup_durable_receipt(DurableReceiptLookupV1 {
                operation: absent,
                client,
                request: RequestId::new(2).unwrap(),
                intent: foreign_intent,
            })
            .unwrap();
        (hidden, truly_absent)
    });
    for _ in 0..3 {
        foreign_daemon.serve_next().unwrap();
    }
    let (hidden, absent_outcome) = foreign_client.join().unwrap();
    assert_eq!(hidden, DurableReceiptLookupOutcomeV1::AbsentOrForeign);
    assert_eq!(hidden, absent_outcome);
    assert_eq!(foreign_daemon.service_usage().durable_effect_requests, 0);
    drop(foreign_daemon);

    let mut one_slot_limits = limits();
    one_slot_limits.request_journal.max_entries = 1;
    let mut owner_daemon = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        one_slot_limits,
        actions(),
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, owner), 2);
    let socket = fixture.socket.clone();
    let owner_client = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .lookup_durable_receipt(DurableReceiptLookupV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        owner_daemon.serve_next().unwrap();
    }
    assert_eq!(
        owner_client.join().unwrap(),
        DurableReceiptLookupOutcomeV1::OutcomeUnknown
    );
    assert_eq!(owner_daemon.service_usage().durable_effect_requests, 0);
    let snapshot = owner_daemon.operational_snapshot();
    assert_eq!(snapshot.utilization.durable_receipts, 1);
    assert_eq!(snapshot.utilization.durable_receipt_capacity, 1);
    assert!(snapshot.saturation.durable_receipts);
    assert_eq!(fs::read_to_string(&fixture.source).unwrap(), "alpha");
}

#[test]
fn duplicate_subject_operation_keys_from_a_trusted_port_refuse_startup() {
    let fixture = Fixture::new();
    let initial = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions(),
    )
    .unwrap();
    drop(initial);

    let (scope, document) = recovered_scope_and_document(&fixture);
    let operation = DurableOperationRefV1::new([0xb1; 16]).unwrap();
    let subject = [0xb2; 16];
    let request = vellum_wire::encode_durable_intent_v1(
        &DurableEffectIntentV1::SaveDocument {
            document,
            expected_revision: 0,
        },
        vellum_wire::WireLimits::reference(),
    )
    .unwrap();
    let record = DurableOperationRecordV1 {
        scope,
        operation,
        authenticated_subject: subject,
        frozen_request: request,
        frozen_reply: None,
    };
    let journal = DuplicateOperationJournal {
        records: vec![record.clone(), record],
    };
    let error = LocalDaemon::bind_with_persistence_ports_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions(),
        LocalWorkspaceIdentityMetadataV1::default(),
        journal,
    )
    .err()
    .expect("duplicate operation relation must refuse endpoint publication");
    assert_eq!(
        error,
        DaemonBuildError::RequestJournal(RequestJournalRefusalV1::Conflict)
    );
    assert!(!fixture.socket.exists());
}

#[test]
fn future_generation_operation_from_a_trusted_port_refuses_startup() {
    let fixture = Fixture::new();
    let initial = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions(),
    )
    .unwrap();
    drop(initial);

    let (scope, document) = recovered_scope_and_document(&fixture);
    let future_scope = RequestJournalScopeV1::new(
        scope.workspace(),
        scope
            .generation()
            .checked_add(1)
            .and_then(|generation| generation.checked_add(1))
            .expect("test generation must have two successors"),
    )
    .unwrap();
    let intent = DurableEffectIntentV1::ExportDocumentCopyExclusive {
        document,
        expected_revision: 0,
        destination: fixture
            .directory
            .join("future-export.txt")
            .to_str()
            .unwrap()
            .to_owned(),
    };
    let record = DurableOperationRecordV1 {
        scope: future_scope,
        operation: DurableOperationRefV1::new([0xc1; 16]).unwrap(),
        authenticated_subject: [0xc2; 16],
        frozen_request: vellum_wire::encode_durable_intent_v1(
            &intent,
            vellum_wire::WireLimits::reference(),
        )
        .unwrap(),
        frozen_reply: None,
    };
    let error = LocalDaemon::bind_with_persistence_ports_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions(),
        LocalWorkspaceIdentityMetadataV1::default(),
        DuplicateOperationJournal {
            records: vec![record],
        },
    )
    .err()
    .expect("future-generation operation must refuse endpoint publication");
    assert_eq!(
        error,
        DaemonBuildError::RequestJournal(RequestJournalRefusalV1::Conflict)
    );
    assert!(!fixture.socket.exists());
}

#[test]
fn canonical_but_intent_mismatched_completion_refuses_startup() {
    let fixture = Fixture::new();
    let initial = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions(),
    )
    .unwrap();
    drop(initial);

    let (_, document) = recovered_scope_and_document(&fixture);
    let intent = DurableEffectIntentV1::SaveDocument {
        document,
        expected_revision: 0,
    };
    let mismatched = DurableExecuteOutcomeV1::Completed(DurableEffectReceiptV1::FileAdmitted(
        FileAdmissionReceiptV1 {
            document,
            root: None,
            status: FileAdmissionStatusV1::AlreadyPresent,
        },
    ));
    inject_operation(
        &fixture,
        DurableOperationRefV1::new([0x61; 16]).unwrap(),
        [0x71; 16],
        &intent,
        Some(&mismatched),
    );

    let error = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions(),
    )
    .err()
    .expect("intent/receipt mismatch must refuse endpoint publication");
    assert_eq!(
        error,
        DaemonBuildError::RequestJournal(RequestJournalRefusalV1::Conflict)
    );
    assert!(!fixture.socket.exists());
}

#[test]
fn durable_membership_receipt_survives_the_membership_generation_checkpoint() {
    let fixture = Fixture::new();
    let claim = [0x91; 16];
    let subject = [0x92; 16];
    let operation = DurableOperationRefV1::new([0x93; 16]).unwrap();
    let intent = DurableEffectIntentV1::AddFile {
        locator: fixture.source.to_str().unwrap().to_owned(),
    };
    let actions = AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::AdmitFile,
    ));
    let mut first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        empty_workspace(),
        limits(),
        actions,
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 2);
    let socket = fixture.socket.clone();
    let first_intent = intent.clone();
    let execute = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .execute_durable(DurableExecuteRequestV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: first_intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        first.serve_next().unwrap();
    }
    let completed = execute.join().unwrap();
    assert!(matches!(completed, DurableExecuteOutcomeV1::Completed(_)));
    drop(first);

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions,
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 2);
    let socket = fixture.socket.clone();
    let lookup = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .lookup_durable_receipt(DurableReceiptLookupV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        restarted.serve_next().unwrap();
    }
    assert_eq!(
        lookup.join().unwrap(),
        DurableReceiptLookupOutcomeV1::Complete(completed)
    );
    assert_eq!(restarted.service_usage().durable_effect_requests, 0);
}

#[test]
fn legacy_and_operation_records_share_one_capacity_without_eviction() {
    let fixture = Fixture::new();
    let initial = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions(),
    )
    .unwrap();
    drop(initial);

    let (_, document) = recovered_scope_and_document(&fixture);
    inject_legacy_receipt(&fixture, [0x81; 16]);
    inject_operation(
        &fixture,
        DurableOperationRefV1::new([0x82; 16]).unwrap(),
        [0x83; 16],
        &DurableEffectIntentV1::SaveDocument {
            document,
            expected_revision: 0,
        },
        None,
    );

    let mut one_slot_limits = limits();
    one_slot_limits.request_journal.max_entries = 1;
    let error = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        one_slot_limits,
        actions(),
    )
    .err()
    .expect("the two journal relations must share one daemon capacity");
    assert_eq!(
        error,
        DaemonBuildError::RequestJournal(RequestJournalRefusalV1::Limit)
    );
    assert!(!fixture.socket.exists());

    let (scope, _) = recovered_scope_and_document(&fixture);
    let mut journal = LocalRequestJournalV1::default();
    let RequestJournalRecoveryV1::Recovered(legacy) =
        journal.recover(&fixture.directory, scope, one_slot_limits.request_journal)
    else {
        panic!("legacy record must remain after capacity refusal")
    };
    let DurableOperationRecoveryV1::Recovered(operations) =
        journal.recover_operations(&fixture.directory, scope, one_slot_limits.request_journal)
    else {
        panic!("operation record must remain after capacity refusal")
    };
    assert_eq!(legacy.len(), 1);
    assert_eq!(operations.len(), 1);
}

#[test]
fn the_same_operation_ref_is_independent_across_authenticated_subjects() {
    let fixture = Fixture::new();
    let claim = [0xa1; 16];
    let subject_a = [0xa2; 16];
    let subject_b = [0xa3; 16];
    let operation = DurableOperationRefV1::new([0xa4; 16]).unwrap();
    let authority = actions().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::EditSource,
    ));

    let initial = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap();
    drop(initial);
    let (_, document) = recovered_scope_and_document(&fixture);
    let save = DurableEffectIntentV1::SaveDocument {
        document,
        expected_revision: 0,
    };
    let replace = DurableEffectIntentV1::ReplaceDocumentAndSave {
        document,
        expected_revision: 0,
        start: 0,
        end: 5,
        inserted: "bravo".into(),
    };

    let mut first = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject_a), 2);
    let socket = fixture.socket.clone();
    let first_intent = save.clone();
    let first_call = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .execute_durable(DurableExecuteRequestV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: first_intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        first.serve_next().unwrap();
    }
    let saved = first_call.join().unwrap();
    assert!(matches!(saved, DurableExecuteOutcomeV1::Completed(_)));
    assert_eq!(first.service_usage().durable_effect_requests, 1);
    drop(first);

    let mut second = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject_b), 2);
    let socket = fixture.socket.clone();
    let second_intent = replace.clone();
    let second_call = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .execute_durable(DurableExecuteRequestV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: second_intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        second.serve_next().unwrap();
    }
    let replaced = second_call.join().unwrap();
    assert!(matches!(replaced, DurableExecuteOutcomeV1::Completed(_)));
    assert_ne!(saved, replaced);
    assert_eq!(second.service_usage().durable_effect_requests, 1);
    assert_eq!(fs::read_to_string(&fixture.source).unwrap(), "bravo");
    drop(second);

    let mut replay = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject_a), 2);
    let socket = fixture.socket.clone();
    let replay_call = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .execute_durable(DurableExecuteRequestV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: save,
            })
            .unwrap()
    });
    for _ in 0..2 {
        replay.serve_next().unwrap();
    }
    assert_eq!(replay_call.join().unwrap(), saved);
    assert_eq!(replay.service_usage().durable_effect_requests, 0);
    assert_eq!(fs::read_to_string(&fixture.source).unwrap(), "bravo");
}

#[test]
fn retired_receipt_is_absorbing_across_restart_and_releases_active_capacity() {
    let fixture = Fixture::new();
    let claim = [0xd1; 16];
    let subject = [0xd2; 16];
    let operation = DurableOperationRefV1::new([0xd3; 16]).unwrap();
    let authority = actions().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::RetireReceipt,
    ));
    let initial = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap();
    drop(initial);
    let (_, document) = recovered_scope_and_document(&fixture);
    let intent = DurableEffectIntentV1::SaveDocument {
        document,
        expected_revision: 0,
    };
    let mut daemon = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 8);

    let socket = fixture.socket.clone();
    let execute_intent = intent.clone();
    let execute = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .execute_durable(DurableExecuteRequestV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: execute_intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        daemon.serve_next().unwrap();
    }
    assert!(matches!(
        execute.join().unwrap(),
        DurableExecuteOutcomeV1::Completed(_)
    ));
    assert_eq!(
        daemon.operational_snapshot().utilization.durable_receipts,
        1
    );

    let socket = fixture.socket.clone();
    let retire_intent = intent.clone();
    let retire = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .retire_durable_receipt(DurableReceiptRetirementV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: retire_intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        daemon.serve_next().unwrap();
    }
    assert_eq!(
        retire.join().unwrap(),
        DurableReceiptRetirementOutcomeV1::Retired
    );
    let usage = daemon.operational_snapshot().utilization;
    assert_eq!(usage.durable_receipts, 0);
    assert_eq!(usage.retired_durable_operations, 1);
    drop(daemon);

    let mut restarted = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 8);
    let socket = fixture.socket.clone();
    let lookup_intent = intent.clone();
    let lookup = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .lookup_durable_receipt(DurableReceiptLookupV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: lookup_intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        restarted.serve_next().unwrap();
    }
    assert_eq!(
        lookup.join().unwrap(),
        DurableReceiptLookupOutcomeV1::AbsentOrForeign
    );

    let socket = fixture.socket.clone();
    let execute = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .execute_durable(DurableExecuteRequestV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        restarted.serve_next().unwrap();
    }
    assert_eq!(execute.join().unwrap(), DurableExecuteOutcomeV1::Retired);
    assert_eq!(restarted.service_usage().durable_effect_requests, 0);
}

#[test]
fn authority_denied_retirement_leaves_complete_receipt_replayable() {
    let fixture = Fixture::new();
    let claim = [0xe1; 16];
    let subject = [0xe2; 16];
    let operation = DurableOperationRefV1::new([0xe3; 16]).unwrap();
    let initial = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions(),
    )
    .unwrap();
    drop(initial);
    let (_, document) = recovered_scope_and_document(&fixture);
    let intent = DurableEffectIntentV1::SaveDocument {
        document,
        expected_revision: 0,
    };
    let mut daemon = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        actions(),
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 8);
    let socket = fixture.socket.clone();
    let execute_intent = intent.clone();
    let execute = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .execute_durable(DurableExecuteRequestV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: execute_intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        daemon.serve_next().unwrap();
    }
    let completed = execute.join().unwrap();

    let socket = fixture.socket.clone();
    let retire_intent = intent.clone();
    let retire = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .retire_durable_receipt(DurableReceiptRetirementV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: retire_intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        daemon.serve_next().unwrap();
    }
    assert_eq!(
        retire.join().unwrap(),
        DurableReceiptRetirementOutcomeV1::Unavailable
    );
    assert_eq!(
        daemon.operational_snapshot().utilization.durable_receipts,
        1
    );
    assert_eq!(
        daemon
            .operational_snapshot()
            .utilization
            .retired_durable_operations,
        0
    );

    let socket = fixture.socket.clone();
    let lookup = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .lookup_durable_receipt(DurableReceiptLookupV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        daemon.serve_next().unwrap();
    }
    assert_eq!(
        lookup.join().unwrap(),
        DurableReceiptLookupOutcomeV1::Complete(completed)
    );
}

#[test]
fn marker_committed_cleanup_uncertainty_fences_before_stale_replay() {
    let fixture = Fixture::new();
    let claim = [0xea; 16];
    let subject = [0xeb; 16];
    let operation = DurableOperationRefV1::new([0xec; 16]).unwrap();
    let authority = actions().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::RetireReceipt,
    ));
    let initial = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap();
    drop(initial);
    let (_, document) = recovered_scope_and_document(&fixture);
    let intent = DurableEffectIntentV1::SaveDocument {
        document,
        expected_revision: 0,
    };

    let mut writer = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 4);
    let socket = fixture.socket.clone();
    let execute_intent = intent.clone();
    let execute = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport
            .execute_durable(DurableExecuteRequestV1 {
                operation,
                client,
                request: RequestId::new(1).unwrap(),
                intent: execute_intent,
            })
            .unwrap()
    });
    for _ in 0..2 {
        writer.serve_next().unwrap();
    }
    assert!(matches!(
        execute.join().unwrap(),
        DurableExecuteOutcomeV1::Completed(_)
    ));
    drop(writer);

    let mut daemon = LocalDaemon::bind_with_persistence_ports_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
        LocalWorkspaceIdentityMetadataV1::default(),
        CleanupOutcomeUnknownJournal::default(),
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim, subject), 4);
    assert_eq!(
        daemon.operational_snapshot().utilization.durable_receipts,
        1
    );
    let socket = fixture.socket.clone();
    let retire = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim);
        let client = transport.connect().unwrap();
        transport.retire_durable_receipt(DurableReceiptRetirementV1 {
            operation,
            client,
            request: RequestId::new(1).unwrap(),
            intent,
        })
    });
    daemon.serve_next().unwrap();
    assert!(matches!(
        daemon.serve_next(),
        Err(DaemonServeError::Request(_))
    ));
    assert!(retire.join().unwrap().is_err());

    let status = daemon.operational_snapshot().status;
    assert_eq!(status.health, DaemonHealth::Fenced);
    assert_eq!(status.readiness, DaemonReadiness::NotReady);
    assert_eq!(daemon.serve_next(), Err(DaemonServeError::OwnershipLost));
}

#[test]
fn foreign_and_unknown_retirement_are_identical_in_result_and_metrics() {
    let fixture = Fixture::new();
    let claim_a = [0xf1; 16];
    let claim_b = [0xf2; 16];
    let subject_a = [0xf3; 16];
    let subject_b = [0xf4; 16];
    let operation = DurableOperationRefV1::new([0xf5; 16]).unwrap();
    let unknown = DurableOperationRefV1::new([0xf6; 16]).unwrap();
    let authority = actions().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::RetireReceipt,
    ));
    let initial = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap();
    drop(initial);
    let (_, document) = recovered_scope_and_document(&fixture);
    let intent = DurableEffectIntentV1::SaveDocument {
        document,
        expected_revision: 0,
    };
    let mut writer = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim_a, subject_a), 4);
    let socket = fixture.socket.clone();
    let execute_intent = intent.clone();
    let execute = std::thread::spawn(move || {
        let mut transport = authenticated_transport(socket, claim_a);
        let client = transport.connect().unwrap();
        transport.execute_durable(DurableExecuteRequestV1 {
            operation,
            client,
            request: RequestId::new(1).unwrap(),
            intent: execute_intent,
        })
    });
    for _ in 0..2 {
        writer.serve_next().unwrap();
    }
    execute.join().unwrap().unwrap();
    drop(writer);

    let mut reader = LocalDaemon::bind_with_persistent_identity_and_action_ceiling(
        &fixture.socket,
        workspace(&fixture.source),
        limits(),
        authority,
    )
    .unwrap()
    .require_authentication(SubjectAuthentication, (claim_b, subject_b), 8);
    let before = reader.usage();
    let mut checkpoints = Vec::with_capacity(2);
    for target in [operation, unknown] {
        let socket = fixture.socket.clone();
        let target_intent = intent.clone();
        let call = std::thread::spawn(move || {
            let mut transport = authenticated_transport(socket, claim_b);
            let client = transport.connect().unwrap();
            transport
                .retire_durable_receipt(DurableReceiptRetirementV1 {
                    operation: target,
                    client,
                    request: RequestId::new(1).unwrap(),
                    intent: target_intent,
                })
                .unwrap()
        });
        for _ in 0..2 {
            reader.serve_next().unwrap();
        }
        assert_eq!(
            call.join().unwrap(),
            DurableReceiptRetirementOutcomeV1::AbsentOrForeign
        );
        checkpoints.push(reader.usage());
    }
    let retirement_delta = |after: DaemonUsage, before: DaemonUsage| {
        [
            after.durable_retirement_requests - before.durable_retirement_requests,
            after.durable_retirement_retired - before.durable_retirement_retired,
            after.durable_retirement_absent_or_foreign
                - before.durable_retirement_absent_or_foreign,
            after.durable_retirement_conflicts - before.durable_retirement_conflicts,
            after.durable_retirement_pending - before.durable_retirement_pending,
            after.durable_retirement_unavailable - before.durable_retirement_unavailable,
            after.durable_retirement_outcome_unknown - before.durable_retirement_outcome_unknown,
        ]
    };
    let foreign_delta = retirement_delta(checkpoints[0], before);
    let unknown_delta = retirement_delta(checkpoints[1], checkpoints[0]);
    assert_eq!(foreign_delta, [1, 0, 1, 0, 0, 0, 0]);
    assert_eq!(unknown_delta, foreign_delta);
}
