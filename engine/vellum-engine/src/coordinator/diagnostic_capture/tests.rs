use super::*;
fn ready() -> (SourceCoordinator, DiagnosticScopeV1, Recorder) {
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
    let mut actor = crate::coordinator::shutdown::tests::context();
    actor.profile = BOUNDED_OPTIONAL_CAPTURE_PROFILE_V1.into();
    let s = DiagnosticScopeV1 {
        context: actor,
        policy: "p1".into(),
    };
    let l = DiagnosticCaptureLimitsV1 {
        max_records: 1,
        max_queue_bytes: 256,
        max_preparations: 1,
        max_prepared_issued: 100,
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
    o.configure_retention_budget(l.required_hold_bytes().unwrap(), &mut r)
        .unwrap();
    o.configure_diagnostic_capture_v1(&s, l, &control(&s), &mut r)
        .unwrap();
    (o, s, r)
}
pub(super) fn control(s: &DiagnosticScopeV1) -> DiagnosticControlV1<'_> {
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
pub(super) fn facts(s: &DiagnosticScopeV1) -> TrustedDiagnosticFactsV1<'_> {
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
#[test]
fn capture_reuses_only_prepaid_native_storage_under_pressure() {
    let (mut o, s, mut r) = ready();
    let p = o.diagnostic_pool().unwrap();
    let initial = (
        p.queue.capacity(),
        p.preparations.capacity(),
        o.retention_usage(),
    );
    for _ in 0..100 {
        let p = o
            .prepare_diagnostic_marker_v1(
                &s,
                DiagnosticMarkerV1 {
                    site: DiagnosticSiteV1::Ins01,
                    milestone: DiagnosticMilestoneV1::Offered,
                },
                &facts(&s),
                &mut r,
            )
            .unwrap();
        o.capture_diagnostic_marker_v1(&s, &p, &facts(&s), &mut r)
            .unwrap();
    }
    let p = o.diagnostic_pool().unwrap();
    assert_eq!(
        (
            p.queue.capacity(),
            p.preparations.capacity(),
            o.retention_usage()
        ),
        initial
    );
    assert_eq!(p.queue.len(), 1);
    assert_eq!(p.issued, 100);
    assert_eq!(p.live, 0);
    assert!(!p.counters.offered.exact);
    assert!(!p.counters.dropped.exact);
}
#[test]
fn shutdown_accounts_for_actual_queue_preparation_binding_and_full_parent_refund() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read};
    let (mut o, s, mut r) = ready();
    let marker = DiagnosticMarkerV1 {
        site: DiagnosticSiteV1::Ins01,
        milestone: DiagnosticMilestoneV1::Completed,
    };
    let token = o
        .prepare_diagnostic_marker_v1(&s, marker, &facts(&s), &mut r)
        .unwrap();
    o.capture_diagnostic_marker_v1(&s, &token, &facts(&s), &mut r)
        .unwrap();
    let _prepared = o
        .prepare_diagnostic_marker_v1(&s, marker, &facts(&s), &mut r)
        .unwrap();
    let binding = o.bind_diagnostic_drain_v1(&control(&s), &mut r).unwrap();
    let cut = inspect(&o, &mut r);
    let read = inspect_read(&o, &cut, &mut r);
    assert!(read.coverage.all_owner_obligations);
    assert_eq!(read.funding.unrepresented_parent_bytes, 0);
    let row = read
        .owners
        .iter()
        .find(|r| r.owner.role() == ShutdownOwnerRoleV1::DiagnosticCapture)
        .unwrap();
    assert_eq!(row.retained_records, Some(4));
    assert_eq!(row.reserved_bytes, Some(o.retention_usage().reserved));
    o.stop_diagnostic_capture_v1(&s, false, &control(&s), &mut r)
        .unwrap();
    o.release_diagnostic_drain_binding_v1(&binding, &control(&s), &mut r)
        .unwrap();
    o.retire_diagnostic_capture_v1(&control(&s), &mut r)
        .unwrap();
    let cut = inspect(&o, &mut r);
    let read = inspect_read(&o, &cut, &mut r);
    assert!(read.coverage.all_owner_obligations);
    assert_eq!(o.retention_usage().reserved, 0);
    let row = read
        .owners
        .iter()
        .find(|r| r.owner.role() == ShutdownOwnerRoleV1::DiagnosticCapture)
        .unwrap();
    assert_eq!(row.state, ShutdownOwnerStateV1::Closed);
    assert_eq!(row.issued_slots, Some(2));
}

#[test]
fn failed_configuration_keeps_actual_parent_and_domain_slot_unspent() {
    let (mut existing, s, mut r) = ready();
    let mut l = existing.diagnostic_pool().unwrap().limits;
    let charge = l.required_hold_bytes().unwrap();
    existing
        .stop_diagnostic_capture_v1(&s, false, &control(&s), &mut r)
        .unwrap();
    existing
        .retire_diagnostic_capture_v1(&control(&s), &mut r)
        .unwrap();
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
    o.configure_retention_budget(charge, &mut r).unwrap();
    l.max_held_bytes = charge - 1;
    assert_eq!(
        o.configure_diagnostic_capture_v1(&s, l, &control(&s), &mut r),
        Err(CoordinatorError::Limit)
    );
    assert!(matches!(o.diagnostic_capture, State::Unconfigured));
    assert_eq!(o.retention_usage().reserved, 0);
    l.max_held_bytes = charge;
    o.configure_diagnostic_capture_v1(&s, l, &control(&s), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, charge);
}
