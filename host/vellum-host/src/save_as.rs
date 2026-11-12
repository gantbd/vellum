//! Atomic, adapter-neutral authority for an existing-destination Save As.
//!
//! Save As is the product of three independent decisions: persistence of one
//! exact source version, rebinding that same version, and replacement of one
//! exact host-installed destination object.  The product is affine and is
//! completed before source bytes are captured or any effect adapter is called.

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
use vellum_engine::resource::CapturedSourceSerializationV1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveAsAuthorizationRefusalV1 {
    Denied,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveAsAttachmentRefusalV1 {
    SourceChanged,
    AuthorityProductIncomplete,
}

/// Opaque identity of the exact engine serialization captured for Save As.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SaveAsSourceWitnessV1([u8; 16]);

impl fmt::Debug for SaveAsSourceWitnessV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SaveAsSourceWitnessV1(<opaque>)")
    }
}

/// The complete policy question for one existing-destination Save As.
#[derive(Clone, Eq, PartialEq)]
pub struct SaveAsAuthorityRequestV1 {
    persist: AuthorityRequestV1,
    rebind: AuthorityRequestV1,
    source: DocumentVersion,
    destination: SaveAsDestinationRequestV1,
}

impl SaveAsAuthorityRequestV1 {
    pub fn new(
        persist: AuthorityRequestV1,
        rebind: AuthorityRequestV1,
        source: DocumentVersion,
        destination: SaveAsDestinationRequestV1,
    ) -> Result<Self, SaveAsAuthorizationRefusalV1> {
        if persist.action != AuthorityActionV1::PersistSource
            || rebind.action != AuthorityActionV1::RebindSource
            || persist.context != rebind.context
            || persist.resource != rebind.resource
            || persist.context != destination.context
            || persist.resource != destination.resource
            || destination.effect != SaveAsDestinationEffectV1::OverwriteExisting
            || !valid_destination(destination.destination())
        {
            return Err(SaveAsAuthorizationRefusalV1::Denied);
        }
        Ok(Self {
            persist,
            rebind,
            source,
            destination,
        })
    }

    pub const fn persist(&self) -> AuthorityRequestV1 {
        self.persist
    }

    pub const fn rebind(&self) -> AuthorityRequestV1 {
        self.rebind
    }

    pub const fn source(&self) -> &DocumentVersion {
        &self.source
    }

    pub const fn destination(&self) -> &SaveAsDestinationRequestV1 {
        &self.destination
    }
}

impl fmt::Debug for SaveAsAuthorityRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SaveAsAuthorityRequestV1")
            .field("persist", &self.persist)
            .field("rebind", &self.rebind)
            .field("source", &"<opaque>")
            .field("destination", &self.destination)
            .finish()
    }
}

fn valid_destination(destination: &Path) -> bool {
    destination.is_absolute()
        && !destination
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
}

/// Non-cloneable attribution for an exact action and source version.
pub struct SaveAsActionEvidenceV1 {
    decision: AuthorityDecisionRefV1,
    request: AuthorityRequestV1,
    source: DocumentVersion,
}

impl SaveAsActionEvidenceV1 {
    pub fn new(
        decision: AuthorityDecisionRefV1,
        request: AuthorityRequestV1,
        source: DocumentVersion,
    ) -> Self {
        Self {
            decision,
            request,
            source,
        }
    }

    pub const fn decision_ref(&self) -> AuthorityDecisionRefV1 {
        self.decision
    }

    pub const fn request(&self) -> AuthorityRequestV1 {
        self.request
    }

    pub const fn source(&self) -> &DocumentVersion {
        &self.source
    }
}

impl fmt::Debug for SaveAsActionEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SaveAsActionEvidenceV1")
            .field("decision", &self.decision)
            .field("request", &self.request)
            .field("source", &"<opaque>")
            .finish()
    }
}

#[allow(
    clippy::large_enum_variant,
    reason = "the affine authority product stays allocation-free and atomic"
)]
pub enum SaveAsAuthorityDecisionV1 {
    Granted {
        persist: SaveAsActionEvidenceV1,
        rebind: SaveAsActionEvidenceV1,
        destination: SaveAsDestinationEvidenceV1,
    },
    Denied,
    Unavailable,
}

pub trait SaveAsAuthorityPortV1 {
    fn authorize_save_as(&mut self, request: SaveAsAuthorityRequestV1)
    -> SaveAsAuthorityDecisionV1;
}

/// One-use exact interpreter for trusted composition roots and fake-port BDDs.
pub struct FixedSaveAsAuthorityV1 {
    request: SaveAsAuthorityRequestV1,
    object: Option<SaveAsDestinationObjectV1>,
    available: bool,
}

impl FixedSaveAsAuthorityV1 {
    pub fn install(
        request: SaveAsAuthorityRequestV1,
        object: SaveAsDestinationObjectV1,
    ) -> Result<Self, SaveAsAuthorizationRefusalV1> {
        if object.request() != request.destination() {
            return Err(SaveAsAuthorizationRefusalV1::Denied);
        }
        Ok(Self {
            request,
            object: Some(object),
            available: true,
        })
    }

    pub fn attenuate(&mut self) {
        self.available = false;
        self.object = None;
    }
}

impl SaveAsAuthorityPortV1 for FixedSaveAsAuthorityV1 {
    fn authorize_save_as(
        &mut self,
        request: SaveAsAuthorityRequestV1,
    ) -> SaveAsAuthorityDecisionV1 {
        if request != self.request {
            return SaveAsAuthorityDecisionV1::Denied;
        }
        if !self.available {
            return SaveAsAuthorityDecisionV1::Unavailable;
        }
        let Some(object) = self.object.take() else {
            return SaveAsAuthorityDecisionV1::Unavailable;
        };
        self.available = false;
        SaveAsAuthorityDecisionV1::Granted {
            persist: SaveAsActionEvidenceV1::new(
                AuthorityDecisionRefV1::new(*b"vellum-save-pers"),
                request.persist,
                request.source.clone(),
            ),
            rebind: SaveAsActionEvidenceV1::new(
                AuthorityDecisionRefV1::new(*b"vellum-save-rebi"),
                request.rebind,
                request.source,
            ),
            destination: SaveAsDestinationEvidenceV1::with_object(
                SaveAsDestinationDecisionRefV1::new(*b"vellum-save-dest"),
                request.destination,
                object,
            ),
        }
    }
}

/// Authority-complete plan. Source content cannot enter until this exists.
pub struct AuthorizedSaveAsPlanV1 {
    persist: SaveAsActionEvidenceV1,
    rebind: SaveAsActionEvidenceV1,
    destination: SaveAsDestinationEvidenceV1,
    source: DocumentVersion,
    serialization: SourceSerializationStateV1,
}

impl fmt::Debug for AuthorizedSaveAsPlanV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedSaveAsPlanV1")
            .field("persist", &self.persist)
            .field("rebind", &self.rebind)
            .field("destination", &self.destination)
            .finish()
    }
}

impl AuthorizedSaveAsPlanV1 {
    pub fn authorize<A: SaveAsAuthorityPortV1 + ?Sized>(
        authority: &mut A,
        persist: AuthorityRequestV1,
        rebind: AuthorityRequestV1,
        destination: SaveAsDestinationRequestV1,
        source: DocumentVersion,
        serialization: SourceSerializationStateV1,
    ) -> Result<Self, SaveAsAuthorizationRefusalV1> {
        let request = SaveAsAuthorityRequestV1::new(persist, rebind, source.clone(), destination)?;
        let expected = request.clone();
        match authority.authorize_save_as(request) {
            SaveAsAuthorityDecisionV1::Granted {
                persist,
                rebind,
                destination,
            } if persist.request() == expected.persist()
                && persist.source() == expected.source()
                && rebind.request() == expected.rebind()
                && rebind.source() == expected.source()
                && destination.request() == expected.destination()
                && destination.has_congruent_object() =>
            {
                Ok(Self {
                    persist,
                    rebind,
                    destination,
                    source,
                    serialization,
                })
            }
            SaveAsAuthorityDecisionV1::Granted { .. } | SaveAsAuthorityDecisionV1::Unavailable => {
                Err(SaveAsAuthorizationRefusalV1::Unavailable)
            }
            SaveAsAuthorityDecisionV1::Denied => Err(SaveAsAuthorizationRefusalV1::Denied),
        }
    }

    pub fn attach_source_capture(
        self,
        capture: CapturedSourceSerializationV1,
    ) -> Result<AuthorizedSaveAsV1, SaveAsAttachmentRefusalV1> {
        if capture.source() != &self.source || capture.serialization() != self.serialization {
            return Err(SaveAsAttachmentRefusalV1::SourceChanged);
        }
        let (witness, bytes) = capture.into_parts();
        Ok(AuthorizedSaveAsV1 {
            source_witness: SaveAsSourceWitnessV1(witness.into_bytes()),
            source: self.source,
            serialization: self.serialization,
            bytes,
            persist: self.persist,
            rebind: self.rebind,
            destination: self.destination,
        })
    }
}

/// Affine adapter handoff carrying exact bytes and the complete authority product.
pub struct AuthorizedSaveAsV1 {
    source_witness: SaveAsSourceWitnessV1,
    source: DocumentVersion,
    serialization: SourceSerializationStateV1,
    bytes: Vec<u8>,
    persist: SaveAsActionEvidenceV1,
    rebind: SaveAsActionEvidenceV1,
    destination: SaveAsDestinationEvidenceV1,
}

impl fmt::Debug for AuthorizedSaveAsV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizedSaveAsV1")
            .field("source", &"<opaque>")
            .field("bytes", &"<redacted>")
            .field("persist", &self.persist)
            .field("rebind", &self.rebind)
            .field("destination", &self.destination)
            .finish()
    }
}

impl AuthorizedSaveAsV1 {
    pub const fn source_version(&self) -> &DocumentVersion {
        &self.source
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn destination(&self) -> &Path {
        self.destination.request().destination()
    }

    pub fn into_effect_parts(
        self,
    ) -> Result<
        (
            SaveAsEvidenceV1,
            SaveAsDestinationRequestV1,
            Vec<u8>,
            SaveAsDestinationObjectV1,
        ),
        SaveAsAttachmentRefusalV1,
    > {
        let evidence = SaveAsEvidenceV1 {
            source_witness: self.source_witness,
            source: self.source,
            serialization: self.serialization,
            resource: self.persist.request().resource,
            persist_decision: self.persist.decision_ref(),
            rebind_decision: self.rebind.decision_ref(),
            destination_decision: self.destination.decision_ref(),
        };
        let (_, destination, object) = self.destination.into_parts();
        let object = object.ok_or(SaveAsAttachmentRefusalV1::AuthorityProductIncomplete)?;
        Ok((evidence, destination, self.bytes, object))
    }
}

/// Locator- and content-free evidence for later durable receipts.
#[derive(Clone, Eq, PartialEq)]
pub struct SaveAsEvidenceV1 {
    source_witness: SaveAsSourceWitnessV1,
    source: DocumentVersion,
    serialization: SourceSerializationStateV1,
    resource: AuthorityResourceRefV1,
    persist_decision: AuthorityDecisionRefV1,
    rebind_decision: AuthorityDecisionRefV1,
    destination_decision: SaveAsDestinationDecisionRefV1,
}

impl fmt::Debug for SaveAsEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SaveAsEvidenceV1")
            .field("source", &"<opaque>")
            .field("serialization", &"<opaque>")
            .field("resource", &self.resource)
            .field("persist_decision", &self.persist_decision)
            .field("rebind_decision", &self.rebind_decision)
            .field("destination_decision", &self.destination_decision)
            .finish()
    }
}

impl SaveAsEvidenceV1 {
    pub const fn source_version(&self) -> &DocumentVersion {
        &self.source
    }

    pub const fn source_revision(&self) -> u64 {
        self.source.revision
    }

    pub const fn source_witness(&self) -> SaveAsSourceWitnessV1 {
        self.source_witness
    }

    pub const fn serialization(&self) -> SourceSerializationStateV1 {
        self.serialization
    }

    pub const fn disposition(&self) -> SaveAsDispositionV1 {
        SaveAsDispositionV1::OverwriteExisting
    }
}

/// The only physical disposition admitted by this Save As profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveAsDispositionV1 {
    OverwriteExisting,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveAsPersistenceV1 {
    Durable,
    NotDurable,
    Unknown,
    NotAttempted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveAsBindingV1 {
    Activated,
    NotActivated,
    Unknown,
}

/// Fixed reasons why a retained operation cannot be treated as terminal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SaveAsFenceReasonV1 {
    PhysicalUncertainty = 0,
    BindingUncertainty = 1,
    RetainedTask = 2,
    ReleaseUncertainty = 3,
    Fault = 4,
}

/// Non-empty bounded join-semilattice used only by `Fenced` lifecycle states.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SaveAsFenceSetV1(u8);

impl SaveAsFenceSetV1 {
    const KNOWN: u8 = (1 << 5) - 1;

    pub const fn singleton(reason: SaveAsFenceReasonV1) -> Self {
        Self(1 << reason as u8)
    }

    pub const fn try_from_bits(bits: u8) -> Option<Self> {
        if bits == 0 || bits & !Self::KNOWN != 0 {
            None
        } else {
            Some(Self(bits))
        }
    }

    pub const fn join(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, reason: SaveAsFenceReasonV1) -> bool {
        self.0 & (1 << reason as u8) != 0
    }

    pub const fn bits(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveAsLifecycleV1 {
    TerminalReplayable,
    Fenced(SaveAsFenceSetV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveAsNotAttemptedReasonV1 {
    AuthorityDenied,
    AuthorityUnavailable,
    StaleSource,
    DestinationRefused,
    Capacity,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveAsNotDurableReasonV1 {
    Conflict,
    Unsupported,
    Limit,
    Capacity,
    Io,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveAsPhysicalUnknownReasonV1 {
    Io,
    Cleanup,
    TaskStateUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveAsNotActivatedReasonV1 {
    Refused,
    Unavailable,
    DestinationChanged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveAsBindingUnknownReasonV1 {
    Faulted,
    TaskStateUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveAsFailureV1 {
    NotAttempted(SaveAsNotAttemptedReasonV1),
    NotDurable(SaveAsNotDurableReasonV1),
    PhysicalUnknown(SaveAsPhysicalUnknownReasonV1),
    NotActivated(SaveAsNotActivatedReasonV1),
    BindingUnknown(SaveAsBindingUnknownReasonV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SaveAsReceiptStateV1 {
    DurableActivated,
    DurableNotActivated(SaveAsNotActivatedReasonV1),
    DurableBindingUnknown(SaveAsBindingUnknownReasonV1),
    NotDurable(SaveAsNotDurableReasonV1),
    PhysicalUnknown(SaveAsPhysicalUnknownReasonV1),
    NotAttempted(SaveAsNotAttemptedReasonV1),
}

/// Locator- and content-free terminal or reconciliation evidence.
///
/// The private state sum makes invalid physical/binding products
/// unrepresentable. In particular, activation requires durable publication and
/// every unknown result requires a corresponding explicit fence.
#[derive(Clone, Eq, PartialEq)]
pub struct SaveAsReceiptV1 {
    evidence: SaveAsEvidenceV1,
    state: SaveAsReceiptStateV1,
    lifecycle: SaveAsLifecycleV1,
}

impl SaveAsReceiptV1 {
    pub fn durable_activated(evidence: SaveAsEvidenceV1, lifecycle: SaveAsLifecycleV1) -> Self {
        Self {
            evidence,
            state: SaveAsReceiptStateV1::DurableActivated,
            lifecycle,
        }
    }

    pub fn durable_not_activated(
        evidence: SaveAsEvidenceV1,
        reason: SaveAsNotActivatedReasonV1,
        lifecycle: SaveAsLifecycleV1,
    ) -> Self {
        Self {
            evidence,
            state: SaveAsReceiptStateV1::DurableNotActivated(reason),
            lifecycle,
        }
    }

    pub fn durable_binding_unknown(
        evidence: SaveAsEvidenceV1,
        reason: SaveAsBindingUnknownReasonV1,
        fence: SaveAsFenceSetV1,
    ) -> Self {
        let fence = fence.join(SaveAsFenceSetV1::singleton(
            SaveAsFenceReasonV1::BindingUncertainty,
        ));
        Self {
            evidence,
            state: SaveAsReceiptStateV1::DurableBindingUnknown(reason),
            lifecycle: SaveAsLifecycleV1::Fenced(fence),
        }
    }

    pub fn not_durable(
        evidence: SaveAsEvidenceV1,
        reason: SaveAsNotDurableReasonV1,
        lifecycle: SaveAsLifecycleV1,
    ) -> Self {
        Self {
            evidence,
            state: SaveAsReceiptStateV1::NotDurable(reason),
            lifecycle,
        }
    }

    pub fn physical_unknown(
        evidence: SaveAsEvidenceV1,
        reason: SaveAsPhysicalUnknownReasonV1,
        fence: SaveAsFenceSetV1,
    ) -> Self {
        let fence = fence.join(SaveAsFenceSetV1::singleton(
            SaveAsFenceReasonV1::PhysicalUncertainty,
        ));
        Self {
            evidence,
            state: SaveAsReceiptStateV1::PhysicalUnknown(reason),
            lifecycle: SaveAsLifecycleV1::Fenced(fence),
        }
    }

    pub fn not_attempted(
        evidence: SaveAsEvidenceV1,
        reason: SaveAsNotAttemptedReasonV1,
        lifecycle: SaveAsLifecycleV1,
    ) -> Self {
        Self {
            evidence,
            state: SaveAsReceiptStateV1::NotAttempted(reason),
            lifecycle,
        }
    }

    pub const fn evidence(&self) -> &SaveAsEvidenceV1 {
        &self.evidence
    }

    pub const fn persistence(&self) -> SaveAsPersistenceV1 {
        match self.state {
            SaveAsReceiptStateV1::DurableActivated
            | SaveAsReceiptStateV1::DurableNotActivated(_)
            | SaveAsReceiptStateV1::DurableBindingUnknown(_) => SaveAsPersistenceV1::Durable,
            SaveAsReceiptStateV1::NotDurable(_) => SaveAsPersistenceV1::NotDurable,
            SaveAsReceiptStateV1::PhysicalUnknown(_) => SaveAsPersistenceV1::Unknown,
            SaveAsReceiptStateV1::NotAttempted(_) => SaveAsPersistenceV1::NotAttempted,
        }
    }

    pub const fn binding(&self) -> SaveAsBindingV1 {
        match self.state {
            SaveAsReceiptStateV1::DurableActivated => SaveAsBindingV1::Activated,
            SaveAsReceiptStateV1::DurableBindingUnknown(_) => SaveAsBindingV1::Unknown,
            SaveAsReceiptStateV1::DurableNotActivated(_)
            | SaveAsReceiptStateV1::NotDurable(_)
            | SaveAsReceiptStateV1::PhysicalUnknown(_)
            | SaveAsReceiptStateV1::NotAttempted(_) => SaveAsBindingV1::NotActivated,
        }
    }

    pub const fn lifecycle(&self) -> SaveAsLifecycleV1 {
        self.lifecycle
    }

    pub const fn failure(&self) -> Option<SaveAsFailureV1> {
        match self.state {
            SaveAsReceiptStateV1::DurableActivated => None,
            SaveAsReceiptStateV1::DurableNotActivated(reason) => {
                Some(SaveAsFailureV1::NotActivated(reason))
            }
            SaveAsReceiptStateV1::DurableBindingUnknown(reason) => {
                Some(SaveAsFailureV1::BindingUnknown(reason))
            }
            SaveAsReceiptStateV1::NotDurable(reason) => Some(SaveAsFailureV1::NotDurable(reason)),
            SaveAsReceiptStateV1::PhysicalUnknown(reason) => {
                Some(SaveAsFailureV1::PhysicalUnknown(reason))
            }
            SaveAsReceiptStateV1::NotAttempted(reason) => {
                Some(SaveAsFailureV1::NotAttempted(reason))
            }
        }
    }

    pub const fn changes_stable_binding(&self) -> bool {
        matches!(self.state, SaveAsReceiptStateV1::DurableActivated)
    }
}

impl fmt::Debug for SaveAsReceiptV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SaveAsReceiptV1")
            .field("evidence", &self.evidence)
            .field("persistence", &self.persistence())
            .field("binding", &self.binding())
            .field("lifecycle", &self.lifecycle)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authority::{AuthorityContextV1, AuthoritySessionRefV1, AuthoritySubjectRefV1};
    use std::path::PathBuf;
    use vellum_engine::{
        coordinator::{CoordinatorLimits, SourceCoordinator},
        instrumentation::Recorder,
        source::Limits,
    };

    fn source_version() -> (DocumentVersion, SourceSerializationStateV1) {
        let mut owner = SourceCoordinator::new(CoordinatorLimits {
            max_document_incarnations: 1,
            max_participants: 1,
            max_staged_bytes: 8,
            source: Limits {
                max_source_bytes: 8,
                max_edits: 1,
                max_edit_payload_bytes: 8,
            },
        });
        let source = owner.create(b"source", &mut Recorder::disabled()).unwrap();
        let serialization = owner
            .source_serialization(&source.document, &mut Recorder::disabled())
            .unwrap();
        (source, serialization)
    }

    fn parts() -> (
        AuthorityRequestV1,
        AuthorityRequestV1,
        SaveAsDestinationRequestV1,
        DocumentVersion,
        SourceSerializationStateV1,
    ) {
        let context = AuthorityContextV1 {
            subject: AuthoritySubjectRefV1::new([1; 16]),
            session: Some(AuthoritySessionRefV1::new([2; 16])),
        };
        let resource = AuthorityResourceRefV1::new([3; 16]);
        let (source, serialization) = source_version();
        (
            AuthorityRequestV1 {
                context,
                resource,
                action: AuthorityActionV1::PersistSource,
            },
            AuthorityRequestV1 {
                context,
                resource,
                action: AuthorityActionV1::RebindSource,
            },
            SaveAsDestinationRequestV1::new(
                context,
                resource,
                PathBuf::from("/private/secret/source.rs"),
                SaveAsDestinationEffectV1::OverwriteExisting,
            ),
            source,
            serialization,
        )
    }

    #[test]
    fn exact_complete_grant_is_one_use_and_denial_consumes_nothing() {
        let (persist, rebind, destination, source, serialization) = parts();
        let request =
            SaveAsAuthorityRequestV1::new(persist, rebind, source.clone(), destination.clone())
                .unwrap();
        let mut authority = FixedSaveAsAuthorityV1::install(
            request,
            SaveAsDestinationObjectV1::install(destination.clone(), 17_u8),
        )
        .unwrap();
        let mut wrong = destination.clone();
        wrong.effect = SaveAsDestinationEffectV1::CreateExclusive;
        assert_eq!(
            AuthorizedSaveAsPlanV1::authorize(
                &mut authority,
                persist,
                rebind,
                wrong,
                source.clone(),
                serialization,
            )
            .unwrap_err(),
            SaveAsAuthorizationRefusalV1::Denied
        );
        assert!(
            AuthorizedSaveAsPlanV1::authorize(
                &mut authority,
                persist,
                rebind,
                destination.clone(),
                source.clone(),
                serialization,
            )
            .is_ok()
        );
        assert_eq!(
            AuthorizedSaveAsPlanV1::authorize(
                &mut authority,
                persist,
                rebind,
                destination,
                source,
                serialization,
            )
            .unwrap_err(),
            SaveAsAuthorizationRefusalV1::Unavailable
        );
    }

    #[test]
    fn partial_or_mismatched_products_fail_before_capture() {
        let (persist, mut rebind, destination, source, serialization) = parts();
        rebind.action = AuthorityActionV1::PersistSource;
        assert_eq!(
            SaveAsAuthorityRequestV1::new(persist, rebind, source.clone(), destination.clone())
                .unwrap_err(),
            SaveAsAuthorizationRefusalV1::Denied
        );

        struct Liar;
        impl SaveAsAuthorityPortV1 for Liar {
            fn authorize_save_as(
                &mut self,
                request: SaveAsAuthorityRequestV1,
            ) -> SaveAsAuthorityDecisionV1 {
                let mut wrong = request.rebind();
                wrong.action = AuthorityActionV1::PersistSource;
                SaveAsAuthorityDecisionV1::Granted {
                    persist: SaveAsActionEvidenceV1::new(
                        AuthorityDecisionRefV1::new([4; 16]),
                        request.persist(),
                        request.source().clone(),
                    ),
                    rebind: SaveAsActionEvidenceV1::new(
                        AuthorityDecisionRefV1::new([5; 16]),
                        wrong,
                        request.source().clone(),
                    ),
                    destination: SaveAsDestinationEvidenceV1::with_object(
                        SaveAsDestinationDecisionRefV1::new([6; 16]),
                        request.destination().clone(),
                        SaveAsDestinationObjectV1::install(request.destination().clone(), ()),
                    ),
                }
            }
        }

        let (_, rebind, _, _, _) = parts();
        assert_eq!(
            AuthorizedSaveAsPlanV1::authorize(
                &mut Liar,
                persist,
                rebind,
                destination,
                source,
                serialization,
            )
            .unwrap_err(),
            SaveAsAuthorizationRefusalV1::Unavailable
        );
    }

    #[test]
    fn debug_surfaces_redact_locator_and_source() {
        let (persist, rebind, destination, source, _) = parts();
        let request = SaveAsAuthorityRequestV1::new(persist, rebind, source, destination).unwrap();
        let debug = format!("{request:?}");
        assert!(debug.contains("<redacted>"));
        assert!(debug.contains("<opaque>"));
        assert!(!debug.contains("secret"));
        assert!(!debug.contains("source.rs"));
    }

    #[test]
    fn destination_object_must_match_the_complete_request() {
        let (persist, rebind, destination, source, _) = parts();
        let request =
            SaveAsAuthorityRequestV1::new(persist, rebind, source, destination.clone()).unwrap();
        let foreign = SaveAsDestinationRequestV1::new(
            destination.context,
            destination.resource,
            PathBuf::from("/private/other/source.rs"),
            destination.effect,
        );
        assert!(matches!(
            FixedSaveAsAuthorityV1::install(
                request,
                SaveAsDestinationObjectV1::install(foreign, ()),
            ),
            Err(SaveAsAuthorizationRefusalV1::Denied)
        ));
    }

    fn receipt_evidence() -> SaveAsEvidenceV1 {
        let (source, serialization) = source_version();
        SaveAsEvidenceV1 {
            source_witness: SaveAsSourceWitnessV1([7; 16]),
            source,
            serialization,
            resource: AuthorityResourceRefV1::new([8; 16]),
            persist_decision: AuthorityDecisionRefV1::new([9; 16]),
            rebind_decision: AuthorityDecisionRefV1::new([10; 16]),
            destination_decision: SaveAsDestinationDecisionRefV1::new([11; 16]),
        }
    }

    fn fenced() -> SaveAsLifecycleV1 {
        SaveAsLifecycleV1::Fenced(SaveAsFenceSetV1::singleton(
            SaveAsFenceReasonV1::RetainedTask,
        ))
    }

    #[test]
    fn receipt_constructors_close_exactly_the_valid_product_rows() {
        let receipts = [
            SaveAsReceiptV1::durable_activated(
                receipt_evidence(),
                SaveAsLifecycleV1::TerminalReplayable,
            ),
            SaveAsReceiptV1::durable_activated(receipt_evidence(), fenced()),
            SaveAsReceiptV1::durable_not_activated(
                receipt_evidence(),
                SaveAsNotActivatedReasonV1::Refused,
                SaveAsLifecycleV1::TerminalReplayable,
            ),
            SaveAsReceiptV1::durable_not_activated(
                receipt_evidence(),
                SaveAsNotActivatedReasonV1::Unavailable,
                fenced(),
            ),
            SaveAsReceiptV1::durable_binding_unknown(
                receipt_evidence(),
                SaveAsBindingUnknownReasonV1::Faulted,
                SaveAsFenceSetV1::singleton(SaveAsFenceReasonV1::Fault),
            ),
            SaveAsReceiptV1::not_durable(
                receipt_evidence(),
                SaveAsNotDurableReasonV1::Conflict,
                SaveAsLifecycleV1::TerminalReplayable,
            ),
            SaveAsReceiptV1::not_durable(
                receipt_evidence(),
                SaveAsNotDurableReasonV1::Io,
                fenced(),
            ),
            SaveAsReceiptV1::physical_unknown(
                receipt_evidence(),
                SaveAsPhysicalUnknownReasonV1::Io,
                SaveAsFenceSetV1::singleton(SaveAsFenceReasonV1::RetainedTask),
            ),
            SaveAsReceiptV1::not_attempted(
                receipt_evidence(),
                SaveAsNotAttemptedReasonV1::AuthorityDenied,
                SaveAsLifecycleV1::TerminalReplayable,
            ),
            SaveAsReceiptV1::not_attempted(
                receipt_evidence(),
                SaveAsNotAttemptedReasonV1::Capacity,
                fenced(),
            ),
        ];
        let projections: Vec<_> = receipts
            .iter()
            .map(|receipt| {
                (
                    receipt.persistence(),
                    receipt.binding(),
                    matches!(receipt.lifecycle(), SaveAsLifecycleV1::Fenced(_)),
                )
            })
            .collect();

        for persistence in [
            SaveAsPersistenceV1::Durable,
            SaveAsPersistenceV1::NotDurable,
            SaveAsPersistenceV1::Unknown,
            SaveAsPersistenceV1::NotAttempted,
        ] {
            for binding in [
                SaveAsBindingV1::Activated,
                SaveAsBindingV1::NotActivated,
                SaveAsBindingV1::Unknown,
            ] {
                for is_fenced in [false, true] {
                    let expected = matches!(
                        (persistence, binding, is_fenced),
                        (SaveAsPersistenceV1::Durable, SaveAsBindingV1::Activated, _)
                            | (
                                SaveAsPersistenceV1::Durable,
                                SaveAsBindingV1::NotActivated,
                                _
                            )
                            | (SaveAsPersistenceV1::Durable, SaveAsBindingV1::Unknown, true)
                            | (
                                SaveAsPersistenceV1::NotDurable,
                                SaveAsBindingV1::NotActivated,
                                _
                            )
                            | (
                                SaveAsPersistenceV1::Unknown,
                                SaveAsBindingV1::NotActivated,
                                true
                            )
                            | (
                                SaveAsPersistenceV1::NotAttempted,
                                SaveAsBindingV1::NotActivated,
                                _
                            )
                    );
                    assert_eq!(
                        projections.contains(&(persistence, binding, is_fenced)),
                        expected,
                        "unexpected receipt product: {persistence:?} {binding:?} fenced={is_fenced}",
                    );
                }
            }
        }
        assert_eq!(projections.len(), 10);
        assert_eq!(
            receipts
                .iter()
                .filter(|receipt| receipt.changes_stable_binding())
                .count(),
            2
        );
        assert!(
            receipts
                .iter()
                .filter(|receipt| receipt.changes_stable_binding())
                .all(
                    |receipt| receipt.persistence() == SaveAsPersistenceV1::Durable
                        && receipt.binding() == SaveAsBindingV1::Activated
                )
        );
    }

    #[test]
    fn unknown_receipts_install_the_required_fence_reason() {
        let physical = SaveAsReceiptV1::physical_unknown(
            receipt_evidence(),
            SaveAsPhysicalUnknownReasonV1::Cleanup,
            SaveAsFenceSetV1::singleton(SaveAsFenceReasonV1::RetainedTask),
        );
        let binding = SaveAsReceiptV1::durable_binding_unknown(
            receipt_evidence(),
            SaveAsBindingUnknownReasonV1::TaskStateUnavailable,
            SaveAsFenceSetV1::singleton(SaveAsFenceReasonV1::ReleaseUncertainty),
        );
        let SaveAsLifecycleV1::Fenced(physical_fence) = physical.lifecycle() else {
            panic!("unknown physical outcome must be fenced")
        };
        let SaveAsLifecycleV1::Fenced(binding_fence) = binding.lifecycle() else {
            panic!("unknown binding outcome must be fenced")
        };
        assert!(physical_fence.contains(SaveAsFenceReasonV1::PhysicalUncertainty));
        assert!(physical_fence.contains(SaveAsFenceReasonV1::RetainedTask));
        assert!(binding_fence.contains(SaveAsFenceReasonV1::BindingUncertainty));
        assert!(binding_fence.contains(SaveAsFenceReasonV1::ReleaseUncertainty));
        assert_eq!(SaveAsFenceSetV1::try_from_bits(0), None);
        assert_eq!(SaveAsFenceSetV1::try_from_bits(1 << 7), None);
    }

    #[test]
    fn receipt_debug_is_locator_content_and_identity_safe() {
        let receipt = SaveAsReceiptV1::durable_not_activated(
            receipt_evidence(),
            SaveAsNotActivatedReasonV1::DestinationChanged,
            SaveAsLifecycleV1::TerminalReplayable,
        );
        let debug = format!("{receipt:?}");
        assert!(debug.contains("<opaque>"));
        assert!(!debug.contains("secret"));
        assert!(!debug.contains("source.rs"));
        assert!(!debug.contains("7, 7"));
        assert!(!debug.contains("8, 8"));
        assert_eq!(
            receipt.evidence().disposition(),
            SaveAsDispositionV1::OverwriteExisting
        );
        assert_eq!(
            receipt.failure(),
            Some(SaveAsFailureV1::NotActivated(
                SaveAsNotActivatedReasonV1::DestinationChanged
            ))
        );
    }
}
