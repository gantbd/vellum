use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

use vellum_adapter_local_fs::{FilesystemLimits, LocalFilesystemFactory};
use vellum_editor::{Workspace, reference_workspace_profile};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) struct Fixture(PathBuf);

pub(super) struct JournalFixture(PathBuf);

impl Fixture {
    pub(super) fn new(source: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "vellum-semantic-apply-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, source).unwrap();
        Self(path)
    }

    pub(super) fn source_on_disk(&self) -> String {
        fs::read_to_string(&self.0).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

impl JournalFixture {
    pub(super) fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vellum-semantic-journal-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    pub(super) fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for JournalFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn workspace(files: &[&Fixture]) -> Workspace {
    Workspace::open(
        &files.iter().map(|file| file.0.clone()).collect::<Vec<_>>(),
        reference_workspace_profile(
            LocalFilesystemFactory::new(FilesystemLimits {
                max_resources: 8,
                max_bytes: 4_096,
                max_stage_attempts: 8,
                max_metadata_bytes: 4_096,
            })
            .erase(),
        ),
    )
    .unwrap()
}
