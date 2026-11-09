use super::*;
use vellum_engine::coordinates::BytePosition;
#[path = "source_context/facade.rs"]
mod facade;
#[path = "source_context/projection.rs"]
mod projection;
#[path = "source_context/result_contracts.rs"]
mod result_contracts;
#[path = "source_context/retained.rs"]
mod retained;
#[path = "source_context/search.rs"]
mod search;
fn set_fact(f: &mut TrustedSourceReadFactsV1<'_>, index: usize, value: Option<bool>) {
    match index {
        0 => f.authorized = value,
        1 => f.scope_allowed = value,
        2 => f.continuous = value,
        3 => f.live = value,
        4 => f.not_stopped = value,
        5 => f.retained = value,
        6 => f.source_readable = value,
        7 => f.resources_ready = value,
        8 => f.reply_ready = value,
        9 => f.transfer_ready = value,
        _ => unreachable!(),
    }
}
fn one_byte() -> SourceContextQueryV1 {
    SourceContextQueryV1::Range {
        start: BytePosition(0),
        end: BytePosition(1),
    }
}
fn pool_control() -> SourceReadPoolControlV1 {
    SourceReadPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}

#[test]
fn contexts_and_internal_holds_share_nonrefilling_lifetime_issuance_even_after_rows_are_retired() {
    let (mut o, mut r, _, s) = ready(b"ab");
    let reserved = o.retention_usage().reserved;
    let hold_facts = SourceReadHoldFactsV1 {
        owner_control_allowed: Some(true),
        retained: Some(true),
        resources_ready: Some(true),
    };
    let mut previous = None;
    for issued in 0..4 {
        let id = o
            .capture_source_read_context(&s, &facts(&s), &mut r)
            .unwrap();
        if let Some(old) = &previous {
            assert_ne!(old, &id);
            assert!(
                o.query_source_read_context(old, &s, one_byte(), &facts(&s), &mut r)
                    .is_none()
            );
        }
        let hold = o
            .hold_source_read_context(&id, &s, &hold_facts, &mut r)
            .unwrap();
        o.stop_source_read_context(&id, &controls(&s), &mut r);
        assert!(o.release_source_read_hold(&hold, &controls(&s), &mut r));
        assert!(o.retire_source_read_context(&id, &controls(&s), &mut r));
        assert_eq!(
            o.source_read_context_usage().issued_lifetime,
            (issued + 1) * 2
        );
        assert_eq!(o.retention_usage().reserved, reserved);
        previous = Some(id);
    }
    assert!(
        o.capture_source_read_context(&s, &facts(&s), &mut r)
            .is_none()
    );
    assert_eq!(o.source_read_context_usage().contexts, 0);
    assert_eq!(o.source_read_context_usage().holds, 0);
    assert!(
        o.retire_source_read_contexts(&pool_control(), &mut r)
            .unwrap()
    );
    assert_eq!(o.source_read_context_usage().issued_lifetime, 8);
}

#[test]
fn context_lifetime_instrumentation_is_optional_bounded_redacted_and_not_a_delivery_authority() {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _, _, s) = ready(b"ab");
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
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
        let reply = o
            .query_source_read_context(&id, &s, one_byte(), &facts(&s), &mut r)
            .unwrap();
        assert_eq!(format!("{reply:?}"), "SourceContextReplyV1(<redacted>)");
        let SourceContextContentV1::Range(disclosed) = reply.content else {
            panic!("wrong product")
        };
        assert_eq!(disclosed.bytes(), b"a");
        let mut lost = facts(&s);
        lost.source_readable = Some(false);
        assert!(
            o.query_source_read_context(&id, &s, one_byte(), &lost, &mut r)
                .is_none()
        );
        assert!(o.stop_source_read_context(&id, &controls(&s), &mut r));
        assert!(o.release_source_read_hold(&hold, &controls(&s), &mut r));
        assert!(o.retire_source_read_context(&id, &controls(&s), &mut r));
        assert!(
            o.retire_source_read_contexts(&pool_control(), &mut r)
                .unwrap()
        );
        assert_eq!(disclosed.bytes(), b"a");
        assert_eq!(o.source_read_context_usage().issued_lifetime, 2);
        assert_eq!(o.retention_usage().reserved, 0);
        if capacity == Some(128) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::SourceContextCapture,
                Boundary::SourceContextQuery,
                Boundary::SourceContextHold,
                Boundary::SourceContextInvalidation,
                Boundary::SourceContextStop,
                Boundary::SourceContextRelease,
                Boundary::SourceContextRetirement,
                Boundary::SourceContextPoolRetirement,
            ] {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == boundary && e.disposition == Disposition::Completed)
                );
            }
            assert!(events.iter().all(|e| e.service_time.is_none()));
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}

#[test]
fn recognized_loss_is_absorbing_but_unknown_and_transient_pressure_do_not_establish_loss() {
    for mode in 0..3 {
        for index in 0..10 {
            let (mut o, mut r, _, s) = ready(b"ab");
            let id = o
                .capture_source_read_context(&s, &facts(&s), &mut r)
                .unwrap();
            let mut f = facts(&s);
            set_fact(&mut f, index, if mode == 0 { None } else { Some(false) });
            if mode == 2 {
                f.resources_ready = Some(false);
            }
            assert!(
                o.query_source_read_context(&id, &s, one_byte(), &f, &mut r)
                    .is_none()
            );
            let absorbing = mode != 0 && [0, 1, 2, 3, 5, 6].contains(&index);
            assert_eq!(
                o.query_source_read_context(&id, &s, one_byte(), &facts(&s), &mut r)
                    .is_none(),
                absorbing,
                "mode {mode} field {index}"
            );
            assert!(!o.is_faulted());
            assert_eq!(o.source_read_context_usage().issued_lifetime, 1);
        }
    }
}

#[test]
fn malformed_boundaries_and_foreign_claims_cannot_poison_an_original_context_or_fault_an_honest_owner()
 {
    let (mut o, mut r, _, s) = ready("aé\r\n".as_bytes());
    let id = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let mut lost = facts(&s);
    lost.source_readable = Some(false);
    for query in [
        SourceContextQueryV1::Range {
            start: BytePosition(2),
            end: BytePosition(3),
        },
        SourceContextQueryV1::Range {
            start: BytePosition(3),
            end: BytePosition(1),
        },
        SourceContextQueryV1::Range {
            start: BytePosition(0),
            end: BytePosition(usize::MAX),
        },
        SourceContextQueryV1::Line { index: usize::MAX },
    ] {
        assert!(
            o.query_source_read_context(&id, &s, query, &lost, &mut r)
                .is_none()
        );
        assert!(
            o.query_source_read_context(&id, &s, one_byte(), &facts(&s), &mut r)
                .is_some()
        );
    }
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
            7 => wrong.document = o.create(b"other", &mut r).unwrap().document,
            8 => wrong.policy = "other".into(),
            _ => unreachable!(),
        }
        let mut f = facts(&wrong);
        f.source_readable = Some(false);
        assert!(
            o.query_source_read_context(&id, &wrong, one_byte(), &f, &mut r)
                .is_none()
        );
        assert!(
            o.query_source_read_context(&id, &s, one_byte(), &f, &mut r)
                .is_none()
        );
        assert!(
            o.query_source_read_context(&id, &s, one_byte(), &facts(&s), &mut r)
                .is_some()
        );
    }
    let (mut foreign, mut rr, _, _) = ready(b"foreign");
    assert!(
        foreign
            .query_source_read_context(&id, &s, one_byte(), &facts(&s), &mut rr)
            .is_none()
    );
    assert!(!foreign.is_faulted());
    assert!(!o.is_faulted());
}

#[test]
fn stopped_snapshots_remain_while_internal_holds_exist_and_only_empty_pool_retirement_refunds_parent()
 {
    let (mut o, mut r, _, s) = ready(b"ab");
    let id = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let alias = id.clone();
    let f = SourceReadHoldFactsV1 {
        owner_control_allowed: Some(true),
        retained: Some(true),
        resources_ready: Some(true),
    };
    let first = o.hold_source_read_context(&id, &s, &f, &mut r).unwrap();
    let second = o.hold_source_read_context(&id, &s, &f, &mut r).unwrap();
    let reserved = o.retention_usage().reserved;
    assert!(o.stop_source_read_context(&id, &controls(&s), &mut r));
    assert!(o.stop_source_read_context(&alias, &controls(&s), &mut r));
    assert_eq!(o.source_read_context_usage().retained_snapshots, 1);
    assert!(o.hold_source_read_context(&id, &s, &f, &mut r).is_none());
    assert!(!o.retire_source_read_context(&id, &controls(&s), &mut r));
    let mut wrong = s.clone();
    wrong.context.recipient = "wrong".into();
    assert!(!o.release_source_read_hold(&first, &controls(&wrong), &mut r));
    assert!(o.release_source_read_hold(&first, &controls(&s), &mut r));
    assert!(!o.release_source_read_hold(&first, &controls(&s), &mut r));
    assert_eq!(o.source_read_context_usage().holds, 1);
    assert_eq!(o.source_read_context_usage().retained_snapshots, 1);
    assert!(o.release_source_read_hold(&second, &controls(&s), &mut r));
    assert_eq!(o.source_read_context_usage().retained_snapshots, 0);
    assert!(o.retire_source_read_context(&id, &controls(&s), &mut r));
    assert_eq!(o.retention_usage().reserved, reserved);
    assert!(
        o.retire_source_read_contexts(&pool_control(), &mut r)
            .unwrap()
    );
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(
        !o.retire_source_read_contexts(&pool_control(), &mut r)
            .unwrap()
    );
    assert_eq!(o.source_read_context_usage().issued_lifetime, 3);
    assert!(o.source_read_context_usage().closed);
    assert_eq!(
        o.configure_source_read_contexts("engine", "local", profile(), &pool_control(), &mut r),
        Err(SourceReadContextErrorV1::Unavailable)
    );
}

#[test]
fn actual_disposal_fences_delivery_and_new_holds_despite_stale_positive_host_facts() {
    let (mut o, mut r, d, s) = ready(b"ab");
    let id = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    let f = SourceReadHoldFactsV1 {
        owner_control_allowed: Some(true),
        retained: Some(true),
        resources_ready: Some(true),
    };
    let hold = o.hold_source_read_context(&id, &s, &f, &mut r).unwrap();
    let plan = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
    o.commit_retained_disposal(plan, &mut r).unwrap();
    assert!(
        o.capture_source_read_context(&s, &facts(&s), &mut r)
            .is_none()
    );
    assert!(o.hold_source_read_context(&id, &s, &f, &mut r).is_none());
    assert!(
        o.query_source_read_context(&id, &s, one_byte(), &facts(&s), &mut r)
            .is_none()
    );
    assert_eq!(o.source_read_context_usage().retained_snapshots, 1);
    assert!(o.release_source_read_hold(&hold, &controls(&s), &mut r));
    assert_eq!(o.source_read_context_usage().retained_snapshots, 0);
    assert!(o.retire_source_read_context(&id, &controls(&s), &mut r));
}

#[test]
fn source_queries_preserve_mixed_delimiters_eof_and_literal_leading_feff_without_bom_reinterpretation()
 {
    let (mut o, mut r, d, s) = ready(b"a\r\nb\rc\n");
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 0,
                    expected: String::new(),
                    inserted: "\u{feff}".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(edit, &mut r).unwrap();
    let id = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    for (index, body, delimiter) in [
        (0, "\u{feff}a", "\r\n"),
        (1, "b", "\r"),
        (2, "c", "\n"),
        (3, "", ""),
    ] {
        let result = o
            .query_source_read_context(
                &id,
                &s,
                SourceContextQueryV1::Line { index },
                &facts(&s),
                &mut r,
            )
            .unwrap();
        let SourceContextContentV1::Line(line) = result.content else {
            panic!("wrong product")
        };
        assert_eq!(line.body(), body.as_bytes());
        assert_eq!(line.delimiter(), delimiter.as_bytes());
    }
    let result = o
        .query_source_read_context(
            &id,
            &s,
            SourceContextQueryV1::Range {
                start: BytePosition(0),
                end: BytePosition(3),
            },
            &facts(&s),
            &mut r,
        )
        .unwrap();
    let SourceContextContentV1::Range(range) = result.content else {
        panic!("wrong product")
    };
    assert_eq!(range.bytes(), "\u{feff}".as_bytes());
}
fn profile() -> SourceReadContextLimitsV1 {
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
fn scope(document: &DocumentId) -> SourceReadScopeV1 {
    SourceReadScopeV1 {
        document: document.clone(),
        context: scoped_basis::context(),
        policy: "p1".into(),
    }
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
fn controls(s: &SourceReadScopeV1) -> SourceReadContextControlV1<'_> {
    SourceReadContextControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn ready(
    bytes: &[u8],
) -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    SourceReadScopeV1,
) {
    ready_with_profile(bytes, profile())
}
fn ready_with_profile(
    bytes: &[u8],
    p: SourceReadContextLimitsV1,
) -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    SourceReadScopeV1,
) {
    let (mut o, mut r) = setup();
    o.configure_retention_budget(2_097_152, &mut r).unwrap();
    o.configure_source_read_contexts(
        "engine",
        "local",
        p,
        &SourceReadPoolControlV1 {
            owner_control_allowed: Some(true),
            resources_ready: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        },
        &mut r,
    )
    .unwrap();
    let d = o.create(bytes, &mut r).unwrap();
    let s = scope(&d.document);
    (o, r, d, s)
}
#[test]
fn mediated_source_capture_is_immutable_across_publication_and_does_not_disclose_annotations_or_latest_state()
 {
    let (mut o, mut r, d, s) = ready("aé\r\nx".as_bytes());
    let generation = o.generation();
    let context = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    assert_eq!(o.generation(), generation);
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
    let reply = o
        .query_source_read_context(
            &context,
            &s,
            SourceContextQueryV1::Range {
                start: BytePosition(0),
                end: BytePosition(3),
            },
            &facts(&s),
            &mut r,
        )
        .unwrap();
    let SourceContextContentV1::Range(range) = reply.content else {
        panic!("wrong product")
    };
    assert_eq!(range.bytes(), "aé".as_bytes());
    assert_eq!(range.source, d);
    let newer = o
        .capture_source_read_context(&s, &facts(&s), &mut r)
        .unwrap();
    assert_ne!(newer, context);
    let reply = o
        .query_source_read_context(
            &newer,
            &s,
            SourceContextQueryV1::Range {
                start: BytePosition(0),
                end: BytePosition(1),
            },
            &facts(&s),
            &mut r,
        )
        .unwrap();
    let SourceContextContentV1::Range(range) = reply.content else {
        panic!("wrong product")
    };
    assert_eq!(range.bytes(), b"B");
    assert!(o.stop_source_read_context(&context, &controls(&s), &mut r));
    assert!(
        o.query_source_read_context(
            &context,
            &s,
            SourceContextQueryV1::Line { index: 0 },
            &facts(&s),
            &mut r
        )
        .is_none()
    );
}
