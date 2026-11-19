#![cfg(target_os = "macos")]

use std::{
    cell::RefCell,
    fs,
    path::PathBuf,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};
use vellum_adapter_local_fs::{FilesystemLimits, LocalDirectoryDiscovery, LocalFilesystemFactory};
use vellum_client::Client;
use vellum_editor::Workspace;
use vellum_host::authority::{
    AuthorityActionSetV1, AuthorityActionV1, AuthorityContextV1, AuthorityDecisionEvidenceV1,
    AuthorityDecisionRefV1, AuthorityDecisionV1, AuthorityPortV1, AuthorityRequestV1,
    AuthorityResourceRefV1, AuthoritySubjectRefV1,
};
use vellum_host::workspace::{
    WorkspaceIdentityAllocationV1, WorkspaceIdentityLimitsV1, WorkspaceIdentityMetadataV1,
};
use vellum_protocol::{
    CommandV1, DurableEffectIntentV1, DurableEffectReceiptV1, DurableExecuteOutcomeV1,
    EditPersistenceV1, OutcomeV1, RefusalV1, ReplyV1, RequestId, RequestV1, SaveApplicationV1,
    SaveSynchronizationV1, ViewCloseStatusV1, ViewOpenStatusV1, WorkspaceChangeCursorV1,
    WorkspaceChangeKindV1, WorkspaceChangesV1, WorkspaceDocumentEditV1, WorkspaceInventoryCursorV1,
    WorkspaceReplacementV1,
};
use vellum_service::{Service, ServiceBuildError, ServiceLimits, ServiceUsage};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn handoff_reads_exact_unsaved_source_from_the_served_workspace() {
    let fixture = Fixture::new("\u{feff}alpha\r\n");
    let mut workspace = workspace_for(std::slice::from_ref(&fixture.0));
    let member = workspace.inventory().unwrap().documents.remove(0);
    let mut expected = workspace.document_version(&member).unwrap();
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
        panic!("expected document inventory");
    };
    let document = inventory.documents[0];
    let edit = CommandV1::WorkspaceEdit {
        documents: vec![WorkspaceDocumentEditV1 {
            document,
            expected_revision: expected.revision,
            replacements: vec![WorkspaceReplacementV1 {
                start: 0,
                end: 5,
                inserted: "beta".into(),
            }],
        }],
    };
    let ReplyV1::Accepted(OutcomeV1::WorkspaceEdited(receipt)) =
        service.execute(request(client, 2, edit))
    else {
        panic!("expected an unsaved edit");
    };
    assert!(matches!(
        service.capture_document_for_handoff(client, document, &expected),
        Err(RefusalV1::Stale)
    ));
    expected.revision = receipt.documents[0].snapshot.revision;
    let capture = service
        .capture_document_for_handoff(client, document, &expected)
        .unwrap();
    assert_eq!(capture.source(), &expected);
    assert_eq!(capture.bytes(), b"\xef\xbb\xbfbeta\r\n");
    assert_eq!(fs::read(&fixture.0).unwrap(), b"\xef\xbb\xbfalpha\r\n");
    service.disconnect(client).unwrap();
    assert!(matches!(
        service.capture_document_for_handoff(client, document, &expected),
        Err(RefusalV1::UnknownClient)
    ));
}

#[test]
fn exact_document_version_observation_is_read_only_and_view_independent() {
    let fixture = Fixture::new("alpha");
    let other = Fixture::new("other");
    let workspace = workspace_for(&[fixture.0.clone(), other.0.clone()]);
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
        panic!("expected document inventory")
    };
    let ReplyV1::Accepted(OutcomeV1::LocatorResolved(document)) = service.execute(request(
        client,
        2,
        CommandV1::ResolveLocator {
            locator: fixture.0.to_string_lossy().into_owned(),
        },
    )) else {
        panic!("expected exact first document resolution")
    };
    let ReplyV1::Accepted(OutcomeV1::LocatorResolved(other_document)) = service.execute(request(
        client,
        3,
        CommandV1::ResolveLocator {
            locator: other.0.to_string_lossy().into_owned(),
        },
    )) else {
        panic!("expected exact other document resolution")
    };
    let document = document.document.unwrap();
    let other_document = other_document.document.unwrap();
    assert!(inventory.documents.contains(&document));
    assert!(inventory.documents.contains(&other_document));
    assert_eq!(
        service.execute(request(
            client,
            4,
            CommandV1::SelectDocument {
                document: other_document,
            },
        )),
        ReplyV1::Accepted(OutcomeV1::DocumentSelected(other_document))
    );
    let ReplyV1::Accepted(OutcomeV1::Snapshot(active_before)) =
        service.execute(request(client, 5, CommandV1::Snapshot))
    else {
        panic!("expected active document snapshot")
    };
    let ReplyV1::Accepted(OutcomeV1::DocumentVersionObserved(before)) = service.execute(request(
        client,
        6,
        CommandV1::ObserveDocumentVersion { document },
    )) else {
        panic!("expected exact document version")
    };
    assert_eq!(before.document, document);
    assert_eq!(before.byte_length, 5);
    let ReplyV1::Accepted(OutcomeV1::Snapshot(active_after)) =
        service.execute(request(client, 7, CommandV1::Snapshot))
    else {
        panic!("expected unchanged active document snapshot")
    };
    assert_eq!(active_after.revision, active_before.revision);
    assert_eq!(active_after.text, active_before.text);
    let ReplyV1::Accepted(OutcomeV1::WorkspaceEdited(receipt)) = service.execute(request(
        client,
        8,
        CommandV1::WorkspaceEdit {
            documents: vec![WorkspaceDocumentEditV1 {
                document,
                expected_revision: before.revision,
                replacements: vec![WorkspaceReplacementV1 {
                    start: 0,
                    end: 5,
                    inserted: "beta".into(),
                }],
            }],
        },
    )) else {
        panic!("expected unsaved exact-document edit")
    };
    let ReplyV1::Accepted(OutcomeV1::DocumentVersionObserved(after)) = service.execute(request(
        client,
        9,
        CommandV1::ObserveDocumentVersion { document },
    )) else {
        panic!("expected changed exact document version")
    };
    assert_eq!(after.revision, receipt.documents[0].snapshot.revision);
    assert_ne!(after.revision, before.revision);
    assert_eq!(after.byte_length, 4);
    assert_eq!(fs::read_to_string(&fixture.0).unwrap(), "alpha");
}

#[test]
fn workspace_edit_publishes_two_documents_atomically_without_persistence_claim() {
    let first = Fixture::new("abcd");
    let second = Fixture::new("wxyz");
    let mut service = empty_service(ServiceLimits::reference());
    let client = service.connect().unwrap();
    for (id, fixture) in [(1, &first), (2, &second)] {
        let reply = service.execute(request(
            client,
            id,
            CommandV1::AddFile {
                locator: fixture.0.to_string_lossy().into_owned(),
            },
        ));
        assert!(matches!(
            reply,
            ReplyV1::Accepted(OutcomeV1::FileAdmitted(_))
        ));
    }
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) =
        service.execute(request(client, 3, CommandV1::WorkspaceInventory))
    else {
        panic!("inventory must be available")
    };
    let mut witnessed = Vec::new();
    for (offset, document) in inventory.documents.iter().copied().enumerate() {
        assert!(matches!(
            service.execute(request(
                client,
                4 + u64::try_from(offset * 2).unwrap(),
                CommandV1::SelectDocument { document },
            )),
            ReplyV1::Accepted(OutcomeV1::DocumentSelected(_))
        ));
        let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) = service.execute(request(
            client,
            5 + u64::try_from(offset * 2).unwrap(),
            CommandV1::Snapshot,
        )) else {
            panic!("snapshot must be available")
        };
        witnessed.push((document, snapshot.revision));
    }
    let edit = CommandV1::WorkspaceEdit {
        documents: witnessed
            .iter()
            .map(|(document, expected_revision)| WorkspaceDocumentEditV1 {
                document: *document,
                expected_revision: *expected_revision,
                replacements: vec![WorkspaceReplacementV1 {
                    start: 1,
                    end: 3,
                    inserted: "QQ".into(),
                }],
            })
            .collect(),
    };
    let reply = service.execute(request(client, 8, edit.clone()));
    let ReplyV1::Accepted(OutcomeV1::WorkspaceEdited(receipt)) = reply else {
        panic!("workspace edit must commit")
    };
    assert_eq!(receipt.documents.len(), 2);
    assert_eq!(&*receipt.documents[0].snapshot.text, "aQQd");
    assert_eq!(&*receipt.documents[1].snapshot.text, "wQQz");
    assert_eq!(fs::read_to_string(&first.0).unwrap(), "abcd");
    assert_eq!(fs::read_to_string(&second.0).unwrap(), "wxyz");
    assert_eq!(
        service.execute(request(client, 8, edit)),
        ReplyV1::Accepted(OutcomeV1::WorkspaceEdited(receipt.clone()))
    );

    let no_op = service.execute(request(
        client,
        9,
        CommandV1::WorkspaceEdit {
            documents: vec![WorkspaceDocumentEditV1 {
                document: receipt.documents[0].document,
                expected_revision: receipt.documents[0].snapshot.revision,
                replacements: vec![WorkspaceReplacementV1 {
                    start: 0,
                    end: 1,
                    inserted: "a".into(),
                }],
            }],
        },
    ));
    let ReplyV1::Accepted(OutcomeV1::WorkspaceEdited(no_op)) = no_op else {
        panic!("exact no-op must remain an accepted atomic transaction")
    };
    assert!(no_op.documents.is_empty());

    let stale = service.execute(request(
        client,
        10,
        CommandV1::WorkspaceEdit {
            documents: vec![
                WorkspaceDocumentEditV1 {
                    document: witnessed[0].0,
                    expected_revision: receipt.documents[0].snapshot.revision - 1,
                    replacements: vec![WorkspaceReplacementV1 {
                        start: 0,
                        end: 1,
                        inserted: "S".into(),
                    }],
                },
                WorkspaceDocumentEditV1 {
                    document: receipt.documents[1].document,
                    expected_revision: receipt.documents[1].snapshot.revision,
                    replacements: vec![WorkspaceReplacementV1 {
                        start: 0,
                        end: 1,
                        inserted: "T".into(),
                    }],
                },
            ],
        },
    ));
    assert_eq!(stale, ReplyV1::Refused(RefusalV1::Stale));
    assert!(matches!(
        service.execute(request(
            client,
            11,
            CommandV1::SelectDocument {
                document: receipt.documents[1].document,
            },
        )),
        ReplyV1::Accepted(OutcomeV1::DocumentSelected(_))
    ));
    let ReplyV1::Accepted(OutcomeV1::Snapshot(after_refusal)) =
        service.execute(request(client, 12, CommandV1::Snapshot))
    else {
        panic!("snapshot must remain available")
    };
    assert_eq!(&*after_refusal.text, "wQQz");
    let usage = service.usage();
    assert_eq!(usage.workspace_edit_requests, 3);
    assert_eq!(usage.workspace_edit_refused, 1);
    assert_eq!(usage.workspace_edit_documents_requested, 5);
    assert_eq!(usage.workspace_edit_documents_changed, 2);
    assert_eq!(usage.workspace_edit_documents_unchanged, 1);
    assert_eq!(usage.workspace_edit_transactions_changed, 1);
    assert_eq!(usage.workspace_edit_transactions_unchanged, 1);
}

struct Fixture(PathBuf);

impl Fixture {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "vellum-service-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, source).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

struct DirectoryFixture(PathBuf);

impl DirectoryFixture {
    fn new(files: &[(&str, &str)]) -> Self {
        let path = std::env::temp_dir().join(format!(
            "vellum-service-directory-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        for (name, content) in files {
            fs::write(path.join(name), content).unwrap();
        }
        Self(path)
    }
}

impl Drop for DirectoryFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn service(source: &str) -> (Fixture, Service) {
    service_with_limits(source, ServiceLimits::reference())
}

#[test]
fn explicit_document_durable_edit_has_stable_receipt_and_stale_is_pre_effect() {
    let (fixture, mut service) = service("abcd");
    let client = service.connect().unwrap();
    let inventory = service.execute(request(client, 1, CommandV1::WorkspaceInventory));
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) = inventory else {
        panic!("workspace inventory must be available")
    };
    let document = inventory.documents[0];
    let snapshot = service.execute(request(client, 2, CommandV1::Snapshot));
    let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) = snapshot else {
        panic!("snapshot must be available")
    };
    let intent = DurableEffectIntentV1::ReplaceDocumentAndSave {
        document,
        expected_revision: snapshot.revision,
        start: 1,
        end: 3,
        inserted: "XY".into(),
    };
    let outcome = service.execute_durable_effect(client, &intent);
    let DurableExecuteOutcomeV1::Completed(DurableEffectReceiptV1::DocumentReplacedAndSaved(
        receipt,
    )) = outcome
    else {
        panic!("durable edit must return its stable receipt")
    };
    assert_eq!(receipt.document, document);
    assert_eq!(&*receipt.text, "aXYd");
    assert!(receipt.source_revision > snapshot.revision);
    assert_eq!(fs::read_to_string(&fixture.0).unwrap(), "aXYd");

    assert_eq!(
        service.execute_durable_effect(client, &intent),
        DurableExecuteOutcomeV1::Refused(RefusalV1::Stale)
    );
    assert_eq!(fs::read_to_string(&fixture.0).unwrap(), "aXYd");
    assert_eq!(service.usage().durable_effect_requests, 2);
    assert_eq!(service.usage().durable_effect_completed, 1);
    assert_eq!(service.usage().durable_effect_refused, 1);
}

fn service_with_limits(source: &str, limits: ServiceLimits) -> (Fixture, Service) {
    let fixture = Fixture::new(source);
    let workspace = Workspace::open(
        std::slice::from_ref(&fixture.0),
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
    (
        fixture,
        Service::new_with_action_ceiling(workspace, limits, AuthorityActionSetV1::top()).unwrap(),
    )
}

#[test]
fn locator_resolution_is_exact_private_and_read_only() {
    let fixture = Fixture::new("locator-private-source");
    let foreign = Fixture::new("foreign-private-source");
    let workspace = workspace_for(std::slice::from_ref(&fixture.0));
    let mut service = Service::new(workspace, ServiceLimits::reference()).unwrap();
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) =
        service.execute(request(client, 1, CommandV1::WorkspaceInventory))
    else {
        panic!("inventory must materialize the existing identities")
    };
    let identity_before = service.identity_revision_evidence();

    let locator = fixture.0.to_str().unwrap().to_owned();
    let reply = service.execute(request(
        client,
        2,
        CommandV1::ResolveLocator {
            locator: locator.clone(),
        },
    ));
    let ReplyV1::Accepted(OutcomeV1::LocatorResolved(resolved)) = &reply else {
        panic!("exact current binding must resolve")
    };
    assert_eq!(resolved.workspace, inventory.workspace);
    assert_eq!(resolved.document, Some(inventory.documents[0]));
    assert_eq!(resolved.root, Some(inventory.roots[0].clone()));
    assert_eq!(
        service.execute(request(
            client,
            2,
            CommandV1::ResolveLocator {
                locator: locator.clone(),
            },
        )),
        reply
    );

    let unknown = service.execute(request(
        client,
        3,
        CommandV1::ResolveLocator {
            locator: foreign.0.to_str().unwrap().into(),
        },
    ));
    let ReplyV1::Accepted(OutcomeV1::LocatorResolved(unknown)) = unknown else {
        panic!("unknown and foreign locators must share the closed absent outcome")
    };
    assert_eq!(unknown.workspace, inventory.workspace);
    assert_eq!(unknown.document, None);
    assert_eq!(unknown.root, None);

    let absent = service.execute(request(
        client,
        4,
        CommandV1::ResolveLocator {
            locator: "/no/such/vellum-workspace-member".into(),
        },
    ));
    let ReplyV1::Accepted(OutcomeV1::LocatorResolved(absent)) = absent else {
        panic!("a nonexistent locator must not trigger filesystem access")
    };
    assert_eq!(absent.document, None);
    assert_eq!(absent.root, None);

    for (id, invalid) in [
        (5, "relative/member".to_owned()),
        (6, format!("{locator}/../member")),
        (7, format!("{locator}/./member")),
    ] {
        assert_eq!(
            service.execute(request(
                client,
                id,
                CommandV1::ResolveLocator { locator: invalid },
            )),
            ReplyV1::Refused(RefusalV1::InvalidRange)
        );
    }

    assert_eq!(service.identity_revision_evidence(), identity_before);
    let ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Page(changes))) = service
        .execute(request(
            client,
            8,
            CommandV1::WorkspaceChanges {
                after: None,
                page_size: 1,
            },
        ))
    else {
        panic!("change feed must remain available")
    };
    assert!(changes.changes.is_empty());
    assert_eq!(
        service.execute(request(
            client,
            9,
            CommandV1::AddFile {
                locator: foreign.0.to_str().unwrap().into(),
            },
        )),
        ReplyV1::Refused(RefusalV1::AuthorityDenied)
    );
    let usage = service.usage();
    assert_eq!(usage.locator_resolution_requests, 6);
    assert_eq!(usage.locators_resolved, 1);
    assert_eq!(usage.locators_absent_or_foreign, 2);
    assert_eq!(usage.locator_resolution_refused, 3);
    assert_eq!(usage.locator_resolution_unavailable, 0);
    let operational = format!("{reply:?} {usage:?}");
    assert!(!operational.contains(&locator));
    assert!(!operational.contains("locator-private-source"));
}

fn service_with_save_task_capacity(source: &str, capacity: usize) -> (Fixture, Service) {
    let fixture = Fixture::new(source);
    let mut profile = vellum_editor::reference_workspace_profile(
        LocalFilesystemFactory::new(FilesystemLimits {
            max_resources: 8,
            max_bytes: 4096,
            max_stage_attempts: 8,
            max_metadata_bytes: 4096,
        })
        .erase(),
    );
    profile.engine.resources.as_mut().unwrap().max_live_tasks = capacity;
    let workspace = Workspace::open(std::slice::from_ref(&fixture.0), profile).unwrap();
    (
        fixture,
        Service::new_with_action_ceiling(
            workspace,
            ServiceLimits::reference(),
            AuthorityActionSetV1::top(),
        )
        .unwrap(),
    )
}

fn empty_service(limits: ServiceLimits) -> Service {
    let workspace = Workspace::empty(vellum_editor::reference_workspace_profile(
        LocalFilesystemFactory::new(FilesystemLimits {
            max_resources: 8,
            max_bytes: 4096,
            max_stage_attempts: 8,
            max_metadata_bytes: 4096,
        })
        .erase(),
    ))
    .unwrap();
    Service::new_with_action_ceiling(workspace, limits, AuthorityActionSetV1::top()).unwrap()
}

fn empty_service_with_discovery(limits: ServiceLimits) -> Service {
    let workspace = Workspace::empty(vellum_editor::reference_workspace_profile(
        LocalFilesystemFactory::new(FilesystemLimits {
            max_resources: 8,
            max_bytes: 4096,
            max_stage_attempts: 8,
            max_metadata_bytes: 4096,
        })
        .erase(),
    ))
    .unwrap();
    Service::new_with_directory_discovery_and_action_ceiling(
        workspace,
        limits,
        LocalDirectoryDiscovery,
        AuthorityActionSetV1::top(),
    )
    .unwrap()
}

fn empty_service_with_document_capacity(capacity: usize) -> Service {
    let mut profile = vellum_editor::reference_workspace_profile(
        LocalFilesystemFactory::new(FilesystemLimits {
            max_resources: 8,
            max_bytes: 4096,
            max_stage_attempts: 8,
            max_metadata_bytes: 4096,
        })
        .erase(),
    );
    profile.engine.limits.max_document_incarnations = capacity;
    let workspace = Workspace::empty(profile).unwrap();
    Service::new_with_action_ceiling(
        workspace,
        ServiceLimits::reference(),
        AuthorityActionSetV1::top(),
    )
    .unwrap()
}

fn request(client: vellum_protocol::ClientId, id: u64, command: CommandV1) -> RequestV1 {
    RequestV1 {
        client,
        request: RequestId::new(id).unwrap(),
        command,
    }
}

fn authority_scope() -> (AuthorityContextV1, AuthorityResourceRefV1) {
    (
        AuthorityContextV1 {
            subject: AuthoritySubjectRefV1::new([31; 16]),
            session: None,
        },
        AuthorityResourceRefV1::new([37; 16]),
    )
}

struct RecordingDeniedAuthority(Rc<RefCell<Vec<AuthorityRequestV1>>>);

impl AuthorityPortV1 for RecordingDeniedAuthority {
    fn authorize(&mut self, request: AuthorityRequestV1) -> AuthorityDecisionV1 {
        self.0.borrow_mut().push(request);
        AuthorityDecisionV1::Denied
    }
}

struct MismatchedEvidenceAuthority;

impl AuthorityPortV1 for MismatchedEvidenceAuthority {
    fn authorize(&mut self, request: AuthorityRequestV1) -> AuthorityDecisionV1 {
        let mismatched = AuthorityRequestV1 {
            action: if request.action == AuthorityActionV1::PersistSource {
                AuthorityActionV1::ReadSource
            } else {
                AuthorityActionV1::PersistSource
            },
            ..request
        };
        AuthorityDecisionV1::Granted(AuthorityDecisionEvidenceV1::new(
            AuthorityDecisionRefV1::new([41; 16]),
            mismatched,
        ))
    }
}

struct UnavailableAuthority;

impl AuthorityPortV1 for UnavailableAuthority {
    fn authorize(&mut self, _request: AuthorityRequestV1) -> AuthorityDecisionV1 {
        AuthorityDecisionV1::Unavailable
    }
}

#[test]
fn default_service_is_read_only_and_replays_denial_without_reconsulting_authority() {
    let fixture = Fixture::new("alpha");
    let workspace = Workspace::open(
        std::slice::from_ref(&fixture.0),
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
    let mut service = Service::new(workspace, ServiceLimits::reference()).unwrap();
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) =
        service.execute(request(client, 1, CommandV1::Snapshot))
    else {
        panic!("read-only authority must permit source reads")
    };
    let edit = request(
        client,
        2,
        CommandV1::Replace {
            expected: snapshot.witness,
            start: 0,
            end: 5,
            inserted: "agent".into(),
        },
    );
    assert_eq!(
        service.execute(edit.clone()),
        ReplyV1::Refused(RefusalV1::AuthorityDenied)
    );
    assert_eq!(
        service.execute(edit),
        ReplyV1::Refused(RefusalV1::AuthorityDenied)
    );
    let ReplyV1::Accepted(OutcomeV1::Snapshot(after)) =
        service.execute(request(client, 3, CommandV1::Snapshot))
    else {
        panic!("expected unchanged snapshot")
    };
    assert_eq!(&*after.text, "alpha");
    assert_eq!(service.usage().authority_checks, 3);
    assert_eq!(service.usage().authority_granted, 2);
    assert_eq!(service.usage().authority_denied, 1);
}

#[test]
fn every_command_maps_to_one_exact_closed_authority_action_before_interpretation() {
    let fixture = Fixture::new("alpha");
    let workspace = Workspace::open(
        std::slice::from_ref(&fixture.0),
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
    let (context, resource) = authority_scope();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let mut service = Service::new_with_authority(
        workspace,
        ServiceLimits::reference(),
        RecordingDeniedAuthority(Rc::clone(&observed)),
        context,
        resource,
    )
    .unwrap();
    let client = service.connect().unwrap();
    let workspace_id = vellum_protocol::WorkspaceIdV1::new(1).unwrap();
    let document = vellum_protocol::WorkspaceDocumentIdV1::new(workspace_id, 1).unwrap();
    let root = vellum_protocol::WorkspaceRootIdV1::new(workspace_id, 1).unwrap();
    let witness = vellum_protocol::VersionWitness::new(1).unwrap();
    for (id, command) in [
        (1, CommandV1::WorkspaceInventory),
        (
            10,
            CommandV1::WorkspaceInventoryPage {
                cursor: None,
                page_size: 1,
            },
        ),
        (
            11,
            CommandV1::WorkspaceChanges {
                after: None,
                page_size: 1,
            },
        ),
        (2, CommandV1::Snapshot),
        (3, CommandV1::OpenView),
        (4, CommandV1::CloseView),
        (5, CommandV1::SelectDocument { document }),
        (
            6,
            CommandV1::AddFile {
                locator: "x".into(),
            },
        ),
        (
            7,
            CommandV1::AddDirectory {
                locator: "x".into(),
            },
        ),
        (
            8,
            CommandV1::Replace {
                expected: witness,
                start: 0,
                end: 0,
                inserted: String::new(),
            },
        ),
        (
            14,
            CommandV1::ReplaceAndSave {
                expected: witness,
                start: 0,
                end: 0,
                inserted: String::new(),
            },
        ),
        (9, CommandV1::Save { expected: witness }),
        (12, CommandV1::RemoveDocument { document }),
        (13, CommandV1::RemoveRoot { root }),
    ] {
        assert_eq!(
            service.execute(request(client, id, command)),
            ReplyV1::Refused(RefusalV1::AuthorityDenied)
        );
    }
    let observed = observed.borrow();
    assert_eq!(observed.len(), 14);
    assert!(
        observed
            .iter()
            .all(|request| request.context == context && request.resource == resource)
    );
    assert_eq!(
        observed
            .iter()
            .map(|request| request.action)
            .collect::<Vec<_>>(),
        vec![
            AuthorityActionV1::InspectWorkspace,
            AuthorityActionV1::InspectWorkspace,
            AuthorityActionV1::InspectWorkspace,
            AuthorityActionV1::ReadSource,
            AuthorityActionV1::ManageView,
            AuthorityActionV1::ManageView,
            AuthorityActionV1::SelectDocument,
            AuthorityActionV1::AdmitFile,
            AuthorityActionV1::DiscoverDirectory,
            AuthorityActionV1::EditSource,
            AuthorityActionV1::EditSource,
            AuthorityActionV1::PersistSource,
            AuthorityActionV1::RetireMembership,
            AuthorityActionV1::RetireMembership,
        ]
    );
}

#[test]
fn view_lifecycle_is_scoped_idempotent_replayable_and_observed() {
    let (_fixture, mut service) = service_with_limits("alpha", ServiceLimits::reference());
    let client = service.connect().unwrap();

    let ReplyV1::Accepted(OutcomeV1::ViewOpened(initial)) =
        service.execute(request(client, 1, CommandV1::OpenView))
    else {
        panic!("connected non-empty client must have a view")
    };
    assert_eq!(initial.status, ViewOpenStatusV1::AlreadyOpen);
    let ReplyV1::Accepted(OutcomeV1::Snapshot(before_close)) =
        service.execute(request(client, 2, CommandV1::Snapshot))
    else {
        panic!("expected pre-close witness")
    };

    let close = request(client, 3, CommandV1::CloseView);
    assert_eq!(
        service.execute(close.clone()),
        ReplyV1::Accepted(OutcomeV1::ViewClosed(ViewCloseStatusV1::Closed))
    );
    assert_eq!(
        service.execute(close),
        ReplyV1::Accepted(OutcomeV1::ViewClosed(ViewCloseStatusV1::Closed))
    );
    assert_eq!(
        service.execute(request(client, 4, CommandV1::Snapshot)),
        ReplyV1::Refused(RefusalV1::NoActiveDocument)
    );
    assert_eq!(
        service.execute(request(client, 5, CommandV1::CloseView)),
        ReplyV1::Accepted(OutcomeV1::ViewClosed(ViewCloseStatusV1::AlreadyClosed))
    );

    let ReplyV1::Accepted(OutcomeV1::ViewOpened(reopened)) =
        service.execute(request(client, 6, CommandV1::OpenView))
    else {
        panic!("expected explicit view open")
    };
    assert_eq!(reopened.status, ViewOpenStatusV1::Opened);
    assert_eq!(reopened.document, initial.document);
    let ReplyV1::Accepted(OutcomeV1::Replaced(after_reopen)) = service.execute(request(
        client,
        7,
        CommandV1::Replace {
            expected: before_close.witness,
            start: 0,
            end: 5,
            inserted: "agent".into(),
        },
    )) else {
        panic!("a still-current client witness must survive view close and reopen")
    };
    assert_eq!(&*after_reopen.text, "agent");

    let usage = service.usage();
    assert_eq!(usage.views_already_open, 1);
    assert_eq!(usage.views_closed, 1);
    assert_eq!(usage.views_already_closed, 1);
    assert_eq!(usage.views_opened, 1);
}

#[test]
fn mismatched_grant_evidence_fails_closed_without_consuming_a_witness() {
    let fixture = Fixture::new("alpha");
    let workspace = Workspace::open(
        std::slice::from_ref(&fixture.0),
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
    let (context, resource) = authority_scope();
    let mut service = Service::new_with_authority(
        workspace,
        ServiceLimits::reference(),
        MismatchedEvidenceAuthority,
        context,
        resource,
    )
    .unwrap();
    let client = service.connect().unwrap();
    assert_eq!(
        service.execute(request(client, 1, CommandV1::Snapshot)),
        ReplyV1::Refused(RefusalV1::AuthorityUnavailable)
    );
    assert_eq!(service.usage().retained_witnesses, 0);
    assert_eq!(service.usage().authority_checks, 1);
    assert_eq!(service.usage().authority_unavailable, 1);
}

#[test]
fn unavailable_authority_precedes_editor_interpretation_and_is_counted_once() {
    let workspace = Workspace::empty(vellum_editor::reference_workspace_profile(
        LocalFilesystemFactory::new(FilesystemLimits {
            max_resources: 8,
            max_bytes: 4096,
            max_stage_attempts: 8,
            max_metadata_bytes: 4096,
        })
        .erase(),
    ))
    .unwrap();
    let (context, resource) = authority_scope();
    let mut service = Service::new_with_authority(
        workspace,
        ServiceLimits::reference(),
        UnavailableAuthority,
        context,
        resource,
    )
    .unwrap();
    let client = service.connect().unwrap();
    let request = request(client, 1, CommandV1::Snapshot);
    assert_eq!(
        service.execute(request.clone()),
        ReplyV1::Refused(RefusalV1::AuthorityUnavailable)
    );
    assert_eq!(
        service.execute(request),
        ReplyV1::Refused(RefusalV1::AuthorityUnavailable)
    );
    assert_eq!(service.usage().authority_checks, 1);
    assert_eq!(service.usage().authority_unavailable, 1);
    assert_eq!(service.usage().no_active_document, 0);
}

#[test]
fn fixed_authority_decision_budget_overflow_is_rejected_before_allocation() {
    let workspace = Workspace::empty(vellum_editor::reference_workspace_profile(
        LocalFilesystemFactory::new(FilesystemLimits {
            max_resources: 8,
            max_bytes: 4096,
            max_stage_attempts: 8,
            max_metadata_bytes: 4096,
        })
        .erase(),
    ))
    .unwrap();
    let mut limits = ServiceLimits::reference();
    limits.max_requests = usize::MAX;
    assert!(matches!(
        Service::new(workspace, limits),
        Err(ServiceBuildError::InvalidLimits)
    ));
}

#[test]
fn clients_share_authority_without_sharing_requests_witnesses_or_views() {
    let mut limits = ServiceLimits::reference();
    limits.max_clients = 2;
    let (_fixture, mut service) = service_with_limits("alpha beta", limits);
    let first_client = service.connect().unwrap();
    let second_client = service.connect().unwrap();
    assert_ne!(first_client, second_client);
    assert_eq!(service.connect(), Err(RefusalV1::ClientCapacity));

    let first = service.execute(request(first_client, 1, CommandV1::Snapshot));
    let ReplyV1::Accepted(OutcomeV1::Snapshot(first)) = first else {
        panic!("expected snapshot")
    };
    assert_eq!(&*first.text, "alpha beta");
    let ReplyV1::Accepted(OutcomeV1::Snapshot(second)) =
        service.execute(request(second_client, 1, CommandV1::Snapshot))
    else {
        panic!("expected second-client snapshot")
    };
    assert_eq!(&*second.text, "alpha beta");
    assert_ne!(first.witness, second.witness);

    let edit = request(
        first_client,
        2,
        CommandV1::Replace {
            expected: first.witness,
            start: 6,
            end: 10,
            inserted: "agent".into(),
        },
    );
    let replaced = service.execute(edit.clone());
    let ReplyV1::Accepted(OutcomeV1::Replaced(snapshot)) = &replaced else {
        panic!("expected replacement")
    };
    assert_eq!(&*snapshot.text, "alpha agent");

    assert_eq!(service.execute(edit), replaced);
    assert_eq!(
        service.execute(request(
            first_client,
            2,
            CommandV1::Replace {
                expected: first.witness,
                start: 0,
                end: 5,
                inserted: "other".into(),
            },
        )),
        ReplyV1::Refused(RefusalV1::RequestConflict)
    );
    assert_eq!(
        service.execute(request(
            second_client,
            2,
            CommandV1::Replace {
                expected: first.witness,
                start: 0,
                end: 5,
                inserted: "foreign".into(),
            },
        )),
        ReplyV1::Refused(RefusalV1::UnknownWitness)
    );
    assert_eq!(
        service.execute(request(
            second_client,
            3,
            CommandV1::Replace {
                expected: second.witness,
                start: 0,
                end: 5,
                inserted: "stale".into(),
            },
        )),
        ReplyV1::Refused(RefusalV1::Stale)
    );
    let ReplyV1::Accepted(OutcomeV1::Snapshot(fresh)) =
        service.execute(request(second_client, 4, CommandV1::Snapshot))
    else {
        panic!("expected refreshed snapshot")
    };
    let ReplyV1::Accepted(OutcomeV1::Replaced(final_snapshot)) = service.execute(request(
        second_client,
        5,
        CommandV1::Replace {
            expected: fresh.witness,
            start: 0,
            end: 5,
            inserted: "human".into(),
        },
    )) else {
        panic!("expected second-client replacement")
    };
    assert_eq!(&*final_snapshot.text, "human agent");
    assert_eq!(
        service.usage(),
        ServiceUsage {
            connect_attempts: 3,
            connected: 2,
            connection_refused: 1,
            requests: 9,
            request_new: 7,
            request_matching_replay: 1,
            request_changed_input: 1,
            accepted: 6,
            refused: 3,
            replayed: 1,
            conflicts: 1,
            stale: 1,
            invalid: 1,
            authority_checks: 7,
            authority_granted: 7,
            change_feed_events_published: 2,
            save_as_capacity: 128,
            active_clients: 2,
            client_capacity: 2,
            issued_clients: 2,
            issued_client_capacity: 64,
            retained_requests: 7,
            request_capacity: 256,
            retained_witnesses: 5,
            witness_capacity: 256,
            ..ServiceUsage::default()
        }
    );
}

#[test]
fn invalid_ranges_and_limits_refuse_without_changing_source() {
    let (_fixture, mut service) = service("éx");
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::Snapshot(first)) =
        service.execute(request(client, 1, CommandV1::Snapshot))
    else {
        panic!("expected snapshot")
    };

    assert_eq!(
        service.execute(request(
            client,
            2,
            CommandV1::Replace {
                expected: first.witness,
                start: 1,
                end: 2,
                inserted: "z".into(),
            },
        )),
        ReplyV1::Refused(RefusalV1::InvalidRange)
    );
    let ReplyV1::Accepted(OutcomeV1::Snapshot(after)) =
        service.execute(request(client, 3, CommandV1::Snapshot))
    else {
        panic!("expected snapshot")
    };
    assert_eq!(&*after.text, "éx");
    assert_eq!(service.usage().requests, 3);
    assert_eq!(service.usage().accepted, 2);
    assert_eq!(service.usage().refused, 1);
    assert_eq!(service.usage().invalid, 1);
}

#[test]
fn native_client_drives_the_real_in_process_service_contract() {
    let (_fixture, service) = service("one two");
    let mut client = Client::connect(service).unwrap();
    let ReplyV1::Accepted(OutcomeV1::Snapshot(first)) = client.snapshot().unwrap() else {
        panic!("expected snapshot")
    };
    let ReplyV1::Accepted(OutcomeV1::Replaced(after)) =
        client.replace(first.witness, 4, 7, "agent".into()).unwrap()
    else {
        panic!("expected replacement")
    };
    assert_eq!(&*after.text, "one agent");

    let service = client.into_transport();
    assert_eq!(service.usage().requests, 2);
    assert_eq!(service.usage().accepted, 2);
    assert_eq!(service.usage().refused, 0);
}

#[test]
fn save_returns_adapter_evidence_replays_without_a_second_write_and_rejects_stale_witnesses() {
    let (fixture, mut service) = service("one two");
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::Snapshot(before)) =
        service.execute(request(client, 1, CommandV1::Snapshot))
    else {
        panic!("expected snapshot")
    };
    let ReplyV1::Accepted(OutcomeV1::Replaced(after)) = service.execute(request(
        client,
        2,
        CommandV1::Replace {
            expected: before.witness,
            start: 4,
            end: 7,
            inserted: "agent".into(),
        },
    )) else {
        panic!("expected replacement")
    };

    assert_eq!(
        service.execute(request(
            client,
            3,
            CommandV1::Save {
                expected: before.witness,
            },
        )),
        ReplyV1::Refused(RefusalV1::Stale)
    );

    let save = request(
        client,
        4,
        CommandV1::Save {
            expected: after.witness,
        },
    );
    let saved = service.execute(save.clone());
    let ReplyV1::Accepted(OutcomeV1::Saved(receipt)) = &saved else {
        panic!("expected save receipt")
    };
    assert_eq!(receipt.application, SaveApplicationV1::Written);
    assert_eq!(receipt.synchronization, SaveSynchronizationV1::Succeeded);
    assert!(receipt.no_future_effects);
    assert!(receipt.quiescent);
    assert!(!receipt.adapter_failure);
    assert!(!receipt.cleanup_failed);
    assert!(receipt.baseline_published);
    assert!(!receipt.outcome_unknown);
    assert_eq!(fs::read_to_string(&fixture.0).unwrap(), "one agent");

    fs::write(&fixture.0, "external-after-save").unwrap();
    assert_eq!(service.execute(save), saved);
    assert_eq!(
        fs::read_to_string(&fixture.0).unwrap(),
        "external-after-save"
    );
}

#[test]
fn external_change_is_reported_without_overwrite_or_false_baseline_publication() {
    let (fixture, mut service) = service("alpha");
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::Snapshot(before)) =
        service.execute(request(client, 1, CommandV1::Snapshot))
    else {
        panic!("expected snapshot")
    };
    let ReplyV1::Accepted(OutcomeV1::Replaced(after)) = service.execute(request(
        client,
        2,
        CommandV1::Replace {
            expected: before.witness,
            start: 0,
            end: 5,
            inserted: "agent".into(),
        },
    )) else {
        panic!("expected replacement")
    };
    fs::write(&fixture.0, "external").unwrap();

    let ReplyV1::Accepted(OutcomeV1::Saved(receipt)) = service.execute(request(
        client,
        3,
        CommandV1::Save {
            expected: after.witness,
        },
    )) else {
        panic!("expected closed save receipt")
    };
    assert_eq!(receipt.application, SaveApplicationV1::NotWritten);
    assert!(!receipt.baseline_published);
    assert!(!receipt.outcome_unknown);
    assert_eq!(fs::read_to_string(&fixture.0).unwrap(), "external");
}

#[test]
fn durable_replace_survives_a_real_workspace_restart() {
    let (fixture, mut service) = service("alpha");
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::Snapshot(before)) =
        service.execute(request(client, 1, CommandV1::Snapshot))
    else {
        panic!("expected snapshot")
    };
    let operation = request(
        client,
        2,
        CommandV1::ReplaceAndSave {
            expected: before.witness,
            start: 0,
            end: 5,
            inserted: "agent".into(),
        },
    );
    let reply = service.execute(operation);
    let ReplyV1::Accepted(OutcomeV1::ReplacedAndSaved(receipt)) = &reply else {
        panic!("expected durable edit receipt")
    };
    assert!(matches!(receipt.persistence, EditPersistenceV1::Durable(_)));
    assert_eq!(fs::read_to_string(&fixture.0).unwrap(), "agent");

    assert_eq!(service.usage().replace_and_save_requests, 1);
    assert_eq!(service.usage().replace_and_save_durable, 1);

    drop(service);
    let workspace = Workspace::open(
        std::slice::from_ref(&fixture.0),
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
    let mut restarted = Service::new_with_action_ceiling(
        workspace,
        ServiceLimits::reference(),
        AuthorityActionSetV1::top(),
    )
    .unwrap();
    let restarted_client = restarted.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) =
        restarted.execute(request(restarted_client, 1, CommandV1::Snapshot))
    else {
        panic!("expected restarted snapshot")
    };
    assert_eq!(&*snapshot.text, "agent");
}

#[test]
fn durable_replace_exact_replay_does_not_repeat_the_write() {
    let (fixture, mut service) = service("alpha");
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::Snapshot(before)) =
        service.execute(request(client, 1, CommandV1::Snapshot))
    else {
        panic!("expected snapshot")
    };
    let operation = request(
        client,
        2,
        CommandV1::ReplaceAndSave {
            expected: before.witness,
            start: 0,
            end: 5,
            inserted: "agent".into(),
        },
    );
    let reply = service.execute(operation.clone());
    assert!(matches!(
        reply,
        ReplyV1::Accepted(OutcomeV1::ReplacedAndSaved(_))
    ));

    // A matching replay returns retained evidence. It does not dispatch a
    // second write that could overwrite a later external writer.
    fs::write(&fixture.0, "external-after-reply").unwrap();
    assert_eq!(service.execute(operation), reply);
    assert_eq!(
        fs::read_to_string(&fixture.0).unwrap(),
        "external-after-reply"
    );
    assert_eq!(service.usage().replace_and_save_requests, 1);
    assert_eq!(service.usage().replace_and_save_durable, 1);
}

#[test]
fn durable_replace_preserves_external_writer_and_reports_definitive_non_durability() {
    let (fixture, mut service) = service("alpha");
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::Snapshot(before)) =
        service.execute(request(client, 1, CommandV1::Snapshot))
    else {
        panic!("expected snapshot")
    };
    fs::write(&fixture.0, "external").unwrap();
    let ReplyV1::Accepted(OutcomeV1::ReplacedAndSaved(receipt)) = service.execute(request(
        client,
        2,
        CommandV1::ReplaceAndSave {
            expected: before.witness,
            start: 0,
            end: 5,
            inserted: "agent".into(),
        },
    )) else {
        panic!("expected partial edit receipt")
    };
    let EditPersistenceV1::NotDurable(save) = receipt.persistence else {
        panic!(
            "expected definitive non-durable evidence, got {:?}",
            receipt.persistence
        )
    };
    assert_eq!(save.application, SaveApplicationV1::NotWritten);
    assert!(save.no_future_effects && save.quiescent);
    assert!(!save.baseline_published && !save.outcome_unknown);
    assert_eq!(fs::read_to_string(&fixture.0).unwrap(), "external");

    // The edit remains explicit in memory; partial success was not erased.
    let ReplyV1::Accepted(OutcomeV1::Snapshot(current)) =
        service.execute(request(client, 3, CommandV1::Snapshot))
    else {
        panic!("expected current snapshot")
    };
    assert_eq!(&*current.text, "agent");
}

#[test]
fn durable_replace_reports_post_edit_save_admission_without_erasing_the_edit() {
    let (fixture, mut service) = service_with_save_task_capacity("alpha", 0);
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::Snapshot(before)) =
        service.execute(request(client, 1, CommandV1::Snapshot))
    else {
        panic!("expected snapshot")
    };
    let ReplyV1::Accepted(OutcomeV1::ReplacedAndSaved(receipt)) = service.execute(request(
        client,
        2,
        CommandV1::ReplaceAndSave {
            expected: before.witness,
            start: 0,
            end: 5,
            inserted: "agent".into(),
        },
    )) else {
        panic!("expected accepted partial edit")
    };
    assert_eq!(
        receipt.persistence,
        EditPersistenceV1::NotAttempted(RefusalV1::EditorCapacity)
    );
    assert_eq!(&*receipt.snapshot.text, "agent");
    assert_eq!(fs::read_to_string(&fixture.0).unwrap(), "alpha");
    assert_eq!(service.usage().replace_and_save_not_attempted, 1);
}

#[test]
fn durable_replace_requires_both_edit_and_persist_authority_before_any_effect() {
    let fixture = Fixture::new("alpha");
    let workspace = Workspace::open(
        std::slice::from_ref(&fixture.0),
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
        AuthorityActionSetV1::singleton(AuthorityActionV1::EditSource),
    )
    .unwrap();
    let client = service.connect().unwrap();
    assert_eq!(
        service.execute(request(
            client,
            1,
            CommandV1::ReplaceAndSave {
                expected: vellum_protocol::VersionWitness::new(1).unwrap(),
                start: 0,
                end: 5,
                inserted: "agent".into(),
            },
        )),
        ReplyV1::Refused(RefusalV1::AuthorityDenied)
    );
    assert_eq!(fs::read_to_string(&fixture.0).unwrap(), "alpha");
    assert_eq!(service.usage().replace_and_save_requests, 0);
    assert_eq!(service.usage().authority_checks, 2);
    assert_eq!(service.usage().authority_granted, 1);
    assert_eq!(service.usage().authority_denied, 1);
}

#[test]
fn durable_replace_refuses_a_stale_source_revision_before_edit_or_persistence() {
    let (fixture, mut service) = service("alpha");
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::Snapshot(before)) =
        service.execute(request(client, 1, CommandV1::Snapshot))
    else {
        panic!("expected snapshot")
    };
    let ReplyV1::Accepted(OutcomeV1::Replaced(_)) = service.execute(request(
        client,
        2,
        CommandV1::Replace {
            expected: before.witness,
            start: 0,
            end: 5,
            inserted: "newer".into(),
        },
    )) else {
        panic!("expected replacement")
    };
    assert_eq!(
        service.execute(request(
            client,
            3,
            CommandV1::ReplaceAndSave {
                expected: before.witness,
                start: 0,
                end: 5,
                inserted: "stale".into(),
            },
        )),
        ReplyV1::Refused(RefusalV1::Stale)
    );
    assert_eq!(fs::read_to_string(&fixture.0).unwrap(), "alpha");
}

#[test]
fn superseded_terminal_save_tasks_are_reclaimed_within_the_live_task_bound() {
    let (fixture, mut service) = service_with_save_task_capacity("a", 2);
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) =
        service.execute(request(client, 1, CommandV1::Snapshot))
    else {
        panic!("expected snapshot")
    };
    let mut witness = snapshot.witness;
    let mut expected = "a";
    for iteration in 0..12 {
        let inserted = if iteration % 2 == 0 { "b" } else { "a" };
        let ReplyV1::Accepted(OutcomeV1::Replaced(snapshot)) = service.execute(request(
            client,
            2 + iteration * 2,
            CommandV1::Replace {
                expected: witness,
                start: 0,
                end: 1,
                inserted: inserted.into(),
            },
        )) else {
            panic!("expected replacement within bounded task lifetime")
        };
        witness = snapshot.witness;
        let ReplyV1::Accepted(OutcomeV1::Saved(receipt)) = service.execute(request(
            client,
            3 + iteration * 2,
            CommandV1::Save { expected: witness },
        )) else {
            panic!("expected repeated save within a two-task live bound")
        };
        assert!(receipt.baseline_published);
        expected = inserted;
    }
    assert_eq!(fs::read_to_string(&fixture.0).unwrap(), expected);
}

#[test]
fn empty_workspace_accepts_clients_exposes_content_free_inventory_and_refuses_snapshot() {
    let service = empty_service(ServiceLimits::reference());
    let mut client = Client::connect(service).unwrap();

    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) =
        client.workspace_inventory().unwrap()
    else {
        panic!("expected workspace inventory")
    };
    assert!(inventory.documents.is_empty());
    assert!(inventory.roots.is_empty());
    assert!(!format!("{inventory:?}").contains('/'));
    assert_eq!(
        client.snapshot(),
        Ok(ReplyV1::Refused(RefusalV1::NoActiveDocument))
    );

    let mut other = Client::connect(empty_service(ServiceLimits::reference())).unwrap();
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(other_inventory)) =
        other.workspace_inventory().unwrap()
    else {
        panic!("expected second workspace inventory")
    };
    assert_ne!(inventory.workspace, other_inventory.workspace);

    let service = client.into_transport();
    assert_eq!(service.usage().active_clients, 1);
    assert_eq!(service.usage().retained_requests, 2);
}

#[test]
fn request_retention_is_bounded_without_eviction_and_observes_each_replay_outcome() {
    let mut limits = ServiceLimits::reference();
    limits.max_requests = 1;
    let (_fixture, mut service) = service_with_limits("alpha", limits);
    let client = service.connect().unwrap();
    let original = request(client, 1, CommandV1::Snapshot);

    let accepted = service.execute(original.clone());
    assert_eq!(service.execute(original.clone()), accepted);
    assert_eq!(
        service.execute(request(
            client,
            1,
            CommandV1::Replace {
                expected: vellum_protocol::VersionWitness::new(1).unwrap(),
                start: 0,
                end: 0,
                inserted: "changed".into(),
            },
        )),
        ReplyV1::Refused(RefusalV1::RequestConflict)
    );
    assert_eq!(
        service.execute(request(client, 2, CommandV1::Snapshot)),
        ReplyV1::Refused(RefusalV1::RequestCapacity)
    );

    // Capacity never evicts the retained receipt: matching replay and changed
    // input remain distinguishable after the register becomes full.
    assert_eq!(service.execute(original), accepted);
    assert_eq!(service.usage().retained_requests, 1);
    assert_eq!(service.usage().request_new, 1);
    assert_eq!(service.usage().request_matching_replay, 2);
    assert_eq!(service.usage().request_changed_input, 1);
    assert_eq!(service.usage().request_capacity_full, 1);
}

#[test]
fn disconnect_is_idempotent_reclaims_client_state_and_preserves_lifetime_bounds() {
    let mut limits = ServiceLimits::reference();
    limits.max_clients = 1;
    limits.max_issued_clients = 2;
    let (_fixture, mut service) = service_with_limits("alpha", limits);
    let first = service.connect().unwrap();
    assert!(matches!(
        service.execute(request(first, 1, CommandV1::Snapshot)),
        ReplyV1::Accepted(OutcomeV1::Snapshot(_))
    ));
    service.disconnect(first).unwrap();
    service.disconnect(first).unwrap();
    assert_eq!(service.usage().active_clients, 0);
    assert_eq!(service.usage().retired_clients, 1);
    assert_eq!(service.usage().retained_requests, 0);
    assert_eq!(service.usage().retained_witnesses, 0);

    let second = service.connect().unwrap();
    assert_ne!(first, second);
    service.disconnect(second).unwrap();
    assert_eq!(service.connect(), Err(RefusalV1::ClientLifetimeCapacity));
}

#[test]
fn empty_service_adds_files_and_selects_stable_scoped_documents() {
    let first = Fixture::new("first");
    let second = Fixture::new("second");
    let mut service = empty_service(ServiceLimits::reference());
    let client = service.connect().unwrap();

    let ReplyV1::Accepted(OutcomeV1::FileAdmitted(first_receipt)) = service.execute(request(
        client,
        1,
        CommandV1::AddFile {
            locator: first.0.to_string_lossy().into_owned(),
        },
    )) else {
        panic!("expected first file admission")
    };
    let first_id = first_receipt.document;
    assert!(first_receipt.root.is_some());
    let ReplyV1::Accepted(OutcomeV1::FileAdmitted(second_receipt)) = service.execute(request(
        client,
        2,
        CommandV1::AddFile {
            locator: second.0.to_string_lossy().into_owned(),
        },
    )) else {
        panic!("expected second file admission")
    };
    assert_ne!(second_receipt.document, first_id);
    assert!(second_receipt.root.is_some());
    let ReplyV1::Accepted(OutcomeV1::FileAdmitted(joined)) = service.execute(request(
        client,
        3,
        CommandV1::AddFile {
            locator: first.0.to_string_lossy().into_owned(),
        },
    )) else {
        panic!("expected existing file admission")
    };
    assert_eq!(joined.document, first_id);
    assert_eq!(joined.root, None);
    assert_eq!(
        joined.status,
        vellum_protocol::FileAdmissionStatusV1::AlreadyPresent
    );

    assert_eq!(
        service.execute(request(
            client,
            4,
            CommandV1::SelectDocument { document: first_id },
        )),
        ReplyV1::Accepted(OutcomeV1::DocumentSelected(first_id))
    );
    let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) =
        service.execute(request(client, 5, CommandV1::Snapshot))
    else {
        panic!("expected selected snapshot")
    };
    assert_eq!(&*snapshot.text, "first");

    let unknown =
        vellum_protocol::WorkspaceDocumentIdV1::new(second_receipt.document.workspace(), 99)
            .unwrap();
    assert_eq!(
        service.execute(request(
            client,
            6,
            CommandV1::SelectDocument { document: unknown },
        )),
        ReplyV1::Refused(RefusalV1::UnknownDocument)
    );
    let foreign = vellum_protocol::WorkspaceDocumentIdV1::new(
        vellum_protocol::WorkspaceIdV1::new(second_receipt.document.workspace().get() + 1).unwrap(),
        first_id.get(),
    )
    .unwrap();
    assert_eq!(
        service.execute(request(
            client,
            7,
            CommandV1::SelectDocument { document: foreign },
        )),
        ReplyV1::Refused(RefusalV1::UnknownDocument)
    );
    let usage = service.usage();
    assert_eq!(usage.file_admission_requests, 3);
    assert_eq!(usage.files_added, 2);
    assert_eq!(usage.files_already_present, 1);
}

#[test]
fn refused_file_addition_leaves_the_published_inventory_unchanged() {
    let first = Fixture::new("first");
    let second = Fixture::new("second");
    let mut service = empty_service_with_document_capacity(1);
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::FileAdmitted(_)) = service.execute(request(
        client,
        1,
        CommandV1::AddFile {
            locator: first.0.to_string_lossy().into_owned(),
        },
    )) else {
        panic!("expected first admission")
    };
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(before)) =
        service.execute(request(client, 2, CommandV1::WorkspaceInventory))
    else {
        panic!("expected inventory")
    };

    assert_eq!(
        service.execute(request(
            client,
            3,
            CommandV1::AddFile {
                locator: second.0.to_string_lossy().into_owned(),
            },
        )),
        ReplyV1::Refused(RefusalV1::EditorCapacity)
    );
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(after)) =
        service.execute(request(client, 4, CommandV1::WorkspaceInventory))
    else {
        panic!("expected inventory")
    };
    assert_eq!(after, before);
}

#[test]
fn directory_addition_is_bounded_replayable_and_does_not_reassign_existing_ids() {
    let existing = Fixture::new("existing");
    let directory = DirectoryFixture::new(&[("b", "second"), ("a", "first")]);
    let mut service = empty_service_with_discovery(ServiceLimits::reference());
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::FileAdmitted(before)) = service.execute(request(
        client,
        1,
        CommandV1::AddFile {
            locator: existing.0.to_string_lossy().into_owned(),
        },
    )) else {
        panic!("expected file admission")
    };
    let retained = before.document;
    let command = request(
        client,
        2,
        CommandV1::AddDirectory {
            locator: directory.0.to_string_lossy().into_owned(),
        },
    );
    let reply = service.execute(command.clone());
    let ReplyV1::Accepted(OutcomeV1::DirectoryAdmitted(admission)) = &reply else {
        panic!("expected directory admission")
    };
    assert_eq!(admission.added, 2);
    assert_eq!(admission.joined, 0);
    assert_eq!(admission.added_documents.len(), 2);
    assert!(!admission.added_documents.contains(&retained));
    assert_eq!(service.execute(command.clone()), reply);
    assert_eq!(
        service.execute(request(
            client,
            2,
            CommandV1::AddDirectory {
                locator: "changed-private-path".into(),
            },
        )),
        ReplyV1::Refused(RefusalV1::RequestConflict)
    );
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(replayed_inventory)) =
        service.execute(request(client, 3, CommandV1::WorkspaceInventory))
    else {
        panic!("expected inventory")
    };
    assert_eq!(replayed_inventory.documents.len(), 3);
    assert_eq!(replayed_inventory.documents[0], retained);
    assert_eq!(replayed_inventory.roots.len(), 2);
    assert_eq!(
        &replayed_inventory.documents[1..],
        admission.added_documents.as_slice()
    );
    let ReplyV1::Accepted(OutcomeV1::DirectoryAdmitted(repeated)) = service.execute(request(
        client,
        4,
        CommandV1::AddDirectory {
            locator: directory.0.to_string_lossy().into_owned(),
        },
    )) else {
        panic!("expected repeated directory admission")
    };
    assert_eq!(repeated.root, admission.root);
    assert_eq!(repeated.added, 0);
    assert_eq!(repeated.joined, 2);
    assert!(repeated.added_documents.is_empty());
    let usage = service.usage();
    assert_eq!(usage.directory_admission_requests, 2);
    assert_eq!(usage.directory_documents_added, 2);
    assert_eq!(usage.directory_documents_joined, 2);
    assert!(!format!("{command:?}").contains(directory.0.to_string_lossy().as_ref()));
}

fn identity_limits() -> WorkspaceIdentityLimitsV1 {
    WorkspaceIdentityLimitsV1 {
        max_documents: 8,
        max_roots: 8,
        max_encoded_bytes: 1024,
    }
}

fn workspace_for(paths: &[PathBuf]) -> Workspace {
    Workspace::open(
        paths,
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

#[test]
fn recovered_metadata_rebinds_only_exact_locators_despite_inventory_reordering() {
    let first = Fixture::new("first");
    let second = Fixture::new("second");
    let paths = vec![first.0.clone(), second.0.clone()];
    let mut original = Service::new(workspace_for(&paths), ServiceLimits::reference()).unwrap();
    let client = original.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(before)) =
        original.execute(request(client, 1, CommandV1::WorkspaceInventory))
    else {
        panic!("expected inventory")
    };
    let metadata = original.identity_metadata(identity_limits()).unwrap();
    drop(original);

    let reversed = vec![second.0.clone(), first.0.clone()];
    let mut restarted = Service::new_with_recovered_identity(
        workspace_for(&reversed),
        ServiceLimits::reference(),
        metadata,
    )
    .unwrap();
    let restarted_client = restarted.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(after)) =
        restarted.execute(request(restarted_client, 1, CommandV1::WorkspaceInventory))
    else {
        panic!("expected recovered inventory")
    };
    assert_eq!(after.workspace, before.workspace);
    assert_eq!(
        after.documents,
        vec![before.documents[1], before.documents[0]]
    );
    assert_eq!(after.roots[0].id, before.roots[1].id);
    assert_eq!(after.roots[1].id, before.roots[0].id);
    assert_eq!(
        restarted.execute(request(
            restarted_client,
            2,
            CommandV1::SelectDocument {
                document: before.documents[0],
            },
        )),
        ReplyV1::Accepted(OutcomeV1::DocumentSelected(before.documents[0]))
    );
}

#[test]
fn recovered_allocator_resumes_at_issued_prefix_not_reservation_ceiling() {
    let first = Fixture::new("first");
    let second = Fixture::new("second");
    let mut original = Service::new(
        workspace_for(std::slice::from_ref(&first.0)),
        ServiceLimits::reference(),
    )
    .unwrap();
    let metadata = original.identity_metadata(identity_limits()).unwrap();
    assert_eq!(metadata.document_issued_through(), 1);
    assert!(metadata.document_reserved_through() > 1);

    let actions = AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
        AuthorityActionV1::AdmitFile,
    ));
    let mut restarted = Service::new_with_recovered_identity_and_action_ceiling(
        workspace_for(std::slice::from_ref(&first.0)),
        ServiceLimits::reference(),
        metadata,
        actions,
    )
    .unwrap();
    let client = restarted.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::FileAdmitted(receipt)) = restarted.execute(request(
        client,
        1,
        CommandV1::AddFile {
            locator: second.0.to_str().unwrap().to_owned(),
        },
    )) else {
        panic!("expected exact admission receipt")
    };
    assert_eq!(receipt.document.get(), 2);
    let advanced = restarted.identity_metadata(identity_limits()).unwrap();
    assert_eq!(advanced.document_issued_through(), 2);
    assert!(advanced.document_reserved_through() >= advanced.document_issued_through());
}

#[test]
fn recovered_workspace_advances_change_epoch_and_refuses_pre_restart_cursor() {
    let fixture = Fixture::new("epoch");
    let mut original = Service::new(
        workspace_for(std::slice::from_ref(&fixture.0)),
        ServiceLimits::reference(),
    )
    .unwrap();
    let client = original.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Page(page))) = original
        .execute(request(
            client,
            1,
            CommandV1::WorkspaceChanges {
                after: None,
                page_size: 1,
            },
        ))
    else {
        panic!("expected initial feed page")
    };
    let stale_cursor = page.next;
    let metadata = original.identity_metadata(identity_limits()).unwrap();
    drop(original);

    let mut restarted = Service::new_with_recovered_identity(
        workspace_for(std::slice::from_ref(&fixture.0)),
        ServiceLimits::reference(),
        metadata,
    )
    .unwrap();
    let client = restarted.connect().unwrap();
    assert_eq!(
        restarted.execute(request(
            client,
            1,
            CommandV1::WorkspaceChanges {
                after: Some(stale_cursor),
                page_size: 1,
            }
        )),
        ReplyV1::Refused(RefusalV1::ForeignChangeCursor)
    );
}

#[test]
fn recovered_metadata_refuses_same_shape_with_different_locator() {
    let first = Fixture::new("binding-original");
    let replacement = Fixture::new("binding-replacement");
    let mut original = Service::new(
        workspace_for(std::slice::from_ref(&first.0)),
        ServiceLimits::reference(),
    )
    .unwrap();
    let metadata = original.identity_metadata(identity_limits()).unwrap();
    assert!(matches!(
        Service::new_with_recovered_identity(
            workspace_for(std::slice::from_ref(&replacement.0)),
            ServiceLimits::reference(),
            metadata,
        ),
        Err(ServiceBuildError::IdentityMismatch)
    ));
}

#[test]
fn recovered_identity_rejects_exhausted_generation_before_service_publication() {
    let metadata = WorkspaceIdentityMetadataV1::try_new(
        71,
        u64::MAX,
        WorkspaceIdentityAllocationV1::try_new(0, 3, 0, 3).unwrap(),
        identity_limits(),
    )
    .unwrap();
    assert!(matches!(
        Service::new_with_recovered_identity(
            Workspace::empty(vellum_editor::reference_workspace_profile(
                LocalFilesystemFactory::new(FilesystemLimits {
                    max_resources: 8,
                    max_bytes: 4096,
                    max_stage_attempts: 8,
                    max_metadata_bytes: 4096,
                })
                .erase(),
            ))
            .unwrap(),
            ServiceLimits::reference(),
            metadata,
        ),
        Err(ServiceBuildError::CounterExhausted)
    ));
}

#[test]
fn inventory_pagination_is_bounded_exact_replayable_and_reports_end() {
    let (_fixture, mut service) = service("alpha");
    let client = service.connect().unwrap();
    let first_request = request(
        client,
        1,
        CommandV1::WorkspaceInventoryPage {
            cursor: None,
            page_size: 1,
        },
    );
    let first_reply = service.execute(first_request.clone());
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(first)) = &first_reply else {
        panic!("expected first inventory page")
    };
    assert_eq!(first.documents.len(), 1);
    assert!(first.roots.is_empty());
    let cursor = first.next.expect("two-entry inventory has a continuation");
    assert_eq!(
        format!("{cursor:?}"),
        "WorkspaceInventoryCursorV1(<opaque>)"
    );

    assert_eq!(service.execute(first_request), first_reply);
    let before_continue = service.usage();
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(last)) = service.execute(request(
        client,
        2,
        CommandV1::WorkspaceInventoryPage {
            cursor: Some(cursor),
            page_size: 1,
        },
    )) else {
        panic!("expected final inventory page")
    };
    assert!(last.documents.is_empty());
    assert_eq!(last.roots.len(), 1);
    assert_eq!(last.next, None);
    let usage = service.usage();
    assert_eq!(usage.inventory_page_requests, 2);
    assert_eq!(usage.inventory_page_started, 1);
    assert_eq!(usage.inventory_page_continued, 1);
    assert_eq!(usage.inventory_page_completed, 1);
    assert_eq!(usage.authority_checks, before_continue.authority_checks + 1);
}

#[test]
fn inventory_pagination_refuses_foreign_stale_invalid_and_oversized_cursors() {
    let (fixture, mut service) = service("alpha");
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(first)) = service.execute(request(
        client,
        1,
        CommandV1::WorkspaceInventoryPage {
            cursor: None,
            page_size: 1,
        },
    )) else {
        panic!("expected first page")
    };
    let cursor = first.next.unwrap();
    let foreign_workspace = vellum_protocol::WorkspaceIdV1::new(first.workspace.get() + 1).unwrap();
    let foreign =
        WorkspaceInventoryCursorV1::new(foreign_workspace, cursor.generation(), cursor.offset())
            .unwrap();
    assert_eq!(
        service.execute(request(
            client,
            2,
            CommandV1::WorkspaceInventoryPage {
                cursor: Some(foreign),
                page_size: 1,
            },
        )),
        ReplyV1::Refused(RefusalV1::ForeignInventoryCursor)
    );
    let invalid =
        WorkspaceInventoryCursorV1::new(first.workspace, cursor.generation(), u64::MAX).unwrap();
    assert_eq!(
        service.execute(request(
            client,
            3,
            CommandV1::WorkspaceInventoryPage {
                cursor: Some(invalid),
                page_size: 1,
            },
        )),
        ReplyV1::Refused(RefusalV1::InvalidInventoryCursor)
    );
    assert_eq!(
        service.execute(request(
            client,
            4,
            CommandV1::WorkspaceInventoryPage {
                cursor: None,
                page_size: 5,
            },
        )),
        ReplyV1::Refused(RefusalV1::InputLimit)
    );
    assert_eq!(
        service.execute(request(
            client,
            7,
            CommandV1::WorkspaceInventoryPage {
                cursor: None,
                page_size: 0,
            },
        )),
        ReplyV1::Refused(RefusalV1::InputLimit)
    );

    let added = Fixture::new("beta");
    service.execute(request(
        client,
        5,
        CommandV1::AddFile {
            locator: added.0.to_string_lossy().into_owned(),
        },
    ));
    assert_eq!(
        service.execute(request(
            client,
            6,
            CommandV1::WorkspaceInventoryPage {
                cursor: Some(cursor),
                page_size: 1,
            },
        )),
        ReplyV1::Refused(RefusalV1::StaleInventoryCursor)
    );
    let usage = service.usage();
    assert_eq!(usage.inventory_page_foreign, 1);
    assert_eq!(usage.inventory_page_invalid, 1);
    assert_eq!(usage.inventory_page_limited, 2);
    assert_eq!(usage.inventory_page_stale, 1);
    drop(fixture);
}

#[test]
fn refused_page_does_not_publish_any_partial_identity_projection() {
    let mut service = empty_service(ServiceLimits::reference());
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(empty)) = service.execute(request(
        client,
        1,
        CommandV1::WorkspaceInventoryPage {
            cursor: None,
            page_size: 1,
        },
    )) else {
        panic!("expected empty terminal page")
    };
    assert_eq!(empty.next, None);
    let forged = WorkspaceInventoryCursorV1::new(empty.workspace, 1, 1).unwrap();
    assert_eq!(
        service.execute(request(
            client,
            2,
            CommandV1::WorkspaceInventoryPage {
                cursor: Some(forged),
                page_size: 1,
            },
        )),
        ReplyV1::Refused(RefusalV1::InvalidInventoryCursor)
    );

    let fixture = Fixture::new("new");
    let ReplyV1::Accepted(OutcomeV1::FileAdmitted(after)) = service.execute(request(
        client,
        3,
        CommandV1::AddFile {
            locator: fixture.0.to_string_lossy().into_owned(),
        },
    )) else {
        panic!("expected file admission")
    };
    assert_eq!(after.document.get(), 1);
    assert_eq!(after.root.unwrap().get(), 1);
}

#[test]
fn membership_removal_blocks_live_views_revokes_witnesses_and_replays_exactly() {
    let (fixture, mut service) = service("alpha");
    let client = service.connect().unwrap();
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) =
        service.execute(request(client, 1, CommandV1::WorkspaceInventory))
    else {
        panic!("expected inventory")
    };
    let document = inventory.documents[0];
    let root = inventory.roots[0].id;
    assert_eq!(
        service
            .identity_metadata(identity_limits())
            .unwrap()
            .generation(),
        1
    );
    let ReplyV1::Accepted(OutcomeV1::Snapshot(snapshot)) =
        service.execute(request(client, 2, CommandV1::Snapshot))
    else {
        panic!("expected snapshot")
    };

    assert_eq!(
        service.execute(request(client, 3, CommandV1::RemoveDocument { document })),
        ReplyV1::Refused(RefusalV1::DocumentInUse)
    );
    assert!(matches!(
        service.execute(request(client, 4, CommandV1::CloseView)),
        ReplyV1::Accepted(OutcomeV1::ViewClosed(ViewCloseStatusV1::Closed))
    ));
    assert!(matches!(
        service.execute(request(client, 5, CommandV1::RemoveRoot { root })),
        ReplyV1::Accepted(OutcomeV1::RootRemoved(
            vellum_protocol::RootRemovalReceiptV1 {
                status: vellum_protocol::MembershipRemovalStatusV1::Removed,
                ..
            }
        ))
    ));
    assert_eq!(
        service
            .identity_metadata(identity_limits())
            .unwrap()
            .generation(),
        2
    );
    let removal = request(client, 6, CommandV1::RemoveDocument { document });
    let accepted = ReplyV1::Accepted(OutcomeV1::DocumentRemoved(
        vellum_protocol::DocumentRemovalReceiptV1 {
            document,
            status: vellum_protocol::MembershipRemovalStatusV1::Removed,
        },
    ));
    assert_eq!(service.execute(removal.clone()), accepted);
    assert_eq!(service.execute(removal), accepted);
    assert_eq!(
        service
            .identity_metadata(identity_limits())
            .unwrap()
            .generation(),
        3
    );

    // The retained request register preserves an exact old read reply, but a
    // fresh effect cannot use the revoked witness after a new member is opened.
    assert!(matches!(
        service.execute(request(client, 2, CommandV1::Snapshot)),
        ReplyV1::Accepted(OutcomeV1::Snapshot(_))
    ));
    assert!(matches!(
        service.execute(request(client, 7, CommandV1::RemoveDocument { document })),
        ReplyV1::Accepted(OutcomeV1::DocumentRemoved(
            vellum_protocol::DocumentRemovalReceiptV1 {
                status: vellum_protocol::MembershipRemovalStatusV1::AlreadyRemoved,
                ..
            }
        ))
    ));
    assert_eq!(
        service
            .identity_metadata(identity_limits())
            .unwrap()
            .generation(),
        3
    );
    let ReplyV1::Accepted(OutcomeV1::FileAdmitted(_)) = service.execute(request(
        client,
        8,
        CommandV1::AddFile {
            locator: fixture.0.to_string_lossy().into_owned(),
        },
    )) else {
        panic!("expected fresh membership")
    };
    assert!(matches!(
        service.execute(request(client, 9, CommandV1::OpenView)),
        ReplyV1::Accepted(OutcomeV1::ViewOpened(_))
    ));
    assert_eq!(
        service.execute(request(
            client,
            10,
            CommandV1::Save {
                expected: snapshot.witness,
            },
        )),
        ReplyV1::Refused(RefusalV1::UnknownWitness)
    );
    assert_eq!(fs::read_to_string(&fixture.0).unwrap(), "alpha");
    let usage = service.usage();
    assert_eq!(usage.document_removal_in_use, 1);
    assert_eq!(usage.documents_removed, 1);
    assert_eq!(usage.documents_already_removed, 1);
    assert_eq!(usage.roots_removed, 1);
    assert_eq!(usage.witnesses_revoked_by_removal, 1);
}

#[test]
fn bounded_change_feed_replays_exact_pages_and_reports_retention_gaps() {
    let mut limits = ServiceLimits::reference();
    limits.max_change_events = 1;
    let (_fixture, mut service) = service_with_limits("alpha", limits);
    let writer = service.connect().unwrap();
    let observer = service.connect().unwrap();

    let ReplyV1::Accepted(OutcomeV1::Snapshot(first)) =
        service.execute(request(writer, 1, CommandV1::Snapshot))
    else {
        panic!("expected snapshot")
    };
    let ReplyV1::Accepted(OutcomeV1::Replaced(second)) = service.execute(request(
        writer,
        2,
        CommandV1::Replace {
            expected: first.witness,
            start: 0,
            end: 0,
            inserted: "x".into(),
        },
    )) else {
        panic!("expected replacement")
    };

    let poll = request(
        observer,
        1,
        CommandV1::WorkspaceChanges {
            after: None,
            page_size: 1,
        },
    );
    let first_page = service.execute(poll.clone());
    let ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Page(page))) =
        &first_page
    else {
        panic!("expected change page")
    };
    assert_eq!(page.changes.len(), 1);
    assert!(
        matches!(page.changes[0].kind, WorkspaceChangeKindV1::DocumentChanged { source_revision, .. } if source_revision == second.revision)
    );
    assert_eq!(
        service.execute(poll),
        first_page,
        "exact request replay must retain its original page"
    );

    let ReplyV1::Accepted(OutcomeV1::Replaced(_)) = service.execute(request(
        writer,
        3,
        CommandV1::Replace {
            expected: second.witness,
            start: 0,
            end: 0,
            inserted: "y".into(),
        },
    )) else {
        panic!("expected second replacement")
    };

    let ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Gap(gap))) = service
        .execute(request(
            observer,
            2,
            CommandV1::WorkspaceChanges {
                after: Some(
                    WorkspaceChangeCursorV1::new(page.workspace, page.next.epoch(), 0).unwrap(),
                ),
                page_size: 1,
            },
        ))
    else {
        panic!("expected explicit retention gap")
    };
    assert_eq!(gap.requested_after, 0);
    assert_eq!(gap.oldest_available, 2);
    assert_eq!(gap.observed_head, 2);
    let foreign = vellum_protocol::WorkspaceIdV1::new(page.workspace.get() + 1).unwrap();
    assert_eq!(
        service.execute(request(
            observer,
            3,
            CommandV1::WorkspaceChanges {
                after: Some(WorkspaceChangeCursorV1::new(foreign, page.next.epoch(), 0).unwrap()),
                page_size: 1,
            }
        )),
        ReplyV1::Refused(RefusalV1::ForeignChangeCursor)
    );
    assert_eq!(
        service.execute(request(
            observer,
            4,
            CommandV1::WorkspaceChanges {
                after: Some(
                    WorkspaceChangeCursorV1::new(page.workspace, page.next.epoch(), 3).unwrap()
                ),
                page_size: 1,
            }
        )),
        ReplyV1::Refused(RefusalV1::InvalidChangeCursor)
    );
    let usage = service.usage();
    assert_eq!(usage.change_feed_events_published, 2);
    assert_eq!(usage.change_feed_events_evicted, 1);
    assert_eq!(usage.change_feed_gaps, 1);
}

#[test]
fn bounded_multi_client_schedules_preserve_views_witnesses_replay_and_change_order() {
    // These are every ordering of three independent, client-local view
    // transitions. The service is deliberately serialized; this explores
    // deterministic command interleavings rather than making a thread-safety
    // or parallel-execution claim.
    const SCHEDULES: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];

    let first = Fixture::new("alpha-private-source");
    let second = Fixture::new("beta-private-source");
    for schedule in SCHEDULES {
        for human_wins in [true, false] {
            fs::write(&first.0, "alpha-private-source").unwrap();
            fs::write(&second.0, "beta-private-source").unwrap();
            let paths = vec![first.0.clone(), second.0.clone()];
            let mut service = Service::new_with_action_ceiling(
                workspace_for(&paths),
                ServiceLimits::reference(),
                AuthorityActionSetV1::top(),
            )
            .unwrap();
            let human = service.connect().unwrap();
            let agent = service.connect().unwrap();
            let observer = service.connect().unwrap();

            let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) =
                service.execute(request(human, 1, CommandV1::WorkspaceInventory))
            else {
                panic!("expected shared inventory")
            };
            let first_document = inventory.documents[0];
            let second_document = inventory.documents[1];

            for step in schedule {
                let reply = match step {
                    0 => service.execute(request(
                        human,
                        2,
                        CommandV1::SelectDocument {
                            document: first_document,
                        },
                    )),
                    1 => service.execute(request(
                        agent,
                        1,
                        CommandV1::SelectDocument {
                            document: second_document,
                        },
                    )),
                    2 => service.execute(request(observer, 1, CommandV1::CloseView)),
                    _ => unreachable!(),
                };
                assert!(matches!(reply, ReplyV1::Accepted(_)));
            }

            let ReplyV1::Accepted(OutcomeV1::Snapshot(human_first)) =
                service.execute(request(human, 3, CommandV1::Snapshot))
            else {
                panic!("human view lost its selection")
            };
            let ReplyV1::Accepted(OutcomeV1::Snapshot(agent_second)) =
                service.execute(request(agent, 2, CommandV1::Snapshot))
            else {
                panic!("agent view lost its selection")
            };
            assert_eq!(&*human_first.text, "alpha-private-source");
            assert_eq!(&*agent_second.text, "beta-private-source");
            assert!(matches!(
                service.execute(request(observer, 2, CommandV1::Snapshot)),
                ReplyV1::Refused(RefusalV1::NoActiveDocument)
            ));
            assert!(matches!(
                service.execute(request(observer, 3, CommandV1::OpenView)),
                ReplyV1::Accepted(OutcomeV1::ViewOpened(_))
            ));

            assert_eq!(
                service.execute(request(
                    agent,
                    3,
                    CommandV1::SelectDocument {
                        document: first_document,
                    },
                )),
                ReplyV1::Accepted(OutcomeV1::DocumentSelected(first_document))
            );
            let ReplyV1::Accepted(OutcomeV1::Snapshot(agent_first)) =
                service.execute(request(agent, 4, CommandV1::Snapshot))
            else {
                panic!("expected agent snapshot of shared document")
            };
            assert_eq!(agent_first.revision, human_first.revision);

            let (winner, winner_id, winner_witness, loser, loser_id, loser_witness, insertion) =
                if human_wins {
                    (
                        human,
                        4,
                        human_first.witness,
                        agent,
                        5,
                        agent_first.witness,
                        "human-won-",
                    )
                } else {
                    (
                        agent,
                        5,
                        agent_first.witness,
                        human,
                        4,
                        human_first.witness,
                        "agent-won-",
                    )
                };
            let winning_request = request(
                winner,
                winner_id,
                CommandV1::Replace {
                    expected: winner_witness,
                    start: 0,
                    end: 0,
                    inserted: insertion.into(),
                },
            );
            let winning_reply = service.execute(winning_request.clone());
            let ReplyV1::Accepted(OutcomeV1::Replaced(winning_snapshot)) = &winning_reply else {
                panic!("first serialized writer must win")
            };
            assert_eq!(service.execute(winning_request), winning_reply);

            let losing_request = request(
                loser,
                loser_id,
                CommandV1::Replace {
                    expected: loser_witness,
                    start: 0,
                    end: 0,
                    inserted: "loser-secret".into(),
                },
            );
            assert_eq!(
                service.execute(losing_request.clone()),
                ReplyV1::Refused(RefusalV1::Stale)
            );
            assert_eq!(
                service.execute(losing_request),
                ReplyV1::Refused(RefusalV1::Stale)
            );
            assert_eq!(
                service.execute(request(loser, loser_id, CommandV1::Snapshot)),
                ReplyV1::Refused(RefusalV1::RequestConflict),
                "changed input cannot reuse a retained request identity"
            );

            let winner_save_id = winner_id + 1;
            let save = request(
                winner,
                winner_save_id,
                CommandV1::Save {
                    expected: winning_snapshot.witness,
                },
            );
            let saved = service.execute(save.clone());
            assert!(matches!(saved, ReplyV1::Accepted(OutcomeV1::Saved(_))));
            assert_eq!(service.execute(save), saved);

            let loser_save_id = loser_id + 1;
            let stale_save = request(
                loser,
                loser_save_id,
                CommandV1::Save {
                    expected: loser_witness,
                },
            );
            assert_eq!(
                service.execute(stale_save.clone()),
                ReplyV1::Refused(RefusalV1::Stale)
            );
            assert_eq!(
                service.execute(stale_save),
                ReplyV1::Refused(RefusalV1::Stale)
            );

            let poll = request(
                observer,
                4,
                CommandV1::WorkspaceChanges {
                    after: None,
                    page_size: 1,
                },
            );
            let observed = service.execute(poll.clone());
            let ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Page(page))) =
                &observed
            else {
                panic!("expected first deterministic change page")
            };
            assert_eq!(page.observed_head, 1);
            assert_eq!(page.changes.len(), 1);
            assert_eq!(page.changes[0].revision(), 1);
            assert!(matches!(
                page.changes[0].kind,
                WorkspaceChangeKindV1::DocumentChanged {
                    document,
                    source_revision
                } if document == first_document && source_revision == winning_snapshot.revision
            ));
            assert_eq!(service.execute(poll), observed);
            let ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Page(end))) =
                service.execute(request(
                    observer,
                    5,
                    CommandV1::WorkspaceChanges {
                        after: Some(page.next),
                        page_size: 1,
                    },
                ))
            else {
                panic!("expected empty page at observed head")
            };
            assert!(end.changes.is_empty());
            assert_eq!(end.observed_head, 1);
            assert_eq!(end.next, page.next);

            let expected_source = format!("{insertion}alpha-private-source");
            assert_eq!(fs::read_to_string(&first.0).unwrap(), expected_source);
            assert_eq!(
                fs::read_to_string(&second.0).unwrap(),
                "beta-private-source"
            );
            let usage = service.usage();
            assert_eq!(usage.request_matching_replay, 5);
            assert_eq!(usage.request_changed_input, 1);
            assert_eq!(usage.change_feed_events_published, 1);
            assert_eq!(usage.change_feed_requests, 2);
            assert_eq!(usage.change_feed_pages, 2);
            assert_eq!(usage.change_feed_gaps, 0);
            assert_eq!(usage.stale, 4);
            assert_eq!(usage.conflicts, 1);

            let operational = format!("{usage:?} {page:?} {end:?}");
            for secret in [
                "alpha-private-source",
                "beta-private-source",
                "human-won-",
                "agent-won-",
                "loser-secret",
                first.0.to_str().unwrap(),
                second.0.to_str().unwrap(),
            ] {
                assert!(
                    !operational.contains(secret),
                    "operational evidence disclosed source or locator content"
                );
            }
        }
    }
}

#[test]
fn client_scoped_request_namespaces_do_not_bypass_read_only_authority() {
    let fixture = Fixture::new("authority-private-source");
    let workspace = workspace_for(std::slice::from_ref(&fixture.0));
    let mut service = Service::new(workspace, ServiceLimits::reference()).unwrap();
    let human = service.connect().unwrap();
    let agent = service.connect().unwrap();

    let ReplyV1::Accepted(OutcomeV1::Snapshot(human_snapshot)) =
        service.execute(request(human, 1, CommandV1::Snapshot))
    else {
        panic!("expected authorized human read")
    };
    assert!(matches!(
        service.execute(request(agent, 1, CommandV1::Snapshot)),
        ReplyV1::Accepted(OutcomeV1::Snapshot(_))
    ));

    let denied = request(
        human,
        2,
        CommandV1::Replace {
            expected: human_snapshot.witness,
            start: 0,
            end: 0,
            inserted: "forbidden-secret".into(),
        },
    );
    assert_eq!(
        service.execute(denied.clone()),
        ReplyV1::Refused(RefusalV1::AuthorityDenied)
    );
    assert_eq!(
        service.execute(denied),
        ReplyV1::Refused(RefusalV1::AuthorityDenied)
    );
    assert!(matches!(
        service.execute(request(agent, 2, CommandV1::Snapshot)),
        ReplyV1::Accepted(OutcomeV1::Snapshot(_))
    ));
    assert_eq!(
        service.execute(request(human, 2, CommandV1::Snapshot)),
        ReplyV1::Refused(RefusalV1::RequestConflict)
    );

    assert_eq!(
        fs::read_to_string(&fixture.0).unwrap(),
        "authority-private-source"
    );
    let usage = service.usage();
    assert_eq!(usage.authority_checks, 4);
    assert_eq!(usage.authority_granted, 3);
    assert_eq!(usage.authority_denied, 1);
    assert_eq!(usage.request_matching_replay, 1);
    assert_eq!(usage.request_changed_input, 1);
}

#[test]
fn membership_admission_publishes_only_resources_created_by_that_command() {
    let fixture = Fixture::new("new member");
    let mut service = empty_service(ServiceLimits::reference());
    let client = service.connect().unwrap();
    assert!(matches!(
        service.execute(request(
            client,
            1,
            CommandV1::AddFile {
                locator: fixture.0.to_string_lossy().into_owned(),
            }
        )),
        ReplyV1::Accepted(OutcomeV1::FileAdmitted(_))
    ));
    let ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Page(first))) = service
        .execute(request(
            client,
            2,
            CommandV1::WorkspaceChanges {
                after: None,
                page_size: 1,
            },
        ))
    else {
        panic!("expected document admission event")
    };
    assert!(matches!(
        first.changes[0].kind,
        WorkspaceChangeKindV1::DocumentAdded(_)
    ));
    let ReplyV1::Accepted(OutcomeV1::WorkspaceChanges(WorkspaceChangesV1::Page(second))) = service
        .execute(request(
            client,
            3,
            CommandV1::WorkspaceChanges {
                after: Some(first.next),
                page_size: 1,
            },
        ))
    else {
        panic!("expected root admission event")
    };
    assert!(matches!(
        second.changes[0].kind,
        WorkspaceChangeKindV1::RootAdded(_)
    ));
    assert_eq!(second.observed_head, 2);
}
