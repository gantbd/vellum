//! Engine-owned resource values. Trusted provider ports attest host observations;
//! these types do not prove OS truth or perform I/O. No paths/SDKs cross this API.
use crate::coordinator::{AnnotationSnapshot, DocumentVersion, SourceSerializationStateV1};
use std::sync::Arc;
mod causal;
mod evidence;
mod read;
mod source_read;
pub(crate) use causal::{CausalEdge, CausalVersion};
pub use causal::{ProviderCausalCapabilitiesV1, ResourceVersionV1};
pub use causal::{SourceContinuityEdgeV1, SourceContinuityReceiptV1};
pub use evidence::ProviderEvidenceV1;
pub(crate) use source_read::{SourceReadAtom, SourceReadVersion};
pub use source_read::{
    SourceReadDispatchV1, SourceReadEvidenceV1, SourceReadFailureV1, SourceReadIdV1,
    SourceReadTargetV1, SourceReadVersionV1,
};
mod restoration;
pub use read::{
    AnnotationReadDispatch, AnnotationReadEvidence, AnnotationReadId, AnnotationReadStatus,
    AnnotationReadTarget, CapturedAnnotationRead, ReadFailure, ReadPhase, ReadStrength,
};
pub(crate) use read::{ReadAtom, ReadCaptureData};
pub use restoration::{
    AnnotationRestoration, AnnotationRestorationOutcome, AnnotationRestorerPort,
    RestorationContext, RestorationIdentity, RestoredAnnotation,
};
pub(crate) use restoration::{RestorationProvenance, RestorationRegistration};

#[derive(Clone, Copy, Debug)]
pub struct ResourceLimits {
    pub max_providers: usize,
    pub max_live_tasks: usize,
    pub max_issued_tasks: usize,
    pub max_queued_per_destination: usize,
    pub max_outstanding: usize,
    pub max_blob_bytes: usize,
    pub max_key_bytes: usize,
    pub max_validator_bytes: usize,
    pub max_encoders: usize,
    pub max_live_artifacts: usize,
    pub max_issued_artifacts: usize,
    pub max_held_artifact_bytes: usize,
    /// Pending native claim/envelope budget, not file size or physical allocation.
    pub max_future_baseline_bytes: usize,
    pub max_live_reads: usize,
    pub max_issued_reads: usize,
    pub max_read_chunks: usize,
    pub max_held_read_bytes: usize,
    pub max_restorers: usize,
    pub max_restoration_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    Source,
    Annotations,
}

/// Original codec-preparation capability. Copies retain no source payload and
/// cannot revive a discarded/committed preparation or authenticate write rights.
/// Dropping this metadata does not discard the owner's preparation; discharge
/// is explicit, not garbage-collection or worker-completion evidence.
#[derive(Clone)]
pub struct PreparedSourceArtifactIdV1 {
    pub(crate) owner: Arc<()>,
    pub(crate) marker: Arc<()>,
}
impl std::fmt::Debug for PreparedSourceArtifactIdV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PreparedSourceArtifactIdV1(<opaque>)")
    }
}

/// Content-correlation witness for codec-owned serialized source bytes. Exact
/// source-incarnation congruence must use the opaque `DocumentVersion` retained
/// by `CapturedSourceSerializationV1`; this truncated digest is not authority or
/// a globally unique process identity.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceSerializationCaptureWitnessV1(pub(crate) [u8; 16]);

impl SourceSerializationCaptureWitnessV1 {
    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }
}

impl std::fmt::Debug for SourceSerializationCaptureWitnessV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceSerializationCaptureWitnessV1(<opaque>)")
    }
}

/// One-use transfer of an exact source serialization out of the engine codec.
/// The carrier cannot be constructed or cloned outside the engine. Consuming it
/// transfers the immutable bytes without re-encoding or consulting current text.
///
/// ```compile_fail
/// use vellum_engine::resource::CapturedSourceSerializationV1;
///
/// // Only the engine's codec preparation can construct this carrier.
/// let _forged = CapturedSourceSerializationV1 {};
/// ```
///
/// ```compile_fail
/// use vellum_engine::resource::CapturedSourceSerializationV1;
///
/// fn duplicate(capture: CapturedSourceSerializationV1) {
///     let _second_effect = capture.clone();
/// }
/// ```
pub struct CapturedSourceSerializationV1 {
    pub(crate) witness: SourceSerializationCaptureWitnessV1,
    pub(crate) source: DocumentVersion,
    pub(crate) serialization: SourceSerializationStateV1,
    pub(crate) bytes: Vec<u8>,
}

impl CapturedSourceSerializationV1 {
    pub const fn witness(&self) -> SourceSerializationCaptureWitnessV1 {
        self.witness
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Exact opaque native incarnation and revision captured by the codec.
    /// `DocumentId` cannot be forged outside its issuing coordinator.
    pub fn source(&self) -> &DocumentVersion {
        &self.source
    }

    pub const fn serialization(&self) -> SourceSerializationStateV1 {
        self.serialization
    }

    pub fn into_parts(self) -> (SourceSerializationCaptureWitnessV1, Vec<u8>) {
        (self.witness, self.bytes)
    }
}

impl std::fmt::Debug for CapturedSourceSerializationV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CapturedSourceSerializationV1(<redacted>)")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SavePurpose {
    OrdinarySource,
    OrdinaryAnnotations,
    AnnotationExport,
    SourceCopyExport,
    SourceSaveAs,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceExportPurposeV1 {
    CopyExport,
    SaveAs,
}
pub struct SourceExportOptionsV1 {
    pub purpose: SourceExportPurposeV1,
    pub requirements: SaveRequirements,
    /// Current structured resource distinction for Save As. Copy export never
    /// activates an ordinary pair and accepts no substitute pair grant.
    pub pair: Option<OrdinaryPairEvidence>,
}
/// Versioned after-result schema, independent of destination presence. It is not
/// a claim that a nonexistent resource has a before version.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveResultVersionProfileV1 {
    Opaque,
    EpochToken,
}
/// Explicit result schema for source export. Present destinations must retain
/// their before-version schema; an absent target can select typed after-results.
/// Provider causal capability and exclusive-create authority remain independent.
pub struct SourceExportOptionsV2 {
    pub intent: SourceExportOptionsV1,
    pub result_version_profile: SaveResultVersionProfileV1,
}

/// A registered association target, not a read, write condition or save grant.
pub struct SourceBindingTargetV1 {
    pub(crate) registration: Arc<Registration>,
    pub(crate) identity: Identity,
}
impl std::fmt::Debug for SourceBindingTargetV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceBindingTargetV1(<opaque>)")
    }
}
pub struct SourceRebindOptionsV1 {
    pub expected_binding: Option<u64>,
    pub max_binding_history: usize,
    pub pair: Option<OrdinaryPairEvidence>,
}

/// Opaque, trusted present-destination observation. This is a host assertion,
/// not proof of freshness, a create grant or a non-ABA external version.
pub struct SourceSaveAsProbeV1 {
    pub(crate) epoch: Option<Blob>,
    pub(crate) registration: Arc<Registration>,
    pub(crate) identity: Identity,
    pub(crate) version: Option<Blob>,
}
impl std::fmt::Debug for SourceSaveAsProbeV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SourceSaveAsProbeV1(<opaque>)")
    }
}

/// Independent host capabilities. Existing provider registration advertises
/// neither by default; rename/probe support alone proves neither guarantee.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProviderDestinationCapabilitiesV1 {
    pub exclusive_create: bool,
    pub atomic_conditional_write: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaselineCapacityState {
    Pending,
    Installed,
    NotWritten,
    TerminalIneligible,
    Superseded,
}
#[derive(Clone)]
pub struct ResourceArtifactId {
    pub(crate) owner: Arc<()>,
    pub(crate) slot: usize,
}
impl PartialEq for ResourceArtifactId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for ResourceArtifactId {}
impl std::fmt::Debug for ResourceArtifactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ResourceArtifactId(<opaque>)")
    }
}
#[derive(Clone)]
pub struct AnnotationEncodingContext {
    pub(crate) owner: Arc<()>,
    pub(crate) marker: Arc<()>,
}
impl PartialEq for AnnotationEncodingContext {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && Arc::ptr_eq(&self.marker, &other.marker)
    }
}
impl Eq for AnnotationEncodingContext {}
impl std::fmt::Debug for AnnotationEncodingContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AnnotationEncodingContext(<opaque>)")
    }
}
#[derive(Debug)]
pub(crate) struct EncoderRegistration {
    pub(crate) context: AnnotationEncodingContext,
    pub(crate) limits: ResourceLimits,
}
/// Privileged host attestation seam, not a grant carried by document content.
/// The configured producer must validate complete reference-JSON v1 output and
/// actual identity/source associations. The engine intentionally has no JSON runtime.
#[derive(Clone, Debug)]
pub struct AnnotationEncoderPort {
    pub(crate) registration: Arc<EncoderRegistration>,
}
impl AnnotationEncoderPort {
    pub fn context(&self) -> AnnotationEncodingContext {
        self.registration.context.clone()
    }
    pub fn seal(
        &self,
        snapshot: AnnotationSnapshot,
        bytes: Vec<u8>,
    ) -> Result<EncodedAnnotations, ResourceError> {
        if !snapshot.belongs_to(&self.registration.context.owner) {
            return Err(ResourceError::Unavailable);
        }
        if bytes.len() > self.registration.limits.max_blob_bytes {
            return Err(ResourceError::Limit);
        }
        Ok(EncodedAnnotations {
            registration: self.registration.clone(),
            snapshot,
            bytes: Blob(Arc::from(bytes)),
        })
    }
}
#[derive(Debug)]
pub struct EncodedAnnotations {
    pub(crate) registration: Arc<EncoderRegistration>,
    pub(crate) snapshot: AnnotationSnapshot,
    pub(crate) bytes: Blob,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderProfile {
    pub synchronization_supported: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protection {
    BestEffort,
    AtomicConditional,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acknowledgment {
    Ordinary,
    Synchronized,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveRequirements {
    pub protection: Protection,
    pub acknowledgment: Acknowledgment,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Application {
    Unknown,
    NotWritten,
    Written,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Synchronization {
    Unknown,
    NotRequested,
    Succeeded,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceDisposition {
    Accepted,
    Duplicate,
    Contradiction,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentComparison {
    Unbound,
    Unchanged,
    Modified,
}

/// Native temporary output and exact comparison bounds, not a wire frame,
/// retained-artifact reservation or physical peak-memory guarantee.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceComparisonBudgetV1 {
    pub max_encoded_bytes: usize,
    /// Current encoded bytes plus historical baseline bytes, checked before
    /// encoding even when their lengths already establish inequality.
    pub max_combined_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SavePhase {
    Queued,
    Dispatched,
    Stopped,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceError {
    Unavailable,
    Unbound,
    Stale,
    Permission,
    Unsupported,
    Limit,
    Capacity,
    Blocked,
    Conflict,
    Contested,
    Malformed,
    EvidenceCapacity,
    AcknowledgmentUnsatisfied,
    MissingEvidence,
    Superseded,
    Retained,
    VersionExhausted,
    Faulted,
    Source(crate::source::SourceError),
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Blob(pub(crate) Arc<[u8]>);
impl std::fmt::Debug for Blob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted bytes>")
    }
}
pub(crate) fn blob(bytes: &[u8], limit: usize, nonempty: bool) -> Result<Blob, ResourceError> {
    if bytes.len() > limit || (nonempty && bytes.is_empty()) {
        return Err(ResourceError::Limit);
    }
    let mut owned = Vec::new();
    owned
        .try_reserve_exact(bytes.len())
        .map_err(|_| ResourceError::Capacity)?;
    owned.extend_from_slice(bytes);
    Ok(Blob(Arc::from(owned)))
}

#[derive(Clone)]
pub struct IdentityDomain {
    pub(crate) owner: Arc<()>,
    pub(crate) marker: Arc<()>,
}
impl PartialEq for IdentityDomain {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && Arc::ptr_eq(&self.marker, &other.marker)
    }
}
impl Eq for IdentityDomain {}
impl std::fmt::Debug for IdentityDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("IdentityDomain(<opaque>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Identity {
    pub(crate) domain: IdentityDomain,
    pub(crate) key: Blob,
}
#[derive(Debug)]
pub(crate) struct Registration {
    pub(crate) causal: ProviderCausalCapabilitiesV1,
    pub(crate) owner: Arc<()>,
    pub(crate) domain: IdentityDomain,
    pub(crate) profile: ProviderProfile,
    pub(crate) destinations: ProviderDestinationCapabilitiesV1,
    pub(crate) limits: ResourceLimits,
}

/// Privileged host issuer. Giving this to a provider is an explicit trust choice,
/// not a grant inferred from document content or a workload identity label.
#[derive(Clone, Debug)]
pub struct ProviderPort {
    pub(crate) registration: Arc<Registration>,
}
#[derive(Clone, Debug)]
pub struct ResourceRead {
    pub(crate) epoch: Option<Blob>,
    pub(crate) registration: Arc<Registration>,
    pub(crate) identity: Identity,
    pub(crate) bytes: Blob,
    pub(crate) validator: Blob,
}

#[derive(Clone, Debug)]
pub(crate) enum DispatchCondition {
    Present {
        bytes: Blob,
        validator: Blob,
        epoch: Option<Blob>,
    },
    Absent,
}
/// Data condition, not destination-write authority. No absence validator exists.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SaveWriteConditionV1<'a> {
    Present {
        bytes: &'a [u8],
        validator: &'a [u8],
    },
    Absent,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SaveVersionV1<'a> {
    Opaque(&'a [u8]),
    Causal(ResourceVersionV1<'a>),
}
impl std::fmt::Debug for SaveVersionV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SaveVersionV1(<redacted>)")
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SaveWriteConditionV2<'a> {
    Present {
        bytes: &'a [u8],
        version: SaveVersionV1<'a>,
    },
    Absent,
}
impl std::fmt::Debug for SaveWriteConditionV2<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SaveWriteConditionV2(<redacted>)")
    }
}
impl std::fmt::Debug for SaveWriteConditionV1<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Present { .. } => "Present(<redacted>)",
            Self::Absent => "Absent",
        })
    }
}
#[derive(Debug)]
pub struct SourceExportDestinationV1 {
    pub(crate) registration: Arc<Registration>,
    pub(crate) identity: Identity,
    pub(crate) condition: DispatchCondition,
}
impl ResourceRead {
    /// Exact equality of two issuer-attributed observations. This neither
    /// authorizes a write nor establishes external freshness or alias freedom.
    pub fn same_observation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.registration, &other.registration)
            && self.identity == other.identity
            && self.epoch == other.epoch
            && self.bytes == other.bytes
            && self.validator == other.validator
    }
    pub fn into_source_binding_target(self) -> SourceBindingTargetV1 {
        SourceBindingTargetV1 {
            registration: self.registration,
            identity: self.identity,
        }
    }
    pub fn into_source_save_as_probe(self) -> SourceSaveAsProbeV1 {
        SourceSaveAsProbeV1 {
            epoch: self.epoch,
            registration: self.registration,
            identity: self.identity,
            version: Some(self.validator),
        }
    }
    /// Preserves the original observation; does not promote read to write rights.
    pub fn into_source_export_destination(self) -> SourceExportDestinationV1 {
        SourceExportDestinationV1 {
            registration: self.registration,
            identity: self.identity,
            condition: DispatchCondition::Present {
                epoch: self.epoch,
                bytes: self.bytes,
                validator: self.validator,
            },
        }
    }
}

/// Current host comparison under its declared guarantee, not key inequality.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceRelation {
    Distinct,
    Equivalent,
    Unknown,
}
#[derive(Clone, Debug)]
pub(crate) struct BindingKey {
    pub(crate) identity: Identity,
    pub(crate) registration: Arc<Registration>,
    pub(crate) generation: u64,
}
impl PartialEq for BindingKey {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
            && self.generation == other.generation
            && Arc::ptr_eq(&self.registration, &other.registration)
    }
}
impl Eq for BindingKey {}
/// Read-only exact-basis challenge. Possession is not comparison authority.
#[derive(Clone, Debug)]
pub struct OrdinaryPairCheck {
    pub(crate) owner: Arc<()>,
    pub(crate) marker: Arc<()>,
    pub(crate) document: crate::coordinator::DocumentId,
    pub(crate) source: Option<BindingKey>,
    pub(crate) annotations: Option<BindingKey>,
    pub(crate) proposed: Option<BindingKey>,
}
#[derive(Debug)]
pub struct OrdinaryPairEvidence {
    pub(crate) check: OrdinaryPairCheck,
    pub(crate) relation: ResourceRelation,
}
#[derive(Debug)]
pub struct AnnotationBindingPlan {
    pub(crate) check: OrdinaryPairCheck,
    pub(crate) destination: ResourceRead,
}
impl AnnotationBindingPlan {
    pub fn pair_check(&self) -> &OrdinaryPairCheck {
        &self.check
    }
}

#[derive(Clone)]
pub struct SaveId {
    pub(crate) owner: Arc<()>,
    pub(crate) slot: usize,
}
impl PartialEq for SaveId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.slot == other.slot
    }
}
impl Eq for SaveId {}
impl std::fmt::Debug for SaveId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SaveId(<opaque>)")
    }
}

/// One owned dispatch, already a disclosure to the registered provider.
#[derive(Debug)]
pub struct SaveDispatch {
    pub(crate) result_version_profile: SaveResultVersionProfileV1,
    pub(crate) continuity: Option<Arc<SourceContinuityReceiptV1>>,
    pub(crate) serial: u64,
    pub(crate) task: SaveId,
    pub(crate) registration: Arc<Registration>,
    pub(crate) identity: Identity,
    pub(crate) nonce: Arc<()>,
    pub(crate) payload: Blob,
    pub(crate) condition: DispatchCondition,
    pub(crate) protection: Protection,
    pub(crate) synchronize: bool,
}
impl SaveDispatch {
    pub fn result_version_profile(&self) -> SaveResultVersionProfileV1 {
        self.result_version_profile
    }
    pub fn dispatch_serial(&self) -> u64 {
        self.serial
    }
    pub fn continuity_receipt(&self) -> Option<&SourceContinuityReceiptV1> {
        self.continuity.as_deref()
    }
    pub fn expected_causal_version(&self) -> Option<ResourceVersionV1<'_>> {
        match &self.condition {
            DispatchCondition::Present {
                validator,
                epoch: Some(epoch),
                ..
            } => Some(ResourceVersionV1 {
                epoch: &epoch.0,
                token: &validator.0,
            }),
            DispatchCondition::Present { epoch: None, .. } | DispatchCondition::Absent => None,
        }
    }
    pub fn condition_v2(&self) -> SaveWriteConditionV2<'_> {
        match &self.condition {
            DispatchCondition::Absent => SaveWriteConditionV2::Absent,
            DispatchCondition::Present {
                bytes,
                validator,
                epoch,
            } => SaveWriteConditionV2::Present {
                bytes: &bytes.0,
                version: match epoch {
                    None => SaveVersionV1::Opaque(&validator.0),
                    Some(epoch) => SaveVersionV1::Causal(ResourceVersionV1 {
                        epoch: &epoch.0,
                        token: &validator.0,
                    }),
                },
            },
        }
    }
    pub fn bytes(&self) -> &[u8] {
        &self.payload.0
    }
    pub fn resource_key(&self) -> &[u8] {
        &self.identity.key.0
    }
    /// Original pinned condition, not the latest baseline or a new grant.
    pub fn expected_bytes(&self) -> Option<&[u8]> {
        match &self.condition {
            DispatchCondition::Present { bytes, .. } => Some(&bytes.0),
            DispatchCondition::Absent => None,
        }
    }
    /// Legacy opaque validator only. Typed causal versions require the full
    /// condition_v2/expected_causal_version view; None is not an absence proof.
    pub fn expected_validator(&self) -> Option<&[u8]> {
        match &self.condition {
            DispatchCondition::Present {
                validator,
                epoch: None,
                ..
            } => Some(&validator.0),
            DispatchCondition::Present { epoch: Some(_), .. } => None,
            DispatchCondition::Absent => None,
        }
    }
    /// Refuses versions that this closed legacy view cannot represent losslessly.
    pub fn condition_v1(&self) -> Result<SaveWriteConditionV1<'_>, ResourceError> {
        if self.result_version_profile == SaveResultVersionProfileV1::EpochToken {
            return Err(ResourceError::Unsupported);
        }
        match &self.condition {
            DispatchCondition::Present {
                bytes,
                validator,
                epoch: None,
            } => Ok(SaveWriteConditionV1::Present {
                bytes: &bytes.0,
                validator: &validator.0,
            }),
            DispatchCondition::Present { epoch: Some(_), .. } => Err(ResourceError::Unsupported),
            DispatchCondition::Absent => Ok(SaveWriteConditionV1::Absent),
        }
    }
    pub fn protection(&self) -> Protection {
        self.protection
    }
    pub fn synchronization_required(&self) -> bool {
        self.synchronize
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResultData {
    pub(crate) epoch: Option<Blob>,
    pub(crate) application: Application,
    pub(crate) observed: usize,
    pub(crate) synchronization: Synchronization,
    pub(crate) version: Option<Blob>,
}
impl Default for ResultData {
    fn default() -> Self {
        Self {
            epoch: None,
            application: Application::Unknown,
            observed: 0,
            synchronization: Synchronization::Unknown,
            version: None,
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) enum EvidenceKind {
    CausalEdge(causal::CausalEdge),
    Result(ResultData),
    NoFuture,
    Quiescence,
}
#[derive(Clone, Debug)]
pub struct SaveEvidence {
    pub(crate) task: SaveId,
    pub(crate) registration: Arc<Registration>,
    pub(crate) nonce: Arc<()>,
    pub(crate) kind: EvidenceKind,
}

impl ProviderPort {
    /// Privileged registration-local naming only. No external resource lookup or
    /// existence/freshness/permission fact is inferred from the key.
    pub fn source_binding_target(
        &self,
        key: &[u8],
    ) -> Result<SourceBindingTargetV1, ResourceError> {
        Ok(SourceBindingTargetV1 {
            registration: self.registration.clone(),
            identity: Identity {
                domain: self.registration.domain.clone(),
                key: blob(key, self.registration.limits.max_key_bytes, true)?,
            },
        })
    }
    pub fn source_save_as_probe(
        &self,
        key: &[u8],
        version: Option<&[u8]>,
    ) -> Result<SourceSaveAsProbeV1, ResourceError> {
        Ok(SourceSaveAsProbeV1 {
            epoch: None,
            registration: self.registration.clone(),
            identity: Identity {
                domain: self.registration.domain.clone(),
                key: blob(key, self.registration.limits.max_key_bytes, true)?,
            },
            version: version
                .map(|v| blob(v, self.registration.limits.max_validator_bytes, true))
                .transpose()?,
        })
    }
    /// Privileged absence attestation. Admission still requires an independent
    /// create grant and registered exclusive-create capability.
    pub fn absent_source_export_destination(
        &self,
        key: &[u8],
    ) -> Result<SourceExportDestinationV1, ResourceError> {
        Ok(SourceExportDestinationV1 {
            registration: self.registration.clone(),
            identity: Identity {
                domain: self.registration.domain.clone(),
                key: blob(key, self.registration.limits.max_key_bytes, true)?,
            },
            condition: DispatchCondition::Absent,
        })
    }
    /// First comparison profile: both resources belong to this exact issuer.
    /// Actual observation freshness/alias truth is a privileged host premise,
    /// just as for read_result; the engine rechecks the complete binding basis.
    pub fn attest_ordinary_pair(
        &self,
        check: &OrdinaryPairCheck,
        relation: ResourceRelation,
    ) -> Result<OrdinaryPairEvidence, ResourceError> {
        self.ordinary_pair_keys(check)?;
        Ok(OrdinaryPairEvidence {
            check: check.clone(),
            relation,
        })
    }
    /// Provider-only keys, not public paths or document-derived authority.
    pub fn ordinary_pair_keys<'a>(
        &self,
        check: &'a OrdinaryPairCheck,
    ) -> Result<(Option<&'a [u8]>, &'a [u8]), ResourceError> {
        if !Arc::ptr_eq(&self.registration.owner, &check.owner) {
            return Err(ResourceError::Unavailable);
        }
        let annotation = check
            .proposed
            .as_ref()
            .or(check.annotations.as_ref())
            .ok_or(ResourceError::Unavailable)?;
        if !Arc::ptr_eq(&self.registration, &annotation.registration)
            || check
                .source
                .as_ref()
                .is_some_and(|s| !Arc::ptr_eq(&self.registration, &s.registration))
        {
            return Err(ResourceError::Unsupported);
        }
        Ok((
            check.source.as_ref().map(|s| s.identity.key.0.as_ref()),
            &annotation.identity.key.0,
        ))
    }
    pub fn identity_domain(&self) -> IdentityDomain {
        self.registration.domain.clone()
    }
    /// The caller asserts a successful provider read under its declared consistency
    /// profile, not an atomic snapshot. Ownership prevents accidental read reuse;
    /// freshness/identity truth remains host work.
    pub fn read_result(
        &self,
        key: &[u8],
        validator: &[u8],
        bytes: &[u8],
    ) -> Result<ResourceRead, ResourceError> {
        let limits = self.registration.limits;
        Ok(ResourceRead {
            epoch: None,
            registration: self.registration.clone(),
            identity: Identity {
                domain: self.identity_domain(),
                key: blob(key, limits.max_key_bytes, true)?,
            },
            bytes: blob(bytes, limits.max_blob_bytes, false)?,
            validator: blob(validator, limits.max_validator_bytes, true)?,
        })
    }
    fn attest(
        &self,
        dispatch: &SaveDispatch,
        kind: EvidenceKind,
    ) -> Result<SaveEvidence, ResourceError> {
        if !Arc::ptr_eq(&self.registration, &dispatch.registration) {
            return Err(ResourceError::Unavailable);
        }
        Ok(SaveEvidence {
            task: dispatch.task.clone(),
            registration: self.registration.clone(),
            nonce: dispatch.nonce.clone(),
            kind,
        })
    }
    pub fn result(
        &self,
        dispatch: &SaveDispatch,
        application: Application,
        observed_bytes: usize,
        synchronization: Synchronization,
        version: Option<&[u8]>,
    ) -> Result<SaveEvidence, ResourceError> {
        if !Arc::ptr_eq(&self.registration, &dispatch.registration) {
            return Err(ResourceError::Unavailable);
        }
        if dispatch.result_version_profile == SaveResultVersionProfileV1::EpochToken
            && version.is_some()
        {
            return Err(ResourceError::Unsupported);
        }
        if observed_bytes > dispatch.payload.0.len()
            || (application == Application::Written && observed_bytes != dispatch.payload.0.len())
            || (application == Application::NotWritten && observed_bytes != 0)
        {
            return Err(ResourceError::Malformed);
        }
        let version = version
            .map(|v| blob(v, self.registration.limits.max_validator_bytes, true))
            .transpose()?;
        self.attest(
            dispatch,
            EvidenceKind::Result(ResultData {
                epoch: None,
                application,
                observed: observed_bytes,
                synchronization,
                version,
            }),
        )
    }
    pub fn no_future_effects(
        &self,
        dispatch: &SaveDispatch,
    ) -> Result<SaveEvidence, ResourceError> {
        self.attest(dispatch, EvidenceKind::NoFuture)
    }
    pub fn quiescence(&self, dispatch: &SaveDispatch) -> Result<SaveEvidence, ResourceError> {
        self.attest(dispatch, EvidenceKind::Quiescence)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveOutcome {
    pub result_version_profile: SaveResultVersionProfileV1,
    pub own_chain: bool,
    pub continuity_edges: Option<usize>,
    pub causal_edge_available: bool,
    pub causal_conflict_witnesses: usize,
    /// Historical local activation, not external write acknowledgment or current
    /// binding freshness. Copy export never has an activation intent.
    pub source_save_as_binding_applied: Option<bool>,
    /// Local destination intent only, separate from write/acknowledgment facts.
    pub source_export_reservation_released: Option<bool>,
    pub baseline_capacity: Option<BaselineCapacityState>,
    pub dispatch_serial: Option<u64>,
    pub ordinary_annotation_generation: Option<u64>,
    pub kind: ArtifactKind,
    pub purpose: SavePurpose,
    pub artifact: ResourceArtifactId,
    pub captured_annotation_set: Option<u64>,
    pub captured: DocumentVersion,
    /// Explicit export has no ordinary binding or source serialization settings.
    pub binding_generation: Option<u64>,
    pub serialization_generation: Option<u64>,
    /// Original source-format capture, never refreshed from current document metadata.
    pub source_serialization: Option<SourceSerializationStateV1>,
    pub encoding_context: Option<AnnotationEncodingContext>,
    pub phase: SavePhase,
    pub synchronization_required: bool,
    pub application: Application,
    pub observed_bytes: usize,
    pub synchronization: Synchronization,
    pub version_available: bool,
    pub no_future_effects: bool,
    pub quiescent: bool,
    pub stop_requested: bool,
    pub contested: bool,
}
/// Historical engine facts, not current external freshness or recipient grants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnotationBaselineFact {
    pub producer: SaveId,
    pub artifact: ResourceArtifactId,
    pub source: DocumentVersion,
    pub set_version: u64,
    pub encoding_context: AnnotationEncodingContext,
    pub ordinary_binding_generation: u64,
    pub source_binding_generation: Option<u64>,
    pub dispatch_serial: u64,
    pub required_acknowledgment: Acknowledgment,
    pub qualified_acknowledgment: Acknowledgment,
    pub external_version_available: bool,
}
/// Read-only identity of the original complete provider registration, including
/// its independent incarnation. It is not a provider issuer or a write grant.
#[derive(Clone)]
pub struct ReadProviderContext {
    pub(crate) registration: Arc<Registration>,
}
impl PartialEq for ReadProviderContext {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.registration, &other.registration)
    }
}
impl Eq for ReadProviderContext {}
impl std::fmt::Debug for ReadProviderContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadProviderContext(<opaque>)")
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedAnnotationBaselineFact {
    pub origin: AnnotationReadId,
    pub source: DocumentVersion,
    pub set_version: u64,
    pub restoration_context: RestorationContext,
    pub encoding_context: AnnotationEncodingContext,
    pub reader: ReadProviderContext,
    pub ordinary_binding_generation: u64,
    pub source_binding_generation: Option<u64>,
    pub coherence: ReadStrength,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotationBaselineEvidence {
    Written(AnnotationBaselineFact),
    Loaded(LoadedAnnotationBaselineFact),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedState {
    pub content: ContentComparison,
    pub queued: usize,
    pub unresolved: usize,
    pub contested: bool,
}

/// Trusted-owner gauges; neither authorization nor a retirement certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceUsage {
    pub continuity_retained_bytes: usize,
    pub source_binding_history_entries: usize,
    pub source_export_reservations: usize,
    pub retained_source_bindings: usize,
    pub retained_annotation_bindings: usize,
    /// Same-owner read/writer relations, not read origins or effect successes.
    pub read_baseline_attachments: usize,
    pub restoration_boundaries: usize,
    pub restored_reads: usize,
    /// Legacy annotation-reader inventory; source inventory has its own gauge.
    pub retained_reads: usize,
    pub issued_reads_lifetime: usize,
    pub held_read_bytes: usize,
    pub submitted_reads: usize,
    pub contested_reads: usize,
    pub annotation_save_admission_stopped: bool,
    pub pending_baseline_publications: usize,
    pub reserved_baseline_bytes: usize,
    pub annotation_baselines: usize,
    /// Logical descriptor + referenced byte occupancy; overlaps artifact gauges,
    /// not a separate physical allocation total or a release certificate.
    pub annotation_baseline_logical_bytes: usize,
    pub annotation_bindings: usize,
    pub encoders: usize,
    pub issued_artifacts_lifetime: usize,
    pub retained_artifacts: usize,
    pub held_artifact_bytes: usize,
    /// Codec preparations reserve one live/issued artifact credit. Their bytes
    /// are already included in held_artifact_bytes, not an additional total.
    pub prepared_sources: usize,
    pub prepared_source_bytes: usize,
    pub enabled: bool,
    pub providers: usize,
    pub bindings: usize,
    pub issued_lifetime: usize,
    pub retained: usize,
    pub queued: usize,
    /// All nonquiescent source/annotation reads and dispatched writes.
    pub outstanding: usize,
    pub unresolved: usize,
    pub contested_tasks: usize,
    pub max_live_tasks: usize,
    pub max_issued_tasks: usize,
    pub max_outstanding: usize,
}
