use super::{intent, owner};
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
#[path = "observation_receipts.rs"]
mod receipts;
#[path = "observation_wait.rs"]
mod wait;

#[test]
fn protected_invocation_status_observes_without_execution_and_keeps_its_reference_after_forgetting()
{
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.configure_retention_budget(32768, &mut r).unwrap();
    let d = o.create(b"A", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let p = policy(&c);
    o.bind_invocation_observation_policy_v1(
        &id,
        &c,
        &p,
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &facts(&c),
        &mut r,
    )
    .unwrap();
    o.submit(&id, &intent(d.clone(), "A", "B"), &mut r).unwrap();
    let mut observer = o
        .open_invocation_observation_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
        .unwrap();
    let pending = o
        .minimal_invocation_control_status_v1(&mut observer, &p, &release(&c), &mut r)
        .unwrap();
    assert_eq!(pending.phase, InvocationObservationPhaseV1::Pending);
    assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "A");
    assert_eq!(
        pending,
        o.minimal_invocation_control_status_v1(&mut observer, &p, &release(&c), &mut r)
            .unwrap()
    );
    o.run(&id, &mut r).unwrap();
    let terminal = o
        .minimal_invocation_control_status_v1(&mut observer, &p, &release(&c), &mut r)
        .unwrap();
    assert_eq!(terminal.phase, InvocationObservationPhaseV1::Terminal);
    assert_ne!(pending.reference, terminal.reference);
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        terminal,
        o.minimal_invocation_control_status_v1(&mut observer, &p, &release(&c), &mut r)
            .unwrap()
    );
    assert_eq!(
        o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r),
        Err(InvocationError::Unavailable)
    );
    o.retire_invocation_observation_v1(&mut observer, &control(&c), &mut r)
        .unwrap();
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut observer, &p, &release(&c), &mut r),
        Err(InvocationError::Unavailable)
    );
    o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
        .unwrap();
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "B");
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "native-observation/1".into(),
        continuity: "c".into(),
    }
}
fn policy(c: &SettingsExecutionContextV1) -> InvocationObservationPolicyV1 {
    InvocationObservationPolicyV1 {
        recipient: c.recipient.clone(),
        epoch: 7,
        grant: true,
        publication: true,
        control: true,
        effect: true,
        reasons: true,
        details: false,
        recovery: true,
    }
}
fn facts(c: &SettingsExecutionContextV1) -> TrustedInvocationPolicyFactsV1<'_> {
    TrustedInvocationPolicyFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        resources_ready: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        preparation_reserved: Some(true),
        registry_ready: Some(true),
        retention_ready: Some(true),
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
fn release(c: &SettingsExecutionContextV1) -> TrustedInvocationObservationReleaseV1<'_> {
    TrustedInvocationObservationReleaseV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        control_ready: Some(true),
    }
}
fn limits() -> InvocationObservationLimitsV1 {
    InvocationObservationLimitsV1 {
        max_waiters: 0,
        max_issued: 0,
        max_wait_steps: 0,
        max_context_bytes: 2048,
        max_wait_bytes: 256,
        max_reference_bytes: 256,
        max_reply_bytes: 2048,
        max_control_reply_bytes: 256,
        max_held_bytes: 8192,
    }
}

fn pending() -> (
    SourceCoordinator,
    InvocationId,
    SettingsExecutionContextV1,
    InvocationObservationPolicyV1,
    Recorder,
) {
    pending_with_budget(65536)
}
fn pending_with_budget(
    budget: usize,
) -> (
    SourceCoordinator,
    InvocationId,
    SettingsExecutionContextV1,
    InvocationObservationPolicyV1,
    Recorder,
) {
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.configure_retention_budget(budget, &mut r).unwrap();
    let d = o.create(b"A", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let p = policy(&c);
    o.bind_invocation_observation_policy_v1(
        &id,
        &c,
        &p,
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &facts(&c),
        &mut r,
    )
    .unwrap();
    o.submit(&id, &intent(d, "A", "B"), &mut r).unwrap();
    (o, id, c, p, r)
}

#[test]
fn ordinary_observation_tracks_only_its_permitted_projection_and_never_reexecutes() {
    let (mut o, id, c, p, mut r) = pending();
    let mut l = limits();
    l.max_issued = 4;
    l.max_held_bytes = 16384;
    let mut v = o
        .open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r)
        .unwrap();
    let f = release(&c);
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let before = o
        .query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::Status,
            &p,
            &ordinary,
            &mut r,
        )
        .unwrap();
    assert_eq!(before.phase, Some(InvocationObservationPhaseV1::Pending));
    assert!(before.outcome.is_none());
    assert_eq!(
        before,
        o.query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::Status,
            &p,
            &ordinary,
            &mut r
        )
        .unwrap()
    );
    assert_eq!(
        o.invocation_state(&id, &mut r),
        Ok(InvocationState::Pending)
    );
    o.run(&id, &mut r).unwrap();
    let after = o
        .query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::RetainedOutcome,
            &p,
            &ordinary,
            &mut r,
        )
        .unwrap();
    assert_eq!(after.phase, None);
    assert_eq!(
        after.outcome.unwrap().publication,
        Some(InvocationPublicationV1::Applied)
    );
    assert_ne!(before.reference, after.reference);
    let status = o
        .query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::Status,
            &p,
            &ordinary,
            &mut r,
        )
        .unwrap();
    assert_eq!(status.reference, after.reference);
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        o.query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::RetainedOutcome,
            &p,
            &ordinary,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    let forgotten = o
        .query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::Status,
            &p,
            &ordinary,
            &mut r,
        )
        .unwrap();
    assert!(forgotten.outcome.is_none());
    assert_ne!(forgotten.reference, status.reference);
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut v, &p, &f, &mut r)
            .unwrap()
            .phase,
        InvocationObservationPhaseV1::Terminal
    );
    o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
}

#[test]
fn ordinary_local_outcomes_distinguish_actual_apply_noop_refusal_and_stop_without_raw_causes() {
    for case in 0..4 {
        let mut o = owner();
        let mut r = Recorder::disabled();
        o.configure_retention_budget(32768, &mut r).unwrap();
        let d = o.create(b"A", &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let c = context();
        let p = policy(&c);
        o.bind_invocation_observation_policy_v1(
            &id,
            &c,
            &p,
            InvocationObservationPolicyLimitsV1 {
                max_context_bytes: 2048,
                max_result_bytes: 512,
            },
            &facts(&c),
            &mut r,
        )
        .unwrap();
        o.submit(
            &id,
            &intent(d.clone(), "A", if case == 1 { "A" } else { "B" }),
            &mut r,
        )
        .unwrap();
        let mut l = limits();
        l.max_issued = 1;
        l.max_held_bytes = 16384;
        let mut v = o
            .open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r)
            .unwrap();
        if case == 2 {
            o.set_read_only(&d.document, true, &mut r).unwrap();
        }
        if case == 3 {
            o.stop_invocation(&id, &mut r).unwrap();
        } else {
            o.run(&id, &mut r).unwrap();
        }
        let f = release(&c);
        let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
            current: &f,
            ordinary_ready: Some(true),
            resources_ready: Some(true),
        };
        let out = o
            .query_invocation_observation_v1(
                &mut v,
                InvocationObservationQueryV1::RetainedOutcome,
                &p,
                &ordinary,
                &mut r,
            )
            .unwrap()
            .outcome
            .unwrap();
        assert_eq!(
            out.publication,
            Some(match case {
                0 => InvocationPublicationV1::Applied,
                1 => InvocationPublicationV1::Unchanged,
                _ => InvocationPublicationV1::NotApplied,
            })
        );
        assert_eq!(
            out.control,
            Some(match case {
                0 | 1 => InvocationControlDispositionV1::Done,
                2 => InvocationControlDispositionV1::Rejected,
                _ => InvocationControlDispositionV1::Stopped,
            })
        );
        assert_eq!(out.effect, Some(InvocationLocalEffectV1::None));
        assert_eq!(out.reason, InvocationOutcomeReasonV1::Unavailable);
        assert_eq!(out.recovery, Some(InvocationRecoveryV1::Inspect));
        assert_eq!(
            o.read(&d.document, &mut r).unwrap().source.text(),
            if case == 0 { "B" } else { "A" }
        );
        o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
            .unwrap();
    }
}

#[test]
fn ordinary_exhaustion_and_unrelated_publication_do_not_consume_protected_control() {
    let (mut o, id, c, p, mut r) = pending();
    let mut l = limits();
    l.max_issued = 1;
    l.max_held_bytes = 16384;
    let mut v = o
        .open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r)
        .unwrap();
    let f = release(&c);
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    let pending = o
        .query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::Status,
            &p,
            &ordinary,
            &mut r,
        )
        .unwrap();
    o.create(b"unrelated publication", &mut r).unwrap();
    assert_eq!(
        pending,
        o.query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::Status,
            &p,
            &ordinary,
            &mut r
        )
        .unwrap()
    );
    o.run(&id, &mut r).unwrap();
    assert_eq!(
        o.query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::Status,
            &p,
            &ordinary,
            &mut r
        ),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut v, &p, &f, &mut r)
            .unwrap()
            .phase,
        InvocationObservationPhaseV1::Terminal
    );
    o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 2048);
}

#[test]
fn ordinary_readiness_and_disclosure_are_separate_from_protected_control_readiness() {
    // Native result and complete frame bounds are checked independently.
    use std::mem::size_of;
    for delta in [-1isize, 0, 1] {
        let mut o = owner();
        let mut r = Recorder::disabled();
        o.configure_retention_budget(32768, &mut r).unwrap();
        let d = o.create(b"A", &mut r).unwrap();
        let id = o.issue_invocation(&mut r).unwrap();
        let c = context();
        let p = policy(&c);
        let result_bytes = size_of::<InvocationOutcomeProjectionV1>()
            .checked_add_signed(delta)
            .unwrap();
        o.bind_invocation_observation_policy_v1(
            &id,
            &c,
            &p,
            InvocationObservationPolicyLimitsV1 {
                max_context_bytes: 2048,
                max_result_bytes: result_bytes,
            },
            &facts(&c),
            &mut r,
        )
        .unwrap();
        o.submit(&id, &intent(d, "A", "B"), &mut r).unwrap();
        let v = o.open_invocation_observation_v1(&id, &c, &p, limits(), &facts(&c), &mut r);
        if delta < 0 {
            assert_eq!(v.unwrap_err(), InvocationError::Limit);
        } else {
            o.retire_invocation_observation_v1(&mut v.unwrap(), &control(&c), &mut r)
                .unwrap();
        }
        assert_eq!(o.retention_usage().reserved, 2048);
    }
    for field in 0..3 {
        for delta in [-1isize, 0, 1] {
            let (mut o, id, c, p, mut r) = pending();
            let mut l = limits();
            match field {
                0 => {
                    l.max_reference_bytes = size_of::<InvocationObservationReferenceV1>()
                        .checked_add_signed(delta)
                        .unwrap()
                }
                1 => {
                    l.max_control_reply_bytes = size_of::<InvocationControlObservationV1>()
                        .checked_add_signed(delta)
                        .unwrap()
                }
                _ => {
                    l.max_reply_bytes = (512 + size_of::<InvocationControlObservationV1>())
                        .max(size_of::<InvocationObservationReplyV1>())
                        .checked_add_signed(delta)
                        .unwrap()
                }
            }
            let v = o.open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r);
            if delta < 0 {
                assert_eq!(v.unwrap_err(), InvocationError::Limit);
            } else {
                o.retire_invocation_observation_v1(&mut v.unwrap(), &control(&c), &mut r)
                    .unwrap();
            }
            assert_eq!(o.retention_usage().reserved, 2048);
        }
    }
    for gate in 0..8 {
        for value in [None, Some(false)] {
            let (mut o, id, c, p, mut r) = pending();
            let mut l = limits();
            l.max_issued = 2;
            l.max_held_bytes = 16384;
            let mut v = o
                .open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r)
                .unwrap();
            let mut f = release(&c);
            let mut ordinary_ready = Some(true);
            let mut resources_ready = Some(true);
            match gate {
                0 => f.authorized = value,
                1 => f.scope = value,
                2 => f.continuous = value,
                3 => f.live = value,
                4 => f.retained = value,
                5 => f.transfer_ready = value,
                6 => ordinary_ready = value,
                _ => resources_ready = value,
            }
            let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
                current: &f,
                ordinary_ready,
                resources_ready,
            };
            assert_eq!(
                o.query_invocation_observation_v1(
                    &mut v,
                    InvocationObservationQueryV1::Status,
                    &p,
                    &ordinary,
                    &mut r
                ),
                Err(InvocationError::Unavailable)
            );
            let f = release(&c);
            let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
                current: &f,
                ordinary_ready: Some(true),
                resources_ready: Some(true),
            };
            assert_eq!(
                o.query_invocation_observation_v1(
                    &mut v,
                    InvocationObservationQueryV1::Status,
                    &p,
                    &ordinary,
                    &mut r
                )
                .is_ok(),
                value.is_none() || gate >= 5
            );
            o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
                .unwrap();
        }
    }
    let (mut o, id, c, p, mut r) = pending();
    let mut l = limits();
    l.max_issued = 1;
    l.max_held_bytes = 16384;
    let mut v = o
        .open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    let mut current = p.clone();
    current.control = false;
    current.publication = false;
    current.effect = false;
    current.recovery = false;
    let mut f = release(&c);
    f.control_ready = None;
    let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
        current: &f,
        ordinary_ready: Some(true),
        resources_ready: Some(true),
    };
    assert_eq!(
        o.query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::Status,
            &current,
            &ordinary,
            &mut r
        ),
        Err(InvocationError::Unavailable)
    );
    let out = o
        .query_invocation_observation_v1(
            &mut v,
            InvocationObservationQueryV1::RetainedOutcome,
            &current,
            &ordinary,
            &mut r,
        )
        .unwrap();
    assert!(out.phase.is_none());
    let out = out.outcome.unwrap();
    assert!(
        out.publication.is_none()
            && out.control.is_none()
            && out.effect.is_none()
            && out.recovery.is_none()
    );
}

#[test]
fn ordinary_query_timing_is_optional_bounded_and_inert_under_saturation() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, id, c, p, _) = pending();
        let mut l = limits();
        l.max_issued = 3;
        l.max_held_bytes = 16384;
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        if mode == 3 {
            r.record(Observation {
                boundary: Boundary::Control,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        let mut v = o
            .open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r)
            .unwrap();
        let f = release(&c);
        let ordinary = TrustedInvocationOrdinaryObservationReleaseV1 {
            current: &f,
            ordinary_ready: Some(true),
            resources_ready: Some(true),
        };
        let first = o
            .query_invocation_observation_v1(
                &mut v,
                InvocationObservationQueryV1::Status,
                &p,
                &ordinary,
                &mut r,
            )
            .unwrap();
        assert_eq!(
            first,
            o.query_invocation_observation_v1(
                &mut v,
                InvocationObservationQueryV1::Status,
                &p,
                &ordinary,
                &mut r
            )
            .unwrap()
        );
        assert_eq!(
            o.query_invocation_observation_v1(
                &mut v,
                InvocationObservationQueryV1::RetainedOutcome,
                &p,
                &ordinary,
                &mut r
            ),
            Err(InvocationError::Unavailable)
        );
        o.run(&id, &mut r).unwrap();
        let terminal = o
            .query_invocation_observation_v1(
                &mut v,
                InvocationObservationQueryV1::RetainedOutcome,
                &p,
                &ordinary,
                &mut r,
            )
            .unwrap();
        assert_eq!(
            terminal.outcome.unwrap().publication,
            Some(InvocationPublicationV1::Applied)
        );
        assert_eq!(
            terminal.reference,
            o.query_invocation_observation_v1(
                &mut v,
                InvocationObservationQueryV1::Status,
                &p,
                &ordinary,
                &mut r
            )
            .unwrap()
            .reference
        );
        o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
            .unwrap();
        let mut count = 0;
        let mut refused = 0;
        while let Some(e) = r.pop_front() {
            if e.boundary == Boundary::InvocationObservationQuery {
                count += 1;
                refused += usize::from(e.disposition == Disposition::Refused);
                assert_eq!(e.service_time.is_some(), mode == 2);
            }
            assert!(!format!("{e:?}").contains("native-observation/1"));
        }
        assert_eq!(count, if mode == 1 || mode == 2 { 5 } else { 0 });
        if mode == 1 || mode == 2 {
            assert_eq!(refused, 1);
        }
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
    }
}

#[test]
fn observer_admission_requires_every_original_gate_and_never_leaks_a_hold_on_refusal() {
    for gate in 0..10 {
        for value in [None, Some(false)] {
            let (mut o, id, c, p, mut r) = pending();
            let mut f = facts(&c);
            match gate {
                0 => f.authorized = value,
                1 => f.scope = value,
                2 => f.continuous = value,
                3 => f.live = value,
                4 => f.resources_ready = value,
                5 => f.retained = value,
                6 => f.transfer_ready = value,
                7 => f.preparation_reserved = value,
                8 => f.registry_ready = value,
                _ => f.retention_ready = value,
            }
            assert_eq!(
                o.open_invocation_observation_v1(&id, &c, &p, limits(), &f, &mut r)
                    .unwrap_err(),
                InvocationError::Unavailable
            );
            assert_eq!(o.retention_usage().reserved, 2048);
            assert_eq!(
                o.invocation_observation_policy_usage_v1(&id, &mut r)
                    .unwrap()
                    .attached_capture_users,
                0
            );
            assert_eq!(
                o.invocation_state(&id, &mut r),
                Ok(InvocationState::Pending)
            );
        }
    }
}

#[test]
fn observer_admission_requires_an_admitted_original_ticket_and_exact_context_policy_binding() {
    let (mut o, id, c, p, mut r) = pending();
    let issued = o.issue_invocation(&mut r).unwrap();
    o.bind_invocation_observation_policy_v1(
        &issued,
        &c,
        &p,
        InvocationObservationPolicyLimitsV1 {
            max_context_bytes: 2048,
            max_result_bytes: 512,
        },
        &facts(&c),
        &mut r,
    )
    .unwrap();
    assert_eq!(
        o.open_invocation_observation_v1(&issued, &c, &p, limits(), &facts(&c), &mut r)
            .unwrap_err(),
        InvocationError::Unavailable
    );
    let (mut foreign, foreign_id, _, _, _) = pending();
    assert_eq!(
        o.open_invocation_observation_v1(&foreign_id, &c, &p, limits(), &facts(&c), &mut r)
            .unwrap_err(),
        InvocationError::Unavailable
    );
    assert_eq!(
        foreign
            .open_invocation_observation_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
            .unwrap_err(),
        InvocationError::Unavailable
    );
    let baseline = o.retention_usage().reserved;
    for mismatch in 0..6 {
        let mut other = c.clone();
        let mut ceiling = p.clone();
        let mut host = c.clone();
        match mismatch {
            0 => other.purpose.push('!'),
            1 => ceiling.epoch += 1,
            2 => ceiling.recipient.push('!'),
            3 => ceiling.grant = false,
            4 => host.continuity.push('!'),
            _ => other.authority = "X".repeat(8192),
        }
        let expected = if mismatch == 5 {
            InvocationError::Limit
        } else {
            InvocationError::Unavailable
        };
        assert_eq!(
            o.open_invocation_observation_v1(
                &id,
                &other,
                &ceiling,
                limits(),
                &facts(&host),
                &mut r
            )
            .unwrap_err(),
            expected
        );
        assert_eq!(o.retention_usage().reserved, baseline);
    }
    for delta in [-1isize, 0, 1] {
        let mut l = limits();
        l.max_context_bytes = InvocationObservationLimitsV1::required_context_bytes(&c, &p)
            .unwrap()
            .checked_add_signed(delta)
            .unwrap();
        let result = o.open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r);
        if delta < 0 {
            assert_eq!(result.unwrap_err(), InvocationError::Limit);
        } else {
            o.retire_invocation_observation_v1(&mut result.unwrap(), &control(&c), &mut r)
                .unwrap();
        }
        assert_eq!(o.retention_usage().reserved, baseline);
    }
}

#[test]
fn explicit_denial_closes_observation_but_unknown_or_mismatched_inputs_do_not_poison_it() {
    for gate in 0..7 {
        for value in [None, Some(false)] {
            let (mut o, id, c, p, mut r) = pending();
            let mut v = o
                .open_invocation_observation_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
                .unwrap();
            let mut f = release(&c);
            match gate {
                0 => f.authorized = value,
                1 => f.scope = value,
                2 => f.continuous = value,
                3 => f.live = value,
                4 => f.retained = value,
                5 => f.transfer_ready = value,
                _ => f.control_ready = value,
            }
            assert_eq!(
                o.minimal_invocation_control_status_v1(&mut v, &p, &f, &mut r),
                Err(InvocationError::Unavailable)
            );
            let recovered =
                o.minimal_invocation_control_status_v1(&mut v, &p, &release(&c), &mut r);
            assert_eq!(recovered.is_ok(), value.is_none() || gate >= 5);
            assert_eq!(
                o.invocation_state(&id, &mut r),
                Ok(InvocationState::Pending)
            );
            o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
                .unwrap();
            assert_eq!(o.retention_usage().reserved, 2048);
        }
    }
    for mismatch in 0..9 {
        let (mut o, id, c, p, mut r) = pending();
        let mut v = o
            .open_invocation_observation_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
            .unwrap();
        let mut other = c.clone();
        let mut current = p.clone();
        match mismatch {
            0 => other.authority.push('!'),
            1 => other.engine.push('!'),
            2 => other.recipient.push('!'),
            3 => other.purpose.push('!'),
            4 => other.destination.push('!'),
            5 => other.profile.push('!'),
            6 => other.continuity.push('!'),
            7 => current.recipient.push('!'),
            _ => current.epoch += 1,
        }
        assert_eq!(
            o.minimal_invocation_control_status_v1(&mut v, &current, &release(&other), &mut r),
            Err(InvocationError::Unavailable)
        );
        assert!(
            o.minimal_invocation_control_status_v1(&mut v, &p, &release(&c), &mut r)
                .is_ok()
        );
    }
}

#[test]
fn control_requires_original_captured_and_current_rights_and_grant_loss_is_absorbing() {
    for denied in 0..3 {
        let (mut o, id, c, p, mut r) = pending();
        // The original ceiling is immutable, so create an independent original
        // with control denied before admission for the first case.
        let (target, original) = if denied == 0 {
            let id = o.issue_invocation(&mut r).unwrap();
            let mut original = p.clone();
            original.control = false;
            o.bind_invocation_observation_policy_v1(
                &id,
                &c,
                &original,
                InvocationObservationPolicyLimitsV1 {
                    max_context_bytes: 2048,
                    max_result_bytes: 512,
                },
                &facts(&c),
                &mut r,
            )
            .unwrap();
            let d = o.create(b"X", &mut r).unwrap();
            o.submit(&id, &intent(d, "X", "Y"), &mut r).unwrap();
            (id, original)
        } else {
            (id, p.clone())
        };
        let mut ceiling = p.clone();
        if denied == 1 {
            ceiling.control = false;
        }
        let mut current = p.clone();
        if denied == 2 {
            current.control = false;
        }
        let mut v = o
            .open_invocation_observation_v1(&target, &c, &ceiling, limits(), &facts(&c), &mut r)
            .unwrap();
        assert_eq!(
            o.minimal_invocation_control_status_v1(&mut v, &current, &release(&c), &mut r),
            Err(InvocationError::Unavailable)
        );
        if !original.control {
            assert_eq!(
                o.invocation_state(&target, &mut r),
                Ok(InvocationState::Pending)
            );
        }
    }
    let (mut o, id, c, p, mut r) = pending();
    let mut v = o
        .open_invocation_observation_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
        .unwrap();
    let mut revoked = p.clone();
    revoked.grant = false;
    assert!(
        o.minimal_invocation_control_status_v1(&mut v, &revoked, &release(&c), &mut r)
            .is_err()
    );
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut v, &p, &release(&c), &mut r),
        Err(InvocationError::Unavailable)
    );
    o.run(&id, &mut r).unwrap();
    assert!(o.invocation_outcome(&id, &mut r).is_ok());
    o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
}

#[test]
fn observer_funding_is_exact_and_independent_of_ordinary_reference_capacity() {
    for delta in [-1isize, 0, 1] {
        let l = limits();
        let charge = l.required_hold_bytes().unwrap();
        let (mut o, id, c, p, mut r) =
            pending_with_budget((2048 + charge).checked_add_signed(delta).unwrap());
        let result = o.open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r);
        if delta < 0 {
            assert_eq!(result.unwrap_err(), InvocationError::Limit);
            assert_eq!(o.retention_usage().reserved, 2048);
        } else {
            let mut v = result.unwrap();
            assert_eq!(o.retention_usage().reserved, 2048 + charge);
            assert!(
                o.minimal_invocation_control_status_v1(&mut v, &p, &release(&c), &mut r)
                    .is_ok()
            );
            o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
                .unwrap();
            assert_eq!(o.retention_usage().reserved, 2048);
            assert_eq!(
                o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r),
                Err(InvocationError::Unavailable)
            );
        }
    }
    for field in 0..9 {
        let (mut o, id, c, p, mut r) = pending();
        let mut l = limits();
        match field {
            0 => l.max_waiters = usize::MAX,
            1 => l.max_issued = usize::MAX,
            2 => l.max_context_bytes = usize::MAX,
            3 => l.max_wait_bytes = usize::MAX,
            4 => l.max_reference_bytes = usize::MAX,
            5 => l.max_reply_bytes = usize::MAX,
            6 => l.max_held_bytes = 1,
            7 => l.max_control_reply_bytes = 0,
            _ => l.max_reference_bytes = 0,
        }
        assert_eq!(
            o.open_invocation_observation_v1(&id, &c, &p, l, &facts(&c), &mut r)
                .unwrap_err(),
            InvocationError::Limit
        );
        assert_eq!(o.retention_usage().reserved, 2048);
    }
}

#[test]
fn close_and_retire_are_owner_controlled_and_neither_stop_nor_settle_the_producer() {
    let (mut o, id, c, p, mut r) = pending();
    let mut v = o
        .open_invocation_observation_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
        .unwrap();
    let foreign = owner();
    let mut denied = release(&c);
    denied.authorized = Some(false);
    assert_eq!(
        foreign.minimal_invocation_control_status_v1(&mut v, &p, &denied, &mut r),
        Err(InvocationError::Unavailable)
    );
    assert!(
        o.minimal_invocation_control_status_v1(&mut v, &p, &release(&c), &mut r)
            .is_ok()
    );
    for gate in 0..3 {
        for value in [None, Some(false)] {
            let mut f = control(&c);
            match gate {
                0 => f.owner_control_allowed = value,
                1 => f.retention_released = value,
                _ => f.delivery_drained = value,
            }
            assert_eq!(
                o.retire_invocation_observation_v1(&mut v, &f, &mut r),
                Err(InvocationError::Unavailable)
            );
        }
    }
    let mut f = control(&c);
    f.owner_control_allowed = Some(false);
    assert_eq!(
        o.close_invocation_observation_v1(&mut v, &f, &mut r),
        Err(InvocationError::Unavailable)
    );
    o.close_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
    assert_eq!(
        o.minimal_invocation_control_status_v1(&mut v, &p, &release(&c), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.invocation_state(&id, &mut r),
        Ok(InvocationState::Pending)
    );
    o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
        .unwrap();
    o.run(&id, &mut r).unwrap();
    assert!(o.invocation_outcome(&id, &mut r).is_ok());
}

#[test]
fn protected_status_instrumentation_is_optional_bounded_and_never_contains_context_or_content() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, id, c, p, _) = pending();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 32 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        if mode == 3 {
            r.record(Observation {
                boundary: Boundary::Control,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        let mut v = o
            .open_invocation_observation_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
            .unwrap();
        // Non-control flags do not authorize disclosure or prevent phase-only
        // control: original/captured/current control are the relevant meet.
        let mut current = p.clone();
        current.publication = false;
        current.effect = false;
        current.reasons = false;
        current.recovery = false;
        current.details = true;
        assert_eq!(
            o.minimal_invocation_control_status_v1(&mut v, &current, &release(&c), &mut r)
                .unwrap()
                .phase,
            InvocationObservationPhaseV1::Pending
        );
        o.close_invocation_observation_v1(&mut v, &control(&c), &mut r)
            .unwrap();
        assert_eq!(
            o.minimal_invocation_control_status_v1(&mut v, &p, &release(&c), &mut r),
            Err(InvocationError::Unavailable)
        );
        o.retire_invocation_observation_v1(&mut v, &control(&c), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 2048);
        assert_eq!(
            o.invocation_state(&id, &mut Recorder::disabled()),
            Ok(InvocationState::Pending)
        );
        let mut counts = [0; 4];
        let mut refusals = 0;
        while let Some(e) = r.pop_front() {
            let i = match e.boundary {
                Boundary::InvocationObservationAdmission => Some(0),
                Boundary::InvocationControlObservation => Some(1),
                Boundary::InvocationObservationClose => Some(2),
                Boundary::InvocationObservationRetirement => Some(3),
                _ => None,
            };
            if let Some(i) = i {
                counts[i] += 1;
                refusals += usize::from(e.disposition == Disposition::Refused);
                assert_eq!(e.service_time.is_some(), mode == 2);
            }
            let debug = format!("{e:?}");
            assert!(!debug.contains("native-observation/1"));
            assert!(!debug.contains("recipient"));
        }
        assert_eq!(
            counts,
            if mode == 1 || mode == 2 {
                [1, 2, 1, 1]
            } else {
                [0; 4]
            }
        );
        if mode == 1 || mode == 2 {
            assert_eq!(refusals, 1);
        }
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
    }
}
