//! Supervised local-process composition for the no-launch-authority LSP adapter.
//!
//! This crate is the sole production owner of executable selection, process
//! groups, stdio and process deadlines. The workspace cannot configure or trust
//! itself. The transport never exposes argv, environment values, paths, source,
//! stderr or process identifiers through public observations.
//! Canonical path validation is not descriptor anchoring: a same-authority actor
//! can still replace the executable or private cwd between validation and spawn.
//! Lifecycle usage distinguishes an accepted whole-group hard-kill request from
//! confirmed reap of the one supervised leader; it does not claim descendant
//! reap.
#![forbid(unsafe_code)]

mod runtime;
pub use runtime::*;
mod restart;
pub use restart::*;
mod restart_runtime;
pub use restart_runtime::*;
mod ordered;
pub use ordered::*;
mod authenticated_dispatch;
pub use authenticated_dispatch::*;
#[cfg(unix)]
mod service;
#[cfg(unix)]
pub use service::*;

use std::{
    collections::BTreeMap,
    fmt,
    num::{NonZeroU32, NonZeroU64},
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError},
    },
    task::{Context, Poll, ready},
    thread,
    time::Duration,
};

#[cfg(unix)]
use std::{
    fs::File,
    io::Read as _,
    os::unix::{ffi::OsStrExt as _, fs::MetadataExt as _},
};

#[cfg(unix)]
use processkit::RlimitResource;
use processkit::prelude::StreamExt as _;
use processkit::{
    Command, LimitReason, Mechanism, OutputBufferPolicy, ParentDeathCleanup, ProcessEvents,
    ProcessGroup, ProcessGroupOptions, ProcessGroupStats, RunningProcess, StdioMode,
};
use sha2::{Digest as _, Sha256};
use tokio::io::AsyncWrite;
use tokio::sync::mpsc as async_mpsc;
use tokio_util::sync::PollSender;
use vellum_adapter_semantic_lsp::{
    LspReceiveOutcomeV1, LspSendOutcomeV1, LspTransportPortV1, LspTransportTerminalV1,
};
use vellum_host::semantic::SemanticProviderProvenanceV1;

pub const MAX_ARGUMENTS_V1: usize = 128;
pub const MAX_ARGUMENT_BYTES_V1: usize = 16 * 1024;
pub const MAX_ENVIRONMENT_ENTRIES_V1: usize = 64;
pub const MAX_ENVIRONMENT_BYTES_V1: usize = 16 * 1024;
pub const MAX_STDIO_QUEUE_V1: usize = 256;
pub const MAX_STDIO_CHUNK_BYTES_V1: usize = 64 * 1024;
pub const MAX_STDIN_FRAME_BYTES_V1: usize = 32 * 1024 * 1024;
pub const MAX_STDIN_QUEUED_BYTES_V1: usize = 64 * 1024 * 1024;
pub const MAX_STDERR_BYTES_V1: usize = 4 * 1024 * 1024;
pub const MAX_DEADLINE_MILLIS_V1: u64 = 10 * 60 * 1_000;
pub const MAX_PROVIDER_ARTIFACT_BYTES_V1: u64 = 512 * 1024 * 1024;
pub const MAX_PROVIDER_MEMORY_BYTES_V1: u64 = 1024 * 1024 * 1024 * 1024;
pub const MAX_PROVIDER_PROCESSES_V1: u32 = 65_536;
pub const MAX_PROVIDER_CPU_MILLICORES_V1: u32 = 1_024_000;
pub const PROVIDER_RESOURCE_SAMPLE_MILLIS_V1: u64 = 250;

static SEMANTIC_LAUNCH_SERIALIZATION: Mutex<()> = Mutex::new(());
static SEMANTIC_LAUNCH_FENCED: AtomicBool = AtomicBool::new(false);
static SEMANTIC_UNRESOLVED_LAUNCH: Mutex<Option<SemanticUnresolvedLaunchStateV1>> =
    Mutex::new(None);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticProcessDeadlinesV1 {
    pub launch: Duration,
    pub graceful_shutdown: Duration,
    pub kill_and_reap: Duration,
}

impl SemanticProcessDeadlinesV1 {
    fn is_valid(self) -> bool {
        [self.launch, self.graceful_shutdown, self.kill_and_reap]
            .into_iter()
            .all(|value| !value.is_zero() && value <= Duration::from_millis(MAX_DEADLINE_MILLIS_V1))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticProcessLimitsV1 {
    pub command_queue: usize,
    pub stdout_queue: usize,
    pub stderr_queue: usize,
    pub max_chunk_bytes: usize,
    pub max_frame_bytes: usize,
    pub max_stderr_bytes: usize,
}

impl SemanticProcessLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            command_queue: 32,
            stdout_queue: 32,
            stderr_queue: 32,
            max_chunk_bytes: MAX_STDIO_CHUNK_BYTES_V1,
            max_frame_bytes: MAX_STDIN_FRAME_BYTES_V1,
            max_stderr_bytes: MAX_STDERR_BYTES_V1,
        }
    }

    fn is_valid(self) -> bool {
        self.command_queue > 0
            && self.command_queue <= MAX_STDIO_QUEUE_V1
            && self.stdout_queue > 0
            && self.stdout_queue <= MAX_STDIO_QUEUE_V1
            && self.stderr_queue > 0
            && self.stderr_queue <= MAX_STDIO_QUEUE_V1
            && self.max_chunk_bytes > 0
            && self.max_chunk_bytes <= MAX_STDIO_CHUNK_BYTES_V1
            && self.max_frame_bytes > 0
            && self.max_frame_bytes <= MAX_STDIN_FRAME_BYTES_V1
            && self.max_stderr_bytes > 0
            && self.max_stderr_bytes <= MAX_STDERR_BYTES_V1
    }
}

/// Closed physical resource policy for one supervised provider tree.
///
/// `Unbounded` is explicit and preserves the original constructor contract. A
/// required policy is all-or-nothing: every axis must be enforced by one
/// ProcessKit whole-tree container before any provider process can start.
/// Unsupported or partially enforceable hosts refuse rather than degrading to
/// per-process observations or an ambient unbounded launch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderResourcePolicyV1 {
    Unbounded,
    RequiredWholeTree {
        max_memory_bytes: NonZeroU64,
        max_processes: NonZeroU32,
        cpu_millicores: NonZeroU32,
    },
}

impl SemanticProviderResourcePolicyV1 {
    fn is_valid(self) -> bool {
        match self {
            Self::Unbounded => true,
            Self::RequiredWholeTree {
                max_memory_bytes,
                max_processes,
                cpu_millicores,
            } => {
                max_memory_bytes.get() <= MAX_PROVIDER_MEMORY_BYTES_V1
                    && max_processes.get() <= MAX_PROVIDER_PROCESSES_V1
                    && cpu_millicores.get() <= MAX_PROVIDER_CPU_MILLICORES_V1
            }
        }
    }

    fn process_group_options(self) -> ProcessGroupOptions {
        match self {
            Self::Unbounded => ProcessGroupOptions::default(),
            Self::RequiredWholeTree {
                max_memory_bytes,
                max_processes,
                cpu_millicores,
            } => ProcessGroupOptions::default()
                .max_memory(max_memory_bytes.get())
                .max_processes(max_processes.get())
                .cpu_quota(f64::from(cpu_millicores.get()) / 1_000.0),
        }
    }
}

/// Operator-installed launch product. Construction validates shape; launch
/// validates the current filesystem objects. Debug deliberately omits all
/// locator and environment material.
#[derive(Clone)]
pub struct SemanticProviderProfileV1 {
    pub provenance: SemanticProviderProvenanceV1,
    executable: PathBuf,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    private_cwd: PathBuf,
    workspace_root: PathBuf,
    deadlines: SemanticProcessDeadlinesV1,
    limits: SemanticProcessLimitsV1,
    resource_policy: SemanticProviderResourcePolicyV1,
}

impl fmt::Debug for SemanticProviderProfileV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticProviderProfileV1")
            .field("provenance", &self.provenance)
            .field("arguments", &self.arguments.len())
            .field("environment", &self.environment.len())
            .field("deadlines", &self.deadlines)
            .field("limits", &self.limits)
            .field("resource_policy", &self.resource_policy)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProfileRefusalV1 {
    InvalidExecutable,
    InvalidWorkingDirectory,
    WorkspaceWorkingDirectory,
    ArgumentLimit,
    EnvironmentLimit,
    InvalidEnvironment,
    InvalidDeadlines,
    InvalidLimits,
    InvalidResourcePolicy,
}

/// Honest support classification for one physical-provider assurance axis.
/// `ObservedOnly` is evidence, not enforcement; `Unsupported` must never be
/// projected as a zero measurement or an ambient fallback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderSupportV1 {
    Enforced,
    ObservedOnly,
    Unsupported,
}

/// Fixed-cardinality conditional launch assurance. `Enforced` means that any
/// successful process application must first pass the corresponding
/// fail-closed setup; qualification alone does not claim that a process ran or
/// that physical utilization was observed. This deliberately says nothing
/// about paths, digests, process identifiers, source or provider labels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticProviderLaunchAssuranceV1 {
    pub executable_identity: SemanticProviderSupportV1,
    pub working_directory_identity: SemanticProviderSupportV1,
    pub ambient_environment_cleared: SemanticProviderSupportV1,
    pub core_dumps_disabled: SemanticProviderSupportV1,
    pub filesystem_confinement: SemanticProviderSupportV1,
    pub network_confinement: SemanticProviderSupportV1,
    pub subprocess_confinement: SemanticProviderSupportV1,
    pub abrupt_parent_cleanup: SemanticProviderSupportV1,
    pub whole_tree_memory_limit: SemanticProviderSupportV1,
    pub whole_tree_process_limit: SemanticProviderSupportV1,
    pub whole_tree_cpu_limit: SemanticProviderSupportV1,
}

impl SemanticProviderLaunchAssuranceV1 {
    pub const fn current() -> Self {
        Self::for_resource_policy(SemanticProviderResourcePolicyV1::Unbounded)
    }

    const fn for_resource_policy(policy: SemanticProviderResourcePolicyV1) -> Self {
        let abrupt_parent_cleanup = match Command::kill_on_parent_death_scope() {
            ParentDeathCleanup::WholeTree => SemanticProviderSupportV1::Enforced,
            ParentDeathCleanup::DirectChildOnly => SemanticProviderSupportV1::ObservedOnly,
            ParentDeathCleanup::Unsupported => SemanticProviderSupportV1::Unsupported,
            _ => SemanticProviderSupportV1::Unsupported,
        };
        Self {
            // The exact bytes and filesystem identity are checked twice, but
            // ProcessKit still launches by path rather than an immutable fd.
            executable_identity: SemanticProviderSupportV1::ObservedOnly,
            working_directory_identity: SemanticProviderSupportV1::ObservedOnly,
            ambient_environment_cleared: SemanticProviderSupportV1::Enforced,
            core_dumps_disabled: if cfg!(unix) {
                SemanticProviderSupportV1::Enforced
            } else {
                SemanticProviderSupportV1::Unsupported
            },
            filesystem_confinement: SemanticProviderSupportV1::Unsupported,
            network_confinement: SemanticProviderSupportV1::Unsupported,
            subprocess_confinement: SemanticProviderSupportV1::Unsupported,
            abrupt_parent_cleanup,
            whole_tree_memory_limit: match policy {
                SemanticProviderResourcePolicyV1::Unbounded => {
                    SemanticProviderSupportV1::Unsupported
                }
                SemanticProviderResourcePolicyV1::RequiredWholeTree { .. } => {
                    SemanticProviderSupportV1::Enforced
                }
            },
            whole_tree_process_limit: match policy {
                SemanticProviderResourcePolicyV1::Unbounded => {
                    SemanticProviderSupportV1::Unsupported
                }
                SemanticProviderResourcePolicyV1::RequiredWholeTree { .. } => {
                    SemanticProviderSupportV1::Enforced
                }
            },
            whole_tree_cpu_limit: match policy {
                SemanticProviderResourcePolicyV1::Unbounded => {
                    SemanticProviderSupportV1::Unsupported
                }
                SemanticProviderResourcePolicyV1::RequiredWholeTree { .. } => {
                    SemanticProviderSupportV1::Enforced
                }
            },
        }
    }

    /// Projects one axis of the fixed production launch requirement product.
    ///
    /// This is a support disposition, not evidence that launch occurred. A
    /// production promotion requires every axis to be `Enforced`; neither
    /// `ObservedOnly` nor `Unsupported` is silently weakened.
    pub const fn support_for(
        self,
        requirement: SemanticProviderProductionRequirementV1,
    ) -> SemanticProviderSupportV1 {
        match requirement {
            SemanticProviderProductionRequirementV1::ExecutableIdentity => self.executable_identity,
            SemanticProviderProductionRequirementV1::WorkingDirectoryIdentity => {
                self.working_directory_identity
            }
            SemanticProviderProductionRequirementV1::AmbientEnvironmentCleared => {
                self.ambient_environment_cleared
            }
            SemanticProviderProductionRequirementV1::CoreDumpsDisabled => self.core_dumps_disabled,
            SemanticProviderProductionRequirementV1::FilesystemConfinement => {
                self.filesystem_confinement
            }
            SemanticProviderProductionRequirementV1::NetworkConfinement => self.network_confinement,
            SemanticProviderProductionRequirementV1::SubprocessConfinement => {
                self.subprocess_confinement
            }
            SemanticProviderProductionRequirementV1::AbruptParentCleanup => {
                self.abrupt_parent_cleanup
            }
            SemanticProviderProductionRequirementV1::WholeTreeMemoryLimit => {
                self.whole_tree_memory_limit
            }
            SemanticProviderProductionRequirementV1::WholeTreeProcessLimit => {
                self.whole_tree_process_limit
            }
            SemanticProviderProductionRequirementV1::WholeTreeCpuLimit => self.whole_tree_cpu_limit,
        }
    }
}

/// Closed production requirement lattice for one background provider launch.
///
/// Adding an assurance axis requires adding it to `ALL`, to `support_for`, and
/// therefore to production promotion. This prevents a new physical nonclaim
/// from being omitted by an open-ended checklist or caller-selected policy.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SemanticProviderProductionRequirementV1 {
    ExecutableIdentity,
    WorkingDirectoryIdentity,
    AmbientEnvironmentCleared,
    CoreDumpsDisabled,
    FilesystemConfinement,
    NetworkConfinement,
    SubprocessConfinement,
    AbruptParentCleanup,
    WholeTreeMemoryLimit,
    WholeTreeProcessLimit,
    WholeTreeCpuLimit,
}

impl SemanticProviderProductionRequirementV1 {
    pub const ALL: [Self; 11] = [
        Self::ExecutableIdentity,
        Self::WorkingDirectoryIdentity,
        Self::AmbientEnvironmentCleared,
        Self::CoreDumpsDisabled,
        Self::FilesystemConfinement,
        Self::NetworkConfinement,
        Self::SubprocessConfinement,
        Self::AbruptParentCleanup,
        Self::WholeTreeMemoryLimit,
        Self::WholeTreeProcessLimit,
        Self::WholeTreeCpuLimit,
    ];
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SemanticProviderArtifactIdentityV1([u8; 32]);

impl SemanticProviderArtifactIdentityV1 {
    /// Returns the domain-separated artifact digest for equality checks and
    /// content-safe qualification receipts. This reveals no executable path.
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Debug for SemanticProviderArtifactIdentityV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticProviderArtifactIdentityV1(<redacted>)")
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SemanticProviderProfileIdentityV1([u8; 32]);

impl SemanticProviderProfileIdentityV1 {
    /// Returns the domain-separated launch-profile digest for equality checks
    /// and content-safe qualification receipts. Profile inputs remain opaque.
    pub const fn digest(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Debug for SemanticProviderProfileIdentityV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SemanticProviderProfileIdentityV1(<redacted>)")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FilesystemIdentityV1 {
    device: u64,
    inode: u64,
    bytes: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    links: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
}

/// Affine qualification product consumed by one launch attempt. It proves a
/// bounded artifact/profile observation, not a signature, sandbox, dynamic
/// loader closure, plugin closure, or immutable descriptor launch.
#[must_use = "qualified provider launch evidence is single-use"]
pub struct QualifiedProviderLaunchV1 {
    profile: SemanticProviderProfileV1,
    executable_identity: FilesystemIdentityV1,
    cwd_identity: FilesystemIdentityV1,
    artifact: SemanticProviderArtifactIdentityV1,
    profile_identity: SemanticProviderProfileIdentityV1,
    assurance: SemanticProviderLaunchAssuranceV1,
}

impl fmt::Debug for QualifiedProviderLaunchV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("QualifiedProviderLaunchV1")
            .field("artifact", &self.artifact)
            .field("profile_identity", &self.profile_identity)
            .field("assurance", &self.assurance)
            .finish()
    }
}

impl QualifiedProviderLaunchV1 {
    pub const fn artifact_identity(&self) -> SemanticProviderArtifactIdentityV1 {
        self.artifact
    }

    pub const fn profile_identity(&self) -> SemanticProviderProfileIdentityV1 {
        self.profile_identity
    }

    pub const fn assurance(&self) -> SemanticProviderLaunchAssuranceV1 {
        self.assurance
    }
}

/// Affine provider launch that satisfies every fixed background-service
/// requirement. It remains conditional launch evidence: ProcessKit must still
/// create the requested container and start the process successfully.
#[must_use = "production-qualified provider launch evidence is single-use"]
pub struct ProductionQualifiedProviderLaunchV1 {
    qualified: QualifiedProviderLaunchV1,
}

impl ProductionQualifiedProviderLaunchV1 {
    pub const fn artifact_identity(&self) -> SemanticProviderArtifactIdentityV1 {
        self.qualified.artifact_identity()
    }

    pub const fn profile_identity(&self) -> SemanticProviderProfileIdentityV1 {
        self.qualified.profile_identity()
    }

    pub const fn assurance(&self) -> SemanticProviderLaunchAssuranceV1 {
        self.qualified.assurance()
    }
}

impl fmt::Debug for ProductionQualifiedProviderLaunchV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ProductionQualifiedProviderLaunchV1")
            .field(&self.qualified)
            .finish()
    }
}

/// Failed production promotion retains the exact affine launch evidence. A
/// caller may inspect every closed support disposition and may explicitly
/// recover the token for a probe launch; refusal never destroys ownership or
/// upgrades an observation into enforcement.
#[must_use = "production qualification refusal retains launch ownership"]
pub struct SemanticProviderProductionQualificationFailureV1 {
    qualified: QualifiedProviderLaunchV1,
}

impl SemanticProviderProductionQualificationFailureV1 {
    pub const fn support_for(
        &self,
        requirement: SemanticProviderProductionRequirementV1,
    ) -> SemanticProviderSupportV1 {
        self.qualified.assurance().support_for(requirement)
    }

    pub fn into_probe_qualified_launch(self: Box<Self>) -> QualifiedProviderLaunchV1 {
        self.qualified
    }
}

impl fmt::Debug for SemanticProviderProductionQualificationFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = formatter.debug_struct("SemanticProviderProductionQualificationFailureV1");
        for requirement in SemanticProviderProductionRequirementV1::ALL {
            debug.field(
                match requirement {
                    SemanticProviderProductionRequirementV1::ExecutableIdentity => {
                        "executable_identity"
                    }
                    SemanticProviderProductionRequirementV1::WorkingDirectoryIdentity => {
                        "working_directory_identity"
                    }
                    SemanticProviderProductionRequirementV1::AmbientEnvironmentCleared => {
                        "ambient_environment_cleared"
                    }
                    SemanticProviderProductionRequirementV1::CoreDumpsDisabled => {
                        "core_dumps_disabled"
                    }
                    SemanticProviderProductionRequirementV1::FilesystemConfinement => {
                        "filesystem_confinement"
                    }
                    SemanticProviderProductionRequirementV1::NetworkConfinement => {
                        "network_confinement"
                    }
                    SemanticProviderProductionRequirementV1::SubprocessConfinement => {
                        "subprocess_confinement"
                    }
                    SemanticProviderProductionRequirementV1::AbruptParentCleanup => {
                        "abrupt_parent_cleanup"
                    }
                    SemanticProviderProductionRequirementV1::WholeTreeMemoryLimit => {
                        "whole_tree_memory_limit"
                    }
                    SemanticProviderProductionRequirementV1::WholeTreeProcessLimit => {
                        "whole_tree_process_limit"
                    }
                    SemanticProviderProductionRequirementV1::WholeTreeCpuLimit => {
                        "whole_tree_cpu_limit"
                    }
                },
                &self.support_for(requirement),
            );
        }
        debug.finish()
    }
}

/// Promotes exact affine launch evidence into the only token accepted by the
/// production launch entry points. The meet of the fixed requirement lattice
/// must be `Enforced`; any weaker member returns the original owned evidence.
pub fn qualify_provider_launch_for_production_v1(
    qualified: QualifiedProviderLaunchV1,
) -> Result<
    ProductionQualifiedProviderLaunchV1,
    Box<SemanticProviderProductionQualificationFailureV1>,
> {
    let assurance = qualified.assurance();
    if SemanticProviderProductionRequirementV1::ALL
        .into_iter()
        .all(|requirement| {
            assurance.support_for(requirement) == SemanticProviderSupportV1::Enforced
        })
    {
        Ok(ProductionQualifiedProviderLaunchV1 { qualified })
    } else {
        Err(Box::new(SemanticProviderProductionQualificationFailureV1 {
            qualified,
        }))
    }
}

impl SemanticProviderProfileV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        provenance: SemanticProviderProvenanceV1,
        executable: PathBuf,
        arguments: Vec<String>,
        environment: BTreeMap<String, String>,
        private_cwd: PathBuf,
        workspace_root: PathBuf,
        deadlines: SemanticProcessDeadlinesV1,
        limits: SemanticProcessLimitsV1,
    ) -> Result<Self, SemanticProfileRefusalV1> {
        Self::try_new_with_resource_policy(
            provenance,
            executable,
            arguments,
            environment,
            private_cwd,
            workspace_root,
            deadlines,
            limits,
            SemanticProviderResourcePolicyV1::Unbounded,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn try_new_with_resource_policy(
        provenance: SemanticProviderProvenanceV1,
        executable: PathBuf,
        arguments: Vec<String>,
        environment: BTreeMap<String, String>,
        private_cwd: PathBuf,
        workspace_root: PathBuf,
        deadlines: SemanticProcessDeadlinesV1,
        limits: SemanticProcessLimitsV1,
        resource_policy: SemanticProviderResourcePolicyV1,
    ) -> Result<Self, SemanticProfileRefusalV1> {
        if !executable.is_absolute() || executable.as_os_str().is_empty() {
            return Err(SemanticProfileRefusalV1::InvalidExecutable);
        }
        if !private_cwd.is_absolute() || !workspace_root.is_absolute() {
            return Err(SemanticProfileRefusalV1::InvalidWorkingDirectory);
        }
        if private_cwd.starts_with(&workspace_root) || workspace_root.starts_with(&private_cwd) {
            return Err(SemanticProfileRefusalV1::WorkspaceWorkingDirectory);
        }
        let argument_bytes = arguments
            .iter()
            .try_fold(0_usize, |total, value| total.checked_add(value.len()));
        if arguments.len() > MAX_ARGUMENTS_V1
            || argument_bytes.is_none_or(|bytes| bytes > MAX_ARGUMENT_BYTES_V1)
            || arguments.iter().any(|value| value.contains('\0'))
        {
            return Err(SemanticProfileRefusalV1::ArgumentLimit);
        }
        let environment_bytes = environment.iter().try_fold(0_usize, |total, (key, value)| {
            total.checked_add(key.len())?.checked_add(value.len())
        });
        if environment.len() > MAX_ENVIRONMENT_ENTRIES_V1
            || environment_bytes.is_none_or(|bytes| bytes > MAX_ENVIRONMENT_BYTES_V1)
        {
            return Err(SemanticProfileRefusalV1::EnvironmentLimit);
        }
        if environment.iter().any(|(key, value)| {
            key.is_empty()
                || key.contains(['=', '\0'])
                || value.contains('\0')
                || loader_affecting_environment_name(key)
                || !key
                    .bytes()
                    .all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
        }) {
            return Err(SemanticProfileRefusalV1::InvalidEnvironment);
        }
        if !deadlines.is_valid() {
            return Err(SemanticProfileRefusalV1::InvalidDeadlines);
        }
        if !limits.is_valid() {
            return Err(SemanticProfileRefusalV1::InvalidLimits);
        }
        if !resource_policy.is_valid() {
            return Err(SemanticProfileRefusalV1::InvalidResourcePolicy);
        }
        Ok(Self {
            provenance,
            executable,
            arguments,
            environment,
            private_cwd,
            workspace_root,
            deadlines,
            limits,
            resource_policy,
        })
    }
}

fn loader_affecting_environment_name(name: &str) -> bool {
    name == "LD_PRELOAD"
        || name == "LD_LIBRARY_PATH"
        || name.starts_with("LD_AUDIT")
        || name.starts_with("DYLD_")
}

#[cfg(unix)]
fn filesystem_identity(metadata: &std::fs::Metadata) -> FilesystemIdentityV1 {
    FilesystemIdentityV1 {
        device: metadata.dev(),
        inode: metadata.ino(),
        bytes: metadata.len(),
        mode: metadata.mode(),
        uid: metadata.uid(),
        gid: metadata.gid(),
        links: metadata.nlink(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
    }
}

#[cfg(unix)]
fn path_bytes(path: &std::path::Path) -> &[u8] {
    path.as_os_str().as_bytes()
}

fn update_bounded_bytes(digest: &mut Sha256, bytes: &[u8]) {
    digest.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
    digest.update(bytes);
}

fn update_filesystem_identity(digest: &mut Sha256, identity: FilesystemIdentityV1) {
    digest.update(identity.device.to_be_bytes());
    digest.update(identity.inode.to_be_bytes());
    digest.update(identity.bytes.to_be_bytes());
    digest.update(identity.mode.to_be_bytes());
    digest.update(identity.uid.to_be_bytes());
    digest.update(identity.gid.to_be_bytes());
    digest.update(identity.links.to_be_bytes());
    digest.update(identity.modified_seconds.to_be_bytes());
    digest.update(identity.modified_nanoseconds.to_be_bytes());
}

#[cfg(unix)]
pub fn qualify_provider_launch_v1(
    profile: SemanticProviderProfileV1,
) -> Result<QualifiedProviderLaunchV1, SemanticProcessLaunchRefusalV1> {
    qualify_provider_launch_for_mechanism_v1(profile, processkit::host_containment().mechanism())
}

fn mechanism_supports_resource_policy(
    policy: SemanticProviderResourcePolicyV1,
    mechanism: Mechanism,
) -> bool {
    match policy {
        SemanticProviderResourcePolicyV1::Unbounded => true,
        SemanticProviderResourcePolicyV1::RequiredWholeTree { .. } => {
            matches!(mechanism, Mechanism::JobObject | Mechanism::CgroupV2)
        }
    }
}

#[cfg(unix)]
fn qualify_provider_launch_for_mechanism_v1(
    profile: SemanticProviderProfileV1,
    mechanism: Mechanism,
) -> Result<QualifiedProviderLaunchV1, SemanticProcessLaunchRefusalV1> {
    if !mechanism_supports_resource_policy(profile.resource_policy, mechanism) {
        return Err(SemanticProcessLaunchRefusalV1::ResourcePolicyUnsupported);
    }
    let executable = std::fs::canonicalize(&profile.executable)
        .map_err(|_| SemanticProcessLaunchRefusalV1::ExecutableUnavailable)?;
    let executable_metadata = executable
        .metadata()
        .map_err(|_| SemanticProcessLaunchRefusalV1::ExecutableUnavailable)?;
    if executable != profile.executable
        || !executable_metadata.is_file()
        || executable_metadata.len() > MAX_PROVIDER_ARTIFACT_BYTES_V1
        || executable_metadata.mode() & 0o111 == 0
        || executable_metadata.mode() & 0o022 != 0
        || (executable_metadata.uid() != 0
            && executable_metadata.uid() != rustix::process::geteuid().as_raw())
        || executable_metadata.nlink() != 1
    {
        return Err(SemanticProcessLaunchRefusalV1::ExecutableUnavailable);
    }
    let private_cwd = std::fs::canonicalize(&profile.private_cwd)
        .map_err(|_| SemanticProcessLaunchRefusalV1::WorkingDirectoryUnavailable)?;
    let workspace_root = std::fs::canonicalize(&profile.workspace_root)
        .map_err(|_| SemanticProcessLaunchRefusalV1::WorkingDirectoryUnavailable)?;
    let cwd_metadata = private_cwd
        .metadata()
        .map_err(|_| SemanticProcessLaunchRefusalV1::WorkingDirectoryUnavailable)?;
    if private_cwd != profile.private_cwd
        || workspace_root != profile.workspace_root
        || !cwd_metadata.is_dir()
        || cwd_metadata.uid() != rustix::process::geteuid().as_raw()
        || cwd_metadata.mode() & 0o7777 != 0o700
        || private_cwd.starts_with(&workspace_root)
        || workspace_root.starts_with(&private_cwd)
    {
        return Err(SemanticProcessLaunchRefusalV1::WorkingDirectoryUnavailable);
    }

    let mut file = File::open(&executable)
        .map_err(|_| SemanticProcessLaunchRefusalV1::ExecutableUnavailable)?;
    let mut artifact_digest = Sha256::new();
    artifact_digest.update(b"vellum-semantic-provider-artifact-v1\0");
    let mut buffer = [0u8; 64 * 1024];
    let mut observed = 0u64;
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| SemanticProcessLaunchRefusalV1::ExecutableUnavailable)?;
        if count == 0 {
            break;
        }
        observed = observed
            .checked_add(u64::try_from(count).unwrap_or(u64::MAX))
            .ok_or(SemanticProcessLaunchRefusalV1::ExecutableUnavailable)?;
        if observed > MAX_PROVIDER_ARTIFACT_BYTES_V1 {
            return Err(SemanticProcessLaunchRefusalV1::ExecutableUnavailable);
        }
        artifact_digest.update(&buffer[..count]);
    }
    if observed != executable_metadata.len() {
        return Err(SemanticProcessLaunchRefusalV1::ExecutableUnavailable);
    }
    let artifact = SemanticProviderArtifactIdentityV1(artifact_digest.finalize().into());
    let executable_identity = filesystem_identity(&executable_metadata);
    let cwd_identity = filesystem_identity(&cwd_metadata);

    let mut profile_digest = Sha256::new();
    profile_digest.update(b"vellum-semantic-provider-profile-v1\0");
    profile_digest.update(artifact.0);
    update_bounded_bytes(&mut profile_digest, path_bytes(&executable));
    update_filesystem_identity(&mut profile_digest, executable_identity);
    update_filesystem_identity(&mut profile_digest, cwd_identity);
    profile_digest.update(profile.provenance.provider().as_bytes());
    update_bounded_bytes(&mut profile_digest, profile.provenance.profile().as_bytes());
    update_bounded_bytes(&mut profile_digest, profile.provenance.version().as_bytes());
    update_bounded_bytes(&mut profile_digest, path_bytes(&private_cwd));
    update_bounded_bytes(&mut profile_digest, path_bytes(&workspace_root));
    for argument in &profile.arguments {
        update_bounded_bytes(&mut profile_digest, argument.as_bytes());
    }
    for (name, value) in &profile.environment {
        update_bounded_bytes(&mut profile_digest, name.as_bytes());
        update_bounded_bytes(&mut profile_digest, value.as_bytes());
    }
    for deadline in [
        profile.deadlines.launch,
        profile.deadlines.graceful_shutdown,
        profile.deadlines.kill_and_reap,
    ] {
        profile_digest.update(deadline.as_nanos().to_be_bytes());
    }
    for limit in [
        profile.limits.command_queue,
        profile.limits.stdout_queue,
        profile.limits.stderr_queue,
        profile.limits.max_chunk_bytes,
        profile.limits.max_frame_bytes,
        profile.limits.max_stderr_bytes,
    ] {
        profile_digest.update(u64::try_from(limit).unwrap_or(u64::MAX).to_be_bytes());
    }
    match profile.resource_policy {
        SemanticProviderResourcePolicyV1::Unbounded => profile_digest.update([0]),
        SemanticProviderResourcePolicyV1::RequiredWholeTree {
            max_memory_bytes,
            max_processes,
            cpu_millicores,
        } => {
            profile_digest.update([1]);
            profile_digest.update(max_memory_bytes.get().to_be_bytes());
            profile_digest.update(max_processes.get().to_be_bytes());
            profile_digest.update(cpu_millicores.get().to_be_bytes());
        }
    }
    let profile_identity = SemanticProviderProfileIdentityV1(profile_digest.finalize().into());
    let assurance = SemanticProviderLaunchAssuranceV1::for_resource_policy(profile.resource_policy);
    Ok(QualifiedProviderLaunchV1 {
        profile,
        executable_identity,
        cwd_identity,
        artifact,
        profile_identity,
        assurance,
    })
}

#[cfg(not(unix))]
pub fn qualify_provider_launch_v1(
    _profile: SemanticProviderProfileV1,
) -> Result<QualifiedProviderLaunchV1, SemanticProcessLaunchRefusalV1> {
    Err(SemanticProcessLaunchRefusalV1::QualificationUnsupported)
}

#[cfg(unix)]
fn revalidate_qualified_launch(
    qualified: &QualifiedProviderLaunchV1,
) -> Result<(), SemanticProcessLaunchRefusalV1> {
    let executable = std::fs::canonicalize(&qualified.profile.executable)
        .map_err(|_| SemanticProcessLaunchRefusalV1::ArtifactChanged)?;
    let metadata = executable
        .metadata()
        .map_err(|_| SemanticProcessLaunchRefusalV1::ArtifactChanged)?;
    if executable != qualified.profile.executable
        || filesystem_identity(&metadata) != qualified.executable_identity
    {
        return Err(SemanticProcessLaunchRefusalV1::ArtifactChanged);
    }
    let mut file =
        File::open(&executable).map_err(|_| SemanticProcessLaunchRefusalV1::ArtifactChanged)?;
    let mut digest = Sha256::new();
    digest.update(b"vellum-semantic-provider-artifact-v1\0");
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| SemanticProcessLaunchRefusalV1::ArtifactChanged)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if SemanticProviderArtifactIdentityV1(digest.finalize().into()) != qualified.artifact {
        return Err(SemanticProcessLaunchRefusalV1::ArtifactChanged);
    }
    let cwd = std::fs::canonicalize(&qualified.profile.private_cwd)
        .map_err(|_| SemanticProcessLaunchRefusalV1::WorkingDirectoryChanged)?;
    let cwd_metadata = cwd
        .metadata()
        .map_err(|_| SemanticProcessLaunchRefusalV1::WorkingDirectoryChanged)?;
    if cwd != qualified.profile.private_cwd
        || filesystem_identity(&cwd_metadata) != qualified.cwd_identity
    {
        return Err(SemanticProcessLaunchRefusalV1::WorkingDirectoryChanged);
    }
    Ok(())
}

#[cfg(not(unix))]
fn revalidate_qualified_launch(
    _qualified: &QualifiedProviderLaunchV1,
) -> Result<(), SemanticProcessLaunchRefusalV1> {
    Err(SemanticProcessLaunchRefusalV1::QualificationUnsupported)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProcessLaunchRefusalV1 {
    ExecutableUnavailable,
    WorkingDirectoryUnavailable,
    ArtifactChanged,
    WorkingDirectoryChanged,
    QualificationUnsupported,
    SpawnRefused,
    LaunchTimedOut,
    RuntimeUnavailable,
    CleanupUnconfirmed,
    LaunchFenced,
    ResourcePolicyUnsupported,
    ResourcePolicyUnenforceable,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticProcessUsageV1 {
    pub launches: u64,
    pub fresh_transport_claims: u64,
    pub fresh_transport_refusals: u64,
    pub frames_admitted: u64,
    pub frame_refusals: u64,
    pub stdout_bytes: u64,
    pub stdout_dropped_bytes: u64,
    pub stderr_bytes: u64,
    pub stderr_dropped_bytes: u64,
    pub crashes: u64,
    pub graceful_shutdowns: u64,
    /// Explicit composition-owned abort requests accepted before owner loss.
    pub explicit_abort_requests: u64,
    /// Abort requests caused solely by losing the transport owner. This is a
    /// request observation, not terminal cleanup evidence.
    pub owner_loss_abort_requests: u64,
    /// Whole-group hard-kill requests accepted by ProcessKit. This is not a
    /// descendant reap count.
    pub group_hard_kills_accepted: u64,
    /// Whole-group hard-kill requests refused or not confirmed by ProcessKit.
    pub group_kill_failures: u64,
    /// Supervised leader finish/reap operations that were not confirmed.
    pub leader_finish_failures: u64,
    pub nonzero_exits: u64,
    /// Supervised leaders whose `RunningProcess::finish` confirmed reap.
    pub leader_reaped: u64,
    /// Successful, content-free process-group resource observations.
    pub resource_samples: u64,
    /// Process-group resource observations that the backend could not produce.
    pub resource_sample_failures: u64,
    /// Latest live process-counting unit reported by the selected backend.
    /// POSIX process groups report groups rather than exact descendant counts.
    pub active_process_units_latest: u64,
    pub active_process_units_high_water: u64,
    pub cpu_time_observations: u64,
    pub cpu_time_nanos_high_water: u64,
    pub peak_memory_observations: u64,
    pub peak_memory_bytes_high_water: u64,
    pub io_read_observations: u64,
    pub io_read_bytes_high_water: u64,
    pub io_write_observations: u64,
    pub io_write_bytes_high_water: u64,
    pub peak_process_observations: u64,
    pub peak_process_units_high_water: u64,
    pub saturated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SemanticProcessResourceSampleV1 {
    active_process_units: usize,
    cpu_time_nanos: Option<u128>,
    peak_memory_bytes: Option<u64>,
    io_read_bytes: Option<u64>,
    io_write_bytes: Option<u64>,
    peak_process_units: Option<usize>,
}

impl From<ProcessGroupStats> for SemanticProcessResourceSampleV1 {
    fn from(stats: ProcessGroupStats) -> Self {
        Self {
            active_process_units: stats.active_process_count,
            cpu_time_nanos: stats.total_cpu_time.map(|value| value.as_nanos()),
            peak_memory_bytes: stats.peak_memory_bytes,
            io_read_bytes: stats.io_read_bytes,
            io_write_bytes: stats.io_write_bytes,
            peak_process_units: stats.peak_process_count,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticUnresolvedLaunchDispositionV1 {
    /// The worker proved that process application never began.
    NoApplicationConfirmed,
    /// The supervisor observed an accepted whole-group kill when required and
    /// confirmed reap of the supervised leader. This is not descendant-reap
    /// evidence or a confinement claim.
    CleanupConfirmed,
    /// Cleanup or process absence could not be proved. Only process restart
    /// closes this fence in V1.
    RestartRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticLaunchFenceDispositionV1 {
    Cleared,
    Retained,
}

#[must_use = "unresolved launch evidence must be retained"]
pub struct SemanticUnresolvedLaunchReceiptV1 {
    initial_refusal: SemanticProcessLaunchRefusalV1,
    disposition: SemanticUnresolvedLaunchDispositionV1,
    fence: SemanticLaunchFenceDispositionV1,
    usage: SemanticProcessUsageV1,
}

impl SemanticUnresolvedLaunchReceiptV1 {
    pub const fn initial_refusal(&self) -> SemanticProcessLaunchRefusalV1 {
        self.initial_refusal
    }

    pub const fn disposition(&self) -> SemanticUnresolvedLaunchDispositionV1 {
        self.disposition
    }

    pub const fn fence(&self) -> SemanticLaunchFenceDispositionV1 {
        self.fence
    }

    pub const fn usage(&self) -> SemanticProcessUsageV1 {
        self.usage
    }
}

impl fmt::Debug for SemanticUnresolvedLaunchReceiptV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticUnresolvedLaunchReceiptV1")
            .field("initial_refusal", &self.initial_refusal)
            .field("disposition", &self.disposition)
            .field("fence", &self.fence)
            .field("usage", &self.usage)
            .finish()
    }
}

struct SemanticUnresolvedLaunchStateV1 {
    initial_refusal: SemanticProcessLaunchRefusalV1,
    worker: thread::JoinHandle<SemanticUnresolvedLaunchDispositionV1>,
    shared: Arc<SharedState>,
}

#[must_use = "unresolved launch ownership must resolve or remain fenced"]
pub struct SemanticUnresolvedLaunchOwnerV1 {
    state: Option<SemanticUnresolvedLaunchStateV1>,
}

impl SemanticUnresolvedLaunchOwnerV1 {
    pub fn initial_refusal(&self) -> SemanticProcessLaunchRefusalV1 {
        self.state
            .as_ref()
            .expect("unresolved launch owner state")
            .initial_refusal
    }

    pub fn is_finished(&self) -> bool {
        self.state
            .as_ref()
            .expect("unresolved launch owner state")
            .worker
            .is_finished()
    }

    pub fn try_resolve(mut self) -> Result<SemanticUnresolvedLaunchReceiptV1, Self> {
        if !self.is_finished() {
            return Err(self);
        }
        let state = self.state.take().expect("unresolved launch owner state");
        let disposition = state
            .worker
            .join()
            .unwrap_or(SemanticUnresolvedLaunchDispositionV1::RestartRequired);
        let usage = *state.shared.usage.lock().expect("semantic usage mutex");
        let fence = match disposition {
            SemanticUnresolvedLaunchDispositionV1::NoApplicationConfirmed
            | SemanticUnresolvedLaunchDispositionV1::CleanupConfirmed => {
                SEMANTIC_LAUNCH_FENCED.store(false, Ordering::Release);
                SemanticLaunchFenceDispositionV1::Cleared
            }
            SemanticUnresolvedLaunchDispositionV1::RestartRequired => {
                SemanticLaunchFenceDispositionV1::Retained
            }
        };
        Ok(SemanticUnresolvedLaunchReceiptV1 {
            initial_refusal: state.initial_refusal,
            disposition,
            fence,
            usage,
        })
    }
}

impl fmt::Debug for SemanticUnresolvedLaunchOwnerV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticUnresolvedLaunchOwnerV1")
            .field("initial_refusal", &self.initial_refusal())
            .field("finished", &self.is_finished())
            .field("state", &"<owned>")
            .finish()
    }
}

impl Drop for SemanticUnresolvedLaunchOwnerV1 {
    fn drop(&mut self) {
        let Some(state) = self.state.take() else {
            return;
        };
        let mut orphan = SEMANTIC_UNRESOLVED_LAUNCH
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if orphan.is_none() {
            *orphan = Some(state);
        }
    }
}

#[must_use = "owned launch failure retains unresolved cleanup authority"]
pub enum SemanticProcessOwnedLaunchFailureV1 {
    Confirmed(SemanticProcessLaunchRefusalV1),
    Unresolved(SemanticUnresolvedLaunchOwnerV1),
}

impl SemanticProcessOwnedLaunchFailureV1 {
    pub fn reason(&self) -> SemanticProcessLaunchRefusalV1 {
        match self {
            Self::Confirmed(reason) => *reason,
            Self::Unresolved(owner) => owner.initial_refusal(),
        }
    }
}

impl fmt::Debug for SemanticProcessOwnedLaunchFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Confirmed(reason) => formatter.debug_tuple("Confirmed").field(reason).finish(),
            Self::Unresolved(owner) => formatter.debug_tuple("Unresolved").field(owner).finish(),
        }
    }
}

#[derive(Default)]
struct SharedState {
    live: AtomicBool,
    closing: AtomicBool,
    graceful_close: AtomicBool,
    abort: AtomicBool,
    terminal: AtomicU8,
    stdout_freshness: AtomicU8,
    queued_stdin_bytes: AtomicUsize,
    admission: Mutex<()>,
    usage: Mutex<SemanticProcessUsageV1>,
}

enum Control {
    Write(Vec<u8>),
    GracefulClose,
    Abort,
}

enum Event {
    Stdout(Vec<u8>),
}

const TERMINAL_PENDING: u8 = 0;
const TERMINAL_CLOSED: u8 = 1;
const TERMINAL_FAILED: u8 = 2;
const STDOUT_FRESH: u8 = 0;
const STDOUT_TAINTED: u8 = 1;
const STDOUT_CLAIMED: u8 = 2;
const FRESH_START_REFUSAL_NONE: u8 = 0;
const FRESH_START_REFUSAL_PREINITIALIZE_OUTPUT: u8 = 1;

enum StdinControl {
    Write(Vec<u8>),
    Close,
}

enum StdinStatus {
    Closed,
    Failed,
}

#[derive(Debug)]
enum PumpEvent {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    StdoutClosed,
    StderrClosed,
}

struct ChannelWriter {
    sender: PollSender<PumpEvent>,
    stream: bool,
    shared: Arc<SharedState>,
    closed: bool,
    max_chunk_bytes: usize,
}

impl ChannelWriter {
    fn new(
        sender: async_mpsc::Sender<PumpEvent>,
        stream: bool,
        shared: Arc<SharedState>,
        max_chunk_bytes: usize,
    ) -> Self {
        Self {
            sender: PollSender::new(sender),
            stream,
            shared,
            closed: false,
            max_chunk_bytes,
        }
    }
}

impl AsyncWrite for ChannelWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        if self.closed {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "semantic process stream closed",
            )));
        }
        ready!(self.sender.poll_reserve(cx)).map_err(std::io::Error::other)?;
        let length = bytes.len().min(self.max_chunk_bytes);
        if self.stream {
            let shared = Arc::clone(&self.shared);
            let _admission = shared.admission.lock().expect("semantic admission mutex");
            let _ = self.shared.stdout_freshness.compare_exchange(
                STDOUT_FRESH,
                STDOUT_TAINTED,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            self.sender
                .send_item(PumpEvent::Stdout(bytes[..length].to_vec()))
                .map_err(std::io::Error::other)?;
        } else {
            self.sender
                .send_item(PumpEvent::Stderr(bytes[..length].to_vec()))
                .map_err(std::io::Error::other)?;
        }
        Poll::Ready(Ok(length))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if self.closed {
            return Poll::Ready(Ok(()));
        }
        ready!(self.sender.poll_reserve(cx)).map_err(std::io::Error::other)?;
        if self.stream {
            let shared = Arc::clone(&self.shared);
            let _admission = shared.admission.lock().expect("semantic admission mutex");
            let _ = self.shared.stdout_freshness.compare_exchange(
                STDOUT_FRESH,
                STDOUT_TAINTED,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            self.sender
                .send_item(PumpEvent::StdoutClosed)
                .map_err(std::io::Error::other)?;
        } else {
            self.sender
                .send_item(PumpEvent::StderrClosed)
                .map_err(std::io::Error::other)?;
        }
        self.closed = true;
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.poll_flush(cx)
    }
}

/// Bounded process-backed implementation of the adapter's already-owned byte
/// port. `close` requests graceful stdin closure and starts a fixed escalation
/// deadline; `Drop` requests an immediate whole-group kill.
pub struct LocalLspTransportV1 {
    controls: SyncSender<Control>,
    events: Receiver<Event>,
    prefetched: Option<Event>,
    shared: Arc<SharedState>,
    max_frame_bytes: usize,
    max_queued_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalLspFreshTransportRefusalV1 {
    BufferedOutput,
    RuntimeUnavailable,
}

/// A failed pre-construction freshness claim retains the exact process
/// transport. Any observed stdout is prefetched back into that transport, so
/// refusal neither discards provider bytes nor fabricates an empty stream.
#[must_use = "failed transport freshness proof retains process cleanup authority"]
pub struct LocalLspFreshTransportFailureV1 {
    reason: LocalLspFreshTransportRefusalV1,
    transport: LocalLspTransportV1,
}

impl LocalLspFreshTransportFailureV1 {
    pub const fn reason(&self) -> LocalLspFreshTransportRefusalV1 {
        self.reason
    }

    pub fn into_transport(self) -> LocalLspTransportV1 {
        self.transport
    }
}

impl fmt::Debug for LocalLspFreshTransportFailureV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalLspFreshTransportFailureV1")
            .field("reason", &self.reason)
            .field("transport", &"<owned>")
            .finish()
    }
}

/// Provisional evidence that Vellum's supervised stdout pump has not yet
/// accepted bytes. The first initialize send and stdout producer share one
/// admission gate; that send is the actual linearization point. This makes no
/// claim about bytes physically buffered by the OS or ProcessKit.
#[must_use = "fresh transport evidence owns process cleanup authority"]
pub struct LocalLspFreshTransportV1 {
    inner: LocalLspTransportV1,
    initialize_admitted: bool,
    fresh_start_refusal: Arc<AtomicU8>,
}

/// Crate-private exact cause witness captured by the composition before the
/// transport moves into the no-launch-authority adapter. Only the first-send
/// gate writes it, so stdout arriving after an unrelated send refusal cannot
/// retroactively change that refusal's cause.
#[derive(Clone)]
pub(crate) struct LocalLspFreshStartWitnessV1(Arc<AtomicU8>);

impl LocalLspFreshStartWitnessV1 {
    pub(crate) fn refusal(&self) -> Option<LocalLspFreshTransportRefusalV1> {
        match self.0.load(Ordering::Acquire) {
            FRESH_START_REFUSAL_NONE => None,
            FRESH_START_REFUSAL_PREINITIALIZE_OUTPUT => {
                Some(LocalLspFreshTransportRefusalV1::BufferedOutput)
            }
            _ => unreachable!("fresh-start refusal witness is a closed two-state encoding"),
        }
    }
}

impl fmt::Debug for LocalLspFreshTransportV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LocalLspFreshTransportV1(<opaque>)")
    }
}

impl LocalLspFreshTransportV1 {
    pub(crate) fn fresh_start_witness(&self) -> LocalLspFreshStartWitnessV1 {
        LocalLspFreshStartWitnessV1(Arc::clone(&self.fresh_start_refusal))
    }
}

impl LocalLspTransportV1 {
    pub fn usage(&self) -> SemanticProcessUsageV1 {
        *self.shared.usage.lock().expect("semantic usage mutex")
    }

    pub fn abort(&self) {
        let _admission = self
            .shared
            .admission
            .lock()
            .expect("semantic admission mutex");
        self.shared.closing.store(true, Ordering::Release);
        if !self.shared.abort.swap(true, Ordering::AcqRel) {
            let mut usage = self.shared.usage.lock().expect("semantic usage mutex");
            let SemanticProcessUsageV1 {
                explicit_abort_requests,
                saturated,
                ..
            } = &mut *usage;
            inc(explicit_abort_requests, saturated);
        }
        let _ = self.controls.try_send(Control::Abort);
    }

    fn reserve_stdin_bytes(&self, bytes: usize) -> bool {
        let mut current = self.shared.queued_stdin_bytes.load(Ordering::Acquire);
        loop {
            let Some(next) = current.checked_add(bytes) else {
                return false;
            };
            if next > self.max_queued_bytes {
                return false;
            }
            match self.shared.queued_stdin_bytes.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(observed) => current = observed,
            }
        }
    }

    fn try_claim_fresh(
        mut self,
    ) -> Result<LocalLspFreshTransportV1, LocalLspFreshTransportFailureV1> {
        let shared = Arc::clone(&self.shared);
        let _admission = shared.admission.lock().expect("semantic admission mutex");
        let reason = if !self.shared.live.load(Ordering::Acquire)
            || self.shared.closing.load(Ordering::Acquire)
            || self.shared.terminal.load(Ordering::Acquire) != TERMINAL_PENDING
        {
            Some(LocalLspFreshTransportRefusalV1::RuntimeUnavailable)
        } else {
            match self.shared.stdout_freshness.load(Ordering::Acquire) {
                STDOUT_FRESH => None,
                STDOUT_TAINTED => {
                    if let Ok(event) = self.events.try_recv() {
                        self.prefetched = Some(event);
                    }
                    Some(LocalLspFreshTransportRefusalV1::BufferedOutput)
                }
                _ => Some(LocalLspFreshTransportRefusalV1::RuntimeUnavailable),
            }
        };
        if reason.is_some() {
            let mut usage = self.shared.usage.lock().expect("semantic usage mutex");
            let SemanticProcessUsageV1 {
                fresh_transport_refusals,
                saturated,
                ..
            } = &mut *usage;
            inc(fresh_transport_refusals, saturated);
        }
        match reason {
            None => Ok(LocalLspFreshTransportV1 {
                inner: self,
                initialize_admitted: false,
                fresh_start_refusal: Arc::new(AtomicU8::new(FRESH_START_REFUSAL_NONE)),
            }),
            Some(reason) => Err(LocalLspFreshTransportFailureV1 {
                reason,
                transport: self,
            }),
        }
    }

    fn try_send_admitted(&mut self, frame: &[u8]) -> LspSendOutcomeV1 {
        if !self.shared.live.load(Ordering::Acquire) || self.shared.closing.load(Ordering::Acquire)
        {
            return LspSendOutcomeV1::Unavailable;
        }
        if frame.is_empty() || frame.len() > self.max_frame_bytes {
            let mut usage = self.shared.usage.lock().expect("semantic usage mutex");
            let SemanticProcessUsageV1 {
                frame_refusals,
                saturated,
                ..
            } = &mut *usage;
            inc(frame_refusals, saturated);
            return LspSendOutcomeV1::Refused;
        }
        if !self.reserve_stdin_bytes(frame.len()) {
            let mut usage = self.shared.usage.lock().expect("semantic usage mutex");
            let SemanticProcessUsageV1 {
                frame_refusals,
                saturated,
                ..
            } = &mut *usage;
            inc(frame_refusals, saturated);
            return LspSendOutcomeV1::Refused;
        }
        let outcome = match self.controls.try_send(Control::Write(frame.to_vec())) {
            Ok(()) => LspSendOutcomeV1::Accepted,
            Err(TrySendError::Full(Control::Write(bytes))) => {
                self.shared
                    .queued_stdin_bytes
                    .fetch_sub(bytes.len(), Ordering::AcqRel);
                LspSendOutcomeV1::Refused
            }
            Err(TrySendError::Disconnected(Control::Write(bytes))) => {
                self.shared
                    .queued_stdin_bytes
                    .fetch_sub(bytes.len(), Ordering::AcqRel);
                LspSendOutcomeV1::Unavailable
            }
            Err(_) => unreachable!("only write controls are sent here"),
        };
        let mut usage = self.shared.usage.lock().expect("semantic usage mutex");
        match outcome {
            LspSendOutcomeV1::Accepted => {
                let SemanticProcessUsageV1 {
                    frames_admitted,
                    saturated,
                    ..
                } = &mut *usage;
                inc(frames_admitted, saturated);
            }
            LspSendOutcomeV1::Unavailable | LspSendOutcomeV1::Refused => {
                let SemanticProcessUsageV1 {
                    frame_refusals,
                    saturated,
                    ..
                } = &mut *usage;
                inc(frame_refusals, saturated);
            }
        }
        outcome
    }
}

impl LspTransportPortV1 for LocalLspTransportV1 {
    fn try_send(&mut self, frame: &[u8]) -> LspSendOutcomeV1 {
        let shared = Arc::clone(&self.shared);
        let _admission = shared.admission.lock().expect("semantic admission mutex");
        self.try_send_admitted(frame)
    }

    fn poll_receive(&mut self) -> LspReceiveOutcomeV1 {
        let event = self
            .prefetched
            .take()
            .map_or_else(|| self.events.try_recv(), Ok);
        match event {
            Ok(Event::Stdout(bytes)) => LspReceiveOutcomeV1::Chunk(bytes),
            Err(TryRecvError::Empty) => LspReceiveOutcomeV1::Pending,
            Err(TryRecvError::Disconnected) => match self.shared.terminal.load(Ordering::Acquire) {
                TERMINAL_CLOSED => LspReceiveOutcomeV1::Closed,
                TERMINAL_FAILED => LspReceiveOutcomeV1::Failed,
                TERMINAL_PENDING => LspReceiveOutcomeV1::Pending,
                _ => LspReceiveOutcomeV1::Failed,
            },
        }
    }

    fn close(&mut self) {
        let _admission = self
            .shared
            .admission
            .lock()
            .expect("semantic admission mutex");
        self.shared.closing.store(true, Ordering::Release);
        self.shared.graceful_close.store(true, Ordering::Release);
        let _ = self.controls.try_send(Control::GracefulClose);
    }

    fn abort(&mut self) {
        LocalLspTransportV1::abort(self);
    }

    fn terminal_state(&self) -> LspTransportTerminalV1 {
        match self.shared.terminal.load(Ordering::Acquire) {
            TERMINAL_PENDING => LspTransportTerminalV1::Active,
            TERMINAL_FAILED => LspTransportTerminalV1::Failed,
            TERMINAL_CLOSED => {
                let usage = self.shared.usage.lock().expect("semantic usage mutex");
                if usage.group_hard_kills_accepted == 0 {
                    LspTransportTerminalV1::GracefulClosed
                } else {
                    LspTransportTerminalV1::ForcedClosed
                }
            }
            _ => LspTransportTerminalV1::Failed,
        }
    }
}

impl LspTransportPortV1 for LocalLspFreshTransportV1 {
    fn try_send(&mut self, frame: &[u8]) -> LspSendOutcomeV1 {
        if self.initialize_admitted {
            return self.inner.try_send(frame);
        }
        let shared = Arc::clone(&self.inner.shared);
        let _admission = shared.admission.lock().expect("semantic admission mutex");
        if shared.stdout_freshness.load(Ordering::Acquire) != STDOUT_FRESH {
            self.fresh_start_refusal
                .store(FRESH_START_REFUSAL_PREINITIALIZE_OUTPUT, Ordering::Release);
            let mut usage = shared.usage.lock().expect("semantic usage mutex");
            let SemanticProcessUsageV1 {
                fresh_transport_refusals,
                frame_refusals,
                saturated,
                ..
            } = &mut *usage;
            inc(fresh_transport_refusals, saturated);
            inc(frame_refusals, saturated);
            return LspSendOutcomeV1::Refused;
        }
        let outcome = self.inner.try_send_admitted(frame);
        if outcome == LspSendOutcomeV1::Accepted {
            shared
                .stdout_freshness
                .store(STDOUT_CLAIMED, Ordering::Release);
            self.initialize_admitted = true;
            let mut usage = shared.usage.lock().expect("semantic usage mutex");
            let SemanticProcessUsageV1 {
                fresh_transport_claims,
                saturated,
                ..
            } = &mut *usage;
            inc(fresh_transport_claims, saturated);
        }
        outcome
    }

    fn poll_receive(&mut self) -> LspReceiveOutcomeV1 {
        self.inner.poll_receive()
    }

    fn close(&mut self) {
        self.inner.close();
    }

    fn abort(&mut self) {
        self.inner.abort();
    }

    fn terminal_state(&self) -> LspTransportTerminalV1 {
        self.inner.terminal_state()
    }
}

impl Drop for LocalLspTransportV1 {
    fn drop(&mut self) {
        let _admission = self
            .shared
            .admission
            .lock()
            .expect("semantic admission mutex");
        self.shared.closing.store(true, Ordering::Release);
        if !self.shared.abort.swap(true, Ordering::AcqRel) {
            let mut usage = self.shared.usage.lock().expect("semantic usage mutex");
            let SemanticProcessUsageV1 {
                owner_loss_abort_requests,
                saturated,
                ..
            } = &mut *usage;
            inc(owner_loss_abort_requests, saturated);
        }
        let _ = self.controls.try_send(Control::Abort);
    }
}

pub fn launch_local_lsp_v1(
    profile: SemanticProviderProfileV1,
) -> Result<LocalLspTransportV1, SemanticProcessLaunchRefusalV1> {
    if SEMANTIC_LAUNCH_FENCED.load(Ordering::Acquire) {
        return Err(SemanticProcessLaunchRefusalV1::LaunchFenced);
    }
    let qualified = qualify_provider_launch_v1(profile)?;
    launch_qualified_local_lsp_v1(qualified)
}

fn serialized_launch_guard() -> Result<MutexGuard<'static, ()>, SemanticProcessLaunchRefusalV1> {
    if SEMANTIC_LAUNCH_FENCED.load(Ordering::Acquire) {
        return Err(SemanticProcessLaunchRefusalV1::LaunchFenced);
    }
    let guard = SEMANTIC_LAUNCH_SERIALIZATION.lock().map_err(|_| {
        SEMANTIC_LAUNCH_FENCED.store(true, Ordering::Release);
        SemanticProcessLaunchRefusalV1::LaunchFenced
    })?;
    if SEMANTIC_LAUNCH_FENCED.load(Ordering::Acquire) {
        return Err(SemanticProcessLaunchRefusalV1::LaunchFenced);
    }
    Ok(guard)
}

pub fn launch_qualified_local_lsp_v1(
    qualified: QualifiedProviderLaunchV1,
) -> Result<LocalLspTransportV1, SemanticProcessLaunchRefusalV1> {
    match launch_qualified_local_lsp_owned_v1(qualified) {
        Ok(transport) => Ok(transport),
        Err(failure) => {
            let reason = failure.reason();
            drop(failure);
            Err(reason)
        }
    }
}

/// Production-only launch boundary. Callers cannot reach it with an ordinary
/// qualified profile; the complete fixed assurance product must first be
/// promoted by [`qualify_provider_launch_for_production_v1`].
pub fn launch_production_local_lsp_v1(
    qualified: ProductionQualifiedProviderLaunchV1,
) -> Result<LocalLspTransportV1, SemanticProcessLaunchRefusalV1> {
    launch_qualified_local_lsp_v1(qualified.qualified)
}

/// Ownership-preserving production launch boundary. Uncertain launch and
/// cleanup retain the same affine failure product as the probe boundary.
pub fn launch_production_local_lsp_owned_v1(
    qualified: ProductionQualifiedProviderLaunchV1,
) -> Result<LocalLspTransportV1, SemanticProcessOwnedLaunchFailureV1> {
    launch_qualified_local_lsp_owned_v1(qualified.qualified)
}

pub fn recover_unresolved_local_lsp_launch_v1() -> Option<SemanticUnresolvedLaunchOwnerV1> {
    let state = SEMANTIC_UNRESOLVED_LAUNCH
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()?;
    Some(SemanticUnresolvedLaunchOwnerV1 { state: Some(state) })
}

pub fn launch_qualified_local_lsp_owned_v1(
    qualified: QualifiedProviderLaunchV1,
) -> Result<LocalLspTransportV1, SemanticProcessOwnedLaunchFailureV1> {
    let _launch_guard =
        serialized_launch_guard().map_err(SemanticProcessOwnedLaunchFailureV1::Confirmed)?;
    revalidate_qualified_launch(&qualified)
        .map_err(SemanticProcessOwnedLaunchFailureV1::Confirmed)?;
    let profile = &qualified.profile;
    let launch_deadline = std::time::Instant::now() + profile.deadlines.launch;
    let (control_tx, control_rx) = mpsc::sync_channel(profile.limits.command_queue);
    let (event_tx, event_rx) = mpsc::sync_channel(profile.limits.stdout_queue);
    let (launch_tx, launch_rx) = mpsc::sync_channel(1);
    let max_frame_bytes = profile.limits.max_frame_bytes;
    let max_queued_bytes = profile
        .limits
        .command_queue
        .saturating_mul(max_frame_bytes)
        .min(MAX_STDIN_QUEUED_BYTES_V1);
    let shared = Arc::new(SharedState::default());
    let worker_shared = Arc::clone(&shared);
    let panic_launch = launch_tx.clone();
    let panic_shared = Arc::clone(&shared);
    let worker = thread::Builder::new()
        .name("vellum-semantic-local".into())
        .spawn(move || {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_worker(
                    qualified,
                    control_rx,
                    event_tx,
                    launch_tx,
                    worker_shared,
                    launch_deadline,
                )
            })) {
                Ok(resolution) => resolution,
                Err(_) => {
                    panic_shared.live.store(false, Ordering::Release);
                    panic_shared
                        .terminal
                        .store(TERMINAL_FAILED, Ordering::Release);
                    let _ = panic_launch
                        .try_send(Err(SemanticProcessLaunchRefusalV1::CleanupUnconfirmed));
                    SemanticUnresolvedLaunchDispositionV1::RestartRequired
                }
            }
        })
        .map_err(|_| {
            SemanticProcessOwnedLaunchFailureV1::Confirmed(
                SemanticProcessLaunchRefusalV1::RuntimeUnavailable,
            )
        })?;
    let mut retained_worker = Some(worker);
    let outcome = match launch_rx
        .recv_timeout(launch_deadline.saturating_duration_since(std::time::Instant::now()))
    {
        Ok(Ok(())) => {
            let mut usage = shared.usage.lock().expect("semantic usage mutex");
            let SemanticProcessUsageV1 {
                launches,
                saturated,
                ..
            } = &mut *usage;
            inc(launches, saturated);
            drop(usage);
            Ok(LocalLspTransportV1 {
                controls: control_tx,
                events: event_rx,
                prefetched: None,
                shared: Arc::clone(&shared),
                max_frame_bytes,
                max_queued_bytes,
            })
        }
        Ok(Err(refusal)) => Err(refusal),
        Err(RecvTimeoutError::Timeout) => Err(SemanticProcessLaunchRefusalV1::LaunchTimedOut),
        Err(RecvTimeoutError::Disconnected) => {
            Err(SemanticProcessLaunchRefusalV1::RuntimeUnavailable)
        }
    };
    match outcome {
        Ok(transport) => {
            drop(retained_worker);
            Ok(transport)
        }
        Err(
            refusal @ (SemanticProcessLaunchRefusalV1::CleanupUnconfirmed
            | SemanticProcessLaunchRefusalV1::LaunchTimedOut),
        ) => {
            shared.abort.store(true, Ordering::Release);
            SEMANTIC_LAUNCH_FENCED.store(true, Ordering::Release);
            Err(SemanticProcessOwnedLaunchFailureV1::Unresolved(
                SemanticUnresolvedLaunchOwnerV1 {
                    state: Some(SemanticUnresolvedLaunchStateV1 {
                        initial_refusal: refusal,
                        worker: retained_worker.take().expect("launch worker retained"),
                        shared,
                    }),
                },
            ))
        }
        Err(refusal) => {
            drop(retained_worker);
            Err(SemanticProcessOwnedLaunchFailureV1::Confirmed(refusal))
        }
    }
}

fn run_worker(
    qualified: QualifiedProviderLaunchV1,
    controls: Receiver<Control>,
    events: SyncSender<Event>,
    launch: SyncSender<Result<(), SemanticProcessLaunchRefusalV1>>,
    shared: Arc<SharedState>,
    launch_deadline: std::time::Instant,
) -> SemanticUnresolvedLaunchDispositionV1 {
    #[cfg(test)]
    if let Some(delay) = qualified
        .profile
        .environment
        .get("VELLUM_TEST_DELAY_BEFORE_START_MILLIS")
        .and_then(|value| value.parse::<u64>().ok())
    {
        thread::sleep(Duration::from_millis(delay));
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_io()
        .enable_time()
        .build()
    {
        Ok(runtime) => runtime,
        Err(_) => {
            let _ = launch.send(Err(SemanticProcessLaunchRefusalV1::RuntimeUnavailable));
            return SemanticUnresolvedLaunchDispositionV1::NoApplicationConfirmed;
        }
    };
    runtime.block_on(async move {
        let group = match ProcessGroup::with_options(
            qualified.profile.resource_policy.process_group_options(),
        ) {
            Ok(group) => Arc::new(group),
            Err(error) => {
                let refusal = match error.reason().limit_reason() {
                    Some(LimitReason::Unsupported) => {
                        SemanticProcessLaunchRefusalV1::ResourcePolicyUnsupported
                    }
                    Some(LimitReason::Invalid | LimitReason::Unenforceable) => {
                        SemanticProcessLaunchRefusalV1::ResourcePolicyUnenforceable
                    }
                    Some(_) => SemanticProcessLaunchRefusalV1::ResourcePolicyUnenforceable,
                    None => SemanticProcessLaunchRefusalV1::SpawnRefused,
                };
                let _ = launch.send(Err(refusal));
                return SemanticUnresolvedLaunchDispositionV1::NoApplicationConfirmed;
            }
        };
        #[cfg(test)]
        if qualified
            .profile
            .environment
            .contains_key("VELLUM_TEST_UNRESOLVED_RESTART_REQUIRED")
        {
            let _ = launch.send(Err(
                SemanticProcessLaunchRefusalV1::CleanupUnconfirmed,
            ));
            return SemanticUnresolvedLaunchDispositionV1::RestartRequired;
        }
        if let Err(refusal) = revalidate_qualified_launch(&qualified) {
            let _ = launch.send(Err(refusal));
            return SemanticUnresolvedLaunchDispositionV1::NoApplicationConfirmed;
        }
        let profile = qualified.profile;
        let pump_capacity = profile
            .limits
            .stdout_queue
            .saturating_add(profile.limits.stderr_queue);
        let (pump_tx, mut pump_rx) = async_mpsc::channel(pump_capacity);
        let stdout = ChannelWriter::new(
            pump_tx.clone(),
            true,
            Arc::clone(&shared),
            profile.limits.max_chunk_bytes,
        );
        let stderr = ChannelWriter::new(
            pump_tx.clone(),
            false,
            Arc::clone(&shared),
            profile.limits.max_chunk_bytes,
        );
        drop(pump_tx);
        let mut command = Command::new(&profile.executable)
            .args(&profile.arguments)
            .current_dir(&profile.private_cwd)
            .env_clear()
            .envs(profile.environment.iter())
            .keep_stdin_open()
            .stdout(StdioMode::Piped)
            .stderr(StdioMode::Piped)
            .output_buffer(
                OutputBufferPolicy::bounded(profile.limits.stdout_queue)
                    .with_max_bytes(profile.limits.max_chunk_bytes),
            )
            .stdout_raw_tee(stdout)
            .stderr_raw_tee(stderr);
        #[cfg(unix)]
        {
            command = command.rlimit(RlimitResource::Core, 0, 0).umask(0o077);
        }
        command = command.kill_on_parent_death();
        let launch_remaining = launch_deadline
            .saturating_duration_since(std::time::Instant::now());
        if launch_remaining.is_zero() {
            let _ = launch.send(Err(SemanticProcessLaunchRefusalV1::LaunchTimedOut));
            return SemanticUnresolvedLaunchDispositionV1::NoApplicationConfirmed;
        }
        let mut run = match tokio::time::timeout(launch_remaining, group.start(&command)).await {
            Ok(Ok(run)) => run,
            Ok(Err(_)) => {
                let empty = group.members().is_ok_and(|members| members.is_empty());
                let refusal = if empty {
                    SemanticProcessLaunchRefusalV1::SpawnRefused
                } else {
                    SemanticProcessLaunchRefusalV1::CleanupUnconfirmed
                };
                let _ = launch.send(Err(refusal));
                return if empty {
                    SemanticUnresolvedLaunchDispositionV1::NoApplicationConfirmed
                } else {
                    SemanticUnresolvedLaunchDispositionV1::RestartRequired
                };
            }
            Err(_) => {
                let _ = launch.send(Err(
                    SemanticProcessLaunchRefusalV1::CleanupUnconfirmed,
                ));
                return SemanticUnresolvedLaunchDispositionV1::RestartRequired;
            }
        };
        #[cfg(test)]
        if profile
            .environment
            .contains_key("VELLUM_TEST_FAIL_AFTER_START")
        {
            let cleaned = cleanup_owned_launch(
                Arc::clone(&group),
                run,
                None,
                &mut pump_rx,
                profile.deadlines.kill_and_reap,
            )
            .await;
            let _ = launch.send(Err(if cleaned {
                SemanticProcessLaunchRefusalV1::SpawnRefused
            } else {
                SemanticProcessLaunchRefusalV1::CleanupUnconfirmed
            }));
            return if cleaned {
                SemanticUnresolvedLaunchDispositionV1::CleanupConfirmed
            } else {
                SemanticUnresolvedLaunchDispositionV1::RestartRequired
            };
        }
        let Some(stdin) = run.take_stdin() else {
            let cleaned = cleanup_owned_launch(
                Arc::clone(&group),
                run,
                None,
                &mut pump_rx,
                profile.deadlines.kill_and_reap,
            )
            .await;
            let _ = launch.send(Err(if cleaned {
                SemanticProcessLaunchRefusalV1::SpawnRefused
            } else {
                SemanticProcessLaunchRefusalV1::CleanupUnconfirmed
            }));
            return if cleaned {
                SemanticUnresolvedLaunchDispositionV1::CleanupConfirmed
            } else {
                SemanticUnresolvedLaunchDispositionV1::RestartRequired
            };
        };
        let mut process_events = match run.events() {
            Ok(events) => events,
            Err(_) => {
                drop(stdin);
                let cleaned = cleanup_owned_launch(
                    Arc::clone(&group),
                    run,
                    None,
                    &mut pump_rx,
                    profile.deadlines.kill_and_reap,
                )
                .await;
                let _ = launch.send(Err(if cleaned {
                    SemanticProcessLaunchRefusalV1::SpawnRefused
                } else {
                    SemanticProcessLaunchRefusalV1::CleanupUnconfirmed
                }));
                return if cleaned {
                    SemanticUnresolvedLaunchDispositionV1::CleanupConfirmed
                } else {
                    SemanticUnresolvedLaunchDispositionV1::RestartRequired
                };
            }
        };
        let Some(leader_pid) = run.pid() else {
            drop(stdin);
            let cleaned = cleanup_owned_launch(
                Arc::clone(&group),
                run,
                Some(process_events),
                &mut pump_rx,
                profile.deadlines.kill_and_reap,
            )
            .await;
            let _ = launch.send(Err(if cleaned {
                SemanticProcessLaunchRefusalV1::SpawnRefused
            } else {
                SemanticProcessLaunchRefusalV1::CleanupUnconfirmed
            }));
            return if cleaned {
                SemanticUnresolvedLaunchDispositionV1::CleanupConfirmed
            } else {
                SemanticUnresolvedLaunchDispositionV1::RestartRequired
            };
        };
        let mut event_drain = tokio::spawn(async move {
            while process_events.next().await.is_some() {}
        });
        // `ProcessEvents::Exited` is published by the consuming finisher. Drive
        // it from launch so leader reap never depends on first seeing pipe EOF;
        // descendants are allowed to inherit those pipes.
        let mut finish_task = tokio::spawn(async move { run.finish().await });
        let (stdin_tx, mut stdin_rx) = async_mpsc::channel(profile.limits.command_queue);
        let (stdin_status_tx, mut stdin_status_rx) = async_mpsc::channel(1);
        let stdin_shared = Arc::clone(&shared);
        let stdin_task = tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(control) = stdin_rx.recv().await {
                match control {
                    StdinControl::Write(bytes) => {
                        let length = bytes.len();
                        let result = stdin.write(&bytes).await;
                        let result = match result {
                            Ok(()) => stdin.flush().await,
                            Err(error) => Err(error),
                        };
                        stdin_shared
                            .queued_stdin_bytes
                            .fetch_sub(length, Ordering::AcqRel);
                        if result.is_err() {
                            let _ = stdin_status_tx.send(StdinStatus::Failed).await;
                            return;
                        }
                    }
                    StdinControl::Close => {
                        let _ = stdin.finish().await;
                        let _ = stdin_status_tx.send(StdinStatus::Closed).await;
                        return;
                    }
                }
            }
        });
        sample_process_resources(&group, &shared);
        shared.live.store(true, Ordering::Release);
        if launch.send(Ok(())).is_err() {
            shared.abort.store(true, Ordering::Release);
        }

        let mut requested_close = false;
        let mut forced = false;
        let mut unexpected_exit = false;
        let mut terminal_failure = false;
        let mut stdout_closed = false;
        let mut stderr_closed = false;
        let mut close_deadline = None;
        let mut stderr_retained = 0_usize;
        let mut stdin_close_pending = false;
        let mut stdin_status_open = true;
        let mut finish = None;
        let mut resource_tick = tokio::time::interval(Duration::from_millis(
            PROVIDER_RESOURCE_SAMPLE_MILLIS_V1,
        ));
        resource_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            let control = match controls.try_recv() {
                Ok(control) => Some(control),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => {
                    requested_close = true;
                    forced = true;
                    None
                }
            };
            let had_control = control.is_some();
            if let Some(control) = control {
                match control {
                    Control::Write(bytes) => {
                        let length = bytes.len();
                        if stdin_tx.try_send(StdinControl::Write(bytes)).is_err() {
                            shared.queued_stdin_bytes.fetch_sub(length, Ordering::AcqRel);
                            requested_close = true;
                            forced = true;
                            terminal_failure = true;
                        }
                    }
                    Control::GracefulClose => {
                        requested_close = true;
                        stdin_close_pending = true;
                    }
                    Control::Abort => {
                        requested_close = true;
                        forced = true;
                    }
                }
            }
            if shared.abort.swap(false, Ordering::AcqRel) {
                requested_close = true;
                forced = true;
            } else if shared.graceful_close.swap(false, Ordering::AcqRel) {
                requested_close = true;
                stdin_close_pending = true;
            }
            if forced {
                break;
            }
            if requested_close && close_deadline.is_none() {
                close_deadline = Some(
                    tokio::time::Instant::now() + profile.deadlines.graceful_shutdown,
                );
            }
            if stdin_close_pending && !had_control {
                match stdin_tx.try_send(StdinControl::Close) {
                    Ok(()) => stdin_close_pending = false,
                    Err(async_mpsc::error::TrySendError::Full(_)) => {}
                    Err(async_mpsc::error::TrySendError::Closed(_)) => {
                        terminal_failure = true;
                        forced = true;
                        break;
                    }
                }
            }
            tokio::select! {
                result = &mut finish_task, if finish.is_none() => {
                    finish = Some(result.ok().and_then(Result::ok));
                    match finish.as_ref().and_then(Option::as_ref) {
                        Some(finished) if requested_close && finished.outcome.code() == Some(0) => {}
                        Some(_) if requested_close => terminal_failure = true,
                        Some(_) => {
                            unexpected_exit = true;
                            terminal_failure = true;
                            forced = true;
                        }
                        None => {
                            terminal_failure = true;
                            forced = true;
                        }
                    }
                    break;
                }
                event = pump_rx.recv() => {
                    if observe_pump(event, &events, &shared, profile.limits.max_stderr_bytes,
                        &mut stderr_retained, &mut stdout_closed, &mut stderr_closed) {
                        forced = true;
                        requested_close = true;
                        terminal_failure = true;
                    }
                }
                status = stdin_status_rx.recv(), if stdin_status_open => match status {
                    Some(StdinStatus::Failed) => {
                        stdin_status_open = false;
                        terminal_failure = true;
                        forced = true;
                        requested_close = true;
                    }
                    Some(StdinStatus::Closed) | None => stdin_status_open = false,
                },
                _ = resource_tick.tick() => sample_process_resources(&group, &shared),
                _ = tokio::time::sleep(Duration::from_millis(5)) => {
                    let leader_state = match group.members() {
                        Ok(members) if !members.contains(&leader_pid) => {
                            Ok((false, !members.is_empty()))
                        }
                        Ok(members) => group.members_info().map(|infos| {
                            (
                                infos.iter().any(|member| member.pid() == leader_pid),
                                !members.is_empty(),
                            )
                        }),
                        Err(error) => Err(error),
                    };
                    match leader_state {
                        Ok((true, _)) => {}
                        Ok((false, group_nonempty)) => {
                            if !requested_close {
                                unexpected_exit = true;
                                terminal_failure = true;
                                forced = true;
                            } else if group_nonempty {
                                forced = true;
                            }
                            break;
                        }
                        Err(_) => {
                            terminal_failure = true;
                            forced = true;
                            break;
                        }
                    }
                }
            }
            if let Some(deadline) = close_deadline
                && tokio::time::Instant::now() >= deadline
            {
                forced = true;
                break;
            }
        }
        if !requested_close {
            let mut usage = shared.usage.lock().expect("semantic usage mutex");
            let SemanticProcessUsageV1 { crashes, saturated, .. } = &mut *usage;
            inc(crashes, saturated);
        }
        let mut kill_succeeded = true;
        let finish_deadline = tokio::time::Instant::now() + profile.deadlines.kill_and_reap;
        let mut kill_rx = None;
        if forced {
            stdin_task.abort();
            let kill_group = Arc::clone(&group);
            let (result_tx, result_rx) = mpsc::sync_channel(1);
            let _ = thread::Builder::new()
                .name("vellum-semantic-kill".into())
                .spawn(move || {
                    let _ = result_tx.send(kill_group.kill_all().is_ok());
                });
            kill_rx = Some(result_rx);
        } else if !requested_close {
            stdin_task.abort();
        }
        drop(stdin_tx);
        let mut kill_result = (!forced).then_some(true);
        let mut event_drain_done = false;
        while finish.is_none() || kill_result.is_none() || !event_drain_done {
            if tokio::time::Instant::now() >= finish_deadline {
                finish_task.abort();
                event_drain.abort();
                terminal_failure = true;
                break;
            }
            if kill_result.is_none()
                && let Some(receiver) = kill_rx.as_ref()
            {
                match receiver.try_recv() {
                    Ok(result) => kill_result = Some(result),
                    Err(TryRecvError::Disconnected) => kill_result = Some(false),
                    Err(TryRecvError::Empty) => {}
                }
            }
            tokio::select! {
                result = &mut finish_task, if finish.is_none() => {
                    finish = Some(result.ok().and_then(Result::ok));
                    while let Ok(event) = pump_rx.try_recv() {
                        if observe_pump(Some(event), &events, &shared, profile.limits.max_stderr_bytes,
                            &mut stderr_retained, &mut stdout_closed, &mut stderr_closed) {
                            terminal_failure = true;
                        }
                    }
                }
                event = pump_rx.recv() => {
                    if observe_pump(event, &events, &shared, profile.limits.max_stderr_bytes,
                        &mut stderr_retained, &mut stdout_closed, &mut stderr_closed) {
                        terminal_failure = true;
                    }
                }
                result = &mut event_drain, if !event_drain_done => {
                    event_drain_done = true;
                    if result.is_err() {
                        terminal_failure = true;
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(5)) => {}
            }
        }
        if forced {
            kill_succeeded = kill_result.unwrap_or(false);
            let mut usage = shared.usage.lock().expect("semantic usage mutex");
            if kill_succeeded {
                let SemanticProcessUsageV1 { group_hard_kills_accepted, saturated, .. } = &mut *usage;
                inc(group_hard_kills_accepted, saturated);
            } else {
                let SemanticProcessUsageV1 { group_kill_failures, saturated, .. } = &mut *usage;
                inc(group_kill_failures, saturated);
                terminal_failure = true;
            }
        }
        shared.live.store(false, Ordering::Release);
        shared.queued_stdin_bytes.store(0, Ordering::Release);
        let mut usage = shared.usage.lock().expect("semantic usage mutex");
        match finish {
            Some(Some(ref finished)) => {
                let SemanticProcessUsageV1 { leader_reaped, saturated, .. } = &mut *usage;
                inc(leader_reaped, saturated);
                if finished.outcome.code() != Some(0) && (!forced || unexpected_exit) {
                    let SemanticProcessUsageV1 { nonzero_exits, saturated, .. } = &mut *usage;
                    inc(nonzero_exits, saturated);
                }
            }
            _ => {
                let SemanticProcessUsageV1 { leader_finish_failures, saturated, .. } = &mut *usage;
                inc(leader_finish_failures, saturated);
                terminal_failure = true;
            }
        }
        if requested_close
            && !forced
            && !terminal_failure
            && finish.as_ref().and_then(Option::as_ref).is_some_and(|finished| finished.outcome.code() == Some(0))
        {
            let SemanticProcessUsageV1 { graceful_shutdowns, saturated, .. } = &mut *usage;
            inc(graceful_shutdowns, saturated);
        }
        drop(usage);
        let cleanup_confirmed = kill_succeeded
            && finish
                .as_ref()
                .and_then(Option::as_ref)
                .is_some();
        shared.terminal.store(
            if terminal_failure || !kill_succeeded {
                TERMINAL_FAILED
            } else {
                TERMINAL_CLOSED
            },
            Ordering::Release,
        );
        if cleanup_confirmed {
            SemanticUnresolvedLaunchDispositionV1::CleanupConfirmed
        } else {
            SemanticUnresolvedLaunchDispositionV1::RestartRequired
        }
    })
}

#[cfg(test)]
fn reset_launch_fence_after_terminal_for_test() {
    let worker = SEMANTIC_UNRESOLVED_LAUNCH
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    if let Some(state) = worker {
        let _ = state.worker.join();
    }
    SEMANTIC_LAUNCH_FENCED.store(false, Ordering::Release);
}

fn observe_pump(
    event: Option<PumpEvent>,
    events: &SyncSender<Event>,
    shared: &SharedState,
    max_stderr_bytes: usize,
    stderr_retained: &mut usize,
    stdout_closed: &mut bool,
    stderr_closed: &mut bool,
) -> bool {
    match event {
        Some(PumpEvent::Stdout(bytes)) => {
            let length = bytes.len();
            let delivered = events.try_send(Event::Stdout(bytes)).is_ok();
            let mut usage = shared.usage.lock().expect("semantic usage mutex");
            let SemanticProcessUsageV1 {
                stdout_bytes,
                stdout_dropped_bytes,
                saturated,
                ..
            } = &mut *usage;
            if delivered {
                add(stdout_bytes, length, saturated);
            } else {
                add(stdout_dropped_bytes, length, saturated);
            }
            !delivered
        }
        Some(PumpEvent::Stderr(bytes)) => {
            let length = bytes.len();
            let admitted = max_stderr_bytes
                .saturating_sub(*stderr_retained)
                .min(length);
            *stderr_retained = stderr_retained.saturating_add(admitted);
            let mut usage = shared.usage.lock().expect("semantic usage mutex");
            let SemanticProcessUsageV1 {
                stderr_bytes,
                stderr_dropped_bytes,
                saturated,
                ..
            } = &mut *usage;
            add(stderr_bytes, admitted, saturated);
            add(stderr_dropped_bytes, length - admitted, saturated);
            false
        }
        Some(PumpEvent::StdoutClosed) => {
            *stdout_closed = true;
            false
        }
        Some(PumpEvent::StderrClosed) => {
            *stderr_closed = true;
            false
        }
        None => {
            *stdout_closed = true;
            *stderr_closed = true;
            false
        }
    }
}

async fn cleanup_owned_launch(
    group: Arc<ProcessGroup>,
    run: RunningProcess,
    process_events: Option<ProcessEvents>,
    pump_rx: &mut async_mpsc::Receiver<PumpEvent>,
    deadline: Duration,
) -> bool {
    let (kill_tx, kill_rx) = mpsc::sync_channel(1);
    let kill_group = Arc::clone(&group);
    if thread::Builder::new()
        .name("vellum-semantic-launch-kill".into())
        .spawn(move || {
            let _ = kill_tx.send(kill_group.kill_all().is_ok());
        })
        .is_err()
    {
        return false;
    }
    let mut finish_task = tokio::spawn(async move { run.finish().await });
    let mut event_task = process_events
        .map(|mut events| tokio::spawn(async move { while events.next().await.is_some() {} }));
    let expires = tokio::time::Instant::now() + deadline;
    let mut kill_result = None;
    let mut finish_result = None;
    let mut event_done = event_task.is_none();
    let mut pump_open = true;
    while kill_result.is_none() || finish_result.is_none() || !event_done {
        if tokio::time::Instant::now() >= expires {
            finish_task.abort();
            if let Some(task) = event_task.as_ref() {
                task.abort();
            }
            return false;
        }
        if kill_result.is_none() {
            match kill_rx.try_recv() {
                Ok(result) => kill_result = Some(result),
                Err(TryRecvError::Disconnected) => kill_result = Some(false),
                Err(TryRecvError::Empty) => {}
            }
        }
        tokio::select! {
            result = &mut finish_task, if finish_result.is_none() => {
                finish_result = Some(matches!(result, Ok(Ok(_))));
                while pump_rx.try_recv().is_ok() {}
            }
            result = async { event_task.as_mut().expect("guarded launch event task").await }, if !event_done => {
                event_done = true;
                if result.is_err() {
                    return false;
                }
            }
            event = pump_rx.recv(), if pump_open => {
                pump_open = event.is_some();
            }
            _ = tokio::time::sleep(Duration::from_millis(5)) => {}
        }
    }
    kill_result == Some(true) && finish_result == Some(true)
}

fn inc(value: &mut u64, saturated: &mut bool) {
    let next = value.saturating_add(1);
    *saturated |= next == u64::MAX;
    *value = next;
}

fn add(value: &mut u64, amount: usize, saturated: &mut bool) {
    let amount = u64::try_from(amount).unwrap_or(u64::MAX);
    let next = value.saturating_add(amount);
    *saturated |= next == u64::MAX;
    *value = next;
}

fn bounded_observation(value: u128, saturated: &mut bool) -> u64 {
    match u64::try_from(value) {
        Ok(value) => value,
        Err(_) => {
            *saturated = true;
            u64::MAX
        }
    }
}

fn record_resource_sample(
    usage: &mut SemanticProcessUsageV1,
    sample: Option<SemanticProcessResourceSampleV1>,
) {
    let SemanticProcessUsageV1 {
        resource_samples,
        resource_sample_failures,
        active_process_units_latest,
        active_process_units_high_water,
        cpu_time_observations,
        cpu_time_nanos_high_water,
        peak_memory_observations,
        peak_memory_bytes_high_water,
        io_read_observations,
        io_read_bytes_high_water,
        io_write_observations,
        io_write_bytes_high_water,
        peak_process_observations,
        peak_process_units_high_water,
        saturated,
        ..
    } = usage;
    let Some(sample) = sample else {
        inc(resource_sample_failures, saturated);
        return;
    };
    inc(resource_samples, saturated);
    let active = bounded_observation(sample.active_process_units as u128, saturated);
    *active_process_units_latest = active;
    *active_process_units_high_water = (*active_process_units_high_water).max(active);
    if let Some(value) = sample.cpu_time_nanos {
        inc(cpu_time_observations, saturated);
        let value = bounded_observation(value, saturated);
        *cpu_time_nanos_high_water = (*cpu_time_nanos_high_water).max(value);
    }
    if let Some(value) = sample.peak_memory_bytes {
        inc(peak_memory_observations, saturated);
        *peak_memory_bytes_high_water = (*peak_memory_bytes_high_water).max(value);
    }
    if let Some(value) = sample.io_read_bytes {
        inc(io_read_observations, saturated);
        *io_read_bytes_high_water = (*io_read_bytes_high_water).max(value);
    }
    if let Some(value) = sample.io_write_bytes {
        inc(io_write_observations, saturated);
        *io_write_bytes_high_water = (*io_write_bytes_high_water).max(value);
    }
    if let Some(value) = sample.peak_process_units {
        inc(peak_process_observations, saturated);
        let value = bounded_observation(value as u128, saturated);
        *peak_process_units_high_water = (*peak_process_units_high_water).max(value);
    }
}

fn sample_process_resources(group: &ProcessGroup, shared: &SharedState) {
    let sample = group
        .stats()
        .ok()
        .map(SemanticProcessResourceSampleV1::from);
    let mut usage = shared.usage.lock().expect("semantic usage mutex");
    record_resource_sample(&mut usage, sample);
}

#[cfg(test)]
#[path = "tests/supervised_matrix.rs"]
mod supervised_matrix_tests;
#[cfg(test)]
mod tests;
