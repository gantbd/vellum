#![cfg(target_os = "macos")]

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
use vellum_editor::{Workspace, reference_workspace_profile};
use vellum_engine::source::Replacement;

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

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
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
