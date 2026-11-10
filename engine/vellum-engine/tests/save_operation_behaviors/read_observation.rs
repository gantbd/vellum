use super::observation::{control, facts, obs_limits, policy};
use super::*;
type Error = NativeOperationObservationErrorV1;
type Query = NativeOperationObservationQueryV1;
pub(super) fn setup() -> (Fixture, AnnotationReadId, SettingsExecutionContextV1) {
    let mut f = read_operations::setup();
    let c = context();
    let target = f.port.annotation_read_target(b"comments").unwrap();
    let annotation = f.port.source_binding_target(b"comments").unwrap();
    let source = f.port.source_binding_target(b"source").unwrap();
    let document = f.doc.clone();
    let request = read_operations::read_request(&document, &target, &annotation, &source, b"load");
    let pair = f.pair();
    let task = f
        .owner
        .request_owned_annotation_read_v1(
            &request,
            &read_operations::facts(&c),
            pair,
            &mut f.recorder,
        )
        .unwrap()
        .task;
    f.owner
        .configure_native_operation_observation_v1(
            NativeOperationObservationPoolLimitsV1 {
                max_contexts: 4,
                max_issued: 4,
                max_pool_bytes: 65536,
            },
            &mut f.recorder,
        )
        .unwrap();
    (f, task, c)
}
pub(super) fn open(
    f: &mut Fixture,
    task: &AnnotationReadId,
    c: &SettingsExecutionContextV1,
) -> NativeOperationObservationIdV1 {
    f.owner
        .open_native_annotation_read_observation_v1(
            task,
            &policy(c),
            obs_limits(),
            &facts(c),
            &mut f.recorder,
        )
        .unwrap()
}
fn query(
    f: &mut Fixture,
    id: &NativeOperationObservationIdV1,
    c: &SettingsExecutionContextV1,
) -> NativeOperationObservationReplyV1 {
    f.owner
        .query_native_operation_observation_v1(
            id,
            Query::RetainedOutcome,
            &policy(c),
            &facts(c),
            &mut f.recorder,
        )
        .unwrap()
}
fn receipt(reply: &NativeOperationObservationReplyV1) -> &NativeAnnotationReadReceiptV1 {
    let NativeOperationObservationReplyV1::Receipt {
        receipt: NativeOperationReceiptV1::AnnotationLoad(value),
        ..
    } = reply
    else {
        panic!("read receipt")
    };
    value
}
#[test]
fn ready_owned_read_observation_never_submits_decodes_or_changes_the_root() {
    let (mut f, task, c) = setup();
    let before = f.owner.generation();
    let work = f.owner.resource_usage();
    let id = open(&mut f, &task, &c);
    let first = query(&mut f, &id, &c);
    let r = receipt(&first);
    assert_eq!(r.phase, ReadPhase::Ready);
    assert_eq!(r.witnesses, [None, None]);
    assert_eq!((r.received_bytes, r.received_chunks), (0, 0));
    assert!(!r.stopped && !r.quiescent && !r.contested && !r.decode_refused);
    let second = f
        .owner
        .query_native_operation_observation_v1(
            &id,
            Query::Status,
            &policy(&c),
            &facts(&c),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(first.observation(), second.observation());
    assert_eq!(f.owner.generation(), before);
    assert_eq!(f.owner.resource_usage(), work);
}
#[test]
fn late_chunk_conflicts_never_release_bytes_and_only_original_terminal_metadata_is_projected() {
    let (mut f, task, c) = setup();
    let id = open(&mut f, &task, &c);
    let before = query(&mut f, &id, &c);
    let pair = f.pair();
    let dispatch = f
        .owner
        .submit_annotation_read(&task, pair, &mut f.recorder)
        .unwrap();
    for e in [
        f.port.annotation_read_start(&dispatch, b"version").unwrap(),
        f.port
            .annotation_read_chunk(&dispatch, 0, 0, b"PRIVATE-BODY")
            .unwrap(),
        f.port
            .annotation_read_end(&dispatch, b"version", 1, 12)
            .unwrap(),
    ] {
        f.owner.record_annotation_read(e, &mut f.recorder).unwrap();
    }
    let after = query(&mut f, &id, &c);
    assert_ne!(before.observation(), after.observation());
    let r = receipt(&after);
    assert_eq!((r.received_chunks, r.received_bytes), (1, 12));
    let Some(NativeReadTerminalWitnessV1::End {
        version,
        coherence,
        chunks: 1,
        bytes: 12,
    }) = &r.witnesses[0]
    else {
        panic!("end")
    };
    assert_eq!(version.bytes(), b"version");
    assert_eq!(*coherence, ReadStrength::CapturedRead);
    assert_eq!(r.witnesses[1], None);
    f.owner
        .record_annotation_read(
            f.port
                .annotation_read_chunk(&dispatch, 0, 0, b"OTHER-SECRET")
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let contested = query(&mut f, &id, &c);
    assert!(receipt(&contested).contested);
    assert_eq!(receipt(&contested).witnesses, r.witnesses);
    assert!(!format!("{contested:?}").contains("PRIVATE-BODY"));
    assert!(!format!("{contested:?}").contains("OTHER-SECRET"));
    f.owner
        .stop_annotation_read(&task, &mut f.recorder)
        .unwrap();
    f.owner
        .record_annotation_read(
            f.port.annotation_read_quiescence(&dispatch).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let terminal = query(&mut f, &id, &c);
    assert!(receipt(&terminal).stopped && receipt(&terminal).quiescent);
    assert_eq!(receipt(&terminal).witnesses, r.witnesses);
}
#[test]
fn two_distinct_terminal_facts_remain_bounded_and_keep_original_coherence() {
    let (mut f, task, c) = setup();
    let id = open(&mut f, &task, &c);
    let pair = f.pair();
    let dispatch = f
        .owner
        .submit_annotation_read(&task, pair, &mut f.recorder)
        .unwrap();
    for e in [
        f.port.annotation_read_start(&dispatch, b"v").unwrap(),
        f.port.annotation_read_end(&dispatch, b"v", 0, 0).unwrap(),
        f.port
            .annotation_read_failure(&dispatch, ReadFailure::Interrupted)
            .unwrap(),
    ] {
        f.owner.record_annotation_read(e, &mut f.recorder).unwrap();
    }
    let value = query(&mut f, &id, &c);
    let r = receipt(&value);
    assert!(r.contested);
    assert!(matches!(
        r.witnesses[0],
        Some(NativeReadTerminalWitnessV1::End { .. })
    ));
    assert_eq!(
        r.witnesses[1],
        Some(NativeReadTerminalWitnessV1::Failure(
            ReadFailure::Interrupted
        ))
    );
    let duplicate = query(&mut f, &id, &c);
    assert_eq!(value, duplicate);
}
#[test]
fn read_disclosure_is_independent_of_source_and_write_disposition_and_can_be_temporarily_withheld()
{
    let (mut f, task, c) = setup();
    let mut g = facts(&c);
    g.source_readable = Some(false);
    g.disposition_readable = None;
    let id = f
        .owner
        .open_native_annotation_read_observation_v1(
            &task,
            &policy(&c),
            obs_limits(),
            &g,
            &mut f.recorder,
        )
        .unwrap();
    let first = f
        .owner
        .query_native_operation_observation_v1(
            &id,
            Query::RetainedOutcome,
            &policy(&c),
            &g,
            &mut f.recorder,
        )
        .unwrap();
    for field in 0..3 {
        for denied in [None, Some(false)] {
            let mut g = facts(&c);
            match field {
                0 => g.read_allowed = denied,
                1 => g.annotation_readable = denied,
                _ => g.operation_receipt_allowed = denied,
            }
            assert_eq!(
                f.owner.query_native_operation_observation_v1(
                    &id,
                    Query::RetainedOutcome,
                    &policy(&c),
                    &g,
                    &mut f.recorder
                ),
                Err(Error::Unavailable)
            );
            assert!(matches!(
                f.owner
                    .query_native_operation_observation_v1(
                        &id,
                        Query::MinimalControlStatus,
                        &policy(&c),
                        &g,
                        &mut f.recorder
                    )
                    .unwrap(),
                NativeOperationObservationReplyV1::Status { receipt: None, .. }
            ));
        }
    }
    assert_eq!(first, query(&mut f, &id, &c));
    let mut lost = facts(&c);
    lost.scope = Some(false);
    assert_eq!(
        f.owner.query_native_operation_observation_v1(
            &id,
            Query::Status,
            &policy(&c),
            &lost,
            &mut f.recorder
        ),
        Err(Error::Unavailable)
    );
    assert_eq!(
        f.owner.query_native_operation_observation_v1(
            &id,
            Query::Status,
            &policy(&c),
            &facts(&c),
            &mut f.recorder
        ),
        Err(Error::Unavailable)
    );
    assert_eq!(
        f.owner
            .annotation_read_status(&task, &mut f.recorder)
            .unwrap()
            .phase,
        ReadPhase::Ready
    );
}
#[test]
fn read_observers_add_no_producer_pin_and_cannot_resurrect_a_retired_parent() {
    let (mut f, task, c) = setup();
    let id = open(&mut f, &task, &c);
    let _ = query(&mut f, &id, &c);
    f.owner
        .stop_annotation_read(&task, &mut f.recorder)
        .unwrap();
    f.owner
        .retire_annotation_read(&task, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner.query_native_operation_observation_v1(
            &id,
            Query::RetainedOutcome,
            &policy(&c),
            &facts(&c),
            &mut f.recorder
        ),
        Err(Error::Unavailable)
    );
    assert!(
        !f.owner
            .native_operation_observation_usage_v1(&id)
            .unwrap()
            .live
    );
    f.owner
        .retire_native_operation_observation_v1(&id, &control(&c), &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .annotation_read_operation_usage_v1()
            .unwrap()
            .spent_origins,
        1
    );
}
#[test]
fn read_waits_share_bounded_progress_but_observer_stop_does_not_stop_a_reader() {
    let (mut f, task, c) = setup();
    let id = open(&mut f, &task, &c);
    let first = query(&mut f, &id, &c);
    let clock = NativeOperationWaitClockV1 { epoch: 4, tick: 1 };
    let wait = f
        .owner
        .start_native_operation_observation_wait_v1(
            &id,
            NativeOperationWaitRequestV1 {
                after: first.observation(),
                budget: 2,
                clock,
            },
            &policy(&c),
            &facts(&c),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner
            .advance_native_operation_observation_wait_v1(
                &id,
                &wait,
                NativeOperationWaitEventV1::Poll,
                &policy(&c),
                &facts(&c),
                &mut f.recorder
            )
            .unwrap(),
        NativeOperationWaitReplyV1::Waiting(wait.clone())
    );
    let pair = f.pair();
    let dispatch = f
        .owner
        .submit_annotation_read(&task, pair, &mut f.recorder)
        .unwrap();
    assert!(matches!(
        f.owner
            .advance_native_operation_observation_wait_v1(
                &id,
                &wait,
                NativeOperationWaitEventV1::Poll,
                &policy(&c),
                &facts(&c),
                &mut f.recorder
            )
            .unwrap(),
        NativeOperationWaitReplyV1::Observed(_)
    ));
    assert_eq!(
        f.owner
            .retire_native_operation_observation_v1(&id, &control(&c), &mut f.recorder),
        Err(Error::Unavailable)
    );
    f.owner
        .retire_native_operation_observation_wait_v1(&id, &wait, &control(&c), &mut f.recorder)
        .unwrap();
    f.owner
        .stop_native_operation_observation_v1(&id, &control(&c), &mut f.recorder)
        .unwrap();
    let status = f
        .owner
        .annotation_read_status(&task, &mut f.recorder)
        .unwrap();
    assert!(!status.stopped && !status.quiescent);
    assert_eq!(dispatch.strength(), ReadStrength::CapturedRead);
}
#[test]
fn read_observation_requires_actual_parent_original_scope_and_every_strict_admission_fact() {
    let (mut f, task, c) = setup();
    let funded = f.owner.retention_usage();
    for field in 0..16 {
        for denied in [None, Some(false)] {
            let mut g = facts(&c);
            let slot = match field {
                0 => &mut g.authorized,
                1 => &mut g.scope,
                2 => &mut g.continuous,
                3 => &mut g.live,
                4 => &mut g.resources_ready,
                5 => &mut g.retained,
                6 => &mut g.transfer_ready,
                7 => &mut g.preparation_reserved,
                8 => &mut g.registry_ready,
                9 => &mut g.retention_ready,
                10 => &mut g.control_allowed,
                11 => &mut g.operation_receipt_admission_allowed,
                12 => &mut g.operation_receipt_allowed,
                13 => &mut g.reply_ready,
                14 => &mut g.annotation_readable,
                _ => &mut g.read_allowed,
            };
            *slot = denied;
            assert_eq!(
                f.owner.open_native_annotation_read_observation_v1(
                    &task,
                    &policy(&c),
                    obs_limits(),
                    &g,
                    &mut f.recorder
                ),
                Err(Error::Unavailable)
            );
            assert_eq!(f.owner.retention_usage(), funded);
            assert_eq!(
                f.owner
                    .native_operation_observation_pool_usage_v1()
                    .unwrap()
                    .issued_lifetime,
                0
            );
        }
    }
    for field in 0..7 {
        let mut wrong = c.clone();
        let slot = match field {
            0 => &mut wrong.authority,
            1 => &mut wrong.engine,
            2 => &mut wrong.recipient,
            3 => &mut wrong.purpose,
            4 => &mut wrong.destination,
            5 => &mut wrong.profile,
            _ => &mut wrong.continuity,
        };
        *slot = "other".into();
        assert_eq!(
            f.owner.open_native_annotation_read_observation_v1(
                &task,
                &policy(&wrong),
                obs_limits(),
                &facts(&wrong),
                &mut f.recorder
            ),
            Err(Error::Unavailable)
        );
    }
    let (mut other, foreign, _) = setup();
    assert_eq!(
        f.owner.open_native_annotation_read_observation_v1(
            &foreign,
            &policy(&c),
            obs_limits(),
            &facts(&c),
            &mut f.recorder
        ),
        Err(Error::Unavailable)
    );
    let target = other.port.annotation_read_target(b"comments").unwrap();
    let pair = other.pair();
    let raw = other
        .owner
        .admit_annotation_read(
            &other.doc,
            0,
            target,
            ReadStrength::CapturedRead,
            pair,
            &mut other.recorder,
        )
        .unwrap();
    assert_eq!(
        other.owner.open_native_annotation_read_observation_v1(
            &raw,
            &policy(&c),
            obs_limits(),
            &facts(&c),
            &mut other.recorder
        ),
        Err(Error::Unavailable)
    );
}
#[test]
fn zero_ordinary_credit_keeps_protected_read_control_and_failure_is_not_success_or_quiescence() {
    let (mut f, task, c) = setup();
    let mut limits = obs_limits();
    limits.max_issued = 0;
    let id = f
        .owner
        .open_native_annotation_read_observation_v1(
            &task,
            &policy(&c),
            limits,
            &facts(&c),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner.query_native_operation_observation_v1(
            &id,
            Query::RetainedOutcome,
            &policy(&c),
            &facts(&c),
            &mut f.recorder
        ),
        Err(Error::Limit)
    );
    let mut g = facts(&c);
    g.ordinary_ready = None;
    g.reply_ready = None;
    g.read_allowed = None;
    g.annotation_readable = None;
    g.operation_receipt_allowed = None;
    let first = f
        .owner
        .query_native_operation_observation_v1(
            &id,
            Query::MinimalControlStatus,
            &policy(&c),
            &g,
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair();
    let dispatch = f
        .owner
        .submit_annotation_read(&task, pair, &mut f.recorder)
        .unwrap();
    f.owner
        .record_annotation_read(
            f.port
                .annotation_read_failure(&dispatch, ReadFailure::Unavailable)
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let same = f
        .owner
        .query_native_operation_observation_v1(
            &id,
            Query::MinimalControlStatus,
            &policy(&c),
            &g,
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(first, same);
    f.owner
        .stop_annotation_read(&task, &mut f.recorder)
        .unwrap();
    let stopped = f
        .owner
        .query_native_operation_observation_v1(
            &id,
            Query::MinimalControlStatus,
            &policy(&c),
            &g,
            &mut f.recorder,
        )
        .unwrap();
    assert_ne!(stopped.observation(), first.observation());
    assert!(matches!(
        stopped,
        NativeOperationObservationReplyV1::Status {
            phase: NativeOperationObservationPhaseV1::Terminal,
            stopped: true,
            quiescent: false,
            receipt: None,
            ..
        }
    ));
    assert_eq!(
        f.owner
            .native_operation_observation_usage_v1(&id)
            .unwrap()
            .ordinary_issued,
        0
    );
}
#[test]
fn optional_read_observation_diagnostics_never_carry_body_scope_version_or_origin_labels() {
    for capacity in [None, Some(0), Some(128)] {
        let (mut f, task, mut c) = setup();
        // The actual original scope must remain unchanged; arbitrary diagnostics
        // cannot silently retarget it.
        let original = c.clone();
        c.purpose = "PRIVATE-PURPOSE".into();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        assert_eq!(
            f.owner.open_native_annotation_read_observation_v1(
                &task,
                &policy(&c),
                obs_limits(),
                &facts(&c),
                &mut f.recorder
            ),
            Err(Error::Unavailable)
        );
        let id = open(&mut f, &task, &original);
        let value = query(&mut f, &id, &original);
        assert_eq!(receipt(&value).phase, ReadPhase::Ready);
        f.owner
            .stop_native_operation_observation_v1(&id, &control(&original), &mut f.recorder)
            .unwrap();
        f.owner
            .retire_native_operation_observation_v1(&id, &control(&original), &mut f.recorder)
            .unwrap();
        let mut events = String::new();
        while let Some(event) = f.recorder.pop_front() {
            events.push_str(&format!("{event:?}"));
        }
        assert!(!events.contains("PRIVATE-PURPOSE"));
        assert!(!events.contains("recipient"));
        if capacity == Some(128) {
            assert!(events.contains("NativeOperationObservationAdmission"));
            assert!(events.contains("NativeOperationObservationSnapshot"));
            assert!(events.contains("NativeOperationObservationRetirement"));
        }
        assert!(
            !f.owner
                .annotation_read_status(&task, &mut f.recorder)
                .unwrap()
                .stopped
        );
    }
}
#[test]
fn refused_read_capture_reports_only_existing_counts_and_failure_metadata() {
    let (mut f, task, c) = setup();
    let id = open(&mut f, &task, &c);
    let pair = f.pair();
    let dispatch = f
        .owner
        .submit_annotation_read(&task, pair, &mut f.recorder)
        .unwrap();
    f.owner
        .record_annotation_read(
            f.port.annotation_read_start(&dispatch, b"v").unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .record_annotation_read(
            f.port
                .annotation_read_chunk(&dispatch, 0, 0, &[b'x'; 64])
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner.record_annotation_read(
            f.port
                .annotation_read_chunk(&dispatch, 1, 64, b"y")
                .unwrap(),
            &mut f.recorder
        ),
        Err(ResourceError::Limit)
    );
    let refused = query(&mut f, &id, &c);
    let r = receipt(&refused);
    assert!(r.decode_refused && !r.quiescent && !r.stopped);
    assert_eq!((r.received_chunks, r.received_bytes), (1, 64));
    assert_eq!(r.witnesses, [None, None]);
    f.owner
        .record_annotation_read(
            f.port
                .annotation_read_failure(&dispatch, ReadFailure::Limit)
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let failed = query(&mut f, &id, &c);
    assert_ne!(refused.observation(), failed.observation());
    assert_eq!(
        receipt(&failed).witnesses,
        [
            Some(NativeReadTerminalWitnessV1::Failure(ReadFailure::Limit)),
            None
        ]
    );
}
