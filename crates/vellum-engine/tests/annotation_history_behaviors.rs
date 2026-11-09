//! Additional authored-record/history and invocation integration scenarios.
use vellum_engine::attachment::{AttachmentTarget, Placement};
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::source::{Limits, Replacement};
#[path = "annotation_history_behaviors/advice.rs"]
mod advice;
#[path = "annotation_history_behaviors/atomic_authoring.rs"]
mod atomic_authoring;
#[path = "support/discovered_context_fixture.rs"]
mod discovered_context;
#[path = "support/inspection_read_fixture.rs"]
mod inspection_read;
#[path = "annotation_history_behaviors/reversal_results.rs"]
mod reversal_results;
#[path = "annotation_history_behaviors/root_admission.rs"]
mod root_admission;
#[path = "support/selection_discovery_fixture.rs"]
mod selection_route;
fn owner() -> SourceCoordinator {
    SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 16,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 32,
                max_edit_payload_bytes: 8192,
            },
        },
        invocation: Some(InvocationLimits {
            max_lifetime_issued: 16,
            max_retained: 16,
            max_intent_bytes: 32768,
        }),
        resources: None,
        search: None,
        history: Some(HistoryLimits {
            max_retained_groups: 32,
            max_journal_bytes: 131072,
            max_pins: 8,
        }),
        annotations: Some(AnnotationLimits {
            max_issued: 32,
            max_records: 16,
            max_body_bytes: 1024,
            max_retained_body_bytes: 8192,
            max_mapping_steps: 8192,
            max_live_snapshots: 8,
            max_snapshot_bytes: 65536,
        }),
    })
}
fn target(version: &DocumentVersion, start: usize, end: usize) -> SourceTarget {
    SourceTarget {
        source: version.clone(),
        target: AttachmentTarget::ContentRange { start, end },
        basis: TargetBasis::Before,
    }
}
fn write(
    version: DocumentVersion,
    start: usize,
    end: usize,
    expected: &str,
    inserted: &str,
) -> DocumentEdits {
    DocumentEdits {
        expected: version,
        edits: vec![Replacement {
            start,
            end,
            expected: expected.into(),
            inserted: inserted.into(),
        }],
    }
}
fn create(
    owner: &mut SourceCoordinator,
    at: SourceTarget,
    body: &str,
    recorder: &mut Recorder,
) -> (AnnotationId, CommitOutcome) {
    let prepared = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                target: at,
                body: body.into(),
            }],
            recorder,
        )
        .unwrap();
    let id = prepared.created_annotations().next().unwrap().clone();
    let outcome = owner.commit(prepared, recorder).unwrap();
    (id, outcome)
}
#[test]
fn matching_create_invocations_retain_one_identity_and_one_publication() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let invocation = owner.issue_invocation(&mut recorder).unwrap();
    let intent = CompoundIntent {
        source: SourceIntent {
            writes: vec![],
            reads: vec![],
        },
        annotations: vec![AnnotationMutation::Create {
            target: target(&source, 0, 1),
            body: "one".into(),
        }],
    };
    owner
        .submit_compound(&invocation, &intent, &mut recorder)
        .unwrap();
    let issued = owner.annotation_usage().issued_lifetime;
    owner
        .submit_compound(&invocation, &intent, &mut recorder)
        .unwrap();
    assert_eq!(owner.annotation_usage().issued_lifetime, issued);
    owner.run(&invocation, &mut recorder).unwrap();
    let generation = owner.generation();
    owner
        .submit_compound(&invocation, &intent, &mut recorder)
        .unwrap();
    owner.run(&invocation, &mut recorder).unwrap();
    assert_eq!(owner.generation(), generation);
    let InvocationOutcome::Committed(outcome) = owner
        .invocation_outcome(&invocation, &mut recorder)
        .unwrap()
    else {
        panic!("not committed")
    };
    assert_eq!(outcome.annotation_changes.len(), 1);
    let id = outcome.annotation_changes[0].annotation.clone();
    assert_eq!(owner.annotation(&id, &mut recorder).unwrap().body, "one");
    let mut different = intent;
    if let AnnotationMutation::Create { body, .. } = &mut different.annotations[0] {
        *body = "two".into();
    }
    assert_eq!(
        owner.submit_compound(&invocation, &different, &mut recorder),
        Err(InvocationError::IntentMismatch)
    );
}
#[test]
fn create_undo_redo_preserves_identity_with_fresh_field_versions() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let (id, created) = create(&mut owner, target(&source, 0, 1), "body", &mut recorder);
    let group = created.history_group.unwrap();
    for _ in 0..2 {
        let undo = owner
            .prepare_reversal(&group, HistoryDirection::Undo, &mut recorder)
            .unwrap();
        owner.commit(undo, &mut recorder).unwrap();
        assert!(owner.annotation(&id, &mut recorder).is_err());
        let redo = owner
            .prepare_reversal(&group, HistoryDirection::Redo, &mut recorder)
            .unwrap();
        owner.commit(redo, &mut recorder).unwrap();
        assert_eq!(owner.annotation(&id, &mut recorder).unwrap().body, "body");
    }
    let current = owner.annotation(&id, &mut recorder).unwrap();
    assert!(current.content_version >= 3 && current.target_generation >= 3);
    assert_eq!(owner.annotation_usage().issued_lifetime, 1);
}
#[test]
fn target_reversal_refuses_unrelated_source_changes_without_mapping_evidence() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abcd", &mut recorder).unwrap();
    let (id, _) = create(&mut owner, target(&source, 0, 1), "body", &mut recorder);
    let prepared = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Reattach {
                annotation: id.clone(),
                expected_target: 1,
                target: target(&source, 2, 4),
            }],
            &mut recorder,
        )
        .unwrap();
    let group = owner
        .commit(prepared, &mut recorder)
        .unwrap()
        .history_group
        .unwrap();
    let prepared = owner
        .prepare(&[write(source.clone(), 0, 0, "", "X")], &[], &mut recorder)
        .unwrap();
    owner.commit(prepared, &mut recorder).unwrap();
    assert!(
        owner
            .prepare_reversal(&group, HistoryDirection::Undo, &mut recorder)
            .is_err()
    );
    assert_eq!(
        owner
            .annotation(&id, &mut recorder)
            .unwrap()
            .authored
            .target,
        AttachmentTarget::ContentRange { start: 2, end: 4 }
    );
}
#[test]
fn linked_candidate_target_reversal_updates_placement_context_without_relabeling_authored_coordinates()
 {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let (id, _) = create(&mut owner, target(&source, 0, 1), "body", &mut recorder);
    let mut candidate_target = target(&source, 0, 1);
    candidate_target.basis = TargetBasis::PreparedSource;
    let prepared = owner
        .prepare_compound(
            &[write(source.clone(), 0, 0, "", "X")],
            &[],
            &[AnnotationMutation::Reattach {
                annotation: id.clone(),
                expected_target: 1,
                target: candidate_target,
            }],
            &mut recorder,
        )
        .unwrap();
    let group = owner
        .commit(prepared, &mut recorder)
        .unwrap()
        .history_group
        .unwrap();
    let authored_revision = owner
        .annotation(&id, &mut recorder)
        .unwrap()
        .authored
        .source
        .revision;
    let undo = owner
        .prepare_reversal(&group, HistoryDirection::Undo, &mut recorder)
        .unwrap();
    owner.commit(undo, &mut recorder).unwrap();
    let redo = owner
        .prepare_reversal(&group, HistoryDirection::Redo, &mut recorder)
        .unwrap();
    owner.commit(redo, &mut recorder).unwrap();
    let current_revision = owner
        .read(&source.document, &mut recorder)
        .unwrap()
        .version
        .revision;
    let current = owner.annotation(&id, &mut recorder).unwrap();
    assert_eq!(current.placement_revision, Some(current_revision));
    assert_eq!(current.authored.source.revision, authored_revision);
    assert_ne!(current.authored.source.revision, current_revision);
}
#[test]
fn restoring_a_deleted_detached_annotation_preserves_detachment_and_body() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let (id, _) = create(&mut owner, target(&source, 1, 2), "body", &mut recorder);
    let prepared = owner
        .prepare(&[write(source.clone(), 1, 2, "b", "")], &[], &mut recorder)
        .unwrap();
    owner.commit(prepared, &mut recorder).unwrap();
    assert_eq!(
        owner.annotation(&id, &mut recorder).unwrap().placement,
        Placement::Detached
    );
    let prepared = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Delete {
                annotation: id.clone(),
                expected_content: 1,
                expected_target: 1,
            }],
            &mut recorder,
        )
        .unwrap();
    let group = owner
        .commit(prepared, &mut recorder)
        .unwrap()
        .history_group
        .unwrap();
    let undo = owner
        .prepare_reversal(&group, HistoryDirection::Undo, &mut recorder)
        .unwrap();
    owner.commit(undo, &mut recorder).unwrap();
    let current = owner.annotation(&id, &mut recorder).unwrap();
    assert_eq!(current.body, "body");
    assert_eq!(current.placement, Placement::Detached);
}
#[test]
fn new_annotation_work_after_linked_undo_invalidates_the_whole_source_redo() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let (id, _) = create(&mut owner, target(&source, 0, 1), "old", &mut recorder);
    let prepared = owner
        .prepare_compound(
            &[write(source.clone(), 0, 0, "", "X")],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: id.clone(),
                expected_content: 1,
                body: "group".into(),
            }],
            &mut recorder,
        )
        .unwrap();
    let group = owner
        .commit(prepared, &mut recorder)
        .unwrap()
        .history_group
        .unwrap();
    let undo = owner
        .prepare_reversal(&group, HistoryDirection::Undo, &mut recorder)
        .unwrap();
    owner.commit(undo, &mut recorder).unwrap();
    let version = owner
        .annotation(&id, &mut recorder)
        .unwrap()
        .content_version;
    let prepared = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: id,
                expected_content: version,
                body: "new".into(),
            }],
            &mut recorder,
        )
        .unwrap();
    owner.commit(prepared, &mut recorder).unwrap();
    assert_eq!(
        owner
            .history_status(&source.document, HistoryDirection::Redo, &mut recorder)
            .unwrap(),
        HistoryAvailability::Unavailable
    );
    assert!(
        owner
            .prepare_reversal(&group, HistoryDirection::Redo, &mut recorder)
            .is_err()
    );
}
#[test]
fn late_permission_change_and_stale_delete_cannot_overwrite_authored_fields() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let (id, _) = create(&mut owner, target(&source, 0, 1), "old", &mut recorder);
    let delete = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Delete {
                annotation: id.clone(),
                expected_content: 1,
                expected_target: 1,
            }],
            &mut recorder,
        )
        .unwrap();
    let body = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: id.clone(),
                expected_content: 1,
                body: "new".into(),
            }],
            &mut recorder,
        )
        .unwrap();
    owner.commit(body, &mut recorder).unwrap();
    assert!(owner.commit(delete, &mut recorder).is_err());
    let body = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: id.clone(),
                expected_content: 2,
                body: "lost".into(),
            }],
            &mut recorder,
        )
        .unwrap();
    owner
        .set_annotations_read_only(&source.document, true, &mut recorder)
        .unwrap();
    owner
        .set_annotations_read_only(&source.document, false, &mut recorder)
        .unwrap();
    assert_eq!(
        owner.commit(body, &mut recorder),
        Err(CoordinatorError::Stale)
    );
    assert_eq!(owner.annotation(&id, &mut recorder).unwrap().body, "new");
}
#[test]
fn repeated_field_wishes_and_foreign_handles_refuse_the_complete_compound() {
    let mut a = owner();
    let mut b = owner();
    let mut recorder = Recorder::disabled();
    let source = a.create(b"abc", &mut recorder).unwrap();
    let (id, _) = create(&mut a, target(&source, 0, 1), "old", &mut recorder);
    let body = AnnotationMutation::UpdateBody {
        annotation: id.clone(),
        expected_content: 1,
        body: "new".into(),
    };
    assert!(
        a.prepare_compound(&[], &[], &[body.clone(), body.clone()], &mut recorder)
            .is_err()
    );
    assert!(
        b.prepare_compound(&[], &[], &[body], &mut recorder)
            .is_err()
    );
    assert_eq!(a.annotation(&id, &mut recorder).unwrap().body, "old");
}
#[test]
fn pinned_annotation_history_prevents_retirement_and_complete_retirement_preserves_live_content() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let (id, result) = create(&mut owner, target(&source, 0, 1), "body", &mut recorder);
    let group = result.history_group.unwrap();
    let pin = owner.pin_history(&group, &mut recorder).unwrap();
    assert_eq!(
        owner.retire_history_group(&group, &mut recorder),
        Err(HistoryError::Pinned)
    );
    owner.release_history_pin(&pin, &mut recorder).unwrap();
    owner.retire_history_group(&group, &mut recorder).unwrap();
    assert_eq!(
        owner
            .annotation_history_status(&id, HistoryDirection::Undo, &mut recorder)
            .unwrap(),
        HistoryAvailability::Unavailable
    );
    assert_eq!(owner.annotation(&id, &mut recorder).unwrap().body, "body");
}

#[test]
fn composed_annotation_capture_modes_preserve_results_and_never_label_body_text() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let mut owner = owner();
        let mut recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let source = owner.create(b"abc", &mut recorder).unwrap();
        let (id, _) = create(
            &mut owner,
            target(&source, 1, 2),
            "PRIVATE_BODY",
            &mut recorder,
        );
        let prepared = owner
            .prepare(&[write(source.clone(), 1, 2, "b", "")], &[], &mut recorder)
            .unwrap();
        let group = owner
            .commit(prepared, &mut recorder)
            .unwrap()
            .history_group
            .unwrap();
        let current_version = owner
            .annotation(&id, &mut recorder)
            .unwrap()
            .content_version;
        let invocation = owner.issue_invocation(&mut recorder).unwrap();
        let intent = CompoundIntent {
            source: SourceIntent {
                writes: vec![],
                reads: vec![],
            },
            annotations: vec![AnnotationMutation::UpdateBody {
                annotation: id.clone(),
                expected_content: current_version,
                body: "PRIVATE_UPDATED".into(),
            }],
        };
        assert!(!format!("{intent:?}").contains("PRIVATE_"));
        owner
            .submit_compound(&invocation, &intent, &mut recorder)
            .unwrap();
        owner.run(&invocation, &mut recorder).unwrap();
        let undo = owner
            .prepare_reversal(&group, HistoryDirection::Undo, &mut recorder)
            .unwrap();
        owner.commit(undo, &mut recorder).unwrap();
        let comment = owner.annotation(&id, &mut recorder).unwrap();
        assert_eq!(comment.body, "PRIVATE_UPDATED");
        assert_eq!(comment.placement, Placement::Detached);
        assert!(!format!("{owner:?}").contains("PRIVATE_"));
        let mut staging = false;
        let mut publication = false;
        while let Some(record) = recorder.pop_front() {
            assert!(!format!("{record:?}").contains("PRIVATE_"));
            staging |= record.boundary == Boundary::AnnotationStaging;
            publication |= record.boundary == Boundary::AnnotationPublication;
        }
        if capacity == Some(128) {
            assert!(staging && publication);
        }
        if capacity == Some(1) {
            assert!(recorder.dropped() > 0);
        }
    }
}

#[test]
fn abandoned_create_preparations_spend_identity_but_do_not_create_live_records() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let prepared = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                target: target(&source, 0, 1),
                body: "abandoned".into(),
            }],
            &mut recorder,
        )
        .unwrap();
    let old = prepared.created_annotations().next().unwrap().clone();
    drop(prepared);
    assert_eq!(owner.annotation_usage().issued_lifetime, 1);
    assert_eq!(owner.annotation_usage().retained_records, 0);
    assert!(owner.annotation(&old, &mut recorder).is_err());
    let (new, _) = create(&mut owner, target(&source, 0, 1), "new", &mut recorder);
    assert_ne!(old, new);
    assert_eq!(owner.annotation_usage().issued_lifetime, 2);
}

#[test]
fn body_bounds_refuse_before_publishing_or_spending_a_fresh_identity() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let before = owner.annotation_usage();
    let generation = owner.generation();
    assert!(
        owner
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::Create {
                    target: target(&source, 0, 1),
                    body: "X".repeat(1025)
                }],
                &mut recorder
            )
            .is_err()
    );
    assert_eq!(owner.annotation_usage(), before);
    assert_eq!(owner.generation(), generation);
}

#[test]
fn retained_absence_does_not_refund_record_or_body_capacity() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let mut first = None;
    for _ in 0..16 {
        let (id, _) = create(&mut owner, target(&source, 0, 1), "body", &mut recorder);
        if first.is_none() {
            first = Some(id);
        }
    }
    let before = owner.generation();
    let request = AnnotationMutation::Create {
        target: target(&source, 0, 1),
        body: "extra".into(),
    };
    assert!(
        owner
            .prepare_compound(&[], &[], std::slice::from_ref(&request), &mut recorder)
            .is_err()
    );
    assert_eq!(owner.generation(), before);
    let prepared = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Delete {
                annotation: first.unwrap(),
                expected_content: 1,
                expected_target: 1,
            }],
            &mut recorder,
        )
        .unwrap();
    owner.commit(prepared, &mut recorder).unwrap();
    let usage = owner.annotation_usage();
    assert_eq!(
        (
            usage.retained_records,
            usage.present_records,
            usage.body_bytes
        ),
        (16, 15, 64)
    );
    assert!(
        owner
            .prepare_compound(&[], &[], &[request], &mut recorder)
            .is_err()
    );
}
