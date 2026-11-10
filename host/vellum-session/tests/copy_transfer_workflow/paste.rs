use super::*;
#[path = "paste_limits.rs"]
mod limits;
use vellum_engine::{
    attachment::{AttachmentTarget, Placement, PointProfile},
    coordinator::{
        AnnotationLimits, AnnotationMutation, HistoryDirection, InvocationOutcome, SourceTarget,
        TargetBasis,
    },
    resource::{Acknowledgment, Application, Protection, SaveRequirements},
};
fn paste_limits() -> PasteLimitsV1 {
    PasteLimitsV1 {
        max_targets: 8,
        max_plain_bytes: 256,
        max_expanded_bytes: 8192,
    }
}
fn paste_host(capacity: usize, timed: bool) -> LocalHost {
    let mut host = host(capacity, timed, limits());
    host.configure_paste_transfers(paste_limits()).unwrap();
    host
}
fn admit() -> PasteAdmissionFactsV1 {
    PasteAdmissionFactsV1 {
        target_read_allowed: Some(true),
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
        retention_allowed: Some(true),
    }
}
fn dispatch_facts() -> PasteDispatchFactsV1 {
    PasteDispatchFactsV1 {
        launch_allowed: Some(true),
        clipboard_read_allowed: Some(true),
    }
}
fn receipt_facts() -> PasteEvidenceFactsV1 {
    PasteEvidenceFactsV1 {
        evidence: evidence(),
        payload_retention_allowed: Some(true),
    }
}
fn publish() -> PastePublicationFactsV1 {
    PastePublicationFactsV1 {
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
        payload_use_allowed: Some(true),
    }
}
fn receive(
    host: &mut LocalHost,
    p: &CopyProviderPortV1,
    task: &CopyTaskV1,
    text: &str,
) -> CopyDispatchV1 {
    let d = host.dispatch_paste(task, dispatch_facts()).unwrap();
    host.record_paste_evidence(p, &d, PasteEvidenceV1::Accepted(text), receipt_facts())
        .unwrap();
    d
}
fn quiet(host: &mut LocalHost, p: &CopyProviderPortV1, d: &CopyDispatchV1) {
    host.record_paste_evidence(p, d, PasteEvidenceV1::Quiescent, receipt_facts())
        .unwrap();
}

#[test]
fn received_plain_text_broadcasts_literally_with_native_companions_history_save_and_reopen() {
    for (capacity, timed) in [(0, false), (0, true), (1, true), (256, true)] {
        let f = Fixture::new();
        let mut cfg = config(capacity, timed);
        cfg.engine.annotations = Some(AnnotationLimits {
            max_issued: 8,
            max_records: 8,
            max_body_bytes: 1024,
            max_retained_body_bytes: 8192,
            max_mapping_steps: 8192,
            max_live_snapshots: 4,
            max_snapshot_bytes: 65536,
        });
        let mut host = LocalHost::new(cfg).unwrap();
        host.configure_copy_transfers(limits()).unwrap();
        host.configure_paste_transfers(paste_limits()).unwrap();
        let p = host.register_copy_provider(registration()).unwrap();
        let (doc, c) = context(&mut host, &f, &[range(0, 2), range(4, 11)]);
        let annotation = host
            .commit_compound(
                &[],
                &[],
                &[AnnotationMutation::Create {
                    target: SourceTarget {
                        source: c.source.clone(),
                        basis: TargetBasis::Before,
                        target: AttachmentTarget::Point {
                            offset: 14,
                            affinity: Affinity::After,
                            profile: PointProfile::Content,
                        },
                    },
                    body: "inert".into(),
                }],
            )
            .unwrap()
            .created[0]
            .clone();
        let c = host.logical_view(&c.view).unwrap().context().unwrap();
        let id = host.issue_invocation().unwrap();
        let task = host.admit_paste(&id, &c, &p, admit()).unwrap();
        let retry = host.admit_paste(&id, &c, &p, admit()).unwrap();
        assert_eq!(retry.origin(), task.origin());
        // A peer attached after admission participates at actual publication.
        let peer = host.create_logical_view().unwrap();
        let stamp = host.logical_view(&peer).unwrap().stamp;
        host.attach_logical_view(&peer, stamp, &c.source, &[range(14, 14)], 0)
            .unwrap();
        error(
            host.continue_paste(&task, publish()),
            CopyErrorV1::Outstanding,
        );
        let d = host.dispatch_paste(&task, dispatch_facts()).unwrap();
        error(
            host.dispatch_paste(&task, dispatch_facts()),
            CopyErrorV1::AlreadyDispatched,
        );
        quiet(&mut host, &p, &d);
        error(
            host.continue_paste(&task, publish()),
            CopyErrorV1::Outstanding,
        ); // no acknowledgment yet
        let payload = "ß\r\n:quit\n\t\u{1b}";
        host.record_paste_evidence(&p, &d, PasteEvidenceV1::Accepted(payload), receipt_facts())
            .unwrap();
        assert_eq!(host.read(&doc).unwrap().version, c.source);
        assert_eq!(
            host.continue_paste(&task, publish()).unwrap(),
            TransferLocalDispositionV1::Applied
        );
        let expected = format!("{payload}\r\n{payload}\nend");
        assert_eq!(host.read(&doc).unwrap().source.text(), expected);
        assert_eq!(
            host.logical_view(&peer).unwrap().selections.selections(),
            &[range(2 * payload.len() + 5, 2 * payload.len() + 5)]
        );
        assert!(
            matches!(host.annotation(&annotation).unwrap().placement, Placement::Located { target: AttachmentTarget::Point { offset, .. }, .. } if offset == 2 * payload.len() + 5)
        );
        let applied = host.read(&doc).unwrap().version;
        assert_eq!(
            host.continue_paste(&retry, publish()).unwrap(),
            TransferLocalDispositionV1::Applied
        );
        assert_eq!(host.read(&doc).unwrap().version, applied);
        assert_eq!(
            host.record_paste_evidence(&p, &d, PasteEvidenceV1::Accepted(payload), receipt_facts())
                .unwrap(),
            CopyEvidenceDispositionV1::Duplicate
        );
        assert!(host.reverse(&doc, HistoryDirection::Undo).unwrap());
        assert_eq!(host.read(&doc).unwrap().source.text(), "ab\r\nGrüße\nend");
        let undone = host.read(&doc).unwrap().version;
        assert_eq!(
            host.continue_paste(&task, publish()).unwrap(),
            TransferLocalDispositionV1::Applied
        );
        assert_eq!(host.read(&doc).unwrap().version, undone);
        assert!(host.reverse(&doc, HistoryDirection::Redo).unwrap());
        let version = host.read(&doc).unwrap().version;
        let save = host
            .queue_save(
                &version,
                SaveRequirements {
                    protection: Protection::BestEffort,
                    acknowledgment: Acknowledgment::Synchronized,
                },
            )
            .unwrap();
        let report = host.service_save(&doc, &save).unwrap();
        assert!(report.failure.is_none());
        assert_eq!(report.outcome.application, Application::Written);
        host.publish_save_baseline(&save).unwrap();
        assert_eq!(
            fs::read(f.0.join("source")).unwrap(),
            format!("\u{feff}{expected}").as_bytes()
        );
        let mut fresh = paste_host(0, false);
        let reopened = fresh.open(&f.0.join("source")).unwrap();
        assert_eq!(fresh.read(&reopened).unwrap().source.text(), expected);
        let status = host.paste_report(&task).unwrap();
        assert_eq!(status.local, TransferLocalDispositionV1::Applied);
        assert_eq!(status.external.first_result_after.is_some(), timed);
        assert_eq!(status.external.quiescent_after.is_some(), timed);
        assert_eq!(host.copy_usage().unwrap().paste_applied, 1);
        assert_eq!(host.copy_usage().unwrap().dispatched, 1);
        assert_eq!(
            host.copy_usage()
                .unwrap()
                .operation(CopyOperationV1::AdmitPaste),
            CopyOperationUsageV1 {
                offers: 2,
                refusals: 0
            }
        );
        host.retire_copy(&task, retirement()).unwrap();
        host.retire_copy_provider(&p, retirement()).unwrap();
        assert_eq!(host.invocation_usage().issued_lifetime, 1);
        assert_eq!(host.invocation_claim_usage().retained, 0);
    }
}

#[test]
fn source_selection_and_attachment_changes_cannot_redirect_dispatch_or_late_paste() {
    for before_dispatch in [true, false] {
        for mode in ["source", "selection", "detach"] {
            let f = Fixture::new();
            let mut host = paste_host(0, false);
            let p = host.register_copy_provider(registration()).unwrap();
            let (doc, c) = context(&mut host, &f, &[range(0, 2)]);
            let id = host.issue_invocation().unwrap();
            let task = host.admit_paste(&id, &c, &p, admit()).unwrap();
            let d = if before_dispatch {
                None
            } else {
                Some(host.dispatch_paste(&task, dispatch_facts()).unwrap())
            };
            match mode {
                "source" => {
                    host.edit(
                        &c.source,
                        vec![Replacement {
                            start: 0,
                            end: 2,
                            expected: "ab".into(),
                            inserted: "XY".into(),
                        }],
                    )
                    .unwrap();
                }
                "selection" => {
                    host.set_logical_selections(&c, &[range(4, 11)], 0).unwrap();
                }
                _ => {
                    host.detach_logical_view(&c.view, c.stamp).unwrap();
                }
            }
            assert!(host.admit_paste(&id, &c, &p, admit()).is_ok()); // original retrieval, not recapture
            let version = host.read(&doc).unwrap().version;
            if let Some(d) = d {
                host.record_paste_evidence(&p, &d, PasteEvidenceV1::Accepted("Z"), receipt_facts())
                    .unwrap();
                assert_eq!(
                    host.continue_paste(&task, publish()).unwrap(),
                    TransferLocalDispositionV1::NotApplied
                );
                quiet(&mut host, &p, &d);
            } else {
                assert!(host.dispatch_paste(&task, dispatch_facts()).is_err());
                host.stop_copy(&task).unwrap();
            }
            assert_eq!(host.read(&doc).unwrap().version, version);
            assert!(host.read(&doc).unwrap().source.text().contains("Grüße"));
            host.retire_copy(&task, retirement()).unwrap();
            host.retire_copy_provider(&p, retirement()).unwrap();
        }
    }
}

#[test]
fn empty_paste_is_an_acknowledged_literal_not_a_missing_payload_or_copy_no_export_rule() {
    for selected in [false, true] {
        let f = Fixture::new();
        let mut host = paste_host(0, false);
        let p = host.register_copy_provider(registration()).unwrap();
        let (doc, c) = context(
            &mut host,
            &f,
            &[if selected { range(0, 2) } else { range(0, 0) }],
        );
        let id = host.issue_invocation().unwrap();
        let task = host.admit_paste(&id, &c, &p, admit()).unwrap();
        assert!(!host.copy_status(&task).unwrap().no_export);
        let d = receive(&mut host, &p, &task, "");
        let expected = if selected {
            TransferLocalDispositionV1::Applied
        } else {
            TransferLocalDispositionV1::Unchanged
        };
        assert_eq!(host.continue_paste(&task, publish()).unwrap(), expected);
        assert_eq!(
            host.read(&doc).unwrap().source.text(),
            if selected {
                "\r\nGrüße\nend"
            } else {
                "ab\r\nGrüße\nend"
            }
        );
        assert_eq!(host.continue_paste(&task, publish()).unwrap(), expected);
        assert_eq!(
            host.copy_usage().unwrap().paste_unchanged,
            u64::from(!selected)
        );
        quiet(&mut host, &p, &d);
        host.retire_copy(&task, retirement()).unwrap();
        host.retire_copy_provider(&p, retirement()).unwrap();
    }
}

#[test]
fn invalid_receipts_cannot_taint_canonical_data_and_conflicting_payloads_are_bounded() {
    let f = Fixture::new();
    let mut host = paste_host(1, true);
    let p = host.register_copy_provider(registration()).unwrap();
    let other = host.register_copy_provider(registration()).unwrap();
    let (doc, c) = context(&mut host, &f, &[range(0, 2)]);
    let id = host.issue_invocation().unwrap();
    let task = host.admit_paste(&id, &c, &p, admit()).unwrap();
    let d = host.dispatch_paste(&task, dispatch_facts()).unwrap();
    let reserved = host.copy_usage().unwrap().held_bytes;
    assert!(
        host.record_paste_evidence(
            &other,
            &d,
            PasteEvidenceV1::Accepted("forged"),
            receipt_facts()
        )
        .is_err()
    );
    error(
        host.record_paste_evidence(
            &p,
            &d,
            PasteEvidenceV1::Accepted("forged"),
            PasteEvidenceFactsV1::default(),
        ),
        CopyErrorV1::Denied,
    );
    error(
        host.record_paste_evidence(
            &p,
            &d,
            PasteEvidenceV1::Accepted("forged"),
            PasteEvidenceFactsV1 {
                payload_retention_allowed: None,
                ..receipt_facts()
            },
        ),
        CopyErrorV1::Denied,
    );
    let oversized = "x".repeat(257);
    error(
        host.record_paste_evidence(
            &p,
            &d,
            PasteEvidenceV1::Accepted(&oversized),
            receipt_facts(),
        ),
        CopyErrorV1::Limit,
    );
    assert_eq!(
        host.copy_status(&task).unwrap().effect,
        CopyEffectV1::Unknown
    );
    host.record_paste_evidence(&p, &d, PasteEvidenceV1::Accepted("A"), receipt_facts())
        .unwrap();
    assert_eq!(
        host.record_paste_evidence(&p, &d, PasteEvidenceV1::Accepted("B"), receipt_facts())
            .unwrap(),
        CopyEvidenceDispositionV1::Contested
    );
    assert_eq!(
        host.record_paste_evidence(&p, &d, PasteEvidenceV1::Accepted("B"), receipt_facts())
            .unwrap(),
        CopyEvidenceDispositionV1::Duplicate
    );
    assert_eq!(
        host.record_paste_evidence(&p, &d, PasteEvidenceV1::Accepted("C"), receipt_facts())
            .unwrap(),
        CopyEvidenceDispositionV1::Contested
    );
    quiet(&mut host, &p, &d);
    error(
        host.continue_paste(&task, publish()),
        CopyErrorV1::Outstanding,
    );
    assert_eq!(host.read(&doc).unwrap().version, c.source);
    assert_eq!(host.copy_usage().unwrap().held_bytes, reserved);
    assert_eq!(host.copy_usage().unwrap().conflicts, 1);
    host.stop_copy(&task).unwrap();
    error(
        host.retire_copy(&task, retirement()),
        CopyErrorV1::Outstanding,
    );
    assert!(host.into_parts().is_err());
}

#[test]
fn current_permission_revocation_stop_and_not_applied_results_do_not_become_retries() {
    for mode in ["denied", "missing", "locked", "provider", "stopped"] {
        let f = Fixture::new();
        let mut host = paste_host(0, false);
        let p = host.register_copy_provider(registration()).unwrap();
        let (doc, c) = context(&mut host, &f, &[range(0, 2)]);
        let id = host.issue_invocation().unwrap();
        let task = host.admit_paste(&id, &c, &p, admit()).unwrap();
        let d = receive(&mut host, &p, &task, "changed");
        let facts = match mode {
            "denied" => PastePublicationFactsV1 {
                payload_use_allowed: Some(false),
                ..publish()
            },
            "missing" => PastePublicationFactsV1::default(),
            "locked" => {
                host.set_editing_locked(&doc, true).unwrap();
                publish()
            }
            "provider" => {
                host.revoke_copy_provider(&p).unwrap();
                publish()
            }
            _ => {
                host.stop_copy(&task).unwrap();
                publish()
            }
        };
        let expected = if mode == "stopped" {
            TransferLocalDispositionV1::Stopped
        } else {
            TransferLocalDispositionV1::NotApplied
        };
        assert_eq!(host.continue_paste(&task, facts).unwrap(), expected);
        host.set_editing_locked(&doc, false).unwrap();
        assert_eq!(host.continue_paste(&task, publish()).unwrap(), expected);
        assert_eq!(host.read(&doc).unwrap().source.text(), "ab\r\nGrüße\nend");
        assert_eq!(host.history_usage().retained_groups, 0);
        quiet(&mut host, &p, &d);
        host.retire_copy(&task, retirement()).unwrap();
        host.retire_copy_provider(&p, retirement()).unwrap();
    }
    let f = Fixture::new();
    let mut host = paste_host(0, false);
    let p = host.register_copy_provider(registration()).unwrap();
    let (_, c) = context(&mut host, &f, &[range(0, 2)]);
    let id = host.issue_invocation().unwrap();
    let task = host.admit_paste(&id, &c, &p, admit()).unwrap();
    let d = host.dispatch_paste(&task, dispatch_facts()).unwrap();
    quiet(&mut host, &p, &d);
    error(
        host.retire_copy(&task, retirement()),
        CopyErrorV1::Outstanding,
    );
    host.record_paste_evidence(&p, &d, PasteEvidenceV1::NotApplied, receipt_facts())
        .unwrap();
    error(
        host.continue_paste(&task, publish()),
        CopyErrorV1::Outstanding,
    );
    host.stop_copy(&task).unwrap();
    host.retire_copy(&task, retirement()).unwrap();
    host.retire_copy_provider(&p, retirement()).unwrap();
}

#[test]
fn late_conflicts_preserve_native_publication_and_wrong_operation_routes_cannot_bypass_payloads() {
    let f = Fixture::new();
    let mut host = paste_host(0, false);
    let p = host.register_copy_provider(registration()).unwrap();
    let (doc, c) = context(&mut host, &f, &[range(0, 2)]);
    let id = host.issue_invocation().unwrap();
    let task = host.admit_paste(&id, &c, &p, admit()).unwrap();
    error(
        host.dispatch_copy(&task, launch()),
        CopyErrorV1::Unavailable,
    );
    error(
        host.continue_cut(
            &task,
            CutPublicationFactsV1 {
                edit_allowed: Some(true),
                view_write_allowed: Some(true),
            },
        ),
        CopyErrorV1::Unavailable,
    );
    error(
        host.admit_copy(&id, &c, &p, admission()),
        CopyErrorV1::IntentMismatch,
    );
    let d = host.dispatch_paste(&task, dispatch_facts()).unwrap();
    error(
        host.release_copy_plain(&p, &d, release()),
        CopyErrorV1::Unavailable,
    );
    error(
        host.record_copy_evidence(&p, &d, CopyEvidenceV1::Accepted, evidence()),
        CopyErrorV1::Unavailable,
    );
    host.record_paste_evidence(&p, &d, PasteEvidenceV1::Accepted("A"), receipt_facts())
        .unwrap();
    assert_eq!(
        host.continue_paste(&task, publish()).unwrap(),
        TransferLocalDispositionV1::Applied
    );
    let applied = host.read(&doc).unwrap().version;
    host.revoke_copy_provider(&p).unwrap();
    host.record_paste_evidence(&p, &d, PasteEvidenceV1::Accepted("B"), receipt_facts())
        .unwrap();
    quiet(&mut host, &p, &d);
    let report = host.paste_report(&task).unwrap();
    assert_eq!(report.local, TransferLocalDispositionV1::Applied);
    assert_eq!(report.external.effect, CopyEffectV1::Contested);
    assert_eq!(host.read(&doc).unwrap().version, applied);
    error(
        host.retire_copy(&task, retirement()),
        CopyErrorV1::Outstanding,
    );
    assert!(matches!(
        host.invocation_outcome(&id).unwrap(),
        InvocationOutcome::Committed(_)
    ));
    assert!(host.into_parts().is_err());
}
