use super::*;
use crate::coordinator::diagnostic_capture::tests::{control as qc, facts as qf};
fn limits() -> DiagnosticDrainLimitsV1 {
    DiagnosticDrainLimitsV1 {
        max_tasks: 3,
        max_issued: 8,
        max_context_bytes: 2048,
        max_binding_bytes: 4096,
        max_request_bytes: 512,
        max_batch_bytes: 4096,
        max_result_bytes: 8192,
        max_control_reply_bytes: 512,
        max_held_bytes: 131072,
    }
}
fn df(s: &DiagnosticScopeV1) -> TrustedDiagnosticDrainFactsV1<'_> {
    TrustedDiagnosticDrainFactsV1 {
        scope: s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        outside_publication_guard: Some(true),
        drain_allowed: Some(true),
        resources_ready: Some(true),
        bookkeeping_reserved: Some(true),
        completion_reserved: Some(true),
        queue_interpretation_current: Some(true),
        drain_interpretation_current: Some(true),
        scheduler_owned: Some(true),
        result_allowed: Some(true),
        queue_release_allowed: Some(true),
        retained: Some(true),
        transfer_ready: Some(true),
        reply_ready: Some(true),
    }
}
fn dc(s: &DiagnosticScopeV1) -> DiagnosticDrainControlV1<'_> {
    DiagnosticDrainControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        outside_publication_guard: Some(true),
        resources_ready: Some(true),
        preparation_reserved: Some(true),
        binding_ready: Some(true),
        queue_interpretation_current: Some(true),
        drain_interpretation_current: Some(true),
        control_allowed: Some(true),
        control_ready: Some(true),
        transfer_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn ready(
    headroom_short: bool,
) -> (
    SourceCoordinator,
    DiagnosticScopeV1,
    DiagnosticScopeV1,
    Recorder,
) {
    ready_extra(headroom_short, 0)
}
fn ready_extra(
    headroom_short: bool,
    additional: usize,
) -> (
    SourceCoordinator,
    DiagnosticScopeV1,
    DiagnosticScopeV1,
    Recorder,
) {
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 1,
        max_participants: 1,
        max_staged_bytes: 32,
        source: Limits {
            max_source_bytes: 32,
            max_edits: 1,
            max_edit_payload_bytes: 32,
        },
    });
    let mut context = crate::coordinator::shutdown::tests::context();
    context.profile = BOUNDED_OPTIONAL_CAPTURE_PROFILE_V1.into();
    let q = DiagnosticScopeV1 {
        context,
        policy: "p1".into(),
    };
    let mut g = q.clone();
    g.context.profile = OWNED_DIAGNOSTIC_DRAIN_PROFILE_V1.into();
    let l = DiagnosticCaptureLimitsV1 {
        max_records: 1,
        max_queue_bytes: 256,
        max_preparations: 1,
        max_prepared_issued: 8,
        max_generations: 2,
        max_record_bytes: 256,
        max_context_bytes: 2048,
        max_reply_bytes: 4096,
        max_health_reply_bytes: 4096,
        max_drain_records: 2,
        max_counter: 10,
        max_retention_steps: 3,
        max_held_bytes: 65536,
    };
    let mut r = Recorder::disabled();
    o.configure_retention_budget(
        l.required_hold_bytes().unwrap() + limits().required_hold_bytes().unwrap() + additional
            - usize::from(headroom_short),
        &mut r,
    )
    .unwrap();
    o.configure_diagnostic_capture_v1(&q, l, &qc(&q), &mut r)
        .unwrap();
    (o, q, g, r)
}
#[test]
fn provisional_frame_and_actual_parent_refusal_never_spend_the_binding() {
    for short in [false, true] {
        let (mut o, _, g, mut r) = ready(short);
        let before = o.retention_usage();
        let mut l = limits();
        if !short {
            l.max_batch_bytes = 4095;
        }
        assert_eq!(
            o.open_diagnostic_drain_v1(&g, l, &dc(&g), &mut r),
            Err(DiagnosticDrainErrorV1::Limit)
        );
        let p = o.diagnostic_pool().unwrap();
        assert!(!p.binding_spent);
        assert!(p.binding.is_none());
        assert!(p.gateway.is_none());
        assert_eq!(o.retention_usage(), before);
        if !short {
            o.open_diagnostic_drain_v1(&g, limits(), &dc(&g), &mut r)
                .unwrap();
        }
    }
}
#[test]
fn before_after_malformed_and_unwind_faults_settle_unknown_without_redrain_or_source_fencing() {
    for fault in [
        Fault::BeforeDrain,
        Fault::AfterDrain,
        Fault::WrongGeneration,
        Fault::UnwindAfterDrain,
    ] {
        let (mut o, q, g, mut r) = ready(false);
        o.open_diagnostic_drain_v1(&g, limits(), &dc(&g), &mut r)
            .unwrap();
        let mut tasks = Vec::new();
        for _ in 0..3 {
            let task = o.issue_diagnostic_drain_v1(&g, &df(&g), &mut r).unwrap();
            o.submit_diagnostic_drain_v1(&task.task, &task.generation, 1, &g, &df(&g), &mut r)
                .unwrap();
            tasks.push(task);
        }
        // An accepted empty drain is performed, and its historical fact must
        // survive a later fault in another task even when payloads are purged.
        o.run_diagnostic_drain_v1(&tasks[0].task, &g, &df(&g), &qf(&q), &mut r)
            .unwrap();
        let p = o
            .prepare_diagnostic_marker_v1(
                &q,
                DiagnosticMarkerV1 {
                    site: DiagnosticSiteV1::Ins01,
                    milestone: DiagnosticMilestoneV1::Completed,
                },
                &qf(&q),
                &mut r,
            )
            .unwrap();
        o.capture_diagnostic_marker_v1(&q, &p, &qf(&q), &mut r)
            .unwrap();
        o.diagnostic_gateway_mut().unwrap().fault = Some(fault);
        assert_eq!(
            o.run_diagnostic_drain_v1(&tasks[1].task, &g, &df(&g), &qf(&q), &mut r),
            Err(DiagnosticDrainErrorV1::Faulted)
        );
        assert!(!o.is_faulted());
        for (task, disposition) in tasks.iter().zip([
            DiagnosticDrainDispositionV1::Performed,
            DiagnosticDrainDispositionV1::Unknown,
            DiagnosticDrainDispositionV1::NotPerformed,
        ]) {
            assert_eq!(
                o.query_diagnostic_drain_control_v1(&task.task, &dc(&g), &mut r)
                    .unwrap()
                    .drain,
                Some(disposition)
            );
        }
        let h = o.read_diagnostic_health_v1(&qc(&q)).unwrap();
        assert_eq!(h.mode, DiagnosticModeV1::Enabled);
        assert_eq!(
            h.queued_records,
            usize::from(matches!(fault, Fault::BeforeDrain))
        );
        assert_eq!(
            o.run_diagnostic_drain_v1(&tasks[1].task, &g, &df(&g), &qf(&q), &mut r),
            Err(DiagnosticDrainErrorV1::Unavailable)
        );
        for task in tasks {
            o.retire_diagnostic_drain_task_v1(&task.task, &dc(&g), &mut r)
                .unwrap();
        }
        o.retire_diagnostic_drain_v1(&dc(&g), &mut r).unwrap();
        o.stop_diagnostic_capture_v1(&q, false, &qc(&q), &mut r)
            .unwrap();
        o.retire_diagnostic_capture_v1(&qc(&q), &mut r).unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
    }
}
#[test]
fn nested_bridge_and_retained_batches_have_actual_shutdown_funding_and_discharge() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read};
    let (mut o, q, g, mut r) = ready(false);
    let base = o.retention_usage().reserved;
    o.open_diagnostic_drain_v1(&g, limits(), &dc(&g), &mut r)
        .unwrap();
    let task = o.issue_diagnostic_drain_v1(&g, &df(&g), &mut r).unwrap();
    o.submit_diagnostic_drain_v1(&task.task, &task.generation, 1, &g, &df(&g), &mut r)
        .unwrap();
    o.run_diagnostic_drain_v1(&task.task, &g, &df(&g), &qf(&q), &mut r)
        .unwrap();
    let cut = inspect(&o, &mut r);
    let read = inspect_read(&o, &cut, &mut r);
    assert!(read.coverage.all_owner_obligations);
    assert_eq!(read.funding.unrepresented_parent_bytes, 0);
    let owner = read
        .owners
        .iter()
        .find(|r| r.owner.role() == ShutdownOwnerRoleV1::DiagnosticCapture)
        .unwrap();
    assert_eq!(owner.reserved_bytes, Some(base));
    assert_eq!(
        owner.additional_reserved_bytes,
        Some(limits().required_hold_bytes().unwrap())
    );
    o.stop_diagnostic_drain_v1(&dc(&g), &mut r).unwrap();
    assert_eq!(
        o.retire_diagnostic_drain_v1(&dc(&g), &mut r),
        Err(DiagnosticDrainErrorV1::Unavailable)
    );
    o.stop_diagnostic_capture_v1(&q, false, &qc(&q), &mut r)
        .unwrap();
    assert_eq!(
        o.retire_diagnostic_capture_v1(&qc(&q), &mut r),
        Err(CoordinatorError::Unavailable)
    );
    o.retire_diagnostic_drain_task_v1(&task.task, &dc(&g), &mut r)
        .unwrap();
    o.retire_diagnostic_drain_v1(&dc(&g), &mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, base);
    o.retire_diagnostic_capture_v1(&qc(&q), &mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}

#[test]
fn domain_fence_blocks_issue_and_first_admission_but_not_retained_service_replay_or_cleanup() {
    let extra = crate::coordinator::shutdown::tests::limits()
        .required_capture_bytes()
        .unwrap();
    let (mut o, q, g, mut r) = ready_extra(false, extra);
    o.open_diagnostic_drain_v1(&g, limits(), &dc(&g), &mut r)
        .unwrap();
    let admitted = o.issue_diagnostic_drain_v1(&g, &df(&g), &mut r).unwrap();
    let issued = o.issue_diagnostic_drain_v1(&g, &df(&g), &mut r).unwrap();
    o.submit_diagnostic_drain_v1(&admitted.task, &admitted.generation, 1, &g, &df(&g), &mut r)
        .unwrap();
    crate::coordinator::shutdown::tests::install(&mut o, &mut r);
    assert_eq!(
        o.issue_diagnostic_drain_v1(&g, &df(&g), &mut r),
        Err(DiagnosticDrainErrorV1::Unavailable)
    );
    assert_eq!(
        o.submit_diagnostic_drain_v1(&issued.task, &issued.generation, 1, &g, &df(&g), &mut r),
        Err(DiagnosticDrainErrorV1::Unavailable)
    );
    assert!(
        !o.query_diagnostic_drain_control_v1(&issued.task, &dc(&g), &mut r)
            .unwrap()
            .admitted
    );
    assert_eq!(
        o.submit_diagnostic_drain_v1(&admitted.task, &admitted.generation, 1, &g, &df(&g), &mut r),
        Ok(DiagnosticDrainPhaseV1::Pending)
    );
    assert_eq!(
        o.run_diagnostic_drain_v1(&admitted.task, &g, &df(&g), &qf(&q), &mut r),
        Ok(DiagnosticDrainPhaseV1::Terminal)
    );
    assert_eq!(
        o.query_diagnostic_drain_result_v1(&admitted.task, &g, &df(&g), &mut r)
            .unwrap()
            .drain,
        DiagnosticDrainDispositionV1::Performed
    );
    for task in [admitted, issued] {
        o.retire_diagnostic_drain_task_v1(&task.task, &dc(&g), &mut r)
            .unwrap();
    }
    o.stop_diagnostic_drain_v1(&dc(&g), &mut r).unwrap();
    o.retire_diagnostic_drain_v1(&dc(&g), &mut r).unwrap();
    o.stop_diagnostic_capture_v1(&q, false, &qc(&q), &mut r)
        .unwrap();
    o.retire_diagnostic_capture_v1(&qc(&q), &mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, extra); // Original shutdown holder is independent.
}

#[test]
fn a_false_empty_gauge_cannot_hide_known_tasks_from_protected_retirement() {
    let (mut o, q, g, mut r) = ready(false);
    o.open_diagnostic_drain_v1(&g, limits(), &dc(&g), &mut r)
        .unwrap();
    let task = o.issue_diagnostic_drain_v1(&g, &df(&g), &mut r).unwrap();
    o.stop_diagnostic_drain_v1(&dc(&g), &mut r).unwrap();
    let reserved = o.retention_usage().reserved;
    o.diagnostic_gateway_mut().unwrap().live = 0;
    assert_eq!(
        o.retire_diagnostic_drain_v1(&dc(&g), &mut r),
        Err(DiagnosticDrainErrorV1::Faulted)
    );
    assert_eq!(o.retention_usage().reserved, reserved);
    o.diagnostic_gateway_mut().unwrap().live = 1;
    o.retire_diagnostic_drain_task_v1(&task.task, &dc(&g), &mut r)
        .unwrap();
    o.retire_diagnostic_drain_v1(&dc(&g), &mut r).unwrap();
    o.stop_diagnostic_capture_v1(&q, false, &qc(&q), &mut r)
        .unwrap();
    o.retire_diagnostic_capture_v1(&qc(&q), &mut r).unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
