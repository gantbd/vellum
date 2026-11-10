use super::decoder::support_decoder as installed_decoder;
use super::*;
enum InstalledTask {
    Annotation(AnnotationReadId),
    Save(SaveId),
}
impl InstalledTask {
    fn as_ref(&self) -> NativeOperationTaskV1<'_> {
        match self {
            Self::Annotation(id) => NativeOperationTaskV1::AnnotationLoad(id),
            Self::Save(id) => NativeOperationTaskV1::Save(id),
        }
    }
}
#[test]
fn installed_native_operation_results_reach_source_save_annotation_save_and_annotation_load() {
    for kind in 0..3 {
        for mode in 0..4 {
            let (mut f, task, c, id) = if kind == 2 {
                let (mut f, task, c) = read_observation::setup();
                let id = read_observation::open(&mut f, &task, &c);
                (f, InstalledTask::Annotation(task), c, id)
            } else {
                let (mut f, task, c) = setup(kind == 1);
                let id = f
                    .owner
                    .open_native_save_observation_v1(
                        &task,
                        &policy(&c),
                        obs_limits(),
                        &facts(&c),
                        &mut f.recorder,
                    )
                    .unwrap();
                (f, InstalledTask::Save(task), c, id)
            };
            f.owner
                .enable_native_operation_decoder_v1(
                    &id,
                    installed_decoder::limits(),
                    &control(&c),
                    &mut f.recorder,
                )
                .unwrap();
            let route = installed_observation::Route::install(&mut f.owner, &c, &mut f.recorder);
            f.recorder = match mode {
                0 => Recorder::disabled(),
                1 => Recorder::try_with_capacity(512).unwrap(),
                n => Recorder::try_with_capacity(if n == 2 { 512 } else { 1 })
                    .unwrap()
                    .with_native_timing(
                        vellum_engine::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
                    ),
            };
            let before = (f.owner.generation(), f.owner.retention_usage());
            for typed in ["typed", "dynamic"] {
                for operation in ["status", "retained-outcome", "minimal-control-status"] {
                    installed_decoder::envelope(
                        typed,
                        operation,
                        task.as_ref(),
                        &id,
                        &c.profile,
                        ["source-save", "annotation-save", "annotation-load"][kind],
                        None,
                        1.0,
                        |e| {
                            route.query(
                                &mut f.owner,
                                &id,
                                e,
                                &policy(&c),
                                &facts(&c),
                                None,
                                &mut f.recorder,
                            )
                        },
                    )
                    .unwrap();
                }
            }
            let mut denied = facts(&c);
            denied.operation_receipt_allowed = None;
            assert_eq!(
                installed_decoder::envelope(
                    "typed",
                    "retained-outcome",
                    task.as_ref(),
                    &id,
                    &c.profile,
                    ["source-save", "annotation-save", "annotation-load"][kind],
                    None,
                    1.0,
                    |e| {
                        route.query(
                            &mut f.owner,
                            &id,
                            e,
                            &policy(&c),
                            &denied,
                            None,
                            &mut f.recorder,
                        )
                    }
                ),
                Err(DiscoveredNativeOperationErrorV1::Native(
                    NativeOperationObservationErrorV1::Unavailable
                ))
            );
            assert_eq!((f.owner.generation(), f.owner.retention_usage()), before);
            let mut dispatches = 0;
            while let Some(record) = f.recorder.pop_front() {
                if record.boundary == Boundary::DiscoveryNativeOperationDispatch {
                    dispatches += 1;
                    assert_eq!(record.service_time.is_some(), mode == 2);
                }
            }
            if mode == 1 || mode == 2 {
                assert_eq!(dispatches, 7);
            }
            f.owner
                .retire_native_operation_observation_v1(&id, &control(&c), &mut f.recorder)
                .unwrap();
            route.retire(&mut f.owner, &mut f.recorder);
        }
    }
}
pub(super) fn obs_limits() -> NativeOperationObservationLimitsV1 {
    NativeOperationObservationLimitsV1 {
        max_waiters: 2,
        max_issued: 8,
        max_wait_steps: 8,
        max_context_bytes: 8192,
        max_wait_bytes: 512,
        max_reference_bytes: 128,
        max_reply_bytes: 16384,
        max_control_reply_bytes: 1024,
        max_held_bytes: 262144,
        max_label_bytes: 64,
    }
}
pub(super) fn policy(c: &SettingsExecutionContextV1) -> NativeOperationObservationPolicyV1 {
    NativeOperationObservationPolicyV1 {
        recipient: c.recipient.clone(),
        epoch: 1,
        grant: true,
        publication: false,
        control: true,
        effect: true,
        reasons: true,
        details: true,
        recovery: true,
    }
}
pub(super) fn facts(c: &SettingsExecutionContextV1) -> NativeOperationObservationFactsV1<'_> {
    NativeOperationObservationFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
        registry_ready: Some(true),
        retention_ready: Some(true),
        control_allowed: Some(true),
        control_ready: Some(true),
        ordinary_ready: Some(true),
        operation_receipt_admission_allowed: Some(true),
        operation_receipt_allowed: Some(true),
        source_readable: Some(true),
        annotation_readable: Some(true),
        disposition_readable: Some(true),
        read_allowed: Some(true),
        reply_ready: Some(true),
        wait_capacity: Some(true),
        wait_clock_valid: Some(true),
        owner_control_allowed: Some(true),
    }
}
pub(super) fn control(c: &SettingsExecutionContextV1) -> NativeOperationObservationControlV1<'_> {
    NativeOperationObservationControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(super) fn setup(annotation: bool) -> (Fixture, SaveId, SettingsExecutionContextV1) {
    let (mut f, id, c) = if annotation {
        annotations::observation_fixture()
    } else {
        let mut f = Fixture::new();
        f.owner
            .configure_save_operations_v1(limits(), &mut f.recorder)
            .unwrap();
        let c = context();
        let common = current();
        let a = admission(&common);
        let pair = f.pair();
        let target = f.port.source_binding_target(b"source").unwrap();
        let d = f.doc.clone();
        let req = request(&d, &target, b"save");
        let id = f
            .owner
            .request_owned_source_save_v1(
                &req,
                &release(&c),
                Some(&a),
                pair.as_ref(),
                &mut f.recorder,
            )
            .unwrap()
            .task;
        (f, id, c)
    };
    f.owner
        .configure_retention_budget(16777216, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_native_operation_observation_v1(
            NativeOperationObservationPoolLimitsV1 {
                max_contexts: 8,
                max_issued: 8,
                max_pool_bytes: 65536,
            },
            &mut f.recorder,
        )
        .unwrap();
    (f, id, c)
}
#[test]
fn source_and_annotation_receipts_project_actual_owned_pending_work_without_executing_it() {
    for annotation in [false, true] {
        let (mut f, task, c) = setup(annotation);
        let p = policy(&c);
        let mut g = facts(&c);
        g.read_allowed = None; // Write evidence does not borrow a load grant.
        if annotation {
            g.source_readable = Some(false);
        } else {
            g.annotation_readable = Some(false);
        }
        let before = f.owner.resource_usage();
        let generation = f.owner.generation();
        let id = f
            .owner
            .open_native_save_observation_v1(&task, &p, obs_limits(), &g, &mut f.recorder)
            .unwrap();
        let first = f
            .owner
            .query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::RetainedOutcome,
                &p,
                &g,
                &mut f.recorder,
            )
            .unwrap();
        let NativeOperationObservationReplyV1::Receipt {
            receipt: NativeOperationReceiptV1::Save(receipt),
            observation,
        } = first
        else {
            panic!("receipt")
        };
        assert_eq!(
            receipt.kind,
            if annotation {
                NativeSaveKindV1::AnnotationSave
            } else {
                NativeSaveKindV1::SourceSave
            }
        );
        assert_eq!(receipt.application, Application::Unknown);
        assert!(!receipt.requested_strength_satisfied);
        assert_eq!(receipt.phase, NativeSaveWritePhaseV1::Queued);
        assert!(receipt.version.is_none());
        let status = f
            .owner
            .query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::Status,
                &p,
                &g,
                &mut f.recorder,
            )
            .unwrap();
        assert_eq!(status.observation(), &observation);
        assert_eq!(f.owner.resource_usage(), before);
        assert_eq!(f.owner.generation(), generation);
    }
}
pub(super) fn open(
    f: &mut Fixture,
    task: &SaveId,
    c: &SettingsExecutionContextV1,
    l: NativeOperationObservationLimitsV1,
) -> NativeOperationObservationIdV1 {
    f.owner
        .open_native_save_observation_v1(task, &policy(c), l, &facts(c), &mut f.recorder)
        .unwrap()
}
#[test]
fn late_write_evidence_changes_ordinary_progress_but_not_control_or_baseline_truth() {
    for annotation in [false, true] {
        let (mut f, task, c) = setup(annotation);
        let p = policy(&c);
        let g = facts(&c);
        let id = open(&mut f, &task, &c, obs_limits());
        let minimal = f
            .owner
            .query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::MinimalControlStatus,
                &p,
                &g,
                &mut f.recorder,
            )
            .unwrap();
        let first = f
            .owner
            .query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::Status,
                &p,
                &g,
                &mut f.recorder,
            )
            .unwrap();
        let pair = f.pair().unwrap();
        let (key, token, bytes) = if annotation {
            (b"comments".as_slice(), b"a0".as_slice(), b"{}".as_slice())
        } else {
            (b"source".as_slice(), b"s0".as_slice(), b"text".as_slice())
        };
        let dispatch = f
            .owner
            .dispatch_save_with_pair(
                &task,
                f.port.read_result(key, token, bytes).unwrap(),
                pair,
                &mut f.recorder,
            )
            .unwrap();
        f.owner
            .record_save_evidence(
                f.port.no_future_effects(&dispatch).unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        let q = f
            .owner
            .query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::RetainedOutcome,
                &p,
                &g,
                &mut f.recorder,
            )
            .unwrap();
        let NativeOperationObservationReplyV1::Receipt {
            receipt: NativeOperationReceiptV1::Save(receipt),
            ..
        } = &q
        else {
            panic!("receipt")
        };
        assert!(receipt.safe_disposition);
        assert_eq!(receipt.application, Application::Unknown);
        assert!(!receipt.requested_strength_satisfied);
        assert_ne!(q.observation(), first.observation());
        f.owner
            .record_save_evidence(
                f.port
                    .result(
                        &dispatch,
                        Application::Written,
                        dispatch.bytes().len(),
                        Synchronization::NotRequested,
                        Some(b"PRIVATE_VERSION"),
                    )
                    .unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        let generation = f.owner.generation();
        let q = f
            .owner
            .query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::Status,
                &p,
                &g,
                &mut f.recorder,
            )
            .unwrap();
        let NativeOperationObservationReplyV1::Status {
            phase: NativeOperationObservationPhaseV1::Pending,
            receipt: Some(NativeOperationReceiptV1::Save(receipt)),
            ..
        } = q
        else {
            panic!("pending written evidence")
        };
        assert!(receipt.requested_strength_satisfied);
        assert!(!receipt.quiescent);
        assert_eq!(receipt.version.as_ref().unwrap().epoch(), None);
        assert_eq!(
            receipt.version.as_ref().unwrap().token(),
            b"PRIVATE_VERSION"
        );
        assert!(!format!("{receipt:?}").contains("PRIVATE"));
        let same = f
            .owner
            .query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::MinimalControlStatus,
                &p,
                &g,
                &mut f.recorder,
            )
            .unwrap();
        assert_eq!(same.observation(), minimal.observation());
        assert_eq!(f.owner.generation(), generation);
        f.owner.stop_save(&task, &mut f.recorder).unwrap();
        let stopped = f
            .owner
            .query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::MinimalControlStatus,
                &p,
                &g,
                &mut f.recorder,
            )
            .unwrap();
        assert_ne!(stopped.observation(), minimal.observation());
        f.owner
            .record_save_evidence(f.port.quiescence(&dispatch).unwrap(), &mut f.recorder)
            .unwrap();
        let done = f
            .owner
            .query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::MinimalControlStatus,
                &p,
                &g,
                &mut f.recorder,
            )
            .unwrap();
        assert!(matches!(
            done,
            NativeOperationObservationReplyV1::Status {
                phase: NativeOperationObservationPhaseV1::Terminal,
                receipt: None,
                ..
            }
        ));
        assert!(
            f.owner
                .native_operation_observation_usage_v1(&id)
                .unwrap()
                .control_issued
                <= 4
        );
    }
}
#[test]
fn disclosure_ceiling_temporary_denials_and_absorbing_scope_loss_remain_distinct() {
    let (mut f, task, c) = setup(false);
    let mut ceiling = policy(&c);
    ceiling.effect = false;
    let mut g = facts(&c);
    g.operation_receipt_admission_allowed = None;
    g.operation_receipt_allowed = None;
    g.source_readable = None;
    let id = f
        .owner
        .open_native_save_observation_v1(&task, &ceiling, obs_limits(), &g, &mut f.recorder)
        .unwrap();
    let p = policy(&c);
    assert_eq!(
        f.owner.query_native_operation_observation_v1(
            &id,
            NativeOperationObservationQueryV1::RetainedOutcome,
            &p,
            &facts(&c),
            &mut f.recorder
        ),
        Err(NativeOperationObservationErrorV1::Unavailable)
    );
    g = facts(&c);
    g.ordinary_ready = Some(false);
    g.resources_ready = Some(false);
    g.source_readable = Some(false);
    assert_eq!(
        f.owner.query_native_operation_observation_v1(
            &id,
            NativeOperationObservationQueryV1::Status,
            &p,
            &g,
            &mut f.recorder
        ),
        Err(NativeOperationObservationErrorV1::Unavailable)
    );
    assert!(matches!(
        f.owner
            .query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::MinimalControlStatus,
                &p,
                &g,
                &mut f.recorder
            )
            .unwrap(),
        NativeOperationObservationReplyV1::Status { receipt: None, .. }
    ));
    assert!(
        f.owner
            .native_operation_observation_usage_v1(&id)
            .unwrap()
            .live
    );
    let mut wrong = p.clone();
    wrong.epoch += 1;
    assert_eq!(
        f.owner.query_native_operation_observation_v1(
            &id,
            NativeOperationObservationQueryV1::Status,
            &wrong,
            &facts(&c),
            &mut f.recorder
        ),
        Err(NativeOperationObservationErrorV1::Unavailable)
    );
    assert!(
        f.owner
            .native_operation_observation_usage_v1(&id)
            .unwrap()
            .live
    );
    for field in 0..5 {
        let id = open(&mut f, &task, &c, obs_limits());
        let mut g = facts(&c);
        match field {
            0 => g.authorized = Some(false),
            1 => g.scope = Some(false),
            2 => g.continuous = Some(false),
            3 => g.live = Some(false),
            _ => g.retained = Some(false),
        }
        let funding = f.owner.retention_usage();
        assert_eq!(
            f.owner.query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::Status,
                &p,
                &g,
                &mut f.recorder
            ),
            Err(NativeOperationObservationErrorV1::Unavailable)
        );
        assert!(
            !f.owner
                .native_operation_observation_usage_v1(&id)
                .unwrap()
                .live
        );
        assert_eq!(f.owner.retention_usage(), funding);
        assert_eq!(
            f.owner.query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::MinimalControlStatus,
                &p,
                &facts(&c),
                &mut f.recorder
            ),
            Err(NativeOperationObservationErrorV1::Unavailable)
        );
        f.owner
            .retire_native_operation_observation_v1(&id, &control(&c), &mut f.recorder)
            .unwrap();
    }
    for denied in [None, Some(false)] {
        for field in 0..16 {
            let (mut f, task, c) = setup(false);
            let mut g = facts(&c);
            match field {
                0 => g.authorized = denied,
                1 => g.scope = denied,
                2 => g.continuous = denied,
                3 => g.live = denied,
                4 => g.resources_ready = denied,
                5 => g.retained = denied,
                6 => g.transfer_ready = denied,
                7 => g.preparation_reserved = denied,
                8 => g.registry_ready = denied,
                9 => g.retention_ready = denied,
                10 => g.control_allowed = denied,
                11 => g.operation_receipt_admission_allowed = denied,
                12 => g.operation_receipt_allowed = denied,
                13 => g.source_readable = denied,
                14 => g.disposition_readable = denied,
                _ => g.reply_ready = denied,
            }
            let funding = f.owner.retention_usage();
            let resources = f.owner.resource_usage();
            assert_eq!(
                f.owner.open_native_save_observation_v1(
                    &task,
                    &policy(&c),
                    obs_limits(),
                    &g,
                    &mut f.recorder
                ),
                Err(NativeOperationObservationErrorV1::Unavailable)
            );
            assert_eq!(f.owner.retention_usage(), funding);
            assert_eq!(f.owner.resource_usage(), resources);
        }
    }
}
#[test]
fn raw_targets_refuse_and_observers_never_pin_or_revive_a_retired_producer() {
    for annotation in [false, true] {
        let (mut f, task, c) = setup(annotation);
        let id = open(&mut f, &task, &c, obs_limits());
        let p = policy(&c);
        if !annotation {
            let pair = f.pair().unwrap();
            let raw = f
                .owner
                .queue_pinned_save_with_pair(
                    &f.doc,
                    request(
                        &f.doc,
                        &f.port.source_binding_target(b"source").unwrap(),
                        b"unused",
                    )
                    .requirements,
                    pair,
                    &mut f.recorder,
                )
                .unwrap();
            assert_eq!(
                f.owner.open_native_save_observation_v1(
                    &raw,
                    &p,
                    obs_limits(),
                    &facts(&c),
                    &mut f.recorder
                ),
                Err(NativeOperationObservationErrorV1::Unavailable)
            );
            f.owner.stop_save(&raw, &mut f.recorder).unwrap();
            f.owner.retire_save(&raw, &mut f.recorder).unwrap();
        }
        let funding = f.owner.retention_usage();
        f.owner.stop_save(&task, &mut f.recorder).unwrap();
        f.owner.retire_save(&task, &mut f.recorder).unwrap();
        assert_eq!(
            f.owner.query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::RetainedOutcome,
                &p,
                &facts(&c),
                &mut f.recorder
            ),
            Err(NativeOperationObservationErrorV1::Unavailable)
        );
        assert!(
            !f.owner
                .native_operation_observation_usage_v1(&id)
                .unwrap()
                .live
        );
        assert_eq!(f.owner.retention_usage(), funding);
        f.owner
            .retire_native_operation_observation_v1(&id, &control(&c), &mut f.recorder)
            .unwrap();
        f.owner
            .stop_native_operation_observation_admission_v1(&mut f.recorder)
            .unwrap();
        f.owner
            .retire_native_operation_observation_pool_v1(
                Some(true),
                Some(true),
                Some(true),
                &mut f.recorder,
            )
            .unwrap();
        assert_eq!(f.owner.retention_usage().reserved, 0);
        assert_eq!(
            f.owner.configure_native_operation_observation_v1(
                NativeOperationObservationPoolLimitsV1 {
                    max_contexts: 8,
                    max_issued: 8,
                    max_pool_bytes: 65536
                },
                &mut f.recorder
            ),
            Err(NativeOperationObservationErrorV1::Unavailable)
        );
    }
}
#[test]
fn zero_ordinary_issuance_preserves_protected_control_without_rich_disclosure() {
    let (mut f, task, c) = setup(false);
    let mut l = obs_limits();
    l.max_issued = 0;
    let id = open(&mut f, &task, &c, l);
    let p = policy(&c);
    assert_eq!(
        f.owner.query_native_operation_observation_v1(
            &id,
            NativeOperationObservationQueryV1::RetainedOutcome,
            &p,
            &facts(&c),
            &mut f.recorder
        ),
        Err(NativeOperationObservationErrorV1::Limit)
    );
    let mut g = facts(&c);
    g.ordinary_ready = None;
    g.resources_ready = None;
    g.operation_receipt_allowed = None;
    g.source_readable = None;
    g.reply_ready = None;
    f.owner
        .stop_native_operation_observation_admission_v1(&mut f.recorder)
        .unwrap();
    let q = f
        .owner
        .query_native_operation_observation_v1(
            &id,
            NativeOperationObservationQueryV1::MinimalControlStatus,
            &p,
            &g,
            &mut f.recorder,
        )
        .unwrap();
    assert!(matches!(
        &q,
        NativeOperationObservationReplyV1::Status { receipt: None, .. }
    ));
    assert_eq!(
        f.owner.start_native_operation_observation_wait_v1(
            &id,
            NativeOperationWaitRequestV1 {
                after: q.observation(),
                budget: 1,
                clock: NativeOperationWaitClockV1 { epoch: 1, tick: 0 }
            },
            &p,
            &facts(&c),
            &mut f.recorder
        ),
        Err(NativeOperationObservationErrorV1::Unavailable)
    );
    assert_eq!(
        f.owner
            .native_operation_observation_usage_v1(&id)
            .unwrap()
            .ordinary_issued,
        0
    );
}
#[test]
fn waiter_budget_is_nonrenewable_slots_require_discharge_and_observer_stop_never_stops_work() {
    let (mut f, task, c) = setup(false);
    let id = open(&mut f, &task, &c, obs_limits());
    let p = policy(&c);
    let g = facts(&c);
    let q = f
        .owner
        .query_native_operation_observation_v1(
            &id,
            NativeOperationObservationQueryV1::Status,
            &p,
            &g,
            &mut f.recorder,
        )
        .unwrap();
    let w = f
        .owner
        .start_native_operation_observation_wait_v1(
            &id,
            NativeOperationWaitRequestV1 {
                after: q.observation(),
                budget: 2,
                clock: NativeOperationWaitClockV1 { epoch: 7, tick: 10 },
            },
            &p,
            &g,
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner.advance_native_operation_observation_wait_v1(
            &id,
            &w,
            NativeOperationWaitEventV1::Poll,
            &p,
            &g,
            &mut f.recorder
        ),
        Ok(NativeOperationWaitReplyV1::Waiting(w.clone()))
    );
    for (tick, end) in [(11, false), (12, true)] {
        let out = f
            .owner
            .advance_native_operation_observation_wait_v1(
                &id,
                &w,
                NativeOperationWaitEventV1::Tick(NativeOperationWaitClockV1 { epoch: 7, tick }),
                &p,
                &g,
                &mut f.recorder,
            )
            .unwrap();
        assert_eq!(
            out,
            if end {
                NativeOperationWaitReplyV1::Ended
            } else {
                NativeOperationWaitReplyV1::Waiting(w.clone())
            }
        );
    }
    assert_eq!(
        f.owner
            .retire_native_operation_observation_v1(&id, &control(&c), &mut f.recorder),
        Err(NativeOperationObservationErrorV1::Unavailable)
    );
    assert_eq!(
        f.owner.advance_native_operation_observation_wait_v1(
            &id,
            &w,
            NativeOperationWaitEventV1::Poll,
            &p,
            &g,
            &mut f.recorder
        ),
        Err(NativeOperationObservationErrorV1::Unavailable)
    );
    let second = f
        .owner
        .start_native_operation_observation_wait_v1(
            &id,
            NativeOperationWaitRequestV1 {
                after: q.observation(),
                budget: 8,
                clock: NativeOperationWaitClockV1 { epoch: 7, tick: 12 },
            },
            &p,
            &g,
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner.start_native_operation_observation_wait_v1(
            &id,
            NativeOperationWaitRequestV1 {
                after: q.observation(),
                budget: 1,
                clock: NativeOperationWaitClockV1 { epoch: 7, tick: 12 }
            },
            &p,
            &g,
            &mut f.recorder
        ),
        Err(NativeOperationObservationErrorV1::Limit)
    );
    let mut denied = control(&c);
    denied.delivery_drained = None;
    assert_eq!(
        f.owner
            .retire_native_operation_observation_wait_v1(&id, &w, &denied, &mut f.recorder),
        Err(NativeOperationObservationErrorV1::Unavailable)
    );
    f.owner
        .retire_native_operation_observation_wait_v1(&id, &w, &control(&c), &mut f.recorder)
        .unwrap();
    let mut denied = facts(&c);
    denied.control_allowed = Some(false);
    denied.operation_receipt_allowed = Some(false);
    assert_eq!(
        f.owner.advance_native_operation_observation_wait_v1(
            &id,
            &second,
            NativeOperationWaitEventV1::Stop,
            &p,
            &denied,
            &mut f.recorder
        ),
        Ok(NativeOperationWaitReplyV1::Ended)
    );
    assert!(
        !f.owner
            .save_outcome(&task, &mut f.recorder)
            .unwrap()
            .stop_requested
    );
    f.owner
        .retire_native_operation_observation_wait_v1(&id, &second, &control(&c), &mut f.recorder)
        .unwrap();
    let next = f
        .owner
        .start_native_operation_observation_wait_v1(
            &id,
            NativeOperationWaitRequestV1 {
                after: q.observation(),
                budget: 8,
                clock: NativeOperationWaitClockV1 { epoch: 7, tick: 12 },
            },
            &p,
            &g,
            &mut f.recorder,
        )
        .unwrap();
    assert_ne!(next, w);
    f.owner.stop_save(&task, &mut f.recorder).unwrap();
    assert!(matches!(
        f.owner
            .advance_native_operation_observation_wait_v1(
                &id,
                &next,
                NativeOperationWaitEventV1::Poll,
                &p,
                &g,
                &mut f.recorder
            )
            .unwrap(),
        NativeOperationWaitReplyV1::Observed(NativeOperationObservationReplyV1::Status {
            phase: NativeOperationObservationPhaseV1::Terminal,
            ..
        })
    ));
}
#[test]
fn foreign_progress_and_invalid_clock_evidence_do_not_acquire_or_renew_waits() {
    let (mut f, task, c) = setup(false);
    let id = open(&mut f, &task, &c, obs_limits());
    let other = open(&mut f, &task, &c, obs_limits());
    let p = policy(&c);
    let g = facts(&c);
    let q = f
        .owner
        .query_native_operation_observation_v1(
            &id,
            NativeOperationObservationQueryV1::Status,
            &p,
            &g,
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner.start_native_operation_observation_wait_v1(
            &other,
            NativeOperationWaitRequestV1 {
                after: q.observation(),
                budget: 1,
                clock: NativeOperationWaitClockV1 { epoch: 1, tick: 0 }
            },
            &p,
            &g,
            &mut f.recorder
        ),
        Err(NativeOperationObservationErrorV1::Unavailable)
    );
    let w = f
        .owner
        .start_native_operation_observation_wait_v1(
            &id,
            NativeOperationWaitRequestV1 {
                after: q.observation(),
                budget: 3,
                clock: NativeOperationWaitClockV1 { epoch: 1, tick: 5 },
            },
            &p,
            &g,
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner.advance_native_operation_observation_wait_v1(
            &id,
            &w,
            NativeOperationWaitEventV1::Tick(NativeOperationWaitClockV1 { epoch: 2, tick: 6 }),
            &p,
            &g,
            &mut f.recorder
        ),
        Err(NativeOperationObservationErrorV1::Unavailable)
    );
    assert_eq!(
        f.owner.advance_native_operation_observation_wait_v1(
            &id,
            &w,
            NativeOperationWaitEventV1::Poll,
            &p,
            &g,
            &mut f.recorder
        ),
        Err(NativeOperationObservationErrorV1::Unavailable)
    );
    assert_eq!(
        f.owner
            .native_operation_observation_usage_v1(&id)
            .unwrap()
            .waiters,
        1
    );
    assert!(
        !f.owner
            .native_operation_observation_usage_v1(&id)
            .unwrap()
            .receipt_faulted
    );
    let next = f
        .owner
        .start_native_operation_observation_wait_v1(
            &id,
            NativeOperationWaitRequestV1 {
                after: q.observation(),
                budget: 3,
                clock: NativeOperationWaitClockV1 { epoch: 1, tick: 5 },
            },
            &p,
            &g,
            &mut f.recorder,
        )
        .unwrap();
    let mut lost = facts(&c);
    lost.scope = Some(false);
    let funding = f.owner.retention_usage();
    assert_eq!(
        f.owner.advance_native_operation_observation_wait_v1(
            &id,
            &next,
            NativeOperationWaitEventV1::Tick(NativeOperationWaitClockV1 { epoch: 1, tick: 6 }),
            &p,
            &lost,
            &mut f.recorder
        ),
        Err(NativeOperationObservationErrorV1::Unavailable)
    );
    assert!(
        !f.owner
            .native_operation_observation_usage_v1(&id)
            .unwrap()
            .live
    );
    assert_eq!(f.owner.retention_usage(), funding);
}

#[test]
fn typed_after_versions_and_requested_synchronization_survive_the_scoped_receipt_projection() {
    let (mut f, _, c) = setup(false);
    let provider = f
        .owner
        .register_provider_with_causal_capabilities(
            ProviderProfile {
                synchronization_supported: true,
            },
            ProviderDestinationCapabilitiesV1::default(),
            ProviderCausalCapabilitiesV1 {
                causal_receipts: true,
                non_reuse_tokens: false,
            },
            None,
            &mut f.recorder,
        )
        .unwrap();
    let before = ResourceVersionV1 {
        epoch: b"old-epoch",
        token: b"v0",
    };
    let d = f
        .owner
        .load_resource(
            provider
                .read_result_causal(b"typed", before, b"text")
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let target = provider.source_binding_target(b"typed").unwrap();
    let common = current();
    let a = admission(&common);
    let mut req = request(&d, &target, b"typed-save");
    req.requirements.acknowledgment = Acknowledgment::Synchronized;
    let task = f
        .owner
        .request_owned_source_save_v1(&req, &release(&c), Some(&a), None, &mut f.recorder)
        .unwrap()
        .task;
    let id = open(&mut f, &task, &c, obs_limits());
    let p = policy(&c);
    let g = facts(&c);
    let dispatch = f
        .owner
        .dispatch_save(
            &task,
            provider
                .read_result_causal(b"typed", before, b"text")
                .unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let after = ResourceVersionV1 {
        epoch: b"actual-after-epoch",
        token: b"v1",
    };
    for sync in [Synchronization::Unknown, Synchronization::Succeeded] {
        f.owner
            .record_save_evidence(
                provider
                    .result_causal(&dispatch, Application::Written, 4, sync, Some(after))
                    .unwrap(),
                &mut f.recorder,
            )
            .unwrap();
        let NativeOperationObservationReplyV1::Receipt {
            receipt: NativeOperationReceiptV1::Save(receipt),
            ..
        } = f
            .owner
            .query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::RetainedOutcome,
                &p,
                &g,
                &mut f.recorder,
            )
            .unwrap()
        else {
            panic!("receipt")
        };
        assert_eq!(
            receipt.required_acknowledgment,
            NativeSaveRequiredAcknowledgmentV1::Synchronized
        );
        assert_eq!(
            receipt.requested_strength_satisfied,
            sync == Synchronization::Succeeded
        );
        assert_eq!(receipt.version.as_ref().unwrap().epoch(), Some(after.epoch));
        assert_eq!(receipt.version.as_ref().unwrap().token(), after.token);
        assert!(!receipt.quiescent);
        assert!(!receipt.safe_disposition);
    }
}
#[test]
fn native_observation_capture_modes_preserve_evidence_and_never_record_private_payloads() {
    for capacity in [None, Some(0), Some(128)] {
        let (mut f, task, c) = setup(false);
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let before = f.owner.resource_usage();
        let id = open(&mut f, &task, &c, obs_limits());
        let p = policy(&c);
        f.owner
            .query_native_operation_observation_v1(
                &id,
                NativeOperationObservationQueryV1::RetainedOutcome,
                &p,
                &facts(&c),
                &mut f.recorder,
            )
            .unwrap();
        f.owner
            .stop_native_operation_observation_v1(&id, &control(&c), &mut f.recorder)
            .unwrap();
        f.owner
            .retire_native_operation_observation_v1(&id, &control(&c), &mut f.recorder)
            .unwrap();
        assert_eq!(f.owner.resource_usage(), before);
        let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
        assert!(!format!("{records:?}").contains("recipient"));
        if capacity == Some(128) {
            for boundary in [
                Boundary::NativeOperationObservationAdmission,
                Boundary::NativeOperationObservationSnapshot,
                Boundary::NativeOperationObservationResultValidation,
                Boundary::NativeOperationObservationQuery,
                Boundary::NativeOperationObservationClose,
                Boundary::NativeOperationObservationRetirement,
            ] {
                assert!(records.iter().any(|v| v.boundary == boundary));
            }
        } else {
            assert!(records.is_empty());
        }
    }
}
