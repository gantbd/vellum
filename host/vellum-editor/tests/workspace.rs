#![cfg(target_os = "macos")]

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use vellum_adapter_local_fs::{FilesystemLimits, LocalDirectoryDiscovery, LocalFilesystemFactory};
use vellum_editor::{
    DirectoryDiscoveryLimits, DirectoryDiscoveryPort, MembershipRemovalStatus, Workspace,
    WorkspaceDocumentEdits, WorkspaceError, WorkspaceRootKind, reference_workspace_profile,
};
use vellum_engine::source::Replacement;
use vellum_host::workspace::{
    DirectoryDiscoveryOutcome, DirectoryDiscoveryUsage, DiscoveredLocatorStatus,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "vellum-workspace-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, source).unwrap();
        Self(path)
    }
}

#[test]
fn document_batch_refuses_duplicate_and_retired_participants_without_publication() {
    let first = Fixture::new("alpha");
    let second = Fixture::new("beta");
    let mut workspace = Workspace::open(&[first.0.clone(), second.0.clone()], profile()).unwrap();
    let ids = workspace.inventory().unwrap().documents;
    let before_first = workspace.document_snapshot(&ids[0]).unwrap();
    let duplicate = || WorkspaceDocumentEdits {
        document: ids[0].clone(),
        expected: before_first.version.clone(),
        edits: vec![Replacement {
            start: 0,
            end: 5,
            expected: "alpha".into(),
            inserted: "changed".into(),
        }],
    };
    assert!(
        workspace
            .edit_document_batch(&[duplicate(), duplicate()])
            .is_err()
    );
    assert_eq!(workspace.document_snapshot(&ids[0]).unwrap().text, "alpha");
    assert_eq!(workspace.linked_history_group_count(), 0);

    let before_second = workspace.document_snapshot(&ids[1]).unwrap();
    workspace.remove_document(&ids[1]).unwrap();
    assert!(
        workspace
            .edit_document_batch(&[
                duplicate(),
                WorkspaceDocumentEdits {
                    document: ids[1].clone(),
                    expected: before_second.version,
                    edits: vec![Replacement {
                        start: 0,
                        end: 4,
                        expected: "beta".into(),
                        inserted: "changed".into(),
                    }],
                },
            ])
            .is_err()
    );
    assert_eq!(workspace.document_snapshot(&ids[0]).unwrap().text, "alpha");
    assert_eq!(fs::read_to_string(&second.0).unwrap(), "beta");
    assert_eq!(workspace.linked_history_group_count(), 0);
}

#[test]
fn document_batch_publishes_one_content_free_linked_history_witness() {
    let first = Fixture::new("alpha");
    let second = Fixture::new("beta");
    let mut workspace = Workspace::open(&[first.0.clone(), second.0.clone()], profile()).unwrap();
    let ids = workspace.inventory().unwrap().documents;
    let first_before = workspace.document_snapshot(&ids[0]).unwrap();
    let second_before = workspace.document_snapshot(&ids[1]).unwrap();
    assert_eq!(workspace.linked_history_group_count(), 0);

    let receipt = workspace
        .edit_document_batch(&[
            WorkspaceDocumentEdits {
                document: ids[0].clone(),
                expected: first_before.version,
                edits: vec![Replacement {
                    start: 0,
                    end: 5,
                    expected: "alpha".into(),
                    inserted: "one".into(),
                }],
            },
            WorkspaceDocumentEdits {
                document: ids[1].clone(),
                expected: second_before.version,
                edits: vec![Replacement {
                    start: 0,
                    end: 4,
                    expected: "beta".into(),
                    inserted: "two".into(),
                }],
            },
        ])
        .unwrap();

    assert_eq!(receipt.changed_documents, 2);
    assert!(receipt.history_group.is_some());
    assert_eq!(workspace.linked_history_group_count(), 1);
    assert_eq!(workspace.document_snapshot(&ids[0]).unwrap().text, "one");
    assert_eq!(workspace.document_snapshot(&ids[1]).unwrap().text, "two");
    assert_eq!(fs::read_to_string(&first.0).unwrap(), "alpha");
    assert_eq!(fs::read_to_string(&second.0).unwrap(), "beta");
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

struct DirectoryFixture(PathBuf);

impl DirectoryFixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vellum-workspace-directory-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for DirectoryFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn profile() -> vellum_editor::WorkspaceConfig {
    reference_workspace_profile(
        LocalFilesystemFactory::new(FilesystemLimits {
            max_resources: 8,
            max_bytes: 4096,
            max_stage_attempts: 8,
            max_metadata_bytes: 4096,
        })
        .erase(),
    )
}

#[test]
fn views_select_independently_while_edits_publish_through_one_workspace() {
    let first_file = Fixture::new("alpha");
    let second_file = Fixture::new("beta");
    let mut workspace =
        Workspace::open(&[first_file.0.clone(), second_file.0.clone()], profile()).unwrap();
    let first_view = workspace.open_view().unwrap();
    let mut second_view = workspace.open_view().unwrap();
    workspace.select_document(&mut second_view, 1).unwrap();

    assert_eq!(first_view.active_document(), 0);
    assert_eq!(second_view.active_document(), 1);
    assert_eq!(workspace.snapshot(&first_view).unwrap().text, "alpha");
    let before = workspace.snapshot(&second_view).unwrap();
    assert_eq!(before.text, "beta");

    workspace
        .edit_snapshot(
            &second_view,
            &before.version,
            vec![Replacement {
                start: 0,
                end: 4,
                expected: "beta".into(),
                inserted: "agent".into(),
            }],
        )
        .unwrap();

    assert_eq!(workspace.snapshot(&first_view).unwrap().text, "alpha");
    assert_eq!(workspace.snapshot(&second_view).unwrap().text, "agent");
    assert_eq!(first_view.active_document(), 0);
}

#[test]
fn empty_workspace_has_stable_identity_and_no_view_until_a_file_is_added() {
    let mut workspace = Workspace::empty(profile()).unwrap();
    let before = workspace.inventory().unwrap();
    assert!(before.documents.is_empty());
    assert!(before.roots.is_empty());
    assert!(matches!(workspace.open_view(), Err(WorkspaceError::Empty)));

    let file = Fixture::new("first");
    let added = workspace.add_file(&file.0).unwrap();
    assert!(added.added);
    assert!(added.root.is_some());
    let after = workspace.inventory().unwrap();
    assert_eq!(after.workspace, before.workspace);
    assert_eq!(after.documents, vec![added.document.clone()]);
    assert_eq!(after.roots.len(), 1);
    assert_eq!(after.roots[0].kind, WorkspaceRootKind::File);
    assert_eq!(
        workspace.open_view().unwrap().active_document_id(),
        &added.document
    );

    let joined = workspace.add_file(&file.0).unwrap();
    assert!(!joined.added);
    assert_eq!(joined.document, added.document);
    assert_eq!(workspace.inventory().unwrap(), after);
}

struct Files(Vec<PathBuf>);

impl DirectoryDiscoveryPort for Files {
    fn discover(
        &mut self,
        _root: &std::path::Path,
        _limits: DirectoryDiscoveryLimits,
    ) -> DirectoryDiscoveryOutcome {
        DirectoryDiscoveryOutcome::Complete {
            files: self.0.clone(),
            usage: DirectoryDiscoveryUsage::default(),
            locator_status: DiscoveredLocatorStatus::RevalidationRequired,
        }
    }
}

#[test]
fn directory_discovery_is_bounded_and_publishes_a_content_free_root_inventory() {
    let first = Fixture::new("first");
    let second = Fixture::new("second");
    let mut workspace = Workspace::empty(profile()).unwrap();
    let mut discovery = Files(vec![first.0.clone(), second.0.clone(), first.0.clone()]);
    let report = workspace
        .add_directory(
            std::path::Path::new("opaque-root"),
            DirectoryDiscoveryLimits {
                max_entries: 4,
                max_files: 3,
                max_depth: 4,
                max_path_bytes: 4096,
                max_total_path_bytes: 12288,
            },
            &mut discovery,
        )
        .unwrap();
    assert_eq!(report.added, 2);
    assert_eq!(report.joined, 1);
    assert_eq!(report.documents.len(), 2);
    let inventory = workspace.inventory().unwrap();
    assert_eq!(inventory.documents.len(), 2);
    assert_eq!(inventory.roots.len(), 1);
    assert_eq!(inventory.roots[0].kind, WorkspaceRootKind::Directory);
    assert_eq!(inventory.roots[0].id, report.root);
    assert_eq!(inventory.documents, report.documents);
    assert!(!format!("{inventory:?}").contains("opaque-root"));

    let generation = workspace.inventory_generation();
    let repeated = workspace
        .add_directory(
            std::path::Path::new("opaque-root"),
            DirectoryDiscoveryLimits {
                max_entries: 4,
                max_files: 3,
                max_depth: 4,
                max_path_bytes: 4096,
                max_total_path_bytes: 12288,
            },
            &mut discovery,
        )
        .unwrap();
    assert_eq!(repeated.root, report.root);
    assert_eq!(repeated.added, 0);
    assert_eq!(repeated.joined, 3);
    assert!(repeated.documents.is_empty());
    assert_eq!(workspace.inventory_generation(), generation);
    assert_eq!(workspace.inventory().unwrap().roots.len(), 1);
}

#[test]
fn inventory_pages_are_bounded_ordered_and_generation_bound() {
    let first = Fixture::new("one");
    let second = Fixture::new("two");
    let third = Fixture::new("three");
    let mut workspace = Workspace::open(&[first.0.clone(), second.0.clone()], profile()).unwrap();

    let first_page = workspace.inventory_page(0, 2).unwrap();
    assert_eq!(first_page.documents.len(), 2);
    assert!(first_page.roots.is_empty());
    assert_eq!(first_page.total_entries, 4);
    let second_page = workspace.inventory_page(2, 2).unwrap();
    assert!(second_page.documents.is_empty());
    assert_eq!(second_page.roots.len(), 2);
    assert_eq!(second_page.generation, first_page.generation);

    workspace.add_file(&third.0).unwrap();
    let changed = workspace.inventory_page(0, 2).unwrap();
    assert_ne!(changed.generation, first_page.generation);
    assert_eq!(changed.total_entries, 6);
    assert!(workspace.inventory_page(7, 1).is_err());
    assert!(workspace.inventory_page(0, 0).is_err());
}

#[test]
fn local_directory_adapter_composes_without_editor_selecting_it() {
    let directory = DirectoryFixture::new();
    fs::write(directory.0.join("b"), "b").unwrap();
    fs::write(directory.0.join("a"), "a").unwrap();
    let mut workspace = Workspace::empty(profile()).unwrap();
    let report = workspace
        .add_directory(
            &directory.0,
            DirectoryDiscoveryLimits {
                max_entries: 3,
                max_files: 2,
                max_depth: 0,
                max_path_bytes: 4096,
                max_total_path_bytes: 8192,
            },
            &mut LocalDirectoryDiscovery,
        )
        .unwrap();
    assert_eq!(report.added, 2);
    assert_eq!(report.joined, 0);
    assert_eq!(workspace.inventory().unwrap().documents.len(), 2);
}

#[test]
fn overproducing_discovery_is_refused_before_workspace_inventory_changes() {
    let first = Fixture::new("first");
    let second = Fixture::new("second");
    let mut workspace = Workspace::empty(profile()).unwrap();
    let before = workspace.inventory().unwrap();
    let mut discovery = Files(vec![first.0.clone(), second.0.clone()]);
    assert!(matches!(
        workspace.add_directory(
            std::path::Path::new("root"),
            DirectoryDiscoveryLimits {
                max_entries: 2,
                max_files: 1,
                max_depth: 4,
                max_path_bytes: 4096,
                max_total_path_bytes: 4096,
            },
            &mut discovery,
        ),
        Err(WorkspaceError::Discovery(
            vellum_editor::DirectoryDiscoveryRefusal::Limit
        ))
    ));
    assert_eq!(workspace.inventory().unwrap(), before);
}

#[test]
fn later_file_open_failure_does_not_publish_a_partial_directory_inventory() {
    let first = Fixture::new("first");
    let missing = std::env::temp_dir().join(format!(
        "vellum-workspace-missing-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut workspace = Workspace::empty(profile()).unwrap();
    let before = workspace.inventory().unwrap();
    let mut discovery = Files(vec![first.0.clone(), missing]);
    assert!(
        workspace
            .add_directory(
                std::path::Path::new("root"),
                DirectoryDiscoveryLimits {
                    max_entries: 2,
                    max_files: 2,
                    max_depth: 4,
                    max_path_bytes: 4096,
                    max_total_path_bytes: 8192,
                },
                &mut discovery,
            )
            .is_err()
    );
    assert_eq!(workspace.inventory().unwrap(), before);
}

#[test]
fn document_id_from_another_workspace_cannot_select_a_local_view() {
    let first = Fixture::new("first");
    let second = Fixture::new("second");
    let mut one = Workspace::empty(profile()).unwrap();
    let mut two = Workspace::empty(profile()).unwrap();
    one.add_file(&first.0).unwrap();
    let foreign = two.add_file(&second.0).unwrap().document;
    let mut view = one.open_view().unwrap();
    assert!(matches!(
        one.select_document_id(&mut view, &foreign),
        Err(WorkspaceError::InvalidView)
    ));
    assert_eq!(view.active_document(), 0);
}

#[test]
fn membership_retirement_is_idempotent_bounded_and_does_not_delete_files_or_cascade() {
    let file = Fixture::new("retained");
    let mut workspace = Workspace::empty(profile()).unwrap();
    let admitted = workspace.add_file(&file.0).unwrap();
    let root = admitted.root.unwrap();
    let generation = workspace.inventory_generation();

    assert_eq!(
        workspace.remove_root(&root).unwrap(),
        MembershipRemovalStatus::Removed
    );
    assert!(workspace.inventory().unwrap().roots.is_empty());
    assert_eq!(workspace.inventory().unwrap().documents.len(), 1);
    assert_eq!(fs::read_to_string(&file.0).unwrap(), "retained");
    assert_eq!(
        workspace.remove_root(&root).unwrap(),
        MembershipRemovalStatus::AlreadyRemoved
    );

    assert_eq!(
        workspace.remove_document(&admitted.document).unwrap(),
        MembershipRemovalStatus::Removed
    );
    assert!(workspace.inventory().unwrap().documents.is_empty());
    assert!(matches!(workspace.open_view(), Err(WorkspaceError::Empty)));
    assert_eq!(fs::read_to_string(&file.0).unwrap(), "retained");
    assert_eq!(
        workspace.remove_document(&admitted.document).unwrap(),
        MembershipRemovalStatus::AlreadyRemoved
    );
    assert_eq!(workspace.inventory_generation(), generation + 2);

    let readmitted = workspace.add_file(&file.0).unwrap();
    assert!(readmitted.added);
    assert_ne!(readmitted.document, admitted.document);
}
