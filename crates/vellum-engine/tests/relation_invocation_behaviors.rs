use vellum_engine::attachment::{Affinity, AttachmentTarget};
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::instrumentation::{Boundary, Disposition};
use vellum_engine::selection::SelectionBudget;
use vellum_engine::selection::{Selection, SelectionError};
use vellum_engine::source::Limits;
use vellum_engine::source::Replacement;
#[path = "relation_invocation_behaviors/arguments.rs"]
mod arguments;
#[path = "relation_invocation_behaviors/discovered.rs"]
mod discovered;
#[path = "relation_invocation_behaviors/results.rs"]
mod results;
fn owner() -> (SourceCoordinator, Recorder) {
    owner_with_live(8)
}
fn owner_with_live(live: usize) -> (SourceCoordinator, Recorder) {
    let mut o = SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 1024,
            source: Limits {
                max_source_bytes: 1024,
                max_edits: 16,
                max_edit_payload_bytes: 4096,
            },
        },
        invocation: Some(InvocationLimits {
            max_lifetime_issued: 32,
            max_retained: 32,
            max_intent_bytes: 16384,
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
        history: Some(HistoryLimits {
            max_retained_groups: 32,
            max_journal_bytes: 131072,
            max_pins: 8,
        }),
        resources: None,
        search: None,
    });
    let mut r = Recorder::disabled();
    o.configure_workspaces(
        WorkspaceLimits {
            max_issued: 8,
            max_live: live,
            max_memberships: 16,
            max_work: 4096,
        },
        &mut r,
    )
    .unwrap();
    o.configure_logical_views(
        LogicalViewLimits {
            max_live_views: live,
            max_issued_views: 8,
            max_total_selections: 32,
            normalization: SelectionBudget {
                max_input_selections: 16,
                max_output_selections: 16,
                max_work: 4096,
            },
            max_mapping_steps: 4096,
        },
        &mut r,
    )
    .unwrap();
    (o, r)
}
#[test]
fn each_container_creation_executes_once_and_forgetting_never_reopens_it() {
    for command in [
        RelationCommandV1::CreateView,
        RelationCommandV1::CreateWorkspace,
    ] {
        let (mut o, mut r) = owner();
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_relation(&id, &command, &mut r).unwrap();
        let before = o.generation();
        assert_eq!(
            o.submit_relation(&id, &command, &mut r),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.generation(), before);
        o.run(&id, &mut r).unwrap();
        assert!(matches!(
            o.invocation_outcome(&id, &mut r),
            Ok(InvocationOutcome::RelationCommitted(_))
        ));
        let InvocationOutcome::RelationCommitted(receipt) =
            o.invocation_outcome(&id, &mut r).unwrap()
        else {
            unreachable!()
        };
        let receipt = receipt.clone();
        match &receipt {
            RelationReceiptV1::ViewCreated(view) => {
                let stamp = o.logical_view(view, &mut r).unwrap().stamp;
                o.retire_logical_view(view, stamp, &mut r).unwrap();
            }
            RelationReceiptV1::WorkspaceCreated(workspace) => {
                let context = o.workspace(workspace, &mut r).unwrap().context;
                o.retire_workspace(&context, &mut r).unwrap();
            }
            _ => unreachable!(),
        }
        assert_eq!(
            o.invocation_outcome(&id, &mut r),
            Ok(&InvocationOutcome::RelationCommitted(receipt))
        );
        let g = o.generation();
        o.run(&id, &mut r).unwrap();
        assert_eq!(o.generation(), g);
        o.forget_invocation_outcome(&id, &mut r).unwrap();
        assert_eq!(
            o.submit_relation(&id, &command, &mut r),
            Ok(InvocationState::Finished)
        );
        o.run(&id, &mut r).unwrap();
        assert_eq!(o.generation(), g);
        o.retire_invocation(&id, &mut r).unwrap();
        assert_eq!(
            o.submit_relation(&id, &command, &mut r),
            Err(InvocationError::Unavailable)
        );
    }
}
fn point(at: usize) -> Selection {
    Selection {
        anchor: BytePosition(at),
        head: BytePosition(at),
        affinity: Affinity::After,
    }
}
fn edit(o: &mut SourceCoordinator, v: &DocumentVersion, from: &str, to: &str, r: &mut Recorder) {
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: v.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: from.len(),
                    expected: from.into(),
                    inserted: to.into(),
                }],
            }],
            &[],
            r,
        )
        .unwrap();
    o.commit(p, r).unwrap();
}
#[test]
fn retiring_attached_view_or_populated_workspace_preserves_document_authored_state_and_peers() {
    for workspace in [false, true] {
        let (mut o, mut r) = owner();
        let a = o.create(b"A", &mut r).unwrap();
        let b = o.create(b"B", &mut r).unwrap();
        let p = o
            .prepare_compound(
                &[],
                &[],
                &[AnnotationMutation::Create {
                    target: SourceTarget {
                        source: a.clone(),
                        target: AttachmentTarget::ContentRange { start: 0, end: 1 },
                        basis: TargetBasis::Before,
                    },
                    body: "note".into(),
                }],
                &mut r,
            )
            .unwrap();
        let annotation = p.created_annotations().next().unwrap().clone();
        o.commit(p, &mut r).unwrap();
        let view = o.create_logical_view(&mut r).unwrap();
        let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
        o.attach_logical_view(&view, stamp, &a, &[point(0)], 0, &mut r)
            .unwrap();
        let group = o.create_workspace(&mut r).unwrap();
        let group = o
            .set_workspace_membership(&group, &a.document, true, &mut r)
            .unwrap()
            .context;
        let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
        let command = if workspace {
            RelationCommandV1::RetireWorkspace {
                expected: group.clone(),
            }
        } else {
            RelationCommandV1::RetireView {
                view: view.clone(),
                expected: stamp,
            }
        };
        let history = o.history_status(&a.document, HistoryDirection::Undo, &mut r);
        let old_source_plan = o
            .prepare(
                &[DocumentEdits {
                    expected: a.clone(),
                    edits: vec![Replacement {
                        start: 0,
                        end: 1,
                        expected: "A".into(),
                        inserted: "a".into(),
                    }],
                }],
                &[],
                &mut r,
            )
            .unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_relation(&id, &command, &mut r).unwrap();
        edit(&mut o, &b, "B", "b", &mut r);
        let g = o.generation();
        o.run(&id, &mut r).unwrap();
        assert_eq!(o.generation(), g + 1);
        let expected = if workspace {
            RelationReceiptV1::WorkspaceRetired(group.workspace.clone())
        } else {
            RelationReceiptV1::ViewRetired(view.clone())
        };
        assert_eq!(
            o.invocation_outcome(&id, &mut r),
            Ok(&InvocationOutcome::RelationCommitted(expected))
        );
        assert_eq!(o.read(&a.document, &mut r).unwrap().version, a);
        assert_eq!(o.annotation(&annotation, &mut r).unwrap().body, "note");
        assert_eq!(
            o.history_status(&a.document, HistoryDirection::Undo, &mut r),
            history
        );
        if workspace {
            assert!(o.workspace(&group.workspace, &mut r).is_err());
            assert_eq!(o.logical_view(&view, &mut r).unwrap().stamp, stamp);
            o.commit(old_source_plan, &mut r).unwrap();
        } else {
            assert!(o.logical_view(&view, &mut r).is_err());
            assert_eq!(
                o.workspace(&group.workspace, &mut r).unwrap().members,
                vec![a.document.clone()]
            );
            assert_eq!(
                o.commit(old_source_plan, &mut r),
                Err(CoordinatorError::Stale)
            );
        }
    }
}
#[test]
fn retirement_revalidates_exact_generations_and_liveness_without_retargeting() {
    for workspace in [false, true] {
        for removed in [false, true] {
            let (mut o, mut r) = owner();
            let a = o.create(b"A", &mut r).unwrap();
            let view = o.create_logical_view(&mut r).unwrap();
            let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
            o.attach_logical_view(&view, stamp, &a, &[point(0)], 0, &mut r)
                .unwrap();
            let snapshot = o.logical_view(&view, &mut r).unwrap();
            let context = snapshot.context().unwrap();
            let group = o.create_workspace(&mut r).unwrap();
            let command = if workspace {
                RelationCommandV1::RetireWorkspace {
                    expected: group.clone(),
                }
            } else {
                RelationCommandV1::RetireView {
                    view: view.clone(),
                    expected: snapshot.stamp,
                }
            };
            let id = o.issue_invocation(&mut r).unwrap();
            o.submit_relation(&id, &command, &mut r).unwrap();
            if workspace {
                if removed {
                    o.retire_workspace(&group, &mut r).unwrap();
                } else {
                    o.set_workspace_membership(&group, &a.document, true, &mut r)
                        .unwrap();
                }
            } else if removed {
                o.retire_logical_view(&view, snapshot.stamp, &mut r)
                    .unwrap();
            } else {
                o.set_logical_selections(&context, &[point(1)], 0, &mut r)
                    .unwrap();
            }
            let g = o.generation();
            o.run(&id, &mut r).unwrap();
            let error = if workspace {
                RelationErrorV1::Workspace(if removed {
                    CoordinatorError::Unavailable
                } else {
                    CoordinatorError::Stale
                })
            } else {
                RelationErrorV1::View(if removed {
                    SelectionError::Unavailable
                } else {
                    SelectionError::Stale
                })
            };
            assert_eq!(
                o.invocation_outcome(&id, &mut r),
                Ok(&InvocationOutcome::RelationRejected(error))
            );
            assert_eq!(o.generation(), g);
            assert!(!o.is_faulted());
            assert_eq!(
                o.submit_relation(&id, &command, &mut r),
                Ok(InvocationState::Finished)
            );
        }
    }
}
#[test]
fn creation_uses_the_service_time_root_and_distinct_ids_after_intervening_creation() {
    for workspace in [false, true] {
        let (mut o, mut r) = owner();
        let a = o.create(b"A", &mut r).unwrap();
        let command = if workspace {
            RelationCommandV1::CreateWorkspace
        } else {
            RelationCommandV1::CreateView
        };
        let first = o.issue_invocation(&mut r).unwrap();
        let second = o.issue_invocation(&mut r).unwrap();
        o.submit_relation(&first, &command, &mut r).unwrap();
        o.submit_relation(&second, &command, &mut r).unwrap();
        let direct = if workspace {
            RelationReceiptV1::WorkspaceCreated(o.create_workspace(&mut r).unwrap().workspace)
        } else {
            RelationReceiptV1::ViewCreated(o.create_logical_view(&mut r).unwrap())
        };
        edit(&mut o, &a, "A", "a", &mut r);
        o.run(&second, &mut r).unwrap();
        let InvocationOutcome::RelationCommitted(second_receipt) =
            o.invocation_outcome(&second, &mut r).unwrap()
        else {
            panic!("not committed")
        };
        let second_receipt = second_receipt.clone();
        o.run(&first, &mut r).unwrap();
        let InvocationOutcome::RelationCommitted(first_receipt) =
            o.invocation_outcome(&first, &mut r).unwrap()
        else {
            panic!("not committed")
        };
        assert_ne!(*first_receipt, second_receipt);
        assert_ne!(*first_receipt, direct);
        assert_ne!(second_receipt, direct);
        assert_eq!(o.read(&a.document, &mut r).unwrap().source.text(), "a");
    }
}
#[test]
fn current_capacity_is_rechecked_but_original_admission_refusal_cannot_refresh() {
    for workspace in [false, true] {
        for admitted in [false, true] {
            let (mut o, mut r) = owner_with_live(1);
            let command = if workspace {
                RelationCommandV1::CreateWorkspace
            } else {
                RelationCommandV1::CreateView
            };
            let existing = if admitted {
                None
            } else {
                Some(if workspace {
                    RelationReceiptV1::WorkspaceCreated(
                        o.create_workspace(&mut r).unwrap().workspace,
                    )
                } else {
                    RelationReceiptV1::ViewCreated(o.create_logical_view(&mut r).unwrap())
                })
            };
            let id = o.issue_invocation(&mut r).unwrap();
            o.submit_relation(&id, &command, &mut r).unwrap();
            if let Some(existing) = existing {
                match existing {
                    RelationReceiptV1::WorkspaceCreated(id) => {
                        let context = o.workspace(&id, &mut r).unwrap().context;
                        o.retire_workspace(&context, &mut r).unwrap();
                    }
                    RelationReceiptV1::ViewCreated(id) => {
                        let stamp = o.logical_view(&id, &mut r).unwrap().stamp;
                        o.retire_logical_view(&id, stamp, &mut r).unwrap();
                    }
                    _ => unreachable!(),
                }
            } else if workspace {
                o.create_workspace(&mut r).unwrap();
            } else {
                o.create_logical_view(&mut r).unwrap();
            }
            let g = o.generation();
            let views = o.logical_view_usage();
            let groups = o.workspace_usage();
            o.run(&id, &mut r).unwrap();
            assert_eq!(
                o.invocation_outcome(&id, &mut r),
                Ok(&InvocationOutcome::RelationRejected(if workspace {
                    RelationErrorV1::Workspace(CoordinatorError::Limit)
                } else {
                    RelationErrorV1::View(SelectionError::Limit)
                }))
            );
            assert_eq!(o.generation(), g);
            assert_eq!(o.logical_view_usage(), views);
            assert_eq!(o.workspace_usage(), groups);
        }
    }
}
#[test]
fn stop_and_intent_mismatch_apply_to_all_four_closed_commands() {
    for kind in 0..4 {
        let (mut o, mut r) = owner();
        let view = o.create_logical_view(&mut r).unwrap();
        let stamp = o.logical_view(&view, &mut r).unwrap().stamp;
        let group = o.create_workspace(&mut r).unwrap();
        let command = match kind {
            0 => RelationCommandV1::CreateView,
            1 => RelationCommandV1::CreateWorkspace,
            2 => RelationCommandV1::RetireView {
                view,
                expected: stamp,
            },
            _ => RelationCommandV1::RetireWorkspace { expected: group },
        };
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_relation(&id, &command, &mut r).unwrap();
        let different = if kind == 0 {
            RelationCommandV1::CreateWorkspace
        } else {
            RelationCommandV1::CreateView
        };
        assert_eq!(
            o.submit_relation(&id, &different, &mut r),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(
            o.retire_invocation(&id, &mut r),
            Err(InvocationError::Pending)
        );
        let g = o.generation();
        let views = o.logical_view_usage();
        let groups = o.workspace_usage();
        o.stop_invocation(&id, &mut r).unwrap();
        o.run(&id, &mut r).unwrap();
        assert_eq!(
            o.invocation_outcome(&id, &mut r),
            Ok(&InvocationOutcome::StoppedBeforePublication)
        );
        assert_eq!(o.generation(), g);
        assert_eq!(o.logical_view_usage(), views);
        assert_eq!(o.workspace_usage(), groups);
    }
}
#[test]
fn disabled_profile_refusals_do_not_adopt_later_activation() {
    for workspace in [false, true] {
        let mut o = SourceCoordinator::with_invocation_limits(
            CoordinatorLimits {
                max_document_incarnations: 8,
                max_participants: 8,
                max_staged_bytes: 1024,
                source: Limits {
                    max_source_bytes: 1024,
                    max_edits: 16,
                    max_edit_payload_bytes: 4096,
                },
            },
            InvocationLimits {
                max_lifetime_issued: 8,
                max_retained: 8,
                max_intent_bytes: 16384,
            },
        );
        let mut r = Recorder::disabled();
        let command = if workspace {
            RelationCommandV1::CreateWorkspace
        } else {
            RelationCommandV1::CreateView
        };
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_relation(&id, &command, &mut r).unwrap();
        if workspace {
            o.configure_workspaces(
                WorkspaceLimits {
                    max_issued: 8,
                    max_live: 8,
                    max_memberships: 8,
                    max_work: 1024,
                },
                &mut r,
            )
            .unwrap();
        } else {
            o.configure_logical_views(
                LogicalViewLimits {
                    max_issued_views: 8,
                    max_live_views: 8,
                    max_total_selections: 8,
                    normalization: SelectionBudget {
                        max_input_selections: 8,
                        max_output_selections: 8,
                        max_work: 1024,
                    },
                    max_mapping_steps: 1024,
                },
                &mut r,
            )
            .unwrap();
        }
        let g = o.generation();
        o.run(&id, &mut r).unwrap();
        assert_eq!(
            o.invocation_outcome(&id, &mut r),
            Ok(&InvocationOutcome::RelationRejected(if workspace {
                RelationErrorV1::Workspace(CoordinatorError::Unavailable)
            } else {
                RelationErrorV1::View(SelectionError::Unavailable)
            }))
        );
        assert_eq!(o.generation(), g);
    }
}
#[test]
fn lifecycle_observations_are_optional_phase_separated_and_not_reexecuted_by_replay() {
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = owner();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        o.create(b"secret", &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_relation(&id, &RelationCommandV1::CreateView, &mut r)
            .unwrap();
        o.run(&id, &mut r).unwrap();
        o.run(&id, &mut r).unwrap();
        assert_eq!(o.logical_view_usage().issued_lifetime, 1);
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        assert!(!format!("{events:?}").contains("secret"));
        if capacity == Some(128) {
            for boundary in [
                Boundary::RelationPreparation,
                Boundary::RelationStaging,
                Boundary::RelationPublication,
            ] {
                assert_eq!(
                    events
                        .iter()
                        .filter(
                            |e| e.boundary == boundary && e.disposition == Disposition::Completed
                        )
                        .count(),
                    1
                );
            }
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
