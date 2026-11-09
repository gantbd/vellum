use super::*;
#[test]
fn document_read_pins_reject_source_hold_counter_and_orphan_drift() {
    for orphan in [false, true] {
        let (mut o, mut r, s) = configured(limits());
        let context = o
            .capture_source_read_context(&s, &facts(&s), &mut r)
            .unwrap();
        if orphan {
            o.hold_source_read_context(
                &context,
                &s,
                &SourceReadHoldFactsV1 {
                    owner_control_allowed: Some(true),
                    retained: Some(true),
                    resources_ready: Some(true),
                },
                &mut r,
            )
            .unwrap();
        }
        let State::Active(p) = &mut o.source_contexts else {
            panic!()
        };
        if orphan {
            p.rows[context.slot] = None;
        } else {
            p.rows[context.slot].as_mut().unwrap().holds += 1;
        }
        assert_eq!(
            o.document_read_pins_v1(&s.document, &mut r),
            Err(CoordinatorError::Faulted)
        );
    }
}

#[test]
fn acquisition_validation_capture_never_controls_health_or_issuance() {
    use crate::coordinator::source_query::result_contract::AcquisitionResultFault as F;
    for retained in [false, true] {
        for capacity in [None, Some(64), Some(1)] {
            for malformed in [false, true] {
                let (mut o, _, s) = configured(limits());
                let mut r = Recorder::disabled();
                let original = o
                    .capture_source_read_context(&s, &facts(&s), &mut r)
                    .unwrap();
                r = capacity.map_or_else(Recorder::disabled, |n| {
                    Recorder::try_with_capacity(n).unwrap()
                });
                o.acquisition_result_fault = malformed.then_some(F::Context);
                let result = if retained {
                    o.capture_retained_source_read_context(&original, &s, &facts(&s), &mut r)
                } else {
                    o.capture_source_read_context(&s, &facts(&s), &mut r)
                };
                assert_eq!(result.is_none(), malformed);
                assert_eq!(o.is_faulted(), malformed);
                assert_eq!(o.read_result_invariant().is_some(), malformed);
                if capacity == Some(64) {
                    let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                    let checks: Vec<_> = events
                        .iter()
                        .filter(|e| e.boundary == Boundary::ReadAcquisitionResultValidation)
                        .collect();
                    assert_eq!(checks.len(), 1);
                    assert_eq!(
                        checks[0].disposition,
                        if malformed {
                            Disposition::Unknown
                        } else {
                            Disposition::Completed
                        }
                    );
                    assert!(
                        events.iter().all(
                            |e| e.service_time.is_none() && e.boundary != Boundary::Publication
                        )
                    );
                } else if capacity == Some(1) {
                    assert!(r.dropped() > 0);
                } else {
                    assert_eq!(r.queued(), 0);
                }
            }
        }
    }
}
mod retained;
mod search;
mod shutdown;
#[test]
fn malformed_source_acquisition_refuses_before_issuance_for_current_and_retained() {
    use crate::coordinator::source_query::result_contract::AcquisitionResultFault as F;
    for retained in [false, true] {
        for fault in [F::Context, F::RetainedParent] {
            if !retained && matches!(fault, F::RetainedParent) {
                continue;
            }
            let (mut o, mut r, s) = configured(limits());
            let original = o
                .capture_source_read_context(&s, &facts(&s), &mut r)
                .unwrap();
            let before = o.source_read_context_usage();
            let reserved = o.retention_usage().reserved;
            let generation = o.generation();
            o.acquisition_result_fault = Some(fault);
            let result = if retained {
                o.capture_retained_source_read_context(&original, &s, &facts(&s), &mut r)
            } else {
                o.capture_source_read_context(&s, &facts(&s), &mut r)
            };
            assert!(result.is_none(), "{retained}/{fault:?}");
            assert!(o.is_faulted());
            assert_eq!(
                o.read_result_invariant(),
                Some(ReadResultInvariantV1::NativeReadAcquisitionResultInvariant)
            );
            assert_eq!(o.source_read_context_usage(), before);
            assert_eq!(o.retention_usage().reserved, reserved);
            assert_eq!(o.generation(), generation);
            assert!(o.stop_source_read_context(&original, &discharge(&s), &mut r));
            assert!(o.retire_source_read_context(&original, &discharge(&s), &mut r));
            assert!(o.is_faulted());
        }
    }
}
fn limits() -> SourceReadContextLimitsV1 {
    SourceReadContextLimitsV1 {
        max_contexts: 4,
        max_holds: 4,
        max_issued: 8,
        max_root_bytes: 1_048_576,
        max_capture_bytes: 4096,
        max_context_bytes: 2048,
        max_request_bytes: 4096,
        max_reply_bytes: 4096,
        max_label_bytes: 128,
        max_scan_bytes: 4096,
        max_pool_bytes: 1_048_576,
    }
}
fn controls() -> SourceReadPoolControlV1 {
    SourceReadPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn base() -> (SourceCoordinator, Recorder, SourceReadScopeV1) {
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 8,
        max_staged_bytes: 4096,
        source: crate::source::Limits {
            max_source_bytes: 4096,
            max_edits: 8,
            max_edit_payload_bytes: 4096,
        },
    });
    let mut r = Recorder::disabled();
    let d = o.create(b"a\r\nx", &mut r).unwrap();
    let scope = SourceReadScopeV1 {
        document: d.document,
        context: crate::coordinator::scoped_definition::tests::context(),
        policy: "p1".into(),
    };
    (o, r, scope)
}
fn configured(p: SourceReadContextLimitsV1) -> (SourceCoordinator, Recorder, SourceReadScopeV1) {
    let (mut o, mut r, s) = base();
    o.configure_retention_budget(usize::MAX, &mut r).unwrap();
    o.configure_source_read_contexts("engine", "local", p, &controls(), &mut r)
        .unwrap();
    (o, r, s)
}
fn facts(s: &SourceReadScopeV1) -> TrustedSourceReadFactsV1<'_> {
    TrustedSourceReadFactsV1 {
        scope: s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        retained: Some(true),
        source_readable: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    }
}
fn discharge(s: &SourceReadScopeV1) -> SourceReadContextControlV1<'_> {
    SourceReadContextControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn range(end: usize) -> SourceContextQueryV1 {
    SourceContextQueryV1::Range {
        start: BytePosition(0),
        end: BytePosition(end),
    }
}

#[test]
fn current_capture_checks_actual_root_complete_scope_capture_request_and_selector_envelopes_before_issuance()
 {
    let (probe, mut r, s) = base();
    let root = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
    let context = scope_bytes(&s).unwrap();
    let capture =
        std::mem::size_of::<Record>() + context - std::mem::size_of::<SourceReadScopeV1>() + 4;
    let request = context + std::mem::size_of::<DocumentId>();
    let reply = std::mem::size_of::<Option<SourceReadContextId>>();
    let label = settings::basis::labels(&s.context)
        .into_iter()
        .map(str::len)
        .max()
        .unwrap();
    for (coordinate, bound) in [root, context, capture, request, reply, label]
        .into_iter()
        .enumerate()
    {
        for delta in [-1isize, 0, 1] {
            let mut p = limits();
            let bound = bound.checked_add_signed(delta).unwrap();
            match coordinate {
                0 => p.max_root_bytes = bound,
                1 => p.max_context_bytes = bound,
                2 => p.max_capture_bytes = bound,
                3 => p.max_request_bytes = bound,
                4 => p.max_reply_bytes = bound,
                5 => p.max_label_bytes = bound,
                _ => unreachable!(),
            }
            let (mut o, mut r, s) = configured(p);
            let generation = o.generation();
            let reserved = o.retention_usage().reserved;
            let result = o.capture_source_read_context(&s, &facts(&s), &mut r);
            assert_eq!(
                result.is_some(),
                delta >= 0,
                "coordinate {coordinate} delta {delta}"
            );
            assert_eq!(
                o.source_read_context_usage().issued_lifetime,
                usize::from(delta >= 0)
            );
            assert_eq!(o.generation(), generation);
            assert_eq!(o.retention_usage().reserved, reserved);
        }
    }
}

#[test]
fn actual_parent_and_pool_bounds_are_independent_workspace_exists_at_zero_context_capacity_and_issuer_cannot_reopen()
 {
    let p = limits();
    let charge = p.required_pool_bytes().unwrap() + "engine".len() + "local".len();
    for coordinate in 0..2 {
        for delta in [-1isize, 0, 1] {
            let (mut o, mut r, _) = base();
            let mut p = p;
            if coordinate == 0 {
                p.max_pool_bytes = charge.checked_add_signed(delta).unwrap();
            }
            o.configure_retention_budget(
                if coordinate == 1 {
                    charge.checked_add_signed(delta).unwrap()
                } else {
                    usize::MAX
                },
                &mut r,
            )
            .unwrap();
            let result =
                o.configure_source_read_contexts("engine", "local", p, &controls(), &mut r);
            assert_eq!(
                result,
                if delta < 0 {
                    Err(Error::Limit)
                } else {
                    Ok(true)
                }
            );
            assert_eq!(
                o.retention_usage().reserved,
                if delta < 0 { 0 } else { charge }
            );
        }
    }
    let p = SourceReadContextLimitsV1 {
        max_contexts: 0,
        max_holds: 0,
        ..limits()
    };
    let (mut o, mut r, s) = configured(p);
    assert!(o.retention_usage().reserved > 0);
    assert!(
        o.capture_source_read_context(&s, &facts(&s), &mut r)
            .is_none()
    );
    assert_eq!(o.source_read_context_usage().issued_lifetime, 0);
    assert!(o.retire_source_read_contexts(&controls(), &mut r).unwrap());
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(
        o.configure_source_read_contexts("engine", "local", p, &controls(), &mut r),
        Err(Error::Unavailable)
    );
    let p = SourceReadContextLimitsV1 {
        max_issued: usize::MAX,
        ..limits()
    };
    assert_eq!(p.required_pool_bytes(), Err(Error::Limit));
}

#[test]
fn complete_query_envelope_and_delivery_budgets_refuse_without_closure_but_cannot_hide_accepted_loss()
 {
    let (_, _, s) = base();
    let request = scope_bytes(&s).unwrap()
        + std::mem::size_of::<SourceContextQueryV1>()
        + std::mem::size_of::<SourceReadContextId>();
    for coordinate in 0..3 {
        for delta in [-1isize, 0, 1] {
            let mut p = limits();
            match coordinate {
                0 => p.max_request_bytes = request.checked_add_signed(delta).unwrap(),
                1 => {
                    p.max_reply_bytes = (std::mem::size_of::<SourceContextReplyV1>() + 1)
                        .checked_add_signed(delta)
                        .unwrap()
                }
                2 => p.max_scan_bytes = 3usize.checked_add_signed(delta).unwrap(),
                _ => unreachable!(),
            }
            let (mut o, mut r, s) = configured(p);
            let id = o
                .capture_source_read_context(&s, &facts(&s), &mut r)
                .unwrap();
            let query = if coordinate == 2 {
                SourceContextQueryV1::Line { index: 0 }
            } else {
                range(1)
            };
            assert_eq!(
                o.query_source_read_context(&id, &s, query, &facts(&s), &mut r)
                    .is_some(),
                delta >= 0
            );
            assert!(
                !o.source_pool().unwrap().rows[id.slot]
                    .as_ref()
                    .unwrap()
                    .closed
            );
            if coordinate != 0 {
                assert!(
                    o.query_source_read_context(&id, &s, range(0), &facts(&s), &mut r)
                        .is_some()
                );
                let mut lost = facts(&s);
                lost.source_readable = Some(false);
                lost.resources_ready = Some(false);
                assert!(
                    o.query_source_read_context(&id, &s, query, &lost, &mut r)
                        .is_none()
                );
                assert!(
                    o.source_pool().unwrap().rows[id.slot]
                        .as_ref()
                        .unwrap()
                        .closed
                );
            }
        }
    }
}

#[test]
fn owner_fault_invalidates_delivery_but_protected_holds_and_pool_discharge_remain_available() {
    let (mut o, mut r, s) = configured(limits());
    let id = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let hold = o
        .hold_source_read_context(
            &id,
            &s,
            &SourceReadHoldFactsV1 {
                owner_control_allowed: Some(true),
                retained: Some(true),
                resources_ready: Some(true),
            },
            &mut r,
        )
        .unwrap();
    o.faulted = true;
    assert!(
        o.query_source_read_context(&id, &s, range(1), &facts(&s), &mut r)
            .is_none()
    );
    assert_eq!(o.source_read_context_usage().retained_snapshots, 1);
    assert!(o.stop_source_read_context(&id, &discharge(&s), &mut r));
    assert!(o.release_source_read_hold(&hold, &discharge(&s), &mut r));
    assert_eq!(o.source_read_context_usage().retained_snapshots, 0);
    assert!(o.retire_source_read_context(&id, &discharge(&s), &mut r));
    assert!(o.retire_source_read_contexts(&controls(), &mut r).unwrap());
    assert!(o.faulted);
    assert_eq!(o.retention_usage().reserved, 0);
    assert_eq!(o.source_read_context_usage().issued_lifetime, 2);
}

#[test]
fn a_corrupt_hold_counter_cannot_drop_an_actual_snapshot_or_refund_parent_capacity() {
    let (mut o, mut r, s) = configured(limits());
    let id = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let hold = o
        .hold_source_read_context(
            &id,
            &s,
            &SourceReadHoldFactsV1 {
                owner_control_allowed: Some(true),
                retained: Some(true),
                resources_ready: Some(true),
            },
            &mut r,
        )
        .unwrap();
    let State::Active(p) = &mut o.source_contexts else {
        unreachable!()
    };
    p.rows[id.slot].as_mut().unwrap().holds = 0;
    let reserved = o.retention_usage().reserved;
    assert!(!o.release_source_read_hold(&hold, &discharge(&s), &mut r));
    assert!(o.faulted);
    assert!(!o.stop_source_read_context(&id, &discharge(&s), &mut r));
    assert_eq!(o.source_read_context_usage().retained_snapshots, 1);
    assert_eq!(o.source_read_context_usage().holds, 1);
    assert_eq!(o.retention_usage().reserved, reserved);
    // Restore the fixture's original private invariant to exercise protected cleanup;
    // this is not a production recovery API or an owner-health reset.
    let State::Active(p) = &mut o.source_contexts else {
        unreachable!()
    };
    p.rows[id.slot].as_mut().unwrap().holds = 1;
    assert!(o.stop_source_read_context(&id, &discharge(&s), &mut r));
    assert!(o.release_source_read_hold(&hold, &discharge(&s), &mut r));
    assert!(o.retire_source_read_context(&id, &discharge(&s), &mut r));
    assert!(o.retire_source_read_contexts(&controls(), &mut r).unwrap());
    assert!(o.faulted);
}
