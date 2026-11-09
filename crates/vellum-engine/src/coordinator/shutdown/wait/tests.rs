use super::*;
use crate::coordinator::shutdown::tests::{context, facts, limits};

fn fenced() -> (SourceCoordinator, Recorder, ShutdownFenceIdV1) {
    let (mut o, mut r, _, _) = resource::source_save_test_fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let c = context();
    let f = facts(&c);
    let basis = o
        .capture_shutdown_fence_basis_v1(limits(), &f, &mut r)
        .unwrap();
    let fence = o.install_shutdown_fence_v1(&basis, &f, &mut r).unwrap();
    (o, r, fence)
}

#[test]
fn wait_controls_require_original_identity_current_authority_and_exact_child_credit() {
    let (mut o, mut r, fence) = fenced();
    let (_, _, foreign) = fenced();
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let wf = TrustedShutdownWaitFactsV1 {
        inventory: &f,
        clock_valid: Some(true),
    };
    o.start_shutdown_wait_v1(&fence, request(), &wf, &mut r)
        .unwrap();
    let held = o.retention_usage();
    assert_eq!(
        o.advance_shutdown_wait_v1(&foreign, ShutdownWaitEventV1::End, &wf, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    for denied in [None, Some(false)] {
        for which in 0..6 {
            let mut denied_control = facts(&c);
            match which {
                0 => denied_control.authorized = denied,
                1 => denied_control.scope_current = denied,
                2 => denied_control.continuous = denied,
                3 => denied_control.owner_control_allowed = denied,
                4 => denied_control.admission_fence_allowed = denied,
                _ => denied_control.resources_ready = denied,
            }
            let df = TrustedShutdownInventoryFactsV1 {
                control: &denied_control,
                inventory_readable: Some(true),
            };
            let dw = TrustedShutdownWaitFactsV1 {
                inventory: &df,
                clock_valid: Some(true),
            };
            assert_eq!(
                o.advance_shutdown_wait_v1(&fence, ShutdownWaitEventV1::End, &dw, &mut r),
                Err(CoordinatorError::Unavailable)
            );
            assert_eq!(o.retention_usage(), held);
        }
    }
    // The aggregate ledger being large enough does not make an unrelated
    // zero-sized reservation the wait's original report envelope.
    let substitute = o.retention.reserve(0).unwrap();
    let State::Active(wait) = &mut o.shutdown.fence.as_mut().unwrap().wait else {
        panic!()
    };
    let _original = std::mem::replace(&mut wait.reservation, substitute);
    assert_eq!(
        o.advance_shutdown_wait_v1(&fence, ShutdownWaitEventV1::End, &wf, &mut r),
        Err(CoordinatorError::Faulted)
    );
    assert_eq!(o.retention_usage(), held);
    assert!(o.is_faulted());
}

#[test]
fn wait_execution_is_identical_with_capture_off_timed_or_saturated() {
    use crate::instrumentation::{Disposition, NativeTimingProfileV1};
    for mode in 0..4 {
        let (mut o, _, fence) = fenced();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        let c = context();
        let control = facts(&c);
        let f = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(true),
        };
        let wf = TrustedShutdownWaitFactsV1 {
            inventory: &f,
            clock_valid: Some(true),
        };
        let before = (o.generation(), o.resource_usage(), o.retention_usage());
        o.start_shutdown_wait_v1(&fence, request(), &wf, &mut r)
            .unwrap();
        assert_eq!(
            o.advance_shutdown_wait_v1(&fence, ShutdownWaitEventV1::End, &wf, &mut r),
            Ok(ShutdownWaitStatusV1::Cutoff {
                reason: ShutdownWaitCutoffReasonV1::AuthorizedWaitEnded,
                remaining_ticks: Some(3),
                report: ShutdownCutoffReportStateV1::Available,
            })
        );
        assert_eq!(
            o.read_shutdown_wait_cutoff_v1(&fence, &f, &mut r)
                .unwrap()
                .documents
                .len(),
            2
        );
        o.faulted = true; // Historical release/protected cleanup do not heal editing.
        o.read_shutdown_wait_cutoff_v1(&fence, &f, &mut r).unwrap();
        o.retire_shutdown_wait_v1(&fence, &control, Some(true), Some(true), &mut r)
            .unwrap();
        assert!(o.is_faulted());
        assert_eq!(
            (o.generation(), o.resource_usage(), o.retention_usage()),
            before
        );
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if mode == 0 {
            assert!(events.is_empty());
        }
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
        if mode == 1 || mode == 2 {
            for boundary in [
                Boundary::ShutdownWaitStart,
                Boundary::ShutdownWaitAdvance,
                Boundary::ShutdownWaitCutoffRelease,
                Boundary::ShutdownWaitRetirement,
            ] {
                assert!(events.iter().any(|e| e.boundary == boundary
                    && e.disposition == Disposition::Completed
                    && e.service_time.is_some() == (mode == 2)));
            }
        }
    }
}

#[test]
fn polling_retry_and_identical_ticks_cannot_extend_wait_and_cutoff_is_absorbing() {
    let (mut o, mut r, fence) = fenced();
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let wf = TrustedShutdownWaitFactsV1 {
        inventory: &f,
        clock_valid: Some(true),
    };
    let req = request();
    o.start_shutdown_wait_v1(&fence, req, &wf, &mut r).unwrap();
    let held = o.retention_usage();
    let generation = o.generation();
    let waiting = ShutdownWaitStatusV1::Waiting { remaining_ticks: 3 };
    for _ in 0..4 {
        assert_eq!(
            o.advance_shutdown_wait_v1(&fence, ShutdownWaitEventV1::Poll, &wf, &mut r),
            Ok(waiting)
        );
        assert_eq!(
            o.advance_shutdown_wait_v1(&fence, ShutdownWaitEventV1::Tick(req.clock), &wf, &mut r),
            Ok(waiting)
        );
    }
    let halfway = ShutdownWaitStatusV1::Waiting { remaining_ticks: 2 };
    assert_eq!(
        o.advance_shutdown_wait_v1(
            &fence,
            ShutdownWaitEventV1::Tick(ShutdownWaitClockV1 { epoch: 1, tick: 11 }),
            &wf,
            &mut r
        ),
        Ok(halfway)
    );
    assert_eq!(
        o.start_shutdown_wait_v1(&fence, req, &wf, &mut r),
        Ok(halfway)
    );
    let changed = ShutdownWaitRequestV1 {
        budget_ticks: 99,
        ..req
    };
    assert_eq!(
        o.start_shutdown_wait_v1(&fence, changed, &wf, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(
        o.retire_shutdown_wait_v1(&fence, &control, Some(true), Some(true), &mut r),
        Err(CoordinatorError::Unavailable)
    );
    let ended = ShutdownWaitStatusV1::Cutoff {
        reason: ShutdownWaitCutoffReasonV1::BudgetEnded,
        remaining_ticks: Some(0),
        report: ShutdownCutoffReportStateV1::Available,
    };
    assert_eq!(
        o.advance_shutdown_wait_v1(
            &fence,
            ShutdownWaitEventV1::Tick(ShutdownWaitClockV1 { epoch: 1, tick: 13 }),
            &wf,
            &mut r
        ),
        Ok(ended)
    );
    for event in [
        ShutdownWaitEventV1::Poll,
        ShutdownWaitEventV1::End,
        ShutdownWaitEventV1::Tick(req.clock),
    ] {
        assert_eq!(
            o.advance_shutdown_wait_v1(&fence, event, &wf, &mut r),
            Ok(ended)
        );
    }
    let view = o.read_shutdown_wait_cutoff_v1(&fence, &f, &mut r).unwrap();
    assert_eq!(view.documents.len(), 2);
    assert!(view.coverage.all_owner_obligations);
    assert_eq!(o.generation(), generation);
    assert_eq!(o.retention_usage(), held);
    for (retained, drained) in [
        (None, Some(true)),
        (Some(false), Some(true)),
        (Some(true), None),
        (Some(true), Some(false)),
    ] {
        assert_eq!(
            o.retire_shutdown_wait_v1(&fence, &control, retained, drained, &mut r),
            Err(CoordinatorError::Unavailable)
        );
        assert_eq!(o.retention_usage(), held);
    }
    o.retire_shutdown_wait_v1(&fence, &control, Some(true), Some(true), &mut r)
        .unwrap();
    assert_eq!(
        o.retention_usage().reserved,
        held.reserved - req.required_retained_bytes().unwrap()
    );
    assert_eq!(
        o.retire_shutdown_wait_v1(&fence, &control, Some(true), Some(true), &mut r),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(
        o.start_shutdown_wait_v1(&fence, req, &wf, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(
        o.create(b"still fenced", &mut r),
        Err(CoordinatorError::Unavailable)
    );
    assert!(!o.is_faulted());
}

#[test]
fn invalid_clock_ends_wait_without_releasing_owners_and_overflow_is_not_a_deadline() {
    for (clock, valid) in [
        (ShutdownWaitClockV1 { epoch: 2, tick: 11 }, Some(true)),
        (ShutdownWaitClockV1 { epoch: 1, tick: 9 }, Some(true)),
        (request().clock, None),
        (request().clock, Some(false)),
    ] {
        let (mut o, mut r, fence) = fenced();
        let c = context();
        let control = facts(&c);
        let f = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(true),
        };
        let wf = TrustedShutdownWaitFactsV1 {
            inventory: &f,
            clock_valid: Some(true),
        };
        o.start_shutdown_wait_v1(&fence, request(), &wf, &mut r)
            .unwrap();
        let before = (o.generation(), o.resource_usage(), o.retention_usage());
        let invalid = TrustedShutdownWaitFactsV1 {
            inventory: &f,
            clock_valid: valid,
        };
        assert_eq!(
            o.advance_shutdown_wait_v1(&fence, ShutdownWaitEventV1::Tick(clock), &invalid, &mut r),
            Ok(ShutdownWaitStatusV1::Cutoff {
                reason: ShutdownWaitCutoffReasonV1::ClockUnavailable,
                remaining_ticks: None,
                report: ShutdownCutoffReportStateV1::Available,
            })
        );
        assert_eq!(
            (o.generation(), o.resource_usage(), o.retention_usage()),
            before
        );
    }
    for immediate in [false, true] {
        let (mut o, mut r, fence) = fenced();
        let c = context();
        let control = facts(&c);
        let f = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(true),
        };
        let wf = TrustedShutdownWaitFactsV1 {
            inventory: &f,
            clock_valid: Some(true),
        };
        let req = ShutdownWaitRequestV1 {
            clock: ShutdownWaitClockV1 {
                epoch: 1,
                tick: u64::MAX - 1,
            },
            budget_ticks: if immediate { 0 } else { u64::MAX },
            ..request()
        };
        let status = o.start_shutdown_wait_v1(&fence, req, &wf, &mut r).unwrap();
        if immediate {
            assert!(matches!(
                status,
                ShutdownWaitStatusV1::Cutoff {
                    reason: ShutdownWaitCutoffReasonV1::BudgetEnded,
                    remaining_ticks: Some(0),
                    ..
                }
            ));
            o.read_shutdown_wait_cutoff_v1(&fence, &f, &mut r).unwrap();
        } else {
            assert_eq!(
                o.advance_shutdown_wait_v1(
                    &fence,
                    ShutdownWaitEventV1::Tick(ShutdownWaitClockV1 {
                        epoch: 1,
                        tick: u64::MAX
                    }),
                    &wf,
                    &mut r
                ),
                Ok(ShutdownWaitStatusV1::Waiting {
                    remaining_ticks: u64::MAX - 1
                })
            );
        }
    }
}

#[test]
fn preflight_and_exact_parent_headroom_refuse_before_consuming_wait_eligibility() {
    for short_parent in [false, true] {
        let (mut o, mut r, _, _) = resource::source_save_test_fixture();
        let charge = limits().required_capture_bytes().unwrap();
        o.configure_retention_budget(
            charge + request().required_retained_bytes().unwrap() - usize::from(short_parent),
            &mut r,
        )
        .unwrap();
        let c = context();
        let control = facts(&c);
        let f = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(true),
        };
        let wf = TrustedShutdownWaitFactsV1 {
            inventory: &f,
            clock_valid: Some(true),
        };
        let basis = o
            .capture_shutdown_fence_basis_v1(limits(), &control, &mut r)
            .unwrap();
        let fence = o
            .install_shutdown_fence_v1(&basis, &control, &mut r)
            .unwrap();
        let before = o.retention_usage();
        let mut short = request();
        short.report_limits.max_owners = 11;
        assert_eq!(
            o.start_shutdown_wait_v1(&fence, short, &wf, &mut r),
            Err(CoordinatorError::Limit)
        );
        assert_eq!(o.retention_usage(), before);
        let result = o.start_shutdown_wait_v1(&fence, request(), &wf, &mut r);
        assert_eq!(result.is_ok(), !short_parent);
        if short_parent {
            assert_eq!(result, Err(CoordinatorError::Limit));
            assert_eq!(o.retention_usage(), before);
        }
        assert!(!o.is_faulted());
    }
}

#[test]
fn protected_end_survives_fault_and_revocation_without_inventing_or_refreshing_a_report() {
    for faulted in [false, true] {
        let (mut o, mut r, fence) = fenced();
        let c = context();
        let control = facts(&c);
        let f = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(true),
        };
        let wf = TrustedShutdownWaitFactsV1 {
            inventory: &f,
            clock_valid: Some(true),
        };
        o.start_shutdown_wait_v1(&fence, request(), &wf, &mut r)
            .unwrap();
        let before = o.retention_usage();
        o.faulted = faulted;
        o.generation = u64::MAX;
        let revoked = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(false),
        };
        let end = TrustedShutdownWaitFactsV1 {
            inventory: if faulted { &f } else { &revoked },
            clock_valid: None,
        };
        let error = if faulted {
            CoordinatorError::Faulted
        } else {
            CoordinatorError::Unavailable
        };
        assert_eq!(
            o.advance_shutdown_wait_v1(&fence, ShutdownWaitEventV1::End, &end, &mut r),
            Ok(ShutdownWaitStatusV1::Cutoff {
                reason: ShutdownWaitCutoffReasonV1::AuthorizedWaitEnded,
                remaining_ticks: Some(3),
                report: ShutdownCutoffReportStateV1::Unavailable(error),
            })
        );
        assert!(matches!(
            o.read_shutdown_wait_cutoff_v1(&fence, &revoked, &mut r),
            Err(CoordinatorError::Unavailable)
        ));
        assert!(matches!(o.read_shutdown_wait_cutoff_v1(&fence, &f, &mut r), Err(e) if e == error));
        assert_eq!(o.retention_usage(), before);
        o.retire_shutdown_wait_v1(&fence, &control, Some(true), Some(true), &mut r)
            .unwrap();
        assert_eq!(o.is_faulted(), faulted);
        assert_eq!(o.generation(), u64::MAX);
        assert_eq!(
            o.retention_usage().reserved,
            before.reserved - request().required_retained_bytes().unwrap()
        );
    }
}

fn request() -> ShutdownWaitRequestV1 {
    ShutdownWaitRequestV1 {
        clock: ShutdownWaitClockV1 { epoch: 1, tick: 10 },
        budget_ticks: 3,
        report_limits: ShutdownInventoryLimitsV1 {
            max_documents: 16,
            max_writes: 16,
            max_artifacts: 16,
            max_bindings: 16,
            max_reads: 16,
            max_owners: 64,
            max_attachments: 16,
            max_dependencies: 128,
            max_work: 1024,
            max_capture_bytes: 65536,
        },
    }
}
#[test]
fn a_fenced_host_can_start_one_funded_wait_without_changing_or_stopping_work() {
    let (mut o, mut r, _, _) = resource::source_save_test_fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let wf = TrustedShutdownWaitFactsV1 {
        inventory: &f,
        clock_valid: Some(true),
    };
    let basis = o
        .capture_shutdown_fence_basis_v1(limits(), &control, &mut r)
        .unwrap();
    let fence = o
        .install_shutdown_fence_v1(&basis, &control, &mut r)
        .unwrap();
    let before = (o.generation(), o.resource_usage(), o.retention_usage());
    assert_eq!(
        o.start_shutdown_wait_v1(&fence, request(), &wf, &mut r),
        Ok(ShutdownWaitStatusV1::Waiting { remaining_ticks: 3 })
    );
    assert_eq!((o.generation(), o.resource_usage()), (before.0, before.1));
    assert_eq!(
        o.retention_usage().reserved,
        before.2.reserved + request().required_retained_bytes().unwrap()
    );
}
