//! Opt-in, deterministic replay/pagination/removal lifecycle qualification.
#![cfg(target_os = "macos")]
#![forbid(unsafe_code)]

use std::{collections::BTreeSet, env, fs, path::PathBuf};
use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
use vellum_editor::Workspace;
use vellum_host::authority::AuthorityActionSetV1;
use vellum_protocol::{
    ClientId, CommandV1, MembershipRemovalStatusV1, OutcomeV1, RefusalV1, ReplyV1, RequestId,
    RequestV1, ViewCloseStatusV1, WorkspaceInventoryCursorV1,
};
use vellum_service::{Service, ServiceLimits};

const DEFAULT_SEED: u64 = 0x5645_4c4c_554d_0003;
const DEFAULT_CYCLES: usize = 64;

struct Generator(u64);

impl Generator {
    fn next(&mut self) -> u64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value
    }
}

fn setting(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn request(client: ClientId, request: u64, command: CommandV1) -> RequestV1 {
    RequestV1 {
        client,
        request: RequestId::new(request).unwrap(),
        command,
    }
}

struct Fixture {
    directory: PathBuf,
    files: Vec<PathBuf>,
}

impl Fixture {
    fn new(cycle: usize) -> Result<Self, &'static str> {
        let directory = env::temp_dir().join(format!(
            "vellum-qualification-service-{}-{cycle}",
            std::process::id()
        ));
        fs::create_dir(&directory).map_err(|_| "fixture directory creation failed")?;
        let mut files = Vec::new();
        for ordinal in 0..4 {
            let path = directory.join(format!("source-{ordinal}.txt"));
            fs::write(&path, format!("source-{ordinal}"))
                .map_err(|_| "fixture file creation failed")?;
            files.push(path);
        }
        Ok(Self { directory, files })
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for file in &self.files {
            let _ = fs::remove_file(file);
        }
        let _ = fs::remove_dir(&self.directory);
    }
}

fn service(fixture: &Fixture) -> Result<Service, &'static str> {
    let workspace = Workspace::open(
        &fixture.files,
        vellum_editor::reference_workspace_profile(
            LocalFilesystemFactory::new(FilesystemLimits {
                max_resources: 16,
                max_bytes: 4_096,
                max_stage_attempts: 8,
                max_metadata_bytes: 4_096,
            })
            .erase(),
        ),
    )
    .map_err(|_| "workspace construction failed")?;
    Service::new_with_action_ceiling(
        workspace,
        ServiceLimits::reference(),
        AuthorityActionSetV1::top(),
    )
    .map_err(|_| "service construction failed")
}

#[test]
#[ignore = "opt-in deterministic qualification; run make check-fuzz-soak"]
fn deterministic_service_soak_preserves_replay_pagination_and_removal_laws() {
    let seed = setting("VELLUM_QUALIFICATION_SEED", DEFAULT_SEED);
    let cycles = usize::try_from(setting("VELLUM_SERVICE_CYCLES", DEFAULT_CYCLES as u64)).unwrap();
    assert!(seed != 0, "qualification seed must be nonzero");
    assert!(
        (1..=10_000).contains(&cycles),
        "service cycle budget is invalid"
    );

    let mut generator = Generator(seed);
    for cycle in 0..cycles {
        let fixture = Fixture::new(cycle).unwrap_or_else(|error| panic!("cycle {cycle}: {error}"));
        let mut service =
            service(&fixture).unwrap_or_else(|error| panic!("cycle {cycle}: {error}"));
        let client = service
            .connect()
            .unwrap_or_else(|_| panic!("cycle {cycle}: connect refused"));

        let first_request = request(
            client,
            1,
            CommandV1::WorkspaceInventoryPage {
                cursor: None,
                page_size: 2,
            },
        );
        let first_reply = service.execute(first_request.clone());
        let ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(first)) = &first_reply else {
            panic!("cycle {cycle}: first page refused")
        };
        let stale_cursor = first
            .next
            .unwrap_or_else(|| panic!("cycle {cycle}: missing continuation"));
        assert_eq!(
            service.execute(first_request.clone()),
            first_reply,
            "cycle {cycle}: exact replay drift"
        );
        assert_eq!(
            service.execute(request(
                client,
                1,
                CommandV1::WorkspaceInventoryPage {
                    cursor: None,
                    page_size: 1,
                },
            )),
            ReplyV1::Refused(RefusalV1::RequestConflict),
            "cycle {cycle}: changed-input replay was not refused"
        );

        assert_eq!(
            service.execute(request(client, 2, CommandV1::CloseView)),
            ReplyV1::Accepted(OutcomeV1::ViewClosed(ViewCloseStatusV1::Closed)),
            "cycle {cycle}: view did not close"
        );

        let root = first_reply_root(&mut service, client, cycle);
        let remove_root = request(client, 4, CommandV1::RemoveRoot { root });
        let root_reply = service.execute(remove_root.clone());
        assert!(matches!(
            root_reply,
            ReplyV1::Accepted(OutcomeV1::RootRemoved(ref receipt))
                if receipt.status == MembershipRemovalStatusV1::Removed
        ));
        assert_eq!(
            service.execute(remove_root),
            root_reply,
            "cycle {cycle}: root removal replay drift"
        );

        let stale_request = request(
            client,
            5,
            CommandV1::WorkspaceInventoryPage {
                cursor: Some(stale_cursor),
                page_size: 1,
            },
        );
        let stale_reply = ReplyV1::Refused(RefusalV1::StaleInventoryCursor);
        assert_eq!(service.execute(stale_request.clone()), stale_reply);
        assert_eq!(
            service.execute(stale_request),
            stale_reply,
            "cycle {cycle}: stale cursor replay drift"
        );

        let document = first_reply_document(&mut service, client, cycle);
        let remove_document = request(client, 7, CommandV1::RemoveDocument { document });
        let document_reply = service.execute(remove_document.clone());
        assert!(matches!(
            document_reply,
            ReplyV1::Accepted(OutcomeV1::DocumentRemoved(ref receipt))
                if receipt.status == MembershipRemovalStatusV1::Removed
        ));
        assert_eq!(
            service.execute(remove_document),
            document_reply,
            "cycle {cycle}: document removal replay drift"
        );
        assert!(matches!(
            service.execute(request(client, 8, CommandV1::RemoveDocument { document })),
            ReplyV1::Accepted(OutcomeV1::DocumentRemoved(receipt))
                if receipt.status == MembershipRemovalStatusV1::AlreadyRemoved
        ));

        let page_size = u16::try_from((generator.next() % 2) + 1).unwrap();
        qualify_terminal_inventory(&mut service, client, page_size, cycle);
        assert!(
            service.usage().request_matching_replay >= 4,
            "cycle {cycle}: replay evidence undercounted"
        );
        service
            .disconnect(client)
            .unwrap_or_else(|_| panic!("cycle {cycle}: disconnect refused"));
        drop(service);
        drop(fixture);
    }
}

fn first_reply_root(
    service: &mut Service,
    client: ClientId,
    cycle: usize,
) -> vellum_protocol::WorkspaceRootIdV1 {
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) =
        service.execute(request(client, 3, CommandV1::WorkspaceInventory))
    else {
        panic!("cycle {cycle}: inventory refused")
    };
    inventory.roots[0].id
}

fn first_reply_document(
    service: &mut Service,
    client: ClientId,
    cycle: usize,
) -> vellum_protocol::WorkspaceDocumentIdV1 {
    let ReplyV1::Accepted(OutcomeV1::WorkspaceInventory(inventory)) =
        service.execute(request(client, 6, CommandV1::WorkspaceInventory))
    else {
        panic!("cycle {cycle}: inventory refused")
    };
    inventory.documents[0]
}

fn qualify_terminal_inventory(
    service: &mut Service,
    client: ClientId,
    page_size: u16,
    cycle: usize,
) {
    let mut cursor: Option<WorkspaceInventoryCursorV1> = None;
    let mut identities = BTreeSet::new();
    let mut request_id = 9_u64;
    let mut page_count = 0_usize;
    loop {
        let ReplyV1::Accepted(OutcomeV1::WorkspaceInventoryPage(page)) = service.execute(request(
            client,
            request_id,
            CommandV1::WorkspaceInventoryPage { cursor, page_size },
        )) else {
            panic!("cycle {cycle}: fresh pagination refused")
        };
        assert!(
            page.documents.len() + page.roots.len() <= usize::from(page_size),
            "cycle {cycle}: page exceeded requested bound"
        );
        assert!(
            !page.documents.is_empty() || !page.roots.is_empty() || page.next.is_none(),
            "cycle {cycle}: empty continuation page"
        );
        for document in page.documents {
            assert!(
                identities.insert((0_u8, document.get())),
                "cycle {cycle}: duplicate document projection"
            );
        }
        for root in page.roots {
            assert!(
                identities.insert((1_u8, root.id.get())),
                "cycle {cycle}: duplicate root projection"
            );
        }
        page_count += 1;
        assert!(
            page_count <= 8,
            "cycle {cycle}: pagination did not terminate"
        );
        let Some(next) = page.next else { break };
        cursor = Some(next);
        request_id += 1;
    }
    assert_eq!(
        identities.len(),
        6,
        "cycle {cycle}: terminal traversal did not cover exact live membership"
    );
}
