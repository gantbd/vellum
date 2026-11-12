//! Authority-complete copy export effects, separate from source rebinding.
//!
//! The destination locator is retained only inside the trusted effect request.
//! Receipts and diagnostics expose opaque source and decision evidence, never a
//! path. An authorized request is one-use and cannot be cloned into a second
//! filesystem effect.

use crate::authority::{
    AuthorityActionV1, AuthorityDecisionRefV1, AuthorityRequestV1, AuthorityResourceRefV1,
    SaveAsDestinationDecisionRefV1, SaveAsDestinationEffectV1, SaveAsDestinationEvidenceV1,
    SaveAsDestinationObjectV1, SaveAsDestinationRequestV1,
};
use std::{
    fmt,
    path::{Component, Path},
};
use vellum_engine::coordinator::{DocumentVersion, SourceSerializationStateV1};
use vellum_engine::instrumentation::Recorder;
use vellum_engine::resource::CapturedSourceSerializationV1;

/// Opaque identity of the exact source serialization captured for this export.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExportCopySourceWitnessV1([u8; 16]);

impl ExportCopySourceWitnessV1 {
    pub(crate) const fn new(value: [u8; 16]) -> Self {
        Self(value)
    }
}

impl fmt::Debug for ExportCopySourceWitnessV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ExportCopySourceWitnessV1(<opaque>)")
    }
}

/// Closed physical effect requested of a qualified destination.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportCopyDispositionV1 {
    OverwriteExisting,
    CreateExclusive,
}

impl From<ExportCopyDispositionV1> for SaveAsDestinationEffectV1 {
    fn from(value: ExportCopyDispositionV1) -> Self {
        match value {
            ExportCopyDispositionV1::OverwriteExisting => Self::OverwriteExisting,
            ExportCopyDispositionV1::CreateExclusive => Self::CreateExclusive,
        }
    }
}

/// Why authority-complete request construction failed before adapter I/O.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportCopyAuthorizationRefusalV1 {
    SourceDenied,
    SourceUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportCopyAttachmentRefusalV1 {
    SourceChanged,
}

/// The complete source-disclosure policy question. The ordinary authority
/// request binds authenticated subject/session, resource, and action; the
/// native version additionally binds the exact document incarnation and
/// revision whose serialized bytes would be disclosed.
#[derive(Clone, Eq, PartialEq)]
pub struct ExportCopySourceRequestV1 {
    authority: AuthorityRequestV1,
    source: DocumentVersion,
}

impl ExportCopySourceRequestV1 {
    pub fn new(
        authority: AuthorityRequestV1,
        source: DocumentVersion,
    ) -> Result<Self, ExportCopyAuthorizationRefusalV1> {
        if authority.action != AuthorityActionV1::ExportSourceCopy {
            return Err(ExportCopyAuthorizationRefusalV1::SourceDenied);
        }
        Ok(Self { authority, source })
    }

    pub const fn authority(&self) -> AuthorityRequestV1 {
        self.authority
    }

    pub const fn source(&self) -> &DocumentVersion {
        &self.source
    }
}

impl fmt::Debug for ExportCopySourceRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExportCopySourceRequestV1")
            .field("authority", &self.authority)
            .field("source", &"<opaque>")
            .finish()
    }
}

/// Non-cloneable attribution for the exact source-disclosure decision. Unlike
/// generic action evidence, this retains the opaque document incarnation and
/// revision presented to the composite interpreter.
pub struct ExportCopySourceDecisionEvidenceV1 {
    decision: AuthorityDecisionRefV1,
    request: ExportCopySourceRequestV1,
}

impl ExportCopySourceDecisionEvidenceV1 {
    pub fn new(decision: AuthorityDecisionRefV1, request: ExportCopySourceRequestV1) -> Self {
        Self { decision, request }
    }

    pub const fn decision_ref(&self) -> AuthorityDecisionRefV1 {
        self.decision
    }

    pub const fn request(&self) -> &ExportCopySourceRequestV1 {
        &self.request
    }
}

impl fmt::Debug for ExportCopySourceDecisionEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExportCopySourceDecisionEvidenceV1")
            .field("decision", &self.decision)
            .field("request", &self.request)
            .finish()
    }
}

/// One atomic decision over the source-copy action and destination object.
/// Implementations must not consume either sub-authority unless they can
/// return the complete grant.
#[allow(
    clippy::large_enum_variant,
    reason = "the one-use grant remains an allocation-free atomic product; heap indirection would add an unmodelled allocation failure to authority evaluation"
)]
pub enum ExportCopyAuthorityDecisionV1 {
    Granted {
        source: ExportCopySourceDecisionEvidenceV1,
        destination: SaveAsDestinationEvidenceV1,
    },
    Denied,
    Unavailable,
}

impl fmt::Debug for ExportCopyAuthorityDecisionV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Granted {
                source,
                destination,
            } => formatter
                .debug_struct("Granted")
                .field("source", source)
                .field("destination", destination)
                .finish(),
            Self::Denied => formatter.write_str("Denied"),
            Self::Unavailable => formatter.write_str("Unavailable"),
        }
    }
}

pub trait ExportCopyAuthorityPortV1 {
    fn authorize_export_copy(
        &mut self,
        source: ExportCopySourceRequestV1,
        destination: SaveAsDestinationRequestV1,
    ) -> ExportCopyAuthorityDecisionV1;
}

/// One-use exact composite authority for trusted local composition roots.
/// The installed destination object—not its locator—is consumed on grant.
pub struct FixedExportCopyAuthorityV1 {
    source: ExportCopySourceRequestV1,
    destination: SaveAsDestinationRequestV1,
    object: Option<SaveAsDestinationObjectV1>,
    available: bool,
}

impl FixedExportCopyAuthorityV1 {
    pub fn install(
        source: ExportCopySourceRequestV1,
        destination: SaveAsDestinationRequestV1,
        object: SaveAsDestinationObjectV1,
    ) -> Result<Self, ExportCopyAuthorizationRefusalV1> {
        if source.authority.action != AuthorityActionV1::ExportSourceCopy
            || source.authority.context != destination.context
            || source.authority.resource != destination.resource
            || !destination.destination().is_absolute()
            || destination
                .destination()
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
            || destination.effect != SaveAsDestinationEffectV1::CreateExclusive
            || object.request() != &destination
        {
            return Err(ExportCopyAuthorizationRefusalV1::SourceDenied);
        }
        Ok(Self {
            source,
            destination,
            object: Some(object),
            available: true,
        })
    }

    pub fn attenuate(&mut self) {
        self.available = false;
        self.object = None;
    }
}

impl ExportCopyAuthorityPortV1 for FixedExportCopyAuthorityV1 {
    fn authorize_export_copy(
        &mut self,
        source: ExportCopySourceRequestV1,
        destination: SaveAsDestinationRequestV1,
    ) -> ExportCopyAuthorityDecisionV1 {
        if source != self.source || destination != self.destination {
            return ExportCopyAuthorityDecisionV1::Denied;
        }
        if !self.available {
            return ExportCopyAuthorityDecisionV1::Unavailable;
        }
        let Some(object) = self.object.take() else {
            return ExportCopyAuthorityDecisionV1::Unavailable;
        };
        self.available = false;
        ExportCopyAuthorityDecisionV1::Granted {
            source: ExportCopySourceDecisionEvidenceV1::new(
                AuthorityDecisionRefV1::new(*b"vellum-exportsrc"),
                source,
            ),
            destination: SaveAsDestinationEvidenceV1::with_object(
                SaveAsDestinationDecisionRefV1::new(*b"vellum-exportdst"),
                destination,
                object,
            ),
        }
    }
}

/// One-use proof that the composite interpreter granted the exact source-copy
/// and destination effects. It contains no source bytes: embeddings can
/// and should obtain this plan before asking the codec to disclose a capture.
pub struct AuthorizedExportCopyPlanV1 {
    source_authority: ExportCopySourceDecisionEvidenceV1,
    destination: SaveAsDestinationEvidenceV1,
    source: DocumentVersion,
    serialization: SourceSerializationStateV1,
}

impl fmt::Debug for AuthorizedExportCopyPlanV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedExportCopyPlanV1")
            .field("source_authority", &self.source_authority)
            .field("destination", &self.destination)
            .finish()
    }
}

impl AuthorizedExportCopyPlanV1 {
    /// Consult both interpreters without capturing or receiving source content.
    pub fn authorize<A: ExportCopyAuthorityPortV1 + ?Sized>(
        authority: &mut A,
        source_request: AuthorityRequestV1,
        destination_request: SaveAsDestinationRequestV1,
        source: DocumentVersion,
        serialization: SourceSerializationStateV1,
    ) -> Result<Self, ExportCopyAuthorizationRefusalV1> {
        if !destination_request.destination().is_absolute()
            || destination_request
                .destination()
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
            || source_request.action != AuthorityActionV1::ExportSourceCopy
            || source_request.context != destination_request.context
            || source_request.resource != destination_request.resource
            || destination_request.effect != SaveAsDestinationEffectV1::CreateExclusive
        {
            return Err(ExportCopyAuthorizationRefusalV1::SourceDenied);
        }
        let source_question = ExportCopySourceRequestV1::new(source_request, source.clone())?;
        let expected_source_question = source_question.clone();
        let expected_destination_request = destination_request.clone();
        let (source_authority, destination) =
            match authority.authorize_export_copy(source_question, destination_request) {
                ExportCopyAuthorityDecisionV1::Granted {
                    source,
                    destination,
                } if source.request() == &expected_source_question
                    && destination.request() == &expected_destination_request
                    && destination.has_congruent_object() =>
                {
                    (source, destination)
                }
                ExportCopyAuthorityDecisionV1::Granted { .. } => {
                    return Err(ExportCopyAuthorizationRefusalV1::SourceUnavailable);
                }
                ExportCopyAuthorityDecisionV1::Denied => {
                    return Err(ExportCopyAuthorizationRefusalV1::SourceDenied);
                }
                ExportCopyAuthorityDecisionV1::Unavailable => {
                    return Err(ExportCopyAuthorizationRefusalV1::SourceUnavailable);
                }
            };
        Ok(Self {
            source_authority,
            destination,
            source,
            serialization,
        })
    }

    /// Attach exactly one engine/codec-owned immutable serialization. The
    /// carrier is consumed and cannot be reused to authorize another effect.
    pub fn attach_source_capture(
        self,
        capture: CapturedSourceSerializationV1,
    ) -> Result<AuthorizedExportCopyV1, ExportCopyAttachmentRefusalV1> {
        if capture.source() != &self.source || capture.serialization() != self.serialization {
            return Err(ExportCopyAttachmentRefusalV1::SourceChanged);
        }
        let (witness, bytes) = capture.into_parts();
        Ok(AuthorizedExportCopyV1 {
            source: ExportCopySourceWitnessV1::new(witness.into_bytes()),
            source_version: self.source,
            serialization: self.serialization,
            bytes,
            source_authority: self.source_authority,
            destination: self.destination,
        })
    }
}

/// A one-use request carrying both independent authority provenances.
pub struct AuthorizedExportCopyV1 {
    source: ExportCopySourceWitnessV1,
    source_version: DocumentVersion,
    serialization: SourceSerializationStateV1,
    bytes: Vec<u8>,
    source_authority: ExportCopySourceDecisionEvidenceV1,
    destination: SaveAsDestinationEvidenceV1,
}

impl fmt::Debug for AuthorizedExportCopyV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedExportCopyV1")
            .field("source", &self.source)
            .field("bytes", &"<redacted>")
            .field("source_authority", &self.source_authority)
            .field("destination", &self.destination)
            .finish()
    }
}

impl AuthorizedExportCopyV1 {
    pub fn source(&self) -> ExportCopySourceWitnessV1 {
        self.source
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn destination(&self) -> &Path {
        self.destination.request().destination()
    }

    pub fn disposition(&self) -> ExportCopyDispositionV1 {
        match self.destination.request().effect {
            SaveAsDestinationEffectV1::OverwriteExisting => {
                ExportCopyDispositionV1::OverwriteExisting
            }
            SaveAsDestinationEffectV1::CreateExclusive => ExportCopyDispositionV1::CreateExclusive,
        }
    }

    pub fn source_evidence(&self) -> &ExportCopySourceDecisionEvidenceV1 {
        &self.source_authority
    }

    pub fn destination_evidence(&self) -> &SaveAsDestinationEvidenceV1 {
        &self.destination
    }

    /// Split trusted internal effect material from the locator-free public
    /// evidence projection. The adapter must drop path and bytes at return.
    pub fn into_effect_parts(
        self,
    ) -> (
        ExportCopyEvidenceV1,
        SaveAsDestinationRequestV1,
        Vec<u8>,
        Option<SaveAsDestinationObjectV1>,
    ) {
        let disposition = self.disposition();
        let destination_decision = self.destination.decision_ref();
        let evidence = ExportCopyEvidenceV1 {
            source: self.source,
            source_version: self.source_version,
            serialization: self.serialization,
            disposition,
            resource: self.source_authority.request().authority().resource,
            source_decision: self.source_authority.decision_ref(),
            destination_decision,
        };
        let (_, destination, object) = self.destination.into_parts();
        (evidence, destination, self.bytes, object)
    }
}

/// Locator- and content-free evidence retained by a public receipt.
#[derive(Clone, Eq, PartialEq)]
pub struct ExportCopyEvidenceV1 {
    source: ExportCopySourceWitnessV1,
    source_version: DocumentVersion,
    serialization: SourceSerializationStateV1,
    disposition: ExportCopyDispositionV1,
    resource: AuthorityResourceRefV1,
    source_decision: AuthorityDecisionRefV1,
    destination_decision: SaveAsDestinationDecisionRefV1,
}

impl ExportCopyEvidenceV1 {
    pub const fn source(&self) -> ExportCopySourceWitnessV1 {
        self.source
    }

    pub const fn disposition(&self) -> ExportCopyDispositionV1 {
        self.disposition
    }
}

impl fmt::Debug for ExportCopyEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExportCopyEvidenceV1")
            .field("source", &self.source)
            .field("source_version", &"<opaque>")
            .field("serialization", &"<opaque>")
            .field("disposition", &self.disposition)
            .field("resource", &self.resource)
            .field("source_decision", &self.source_decision)
            .field("destination_decision", &self.destination_decision)
            .finish()
    }
}

/// Truthful crash-persistence classification after a synchronous adapter return.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportCopyPersistenceV1 {
    Durable,
    NotDurable,
    Unknown,
    NotAttempted,
}

/// Persistence classifications which may truthfully accompany a failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailedExportCopyPersistenceV1 {
    NotDurable,
    Unknown,
    NotAttempted,
}

impl From<FailedExportCopyPersistenceV1> for ExportCopyPersistenceV1 {
    fn from(value: FailedExportCopyPersistenceV1) -> Self {
        match value {
            FailedExportCopyPersistenceV1::NotDurable => Self::NotDurable,
            FailedExportCopyPersistenceV1::Unknown => Self::Unknown,
            FailedExportCopyPersistenceV1::NotAttempted => Self::NotAttempted,
        }
    }
}

/// Content-free physical refusal/failure classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportCopyFailureV1 {
    Conflict,
    Unsupported,
    Limit,
    Capacity,
    Io,
}

/// Receipt for exactly one authorized copy export. It deliberately has no
/// locator and makes no source-binding or rollback claim.
pub struct ExportCopyReceiptV1 {
    evidence: ExportCopyEvidenceV1,
    persistence: ExportCopyPersistenceV1,
    failure: Option<ExportCopyFailureV1>,
    cleanup_failed: bool,
}

impl ExportCopyReceiptV1 {
    pub fn durable(evidence: ExportCopyEvidenceV1) -> Self {
        Self {
            evidence,
            persistence: ExportCopyPersistenceV1::Durable,
            failure: None,
            cleanup_failed: false,
        }
    }

    pub fn failed(
        evidence: ExportCopyEvidenceV1,
        persistence: FailedExportCopyPersistenceV1,
        failure: ExportCopyFailureV1,
        cleanup_failed: bool,
    ) -> Self {
        Self {
            evidence,
            persistence: persistence.into(),
            failure: Some(failure),
            cleanup_failed,
        }
    }

    pub fn source(&self) -> ExportCopySourceWitnessV1 {
        self.evidence.source()
    }

    pub fn disposition(&self) -> ExportCopyDispositionV1 {
        self.evidence.disposition()
    }

    pub const fn source_revision(&self) -> u64 {
        self.evidence.source_version.revision
    }

    pub const fn persistence(&self) -> ExportCopyPersistenceV1 {
        self.persistence
    }

    pub const fn failure(&self) -> Option<ExportCopyFailureV1> {
        self.failure
    }

    pub const fn cleanup_failed(&self) -> bool {
        self.cleanup_failed
    }

    pub const fn evidence(&self) -> &ExportCopyEvidenceV1 {
        &self.evidence
    }
}

impl fmt::Debug for ExportCopyReceiptV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExportCopyReceiptV1")
            .field("source", &self.source())
            .field("disposition", &self.disposition())
            .field("persistence", &self.persistence)
            .field("failure", &self.failure)
            .field("cleanup_failed", &self.cleanup_failed)
            .field("evidence", &self.evidence)
            .finish()
    }
}

/// Adapter-neutral interpreter for the closed copy-export effect.
///
/// The authorized carrier is affine: it is accepted by value, is not
/// cloneable, and therefore can cause at most one adapter invocation.  Host
/// owns this operation and its receipt algebra; adapters retain ownership of
/// destination-object decoding and physical publication.
///
/// ```compile_fail
/// use vellum_engine::instrumentation::Recorder;
/// use vellum_host::export_copy::{
///     AuthorizedExportCopyV1, ExportCopyEffectPortV1,
/// };
///
/// fn replay(
///     port: &mut impl ExportCopyEffectPortV1,
///     authorization: AuthorizedExportCopyV1,
///     recorder: &mut Recorder,
/// ) {
///     let _ = port.execute_export_copy(authorization, recorder);
///     let _ = port.execute_export_copy(authorization, recorder);
/// }
/// ```
pub trait ExportCopyEffectPortV1 {
    fn execute_export_copy(
        &mut self,
        authorization: AuthorizedExportCopyV1,
        recorder: &mut Recorder,
    ) -> ExportCopyReceiptV1;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authority::{
        AuthorityContextV1, AuthorityResourceRefV1, AuthoritySessionRefV1, AuthoritySubjectRefV1,
    };
    use std::path::PathBuf;

    fn source_version() -> DocumentVersion {
        let mut owner = vellum_engine::coordinator::SourceCoordinator::new(
            vellum_engine::coordinator::CoordinatorLimits {
                max_document_incarnations: 1,
                max_participants: 1,
                max_staged_bytes: 8,
                source: vellum_engine::source::Limits {
                    max_source_bytes: 8,
                    max_edits: 1,
                    max_edit_payload_bytes: 8,
                },
            },
        );
        owner.create(b"source", &mut Recorder::disabled()).unwrap()
    }

    fn requests() -> (ExportCopySourceRequestV1, SaveAsDestinationRequestV1) {
        let context = AuthorityContextV1 {
            subject: AuthoritySubjectRefV1::new([1; 16]),
            session: Some(AuthoritySessionRefV1::new([2; 16])),
        };
        let resource = AuthorityResourceRefV1::new([3; 16]);
        (
            ExportCopySourceRequestV1::new(
                AuthorityRequestV1 {
                    context,
                    resource,
                    action: AuthorityActionV1::ExportSourceCopy,
                },
                source_version(),
            )
            .unwrap(),
            SaveAsDestinationRequestV1::new(
                context,
                resource,
                PathBuf::from("/private/secret/source.rs"),
                SaveAsDestinationEffectV1::CreateExclusive,
            ),
        )
    }

    #[test]
    fn composite_denial_consumes_neither_half_and_exact_grant_is_one_use() {
        let (source, destination) = requests();
        let mut authority = FixedExportCopyAuthorityV1::install(
            source.clone(),
            destination.clone(),
            SaveAsDestinationObjectV1::install(destination.clone(), 17_u8),
        )
        .unwrap();
        let mut wrong = destination.clone();
        wrong.effect = SaveAsDestinationEffectV1::OverwriteExisting;
        assert!(matches!(
            authority.authorize_export_copy(source.clone(), wrong),
            ExportCopyAuthorityDecisionV1::Denied
        ));
        let granted = authority.authorize_export_copy(source.clone(), destination.clone());
        let ExportCopyAuthorityDecisionV1::Granted { destination, .. } = granted else {
            panic!("exact composite request must grant after a denial")
        };
        let (_, _, object) = destination.into_parts();
        assert_eq!(object.unwrap().downcast::<u8>().unwrap(), 17);
        assert!(matches!(
            authority.authorize_export_copy(source, destination_request()),
            ExportCopyAuthorityDecisionV1::Unavailable
        ));
    }

    #[test]
    fn same_leaf_object_from_another_parent_cannot_enter_the_authority_product() {
        let (source, destination) = requests();
        let other_parent = SaveAsDestinationRequestV1::new(
            destination.context,
            destination.resource,
            PathBuf::from("/private/other/source.rs"),
            destination.effect,
        );
        let object = SaveAsDestinationObjectV1::install(other_parent, 17_u8);

        assert!(matches!(
            FixedExportCopyAuthorityV1::install(source, destination, object),
            Err(ExportCopyAuthorizationRefusalV1::SourceDenied)
        ));
    }

    fn destination_request() -> SaveAsDestinationRequestV1 {
        requests().1
    }

    #[test]
    fn generic_action_grant_for_another_source_version_is_refused() {
        struct LyingAuthority {
            wrong_source: Option<ExportCopySourceRequestV1>,
        }

        impl ExportCopyAuthorityPortV1 for LyingAuthority {
            fn authorize_export_copy(
                &mut self,
                _source: ExportCopySourceRequestV1,
                destination: SaveAsDestinationRequestV1,
            ) -> ExportCopyAuthorityDecisionV1 {
                ExportCopyAuthorityDecisionV1::Granted {
                    source: ExportCopySourceDecisionEvidenceV1::new(
                        AuthorityDecisionRefV1::new([7; 16]),
                        self.wrong_source.take().unwrap(),
                    ),
                    destination: SaveAsDestinationEvidenceV1::with_object(
                        SaveAsDestinationDecisionRefV1::new([8; 16]),
                        destination.clone(),
                        SaveAsDestinationObjectV1::install(destination, ()),
                    ),
                }
            }
        }

        let mut owner = vellum_engine::coordinator::SourceCoordinator::new(
            vellum_engine::coordinator::CoordinatorLimits {
                max_document_incarnations: 2,
                max_participants: 1,
                max_staged_bytes: 16,
                source: vellum_engine::source::Limits {
                    max_source_bytes: 8,
                    max_edits: 1,
                    max_edit_payload_bytes: 8,
                },
            },
        );
        let expected = owner.create(b"first", &mut Recorder::disabled()).unwrap();
        let other = owner.create(b"other", &mut Recorder::disabled()).unwrap();
        let serialization = owner
            .source_serialization(&expected.document, &mut Recorder::disabled())
            .unwrap();
        let context = AuthorityContextV1 {
            subject: AuthoritySubjectRefV1::new([1; 16]),
            session: Some(AuthoritySessionRefV1::new([2; 16])),
        };
        let resource = AuthorityResourceRefV1::new([3; 16]);
        let source_request = AuthorityRequestV1 {
            context,
            resource,
            action: AuthorityActionV1::ExportSourceCopy,
        };
        let destination = SaveAsDestinationRequestV1::new(
            context,
            resource,
            PathBuf::from("/private/secret/source.rs"),
            SaveAsDestinationEffectV1::CreateExclusive,
        );
        let mut authority = LyingAuthority {
            wrong_source: Some(ExportCopySourceRequestV1::new(source_request, other).unwrap()),
        };

        assert!(matches!(
            AuthorizedExportCopyPlanV1::authorize(
                &mut authority,
                source_request,
                destination,
                expected,
                serialization,
            ),
            Err(ExportCopyAuthorizationRefusalV1::SourceUnavailable)
        ));
    }
}
