use super::*;
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "authority".into(),
        engine: "engine".into(),
        recipient: "reader".into(),
        purpose: "receipt".into(),
        destination: "host".into(),
        profile: "native-relation-lifecycle/1".into(),
        continuity: "one-session".into(),
    }
}
fn admission(c: &SettingsExecutionContextV1) -> TrustedRelationResultAdmissionV1<'_> {
    TrustedRelationResultAdmissionV1 {
        context: c,
        max_result_bytes: RelationResultV1::required_bytes(),
        result_admission_allowed: Some(true),
        context_current: Some(true),
        relation_result_identifiers_allowed: Some(true),
    }
}
fn release(c: &SettingsExecutionContextV1) -> TrustedRelationResultReleaseV1<'_> {
    TrustedRelationResultReleaseV1 {
        context: c,
        max_result_bytes: RelationResultV1::required_bytes(),
        outcome_readable: Some(true),
        relation_result_readable: Some(true),
        relation_result_context_current: Some(true),
        historical_target_readable: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
    }
}

#[test]
fn captured_relation_receipts_remain_historical_and_forget_never_reopens_execution() {
    for workspace in [false, true] {
        let (mut o, mut r) = owner();
        let c = context();
        let id = o.issue_invocation(&mut r).unwrap();
        let command = if workspace {
            RelationCommandV1::CreateWorkspace
        } else {
            RelationCommandV1::CreateView
        };
        o.submit_relation_with_result_v1(&id, &command, &admission(&c), &mut r)
            .unwrap();
        assert_eq!(
            o.retained_relation_result_v1(&id, &release(&c), &mut r)
                .err(),
            Some(InvocationError::OutcomeUnavailable)
        );
        o.run(&id, &mut r).unwrap();
        let delivered = o
            .retained_relation_result_v1(&id, &release(&c), &mut r)
            .unwrap();
        assert_eq!(delivered.invocation, id);
        let retire = match &delivered.receipt {
            RelationReceiptV1::ViewCreated(view) => RelationCommandV1::RetireView {
                view: view.clone(),
                expected: o.logical_view(view, &mut r).unwrap().stamp,
            },
            RelationReceiptV1::WorkspaceCreated(workspace) => RelationCommandV1::RetireWorkspace {
                expected: o.workspace(workspace, &mut r).unwrap().context,
            },
            _ => panic!("wrong creation receipt"),
        };
        let retire_id = o.issue_invocation(&mut r).unwrap();
        o.submit_relation_with_result_v1(&retire_id, &retire, &admission(&c), &mut r)
            .unwrap();
        o.run(&retire_id, &mut r).unwrap();
        let retired = o
            .retained_relation_result_v1(&retire_id, &release(&c), &mut r)
            .unwrap();
        assert!(matches!(
            retired.receipt,
            RelationReceiptV1::ViewRetired(_) | RelationReceiptV1::WorkspaceRetired(_)
        ));
        assert_eq!(
            o.retained_relation_result_v1(&id, &release(&c), &mut r)
                .unwrap()
                .receipt,
            delivered.receipt
        );
        let generation = o.generation();
        o.forget_invocation_outcome(&id, &mut r).unwrap();
        assert_eq!(
            o.retained_relation_result_v1(&id, &release(&c), &mut r)
                .err(),
            Some(InvocationError::OutcomeUnavailable)
        );
        assert_eq!(
            o.submit_relation_with_result_v1(&id, &command, &admission(&c), &mut r),
            Ok(InvocationState::Finished)
        );
        o.run(&id, &mut r).unwrap();
        assert_eq!(o.generation(), generation);
    }
}

#[test]
fn every_release_gate_and_complete_original_context_remains_independent() {
    let (mut o, mut r) = owner();
    let c = context();
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_relation_with_result_v1(&id, &RelationCommandV1::CreateView, &admission(&c), &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    let generation = o.generation();
    for value in [None, Some(false)] {
        for index in 0..6 {
            let mut facts = release(&c);
            match index {
                0 => facts.outcome_readable = value,
                1 => facts.relation_result_readable = value,
                2 => facts.relation_result_context_current = value,
                3 => facts.historical_target_readable = value,
                4 => facts.reply_ready = value,
                _ => facts.resources_ready = value,
            }
            assert_eq!(
                o.retained_relation_result_v1(&id, &facts, &mut r).err(),
                Some(InvocationError::Unavailable)
            );
            assert!(
                o.retained_relation_result_v1(&id, &release(&c), &mut r)
                    .is_ok()
            );
        }
    }
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
            o.retained_relation_result_v1(&id, &release(&other), &mut r)
                .err(),
            Some(InvocationError::Unavailable)
        );
    }
    let mut short = release(&c);
    short.max_result_bytes -= 1;
    assert_eq!(
        o.retained_relation_result_v1(&id, &short, &mut r).err(),
        Some(InvocationError::Limit)
    );
    assert!(
        o.retained_relation_result_v1(&id, &release(&c), &mut r)
            .is_ok()
    );
    assert_eq!(o.generation(), generation);
}

#[test]
fn false_admission_ceiling_and_legacy_execution_cannot_gain_late_identifier_disclosure() {
    for legacy in [false, true] {
        let (mut o, mut r) = owner();
        let c = context();
        let id = o.issue_invocation(&mut r).unwrap();
        let command = RelationCommandV1::CreateWorkspace;
        let mut denied = admission(&c);
        denied.relation_result_identifiers_allowed = Some(false);
        if legacy {
            o.submit_relation(&id, &command, &mut r).unwrap();
        } else {
            o.submit_relation_with_result_v1(&id, &command, &denied, &mut r)
                .unwrap();
        }
        o.run(&id, &mut r).unwrap();
        let generation = o.generation();
        assert_eq!(
            o.retained_relation_result_v1(&id, &release(&c), &mut r)
                .err(),
            Some(InvocationError::Unavailable)
        );
        assert_eq!(
            o.submit_relation_with_result_v1(&id, &command, &admission(&c), &mut r)
                .err(),
            Some(if legacy {
                InvocationError::Unavailable
            } else {
                InvocationError::IntentMismatch
            })
        );
        assert_eq!(o.workspace_usage().issued_lifetime, 1);
        assert_eq!(o.generation(), generation);
    }
}

#[test]
fn missing_admission_grants_and_short_result_limit_refuse_before_job_binding() {
    let (mut o, mut r) = owner();
    let c = context();
    let id = o.issue_invocation(&mut r).unwrap();
    let generation = o.generation();
    for index in 0..3 {
        for value in [None, Some(false)] {
            if index == 2 && value == Some(false) {
                continue;
            }
            let mut f = admission(&c);
            match index {
                0 => f.result_admission_allowed = value,
                1 => f.context_current = value,
                _ => f.relation_result_identifiers_allowed = value,
            }
            assert_eq!(
                o.submit_relation_with_result_v1(&id, &RelationCommandV1::CreateView, &f, &mut r),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        }
    }
    let mut f = admission(&c);
    f.max_result_bytes -= 1;
    assert_eq!(
        o.submit_relation_with_result_v1(&id, &RelationCommandV1::CreateView, &f, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.logical_view_usage().issued_lifetime, 0);
}

#[test]
fn receipt_capture_is_optional_timed_redacted_and_does_not_change_stop_or_execution() {
    use vellum_engine::instrumentation::NativeTimingProfileV1;
    for capacity in [0, 1, 64] {
        for timed in [false, true] {
            for stop in [false, true] {
                let (mut o, _) = owner();
                let c = context();
                let mut setup = Recorder::disabled();
                let id = o.issue_invocation(&mut setup).unwrap();
                let mut r = Recorder::try_with_capacity(capacity)
                    .unwrap()
                    .with_native_timing(if timed {
                        NativeTimingProfileV1::ProcessMonotonicV1
                    } else {
                        NativeTimingProfileV1::Off
                    });
                let before = o.generation();
                o.submit_relation_with_result_v1(
                    &id,
                    &RelationCommandV1::CreateWorkspace,
                    &admission(&c),
                    &mut r,
                )
                .unwrap();
                if stop {
                    o.stop_invocation(&id, &mut r).unwrap();
                }
                o.run(&id, &mut r).unwrap();
                let result = o.retained_relation_result_v1(&id, &release(&c), &mut r);
                if stop {
                    assert_eq!(result.err(), Some(InvocationError::OutcomeUnavailable));
                    assert_eq!(o.generation(), before);
                } else {
                    assert!(!format!("{:?}", result.unwrap()).contains("reader"));
                }
                assert_eq!(o.workspace_usage().issued_lifetime, usize::from(!stop));
                let mut seen = 0;
                while let Some(record) = r.pop_front() {
                    if matches!(
                        record.boundary,
                        Boundary::RelationResultAdmission | Boundary::RelationResultRelease
                    ) {
                        assert_eq!(record.service_time.is_some(), timed);
                        seen += 1;
                    }
                    assert!(!format!("{record:?}").contains("reader"));
                }
                if capacity == 64 {
                    assert_eq!(seen, 2);
                }
                if capacity == 1 {
                    assert!(r.dropped() > 0);
                }
            }
        }
    }
}
