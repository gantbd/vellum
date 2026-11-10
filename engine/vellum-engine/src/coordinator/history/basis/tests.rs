use super::*;
use crate::attachment::AttachmentTarget;
fn limits() -> HistoryBasisLimitsV1 {
    HistoryBasisLimitsV1 {
        max_groups: 16,
        max_scopes: 16,
        max_participants: 8,
        max_basis_bytes: 32768,
    }
}
#[test]
fn duplicate_actual_group_identity_is_an_integrity_fault_not_an_arbitrary_first_match() {
    let (mut o, mut r, _, _, g) = fixture();
    let basis = o
        .capture_history_basis_v1(std::slice::from_ref(&g), &[], limits(), &mut r)
        .unwrap();
    o.history.groups.push(Group {
        id: g,
        members: vec![],
        annotations: vec![],
        state: GroupState::Applied,
        bytes: 0,
    });
    assert_eq!(
        o.history_basis_is_current_v1(&basis, &mut r),
        Err(HistoryError::Faulted)
    );
    assert!(o.is_faulted());
}
#[test]
fn authored_annotation_participants_are_included_even_when_only_source_scope_is_selected() {
    let (mut o, mut r) = owner();
    o.annotations.limits = Some(AnnotationLimits {
        max_issued: 8,
        max_records: 8,
        max_body_bytes: 128,
        max_retained_body_bytes: 4096,
        max_mapping_steps: 4096,
        max_live_snapshots: 8,
        max_snapshot_bytes: 8192,
    });
    let d = o.create(b"A", &mut r).unwrap();
    let p = o
        .prepare_compound(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "a".into(),
                }],
            }],
            &[],
            &[AnnotationMutation::Create {
                target: SourceTarget {
                    source: d.clone(),
                    basis: TargetBasis::Before,
                    target: AttachmentTarget::Document,
                },
                body: "comment".into(),
            }],
            &mut r,
        )
        .unwrap();
    let outcome = o.commit(p, &mut r).unwrap();
    let group = outcome.history_group.unwrap();
    let annotation = outcome.annotation_changes[0].annotation.clone();
    let basis = o
        .capture_history_basis_v1(
            &[group],
            &[HistoryBasisScopeV1::Source(d.document)],
            limits(),
            &mut r,
        )
        .unwrap();
    assert_eq!(basis.usage().scopes, 2);
    let version = o.annotation(&annotation, &mut r).unwrap().content_version;
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation,
                expected_content: version,
                body: "later".into(),
            }],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(false));
}
#[test]
fn basis_charge_depends_on_metadata_not_journal_or_source_payload_length() {
    let mut charges = Vec::new();
    for text in ["small".to_string(), "large".repeat(200)] {
        let (mut o, mut r) = owner();
        let d = o.create(b"A", &mut r).unwrap().document;
        let g = write(&mut o, &[(d.clone(), "A", &text)], &mut r);
        let basis = o
            .capture_history_basis_v1(&[g], &[HistoryBasisScopeV1::Source(d)], limits(), &mut r)
            .unwrap();
        charges.push(basis.usage());
    }
    assert_eq!(charges[0], charges[1]);
}
fn owner() -> (SourceCoordinator, Recorder) {
    (
        SourceCoordinator::with_history_limits(
            CoordinatorLimits {
                max_document_incarnations: 16,
                max_participants: 8,
                max_staged_bytes: 8192,
                source: Limits {
                    max_source_bytes: 4096,
                    max_edits: 16,
                    max_edit_payload_bytes: 8192,
                },
            },
            HistoryLimits {
                max_retained_groups: 16,
                max_journal_bytes: 65536,
                max_pins: 8,
            },
        ),
        Recorder::disabled(),
    )
}
fn write(
    o: &mut SourceCoordinator,
    edits: &[(DocumentId, &str, &str)],
    r: &mut Recorder,
) -> HistoryGroupId {
    let edits: Vec<_> = edits
        .iter()
        .map(|(id, from, to)| DocumentEdits {
            expected: o.read(id, r).unwrap().version,
            edits: vec![Replacement {
                start: 0,
                end: from.len(),
                expected: (*from).into(),
                inserted: (*to).into(),
            }],
        })
        .collect();
    let p = o.prepare(&edits, &[], r).unwrap();
    o.commit(p, r).unwrap().history_group.unwrap()
}
fn fixture() -> (
    SourceCoordinator,
    Recorder,
    DocumentId,
    DocumentId,
    HistoryGroupId,
) {
    let (mut o, mut r) = owner();
    let a = o.create(b"A", &mut r).unwrap().document;
    let b = o.create(b"B", &mut r).unwrap().document;
    let g = write(
        &mut o,
        &[(a.clone(), "A", "a"), (b.clone(), "B", "b")],
        &mut r,
    );
    (o, r, a, b, g)
}
#[test]
fn linked_basis_includes_hidden_actual_participants_and_ignores_unrelated_publication() {
    let (mut o, mut r, a, b, g) = fixture();
    let basis = o
        .capture_history_basis_v1(&[g], &[HistoryBasisScopeV1::Source(a)], limits(), &mut r)
        .unwrap();
    assert_eq!(basis.usage().scopes, 2);
    let c = o.create(b"C", &mut r).unwrap().document;
    write(&mut o, &[(c, "C", "c")], &mut r);
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(true));
    write(&mut o, &[(b, "b", "later")], &mut r);
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(false));
}
#[test]
fn both_stacks_are_order_sensitive_but_set_like_requests_and_participant_order_are_not() {
    let (mut o, mut r, a, b, g) = fixture();
    let h = write(&mut o, &[(a.clone(), "a", "new")], &mut r);
    let one = o
        .capture_history_basis_v1(
            &[g.clone(), h.clone()],
            &[
                HistoryBasisScopeV1::Source(a.clone()),
                HistoryBasisScopeV1::Source(b.clone()),
            ],
            limits(),
            &mut r,
        )
        .unwrap();
    let two = o
        .capture_history_basis_v1(
            &[h, g],
            &[
                HistoryBasisScopeV1::Source(b),
                HistoryBasisScopeV1::Source(a),
            ],
            limits(),
            &mut r,
        )
        .unwrap();
    assert_eq!(one.usage(), two.usage());
    assert_eq!(
        one.groups.iter().map(|g| &g.id).collect::<Vec<_>>(),
        two.groups.iter().map(|g| &g.id).collect::<Vec<_>>()
    );
    o.history.groups[0].members.reverse();
    assert_eq!(o.history_basis_is_current_v1(&one, &mut r), Ok(true));
    let id = &one.scopes[0].scope;
    let HistoryBasisScopeV1::Source(id) = id else {
        panic!("source")
    };
    o.documents[id.slot]
        .as_mut()
        .unwrap()
        .history
        .undo
        .swap(0, 1);
    assert_eq!(o.history_basis_is_current_v1(&one, &mut r), Ok(false));
    o.documents[id.slot]
        .as_mut()
        .unwrap()
        .history
        .undo
        .swap(0, 1);
    o.documents[id.slot]
        .as_mut()
        .unwrap()
        .history
        .redo
        .push(one.groups[0].id.clone());
    assert_eq!(o.history_basis_is_current_v1(&one, &mut r), Ok(false));
}
#[test]
fn metadata_equality_does_not_authorize_reversal_or_include_frontier_version_as_a_global_clock() {
    let (mut o, mut r, a, _, g) = fixture();
    let basis = o
        .capture_history_basis_v1(
            std::slice::from_ref(&g),
            &[HistoryBasisScopeV1::Source(a.clone())],
            limits(),
            &mut r,
        )
        .unwrap();
    o.set_read_only(&a, true, &mut r).unwrap();
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(true));
    assert_eq!(
        o.prepare_reversal(&g, HistoryDirection::Undo, &mut r)
            .unwrap_err(),
        HistoryError::Permission
    );
    o.set_read_only(&a, false, &mut r).unwrap();
    let p = o
        .prepare_reversal(&g, HistoryDirection::Undo, &mut r)
        .unwrap();
    o.commit(p, &mut r).unwrap();
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(false));
    let p = o
        .prepare_reversal(&g, HistoryDirection::Redo, &mut r)
        .unwrap();
    o.commit(p, &mut r).unwrap();
    // Endpoint primitive, not an absorbing retained-context lifecycle or ABA proof.
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(true));
}
#[test]
fn basis_does_not_pin_inverse_history_and_retirement_or_validity_change_makes_it_stale() {
    let (mut o, mut r, _, _, g) = fixture();
    let basis = o
        .capture_history_basis_v1(std::slice::from_ref(&g), &[], limits(), &mut r)
        .unwrap();
    assert_eq!(o.history_usage().active_pins, 0);
    o.retire_history_group(&g, &mut r).unwrap();
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(false));
    let (mut o, mut r, _, _, g) = fixture();
    let basis = o
        .capture_history_basis_v1(&[g], &[], limits(), &mut r)
        .unwrap();
    o.history.groups[0].state = GroupState::Invalidated;
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(false));
}
#[test]
fn missing_scope_metadata_and_unknown_history_floors_are_not_ordinary_empty_stacks() {
    let (mut o, mut r) = owner();
    let a = o.create(b"A", &mut r).unwrap().document;
    let basis = o
        .capture_history_basis_v1(
            &[],
            &[HistoryBasisScopeV1::Source(a.clone())],
            limits(),
            &mut r,
        )
        .unwrap();
    o.documents[a.slot].as_mut().unwrap().history.undo_lost = true;
    assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(false));
    let lost = o
        .capture_history_basis_v1(
            &[],
            &[HistoryBasisScopeV1::Source(a.clone())],
            limits(),
            &mut r,
        )
        .unwrap();
    o.documents[a.slot] = None; // private storage-absence probe, not a disposal implementation
    assert_eq!(o.history_basis_is_current_v1(&lost, &mut r), Ok(false));
    let absent = o
        .capture_history_basis_v1(&[], &[HistoryBasisScopeV1::Source(a)], limits(), &mut r)
        .unwrap();
    assert!(absent.scopes[0].stacks.is_none());
    assert!(basis.scopes[0].stacks.is_some());
}
#[test]
fn explicit_byte_group_scope_and_participant_limits_refuse_before_returning_a_partial_basis() {
    let (mut o, mut r, a, _, g) = fixture();
    let usage = o
        .capture_history_basis_v1(
            std::slice::from_ref(&g),
            &[HistoryBasisScopeV1::Source(a.clone())],
            limits(),
            &mut r,
        )
        .unwrap()
        .usage();
    for (coordinate, bound) in [usage.logical_bytes, 1, 2, 2].into_iter().enumerate() {
        for delta in [-1isize, 0, 1] {
            let mut p = limits();
            let n = bound.checked_add_signed(delta).unwrap();
            match coordinate {
                0 => p.max_basis_bytes = n,
                1 => p.max_groups = n,
                2 => p.max_scopes = n,
                _ => p.max_participants = n,
            }
            assert_eq!(
                o.capture_history_basis_v1(
                    std::slice::from_ref(&g),
                    &[HistoryBasisScopeV1::Source(a.clone())],
                    p,
                    &mut r
                )
                .is_ok(),
                delta >= 0,
                "{coordinate}/{delta}"
            );
            assert!(!o.is_faulted());
        }
    }
}
#[test]
fn ordinary_foreign_or_duplicate_selections_and_foreign_basis_refuse_without_fault() {
    let (mut o, mut r, a, _, g) = fixture();
    let (mut foreign, _, _, _, other) = fixture();
    assert!(matches!(
        o.capture_history_basis_v1(&[other], &[], limits(), &mut r),
        Err(HistoryError::Unavailable)
    ));
    assert!(matches!(
        o.capture_history_basis_v1(&[g.clone(), g.clone()], &[], limits(), &mut r),
        Err(HistoryError::Unavailable)
    ));
    let scope = HistoryBasisScopeV1::Source(a);
    assert!(matches!(
        o.capture_history_basis_v1(&[], &[scope.clone(), scope], limits(), &mut r),
        Err(HistoryError::Unavailable)
    ));
    let basis = o
        .capture_history_basis_v1(&[g], &[], limits(), &mut r)
        .unwrap();
    assert_eq!(
        foreign.history_basis_is_current_v1(&basis, &mut r),
        Err(HistoryError::Unavailable)
    );
    assert!(!o.is_faulted());
    assert!(!foreign.is_faulted());
}
#[test]
fn declared_global_metadata_pressure_is_distinct_from_scoped_metadata_mismatch() {
    let (mut o, mut r, _, _, g) = fixture();
    let mut p = limits();
    p.max_groups = 1;
    let basis = o.capture_history_basis_v1(&[g], &[], p, &mut r).unwrap();
    let c = o.create(b"C", &mut r).unwrap().document;
    write(&mut o, &[(c, "C", "c")], &mut r);
    assert_eq!(
        o.history_basis_is_current_v1(&basis, &mut r),
        Err(HistoryError::Limit)
    );
    assert!(!o.is_faulted());
}
#[test]
fn corrupt_actual_duplicate_participants_fault_the_owner_instead_of_issuing_a_basis() {
    let (mut o, mut r, _, _, g) = fixture();
    let id = o.history.groups[0].members[0].document.clone();
    o.history.groups[0].members.push(Member {
        document: id,
        forward: vec![],
        inverse: vec![],
    });
    assert!(matches!(
        o.capture_history_basis_v1(&[g], &[], limits(), &mut r),
        Err(HistoryError::Faulted)
    ));
    assert!(o.is_faulted());
}
#[test]
fn metadata_capture_and_comparison_are_optional_bounded_observations_not_authority_or_content() {
    for capacity in [None, Some(16), Some(1)] {
        let (mut o, _, a, _, g) = fixture();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let generation = o.generation();
        let usage = o.history_usage();
        let basis = o
            .capture_history_basis_v1(&[g], &[HistoryBasisScopeV1::Source(a)], limits(), &mut r)
            .unwrap();
        assert_eq!(o.history_basis_is_current_v1(&basis, &mut r), Ok(true));
        assert_eq!(o.generation(), generation);
        assert_eq!(o.history_usage(), usage);
        assert_eq!(format!("{basis:?}"), "HistoryBasisV1(<opaque>)");
        if capacity == Some(16) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].boundary, Boundary::HistoryBasisCapture);
            assert_eq!(events[1].boundary, Boundary::HistoryBasisValidation);
            assert!(
                events
                    .iter()
                    .all(|e| e.service_time.is_none() && e.disposition == Disposition::Completed)
            );
        } else if capacity == Some(1) {
            assert_eq!(r.dropped(), 1);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}
