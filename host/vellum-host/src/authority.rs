//! Vellum-owned authority vocabulary and host authorization port.
//!
//! Authentication and policy evaluation are host responsibilities. Vellum
//! owns the closed action vocabulary because collapsing read, edit, discovery,
//! persistence, and rebinding into a generic action would widen authority.
//! Decision evidence is request-bound attribution; it is not a bearer grant.

use std::{
    any::Any,
    fmt,
    path::{Path, PathBuf},
};

macro_rules! opaque_ref {
    ($name:ident) => {
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 16]);

        impl $name {
            /// Constructs a host-established opaque reference.
            ///
            /// The value identifies a host fact; possession grants no Vellum
            /// authority.
            pub const fn new(value: [u8; 16]) -> Self {
                Self(value)
            }

            /// Projects the opaque correlation value for trusted persistence
            /// adapters. The bytes are identity evidence, never credentials
            /// or authority by possession.
            pub const fn to_bytes(self) -> [u8; 16] {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "(<opaque>)"))
            }
        }
    };
}

opaque_ref!(AuthoritySubjectRefV1);
opaque_ref!(AuthoritySessionRefV1);
opaque_ref!(AuthorityResourceRefV1);
opaque_ref!(AuthorityDecisionRefV1);
opaque_ref!(SaveAsDestinationDecisionRefV1);

/// The two filesystem effects which a Save As destination policy may admit.
/// Replacement and exclusive creation have different race and data-loss laws.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveAsDestinationEffectV1 {
    OverwriteExisting,
    CreateExclusive,
}

/// One exact host-side destination policy question.
#[derive(Clone, Eq, PartialEq)]
pub struct SaveAsDestinationRequestV1 {
    pub context: AuthorityContextV1,
    pub resource: AuthorityResourceRefV1,
    destination: PathBuf,
    pub effect: SaveAsDestinationEffectV1,
}

impl SaveAsDestinationRequestV1 {
    pub fn new(
        context: AuthorityContextV1,
        resource: AuthorityResourceRefV1,
        destination: PathBuf,
        effect: SaveAsDestinationEffectV1,
    ) -> Self {
        Self {
            context,
            resource,
            destination,
            effect,
        }
    }

    pub fn destination(&self) -> &Path {
        &self.destination
    }
}

impl fmt::Debug for SaveAsDestinationRequestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SaveAsDestinationRequestV1")
            .field("context", &self.context)
            .field("resource", &self.resource)
            .field("destination", &"<redacted>")
            .field("effect", &self.effect)
            .finish()
    }
}

/// Non-cloneable, request-bound attribution for one destination decision.
/// This is evidence, not a bearer capability.
pub struct SaveAsDestinationEvidenceV1 {
    decision: SaveAsDestinationDecisionRefV1,
    request: SaveAsDestinationRequestV1,
    object: Option<SaveAsDestinationObjectV1>,
}

impl SaveAsDestinationEvidenceV1 {
    pub const fn new(
        decision: SaveAsDestinationDecisionRefV1,
        request: SaveAsDestinationRequestV1,
    ) -> Self {
        Self {
            decision,
            request,
            object: None,
        }
    }

    /// Constructs evidence carrying a host-installed, one-use destination
    /// object. A locator remains policy input; this object is the authority
    /// the effect adapter must consume.
    pub fn with_object(
        decision: SaveAsDestinationDecisionRefV1,
        request: SaveAsDestinationRequestV1,
        object: SaveAsDestinationObjectV1,
    ) -> Self {
        Self {
            decision,
            request,
            object: Some(object),
        }
    }

    pub const fn request(&self) -> &SaveAsDestinationRequestV1 {
        &self.request
    }

    /// True only when this grant carries a destination object installed for
    /// the exact same context, resource, locator, and effect. Export planning
    /// uses this before asking a codec to disclose source bytes.
    pub fn has_congruent_object(&self) -> bool {
        self.object
            .as_ref()
            .is_some_and(|object| object.request() == &self.request)
    }

    pub const fn decision_ref(&self) -> SaveAsDestinationDecisionRefV1 {
        self.decision
    }

    pub fn into_parts(
        self,
    ) -> (
        SaveAsDestinationDecisionRefV1,
        SaveAsDestinationRequestV1,
        Option<SaveAsDestinationObjectV1>,
    ) {
        (self.decision, self.request, self.object)
    }
}

impl fmt::Debug for SaveAsDestinationEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SaveAsDestinationEvidenceV1")
            .field("decision", &self.decision)
            .field("request", &self.request)
            .field("object", &self.object)
            .finish()
    }
}

/// Opaque one-use host object installed by a destination adapter.
///
/// The payload is deliberately type-erased at the host boundary. The effect
/// adapter accepts only its own payload type, so a string or foreign object
/// cannot be confused for filesystem authority.
pub struct SaveAsDestinationObjectV1 {
    request: SaveAsDestinationRequestV1,
    object: Box<dyn Any + Send>,
}

impl SaveAsDestinationObjectV1 {
    pub fn install<T: Any + Send>(request: SaveAsDestinationRequestV1, object: T) -> Self {
        Self {
            request,
            object: Box::new(object),
        }
    }

    pub const fn request(&self) -> &SaveAsDestinationRequestV1 {
        &self.request
    }

    pub fn downcast<T: Any + Send>(self) -> Result<T, SaveAsDestinationObjectV1> {
        let Self { request, object } = self;
        object
            .downcast::<T>()
            .map(|object| *object)
            .map_err(|object| SaveAsDestinationObjectV1 { request, object })
    }
}

impl fmt::Debug for SaveAsDestinationObjectV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SaveAsDestinationObjectV1(<opaque>)")
    }
}

#[derive(Debug)]
pub enum SaveAsDestinationDecisionV1 {
    Granted(SaveAsDestinationEvidenceV1),
    Denied,
    Unavailable,
}

impl PartialEq for SaveAsDestinationDecisionV1 {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Granted(left), Self::Granted(right)) => {
                left.decision == right.decision && left.request == right.request
            }
            (Self::Denied, Self::Denied) | (Self::Unavailable, Self::Unavailable) => true,
            _ => false,
        }
    }
}

impl Eq for SaveAsDestinationDecisionV1 {}

/// Host-owned interpreter for exact Save As destinations and dispositions.
pub trait SaveAsDestinationPolicyV1 {
    fn authorize_destination(
        &mut self,
        request: SaveAsDestinationRequestV1,
    ) -> SaveAsDestinationDecisionV1;
}

/// Bounded exact-path policy for local trusted composition and tests.
/// Runtime attenuation can only remove effects.
pub struct FixedSaveAsDestinationPolicyV1 {
    context: AuthorityContextV1,
    resource: AuthorityResourceRefV1,
    destination: PathBuf,
    overwrite_existing: bool,
    create_exclusive: bool,
    next_decision: u64,
    max_decisions: u64,
}

impl FixedSaveAsDestinationPolicyV1 {
    pub fn try_new(
        context: AuthorityContextV1,
        resource: AuthorityResourceRefV1,
        destination: PathBuf,
        overwrite_existing: bool,
        create_exclusive: bool,
        max_decisions: u64,
    ) -> Result<Self, FixedAuthorityBuildErrorV1> {
        if destination.as_os_str().is_empty()
            || (!overwrite_existing && !create_exclusive)
            || max_decisions == 0
        {
            return Err(FixedAuthorityBuildErrorV1::Disabled);
        }
        Ok(Self {
            context,
            resource,
            destination,
            overwrite_existing,
            create_exclusive,
            next_decision: 0,
            max_decisions,
        })
    }

    pub fn attenuate(&mut self, overwrite_existing: bool, create_exclusive: bool) {
        self.overwrite_existing &= overwrite_existing;
        self.create_exclusive &= create_exclusive;
    }
}

impl SaveAsDestinationPolicyV1 for FixedSaveAsDestinationPolicyV1 {
    fn authorize_destination(
        &mut self,
        request: SaveAsDestinationRequestV1,
    ) -> SaveAsDestinationDecisionV1 {
        let effect_allowed = match request.effect {
            SaveAsDestinationEffectV1::OverwriteExisting => self.overwrite_existing,
            SaveAsDestinationEffectV1::CreateExclusive => self.create_exclusive,
        };
        if request.context != self.context
            || request.resource != self.resource
            || request.destination() != self.destination
            || !effect_allowed
        {
            return SaveAsDestinationDecisionV1::Denied;
        }
        if self.next_decision >= self.max_decisions {
            return SaveAsDestinationDecisionV1::Unavailable;
        }
        let mut identity = [0_u8; 16];
        identity[..8].copy_from_slice(b"vellum-d");
        identity[8..].copy_from_slice(&self.next_decision.to_be_bytes());
        self.next_decision += 1;
        SaveAsDestinationDecisionV1::Granted(SaveAsDestinationEvidenceV1::new(
            SaveAsDestinationDecisionRefV1::new(identity),
            request,
        ))
    }
}

/// Exhaustive Vellum actions mediated at the service/host boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum AuthorityActionV1 {
    InspectWorkspace = 0,
    ReadSource = 1,
    SelectDocument = 2,
    AdmitFile = 3,
    DiscoverDirectory = 4,
    EditSource = 5,
    PersistSource = 6,
    RebindSource = 7,
    ManageView = 8,
    RetireMembership = 9,
    /// Erases replayable receipt payload while retaining the operation's
    /// absorbing identity tombstone. It is intentionally distinct from
    /// workspace-membership retirement.
    RetireReceipt = 10,
    /// Read an exact engine-owned serialization and publish it as a copy. This
    /// does not grant ordinary save, rebinding, or destination replacement.
    ExportSourceCopy = 11,
    /// Submit and collect provider-backed semantic queries. This does not
    /// grant source synchronization, editing, persistence, or evaluation.
    SemanticQuery = 12,
    /// Install and advance the provider's source projection. This does not
    /// imply query, edit, persistence, process-launch, or evaluation authority.
    SemanticSynchronize = 13,
}

/// Finite bounded lattice of Vellum actions.
///
/// Subset is the order, intersection is meet, union is join, the empty set is
/// bottom, and every admitted action is top. Runtime attenuation must use
/// `meet`; `join` is reserved for explicit trusted composition.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AuthorityActionSetV1(u16);

impl AuthorityActionSetV1 {
    const ALL: u16 = (1 << 14) - 1;

    pub const fn bottom() -> Self {
        Self(0)
    }

    pub const fn top() -> Self {
        Self(Self::ALL)
    }

    pub const fn read_only() -> Self {
        Self::singleton(AuthorityActionV1::InspectWorkspace)
            .join(Self::singleton(AuthorityActionV1::ReadSource))
            .join(Self::singleton(AuthorityActionV1::SelectDocument))
            .join(Self::singleton(AuthorityActionV1::ManageView))
    }

    pub const fn singleton(action: AuthorityActionV1) -> Self {
        Self(1 << action as u8)
    }

    pub const fn meet(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    pub const fn join(self, other: Self) -> Self {
        Self((self.0 | other.0) & Self::ALL)
    }

    pub const fn is_subset_of(self, other: Self) -> bool {
        self.0 & !other.0 == 0
    }

    pub const fn contains(self, action: AuthorityActionV1) -> bool {
        self.0 & (1 << action as u8) != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// Host-established caller context. These references are correlation facts,
/// never credentials or ambient authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthorityContextV1 {
    pub subject: AuthoritySubjectRefV1,
    pub session: Option<AuthoritySessionRefV1>,
}

/// One exact authorization question.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthorityRequestV1 {
    pub context: AuthorityContextV1,
    pub resource: AuthorityResourceRefV1,
    pub action: AuthorityActionV1,
}

/// Request-bound attribution for a granted decision.
///
/// This value deliberately cannot be cloned. A service may retain it with the
/// operation evidence that consumed the decision, but it cannot use it to
/// authorize another request.
#[derive(Eq, PartialEq)]
pub struct AuthorityDecisionEvidenceV1 {
    decision: AuthorityDecisionRefV1,
    request: AuthorityRequestV1,
}

impl AuthorityDecisionEvidenceV1 {
    pub const fn new(decision: AuthorityDecisionRefV1, request: AuthorityRequestV1) -> Self {
        Self { decision, request }
    }

    pub const fn request(&self) -> AuthorityRequestV1 {
        self.request
    }

    pub const fn decision_ref(&self) -> AuthorityDecisionRefV1 {
        self.decision
    }
}

impl fmt::Debug for AuthorityDecisionEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorityDecisionEvidenceV1")
            .field("decision", &self.decision)
            .field("request", &self.request)
            .finish()
    }
}

/// Closed result of consulting an authority interpreter.
#[derive(Debug, Eq, PartialEq)]
pub enum AuthorityDecisionV1 {
    Granted(AuthorityDecisionEvidenceV1),
    Denied,
    Unavailable,
}

/// Host-selected authority interpreter.
///
/// Implementations must return evidence bound to the exact request on grant.
/// `Denied` and `Unavailable` both fail closed; they remain distinct for
/// content-safe operational evidence and recovery policy.
pub trait AuthorityPortV1 {
    fn authorize(&mut self, request: AuthorityRequestV1) -> AuthorityDecisionV1;
}

/// Bounded static interpreter for a trusted composition root.
///
/// This is useful for the local single-principal profile and tests. It is not
/// principal authentication or an Aegis adapter. After construction its action
/// set can only be attenuated by meet.
pub struct FixedAuthorityV1 {
    context: AuthorityContextV1,
    resource: AuthorityResourceRefV1,
    actions: AuthorityActionSetV1,
    next_decision: u64,
    max_decisions: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixedAuthorityBuildErrorV1 {
    Disabled,
}

impl FixedAuthorityV1 {
    pub const fn try_new(
        context: AuthorityContextV1,
        resource: AuthorityResourceRefV1,
        actions: AuthorityActionSetV1,
        max_decisions: u64,
    ) -> Result<Self, FixedAuthorityBuildErrorV1> {
        if max_decisions == 0 {
            return Err(FixedAuthorityBuildErrorV1::Disabled);
        }
        Ok(Self {
            context,
            resource,
            actions,
            next_decision: 0,
            max_decisions,
        })
    }

    pub const fn actions(&self) -> AuthorityActionSetV1 {
        self.actions
    }

    pub fn attenuate(&mut self, ceiling: AuthorityActionSetV1) {
        self.actions = self.actions.meet(ceiling);
    }
}

impl AuthorityPortV1 for FixedAuthorityV1 {
    fn authorize(&mut self, request: AuthorityRequestV1) -> AuthorityDecisionV1 {
        if request.context != self.context
            || request.resource != self.resource
            || !self.actions.contains(request.action)
        {
            return AuthorityDecisionV1::Denied;
        }
        if self.next_decision >= self.max_decisions {
            return AuthorityDecisionV1::Unavailable;
        }
        let mut identity = [0_u8; 16];
        identity[0] = b'v';
        identity[8..].copy_from_slice(&self.next_decision.to_be_bytes());
        self.next_decision += 1;
        AuthorityDecisionV1::Granted(AuthorityDecisionEvidenceV1::new(
            AuthorityDecisionRefV1::new(identity),
            request,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(bits: u16) -> AuthorityActionSetV1 {
        AuthorityActionSetV1(bits)
    }

    #[test]
    fn action_sets_form_the_declared_finite_bounded_lattice() {
        for left in 0..=AuthorityActionSetV1::ALL {
            let left = set(left);
            assert_eq!(left.meet(AuthorityActionSetV1::top()), left);
            assert_eq!(left.join(AuthorityActionSetV1::bottom()), left);
            assert_eq!(left.meet(left), left);
            assert_eq!(left.join(left), left);
            assert!(AuthorityActionSetV1::bottom().is_subset_of(left));
            assert!(left.is_subset_of(AuthorityActionSetV1::top()));

            for right in 0..=AuthorityActionSetV1::ALL {
                let right = set(right);
                assert_eq!(left.meet(right), right.meet(left));
                assert_eq!(left.join(right), right.join(left));
                assert_eq!(left.meet(left.join(right)), left);
                assert_eq!(left.join(left.meet(right)), left);
                assert!(left.meet(right).is_subset_of(left));
                assert!(left.is_subset_of(left.join(right)));
            }
        }
    }

    #[test]
    fn default_is_no_authority_and_read_only_is_exact() {
        let none = AuthorityActionSetV1::default();
        assert!(none.is_empty());

        let read_only = AuthorityActionSetV1::read_only();
        for action in [
            AuthorityActionV1::InspectWorkspace,
            AuthorityActionV1::ReadSource,
            AuthorityActionV1::SelectDocument,
            AuthorityActionV1::ManageView,
        ] {
            assert!(read_only.contains(action));
        }
        for action in [
            AuthorityActionV1::AdmitFile,
            AuthorityActionV1::DiscoverDirectory,
            AuthorityActionV1::EditSource,
            AuthorityActionV1::PersistSource,
            AuthorityActionV1::ExportSourceCopy,
            AuthorityActionV1::RebindSource,
            AuthorityActionV1::RetireMembership,
            AuthorityActionV1::RetireReceipt,
            AuthorityActionV1::SemanticQuery,
            AuthorityActionV1::SemanticSynchronize,
        ] {
            assert!(!read_only.contains(action));
        }
        assert!(AuthorityActionSetV1::top().contains(AuthorityActionV1::SemanticQuery));
        assert!(AuthorityActionSetV1::top().contains(AuthorityActionV1::SemanticSynchronize));
        assert!(
            !AuthorityActionSetV1::singleton(AuthorityActionV1::SemanticQuery)
                .contains(AuthorityActionV1::SemanticSynchronize)
        );
    }

    #[test]
    fn granted_evidence_is_exact_request_bound_and_debug_redacted() {
        let request = AuthorityRequestV1 {
            context: AuthorityContextV1 {
                subject: AuthoritySubjectRefV1::new([7; 16]),
                session: Some(AuthoritySessionRefV1::new([11; 16])),
            },
            resource: AuthorityResourceRefV1::new([13; 16]),
            action: AuthorityActionV1::PersistSource,
        };
        let evidence =
            AuthorityDecisionEvidenceV1::new(AuthorityDecisionRefV1::new([17; 16]), request);

        assert_eq!(evidence.request(), request);
        let debug = format!("{evidence:?}");
        assert!(debug.contains("<opaque>"));
        assert!(!debug.contains("7, 7"));
        assert!(!debug.contains("11, 11"));
        assert!(!debug.contains("13, 13"));
        assert!(!debug.contains("17, 17"));
    }

    #[test]
    fn fixed_interpreter_is_scoped_bounded_and_tighten_only() {
        let context = AuthorityContextV1 {
            subject: AuthoritySubjectRefV1::new([1; 16]),
            session: Some(AuthoritySessionRefV1::new([2; 16])),
        };
        let resource = AuthorityResourceRefV1::new([3; 16]);
        let mut authority = FixedAuthorityV1::try_new(
            context,
            resource,
            AuthorityActionSetV1::read_only().join(AuthorityActionSetV1::singleton(
                AuthorityActionV1::EditSource,
            )),
            2,
        )
        .unwrap();
        let request = |action| AuthorityRequestV1 {
            context,
            resource,
            action,
        };

        let AuthorityDecisionV1::Granted(first) =
            authority.authorize(request(AuthorityActionV1::ReadSource))
        else {
            panic!("read must be admitted");
        };
        assert_eq!(first.request(), request(AuthorityActionV1::ReadSource));
        authority.attenuate(AuthorityActionSetV1::read_only());
        assert_eq!(
            authority.authorize(request(AuthorityActionV1::EditSource)),
            AuthorityDecisionV1::Denied
        );
        assert!(
            authority
                .actions()
                .is_subset_of(AuthorityActionSetV1::read_only())
        );
        assert!(matches!(
            authority.authorize(request(AuthorityActionV1::InspectWorkspace)),
            AuthorityDecisionV1::Granted(_)
        ));
        assert_eq!(
            authority.authorize(request(AuthorityActionV1::ReadSource)),
            AuthorityDecisionV1::Unavailable
        );

        let foreign = AuthorityRequestV1 {
            context,
            resource: AuthorityResourceRefV1::new([4; 16]),
            action: AuthorityActionV1::ReadSource,
        };
        assert_eq!(authority.authorize(foreign), AuthorityDecisionV1::Denied);
    }

    #[test]
    fn save_as_destination_policy_is_exact_request_bound_and_redacted() {
        let context = AuthorityContextV1 {
            subject: AuthoritySubjectRefV1::new([1; 16]),
            session: Some(AuthoritySessionRefV1::new([2; 16])),
        };
        let resource = AuthorityResourceRefV1::new([3; 16]);
        let private = PathBuf::from("/private/work/secret.txt");
        let mut policy = FixedSaveAsDestinationPolicyV1::try_new(
            context,
            resource,
            private.clone(),
            true,
            false,
            1,
        )
        .unwrap();
        let exact = SaveAsDestinationRequestV1::new(
            context,
            resource,
            private.clone(),
            SaveAsDestinationEffectV1::OverwriteExisting,
        );
        let debug = format!("{exact:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("secret.txt"));
        let SaveAsDestinationDecisionV1::Granted(evidence) =
            policy.authorize_destination(exact.clone())
        else {
            panic!("exact overwrite was not granted")
        };
        assert_eq!(evidence.request(), &exact);
        assert_eq!(
            policy.authorize_destination(SaveAsDestinationRequestV1::new(
                context,
                resource,
                private,
                SaveAsDestinationEffectV1::OverwriteExisting,
            )),
            SaveAsDestinationDecisionV1::Unavailable
        );
    }

    #[test]
    fn save_as_destination_policy_does_not_join_paths_or_effects() {
        let context = AuthorityContextV1 {
            subject: AuthoritySubjectRefV1::new([1; 16]),
            session: None,
        };
        let resource = AuthorityResourceRefV1::new([3; 16]);
        let destination = PathBuf::from("/work/allowed.txt");
        let mut policy = FixedSaveAsDestinationPolicyV1::try_new(
            context,
            resource,
            destination.clone(),
            true,
            false,
            4,
        )
        .unwrap();
        for request in [
            SaveAsDestinationRequestV1::new(
                context,
                resource,
                PathBuf::from("/work/other.txt"),
                SaveAsDestinationEffectV1::OverwriteExisting,
            ),
            SaveAsDestinationRequestV1::new(
                context,
                resource,
                destination,
                SaveAsDestinationEffectV1::CreateExclusive,
            ),
        ] {
            assert_eq!(
                policy.authorize_destination(request),
                SaveAsDestinationDecisionV1::Denied
            );
        }

        policy.attenuate(false, false);
        assert_eq!(
            policy.authorize_destination(SaveAsDestinationRequestV1::new(
                context,
                resource,
                PathBuf::from("/work/allowed.txt"),
                SaveAsDestinationEffectV1::OverwriteExisting,
            )),
            SaveAsDestinationDecisionV1::Denied
        );
    }
}
