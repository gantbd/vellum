use super::*;
fn facts() -> TrustedAnnotationRetirementV1 {
    TrustedAnnotationRetirementV1 {
        authorized: Some(true),
        membership_current: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
        session_references_released: Some(true),
        authored_disposition_ready: Some(true),
        history_released: Some(true),
        versions_available: Some(true),
        resources_ready: Some(true),
        bookkeeping_ready: Some(true),
    }
}
fn delete(
    o: &mut SourceCoordinator,
    id: &AnnotationId,
    r: &mut Recorder,
) -> (AnnotationFact, HistoryGroupId) {
    let current = o.annotation(id, r).unwrap();
    let content = current.content_version;
    let target = current.target_generation;
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Delete {
                annotation: id.clone(),
                expected_content: content,
                expected_target: target,
            }],
            r,
        )
        .unwrap();
    let outcome = o.commit(p, r).unwrap();
    (
        outcome.annotation_changes.into_iter().next().unwrap(),
        outcome.history_group.unwrap(),
    )
}

#[test]
fn given_released_absent_annotation_then_final_retirement_frees_payload_without_changing_live_state()
 {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let (id, _) = create(&mut o, target(&source, 0, 1), "private payload", &mut r);
    let (absent, deleted) = delete(&mut o, &id, &mut r);
    o.retire_history_group(&deleted, &mut r).unwrap();
    let set = o.annotation_set_version(&source.document, &mut r).unwrap();
    let before = o.annotation_usage();
    let root = o.semantic_root_usage(&mut r).unwrap();
    let generation = o.generation();
    o.retire_absent_annotation(&absent, &facts(), &mut r)
        .unwrap();
    let after = o.annotation_usage();
    assert_eq!(after.issued_lifetime, before.issued_lifetime);
    assert_eq!(after.retained_records, before.retained_records - 1);
    assert_eq!(
        after.body_bytes,
        before.body_bytes - "private payload".len()
    );
    assert_eq!(o.generation(), generation + 1);
    let after_root = o.semantic_root_usage(&mut r).unwrap();
    assert_eq!(
        root.annotations_bytes - after_root.annotations_bytes,
        "private payload".len()
    );
    assert_eq!(
        o.annotation_set_version(&source.document, &mut r).unwrap(),
        set
    );
    assert_eq!(o.read(&source.document, &mut r).unwrap().version, source);
    assert!(matches!(
        o.annotation(&id, &mut r),
        Err(CoordinatorError::Unavailable)
    ));
    assert!(
        o.retire_absent_annotation(&absent, &facts(), &mut r)
            .is_err()
    );
    let (next, _) = create(&mut o, target(&source, 1, 2), "new", &mut r);
    assert_ne!(id, next);
    assert!(!o.is_faulted());
}

#[test]
fn given_history_and_cloned_captures_then_attested_release_cannot_bypass_actual_holds() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let (id, _) = create(&mut o, target(&source, 0, 1), "held", &mut r);
    let snapshot = o.capture_annotations(&source, 1, &mut r).unwrap();
    let clone = snapshot.clone();
    let (absent, group) = delete(&mut o, &id, &mut r);
    let retained = Err(CoordinatorError::Annotation(AnnotationError::Retained));
    assert_eq!(
        o.retire_absent_annotation(&absent, &facts(), &mut r),
        retained
    );
    o.retire_history_group(&group, &mut r).unwrap();
    assert_eq!(
        o.retire_absent_annotation(&absent, &facts(), &mut r),
        retained
    );
    drop(snapshot);
    assert_eq!(
        o.retire_absent_annotation(&absent, &facts(), &mut r),
        retained
    );
    assert_eq!(clone.records()[0].body(), "held");
    drop(clone);
    // A later empty capture owns no version of this record and need not drain.
    let empty = o.capture_annotations(&source, 2, &mut r).unwrap();
    o.retire_absent_annotation(&absent, &facts(), &mut r)
        .unwrap();
    assert!(empty.records().is_empty());
    assert_eq!(o.annotation_usage().retained_snapshots, 1);
}

#[test]
fn given_invalidated_history_then_frontier_removal_alone_cannot_authorize_retirement() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let (id, _) = create(&mut o, target(&source, 0, 1), "journal", &mut r);
    let (_, old_delete) = delete(&mut o, &id, &mut r);
    let undo = o
        .prepare_reversal(&old_delete, HistoryDirection::Undo, &mut r)
        .unwrap();
    o.commit(undo, &mut r).unwrap();
    let (absent, new_delete) = delete(&mut o, &id, &mut r);
    assert!(o.history_usage().invalidated_groups > 0);
    o.retire_history_group(&new_delete, &mut r).unwrap();
    assert_eq!(o.history_usage().retained_groups, 1);
    let generation = o.generation();
    let usage = o.annotation_usage();
    assert_eq!(
        o.retire_absent_annotation(&absent, &facts(), &mut r),
        Err(CoordinatorError::Annotation(AnnotationError::Retained))
    );
    assert_eq!(o.annotation_usage(), usage);
    assert_eq!(o.generation(), generation);
    o.retire_history_group(&old_delete, &mut r).unwrap();
    o.retire_absent_annotation(&absent, &facts(), &mut r)
        .unwrap();
}

#[test]
fn given_missing_current_gates_or_wrong_floors_then_retirement_changes_nothing() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let source = o.create(b"abc", &mut r).unwrap();
    let (id, created) = create(&mut o, target(&source, 0, 1), "keep", &mut r);
    assert!(
        o.retire_absent_annotation(&created.annotation_changes[0], &facts(), &mut r)
            .is_err()
    );
    let (absent, group) = delete(&mut o, &id, &mut r);
    o.retire_history_group(&group, &mut r).unwrap();
    let generation = o.generation();
    let usage = o.annotation_usage();
    for index in 0..10 {
        for value in [None, Some(false)] {
            let mut f = facts();
            *match index {
                0 => &mut f.authorized,
                1 => &mut f.membership_current,
                2 => &mut f.retention_released,
                3 => &mut f.delivery_drained,
                4 => &mut f.session_references_released,
                5 => &mut f.authored_disposition_ready,
                6 => &mut f.history_released,
                7 => &mut f.versions_available,
                8 => &mut f.resources_ready,
                _ => &mut f.bookkeeping_ready,
            } = value;
            assert_eq!(
                o.retire_absent_annotation(&absent, &f, &mut r),
                Err(CoordinatorError::Unavailable)
            );
            assert_eq!(o.generation(), generation);
            assert_eq!(o.annotation_usage(), usage);
        }
    }
    for index in 0..4 {
        let mut wrong = absent.clone();
        match index {
            0 => wrong.content_version += 1,
            1 => wrong.target_generation += 1,
            2 => wrong.placement_generation += 1,
            _ => wrong.present = true,
        }
        assert_eq!(
            o.retire_absent_annotation(&wrong, &facts(), &mut r),
            Err(CoordinatorError::Stale)
        );
    }
    let mut peer = owner();
    let peer_source = peer.create(b"abc", &mut r).unwrap();
    let (peer_id, _) = create(&mut peer, target(&peer_source, 0, 1), "peer", &mut r);
    let mut foreign = absent.clone();
    foreign.annotation = peer_id;
    assert_eq!(
        o.retire_absent_annotation(&foreign, &facts(), &mut r),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.annotation_usage(), usage);
    assert!(!o.is_faulted());
    o.retire_absent_annotation(&absent, &facts(), &mut r)
        .unwrap();
}

#[test]
fn given_optional_or_saturated_capture_then_retirement_has_same_result_and_content_free_markers() {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for capacity in [None, Some(8), Some(1)] {
        let mut o = owner();
        let mut r = Recorder::disabled();
        let source = o.create(b"abc", &mut r).unwrap();
        let (id, _) = create(&mut o, target(&source, 0, 1), "PRIVATE_RETIRE", &mut r);
        let (absent, group) = delete(&mut o, &id, &mut r);
        o.retire_history_group(&group, &mut r).unwrap();
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let mut denied = facts();
        denied.authorized = Some(false);
        assert!(
            o.retire_absent_annotation(&absent, &denied, &mut r)
                .is_err()
        );
        o.retire_absent_annotation(&absent, &facts(), &mut r)
            .unwrap();
        assert_eq!(o.annotation_usage().retained_records, 0);
        assert!(!o.is_faulted());
        let mut markers = vec![];
        while let Some(v) = r.pop_front() {
            markers.push(v);
        }
        assert!(
            markers
                .iter()
                .all(|v| v.boundary == Boundary::AnnotationRetirement && v.service_time.is_none())
        );
        assert!(!format!("{markers:?}").contains("PRIVATE_RETIRE"));
        if capacity == Some(8) {
            assert_eq!(
                markers.iter().map(|v| v.disposition).collect::<Vec<_>>(),
                vec![Disposition::Refused, Disposition::Completed]
            );
        }
        if capacity == Some(1) {
            assert_eq!(r.dropped(), 1);
        }
    }
}
