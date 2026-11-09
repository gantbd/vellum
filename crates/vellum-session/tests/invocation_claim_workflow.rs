#![cfg(target_os = "macos")]
#[path = "../examples/support/mod.rs"]
mod support;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use support::LocalHost;
use vellum_engine::{
    coordinator::{
        DocumentEdits, HistoryDirection, InvocationLimits, InvocationOutcome, InvocationState,
        SourceIntent,
    },
    source::Replacement,
};
use vellum_session::host::{InvocationClaimErrorV1, InvocationClaimKindV1 as Kind, SessionError};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "vellum-claim-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::write(path.join("source"), "\u{feff}alpha\r\n").unwrap();
                    return Self(path);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("fixture: {e}"),
            }
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_file(self.0.join("source")).unwrap();
        fs::remove_dir(&self.0).unwrap();
    }
}
fn host(capacity: usize) -> LocalHost {
    let mut config = support::config(capacity, true);
    config.engine.invocation = Some(InvocationLimits {
        max_lifetime_issued: 4,
        max_retained: 2,
        max_intent_bytes: 65536,
    });
    LocalHost::new(config).unwrap()
}
fn owned<T>(result: Result<T, SessionError>) {
    assert!(matches!(
        result,
        Err(SessionError::InvocationClaim(InvocationClaimErrorV1::Owned))
    ));
}
fn wrong_kind<T>(result: Result<T, SessionError>) {
    assert!(matches!(
        result,
        Err(SessionError::InvocationClaim(
            InvocationClaimErrorV1::WrongKind
        ))
    ));
}

#[test]
fn publication_claim_blocks_bypass_and_reuses_original_native_outcome_and_history() {
    for capacity in [0, 1, 256] {
        let fixture = Fixture::new();
        let mut host = host(capacity);
        let document = host.open(&fixture.0.join("source")).unwrap();
        let original = host.read(&document).unwrap().version;
        let id = host.issue_invocation().unwrap();
        let claim = host.claim_invocation(&id, Kind::SourcePublication).unwrap();
        let request = SourceIntent {
            writes: vec![DocumentEdits {
                expected: original.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 5,
                    expected: "alpha".into(),
                    inserted: "Grüße".into(),
                }],
            }],
            reads: vec![],
        };
        owned(host.claim_invocation(&id, Kind::SourceNeutral));
        owned(host.submit_source_invocation(&id, &request));
        owned(host.run_invocation(&id));
        owned(host.stop_invocation(&id));
        owned(host.forget_invocation_outcome(&id));
        owned(host.retire_invocation(&id));
        assert_eq!(host.invocation_state(&id).unwrap(), InvocationState::Issued);
        assert_eq!(host.read(&document).unwrap().version, original);
        assert_eq!(
            host.submit_claimed_source_invocation(&claim, &request)
                .unwrap(),
            InvocationState::Pending
        );
        let mut changed = request.clone();
        changed.writes[0].edits[0].inserted = "different".into();
        assert!(
            host.submit_claimed_source_invocation(&claim, &changed)
                .is_err()
        );
        assert!(host.retire_claimed_invocation(&claim).is_err());
        assert_eq!(host.invocation_claim_usage().retained, 1);
        assert_eq!(
            host.run_claimed_invocation(&claim).unwrap(),
            InvocationState::Finished
        );
        assert!(matches!(
            host.invocation_outcome(&id).unwrap(),
            InvocationOutcome::Committed(_)
        ));
        let committed = host.read(&document).unwrap().version;
        assert_eq!(host.read(&document).unwrap().source.text(), "Grüße\r\n");
        assert_eq!(
            host.submit_claimed_source_invocation(&claim, &request)
                .unwrap(),
            InvocationState::Finished
        );
        assert_eq!(
            host.run_claimed_invocation(&claim).unwrap(),
            InvocationState::Finished
        );
        assert_eq!(host.read(&document).unwrap().version, committed);
        assert!(host.reverse(&document, HistoryDirection::Undo).unwrap());
        let undone = host.read(&document).unwrap().version;
        host.run_claimed_invocation(&claim).unwrap();
        assert_eq!(host.read(&document).unwrap().version, undone);
        assert_eq!(host.read(&document).unwrap().source.text(), "alpha\r\n");
        host.forget_claimed_invocation_outcome(&claim).unwrap();
        assert!(host.invocation_outcome(&id).is_err());
        host.retire_claimed_invocation(&claim).unwrap();
        assert!(host.claim_invocation(&id, Kind::SourcePublication).is_err());
        assert!(host.run_claimed_invocation(&claim).is_err());
        assert!(host.submit_source_invocation(&id, &request).is_err());
        let usage = host.invocation_claim_usage();
        assert_eq!(
            (usage.retained, usage.limit, usage.accepted, usage.retired),
            (0, 2, 1, 1)
        );
        assert!(usage.refused >= 8);
        assert_eq!(host.invocation_usage().issued_lifetime, 1);
        assert_eq!(
            fs::read(fixture.0.join("source")).unwrap(),
            "\u{feff}alpha\r\n".as_bytes()
        );
        assert!(host.into_parts().is_ok());
    }
}

#[test]
fn neutral_claim_never_creates_a_source_outcome_and_unpack_refusal_preserves_ownership() {
    let mut host = host(0);
    let id = host.issue_invocation().unwrap();
    let claim = host.claim_invocation(&id, Kind::SourceNeutral).unwrap();
    assert_eq!(claim.origin(), &id);
    assert_eq!(claim.kind(), Kind::SourceNeutral);
    assert_eq!(format!("{claim:?}"), "InvocationClaimV1(<redacted>)");
    let empty = SourceIntent {
        writes: vec![],
        reads: vec![],
    };
    wrong_kind(host.submit_claimed_source_invocation(&claim, &empty));
    wrong_kind(host.run_claimed_invocation(&claim));
    wrong_kind(host.stop_claimed_invocation(&claim));
    wrong_kind(host.forget_claimed_invocation_outcome(&claim));
    let mut host = match host.into_parts() {
        Err(host) => host,
        Ok(_) => panic!("unpacked an outstanding claim"),
    };
    assert_eq!(host.invocation_state(&id).unwrap(), InvocationState::Issued);
    assert!(host.invocation_outcome(&id).is_err());
    assert_eq!(host.invocation_claim_usage().retained, 1);
    host.retire_claimed_invocation(&claim).unwrap();
    assert!(host.invocation_state(&id).is_err());
    assert!(host.into_parts().is_ok());
}

#[test]
fn foreign_and_pending_claims_refuse_and_drop_cannot_unlock_or_reissue() {
    let mut a = host(1);
    let mut b = host(256);
    let id = a.issue_invocation().unwrap();
    assert!(b.claim_invocation(&id, Kind::SourceNeutral).is_err());
    let claim = a.claim_invocation(&id, Kind::SourcePublication).unwrap();
    assert!(b.run_claimed_invocation(&claim).is_err());
    assert!(b.retire_claimed_invocation(&claim).is_err());
    assert_eq!(a.invocation_claim_usage().retained, 1);
    drop(claim);
    owned(a.claim_invocation(&id, Kind::SourcePublication));
    owned(a.retire_invocation(&id));
    assert!(a.into_parts().is_err());

    let fixture = Fixture::new();
    let document = b.open(&fixture.0.join("source")).unwrap();
    let id = b.issue_invocation().unwrap();
    let intent = SourceIntent {
        writes: vec![],
        reads: vec![b.read(&document).unwrap().version],
    };
    b.submit_source_invocation(&id, &intent).unwrap();
    assert!(b.claim_invocation(&id, Kind::SourcePublication).is_err());
    b.stop_invocation(&id).unwrap();
    assert!(b.claim_invocation(&id, Kind::SourceNeutral).is_err());
    b.retire_invocation(&id).unwrap();
    assert_eq!(b.invocation_claim_usage().retained, 0);
}

#[test]
fn explicit_stop_and_retirement_preserve_native_retained_and_lifetime_limits() {
    let fixture = Fixture::new();
    let mut host = host(256);
    let document = host.open(&fixture.0.join("source")).unwrap();
    let source = host.read(&document).unwrap().version;
    let intent = SourceIntent {
        writes: vec![],
        reads: vec![source.clone()],
    };
    let id = host.issue_invocation().unwrap();
    let claim = host.claim_invocation(&id, Kind::SourcePublication).unwrap();
    host.submit_claimed_source_invocation(&claim, &intent)
        .unwrap();
    host.stop_claimed_invocation(&claim).unwrap();
    assert!(matches!(
        host.invocation_outcome(&id).unwrap(),
        InvocationOutcome::StoppedBeforePublication
    ));
    assert_eq!(host.read(&document).unwrap().version, source);
    host.retire_claimed_invocation(&claim).unwrap();
    let a = host.issue_invocation().unwrap();
    let b = host.issue_invocation().unwrap();
    let a = host.claim_invocation(&a, Kind::SourceNeutral).unwrap();
    let b = host.claim_invocation(&b, Kind::SourceNeutral).unwrap();
    assert_eq!(host.invocation_claim_usage().retained, 2);
    assert!(host.issue_invocation().is_err());
    host.retire_claimed_invocation(&a).unwrap();
    host.retire_claimed_invocation(&b).unwrap();
    let last = host.issue_invocation().unwrap();
    let last = host.claim_invocation(&last, Kind::SourceNeutral).unwrap();
    host.retire_claimed_invocation(&last).unwrap();
    assert_eq!(host.invocation_usage().issued_lifetime, 4);
    assert!(host.issue_invocation().is_err());
    let usage = host.invocation_claim_usage();
    assert_eq!((usage.retained, usage.accepted, usage.retired), (0, 4, 4));
}
