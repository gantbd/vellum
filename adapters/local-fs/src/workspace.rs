//! Bounded discovery of regular files for application workspace composition.
//!
//! Discovery is deliberately separate from resource opening. It grants no
//! read, edit, or save authority and follows no symbolic links.

use std::{fs, os::unix::ffi::OsStrExt, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkspaceDiscoveryLimits {
    pub max_entries: usize,
    pub max_files: usize,
    pub max_depth: usize,
    pub max_path_bytes: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceDiscoveryUsage {
    pub examined_entries: usize,
    pub directories: usize,
    pub regular_files: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceDiscoveryError {
    InvalidLimits,
    Io,
    Limit,
    UnsupportedResource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkspaceDiscoveryFailure {
    pub error: WorkspaceDiscoveryError,
    pub usage: WorkspaceDiscoveryUsage,
}

impl std::fmt::Display for WorkspaceDiscoveryFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "workspace discovery: {:?}", self.error)
    }
}

impl std::error::Error for WorkspaceDiscoveryFailure {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceDiscovery {
    pub files: Vec<PathBuf>,
    pub usage: WorkspaceDiscoveryUsage,
}

/// Expands regular-file and directory inputs into one sorted, duplicate-free
/// file set. Directories are traversed recursively within explicit limits.
/// Symbolic links and special files refuse the complete discovery operation.
pub fn discover_workspace_files(
    inputs: &[PathBuf],
    limits: WorkspaceDiscoveryLimits,
) -> Result<WorkspaceDiscovery, WorkspaceDiscoveryFailure> {
    if limits.max_entries == 0
        || limits.max_files == 0
        || limits.max_path_bytes == 0
        || inputs.len() > limits.max_entries
    {
        return Err(failure(
            WorkspaceDiscoveryError::InvalidLimits,
            WorkspaceDiscoveryUsage::default(),
        ));
    }

    let mut usage = WorkspaceDiscoveryUsage::default();
    let mut pending: Vec<(PathBuf, usize)> = inputs.iter().cloned().map(|path| (path, 0)).collect();
    let mut examined = Vec::new();
    let mut files = Vec::new();
    files
        .try_reserve_exact(inputs.len().min(limits.max_files))
        .map_err(|_| failure(WorkspaceDiscoveryError::Limit, usage))?;

    while let Some((path, depth)) = pending.pop() {
        if path.as_os_str().as_bytes().len() > limits.max_path_bytes {
            return Err(failure(WorkspaceDiscoveryError::Limit, usage));
        }
        if examined.contains(&path) {
            continue;
        }
        examined
            .try_reserve(1)
            .map_err(|_| failure(WorkspaceDiscoveryError::Limit, usage))?;
        examined.push(path.clone());
        usage.examined_entries = usage
            .examined_entries
            .checked_add(1)
            .ok_or_else(|| failure(WorkspaceDiscoveryError::Limit, usage))?;
        if usage.examined_entries > limits.max_entries {
            return Err(failure(WorkspaceDiscoveryError::Limit, usage));
        }
        let metadata =
            fs::symlink_metadata(&path).map_err(|_| failure(WorkspaceDiscoveryError::Io, usage))?;
        let kind = metadata.file_type();
        if kind.is_symlink() {
            return Err(failure(WorkspaceDiscoveryError::UnsupportedResource, usage));
        }
        if kind.is_file() {
            usage.regular_files = usage
                .regular_files
                .checked_add(1)
                .ok_or_else(|| failure(WorkspaceDiscoveryError::Limit, usage))?;
            if usage.regular_files > limits.max_files {
                return Err(failure(WorkspaceDiscoveryError::Limit, usage));
            }
            files
                .try_reserve(1)
                .map_err(|_| failure(WorkspaceDiscoveryError::Limit, usage))?;
            files.push(path);
            continue;
        }
        if !kind.is_dir() {
            return Err(failure(WorkspaceDiscoveryError::UnsupportedResource, usage));
        }
        usage.directories = usage
            .directories
            .checked_add(1)
            .ok_or_else(|| failure(WorkspaceDiscoveryError::Limit, usage))?;
        if depth > limits.max_depth {
            return Err(failure(WorkspaceDiscoveryError::Limit, usage));
        }
        let mut children = Vec::new();
        for entry in fs::read_dir(&path).map_err(|_| failure(WorkspaceDiscoveryError::Io, usage))? {
            let entry = entry.map_err(|_| failure(WorkspaceDiscoveryError::Io, usage))?;
            children
                .try_reserve(1)
                .map_err(|_| failure(WorkspaceDiscoveryError::Limit, usage))?;
            children.push(entry.path());
            if usage
                .examined_entries
                .saturating_add(pending.len())
                .saturating_add(children.len())
                > limits.max_entries
            {
                return Err(failure(WorkspaceDiscoveryError::Limit, usage));
            }
        }
        children.sort();
        for child in children.into_iter().rev() {
            pending
                .try_reserve(1)
                .map_err(|_| failure(WorkspaceDiscoveryError::Limit, usage))?;
            pending.push((child, depth.saturating_add(1)));
        }
    }

    files.sort();
    files.dedup();
    Ok(WorkspaceDiscovery { files, usage })
}

fn failure(
    error: WorkspaceDiscoveryError,
    usage: WorkspaceDiscoveryUsage,
) -> WorkspaceDiscoveryFailure {
    WorkspaceDiscoveryFailure { error, usage }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::fs::symlink,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "vellum-workspace-discovery-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn limits() -> WorkspaceDiscoveryLimits {
        WorkspaceDiscoveryLimits {
            max_entries: 16,
            max_files: 8,
            max_depth: 4,
            max_path_bytes: 4096,
        }
    }

    #[test]
    fn directory_discovery_is_recursive_sorted_and_duplicate_free() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("nested")).unwrap();
        let a = fixture.0.join("a");
        let b = fixture.0.join("nested/b");
        fs::write(&b, "b").unwrap();
        fs::write(&a, "a").unwrap();
        let result = discover_workspace_files(&[fixture.0.clone(), a.clone()], limits()).unwrap();
        assert_eq!(result.files, vec![a, b]);
        assert_eq!(result.usage.regular_files, 2);
        assert_eq!(result.usage.directories, 2);
    }

    #[test]
    fn discovery_refuses_symlinks_and_never_returns_a_partial_file_set() {
        let fixture = Fixture::new();
        let file = fixture.0.join("file");
        fs::write(&file, "a").unwrap();
        symlink(&file, fixture.0.join("link")).unwrap();
        let error =
            discover_workspace_files(std::slice::from_ref(&fixture.0), limits()).unwrap_err();
        assert_eq!(error.error, WorkspaceDiscoveryError::UnsupportedResource);
    }

    #[test]
    fn discovery_refuses_file_and_depth_overflow_instead_of_truncating() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("a"), "a").unwrap();
        fs::write(fixture.0.join("b"), "b").unwrap();
        let mut bounded = limits();
        bounded.max_files = 1;
        assert_eq!(
            discover_workspace_files(std::slice::from_ref(&fixture.0), bounded)
                .unwrap_err()
                .error,
            WorkspaceDiscoveryError::Limit
        );

        fs::create_dir(fixture.0.join("nested")).unwrap();
        fs::write(fixture.0.join("nested/c"), "c").unwrap();
        bounded = limits();
        bounded.max_depth = 0;
        assert_eq!(
            discover_workspace_files(std::slice::from_ref(&fixture.0), bounded)
                .unwrap_err()
                .error,
            WorkspaceDiscoveryError::Limit
        );
    }
}
