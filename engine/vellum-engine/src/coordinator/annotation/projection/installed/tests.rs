use super::super::tests::{base, cleanup, control, current, facts, limits, ready};
use super::*;
mod shutdown;

#[test]
fn acquisition_validation_capture_never_controls_health_or_issuance() {
    use crate::coordinator::source_query::result_contract::AcquisitionResultFault as F;

    for capacity in [None, Some(64), Some(1)] {
        for malformed in [false, true] {
            let (mut o, mut r, _, s, g) = ready(limits());
            o.configure_annotation_contexts(profile(), &control(), &mut r)
                .unwrap();
            let cut = o
                .prepare_annotation_projection(
                    AnnotationProjectionClassesV1::Combined,
                    &s,
                    &facts(&s, &g),
                    &mut r,
                )
                .unwrap();
            r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            o.acquisition_result_fault = malformed.then_some(F::Context);
            let result = o.install_annotation_context(&cut, &s, &current(&cut, &s), &mut r);
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
                    events
                        .iter()
                        .all(|e| e.service_time.is_none() && e.boundary != Boundary::Publication)
                );
            } else if capacity == Some(1) {
                assert!(r.dropped() > 0);
            } else {
                assert_eq!(r.queued(), 0);
            }
        }
    }
}

#[test]
fn malformed_acquisition_refuses_before_registry_or_preparation_commit() {
    use crate::coordinator::source_query::result_contract::AcquisitionResultFault as F;
    for fault in [F::Context, F::Classes] {
        let (mut o, mut r, _, s, g) = ready(limits());
        o.configure_annotation_contexts(profile(), &control(), &mut r)
            .unwrap();
        let cut = o
            .prepare_annotation_projection(
                AnnotationProjectionClassesV1::Combined,
                &s,
                &facts(&s, &g),
                &mut r,
            )
            .unwrap();
        let before = o.annotation_context_usage();
        let prepared = o.annotation_preparation_usage();
        let reserved = o.retention_usage().reserved;
        let generation = o.generation();
        o.acquisition_result_fault = Some(fault);
        assert!(
            o.install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
                .is_none(),
            "{fault:?}"
        );
        assert!(o.is_faulted());
        assert_eq!(
            o.read_result_invariant(),
            Some(ReadResultInvariantV1::NativeReadAcquisitionResultInvariant)
        );
        assert_eq!(o.annotation_context_usage(), before);
        assert_eq!(o.annotation_preparation_usage(), prepared);
        assert!(o.prepared_annotation_cut(&cut, &s).is_ok());
        assert_eq!(o.retention_usage().reserved, reserved);
        assert_eq!(o.generation(), generation);
        assert!(o.discard_prepared_annotation_projection(&cut, &cleanup(&s), &mut r));
        assert!(o.is_faulted());
    }
}

#[test]
fn combined_hold_count_and_missing_ledger_surprises_fault_without_retirement_or_refund() {
    for fault in 0..3 {
        let (mut o, mut r, _, s, g) = ready(limits());
        o.configure_annotation_contexts(profile(), &control(), &mut r)
            .unwrap();
        let cut = o
            .prepare_annotation_projection(
                AnnotationProjectionClassesV1::Combined,
                &s,
                &facts(&s, &g),
                &mut r,
            )
            .unwrap();
        let id = o
            .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
            .unwrap()
            .context;
        let hold = o
            .hold_annotation_search_source(&id, &s, &eligible(&id, &s), &mut r)
            .unwrap();
        let reserved = o.retention_usage().reserved;
        let InstalledState::Active(p) = &mut o.annotation_contexts else {
            unreachable!()
        };
        match fault {
            0 => p.rows[id.slot].as_mut().unwrap().holds = 0,
            1 => p.holds.clear(),
            _ => p.holds = Vec::new(),
        }
        assert_eq!(
            o.validate_annotation_source_hold(&hold, &id, &s),
            Err(Error::Faulted)
        );
        assert!(o.stop_annotation_context(&id, &cleanup(&s), &mut r));
        assert!(!o.retire_annotation_context(&id, &cleanup(&s), &mut r));
        assert!(o.is_faulted());
        assert_eq!(o.annotation_context_usage().contexts, 1);
        assert_eq!(o.retention_usage().reserved, reserved);
    }
}

#[test]
fn protected_combined_hold_discharge_survives_global_fault_without_refilling_issuance() {
    let (mut o, mut r, _, s, g) = ready(limits());
    o.configure_annotation_contexts(profile(), &control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &facts(&s, &g),
            &mut r,
        )
        .unwrap();
    let id = o
        .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
        .unwrap()
        .context;
    let hold = o
        .hold_annotation_search_source(&id, &s, &eligible(&id, &s), &mut r)
        .unwrap();
    let reserved = o.retention_usage().reserved;
    o.faulted = true;
    assert_eq!(
        o.release_annotation_source_hold(&hold, &cleanup(&s)),
        Ok(())
    );
    assert_eq!(
        o.release_annotation_source_hold(&hold, &cleanup(&s)),
        Err(Error::Unavailable)
    );
    assert_eq!(o.annotation_context_usage().holds, 0);
    assert_eq!(o.annotation_context_usage().issued_lifetime, 2);
    assert_eq!(o.retention_usage().reserved, reserved);
    assert!(o.is_faulted());
}

pub(super) fn profile() -> AnnotationContextLimitsV1 {
    AnnotationContextLimitsV1 {
        max_selectors: 32,
        max_holds: 4,
        max_page_rows: 16,
        max_row_bytes: 4096,
        max_selector_bytes: 128,
        max_contexts: 4,
        max_issued: 8,
        max_rows: 8,
        max_capture_bytes: 8192,
        max_context_bytes: 2048,
        max_request_bytes: 8192,
        max_reply_bytes: 4096,
        max_label_bytes: 128,
        max_pool_bytes: 262144,
    }
}
pub(super) fn eligible<'a>(
    id: &'a AnnotationReadContextId,
    s: &'a SourceReadScopeV1,
) -> TrustedAnnotationContextFactsV1<'a> {
    TrustedAnnotationContextFactsV1 {
        context: id,
        scope: s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        annotation_readable: Some(true),
        source_readable: Some(true),
        projection_current: Some(true),
        retained: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    }
}
#[test]
fn installed_native_capture_scope_request_row_reply_and_label_envelopes_precede_consumption() {
    let (mut probe, mut r, _, s, g) = ready(limits());
    let cut = probe
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &facts(&s, &g),
            &mut r,
        )
        .unwrap();
    let capture = probe
        .prepared_annotation_cut(&cut, &s)
        .unwrap()
        .value
        .bytes()
        .unwrap();
    let context = scope_bytes(&s).unwrap();
    let request = context + std::mem::size_of::<PreparedAnnotationProjectionId>();
    let reply = std::mem::size_of::<AnnotationContextAcquisitionV1>();
    let label = settings::basis::labels(&s.context)
        .into_iter()
        .map(str::len)
        .max()
        .unwrap();
    for (coordinate, bound) in [capture, context, request, 1, reply, label]
        .into_iter()
        .enumerate()
    {
        for delta in [-1isize, 0, 1] {
            let mut p = profile();
            let n = bound.checked_add_signed(delta).unwrap();
            match coordinate {
                0 => p.max_capture_bytes = n,
                1 => p.max_context_bytes = n,
                2 => p.max_request_bytes = n,
                3 => p.max_rows = n,
                4 => p.max_reply_bytes = n,
                _ => p.max_label_bytes = n,
            }
            let (mut o, mut r, _, s, g) = ready(limits());
            o.configure_annotation_contexts(p, &control(), &mut r)
                .unwrap();
            let cut = o
                .prepare_annotation_projection(
                    AnnotationProjectionClassesV1::Combined,
                    &s,
                    &facts(&s, &g),
                    &mut r,
                )
                .unwrap();
            let reserved = o.retention_usage().reserved;
            assert_eq!(
                o.install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
                    .is_some(),
                delta >= 0,
                "coordinate {coordinate} delta {delta}"
            );
            assert_eq!(
                o.annotation_preparation_usage().prepared,
                usize::from(delta < 0)
            );
            assert_eq!(
                o.annotation_context_usage().issued_lifetime,
                usize::from(delta >= 0)
            );
            assert_eq!(o.retention_usage().reserved, reserved);
        }
    }
}
#[test]
fn independent_installed_parent_and_pool_funding_include_workspace_at_zero_context_capacity() {
    for zero in [false, true] {
        for coordinate in 0..2 {
            for delta in [-1isize, 0, 1] {
                let mut p = profile();
                if zero {
                    p.max_contexts = 0;
                }
                let charge = p.required_pool_bytes().unwrap() + "engine".len() + "local".len();
                let prep = limits().required_pool_bytes().unwrap() + "engine".len() + "local".len();
                let n = charge.checked_add_signed(delta).unwrap();
                if coordinate == 0 {
                    p.max_pool_bytes = n;
                }
                let (mut o, mut r, _, _, _) = base();
                o.configure_retention_budget(
                    if coordinate == 1 {
                        prep + n
                    } else {
                        usize::MAX
                    },
                    &mut r,
                )
                .unwrap();
                o.configure_annotation_preparation("engine", "local", limits(), &control(), &mut r)
                    .unwrap();
                assert_eq!(
                    o.configure_annotation_contexts(p, &control(), &mut r),
                    if delta >= 0 {
                        Ok(true)
                    } else {
                        Err(Error::Limit)
                    }
                );
                assert_eq!(
                    o.retention_usage().reserved,
                    prep + if delta >= 0 { charge } else { 0 }
                );
            }
        }
    }
}
#[test]
fn precommit_surprise_or_owned_registry_drift_never_consumes_cut_or_partially_installs_context() {
    for fault in 0..3 {
        let trap = fault == 1;
        let (mut o, mut r, d, s, g) = ready(limits());
        o.configure_annotation_contexts(profile(), &control(), &mut r)
            .unwrap();
        let cut = o
            .prepare_annotation_projection(
                AnnotationProjectionClassesV1::Combined,
                &s,
                &facts(&s, &g),
                &mut r,
            )
            .unwrap();
        let generation = o.generation();
        let reserved = o.retention_usage().reserved;
        if trap {
            o.annotation_context_install_fault = true;
        } else if fault == 0 {
            let InstalledState::Active(p) = &mut o.annotation_contexts else {
                unreachable!()
            };
            p.rows = Vec::new();
        } else {
            let State::Active(p) = &mut o.annotation_preparations else {
                unreachable!()
            };
            let value = &mut p.cuts[cut.slot].as_mut().unwrap().value;
            value.rows.push(Row {
                id: value.rows[0].id.clone(),
                body: None,
                location: None,
            });
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            o.install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
        }));
        assert_eq!(result.is_err(), trap);
        assert!(o.is_faulted());
        assert_eq!(o.generation(), generation);
        assert_eq!(o.annotation_preparation_usage().prepared, 1);
        assert_eq!(o.annotation_context_usage().contexts, 0);
        assert_eq!(o.retention_usage().reserved, reserved);
        assert_eq!(o.document(&d.document).unwrap().source.text(), "ab");
        assert!(o.discard_prepared_annotation_projection(&cut, &cleanup(&s), &mut r));
        assert_eq!(o.retire_annotation_contexts(&control(), &mut r), Ok(true));
        assert!(o.is_faulted());
    }
}
#[test]
fn combined_source_request_reply_and_scan_caps_have_separate_preflight_and_loss_ordering() {
    let (_, _, _, s, _) = base();
    let request = scope_bytes(&s).unwrap()
        + std::mem::size_of::<AnnotationReadContextId>()
        + std::mem::size_of::<SourceContextQueryV1>();
    let reply = std::mem::size_of::<AnnotationSourceReplyV1>() + 1;
    for coordinate in 0..4 {
        for delta in [-1isize, 0, 1] {
            let mut prep = limits();
            let mut p = profile();
            match coordinate {
                0 => p.max_request_bytes = request.checked_add_signed(delta).unwrap(),
                1 => prep.source.max_request_bytes = request.checked_add_signed(delta).unwrap(),
                2 => p.max_reply_bytes = reply.checked_add_signed(delta).unwrap(),
                _ => prep.source.max_reply_bytes = reply.checked_add_signed(delta).unwrap(),
            }
            let (mut o, mut r, _, s, g) = ready(prep);
            o.configure_annotation_contexts(p, &control(), &mut r)
                .unwrap();
            let cut = o
                .prepare_annotation_projection(
                    AnnotationProjectionClassesV1::Combined,
                    &s,
                    &facts(&s, &g),
                    &mut r,
                )
                .unwrap();
            let id = o
                .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
                .unwrap()
                .context;
            let q = SourceContextQueryV1::Range {
                start: BytePosition(0),
                end: BytePosition(1),
            };
            assert_eq!(
                o.query_annotation_context_source(&id, &s, q, &eligible(&id, &s), &mut r)
                    .is_some(),
                delta >= 0
            );
            let mut loss = eligible(&id, &s);
            loss.annotation_readable = Some(false);
            assert!(
                o.query_annotation_context_source(&id, &s, q, &loss, &mut r)
                    .is_none()
            );
            assert_eq!(
                o.annotation_context_usage().stopped,
                usize::from(coordinate >= 2 || delta >= 0)
            );
        }
    }
    let mut prep = limits();
    prep.source.max_scan_bytes = 0;
    let (mut o, mut r, _, s, g) = ready(prep);
    o.configure_annotation_contexts(profile(), &control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &facts(&s, &g),
            &mut r,
        )
        .unwrap();
    let id = o
        .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
        .unwrap()
        .context;
    assert!(
        o.query_annotation_context_source(
            &id,
            &s,
            SourceContextQueryV1::Line { index: 0 },
            &eligible(&id, &s),
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.annotation_context_usage().stopped, 0);
    let mut loss = eligible(&id, &s);
    loss.source_readable = Some(false);
    assert!(
        o.query_annotation_context_source(
            &id,
            &s,
            SourceContextQueryV1::Line { index: 0 },
            &loss,
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.annotation_context_usage().stopped, 1);
}
