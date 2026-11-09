use super::scoped_basis::{context, facts, ready};
use super::*;
fn control(c: &SettingsExecutionContextV1) -> TrustedInvocationControlV1<'_> {
    TrustedInvocationControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        control_context_current: Some(true),
        control_scope_valid: Some(true),
        protected_control_ready: Some(true),
    }
}
fn dependencies() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 4096,
        },
    }
}
fn enqueue(
    o: &mut SourceCoordinator,
    c: &SettingsExecutionContextV1,
    r: &mut Recorder,
) -> InvocationId {
    let layers = [id("left")];
    let f = facts(c, &layers);
    let bases = [o
        .capture_settings_write_basis(&layers[0], c, &f, r)
        .unwrap()];
    let changes = [ScopedSettingChangeV1 {
        basis: 0,
        key: "unit".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Number(2.0)),
    }];
    let dependencies = dependencies();
    let request = ScopedSettingsRequestV1 {
        bases: &bases,
        changes: &changes,
        dependencies: &dependencies,
    };
    let gateway = TrustedSettingsMutationFactsV1 {
        basis: &f,
        host_basis_allowed: Some(true),
        required_reads: &[],
    };
    let ticket = o.issue_invocation(r).unwrap();
    o.submit_scoped_settings_change(&ticket, &request, c, &gateway, r)
        .unwrap();
    ticket
}

#[test]
fn every_strict_control_fact_and_original_context_field_is_required_without_poisoning_the_job() {
    for case in 0..15 {
        let (mut o, mut r) = ready(8, 8);
        let c = context();
        let ticket = enqueue(&mut o, &c, &mut r);
        let mut wrong = c.clone();
        match case {
            8 => wrong.authority = "wrong".into(),
            9 => wrong.engine = "wrong".into(),
            10 => wrong.recipient = "wrong".into(),
            11 => wrong.purpose = "wrong".into(),
            12 => wrong.destination = "wrong".into(),
            13 => wrong.profile = "wrong".into(),
            14 => wrong.continuity = "wrong".into(),
            _ => {}
        }
        let mut f = control(&wrong);
        let fact = if case % 2 == 0 { None } else { Some(false) };
        match case / 2 {
            0 => f.owner_control_allowed = fact,
            1 => f.control_context_current = fact,
            2 => f.control_scope_valid = fact,
            3 => f.protected_control_ready = fact,
            _ => {}
        }
        let generation = o.generation();
        let usage = o.settings_retention_usage();
        assert!(o.request_invocation_stop(&ticket, &f, &mut r).is_err());
        assert!(!o.is_faulted());
        assert_eq!(
            o.invocation_state(&ticket, &mut r),
            Ok(InvocationState::Pending)
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.settings_retention_usage(), usage);
        assert_eq!(o.run(&ticket, &mut r), Err(InvocationError::Unavailable));
        let layers = [id("left")];
        let facts = facts(&c, &layers);
        let gateway = TrustedSettingsMutationFactsV1 {
            basis: &facts,
            host_basis_allowed: Some(true),
            required_reads: &[],
        };
        o.run_scoped_settings_change(&ticket, &c, &gateway, &mut r)
            .unwrap();
        assert_eq!(read(&o, "A", &mut r), 2.0);
    }
}

#[test]
fn cancellation_needs_no_current_edit_or_read_permission_and_wrong_service_context_does_not_consume_it()
 {
    for generic in [false, true] {
        let (mut o, mut r) = ready(8, 8);
        let c = context();
        let ticket = enqueue(&mut o, &c, &mut r);
        o.request_invocation_stop(&ticket, &control(&c), &mut r)
            .unwrap();
        let mut f = facts(&c, &[]);
        f.authorized = Some(false);
        f.settings_writable = Some(false);
        f.basis_allowed = Some(false);
        f.resources_ready = None;
        let gateway = TrustedSettingsMutationFactsV1 {
            basis: &f,
            host_basis_allowed: None,
            required_reads: &[],
        };
        let mut wrong = c.clone();
        wrong.recipient = "other".into();
        assert_eq!(
            o.run_scoped_settings_change(&ticket, &wrong, &gateway, &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            o.invocation_state(&ticket, &mut r),
            Ok(InvocationState::Pending)
        );
        let generation = o.generation();
        if generic {
            o.run(&ticket, &mut r).unwrap();
        } else {
            o.run_scoped_settings_change(&ticket, &c, &gateway, &mut r)
                .unwrap();
        }
        assert_eq!(o.generation(), generation);
        assert_eq!(read(&o, "A", &mut r), 1.0);
        assert!(matches!(
            o.invocation_outcome(&ticket, &mut r),
            Ok(InvocationOutcome::StoppedBeforePublication)
        ));
    }
}

#[test]
fn publication_winning_first_cannot_be_relabelled_by_late_cancellation_or_forgotten_outcome() {
    let (mut o, mut r) = ready(8, 8);
    let c = context();
    let ticket = enqueue(&mut o, &c, &mut r);
    let layers = [id("left")];
    let f = facts(&c, &layers);
    let gateway = TrustedSettingsMutationFactsV1 {
        basis: &f,
        host_basis_allowed: Some(true),
        required_reads: &[],
    };
    o.run_scoped_settings_change(&ticket, &c, &gateway, &mut r)
        .unwrap();
    let generation = o.generation();
    assert!(
        o.request_invocation_stop(&ticket, &control(&c), &mut r)
            .is_err()
    );
    assert!(matches!(
        o.invocation_outcome(&ticket, &mut r),
        Ok(InvocationOutcome::ScopedSettingsCommitted(_))
    ));
    o.forget_invocation_outcome(&ticket, &mut r).unwrap();
    assert!(
        o.request_invocation_stop(&ticket, &control(&c), &mut r)
            .is_err()
    );
    assert_eq!(
        o.run_scoped_settings_change(&ticket, &c, &gateway, &mut r),
        Ok(InvocationState::Finished)
    );
    o.retire_invocation(&ticket, &mut r).unwrap();
    assert!(
        o.request_invocation_stop(&ticket, &control(&c), &mut r)
            .is_err()
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(read(&o, "A", &mut r), 2.0);
}

#[test]
fn stopping_one_invocation_preserves_eligible_peer_work_on_the_same_owner() {
    let (mut o, mut r) = ready(8, 8);
    let c = context();
    let stopped = enqueue(&mut o, &c, &mut r);
    let layers = [id("right")];
    let f = facts(&c, &layers);
    let bases = [o
        .capture_settings_write_basis(&layers[0], &c, &f, &mut r)
        .unwrap()];
    let changes = [ScopedSettingChangeV1 {
        basis: 0,
        key: "unit".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Number(3.0)),
    }];
    let dependencies = dependencies();
    let request = ScopedSettingsRequestV1 {
        bases: &bases,
        changes: &changes,
        dependencies: &dependencies,
    };
    let gateway = TrustedSettingsMutationFactsV1 {
        basis: &f,
        host_basis_allowed: Some(true),
        required_reads: &[],
    };
    let peer = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_change(&peer, &request, &c, &gateway, &mut r)
        .unwrap();
    o.request_invocation_stop(&stopped, &control(&c), &mut r)
        .unwrap();
    o.run_scoped_settings_change(&peer, &c, &gateway, &mut r)
        .unwrap();
    let generation = o.generation();
    o.run(&stopped, &mut r).unwrap();
    assert_eq!(o.generation(), generation);
    assert_eq!(read(&o, "A", &mut r), 1.0);
    assert_eq!(read(&o, "B", &mut r), 3.0);
    assert!(!o.is_faulted());
    assert!(matches!(
        o.invocation_outcome(&peer, &mut r),
        Ok(InvocationOutcome::ScopedSettingsCommitted(_))
    ));
    assert!(matches!(
        o.invocation_outcome(&stopped, &mut r),
        Ok(InvocationOutcome::StoppedBeforePublication)
    ));
}

#[test]
fn foreign_unsubmitted_and_context_free_jobs_refuse_scoped_control_without_impersonating_an_original_context()
 {
    let (mut o, mut r) = ready(8, 8);
    let c = context();
    let ticket = enqueue(&mut o, &c, &mut r);
    let (mut foreign, mut fr) = ready(8, 8);
    assert!(
        foreign
            .request_invocation_stop(&ticket, &control(&c), &mut fr)
            .is_err()
    );
    assert!(!foreign.is_faulted());
    let issued = o.issue_invocation(&mut r).unwrap();
    assert!(
        o.request_invocation_stop(&issued, &control(&c), &mut r)
            .is_err()
    );
    let source = o.create(b"A", &mut r).unwrap();
    let intent = SourceIntent {
        writes: vec![DocumentEdits {
            expected: source.clone(),
            edits: vec![vellum_engine::source::Replacement {
                start: 0,
                end: 1,
                expected: "A".into(),
                inserted: "B".into(),
            }],
        }],
        reads: vec![],
    };
    o.submit(&issued, &intent, &mut r).unwrap();
    assert!(
        o.request_invocation_stop(&issued, &control(&c), &mut r)
            .is_err()
    );
    o.run(&issued, &mut r).unwrap();
    assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "B");
    assert_eq!(
        o.invocation_state(&ticket, &mut r),
        Ok(InvocationState::Pending)
    );
}

#[test]
fn request_and_actual_settlement_have_separate_bounded_optional_content_free_observations() {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = ready(8, 8);
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let c = context();
        let ticket = enqueue(&mut o, &c, &mut r);
        let generation = o.generation();
        o.request_invocation_stop(&ticket, &control(&c), &mut r)
            .unwrap();
        let first: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        assert!(
            !first
                .iter()
                .any(|e| e.boundary == Boundary::InvocationStoppedSettlement)
        );
        if capacity == Some(128) {
            assert!(
                first
                    .iter()
                    .any(|e| e.boundary == Boundary::InvocationStopRequest
                        && e.disposition == Disposition::Completed)
            );
        }
        let dropped = r.dropped();
        o.run(&ticket, &mut r).unwrap();
        assert_eq!(o.generation(), generation);
        assert_eq!(read(&o, "A", &mut r), 1.0);
        let next: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            assert!(
                next.iter()
                    .any(|e| e.boundary == Boundary::InvocationStoppedSettlement
                        && e.disposition == Disposition::Completed)
            );
        }
        if capacity == Some(1) {
            assert!(dropped > 0 || r.dropped() > 0);
        }
        if capacity.is_none() {
            assert!(first.is_empty() && next.is_empty());
        }
        assert!(first.iter().chain(&next).all(|e| e.service_time.is_none()));
    }
}
#[test]
fn cancellation_records_intent_without_settlement_refund_or_publication_until_actual_service() {
    let (mut o, mut r) = ready(8, 8);
    let c = context();
    let layers = [id("left")];
    let f = facts(&c, &layers);
    let bases = [o
        .capture_settings_write_basis(&layers[0], &c, &f, &mut r)
        .unwrap()];
    let changes = [ScopedSettingChangeV1 {
        basis: 0,
        key: "unit".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Number(2.0)),
    }];
    let dependencies = dependencies();
    let request = ScopedSettingsRequestV1 {
        bases: &bases,
        changes: &changes,
        dependencies: &dependencies,
    };
    let gateway = TrustedSettingsMutationFactsV1 {
        basis: &f,
        host_basis_allowed: Some(true),
        required_reads: &[],
    };
    let ticket = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_change(&ticket, &request, &c, &gateway, &mut r)
        .unwrap();
    let generation = o.generation();
    let usage = o.settings_retention_usage();
    let invocation_usage = o.invocation_usage();
    o.request_invocation_stop(&ticket, &control(&c), &mut r)
        .unwrap();
    o.request_invocation_stop(&ticket, &control(&c), &mut r)
        .unwrap();
    assert_eq!(
        o.invocation_state(&ticket, &mut r),
        Ok(InvocationState::Pending)
    );
    assert!(o.invocation_outcome(&ticket, &mut r).is_err());
    assert_eq!(o.settings_retention_usage(), usage);
    assert_eq!(o.invocation_usage(), invocation_usage);
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.submit_scoped_settings_change(&ticket, &request, &c, &gateway, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        o.run_scoped_settings_change(&ticket, &c, &gateway, &mut r),
        Ok(InvocationState::Finished)
    );
    assert!(matches!(
        o.invocation_outcome(&ticket, &mut r),
        Ok(InvocationOutcome::StoppedBeforePublication)
    ));
    assert_eq!(o.generation(), generation);
    assert_eq!(read(&o, "A", &mut r), 1.0);
    assert_eq!(o.settings_retention_usage().retained, usage.retained);
    assert_eq!(
        o.settings_retention_usage().issued_lifetime,
        usage.issued_lifetime
    );
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    assert!(
        o.request_invocation_stop(&ticket, &control(&c), &mut r)
            .is_err()
    );
}
