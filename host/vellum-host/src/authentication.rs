//! Transport-neutral principal and session establishment.
//!
//! This module is deliberately separate from [`crate::authority`]. An
//! authenticator establishes who a caller is for one host-allocated session;
//! it neither selects Vellum actions nor grants them. Unix ownership, socket
//! reachability, protocol IDs and MCP metadata are not authentication evidence.

use crate::authority::{AuthorityContextV1, AuthoritySessionRefV1, AuthoritySubjectRefV1};
use std::fmt;

macro_rules! opaque_ref {
    ($name:ident) => {
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name([u8; 16]);

        impl $name {
            /// Constructs an opaque fact at the trusted host boundary.
            pub const fn new(value: [u8; 16]) -> Self {
                Self(value)
            }

            /// Projects correlation bytes for a trusted credential adapter.
            /// The value remains evidence input, never a credential or grant.
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

opaque_ref!(AuthenticationChallengeRefV1);
opaque_ref!(AuthenticationClaimRefV1);

/// One exact authentication question prepared by a trusted host.
///
/// The fixed-width claim is a correlation reference into adapter-owned
/// credential state. It is not a username, credential, principal or grant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthenticationRequestV1 {
    pub challenge: AuthenticationChallengeRefV1,
    pub claim: AuthenticationClaimRefV1,
    pub session: AuthoritySessionRefV1,
}

/// Exact result produced by a trusted authentication adapter.
///
/// Construction is public because adapters live outside this crate. The gate
/// accepts it only from its injected port and only when it is bound to the
/// exact request. This evidence is attribution, not a bearer credential.
#[derive(Eq, PartialEq)]
pub struct AuthenticationEvidenceV1 {
    request: AuthenticationRequestV1,
    subject: AuthoritySubjectRefV1,
}

impl AuthenticationEvidenceV1 {
    pub const fn from_port(
        request: AuthenticationRequestV1,
        subject: AuthoritySubjectRefV1,
    ) -> Self {
        Self { request, subject }
    }

    pub const fn request(&self) -> AuthenticationRequestV1 {
        self.request
    }
}

impl fmt::Debug for AuthenticationEvidenceV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthenticationEvidenceV1")
            .field("request", &self.request)
            .field("subject", &self.subject)
            .finish()
    }
}

/// Closed result of a credential adapter consultation.
///
/// `Rejected` means the offered credential facts did not establish the claim.
/// `Unavailable` means the adapter could not establish either truth value.
/// Both fail closed. Public transports should normally project both to one
/// non-enumerating refusal.
#[derive(Debug, Eq, PartialEq)]
pub enum AuthenticationDecisionV1 {
    Authenticated(AuthenticationEvidenceV1),
    Rejected,
    Unavailable,
}

/// Trusted credential-adapter port.
///
/// The context remains adapter-owned, so credentials and provider-specific
/// errors never enter Vellum's vocabulary. A future Aegis composition may be
/// implemented behind this port, but Aegis itself also requires a host to
/// establish credential facts.
pub trait AuthenticationPortV1 {
    type Context: ?Sized;

    fn authenticate(
        &mut self,
        request: AuthenticationRequestV1,
        context: &Self::Context,
    ) -> AuthenticationDecisionV1;
}

/// Bounded, content-safe observation of authentication consultation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AuthenticationUsageV1 {
    pub attempts: u64,
    pub authenticated: u64,
    pub rejected: u64,
    pub unavailable: u64,
    pub evidence_mismatches: u64,
    pub saturated: bool,
}

/// A successfully established subject/session binding.
///
/// Its authority handoff has no parameters: callers cannot widen or substitute
/// either identity while converting authentication into authority context.
#[derive(Debug, Eq, PartialEq)]
pub struct AuthenticatedSessionV1 {
    context: AuthorityContextV1,
}

impl AuthenticatedSessionV1 {
    pub const fn authority_context(&self) -> AuthorityContextV1 {
        self.context
    }
}

/// Bounded host-side interpreter for the authentication port.
///
/// It does not retain credentials or sessions. The composition root owns
/// challenge generation, uniqueness, active-session lifetime and cleanup.
pub struct AuthenticationGateV1<P> {
    port: P,
    remaining_attempts: u64,
    usage: AuthenticationUsageV1,
}

impl<P: AuthenticationPortV1> AuthenticationGateV1<P> {
    pub const fn new(port: P, max_attempts: u64) -> Self {
        Self {
            port,
            remaining_attempts: max_attempts,
            usage: AuthenticationUsageV1 {
                attempts: 0,
                authenticated: 0,
                rejected: 0,
                unavailable: 0,
                evidence_mismatches: 0,
                saturated: false,
            },
        }
    }

    pub const fn usage(&self) -> AuthenticationUsageV1 {
        self.usage
    }

    pub fn establish(
        &mut self,
        request: AuthenticationRequestV1,
        context: &P::Context,
    ) -> Result<AuthenticatedSessionV1, AuthenticationRefusalV1> {
        if self.remaining_attempts == 0 {
            increment(&mut self.usage.unavailable, &mut self.usage.saturated);
            return Err(AuthenticationRefusalV1::Unavailable);
        }
        self.remaining_attempts -= 1;
        increment(&mut self.usage.attempts, &mut self.usage.saturated);

        match self.port.authenticate(request, context) {
            AuthenticationDecisionV1::Authenticated(evidence) => {
                if evidence.request != request {
                    increment(
                        &mut self.usage.evidence_mismatches,
                        &mut self.usage.saturated,
                    );
                    increment(&mut self.usage.unavailable, &mut self.usage.saturated);
                    return Err(AuthenticationRefusalV1::Unavailable);
                }
                increment(&mut self.usage.authenticated, &mut self.usage.saturated);
                Ok(AuthenticatedSessionV1 {
                    context: AuthorityContextV1 {
                        subject: evidence.subject,
                        session: Some(request.session),
                    },
                })
            }
            AuthenticationDecisionV1::Rejected => {
                increment(&mut self.usage.rejected, &mut self.usage.saturated);
                Err(AuthenticationRefusalV1::Rejected)
            }
            AuthenticationDecisionV1::Unavailable => {
                increment(&mut self.usage.unavailable, &mut self.usage.saturated);
                Err(AuthenticationRefusalV1::Unavailable)
            }
        }
    }
}

/// Host-internal closed authentication refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthenticationRefusalV1 {
    Rejected,
    Unavailable,
}

fn increment(value: &mut u64, saturated: &mut bool) {
    let prior = *value;
    *value = value.saturating_add(1);
    *saturated |= prior == u64::MAX;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Port {
        decision: Option<AuthenticationDecisionV1>,
        calls: u64,
    }

    impl AuthenticationPortV1 for Port {
        type Context = ();

        fn authenticate(
            &mut self,
            _request: AuthenticationRequestV1,
            _context: &Self::Context,
        ) -> AuthenticationDecisionV1 {
            self.calls += 1;
            self.decision
                .take()
                .unwrap_or(AuthenticationDecisionV1::Unavailable)
        }
    }

    fn request(tag: u8) -> AuthenticationRequestV1 {
        AuthenticationRequestV1 {
            challenge: AuthenticationChallengeRefV1::new([tag; 16]),
            claim: AuthenticationClaimRefV1::new([tag.wrapping_add(1); 16]),
            session: AuthoritySessionRefV1::new([tag.wrapping_add(2); 16]),
        }
    }

    #[test]
    fn exact_evidence_hands_off_only_the_authenticated_context() {
        let offered = request(1);
        let subject = AuthoritySubjectRefV1::new([7; 16]);
        let port = Port {
            decision: Some(AuthenticationDecisionV1::Authenticated(
                AuthenticationEvidenceV1::from_port(offered, subject),
            )),
            calls: 0,
        };
        let mut gate = AuthenticationGateV1::new(port, 1);
        let session = gate.establish(offered, &()).unwrap();

        assert_eq!(
            session.authority_context(),
            AuthorityContextV1 {
                subject,
                session: Some(offered.session),
            }
        );
        assert_eq!(
            gate.usage(),
            AuthenticationUsageV1 {
                attempts: 1,
                authenticated: 1,
                ..AuthenticationUsageV1::default()
            }
        );
    }

    #[test]
    fn transplanted_evidence_fails_closed_without_a_context() {
        let offered = request(1);
        let port = Port {
            decision: Some(AuthenticationDecisionV1::Authenticated(
                AuthenticationEvidenceV1::from_port(
                    request(2),
                    AuthoritySubjectRefV1::new([7; 16]),
                ),
            )),
            calls: 0,
        };
        let mut gate = AuthenticationGateV1::new(port, 1);

        assert_eq!(
            gate.establish(offered, &()),
            Err(AuthenticationRefusalV1::Unavailable)
        );
        assert_eq!(gate.usage().authenticated, 0);
        assert_eq!(gate.usage().evidence_mismatches, 1);
    }

    #[test]
    fn rejection_unavailability_and_budget_exhaustion_are_exact() {
        let mut rejected = AuthenticationGateV1::new(
            Port {
                decision: Some(AuthenticationDecisionV1::Rejected),
                calls: 0,
            },
            1,
        );
        assert_eq!(
            rejected.establish(request(1), &()),
            Err(AuthenticationRefusalV1::Rejected)
        );

        let mut unavailable = AuthenticationGateV1::new(
            Port {
                decision: Some(AuthenticationDecisionV1::Unavailable),
                calls: 0,
            },
            1,
        );
        assert_eq!(
            unavailable.establish(request(1), &()),
            Err(AuthenticationRefusalV1::Unavailable)
        );
        assert_eq!(
            unavailable.establish(request(2), &()),
            Err(AuthenticationRefusalV1::Unavailable)
        );
        assert_eq!(unavailable.port.calls, 1);
        assert_eq!(unavailable.usage().attempts, 1);
        assert_eq!(unavailable.usage().unavailable, 2);
    }

    #[test]
    fn debug_output_is_content_safe() {
        let offered = request(0x61);
        let evidence =
            AuthenticationEvidenceV1::from_port(offered, AuthoritySubjectRefV1::new([0x62; 16]));
        let debug = format!("{evidence:?}");
        assert!(!debug.contains("97"));
        assert!(!debug.contains("98"));
        assert!(debug.contains("<opaque>"));
    }
}
