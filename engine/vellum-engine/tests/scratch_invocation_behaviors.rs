#[path = "support/scratch_invocation_fixture.rs"]
mod support;
use support::*;
use vellum_engine::coordinator::*;
#[test]
fn scratch_invocation_creates_one_original_allocation_without_handshake_or_replay_birth() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    let source = "\u{feff}PRIVATE prose\r\n```mermaid\ngraph TD\n```";
    let q = request(source.as_bytes());
    let history = o.history_usage();
    let generation = o.generation();
    o.submit_scratch_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    let held = o.retention_usage();
    assert_eq!(o.lifecycle_usage().issued_lifetime, 0);
    assert_eq!(o.run(&id, &mut r), Err(InvocationError::Unavailable));
    assert_eq!(
        o.scratch_result_v1(&id, &release(&c), &mut r),
        Err(InvocationError::OutcomeUnavailable)
    );
    o.run_scratch_v1(&id, &f, &mut r).unwrap();
    let result = o.scratch_result_v1(&id, &release(&c), &mut r).unwrap();
    assert_eq!(result.invocation, id);
    let current = o.read(&result.document, &mut r).unwrap();
    assert_eq!(current.source.text(), source);
    assert_eq!(current.version.revision, 1);
    assert_eq!(o.history_usage(), history);
    assert_eq!(o.generation(), generation + 1);
    assert_eq!(o.lifecycle_usage().issued_lifetime, 1);
    o.submit_scratch_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    o.run_scratch_v1(&id, &f, &mut r).unwrap();
    assert_eq!(
        o.scratch_result_v1(&id, &release(&c), &mut r).unwrap(),
        result
    );
    assert_eq!(o.lifecycle_usage().issued_lifetime, 1);
    assert_eq!(o.generation(), generation + 1);
    assert_eq!(o.retention_usage(), held);
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        o.scratch_result_v1(&id, &release(&c), &mut r),
        Err(InvocationError::OutcomeUnavailable)
    );
    o.submit_scratch_v1(&id, &q, &admission(&f), &mut r)
        .unwrap();
    o.run_scratch_v1(&id, &f, &mut r).unwrap();
    assert_eq!(o.lifecycle_usage().issued_lifetime, 1);
    o.retire_scratch_v1(&id, &control(&c), &mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
#[test]
fn scratch_captures_authored_bytes_and_complete_intent_without_pinning_an_old_root() {
    let (mut o, mut r) = owner();
    let c = context();
    let f = facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    let mut bytes = "\u{feff}original".as_bytes().to_vec();
    {
        let q = request(&bytes);
        o.submit_scratch_v1(&id, &q, &admission(&f), &mut r)
            .unwrap();
    }
    bytes.fill(b'X');
    assert_eq!(
        o.submit_scratch_v1(&id, &request(&bytes), &admission(&f), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    let q = request("\u{feff}original".as_bytes());
    for bad in 0..6 {
        let mut changed = request(q.source);
        let mut a = admission(&f);
        match bad {
            0 => changed.serialization.bom = true,
            1 => changed.insertion_policy = GeneratedNewlineV1::Lf,
            2 => a.publication_epoch = 8,
            3 => a.limits.max_capture_bytes -= 1,
            4 => a.result_identifiers_allowed = Some(false),
            _ => a.result_admission_allowed = Some(false),
        }
        assert_eq!(
            o.submit_scratch_v1(&id, &changed, &a, &mut r),
            Err(InvocationError::IntentMismatch)
        );
    }
    let peer = o.create(b"intervening", &mut r).unwrap();
    o.run_scratch_v1(&id, &f, &mut r).unwrap();
    let out = o.scratch_result_v1(&id, &release(&c), &mut r).unwrap();
    assert_ne!(out.document, peer.document);
    assert_eq!(
        o.read(&out.document, &mut r).unwrap().source.text(),
        "\u{feff}original"
    );
    assert_eq!(
        o.read(&peer.document, &mut r).unwrap().source.text(),
        "intervening"
    );
    assert_eq!(o.lifecycle_usage().issued_lifetime, 2);
}
fn deny(f: &mut TrustedScratchFactsV1<'_>, index: usize, v: Option<bool>) {
    match index {
        0 => f.authorized = v,
        1 => f.scope = v,
        2 => f.continuous = v,
        3 => f.live = v,
        4 => f.not_stopped = v,
        5 => f.interpretation_current = v,
        6 => f.resources_ready = v,
        7 => f.companions_ready = v,
        8 => f.versions_available = v,
        9 => f.reply_ready = v,
        10 => f.lifecycle_allowed = v,
        11 => f.coverage_complete = v,
        _ => f.creation_allowed = v,
    }
}
#[test]
fn every_creation_admission_and_fresh_service_premise_is_strict_without_spending_an_allocation() {
    for index in 0..13 {
        for value in [None, Some(false)] {
            let (mut o, mut r) = owner();
            let c = context();
            let id = o.issue_invocation(&mut r).unwrap();
            let q = request(b"PRIVATE");
            let mut denied = facts(&c);
            deny(&mut denied, index, value);
            let held = o.retention_usage();
            let generation = o.generation();
            assert_eq!(
                o.submit_scratch_v1(&id, &q, &admission(&denied), &mut r),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(o.retention_usage(), held);
            assert_eq!(
                o.invocation_state(&id, &mut r).unwrap(),
                InvocationState::Issued
            );
            assert_eq!(o.lifecycle_usage().issued_lifetime, 0);
            o.submit_scratch_v1(&id, &q, &admission(&facts(&c)), &mut r)
                .unwrap();
            assert_eq!(
                o.run_scratch_v1(&id, &denied, &mut r),
                Err(InvocationError::Unavailable)
            );
            assert_eq!(
                o.invocation_state(&id, &mut r).unwrap(),
                InvocationState::Pending
            );
            assert_eq!(o.generation(), generation);
            assert_eq!(o.lifecycle_usage().issued_lifetime, 0);
            o.run_scratch_v1(&id, &facts(&c), &mut r).unwrap();
            assert_eq!(o.lifecycle_usage().issued_lifetime, 1);
        }
    }
}
#[test]
fn scratch_result_disclosure_is_separate_and_historical_identity_does_not_claim_live_state() {
    for bits in 0..8 {
        let (mut o, mut r) = owner();
        let c = context();
        let f = facts(&c);
        let id = o.issue_invocation(&mut r).unwrap();
        let mut a = admission(&f);
        a.publication_allowed = Some(bits & 1 != 0);
        a.result_admission_allowed = Some(bits & 2 != 0);
        a.result_identifiers_allowed = Some(bits & 4 != 0);
        o.submit_scratch_v1(&id, &request(b"content"), &a, &mut r)
            .unwrap();
        o.run_scratch_v1(&id, &f, &mut r).unwrap();
        assert_eq!(
            o.scratch_result_v1(&id, &release(&c), &mut r).is_ok(),
            bits == 7
        );
        assert_eq!(o.lifecycle_usage().live, 1);
    }
    let (mut o, mut r) = owner();
    let c = context();
    let mut wrong = c.clone();
    wrong.recipient = "other".into();
    let f = facts(&c);
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_scratch_v1(&id, &request(b"content"), &admission(&f), &mut r)
        .unwrap();
    o.run_scratch_v1(&id, &f, &mut r).unwrap();
    let original = o.scratch_result_v1(&id, &release(&c), &mut r).unwrap();
    let generation = o.generation();
    let held = o.retention_usage();
    for bad in 0..17 {
        let mut n = release(&c);
        match bad {
            0 => n.publication_allowed = None,
            1 => n.publication_allowed = Some(false),
            2 => n.outcome_readable = None,
            3 => n.outcome_readable = Some(false),
            4 => n.result_readable = None,
            5 => n.result_readable = Some(false),
            6 => n.result_context_current = None,
            7 => n.result_context_current = Some(false),
            8 => n.historical_document_readable = None,
            9 => n.historical_document_readable = Some(false),
            10 => n.reply_ready = None,
            11 => n.reply_ready = Some(false),
            12 => n.resources_ready = None,
            13 => n.resources_ready = Some(false),
            14 => n.context = &wrong,
            15 => n.publication_epoch = 8,
            _ => n.max_result_bytes = 0,
        }
        assert!(o.scratch_result_v1(&id, &n, &mut r).is_err());
        assert_eq!(
            o.scratch_result_v1(&id, &release(&c), &mut r).unwrap(),
            original
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
    }
    o.dispose(&original.document, &mut r).unwrap();
    assert!(o.read(&original.document, &mut r).is_err());
    assert_eq!(
        o.scratch_result_v1(&id, &release(&c), &mut r).unwrap(),
        original
    );
}
#[test]
fn malformed_short_and_unknown_admission_refunds_provisional_funding_and_does_not_publish() {
    for bad in 0..7 {
        let (mut o, mut r) = owner();
        let c = context();
        let f = facts(&c);
        let id = o.issue_invocation(&mut r).unwrap();
        let source = if bad == 0 {
            &[0xed, 0xa0, 0x80][..]
        } else {
            b"valid"
        };
        let q = request(source);
        let mut a = admission(&f);
        match bad {
            1 => a.limits.max_capture_bytes = 1,
            2 => a.limits.max_result_bytes = 1,
            3 => a.publication_allowed = None,
            4 => a.result_admission_allowed = None,
            5 => a.result_identifiers_allowed = None,
            6 => a.limits.max_capture_bytes = usize::MAX,
            _ => {}
        }
        let generation = o.generation();
        let held = o.retention_usage();
        assert!(o.submit_scratch_v1(&id, &q, &a, &mut r).is_err());
        assert_eq!(
            o.invocation_state(&id, &mut r).unwrap(),
            InvocationState::Issued
        );
        assert_eq!(o.lifecycle_usage().issued_lifetime, 0);
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage(), held);
        assert!(!o.is_faulted());
    }
}
#[test]
fn stop_and_late_capacity_refusal_are_spent_and_protected_cleanup_cannot_refund_other_holds() {
    for stopped in [false, true] {
        let (mut o, mut r) = owner();
        let c = context();
        let f = facts(&c);
        let id = o.issue_invocation(&mut r).unwrap();
        let q = request(b"content");
        o.submit_scratch_v1(&id, &q, &admission(&f), &mut r)
            .unwrap();
        let second = o.issue_invocation(&mut r).unwrap();
        o.submit_scratch_v1(&second, &q, &admission(&f), &mut r)
            .unwrap();
        let held = o.retention_usage().reserved;
        if stopped {
            o.request_invocation_stop(
                &id,
                &TrustedInvocationControlV1 {
                    context: &c,
                    owner_control_allowed: Some(true),
                    control_context_current: Some(true),
                    control_scope_valid: Some(true),
                    protected_control_ready: Some(true),
                },
                &mut r,
            )
            .unwrap();
            let mut lost = facts(&c);
            lost.creation_allowed = Some(false);
            o.run_scratch_v1(&id, &lost, &mut r).unwrap();
        } else {
            for _ in 0..8 {
                o.create(b"peer", &mut r).unwrap();
            }
            o.run_scratch_v1(&id, &f, &mut r).unwrap();
            assert!(matches!(
                o.invocation_outcome(&id, &mut r).unwrap(),
                InvocationOutcome::ScratchRejected(_)
            ));
        }
        assert_eq!(
            o.scratch_result_v1(&id, &release(&c), &mut r),
            Err(InvocationError::OutcomeUnavailable)
        );
        let generation = o.generation();
        o.forget_invocation_outcome(&id, &mut r).unwrap();
        o.submit_scratch_v1(&id, &q, &admission(&f), &mut r)
            .unwrap();
        o.run_scratch_v1(&id, &f, &mut r).unwrap();
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage().reserved, held);
        assert_eq!(
            o.retire_invocation(&id, &mut r),
            Err(InvocationError::Unavailable)
        );
        for bad in 0..6 {
            let mut ctl = control(&c);
            match bad {
                0 => ctl.owner_control_allowed = None,
                1 => ctl.owner_control_allowed = Some(false),
                2 => ctl.retention_released = None,
                3 => ctl.retention_released = Some(false),
                4 => ctl.delivery_drained = None,
                _ => ctl.delivery_drained = Some(false),
            }
            assert_eq!(
                o.retire_scratch_v1(&id, &ctl, &mut r),
                Err(InvocationError::Unavailable)
            );
        }
        o.retire_scratch_v1(&id, &control(&c), &mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, held / 2);
        assert_eq!(
            o.retire_scratch_v1(&id, &control(&c), &mut r),
            Err(InvocationError::Unavailable)
        );
        o.retire_invocation(&id, &mut r).unwrap();
        assert_eq!(o.invocation_usage().issued_lifetime, 2);
        o.stop_invocation(&second, &mut r).unwrap();
        o.retire_scratch_v1(&second, &control(&c), &mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
#[test]
fn scratch_invocation_instrumentation_is_optional_bounded_and_never_contains_authored_data() {
    use vellum_engine::instrumentation::{
        Boundary, Disposition, NativeTimingProfileV1, Observation, Recorder,
    };
    for mode in 0..4 {
        let (mut o, mut setup) = owner();
        let c = context();
        let f = facts(&c);
        let id = o.issue_invocation(&mut setup).unwrap();
        let mut r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1)
        }
        if mode == 3 {
            r.record(Observation {
                boundary: Boundary::Control,
                disposition: Disposition::Completed,
                service_time: None,
            });
        }
        o.submit_scratch_v1(
            &id,
            &request("\u{feff}PRIVATE".as_bytes()),
            &admission(&f),
            &mut r,
        )
        .unwrap();
        o.run_scratch_v1(&id, &f, &mut r).unwrap();
        let result = o.scratch_result_v1(&id, &release(&c), &mut r).unwrap();
        o.retire_scratch_v1(&id, &control(&c), &mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        assert_eq!(
            o.read(&result.document, &mut setup).unwrap().source.text(),
            "\u{feff}PRIVATE"
        );
        let mut counts = [0; 4];
        while let Some(e) = r.pop_front() {
            let i = match e.boundary {
                Boundary::ScratchInvocationAdmission => Some(0),
                Boundary::ScratchInvocationService => Some(1),
                Boundary::ScratchResultRelease => Some(2),
                Boundary::ScratchInvocationRetirement => Some(3),
                _ => None,
            };
            if let Some(i) = i {
                counts[i] += 1;
                assert_eq!(e.disposition, Disposition::Completed);
                assert_eq!(e.service_time.is_some(), mode == 2)
            }
            assert!(!format!("{e:?}").contains("PRIVATE"));
        }
        if mode == 1 || mode == 2 {
            assert_eq!(counts, [1, 1, 1, 1])
        } else {
            assert_eq!(counts, [0, 0, 0, 0])
        }
    }
}
