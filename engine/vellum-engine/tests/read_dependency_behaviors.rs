use vellum_engine::attachment::AttachmentTarget;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, Recorder};
use vellum_engine::source::{Limits, Replacement};
fn owner() -> SourceCoordinator {
    SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 16,
            max_participants: 16,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 32,
                max_edit_payload_bytes: 4096,
            },
        },
        invocation: Some(InvocationLimits {
            max_lifetime_issued: 32,
            max_retained: 32,
            max_intent_bytes: 32768,
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
fn target(v: &DocumentVersion) -> SourceTarget {
    SourceTarget {
        source: v.clone(),
        target: AttachmentTarget::ContentRange { start: 0, end: 1 },
        basis: TargetBasis::Before,
    }
}
fn request(predicates: Vec<ReadDependencyV1>) -> ReadDependencyRequest {
    ReadDependencyRequest {
        predicates,
        budget: ReadDependencyBudget {
            max_predicates: 16,
            max_work: 4096,
        },
    }
}
fn edit(v: &DocumentVersion, from: &str, to: &str) -> CompoundIntent {
    CompoundIntent {
        source: SourceIntent {
            writes: vec![DocumentEdits {
                expected: v.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: from.len(),
                    expected: from.into(),
                    inserted: to.into(),
                }],
            }],
            reads: vec![],
        },
        annotations: vec![],
    }
}
fn create(o: &mut SourceCoordinator, v: &DocumentVersion, r: &mut Recorder) -> AnnotationId {
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                target: target(v),
                body: "note".into(),
            }],
            r,
        )
        .unwrap();
    let id = p.created_annotations().next().unwrap().clone();
    o.commit(p, r).unwrap();
    id
}
#[test]
fn stale_body_dependency_refuses_the_whole_batch_even_when_source_revision_still_matches() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let a = o.create(b"A", &mut r).unwrap();
    let b = o.create(b"B", &mut r).unwrap();
    let id = create(&mut o, &b, &mut r);
    let version = o.annotation(&id, &mut r).unwrap().content_version;
    let reads = request(vec![
        ReadDependencyV1::Source(b.clone()),
        ReadDependencyV1::AnnotationBody {
            annotation: id.clone(),
            expected: version,
        },
    ]);
    let p = o
        .prepare_compound_with_reads(&edit(&a, "A", "a"), &reads, &mut r)
        .unwrap();
    let update = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: id,
                expected_content: version,
                body: "later".into(),
            }],
            &mut r,
        )
        .unwrap();
    o.commit(update, &mut r).unwrap();
    let g = o.generation();
    assert_eq!(o.commit(p, &mut r), Err(CoordinatorError::Stale));
    assert_eq!(o.generation(), g);
    assert_eq!(o.read(&a.document, &mut r).unwrap().source.text(), "A");
    assert_eq!(o.read(&b.document, &mut r).unwrap().version, b);
    let creation = CompoundIntent {
        source: SourceIntent {
            writes: vec![],
            reads: vec![],
        },
        annotations: vec![AnnotationMutation::Create {
            target: target(&a),
            body: "not created".into(),
        }],
    };
    let issued = o.annotation_usage().issued_lifetime;
    assert_eq!(
        o.prepare_compound_with_reads(&creation, &reads, &mut r)
            .err(),
        Some(CoordinatorError::Stale)
    );
    let invocation = o.issue_invocation(&mut r).unwrap();
    o.submit_compound_with_reads(&invocation, &creation, &reads, &mut r)
        .unwrap();
    o.run(&invocation, &mut r).unwrap();
    assert_eq!(
        o.invocation_outcome(&invocation, &mut r),
        Ok(&InvocationOutcome::Rejected(CoordinatorError::Stale))
    );
    assert_eq!(o.annotation_usage().issued_lifetime, issued);
}

#[test]
fn every_selected_predicate_is_required_for_changed_and_noop_publication() {
    for changed in [false, true] {
        for mismatch in 0u8..16 {
            for selected in 0u8..16 {
                let mut o = owner();
                let mut r = Recorder::disabled();
                let a = o.create(b"A", &mut r).unwrap();
                let b = o.create(b"B", &mut r).unwrap();
                let id = create(&mut o, &b, &mut r);
                let body = o.annotation(&id, &mut r).unwrap().content_version;
                let target = o.annotation(&id, &mut r).unwrap().target_generation;
                let set = o.annotation_set_version(&b.document, &mut r).unwrap();
                o.set_read_only(&b.document, true, &mut r).unwrap();
                o.set_annotations_read_only(&b.document, true, &mut r)
                    .unwrap();
                let mut source = b.clone();
                source.revision += u64::from(mismatch & 1 != 0);
                let all = [
                    ReadDependencyV1::Source(source),
                    ReadDependencyV1::AnnotationSet {
                        document: b.document.clone(),
                        expected: set + u64::from(mismatch & 2 != 0),
                    },
                    ReadDependencyV1::AnnotationBody {
                        annotation: id.clone(),
                        expected: body + u64::from(mismatch & 4 != 0),
                    },
                    ReadDependencyV1::AnnotationTarget {
                        annotation: id.clone(),
                        expected: target + u64::from(mismatch & 8 != 0),
                    },
                ];
                let req = request(
                    all.into_iter()
                        .enumerate()
                        .filter(|(i, _)| selected & (1 << i) != 0)
                        .map(|(_, p)| p)
                        .collect(),
                );
                let g = o.generation();
                let records = o.annotation_usage();
                let b_history = o.history_status(&b.document, HistoryDirection::Undo, &mut r);
                let result = o.prepare_compound_with_reads(
                    &edit(&a, "A", if changed { "a" } else { "A" }),
                    &req,
                    &mut r,
                );
                assert_eq!(result.is_ok(), mismatch & selected == 0);
                if let Ok(p) = result {
                    let outcome = o.commit(p, &mut r).unwrap();
                    assert_eq!(outcome.operation.is_some(), changed);
                } else {
                    assert_eq!(o.generation(), g);
                }
                assert_eq!(o.read(&b.document, &mut r).unwrap().version, b);
                assert_eq!(o.annotation_usage(), records);
                assert_eq!(
                    o.history_status(&b.document, HistoryDirection::Undo, &mut r),
                    b_history
                );
            }
        }
    }
}

#[test]
fn target_reads_are_not_placement_reads_and_unrelated_body_set_changes_do_not_stale_precise_reads()
{
    for kind in 0..3 {
        let mut o = owner();
        let mut r = Recorder::disabled();
        let a = o.create(b"A", &mut r).unwrap();
        let b = o.create(b"B", &mut r).unwrap();
        let id = create(&mut o, &b, &mut r);
        let current = o.annotation(&id, &mut r).unwrap();
        let body = current.content_version;
        let generation = current.target_generation;
        let placement = current.placement_generation;
        let predicate = match kind {
            0 => ReadDependencyV1::AnnotationTarget {
                annotation: id.clone(),
                expected: generation,
            },
            1 => ReadDependencyV1::AnnotationBody {
                annotation: id.clone(),
                expected: body,
            },
            _ => ReadDependencyV1::AnnotationSet {
                document: b.document.clone(),
                expected: o.annotation_set_version(&b.document, &mut r).unwrap(),
            },
        };
        let p = o
            .prepare_compound_with_reads(&edit(&a, "A", "a"), &request(vec![predicate]), &mut r)
            .unwrap();
        if kind == 0 {
            let p = o
                .prepare(
                    &[DocumentEdits {
                        expected: b.clone(),
                        edits: vec![Replacement {
                            start: 0,
                            end: 0,
                            expected: "".into(),
                            inserted: "prefix".into(),
                        }],
                    }],
                    &[],
                    &mut r,
                )
                .unwrap();
            o.commit(p, &mut r).unwrap();
            let current = o.annotation(&id, &mut r).unwrap();
            assert!(current.placement_generation > placement);
            assert_eq!(current.target_generation, generation);
        } else {
            create(&mut o, &b, &mut r);
        }
        let result = o.commit(p, &mut r);
        if kind == 2 {
            assert_eq!(result, Err(CoordinatorError::Stale));
        } else {
            result.unwrap();
        }
    }
}

#[test]
fn same_object_reads_validate_before_authored_changes_and_advice_never_reserves_validity() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let a = o.create(b"A", &mut r).unwrap();
    let id = create(&mut o, &a, &mut r);
    let body = o.annotation(&id, &mut r).unwrap().content_version;
    let req = request(vec![
        ReadDependencyV1::Source(a.clone()),
        ReadDependencyV1::AnnotationBody {
            annotation: id.clone(),
            expected: body,
        },
    ]);
    let g = o.generation();
    let inv = o.invocation_usage();
    let ann = o.annotation_usage();
    o.check_read_dependencies(&[], &req, &mut r).unwrap();
    assert_eq!(o.generation(), g);
    assert_eq!(o.invocation_usage(), inv);
    assert_eq!(o.annotation_usage(), ann);
    let mut intent = edit(&a, "A", "a");
    intent.annotations.push(AnnotationMutation::UpdateBody {
        annotation: id.clone(),
        expected_content: body,
        body: "changed".into(),
    });
    let p = o
        .prepare_compound_with_reads(&intent, &req, &mut r)
        .unwrap();
    o.commit(p, &mut r).unwrap();
    assert_eq!(o.annotation(&id, &mut r).unwrap().body, "changed");
    assert_eq!(
        o.check_read_dependencies(&[], &req, &mut r),
        Err(CoordinatorError::Stale)
    );
}

#[test]
fn repeated_foreign_deleted_and_disposed_subjects_never_satisfy_a_read() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let a = o.create(b"A", &mut r).unwrap();
    let b = o.create(b"B", &mut r).unwrap();
    let id = create(&mut o, &b, &mut r);
    let body = o.annotation(&id, &mut r).unwrap().content_version;
    let target_generation = o.annotation(&id, &mut r).unwrap().target_generation;
    let mut foreign = owner();
    let f = foreign.create(b"F", &mut r).unwrap();
    let fid = create(&mut foreign, &f, &mut r);
    for req in [
        request(vec![
            ReadDependencyV1::Source(b.clone()),
            ReadDependencyV1::Source(b.clone()),
        ]),
        request(vec![
            ReadDependencyV1::AnnotationBody {
                annotation: id.clone(),
                expected: body,
            },
            ReadDependencyV1::AnnotationBody {
                annotation: id.clone(),
                expected: body + 1,
            },
        ]),
        request(vec![ReadDependencyV1::Source(f)]),
        request(vec![ReadDependencyV1::AnnotationTarget {
            annotation: fid,
            expected: target_generation,
        }]),
    ] {
        let g = o.generation();
        assert!(
            o.prepare_compound_with_reads(&edit(&a, "A", "a"), &req, &mut r)
                .is_err()
        );
        assert_eq!(o.generation(), g);
    }
    let mut intent = edit(&a, "A", "a");
    intent.source.reads.push(b.clone());
    assert_eq!(
        o.prepare_compound_with_reads(
            &intent,
            &request(vec![ReadDependencyV1::Source(b.clone())]),
            &mut r
        )
        .err(),
        Some(CoordinatorError::ReadDependency(
            ReadDependencyError::DuplicateSubject
        ))
    );
    let deletion = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Delete {
                annotation: id.clone(),
                expected_content: body,
                expected_target: target_generation,
            }],
            &mut r,
        )
        .unwrap();
    o.commit(deletion, &mut r).unwrap();
    assert_eq!(
        o.check_read_dependencies(
            &[],
            &request(vec![ReadDependencyV1::AnnotationBody {
                annotation: id,
                expected: body
            }]),
            &mut r
        ),
        Err(CoordinatorError::Unavailable)
    );
    let disposable = o.create(b"gone", &mut r).unwrap();
    o.dispose(&disposable.document, &mut r).unwrap();
    assert_eq!(
        o.check_read_dependencies(
            &[],
            &request(vec![ReadDependencyV1::Source(disposable)]),
            &mut r
        ),
        Err(CoordinatorError::Unavailable)
    );
}

#[test]
fn replay_canonicalizes_the_read_conjunction_without_reacquiring_current_state() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let a = o.create(b"A", &mut r).unwrap();
    let b = o.create(b"B", &mut r).unwrap();
    let annotation = create(&mut o, &b, &mut r);
    let body = o.annotation(&annotation, &mut r).unwrap().content_version;
    let req = request(vec![
        ReadDependencyV1::AnnotationBody {
            annotation: annotation.clone(),
            expected: body,
        },
        ReadDependencyV1::Source(b.clone()),
    ]);
    let intent = edit(&a, "A", "a");
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_compound_with_reads(&id, &intent, &req, &mut r)
        .unwrap();
    let mut normalized = intent.clone();
    normalized.source.reads.push(b.clone());
    let moved = request(vec![req.predicates[0].clone()]);
    assert_eq!(
        o.submit_compound_with_reads(&id, &normalized, &moved, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        o.submit_compound(&id, &intent, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    assert_eq!(
        o.submit_compound_with_reads(&id, &intent, &request(vec![]), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    let update = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation,
                expected_content: body,
                body: "later".into(),
            }],
            &mut r,
        )
        .unwrap();
    o.commit(update, &mut r).unwrap();
    o.run(&id, &mut r).unwrap();
    assert_eq!(
        o.invocation_outcome(&id, &mut r),
        Ok(&InvocationOutcome::Rejected(CoordinatorError::Stale))
    );
    let g = o.generation();
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        o.submit_compound_with_reads(&id, &normalized, &moved, &mut r),
        Ok(InvocationState::Finished)
    );
    o.run(&id, &mut r).unwrap();
    assert_eq!(o.generation(), g);
}

#[test]
fn scoped_dependent_compound_and_capture_limits_use_shared_publication_without_new_authority() {
    for capacity in [None, Some(64), Some(1)] {
        let mut o = owner();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let a = o.create(b"A", &mut r).unwrap();
        let b = o.create(b"B", &mut r).unwrap();
        let mut req = request(vec![ReadDependencyV1::Source(b)]);
        let intent = edit(&a, "A", "a");
        for bounds in [
            ReadDependencyBudget {
                max_predicates: 0,
                max_work: 16,
            },
            ReadDependencyBudget {
                max_predicates: 16,
                max_work: 0,
            },
        ] {
            req.budget = bounds;
            assert_eq!(
                o.prepare_compound_with_reads(&intent, &req, &mut r).err(),
                Some(CoordinatorError::ReadDependency(ReadDependencyError::Limit))
            );
        }
        req.budget = request(vec![]).budget;
        let scopes = [SourceOperationScope {
            expected: a.clone(),
            replacement_ranges: vec![ByteRange { start: 0, end: 1 }],
            insertion_sites: vec![],
        }];
        let scope_budget = SourceScopeBudget {
            max_scopes: 8,
            max_records: 8,
            max_work: 128,
        };
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_scoped_compound_with_reads(&id, &intent, &req, &scopes, scope_budget, &mut r)
            .unwrap();
        o.run(&id, &mut r).unwrap();
        assert_eq!(o.read(&a.document, &mut r).unwrap().source.text(), "a");
        assert!(!format!("{req:?}").contains("expected"));
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(64) {
            assert!(
                records
                    .iter()
                    .any(|e| e.boundary == Boundary::ReadDependencyValidation)
            );
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

fn workspaces(o: &mut SourceCoordinator, r: &mut Recorder) {
    o.configure_workspaces(
        WorkspaceLimits {
            max_issued: 8,
            max_live: 8,
            max_memberships: 16,
            max_work: 4096,
        },
        r,
    )
    .unwrap();
}
#[test]
fn v2_mixed_reads_require_every_selected_predicate_in_changed_and_noop_cases() {
    for changed in [false, true] {
        for mismatch in 0u8..32 {
            for selected in 0u8..32 {
                let mut o = owner();
                let mut r = Recorder::disabled();
                workspaces(&mut o, &mut r);
                let a = o.create(b"A", &mut r).unwrap();
                let b = o.create(b"B", &mut r).unwrap();
                let id = create(&mut o, &b, &mut r);
                let workspace = o.create_workspace(&mut r).unwrap();
                let body = o.annotation(&id, &mut r).unwrap().content_version;
                let target = o.annotation(&id, &mut r).unwrap().target_generation;
                let set = o.annotation_set_version(&b.document, &mut r).unwrap();
                let mut source = b.clone();
                source.revision += u64::from(mismatch & 1 != 0);
                let all = [
                    ReadDependencyV2::Base(ReadDependencyV1::Source(source)),
                    ReadDependencyV2::Base(ReadDependencyV1::AnnotationSet {
                        document: b.document.clone(),
                        expected: set + u64::from(mismatch & 2 != 0),
                    }),
                    ReadDependencyV2::Base(ReadDependencyV1::AnnotationBody {
                        annotation: id.clone(),
                        expected: body + u64::from(mismatch & 4 != 0),
                    }),
                    ReadDependencyV2::Base(ReadDependencyV1::AnnotationTarget {
                        annotation: id,
                        expected: target + u64::from(mismatch & 8 != 0),
                    }),
                    ReadDependencyV2::WorkspaceMembership {
                        workspace: workspace.workspace.clone(),
                        expected: workspace.generation + u64::from(mismatch & 16 != 0),
                    },
                ];
                let req = ReadDependencyRequestV2 {
                    predicates: all
                        .into_iter()
                        .enumerate()
                        .filter(|(i, _)| selected & (1 << i) != 0)
                        .map(|(_, p)| p)
                        .collect(),
                    budget: request(vec![]).budget,
                };
                let result = o.prepare_compound_with_reads_v2(
                    &edit(&a, "A", if changed { "a" } else { "A" }),
                    &req,
                    &mut r,
                );
                assert_eq!(result.is_ok(), mismatch & selected == 0);
                if let Ok(p) = result {
                    let outcome = o.commit(p, &mut r).unwrap();
                    assert_eq!(outcome.operation.is_some(), changed);
                }
                assert_eq!(
                    o.workspace(&workspace.workspace, &mut r).unwrap().context,
                    workspace
                );
                assert_eq!(o.read(&b.document, &mut r).unwrap().version, b);
            }
        }
    }
}

#[test]
fn membership_reads_fence_aba_retirement_and_member_disposal_but_not_disjoint_changes() {
    for case in 0..5 {
        let mut o = owner();
        let mut r = Recorder::disabled();
        workspaces(&mut o, &mut r);
        let a = o.create(b"A", &mut r).unwrap();
        let b = o.create(b"B", &mut r).unwrap();
        create(&mut o, &b, &mut r);
        let w = o.create_workspace(&mut r).unwrap();
        let w = o
            .set_workspace_membership(&w, &b.document, true, &mut r)
            .unwrap()
            .context;
        let req = ReadDependencyRequestV2 {
            predicates: vec![ReadDependencyV2::WorkspaceMembership {
                workspace: w.workspace.clone(),
                expected: w.generation,
            }],
            budget: request(vec![]).budget,
        };
        o.check_read_dependencies_v2(&[], &req, &mut r).unwrap();
        let intent = edit(&a, "A", "a");
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_compound_with_reads_v2(&id, &intent, &req, &mut r)
            .unwrap();
        match case {
            0 => {
                let left = o
                    .set_workspace_membership(&w, &b.document, false, &mut r)
                    .unwrap()
                    .context;
                o.set_workspace_membership(&left, &b.document, true, &mut r)
                    .unwrap();
            }
            1 => o.retire_workspace(&w, &mut r).unwrap(),
            2 => {
                let set = o.annotation_set_version(&b.document, &mut r).unwrap();
                let p = o.prepare_retained_disposal(&b, set, &mut r).unwrap();
                o.commit_retained_disposal(p, &mut r).unwrap();
            }
            3 => {
                let peer = o.create_workspace(&mut r).unwrap();
                o.set_workspace_membership(&peer, &b.document, true, &mut r)
                    .unwrap();
            }
            _ => {
                let p = o
                    .prepare_compound(&edit(&b, "B", "b").source.writes, &[], &[], &mut r)
                    .unwrap();
                o.commit(p, &mut r).unwrap();
            }
        }
        let g = o.generation();
        o.run(&id, &mut r).unwrap();
        match case {
            0 | 2 => assert_eq!(
                o.invocation_outcome(&id, &mut r),
                Ok(&InvocationOutcome::Rejected(CoordinatorError::Stale))
            ),
            1 => assert_eq!(
                o.invocation_outcome(&id, &mut r),
                Ok(&InvocationOutcome::Rejected(CoordinatorError::Unavailable))
            ),
            _ => assert!(matches!(
                o.invocation_outcome(&id, &mut r),
                Ok(InvocationOutcome::Committed(_))
            )),
        }
        assert_eq!(
            o.read(&a.document, &mut r).unwrap().source.text(),
            if case < 3 { "A" } else { "a" }
        );
        if case < 3 {
            assert_eq!(o.generation(), g);
        }
        o.forget_invocation_outcome(&id, &mut r).unwrap();
        let g = o.generation();
        assert_eq!(
            o.submit_compound_with_reads_v2(&id, &intent, &req, &mut r),
            Ok(InvocationState::Finished)
        );
        o.run(&id, &mut r).unwrap();
        assert_eq!(o.generation(), g);
    }
}

#[test]
fn workspace_read_version_is_explicit_and_does_not_reinterpret_v1_or_foreign_subjects() {
    let mut bare = owner();
    let mut capture = Recorder::disabled();
    let source = bare.create(b"A", &mut capture).unwrap();
    let no_workspace_service = ReadDependencyRequestV2 {
        predicates: vec![],
        budget: request(vec![]).budget,
    };
    let p = bare
        .prepare_compound_with_reads_v2(
            &edit(&source, "A", "a"),
            &no_workspace_service,
            &mut capture,
        )
        .unwrap();
    bare.commit(p, &mut capture).unwrap();
    let mut o = owner();
    let mut r = Recorder::disabled();
    workspaces(&mut o, &mut r);
    let a = o.create(b"A", &mut r).unwrap();
    let w = o.create_workspace(&mut r).unwrap();
    let intent = edit(&a, "A", "a");
    let id = o.issue_invocation(&mut r).unwrap();
    let empty = request(vec![]);
    o.submit_compound_with_reads(&id, &intent, &empty, &mut r)
        .unwrap();
    let empty_v2 = ReadDependencyRequestV2 {
        predicates: vec![],
        budget: empty.budget,
    };
    assert_eq!(
        o.submit_compound_with_reads_v2(&id, &intent, &empty_v2, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    let zero = ReadDependencyRequestV2 {
        predicates: vec![ReadDependencyV2::WorkspaceMembership {
            workspace: w.workspace.clone(),
            expected: 0,
        }],
        budget: empty.budget,
    };
    assert_eq!(
        o.check_read_dependencies_v2(&[], &zero, &mut r),
        Err(CoordinatorError::Stale)
    );
    let (mut foreign, mut fr) = (owner(), Recorder::disabled());
    workspaces(&mut foreign, &mut fr);
    let foreign = foreign.create_workspace(&mut fr).unwrap();
    let foreign = ReadDependencyRequestV2 {
        predicates: vec![ReadDependencyV2::WorkspaceMembership {
            workspace: foreign.workspace,
            expected: foreign.generation,
        }],
        budget: empty.budget,
    };
    assert_eq!(
        o.check_read_dependencies_v2(&[], &foreign, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    let duplicate = ReadDependencyRequestV2 {
        predicates: vec![
            ReadDependencyV2::WorkspaceMembership {
                workspace: w.workspace.clone(),
                expected: w.generation,
            },
            ReadDependencyV2::WorkspaceMembership {
                workspace: w.workspace,
                expected: w.generation + 1,
            },
        ],
        budget: empty.budget,
    };
    let id = o.issue_invocation(&mut r).unwrap();
    assert_eq!(
        o.submit_compound_with_reads_v2(&id, &intent, &duplicate, &mut r),
        Err(InvocationError::MalformedIntent)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
}

#[test]
fn v2_scoped_preparation_and_replay_have_identical_optional_capture_semantics() {
    for retained in [false, true] {
        for capacity in [None, Some(64), Some(1)] {
            let mut o = owner();
            let mut r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            workspaces(&mut o, &mut r);
            let a = o.create(b"A", &mut r).unwrap();
            let b = o.create(b"B", &mut r).unwrap();
            let w = o.create_workspace(&mut r).unwrap();
            let req = ReadDependencyRequestV2 {
                predicates: vec![
                    ReadDependencyV2::WorkspaceMembership {
                        workspace: w.workspace.clone(),
                        expected: w.generation,
                    },
                    ReadDependencyV2::Base(ReadDependencyV1::Source(b)),
                ],
                budget: request(vec![]).budget,
            };
            let scopes = [SourceOperationScope {
                expected: a.clone(),
                replacement_ranges: vec![ByteRange { start: 0, end: 1 }],
                insertion_sites: vec![],
            }];
            let scope_budget = SourceScopeBudget {
                max_scopes: 8,
                max_records: 8,
                max_work: 128,
            };
            let intent = edit(&a, "A", "a");
            if retained {
                let id = o.issue_invocation(&mut r).unwrap();
                o.submit_scoped_compound_with_reads_v2(
                    &id,
                    &intent,
                    &req,
                    &scopes,
                    scope_budget,
                    &mut r,
                )
                .unwrap();
                o.run(&id, &mut r).unwrap();
                let g = o.generation();
                o.run(&id, &mut r).unwrap();
                assert_eq!(o.generation(), g);
            } else {
                let p = o
                    .prepare_scoped_compound_with_reads_v2(
                        &intent,
                        &req,
                        &scopes,
                        scope_budget,
                        &mut r,
                    )
                    .unwrap();
                o.commit(p, &mut r).unwrap();
            }
            assert_eq!(o.read(&a.document, &mut r).unwrap().source.text(), "a");
            assert_eq!(o.workspace(&w.workspace, &mut r).unwrap().context, w);
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            if capacity == Some(64) {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == Boundary::ReadDependencyValidation)
                );
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == Boundary::SourceScopeValidation)
                );
            }
            if capacity == Some(1) {
                assert!(r.dropped() > 0);
            }
        }
    }
}
