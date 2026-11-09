use super::*;
#[test]
fn retained_acquisition_copies_the_original_cut_and_survives_original_stop_and_retirement() {
    let (mut o, mut r, d, s) = ready(b"ab");
    let original = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 1,
                    expected: "a".into(),
                    inserted: "B".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(edit, &mut r).unwrap();
    let generation = o.generation();
    let retained = o
        .capture_retained_source_read_context(&original, &s, &facts(&s), &mut r)
        .unwrap();
    assert_ne!(retained, original);
    assert_eq!(o.generation(), generation);
    assert_eq!(o.source_read_context_usage().holds, 0);
    assert!(o.stop_source_read_context(&original, &controls(&s), &mut r));
    assert!(o.retire_source_read_context(&original, &controls(&s), &mut r));
    assert!(
        o.capture_retained_source_read_context(&original, &s, &facts(&s), &mut r)
            .is_none()
    );
    let result = o
        .query_source_read_context(&retained, &s, one_byte(), &facts(&s), &mut r)
        .unwrap();
    let SourceContextContentV1::Range(range) = result.content else {
        panic!("wrong product")
    };
    assert_eq!(range.bytes(), b"a");
    assert_eq!(range.source, d);
    let stale = o.prepare(
        &[DocumentEdits {
            expected: range.source.clone(),
            edits: vec![vellum_engine::source::Replacement {
                start: 0,
                end: 1,
                expected: "a".into(),
                inserted: "Z".into(),
            }],
        }],
        &[],
        &mut r,
    );
    assert_eq!(stale.err(), Some(CoordinatorError::Stale));
    let current = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let result = o
        .query_source_read_context(&current, &s, one_byte(), &facts(&s), &mut r)
        .unwrap();
    let SourceContextContentV1::Range(range) = result.content else {
        panic!("wrong product")
    };
    assert_eq!(range.bytes(), b"B");
}

#[test]
fn accepted_retained_loss_is_absorbing_even_when_new_context_capacity_or_lifetime_issuance_is_exhausted()
 {
    for dimension in 0..3 {
        for index in [0, 1, 2, 3, 5, 6] {
            let mut p = profile();
            if dimension != 1 {
                p.max_contexts = 1;
            }
            if dimension != 0 {
                p.max_issued = 1;
            }
            let (mut o, mut r, _, s) = ready_with_profile(b"ab", p);
            let original = o
                .capture_source_read_context(&s, &facts(&s), &mut r)
                .unwrap();
            let reserved = o.retention_usage().reserved;
            let mut lost = facts(&s);
            set_fact(&mut lost, index, Some(false));
            lost.resources_ready = Some(false);
            assert!(
                o.capture_retained_source_read_context(&original, &s, &lost, &mut r)
                    .is_none()
            );
            assert!(
                o.query_source_read_context(&original, &s, one_byte(), &facts(&s), &mut r)
                    .is_none()
            );
            assert_eq!(o.source_read_context_usage().retained_snapshots, 0);
            assert_eq!(o.source_read_context_usage().issued_lifetime, 1);
            assert_eq!(o.retention_usage().reserved, reserved);
            assert!(!o.is_faulted());
        }
    }
}

#[test]
fn unknown_and_temporary_retained_refusals_do_not_close_or_issue_and_explicit_retry_can_copy() {
    for mode in 0..2 {
        for index in 0..10 {
            if mode == 1 && ![4, 7, 8, 9].contains(&index) {
                continue;
            }
            let (mut o, mut r, _, s) = ready(b"ab");
            let original = o
                .capture_source_read_context(&s, &facts(&s), &mut r)
                .unwrap();
            let mut f = facts(&s);
            set_fact(&mut f, index, if mode == 0 { None } else { Some(false) });
            assert!(
                o.capture_retained_source_read_context(&original, &s, &f, &mut r)
                    .is_none()
            );
            assert_eq!(o.source_read_context_usage().issued_lifetime, 1);
            assert!(
                o.query_source_read_context(&original, &s, one_byte(), &facts(&s), &mut r)
                    .is_some()
            );
            assert!(
                o.capture_retained_source_read_context(&original, &s, &facts(&s), &mut r)
                    .is_some()
            );
            assert_eq!(o.source_read_context_usage().issued_lifetime, 2);
            assert_eq!(o.source_read_context_usage().holds, 0);
        }
    }
}

#[test]
fn foreign_or_mismatched_retained_requests_do_not_poison_the_original_frontier() {
    let (mut o, mut r, _, s) = ready_with_profile(
        b"ab",
        SourceReadContextLimitsV1 {
            max_contexts: 1,
            ..profile()
        },
    );
    let original = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    for index in 0..9 {
        let mut wrong = s.clone();
        match index {
            0 => wrong.context.authority = "other".into(),
            1 => wrong.context.engine = "other".into(),
            2 => wrong.context.recipient = "other".into(),
            3 => wrong.context.purpose = "other".into(),
            4 => wrong.context.destination = "other".into(),
            5 => wrong.context.profile = "other".into(),
            6 => wrong.context.continuity = "other".into(),
            7 => wrong.policy = "other".into(),
            8 => wrong.document = o.create(b"other", &mut r).unwrap().document,
            _ => unreachable!(),
        }
        let mut lost = facts(&wrong);
        lost.source_readable = Some(false);
        assert!(
            o.capture_retained_source_read_context(&original, &wrong, &lost, &mut r)
                .is_none()
        );
        assert!(
            o.capture_retained_source_read_context(&original, &s, &lost, &mut r)
                .is_none()
        );
        assert!(
            o.query_source_read_context(&original, &s, one_byte(), &facts(&s), &mut r)
                .is_some()
        );
        assert!(!o.is_faulted());
    }
    let (mut foreign, mut rr, _, _) = ready(b"foreign");
    assert!(
        foreign
            .capture_retained_source_read_context(&original, &s, &facts(&s), &mut rr)
            .is_none()
    );
    assert_eq!(foreign.source_read_context_usage().issued_lifetime, 0);
    assert!(!foreign.is_faulted());
    assert_eq!(o.source_read_context_usage().issued_lifetime, 1);
}

#[test]
fn held_stopped_or_disposed_source_data_is_not_reacquisition_authority() {
    for disposed in [false, true] {
        let (mut o, mut r, d, s) = ready(b"ab");
        let original = o
            .capture_source_read_context(&s, &facts(&s), &mut r)
            .unwrap();
        let hold = o
            .hold_source_read_context(
                &original,
                &s,
                &SourceReadHoldFactsV1 {
                    owner_control_allowed: Some(true),
                    retained: Some(true),
                    resources_ready: Some(true),
                },
                &mut r,
            )
            .unwrap();
        if disposed {
            let plan = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
            o.commit_retained_disposal(plan, &mut r).unwrap();
        } else {
            o.stop_source_read_context(&original, &controls(&s), &mut r);
        }
        assert!(
            o.capture_retained_source_read_context(&original, &s, &facts(&s), &mut r)
                .is_none()
        );
        assert_eq!(o.source_read_context_usage().retained_snapshots, 1);
        assert_eq!(o.source_read_context_usage().issued_lifetime, 2);
        assert!(o.release_source_read_hold(&hold, &controls(&s), &mut r));
        assert_eq!(o.source_read_context_usage().retained_snapshots, 0);
        assert!(o.retire_source_read_context(&original, &controls(&s), &mut r));
    }
}

#[test]
fn retained_copy_chains_spend_shared_issuance_without_implicitly_pinning_or_refunding_original_rows()
 {
    let (mut o, mut r, _, s) = ready_with_profile(
        b"ab",
        SourceReadContextLimitsV1 {
            max_contexts: 2,
            max_holds: 0,
            max_issued: 4,
            ..profile()
        },
    );
    let mut current = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let reserved = o.retention_usage().reserved;
    for issued in 2..=4 {
        let child = o
            .capture_retained_source_read_context(&current, &s, &facts(&s), &mut r)
            .unwrap();
        assert_ne!(child, current);
        assert!(o.stop_source_read_context(&current, &controls(&s), &mut r));
        assert!(o.retire_source_read_context(&current, &controls(&s), &mut r));
        assert_eq!(o.source_read_context_usage().contexts, 1);
        assert_eq!(o.source_read_context_usage().holds, 0);
        assert_eq!(o.source_read_context_usage().issued_lifetime, issued);
        current = child;
    }
    assert!(
        o.capture_retained_source_read_context(&current, &s, &facts(&s), &mut r)
            .is_none()
    );
    assert!(
        o.query_source_read_context(&current, &s, one_byte(), &facts(&s), &mut r)
            .is_some()
    );
    assert_eq!(o.retention_usage().reserved, reserved);
    o.stop_source_read_context(&current, &controls(&s), &mut r);
    assert!(o.retire_source_read_context(&current, &controls(&s), &mut r));
    o.retire_source_read_contexts(&pool_control(), &mut r)
        .unwrap();
    assert_eq!(o.source_read_context_usage().issued_lifetime, 4);
    assert_eq!(o.retention_usage().reserved, 0);
}

#[test]
fn retained_capture_and_loss_signals_are_optional_bounded_and_content_free() {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _, _, s) = ready(b"ab");
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let original = o
            .capture_source_read_context(&s, &facts(&s), &mut r)
            .unwrap();
        let child = o
            .capture_retained_source_read_context(&original, &s, &facts(&s), &mut r)
            .unwrap();
        let result = o
            .query_source_read_context(&child, &s, one_byte(), &facts(&s), &mut r)
            .unwrap();
        let SourceContextContentV1::Range(range) = result.content else {
            panic!("wrong product")
        };
        assert_eq!(range.bytes(), b"a");
        let mut lost = facts(&s);
        lost.source_readable = Some(false);
        assert!(
            o.capture_retained_source_read_context(&child, &s, &lost, &mut r)
                .is_none()
        );
        assert_eq!(o.source_read_context_usage().issued_lifetime, 2);
        assert_eq!(o.source_read_context_usage().holds, 0);
        if capacity == Some(128) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::SourceContextRetainedCapture
                        && e.disposition == Disposition::Completed)
            );
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::SourceContextRetainedCapture
                        && e.disposition == Disposition::Refused)
            );
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::SourceContextInvalidation)
            );
            assert!(events.iter().all(|e| e.service_time.is_none()));
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}
