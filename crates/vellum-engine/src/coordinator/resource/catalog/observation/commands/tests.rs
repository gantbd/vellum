use super::super::tests::{
    addresses, change, cleanup, control, facts, fixture, limits, open, rows, snapshot,
};
use super::*;

#[test]
fn registry_command_issuance_is_not_native_admission_and_busy_state_cannot_reenter_or_retire() {
    let mut l = limits();
    l.max_streams = 0;
    l.max_packet_bytes = 65536;
    l.max_held_bytes = 2097152;
    let (mut o, mut r, s) = fixture(l);
    let f = facts(&s);
    let op = ResourceRegistryCommandOperationV1::Establish;
    let id = o
        .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
        .unwrap();
    let p = prepare(&mut o, &id, &s, &mut r);
    assert!(matches!(
        request(&mut o, &id, &op, &s, Some(&p), &mut r),
        Err(ResourceError::Limit)
    ));
    assert_eq!(
        o.resource_registry_observation_usage_v1()
            .unwrap()
            .preparations,
        2
    );
    let host = o
        .resource_registry_command_facts_v1(&id, &s, &mut r)
        .unwrap();
    assert!(!host.native_started);
    assert_eq!(host.phase, ResourceRegistryCommandPhaseV1::Issued);
    // Model reentry at the actual private busy point; safe Rust's exclusive
    // owner borrow already prevents a simultaneous mutable supplier call.
    o.registry_command_mut_v1(&id).phase = ResourceRegistryCommandPhaseV1::Executing;
    assert!(request(&mut o, &id, &op, &s, Some(&p), &mut r).is_err());
    assert!(
        o.retire_resource_registry_command_v1(&id, &cleanup(&s), &mut r)
            .is_err()
    );
    o.registry_command_mut_v1(&id).phase = ResourceRegistryCommandPhaseV1::Issued;
    o.retire_resource_registry_command_v1(&id, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_resource_registry_preparation_v1(&p, &cleanup(&s), &mut r)
        .unwrap();
}

#[test]
fn registry_command_optional_capture_preserves_native_results_and_separates_lookup_timing() {
    use crate::instrumentation::NativeTimingProfileV1;
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 64] {
            let (mut o, mut setup, s) = setup();
            let f = facts(&s);
            let op = ResourceRegistryCommandOperationV1::Establish;
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            let debt = o.retention_usage();
            let root = o.generation();
            let id = o
                .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
                .unwrap();
            let p = prepare(&mut o, &id, &s, &mut r);
            request(&mut o, &id, &op, &s, Some(&p), &mut r).unwrap();
            let host = o
                .resource_registry_command_facts_v1(&id, &s, &mut setup)
                .unwrap();
            let baseline = host.as_observed.unwrap().as_ref().unwrap();
            assert_eq!(baseline.projection.rows().len(), 2);
            assert_eq!(host.historical_slots.len(), 2);
            for (slot, row) in host.historical_slots.iter().zip(baseline.projection.rows()) {
                assert_eq!(slot.reference, row.reference);
                assert_eq!(slot.address.table, row.table);
            }
            let before = o.resource_registry_observation_usage_v1().unwrap();
            assert!(matches!(
                request(&mut o, &id, &op, &s, None, &mut r),
                Ok(ResourceRegistryCommandReplyV1::ObserveExisting { .. })
            ));
            assert_eq!(o.resource_registry_observation_usage_v1().unwrap(), before);
            assert_eq!(o.retention_usage(), debt);
            assert_eq!(o.generation(), root);
            if capacity == 64 {
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                for boundary in [
                    Boundary::RegistryCommandIssue,
                    Boundary::RegistryCommandExecution,
                    Boundary::RegistryCommandLookup,
                ] {
                    assert!(events.iter().any(|e| e.boundary == boundary
                        && e.service_time.is_some()
                            == (timing == NativeTimingProfileV1::ProcessMonotonicV1)));
                }
            }
        }
    }
}
pub(super) fn setup() -> (
    SourceCoordinator,
    Recorder,
    ResourceRegistryObservationScopeV1,
) {
    let mut l = limits();
    l.max_packet_bytes = 65536;
    l.max_held_bytes = 2097152;
    fixture(l)
}
pub(super) fn admission<'a>(
    f: &'a TrustedResourceRegistryObservationFactsV1<'a>,
) -> TrustedResourceRegistryCommandAdmissionV1<'a> {
    TrustedResourceRegistryCommandAdmissionV1 {
        current: f,
        owner_control_allowed: Some(true),
        baseline_allowed: Some(true),
    }
}
pub(super) fn prepare(
    o: &mut SourceCoordinator,
    id: &ResourceRegistryCommandIdV1,
    s: &ResourceRegistryObservationScopeV1,
    r: &mut Recorder,
) -> ResourceRegistryPreparationIdV1 {
    let a = addresses(o);
    let rows = rows(&a, None);
    let f = facts(s);
    o.prepare_resource_registry_command_snapshot_v1(
        id,
        s,
        &TrustedResourceRegistrySnapshotFactsV1 {
            current: &f,
            rows: &rows,
            capture_policy_current: Some(true),
            visibility_complete: Some(true),
            order_attested: Some(true),
        },
        r,
    )
    .unwrap()
}
#[test]
fn registry_command_matching_retry_is_identity_only_even_after_root_change_stop_and_fault() {
    let (mut o, mut r, s) = setup();
    let f = facts(&s);
    let op = ResourceRegistryCommandOperationV1::Establish;
    let id = o
        .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
        .unwrap();
    let p = prepare(&mut o, &id, &s, &mut r);
    let request = ResourceRegistryCommandRequestV1 {
        operation: &op,
        scope: &s,
        preparation: Some(&p),
        current: Some(&f),
        owner_control_allowed: Some(true),
        baseline_allowed: Some(true),
    };
    assert_eq!(
        o.request_resource_registry_command_v1(&id, &request, &mut r),
        Ok(ResourceRegistryCommandReplyV1::Settled {
            command: id.clone()
        })
    );
    let host = o
        .resource_registry_command_facts_v1(&id, &s, &mut r)
        .unwrap();
    assert_eq!(host.phase, ResourceRegistryCommandPhaseV1::Settled);
    let baseline = host.as_observed.unwrap().as_ref().unwrap().clone();
    let stream = baseline.stream.clone();
    change(&mut o, 0, false, false, &mut r);
    o.stop_resource_registry_observation_v1(&stream, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_resource_registry_observation_v1(&stream, &cleanup(&s), &mut r)
        .unwrap();
    let before = o.resource_registry_observation_usage_v1().unwrap();
    o.faulted = true;
    let replay = ResourceRegistryCommandRequestV1 {
        operation: &op,
        scope: &s,
        preparation: None,
        current: None,
        owner_control_allowed: Some(true),
        baseline_allowed: None,
    };
    assert_eq!(
        o.request_resource_registry_command_v1(&id, &replay, &mut r),
        Ok(ResourceRegistryCommandReplyV1::ObserveExisting {
            command: id.clone()
        })
    );
    assert!(Arc::ptr_eq(
        &baseline,
        o.resource_registry_command_facts_v1(&id, &s, &mut r)
            .unwrap()
            .as_observed
            .unwrap()
            .as_ref()
            .unwrap()
    ));
    assert_eq!(o.resource_registry_observation_usage_v1().unwrap(), before);
    assert!(o.is_faulted());
}
#[test]
fn registry_command_stale_engine_root_refuses_without_consuming_intent_or_preparation() {
    let (mut o, mut r, s) = setup();
    let f = facts(&s);
    let op = ResourceRegistryCommandOperationV1::Establish;
    let id = o
        .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
        .unwrap();
    let p = prepare(&mut o, &id, &s, &mut r);
    o.create(b"unrelated root publication", &mut r).unwrap();
    let request = ResourceRegistryCommandRequestV1 {
        operation: &op,
        scope: &s,
        preparation: Some(&p),
        current: Some(&f),
        owner_control_allowed: Some(true),
        baseline_allowed: Some(true),
    };
    assert!(
        o.request_resource_registry_command_v1(&id, &request, &mut r)
            .is_err()
    );
    let host = o
        .resource_registry_command_facts_v1(&id, &s, &mut r)
        .unwrap();
    assert_eq!(host.phase, ResourceRegistryCommandPhaseV1::Issued);
    assert!(!host.native_started);
    assert_eq!(
        o.resource_registry_observation_usage_v1().unwrap().streams,
        0
    );
    assert_eq!(
        o.resource_registry_observation_usage_v1()
            .unwrap()
            .preparations,
        2
    );
}

fn request(
    o: &mut SourceCoordinator,
    id: &ResourceRegistryCommandIdV1,
    op: &ResourceRegistryCommandOperationV1,
    s: &ResourceRegistryObservationScopeV1,
    p: Option<&ResourceRegistryPreparationIdV1>,
    r: &mut Recorder,
) -> Result<ResourceRegistryCommandReplyV1> {
    let f = facts(s);
    o.request_resource_registry_command_v1(
        id,
        &ResourceRegistryCommandRequestV1 {
            operation: op,
            scope: s,
            preparation: p,
            current: Some(&f),
            owner_control_allowed: Some(true),
            baseline_allowed: Some(true),
        },
        r,
    )
}

#[test]
fn registry_command_denied_pre_admission_preserves_original_issued_state_and_packet() {
    let (mut o, mut r, s) = setup();
    let f = facts(&s);
    let op = ResourceRegistryCommandOperationV1::Establish;
    let id = o
        .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
        .unwrap();
    let p = prepare(&mut o, &id, &s, &mut r);
    for denied in [None, Some(false)] {
        let q = ResourceRegistryCommandRequestV1 {
            operation: &op,
            scope: &s,
            preparation: Some(&p),
            current: Some(&f),
            owner_control_allowed: denied,
            baseline_allowed: Some(true),
        };
        assert!(
            o.request_resource_registry_command_v1(&id, &q, &mut r)
                .is_err()
        );
        let c = o
            .resource_registry_command_facts_v1(&id, &s, &mut r)
            .unwrap();
        assert_eq!(c.phase, ResourceRegistryCommandPhaseV1::Issued);
        assert!(!c.native_started);
        assert_eq!(
            o.resource_registry_observation_usage_v1()
                .unwrap()
                .preparations,
            2
        );
    }
    assert!(matches!(
        request(&mut o, &id, &op, &s, Some(&p), &mut r),
        Ok(ResourceRegistryCommandReplyV1::Settled { .. })
    ));
}

#[test]
fn registry_command_resync_basis_is_generation_exact_and_replay_never_resets_later_queue() {
    let (mut o, mut r, s) = setup();
    let initial = open(&mut o, &s, None, &mut r);
    let f = facts(&s);
    let op = ResourceRegistryCommandOperationV1::Resynchronize {
        stream: initial.stream.clone(),
    };
    let stale = o
        .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
        .unwrap();
    let oldprep = prepare(&mut o, &stale, &s, &mut r);
    let p = snapshot(&mut o, &initial.stream, &s, None, &mut r);
    o.resync_resource_registry_observation_v1(&initial.stream, &s, &p, &f, Some(true), &mut r)
        .unwrap();
    assert!(request(&mut o, &stale, &op, &s, Some(&oldprep), &mut r).is_err());
    assert!(
        !o.resource_registry_command_facts_v1(&stale, &s, &mut r)
            .unwrap()
            .native_started
    );
    o.retire_resource_registry_command_v1(&stale, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_resource_registry_preparation_v1(&oldprep, &cleanup(&s), &mut r)
        .unwrap();
    let id = o
        .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
        .unwrap();
    let p = prepare(&mut o, &id, &s, &mut r);
    request(&mut o, &id, &op, &s, Some(&p), &mut r).unwrap();
    let baseline = o
        .resource_registry_command_facts_v1(&id, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap()
        .as_ref()
        .unwrap()
        .clone();
    change(&mut o, 0, false, false, &mut r);
    let p = snapshot(&mut o, &initial.stream, &s, None, &mut r);
    o.sample_resource_registry_observation_v1(&initial.stream, &s, &p, &f, &mut r)
        .unwrap();
    let usage = o.resource_registry_observation_usage_v1().unwrap();
    assert_eq!(usage.queued, 1);
    assert!(matches!(
        request(&mut o, &id, &op, &s, None, &mut r),
        Ok(ResourceRegistryCommandReplyV1::ObserveExisting { .. })
    ));
    assert_eq!(o.resource_registry_observation_usage_v1().unwrap(), usage);
    assert!(Arc::ptr_eq(
        &baseline,
        o.resource_registry_command_facts_v1(&id, &s, &mut r)
            .unwrap()
            .as_observed
            .unwrap()
            .as_ref()
            .unwrap()
    ));
}

#[test]
fn registry_command_envelopes_and_shared_lifetime_limits_refuse_before_reference_or_native_attempt()
{
    for case in 0..3 {
        let mut l = limits();
        l.max_packet_bytes = 65536;
        l.max_held_bytes = 2097152;
        match case {
            0 => l.max_preparations = 1,
            1 => l.max_packet_bytes = 8192,
            _ => l.max_reply_bytes = size_of::<ResourceRegistryCommandReplyV1>() - 1,
        }
        let (mut o, mut r, s) = fixture(l);
        let f = facts(&s);
        let op = ResourceRegistryCommandOperationV1::Establish;
        assert!(
            o.issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
                .is_err()
        );
        assert_eq!(
            o.resource_registry_observation_usage_v1()
                .unwrap()
                .issued_lifetime,
            0
        );
    }
    let mut l = limits();
    l.max_packet_bytes = 65536;
    l.max_held_bytes = 2097152;
    l.max_issued = 1;
    let (mut o, mut r, s) = fixture(l);
    let f = facts(&s);
    let op = ResourceRegistryCommandOperationV1::Establish;
    let id = o
        .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
        .unwrap();
    let a = addresses(&o);
    let rows = rows(&a, None);
    assert!(
        o.prepare_resource_registry_command_snapshot_v1(
            &id,
            &s,
            &TrustedResourceRegistrySnapshotFactsV1 {
                current: &f,
                rows: &rows,
                capture_policy_current: Some(true),
                visibility_complete: Some(true),
                order_attested: Some(true)
            },
            &mut r
        )
        .is_err()
    );
    o.faulted = true;
    o.retire_resource_registry_command_v1(&id, &cleanup(&s), &mut r)
        .unwrap();
    assert!(o.is_faulted());
    assert_eq!(
        o.resource_registry_observation_usage_v1()
            .unwrap()
            .issued_lifetime,
        1
    );
}

#[test]
fn registry_command_wrong_operation_roles_and_generic_preparation_cleanup_do_not_alias_command() {
    let (mut o, mut r, s) = setup();
    let f = facts(&s);
    let op = ResourceRegistryCommandOperationV1::Establish;
    let id = o
        .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
        .unwrap();
    let p = prepare(&mut o, &id, &s, &mut r);
    let fake_command = ResourceRegistryCommandIdV1 {
        owner: p.owner.clone(),
        slot: p.slot,
    };
    assert!(
        o.resource_registry_command_facts_v1(&fake_command, &s, &mut r)
            .is_err()
    );
    let fake_prep = ResourceRegistryPreparationIdV1 {
        owner: id.owner.clone(),
        slot: id.slot,
    };
    assert!(
        o.retire_resource_registry_preparation_v1(&fake_prep, &cleanup(&s), &mut r)
            .is_err()
    );
    let mut wrong = s.clone();
    wrong.context.recipient = "other".into();
    assert!(request(&mut o, &id, &op, &wrong, Some(&p), &mut r).is_err());
    let other = open(&mut o, &s, None, &mut r);
    let wrong_op = ResourceRegistryCommandOperationV1::Resynchronize {
        stream: other.stream,
    };
    assert!(request(&mut o, &id, &wrong_op, &s, Some(&p), &mut r).is_err());
    assert_eq!(
        o.resource_registry_command_facts_v1(&id, &s, &mut r)
            .unwrap()
            .phase,
        ResourceRegistryCommandPhaseV1::Issued
    );
}

#[test]
fn registry_command_native_returned_unavailable_is_retained_once_not_an_automatic_retry() {
    let (mut o, mut r, s) = setup();
    let f = facts(&s);
    let op = ResourceRegistryCommandOperationV1::Establish;
    let id = o
        .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
        .unwrap();
    let p = prepare(&mut o, &id, &s, &mut r);
    let State::Active(pool) = &mut o.registry_observations else {
        panic!()
    };
    pool.command_fault = Some(FaultCut::NativeRefusal);
    request(&mut o, &id, &op, &s, Some(&p), &mut r).unwrap();
    let host = o
        .resource_registry_command_facts_v1(&id, &s, &mut r)
        .unwrap();
    assert_eq!(host.phase, ResourceRegistryCommandPhaseV1::Settled);
    assert!(host.native_started);
    assert!(matches!(
        host.as_observed,
        Some(Err(ResourceError::Unavailable))
    ));
    let before = o.resource_registry_observation_usage_v1().unwrap();
    assert_eq!(before.streams, 0);
    assert!(matches!(
        request(&mut o, &id, &op, &s, None, &mut r),
        Ok(ResourceRegistryCommandReplyV1::ObserveExisting { .. })
    ));
    assert_eq!(o.resource_registry_observation_usage_v1().unwrap(), before);
}

#[test]
fn registry_command_protocol_fault_preserves_started_uncertainty_and_does_not_fault_editing() {
    for cut in [FaultCut::BeforeNative, FaultCut::AfterNative] {
        let (mut o, mut r, s) = setup();
        let f = facts(&s);
        let op = ResourceRegistryCommandOperationV1::Establish;
        let id = o
            .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
            .unwrap();
        let p = prepare(&mut o, &id, &s, &mut r);
        let State::Active(pool) = &mut o.registry_observations else {
            panic!()
        };
        pool.command_fault = Some(cut);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            request(&mut o, &id, &op, &s, Some(&p), &mut r)
        }))
        .unwrap_err();
        assert!(
            panic
                .downcast_ref::<ResourceRegistryCommandProtocolFaultV1>()
                .is_some()
        );
        let host = o
            .resource_registry_command_facts_v1(&id, &s, &mut r)
            .unwrap();
        assert_eq!(host.phase, ResourceRegistryCommandPhaseV1::Faulted);
        assert_eq!(host.native_started, cut == FaultCut::AfterNative);
        let streams = o.resource_registry_observation_usage_v1().unwrap().streams;
        assert_eq!(streams, usize::from(cut == FaultCut::AfterNative));
        assert!(!o.is_faulted());
        o.create(b"editing remains a separate healthy owner", &mut r)
            .unwrap();
        if cut == FaultCut::AfterNative {
            assert!(
                o.issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
                    .is_err()
            );
            assert!(
                o.retire_resource_registry_command_v1(&id, &cleanup(&s), &mut r)
                    .is_err()
            );
            let fake = ResourceRegistryPreparationIdV1 {
                owner: id.owner.clone(),
                slot: id.slot,
            };
            assert!(
                o.retire_resource_registry_preparation_v1(&fake, &cleanup(&s), &mut r)
                    .is_err()
            );
            let stream = {
                let State::Active(pool) = &o.registry_observations else {
                    panic!()
                };
                ResourceRegistryStreamIdV1 {
                    owner: pool.owner.clone(),
                    slot: 0,
                }
            };
            o.stop_resource_registry_observation_v1(&stream, &cleanup(&s), &mut r)
                .unwrap();
            o.retire_resource_registry_observation_v1(&stream, &cleanup(&s), &mut r)
                .unwrap();
            assert!(
                o.retire_resource_registry_observation_pool_v1(&control(), &mut r)
                    .is_err()
            );
        } else {
            o.retire_resource_registry_command_v1(&id, &cleanup(&s), &mut r)
                .unwrap();
            o.retire_resource_registry_preparation_v1(&p, &cleanup(&s), &mut r)
                .unwrap();
            o.retire_resource_registry_observation_pool_v1(&control(), &mut r)
                .unwrap();
            assert_eq!(o.retention_usage().reserved, 0);
        }
    }
}
