use super::scoped_basis::{context, facts, ready};
use super::*;
#[path = "scoped/arguments.rs"]
pub(super) mod arguments;
fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 4096,
        },
    }
}
fn change(value: f64) -> ScopedSettingChangeV1 {
    ScopedSettingChangeV1 {
        basis: 0,
        key: "unit".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Number(value)),
    }
}
pub(super) fn gateway<'a>(
    basis: &'a TrustedSettingsBasisFactsV1<'a>,
) -> TrustedSettingsMutationFactsV1<'a> {
    TrustedSettingsMutationFactsV1 {
        basis,
        host_basis_allowed: Some(true),
        required_reads: &[],
    }
}
#[test]
fn changed_mandatory_read_count_refuses_before_normalization_or_basis_loss_with_optional_signals() {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = ready(8, 8);
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let c = context();
        let layers = [id("left")];
        let mut f = facts(&c, &layers);
        let bases = [o
            .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
            .unwrap()];
        let changes = [change(2.0)];
        let reads = reads();
        let request = ScopedSettingsRequestV1 {
            bases: &bases,
            changes: &changes,
            dependencies: &reads,
        };
        let p = o
            .prepare_scoped_settings_change(&request, &c, &gateway(&f), &mut r)
            .unwrap();
        let retained = o.settings_retention_usage().retained;
        let invalid = [ReadDependencyV3::ConfigurationState {
            registry: o.settings_registry(&mut r).unwrap(),
            expected: 0,
            interpretation: o.settings_basis(&mut r).unwrap().interpretation(),
        }];
        f.basis_allowed = Some(false);
        let mut g = gateway(&f);
        g.required_reads = &invalid;
        let before = o.generation();
        assert_eq!(
            o.commit_scoped_settings_change(p, &c, &g, &mut r),
            Err(SettingsChangeErrorV2::Settings(SettingsError::Stale))
        );
        assert!(!o.is_faulted());
        assert_eq!(o.generation(), before);
        assert_eq!(o.settings_retention_usage().retained, retained);
        assert_eq!(o.settings_retention_usage().open_preparations, 0);
        f.basis_allowed = Some(true);
        assert_eq!(
            o.inspect_settings_write_basis(&bases[0], &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert_eq!(read(&o, "A", &mut r), 1.0);
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            for boundary in [
                Boundary::ScopedSettingsValidation,
                Boundary::ScopedSettingsPublication,
            ] {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == boundary && e.disposition == Disposition::Refused)
                );
            }
        }
        if capacity.is_none() {
            assert!(events.is_empty());
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
#[test]
fn scoped_direct_change_uses_original_basis_and_rechecks_current_rights_before_publication() {
    for revoked in [false, true] {
        let (mut o, mut r) = ready(8, 8);
        let c = context();
        let layers = [id("left")];
        let mut f = facts(&c, &layers);
        let b = o
            .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
            .unwrap();
        let bases = [b];
        let changes = [change(2.0)];
        let reads = reads();
        let request = ScopedSettingsRequestV1 {
            bases: &bases,
            changes: &changes,
            dependencies: &reads,
        };
        let p = o
            .prepare_scoped_settings_change(&request, &c, &gateway(&f), &mut r)
            .unwrap();
        assert_eq!(read(&o, "A", &mut r), 1.0);
        set(&mut o, "right", 3.0, &mut r);
        if revoked {
            f.basis_allowed = Some(false);
        }
        let result = o.commit_scoped_settings_change(p, &c, &gateway(&f), &mut r);
        if revoked {
            assert!(result.is_err());
            assert_eq!(read(&o, "A", &mut r), 1.0);
        } else {
            assert!(result.unwrap().changed);
            assert_eq!(read(&o, "A", &mut r), 2.0);
        }
        assert_eq!(read(&o, "B", &mut r), 3.0);
    }
}
#[test]
fn a_semantically_invalid_later_payload_cannot_observe_loss_or_bind_a_job() {
    let (mut o, mut r) = ready(8, 8);
    let c = context();
    let layers = [id("left")];
    let mut f = facts(&c, &layers);
    let b = o
        .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
        .unwrap();
    let bases = [b];
    let changes = [ScopedSettingChangeV1 {
        basis: 0,
        key: "unit".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Boolean(true)),
    }];
    let reads = reads();
    let request = ScopedSettingsRequestV1 {
        bases: &bases,
        changes: &changes,
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    f.basis_allowed = Some(false);
    assert!(
        o.submit_scoped_settings_change(&job, &request, &c, &gateway(&f), &mut r)
            .is_err()
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
    f.authorized = Some(false);
    assert_eq!(
        o.submit_scoped_settings_change(&job, &request, &c, &gateway(&f), &mut r),
        Err(InvocationError::Unavailable)
    );
    f.authorized = Some(true);
    f.basis_allowed = Some(true);
    assert_eq!(
        o.inspect_settings_write_basis(&bases[0], &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
}
#[test]
fn every_reference_and_payload_is_bound_before_any_basis_loss_observation() {
    for kind in 0..6 {
        let (mut o, mut r) = ready(8, 8);
        let c = context();
        let layers = [id("left"), id("right")];
        let mut f = facts(&c, &layers);
        let first = o
            .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
            .unwrap();
        let second = o
            .capture_settings_write_basis(&id("right"), &c, &f, &mut r)
            .unwrap();
        let mut bases = vec![first.clone()];
        let mut changes = vec![change(2.0)];
        let reads = reads();
        match kind {
            0 => bases.push(second),
            1 => {
                bases.push(first.clone());
                changes.push(ScopedSettingChangeV1 {
                    basis: 1,
                    key: "unit".into(),
                    operation: SettingOperationV1::Reset,
                });
            }
            2 => changes[0].basis = 1,
            3 => changes.push(change(3.0)),
            4 => changes[0].key = "unknown".into(),
            _ => {
                let (mut foreign, mut rr) = ready(8, 8);
                bases.push(
                    foreign
                        .capture_settings_write_basis(&id("right"), &c, &f, &mut rr)
                        .unwrap(),
                );
                changes.push(ScopedSettingChangeV1 {
                    basis: 1,
                    key: "unit".into(),
                    operation: SettingOperationV1::Reset,
                });
            }
        }
        f.basis_allowed = Some(false);
        let request = ScopedSettingsRequestV1 {
            bases: &bases,
            changes: &changes,
            dependencies: &reads,
        };
        assert!(
            o.prepare_scoped_settings_change(&request, &c, &gateway(&f), &mut r)
                .is_err(),
            "kind {kind}"
        );
        f.basis_allowed = Some(true);
        assert_eq!(
            o.inspect_settings_write_basis(&first, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert_eq!(o.settings_retention_usage().open_preparations, 0);
    }
}
#[test]
fn multi_basis_batch_is_atomic_and_remove_reset_use_the_existing_semantics() {
    for action in ["set", "remove", "reset", "revoke"] {
        let (mut o, mut r) = ready(8, 8);
        let c = context();
        let layers = [id("left"), id("right")];
        let mut f = facts(&c, &layers);
        set(&mut o, "host", 3.0, &mut r);
        set(&mut o, "left", 2.0, &mut r);
        set(&mut o, "right", 2.0, &mut r);
        let bases = [
            o.capture_settings_write_basis(&id("left"), &c, &f, &mut r)
                .unwrap(),
            o.capture_settings_write_basis(&id("right"), &c, &f, &mut r)
                .unwrap(),
        ];
        let operation = || match action {
            "remove" => SettingOperationV1::Remove,
            "reset" => SettingOperationV1::Reset,
            _ => SettingOperationV1::Set(SettingValueV1::Number(4.0)),
        };
        let changes = [
            ScopedSettingChangeV1 {
                basis: 0,
                key: "unit".into(),
                operation: operation(),
            },
            ScopedSettingChangeV1 {
                basis: 1,
                key: "unit".into(),
                operation: operation(),
            },
        ];
        let reads = reads();
        let request = ScopedSettingsRequestV1 {
            bases: &bases,
            changes: &changes,
            dependencies: &reads,
        };
        let p = o
            .prepare_scoped_settings_change(&request, &c, &gateway(&f), &mut r)
            .unwrap();
        let only_left = [id("left")];
        if action == "revoke" {
            f.writable_layers = Some(&only_left);
        }
        let generation = o.generation();
        let result = o.commit_scoped_settings_change(p, &c, &gateway(&f), &mut r);
        let expected = match action {
            "remove" => 3.0,
            "reset" => 1.0,
            "revoke" => 2.0,
            _ => 4.0,
        };
        if action == "revoke" {
            assert!(result.is_err());
            assert_eq!(o.generation(), generation);
        } else {
            assert!(result.unwrap().changed);
            assert_eq!(o.generation(), generation + 1);
        }
        assert_eq!(read(&o, "A", &mut r), expected);
        assert_eq!(read(&o, "B", &mut r), expected);
    }
}
#[test]
fn generic_runner_cannot_bypass_fresh_facts_and_scoped_retry_never_refreshes_original_bases() {
    let (mut o, _) = ready(8, 8);
    let mut r = native_service_recorder();
    let c = context();
    let layers = [id("left")];
    let mut f = facts(&c, &layers);
    let original = o
        .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
        .unwrap();
    let other = o
        .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
        .unwrap();
    let bases = [original];
    let changes = [change(2.0)];
    let reads = reads();
    let request = ScopedSettingsRequestV1 {
        bases: &bases,
        changes: &changes,
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_change(&job, &request, &c, &gateway(&f), &mut r)
        .unwrap();
    assert_eq!(o.run(&job, &mut r), Err(InvocationError::Unavailable));
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Pending)
    );
    let alternate = [other];
    let replacement = ScopedSettingsRequestV1 {
        bases: &alternate,
        changes: &changes,
        dependencies: &reads,
    };
    assert_eq!(
        o.submit_scoped_settings_change(&job, &replacement, &c, &gateway(&f), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    let mut foreign_context = c.clone();
    foreign_context.recipient.push('x');
    assert_eq!(
        o.run_scoped_settings_change(&job, &foreign_context, &gateway(&f), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Pending)
    );
    f.basis_allowed = Some(false);
    o.run_scoped_settings_change(&job, &c, &gateway(&f), &mut r)
        .unwrap();
    assert!(matches!(
        o.invocation_outcome(&job, &mut r),
        Ok(InvocationOutcome::ScopedSettingsRejected(_))
    ));
    assert_eq!(read(&o, "A", &mut r), 1.0);
    f.basis_allowed = Some(true);
    let generation = o.generation();
    assert_eq!(
        o.submit_scoped_settings_change(&job, &request, &c, &gateway(&f), &mut r),
        Ok(InvocationState::Finished)
    );
    o.run_scoped_settings_change(&job, &c, &gateway(&f), &mut r)
        .unwrap();
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.inspect_settings_write_basis(&bases[0], &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    use vellum_engine::instrumentation::{Boundary, Disposition};
    assert_native_service_records(
        &mut r,
        &[
            (Boundary::InvocationExecution, Disposition::Refused),
            (Boundary::Outcome, Disposition::Completed),
            (Boundary::InvocationExecution, Disposition::Refused),
            (Boundary::Outcome, Disposition::Completed),
            (Boundary::InvocationExecution, Disposition::Completed),
            (Boundary::Outcome, Disposition::Completed),
            (Boundary::InvocationReplay, Disposition::Completed),
        ],
    );
}
#[test]
fn mandatory_and_caller_reads_conjoin_with_overlap_but_current_required_sets_cannot_be_dropped() {
    for changed in ["none", "source", "required"] {
        let (mut o, mut r) = ready(8, 8);
        let c = context();
        let layers = [id("left")];
        let f = facts(&c, &layers);
        let source = o.create(b"S", &mut r).unwrap();
        let predicate = ReadDependencyV3::Base(ReadDependencyV2::Base(ReadDependencyV1::Source(
            source.clone(),
        )));
        let required = [predicate.clone()];
        let mut reads = reads();
        reads.budget.max_predicates = 1;
        reads.predicates.push(predicate);
        let bases = [o
            .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
            .unwrap()];
        let changes = [change(2.0)];
        let request = ScopedSettingsRequestV1 {
            bases: &bases,
            changes: &changes,
            dependencies: &reads,
        };
        let mut g = gateway(&f);
        g.required_reads = &required;
        let p = o
            .prepare_scoped_settings_change(&request, &c, &g, &mut r)
            .unwrap();
        if changed == "source" {
            super::settings_reads::source_edit(&mut o, &source, &mut r);
        }
        if changed == "required" {
            g.required_reads = &[];
        }
        let generation = o.generation();
        let result = o.commit_scoped_settings_change(p, &c, &g, &mut r);
        if changed == "none" {
            assert!(result.unwrap().changed);
        } else {
            assert!(result.is_err());
            assert_eq!(o.generation(), generation);
        }
    }
}
#[test]
fn caller_mutation_and_raw_scoped_profile_substitution_cannot_rewrite_the_admitted_job() {
    let (mut o, mut r) = ready(8, 8);
    let c = context();
    let layers = [id("left")];
    let f = facts(&c, &layers);
    let b = o
        .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
        .unwrap();
    let mut bases = vec![b];
    let mut changes = vec![change(2.0)];
    let reads = reads();
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_change(
        &job,
        &ScopedSettingsRequestV1 {
            bases: &bases,
            changes: &changes,
            dependencies: &reads,
        },
        &c,
        &gateway(&f),
        &mut r,
    )
    .unwrap();
    assert_eq!(
        o.submit_settings_change(
            &job,
            &[super::change("left", 1, SettingOperationV1::Reset)],
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
    changes[0].operation = SettingOperationV1::Set(SettingValueV1::Number(8.0));
    bases.clear();
    o.run_scoped_settings_change(&job, &c, &gateway(&f), &mut r)
        .unwrap();
    assert_eq!(read(&o, "A", &mut r), 2.0);
    assert!(matches!(
        o.invocation_outcome(&job, &mut r),
        Ok(InvocationOutcome::ScopedSettingsCommitted(
            ScopedSettingsUpdateV1 { changed: true }
        ))
    ));
}
#[test]
fn empty_and_noop_requests_still_require_the_gateway_and_current_dependencies() {
    for empty in [false, true] {
        for denied in [false, true] {
            let (mut o, mut r) = ready(8, 8);
            let c = context();
            let layers = [id("left")];
            let f = facts(&c, &layers);
            set(&mut o, "left", 1.0, &mut r);
            let bases = if empty {
                vec![]
            } else {
                vec![
                    o.capture_settings_write_basis(&id("left"), &c, &f, &mut r)
                        .unwrap(),
                ]
            };
            let changes = if empty { vec![] } else { vec![change(1.0)] };
            let reads = reads();
            let p = o
                .prepare_scoped_settings_change(
                    &ScopedSettingsRequestV1 {
                        bases: &bases,
                        changes: &changes,
                        dependencies: &reads,
                    },
                    &c,
                    &gateway(&f),
                    &mut r,
                )
                .unwrap();
            let mut g = gateway(&f);
            if denied {
                g.host_basis_allowed = None;
            }
            let generation = o.generation();
            let result = o.commit_scoped_settings_change(p, &c, &g, &mut r);
            if denied {
                assert!(result.is_err());
            } else {
                assert!(!result.unwrap().changed);
            }
            assert_eq!(o.generation(), generation);
        }
    }
}
#[test]
fn stale_contributors_and_stopped_or_retired_bases_refuse_even_an_unchanged_write() {
    use super::scoped_basis::control;
    for event in ["contributor", "stop", "retire", "none"] {
        let (mut o, mut r) = ready(8, 8);
        let c = context();
        let layers = [id("left")];
        let f = facts(&c, &layers);
        set(&mut o, "left", 1.0, &mut r);
        let bases = [o
            .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
            .unwrap()];
        let changes = [change(1.0)];
        let reads = reads();
        let p = o
            .prepare_scoped_settings_change(
                &ScopedSettingsRequestV1 {
                    bases: &bases,
                    changes: &changes,
                    dependencies: &reads,
                },
                &c,
                &gateway(&f),
                &mut r,
            )
            .unwrap();
        match event {
            "contributor" => {
                set(&mut o, "host", 1.0, &mut r);
            }
            "stop" => {
                assert!(o.stop_settings_write_basis(&bases[0], &c, &control(&c), &mut r));
            }
            "retire" => {
                assert!(o.retire_settings_write_basis(&bases[0], &c, &control(&c), &mut r));
            }
            _ => {}
        }
        let generation = o.generation();
        let result = o.commit_scoped_settings_change(p, &c, &gateway(&f), &mut r);
        if event == "none" {
            assert!(!result.unwrap().changed);
        } else {
            assert!(result.is_err());
        }
        assert_eq!(o.generation(), generation);
    }
}
#[test]
fn cancelling_and_forgetting_a_scoped_job_does_not_discharge_bases_or_rearm_execution() {
    let (mut o, mut r) = ready(8, 8);
    let c = context();
    let layers = [id("left")];
    let f = facts(&c, &layers);
    let bases = [o
        .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
        .unwrap()];
    let changes = [change(2.0)];
    let reads = reads();
    let request = ScopedSettingsRequestV1 {
        bases: &bases,
        changes: &changes,
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_change(&job, &request, &c, &gateway(&f), &mut r)
        .unwrap();
    o.stop_invocation(&job, &mut r).unwrap();
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    assert_eq!(
        o.inspect_settings_write_basis(&bases[0], &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert_eq!(o.settings_retention_usage().retained, 1);
    o.forget_invocation_outcome(&job, &mut r).unwrap();
    assert_eq!(
        o.submit_scoped_settings_change(&job, &request, &c, &gateway(&f), &mut r),
        Ok(InvocationState::Finished)
    );
    o.run_scoped_settings_change(&job, &c, &gateway(&f), &mut r)
        .unwrap();
    assert_eq!(read(&o, "A", &mut r), 1.0);
    o.retire_invocation(&job, &mut r).unwrap();
    assert_eq!(
        o.submit_scoped_settings_change(&job, &request, &c, &gateway(&f), &mut r),
        Err(InvocationError::Unavailable)
    );
}
#[test]
fn scoped_preparation_service_and_replay_have_optional_content_free_signals() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(256), Some(1)] {
        let (mut o, _) = ready(8, 8);
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let c = context();
        let layers = [id("left")];
        let f = facts(&c, &layers);
        let bases = [o
            .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
            .unwrap()];
        let changes = [change(2.0)];
        let reads = reads();
        let request = ScopedSettingsRequestV1 {
            bases: &bases,
            changes: &changes,
            dependencies: &reads,
        };
        let job = o.issue_invocation(&mut r).unwrap();
        o.submit_scoped_settings_change(&job, &request, &c, &gateway(&f), &mut r)
            .unwrap();
        o.run_scoped_settings_change(&job, &c, &gateway(&f), &mut r)
            .unwrap();
        assert_eq!(read(&o, "A", &mut r), 2.0);
        o.run_scoped_settings_change(&job, &c, &gateway(&f), &mut r)
            .unwrap();
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(256) {
            for boundary in [
                Boundary::ScopedSettingsPreparation,
                Boundary::ScopedSettingsValidation,
                Boundary::ScopedSettingsPublication,
                Boundary::InvocationReplay,
                Boundary::SettingsBasisInspection,
            ] {
                assert!(events.iter().any(|e| e.boundary == boundary));
            }
        }
        if capacity.is_none() {
            assert!(events.is_empty());
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
