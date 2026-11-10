#![cfg(target_os = "macos")]

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
use vellum_client::Client;
use vellum_editor::Workspace;
use vellum_protocol::{CommandV1, OutcomeV1, RefusalV1, ReplyV1, RequestId, RequestV1};
use vellum_service::{Service, ServiceLimits, ServiceUsage};

static NEXT: AtomicU64 = AtomicU64::new(0);

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

fn service(source: &str) -> (Fixture, Service) {
    service_with_limits(source, ServiceLimits::reference())
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
    (fixture, Service::new(workspace, limits).unwrap())
}

fn request(client: vellum_protocol::ClientId, id: u64, command: CommandV1) -> RequestV1 {
    RequestV1 {
        client,
        request: RequestId::new(id).unwrap(),
        command,
    }
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
            accepted: 6,
            refused: 3,
            replayed: 1,
            conflicts: 1,
            stale: 1,
            invalid: 1,
            active_clients: 2,
            client_capacity: 2,
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
    let ReplyV1::Accepted(OutcomeV1::Snapshot(first)) = client.snapshot() else {
        panic!("expected snapshot")
    };
    let ReplyV1::Accepted(OutcomeV1::Replaced(after)) =
        client.replace(first.witness, 4, 7, "agent".into())
    else {
        panic!("expected replacement")
    };
    assert_eq!(&*after.text, "one agent");

    let service = client.into_transport();
    assert_eq!(service.usage().requests, 2);
    assert_eq!(service.usage().accepted, 2);
    assert_eq!(service.usage().refused, 0);
}
