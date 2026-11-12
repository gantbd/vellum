use super::*;

use std::fs;
use std::os::unix::{
    fs::PermissionsExt as _,
    net::{UnixListener, UnixStream},
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use vellum_host::{
    authentication::{
        AuthenticationChallengeRefV1, AuthenticationClaimRefV1, AuthenticationDecisionV1,
        AuthenticationEvidenceV1, AuthenticationGateV1, AuthenticationPortV1,
        AuthenticationRequestV1,
    },
    authority::{AuthoritySessionRefV1, AuthoritySubjectRefV1},
    semantic::{SemanticRequestRefV1, SemanticRequestV1, SemanticTargetV1},
};
use vellum_semantic_protocol::{
    SemanticExchangeRefV1, SemanticSessionCommandV1, SemanticSessionIncarnationV1,
    SemanticSessionLimitsV1, SemanticSessionOperationRefV1, SemanticSessionOutcomeV1,
    SemanticSessionRefusalV1, SemanticSessionSnapshotV1,
};
use vellum_semantic_session_actor::{
    SemanticBrokerEffectFailureV1, SemanticBrokerEffectObligationsV1, SemanticBrokerEffectPortV1,
    SemanticBrokerProviderDriveV1, SemanticSessionActorLimitsV1, spawn_semantic_session_actor_v1,
};
use vellum_semantic_session_broker::{
    SemanticBrokerCancellationReportV1, SemanticBrokerExpiryDriveV1, SemanticBrokerOpenReportV1,
    SemanticBrokerSourceReportV1, SemanticBrokerSubmissionReportV1, SemanticSessionBrokerV1,
};
use vellum_semantic_session_client::{SemanticSessionClientLimitsV1, SemanticSessionClientV1};
use vellum_semantic_transport::{
    SemanticRemoteSessionV1, SemanticTransportLimitsV1, UnixSemanticFrameDeadlinesV1,
    UnixSemanticFrameExchangeV1,
};
use vellum_semantic_wire::{decode_reply_v1, encode_command_v1};

const INCARNATION: [u8; 16] = [0x31; 16];
static NEXT_SOCKET_DIRECTORY: AtomicU64 = AtomicU64::new(1);

struct ExactAuthentication {
    subject: AuthoritySubjectRefV1,
}

impl AuthenticationPortV1 for ExactAuthentication {
    type Context = (AuthenticationClaimRefV1,);

    fn authenticate(
        &mut self,
        request: AuthenticationRequestV1,
        context: &Self::Context,
    ) -> AuthenticationDecisionV1 {
        if request.claim == context.0 {
            AuthenticationDecisionV1::Authenticated(AuthenticationEvidenceV1::from_port(
                request,
                self.subject,
            ))
        } else {
            AuthenticationDecisionV1::Rejected
        }
    }
}

struct FixedRolePolicy {
    decision: AuthenticatedSemanticRoleDecisionV1,
    observed: Option<AuthorityContextV1>,
}

impl AuthenticatedSemanticRolePortV1 for FixedRolePolicy {
    type Context = ();

    fn derive_roles(
        &mut self,
        authenticated: AuthorityContextV1,
        _context: &Self::Context,
    ) -> AuthenticatedSemanticRoleDecisionV1 {
        self.observed = Some(authenticated);
        self.decision
    }
}

struct NoEffects;

impl SemanticBrokerEffectPortV1 for NoEffects {
    fn obligations(&self) -> SemanticBrokerEffectObligationsV1 {
        SemanticBrokerEffectObligationsV1 {
            request_deadlines: 0,
            due_deadlines: 0,
            poisoned: false,
            consistent: true,
        }
    }

    fn open(
        &mut self,
        _snapshot: &SemanticSessionSnapshotV1,
    ) -> Result<SemanticBrokerOpenReportV1, SemanticBrokerEffectFailureV1> {
        Err(SemanticBrokerEffectFailureV1::RuntimeUnavailable)
    }

    fn change(
        &mut self,
        _snapshot: &SemanticSessionSnapshotV1,
    ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
        Err(SemanticBrokerEffectFailureV1::RuntimeUnavailable)
    }

    fn close(
        &mut self,
        _target: SemanticTargetV1,
    ) -> Result<SemanticBrokerSourceReportV1, SemanticBrokerEffectFailureV1> {
        Err(SemanticBrokerEffectFailureV1::RuntimeUnavailable)
    }

    fn submit(
        &mut self,
        _request: &SemanticRequestV1,
    ) -> Result<SemanticBrokerSubmissionReportV1, SemanticBrokerEffectFailureV1> {
        Err(SemanticBrokerEffectFailureV1::RuntimeUnavailable)
    }

    fn cancel(
        &mut self,
        _request: SemanticRequestRefV1,
    ) -> Result<SemanticBrokerCancellationReportV1, SemanticBrokerEffectFailureV1> {
        Err(SemanticBrokerEffectFailureV1::RuntimeUnavailable)
    }

    fn drive_provider(
        &mut self,
    ) -> Result<SemanticBrokerProviderDriveV1, SemanticBrokerEffectFailureV1> {
        Err(SemanticBrokerEffectFailureV1::RuntimeUnavailable)
    }

    fn drive_expiry(
        &mut self,
    ) -> Result<SemanticBrokerExpiryDriveV1, SemanticBrokerEffectFailureV1> {
        Err(SemanticBrokerEffectFailureV1::RuntimeUnavailable)
    }
}

fn authenticated(tag: u8) -> (AuthenticatedSessionV1, AuthorityContextV1) {
    let subject = AuthoritySubjectRefV1::new([tag; 16]);
    let request = AuthenticationRequestV1 {
        challenge: AuthenticationChallengeRefV1::new([tag.wrapping_add(1); 16]),
        claim: AuthenticationClaimRefV1::new([tag.wrapping_add(2); 16]),
        session: AuthoritySessionRefV1::new([tag.wrapping_add(3); 16]),
    };
    let mut gate = AuthenticationGateV1::new(ExactAuthentication { subject }, 1);
    let session = gate.establish(request, &(request.claim,)).unwrap();
    let context = session.authority_context();
    (session, context)
}

fn actor() -> (
    vellum_semantic_session_actor::SemanticSessionActorOwnerV1,
    SemanticSessionActorSenderV1,
    vellum_semantic_session_actor::SemanticSessionActorRecoveryV1<NoEffects>,
) {
    let broker = SemanticSessionBrokerV1::try_new(
        SemanticSessionIncarnationV1::new(INCARNATION).unwrap(),
        SemanticSessionLimitsV1::reference(),
    )
    .unwrap();
    spawn_semantic_session_actor_v1(broker, NoEffects, SemanticSessionActorLimitsV1::reference())
        .unwrap()
}

fn join(tag: u8, requested_roles: SemanticSessionRolesV1) -> SemanticSessionCommandV1 {
    SemanticSessionCommandV1::JoinSession {
        exchange: SemanticExchangeRefV1::new([tag; 16]).unwrap(),
        operation: SemanticSessionOperationRefV1::new([tag.wrapping_add(1); 16]).unwrap(),
        incarnation: SemanticSessionIncarnationV1::new(INCARNATION).unwrap(),
        requested_roles,
    }
}

fn write_frame(stream: &mut UnixStream, frame: &[u8]) {
    stream
        .write_all(&u32::try_from(frame.len()).unwrap().to_be_bytes())
        .unwrap();
    stream.write_all(frame).unwrap();
    stream.flush().unwrap();
}

fn read_frame(stream: &mut UnixStream) -> Vec<u8> {
    let mut length = [0; 4];
    stream.read_exact(&mut length).unwrap();
    let mut frame = vec![0; u32::from_be_bytes(length) as usize];
    stream.read_exact(&mut frame).unwrap();
    frame
}

fn recover_actor(
    owner: vellum_semantic_session_actor::SemanticSessionActorOwnerV1,
    recovery: vellum_semantic_session_actor::SemanticSessionActorRecoveryV1<NoEffects>,
) {
    drop(owner);
    let mut recovery = recovery;
    loop {
        match recovery.try_recover() {
            Ok(_) => break,
            Err(pending) => recovery = pending,
        }
        std::thread::yield_now();
    }
}

struct SocketFixture {
    directory: std::path::PathBuf,
    path: std::path::PathBuf,
}

impl SocketFixture {
    fn new() -> (Self, UnixListener) {
        let sequence = NEXT_SOCKET_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let directory = std::path::PathBuf::from("/tmp").join(format!(
            "vellum-semantic-local-integration-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let directory = fs::canonicalize(directory).unwrap();
        let path = directory.join("session.sock");
        let listener = UnixListener::bind(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        (Self { directory, path }, listener)
    }
}

impl Drop for SocketFixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_dir(&self.directory);
    }
}

fn run_connection(
    roles: SemanticSessionRolesV1,
    command: SemanticSessionCommandV1,
) -> (
    vellum_semantic_protocol::SemanticSessionReplyV1,
    AuthenticatedSemanticConnectionReceiptV1,
) {
    let (owner, sender, recovery) = actor();
    let (authenticated, expected_context) = authenticated(0x51);
    let mut policy = FixedRolePolicy {
        decision: AuthenticatedSemanticRoleDecisionV1::Grant {
            roles,
            policy_epoch: 7,
        },
        observed: None,
    };
    let mut dispatcher = AuthenticatedSemanticDispatcherV1::try_new(
        authenticated,
        &mut policy,
        &(),
        sender,
        AuthenticatedSemanticDispatchLimitsV1 {
            max_exchanges: 1,
            wire: SemanticWireLimitsV1::reference(),
        },
    )
    .unwrap();
    assert_eq!(policy.observed, Some(expected_context));
    assert_eq!(
        dispatcher.authority_lost_event(),
        SemanticSessionActorExternalEventV1::PolicyLost {
            subject: expected_context.subject,
            session: expected_context.session.unwrap(),
            policy_epoch: 7,
        }
    );
    let (mut client, mut server) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    server
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let encoded = encode_command_v1(&command, SemanticWireLimitsV1::reference()).unwrap();
    let worker = std::thread::spawn(move || {
        let receipt = serve_one_authenticated_semantic_exchange_v1(
            &mut dispatcher,
            &mut server,
            AuthenticatedSemanticConnectionLimitsV1::reference(),
        )
        .unwrap();
        (receipt, dispatcher)
    });
    write_frame(&mut client, &encoded);
    let reply =
        decode_reply_v1(&read_frame(&mut client), SemanticWireLimitsV1::reference()).unwrap();
    let (receipt, dispatcher) = worker.join().unwrap();
    assert_eq!(receipt.request_bytes, encoded.len());
    assert!(receipt.reply_bytes > 0);
    assert_eq!(receipt.usage.connection_exchanges, 1);
    assert_eq!(receipt.usage.actor_submissions, 1);
    assert_eq!(receipt.usage.replies, 1);
    assert!(!format!("{dispatcher:?}").contains("Authority"));
    drop(dispatcher);
    recover_actor(owner, recovery);
    (reply, receipt)
}

#[test]
fn unix_transport_and_authenticated_dispatch_complete_a_real_client_session() {
    let (owner, sender, recovery) = actor();
    let (authenticated, _) = authenticated(0x91);
    let mut policy = FixedRolePolicy {
        decision: AuthenticatedSemanticRoleDecisionV1::Grant {
            roles: SemanticSessionRolesV1::query(),
            policy_epoch: 11,
        },
        observed: None,
    };
    let mut dispatcher = AuthenticatedSemanticDispatcherV1::try_new(
        authenticated,
        &mut policy,
        &(),
        sender,
        AuthenticatedSemanticDispatchLimitsV1 {
            max_exchanges: 2,
            wire: SemanticWireLimitsV1::reference(),
        },
    )
    .unwrap();
    let (fixture, listener) = SocketFixture::new();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        for _ in 0..2 {
            serve_one_authenticated_semantic_exchange_v1(
                &mut dispatcher,
                &mut stream,
                AuthenticatedSemanticConnectionLimitsV1::reference(),
            )
            .unwrap();
        }
        dispatcher
    });

    let exchange = UnixSemanticFrameExchangeV1::try_new(
        fixture.path.clone(),
        UnixSemanticFrameDeadlinesV1::reference(),
    )
    .unwrap();
    let remote =
        SemanticRemoteSessionV1::try_new(exchange, SemanticTransportLimitsV1::reference()).unwrap();
    let mut client =
        SemanticSessionClientV1::try_new(remote, SemanticSessionClientLimitsV1::reference())
            .unwrap();

    let described = client
        .attempt(SemanticSessionCommandV1::Describe {
            exchange: SemanticExchangeRefV1::new([0xa1; 16]).unwrap(),
        })
        .unwrap();
    assert!(matches!(
        described,
        SemanticExchangeDispositionV1::Reply(ref reply)
            if matches!(reply.outcome(), SemanticSessionOutcomeV1::Description(_))
    ));
    let joined = client
        .attempt(join(0xa2, SemanticSessionRolesV1::top()))
        .unwrap();
    assert!(matches!(
        joined,
        SemanticExchangeDispositionV1::Reply(ref reply)
            if matches!(
                reply.outcome(),
                SemanticSessionOutcomeV1::Joined { granted_roles, .. }
                    if *granted_roles == SemanticSessionRolesV1::query()
            )
    ));
    assert_eq!(
        client.binding().unwrap().roles,
        SemanticSessionRolesV1::query()
    );

    let dispatcher = server.join().unwrap();
    assert_eq!(dispatcher.usage().connection_exchanges, 2);
    drop(dispatcher);
    recover_actor(owner, recovery);
}

#[test]
fn real_authenticated_stream_grants_only_the_independent_role_meet() {
    let (reply, _) = run_connection(
        SemanticSessionRolesV1::query(),
        join(1, SemanticSessionRolesV1::top()),
    );
    assert!(matches!(
        reply.outcome(),
        SemanticSessionOutcomeV1::Joined { granted_roles, .. }
            if *granted_roles == SemanticSessionRolesV1::query()
    ));
}

#[test]
fn real_authenticated_stream_with_bottom_policy_refuses_join() {
    let (reply, receipt) = run_connection(
        SemanticSessionRolesV1::bottom(),
        join(3, SemanticSessionRolesV1::query()),
    );
    assert_eq!(
        reply.outcome(),
        &SemanticSessionOutcomeV1::Refused(SemanticSessionRefusalV1::RoleDenied)
    );
    assert_eq!(receipt.usage.role_derivations, 1);
}

#[test]
fn role_policy_refusal_retains_authenticated_and_actor_ownership() {
    let (owner, sender, recovery) = actor();
    let (authenticated, _) = authenticated(0x61);
    let mut policy = FixedRolePolicy {
        decision: AuthenticatedSemanticRoleDecisionV1::Denied,
        observed: None,
    };
    let failure = AuthenticatedSemanticDispatcherV1::try_new(
        authenticated,
        &mut policy,
        &(),
        sender,
        AuthenticatedSemanticDispatchLimitsV1::reference(),
    )
    .unwrap_err();
    assert_eq!(
        failure.reason(),
        AuthenticatedSemanticDispatchBuildRefusalV1::RoleDenied
    );
    assert_eq!(failure.usage().role_derivations, 1);
    assert!(!format!("{failure:?}").contains("0x61"));
    let (_authenticated, sender) = failure.into_parts();
    drop(sender);
    recover_actor(owner, recovery);
}

#[test]
fn malformed_frame_consumes_bounded_connection_budget_before_actor_admission() {
    let (owner, sender, recovery) = actor();
    let (authenticated, _) = authenticated(0x71);
    let mut policy = FixedRolePolicy {
        decision: AuthenticatedSemanticRoleDecisionV1::Grant {
            roles: SemanticSessionRolesV1::query(),
            policy_epoch: 9,
        },
        observed: None,
    };
    let mut dispatcher = AuthenticatedSemanticDispatcherV1::try_new(
        authenticated,
        &mut policy,
        &(),
        sender,
        AuthenticatedSemanticDispatchLimitsV1 {
            max_exchanges: 1,
            wire: SemanticWireLimitsV1::reference(),
        },
    )
    .unwrap();
    assert!(matches!(
        dispatcher.try_dispatch(b"not-semantic-wire"),
        Err(AuthenticatedSemanticDispatchRefusalV1::Decode(_))
    ));
    let valid = encode_command_v1(
        &join(8, SemanticSessionRolesV1::query()),
        SemanticWireLimitsV1::reference(),
    )
    .unwrap();
    assert_eq!(
        dispatcher.try_dispatch(&valid).unwrap_err(),
        AuthenticatedSemanticDispatchRefusalV1::BudgetExhausted
    );
    let usage = dispatcher.usage();
    assert_eq!(usage.frames_attempted, 2);
    assert_eq!(usage.decode_refusals, 1);
    assert_eq!(usage.budget_refusals, 1);
    assert_eq!(usage.actor_submissions, 0);
    drop(dispatcher);
    recover_actor(owner, recovery);
}

#[test]
fn application_owned_client_bridge_preserves_actor_exchange_behavior() {
    let (owner, sender, recovery) = actor();
    let (_, authority) = authenticated(0x81);
    let mut port = SemanticSessionActorExchangePortV1::new(
        sender,
        SemanticBrokerContextV1 {
            subject: authority.subject,
            session: authority.session.unwrap(),
            allowed_roles: SemanticSessionRolesV1::query(),
            policy_epoch: 11,
        },
    );
    let disposition =
        SemanticSessionExchangePortV1::exchange(&mut port, &join(9, SemanticSessionRolesV1::top()));
    assert!(matches!(
        disposition,
        SemanticExchangeDispositionV1::Reply(reply)
            if matches!(
                reply.outcome(),
                SemanticSessionOutcomeV1::Joined { granted_roles, .. }
                    if *granted_roles == SemanticSessionRolesV1::query()
            )
    ));
    assert_eq!(
        format!("{port:?}"),
        "SemanticSessionActorExchangePortV1(<redacted>)"
    );
    drop(port);
    recover_actor(owner, recovery);
}
