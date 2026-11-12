//! Bounded discovery of regular files for application workspace composition.
//!
//! Discovery is deliberately separate from resource opening. It grants no
//! read, edit, or save authority. Traversal is descriptor-anchored and refuses
//! symbolic-link entries, but returned path locators are not capabilities and
//! explicitly require revalidation when later opened.

use rustix::fs::{CWD, Dir, FileType, Mode, OFlags, fstat, openat};
use std::{
    ffi::OsStr,
    os::{fd::OwnedFd, unix::ffi::OsStrExt},
    path::{Path, PathBuf},
};
pub use vellum_host::workspace::{
    DirectoryDiscoveryLimits, DirectoryDiscoveryOutcome, DirectoryDiscoveryPort,
    DirectoryDiscoveryRefusal, DirectoryDiscoveryUsage, DiscoveredLocatorStatus,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct LocalDirectoryDiscovery;

impl DirectoryDiscoveryPort for LocalDirectoryDiscovery {
    fn discover(
        &mut self,
        root: &Path,
        limits: DirectoryDiscoveryLimits,
    ) -> DirectoryDiscoveryOutcome {
        discover_workspace_files(root, limits)
    }
}

/// Product bound for planning a finite set of application-selected roots.
/// The discovery limits apply to the complete plan, not independently to each
/// root, so a caller cannot multiply work or retained locator bytes by adding
/// more roots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkspaceRootPlanLimitsV1 {
    pub max_roots: usize,
    pub discovery: DirectoryDiscoveryLimits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceRootPlanRefusalV1 {
    InvalidLimits,
    RootLimit,
    Discovery(DirectoryDiscoveryRefusal),
    Capacity,
}

/// Closed multi-root plan. Refusal exposes only content-free cumulative usage;
/// no successfully discovered prefix escapes. Complete locators still require
/// independent revalidation when an editor or workspace opens them.
#[derive(PartialEq, Eq)]
pub enum WorkspaceRootPlanOutcomeV1 {
    Complete {
        files: Vec<PathBuf>,
        roots_examined: usize,
        usage: DirectoryDiscoveryUsage,
        locator_status: DiscoveredLocatorStatus,
    },
    Refused {
        reason: WorkspaceRootPlanRefusalV1,
        roots_examined: usize,
        usage: DirectoryDiscoveryUsage,
    },
}

impl std::fmt::Debug for WorkspaceRootPlanOutcomeV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Complete {
                files,
                roots_examined,
                usage,
                locator_status,
            } => formatter
                .debug_struct("WorkspaceRootPlanOutcomeV1::Complete")
                .field("files", &files.len())
                .field("roots_examined", roots_examined)
                .field("usage", usage)
                .field("locator_status", locator_status)
                .finish(),
            Self::Refused {
                reason,
                roots_examined,
                usage,
            } => formatter
                .debug_struct("WorkspaceRootPlanOutcomeV1::Refused")
                .field("reason", reason)
                .field("roots_examined", roots_examined)
                .field("usage", usage)
                .finish(),
        }
    }
}

/// Plans several regular-file or directory roots as one bounded effect-free
/// admission proposal. The empty iterator is the lawful identity and returns
/// an empty complete plan. Duplicate locators are removed only after all
/// physical discovery work has remained within the aggregate bounds.
pub fn plan_workspace_roots_v1<P>(
    roots: impl IntoIterator<Item = P>,
    limits: WorkspaceRootPlanLimitsV1,
) -> WorkspaceRootPlanOutcomeV1
where
    P: AsRef<Path>,
{
    let discovery = limits.discovery;
    if limits.max_roots == 0
        || discovery.max_entries == 0
        || discovery.max_files == 0
        || discovery.max_path_bytes == 0
        || discovery.max_total_path_bytes == 0
    {
        return root_plan_refused(
            WorkspaceRootPlanRefusalV1::InvalidLimits,
            0,
            DirectoryDiscoveryUsage::default(),
        );
    }
    let mut files = Vec::new();
    if files
        .try_reserve_exact(discovery.max_files.min(16))
        .is_err()
    {
        return root_plan_refused(
            WorkspaceRootPlanRefusalV1::Capacity,
            0,
            DirectoryDiscoveryUsage::default(),
        );
    }
    let mut usage = DirectoryDiscoveryUsage::default();
    let mut roots_examined = 0usize;
    for root in roots {
        if roots_examined == limits.max_roots {
            return root_plan_refused(WorkspaceRootPlanRefusalV1::RootLimit, roots_examined, usage);
        }
        let Some(remaining_entries) = discovery
            .max_entries
            .checked_sub(usage.examined_entries)
            .filter(|remaining| *remaining > 0)
        else {
            return root_plan_refused(
                WorkspaceRootPlanRefusalV1::Discovery(DirectoryDiscoveryRefusal::Limit),
                roots_examined,
                usage,
            );
        };
        let Some(remaining_files) = discovery
            .max_files
            .checked_sub(usage.regular_files)
            .filter(|remaining| *remaining > 0)
        else {
            return root_plan_refused(
                WorkspaceRootPlanRefusalV1::Discovery(DirectoryDiscoveryRefusal::Limit),
                roots_examined,
                usage,
            );
        };
        let Some(remaining_path_bytes) = discovery
            .max_total_path_bytes
            .checked_sub(usage.retained_path_bytes)
            .filter(|remaining| *remaining > 0)
        else {
            return root_plan_refused(
                WorkspaceRootPlanRefusalV1::Discovery(DirectoryDiscoveryRefusal::Limit),
                roots_examined,
                usage,
            );
        };
        let root_limits = DirectoryDiscoveryLimits {
            max_entries: remaining_entries,
            max_files: remaining_files,
            max_depth: discovery.max_depth,
            max_path_bytes: discovery.max_path_bytes,
            max_total_path_bytes: remaining_path_bytes,
        };
        roots_examined += 1;
        let (discovered, root_usage) = match discover_workspace_files(root.as_ref(), root_limits) {
            DirectoryDiscoveryOutcome::Complete { files, usage, .. } => (files, usage),
            DirectoryDiscoveryOutcome::Refused {
                reason,
                usage: root,
            } => {
                let aggregate = aggregate_usage(usage, root).unwrap_or(usage);
                return root_plan_refused(
                    WorkspaceRootPlanRefusalV1::Discovery(reason),
                    roots_examined,
                    aggregate,
                );
            }
        };
        let Some(aggregate) = aggregate_usage(usage, root_usage) else {
            return root_plan_refused(WorkspaceRootPlanRefusalV1::Capacity, roots_examined, usage);
        };
        usage = aggregate;
        for file in discovered {
            if files.contains(&file) {
                continue;
            }
            if files.try_reserve(1).is_err() {
                return root_plan_refused(
                    WorkspaceRootPlanRefusalV1::Capacity,
                    roots_examined,
                    usage,
                );
            }
            files.push(file);
        }
    }
    files.sort();
    WorkspaceRootPlanOutcomeV1::Complete {
        files,
        roots_examined,
        usage,
        locator_status: DiscoveredLocatorStatus::RevalidationRequired,
    }
}

fn aggregate_usage(
    left: DirectoryDiscoveryUsage,
    right: DirectoryDiscoveryUsage,
) -> Option<DirectoryDiscoveryUsage> {
    Some(DirectoryDiscoveryUsage {
        examined_entries: left.examined_entries.checked_add(right.examined_entries)?,
        directories: left.directories.checked_add(right.directories)?,
        regular_files: left.regular_files.checked_add(right.regular_files)?,
        retained_path_bytes: left
            .retained_path_bytes
            .checked_add(right.retained_path_bytes)?,
    })
}

fn root_plan_refused(
    reason: WorkspaceRootPlanRefusalV1,
    roots_examined: usize,
    usage: DirectoryDiscoveryUsage,
) -> WorkspaceRootPlanOutcomeV1 {
    WorkspaceRootPlanOutcomeV1::Refused {
        reason,
        roots_examined,
        usage,
    }
}

/// Expands one regular-file or directory root into a sorted, duplicate-free
/// file set. Every traversed node is opened relative to its already-open parent
/// with `NOFOLLOW`; directory replacement cannot redirect the walk. Symbolic
/// links and special files refuse the complete discovery operation.
pub fn discover_workspace_files(
    root: &Path,
    limits: DirectoryDiscoveryLimits,
) -> DirectoryDiscoveryOutcome {
    if limits.max_entries == 0
        || limits.max_files == 0
        || limits.max_path_bytes == 0
        || limits.max_total_path_bytes == 0
    {
        return refused(
            DirectoryDiscoveryRefusal::InvalidLimits,
            DirectoryDiscoveryUsage::default(),
        );
    }

    let usage = DirectoryDiscoveryUsage::default();
    let root_fd = match openat(
        CWD,
        root,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(_) => return refused(DirectoryDiscoveryRefusal::Io, usage),
    };
    discover_opened_root(root_fd, root, limits, usage)
}

fn discover_opened_root(
    root_fd: OwnedFd,
    root: &Path,
    limits: DirectoryDiscoveryLimits,
    mut usage: DirectoryDiscoveryUsage,
) -> DirectoryDiscoveryOutcome {
    struct Pending {
        fd: OwnedFd,
        path: PathBuf,
        depth: usize,
    }
    let mut pending = vec![Pending {
        fd: root_fd,
        path: root.to_path_buf(),
        depth: 0,
    }];
    let mut files = Vec::new();
    if files.try_reserve_exact(limits.max_files.min(16)).is_err() {
        return refused(DirectoryDiscoveryRefusal::Limit, usage);
    }

    while let Some(Pending { fd, path, depth }) = pending.pop() {
        if path.as_os_str().as_bytes().len() > limits.max_path_bytes {
            return refused(DirectoryDiscoveryRefusal::Limit, usage);
        }
        let Some(examined_entries) = usage.examined_entries.checked_add(1) else {
            return refused(DirectoryDiscoveryRefusal::Limit, usage);
        };
        usage.examined_entries = examined_entries;
        if usage.examined_entries > limits.max_entries {
            return refused(DirectoryDiscoveryRefusal::Limit, usage);
        }
        let stat = match fstat(&fd) {
            Ok(stat) => stat,
            Err(_) => return refused(DirectoryDiscoveryRefusal::Io, usage),
        };
        match FileType::from_raw_mode(stat.st_mode) {
            FileType::RegularFile => {
                let Some(regular_files) = usage.regular_files.checked_add(1) else {
                    return refused(DirectoryDiscoveryRefusal::Limit, usage);
                };
                usage.regular_files = regular_files;
                if usage.regular_files > limits.max_files {
                    return refused(DirectoryDiscoveryRefusal::Limit, usage);
                }
                let path_bytes = path.as_os_str().as_bytes().len();
                let Some(retained_path_bytes) = usage.retained_path_bytes.checked_add(path_bytes)
                else {
                    return refused(DirectoryDiscoveryRefusal::Limit, usage);
                };
                if retained_path_bytes > limits.max_total_path_bytes {
                    return refused(DirectoryDiscoveryRefusal::Limit, usage);
                }
                usage.retained_path_bytes = retained_path_bytes;
                if files.try_reserve(1).is_err() {
                    return refused(DirectoryDiscoveryRefusal::Limit, usage);
                }
                files.push(path);
                continue;
            }
            FileType::Directory => {}
            _ => return refused(DirectoryDiscoveryRefusal::UnsupportedResource, usage),
        }
        let Some(directories) = usage.directories.checked_add(1) else {
            return refused(DirectoryDiscoveryRefusal::Limit, usage);
        };
        usage.directories = directories;
        if depth > limits.max_depth {
            return refused(DirectoryDiscoveryRefusal::Limit, usage);
        }
        let mut directory = match Dir::read_from(&fd) {
            Ok(directory) => directory,
            Err(_) => return refused(DirectoryDiscoveryRefusal::Io, usage),
        };
        let mut children: Vec<(Vec<u8>, PathBuf)> = Vec::new();
        for entry in &mut directory {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => return refused(DirectoryDiscoveryRefusal::Io, usage),
            };
            let name = entry.file_name().to_bytes();
            if name == b"." || name == b".." {
                continue;
            }
            if children.try_reserve(1).is_err() {
                return refused(DirectoryDiscoveryRefusal::Limit, usage);
            }
            let Some(queued) = usage
                .examined_entries
                .checked_add(pending.len())
                .and_then(|count| count.checked_add(children.len()))
                .and_then(|count| count.checked_add(1))
            else {
                return refused(DirectoryDiscoveryRefusal::Limit, usage);
            };
            if queued > limits.max_entries {
                return refused(DirectoryDiscoveryRefusal::Limit, usage);
            }
            let child_path = path.join(OsStr::from_bytes(name));
            children.push((name.to_vec(), child_path));
        }
        children.sort_by(|left, right| left.0.cmp(&right.0));
        for (name, child_path) in children.into_iter().rev() {
            if pending.try_reserve(1).is_err() {
                return refused(DirectoryDiscoveryRefusal::Limit, usage);
            }
            let child_fd = match openat(
                &fd,
                OsStr::from_bytes(&name),
                OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
                Mode::empty(),
            ) {
                Ok(child_fd) => child_fd,
                Err(_) => return refused(DirectoryDiscoveryRefusal::UnsupportedResource, usage),
            };
            let Some(child_depth) = depth.checked_add(1) else {
                return refused(DirectoryDiscoveryRefusal::Limit, usage);
            };
            pending.push(Pending {
                fd: child_fd,
                path: child_path,
                depth: child_depth,
            });
        }
    }

    files.sort();
    files.dedup();
    DirectoryDiscoveryOutcome::Complete {
        files,
        usage,
        locator_status: DiscoveredLocatorStatus::RevalidationRequired,
    }
}

fn refused(
    reason: DirectoryDiscoveryRefusal,
    usage: DirectoryDiscoveryUsage,
) -> DirectoryDiscoveryOutcome {
    DirectoryDiscoveryOutcome::Refused { reason, usage }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
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

    fn limits() -> DirectoryDiscoveryLimits {
        DirectoryDiscoveryLimits {
            max_entries: 16,
            max_files: 8,
            max_depth: 4,
            max_path_bytes: 4096,
            max_total_path_bytes: 16384,
        }
    }

    fn plan_limits() -> WorkspaceRootPlanLimitsV1 {
        WorkspaceRootPlanLimitsV1 {
            max_roots: 4,
            discovery: limits(),
        }
    }

    #[test]
    fn multi_root_plan_has_a_lawful_empty_identity_and_deterministic_union() {
        let WorkspaceRootPlanOutcomeV1::Complete {
            files,
            roots_examined,
            usage,
            ..
        } = plan_workspace_roots_v1(std::iter::empty::<&Path>(), plan_limits())
        else {
            panic!("empty plan refused")
        };
        assert!(files.is_empty());
        assert_eq!(roots_examined, 0);
        assert_eq!(usage, DirectoryDiscoveryUsage::default());

        let fixture = Fixture::new();
        let nested = fixture.0.join("nested");
        fs::create_dir(&nested).unwrap();
        let a = fixture.0.join("a");
        let b = nested.join("b");
        fs::write(&b, "b").unwrap();
        fs::write(&a, "a").unwrap();
        let WorkspaceRootPlanOutcomeV1::Complete {
            files,
            roots_examined,
            usage,
            ..
        } = plan_workspace_roots_v1([fixture.0.as_path(), a.as_path()], plan_limits())
        else {
            panic!("multi-root plan refused")
        };
        assert_eq!(files, vec![a, b]);
        assert_eq!(roots_examined, 2);
        // The repeated file root remains physical discovery work even though
        // its locator appears only once in the extensional result.
        assert_eq!(usage.regular_files, 3);
    }

    #[test]
    fn multi_root_plan_enforces_global_root_and_discovery_bounds_without_prefix_paths() {
        let fixture = Fixture::new();
        let a = fixture.0.join("a");
        let b = fixture.0.join("b");
        fs::write(&a, "a").unwrap();
        fs::write(&b, "b").unwrap();

        let mut bounded = plan_limits();
        bounded.max_roots = 1;
        let refusal = plan_workspace_roots_v1([a.as_path(), b.as_path()], bounded);
        assert!(matches!(
            refusal,
            WorkspaceRootPlanOutcomeV1::Refused {
                reason: WorkspaceRootPlanRefusalV1::RootLimit,
                roots_examined: 1,
                ..
            }
        ));
        let evidence = format!("{refusal:?}");
        assert!(!evidence.contains(a.to_string_lossy().as_ref()));
        assert!(!evidence.contains(b.to_string_lossy().as_ref()));

        bounded = plan_limits();
        bounded.discovery.max_files = 1;
        assert!(matches!(
            plan_workspace_roots_v1([a.as_path(), b.as_path()], bounded),
            WorkspaceRootPlanOutcomeV1::Refused {
                reason: WorkspaceRootPlanRefusalV1::Discovery(DirectoryDiscoveryRefusal::Limit),
                roots_examined: 1,
                ..
            }
        ));
    }

    #[test]
    fn multi_root_plan_refuses_invalid_bounds_and_symlinks_without_partial_files() {
        let fixture = Fixture::new();
        let file = fixture.0.join("file");
        fs::write(&file, "a").unwrap();
        let mut invalid = plan_limits();
        invalid.max_roots = 0;
        assert!(matches!(
            plan_workspace_roots_v1([file.as_path()], invalid),
            WorkspaceRootPlanOutcomeV1::Refused {
                reason: WorkspaceRootPlanRefusalV1::InvalidLimits,
                roots_examined: 0,
                ..
            }
        ));

        let outside = Fixture::new();
        symlink(&outside.0, fixture.0.join("escape")).unwrap();
        assert!(matches!(
            plan_workspace_roots_v1([fixture.0.as_path()], plan_limits()),
            WorkspaceRootPlanOutcomeV1::Refused {
                reason: WorkspaceRootPlanRefusalV1::Discovery(
                    DirectoryDiscoveryRefusal::UnsupportedResource
                ),
                roots_examined: 1,
                ..
            }
        ));
    }

    #[test]
    fn directory_discovery_is_recursive_sorted_and_duplicate_free() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("nested")).unwrap();
        let a = fixture.0.join("a");
        let b = fixture.0.join("nested/b");
        fs::write(&b, "b").unwrap();
        fs::write(&a, "a").unwrap();
        let DirectoryDiscoveryOutcome::Complete { files, usage, .. } =
            discover_workspace_files(&fixture.0, limits())
        else {
            panic!("discovery refused")
        };
        assert_eq!(files, vec![a, b]);
        assert_eq!(usage.regular_files, 2);
        assert_eq!(usage.directories, 2);
        let DirectoryDiscoveryOutcome::Complete { files: again, .. } =
            discover_workspace_files(&fixture.0, limits())
        else {
            panic!("repeat discovery refused")
        };
        assert_eq!(again, files);
    }

    #[test]
    fn discovery_refuses_symlink_escape_without_a_partial_file_set_or_path_evidence() {
        let fixture = Fixture::new();
        let file = fixture.0.join("file");
        fs::write(&file, "a").unwrap();
        let outside = Fixture::new();
        let secret = outside.0.join("not-disclosed");
        fs::write(&secret, "secret").unwrap();
        symlink(&outside.0, fixture.0.join("escape")).unwrap();
        let outcome = discover_workspace_files(&fixture.0, limits());
        let DirectoryDiscoveryOutcome::Refused { reason, usage } = outcome else {
            panic!("symlink traversal completed")
        };
        assert_eq!(reason, DirectoryDiscoveryRefusal::UnsupportedResource);
        assert!(usage.examined_entries > 0);
        let evidence = format!("{reason:?} {usage:?}");
        assert!(!evidence.contains("not-disclosed"));
        assert!(!evidence.contains(fixture.0.to_string_lossy().as_ref()));
    }

    #[test]
    fn replacing_root_after_open_cannot_redirect_descriptor_anchored_traversal() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("original"), "original").unwrap();
        let outside = Fixture::new();
        fs::write(outside.0.join("outside"), "outside").unwrap();
        let root_fd = openat(
            CWD,
            &fixture.0,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .unwrap();
        let moved = fixture.0.with_extension("pinned");
        fs::rename(&fixture.0, &moved).unwrap();
        symlink(&outside.0, &fixture.0).unwrap();

        let outcome = discover_opened_root(
            root_fd,
            &fixture.0,
            limits(),
            DirectoryDiscoveryUsage::default(),
        );
        let DirectoryDiscoveryOutcome::Complete {
            files,
            locator_status,
            ..
        } = outcome
        else {
            panic!("pinned traversal refused")
        };
        assert_eq!(files, vec![fixture.0.join("original")]);
        assert_eq!(
            locator_status,
            DiscoveredLocatorStatus::RevalidationRequired
        );

        fs::remove_file(&fixture.0).unwrap();
        fs::rename(moved, &fixture.0).unwrap();
    }

    #[test]
    fn discovery_refuses_file_and_depth_overflow_instead_of_truncating() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("a"), "a").unwrap();
        fs::write(fixture.0.join("b"), "b").unwrap();
        let mut bounded = limits();
        bounded.max_files = 1;
        assert!(matches!(
            discover_workspace_files(&fixture.0, bounded),
            DirectoryDiscoveryOutcome::Refused {
                reason: DirectoryDiscoveryRefusal::Limit,
                ..
            }
        ));

        fs::create_dir(fixture.0.join("nested")).unwrap();
        fs::write(fixture.0.join("nested/c"), "c").unwrap();
        bounded = limits();
        bounded.max_depth = 0;
        assert!(matches!(
            discover_workspace_files(&fixture.0, bounded),
            DirectoryDiscoveryOutcome::Refused {
                reason: DirectoryDiscoveryRefusal::Limit,
                ..
            }
        ));
    }

    #[test]
    fn individual_and_aggregate_path_bounds_refuse_the_whole_result() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("a"), "a").unwrap();
        fs::write(fixture.0.join("b"), "b").unwrap();
        let mut bounded = limits();
        bounded.max_path_bytes = fixture.0.as_os_str().as_bytes().len();
        assert!(matches!(
            discover_workspace_files(&fixture.0, bounded),
            DirectoryDiscoveryOutcome::Refused {
                reason: DirectoryDiscoveryRefusal::Limit,
                ..
            }
        ));

        bounded = limits();
        bounded.max_total_path_bytes = fixture.0.join("a").as_os_str().as_bytes().len();
        assert!(matches!(
            discover_workspace_files(&fixture.0, bounded),
            DirectoryDiscoveryOutcome::Refused {
                reason: DirectoryDiscoveryRefusal::Limit,
                ..
            }
        ));
    }
}
