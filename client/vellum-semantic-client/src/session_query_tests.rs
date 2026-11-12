use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

use super::*;
use vellum_semantic_protocol::{
    SemanticExchangeDispositionV1, SemanticExchangeRefV1, SemanticMembershipRefV1,
    SemanticProviderObservationV1 as SessionProviderObservationV1,
    SemanticProviderReadinessProjectionV1, SemanticSessionCommandV1, SemanticSessionDescriptionV1,
    SemanticSessionIncarnationV1, SemanticSessionLimitsV1, SemanticSessionOperationRefV1,
    SemanticSessionOutcomeV1, SemanticSessionReadinessV1, SemanticSessionReplyV1,
    SemanticSessionRolesV1, SemanticSessionSnapshotV1, SemanticSynchronizationEpochV1,
    SemanticSynchronizationManifestV1,
};
use vellum_semantic_session_client::{
    SemanticSessionClientLimitsV1, SemanticSessionClientV1, SemanticSessionExchangePortV1,
};

fn run<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("model future unexpectedly pending"),
    }
}

fn opaque<T>(tag: u8, constructor: impl FnOnce([u8; 16]) -> Option<T>) -> T {
    constructor([tag; 16]).unwrap()
}

struct WitnessServer {
    incarnation: SemanticSessionIncarnationV1,
    membership: SemanticMembershipRefV1,
    epoch: SemanticSynchronizationEpochV1,
    ready: bool,
    request: Option<SemanticRequestV1>,
    granted_roles: SemanticSessionRolesV1,
    unknown_submit_once: bool,
    not_sent_submit_replay_once: bool,
}

impl WitnessServer {
    fn reply(
        &self,
        command: &SemanticSessionCommandV1,
        outcome: SemanticSessionOutcomeV1,
    ) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1> {
        let membership = match (&outcome, command) {
            (SemanticSessionOutcomeV1::Joined { membership, .. }, _) => Some(*membership),
            (_, SemanticSessionCommandV1::Describe { .. })
            | (_, SemanticSessionCommandV1::JoinSession { .. }) => None,
            _ => Some(self.membership),
        };
        SemanticExchangeDispositionV1::Reply(
            SemanticSessionReplyV1::try_new(
                command.kind(),
                command.exchange(),
                self.incarnation,
                membership,
                command.operation(),
                outcome,
            )
            .unwrap(),
        )
    }

    fn description(&self) -> SemanticSessionDescriptionV1 {
        let readiness = if self.ready {
            SemanticSessionReadinessV1::Ready { epoch: self.epoch }
        } else {
            SemanticSessionReadinessV1::Unavailable
        };
        let provider_readiness = if self.ready {
            SemanticProviderReadinessProjectionV1::Ready {
                generation: SemanticIndexGenerationV1::new(3).unwrap(),
            }
        } else {
            SemanticProviderReadinessProjectionV1::Unavailable
        };
        SemanticSessionDescriptionV1::try_new(
            self.incarnation,
            SemanticSessionRolesV1::top(),
            readiness,
            SessionProviderObservationV1::new(provider_readiness, SemanticCapabilitySetV1::top()),
            SemanticSessionLimitsV1::reference(),
        )
        .unwrap()
    }
}

impl SemanticSessionExchangePortV1 for WitnessServer {
    fn exchange(
        &mut self,
        command: &SemanticSessionCommandV1,
    ) -> SemanticExchangeDispositionV1<SemanticSessionReplyV1> {
        if matches!(command, SemanticSessionCommandV1::Submit { .. }) && self.unknown_submit_once {
            self.unknown_submit_once = false;
            return SemanticExchangeDispositionV1::OutcomeUnknown;
        }
        if matches!(command, SemanticSessionCommandV1::Submit { .. })
            && self.not_sent_submit_replay_once
        {
            self.not_sent_submit_replay_once = false;
            return SemanticExchangeDispositionV1::NotSent;
        }
        let outcome = match command {
            SemanticSessionCommandV1::Describe { .. } => {
                SemanticSessionOutcomeV1::Description(self.description())
            }
            SemanticSessionCommandV1::JoinSession { .. } => SemanticSessionOutcomeV1::Joined {
                membership: self.membership,
                granted_roles: self.granted_roles,
                remaining_operations: 64,
            },
            SemanticSessionCommandV1::BeginSynchronization { .. }
            | SemanticSessionCommandV1::SyncOpen { .. } => SemanticSessionOutcomeV1::Accepted,
            SemanticSessionCommandV1::CompleteSynchronization { .. } => {
                self.ready = true;
                SemanticSessionOutcomeV1::Accepted
            }
            SemanticSessionCommandV1::Submit { request, .. } => {
                self.request = Some(request.clone());
                SemanticSessionOutcomeV1::Accepted
            }
            SemanticSessionCommandV1::Collect { request, .. } => {
                let retained = self
                    .request
                    .as_ref()
                    .filter(|value| value.request == *request)
                    .unwrap();
                SemanticSessionOutcomeV1::Complete(SemanticReplyV1 {
                    request: retained.request,
                    target: retained.target,
                    index_generation: SemanticIndexGenerationV1::new(3).unwrap(),
                    provider: SemanticProviderProvenanceV1::try_new(
                        SemanticProviderRefV1::new([9; 16]),
                        "fixture".into(),
                        "1".into(),
                    )
                    .unwrap(),
                    result: SemanticResultV1::try_diagnostics(
                        Vec::new(),
                        SemanticLimitsV1::reference(),
                    )
                    .unwrap(),
                })
            }
            SemanticSessionCommandV1::ReleaseRequest { request, .. } => {
                assert_eq!(
                    self.request.as_ref().map(|value| value.request),
                    Some(*request)
                );
                self.request = None;
                SemanticSessionOutcomeV1::Accepted
            }
            SemanticSessionCommandV1::LeaveSession { .. } => SemanticSessionOutcomeV1::Accepted,
            _ => panic!("unexpected command"),
        };
        self.reply(command, outcome)
    }
}

fn attempt(client: &mut SemanticSessionClientV1<WitnessServer>, command: SemanticSessionCommandV1) {
    assert!(matches!(
        client.attempt(command).unwrap(),
        SemanticExchangeDispositionV1::Reply(_)
    ));
}

fn joined_session(roles: SemanticSessionRolesV1) -> SemanticSessionClientV1<WitnessServer> {
    let incarnation = opaque(21, SemanticSessionIncarnationV1::new);
    let membership = opaque(22, SemanticMembershipRefV1::new);
    let mut session = SemanticSessionClientV1::try_new(
        WitnessServer {
            incarnation,
            membership,
            epoch: opaque(23, SemanticSynchronizationEpochV1::new),
            ready: false,
            request: None,
            granted_roles: roles,
            unknown_submit_once: false,
            not_sent_submit_replay_once: false,
        },
        SemanticSessionClientLimitsV1::reference(),
    )
    .unwrap();
    attempt(
        &mut session,
        SemanticSessionCommandV1::Describe {
            exchange: opaque(24, SemanticExchangeRefV1::new),
        },
    );
    attempt(
        &mut session,
        SemanticSessionCommandV1::JoinSession {
            exchange: opaque(25, SemanticExchangeRefV1::new),
            operation: opaque(25, SemanticSessionOperationRefV1::new),
            incarnation,
            requested_roles: roles,
        },
    );
    session
}

#[test]
fn sealed_query_binding_refuses_wrong_role_unsealed_state_and_exhausted_seed() {
    assert!(!valid_semantic_session_correlation_seed(0));
    assert!(!valid_semantic_session_correlation_seed(u128::MAX - 2));
    assert!(valid_semantic_session_correlation_seed(u128::MAX - 3));
    assert!(matches!(
        bind_semantic_session_query_v1(joined_session(SemanticSessionRolesV1::synchronize()), 1),
        Err(SemanticSessionQueryPortBuildRefusalV1::RoleDenied)
    ));
    assert!(matches!(
        bind_semantic_session_query_v1(joined_session(SemanticSessionRolesV1::query()), 1),
        Err(SemanticSessionQueryPortBuildRefusalV1::Unsealed)
    ));
    let failure = match try_bind_semantic_session_query_v1(
        joined_session(SemanticSessionRolesV1::synchronize()),
        1,
    ) {
        Ok(_) => panic!("synchronize-only session unexpectedly bound for queries"),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.reason(),
        SemanticSessionQueryPortBuildRefusalV1::RoleDenied
    );
    assert!(failure.into_session().binding().is_some());
}

#[test]
fn witness_validated_session_becomes_a_sealed_query_port() {
    let incarnation = opaque(1, SemanticSessionIncarnationV1::new);
    let membership = opaque(2, SemanticMembershipRefV1::new);
    let epoch = opaque(3, SemanticSynchronizationEpochV1::new);
    let target = SemanticTargetV1 {
        document: SemanticDocumentRefV1::new([4; 16]),
        source_revision: SemanticSourceRevisionV1::new(1),
    };
    let snapshot = SemanticSessionSnapshotV1::try_new(
        target,
        "rust".into(),
        "fn main() {}".into(),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let manifest = SemanticSynchronizationManifestV1::try_from_snapshots(
        std::slice::from_ref(&snapshot),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    let mut session = SemanticSessionClientV1::try_new(
        WitnessServer {
            incarnation,
            membership,
            epoch,
            ready: false,
            request: None,
            granted_roles: SemanticSessionRolesV1::top(),
            unknown_submit_once: true,
            not_sent_submit_replay_once: true,
        },
        SemanticSessionClientLimitsV1::reference(),
    )
    .unwrap();
    attempt(
        &mut session,
        SemanticSessionCommandV1::Describe {
            exchange: opaque(10, SemanticExchangeRefV1::new),
        },
    );
    attempt(
        &mut session,
        SemanticSessionCommandV1::JoinSession {
            exchange: opaque(11, SemanticExchangeRefV1::new),
            operation: opaque(11, SemanticSessionOperationRefV1::new),
            incarnation,
            requested_roles: SemanticSessionRolesV1::top(),
        },
    );
    attempt(
        &mut session,
        SemanticSessionCommandV1::BeginSynchronization {
            exchange: opaque(12, SemanticExchangeRefV1::new),
            operation: opaque(12, SemanticSessionOperationRefV1::new),
            incarnation,
            membership,
            epoch,
            manifest,
        },
    );
    attempt(
        &mut session,
        SemanticSessionCommandV1::SyncOpen {
            exchange: opaque(13, SemanticExchangeRefV1::new),
            operation: opaque(13, SemanticSessionOperationRefV1::new),
            incarnation,
            membership,
            epoch,
            snapshot,
        },
    );
    attempt(
        &mut session,
        SemanticSessionCommandV1::CompleteSynchronization {
            exchange: opaque(14, SemanticExchangeRefV1::new),
            operation: opaque(14, SemanticSessionOperationRefV1::new),
            incarnation,
            membership,
            epoch,
            manifest,
        },
    );
    attempt(
        &mut session,
        SemanticSessionCommandV1::Describe {
            exchange: opaque(15, SemanticExchangeRefV1::new),
        },
    );

    let mut client = bind_semantic_session_query_v1(session, 100).unwrap();
    let request = SemanticRequestV1 {
        request: SemanticRequestRefV1::new([5; 16]),
        target,
        expected_index: Some(SemanticIndexGenerationV1::new(3).unwrap()),
        language: SemanticTextV1::try_new("rust".into(), 16).unwrap(),
        query: SemanticQueryV1::Diagnostics,
    };
    assert_eq!(
        run(client.submit(request.clone())),
        SemanticSubmitOutcomeV1::OutcomeUnknown
    );
    let mut changed = request.clone();
    changed.language = SemanticTextV1::try_new("other".into(), 16).unwrap();
    assert_eq!(
        run(client.submit(changed)),
        SemanticSubmitOutcomeV1::Refused(SemanticServiceRefusalV1::DuplicateRequest)
    );
    assert_eq!(
        run(client.submit(request.clone())),
        SemanticSubmitOutcomeV1::OutcomeUnknown
    );
    assert_eq!(
        run(client.submit(request)),
        SemanticSubmitOutcomeV1::Accepted
    );
    let suspension = match client.into_port().try_suspend() {
        Ok(suspension) => suspension,
        Err(_) => panic!("definitive submit must suspend between MCP frames"),
    };
    let port = match suspension.try_resume() {
        Ok(port) => port,
        Err(_) => panic!("unchanged witness-validated session must resume"),
    };
    let mut client = SemanticClientV1::new(port);
    let delivery = run(client.take_delivery()).unwrap().unwrap();
    assert!(matches!(
        delivery.outcome,
        SemanticDeliveryOutcomeV1::Peer(SemanticOutcomeV1::Complete(_))
    ));
    assert_eq!(
        run(client.release_request(SemanticRequestRefV1::new([5; 16]))),
        SemanticReleaseOutcomeV1::Released
    );
    let mut suspension = match client.into_port().try_suspend() {
        Ok(suspension) => suspension,
        Err(_) => panic!("completed query port must suspend for reconnect"),
    };
    let binding = suspension.session_mut().binding().unwrap();
    attempt(
        suspension.session_mut(),
        SemanticSessionCommandV1::LeaveSession {
            exchange: opaque(16, SemanticExchangeRefV1::new),
            operation: opaque(16, SemanticSessionOperationRefV1::new),
            incarnation: binding.incarnation,
            membership: binding.membership,
        },
    );
    let (reconnect, mut old_port) = suspension
        .try_into_reconnect()
        .expect("definitively retired session must yield its old transport");
    old_port.incarnation = opaque(31, SemanticSessionIncarnationV1::new);
    old_port.membership = opaque(32, SemanticMembershipRefV1::new);
    old_port.epoch = opaque(33, SemanticSynchronizationEpochV1::new);
    old_port.ready = false;
    old_port.request = None;
    let mut replacement =
        SemanticSessionClientV1::try_new(old_port, SemanticSessionClientLimitsV1::reference())
            .unwrap();
    attempt(
        &mut replacement,
        SemanticSessionCommandV1::Describe {
            exchange: opaque(34, SemanticExchangeRefV1::new),
        },
    );
    attempt(
        &mut replacement,
        SemanticSessionCommandV1::JoinSession {
            exchange: opaque(35, SemanticExchangeRefV1::new),
            operation: opaque(35, SemanticSessionOperationRefV1::new),
            incarnation: opaque(31, SemanticSessionIncarnationV1::new),
            requested_roles: SemanticSessionRolesV1::top(),
        },
    );
    let empty_manifest = SemanticSynchronizationManifestV1::try_from_seals(
        &[],
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    attempt(
        &mut replacement,
        SemanticSessionCommandV1::BeginSynchronization {
            exchange: opaque(36, SemanticExchangeRefV1::new),
            operation: opaque(36, SemanticSessionOperationRefV1::new),
            incarnation: opaque(31, SemanticSessionIncarnationV1::new),
            membership: opaque(32, SemanticMembershipRefV1::new),
            epoch: opaque(33, SemanticSynchronizationEpochV1::new),
            manifest: empty_manifest,
        },
    );
    attempt(
        &mut replacement,
        SemanticSessionCommandV1::CompleteSynchronization {
            exchange: opaque(37, SemanticExchangeRefV1::new),
            operation: opaque(37, SemanticSessionOperationRefV1::new),
            incarnation: opaque(31, SemanticSessionIncarnationV1::new),
            membership: opaque(32, SemanticMembershipRefV1::new),
            epoch: opaque(33, SemanticSynchronizationEpochV1::new),
            manifest: empty_manifest,
        },
    );
    attempt(
        &mut replacement,
        SemanticSessionCommandV1::Describe {
            exchange: opaque(38, SemanticExchangeRefV1::new),
        },
    );
    let rebound = reconnect
        .try_bind(replacement)
        .expect("fresh witness-validated session must bind the reconnect owner");
    let extracted = rebound.into_extraction();
    assert_eq!(extracted.next_exchange, 110);
    assert_eq!(extracted.next_operation, 105);
    assert!(extracted.session.binding().is_some());
}
