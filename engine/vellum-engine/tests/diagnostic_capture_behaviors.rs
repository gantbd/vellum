use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::Recorder;
use vellum_engine::source::Limits;
#[path = "support/discovered_context_fixture.rs"]
mod discovered_context;
#[path = "diagnostic_capture_behaviors/drain.rs"]
mod drain;
#[path = "support/selection_discovery_fixture.rs"]
mod selection_discovery;
fn scope() -> DiagnosticScopeV1 {
    DiagnosticScopeV1 {
        context: SettingsExecutionContextV1 {
            authority: "host".into(),
            engine: "engine".into(),
            recipient: "owner".into(),
            purpose: "diagnostics".into(),
            destination: "local".into(),
            profile: BOUNDED_OPTIONAL_CAPTURE_PROFILE_V1.into(),
            continuity: "session".into(),
        },
        policy: "p1".into(),
    }
}
fn limits() -> DiagnosticCaptureLimitsV1 {
    DiagnosticCaptureLimitsV1 {
        max_records: 1,
        max_queue_bytes: 256,
        max_preparations: 2,
        max_prepared_issued: 4,
        max_generations: 2,
        max_record_bytes: 256,
        max_context_bytes: 2048,
        max_reply_bytes: 4096,
        max_health_reply_bytes: 4096,
        max_drain_records: 2,
        max_counter: 10,
        max_retention_steps: 3,
        max_held_bytes: 65536,
    }
}
fn control(s: &DiagnosticScopeV1) -> DiagnosticControlV1<'_> {
    DiagnosticControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
        binding_ready: Some(true),
        outside_publication_guard: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
        health_allowed: Some(true),
        reply_ready: Some(true),
    }
}
fn facts(s: &DiagnosticScopeV1) -> TrustedDiagnosticFactsV1<'_> {
    TrustedDiagnosticFactsV1 {
        scope: s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        capture_allowed: Some(true),
        capture_ready: Some(true),
        release_allowed: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        reply_ready: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
        outside_publication_guard: Some(true),
    }
}
fn ready(
    l: DiagnosticCaptureLimitsV1,
) -> (
    SourceCoordinator,
    Recorder,
    DiagnosticScopeV1,
    DiagnosticGenerationV1,
) {
    ready_with_budget(l, 0)
}
fn ready_with_budget(
    l: DiagnosticCaptureLimitsV1,
    additional: usize,
) -> (
    SourceCoordinator,
    Recorder,
    DiagnosticScopeV1,
    DiagnosticGenerationV1,
) {
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 2,
        max_participants: 2,
        max_staged_bytes: 1024,
        source: Limits {
            max_source_bytes: 1024,
            max_edits: 8,
            max_edit_payload_bytes: 1024,
        },
    });
    let mut r = Recorder::disabled();
    o.configure_retention_budget(
        l.required_hold_bytes()
            .unwrap()
            .checked_add(additional)
            .unwrap(),
        &mut r,
    )
    .unwrap();
    let s = scope();
    let generation = o
        .configure_diagnostic_capture_v1(&s, l, &control(&s), &mut r)
        .unwrap();
    (o, r, s, generation)
}
fn marker(milestone: DiagnosticMilestoneV1) -> DiagnosticMarkerV1 {
    DiagnosticMarkerV1 {
        site: DiagnosticSiteV1::Ins01,
        milestone,
    }
}
#[test]
fn prepared_capture_is_single_use_drop_newest_and_drain_is_staged_before_consumption() {
    let (mut o, mut r, s, generation) = ready(limits());
    let source = o.create(b"prose", &mut r).unwrap();
    let native_generation = o.generation();
    let first = marker(DiagnosticMilestoneV1::Offered);
    let p = o
        .prepare_diagnostic_marker_v1(&s, first, &facts(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.capture_diagnostic_marker_v1(&s, &p, &facts(&s), &mut r),
        Ok(DiagnosticCaptureDecisionV1::Captured)
    );
    assert_eq!(
        o.capture_diagnostic_marker_v1(&s, &p, &facts(&s), &mut r),
        Err(CoordinatorError::Unavailable)
    );
    let p = o
        .prepare_diagnostic_marker_v1(
            &s,
            marker(DiagnosticMilestoneV1::Completed),
            &facts(&s),
            &mut r,
        )
        .unwrap();
    assert_eq!(
        o.capture_diagnostic_marker_v1(&s, &p, &facts(&s), &mut r),
        Ok(DiagnosticCaptureDecisionV1::Dropped)
    );
    let mut unavailable = facts(&s);
    unavailable.reply_ready = None;
    assert!(
        o.drain_diagnostic_markers_v1(&s, &generation, 1, &unavailable, &mut r)
            .is_err()
    );
    let health = o.read_diagnostic_health_v1(&control(&s)).unwrap();
    assert_eq!(health.queued_records, 1);
    assert_eq!(health.counters.offered.value, 2);
    assert_eq!(health.counters.dropped.value, 1);
    assert_eq!(health.counters.drained.value, 0);
    let batch = o
        .drain_diagnostic_markers_v1(&s, &generation, 1, &facts(&s), &mut r)
        .unwrap();
    assert_eq!(batch.records, [first]);
    assert!(!batch.remaining);
    assert_eq!(o.generation(), native_generation);
    assert_eq!(
        o.read(&source.document, &mut r).unwrap().source.text(),
        "prose"
    );
}
#[test]
fn known_binding_blocks_retirement_and_reset_never_revives_old_tokens_or_issuance() {
    let mut l = limits();
    l.max_prepared_issued = 1;
    let (mut o, mut r, s, old) = ready(l);
    let token = o
        .prepare_diagnostic_marker_v1(
            &s,
            marker(DiagnosticMilestoneV1::Admitted),
            &facts(&s),
            &mut r,
        )
        .unwrap();
    let binding = o.bind_diagnostic_drain_v1(&control(&s), &mut r).unwrap();
    o.stop_diagnostic_capture_v1(&s, false, &control(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.retire_diagnostic_capture_v1(&control(&s), &mut r),
        Err(CoordinatorError::Unavailable)
    );
    let new = o.reset_diagnostic_capture_v1(&control(&s), &mut r).unwrap();
    assert_ne!(new, old);
    assert_eq!(
        o.capture_diagnostic_marker_v1(&s, &token, &facts(&s), &mut r),
        Err(CoordinatorError::Unavailable)
    );
    assert!(
        o.prepare_diagnostic_marker_v1(
            &s,
            marker(DiagnosticMilestoneV1::Completed),
            &facts(&s),
            &mut r
        )
        .is_err()
    );
    assert_eq!(
        o.read_diagnostic_health_v1(&control(&s))
            .unwrap()
            .counters
            .preparation_refused
            .value,
        1
    );
    o.stop_diagnostic_capture_v1(&s, false, &control(&s), &mut r)
        .unwrap();
    o.release_diagnostic_drain_binding_v1(&binding, &control(&s), &mut r)
        .unwrap();
    assert!(o.bind_diagnostic_drain_v1(&control(&s), &mut r).is_err());
    o.retire_diagnostic_capture_v1(&control(&s), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(
        o.configure_diagnostic_capture_v1(&s, l, &control(&s), &mut r)
            .is_err()
    );
}

#[test]
fn equal_label_foreign_owner_tokens_and_generations_are_not_authority() {
    let (mut a, mut r, s, ga) = ready(limits());
    let (mut b, mut rb, _, gb) = ready(limits());
    let pa = a
        .prepare_diagnostic_marker_v1(
            &s,
            marker(DiagnosticMilestoneV1::Offered),
            &facts(&s),
            &mut r,
        )
        .unwrap();
    let pb = b
        .prepare_diagnostic_marker_v1(
            &s,
            marker(DiagnosticMilestoneV1::Completed),
            &facts(&s),
            &mut rb,
        )
        .unwrap();
    assert_ne!(ga, gb);
    assert_eq!(
        a.capture_diagnostic_marker_v1(&s, &pb, &facts(&s), &mut r),
        Err(CoordinatorError::Unavailable)
    );
    a.capture_diagnostic_marker_v1(&s, &pa, &facts(&s), &mut r)
        .unwrap();
    assert!(
        a.drain_diagnostic_markers_v1(&s, &gb, 1, &facts(&s), &mut r)
            .is_err()
    );
    assert_eq!(
        a.read_diagnostic_health_v1(&control(&s))
            .unwrap()
            .queued_records,
        1
    );
}

#[test]
fn missing_authority_stops_only_matching_scope_and_protected_health_survives() {
    let (mut o, mut r, s, _) = ready(limits());
    let p = o
        .prepare_diagnostic_marker_v1(
            &s,
            marker(DiagnosticMilestoneV1::Offered),
            &facts(&s),
            &mut r,
        )
        .unwrap();
    o.capture_diagnostic_marker_v1(&s, &p, &facts(&s), &mut r)
        .unwrap();
    let mut foreign = s.clone();
    foreign.context.recipient.push_str("-other");
    let mut lost = facts(&foreign);
    lost.authorized = Some(false);
    assert!(
        o.capture_diagnostic_marker_v1(&foreign, &p, &lost, &mut r)
            .is_err()
    );
    assert_eq!(
        o.read_diagnostic_health_v1(&control(&s)).unwrap().mode,
        DiagnosticModeV1::Enabled
    );
    let mut unavailable = facts(&s);
    unavailable.capture_allowed = None;
    assert!(
        o.prepare_diagnostic_marker_v1(
            &s,
            marker(DiagnosticMilestoneV1::Admitted),
            &unavailable,
            &mut r
        )
        .is_err()
    );
    assert_eq!(
        o.read_diagnostic_health_v1(&control(&s))
            .unwrap()
            .queued_records,
        1
    );
    let mut lost = facts(&s);
    lost.authorized = None;
    assert!(
        o.capture_diagnostic_marker_v1(&s, &p, &lost, &mut r)
            .is_err()
    );
    let h = o.read_diagnostic_health_v1(&control(&s)).unwrap();
    assert_eq!(h.mode, DiagnosticModeV1::Stopped);
    assert_eq!(h.queued_records, 0);
    assert_eq!(h.counters.discarded.value, 1);
    assert!(
        o.prepare_diagnostic_marker_v1(
            &s,
            marker(DiagnosticMilestoneV1::Admitted),
            &facts(&s),
            &mut r
        )
        .is_err()
    );
}

#[test]
fn counters_have_exact_saturation_and_logical_retention_does_not_invent_clock_truth() {
    for maximum in [0, 1] {
        let mut l = limits();
        l.max_counter = maximum;
        let (mut o, mut r, s, _) = ready(l);
        for _ in 0..2 {
            let p = o
                .prepare_diagnostic_marker_v1(
                    &s,
                    marker(DiagnosticMilestoneV1::Completed),
                    &facts(&s),
                    &mut r,
                )
                .unwrap();
            o.capture_diagnostic_marker_v1(&s, &p, &facts(&s), &mut r)
                .unwrap();
        }
        let h = o.read_diagnostic_health_v1(&control(&s)).unwrap();
        assert_eq!(h.counters.offered.value, maximum);
        assert!(!h.counters.offered.exact);
        assert_eq!(h.counters.captured.value, maximum);
        assert_eq!(h.counters.captured.exact, maximum == 1);
        o.advance_diagnostic_retention_v1(2, Some(true), &control(&s), &mut r)
            .unwrap();
        assert_eq!(
            o.read_diagnostic_health_v1(&control(&s))
                .unwrap()
                .queued_records,
            1
        );
        o.advance_diagnostic_retention_v1(1, Some(true), &control(&s), &mut r)
            .unwrap();
        let h = o.read_diagnostic_health_v1(&control(&s)).unwrap();
        assert_eq!(h.queued_records, 0);
        assert_eq!(h.clock, DiagnosticClockV1::Valid);
        o.advance_diagnostic_retention_v1(1, None, &control(&s), &mut r)
            .unwrap();
        let h = o.read_diagnostic_health_v1(&control(&s)).unwrap();
        assert_eq!(h.clock, DiagnosticClockV1::Invalid);
        assert_eq!(h.mode, DiagnosticModeV1::Stopped);
        assert_eq!(h.counters.discarded.exact, maximum == 1);
    }
}

#[test]
fn decoded_marker_vocabulary_is_closed_and_size_units_match_the_marker_product() {
    use DiagnosticMilestoneV1::*;
    use DiagnosticSiteV1::*;
    use vellum_engine::capture::InertValueV1 as D;
    let mut l = limits();
    l.max_prepared_issued = 100;
    let (mut o, mut r, s, _) = ready(l);
    for site in [
        Ins01, Ins02, Ins03, Ins04, Ins05, Ins06, Ins07, Ins08, Ins09, Ins10, Ins11, Ins12, Ins13,
    ] {
        for milestone in [Offered, Admitted, Completed, Rejected, Cancelled, Discarded] {
            let marker = DiagnosticMarkerV1 { site, milestone };
            let serialized = format!(
                "{{\"schema\":\"boundary-marker/1\",\"site\":\"{}\",\"milestone\":\"{}\"}}",
                site.code(),
                milestone.code()
            );
            assert_eq!(marker.canonical_record_bytes(), serialized.len());
            let fields = [
                ("schema", D::Text("boundary-marker/1")),
                ("site", D::Text(site.code())),
                ("milestone", D::Text(milestone.code())),
            ];
            let p = o
                .prepare_decoded_diagnostic_marker_v1(&s, &D::Object(&fields), &facts(&s), &mut r)
                .unwrap();
            o.retire_diagnostic_preparation_v1(&p, &control(&s), &mut r)
                .unwrap();
        }
    }
    let fields = [
        ("schema", D::Text("boundary-marker/1")),
        ("site", D::Text("INS-01")),
        ("milestone", D::Text("offered")),
        ("content", D::Text("never retained")),
    ];
    assert!(
        o.prepare_decoded_diagnostic_marker_v1(&s, &D::Object(&fields), &facts(&s), &mut r)
            .is_err()
    );
    let h = o.read_diagnostic_health_v1(&control(&s)).unwrap();
    assert_eq!(h.counters.invalid.value, 1);
    assert_eq!(h.counters.offered.value, 0);
}

#[test]
fn optional_recording_modes_do_not_change_data_and_token_capture_never_reads_a_clock() {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for mode in 0..4 {
        let (mut o, _, s, g) = ready(limits());
        let mut r = match mode {
            0 => Recorder::disabled(),
            1 => Recorder::try_with_capacity(16).unwrap(),
            2 => Recorder::try_with_capacity(16)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
            _ => Recorder::try_with_capacity(0)
                .unwrap()
                .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1),
        };
        let p = o
            .prepare_diagnostic_marker_v1(
                &s,
                marker(DiagnosticMilestoneV1::Completed),
                &facts(&s),
                &mut r,
            )
            .unwrap();
        o.capture_diagnostic_marker_v1(&s, &p, &facts(&s), &mut r)
            .unwrap();
        let recorded = r.queued();
        o.read_diagnostic_health_v1(&control(&s)).unwrap();
        assert_eq!(r.queued(), recorded);
        let reply = o
            .drain_diagnostic_markers_v1(&s, &g, 1, &facts(&s), &mut r)
            .unwrap();
        assert_eq!(reply.records, [marker(DiagnosticMilestoneV1::Completed)]);
        while let Some(record) = r.pop_front() {
            assert_eq!(
                record.service_time.is_some(),
                mode == 2 && record.boundary != Boundary::DiagnosticMarkerCapture
            );
        }
    }
}

#[test]
fn whole_reply_capacity_and_canonical_queue_byte_pressure_are_independent() {
    use std::mem::size_of;
    for byte_pressure in [false, true] {
        let m = marker(DiagnosticMilestoneV1::Completed);
        let mut l = limits();
        l.max_records = 2;
        l.max_reply_bytes = size_of::<DiagnosticBatchV1>() + size_of::<DiagnosticMarkerV1>();
        if byte_pressure {
            l.max_queue_bytes = m.canonical_record_bytes();
        }
        let (mut o, mut r, s, generation) = ready(l);
        for _ in 0..2 {
            let p = o
                .prepare_diagnostic_marker_v1(&s, m, &facts(&s), &mut r)
                .unwrap();
            o.capture_diagnostic_marker_v1(&s, &p, &facts(&s), &mut r)
                .unwrap();
        }
        let h = o.read_diagnostic_health_v1(&control(&s)).unwrap();
        assert_eq!(h.queued_records, if byte_pressure { 1 } else { 2 });
        if byte_pressure {
            assert_eq!(h.counters.dropped.value, 1);
            assert_eq!(h.queued_canonical_bytes, m.canonical_record_bytes());
        } else {
            assert_eq!(
                o.drain_diagnostic_markers_v1(&s, &generation, 2, &facts(&s), &mut r),
                Err(CoordinatorError::Limit)
            );
            let after = o.read_diagnostic_health_v1(&control(&s)).unwrap();
            assert_eq!(after.queued_records, 2);
            assert_eq!(after.counters.drained.value, 0);
        }
        let batch = o
            .drain_diagnostic_markers_v1(&s, &generation, 1, &facts(&s), &mut r)
            .unwrap();
        assert_eq!(batch.records, [m]);
        assert_eq!(batch.remaining, !byte_pressure);
    }
}

#[test]
fn zero_optional_capacity_preserves_protected_health_and_empty_drain_is_successful() {
    for slots in [false, true] {
        let mut l = limits();
        if slots {
            l.max_records = 0;
        } else {
            l.max_queue_bytes = 0;
        }
        let (mut o, mut r, s, g) = ready(l);
        let p = o
            .prepare_diagnostic_marker_v1(
                &s,
                marker(DiagnosticMilestoneV1::Offered),
                &facts(&s),
                &mut r,
            )
            .unwrap();
        assert_eq!(
            o.capture_diagnostic_marker_v1(&s, &p, &facts(&s), &mut r),
            Ok(DiagnosticCaptureDecisionV1::Dropped)
        );
        let h = o.read_diagnostic_health_v1(&control(&s)).unwrap();
        assert_eq!(h.queued_records, 0);
        assert_eq!(h.queued_canonical_bytes, 0);
        assert_eq!(h.counters.dropped.value, 1);
        let batch = o
            .drain_diagnostic_markers_v1(&s, &g, 1, &facts(&s), &mut r)
            .unwrap();
        assert!(batch.records.is_empty());
        assert!(!batch.remaining);
        o.stop_diagnostic_capture_v1(&s, true, &control(&s), &mut r)
            .unwrap();
        assert_eq!(
            o.read_diagnostic_health_v1(&control(&s)).unwrap().mode,
            DiagnosticModeV1::Disabled
        );
    }
}
