use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::source::{Limits, Replacement};

fn owner() -> SourceCoordinator {
    SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 16,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 32,
                max_edit_payload_bytes: 4096,
            },
        },
        invocation: Some(InvocationLimits {
            max_lifetime_issued: 16,
            max_retained: 16,
            max_intent_bytes: 16384,
        }),
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
        resources: None,
        search: None,
    })
}
fn budget() -> SourceScopeBudget {
    SourceScopeBudget {
        max_scopes: 8,
        max_records: 32,
        max_work: 4096,
    }
}
fn scope(
    v: &DocumentVersion,
    ranges: &[(usize, usize)],
    sites: &[(usize, usize)],
) -> SourceOperationScope {
    SourceOperationScope {
        expected: v.clone(),
        replacement_ranges: ranges
            .iter()
            .map(|&(start, end)| ByteRange { start, end })
            .collect(),
        insertion_sites: sites
            .iter()
            .map(|&(first, last)| InsertionSiteRange { first, last })
            .collect(),
    }
}
fn write(
    v: &DocumentVersion,
    start: usize,
    end: usize,
    expected: &str,
    inserted: &str,
) -> DocumentEdits {
    DocumentEdits {
        expected: v.clone(),
        edits: vec![Replacement {
            start,
            end,
            expected: expected.into(),
            inserted: inserted.into(),
        }],
    }
}
#[test]
fn replacement_cells_and_insertion_gaps_are_independent_even_for_identity_edits() {
    for (start, end, allowed) in [
        (0, 2, true),
        (1, 3, true),
        (0, 3, true),
        (2, 2, false),
        (3, 3, false),
    ] {
        let mut o = owner();
        let mut r = Recorder::disabled();
        let v = o.create(b"abcd", &mut r).unwrap();
        let w = write(&v, start, end, &"abcd"[start..end], "X");
        let result = o.prepare_scoped(
            &[w],
            &[],
            &[scope(&v, &[(0, 1), (1, 3)], &[])],
            budget(),
            &mut r,
        );
        if allowed {
            o.commit(result.unwrap(), &mut r).unwrap();
        } else {
            assert_eq!(
                result.err(),
                Some(CoordinatorError::Scope(SourceScopeError::OutOfScope))
            );
        }
    }
    let mut o = owner();
    let mut r = Recorder::disabled();
    let v = o.create(b"", &mut r).unwrap();
    let w = write(&v, 0, 0, "", "");
    assert!(
        o.prepare_scoped(
            std::slice::from_ref(&w),
            &[],
            &[scope(&v, &[], &[])],
            budget(),
            &mut r
        )
        .is_err()
    );
    let p = o
        .prepare_scoped(&[w], &[], &[scope(&v, &[], &[(0, 0)])], budget(), &mut r)
        .unwrap();
    let g = o.generation();
    assert!(o.commit(p, &mut r).unwrap().operation.is_none());
    assert_eq!(o.generation(), g);
}
#[test]
fn a_gap_in_replacement_permission_cannot_be_replaced_by_its_convex_hull() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let v = o.create(b"abcd", &mut r).unwrap();
    let p = o.prepare_scoped(
        &[write(&v, 0, 4, "abcd", "X")],
        &[],
        &[scope(&v, &[(0, 1), (2, 4)], &[])],
        budget(),
        &mut r,
    );
    assert_eq!(
        p.err(),
        Some(CoordinatorError::Scope(SourceScopeError::OutOfScope))
    );
    assert_eq!(o.read(&v.document, &mut r).unwrap().source.text(), "abcd");
}

#[test]
fn finite_cell_gap_oracle_agrees_for_every_permission_set_and_single_edit() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let v = o.create(b"abcd", &mut r).unwrap();
    for cells in 0u32..16 {
        for gaps in 0u32..32 {
            let ranges: Vec<_> = (0..4)
                .filter(|i| cells & (1 << i) != 0)
                .map(|i| (i, i + 1))
                .collect();
            let sites: Vec<_> = (0..5)
                .filter(|i| gaps & (1 << i) != 0)
                .map(|i| (i, i))
                .collect();
            let scopes = [scope(&v, &ranges, &sites)];
            for start in 0..=4 {
                for end in start..=4 {
                    let allowed = if start == end {
                        gaps & (1 << start) != 0
                    } else {
                        (start..end).all(|i| cells & (1 << i) != 0)
                    };
                    let result = o.prepare_scoped(
                        &[write(&v, start, end, &"abcd"[start..end], "X")],
                        &[],
                        &scopes,
                        budget(),
                        &mut r,
                    );
                    assert_eq!(
                        result.is_ok(),
                        allowed,
                        "cell mask {cells}, gap mask {gaps}, [{start},{end})"
                    );
                    if !allowed {
                        assert_eq!(
                            result.err(),
                            Some(CoordinatorError::Scope(SourceScopeError::OutOfScope))
                        );
                    }
                }
            }
        }
    }
    for first in 0..=4 {
        for last in first..=4 {
            for site in 0..=4 {
                let result = o.prepare_scoped(
                    &[write(&v, site, site, "", "X")],
                    &[],
                    &[scope(&v, &[], &[(first, last)])],
                    budget(),
                    &mut r,
                );
                assert_eq!(result.is_ok(), (first..=last).contains(&site));
            }
        }
    }
    assert_eq!(o.read(&v.document, &mut r).unwrap().version, v);
}

#[test]
fn malformed_missing_unused_foreign_and_non_scalar_scopes_refuse_before_publication() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let v = o.create("aéz".as_bytes(), &mut r).unwrap();
    let other = o.create(b"peer", &mut r).unwrap();
    let mut foreign = owner();
    let fv = foreign.create(b"peer", &mut r).unwrap();
    let writes = [write(&v, 1, 3, "é", "Q")];
    for scopes in [
        vec![],
        vec![scope(&other, &[(0, 4)], &[])],
        vec![scope(&fv, &[(0, 4)], &[])],
        vec![scope(&v, &[(0, 4)], &[]), scope(&other, &[], &[])],
        vec![scope(&v, &[(1, 1)], &[])],
        vec![scope(&v, &[(3, 1)], &[])],
        vec![scope(&v, &[(0, 3), (1, 4)], &[])],
        vec![scope(&v, &[(1, 3), (1, 3)], &[])],
        vec![scope(&v, &[(1, 2)], &[])],
        vec![scope(&v, &[(0, 5)], &[])],
        vec![scope(&v, &[(1, 3)], &[(0, 1), (1, 3)])],
        vec![scope(&v, &[(1, 3)], &[(2, 3)])],
        vec![scope(&v, &[(1, 3)], &[(3, 0)])],
    ] {
        let g = o.generation();
        assert!(
            o.prepare_scoped(&writes, &[], &scopes, budget(), &mut r)
                .is_err()
        );
        assert_eq!(o.generation(), g);
        assert_eq!(o.read(&v.document, &mut r).unwrap().version, v);
    }
    let repeated = [scope(&v, &[], &[]), scope(&v, &[], &[])];
    let empty = DocumentEdits {
        expected: other,
        edits: vec![],
    };
    assert!(
        o.prepare_scoped(
            &[writes[0].clone(), empty],
            &[],
            &repeated,
            budget(),
            &mut r
        )
        .is_err()
    );
    // Scalar-safe endpoints on an inclusive site interval still cannot license a UTF-8 interior edit.
    assert!(
        o.prepare_scoped(
            &[write(&v, 2, 2, "", "X")],
            &[],
            &[scope(&v, &[], &[(0, 4)])],
            budget(),
            &mut r
        )
        .is_err()
    );
}

#[test]
fn original_coordinates_whole_batch_refusal_and_stale_policy_are_preserved() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let a = o.create(b"abcd", &mut r).unwrap();
    let b = o.create(b"peer", &mut r).unwrap();
    let mut w = write(&a, 0, 0, "", "LONG");
    w.edits.push(Replacement {
        start: 2,
        end: 3,
        expected: "c".into(),
        inserted: "C".into(),
    });
    let scopes = [scope(&a, &[(2, 3)], &[(0, 0)]), scope(&b, &[(0, 1)], &[])];
    assert!(
        o.prepare_scoped(
            &[w.clone(), write(&b, 0, 4, "peer", "P")],
            &[],
            &scopes,
            budget(),
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.read(&a.document, &mut r).unwrap().source.text(), "abcd");
    let p = o
        .prepare_scoped(
            std::slice::from_ref(&w),
            std::slice::from_ref(&b),
            &scopes[..1],
            budget(),
            &mut r,
        )
        .unwrap();
    o.set_read_only(&a.document, true, &mut r).unwrap();
    o.set_read_only(&a.document, false, &mut r).unwrap();
    assert_eq!(o.commit(p, &mut r), Err(CoordinatorError::Stale));
    let p = o
        .prepare_scoped(&[w], &[], &scopes[..1], budget(), &mut r)
        .unwrap();
    o.commit(p, &mut r).unwrap();
    assert_eq!(
        o.read(&a.document, &mut r).unwrap().source.text(),
        "LONGabCd"
    );
}

#[test]
fn scoped_replay_binds_restrictions_and_never_reacquires_after_source_change_or_forgetting() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let v = o.create(b"abcd", &mut r).unwrap();
    let intent = SourceIntent {
        writes: vec![write(&v, 0, 2, "ab", "AB")],
        reads: vec![],
    };
    let original = [scope(&v, &[(1, 2), (0, 1)], &[(4, 4), (0, 0)])];
    let ordered = [scope(&v, &[(0, 1), (1, 2)], &[(0, 0), (4, 4)])];
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_source(&id, &intent, &original, budget(), &mut r)
        .unwrap();
    assert_eq!(
        o.submit_scoped_source(&id, &intent, &ordered, budget(), &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        o.submit_scoped_source(&id, &intent, &[scope(&v, &[(0, 4)], &[])], budget(), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(
        o.submit(&id, &intent, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.run(&id, &mut r).unwrap();
    let g = o.generation();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::Committed(_)
    ));
    assert_eq!(
        o.submit_scoped_source(&id, &intent, &ordered, budget(), &mut r),
        Ok(InvocationState::Finished)
    );
    o.run(&id, &mut r).unwrap();
    assert_eq!(o.generation(), g);
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        o.submit_scoped_source(&id, &intent, &ordered, budget(), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        o.invocation_outcome(&id, &mut r),
        Err(InvocationError::OutcomeUnavailable)
    );
}

#[test]
fn bounds_and_optional_capture_refuse_without_widening_or_sensitive_labels() {
    for capacity in [None, Some(64), Some(1)] {
        let mut o = owner();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let v = o.create(b"abcd", &mut r).unwrap();
        let w = [write(&v, 0, 2, "ab", "AB")];
        let scopes = [scope(&v, &[(0, 1), (1, 2)], &[])];
        for b in [
            SourceScopeBudget {
                max_scopes: 0,
                ..budget()
            },
            SourceScopeBudget {
                max_records: 1,
                ..budget()
            },
            SourceScopeBudget {
                max_work: 0,
                ..budget()
            },
        ] {
            assert_eq!(
                o.prepare_scoped(&w, &[], &scopes, b, &mut r).err(),
                Some(CoordinatorError::Scope(SourceScopeError::Limit))
            );
        }
        assert!(!format!("{:?}", scopes[0]).contains("replacement_ranges"));
        let p = o
            .prepare_scoped(&w, &[], &scopes, budget(), &mut r)
            .unwrap();
        o.commit(p, &mut r).unwrap();
        assert_eq!(o.read(&v.document, &mut r).unwrap().source.text(), "ABcd");
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(64) {
            assert!(
                records
                    .iter()
                    .any(|e| e.boundary == Boundary::SourceScopeValidation)
            );
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}

#[test]
fn scoped_compound_uses_actual_annotation_and_history_companions_without_granting_authority() {
    use vellum_engine::attachment::AttachmentTarget;
    let mut o = owner();
    let mut r = Recorder::disabled();
    let v = o.create(b"abcd", &mut r).unwrap();
    let intent = CompoundIntent {
        source: SourceIntent {
            writes: vec![write(&v, 1, 2, "b", "B")],
            reads: vec![],
        },
        annotations: vec![AnnotationMutation::Create {
            target: SourceTarget {
                source: v.clone(),
                target: AttachmentTarget::ContentRange { start: 2, end: 4 },
                basis: TargetBasis::Before,
            },
            body: "note".into(),
        }],
    };
    let scopes = [scope(&v, &[(1, 2)], &[])];
    o.set_annotations_read_only(&v.document, true, &mut r)
        .unwrap();
    assert!(
        o.prepare_scoped_compound(&intent, &scopes, budget(), &mut r)
            .is_err()
    );
    assert_eq!(o.annotation_usage().issued_lifetime, 0);
    o.set_annotations_read_only(&v.document, false, &mut r)
        .unwrap();
    let p = o
        .prepare_scoped_compound(&intent, &scopes, budget(), &mut r)
        .unwrap();
    let id = p.created_annotations().next().unwrap().clone();
    let outcome = o.commit(p, &mut r).unwrap();
    let group = outcome.history_group.unwrap();
    assert_eq!(o.annotation(&id, &mut r).unwrap().body, "note");
    assert_eq!(o.read(&v.document, &mut r).unwrap().source.text(), "aBcd");
    // The old forward scope is not a standing constraint or an undo grant.
    let reverse = o
        .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
        .unwrap();
    o.commit(reverse, &mut r).unwrap();
    assert!(o.annotation(&id, &mut r).is_err());
    assert_eq!(o.read(&v.document, &mut r).unwrap().source.text(), "abcd");
    let redo = o
        .prepare_reversal(&group, HistoryDirection::Redo, &mut r)
        .unwrap();
    o.commit(redo, &mut r).unwrap();
    assert_eq!(o.annotation(&id, &mut r).unwrap().body, "note");
    // No source participants means precisely the empty scope list, even for annotation-only work.
    let empty = CompoundIntent {
        source: SourceIntent {
            writes: vec![],
            reads: vec![],
        },
        annotations: vec![],
    };
    let p = o
        .prepare_scoped_compound(&empty, &[], budget(), &mut r)
        .unwrap();
    assert!(o.commit(p, &mut r).unwrap().operation.is_none());
}

#[test]
fn scoped_invocations_retain_refusal_stop_and_stale_read_dependencies() {
    for case in 0..3 {
        let mut o = owner();
        let mut r = Recorder::disabled();
        let v = o.create(b"abcd", &mut r).unwrap();
        let peer = o.create(b"peer", &mut r).unwrap();
        let intent = SourceIntent {
            writes: vec![write(&v, 0, 1, "a", "A")],
            reads: vec![peer.clone()],
        };
        let scopes = [scope(&v, &[], &[])];
        let allowed = [scope(&v, &[(0, 1)], &[])];
        let chosen = if case == 0 { &scopes } else { &allowed };
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_scoped_source(&id, &intent, chosen, budget(), &mut r)
            .unwrap();
        if case == 1 {
            o.stop_invocation(&id, &mut r).unwrap();
        }
        if case == 2 {
            let p = o
                .prepare(&[write(&peer, 0, 4, "peer", "PEER")], &[], &mut r)
                .unwrap();
            o.commit(p, &mut r).unwrap();
        }
        let g = o.generation();
        o.run(&id, &mut r).unwrap();
        let expected = match case {
            0 => InvocationOutcome::Rejected(CoordinatorError::Scope(SourceScopeError::OutOfScope)),
            1 => InvocationOutcome::StoppedBeforePublication,
            _ => InvocationOutcome::Rejected(CoordinatorError::Stale),
        };
        assert_eq!(o.invocation_outcome(&id, &mut r), Ok(&expected));
        assert_eq!(o.generation(), g);
        assert_eq!(o.read(&v.document, &mut r).unwrap().source.text(), "abcd");
        assert_eq!(
            o.submit_scoped_source(&id, &intent, chosen, budget(), &mut r),
            Ok(InvocationState::Finished)
        );
    }
}

#[test]
fn scoped_compound_replay_retains_one_authored_identity_and_refuses_before_creation_when_outside_scope()
 {
    use vellum_engine::attachment::AttachmentTarget;
    for has_source in [false, true] {
        let mut o = owner();
        let mut r = Recorder::disabled();
        let v = o.create(b"abcd", &mut r).unwrap();
        let intent = CompoundIntent {
            source: SourceIntent {
                writes: if has_source {
                    vec![write(&v, 0, 1, "a", "A")]
                } else {
                    vec![]
                },
                reads: vec![],
            },
            annotations: vec![AnnotationMutation::Create {
                target: SourceTarget {
                    source: v.clone(),
                    target: AttachmentTarget::ContentRange { start: 2, end: 4 },
                    basis: TargetBasis::Before,
                },
                body: "note".into(),
            }],
        };
        if has_source {
            let refused = o.issue_invocation(&mut r).unwrap();
            o.submit_scoped_compound(&refused, &intent, &[scope(&v, &[], &[])], budget(), &mut r)
                .unwrap();
            assert_eq!(o.annotation_usage().issued_lifetime, 0);
            o.run(&refused, &mut r).unwrap();
            assert_eq!(
                o.invocation_outcome(&refused, &mut r),
                Ok(&InvocationOutcome::Rejected(CoordinatorError::Scope(
                    SourceScopeError::OutOfScope
                )))
            );
        }
        let scopes = if has_source {
            vec![scope(&v, &[(0, 1)], &[])]
        } else {
            vec![]
        };
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_scoped_compound(&id, &intent, &scopes, budget(), &mut r)
            .unwrap();
        let issued = o.annotation_usage().issued_lifetime;
        o.submit_scoped_compound(&id, &intent, &scopes, budget(), &mut r)
            .unwrap();
        assert_eq!(o.annotation_usage().issued_lifetime, issued);
        assert_eq!(
            o.submit_compound(&id, &intent, &mut r),
            Err(InvocationError::IntentMismatch)
        );
        let mut different = intent.clone();
        let AnnotationMutation::Create { body, .. } = &mut different.annotations[0] else {
            unreachable!()
        };
        *body = "changed".into();
        assert_eq!(
            o.submit_scoped_compound(&id, &different, &scopes, budget(), &mut r),
            Err(InvocationError::IntentMismatch)
        );
        o.run(&id, &mut r).unwrap();
        let InvocationOutcome::Committed(outcome) = o.invocation_outcome(&id, &mut r).unwrap()
        else {
            panic!("not committed")
        };
        let annotation = outcome.annotation_changes[0].annotation.clone();
        assert_eq!(o.annotation(&annotation, &mut r).unwrap().body, "note");
        assert_eq!(
            o.read(&v.document, &mut r).unwrap().source.text(),
            if has_source { "Abcd" } else { "abcd" }
        );
        let g = o.generation();
        o.forget_invocation_outcome(&id, &mut r).unwrap();
        assert_eq!(
            o.submit_scoped_compound(&id, &intent, &scopes, budget(), &mut r),
            Ok(InvocationState::Finished)
        );
        o.run(&id, &mut r).unwrap();
        assert_eq!(o.generation(), g);
        assert_eq!(o.annotation_usage().issued_lifetime, issued);
    }
}
