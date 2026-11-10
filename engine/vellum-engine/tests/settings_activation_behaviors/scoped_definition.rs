use super::scoped_basis::{context, facts};
use super::*;
#[path = "scoped_definition/arguments.rs"]
pub(super) mod arguments;
pub(super) fn limits() -> SettingsDefinitionLimitsV1 {
    SettingsDefinitionLimitsV1 {
        max_request_bytes: 65536,
        max_plan_bytes: 1_048_576,
        max_work: 1_048_576,
    }
}
fn ready() -> (SourceCoordinator, Recorder) {
    let (mut o, mut r) = super::scoped_basis::ready(16, 8);
    o.configure_settings_definitions(limits(), &mut r).unwrap();
    (o, r)
}
pub(super) fn basis<'a>(
    common: &'a TrustedSettingsBasisFactsV1<'a>,
) -> TrustedSettingsDefinitionBasisFactsV1<'a> {
    TrustedSettingsDefinitionBasisFactsV1 {
        common,
        definition: limits(),
        basis_allowed: Some(true),
    }
}
pub(super) fn gate<'a>(
    basis: &'a TrustedSettingsDefinitionBasisFactsV1<'a>,
) -> TrustedSettingsDefinitionMutationFactsV1<'a> {
    TrustedSettingsDefinitionMutationFactsV1 {
        basis,
        host_basis_allowed: Some(true),
        schema_writable: Some(true),
        restrictions_writable: Some(true),
        associations_writable: Some(true),
        required_reads: &[],
    }
}
fn reads() -> ReadDependencyRequestV3 {
    ReadDependencyRequestV3 {
        predicates: vec![],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 4096,
        },
    }
}
fn control(c: &SettingsExecutionContextV1) -> SettingsBasisControlV1<'_> {
    SettingsBasisControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
#[test]
fn definition_role_is_separate_from_layer_permission_and_shares_funded_capacity_and_spent_issuance()
{
    let (mut o, mut r) = ready();
    let c = context();
    let mut common = facts(&c, &[]);
    common.basis_allowed = None;
    common.writable_layers = None;
    let f = basis(&common);
    let generation = o.generation();
    let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    assert_eq!(
        o.inspect_settings_definition_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    let interpretation = o.acquire_settings_interpretation(&mut r).unwrap();
    assert_eq!(o.settings_retention_usage().retained, 2);
    assert_eq!(o.settings_retention_usage().issued_lifetime, 2);
    assert_eq!(o.generation(), generation);
    assert!(o.stop_settings_definition_basis(&b, &c, &control(&c), &mut r));
    assert_eq!(o.settings_retention_usage().retained, 2);
    assert!(o.retire_settings_definition_basis(&b, &c, &control(&c), &mut r));
    o.release_settings_interpretation(&interpretation, &mut r)
        .unwrap();
    for _ in 2..16 {
        let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
        assert!(o.retire_settings_definition_basis(&b, &c, &control(&c), &mut r));
    }
    assert!(o.capture_settings_definition_basis(&c, &f, &mut r).is_err());
    assert_eq!(o.settings_retention_usage().issued_lifetime, 16);
    assert_eq!(o.settings_retention_usage().retained, 0);
}
#[test]
fn definition_binding_rejects_foreign_context_profile_and_owner_before_loss_observation() {
    for field in 0..9 {
        let (mut o, mut r) = ready();
        let c = context();
        let common = facts(&c, &[]);
        let f = basis(&common);
        let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
        let mut wrong = c.clone();
        match field {
            0 => wrong.authority = "other".into(),
            1 => wrong.engine = "other".into(),
            2 => wrong.recipient = "other".into(),
            3 => wrong.purpose = "other".into(),
            4 => wrong.destination = "other".into(),
            5 => wrong.profile = "other".into(),
            6 => wrong.continuity = "other".into(),
            _ => {}
        }
        let mut wrong_common = facts(&wrong, &[]);
        if field == 7 {
            wrong_common.activation.max_work += 1;
        }
        let mut bad = basis(&wrong_common);
        if field == 8 {
            bad.definition.max_work += 1;
        }
        bad.basis_allowed = Some(false);
        assert_eq!(
            o.inspect_settings_definition_basis(&b, &wrong, &bad, &mut r),
            SettingsBasisAvailabilityV1::Unavailable
        );
        assert_eq!(
            o.inspect_settings_definition_basis(&b, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        let (mut other, mut rr) = ready();
        assert_eq!(
            other.inspect_settings_definition_basis(&b, &c, &f, &mut rr),
            SettingsBasisAvailabilityV1::Unavailable
        );
    }
}
#[test]
fn recognized_definition_loss_is_absorbing_but_unknown_and_transient_refusals_do_not_poison() {
    for field in 0..10 {
        for value in [None, Some(false)] {
            let (mut o, mut r) = ready();
            let c = context();
            let mut common = facts(&c, &[]);
            let b = o
                .capture_settings_definition_basis(&c, &basis(&common), &mut r)
                .unwrap();
            match field {
                0 => common.authorized = value,
                1 => common.scope = value,
                2 => common.continuous = value,
                3 => common.live = value,
                4 => common.interpretation_current = value,
                5 => common.settings_writable = value,
                6 => common.resources_ready = value,
                7 => common.not_stopped = value,
                8 => common.owner_control_allowed = value,
                _ => {}
            }
            let mut f = basis(&common);
            if field == 9 {
                f.basis_allowed = value;
            }
            assert_eq!(
                o.inspect_settings_definition_basis(&b, &c, &f, &mut r),
                SettingsBasisAvailabilityV1::Unavailable
            );
            let common = facts(&c, &[]);
            let lost = value == Some(false) && (field <= 5 || field == 9);
            assert_eq!(
                o.inspect_settings_definition_basis(&b, &c, &basis(&common), &mut r),
                if lost {
                    SettingsBasisAvailabilityV1::Unavailable
                } else {
                    SettingsBasisAvailabilityV1::Current
                }
            );
        }
    }
}
#[test]
fn layer_and_source_progress_commute_with_basis_while_scoped_publication_preserves_authored_values()
{
    let (mut o, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    set(&mut o, "left", 4.0, &mut r);
    assert_eq!(
        o.inspect_settings_definition_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    let source = o.create(b"A", &mut r).unwrap();
    let mut declaration = super::definition::declaration();
    declaration.schemas[0].sensitive = true;
    let reads = reads();
    let p = o
        .prepare_scoped_settings_definition(
            &ScopedSettingsDefinitionRequestV1 {
                basis: &b,
                declaration: &declaration,
                dependencies: &reads,
            },
            &c,
            &gate(&f),
            &mut r,
        )
        .unwrap();
    assert_eq!(
        format!("{p:?}"),
        "PreparedScopedSettingsDefinitionV1(<redacted>)"
    );
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: source.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "B".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(edit, &mut r).unwrap();
    let before = o.read(&source.document, &mut r).unwrap().version;
    assert!(
        o.commit_scoped_settings_definition(p, &c, &gate(&f), &mut r)
            .unwrap()
            .changed
    );
    assert_eq!(read(&o, "A", &mut r), 4.0);
    assert_eq!(o.read(&source.document, &mut r).unwrap().version, before);
    assert_eq!(
        o.inspect_settings_definition_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
}
#[test]
fn changed_facets_need_independent_fresh_native_grants_and_noop_needs_no_unrelated_grant() {
    for facet in 0..3 {
        for revoke in [false, true] {
            let (mut o, mut r) = ready();
            let c = context();
            let common = facts(&c, &[]);
            let f = basis(&common);
            let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
            let mut declaration = super::definition::declaration();
            match facet {
                0 => declaration.schemas[0].sensitive = true,
                1 => declaration.restrictions.push((
                    "unit".into(),
                    vec![SettingValueV1::Number(1.0), SettingValueV1::Number(2.0)],
                )),
                _ => declaration.targets[0].layers = vec![id("host")],
            }
            let reads = reads();
            let req = ScopedSettingsDefinitionRequestV1 {
                basis: &b,
                declaration: &declaration,
                dependencies: &reads,
            };
            let mut g = gate(&f);
            g.schema_writable = None;
            g.restrictions_writable = None;
            g.associations_writable = None;
            assert!(
                o.prepare_scoped_settings_definition(&req, &c, &g, &mut r)
                    .is_err()
            );
            match facet {
                0 => g.schema_writable = Some(true),
                1 => g.restrictions_writable = Some(true),
                _ => g.associations_writable = Some(true),
            }
            let p = o
                .prepare_scoped_settings_definition(&req, &c, &g, &mut r)
                .unwrap();
            if revoke {
                match facet {
                    0 => g.schema_writable = None,
                    1 => g.restrictions_writable = Some(false),
                    _ => g.associations_writable = None,
                }
            }
            let before = o.generation();
            let result = o.commit_scoped_settings_definition(p, &c, &g, &mut r);
            assert_eq!(result.is_ok(), !revoke);
            if revoke {
                assert_eq!(o.generation(), before);
            }
        }
    }
    let (mut o, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let declaration = super::definition::declaration();
    let reads = reads();
    let mut g = gate(&f);
    g.schema_writable = None;
    g.restrictions_writable = None;
    g.associations_writable = None;
    let before = o.generation();
    let p = o
        .prepare_scoped_settings_definition(
            &ScopedSettingsDefinitionRequestV1 {
                basis: &b,
                declaration: &declaration,
                dependencies: &reads,
            },
            &c,
            &g,
            &mut r,
        )
        .unwrap();
    assert!(
        !o.commit_scoped_settings_definition(p, &c, &g, &mut r)
            .unwrap()
            .changed
    );
    assert_eq!(o.generation(), before);
}
#[test]
fn invalid_proposal_and_denied_host_resolution_cannot_close_a_valid_basis_or_bind_a_job() {
    let (mut o, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let mut f = basis(&common);
    let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let mut d = super::definition::declaration();
    d.schemas[0].default = SettingValueV1::Number(99.0);
    let reads = reads();
    let job = o.issue_invocation(&mut r).unwrap();
    f.basis_allowed = Some(false);
    assert!(
        o.submit_scoped_settings_definition(
            &job,
            &ScopedSettingsDefinitionRequestV1 {
                basis: &b,
                declaration: &d,
                dependencies: &reads
            },
            &c,
            &gate(&f),
            &mut r
        )
        .is_err()
    );
    assert_eq!(
        o.invocation_state(&job, &mut r),
        Ok(InvocationState::Issued)
    );
    f.basis_allowed = Some(true);
    assert_eq!(
        o.inspect_settings_definition_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    let d = super::definition::declaration();
    let mut g = gate(&f);
    g.host_basis_allowed = None;
    assert!(
        o.prepare_scoped_settings_definition(
            &ScopedSettingsDefinitionRequestV1 {
                basis: &b,
                declaration: &d,
                dependencies: &reads
            },
            &c,
            &g,
            &mut r
        )
        .is_err()
    );
    assert_eq!(
        o.inspect_settings_definition_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
}
#[test]
fn scoped_original_basis_identity_raw_exclusivity_and_historical_replay_are_once_only() {
    let (mut o, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let a = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let mut d = super::definition::declaration();
    d.schemas[0].sensitive = true;
    let reads = reads();
    let req = ScopedSettingsDefinitionRequestV1 {
        basis: &a,
        declaration: &d,
        dependencies: &reads,
    };
    let job = o.issue_invocation(&mut r).unwrap();
    let g = gate(&f);
    o.submit_scoped_settings_definition(&job, &req, &c, &g, &mut r)
        .unwrap();
    assert_eq!(o.run(&job, &mut r), Err(InvocationError::Unavailable));
    assert_eq!(
        o.submit_scoped_settings_definition(
            &job,
            &ScopedSettingsDefinitionRequestV1 {
                basis: &b,
                declaration: &d,
                dependencies: &reads
            },
            &c,
            &g,
            &mut r
        ),
        Err(InvocationError::IntentMismatch)
    );
    let expected = o.settings_basis(&mut r).unwrap().interpretation();
    assert_eq!(
        o.submit_settings_definition(&job, expected, &d, &reads, &mut r),
        Err(InvocationError::IntentMismatch)
    );
    o.run_scoped_settings_definition(&job, &c, &g, &mut r)
        .unwrap();
    let before = o.generation();
    assert!(matches!(
        o.invocation_outcome(&job, &mut r).unwrap(),
        InvocationOutcome::ScopedDefinitionCommitted(ScopedSettingsUpdateV1 { changed: true })
    ));
    assert!(o.retire_settings_definition_basis(&a, &c, &control(&c), &mut r));
    assert_eq!(
        o.submit_scoped_settings_definition(&job, &req, &c, &g, &mut r),
        Ok(InvocationState::Finished)
    );
    o.run_scoped_settings_definition(&job, &c, &g, &mut r)
        .unwrap();
    assert_eq!(o.generation(), before);
}
#[test]
fn scoped_current_reads_and_required_set_are_rechecked_even_for_noop() {
    for change in ["none", "source", "required"] {
        let (mut o, mut r) = ready();
        let c = context();
        let common = facts(&c, &[]);
        let f = basis(&common);
        let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
        let source = o.create(b"A", &mut r).unwrap();
        let p = ReadDependencyV3::Base(ReadDependencyV2::Base(ReadDependencyV1::Source(
            source.clone(),
        )));
        let required = [p.clone()];
        let mut reads = reads();
        reads.predicates.push(p);
        reads.budget.max_predicates = 1;
        let d = super::definition::declaration();
        let mut g = gate(&f);
        g.required_reads = &required;
        let job = o.issue_invocation(&mut r).unwrap();
        o.submit_scoped_settings_definition(
            &job,
            &ScopedSettingsDefinitionRequestV1 {
                basis: &b,
                declaration: &d,
                dependencies: &reads,
            },
            &c,
            &g,
            &mut r,
        )
        .unwrap();
        if change == "source" {
            let p = o
                .prepare(
                    &[DocumentEdits {
                        expected: source,
                        edits: vec![vellum_engine::source::Replacement {
                            start: 0,
                            end: 1,
                            expected: "A".into(),
                            inserted: "B".into(),
                        }],
                    }],
                    &[],
                    &mut r,
                )
                .unwrap();
            o.commit(p, &mut r).unwrap();
        }
        if change == "required" {
            g.required_reads = &[];
        }
        let before = o.generation();
        o.run_scoped_settings_definition(&job, &c, &g, &mut r)
            .unwrap();
        assert_eq!(
            matches!(
                o.invocation_outcome(&job, &mut r).unwrap(),
                InvocationOutcome::ScopedDefinitionCommitted(_)
            ),
            change == "none"
        );
        assert_eq!(o.generation(), before);
    }
}
#[test]
fn cancellation_forgetting_and_retirement_do_not_rearm_or_discharge_independent_basis() {
    let (mut o, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let mut d = super::definition::declaration();
    d.schemas[0].sensitive = true;
    let reads = reads();
    let job = o.issue_invocation(&mut r).unwrap();
    let req = ScopedSettingsDefinitionRequestV1 {
        basis: &b,
        declaration: &d,
        dependencies: &reads,
    };
    o.submit_scoped_settings_definition(&job, &req, &c, &gate(&f), &mut r)
        .unwrap();
    o.stop_invocation(&job, &mut r).unwrap();
    o.forget_invocation_outcome(&job, &mut r).unwrap();
    let before = o.generation();
    o.run_scoped_settings_definition(&job, &c, &gate(&f), &mut r)
        .unwrap();
    assert_eq!(o.generation(), before);
    assert_eq!(
        o.inspect_settings_definition_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    o.retire_invocation(&job, &mut r).unwrap();
    assert_eq!(
        o.submit_scoped_settings_definition(&job, &req, &c, &gate(&f), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(o.settings_retention_usage().retained, 1);
}
#[test]
fn definition_basis_and_scoped_signals_are_bounded_optional_and_content_free() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = ready();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let c = context();
        let common = facts(&c, &[]);
        let f = basis(&common);
        let b = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
        o.inspect_settings_definition_basis(&b, &c, &f, &mut r);
        let mut d = super::definition::declaration();
        d.schemas[0].sensitive = true;
        let reads = reads();
        let p = o
            .prepare_scoped_settings_definition(
                &ScopedSettingsDefinitionRequestV1 {
                    basis: &b,
                    declaration: &d,
                    dependencies: &reads,
                },
                &c,
                &gate(&f),
                &mut r,
            )
            .unwrap();
        assert!(
            o.commit_scoped_settings_definition(p, &c, &gate(&f), &mut r)
                .unwrap()
                .changed
        );
        assert!(o.stop_settings_definition_basis(&b, &c, &control(&c), &mut r));
        assert!(o.retire_settings_definition_basis(&b, &c, &control(&c), &mut r));
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == Some(128) {
            for boundary in [
                Boundary::DefinitionBasisCapture,
                Boundary::DefinitionBasisInspection,
                Boundary::DefinitionBasisStop,
                Boundary::DefinitionBasisRetirement,
                Boundary::ScopedDefinitionPreparation,
                Boundary::ScopedDefinitionValidation,
                Boundary::ScopedDefinitionPublication,
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
        assert_eq!(o.settings_retention_usage().retained, 0);
    }
}
