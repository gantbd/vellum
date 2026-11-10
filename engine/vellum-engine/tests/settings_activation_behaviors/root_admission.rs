use super::*;
use vellum_engine::instrumentation::{Boundary, Disposition};

#[derive(Clone, Copy, Debug)]
enum Kind {
    Value,
    WithReads,
    Declaration,
    Layers,
}
const KINDS: [Kind; 4] = [
    Kind::Value,
    Kind::WithReads,
    Kind::Declaration,
    Kind::Layers,
];
enum Pending {
    Value(PreparedSettingsChange),
    Reads(PreparedSettingsChangeV2),
    Declaration(PreparedSettingsDefinitionV1),
    Layers(PreparedSettingsLayersV1),
}
fn ready() -> (SourceCoordinator, Recorder, DocumentVersion) {
    let (mut o, mut r) = setup_with_root_budget(profile(), false, None);
    o.configure_settings_definitions(
        SettingsDefinitionLimitsV1 {
            max_request_bytes: 65536,
            max_plan_bytes: 1_048_576,
            max_work: 1_048_576,
        },
        &mut r,
    )
    .unwrap();
    o.configure_settings_layers(
        SettingsLayerLimitsV1 {
            max_changes: 16,
            max_created_value_bytes: 65536,
            max_request_bytes: 65536,
            max_plan_bytes: 1_048_576,
            max_work: 1_048_576,
        },
        &mut r,
    )
    .unwrap();
    let source = o.create(b"A", &mut r).unwrap();
    (o, r, source)
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
fn declaration() -> SettingsDeclarationV1 {
    let mut d = definition::declaration();
    d.schemas.push(SettingSchemaV1 {
        key: "extra".into(),
        value_type: SettingTypeV1::Boolean,
        default: SettingValueV1::Boolean(false),
        domain: SettingDomainV1::Analysis,
        scopes: vec![SettingScopeV1::Host],
        composition: SettingCompositionV1::Replace,
        sensitive: true,
    });
    d
}
fn prepare(
    o: &SourceCoordinator,
    k: Kind,
    r: &mut Recorder,
) -> Result<Pending, SettingsChangeErrorV2> {
    let q = [change(
        "left",
        1,
        SettingOperationV1::Set(SettingValueV1::Number(2.0)),
    )];
    match k {
        Kind::Value => o
            .prepare_settings_change(&q, r)
            .map(Pending::Value)
            .map_err(SettingsChangeErrorV2::Settings),
        Kind::WithReads => o
            .prepare_settings_change_with_reads(&q, &reads(), r)
            .map(Pending::Reads),
        Kind::Declaration => o
            .prepare_settings_definition(
                o.settings_basis(r).unwrap().interpretation(),
                &declaration(),
                &reads(),
                r,
            )
            .map(Pending::Declaration),
        Kind::Layers => o
            .prepare_settings_layers(
                o.settings_layer_basis(r).unwrap(),
                &SettingsLayerChangeV1 {
                    create: vec![SettingLayerCreateV1 {
                        identity: SettingIdentityV1 {
                            authority: "engine".into(),
                            id: "new".into(),
                            incarnation: 2,
                        },
                        scope: SettingScopeV1::Workspace,
                        values: vec![],
                    }],
                    remove: vec![],
                    targets: definition::declaration().targets,
                },
                &reads(),
                r,
            )
            .map(Pending::Layers),
    }
}
fn commit(
    o: &mut SourceCoordinator,
    p: Pending,
    r: &mut Recorder,
) -> Result<bool, SettingsChangeErrorV2> {
    match p {
        Pending::Value(p) => o
            .commit_settings_change(p, r)
            .map(|v| v.changed)
            .map_err(SettingsChangeErrorV2::Settings),
        Pending::Reads(p) => o.commit_settings_change_with_reads(p, r).map(|v| v.changed),
        Pending::Declaration(p) => o.commit_settings_definition(p, r).map(|v| v.changed),
        Pending::Layers(p) => o.commit_settings_layers(p, r).map(|v| v.changed),
    }
}
fn candidate_size(k: Kind) -> usize {
    let (mut o, mut r, _) = ready();
    o.configure_semantic_root_budget(1_048_576, &mut r).unwrap();
    let p = prepare(&o, k, &mut r).unwrap();
    assert_eq!(commit(&mut o, p, &mut r), Ok(true));
    o.semantic_root_usage(&mut r).unwrap().total_bytes
}
#[test]
fn every_shared_settings_path_requires_an_explicit_root_budget() {
    for k in KINDS {
        let (o, mut r, _) = ready();
        let before = o.generation();
        assert!(matches!(
            prepare(&o, k, &mut r),
            Err(SettingsChangeErrorV2::Settings(SettingsError::Unavailable))
        ));
        assert_eq!(o.generation(), before);
        assert_eq!(o.settings_retention_usage().open_preparations, 0);
    }
}
#[test]
fn every_shared_settings_path_checks_actual_candidate_whole_root_at_exact_boundaries() {
    for k in KINDS {
        let exact = candidate_size(k);
        for delta in [-1isize, 0, 1] {
            for capacity in [None, Some(128), Some(1)] {
                let (mut o, _, _) = ready();
                let mut r = capacity.map_or_else(Recorder::disabled, |n| {
                    Recorder::try_with_capacity(n).unwrap()
                });
                o.configure_semantic_root_budget(exact.checked_add_signed(delta).unwrap(), &mut r)
                    .unwrap();
                let generation = o.generation();
                let p = prepare(&o, k, &mut r);
                if delta < 0 {
                    assert!(
                        matches!(
                            p,
                            Err(SettingsChangeErrorV2::Settings(SettingsError::Limit))
                        ),
                        "{k:?}"
                    );
                    assert_eq!(o.generation(), generation);
                    assert_eq!(o.settings_retention_usage().open_preparations, 0);
                } else {
                    assert_eq!(commit(&mut o, p.unwrap(), &mut r), Ok(true));
                    assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, exact);
                }
                if capacity == Some(128) {
                    assert!(std::iter::from_fn(|| r.pop_front()).any(|e| e.boundary
                        == Boundary::SemanticRootAdmission
                        && e.disposition
                            == if delta < 0 {
                                Disposition::Refused
                            } else {
                                Disposition::Completed
                            }));
                }
                if capacity == Some(1) {
                    assert!(r.dropped() > 0);
                }
                if capacity.is_none() {
                    assert_eq!(r.queued(), 0);
                }
                assert!(!o.is_faulted());
            }
        }
    }
}
#[test]
fn every_shared_settings_publisher_rechecks_later_unrelated_state_without_partial_publication() {
    for k in KINDS {
        let (mut o, mut r, source) = ready();
        let exact = candidate_size(k);
        o.configure_semantic_root_budget(exact, &mut r).unwrap();
        let p = prepare(&o, k, &mut r).unwrap();
        let edit = o
            .prepare(
                &[DocumentEdits {
                    expected: source,
                    edits: vec![vellum_engine::source::Replacement {
                        start: 1,
                        end: 1,
                        expected: "".into(),
                        inserted: "B".into(),
                    }],
                }],
                &[],
                &mut r,
            )
            .unwrap();
        o.commit(edit, &mut r).unwrap();
        let before = o.semantic_root_usage(&mut r).unwrap();
        assert!(before.total_bytes <= exact);
        let basis = o.settings_basis(&mut r).unwrap();
        let generation = o.generation();
        assert_eq!(
            commit(&mut o, p, &mut r),
            Err(SettingsChangeErrorV2::Settings(SettingsError::Limit)),
            "{k:?}"
        );
        assert_eq!(o.settings_basis(&mut r).unwrap(), basis);
        assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
        assert_eq!(o.generation(), generation);
        assert!(!o.is_faulted());
    }
}
#[test]
fn retained_root_capacity_refusal_is_terminal_and_replay_does_not_rearm_it() {
    let (mut o, mut r, source) = ready();
    o.configure_semantic_root_budget(candidate_size(Kind::Value), &mut r)
        .unwrap();
    let id = o.issue_invocation(&mut r).unwrap();
    let q = [change(
        "left",
        1,
        SettingOperationV1::Set(SettingValueV1::Number(2.0)),
    )];
    o.submit_settings_change(&id, &q, &mut r).unwrap();
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: source,
                edits: vec![vellum_engine::source::Replacement {
                    start: 1,
                    end: 1,
                    expected: "".into(),
                    inserted: "B".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    let before = o.generation();
    assert_eq!(o.run(&id, &mut r), Ok(InvocationState::Finished));
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::SettingsRejected(SettingsError::Limit)
    ));
    assert_eq!(o.run(&id, &mut r), Ok(InvocationState::Finished));
    assert_eq!(
        o.submit_settings_change(&id, &q, &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), before);
    assert_eq!(read(&o, "A", &mut r), 1.0);
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
}
#[test]
fn whole_root_preflight_refusal_precedes_absorbing_scoped_basis_loss() {
    use super::scoped_basis::{context, facts, ready_with_root_budget};
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = ready_with_root_budget(8, 8, None);
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let c = context();
        let grants = [id("left")];
        let mut f = facts(&c, &grants);
        let bases = [o
            .capture_settings_write_basis(&id("left"), &c, &f, &mut r)
            .unwrap()];
        let bytes = o.semantic_root_usage(&mut r).unwrap().total_bytes;
        o.configure_semantic_root_budget(bytes, &mut r).unwrap();
        f.basis_allowed = Some(false);
        let q = [ScopedSettingChangeV1 {
            basis: 0,
            key: "unit".into(),
            operation: SettingOperationV1::Set(SettingValueV1::Number(2.0)),
        }];
        let reads = reads();
        let request = ScopedSettingsRequestV1 {
            bases: &bases,
            changes: &q,
            dependencies: &reads,
        };
        let g = TrustedSettingsMutationFactsV1 {
            basis: &f,
            host_basis_allowed: Some(true),
            required_reads: &[],
        };
        let before = o.generation();
        assert!(matches!(
            o.prepare_scoped_settings_change(&request, &c, &g, &mut r),
            Err(SettingsChangeErrorV2::Settings(SettingsError::Limit))
        ));
        f.basis_allowed = Some(true);
        assert_eq!(
            o.inspect_settings_write_basis(&bases[0], &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert_eq!(o.generation(), before);
        assert_eq!(o.settings_retention_usage().open_preparations, 0);
        assert_eq!(read(&o, "A", &mut r), 1.0);
    }
}

#[test]
fn declaration_root_preflight_does_not_absorb_a_basis_loss_on_a_refused_proposal() {
    use super::scoped_basis::{context, facts, ready_with_root_budget};
    let (mut o, mut r) = ready_with_root_budget(8, 8, None);
    let limits = SettingsDefinitionLimitsV1 {
        max_request_bytes: 65536,
        max_plan_bytes: 1_048_576,
        max_work: 1_048_576,
    };
    o.configure_settings_definitions(limits, &mut r).unwrap();
    let c = context();
    let common = facts(&c, &[]);
    let mut f = TrustedSettingsDefinitionBasisFactsV1 {
        common: &common,
        definition: limits,
        basis_allowed: Some(true),
    };
    let basis = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
    let bytes = o.semantic_root_usage(&mut r).unwrap().total_bytes;
    o.configure_semantic_root_budget(bytes, &mut r).unwrap();
    let d = declaration();
    let reads = reads();
    let request = ScopedSettingsDefinitionRequestV1 {
        basis: &basis,
        declaration: &d,
        dependencies: &reads,
    };
    f.basis_allowed = Some(false);
    let g = TrustedSettingsDefinitionMutationFactsV1 {
        basis: &f,
        host_basis_allowed: Some(true),
        schema_writable: Some(true),
        restrictions_writable: Some(true),
        associations_writable: Some(true),
        required_reads: &[],
    };
    let before = o.generation();
    assert!(matches!(
        o.prepare_scoped_settings_definition(&request, &c, &g, &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Limit))
    ));
    f.basis_allowed = Some(true);
    assert_eq!(
        o.inspect_settings_definition_basis(&basis, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert_eq!(o.generation(), before);
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
}
