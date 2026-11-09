use vellum_engine::attachment::{AttachmentTarget, Placement};
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::source::{Limits, Replacement};
fn owner(max_snapshots: usize, max_snapshot_bytes: usize) -> SourceCoordinator {
    SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 4,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 16,
                max_edit_payload_bytes: 8192,
            },
        },
        invocation: None,
        resources: None,
        search: None,
        history: None,
        annotations: Some(AnnotationLimits {
            max_issued: 16,
            max_records: 16,
            max_body_bytes: 1024,
            max_retained_body_bytes: 8192,
            max_mapping_steps: 8192,
            max_live_snapshots: max_snapshots,
            max_snapshot_bytes,
        }),
    })
}
fn create(
    owner: &mut SourceCoordinator,
    source: &DocumentVersion,
    body: &str,
    recorder: &mut Recorder,
) -> AnnotationId {
    let prepared = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                body: body.into(),
                target: SourceTarget {
                    source: source.clone(),
                    basis: TargetBasis::Before,
                    target: AttachmentTarget::ContentRange { start: 0, end: 1 },
                },
            }],
            recorder,
        )
        .unwrap();
    let id = prepared.created_annotations().next().unwrap().clone();
    owner.commit(prepared, recorder).unwrap();
    id
}
#[test]
fn semantic_root_counts_authored_body_but_not_a_separately_retained_snapshot() {
    let mut o = owner(4, 16384);
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let empty = o.semantic_root_usage(&mut r).unwrap();
    let annotation = create(&mut o, &source, "old", &mut r);
    let authored = o.semantic_root_usage(&mut r).unwrap();
    assert!(authored.annotations_bytes > empty.annotations_bytes);
    let set = o.annotation_set_version(&source.document, &mut r).unwrap();
    let snapshot = o.capture_annotations(&source, set, &mut r).unwrap();
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), authored);
    let update = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation,
                expected_content: 1,
                body: "longer".into(),
            }],
            &mut r,
        )
        .unwrap();
    o.commit(update, &mut r).unwrap();
    let changed = o.semantic_root_usage(&mut r).unwrap();
    assert_eq!(changed.annotations_bytes - authored.annotations_bytes, 3);
    assert_eq!(snapshot.records()[0].body(), "old");
    drop(snapshot);
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), changed);
}

#[test]
fn capture_is_complete_immutable_and_separate_from_live_authored_state() {
    let mut owner = owner(4, 16384);
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let id = create(&mut owner, &source, "PRIVATE_ORIGINAL", &mut recorder);
    let set = owner
        .annotation_set_version(&source.document, &mut recorder)
        .unwrap();
    let generation = owner.generation();
    let snapshot = owner
        .capture_annotations(&source, set, &mut recorder)
        .unwrap();
    assert_eq!(owner.generation(), generation);
    assert_eq!(snapshot.source(), &source);
    assert_eq!(snapshot.records().len(), 1);
    let update = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: id.clone(),
                expected_content: 1,
                body: "PRIVATE_NEW".into(),
            }],
            &mut recorder,
        )
        .unwrap();
    owner.commit(update, &mut recorder).unwrap();
    assert_eq!(snapshot.records()[0].body(), "PRIVATE_ORIGINAL");
    assert_eq!(snapshot.set_version(), set);
    assert_eq!(
        owner.annotation(&id, &mut recorder).unwrap().body,
        "PRIVATE_NEW"
    );
    assert!(!format!("{snapshot:?} {:?}", snapshot.records()[0]).contains("PRIVATE_"));
}
#[test]
fn stale_source_or_set_and_capacity_refuse_without_returning_a_prefix() {
    let mut owner = owner(1, 16384);
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    create(&mut owner, &source, "one", &mut recorder);
    assert!(
        owner
            .capture_annotations(&source, 0, &mut recorder)
            .is_err()
    );
    let set = owner
        .annotation_set_version(&source.document, &mut recorder)
        .unwrap();
    let snapshot = owner
        .capture_annotations(&source, set, &mut recorder)
        .unwrap();
    let retained = snapshot.clone();
    drop(snapshot);
    assert_eq!(owner.annotation_usage().retained_snapshots, 1);
    assert!(
        owner
            .capture_annotations(&source, set, &mut recorder)
            .is_err()
    );
    drop(retained);
    assert!(
        owner
            .capture_annotations(&source, set, &mut recorder)
            .is_ok()
    );
    let edit = owner
        .prepare(
            &[DocumentEdits {
                expected: source.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 0,
                    expected: "".into(),
                    inserted: "X".into(),
                }],
            }],
            &[],
            &mut recorder,
        )
        .unwrap();
    owner.commit(edit, &mut recorder).unwrap();
    assert!(
        owner
            .capture_annotations(&source, set, &mut recorder)
            .is_err()
    );
}
#[test]
fn capture_excludes_retained_absence_but_preserves_detached_live_bodies() {
    let mut owner = owner(4, 16384);
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    let removed = create(&mut owner, &source, "removed", &mut recorder);
    create(&mut owner, &source, "keep", &mut recorder);
    let delete = owner
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Delete {
                annotation: removed,
                expected_content: 1,
                expected_target: 1,
            }],
            &mut recorder,
        )
        .unwrap();
    owner.commit(delete, &mut recorder).unwrap();
    let edit = owner
        .prepare(
            &[DocumentEdits {
                expected: source.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "a".into(),
                    inserted: "".into(),
                }],
            }],
            &[],
            &mut recorder,
        )
        .unwrap();
    owner.commit(edit, &mut recorder).unwrap();
    let current = owner.read(&source.document, &mut recorder).unwrap().version;
    let set = owner
        .annotation_set_version(&source.document, &mut recorder)
        .unwrap();
    let snapshot = owner
        .capture_annotations(&current, set, &mut recorder)
        .unwrap();
    assert_eq!(snapshot.records().len(), 1);
    assert_eq!(snapshot.records()[0].body(), "keep");
    assert_eq!(snapshot.records()[0].placement, Placement::Detached);
    assert_eq!(snapshot.records()[0].authored.source, source);
    assert_eq!(snapshot.source_text(), "bc");
}
#[test]
fn capture_storage_budget_is_checked_before_accepting_a_complete_snapshot() {
    let mut owner = owner(4, 1);
    let mut recorder = Recorder::disabled();
    let source = owner.create(b"abc", &mut recorder).unwrap();
    create(&mut owner, &source, "body", &mut recorder);
    let set = owner
        .annotation_set_version(&source.document, &mut recorder)
        .unwrap();
    let generation = owner.generation();
    assert!(
        owner
            .capture_annotations(&source, set, &mut recorder)
            .is_err()
    );
    assert_eq!(owner.annotation_usage().retained_snapshots, 0);
    assert_eq!(owner.generation(), generation);
}

#[test]
fn disabled_available_or_saturated_capture_keeps_the_same_snapshot_and_safe_diagnostics() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(64), Some(1)] {
        let mut owner = owner(4, 16384);
        let mut recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let source = owner.create(b"abc", &mut recorder).unwrap();
        create(&mut owner, &source, "PRIVATE_BODY", &mut recorder);
        let set = owner
            .annotation_set_version(&source.document, &mut recorder)
            .unwrap();
        let generation = owner.generation();
        let snapshot = owner
            .capture_annotations(&source, set, &mut recorder)
            .unwrap();
        assert_eq!(snapshot.source(), &source);
        assert_eq!(snapshot.set_version(), set);
        assert_eq!(snapshot.records()[0].body(), "PRIVATE_BODY");
        assert_eq!(snapshot.publication_generation(), generation);
        assert_eq!(owner.generation(), generation);
        let mut captured = false;
        while let Some(record) = recorder.pop_front() {
            captured |= record.boundary == Boundary::AnnotationCapture;
            assert!(!format!("{record:?}").contains("PRIVATE_"));
        }
        if capacity == Some(64) {
            assert!(captured);
        }
        if capacity == Some(1) {
            assert!(recorder.dropped() > 0);
        }
    }
}
