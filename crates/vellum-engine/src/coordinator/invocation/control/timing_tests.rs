use super::*;
use crate::instrumentation::NativeTimingProfileV1;

// Every call has an independently bounded observer. Saturation is not a fast
// measured completion: the sentinel stays intact and the missing event is lost.
fn episode(
    mode: usize,
    boundary: Boundary,
    disposition: Disposition,
    call: impl FnOnce(&mut Recorder),
) {
    let mut r = if mode == 0 {
        Recorder::disabled()
    } else {
        Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
    };
    if mode >= 2 {
        r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
    }
    let sentinel = Observation {
        boundary: Boundary::Control,
        disposition: Disposition::Completed,
        service_time: None,
    };
    if mode == 3 {
        r.record(sentinel);
    }
    call(&mut r);
    if mode == 0 {
        assert_eq!(r.queued(), 0);
        assert_eq!(r.dropped(), 0);
    } else if mode == 3 {
        assert_eq!(r.pop_front(), Some(sentinel));
        assert!(r.dropped() > 0);
        assert_eq!(r.queued(), 0);
    } else {
        let mut matches = 0;
        while let Some(event) = r.pop_front() {
            if event.boundary == boundary {
                matches += 1;
                assert_eq!(event.disposition, disposition);
                assert_eq!(event.service_time.is_some(), mode == 2);
            }
        }
        assert_eq!(matches, 1, "one endpoint for the actual service call");
        assert_eq!(r.dropped(), 0);
    }
}

#[test]
fn native_query_and_control_timing_preserves_issued_and_retired_meanings() {
    for mode in 0..4 {
        let (mut o, mut r, _, pending) = tests::queued(0);
        let id = o.issue_invocation(&mut r).unwrap();
        let generation = o.generation();
        episode(mode, Boundary::Outcome, Disposition::Completed, |r| {
            assert_eq!(o.invocation_state(&id, r), Ok(InvocationState::Issued));
        });
        episode(mode, Boundary::Outcome, Disposition::Refused, |r| {
            assert_eq!(
                o.invocation_outcome(&id, r),
                Err(InvocationError::OutcomeUnavailable)
            );
        });
        episode(mode, Boundary::Control, Disposition::Refused, |r| {
            assert_eq!(
                o.forget_invocation_outcome(&id, r),
                Err(InvocationError::Pending)
            );
        });
        episode(mode, Boundary::Control, Disposition::Refused, |r| {
            assert_eq!(
                o.stop_invocation(&id, r),
                Err(InvocationError::NotSubmitted)
            );
        });
        episode(mode, Boundary::Control, Disposition::Completed, |r| {
            assert_eq!(o.retire_invocation(&id, r), Ok(()));
        });
        episode(mode, Boundary::Outcome, Disposition::Refused, |r| {
            assert_eq!(
                o.invocation_state(&id, r),
                Err(InvocationError::Unavailable)
            );
        });
        assert_eq!(o.generation(), generation);
        assert_eq!(
            o.invocation_state(&pending, &mut r),
            Ok(InvocationState::Pending)
        );
    }
}

#[test]
fn native_control_timing_preserves_pending_stop_forget_and_one_time_retirement() {
    for mode in 0..4 {
        let (mut o, mut r, _, id) = tests::queued(0);
        let generation = o.generation();
        episode(mode, Boundary::Control, Disposition::Refused, |r| {
            assert_eq!(
                o.forget_invocation_outcome(&id, r),
                Err(InvocationError::Pending)
            );
        });
        episode(mode, Boundary::Control, Disposition::Refused, |r| {
            assert_eq!(o.retire_invocation(&id, r), Err(InvocationError::Pending));
        });
        for _ in 0..2 {
            episode(mode, Boundary::Control, Disposition::Completed, |r| {
                assert_eq!(o.stop_invocation(&id, r), Ok(()));
            });
        }
        episode(mode, Boundary::Outcome, Disposition::Completed, |r| {
            assert_eq!(
                o.invocation_outcome(&id, r),
                Ok(&InvocationOutcome::StoppedBeforePublication)
            );
        });
        for _ in 0..2 {
            episode(mode, Boundary::Control, Disposition::Completed, |r| {
                assert_eq!(o.forget_invocation_outcome(&id, r), Ok(()));
            });
        }
        episode(mode, Boundary::Outcome, Disposition::Refused, |r| {
            assert_eq!(
                o.invocation_outcome(&id, r),
                Err(InvocationError::OutcomeUnavailable)
            );
        });
        episode(mode, Boundary::Control, Disposition::Completed, |r| {
            assert_eq!(o.retire_invocation(&id, r), Ok(()));
        });
        episode(mode, Boundary::Control, Disposition::Refused, |r| {
            assert_eq!(
                o.retire_invocation(&id, r),
                Err(InvocationError::Unavailable)
            );
        });
        assert_eq!(o.generation(), generation);
        assert_eq!(o.invocation_usage().retained, 0);
        assert_eq!(o.run(&id, &mut r), Err(InvocationError::Unavailable));
    }
}

#[test]
fn native_control_timing_keeps_unknown_running_jobs_unknown() {
    for mode in 0..4 {
        let (mut o, _, _, id) = tests::queued(0);
        o.invocations.slots[id.slot].as_mut().unwrap().phase = Phase::Running;
        episode(mode, Boundary::Outcome, Disposition::Unknown, |r| {
            assert_eq!(o.invocation_state(&id, r), Err(InvocationError::Faulted));
        });
        episode(mode, Boundary::Outcome, Disposition::Unknown, |r| {
            assert_eq!(o.invocation_outcome(&id, r), Err(InvocationError::Faulted));
        });
        episode(mode, Boundary::Control, Disposition::Unknown, |r| {
            assert_eq!(o.stop_invocation(&id, r), Err(InvocationError::Faulted));
        });
        episode(mode, Boundary::Control, Disposition::Unknown, |r| {
            assert_eq!(
                o.forget_invocation_outcome(&id, r),
                Err(InvocationError::Faulted)
            );
        });
        episode(mode, Boundary::Control, Disposition::Unknown, |r| {
            assert_eq!(o.retire_invocation(&id, r), Err(InvocationError::Faulted));
        });
        assert!(matches!(
            o.invocations.slots[id.slot].as_ref().unwrap().phase,
            Phase::Running
        ));
    }
}

#[test]
fn protected_stop_timing_separates_request_from_actual_settlement_for_all_scoped_jobs() {
    for kind in 0..4 {
        for mode in 0..4 {
            let (mut o, _, c, id) = tests::queued(kind);
            let generation = o.generation();
            let mut f = tests::control(&c);
            f.owner_control_allowed = None;
            episode(
                mode,
                Boundary::InvocationStopRequest,
                Disposition::Refused,
                |r| {
                    assert_eq!(
                        o.request_invocation_stop(&id, &f, r),
                        Err(InvocationError::Unavailable)
                    );
                },
            );
            episode(
                mode,
                Boundary::InvocationStopRequest,
                Disposition::Completed,
                |r| {
                    assert_eq!(
                        o.request_invocation_stop(&id, &tests::control(&c), r),
                        Ok(())
                    );
                },
            );
            assert!(matches!(
                o.invocations.slots[id.slot].as_ref().unwrap().phase,
                Phase::ScopedSettingsPending(_)
                    | Phase::ScopedDefinitionPending(_)
                    | Phase::ScopedLifecyclePending(_)
                    | Phase::SerializationPending(_)
            ));
            episode(
                mode,
                Boundary::InvocationStoppedSettlement,
                Disposition::Completed,
                |r| {
                    assert_eq!(o.run(&id, r), Ok(InvocationState::Finished));
                },
            );
            assert_eq!(o.generation(), generation);
        }
    }
}

#[test]
fn protected_settlement_timing_never_turns_a_broken_witness_into_stopped_success() {
    for mode in 0..4 {
        let (mut o, mut r, c, id) = tests::queued(0);
        o.request_invocation_stop(&id, &tests::control(&c), &mut r)
            .unwrap();
        let (foreign, _, _, foreign_id) = tests::queued(0);
        let Some(BoundIntent::ScopedSettings(foreign_capture)) = &foreign.invocations.slots
            [foreign_id.slot]
            .as_ref()
            .unwrap()
            .intent
        else {
            unreachable!()
        };
        let Phase::ScopedSettingsPending(p) =
            &mut o.invocations.slots[id.slot].as_mut().unwrap().phase
        else {
            unreachable!()
        };
        p.capture = foreign_capture.clone();
        let generation = o.generation();
        episode(
            mode,
            Boundary::InvocationStoppedSettlement,
            Disposition::Unknown,
            |r| {
                assert_eq!(o.run(&id, r), Err(InvocationError::Faulted));
            },
        );
        assert!(o.faulted);
        assert_eq!(o.generation(), generation);
        assert_eq!(
            o.invocation_state(&id, &mut r),
            Ok(InvocationState::Pending)
        );
        assert_eq!(
            o.invocation_outcome(&id, &mut r),
            Err(InvocationError::OutcomeUnavailable)
        );
    }
}

#[test]
fn no_stop_poll_and_already_settled_stop_have_no_settlement_episode() {
    let (mut o, mut setup, c, id) = tests::queued(0);
    let mut r = Recorder::try_with_capacity(4)
        .unwrap()
        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
    assert_eq!(o.service_requested_stop(&id, &mut r), Ok(false));
    assert_eq!(r.queued(), 0);
    o.request_invocation_stop(&id, &tests::control(&c), &mut setup)
        .unwrap();
    assert_eq!(o.service_requested_stop(&id, &mut r), Ok(true));
    assert!(r.pop_front().unwrap().service_time.is_some());
    assert_eq!(o.service_requested_stop(&id, &mut r), Ok(false));
    assert_eq!(r.queued(), 0);
}
