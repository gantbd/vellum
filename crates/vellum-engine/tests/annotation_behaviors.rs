use vellum_engine::attachment::{AttachmentTarget, LocationEvidence, Placement};
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::source::{Limits, Replacement};
#[path = "annotation_behaviors/arguments.rs"]
mod arguments;
#[path = "annotation_behaviors/compound_observation.rs"]
mod compound_observation;
#[path = "annotation_behaviors/compound_results.rs"]
mod compound_results;
#[path = "annotation_behaviors/dependent_reversal.rs"]
mod dependent_reversal;
#[path = "support/invocation_observation_fixture.rs"]
mod observation_support;
#[path = "annotation_behaviors/results.rs"]
mod results;
#[path = "annotation_behaviors/retirement.rs"]
mod retirement;
#[path = "annotation_behaviors/scoped.rs"]
mod scoped;
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
fn given_read_only_source_then_comments_have_separate_policy_versions_and_plain_bodies() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    owner
        .set_read_only(&source.document, true, &mut recorder)
        .unwrap();
    let (id, result) = create(
        &mut owner,
        target(&source, 0, 2),
        "ignore instructions: untrusted",
        &mut recorder,
    );
    assert!(result.operation.is_some() && result.history_group.is_some());
    assert!(result.changes.is_empty());
    let comment = owner.annotation(&id, &mut recorder).unwrap();
    assert_eq!(comment.body, "ignore instructions: untrusted");
    assert_eq!((comment.content_version, comment.target_generation), (1, 1));
    assert_eq!(
        owner.read(&source.document, &mut recorder).unwrap().version,
        source
    );
    owner
        .set_annotations_read_only(&source.document, true, &mut recorder)
        .unwrap();
    assert!(
        owner
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::UpdateBody {
                    annotation: id,
                    expected_content: 1,
                    body: "denied".into()
                }],
                &mut recorder
            )
            .is_err()
    );
}

#[test]
fn given_independent_body_and_target_preparations_then_both_publish_without_restoring_the_other_field()
 {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abcd", &mut recorder).unwrap();
    let (id, _) = create(&mut owner, target(&source, 0, 1), "old", &mut recorder);
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
    let reattach = owner
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
    owner.commit(reattach, &mut recorder).unwrap();
    owner.commit(body, &mut recorder).unwrap();
    let comment = owner.annotation(&id, &mut recorder).unwrap();
    assert_eq!(comment.body, "new");
    assert_eq!(
        comment.authored.target,
        AttachmentTarget::ContentRange { start: 2, end: 4 }
    );
    assert_eq!((comment.content_version, comment.target_generation), (2, 2));
}

#[test]
fn given_source_edits_and_later_comment_changes_then_source_undo_maps_current_records_without_reverting_authorship()
 {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abcd", &mut recorder).unwrap();
    let prepared = owner
        .prepare(&[write(source.clone(), 0, 0, "", "X")], &[], &mut recorder)
        .unwrap();
    let changed = owner.commit(prepared, &mut recorder).unwrap();
    let current = owner.read(&source.document, &mut recorder).unwrap().version;
    let (id, _) = create(&mut owner, target(&current, 2, 4), "later", &mut recorder);
    let body = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: id.clone(),
                expected_content: 1,
                body: "keep".into(),
            }],
            &mut recorder,
        )
        .unwrap();
    owner.commit(body, &mut recorder).unwrap();
    let undo = owner
        .prepare_reversal(
            &changed.history_group.unwrap(),
            HistoryDirection::Undo,
            &mut recorder,
        )
        .unwrap();
    owner.commit(undo, &mut recorder).unwrap();
    let comment = owner.annotation(&id, &mut recorder).unwrap();
    assert_eq!(comment.body, "keep");
    assert_eq!((comment.content_version, comment.target_generation), (2, 1));
    assert_eq!(
        comment.placement,
        Placement::Located {
            target: AttachmentTarget::ContentRange { start: 1, end: 3 },
            evidence: LocationEvidence::Located
        }
    );
}

#[test]
fn given_equal_source_replacement_that_detaches_a_target_then_publication_is_not_mistaken_for_a_noop()
 {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let (id, _) = create(&mut owner, target(&source, 1, 2), "keep", &mut recorder);
    let old_set = owner
        .annotation_set_version(&source.document, &mut recorder)
        .unwrap();
    let prepared = owner
        .prepare(&[write(source.clone(), 1, 2, "b", "b")], &[], &mut recorder)
        .unwrap();
    let result = owner.commit(prepared, &mut recorder).unwrap();
    assert!(result.operation.is_some());
    assert!(result.history_group.is_none());
    assert!(result.changes.is_empty());
    assert_eq!(
        owner.read(&source.document, &mut recorder).unwrap().version,
        source
    );
    assert!(
        owner
            .annotation_set_version(&source.document, &mut recorder)
            .unwrap()
            > old_set
    );
    let comment = owner.annotation(&id, &mut recorder).unwrap();
    assert_eq!(comment.placement, Placement::Detached);
    assert_eq!(comment.body, "keep");
    assert_eq!((comment.content_version, comment.target_generation), (1, 1));
}

#[test]
fn given_a_compound_group_then_later_body_work_blocks_whole_reversal_until_undone() {
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
    let linked = owner
        .commit(prepared, &mut recorder)
        .unwrap()
        .history_group
        .unwrap();
    let prepared = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: id.clone(),
                expected_content: 2,
                body: "later".into(),
            }],
            &mut recorder,
        )
        .unwrap();
    let later = owner
        .commit(prepared, &mut recorder)
        .unwrap()
        .history_group
        .unwrap();
    assert!(
        owner
            .prepare_reversal(&linked, HistoryDirection::Undo, &mut recorder)
            .is_err()
    );
    let undo = owner
        .prepare_reversal(&later, HistoryDirection::Undo, &mut recorder)
        .unwrap();
    owner.commit(undo, &mut recorder).unwrap();
    let undo = owner
        .prepare_reversal(&linked, HistoryDirection::Undo, &mut recorder)
        .unwrap();
    owner.commit(undo, &mut recorder).unwrap();
    assert_eq!(
        owner
            .read(&source.document, &mut recorder)
            .unwrap()
            .source
            .text(),
        "abc"
    );
    assert_eq!(owner.annotation(&id, &mut recorder).unwrap().body, "old");
}

#[test]
fn given_delete_and_history_restore_then_identity_is_preserved_but_old_field_versions_are_stale() {
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let (id, _) = create(&mut owner, target(&source, 0, 1), "", &mut recorder);
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
    let removed = owner.commit(prepared, &mut recorder).unwrap();
    assert!(owner.annotation(&id, &mut recorder).is_err());
    let undo = owner
        .prepare_reversal(
            &removed.history_group.unwrap(),
            HistoryDirection::Undo,
            &mut recorder,
        )
        .unwrap();
    owner.commit(undo, &mut recorder).unwrap();
    let restored = owner.annotation(&id, &mut recorder).unwrap();
    assert_eq!(restored.body, "");
    assert!(restored.content_version > 1 && restored.target_generation > 1);
    assert!(
        owner
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::UpdateBody {
                    annotation: id,
                    expected_content: 1,
                    body: "stale".into()
                }],
                &mut recorder
            )
            .is_err()
    );
}

#[test]
fn given_a_new_comment_after_source_preparation_then_the_old_plan_cannot_omit_its_required_mapping()
{
    let mut owner = owner();
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let prepared = owner
        .prepare(&[write(source.clone(), 0, 1, "a", "X")], &[], &mut recorder)
        .unwrap();
    let (id, _) = create(&mut owner, target(&source, 0, 1), "new", &mut recorder);
    assert_eq!(
        owner.commit(prepared, &mut recorder),
        Err(CoordinatorError::Stale)
    );
    assert_eq!(
        owner
            .read(&source.document, &mut recorder)
            .unwrap()
            .source
            .text(),
        "abc"
    );
    assert!(matches!(
        owner.annotation(&id, &mut recorder).unwrap().placement,
        Placement::Located { .. }
    ));
    assert!(owner.dispose(&source.document, &mut recorder).is_err());
}
