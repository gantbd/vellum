use super::*;
#[test]
fn operation_service_timing_is_optional_and_does_not_change_recovery_or_discharge() {
    use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1};
    for timed in [false, true] {
        for capacity in [None, Some(0), Some(1), Some(256)] {
            let mut f = setup();
            let c = context();
            f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            if timed {
                f.recorder = f
                    .recorder
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
            }
            let initial = f.owner.retention_usage();
            f.owner
                .configure_source_read_operations_v1(operation_limits(), &mut f.recorder)
                .unwrap();
            let target = f.port.source_read_target_v1(b"PRIVATE-TARGET").unwrap();
            let q = request(&target, b"PRIVATE-OPERATION");
            let mut denied = facts(&c);
            denied.authorized = None;
            assert_eq!(
                f.owner
                    .request_owned_source_read_v1(&q, &denied, &mut f.recorder),
                Err(ResourceError::Unavailable)
            );
            let first = f
                .owner
                .request_owned_source_read_v1(&q, &facts(&c), &mut f.recorder)
                .unwrap();
            let funding = f.owner.retention_usage();
            let generation = f.owner.generation();
            let replay = f
                .owner
                .request_owned_source_read_v1(&q, &facts(&c), &mut f.recorder)
                .unwrap();
            assert_eq!(replay.task, first.task);
            assert_eq!(
                replay.status,
                SourceReadOperationAdmissionStatusV1::ObserveExisting
            );
            assert_eq!(f.owner.retention_usage(), funding);
            assert_eq!(f.owner.generation(), generation);
            // The legacy explicitly labeled path shares this admission boundary,
            // but must not adopt the already-owned operation.
            let raw = f
                .owner
                .admit_labeled_source_read_v1(
                    SourceReadOperationOriginV1 {
                        operation: b"PRIVATE-RAW",
                        effect: b"read",
                        attempt: b"one",
                    },
                    f.port.source_read_target_v1(b"PRIVATE-RAW-TARGET").unwrap(),
                    ReadStrength::CapturedRead,
                    &mut f.recorder,
                )
                .unwrap();
            assert_ne!(raw, first.task);
            f.owner.stop_source_read_v1(&raw, &mut f.recorder).unwrap();
            f.owner
                .retire_source_read_v1(&raw, &mut f.recorder)
                .unwrap();
            f.owner
                .stop_source_read_admission_v1(&mut f.recorder)
                .unwrap();
            assert_eq!(
                f.owner.retire_source_read_operations_v1(
                    Some(true),
                    Some(true),
                    Some(true),
                    &mut f.recorder
                ),
                Err(ResourceError::Retained)
            );
            f.owner
                .stop_source_read_v1(&first.task, &mut f.recorder)
                .unwrap();
            f.owner
                .retire_source_read_v1(&first.task, &mut f.recorder)
                .unwrap();
            f.owner
                .retire_source_read_operations_v1(
                    Some(true),
                    Some(true),
                    Some(true),
                    &mut f.recorder,
                )
                .unwrap();
            assert_eq!(f.owner.retention_usage(), initial);
            let events: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
            assert!(!format!("{events:?}").contains("PRIVATE"));
            if capacity == Some(256) {
                for boundary in [
                    Boundary::SourceReadOperationConfiguration,
                    Boundary::SourceReadOperationAdmission,
                    Boundary::SourceReadOperationRecovery,
                    Boundary::SourceReadOperationRetirement,
                ] {
                    let matching: Vec<_> =
                        events.iter().filter(|e| e.boundary == boundary).collect();
                    assert!(!matching.is_empty());
                    assert!(
                        matching.iter().all(|e| e.service_time.is_some() == timed),
                        "{boundary:?}"
                    );
                }
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == Boundary::SourceReadOperationAdmission
                            && e.disposition == Disposition::Refused)
                );
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == Boundary::SourceReadOperationRetirement
                            && e.disposition == Disposition::Refused)
                );
            }
            if capacity == Some(1) {
                assert!(f.recorder.dropped() > 0);
            }
            if capacity.is_none() || capacity == Some(0) {
                assert!(events.is_empty());
            }
        }
    }
}
pub(super) fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "authority".into(),
        engine: "engine".into(),
        recipient: "recipient".into(),
        purpose: "load".into(),
        destination: "destination".into(),
        profile: "native/1".into(),
        continuity: "continuity".into(),
    }
}
fn operation_limits() -> SourceReadOperationLimitsV1 {
    SourceReadOperationLimitsV1 {
        max_capture_bytes: 8192,
        max_reply_bytes: 1024,
        max_label_bytes: 64,
        max_store_bytes: 262144,
    }
}
pub(super) fn facts(c: &SettingsExecutionContextV1) -> TrustedSourceReadOperationFactsV1<'_> {
    TrustedSourceReadOperationFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        read_allowed: Some(true),
        retained: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
        resources_ready: Some(true),
        identity_allowed: Some(true),
        binding_coverage_complete: Some(true),
        provider_available: Some(true),
        launch_allowed: Some(true),
    }
}
pub(super) fn request<'a>(
    target: &'a SourceReadTargetV1,
    operation: &'a [u8],
) -> OwnedSourceReadRequestV1<'a> {
    OwnedSourceReadRequestV1 {
        origin: SourceReadOperationOriginV1 {
            operation,
            effect: b"read",
            attempt: b"one",
        },
        target,
        strength: ReadStrength::CapturedRead,
    }
}
pub(super) fn configured() -> (Fixture, SettingsExecutionContextV1) {
    let mut f = setup();
    let c = context();
    f.owner
        .configure_source_read_operations_v1(operation_limits(), &mut f.recorder)
        .unwrap();
    (f, c)
}
#[test]
fn matching_owned_source_load_returns_one_actual_ready_task_without_submission_or_publication() {
    let mut f = setup();
    let c = context();
    f.owner
        .configure_source_read_operations_v1(operation_limits(), &mut f.recorder)
        .unwrap();
    let target = f.port.source_read_target_v1(b"new").unwrap();
    let req = request(&target, b"load");
    let before = f.owner.generation();
    let first = f
        .owner
        .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder)
        .unwrap();
    assert_eq!(first.status, SourceReadOperationAdmissionStatusV1::Admitted);
    let usage = f.owner.source_reader_usage_v1().unwrap();
    let funding = f.owner.retention_usage();
    let second = f
        .owner
        .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder)
        .unwrap();
    assert_eq!(
        second.status,
        SourceReadOperationAdmissionStatusV1::ObserveExisting
    );
    assert_eq!(second.task, first.task);
    assert_eq!(
        f.owner
            .source_read_status_v1(&first.task, &mut f.recorder)
            .unwrap()
            .phase,
        ReadPhase::Ready
    );
    assert_eq!(f.owner.source_reader_usage_v1().unwrap(), usage);
    assert_eq!(f.owner.retention_usage(), funding);
    assert_eq!(f.owner.generation(), before);
}
#[test]
fn every_current_release_and_fresh_launch_fact_is_exact_and_recovery_never_relaunches() {
    let (mut f, c) = configured();
    let target = f.port.source_read_target_v1(b"new").unwrap();
    let req = request(&target, b"load");
    for field in 0..14 {
        for denied in [None, Some(false)] {
            let mut g = facts(&c);
            let slot = match field {
                0 => &mut g.authorized,
                1 => &mut g.scope,
                2 => &mut g.continuous,
                3 => &mut g.live,
                4 => &mut g.not_stopped,
                5 => &mut g.read_allowed,
                6 => &mut g.retained,
                7 => &mut g.reply_ready,
                8 => &mut g.transfer_ready,
                9 => &mut g.resources_ready,
                10 => &mut g.identity_allowed,
                11 => &mut g.binding_coverage_complete,
                12 => &mut g.provider_available,
                _ => &mut g.launch_allowed,
            };
            *slot = denied;
            assert_eq!(
                f.owner
                    .request_owned_source_read_v1(&req, &g, &mut f.recorder),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(f.owner.source_reader_usage_v1().unwrap().issued, 0);
        }
    }
    let id = f
        .owner
        .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder)
        .unwrap()
        .task;
    f.owner
        .stop_source_read_admission_v1(&mut f.recorder)
        .unwrap();
    let mut g = facts(&c);
    g.identity_allowed = None;
    g.binding_coverage_complete = Some(false);
    g.provider_available = None;
    g.launch_allowed = None;
    assert_eq!(
        f.owner
            .request_owned_source_read_v1(&req, &g, &mut f.recorder)
            .unwrap()
            .task,
        id
    );
    for field in 0..10 {
        let mut g = facts(&c);
        let slot = match field {
            0 => &mut g.authorized,
            1 => &mut g.scope,
            2 => &mut g.continuous,
            3 => &mut g.live,
            4 => &mut g.not_stopped,
            5 => &mut g.read_allowed,
            6 => &mut g.retained,
            7 => &mut g.reply_ready,
            8 => &mut g.transfer_ready,
            _ => &mut g.resources_ready,
        };
        *slot = None;
        assert_eq!(
            f.owner
                .request_owned_source_read_v1(&req, &g, &mut f.recorder),
            Err(ResourceError::Unavailable)
        );
    }
    assert_eq!(f.owner.source_reader_usage_v1().unwrap().issued, 1);
}
#[test]
fn changed_complete_intent_context_or_actual_registration_cannot_retarget_an_operation() {
    let (mut f, c) = configured();
    let target = f.port.source_read_target_v1(b"new").unwrap();
    let req = request(&target, b"load");
    f.owner
        .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder)
        .unwrap();
    for field in 0..7 {
        let mut changed = c.clone();
        let slot = match field {
            0 => &mut changed.authority,
            1 => &mut changed.engine,
            2 => &mut changed.recipient,
            3 => &mut changed.purpose,
            4 => &mut changed.destination,
            5 => &mut changed.profile,
            _ => &mut changed.continuity,
        };
        *slot = "changed".into();
        assert_eq!(
            f.owner
                .request_owned_source_read_v1(&req, &facts(&changed), &mut f.recorder),
            Err(ResourceError::Unavailable)
        );
    }
    let other_key = f.port.source_read_target_v1(b"other").unwrap();
    let reader = f
        .owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            Some(&f.port.identity_domain()),
            &mut f.recorder,
        )
        .unwrap();
    let other_registration = reader.source_read_target_v1(b"new").unwrap();
    for field in 0..5 {
        let mut changed = request(&target, b"load");
        match field {
            0 => changed.origin.effect = b"other",
            1 => changed.origin.attempt = b"two",
            2 => changed.target = &other_key,
            3 => changed.target = &other_registration,
            _ => changed.strength = ReadStrength::AtomicResourceSnapshot,
        }
        assert_eq!(
            f.owner
                .request_owned_source_read_v1(&changed, &facts(&c), &mut f.recorder),
            Err(ResourceError::Unavailable)
        );
    }
    assert_eq!(f.owner.source_reader_usage_v1().unwrap().issued, 1);
}
#[test]
fn labeled_raw_origins_cannot_be_adopted_and_owned_origins_refuse_raw_or_relabeled_attempts() {
    let (mut f, c) = configured();
    let target = f.port.source_read_target_v1(b"new").unwrap();
    let req = request(&target, b"raw");
    let raw = f
        .owner
        .admit_labeled_source_read_v1(
            req.origin,
            f.port.source_read_target_v1(b"new").unwrap(),
            req.strength,
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner
            .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
    f.owner.stop_source_read_v1(&raw, &mut f.recorder).unwrap();
    f.owner
        .retire_source_read_v1(&raw, &mut f.recorder)
        .unwrap();
    let mut changed = request(&target, b"raw");
    changed.origin.attempt = b"two";
    assert_eq!(
        f.owner
            .request_owned_source_read_v1(&changed, &facts(&c), &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
    let req = request(&target, b"owned");
    let owned = f
        .owner
        .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder)
        .unwrap()
        .task;
    assert_eq!(
        f.owner.admit_labeled_source_read_v1(
            req.origin,
            f.port.source_read_target_v1(b"new").unwrap(),
            req.strength,
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
    f.owner
        .stop_source_read_v1(&owned, &mut f.recorder)
        .unwrap();
    f.owner
        .retire_source_read_v1(&owned, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
    let usage = f.owner.source_read_operation_usage_v1().unwrap();
    assert_eq!(
        (
            usage.raw_labeled,
            usage.owned_live,
            usage.retired,
            usage.spent_origins
        ),
        (1, 0, 2, 2)
    );
}
#[test]
fn refused_child_and_unlabeled_raw_work_never_spend_or_adopt_an_owned_parent() {
    let (mut f, c) = configured();
    let raw = admit(&mut f, b"new");
    let target = f.port.source_read_target_v1(b"new").unwrap();
    let mut req = request(&target, b"load");
    let funding = f.owner.retention_usage();
    req.strength = ReadStrength::AtomicResourceSnapshot;
    assert_eq!(
        f.owner
            .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder),
        Err(ResourceError::Unsupported)
    );
    assert_eq!(f.owner.source_read_operation_usage_v1().unwrap().issued, 0);
    assert_eq!(f.owner.retention_usage(), funding);
    req.strength = ReadStrength::CapturedRead;
    let owned = f
        .owner
        .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder)
        .unwrap()
        .task;
    assert_ne!(owned, raw);
    assert_eq!(f.owner.source_reader_usage_v1().unwrap().issued, 2);
}
#[test]
fn matching_recovery_preserves_actual_eof_activation_stop_and_quiescence_without_repeating_them() {
    let (mut f, c) = configured();
    let target = f.port.source_read_target_v1(b"new").unwrap();
    let req = request(&target, b"load");
    let id = f
        .owner
        .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder)
        .unwrap()
        .task;
    let dispatch = f.owner.submit_source_read_v1(&id, &mut f.recorder).unwrap();
    complete(&mut f, &dispatch, b"text");
    let activation = f
        .owner
        .activate_source_read_v1(&id, &mut f.recorder)
        .unwrap();
    let generation = f.owner.generation();
    f.owner.stop_source_read_v1(&id, &mut f.recorder).unwrap();
    f.owner
        .record_source_read_v1(
            f.port.source_read_quiescence_v1(&dispatch).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    let before = f.owner.source_read_status_v1(&id, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner
            .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder)
            .unwrap()
            .task,
        id
    );
    assert_eq!(
        f.owner.source_read_status_v1(&id, &mut f.recorder).unwrap(),
        before
    );
    assert_eq!(before.activated, Some(activation));
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(
        f.owner.retire_source_read_v1(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    let funding = f.owner.retention_usage();
    drop(dispatch);
    f.owner.retire_source_read_v1(&id, &mut f.recorder).unwrap();
    assert_eq!(f.owner.source_read_operation_usage_v1().unwrap().retired, 1);
    assert_eq!(f.owner.retention_usage(), funding);
}
#[test]
fn separate_operation_and_reader_sponsorship_require_actual_global_drain_and_irreversible_retirement()
 {
    let (mut f, c) = configured();
    let target = f.port.source_read_target_v1(b"new").unwrap();
    let req = request(&target, b"load");
    let id = f
        .owner
        .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder)
        .unwrap()
        .task;
    f.owner
        .stop_source_read_admission_v1(&mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner.retire_source_read_operations_v1(
            Some(true),
            Some(true),
            Some(true),
            &mut f.recorder
        ),
        Err(ResourceError::Retained)
    );
    f.owner.stop_source_read_v1(&id, &mut f.recorder).unwrap();
    f.owner.retire_source_read_v1(&id, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner
            .retire_source_reader_v1(Some(true), Some(true), Some(true), &mut f.recorder),
        Err(ResourceError::Retained)
    );
    let funding = f.owner.retention_usage();
    for field in 0..3 {
        let mut g = [Some(true); 3];
        g[field] = None;
        assert_eq!(
            f.owner
                .retire_source_read_operations_v1(g[0], g[1], g[2], &mut f.recorder),
            Err(ResourceError::Retained)
        );
    }
    assert_eq!(f.owner.retention_usage(), funding);
    f.owner
        .retire_source_read_operations_v1(Some(true), Some(true), Some(true), &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .configure_source_read_operations_v1(operation_limits(), &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
    assert_eq!(
        f.owner.admit_source_read_v1(
            f.port.source_read_target_v1(b"later").unwrap(),
            ReadStrength::CapturedRead,
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
    f.owner
        .retire_source_reader_v1(Some(true), Some(true), Some(true), &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.retention_usage().reserved, 0);
}
#[test]
fn operation_diagnostics_are_bounded_optional_and_never_contain_origins_or_scope() {
    for capacity in [None, Some(0), Some(128)] {
        let (mut f, mut c) = configured();
        c.purpose = "PRIVATE-PURPOSE".into();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let target = f.port.source_read_target_v1(b"PRIVATE-KEY").unwrap();
        let req = request(&target, b"PRIVATE-ORIGIN");
        let id = f
            .owner
            .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder)
            .unwrap()
            .task;
        assert_eq!(
            f.owner
                .request_owned_source_read_v1(&req, &facts(&c), &mut f.recorder)
                .unwrap()
                .task,
            id
        );
        f.owner.stop_source_read_v1(&id, &mut f.recorder).unwrap();
        f.owner.retire_source_read_v1(&id, &mut f.recorder).unwrap();
        let mut events = String::new();
        while let Some(event) = f.recorder.pop_front() {
            events.push_str(&format!("{event:?}"));
        }
        assert!(!events.contains("PRIVATE"));
        if capacity == Some(128) {
            assert!(
                events.contains("SourceReadOperationAdmission")
                    && events.contains("SourceReadOperationRecovery")
                    && events.contains("SourceReadOperationRetirement")
            );
        }
    }
}
