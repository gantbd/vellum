use super::*;
use vellum_engine::coordinator::*;

#[test]
fn f06_receipt_assembly_preserves_reversal_target_withholding_and_original_causality() {
    let mut o = invocation_owner();
    let mut r = Recorder::disabled();
    o.configure_retention_budget(131072, &mut r).unwrap();
    let c = context();
    let d = o.create(b"A", &mut r).unwrap().document;
    let group = change(&mut o, &[(&d, "A", "B")], &mut r)
        .unwrap()
        .history_group
        .unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let profile = super::observation_support::Profile {
        epoch: 0,
        maximum: 512,
        capabilities: InvocationReceiptCapabilitiesV1 {
            reversal: true,
            ..Default::default()
        },
    };
    profile.bind(&mut o, &id, &c, &mut r);
    o.submit_reversal_with_result_v1(&id, &group, HistoryDirection::Undo, &admission(&c), &mut r)
        .unwrap();
    let mut observer = profile.open(&mut o, &id, &c, &mut r);
    o.run(&id, &mut r).unwrap();
    let native = release(&c);
    let InvocationReceiptV1::Reversal(out) = profile
        .query(
            &o,
            &mut observer,
            InvocationReceiptRequestV1::Reversal(&native),
            &c,
            &mut r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(out.invocation, id);
    assert!(
        matches!(out.decision,ReversalDecisionV1::Applied {publication,target:None} if publication==id)
    );
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "authority".into(),
        engine: "engine".into(),
        recipient: "reader".into(),
        purpose: "reversal-result".into(),
        destination: "host".into(),
        profile: "reversal-result/1".into(),
        continuity: "session".into(),
    }
}
fn admission(c: &SettingsExecutionContextV1) -> TrustedReversalResultAdmissionV1<'_> {
    TrustedReversalResultAdmissionV1 {
        context: c,
        publication_epoch: 0,
        publication_allowed: Some(true),
        context_current: Some(true),
        result_admission_allowed: Some(true),
        max_result_bytes: ReversalResultV1::required_bytes(),
        target_grant: None,
    }
}
fn release(c: &SettingsExecutionContextV1) -> TrustedReversalResultReleaseV1<'_> {
    TrustedReversalResultReleaseV1 {
        context: c,
        publication_epoch: 0,
        publication_allowed: Some(true),
        outcome_readable: Some(true),
        result_context_current: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        max_result_bytes: ReversalResultV1::required_bytes(),
        target_grant: None,
    }
}
#[test]
fn minimal_reversal_decisions_bind_causal_invocation_not_authored_group_or_root_counter() {
    let mut o = invocation_owner();
    let mut r = Recorder::disabled();
    let c = context();
    let a = o.create(b"A", &mut r).unwrap().document;
    let b = o.create(b"B", &mut r).unwrap().document;
    let group = change(&mut o, &[(&a, "A", "a"), (&b, "B", "b")], &mut r)
        .unwrap()
        .history_group
        .unwrap();
    for direction in [HistoryDirection::Undo, HistoryDirection::Redo] {
        let id = o.issue_invocation(&mut r).unwrap();
        o.submit_reversal_with_result_v1(&id, &group, direction, &admission(&c), &mut r)
            .unwrap();
        assert_eq!(
            o.retained_reversal_result_v1(&id, &release(&c), &mut r)
                .err(),
            Some(InvocationError::OutcomeUnavailable)
        );
        o.run(&id, &mut r).unwrap();
        let result = o
            .retained_reversal_result_v1(&id, &release(&c), &mut r)
            .unwrap();
        assert_eq!(result.invocation, id);
        assert_eq!(result.operation, direction);
        assert!(
            matches!(result.decision, ReversalDecisionV1::Applied { publication, target: None } if publication == id)
        );
        let generation = o.generation();
        o.run(&id, &mut r).unwrap();
        assert_eq!(o.generation(), generation);
        o.forget_invocation_outcome(&id, &mut r).unwrap();
        assert_eq!(
            o.retained_reversal_result_v1(&id, &release(&c), &mut r)
                .err(),
            Some(InvocationError::OutcomeUnavailable)
        );
    }
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_reversal_with_result_v1(&id, &group, HistoryDirection::Undo, &admission(&c), &mut r)
        .unwrap();
    o.set_read_only(&a, true, &mut r).unwrap();
    let generation = o.generation();
    o.run(&id, &mut r).unwrap();
    let result = o
        .retained_reversal_result_v1(&id, &release(&c), &mut r)
        .unwrap();
    assert_eq!(result.decision, ReversalDecisionV1::NotPublished);
    assert_eq!(o.generation(), generation);
    // A confirmed nonpublication decision has no target field to disclose;
    // the optional target grant is not consulted for this closed shape.
    let other = o.issue_invocation(&mut r).unwrap();
    let target = HistoryReversalTargetV1::Group(group);
    let mut f = release(&c);
    f.target_grant = Some(TrustedReversalResultGrantV1 {
        invocation: &other,
        authority: "other",
        target: &target,
        disclose_target: true,
    });
    assert_eq!(
        o.retained_reversal_result_v1(&id, &f, &mut r)
            .unwrap()
            .decision,
        ReversalDecisionV1::NotPublished
    );
}

#[test]
fn optional_actual_receipt_timing_and_capture_saturation_preserve_execution_and_stop() {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for capacity in [0, 1, 128] {
        for timed in [false, true] {
            for stop in [false, true] {
                let mut o = invocation_owner();
                let mut setup = Recorder::disabled();
                let c = context();
                let a = o.create(b"A", &mut setup).unwrap().document;
                let group = change(&mut o, &[(&a, "A", "a")], &mut setup)
                    .unwrap()
                    .history_group
                    .unwrap();
                let id = o.issue_invocation(&mut setup).unwrap();
                let generation = o.generation();
                let mut r = Recorder::try_with_capacity(capacity)
                    .unwrap()
                    .with_native_timing(if timed {
                        NativeTimingProfileV1::ProcessMonotonicV1
                    } else {
                        NativeTimingProfileV1::Off
                    });
                o.submit_reversal_with_result_v1(
                    &id,
                    &group,
                    HistoryDirection::Undo,
                    &admission(&c),
                    &mut r,
                )
                .unwrap();
                if stop {
                    o.stop_invocation(&id, &mut r).unwrap();
                }
                o.run(&id, &mut r).unwrap();
                let result = o.retained_reversal_result_v1(&id, &release(&c), &mut r);
                if stop {
                    assert_eq!(result.err(), Some(InvocationError::OutcomeUnavailable));
                    assert_eq!(o.generation(), generation);
                } else {
                    let result = result.unwrap();
                    assert!(matches!(
                        result.decision,
                        ReversalDecisionV1::Applied { target: None, .. }
                    ));
                    assert!(!format!("{result:?}").contains("reader"));
                }
                assert_eq!(
                    o.read(&a, &mut setup).unwrap().source.text(),
                    if stop { "a" } else { "A" }
                );
                let mut observed = Vec::new();
                while let Some(record) = r.pop_front() {
                    if matches!(
                        record.boundary,
                        Boundary::ReversalResultAdmission
                            | Boundary::ReversalResultStaging
                            | Boundary::ReversalResultRelease
                    ) {
                        assert_eq!(record.service_time.is_some(), timed);
                        observed.push(record.boundary);
                    }
                    assert!(!format!("{record:?}").contains("reader"));
                }
                if capacity == 128 {
                    assert_eq!(
                        observed,
                        if stop {
                            vec![
                                Boundary::ReversalResultAdmission,
                                Boundary::ReversalResultRelease,
                            ]
                        } else {
                            vec![
                                Boundary::ReversalResultAdmission,
                                Boundary::ReversalResultStaging,
                                Boundary::ReversalResultRelease,
                            ]
                        }
                    );
                }
                if capacity == 1 {
                    assert!(r.dropped() > 0);
                }
            }
        }
    }
}

#[test]
fn target_disclosure_is_the_intersection_of_original_and_current_exact_grants() {
    for captured in [false, true] {
        let mut o = invocation_owner();
        let mut r = Recorder::disabled();
        let c = context();
        let a = o.create(b"A", &mut r).unwrap().document;
        let group = change(&mut o, &[(&a, "A", "a")], &mut r)
            .unwrap()
            .history_group
            .unwrap();
        let target = HistoryReversalTargetV1::Group(group.clone());
        let id = o.issue_invocation(&mut r).unwrap();
        let grant = TrustedReversalResultGrantV1 {
            invocation: &id,
            authority: &c.authority,
            target: &target,
            disclose_target: captured,
        };
        let mut f = admission(&c);
        f.target_grant = Some(grant);
        o.submit_reversal_with_result_v1(&id, &group, HistoryDirection::Undo, &f, &mut r)
            .unwrap();
        o.run(&id, &mut r).unwrap();
        let generation = o.generation();
        let issued = o.invocation_usage().issued_lifetime;
        for current in [None, Some(false), Some(true)] {
            let mut f = release(&c);
            f.target_grant = current.map(|disclose_target| TrustedReversalResultGrantV1 {
                disclose_target,
                ..grant
            });
            let ReversalDecisionV1::Applied {
                target: disclosed, ..
            } = o
                .retained_reversal_result_v1(&id, &f, &mut r)
                .unwrap()
                .decision
            else {
                panic!("wrong decision")
            };
            assert_eq!(
                disclosed,
                (captured && current == Some(true)).then(|| target.clone())
            );
        }
        let other = o.issue_invocation(&mut r).unwrap();
        let mut f = release(&c);
        f.target_grant = Some(TrustedReversalResultGrantV1 {
            invocation: &other,
            disclose_target: true,
            ..grant
        });
        assert_eq!(
            o.retained_reversal_result_v1(&id, &f, &mut r).err(),
            Some(InvocationError::Unavailable)
        );
        f.target_grant = Some(TrustedReversalResultGrantV1 {
            authority: "other",
            ..grant
        });
        assert_eq!(
            o.retained_reversal_result_v1(&id, &f, &mut r).err(),
            Some(InvocationError::Unavailable)
        );
        let wrong_target = HistoryReversalTargetV1::SourceFrontier {
            document: a.clone(),
            expected_group: group.clone(),
        };
        f.target_grant = Some(TrustedReversalResultGrantV1 {
            target: &wrong_target,
            ..grant
        });
        assert_eq!(
            o.retained_reversal_result_v1(&id, &f, &mut r).err(),
            Some(InvocationError::Unavailable)
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.invocation_usage().issued_lifetime, issued + 1);
    }
}

#[test]
fn original_publication_ceiling_epoch_complete_frame_and_current_release_gates_remain_required() {
    for original_publication in [false, true] {
        let mut o = invocation_owner();
        let mut r = Recorder::disabled();
        let c = context();
        let a = o.create(b"A", &mut r).unwrap().document;
        let group = change(&mut o, &[(&a, "A", "a")], &mut r)
            .unwrap()
            .history_group
            .unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let mut f = admission(&c);
        f.publication_allowed = Some(original_publication);
        o.submit_reversal_with_result_v1(&id, &group, HistoryDirection::Undo, &f, &mut r)
            .unwrap();
        o.run(&id, &mut r).unwrap();
        if !original_publication {
            assert_eq!(
                o.retained_reversal_result_v1(&id, &release(&c), &mut r)
                    .err(),
                Some(InvocationError::Unavailable)
            );
            continue;
        }
        for index in 0..5 {
            for value in [None, Some(false)] {
                let mut f = release(&c);
                match index {
                    0 => f.publication_allowed = value,
                    1 => f.outcome_readable = value,
                    2 => f.result_context_current = value,
                    3 => f.reply_ready = value,
                    _ => f.resources_ready = value,
                }
                assert_eq!(
                    o.retained_reversal_result_v1(&id, &f, &mut r).err(),
                    Some(InvocationError::Unavailable)
                );
            }
        }
        let mut f = release(&c);
        f.publication_epoch = 1;
        assert_eq!(
            o.retained_reversal_result_v1(&id, &f, &mut r).err(),
            Some(InvocationError::Unavailable)
        );
        for index in 0..7 {
            let mut other = c.clone();
            match index {
                0 => other.authority.push('x'),
                1 => other.engine.push('x'),
                2 => other.recipient.push('x'),
                3 => other.purpose.push('x'),
                4 => other.destination.push('x'),
                5 => other.profile.push('x'),
                _ => other.continuity.push('x'),
            }
            assert_eq!(
                o.retained_reversal_result_v1(&id, &release(&other), &mut r)
                    .err(),
                Some(InvocationError::Unavailable)
            );
        }
        assert!(
            o.retained_reversal_result_v1(&id, &release(&c), &mut r)
                .is_ok()
        );
    }
}

#[test]
fn frontier_results_retain_original_target_and_complete_read_conjunction_through_service() {
    for stale in [false, true] {
        let mut o = invocation_owner();
        let mut r = Recorder::disabled();
        let c = context();
        let a = o.create(b"A", &mut r).unwrap().document;
        let dependency = o.create(b"D", &mut r).unwrap();
        let reads = ReadDependencyRequestV3 {
            predicates: vec![ReadDependencyV3::Base(ReadDependencyV2::Base(
                ReadDependencyV1::Source(dependency.clone()),
            ))],
            budget: ReadDependencyBudget {
                max_predicates: 4,
                max_work: 128,
            },
        };
        let group = change(&mut o, &[(&a, "A", "a")], &mut r)
            .unwrap()
            .history_group
            .unwrap();
        let target = HistoryReversalTargetV1::SourceFrontier {
            document: a.clone(),
            expected_group: group.clone(),
        };
        let id = o.issue_invocation(&mut r).unwrap();
        let grant = TrustedReversalResultGrantV1 {
            invocation: &id,
            authority: &c.authority,
            target: &target,
            disclose_target: true,
        };
        let mut f = admission(&c);
        f.target_grant = Some(grant);
        o.submit_reversal_target_with_reads_and_result_v3(
            &id,
            &target,
            HistoryDirection::Undo,
            &reads,
            &f,
            &mut r,
        )
        .unwrap();
        if stale {
            change(&mut o, &[(&dependency.document, "D", "d")], &mut r).unwrap();
        }
        o.run(&id, &mut r).unwrap();
        let mut f = release(&c);
        f.target_grant = Some(grant);
        let receipt = o.retained_reversal_result_v1(&id, &f, &mut r).unwrap();
        if stale {
            assert_eq!(receipt.decision, ReversalDecisionV1::NotPublished);
            assert_eq!(o.read(&a, &mut r).unwrap().source.text(), "a");
        } else {
            assert!(
                matches!(&receipt.decision, ReversalDecisionV1::Applied { target: Some(original), .. } if original == &target)
            );
            o.retire_history_group(&group, &mut r).unwrap();
            assert_eq!(
                o.retained_reversal_result_v1(&id, &f, &mut r).unwrap(),
                receipt
            );
        }
    }
}

#[test]
fn admission_requires_full_result_bound_and_cannot_upgrade_legacy_or_changed_meaning() {
    let mut o = invocation_owner();
    let mut r = Recorder::disabled();
    let c = context();
    let a = o.create(b"A", &mut r).unwrap().document;
    let group = change(&mut o, &[(&a, "A", "a")], &mut r)
        .unwrap()
        .history_group
        .unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let mut f = admission(&c);
    f.max_result_bytes -= 1;
    assert_eq!(
        o.submit_reversal_with_result_v1(&id, &group, HistoryDirection::Undo, &f, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    o.submit_reversal(&id, &group, HistoryDirection::Undo, &mut r)
        .unwrap();
    assert_eq!(
        o.submit_reversal_with_result_v1(
            &id,
            &group,
            HistoryDirection::Undo,
            &admission(&c),
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.retained_reversal_result_v1(&id, &release(&c), &mut r)
            .err(),
        Some(InvocationError::Unavailable)
    );
    o.run(&id, &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_reversal_with_result_v1(&id, &group, HistoryDirection::Redo, &admission(&c), &mut r)
        .unwrap();
    let mut changed = admission(&c);
    changed.publication_epoch = 1;
    assert_eq!(
        o.submit_reversal_with_result_v1(&id, &group, HistoryDirection::Redo, &changed, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.stop_invocation(&id, &mut r).unwrap();
    assert_eq!(
        o.retained_reversal_result_v1(&id, &release(&c), &mut r)
            .err(),
        Some(InvocationError::OutcomeUnavailable)
    );
}

#[test]
fn unknown_admission_facts_and_substituted_target_grants_leave_the_ticket_unbound() {
    let mut o = invocation_owner();
    let mut r = Recorder::disabled();
    let c = context();
    let a = o.create(b"A", &mut r).unwrap().document;
    let group = change(&mut o, &[(&a, "A", "a")], &mut r)
        .unwrap()
        .history_group
        .unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let other = o.issue_invocation(&mut r).unwrap();
    let target = HistoryReversalTargetV1::Group(group.clone());
    let generation = o.generation();
    for index in 0..3 {
        for value in [None, Some(false)] {
            if index == 2 && value == Some(false) {
                continue;
            }
            let mut f = admission(&c);
            match index {
                0 => f.context_current = value,
                1 => f.result_admission_allowed = value,
                _ => f.publication_allowed = value,
            }
            assert_eq!(
                o.submit_reversal_with_result_v1(&id, &group, HistoryDirection::Undo, &f, &mut r),
                Err(InvocationError::Unavailable)
            );
        }
    }
    for grant in [
        TrustedReversalResultGrantV1 {
            invocation: &other,
            authority: &c.authority,
            target: &target,
            disclose_target: true,
        },
        TrustedReversalResultGrantV1 {
            invocation: &id,
            authority: "other",
            target: &target,
            disclose_target: true,
        },
    ] {
        let mut f = admission(&c);
        f.target_grant = Some(grant);
        assert_eq!(
            o.submit_reversal_with_result_v1(&id, &group, HistoryDirection::Undo, &f, &mut r),
            Err(InvocationError::Unavailable)
        );
    }
    assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
    assert_eq!(o.generation(), generation);
}
