use super::*;
#[test]
fn document_resource_pin_census_keeps_read_and_parent_until_real_retirement() {
    let mut f = setup();
    let c = context();
    let d = f.doc.clone();
    let target = f.port.annotation_read_target(b"comments").unwrap();
    let annotation = f.port.source_binding_target(b"comments").unwrap();
    let source = f.port.source_binding_target(b"source").unwrap();
    let initial = f
        .owner
        .document_resource_pins_v1(&d.document, &mut f.recorder)
        .unwrap();
    let q = read_request(&d, &target, &annotation, &source, b"pins");
    let pair = f.pair();
    let task = f
        .owner
        .request_owned_annotation_read_v1(&q, &facts(&c), pair, &mut f.recorder)
        .unwrap()
        .task;
    let pins = f
        .owner
        .document_resource_pins_v1(&d.document, &mut f.recorder)
        .unwrap();
    assert_eq!(
        (pins.annotation_reads, pins.annotation_read_parents),
        (1, 1)
    );
    f.owner
        .stop_annotation_read(&task, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .document_resource_pins_v1(&d.document, &mut f.recorder)
            .unwrap(),
        pins
    );
    f.owner
        .retire_annotation_read(&task, &mut f.recorder)
        .unwrap();
    assert_eq!(
        f.owner
            .document_resource_pins_v1(&d.document, &mut f.recorder)
            .unwrap(),
        initial
    );
}
#[path = "read_facade.rs"]
mod facade;
#[test]
fn operation_service_timing_is_optional_and_does_not_change_recovery_or_discharge() {
    use vellum_engine::instrumentation::{Disposition, NativeTimingProfileV1};
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
            // Idempotent configuration must have the same timing contract as initial configuration.
            f.owner
                .configure_annotation_read_operations_v1(read_limits(), &mut f.recorder)
                .unwrap();
            let d = f.doc.clone();
            let target = f.port.annotation_read_target(b"comments").unwrap();
            let annotation = f.port.source_binding_target(b"comments").unwrap();
            let source = f.port.source_binding_target(b"source").unwrap();
            let q = read_request(&d, &target, &annotation, &source, b"PRIVATE-OPERATION");
            let mut denied = facts(&c);
            denied.authorized = None;
            let pair = f.pair();
            assert_eq!(
                f.owner
                    .request_owned_annotation_read_v1(&q, &denied, pair, &mut f.recorder),
                Err(ResourceError::Unavailable)
            );
            let pair = f.pair();
            let first = f
                .owner
                .request_owned_annotation_read_v1(&q, &facts(&c), pair, &mut f.recorder)
                .unwrap();
            let funding = f.owner.retention_usage();
            let generation = f.owner.generation();
            let replay = f
                .owner
                .request_owned_annotation_read_v1(&q, &facts(&c), None, &mut f.recorder)
                .unwrap();
            assert_eq!(replay.task, first.task);
            assert_eq!(
                replay.status,
                AnnotationReadOperationAdmissionStatusV1::ObserveExisting
            );
            assert_eq!(f.owner.retention_usage(), funding);
            assert_eq!(f.owner.generation(), generation);
            f.owner.stop_annotation_read_admission_v1(&mut f.recorder);
            assert_eq!(
                f.owner.retire_annotation_read_operations_v1(
                    Some(true),
                    Some(true),
                    Some(true),
                    &mut f.recorder
                ),
                Err(ResourceError::Retained)
            );
            f.owner
                .stop_annotation_read(&first.task, &mut f.recorder)
                .unwrap();
            f.owner
                .retire_annotation_read(&first.task, &mut f.recorder)
                .unwrap();
            f.owner
                .retire_annotation_read_operations_v1(
                    Some(true),
                    Some(true),
                    Some(true),
                    &mut f.recorder,
                )
                .unwrap();
            assert_eq!(f.owner.retention_usage().reserved, 0);
            let events: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
            assert!(!format!("{events:?}").contains("PRIVATE"));
            if capacity == Some(256) {
                for boundary in [
                    Boundary::AnnotationReadOperationConfiguration,
                    Boundary::AnnotationReadOperationAdmission,
                    Boundary::AnnotationReadOperationRecovery,
                    Boundary::AnnotationReadOperationRetirement,
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
                        .any(|e| e.boundary == Boundary::AnnotationReadOperationAdmission
                            && e.disposition == Disposition::Refused)
                );
                assert!(events.iter().any(|e| e.boundary
                    == Boundary::AnnotationReadOperationRetirement
                    && e.disposition == Disposition::Refused));
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
fn read_limits() -> AnnotationReadOperationLimitsV1 {
    AnnotationReadOperationLimitsV1 {
        max_capture_bytes: 8192,
        max_reply_bytes: 1024,
        max_label_bytes: 64,
        max_store_bytes: 1048576,
    }
}
pub(super) fn setup() -> Fixture {
    let mut f = Fixture::new();
    f.owner
        .configure_retention_budget(2097152, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_annotation_read_operations_v1(read_limits(), &mut f.recorder)
        .unwrap();
    f
}
pub(super) fn facts(c: &SettingsExecutionContextV1) -> TrustedAnnotationReadOperationFactsV1<'_> {
    TrustedAnnotationReadOperationFactsV1 {
        context: c,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        read_allowed: Some(true),
        annotation_readable: Some(true),
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
pub(super) fn read_request<'a>(
    d: &'a DocumentVersion,
    target: &'a AnnotationReadTarget,
    annotation: &'a SourceBindingTargetV1,
    source: &'a SourceBindingTargetV1,
    operation: &'a [u8],
) -> OwnedAnnotationReadRequestV1<'a> {
    OwnedAnnotationReadRequestV1 {
        origin: AnnotationReadOperationOriginV1 {
            operation,
            effect: b"read",
            attempt: b"one",
        },
        source: d,
        expected_set: 0,
        annotation_binding: ExpectedAnnotationReadBindingV1 {
            target: annotation,
            generation: 1,
        },
        source_binding: Some(ExpectedAnnotationReadBindingV1 {
            target: source,
            generation: 0,
        }),
        target,
        strength: ReadStrength::CapturedRead,
    }
}
#[test]
fn annotation_load_recovery_returns_one_actual_ready_task_without_submission_or_source_disclosure()
{
    let mut f = setup();
    let c = context();
    let d = f.doc.clone();
    let target = f.port.annotation_read_target(b"comments").unwrap();
    let annotation = f.port.source_binding_target(b"comments").unwrap();
    let source = f.port.source_binding_target(b"source").unwrap();
    let pair = f.pair();
    let req = read_request(&d, &target, &annotation, &source, b"load");
    let generation = f.owner.generation();
    let first = f
        .owner
        .request_owned_annotation_read_v1(&req, &facts(&c), pair, &mut f.recorder)
        .unwrap();
    assert_eq!(
        first.status,
        AnnotationReadOperationAdmissionStatusV1::Admitted
    );
    let before = f.owner.resource_usage();
    let funding = f.owner.retention_usage();
    let again = f
        .owner
        .request_owned_annotation_read_v1(&req, &facts(&c), None, &mut f.recorder)
        .unwrap();
    assert_eq!(
        again.status,
        AnnotationReadOperationAdmissionStatusV1::ObserveExisting
    );
    assert_eq!(again.task, first.task);
    assert_eq!(
        f.owner
            .annotation_read_status(&first.task, &mut f.recorder)
            .unwrap()
            .phase,
        ReadPhase::Ready
    );
    assert_eq!(f.owner.resource_usage(), before);
    assert_eq!(f.owner.retention_usage(), funding);
    assert_eq!(f.owner.generation(), generation);
}

#[test]
fn changed_load_intent_and_original_scope_refuse_without_another_native_task() {
    let mut f = setup();
    let c = context();
    let d = f.doc.clone();
    let target = f.port.annotation_read_target(b"comments").unwrap();
    let annotation = f.port.source_binding_target(b"comments").unwrap();
    let source = f.port.source_binding_target(b"source").unwrap();
    let pair = f.pair();
    let req = read_request(&d, &target, &annotation, &source, b"load");
    let id = f
        .owner
        .request_owned_annotation_read_v1(&req, &facts(&c), pair, &mut f.recorder)
        .unwrap()
        .task;
    let other_target = f.port.annotation_read_target(b"other").unwrap();
    let other_binding = f.port.source_binding_target(b"other").unwrap();
    let before = f.owner.resource_usage();
    let parent = f.owner.annotation_read_operation_usage_v1().unwrap();
    for field in 0..9 {
        let mut r = read_request(&d, &target, &annotation, &source, b"load");
        match field {
            0 => r.origin.effect = b"different",
            1 => r.origin.attempt = b"two",
            2 => r.expected_set += 1,
            3 => r.annotation_binding.generation += 1,
            4 => r.annotation_binding.target = &other_binding,
            5 => r.source_binding = None,
            6 => r.source_binding.as_mut().unwrap().generation += 1,
            7 => r.target = &other_target,
            _ => r.strength = ReadStrength::AtomicResourceSnapshot,
        }
        assert_eq!(
            f.owner
                .request_owned_annotation_read_v1(&r, &facts(&c), None, &mut f.recorder),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(f.owner.resource_usage(), before);
        assert_eq!(
            f.owner.annotation_read_operation_usage_v1().unwrap(),
            parent
        );
    }
    for field in 0..7 {
        let mut changed = c.clone();
        match field {
            0 => changed.authority.push('x'),
            1 => changed.engine.push('x'),
            2 => changed.recipient.push('x'),
            3 => changed.purpose.push('x'),
            4 => changed.destination.push('x'),
            5 => changed.profile.push('x'),
            _ => changed.continuity.push('x'),
        }
        assert_eq!(
            f.owner
                .request_owned_annotation_read_v1(&req, &facts(&changed), None, &mut f.recorder),
            Err(ResourceError::Unavailable)
        );
    }
    assert_eq!(
        f.owner
            .request_owned_annotation_read_v1(&req, &facts(&c), None, &mut f.recorder)
            .unwrap()
            .task,
        id
    );
}

#[test]
fn all_read_release_and_fresh_launch_facts_are_exact_and_independent() {
    for denied in [None, Some(false)] {
        for field in 0..15 {
            let mut f = setup();
            let c = context();
            let d = f.doc.clone();
            let target = f.port.annotation_read_target(b"comments").unwrap();
            let annotation = f.port.source_binding_target(b"comments").unwrap();
            let source = f.port.source_binding_target(b"source").unwrap();
            let pair = f.pair();
            let req = read_request(&d, &target, &annotation, &source, b"load");
            let mut g = facts(&c);
            match field {
                0 => g.authorized = denied,
                1 => g.scope = denied,
                2 => g.continuous = denied,
                3 => g.live = denied,
                4 => g.not_stopped = denied,
                5 => g.read_allowed = denied,
                6 => g.annotation_readable = denied,
                7 => g.retained = denied,
                8 => g.reply_ready = denied,
                9 => g.transfer_ready = denied,
                10 => g.resources_ready = denied,
                11 => g.identity_allowed = denied,
                12 => g.binding_coverage_complete = denied,
                13 => g.provider_available = denied,
                _ => g.launch_allowed = denied,
            }
            let before = f.owner.resource_usage();
            let parent = f.owner.annotation_read_operation_usage_v1().unwrap();
            let funding = f.owner.retention_usage();
            assert_eq!(
                f.owner
                    .request_owned_annotation_read_v1(&req, &g, pair, &mut f.recorder),
                Err(ResourceError::Unavailable)
            );
            assert_eq!(f.owner.resource_usage(), before);
            assert_eq!(
                f.owner.annotation_read_operation_usage_v1().unwrap(),
                parent
            );
            assert_eq!(f.owner.retention_usage(), funding);
            let pair = f.pair();
            let id = f
                .owner
                .request_owned_annotation_read_v1(&req, &facts(&c), pair, &mut f.recorder)
                .unwrap()
                .task;
            if field < 11 {
                assert_eq!(
                    f.owner
                        .request_owned_annotation_read_v1(&req, &g, None, &mut f.recorder),
                    Err(ResourceError::Unavailable)
                );
            } else {
                assert_eq!(
                    f.owner
                        .request_owned_annotation_read_v1(&req, &g, None, &mut f.recorder)
                        .unwrap()
                        .task,
                    id
                );
            }
        }
    }
}

#[test]
fn old_load_metadata_survives_edits_and_admission_fencing_without_becoming_applicable_again() {
    let mut f = setup();
    let c = context();
    let d = f.doc.clone();
    let target = f.port.annotation_read_target(b"comments").unwrap();
    let annotation = f.port.source_binding_target(b"comments").unwrap();
    let source = f.port.source_binding_target(b"source").unwrap();
    let pair = f.pair();
    let req = read_request(&d, &target, &annotation, &source, b"load");
    let id = f
        .owner
        .request_owned_annotation_read_v1(&req, &facts(&c), pair, &mut f.recorder)
        .unwrap()
        .task;
    let edit = f
        .owner
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 4,
                    expected: "text".into(),
                    inserted: "later".into(),
                }],
            }],
            &[],
            &mut f.recorder,
        )
        .unwrap();
    f.owner.commit(edit, &mut f.recorder).unwrap();
    f.owner.stop_annotation_read_admission_v1(&mut f.recorder);
    let mut g = facts(&c);
    g.provider_available = None;
    g.launch_allowed = None;
    g.identity_allowed = None;
    g.binding_coverage_complete = None;
    let before = f.owner.resource_usage();
    assert_eq!(
        f.owner
            .request_owned_annotation_read_v1(&req, &g, None, &mut f.recorder)
            .unwrap()
            .task,
        id
    );
    assert_eq!(f.owner.resource_usage(), before);
    let now = f.owner.read(&d.document, &mut f.recorder).unwrap().version;
    let changed = read_request(&now, &target, &annotation, &source, b"load");
    assert_eq!(
        f.owner
            .request_owned_annotation_read_v1(&changed, &facts(&c), None, &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
    assert!(matches!(
        f.owner.submit_annotation_read(&id, None, &mut f.recorder),
        Err(ResourceError::Stale)
    ));
    assert_eq!(
        f.owner.request_owned_annotation_read_v1(
            &read_request(&now, &target, &annotation, &source, b"new-load"),
            &facts(&c),
            None,
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
}

#[test]
fn read_eof_stop_quiescence_and_capture_drain_are_separate_from_parent_retirement() {
    let mut f = setup();
    let c = context();
    let d = f.doc.clone();
    let target = f.port.annotation_read_target(b"comments").unwrap();
    let annotation = f.port.source_binding_target(b"comments").unwrap();
    let source = f.port.source_binding_target(b"source").unwrap();
    let pair = f.pair();
    let req = read_request(&d, &target, &annotation, &source, b"load");
    let id = f
        .owner
        .request_owned_annotation_read_v1(&req, &facts(&c), pair, &mut f.recorder)
        .unwrap()
        .task;
    let pair = f.pair();
    let dispatch = f
        .owner
        .submit_annotation_read(&id, pair, &mut f.recorder)
        .unwrap();
    for e in [
        f.port.annotation_read_start(&dispatch, b"v1").unwrap(),
        f.port
            .annotation_read_chunk(&dispatch, 0, 0, b"{}")
            .unwrap(),
        f.port.annotation_read_end(&dispatch, b"v1", 1, 2).unwrap(),
    ] {
        f.owner.record_annotation_read(e, &mut f.recorder).unwrap();
    }
    let pair = f.pair();
    let capture = f
        .owner
        .annotation_read_capture(&id, pair, &mut f.recorder)
        .unwrap();
    assert_eq!(capture.bytes(), b"{}");
    f.owner.stop_annotation_read(&id, &mut f.recorder).unwrap();
    assert_eq!(
        f.owner.retire_annotation_read(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    f.owner
        .record_annotation_read(
            f.port.annotation_read_quiescence(&dispatch).unwrap(),
            &mut f.recorder,
        )
        .unwrap();
    assert_eq!(
        f.owner.retire_annotation_read(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(
        f.owner
            .request_owned_annotation_read_v1(&req, &facts(&c), None, &mut f.recorder)
            .unwrap()
            .task,
        id
    );
    let funding = f.owner.retention_usage();
    drop(dispatch);
    drop(capture);
    f.owner
        .retire_annotation_read(&id, &mut f.recorder)
        .unwrap();
    let usage = f.owner.annotation_read_operation_usage_v1().unwrap();
    assert_eq!((usage.owned, usage.retired, usage.spent_origins), (0, 1, 1));
    assert_eq!(f.owner.retention_usage(), funding);
    assert_eq!(f.owner.resource_usage().held_read_bytes, 0);
    assert_eq!(
        f.owner
            .request_owned_annotation_read_v1(&req, &facts(&c), None, &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
}

#[test]
fn reader_sponsorship_needs_global_admission_stop_and_actual_discharge_before_returning_credit() {
    let mut f = setup();
    let c = context();
    let d = f.doc.clone();
    let target = f.port.annotation_read_target(b"comments").unwrap();
    let annotation = f.port.source_binding_target(b"comments").unwrap();
    let source = f.port.source_binding_target(b"source").unwrap();
    let pair = f.pair();
    let req = read_request(&d, &target, &annotation, &source, b"load");
    let id = f
        .owner
        .request_owned_annotation_read_v1(&req, &facts(&c), pair, &mut f.recorder)
        .unwrap()
        .task;
    assert_eq!(
        f.owner.retire_annotation_read_operations_v1(
            Some(true),
            Some(true),
            Some(true),
            &mut f.recorder
        ),
        Err(ResourceError::Retained)
    );
    f.owner.stop_annotation_read_admission_v1(&mut f.recorder);
    f.owner.stop_annotation_read(&id, &mut f.recorder).unwrap();
    f.owner
        .retire_annotation_read(&id, &mut f.recorder)
        .unwrap();
    let funding = f.owner.retention_usage();
    assert_eq!(
        f.owner
            .retire_annotation_read_operations_v1(Some(true), None, Some(true), &mut f.recorder),
        Err(ResourceError::Retained)
    );
    assert_eq!(f.owner.retention_usage(), funding);
    f.owner
        .retire_annotation_read_operations_v1(Some(true), Some(true), Some(true), &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.retention_usage().reserved, 0);
    assert_eq!(
        f.owner
            .configure_annotation_read_operations_v1(read_limits(), &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
    assert_eq!(
        f.owner.admit_annotation_read(
            &d,
            0,
            f.port.annotation_read_target(b"comments").unwrap(),
            ReadStrength::CapturedRead,
            None,
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
}

#[test]
fn reader_registration_is_not_the_writer_binding_and_cannot_be_substituted_on_retry() {
    let mut f = setup();
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
    let replacement = f
        .owner
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            Some(&f.port.identity_domain()),
            &mut f.recorder,
        )
        .unwrap();
    let c = context();
    let d = f.doc.clone();
    let target = reader.annotation_read_target(b"comments").unwrap();
    let changed = replacement.annotation_read_target(b"comments").unwrap();
    let annotation = f.port.source_binding_target(b"comments").unwrap();
    let source = f.port.source_binding_target(b"source").unwrap();
    let pair = f.pair();
    let req = read_request(&d, &target, &annotation, &source, b"load");
    let id = f
        .owner
        .request_owned_annotation_read_v1(&req, &facts(&c), pair, &mut f.recorder)
        .unwrap()
        .task;
    assert_eq!(
        f.owner.request_owned_annotation_read_v1(
            &read_request(&d, &changed, &annotation, &source, b"load"),
            &facts(&c),
            None,
            &mut f.recorder
        ),
        Err(ResourceError::Unavailable)
    );
    let pair = f.pair();
    let dispatch = f
        .owner
        .submit_annotation_read(&id, pair, &mut f.recorder)
        .unwrap();
    assert!(matches!(
        f.port.annotation_read_start(&dispatch, b"v0"),
        Err(ResourceError::Unavailable)
    ));
    f.owner
        .record_annotation_read(
            reader.annotation_read_start(&dispatch, b"v0").unwrap(),
            &mut f.recorder,
        )
        .unwrap();
}

#[test]
fn annotation_load_diagnostics_preserve_actual_recovery_and_tombstones_without_private_labels() {
    for capacity in [None, Some(0), Some(128)] {
        let mut f = setup();
        f.recorder = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let c = context();
        let d = f.doc.clone();
        let target = f.port.annotation_read_target(b"comments").unwrap();
        let annotation = f.port.source_binding_target(b"comments").unwrap();
        let source = f.port.source_binding_target(b"source").unwrap();
        let pair = f.pair();
        let req = read_request(&d, &target, &annotation, &source, b"PRIVATE_READ");
        let id = f
            .owner
            .request_owned_annotation_read_v1(&req, &facts(&c), pair, &mut f.recorder)
            .unwrap()
            .task;
        assert_eq!(
            f.owner
                .request_owned_annotation_read_v1(&req, &facts(&c), None, &mut f.recorder)
                .unwrap()
                .task,
            id
        );
        f.owner.stop_annotation_read(&id, &mut f.recorder).unwrap();
        f.owner
            .retire_annotation_read(&id, &mut f.recorder)
            .unwrap();
        let records: Vec<_> = std::iter::from_fn(|| f.recorder.pop_front()).collect();
        assert!(!format!("{records:?}").contains("PRIVATE"));
        if capacity == Some(128) {
            for b in [
                Boundary::AnnotationReadOperationAdmission,
                Boundary::AnnotationReadOperationRecovery,
                Boundary::AnnotationReadOperationRetirement,
            ] {
                assert!(records.iter().any(|v| v.boundary == b));
            }
        } else {
            assert!(records.is_empty());
        }
        assert_eq!(
            f.owner
                .annotation_read_operation_usage_v1()
                .unwrap()
                .retired,
            1
        );
    }
}

#[test]
fn refused_child_reads_spend_no_parent_and_never_adopt_prior_raw_work() {
    let mut f = setup();
    let c = context();
    let d = f.doc.clone();
    let target = f.port.annotation_read_target(b"comments").unwrap();
    let annotation = f.port.source_binding_target(b"comments").unwrap();
    let source = f.port.source_binding_target(b"source").unwrap();
    let pair = f.pair();
    let raw = f
        .owner
        .admit_annotation_read(
            &d,
            0,
            f.port.annotation_read_target(b"comments").unwrap(),
            ReadStrength::CapturedRead,
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let before = f.owner.resource_usage();
    let parent = f.owner.annotation_read_operation_usage_v1().unwrap();
    for kind in 0..3 {
        let mut req = read_request(&d, &target, &annotation, &source, b"load");
        let expected = match kind {
            0 => {
                req.expected_set = 1;
                ResourceError::Stale
            }
            1 => {
                req.strength = ReadStrength::AtomicResourceSnapshot;
                ResourceError::Unsupported
            }
            _ => {
                req.annotation_binding.generation = 2;
                ResourceError::Stale
            }
        };
        let pair = f.pair();
        assert_eq!(
            f.owner
                .request_owned_annotation_read_v1(&req, &facts(&c), pair, &mut f.recorder),
            Err(expected)
        );
        assert_eq!(f.owner.resource_usage(), before);
        assert_eq!(
            f.owner.annotation_read_operation_usage_v1().unwrap(),
            parent
        );
    }
    let pair = f.pair();
    let id = f
        .owner
        .request_owned_annotation_read_v1(
            &read_request(&d, &target, &annotation, &source, b"load"),
            &facts(&c),
            pair,
            &mut f.recorder,
        )
        .unwrap()
        .task;
    assert_ne!(raw, id);
    assert_eq!(f.owner.resource_usage().issued_reads_lifetime, 2);
    assert_eq!(
        f.owner
            .annotation_read_operation_usage_v1()
            .unwrap()
            .issued_owned,
        1
    );
}
