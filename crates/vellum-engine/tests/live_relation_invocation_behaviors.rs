use vellum_engine::attachment::Affinity;
use vellum_engine::coordinates::BytePosition;
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::selection::{Selection, SelectionBudget};
use vellum_engine::source::Limits;
#[path = "live_relation_invocation_behaviors/observation.rs"]
mod observation;
fn owner() -> (SourceCoordinator, Recorder) {
    let mut o = SourceCoordinator::with_invocation_limits(
        CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 4096,
            source: Limits {
                max_source_bytes: 4096,
                max_edits: 16,
                max_edit_payload_bytes: 4096,
            },
        },
        InvocationLimits {
            max_lifetime_issued: 32,
            max_retained: 32,
            max_intent_bytes: 32768,
        },
    );
    let mut r = Recorder::disabled();
    o.configure_semantic_root_budget(1048576, &mut r).unwrap();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    o.configure_logical_views(
        LogicalViewLimits {
            max_live_views: 8,
            max_issued_views: 16,
            max_total_selections: 32,
            normalization: SelectionBudget {
                max_input_selections: 8,
                max_output_selections: 8,
                max_work: 4096,
            },
            max_mapping_steps: 4096,
        },
        &mut r,
    )
    .unwrap();
    o.configure_workspaces(
        WorkspaceLimits {
            max_issued: 8,
            max_live: 8,
            max_memberships: 16,
            max_work: 4096,
        },
        &mut r,
    )
    .unwrap();
    (o, r)
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "native-live-relations/1".into(),
        continuity: "c".into(),
    }
}
fn facts(c: &SettingsExecutionContextV1) -> TrustedLiveRelationFactsV1<'_> {
    TrustedLiveRelationFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        interpretation_current: Some(true),
        resources_ready: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        lifecycle_allowed: Some(true),
        coverage_complete: Some(true),
        view_writable: Some(true),
        source_readable: Some(true),
        membership_writable: Some(true),
        workspace_readable: Some(true),
    }
}
fn admission<'a>(f: &'a TrustedLiveRelationFactsV1<'a>) -> LiveRelationAdmissionV1<'a> {
    LiveRelationAdmissionV1 {
        facts: f,
        limits: LiveRelationLimitsV1 {
            max_capture_bytes: 8192,
            max_result_bytes: 512,
        },
        publication_epoch: 7,
        publication_allowed: Some(true),
        result_admission_allowed: Some(true),
        result_identifiers_allowed: Some(true),
    }
}
fn release(c: &SettingsExecutionContextV1) -> TrustedLiveRelationResultV1<'_> {
    TrustedLiveRelationResultV1 {
        context: c,
        publication_epoch: 7,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_readable: Some(true),
        result_context_current: Some(true),
        historical_document_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        max_result_bytes: 512,
    }
}
fn caret(at: usize) -> Selection {
    Selection {
        anchor: BytePosition(at),
        head: BytePosition(at),
        affinity: Affinity::After,
    }
}
#[test]
fn membership_is_one_owned_publication_and_historical_result_not_current_relation_truth() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let d = o.create(b"abc", &mut r).unwrap();
    let w = o.create_workspace(&mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let q = LiveRelationRequestV1 {
        document: d.document.clone(),
        expected_lifecycle: o.lifecycle_state(&d.document, &mut r).unwrap(),
        operation: LiveRelationOperationV1::Membership {
            workspace: w.clone(),
            present: true,
        },
    };
    o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    let held = o.retention_usage();
    assert_eq!(o.run(&id, &mut r), Err(InvocationError::Unavailable));
    o.run_live_relation_v1(&id, &f, &mut r).unwrap();
    let generation = o.generation();
    let result = o
        .live_relation_result_v1(&id, &release(&c), &mut r)
        .unwrap();
    assert!(result.changed);
    assert_eq!(result.document, d.document);
    let now = o.workspace(&w.workspace, &mut r).unwrap().context;
    o.set_workspace_membership(&now, &d.document, false, &mut r)
        .unwrap();
    assert_eq!(
        o.live_relation_result_v1(&id, &release(&c), &mut r)
            .unwrap(),
        result
    );
    o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    o.run_live_relation_v1(&id, &f, &mut r).unwrap();
    assert_eq!(o.generation(), generation + 1);
    assert_eq!(o.retention_usage(), held);
}
fn request(o: &mut SourceCoordinator, r: &mut Recorder, kind: usize) -> LiveRelationRequestV1 {
    let d = o.create(b"abc", r).unwrap();
    let expected_lifecycle = o.lifecycle_state(&d.document, r).unwrap();
    let operation = if kind == 2 {
        LiveRelationOperationV1::Membership {
            workspace: o.create_workspace(r).unwrap(),
            present: true,
        }
    } else {
        let view = o.create_logical_view(r).unwrap();
        let mut expected = o.logical_view(&view, r).unwrap().stamp;
        if kind == 1 {
            expected = o
                .attach_logical_view(&view, expected, &d, &[caret(1)], 0, r)
                .unwrap()
                .stamp;
            LiveRelationOperationV1::Detach { view, expected }
        } else {
            LiveRelationOperationV1::Attach {
                view,
                expected,
                source: d.clone(),
                selections: vec![caret(1), caret(1)],
                primary: 1,
            }
        }
    };
    LiveRelationRequestV1 {
        document: d.document,
        expected_lifecycle,
        operation,
    }
}
fn control(c: &SettingsExecutionContextV1) -> InvocationObservationPolicyControlV1<'_> {
    InvocationObservationPolicyControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
#[test]
fn attach_and_exact_detach_keep_logical_selection_and_history_in_the_engine_not_ui() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let mut q = request(&mut o, &mut r, 0);
    let original = q.clone();
    let source = o.read(&q.document, &mut r).unwrap().version;
    let lifecycle = o.lifecycle_state(&q.document, &mut r).unwrap();
    let history = o.history_usage();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    if let LiveRelationOperationV1::Attach { selections, .. } = &mut q.operation {
        selections[0] = caret(3)
    }
    assert_eq!(
        o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.run_live_relation_v1(&id, &f, &mut r).unwrap();
    let LiveRelationOperationV1::Attach { view, expected, .. } = &original.operation else {
        panic!()
    };
    let snap = o.logical_view(view, &mut r).unwrap();
    assert_eq!(snap.source, Some(source.clone()));
    assert_eq!(snap.selections.selections(), &[caret(1)]);
    assert_eq!(
        snap.stamp.attachment_generation,
        expected.attachment_generation + 1
    );
    assert_eq!(
        snap.stamp.selection_generation,
        expected.selection_generation + 1
    );
    assert_eq!(
        snap.stamp.viewport_generation,
        expected.viewport_generation + 1
    );
    let other = o.create(b"peer", &mut r).unwrap();
    let mut detach = LiveRelationRequestV1 {
        document: other.document.clone(),
        expected_lifecycle: o.lifecycle_state(&other.document, &mut r).unwrap(),
        operation: LiveRelationOperationV1::Detach {
            view: view.clone(),
            expected: snap.stamp,
        },
    };
    let ticket = o.issue_invocation(&mut r).unwrap();
    let held = o.retention_usage();
    assert!(
        o.submit_live_relation_v1(&ticket, &detach, &admission(&f), &mut r)
            .is_err()
    );
    assert_eq!(o.retention_usage(), held);
    detach.document = source.document.clone();
    detach.expected_lifecycle = lifecycle;
    o.submit_live_relation_v1(&ticket, &detach, &admission(&f), &mut r)
        .unwrap();
    o.run_live_relation_v1(&ticket, &f, &mut r).unwrap();
    let now = o.logical_view(view, &mut r).unwrap();
    assert!(now.source.is_none());
    assert!(now.selections.selections().is_empty());
    assert_eq!(
        now.stamp.attachment_generation,
        snap.stamp.attachment_generation + 1
    );
    assert_eq!(
        o.lifecycle_state(&source.document, &mut r).unwrap(),
        lifecycle
    );
    assert_eq!(o.read(&source.document, &mut r).unwrap().version, source);
    assert_eq!(o.history_usage(), history);
    let out = o
        .live_relation_result_v1(&ticket, &release(&c), &mut r)
        .unwrap();
    assert_eq!(out.operation, LiveRelationKindV1::Detach);
    assert!(out.changed);
    let fresh = o.issue_invocation(&mut r).unwrap();
    detach.operation = LiveRelationOperationV1::Detach {
        view: view.clone(),
        expected: now.stamp,
    };
    assert!(
        o.submit_live_relation_v1(&fresh, &detach, &admission(&f), &mut r)
            .is_err()
    );
}
#[test]
fn stale_membership_is_spent_even_if_desired_state_matches_and_unrelated_source_edits_do_not_stale_it()
 {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let mut q = request(&mut o, &mut r, 2);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    let LiveRelationOperationV1::Membership { workspace, present } = &mut q.operation else {
        panic!()
    };
    let old = workspace.clone();
    *workspace = o
        .set_workspace_membership(workspace, &q.document, *present, &mut r)
        .unwrap()
        .context;
    o.run_live_relation_v1(&id, &f, &mut r).unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::LiveRelationRejected(_)
    ));
    assert_eq!(
        o.live_relation_result_v1(&id, &release(&c), &mut r),
        Err(InvocationError::OutcomeUnavailable)
    );
    assert_eq!(
        o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    let next = o.issue_invocation(&mut r).unwrap();
    o.submit_live_relation_v1(&next, &q, &admission(&f), &mut r)
        .unwrap();
    let generation = o.generation();
    o.run_live_relation_v1(&next, &f, &mut r).unwrap();
    assert!(
        !o.live_relation_result_v1(&next, &release(&c), &mut r)
            .unwrap()
            .changed
    );
    assert_eq!(o.generation(), generation);
    let mut remove = q.clone();
    remove.operation = LiveRelationOperationV1::Membership {
        workspace: match &q.operation {
            LiveRelationOperationV1::Membership { workspace, .. } => workspace.clone(),
            _ => panic!(),
        },
        present: false,
    };
    let last = o.issue_invocation(&mut r).unwrap();
    o.submit_live_relation_v1(&last, &remove, &admission(&f), &mut r)
        .unwrap();
    let _ = o.create(b"unrelated", &mut r).unwrap();
    o.set_read_only(&q.document, true, &mut r).unwrap();
    o.run_live_relation_v1(&last, &f, &mut r).unwrap();
    assert!(
        o.live_relation_result_v1(&last, &release(&c), &mut r)
            .unwrap()
            .changed
    );
    assert_ne!(o.workspace(&old.workspace, &mut r).unwrap().context, old);
    assert_eq!(
        o.lifecycle_state(&q.document, &mut r).unwrap(),
        q.expected_lifecycle
    );
}
fn deny(f: &mut TrustedLiveRelationFactsV1<'_>, index: usize, value: Option<bool>) {
    match index {
        0 => f.authorized = value,
        1 => f.scope = value,
        2 => f.continuous = value,
        3 => f.live = value,
        4 => f.not_stopped = value,
        5 => f.interpretation_current = value,
        6 => f.resources_ready = value,
        7 => f.companions_ready = value,
        8 => f.versions_available = value,
        9 => f.reply_ready = value,
        10 => f.lifecycle_allowed = value,
        11 => f.coverage_complete = value,
        12 => f.view_writable = value,
        13 => f.source_readable = value,
        14 => f.membership_writable = value,
        _ => f.workspace_readable = value,
    }
}
#[test]
fn every_applicable_admission_and_fresh_service_gate_is_strict_and_other_family_rights_are_not_required()
 {
    for kind in 0..3 {
        for index in 0..16 {
            let applicable = index < 12
                || match kind {
                    0 => index == 12 || index == 13,
                    1 => index == 12,
                    _ => index >= 14,
                };
            for value in [None, Some(false)] {
                let (mut o, mut r) = owner();
                let c = context();
                let q = request(&mut o, &mut r, kind);
                let id = o.issue_invocation(&mut r).unwrap();
                let mut f = facts(&c);
                deny(&mut f, index, value);
                let held = o.retention_usage();
                let result = o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r);
                if applicable {
                    assert_eq!(result, Err(InvocationError::Unavailable));
                    assert_eq!(o.retention_usage(), held);
                    assert_eq!(
                        o.invocation_state(&id, &mut r).unwrap(),
                        InvocationState::Issued
                    );
                    o.submit_live_relation_v1(&id, &q, &admission(&facts(&c)), &mut r)
                        .unwrap();
                    let generation = o.generation();
                    assert_eq!(
                        o.run_live_relation_v1(&id, &f, &mut r),
                        Err(InvocationError::Unavailable)
                    );
                    assert_eq!(o.generation(), generation);
                    assert_eq!(
                        o.invocation_state(&id, &mut r).unwrap(),
                        InvocationState::Pending
                    );
                    o.run_live_relation_v1(&id, &facts(&c), &mut r).unwrap();
                } else {
                    result.unwrap();
                    o.run_live_relation_v1(&id, &f, &mut r).unwrap();
                }
                assert!(
                    o.live_relation_result_v1(&id, &release(&c), &mut r)
                        .unwrap()
                        .changed
                );
            }
        }
    }
}
#[test]
fn stopping_forgetting_and_protected_retirement_do_not_reopen_work_or_refund_other_holds() {
    for kind in 0..3 {
        let (mut o, mut r) = owner();
        let c = context();
        let f = facts(&c);
        let q = request(&mut o, &mut r, kind);
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
            .unwrap();
        let second = o.issue_invocation(&mut r).unwrap();
        o.submit_live_relation_v1(&second, &q, &admission(&f), &mut r)
            .unwrap();
        let held = o.retention_usage().reserved;
        let generation = o.generation();
        o.request_invocation_stop(
            &id,
            &TrustedInvocationControlV1 {
                context: &c,
                owner_control_allowed: Some(true),
                control_context_current: Some(true),
                control_scope_valid: Some(true),
                protected_control_ready: Some(true),
            },
            &mut r,
        )
        .unwrap();
        let mut denied = facts(&c);
        denied.authorized = Some(false);
        o.run_live_relation_v1(&id, &denied, &mut r).unwrap();
        assert_eq!(o.generation(), generation);
        assert!(matches!(
            o.invocation_outcome(&id, &mut r).unwrap(),
            InvocationOutcome::StoppedBeforePublication
        ));
        assert_eq!(
            o.live_relation_result_v1(&id, &release(&c), &mut r),
            Err(InvocationError::OutcomeUnavailable)
        );
        o.forget_invocation_outcome(&id, &mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, held);
        assert_eq!(
            o.retire_invocation(&id, &mut r),
            Err(InvocationError::Unavailable)
        );
        o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
            .unwrap();
        o.run_live_relation_v1(&id, &f, &mut r).unwrap();
        assert_eq!(o.generation(), generation);
        for bad in 0..6 {
            let mut ctl = control(&c);
            match bad {
                0 => ctl.owner_control_allowed = None,
                1 => ctl.owner_control_allowed = Some(false),
                2 => ctl.retention_released = None,
                3 => ctl.retention_released = Some(false),
                4 => ctl.delivery_drained = None,
                _ => ctl.delivery_drained = Some(false),
            }
            assert_eq!(
                o.retire_live_relation_v1(&id, &ctl, &mut r),
                Err(InvocationError::Unavailable)
            );
        }
        o.retire_live_relation_v1(&id, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, held / 2);
        assert_eq!(
            o.retire_live_relation_v1(&id, &control(&c), &mut r),
            Err(InvocationError::Unavailable)
        );
        o.retire_invocation(&id, &mut r).unwrap();
        assert_eq!(o.invocation_usage().issued_lifetime, 2);
        o.stop_invocation(&second, &mut r).unwrap();
        o.retire_live_relation_v1(&second, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}

#[test]
fn result_rights_are_independent_original_ceilings_and_current_denials_never_undo_publication() {
    for kind in 0..3 {
        for bits in 0..8 {
            let (mut o, mut r) = owner();
            let c = context();
            let f = facts(&c);
            let q = request(&mut o, &mut r, kind);
            let id = o.issue_invocation(&mut r).unwrap();
            let mut a = admission(&f);
            a.publication_allowed = Some(bits & 1 != 0);
            a.result_admission_allowed = Some(bits & 2 != 0);
            a.result_identifiers_allowed = Some(bits & 4 != 0);
            o.submit_live_relation_v1(&id, &q, &a, &mut r).unwrap();
            o.run_live_relation_v1(&id, &f, &mut r).unwrap();
            assert_eq!(
                o.live_relation_result_v1(&id, &release(&c), &mut r).is_ok(),
                bits == 7
            );
        }
    }
    let (mut o, mut r) = owner();
    let c = context();
    let mut wrong = c.clone();
    wrong.recipient = "foreign".into();
    let f = facts(&c);
    let q = request(&mut o, &mut r, 0);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    o.run_live_relation_v1(&id, &f, &mut r).unwrap();
    let generation = o.generation();
    let held = o.retention_usage();
    for bad in 0..17 {
        let mut n = release(&c);
        match bad {
            0 => n.publication_allowed = None,
            1 => n.publication_allowed = Some(false),
            2 => n.outcome_readable = None,
            3 => n.outcome_readable = Some(false),
            4 => n.result_readable = None,
            5 => n.result_readable = Some(false),
            6 => n.result_context_current = None,
            7 => n.result_context_current = Some(false),
            8 => n.historical_document_readable = None,
            9 => n.historical_document_readable = Some(false),
            10 => n.reply_ready = None,
            11 => n.reply_ready = Some(false),
            12 => n.resources_ready = None,
            13 => n.resources_ready = Some(false),
            14 => n.context = &wrong,
            15 => n.publication_epoch = 8,
            _ => n.max_result_bytes = 0,
        }
        assert!(o.live_relation_result_v1(&id, &n, &mut r).is_err());
        assert!(o.live_relation_result_v1(&id, &release(&c), &mut r).is_ok());
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
    }
}

#[test]
fn live_relation_capture_modes_preserve_results_with_bounded_optional_service_timing() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for kind in 0..3 {
        for mode in 0..4 {
            let (mut o, mut setup) = owner();
            let c = context();
            let f = facts(&c);
            let q = request(&mut o, &mut setup, kind);
            let id = o.issue_invocation(&mut setup).unwrap();
            let mut r = if mode == 0 {
                Recorder::disabled()
            } else {
                Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
            };
            if mode >= 2 {
                r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1)
            }
            if mode == 3 {
                r.record(Observation {
                    boundary: Boundary::Control,
                    disposition: Disposition::Completed,
                    service_time: None,
                });
            }
            o.submit_live_relation_v1(&id, &q, &admission(&f), &mut r)
                .unwrap();
            o.run_live_relation_v1(&id, &f, &mut r).unwrap();
            assert!(
                o.live_relation_result_v1(&id, &release(&c), &mut r)
                    .unwrap()
                    .changed
            );
            o.retire_live_relation_v1(&id, &control(&c), &mut r)
                .unwrap();
            assert_eq!(o.retention_usage().reserved, 0);
            let mut counts = [0; 4];
            while let Some(e) = r.pop_front() {
                let i = match e.boundary {
                    Boundary::LiveRelationAdmission => Some(0),
                    Boundary::LiveRelationService => Some(1),
                    Boundary::LiveRelationResultRelease => Some(2),
                    Boundary::LiveRelationRetirement => Some(3),
                    _ => None,
                };
                if let Some(i) = i {
                    counts[i] += 1;
                    assert_eq!(e.disposition, Disposition::Completed);
                    assert_eq!(e.service_time.is_some(), mode == 2)
                }
                assert!(!format!("{e:?}").contains("PRIVATE"));
            }
            if mode == 1 || mode == 2 {
                assert_eq!(counts, [1, 1, 1, 1])
            } else {
                assert_eq!(counts, [0, 0, 0, 0])
            }
        }
    }
}
