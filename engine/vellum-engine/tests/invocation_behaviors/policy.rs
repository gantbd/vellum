use super::{intent, owner};
use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
fn issued(
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
    let id = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let p = policy(&c);
    (o, id, c, p, r)
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "p".into(),
        destination: "d".into(),
        profile: "source-policy/1".into(),
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
fn limits() -> InvocationObservationPolicyLimitsV1 {
    InvocationObservationPolicyLimitsV1 {
        max_context_bytes: 2048,
        max_result_bytes: 512,
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
#[test]
fn original_policy_is_funded_once_before_admission_and_retired_independently_from_the_producer() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    o.configure_retention_budget(4096, &mut r).unwrap();
    let d = o.create(b"A", &mut r).unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let c = context();
    let p = policy(&c);
    assert!(
        o.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
            .unwrap()
    );
    assert_eq!(o.retention_usage().reserved, 2048);
    assert!(
        !o.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
            .unwrap()
    );
    o.submit(&id, &intent(d.clone(), "A", "B"), &mut r).unwrap();
    assert_eq!(
        o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r),
        Err(InvocationError::Pending)
    );
    o.run(&id, &mut r).unwrap();
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 2048);
    assert_eq!(
        o.retire_invocation(&id, &mut r),
        Err(InvocationError::Unavailable)
    );
    o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(
        o.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
            .is_err()
    );
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "B");
    assert_eq!(o.invocation_usage().issued_lifetime, 1);
}

#[test]
fn exact_policy_capture_and_parent_headroom_are_independent_pre_copy_bounds() {
    let c = context();
    let p = policy(&c);
    let exact = InvocationObservationPolicyLimitsV1::required_capture_bytes(&c, &p).unwrap();
    for delta in [-1isize, 0, 1] {
        for headroom in [-1isize, 0, 1] {
            let bytes = exact.checked_add_signed(delta).unwrap();
            let budget = bytes.checked_add_signed(headroom).unwrap();
            let (mut o, id, c, p, mut r) = issued(budget);
            let mut l = limits();
            l.max_context_bytes = bytes;
            let result =
                o.bind_invocation_observation_policy_v1(&id, &c, &p, l, &facts(&c), &mut r);
            let accepted = delta >= 0 && headroom >= 0;
            assert_eq!(result.is_ok(), accepted);
            assert_eq!(
                o.retention_usage().reserved,
                if accepted { bytes } else { 0 }
            );
            assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
            assert_eq!(o.invocation_usage().issued_lifetime, 1);
        }
    }
    for invalid in 0..4 {
        let (mut o, id, c, p, mut r) = issued(4096);
        let mut l = limits();
        match invalid {
            0 => l.max_context_bytes = 0,
            1 => l.max_context_bytes = usize::MAX,
            2 => l.max_result_bytes = 0,
            _ => l.max_result_bytes = usize::MAX,
        }
        assert_eq!(
            o.bind_invocation_observation_policy_v1(&id, &c, &p, l, &facts(&c), &mut r),
            Err(InvocationError::Limit)
        );
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
#[test]
fn every_unknown_or_false_host_fact_and_unsupported_policy_refuses_without_binding() {
    for oversized in 0..3 {
        let (mut o, id, mut c, mut p, mut r) = issued(4096);
        let mut host_context = context();
        match oversized {
            0 => c.purpose = "x".repeat(4096),
            1 => p.recipient = "x".repeat(4096),
            _ => host_context.purpose = "x".repeat(4096),
        }
        assert_eq!(
            o.bind_invocation_observation_policy_v1(
                &id,
                &c,
                &p,
                limits(),
                &facts(&host_context),
                &mut r
            ),
            Err(InvocationError::Limit)
        );
        assert_eq!(o.retention_usage().reserved, 0);
        assert!(!o.is_faulted());
    }
    for gate in 0..10 {
        for value in [None, Some(false)] {
            let (mut o, id, c, p, mut r) = issued(4096);
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
                o.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &f, &mut r),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(o.retention_usage().reserved, 0);
            assert!(
                !o.invocation_observation_policy_usage_v1(&id, &mut r)
                    .unwrap()
                    .bound
            );
        }
    }
    for invalid in 0..4 {
        let (mut o, id, c, mut p, mut r) = issued(4096);
        let mut foreign = context();
        foreign.purpose.push('!');
        let mut f = facts(&c);
        match invalid {
            0 => p.grant = false,
            1 => p.details = true,
            2 => p.recipient.push('!'),
            _ => f.context = &foreign,
        }
        assert_eq!(
            o.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &f, &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(o.retention_usage().reserved, 0);
        assert!(!o.is_faulted());
    }
}
#[test]
fn policy_cannot_be_added_late_or_reinterpreted_after_binding() {
    let (mut o, id, c, p, mut r) = issued(4096);
    let d = o.create(b"A", &mut r).unwrap();
    o.submit(&id, &intent(d, "A", "B"), &mut r).unwrap();
    assert_eq!(
        o.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &facts(&c), &mut r),
        Err(InvocationError::Unavailable)
    );
    o.run(&id, &mut r).unwrap();
    assert_eq!(
        o.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &facts(&c), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(o.retention_usage().reserved, 0);
    let (mut o, id, c, p, mut r) = issued(4096);
    o.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
        .unwrap();
    for field in 0..9 {
        let mut changed = p.clone();
        let mut l = limits();
        let mut other = context();
        match field {
            0 => changed.epoch += 1,
            1 => changed.publication = false,
            2 => changed.control = false,
            3 => changed.effect = false,
            4 => changed.reasons = false,
            5 => changed.recovery = false,
            6 => l.max_context_bytes += 1,
            7 => l.max_result_bytes += 1,
            _ => other.profile.push('!'),
        }
        assert_eq!(
            o.bind_invocation_observation_policy_v1(
                &id,
                &other,
                &changed,
                l,
                &facts(&other),
                &mut r
            ),
            Err(InvocationError::IntentMismatch)
        );
        assert_eq!(o.retention_usage().reserved, 2048);
    }
    assert!(
        !o.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
            .unwrap()
    );
}
#[test]
fn policy_retirement_requires_exact_context_and_all_discharge_facts() {
    let (mut o, id, c, p, mut r) = issued(4096);
    o.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
        .unwrap();
    for gate in 0..3 {
        for value in [None, Some(false)] {
            let mut f = control(&c);
            match gate {
                0 => f.owner_control_allowed = value,
                1 => f.retention_released = value,
                _ => f.delivery_drained = value,
            }
            assert_eq!(
                o.retire_invocation_observation_policy_v1(&id, &f, &mut r),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(o.retention_usage().reserved, 2048);
        }
    }
    let mut other = context();
    other.recipient.push('!');
    assert_eq!(
        o.retire_invocation_observation_policy_v1(&id, &control(&other), &mut r),
        Err(InvocationError::Unavailable)
    );
    o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(
        o.invocation_observation_policy_usage_v1(&id, &mut r)
            .unwrap()
            .spent
    );
    assert!(
        o.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
            .is_err()
    );
    let (mut foreign, _, _, _, mut rr) = issued(4096);
    assert_eq!(
        foreign.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &facts(&c), &mut rr),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(foreign.retention_usage().reserved, 0);
}
#[test]
fn original_policy_diagnostics_are_optional_bounded_and_contain_no_context_values() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation,
    };
    for mode in 0..4 {
        let (mut o, id, mut c, _, _) = issued(4096);
        c.recipient = "PRIVATE_RECIPIENT".into();
        let p = policy(&c);
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
        o.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
            .unwrap();
        assert!(
            !o.bind_invocation_observation_policy_v1(&id, &c, &p, limits(), &facts(&c), &mut r)
                .unwrap()
        );
        let mut changed = p.clone();
        changed.epoch += 1;
        assert_eq!(
            o.bind_invocation_observation_policy_v1(
                &id,
                &c,
                &changed,
                limits(),
                &facts(&c),
                &mut r
            ),
            Err(InvocationError::IntentMismatch)
        );
        assert!(
            o.invocation_observation_policy_usage_v1(&id, &mut r)
                .unwrap()
                .bound
        );
        o.retire_invocation_observation_policy_v1(&id, &control(&c), &mut r)
            .unwrap();
        assert!(
            o.invocation_observation_policy_usage_v1(&id, &mut r)
                .unwrap()
                .spent
        );
        assert_eq!(o.retention_usage().reserved, 0);
        assert_eq!(o.invocation_state(&id, &mut r), Ok(InvocationState::Issued));
        let mut counts = [0; 3];
        let mut refused = 0;
        while let Some(event) = r.pop_front() {
            let index = match event.boundary {
                Boundary::InvocationPolicyBinding => Some(0),
                Boundary::InvocationPolicyUsage => Some(1),
                Boundary::InvocationPolicyRetirement => Some(2),
                _ => None,
            };
            if let Some(index) = index {
                counts[index] += 1;
                refused += usize::from(event.disposition == Disposition::Refused);
                assert_eq!(event.service_time.is_some(), mode == 2);
            }
            assert!(!format!("{event:?}").contains("PRIVATE"));
        }
        assert_eq!(
            counts,
            if mode == 1 || mode == 2 {
                [3, 2, 1]
            } else {
                [0, 0, 0]
            }
        );
        if mode == 1 || mode == 2 {
            assert_eq!(refused, 1);
        }
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
    }
}
