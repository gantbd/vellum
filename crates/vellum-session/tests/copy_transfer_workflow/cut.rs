// Included by copy_transfer_workflow; helpers use real temporary files.
use super::*;
use vellum_engine::{
    attachment::{AttachmentTarget, Placement, PointProfile},
    coordinator::{
        AnnotationLimits, AnnotationMutation, HistoryDirection, InvocationOutcome, SourceTarget,
        TargetBasis,
    },
    resource::{Acknowledgment, Application, Protection, SaveRequirements},
};
fn cut_admission() -> CutAdmissionFactsV1 {
    CutAdmissionFactsV1 {
        transfer: admission(),
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
    }
}
fn publication() -> CutPublicationFactsV1 {
    CutPublicationFactsV1 {
        edit_allowed: Some(true),
        view_write_allowed: Some(true),
    }
}
fn accepted(
    host: &mut LocalHost,
    provider: &CopyProviderPortV1,
    task: &CopyTaskV1,
) -> CopyDispatchV1 {
    let dispatch = host.dispatch_copy(task, launch()).unwrap();
    let _payload = host
        .release_copy_plain(provider, &dispatch, release())
        .unwrap();
    host.record_copy_evidence(provider, &dispatch, CopyEvidenceV1::Accepted, evidence())
        .unwrap();
    dispatch
}
fn annotation_host(capacity: usize) -> LocalHost {
    let mut config = config(capacity, true);
    config.engine.annotations = Some(AnnotationLimits {
        max_issued: 8,
        max_records: 8,
        max_body_bytes: 1024,
        max_retained_body_bytes: 8192,
        max_mapping_steps: 8192,
        max_live_snapshots: 4,
        max_snapshot_bytes: 65536,
    });
    let mut host = LocalHost::new(config).unwrap();
    host.configure_copy_transfers(limits()).unwrap();
    host
}

#[test]
fn accepted_cut_publishes_native_multi_selection_annotation_peer_views_history_and_lossless_save() {
    for capacity in [0, 1, 256] {
        let f = Fixture::new();
        let mut host = annotation_host(capacity);
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
                    body: "inert comment".into(),
                }],
            )
            .unwrap()
            .created[0]
            .clone();
        let c = host.logical_view(&c.view).unwrap().context().unwrap();
        let peer = host.create_logical_view().unwrap();
        let stamp = host.logical_view(&peer).unwrap().stamp;
        host.attach_logical_view(&peer, stamp, &c.source, &[range(14, 14)], 0)
            .unwrap();
        let id = host.issue_invocation().unwrap();
        let task = host.admit_cut(&id, &c, &p, cut_admission()).unwrap();
        assert_eq!(
            host.copy_status(&task).unwrap().operation,
            TextTransferKindV1::Cut
        );
        error(
            host.admit_copy(&id, &c, &p, admission()),
            CopyErrorV1::IntentMismatch,
        );
        error(
            host.continue_cut(&task, publication()),
            CopyErrorV1::Outstanding,
        );
        let dispatch = host.dispatch_copy(&task, launch()).unwrap();
        assert_eq!(
            host.release_copy_plain(&p, &dispatch, release()).unwrap(),
            "ab\nGrüße"
        );
        host.record_copy_evidence(&p, &dispatch, CopyEvidenceV1::Quiescent, evidence())
            .unwrap();
        error(
            host.continue_cut(&task, publication()),
            CopyErrorV1::Outstanding,
        );
        assert_eq!(host.read(&doc).unwrap().version, c.source);
        host.record_copy_evidence(&p, &dispatch, CopyEvidenceV1::Accepted, evidence())
            .unwrap();
        error(
            host.retire_copy(&task, retirement()),
            CopyErrorV1::Outstanding,
        );
        assert_eq!(
            host.continue_cut(&task, publication()).unwrap(),
            TransferLocalDispositionV1::Applied
        );
        assert_eq!(host.read(&doc).unwrap().source.text(), "\r\n\nend");
        assert_eq!(
            host.logical_view(&peer).unwrap().selections.selections(),
            &[range(5, 5)]
        );
        assert!(matches!(
            host.annotation(&annotation).unwrap().placement,
            Placement::Located {
                target: AttachmentTarget::Point { offset: 5, .. },
                ..
            }
        ));
        assert!(matches!(
            host.invocation_outcome(&id).unwrap(),
            InvocationOutcome::Committed(_)
        ));
        let committed = host.read(&doc).unwrap().version;
        assert_eq!(
            host.continue_cut(&task, publication()).unwrap(),
            TransferLocalDispositionV1::Applied
        );
        assert_eq!(host.read(&doc).unwrap().version, committed);
        assert!(host.reverse(&doc, HistoryDirection::Undo).unwrap());
        assert_eq!(host.read(&doc).unwrap().source.text(), "ab\r\nGrüße\nend");
        assert!(matches!(
            host.annotation(&annotation).unwrap().placement,
            Placement::Located {
                target: AttachmentTarget::Point { offset: 14, .. },
                ..
            }
        ));
        let undone = host.read(&doc).unwrap().version;
        assert_eq!(
            host.continue_cut(&task, publication()).unwrap(),
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
            "\u{feff}\r\n\nend".as_bytes()
        );
        let mut reopened = super::host(0, false, limits());
        let d = reopened.open(&f.0.join("source")).unwrap();
        assert_eq!(reopened.read(&d).unwrap().source.text(), "\r\n\nend");
        assert_eq!(host.copy_usage().unwrap().cut_applied, 1);
        assert_eq!(host.copy_usage().unwrap().dispatched, 1);
        host.retire_copy(&task, retirement()).unwrap();
        host.retire_copy_provider(&p, retirement()).unwrap();
        assert_eq!(host.invocation_usage().issued_lifetime, 1);
    }
}

#[test]
fn acknowledged_cut_current_denial_and_policy_refusal_are_terminal_not_implicit_retries() {
    for mode in ["missing-right", "denied-right", "locked", "revoked"] {
        let f = Fixture::new();
        let mut host = host(1, true, limits());
        let p = host.register_copy_provider(registration()).unwrap();
        let (doc, c) = context(&mut host, &f, &[range(0, 2)]);
        let id = host.issue_invocation().unwrap();
        let task = host.admit_cut(&id, &c, &p, cut_admission()).unwrap();
        let dispatch = accepted(&mut host, &p, &task);
        let facts = match mode {
            "missing-right" => CutPublicationFactsV1::default(),
            "denied-right" => CutPublicationFactsV1 {
                edit_allowed: Some(false),
                ..publication()
            },
            "locked" => {
                host.set_editing_locked(&doc, true).unwrap();
                publication()
            }
            _ => {
                host.revoke_copy_provider(&p).unwrap();
                publication()
            }
        };
        assert_eq!(
            host.continue_cut(&task, facts).unwrap(),
            TransferLocalDispositionV1::NotApplied,
            "{mode}"
        );
        assert!(matches!(
            host.invocation_outcome(&id).unwrap(),
            InvocationOutcome::Rejected(_)
        ));
        host.set_editing_locked(&doc, false).unwrap();
        assert_eq!(
            host.continue_cut(&task, publication()).unwrap(),
            TransferLocalDispositionV1::NotApplied
        );
        assert_eq!(host.read(&doc).unwrap().source.text(), "ab\r\nGrüße\nend");
        assert_eq!(host.history_usage().retained_groups, 0);
        assert_eq!(
            host.copy_status(&task).unwrap().effect,
            CopyEffectV1::Accepted
        ); // copied, not cut
        assert!(host.cut_report(&task).unwrap().copied_not_cut);
        assert_eq!(host.copy_usage().unwrap().cut_not_applied, 1);
        host.record_copy_evidence(&p, &dispatch, CopyEvidenceV1::Quiescent, evidence())
            .unwrap();
        host.retire_copy(&task, retirement()).unwrap();
        host.retire_copy_provider(&p, retirement()).unwrap();
    }
}

#[test]
fn original_source_selection_or_view_changes_refuse_cut_without_focus_redirection() {
    for mode in ["source", "selection", "detached"] {
        let f = Fixture::new();
        let mut host = host(0, false, limits());
        let p = host.register_copy_provider(registration()).unwrap();
        let (doc, c) = context(&mut host, &f, &[range(0, 2)]);
        let id = host.issue_invocation().unwrap();
        let task = host.admit_cut(&id, &c, &p, cut_admission()).unwrap();
        let dispatch = accepted(&mut host, &p, &task);
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
        let current = host.read(&doc).unwrap().version;
        assert_eq!(
            host.continue_cut(&task, publication()).unwrap(),
            TransferLocalDispositionV1::NotApplied
        );
        assert_eq!(host.read(&doc).unwrap().version, current);
        assert!(host.read(&doc).unwrap().source.text().contains("Grüße"));
        host.record_copy_evidence(&p, &dispatch, CopyEvidenceV1::Quiescent, evidence())
            .unwrap();
        host.retire_copy(&task, retirement()).unwrap();
        host.retire_copy_provider(&p, retirement()).unwrap();
    }
}

#[test]
fn stopped_empty_unacknowledged_and_contested_cuts_cannot_delete_or_hide_obligations() {
    let f = Fixture::new();
    let mut host = host(0, false, limits());
    let p = host.register_copy_provider(registration()).unwrap();
    let (doc, c) = context(&mut host, &f, &[range(0, 2)]);
    let id = host.issue_invocation().unwrap();
    error(
        host.admit_cut(&id, &c, &p, CutAdmissionFactsV1::default()),
        CopyErrorV1::Denied,
    );
    let task = host.admit_cut(&id, &c, &p, cut_admission()).unwrap();
    let dispatch = host.dispatch_copy(&task, launch()).unwrap();
    host.stop_copy(&task).unwrap();
    assert_eq!(
        host.continue_cut(&task, publication()).unwrap(),
        TransferLocalDispositionV1::Stopped
    );
    error(
        host.retire_copy(&task, retirement()),
        CopyErrorV1::Outstanding,
    ); // unknown export
    host.record_copy_evidence(&p, &dispatch, CopyEvidenceV1::Accepted, evidence())
        .unwrap();
    assert_eq!(
        host.continue_cut(&task, publication()).unwrap(),
        TransferLocalDispositionV1::Stopped
    );
    host.record_copy_evidence(&p, &dispatch, CopyEvidenceV1::Quiescent, evidence())
        .unwrap();
    host.retire_copy(&task, retirement()).unwrap();
    let id = host.issue_invocation().unwrap();
    let task = host.admit_cut(&id, &c, &p, cut_admission()).unwrap();
    let dispatch = accepted(&mut host, &p, &task);
    assert_eq!(
        host.continue_cut(&task, publication()).unwrap(),
        TransferLocalDispositionV1::Applied
    );
    let version = host.read(&doc).unwrap().version;
    host.record_copy_evidence(&p, &dispatch, CopyEvidenceV1::NotApplied, evidence())
        .unwrap();
    assert_eq!(
        host.cut_disposition(&task).unwrap(),
        TransferLocalDispositionV1::Applied
    ); // historical
    assert!(!host.cut_report(&task).unwrap().copied_not_cut);
    assert_eq!(host.read(&doc).unwrap().version, version);
    host.stop_copy(&task).unwrap();
    host.record_copy_evidence(&p, &dispatch, CopyEvidenceV1::Quiescent, evidence())
        .unwrap();
    error(
        host.retire_copy(&task, retirement()),
        CopyErrorV1::Outstanding,
    ); // contested
    assert_eq!(host.invocation_claim_usage().retained, 1);
    assert!(host.into_parts().is_err());
}

#[test]
fn empty_cut_is_no_export_no_source_job_and_copy_is_not_a_cut_capability() {
    let f = Fixture::new();
    let mut host = host(0, false, limits());
    let p = host.register_copy_provider(registration()).unwrap();
    let (_, c) = context(&mut host, &f, &[range(0, 0), range(2, 2)]);
    let id = host.issue_invocation().unwrap();
    let task = host.admit_cut(&id, &c, &p, cut_admission()).unwrap();
    assert_eq!(
        host.continue_cut(&task, publication()).unwrap(),
        TransferLocalDispositionV1::Unchanged
    );
    assert_eq!(host.invocation_state(&id).unwrap(), InvocationState::Issued);
    assert!(host.invocation_outcome(&id).is_err());
    error(
        host.dispatch_copy(&task, launch()),
        CopyErrorV1::Unavailable,
    );
    host.retire_copy(&task, retirement()).unwrap();
    let id = host.issue_invocation().unwrap();
    let task = host.admit_copy(&id, &c, &p, admission()).unwrap();
    error(
        host.continue_cut(&task, publication()),
        CopyErrorV1::Unavailable,
    );
    host.retire_copy(&task, retirement()).unwrap();
    host.retire_copy_provider(&p, retirement()).unwrap();
}

#[test]
fn descriptor_and_expanded_edit_limits_are_real_refusals_without_partial_deletion() {
    let f = Fixture::new();
    let mut cfg = config(0, false);
    cfg.engine.invocation.as_mut().unwrap().max_intent_bytes = 0;
    let mut host = LocalHost::new(cfg).unwrap();
    host.configure_copy_transfers(limits()).unwrap();
    let p = host.register_copy_provider(registration()).unwrap();
    let (_, c) = context(&mut host, &f, &[range(0, 2)]);
    let id = host.issue_invocation().unwrap();
    error(
        host.admit_cut(&id, &c, &p, cut_admission()),
        CopyErrorV1::Limit,
    );
    assert_eq!(host.invocation_state(&id).unwrap(), InvocationState::Issued);
    assert_eq!(host.invocation_claim_usage().retained, 0);
    // Source-neutral copy does not require a literal source descriptor.
    let copy = host.admit_copy(&id, &c, &p, admission()).unwrap();
    host.stop_copy(&copy).unwrap();
    host.retire_copy(&copy, retirement()).unwrap();
    host.retire_copy_provider(&p, retirement()).unwrap();

    let mut cfg = config(0, false);
    cfg.engine.limits.source.max_edits = 1;
    let mut host = LocalHost::new(cfg).unwrap();
    host.configure_copy_transfers(limits()).unwrap();
    let p = host.register_copy_provider(registration()).unwrap();
    let (doc, c) = context(&mut host, &f, &[range(0, 2), range(4, 11)]);
    let id = host.issue_invocation().unwrap();
    let task = host.admit_cut(&id, &c, &p, cut_admission()).unwrap();
    let dispatch = accepted(&mut host, &p, &task);
    assert_eq!(
        host.continue_cut(&task, publication()).unwrap(),
        TransferLocalDispositionV1::NotApplied
    );
    assert_eq!(host.read(&doc).unwrap().version, c.source);
    assert_eq!(host.logical_view(&c.view).unwrap().context(), Some(c));
    assert_eq!(host.history_usage().retained_groups, 0);
    assert_eq!(
        host.continue_cut(&task, publication()).unwrap(),
        TransferLocalDispositionV1::NotApplied
    );
    host.record_copy_evidence(&p, &dispatch, CopyEvidenceV1::Quiescent, evidence())
        .unwrap();
    host.retire_copy(&task, retirement()).unwrap();
    host.retire_copy_provider(&p, retirement()).unwrap();
}

#[test]
fn contested_export_before_publication_never_authorizes_a_cut_even_if_quiescent() {
    let f = Fixture::new();
    let mut host = host(1, true, limits());
    let p = host.register_copy_provider(registration()).unwrap();
    let (doc, c) = context(&mut host, &f, &[range(0, 2)]);
    let id = host.issue_invocation().unwrap();
    let task = host.admit_cut(&id, &c, &p, cut_admission()).unwrap();
    let dispatch = accepted(&mut host, &p, &task);
    host.record_copy_evidence(&p, &dispatch, CopyEvidenceV1::NotApplied, evidence())
        .unwrap();
    host.record_copy_evidence(&p, &dispatch, CopyEvidenceV1::Quiescent, evidence())
        .unwrap();
    error(
        host.continue_cut(&task, publication()),
        CopyErrorV1::Outstanding,
    );
    assert_eq!(
        host.cut_disposition(&task).unwrap(),
        TransferLocalDispositionV1::Pending
    );
    assert_eq!(host.invocation_state(&id).unwrap(), InvocationState::Issued);
    assert_eq!(host.read(&doc).unwrap().version, c.source);
    assert_eq!(host.history_usage().retained_groups, 0);
    host.stop_copy(&task).unwrap();
    assert_eq!(
        host.cut_disposition(&task).unwrap(),
        TransferLocalDispositionV1::Stopped
    );
    error(
        host.retire_copy(&task, retirement()),
        CopyErrorV1::Outstanding,
    );
}
