use super::*;

fn prepared(
    o: &mut SourceCoordinator,
    source: &DocumentVersion,
    r: &mut Recorder,
) -> PreparedTransaction {
    o.prepare_compound(
        &[write(source.clone(), 0, 1, "a", "longer")],
        &[],
        &[AnnotationMutation::Create {
            target: target(source, 1, 2),
            body: "comment".into(),
        }],
        r,
    )
    .unwrap()
}
fn candidate_size() -> usize {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let p = prepared(&mut o, &source, &mut r);
    o.commit(p, &mut r).unwrap();
    o.semantic_root_usage(&mut r).unwrap().total_bytes
}
#[test]
fn whole_compound_candidate_counts_source_annotations_history_and_frontiers_before_any_installation()
 {
    let exact = candidate_size();
    for delta in [-1isize, 0, 1] {
        for capacity in [None, Some(128), Some(1)] {
            let mut o = owner();
            let mut r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            let source = o.create(b"abc", &mut r).unwrap();
            o.configure_semantic_root_budget(exact.checked_add_signed(delta).unwrap(), &mut r)
                .unwrap();
            let p = prepared(&mut o, &source, &mut r);
            let annotation = p.created_annotations().next().unwrap().clone();
            let before = o.semantic_root_usage(&mut r).unwrap();
            let generation = o.generation();
            let history = o.history_usage();
            let outcome = o.commit(p, &mut r);
            if delta < 0 {
                assert_eq!(outcome, Err(CoordinatorError::Limit));
                assert_eq!(o.generation(), generation);
                assert_eq!(o.history_usage(), history);
                assert!(o.annotation(&annotation, &mut r).is_err());
                assert_eq!(
                    o.read(&source.document, &mut r).unwrap().source.text(),
                    "abc"
                );
                assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
            } else {
                assert!(outcome.is_ok());
                assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, exact);
            }
            assert!(!o.is_faulted());
            if capacity == Some(128) {
                use vellum_engine::instrumentation::{Boundary, Disposition};
                assert!(std::iter::from_fn(|| r.pop_front()).any(|e| e.boundary
                    == Boundary::SemanticRootAdmission
                    && e.disposition
                        == if delta < 0 {
                            Disposition::Refused
                        } else {
                            Disposition::Completed
                        }));
            }
            if capacity == Some(1) {
                assert!(r.dropped() > 0);
            }
            if capacity.is_none() {
                assert_eq!(r.queued(), 0);
            }
        }
    }
}

#[test]
fn retained_compound_capacity_refusal_has_no_partial_changes_and_cannot_rearm_on_retry() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    o.configure_semantic_root_budget(candidate_size() - 1, &mut r)
        .unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let request = CompoundIntent {
        source: SourceIntent {
            writes: vec![write(source.clone(), 0, 1, "a", "longer")],
            reads: vec![],
        },
        annotations: vec![AnnotationMutation::Create {
            target: target(&source, 1, 2),
            body: "comment".into(),
        }],
    };
    o.submit_compound(&id, &request, &mut r).unwrap();
    let before = o.semantic_root_usage(&mut r).unwrap();
    let generation = o.generation();
    let issued = o.annotation_usage().issued_lifetime;
    assert_eq!(o.run(&id, &mut r), Ok(InvocationState::Finished));
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::Rejected(CoordinatorError::Limit)
    ));
    assert_eq!(o.run(&id, &mut r), Ok(InvocationState::Finished));
    assert_eq!(
        o.submit_compound(&id, &request, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.annotation_usage().issued_lifetime, issued);
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
    assert_eq!(
        o.read(&source.document, &mut r).unwrap().source.text(),
        "abc"
    );
}

#[test]
fn undo_redo_accounts_retained_absent_annotation_bodies_and_transferred_frontier_links() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let p = prepared(&mut o, &source, &mut r);
    let annotation = p.created_annotations().next().unwrap().clone();
    let group = o.commit(p, &mut r).unwrap().history_group.unwrap();
    let before = o.semantic_root_usage(&mut r).unwrap();
    o.configure_semantic_root_budget(before.total_bytes, &mut r)
        .unwrap();
    let undo = o
        .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
        .unwrap();
    o.commit(undo, &mut r).unwrap();
    let undone = o.semantic_root_usage(&mut r).unwrap();
    assert_eq!(undone.history_bytes, before.history_bytes);
    assert_eq!(undone.annotations_bytes, before.annotations_bytes);
    assert_eq!(undone.total_bytes + 5, before.total_bytes);
    assert!(o.annotation(&annotation, &mut r).is_err());
    let redo = o
        .prepare_reversal(&group, HistoryDirection::Redo, &mut r)
        .unwrap();
    o.commit(redo, &mut r).unwrap();
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
    assert_eq!(o.annotation(&annotation, &mut r).unwrap().body, "comment");
}

fn branch_ready() -> (SourceCoordinator, Recorder, DocumentVersion) {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let p = prepared(&mut o, &source, &mut r);
    let group = o.commit(p, &mut r).unwrap().history_group.unwrap();
    let undo = o
        .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
        .unwrap();
    o.commit(undo, &mut r).unwrap();
    let current = o.read(&source.document, &mut r).unwrap().version;
    (o, r, current)
}
#[test]
fn a_new_history_branch_uses_actual_frontier_replacement_not_a_monotonic_link_estimate() {
    let (mut probe, mut r, current) = branch_ready();
    let p = probe
        .prepare(&[write(current, 0, 1, "a", "z")], &[], &mut r)
        .unwrap();
    probe.commit(p, &mut r).unwrap();
    let exact = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
    for delta in [-1isize, 0] {
        let (mut o, mut r, current) = branch_ready();
        o.configure_semantic_root_budget(exact.checked_add_signed(delta).unwrap(), &mut r)
            .unwrap();
        let p = o
            .prepare(&[write(current, 0, 1, "a", "z")], &[], &mut r)
            .unwrap();
        let before = o.semantic_root_usage(&mut r).unwrap();
        let result = o.commit(p, &mut r);
        if delta < 0 {
            assert_eq!(result, Err(CoordinatorError::Limit));
            assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
        } else {
            result.unwrap();
            assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, exact);
        }
    }
}

#[test]
fn annotation_slot_capacity_is_checked_before_spending_a_new_identity() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let bytes = o.semantic_root_usage(&mut r).unwrap().total_bytes;
    o.configure_semantic_root_budget(bytes, &mut r).unwrap();
    let usage = o.annotation_usage();
    let generation = o.generation();
    assert!(matches!(
        o.prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                target: target(&source, 0, 1),
                body: "new".into(),
            }],
            &mut r
        ),
        Err(CoordinatorError::Limit)
    ));
    assert_eq!(o.annotation_usage(), usage);
    assert_eq!(o.generation(), generation);
    assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, bytes);
}

#[test]
fn later_peer_history_growth_is_remeasured_without_rebinding_the_original_preparation() {
    let mut probe = owner();
    let mut r = Recorder::disabled();
    let a = probe.create(b"abc", &mut r).unwrap();
    probe.create(b"p", &mut r).unwrap();
    let p = prepared(&mut probe, &a, &mut r);
    probe.commit(p, &mut r).unwrap();
    let exact = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
    let mut o = owner();
    let a = o.create(b"abc", &mut r).unwrap();
    let peer = o.create(b"p", &mut r).unwrap();
    o.configure_semantic_root_budget(exact, &mut r).unwrap();
    let p = prepared(&mut o, &a, &mut r);
    let peer_edit = o
        .prepare(&[write(peer.clone(), 0, 1, "p", "q")], &[], &mut r)
        .unwrap();
    o.commit(peer_edit, &mut r).unwrap();
    let before = o.semantic_root_usage(&mut r).unwrap();
    assert!(before.total_bytes <= exact);
    let generation = o.generation();
    assert_eq!(o.commit(p, &mut r), Err(CoordinatorError::Limit));
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
    assert_eq!(o.generation(), generation);
    assert_eq!(o.read(&peer.document, &mut r).unwrap().source.text(), "q");
    assert_eq!(o.read(&a.document, &mut r).unwrap().source.text(), "abc");
}
