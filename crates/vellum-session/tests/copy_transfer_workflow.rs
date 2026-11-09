#![cfg(target_os = "macos")]
#[path = "copy_transfer_workflow/cut.rs"]
mod cut;
#[path = "copy_transfer_workflow/paste.rs"]
mod paste;
#[path = "../examples/support/mod.rs"]
mod support;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use vellum_engine::{
    attachment::Affinity,
    coordinates::BytePosition,
    coordinator::{InvocationLimits, InvocationState, LogicalViewContext, LogicalViewLimits},
    selection::{Selection, SelectionBudget},
    source::Replacement,
};
use vellum_session::host::{
    LocalDocument, LocalHost, SessionError, TextTransferCaptureLimitsV1, copy_transfer::*,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "vellum-copy-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::write(path.join("source"), "\u{feff}ab\r\nGrüße\nend").unwrap();
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
fn limits() -> CopyLimitsV1 {
    CopyLimitsV1 {
        max_provider_registrations: 2,
        max_profile_bytes: 64,
        max_tasks: 4,
        max_held_bytes: 32768,
        capture: TextTransferCaptureLimitsV1 {
            max_fragments: 8,
            max_plain_bytes: 4096,
            max_logical_bytes: 8192,
        },
    }
}
fn config(capacity: usize, timed: bool) -> vellum_session::host::SessionConfig {
    let mut config = support::config(capacity, timed);
    config.engine.invocation = Some(InvocationLimits {
        max_lifetime_issued: 16,
        max_retained: 8,
        max_intent_bytes: 65536,
    });
    config.logical_views = Some(LogicalViewLimits {
        max_live_views: 4,
        max_issued_views: 4,
        max_total_selections: 16,
        normalization: SelectionBudget {
            max_input_selections: 8,
            max_output_selections: 8,
            max_work: 65536,
        },
        max_mapping_steps: 65536,
    });
    config
}
fn host(capacity: usize, timed: bool, limits: CopyLimitsV1) -> LocalHost {
    let mut host = LocalHost::new(config(capacity, timed)).unwrap();
    host.configure_copy_transfers(limits).unwrap();
    host
}
fn registration() -> CopyProviderRegistrationV1 {
    CopyProviderRegistrationV1 {
        destination: CopyDestinationV1::SessionClipboard,
        readable: true,
        writable: true,
        acknowledgment_profile: "test-owned-copy-acceptance/1".into(),
    }
}
fn range(a: usize, b: usize) -> Selection {
    Selection {
        anchor: BytePosition(a),
        head: BytePosition(b),
        affinity: Affinity::After,
    }
}
fn context(
    host: &mut LocalHost,
    f: &Fixture,
    selections: &[Selection],
) -> (LocalDocument, LogicalViewContext) {
    let document = host.open(&f.0.join("source")).unwrap();
    let source = host.read(&document).unwrap().version;
    let view = host.create_logical_view().unwrap();
    let stamp = host.logical_view(&view).unwrap().stamp;
    let attached = host
        .attach_logical_view(&view, stamp, &source, selections, 0)
        .unwrap();
    (
        document,
        LogicalViewContext {
            view,
            source,
            stamp: attached.stamp,
        },
    )
}
fn admission() -> CopyAdmissionFactsV1 {
    CopyAdmissionFactsV1 {
        read_allowed: Some(true),
        disclosure_allowed: Some(true),
        retention_allowed: Some(true),
    }
}
fn launch() -> CopyDispatchFactsV1 {
    CopyDispatchFactsV1 {
        launch_allowed: Some(true),
        disclosure_allowed: Some(true),
    }
}
fn release() -> CopyReleaseFactsV1 {
    CopyReleaseFactsV1 {
        release_allowed: Some(true),
        disclosure_allowed: Some(true),
    }
}
fn evidence() -> CopyEvidenceFactsV1 {
    CopyEvidenceFactsV1 {
        authentic: Some(true),
        evidence_allowed: Some(true),
        ingress_ready: Some(true),
    }
}
fn retirement() -> CopyRetirementFactsV1 {
    CopyRetirementFactsV1 {
        retention_released: Some(true),
        delivery_drained: Some(true),
        session_references_released: Some(true),
    }
}
fn error<T>(result: Result<T, SessionError>, expected: CopyErrorV1) {
    assert!(matches!(result, Err(SessionError::CopyTransfer(actual)) if actual == expected));
}

#[test]
fn copy_uses_original_native_claim_and_receipts_without_source_history_or_disk_mutation() {
    for (capacity, timed) in [(0, false), (0, true), (1, true), (256, true)] {
        let f = Fixture::new();
        let mut host = host(capacity, timed, limits());
        let provider = host.register_copy_provider(registration()).unwrap();
        let (document, context) = context(&mut host, &f, &[range(4, 11), range(0, 1)]);
        host.set_editing_locked(&document, true).unwrap();
        let id = host.issue_invocation().unwrap();
        let task = host
            .admit_copy(&id, &context, &provider, admission())
            .unwrap();
        let retry = host
            .admit_copy(&id, &context, &provider, admission())
            .unwrap();
        assert_eq!(retry.origin(), task.origin());
        assert_eq!(
            host.copy_status(&task).unwrap().effect,
            CopyEffectV1::NotDispatched
        );
        assert!(host.retire_invocation(&id).is_err());
        assert!(host.run_invocation(&id).is_err());
        assert_eq!(host.invocation_claim_usage().retained, 1);
        let dispatch = host.dispatch_copy(&task, launch()).unwrap();
        error(
            host.dispatch_copy(&retry, launch()),
            CopyErrorV1::AlreadyDispatched,
        );
        assert_eq!(
            host.copy_status(&task).unwrap().effect,
            CopyEffectV1::Unknown
        );
        let usage = host.copy_usage().unwrap();
        assert_eq!((usage.unknown_tasks, usage.provider_work), (1, 1));
        assert_eq!(usage.oldest_unknown.is_some(), timed);
        assert_eq!(usage.oldest_provider_work.is_some(), timed);
        assert_eq!(
            host.release_copy_plain(&provider, &dispatch, release())
                .unwrap(),
            "a\nGrüße"
        );
        error(
            host.retire_copy_provider(&provider, retirement()),
            CopyErrorV1::Outstanding,
        );
        error(
            host.retire_copy(&task, retirement()),
            CopyErrorV1::Outstanding,
        );
        assert_eq!(
            host.record_copy_evidence(&provider, &dispatch, CopyEvidenceV1::Accepted, evidence())
                .unwrap(),
            CopyEvidenceDispositionV1::Recorded
        );
        assert_eq!(
            host.record_copy_evidence(&provider, &dispatch, CopyEvidenceV1::Accepted, evidence())
                .unwrap(),
            CopyEvidenceDispositionV1::Duplicate
        );
        error(
            host.retire_copy(&task, retirement()),
            CopyErrorV1::Outstanding,
        );
        host.record_copy_evidence(&provider, &dispatch, CopyEvidenceV1::Quiescent, evidence())
            .unwrap();
        let status = host.copy_status(&task).unwrap();
        assert_eq!(status.effect, CopyEffectV1::Accepted);
        assert!(status.quiescent);
        assert_eq!(status.first_result_after.is_some(), timed);
        assert_eq!(status.quiescent_after.is_some(), timed);
        assert_eq!(host.invocation_state(&id).unwrap(), InvocationState::Issued);
        assert!(host.invocation_outcome(&id).is_err());
        assert_eq!(host.read(&document).unwrap().version, context.source);
        assert_eq!(
            host.logical_view(&context.view).unwrap().context(),
            Some(context.clone())
        );
        assert_eq!(host.history_usage().retained_groups, 0);
        assert_eq!(
            fs::read(f.0.join("source")).unwrap(),
            "\u{feff}ab\r\nGrüße\nend".as_bytes()
        );
        assert_eq!(
            format!("{provider:?} {task:?} {dispatch:?}"),
            "CopyProviderPortV1(<redacted>) CopyTaskV1(<redacted>) CopyDispatchV1(<redacted>)"
        );
        host.retire_copy(&task, retirement()).unwrap();
        assert!(host.invocation_state(&id).is_err());
        assert!(host.copy_status(&retry).is_err());
        assert!(
            host.admit_copy(&id, &context, &provider, admission())
                .is_err()
        );
        assert_eq!(host.invocation_claim_usage().retained, 0);
        let mut host = match host.into_parts() {
            Err(host) => host,
            Ok(_) => panic!("forgot provider ownership"),
        };
        let usage = host.copy_usage().unwrap();
        assert_eq!(
            (
                usage.admitted,
                usage.request_replays,
                usage.dispatched,
                usage.payload_releases
            ),
            (1, 1, 1, 1)
        );
        assert_eq!(
            (
                usage.receipts_recorded,
                usage.receipts_duplicated,
                usage.conflicts,
                usage.retired
            ),
            (2, 1, 0, 1)
        );
        assert_eq!(
            (usage.held_tasks, usage.unknown_tasks, usage.provider_work),
            (0, 0, 0)
        );
        assert_eq!(
            usage.operation(CopyOperationV1::Admit),
            CopyOperationUsageV1 {
                offers: 3,
                refusals: 1
            }
        );
        assert_eq!(
            usage.operation(CopyOperationV1::Dispatch),
            CopyOperationUsageV1 {
                offers: 2,
                refusals: 1
            }
        );
        let phases = [
            CopyOperationV1::RegisterProvider,
            CopyOperationV1::RevokeProvider,
            CopyOperationV1::Admit,
            CopyOperationV1::Dispatch,
            CopyOperationV1::ReleasePayload,
            CopyOperationV1::RecordEvidence,
            CopyOperationV1::Stop,
            CopyOperationV1::RetireTask,
            CopyOperationV1::RetireProvider,
        ];
        assert_eq!(
            phases
                .iter()
                .map(|p| usage.operation(*p).offers)
                .sum::<u64>(),
            usage.attempts
        );
        assert_eq!(
            phases
                .iter()
                .map(|p| usage.operation(*p).refusals)
                .sum::<u64>(),
            usage.refusals
        );
        host.retire_copy_provider(&provider, retirement()).unwrap();
        if capacity == 1 {
            assert!(host.dropped_diagnostics() > 0);
        }
        assert!(host.into_parts().is_ok());
    }
}

#[test]
fn stop_quiescence_revocation_and_forgery_never_erase_unknown_or_contested_effects() {
    let f = Fixture::new();
    let mut host = host(1, true, limits());
    let provider = host.register_copy_provider(registration()).unwrap();
    let impostor = host.register_copy_provider(registration()).unwrap();
    let (document, context) = context(&mut host, &f, &[range(0, 2)]);
    let id = host.issue_invocation().unwrap();
    let task = host
        .admit_copy(&id, &context, &provider, admission())
        .unwrap();
    let dispatch = host.dispatch_copy(&task, launch()).unwrap();
    assert!(
        host.release_copy_plain(&impostor, &dispatch, release())
            .is_err()
    );
    assert!(
        host.record_copy_evidence(&impostor, &dispatch, CopyEvidenceV1::NotApplied, evidence())
            .is_err()
    );
    error(
        host.record_copy_evidence(
            &provider,
            &dispatch,
            CopyEvidenceV1::NotApplied,
            CopyEvidenceFactsV1::default(),
        ),
        CopyErrorV1::Denied,
    );
    assert_eq!(
        host.copy_status(&task).unwrap().effect,
        CopyEffectV1::Unknown
    );
    host.record_copy_evidence(&provider, &dispatch, CopyEvidenceV1::Quiescent, evidence())
        .unwrap();
    host.stop_copy(&task).unwrap();
    host.revoke_copy_provider(&provider).unwrap();
    error(
        host.retire_copy(&task, retirement()),
        CopyErrorV1::Outstanding,
    );
    assert!(
        host.release_copy_plain(&provider, &dispatch, release())
            .is_err()
    );
    let mut host = match host.into_parts() {
        Err(host) => host,
        Ok(_) => panic!("lost unknown work"),
    };
    // Authentic late evidence remains admissible despite stop/revocation.
    host.record_copy_evidence(&provider, &dispatch, CopyEvidenceV1::Accepted, evidence())
        .unwrap();
    assert_eq!(
        host.copy_status(&task).unwrap().effect,
        CopyEffectV1::Accepted
    );
    assert_eq!(
        host.record_copy_evidence(&provider, &dispatch, CopyEvidenceV1::NotApplied, evidence())
            .unwrap(),
        CopyEvidenceDispositionV1::Contested
    );
    assert_eq!(
        host.record_copy_evidence(&provider, &dispatch, CopyEvidenceV1::Accepted, evidence())
            .unwrap(),
        CopyEvidenceDispositionV1::Duplicate
    );
    assert_eq!(
        host.copy_status(&task).unwrap().effect,
        CopyEffectV1::Contested
    );
    error(
        host.retire_copy(&task, retirement()),
        CopyErrorV1::Outstanding,
    );
    let usage = host.copy_usage().unwrap();
    assert_eq!(
        (
            usage.unknown_tasks,
            usage.contested_tasks,
            usage.provider_work,
            usage.conflicts
        ),
        (0, 1, 0, 1)
    );
    assert_eq!(host.read(&document).unwrap().version, context.source);
    assert_eq!(host.history_usage().retained_groups, 0);
    assert!(host.into_parts().is_err()); // Deliberately unresolved; Drop is not completion.
}

#[test]
fn empty_copy_never_dispatches_and_queued_stop_releases_capacity_without_reusing_origin() {
    let f = Fixture::new();
    let mut bounds = limits();
    bounds.max_tasks = 1;
    bounds.max_provider_registrations = 1;
    let mut host = host(0, false, bounds);
    let provider = host.register_copy_provider(registration()).unwrap();
    let (_, context) = context(&mut host, &f, &[range(0, 0), range(2, 2)]);
    let id = host.issue_invocation().unwrap();
    let task = host
        .admit_copy(&id, &context, &provider, admission())
        .unwrap();
    let status = host.copy_status(&task).unwrap();
    assert!(status.no_export && status.quiescent);
    assert_eq!(status.effect, CopyEffectV1::NotDispatched);
    error(
        host.dispatch_copy(&task, launch()),
        CopyErrorV1::Unavailable,
    );
    let next = host.issue_invocation().unwrap();
    error(
        host.admit_copy(&next, &context, &provider, admission()),
        CopyErrorV1::Limit,
    );
    assert_eq!(
        host.invocation_state(&next).unwrap(),
        InvocationState::Issued
    );
    host.retire_copy(&task, retirement()).unwrap();
    let changed = host
        .set_logical_selections(&context, &[range(0, 2)], 0)
        .unwrap();
    let context = LogicalViewContext {
        stamp: changed.stamp,
        ..context
    };
    let task = host
        .admit_copy(&next, &context, &provider, admission())
        .unwrap();
    error(
        host.retire_copy(&task, retirement()),
        CopyErrorV1::Outstanding,
    );
    host.stop_copy(&task).unwrap();
    host.stop_copy(&task).unwrap();
    assert_eq!(host.copy_usage().unwrap().stopped, 1);
    error(
        host.dispatch_copy(&task, launch()),
        CopyErrorV1::Unavailable,
    );
    host.retire_copy(&task, retirement()).unwrap();
    host.retire_copy_provider(&provider, retirement()).unwrap();
    error(
        host.register_copy_provider(registration()),
        CopyErrorV1::Limit,
    );
    assert_eq!(host.copy_usage().unwrap().provider_registrations_issued, 1);
    assert!(host.into_parts().is_ok());
}

#[test]
fn exact_replay_keeps_original_payload_after_source_changes_and_cannot_retarget_or_substitute_provider()
 {
    let f = Fixture::new();
    let mut host = host(256, true, limits());
    let provider = host.register_copy_provider(registration()).unwrap();
    let other = host.register_copy_provider(registration()).unwrap();
    let (document, context) = context(&mut host, &f, &[range(0, 2)]);
    let id = host.issue_invocation().unwrap();
    let task = host
        .admit_copy(&id, &context, &provider, admission())
        .unwrap();
    host.edit(
        &context.source,
        vec![Replacement {
            start: 0,
            end: 2,
            expected: "ab".into(),
            inserted: "new".into(),
        }],
    )
    .unwrap();
    let current = host.logical_view(&context.view).unwrap().context().unwrap();
    assert_ne!(current, context);
    let retry = host
        .admit_copy(&id, &context, &provider, admission())
        .unwrap();
    error(
        host.admit_copy(&id, &current, &provider, admission()),
        CopyErrorV1::IntentMismatch,
    );
    error(
        host.admit_copy(&id, &context, &other, admission()),
        CopyErrorV1::IntentMismatch,
    );
    let stale_id = host.issue_invocation().unwrap();
    assert!(
        host.admit_copy(&stale_id, &context, &provider, admission())
            .is_err()
    );
    assert_eq!(
        host.invocation_state(&stale_id).unwrap(),
        InvocationState::Issued
    );
    host.retire_invocation(&stale_id).unwrap();
    let dispatch = host.dispatch_copy(&retry, launch()).unwrap();
    assert_eq!(
        host.release_copy_plain(&provider, &dispatch, release())
            .unwrap(),
        "ab"
    );
    assert_eq!(
        host.read(&document).unwrap().source.text(),
        "new\r\nGrüße\nend"
    );
    host.record_copy_evidence(&provider, &dispatch, CopyEvidenceV1::NotApplied, evidence())
        .unwrap();
    host.record_copy_evidence(&provider, &dispatch, CopyEvidenceV1::Quiescent, evidence())
        .unwrap();
    host.retire_copy(&task, retirement()).unwrap();
    host.retire_copy_provider(&provider, retirement()).unwrap();
    host.retire_copy_provider(&other, retirement()).unwrap();
    assert_eq!(host.invocation_usage().issued_lifetime, 2);
}

#[test]
fn logical_budget_refusal_is_exact_and_does_not_consume_invocation_or_claim() {
    let f = Fixture::new();
    let mut probe = host(0, false, limits());
    let p = probe.register_copy_provider(registration()).unwrap();
    let (_, c) = context(&mut probe, &f, &[range(0, 2)]);
    let id = probe.issue_invocation().unwrap();
    let t = probe.admit_copy(&id, &c, &p, admission()).unwrap();
    let exact = probe.copy_usage().unwrap().held_bytes;
    probe.stop_copy(&t).unwrap();
    probe.retire_copy(&t, retirement()).unwrap();
    probe.retire_copy_provider(&p, retirement()).unwrap();
    for (budget, succeeds) in [(exact - 1, false), (exact, true)] {
        let mut bounds = limits();
        bounds.max_held_bytes = budget;
        let mut host = host(0, false, bounds);
        let p = host.register_copy_provider(registration()).unwrap();
        let before = host.copy_usage().unwrap().held_bytes;
        let (_, c) = context(&mut host, &f, &[range(0, 2)]);
        let id = host.issue_invocation().unwrap();
        let result = host.admit_copy(&id, &c, &p, admission());
        if succeeds {
            let task = result.unwrap();
            assert_eq!(host.copy_usage().unwrap().held_bytes, exact);
            host.stop_copy(&task).unwrap();
            host.retire_copy(&task, retirement()).unwrap();
        } else {
            error(result, CopyErrorV1::Limit);
            assert_eq!(host.invocation_claim_usage().retained, 0);
            assert_eq!(host.invocation_state(&id).unwrap(), InvocationState::Issued);
            host.retire_invocation(&id).unwrap();
        }
        assert_eq!(host.copy_usage().unwrap().held_bytes, before);
        host.retire_copy_provider(&p, retirement()).unwrap();
        assert!(host.copy_usage().unwrap().held_bytes > 0); // Lifetime tombstone header remains charged.
        assert!(host.into_parts().is_ok());
    }
}

#[test]
fn unknown_or_denied_rights_and_foreign_handles_do_not_authorize_effects_or_evidence() {
    let f = Fixture::new();
    let mut a = host(0, false, limits());
    let mut b = host(0, false, limits());
    let p = a.register_copy_provider(registration()).unwrap();
    let foreign = b.register_copy_provider(registration()).unwrap();
    let (_, c) = context(&mut a, &f, &[range(0, 2)]);
    let id = a.issue_invocation().unwrap();
    error(
        a.admit_copy(&id, &c, &p, CopyAdmissionFactsV1::default()),
        CopyErrorV1::Denied,
    );
    error(
        a.admit_copy(
            &id,
            &c,
            &p,
            CopyAdmissionFactsV1 {
                retention_allowed: Some(false),
                ..admission()
            },
        ),
        CopyErrorV1::Denied,
    );
    assert!(a.admit_copy(&id, &c, &foreign, admission()).is_err());
    assert!(b.admit_copy(&id, &c, &foreign, admission()).is_err());
    let task = a.admit_copy(&id, &c, &p, admission()).unwrap();
    error(
        a.dispatch_copy(&task, CopyDispatchFactsV1::default()),
        CopyErrorV1::Denied,
    );
    let dispatch = a.dispatch_copy(&task, launch()).unwrap();
    assert!(
        b.release_copy_plain(&foreign, &dispatch, release())
            .is_err()
    );
    assert!(
        b.record_copy_evidence(&foreign, &dispatch, CopyEvidenceV1::Accepted, evidence())
            .is_err()
    );
    assert!(b.copy_status(&task).is_err());
    assert!(b.stop_copy(&task).is_err());
    error(
        a.release_copy_plain(&p, &dispatch, CopyReleaseFactsV1::default()),
        CopyErrorV1::Denied,
    );
    error(
        a.record_copy_evidence(
            &p,
            &dispatch,
            CopyEvidenceV1::Accepted,
            CopyEvidenceFactsV1 {
                authentic: Some(false),
                ..evidence()
            },
        ),
        CopyErrorV1::Denied,
    );
    assert_eq!(a.copy_status(&task).unwrap().effect, CopyEffectV1::Unknown);
    a.record_copy_evidence(&p, &dispatch, CopyEvidenceV1::NotApplied, evidence())
        .unwrap();
    a.record_copy_evidence(&p, &dispatch, CopyEvidenceV1::Quiescent, evidence())
        .unwrap();
    error(
        a.retire_copy(&task, CopyRetirementFactsV1::default()),
        CopyErrorV1::Denied,
    );
    a.retire_copy(&task, retirement()).unwrap();
    a.retire_copy_provider(&p, retirement()).unwrap();
    b.retire_copy_provider(&foreign, retirement()).unwrap();
    assert!(a.copy_usage().unwrap().refusals >= 7);
}

#[test]
fn malformed_read_only_revoked_and_over_budget_profiles_refuse_without_claiming_source() {
    let f = Fixture::new();
    let mut bounds = limits();
    bounds.capture.max_plain_bytes = 1;
    let mut host = host(0, false, bounds);
    assert!(host.configure_copy_transfers(bounds).is_err());
    let mut bad = registration();
    bad.acknowledgment_profile.clear();
    error(host.register_copy_provider(bad), CopyErrorV1::Limit);
    let mut bad = registration();
    bad.acknowledgment_profile = "X".repeat(65);
    error(host.register_copy_provider(bad), CopyErrorV1::Limit);
    assert_eq!(host.copy_usage().unwrap().provider_registrations_issued, 0);
    let mut read_only = registration();
    read_only.writable = false;
    let read_only = host.register_copy_provider(read_only).unwrap();
    let provider = host.register_copy_provider(registration()).unwrap();
    let (document, c) = context(&mut host, &f, &[range(0, 2)]);
    let id = host.issue_invocation().unwrap();
    error(
        host.admit_copy(&id, &c, &read_only, admission()),
        CopyErrorV1::Unavailable,
    );
    assert!(matches!(
        host.admit_copy(&id, &c, &provider, admission()),
        Err(SessionError::TextTransfer(_))
    ));
    assert_eq!(host.invocation_claim_usage().retained, 0);
    assert_eq!(host.invocation_state(&id).unwrap(), InvocationState::Issued);
    assert_eq!(host.copy_usage().unwrap().held_tasks, 0);
    let next = host.set_logical_selections(&c, &[range(0, 1)], 0).unwrap();
    let c = LogicalViewContext {
        stamp: next.stamp,
        ..c
    };
    let task = host.admit_copy(&id, &c, &provider, admission()).unwrap();
    host.revoke_copy_provider(&provider).unwrap();
    error(
        host.dispatch_copy(&task, launch()),
        CopyErrorV1::Unavailable,
    );
    // Exact admission replay is retrieval, not new dispatch authority.
    assert!(host.admit_copy(&id, &c, &provider, admission()).is_ok());
    host.stop_copy(&task).unwrap();
    host.retire_copy(&task, retirement()).unwrap();
    assert_eq!(host.read(&document).unwrap().version, c.source);
    assert_eq!(host.history_usage().retained_groups, 0);
    host.retire_copy_provider(&provider, retirement()).unwrap();
    host.retire_copy_provider(&read_only, retirement()).unwrap();
    assert!(host.into_parts().is_ok());
}
