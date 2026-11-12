//! Closed provider-neutral protocol for one local semantic session.
//!
//! Correlation values in this crate are never authentication or authority.
//! A composition authenticates a subject separately, intersects requested
//! roles with configured grants, and installs only that meet. Every post-join
//! command binds the exact daemon incarnation and membership. The protocol is
//! volatile across restart: a fresh incarnation requires a complete bounded
//! synchronization epoch before query readiness may be published.
#![forbid(unsafe_code)]

use sha2::{Digest, Sha256};
use std::fmt;
use std::sync::Arc;
use vellum_host::semantic::{
    MAX_SEMANTIC_PROVIDER_PROFILE_BYTES_V1, MAX_SEMANTIC_PROVIDER_VERSION_BYTES_V1,
    SemanticCapabilitySetV1, SemanticDocumentRefV1, SemanticIndexGenerationV1, SemanticLimitsV1,
    SemanticQueryKindV1, SemanticRefusalV1, SemanticReplyV1, SemanticRequestRefV1,
    SemanticRequestV1, SemanticTargetV1,
};

pub const MAX_SEMANTIC_SESSION_SNAPSHOT_BYTES_V1: usize = 4 * 1024 * 1024;
pub const MAX_SEMANTIC_SESSION_LANGUAGE_BYTES_V1: usize = 512;
pub const MAX_SEMANTIC_SESSION_DOCUMENTS_V1: u32 = 4_096;
pub const MAX_SEMANTIC_SESSION_PENDING_V1: u32 = 1_024;
pub const MAX_SEMANTIC_SESSION_OPERATIONS_PER_MEMBERSHIP_V1: u32 = 65_536;
pub const MAX_SEMANTIC_SESSION_MEMBERSHIPS_V1: u32 = 64;
pub const MAX_SEMANTIC_SESSION_REPLAY_INTENT_BYTES_V1: usize = 128 * 1024 * 1024;
pub const MAX_SEMANTIC_SESSION_RETAINED_RESULT_BYTES_V1: usize = 64 * 1024 * 1024;
/// Logical replay bytes retained by one fresh `JoinSession` operation.
pub const SEMANTIC_SESSION_JOIN_INTENT_BYTES_V1: usize = 1 + 16 + 16 + 1;
/// Logical replay bytes retained by one fresh `LeaveSession` operation.
pub const SEMANTIC_SESSION_LEAVE_INTENT_BYTES_V1: usize = 1 + 16 + 16 + 16;

macro_rules! opaque_ref {
    ($name:ident) => {
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 16]);

        impl $name {
            pub fn new(bytes: [u8; 16]) -> Option<Self> {
                if bytes == [0; 16] {
                    None
                } else {
                    Some(Self(bytes))
                }
            }

            /// Canonical correlation bytes. Possession conveys no authority.
            pub const fn as_bytes(self) -> [u8; 16] {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "(<opaque>)"))
            }
        }
    };
}

opaque_ref!(SemanticSessionIncarnationV1);
opaque_ref!(SemanticMembershipRefV1);
opaque_ref!(SemanticExchangeRefV1);
opaque_ref!(SemanticSessionOperationRefV1);
opaque_ref!(SemanticSynchronizationEpochV1);

/// Independent finite role product. `meet` is attenuation. Only trusted
/// composition may join configured grants; wire-requested roles are requests.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticSessionRolesV1(u8);

impl SemanticSessionRolesV1 {
    const SYNCHRONIZE: u8 = 1;
    const QUERY: u8 = 2;
    const ALL: u8 = Self::SYNCHRONIZE | Self::QUERY;

    pub const fn bottom() -> Self {
        Self(0)
    }
    pub const fn top() -> Self {
        Self(Self::ALL)
    }
    pub const fn synchronize() -> Self {
        Self(Self::SYNCHRONIZE)
    }
    pub const fn query() -> Self {
        Self(Self::QUERY)
    }
    pub const fn meet(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
    pub const fn join_trusted(self, other: Self) -> Self {
        Self((self.0 | other.0) & Self::ALL)
    }
    pub const fn contains(self, required: Self) -> bool {
        self.0 & required.0 == required.0
    }
    pub const fn bits(self) -> u8 {
        self.0
    }
    pub const fn from_bits(bits: u8) -> Option<Self> {
        if bits & !Self::ALL == 0 {
            Some(Self(bits))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticSessionLimitsV1 {
    pub max_snapshot_bytes: usize,
    pub max_total_snapshot_bytes: usize,
    pub max_language_bytes: usize,
    pub max_documents: u32,
    pub max_pending: u32,
    pub max_memberships: u32,
    /// Aggregate exact canonical mutation intent retained for replay.
    pub max_replay_intent_bytes: usize,
    /// Aggregate result reservation retained from Submit through Release.
    pub max_retained_result_bytes: usize,
    /// Lifetime issued-request/tombstone budget. Request references are never
    /// reusable within a membership, including after result release.
    pub max_requests_per_membership: u32,
    /// Exact lifetime replay budget. Records are never silently evicted or
    /// acknowledged by random operation reference. Exhaustion requires leave
    /// and rejoin; a synchronizer then performs a fresh complete epoch.
    pub max_operations_per_membership: u32,
    pub semantic: SemanticLimitsV1,
}

impl SemanticSessionLimitsV1 {
    pub const fn reference() -> Self {
        Self {
            max_snapshot_bytes: MAX_SEMANTIC_SESSION_SNAPSHOT_BYTES_V1,
            max_total_snapshot_bytes: 64 * 1024 * 1024,
            max_language_bytes: MAX_SEMANTIC_SESSION_LANGUAGE_BYTES_V1,
            max_documents: MAX_SEMANTIC_SESSION_DOCUMENTS_V1,
            max_pending: MAX_SEMANTIC_SESSION_PENDING_V1,
            max_memberships: MAX_SEMANTIC_SESSION_MEMBERSHIPS_V1,
            max_replay_intent_bytes: MAX_SEMANTIC_SESSION_REPLAY_INTENT_BYTES_V1,
            max_retained_result_bytes: MAX_SEMANTIC_SESSION_RETAINED_RESULT_BYTES_V1,
            max_requests_per_membership: MAX_SEMANTIC_SESSION_OPERATIONS_PER_MEMBERSHIP_V1,
            max_operations_per_membership: MAX_SEMANTIC_SESSION_OPERATIONS_PER_MEMBERSHIP_V1,
            semantic: SemanticLimitsV1::reference(),
        }
    }

    pub const fn is_valid(self) -> bool {
        self.max_snapshot_bytes > 0
            && self.max_snapshot_bytes <= MAX_SEMANTIC_SESSION_SNAPSHOT_BYTES_V1
            && self.max_total_snapshot_bytes >= self.max_snapshot_bytes
            && self.max_total_snapshot_bytes <= 64 * 1024 * 1024
            && self.max_language_bytes > 0
            && self.max_language_bytes <= MAX_SEMANTIC_SESSION_LANGUAGE_BYTES_V1
            && self.max_language_bytes <= self.semantic.max_label_bytes
            && self.max_documents > 0
            && self.max_documents <= MAX_SEMANTIC_SESSION_DOCUMENTS_V1
            && self.max_pending > 0
            && self.max_pending <= MAX_SEMANTIC_SESSION_PENDING_V1
            && self.max_memberships > 0
            && self.max_memberships <= MAX_SEMANTIC_SESSION_MEMBERSHIPS_V1
            && self.max_replay_intent_bytes > 0
            && self.max_replay_intent_bytes <= MAX_SEMANTIC_SESSION_REPLAY_INTENT_BYTES_V1
            && self.max_retained_result_bytes > 0
            && self.max_retained_result_bytes <= MAX_SEMANTIC_SESSION_RETAINED_RESULT_BYTES_V1
            && self.max_requests_per_membership > 0
            && self.max_requests_per_membership <= MAX_SEMANTIC_SESSION_OPERATIONS_PER_MEMBERSHIP_V1
            && self.ordinary_operation_budget().is_some()
            && self.max_operations_per_membership
                <= MAX_SEMANTIC_SESSION_OPERATIONS_PER_MEMBERSHIP_V1
            && self.has_minimum_membership_replay_bytes()
            && self.semantic.is_valid()
    }

    /// Fresh leave operations reserved for cancelling every pending request
    /// and closing every synchronized document without reusing correlations.
    pub const fn retirement_operation_reserve(self) -> Option<u32> {
        self.max_documents.checked_add(self.max_pending)
    }

    /// Operation headroom advertised for synchronization and query work. The
    /// retirement tail is never offered as ordinary capacity.
    pub const fn ordinary_operation_budget(self) -> Option<u32> {
        match self.retirement_operation_reserve() {
            Some(reserve) => match self.max_operations_per_membership.checked_sub(reserve) {
                Some(0) | None => None,
                budget => budget,
            },
            None => None,
        }
    }

    /// Minimum replay space for every declared concurrent membership to join
    /// and then consume its complete retirement tail.
    pub const fn minimum_membership_replay_bytes(self) -> Option<usize> {
        let reserve = match self.retirement_operation_reserve() {
            Some(value) => value as usize,
            None => return None,
        };
        let retirement = match reserve.checked_mul(SEMANTIC_SESSION_LEAVE_INTENT_BYTES_V1) {
            Some(value) => value,
            None => return None,
        };
        match retirement.checked_add(SEMANTIC_SESSION_JOIN_INTENT_BYTES_V1) {
            Some(value) => value.checked_mul(self.max_memberships as usize),
            None => None,
        }
    }

    const fn has_minimum_membership_replay_bytes(self) -> bool {
        match self.minimum_membership_replay_bytes() {
            Some(bytes) => bytes <= self.max_replay_intent_bytes,
            None => false,
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct SemanticSessionSnapshotV1 {
    target: SemanticTargetV1,
    language: Arc<str>,
    source: Arc<str>,
}

impl SemanticSessionSnapshotV1 {
    pub fn try_new(
        target: SemanticTargetV1,
        language: String,
        source: String,
        limits: SemanticSessionLimitsV1,
    ) -> Option<Self> {
        (limits.is_valid()
            && !language.is_empty()
            && language.len() <= limits.max_language_bytes
            && source.len() <= limits.max_snapshot_bytes)
            .then_some(Self {
                target,
                language: Arc::from(language),
                source: Arc::from(source),
            })
    }

    pub const fn target(&self) -> SemanticTargetV1 {
        self.target
    }
    pub fn language(&self) -> &str {
        &self.language
    }
    pub fn source(&self) -> &str {
        &self.source
    }
}

impl fmt::Debug for SemanticSessionSnapshotV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SemanticSessionSnapshotV1")
            .field("target", &self.target)
            .field("language_bytes", &self.language.len())
            .field("source_bytes", &self.source.len())
            .finish()
    }
}

/// Canonical manifest commitment for one complete synchronization epoch. Its
/// equality is the digest-induced quotient under the SHA-256 collision-
/// resistance assumption, not literal source equality, authority, replay
/// identity or proof of current files.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SemanticSynchronizationManifestV1 {
    documents: u32,
    source_bytes: u64,
    digest: [u8; 32],
}

/// Bounded source-free synchronization evidence retained by the broker. The
/// leaf digest commits to exact document, revision, language and source bytes;
/// Debug intentionally reveals none of them.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SemanticSynchronizationDocumentSealV1 {
    target: SemanticTargetV1,
    source_bytes: u64,
    leaf: [u8; 32],
}

impl SemanticSynchronizationDocumentSealV1 {
    pub fn try_from_snapshot(
        snapshot: &SemanticSessionSnapshotV1,
        limits: SemanticSessionLimitsV1,
    ) -> Option<Self> {
        if !limits.is_valid()
            || snapshot.source().len() > limits.max_snapshot_bytes
            || snapshot.language().len() > limits.max_language_bytes
        {
            return None;
        }
        let mut digest = Sha256::new();
        digest.update(b"vellum-semantic-synchronization-document-leaf-v1\0");
        digest.update(snapshot.target().document.as_bytes());
        digest.update(snapshot.target().source_revision.get().to_be_bytes());
        digest.update((snapshot.language().len() as u32).to_be_bytes());
        digest.update(snapshot.language().as_bytes());
        digest.update((snapshot.source().len() as u64).to_be_bytes());
        digest.update(snapshot.source().as_bytes());
        Some(Self {
            target: snapshot.target(),
            source_bytes: snapshot.source().len() as u64,
            leaf: digest.finalize().into(),
        })
    }

    pub const fn document(self) -> SemanticDocumentRefV1 {
        self.target.document
    }
    pub const fn target(self) -> SemanticTargetV1 {
        self.target
    }
    pub const fn source_bytes(self) -> u64 {
        self.source_bytes
    }
}

impl fmt::Debug for SemanticSynchronizationDocumentSealV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SemanticSynchronizationDocumentSealV1")
            .field("document", &"<redacted>")
            .field("source_bytes", &"<redacted>")
            .field("leaf", &"<redacted>")
            .finish()
    }
}

impl SemanticSynchronizationManifestV1 {
    pub const fn new(
        documents: u32,
        source_bytes: u64,
        digest: [u8; 32],
        limits: SemanticSessionLimitsV1,
    ) -> Option<Self> {
        if limits.is_valid()
            && documents <= limits.max_documents
            && source_bytes <= limits.max_total_snapshot_bytes as u64
        {
            Some(Self {
                documents,
                source_bytes,
                digest,
            })
        } else {
            None
        }
    }

    /// Domain-separated seal over exact snapshots sorted by document identity.
    /// Duplicate documents and aggregate overflow refuse. The digest is
    /// private linkable evidence and must never enter errors or metric labels.
    pub fn try_from_snapshots(
        snapshots: &[SemanticSessionSnapshotV1],
        limits: SemanticSessionLimitsV1,
    ) -> Option<Self> {
        if !limits.is_valid() || snapshots.len() > limits.max_documents as usize {
            return None;
        }
        let seals: Option<Vec<_>> = snapshots
            .iter()
            .map(|snapshot| {
                SemanticSynchronizationDocumentSealV1::try_from_snapshot(snapshot, limits)
            })
            .collect();
        Self::try_from_seals(&seals?, limits)
    }

    /// Compositional root over source-free fixed-width leaves sorted by exact
    /// document identity. This permits verification without a second source
    /// store beside the sovereign semantic service.
    pub fn try_from_seals(
        seals: &[SemanticSynchronizationDocumentSealV1],
        limits: SemanticSessionLimitsV1,
    ) -> Option<Self> {
        if !limits.is_valid() || seals.len() > limits.max_documents as usize {
            return None;
        }
        let mut ordered = seals.to_vec();
        ordered.sort_by_key(|seal| seal.target.document);
        if ordered
            .windows(2)
            .any(|pair| pair[0].target.document == pair[1].target.document)
        {
            return None;
        }
        let mut source_bytes = 0u64;
        let mut digest = Sha256::new();
        digest.update(b"vellum-semantic-synchronization-manifest-root-v1\0");
        digest.update((ordered.len() as u32).to_be_bytes());
        for seal in ordered {
            source_bytes = source_bytes.checked_add(seal.source_bytes)?;
            if source_bytes > limits.max_total_snapshot_bytes as u64 {
                return None;
            }
            digest.update(seal.target.document.as_bytes());
            digest.update(seal.leaf);
        }
        digest.update(source_bytes.to_be_bytes());
        Self::new(
            u32::try_from(seals.len()).ok()?,
            source_bytes,
            digest.finalize().into(),
            limits,
        )
    }

    pub const fn documents(self) -> u32 {
        self.documents
    }
    pub const fn source_bytes(self) -> u64 {
        self.source_bytes
    }
    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }
}

impl fmt::Debug for SemanticSynchronizationManifestV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SemanticSynchronizationManifestV1")
            .field("documents", &"<redacted>")
            .field("source_bytes", &"<redacted>")
            .field("digest", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticSessionCommandV1 {
    Describe {
        exchange: SemanticExchangeRefV1,
    },
    JoinSession {
        exchange: SemanticExchangeRefV1,
        operation: SemanticSessionOperationRefV1,
        incarnation: SemanticSessionIncarnationV1,
        requested_roles: SemanticSessionRolesV1,
    },
    BeginSynchronization {
        exchange: SemanticExchangeRefV1,
        operation: SemanticSessionOperationRefV1,
        incarnation: SemanticSessionIncarnationV1,
        membership: SemanticMembershipRefV1,
        epoch: SemanticSynchronizationEpochV1,
        manifest: SemanticSynchronizationManifestV1,
    },
    SyncOpen {
        exchange: SemanticExchangeRefV1,
        operation: SemanticSessionOperationRefV1,
        incarnation: SemanticSessionIncarnationV1,
        membership: SemanticMembershipRefV1,
        epoch: SemanticSynchronizationEpochV1,
        snapshot: SemanticSessionSnapshotV1,
    },
    SyncChange {
        exchange: SemanticExchangeRefV1,
        operation: SemanticSessionOperationRefV1,
        incarnation: SemanticSessionIncarnationV1,
        membership: SemanticMembershipRefV1,
        epoch: SemanticSynchronizationEpochV1,
        snapshot: SemanticSessionSnapshotV1,
    },
    SyncClose {
        exchange: SemanticExchangeRefV1,
        operation: SemanticSessionOperationRefV1,
        incarnation: SemanticSessionIncarnationV1,
        membership: SemanticMembershipRefV1,
        epoch: SemanticSynchronizationEpochV1,
        target: SemanticTargetV1,
    },
    CompleteSynchronization {
        exchange: SemanticExchangeRefV1,
        operation: SemanticSessionOperationRefV1,
        incarnation: SemanticSessionIncarnationV1,
        membership: SemanticMembershipRefV1,
        epoch: SemanticSynchronizationEpochV1,
        manifest: SemanticSynchronizationManifestV1,
    },
    Submit {
        exchange: SemanticExchangeRefV1,
        operation: SemanticSessionOperationRefV1,
        incarnation: SemanticSessionIncarnationV1,
        membership: SemanticMembershipRefV1,
        request: SemanticRequestV1,
    },
    Cancel {
        exchange: SemanticExchangeRefV1,
        operation: SemanticSessionOperationRefV1,
        incarnation: SemanticSessionIncarnationV1,
        membership: SemanticMembershipRefV1,
        request: SemanticRequestRefV1,
    },
    Collect {
        exchange: SemanticExchangeRefV1,
        incarnation: SemanticSessionIncarnationV1,
        membership: SemanticMembershipRefV1,
        request: SemanticRequestRefV1,
    },
    ReleaseRequest {
        exchange: SemanticExchangeRefV1,
        operation: SemanticSessionOperationRefV1,
        incarnation: SemanticSessionIncarnationV1,
        membership: SemanticMembershipRefV1,
        request: SemanticRequestRefV1,
    },
    LeaveSession {
        exchange: SemanticExchangeRefV1,
        operation: SemanticSessionOperationRefV1,
        incarnation: SemanticSessionIncarnationV1,
        membership: SemanticMembershipRefV1,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SemanticSessionCommandKindV1 {
    Describe = 1,
    JoinSession = 2,
    BeginSynchronization = 3,
    SyncOpen = 4,
    SyncChange = 5,
    SyncClose = 6,
    CompleteSynchronization = 7,
    Submit = 8,
    Cancel = 9,
    Collect = 10,
    ReleaseRequest = 11,
    LeaveSession = 12,
}

impl SemanticSessionCommandV1 {
    /// Checked logical bytes retained for exact mutation replay. This metric
    /// counts every closed-domain field and length-prefixed text byte, excludes
    /// the replaceable exchange correlation, and is neither heap usage nor a
    /// particular wire encoding.
    pub fn retained_intent_bytes_v1(&self) -> Option<usize> {
        const TAG: usize = 1;
        const REF: usize = 16;
        const TARGET: usize = REF + 8;
        const MANIFEST: usize = 4 + 8 + 32;
        const BOUND_MUTATION: usize = TAG + REF + REF + REF;
        let text = |value: &str| 8usize.checked_add(value.len());
        let query = |value: &vellum_host::semantic::SemanticQueryV1| {
            use vellum_host::semantic::SemanticQueryV1 as Q;
            match value {
                Q::Symbols { needle } => TAG.checked_add(text(needle.as_str())?),
                Q::Definition { .. }
                | Q::References { .. }
                | Q::Implementations { .. }
                | Q::IncomingCalls { .. }
                | Q::OutgoingCalls { .. }
                | Q::TypeAtPoint { .. } => TAG.checked_add(8),
                Q::Diagnostics => Some(TAG),
                Q::PlanRename { new_name, .. } => {
                    TAG.checked_add(8)?.checked_add(text(new_name.as_str())?)
                }
            }
        };
        match self {
            Self::Describe { .. } | Self::Collect { .. } => Some(0),
            Self::JoinSession { .. } => Some(SEMANTIC_SESSION_JOIN_INTENT_BYTES_V1),
            Self::BeginSynchronization { .. } | Self::CompleteSynchronization { .. } => {
                BOUND_MUTATION.checked_add(REF)?.checked_add(MANIFEST)
            }
            Self::SyncOpen { snapshot, .. } | Self::SyncChange { snapshot, .. } => BOUND_MUTATION
                .checked_add(REF)?
                .checked_add(TARGET)?
                .checked_add(text(snapshot.language())?)?
                .checked_add(text(snapshot.source())?),
            Self::SyncClose { .. } => BOUND_MUTATION.checked_add(REF)?.checked_add(TARGET),
            Self::Submit { request, .. } => BOUND_MUTATION
                .checked_add(REF)?
                .checked_add(TARGET)?
                .checked_add(1 + usize::from(request.expected_index.is_some()) * 8)?
                .checked_add(text(request.language.as_str())?)?
                .checked_add(query(&request.query)?),
            Self::Cancel { .. } | Self::ReleaseRequest { .. } => BOUND_MUTATION.checked_add(REF),
            Self::LeaveSession { .. } => Some(SEMANTIC_SESSION_LEAVE_INTENT_BYTES_V1),
        }
    }

    pub const fn kind(&self) -> SemanticSessionCommandKindV1 {
        match self {
            Self::Describe { .. } => SemanticSessionCommandKindV1::Describe,
            Self::JoinSession { .. } => SemanticSessionCommandKindV1::JoinSession,
            Self::BeginSynchronization { .. } => SemanticSessionCommandKindV1::BeginSynchronization,
            Self::SyncOpen { .. } => SemanticSessionCommandKindV1::SyncOpen,
            Self::SyncChange { .. } => SemanticSessionCommandKindV1::SyncChange,
            Self::SyncClose { .. } => SemanticSessionCommandKindV1::SyncClose,
            Self::CompleteSynchronization { .. } => {
                SemanticSessionCommandKindV1::CompleteSynchronization
            }
            Self::Submit { .. } => SemanticSessionCommandKindV1::Submit,
            Self::Cancel { .. } => SemanticSessionCommandKindV1::Cancel,
            Self::Collect { .. } => SemanticSessionCommandKindV1::Collect,
            Self::ReleaseRequest { .. } => SemanticSessionCommandKindV1::ReleaseRequest,
            Self::LeaveSession { .. } => SemanticSessionCommandKindV1::LeaveSession,
        }
    }
    pub const fn exchange(&self) -> SemanticExchangeRefV1 {
        match self {
            Self::Describe { exchange }
            | Self::JoinSession { exchange, .. }
            | Self::BeginSynchronization { exchange, .. }
            | Self::SyncOpen { exchange, .. }
            | Self::SyncChange { exchange, .. }
            | Self::SyncClose { exchange, .. }
            | Self::CompleteSynchronization { exchange, .. }
            | Self::Submit { exchange, .. }
            | Self::Cancel { exchange, .. }
            | Self::Collect { exchange, .. }
            | Self::ReleaseRequest { exchange, .. }
            | Self::LeaveSession { exchange, .. } => *exchange,
        }
    }

    pub const fn operation(&self) -> Option<SemanticSessionOperationRefV1> {
        match self {
            Self::Describe { .. } | Self::Collect { .. } => None,
            Self::JoinSession { operation, .. }
            | Self::BeginSynchronization { operation, .. }
            | Self::SyncOpen { operation, .. }
            | Self::SyncChange { operation, .. }
            | Self::SyncClose { operation, .. }
            | Self::CompleteSynchronization { operation, .. }
            | Self::Submit { operation, .. }
            | Self::Cancel { operation, .. }
            | Self::ReleaseRequest { operation, .. }
            | Self::LeaveSession { operation, .. } => Some(*operation),
        }
    }

    pub const fn required_roles(&self) -> SemanticSessionRolesV1 {
        match self {
            Self::BeginSynchronization { .. }
            | Self::SyncOpen { .. }
            | Self::SyncChange { .. }
            | Self::SyncClose { .. }
            | Self::CompleteSynchronization { .. } => SemanticSessionRolesV1::synchronize(),
            Self::Submit { .. }
            | Self::Cancel { .. }
            | Self::Collect { .. }
            | Self::ReleaseRequest { .. } => SemanticSessionRolesV1::query(),
            Self::Describe { .. } | Self::JoinSession { .. } | Self::LeaveSession { .. } => {
                SemanticSessionRolesV1::bottom()
            }
        }
    }

    pub const fn binding(&self) -> Option<(SemanticSessionIncarnationV1, SemanticMembershipRefV1)> {
        match self {
            Self::Describe { .. } | Self::JoinSession { .. } => None,
            Self::BeginSynchronization {
                incarnation,
                membership,
                ..
            }
            | Self::SyncOpen {
                incarnation,
                membership,
                ..
            }
            | Self::SyncChange {
                incarnation,
                membership,
                ..
            }
            | Self::SyncClose {
                incarnation,
                membership,
                ..
            }
            | Self::CompleteSynchronization {
                incarnation,
                membership,
                ..
            }
            | Self::Submit {
                incarnation,
                membership,
                ..
            }
            | Self::Cancel {
                incarnation,
                membership,
                ..
            }
            | Self::Collect {
                incarnation,
                membership,
                ..
            }
            | Self::ReleaseRequest {
                incarnation,
                membership,
                ..
            }
            | Self::LeaveSession {
                incarnation,
                membership,
                ..
            } => Some((*incarnation, *membership)),
        }
    }
}

/// Checked worst-case logical terminal reply bytes reserved from Submit until
/// Release. Includes the outer reply identity, target, generation, provider
/// provenance maxima, and the exact result-family ceiling.
pub fn semantic_session_result_reservation_bytes_v1(
    kind: SemanticQueryKindV1,
    limits: SemanticLimitsV1,
) -> Option<usize> {
    if !limits.is_valid() {
        return None;
    }
    const TARGET: usize = 16 + 8;
    const SPAN: usize = 8 + 8;
    const LOCATION: usize = TARGET + SPAN;
    let fixed = 1usize
        .checked_add(16)?
        .checked_add(TARGET)?
        .checked_add(8)?
        .checked_add(16)?
        .checked_add(8)?
        .checked_add(MAX_SEMANTIC_PROVIDER_PROFILE_BYTES_V1)?
        .checked_add(8)?
        .checked_add(MAX_SEMANTIC_PROVIDER_VERSION_BYTES_V1)?;
    let variable = match kind {
        SemanticQueryKindV1::Symbols => 1usize.checked_add(8)?.checked_add(
            limits.max_results.checked_mul(
                8usize
                    .checked_add(limits.max_label_bytes)?
                    .checked_add(1)?
                    .checked_add(LOCATION)?,
            )?,
        )?,
        SemanticQueryKindV1::Definition
        | SemanticQueryKindV1::References
        | SemanticQueryKindV1::Implementations
        | SemanticQueryKindV1::IncomingCalls
        | SemanticQueryKindV1::OutgoingCalls => 1usize
            .checked_add(8)?
            .checked_add(limits.max_results.checked_mul(LOCATION)?)?,
        SemanticQueryKindV1::Diagnostics => 1usize.checked_add(8)?.checked_add(
            limits.max_results.checked_mul(
                1usize
                    .checked_add(LOCATION)?
                    .checked_add(8)?
                    .checked_add(limits.max_detail_bytes)?
                    .checked_add(1)?
                    .checked_add(8)?
                    .checked_add(limits.max_label_bytes)?,
            )?,
        )?,
        SemanticQueryKindV1::TypeAtPoint => 1usize
            .checked_add(8)?
            .checked_add(limits.max_detail_bytes)?,
        SemanticQueryKindV1::PlanRename => 1usize.checked_add(8)?.checked_add(
            limits.max_rename_edits.checked_mul(
                LOCATION
                    .checked_add(8)?
                    .checked_add(limits.max_label_bytes)?,
            )?,
        )?,
    };
    fixed.checked_add(variable)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionReadinessV1 {
    Unavailable,
    Synchronizing {
        epoch: SemanticSynchronizationEpochV1,
    },
    Ready {
        epoch: SemanticSynchronizationEpochV1,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticProviderReadinessProjectionV1 {
    Unavailable,
    Indexing,
    Ready {
        generation: SemanticIndexGenerationV1,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticProviderObservationV1 {
    readiness: SemanticProviderReadinessProjectionV1,
    capabilities: SemanticCapabilitySetV1,
}

impl SemanticProviderObservationV1 {
    pub const fn new(
        readiness: SemanticProviderReadinessProjectionV1,
        capabilities: SemanticCapabilitySetV1,
    ) -> Self {
        Self {
            readiness,
            capabilities,
        }
    }
    pub const fn readiness(self) -> SemanticProviderReadinessProjectionV1 {
        self.readiness
    }
    pub const fn capabilities(self) -> SemanticCapabilitySetV1 {
        self.capabilities
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticSessionDescriptionV1 {
    incarnation: SemanticSessionIncarnationV1,
    role_ceiling: SemanticSessionRolesV1,
    readiness: SemanticSessionReadinessV1,
    /// Provenance-free provider projection. Provider fingerprint remains
    /// private to the service result and never becomes discovery metadata.
    provider: SemanticProviderObservationV1,
    limits: SemanticSessionLimitsV1,
}

impl SemanticSessionDescriptionV1 {
    pub fn try_new(
        incarnation: SemanticSessionIncarnationV1,
        role_ceiling: SemanticSessionRolesV1,
        readiness: SemanticSessionReadinessV1,
        provider: SemanticProviderObservationV1,
        limits: SemanticSessionLimitsV1,
    ) -> Option<Self> {
        limits.is_valid().then_some(Self {
            incarnation,
            role_ceiling,
            readiness,
            provider,
            limits,
        })
    }
    pub const fn incarnation(self) -> SemanticSessionIncarnationV1 {
        self.incarnation
    }
    pub const fn role_ceiling(self) -> SemanticSessionRolesV1 {
        self.role_ceiling
    }
    pub const fn readiness(self) -> SemanticSessionReadinessV1 {
        self.readiness
    }
    pub const fn provider(self) -> SemanticProviderObservationV1 {
        self.provider
    }
    pub const fn limits(self) -> SemanticSessionLimitsV1 {
        self.limits
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionRefusalV1 {
    InvalidSession,
    UnknownMembership,
    RoleDenied,
    InputLimit,
    Capacity,
    SynchronizationRequired,
    SynchronizationConflict,
    ProviderRefused,
    ClientRetiring,
    AbsentOrForeign,
    ProtocolViolation,
}

/// Closed refusal fiber for each command. Reasons not in this table are not
/// interchangeable generic errors: emitting one would claim a state that the
/// command cannot lawfully observe.
pub const fn semantic_session_refusal_allowed_v1(
    kind: SemanticSessionCommandKindV1,
    reason: SemanticSessionRefusalV1,
) -> bool {
    use SemanticSessionCommandKindV1 as K;
    use SemanticSessionRefusalV1 as R;
    match kind {
        K::Describe => matches!(
            reason,
            R::Capacity | R::ClientRetiring | R::ProtocolViolation
        ),
        K::JoinSession => matches!(
            reason,
            R::InvalidSession
                | R::RoleDenied
                | R::Capacity
                | R::ClientRetiring
                | R::ProtocolViolation
        ),
        K::BeginSynchronization
        | K::SyncOpen
        | K::SyncChange
        | K::SyncClose
        | K::CompleteSynchronization => matches!(
            reason,
            R::InvalidSession
                | R::UnknownMembership
                | R::RoleDenied
                | R::InputLimit
                | R::Capacity
                | R::SynchronizationRequired
                | R::SynchronizationConflict
                | R::ProviderRefused
                | R::ClientRetiring
                | R::ProtocolViolation
        ),
        K::Submit => matches!(
            reason,
            R::InvalidSession
                | R::UnknownMembership
                | R::RoleDenied
                | R::InputLimit
                | R::Capacity
                | R::SynchronizationRequired
                | R::ProviderRefused
                | R::ClientRetiring
                | R::AbsentOrForeign
                | R::ProtocolViolation
        ),
        K::Cancel => matches!(
            reason,
            R::InvalidSession
                | R::UnknownMembership
                | R::RoleDenied
                | R::Capacity
                | R::ClientRetiring
                | R::AbsentOrForeign
                | R::ProtocolViolation
        ),
        K::Collect => matches!(
            reason,
            R::InvalidSession
                | R::UnknownMembership
                | R::RoleDenied
                | R::ClientRetiring
                | R::AbsentOrForeign
                | R::ProtocolViolation
        ),
        K::ReleaseRequest => matches!(
            reason,
            R::InvalidSession
                | R::UnknownMembership
                | R::RoleDenied
                | R::ClientRetiring
                | R::AbsentOrForeign
                | R::ProtocolViolation
        ),
        K::LeaveSession => matches!(
            reason,
            R::InvalidSession
                | R::UnknownMembership
                | R::Capacity
                | R::ClientRetiring
                | R::ProtocolViolation
        ),
    }
}

/// V1 admits exactly one active synchronize membership and epoch. A competing
/// begin refuses with `SynchronizationConflict` without changing readiness or
/// the authoritative epoch. Query readiness is published only after the owner
/// completes an exactly matching manifest. Synchronizer disconnect/leave
/// starts bounded epoch cleanup; no replacement synchronizer enters until that
/// obligation closes. Successful cleanup leaves `Unavailable` and requires a
/// fresh complete epoch—it never resurrects a partial prior epoch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticSessionPendingV1 {
    Request(SemanticRequestRefV1),
    Retirement {
        remaining_documents: u32,
        quarantined_requests: u32,
    },
}

/// Closed application outcome. Mutating operations are retained by exact
/// operation reference and therefore distinguish matching replay, conflicting
/// reuse, full replay storage and lost-reply uncertainty in the broker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticSessionOutcomeV1 {
    Description(SemanticSessionDescriptionV1),
    Joined {
        /// Server-issued and replayed under the Join operation identity.
        membership: SemanticMembershipRefV1,
        /// Trusted policy grant meet requested roles; never the request alone.
        granted_roles: SemanticSessionRolesV1,
        remaining_operations: u32,
    },
    /// Mutation admission only. In particular, accepted Cancel is nonterminal;
    /// its terminal `Cancelled` delivery is observed through exact Collect.
    Accepted,
    Pending(SemanticSessionPendingV1),
    Complete(SemanticReplyV1),
    Unsupported(SemanticQueryKindV1),
    Stale,
    Unavailable,
    Cancelled,
    TimedOut,
    PeerRefused(SemanticRefusalV1),
    Refused(SemanticSessionRefusalV1),
    RequestConflict,
    ReplayCapacity,
}

/// Transport truth is separate from application outcome. A phase-aware local
/// exchange maps one attempt to exactly one of these products.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticExchangeDispositionV1<T> {
    NotSent,
    Reply(T),
    OutcomeUnknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticSessionReplyV1 {
    kind: SemanticSessionCommandKindV1,
    exchange: SemanticExchangeRefV1,
    incarnation: SemanticSessionIncarnationV1,
    /// `None` only for Describe and pre-membership Join refusals. Successful
    /// Join and every post-join reply carry the exact server-issued membership.
    membership: Option<SemanticMembershipRefV1>,
    operation: Option<SemanticSessionOperationRefV1>,
    outcome: SemanticSessionOutcomeV1,
}

impl SemanticSessionReplyV1 {
    pub fn try_new(
        kind: SemanticSessionCommandKindV1,
        exchange: SemanticExchangeRefV1,
        incarnation: SemanticSessionIncarnationV1,
        membership: Option<SemanticMembershipRefV1>,
        operation: Option<SemanticSessionOperationRefV1>,
        outcome: SemanticSessionOutcomeV1,
    ) -> Option<Self> {
        let binding_valid = match kind {
            SemanticSessionCommandKindV1::Describe => membership.is_none() && operation.is_none(),
            SemanticSessionCommandKindV1::JoinSession => {
                operation.is_some()
                    && match &outcome {
                        SemanticSessionOutcomeV1::Joined {
                            membership: issued, ..
                        } => membership == Some(*issued),
                        _ => membership.is_none(),
                    }
            }
            SemanticSessionCommandKindV1::Collect => membership.is_some() && operation.is_none(),
            _ => membership.is_some() && operation.is_some(),
        };
        let outcome_valid = match &outcome {
            SemanticSessionOutcomeV1::Description(value) => {
                kind == SemanticSessionCommandKindV1::Describe
                    && value.incarnation() == incarnation
                    && membership.is_none()
            }
            SemanticSessionOutcomeV1::Joined {
                membership: issued,
                remaining_operations,
                ..
            } => {
                kind == SemanticSessionCommandKindV1::JoinSession
                    && membership == Some(*issued)
                    && *remaining_operations > 0
                    && *remaining_operations <= MAX_SEMANTIC_SESSION_OPERATIONS_PER_MEMBERSHIP_V1
            }
            SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Retirement {
                remaining_documents,
                quarantined_requests,
            }) => {
                (*remaining_documents != 0 || *quarantined_requests != 0)
                    && *remaining_documents <= MAX_SEMANTIC_SESSION_DOCUMENTS_V1
                    && *quarantined_requests <= MAX_SEMANTIC_SESSION_PENDING_V1
            }
            SemanticSessionOutcomeV1::Refused(reason) => {
                semantic_session_refusal_allowed_v1(kind, *reason)
            }
            _ => true,
        };
        let closed_product = match kind {
            SemanticSessionCommandKindV1::Describe => matches!(
                outcome,
                SemanticSessionOutcomeV1::Description(_)
                    | SemanticSessionOutcomeV1::Unavailable
                    | SemanticSessionOutcomeV1::Refused(_)
            ),
            SemanticSessionCommandKindV1::JoinSession => matches!(
                outcome,
                SemanticSessionOutcomeV1::Joined { .. }
                    | SemanticSessionOutcomeV1::Refused(_)
                    | SemanticSessionOutcomeV1::RequestConflict
                    | SemanticSessionOutcomeV1::ReplayCapacity
            ),
            SemanticSessionCommandKindV1::BeginSynchronization
            | SemanticSessionCommandKindV1::SyncOpen
            | SemanticSessionCommandKindV1::SyncChange
            | SemanticSessionCommandKindV1::SyncClose
            | SemanticSessionCommandKindV1::CompleteSynchronization => matches!(
                outcome,
                SemanticSessionOutcomeV1::Accepted
                    | SemanticSessionOutcomeV1::Unavailable
                    | SemanticSessionOutcomeV1::Refused(_)
                    | SemanticSessionOutcomeV1::RequestConflict
                    | SemanticSessionOutcomeV1::ReplayCapacity
            ),
            SemanticSessionCommandKindV1::Submit => matches!(
                outcome,
                SemanticSessionOutcomeV1::Accepted
                    | SemanticSessionOutcomeV1::Unsupported(_)
                    | SemanticSessionOutcomeV1::Stale
                    | SemanticSessionOutcomeV1::Unavailable
                    | SemanticSessionOutcomeV1::Refused(_)
                    | SemanticSessionOutcomeV1::RequestConflict
                    | SemanticSessionOutcomeV1::ReplayCapacity
            ),
            SemanticSessionCommandKindV1::Cancel => matches!(
                outcome,
                SemanticSessionOutcomeV1::Accepted
                    | SemanticSessionOutcomeV1::Refused(_)
                    | SemanticSessionOutcomeV1::RequestConflict
                    | SemanticSessionOutcomeV1::ReplayCapacity
            ),
            SemanticSessionCommandKindV1::Collect => matches!(
                outcome,
                SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Request(_))
                    | SemanticSessionOutcomeV1::Complete(_)
                    | SemanticSessionOutcomeV1::Unsupported(_)
                    | SemanticSessionOutcomeV1::Stale
                    | SemanticSessionOutcomeV1::Unavailable
                    | SemanticSessionOutcomeV1::Cancelled
                    | SemanticSessionOutcomeV1::TimedOut
                    | SemanticSessionOutcomeV1::PeerRefused(_)
                    | SemanticSessionOutcomeV1::Refused(_)
            ),
            SemanticSessionCommandKindV1::ReleaseRequest => matches!(
                outcome,
                SemanticSessionOutcomeV1::Accepted
                    | SemanticSessionOutcomeV1::Refused(_)
                    | SemanticSessionOutcomeV1::RequestConflict
                    | SemanticSessionOutcomeV1::ReplayCapacity
            ),
            SemanticSessionCommandKindV1::LeaveSession => matches!(
                outcome,
                SemanticSessionOutcomeV1::Accepted
                    | SemanticSessionOutcomeV1::Pending(
                        SemanticSessionPendingV1::Retirement { .. }
                    )
                    | SemanticSessionOutcomeV1::Refused(_)
                    | SemanticSessionOutcomeV1::RequestConflict
                    | SemanticSessionOutcomeV1::ReplayCapacity
            ),
        };
        (binding_valid && outcome_valid && closed_product).then_some(Self {
            kind,
            exchange,
            incarnation,
            membership,
            operation,
            outcome,
        })
    }

    pub const fn kind(&self) -> SemanticSessionCommandKindV1 {
        self.kind
    }
    pub const fn exchange(&self) -> SemanticExchangeRefV1 {
        self.exchange
    }
    pub const fn incarnation(&self) -> SemanticSessionIncarnationV1 {
        self.incarnation
    }
    pub const fn membership(&self) -> Option<SemanticMembershipRefV1> {
        self.membership
    }
    pub const fn operation(&self) -> Option<SemanticSessionOperationRefV1> {
        self.operation
    }
    pub const fn outcome(&self) -> &SemanticSessionOutcomeV1 {
        &self.outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_form_a_bounded_attenuating_product() {
        let sync_query =
            SemanticSessionRolesV1::synchronize().join_trusted(SemanticSessionRolesV1::query());
        assert!(sync_query.contains(SemanticSessionRolesV1::synchronize()));
        assert!(sync_query.contains(SemanticSessionRolesV1::query()));
        assert_eq!(
            sync_query.meet(SemanticSessionRolesV1::query()),
            SemanticSessionRolesV1::query()
        );
        assert_eq!(SemanticSessionRolesV1::top().meet(sync_query), sync_query);
        assert_eq!(SemanticSessionRolesV1::from_bits(8), None);
    }

    #[test]
    fn every_post_join_command_is_exactly_bound() {
        let operation = SemanticSessionOperationRefV1::new([1; 16]).unwrap();
        let exchange = SemanticExchangeRefV1::new([4; 16]).unwrap();
        let incarnation = SemanticSessionIncarnationV1::new([2; 16]).unwrap();
        let membership = SemanticMembershipRefV1::new([3; 16]).unwrap();
        let command = SemanticSessionCommandV1::LeaveSession {
            exchange,
            operation,
            incarnation,
            membership,
        };
        assert_eq!(command.binding(), Some((incarnation, membership)));
        assert_eq!(command.operation(), Some(operation));
        assert_eq!(command.exchange(), exchange);
    }

    #[test]
    fn synchronization_manifest_is_bounded() {
        let limits = SemanticSessionLimitsV1::reference();
        assert!(
            SemanticSynchronizationManifestV1::new(limits.max_documents, 0, [7; 32], limits)
                .is_some()
        );
        assert!(
            SemanticSynchronizationManifestV1::new(limits.max_documents + 1, 0, [7; 32], limits,)
                .is_none()
        );
    }

    #[test]
    fn accepted_outcome_is_closed_over_all_command_kinds() {
        let exchange = SemanticExchangeRefV1::new([1; 16]).unwrap();
        let operation = SemanticSessionOperationRefV1::new([2; 16]).unwrap();
        let incarnation = SemanticSessionIncarnationV1::new([3; 16]).unwrap();
        let membership = SemanticMembershipRefV1::new([4; 16]).unwrap();
        let cases = [
            (SemanticSessionCommandKindV1::Describe, false),
            (SemanticSessionCommandKindV1::JoinSession, false),
            (SemanticSessionCommandKindV1::BeginSynchronization, true),
            (SemanticSessionCommandKindV1::SyncOpen, true),
            (SemanticSessionCommandKindV1::SyncChange, true),
            (SemanticSessionCommandKindV1::SyncClose, true),
            (SemanticSessionCommandKindV1::CompleteSynchronization, true),
            (SemanticSessionCommandKindV1::Submit, true),
            (SemanticSessionCommandKindV1::Cancel, true),
            (SemanticSessionCommandKindV1::Collect, false),
            (SemanticSessionCommandKindV1::ReleaseRequest, true),
            (SemanticSessionCommandKindV1::LeaveSession, true),
        ];
        for (kind, expected) in cases {
            let (member, op) = match kind {
                SemanticSessionCommandKindV1::Describe => (None, None),
                SemanticSessionCommandKindV1::JoinSession => (None, Some(operation)),
                SemanticSessionCommandKindV1::Collect => (Some(membership), None),
                _ => (Some(membership), Some(operation)),
            };
            assert_eq!(
                SemanticSessionReplyV1::try_new(
                    kind,
                    exchange,
                    incarnation,
                    member,
                    op,
                    SemanticSessionOutcomeV1::Accepted
                )
                .is_some(),
                expected,
                "unexpected Accepted closure for {kind:?}",
            );
        }
    }

    #[test]
    fn manifest_debug_redacts_linkable_workspace_evidence() {
        let manifest = SemanticSynchronizationManifestV1::new(
            37,
            4096,
            [0xab; 32],
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap();
        let debug = format!("{manifest:?}");
        assert!(!debug.contains("37"));
        assert!(!debug.contains("4096"));
        assert!(!debug.contains("ab"));
    }

    #[test]
    fn degenerate_membership_and_retirement_states_are_uninhabited() {
        let exchange = SemanticExchangeRefV1::new([1; 16]).unwrap();
        let operation = SemanticSessionOperationRefV1::new([2; 16]).unwrap();
        let incarnation = SemanticSessionIncarnationV1::new([3; 16]).unwrap();
        let membership = SemanticMembershipRefV1::new([4; 16]).unwrap();
        assert!(
            SemanticSessionReplyV1::try_new(
                SemanticSessionCommandKindV1::JoinSession,
                exchange,
                incarnation,
                Some(membership),
                Some(operation),
                SemanticSessionOutcomeV1::Joined {
                    membership,
                    granted_roles: SemanticSessionRolesV1::query(),
                    remaining_operations: 0,
                },
            )
            .is_none()
        );
        assert!(
            SemanticSessionReplyV1::try_new(
                SemanticSessionCommandKindV1::LeaveSession,
                exchange,
                incarnation,
                Some(membership),
                Some(operation),
                SemanticSessionOutcomeV1::Pending(SemanticSessionPendingV1::Retirement {
                    remaining_documents: 0,
                    quarantined_requests: 0,
                }),
            )
            .is_none()
        );
    }

    #[test]
    fn session_limits_preserve_ordinary_and_retirement_capacity() {
        let limits = SemanticSessionLimitsV1::reference();
        let reserve = limits.retirement_operation_reserve().unwrap();
        assert_eq!(reserve, limits.max_documents + limits.max_pending);
        assert_eq!(
            limits.ordinary_operation_budget(),
            Some(limits.max_operations_per_membership - reserve)
        );
        assert!(
            limits.minimum_membership_replay_bytes().unwrap() <= limits.max_replay_intent_bytes
        );

        let mut no_ordinary_headroom = limits;
        no_ordinary_headroom.max_operations_per_membership = reserve;
        assert!(!no_ordinary_headroom.is_valid());

        let mut no_retirement_replay = limits;
        no_retirement_replay.max_replay_intent_bytes =
            limits.minimum_membership_replay_bytes().unwrap() - 1;
        assert!(!no_retirement_replay.is_valid());
    }

    #[test]
    fn refusal_fibers_are_exhaustively_closed() {
        use SemanticSessionCommandKindV1 as K;
        use SemanticSessionRefusalV1 as R;
        let exchange = SemanticExchangeRefV1::new([1; 16]).unwrap();
        let operation = SemanticSessionOperationRefV1::new([2; 16]).unwrap();
        let incarnation = SemanticSessionIncarnationV1::new([3; 16]).unwrap();
        let membership = SemanticMembershipRefV1::new([4; 16]).unwrap();
        let kinds = [
            K::Describe,
            K::JoinSession,
            K::BeginSynchronization,
            K::SyncOpen,
            K::SyncChange,
            K::SyncClose,
            K::CompleteSynchronization,
            K::Submit,
            K::Cancel,
            K::Collect,
            K::ReleaseRequest,
            K::LeaveSession,
        ];
        let reasons = [
            R::InvalidSession,
            R::UnknownMembership,
            R::RoleDenied,
            R::InputLimit,
            R::Capacity,
            R::SynchronizationRequired,
            R::SynchronizationConflict,
            R::ProviderRefused,
            R::ClientRetiring,
            R::AbsentOrForeign,
            R::ProtocolViolation,
        ];
        for kind in kinds {
            for reason in reasons {
                let expected = match kind {
                    K::Describe => matches!(
                        reason,
                        R::Capacity | R::ClientRetiring | R::ProtocolViolation
                    ),
                    K::JoinSession => matches!(
                        reason,
                        R::InvalidSession
                            | R::RoleDenied
                            | R::Capacity
                            | R::ClientRetiring
                            | R::ProtocolViolation
                    ),
                    K::BeginSynchronization
                    | K::SyncOpen
                    | K::SyncChange
                    | K::SyncClose
                    | K::CompleteSynchronization => !matches!(reason, R::AbsentOrForeign),
                    K::Submit => !matches!(reason, R::SynchronizationConflict),
                    K::Cancel => matches!(
                        reason,
                        R::InvalidSession
                            | R::UnknownMembership
                            | R::RoleDenied
                            | R::Capacity
                            | R::ClientRetiring
                            | R::AbsentOrForeign
                            | R::ProtocolViolation
                    ),
                    K::Collect | K::ReleaseRequest => matches!(
                        reason,
                        R::InvalidSession
                            | R::UnknownMembership
                            | R::RoleDenied
                            | R::ClientRetiring
                            | R::AbsentOrForeign
                            | R::ProtocolViolation
                    ),
                    K::LeaveSession => matches!(
                        reason,
                        R::InvalidSession
                            | R::UnknownMembership
                            | R::Capacity
                            | R::ClientRetiring
                            | R::ProtocolViolation
                    ),
                };
                assert_eq!(semantic_session_refusal_allowed_v1(kind, reason), expected);
                let (member, op) = match kind {
                    K::Describe => (None, None),
                    K::JoinSession => (None, Some(operation)),
                    K::Collect => (Some(membership), None),
                    _ => (Some(membership), Some(operation)),
                };
                assert_eq!(
                    SemanticSessionReplyV1::try_new(
                        kind,
                        exchange,
                        incarnation,
                        member,
                        op,
                        SemanticSessionOutcomeV1::Refused(reason)
                    )
                    .is_some(),
                    expected,
                    "unexpected refusal fiber for {kind:?}/{reason:?}",
                );
            }
        }
    }

    fn snapshot(tag: u8, revision: u64, source: &str) -> SemanticSessionSnapshotV1 {
        SemanticSessionSnapshotV1::try_new(
            SemanticTargetV1 {
                document: SemanticDocumentRefV1::new([tag; 16]),
                source_revision: vellum_host::semantic::SemanticSourceRevisionV1::new(revision),
            },
            "rust".into(),
            source.into(),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap()
    }

    #[test]
    fn compositional_manifest_is_order_independent_and_change_sensitive() {
        let limits = SemanticSessionLimitsV1::reference();
        let first = snapshot(1, 1, "a");
        let second = snapshot(2, 1, "b");
        let forward = SemanticSynchronizationManifestV1::try_from_snapshots(
            &[first.clone(), second.clone()],
            limits,
        )
        .unwrap();
        let reverse = SemanticSynchronizationManifestV1::try_from_snapshots(
            &[second.clone(), first.clone()],
            limits,
        )
        .unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(
            forward.digest(),
            [
                33, 137, 48, 189, 243, 49, 154, 46, 22, 9, 188, 125, 255, 118, 52, 78, 156, 184,
                196, 69, 31, 165, 194, 32, 13, 40, 133, 174, 133, 144, 174, 152
            ]
        );
        assert_ne!(
            forward,
            SemanticSynchronizationManifestV1::try_from_snapshots(
                &[snapshot(1, 2, "a"), second],
                limits
            )
            .unwrap()
        );
        assert!(
            SemanticSynchronizationManifestV1::try_from_snapshots(&[first.clone(), first], limits)
                .is_none()
        );
        let empty = SemanticSynchronizationManifestV1::try_from_snapshots(&[], limits).unwrap();
        assert_eq!(
            empty.digest(),
            [
                199, 56, 206, 55, 181, 220, 252, 136, 158, 149, 248, 205, 126, 19, 179, 248, 243,
                188, 45, 49, 114, 13, 233, 91, 96, 133, 186, 160, 252, 55, 255, 78
            ]
        );
    }

    #[test]
    fn snapshot_clones_share_immutable_storage() {
        let original = snapshot(1, 1, "source");
        let cloned = original.clone();
        assert!(Arc::ptr_eq(&original.language, &cloned.language));
        assert!(Arc::ptr_eq(&original.source, &cloned.source));
        assert_eq!(original, cloned);
    }

    #[test]
    fn result_reservations_count_every_logical_field_for_all_query_kinds() {
        let limits = SemanticLimitsV1::reference();
        let target = 16 + 8;
        let location = target + 8 + 8;
        let fixed = 1
            + 16
            + target
            + 8
            + 16
            + 8
            + MAX_SEMANTIC_PROVIDER_PROFILE_BYTES_V1
            + 8
            + MAX_SEMANTIC_PROVIDER_VERSION_BYTES_V1;
        let cases = [
            (
                SemanticQueryKindV1::Symbols,
                fixed + 1 + 8 + limits.max_results * (8 + limits.max_label_bytes + 1 + location),
            ),
            (
                SemanticQueryKindV1::Definition,
                fixed + 1 + 8 + limits.max_results * location,
            ),
            (
                SemanticQueryKindV1::References,
                fixed + 1 + 8 + limits.max_results * location,
            ),
            (
                SemanticQueryKindV1::Implementations,
                fixed + 1 + 8 + limits.max_results * location,
            ),
            (
                SemanticQueryKindV1::IncomingCalls,
                fixed + 1 + 8 + limits.max_results * location,
            ),
            (
                SemanticQueryKindV1::OutgoingCalls,
                fixed + 1 + 8 + limits.max_results * location,
            ),
            (
                SemanticQueryKindV1::Diagnostics,
                fixed
                    + 1
                    + 8
                    + limits.max_results
                        * (1 + location
                            + 8
                            + limits.max_detail_bytes
                            + 1
                            + 8
                            + limits.max_label_bytes),
            ),
            (
                SemanticQueryKindV1::TypeAtPoint,
                fixed + 1 + 8 + limits.max_detail_bytes,
            ),
            (
                SemanticQueryKindV1::PlanRename,
                fixed + 1 + 8 + limits.max_rename_edits * (location + 8 + limits.max_label_bytes),
            ),
        ];
        for (kind, expected) in cases {
            assert_eq!(
                semantic_session_result_reservation_bytes_v1(kind, limits),
                Some(expected)
            );
        }
        let mut invalid = limits;
        invalid.max_results = 0;
        assert_eq!(
            semantic_session_result_reservation_bytes_v1(SemanticQueryKindV1::Diagnostics, invalid),
            None
        );
    }
}
