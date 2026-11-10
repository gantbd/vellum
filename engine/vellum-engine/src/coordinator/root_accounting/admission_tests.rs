use super::*;
use crate::settings::*;

fn owner() -> (SourceCoordinator, Recorder) {
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 8,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 4096,
            max_edits: 8,
            max_edit_payload_bytes: 4096,
        },
    });
    let mut r = Recorder::disabled();
    o.install_initial_settings(settings::settings_test_fixture(&mut r), &mut r)
        .unwrap();
    o.configure_settings_activation(
        SettingsActivationLimitsV1 {
            max_changes: 8,
            max_request_nodes: 64,
            max_request_depth: 32,
            max_request_bytes: 16384,
            max_plan_bytes: 65536,
            max_work: 65536,
        },
        &mut r,
    )
    .unwrap();
    (o, r)
}
fn change() -> SettingChangeV1 {
    SettingChangeV1 {
        layer: SettingIdentityV1 {
            authority: "engine".into(),
            id: "target".into(),
            incarnation: 1,
        },
        expected_generation: 1,
        key: "flag".into(),
        operation: SettingOperationV1::Set(SettingValueV1::Boolean(true)),
    }
}
#[test]
fn settings_plan_requires_explicit_whole_root_budget_without_issuing_or_publishing() {
    let (o, mut r) = owner();
    let before = o.generation();
    assert!(matches!(
        o.prepare_settings_change(&[change()], &mut r),
        Err(SettingsError::Unavailable)
    ));
    assert_eq!(o.generation(), before);
    assert_eq!(o.semantic_root_byte_ceiling(), None);
}
#[test]
fn ceiling_configuration_is_exact_immutable_and_failure_has_no_prefix() {
    let (mut o, mut r) = owner();
    let bytes = o.semantic_root_usage(&mut r).unwrap().total_bytes;
    assert_eq!(
        o.configure_semantic_root_budget(bytes - 1, &mut r),
        Err(RootAccountingErrorV1::Limit)
    );
    assert_eq!(o.semantic_root_byte_ceiling(), None);
    let generation = o.generation();
    assert_eq!(o.configure_semantic_root_budget(bytes, &mut r), Ok(true));
    assert_eq!(o.configure_semantic_root_budget(bytes, &mut r), Ok(false));
    assert_eq!(
        o.configure_semantic_root_budget(bytes + 1, &mut r),
        Err(RootAccountingErrorV1::Unavailable)
    );
    assert_eq!(o.generation(), generation);
}
#[test]
fn whole_candidate_not_just_configuration_is_checked_before_plan_retention() {
    let (mut probe, mut r) = owner();
    probe
        .configure_semantic_root_budget(1_048_576, &mut r)
        .unwrap();
    let p = probe.prepare_settings_change(&[change()], &mut r).unwrap();
    probe.commit_settings_change(p, &mut r).unwrap();
    let exact = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
    for delta in [-1isize, 0, 1] {
        let (mut o, mut r) = owner();
        o.configure_semantic_root_budget(exact.checked_add_signed(delta).unwrap(), &mut r)
            .unwrap();
        let before = o.generation();
        let p = o.prepare_settings_change(&[change()], &mut r);
        if delta < 0 {
            assert!(matches!(p, Err(SettingsError::Limit)));
            assert_eq!(o.generation(), before);
        } else {
            o.commit_settings_change(p.unwrap(), &mut r).unwrap();
            assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, exact);
        }
    }
}
#[test]
fn later_unrelated_source_growth_is_remeasured_and_refuses_without_partial_settings_publication() {
    let (mut probe, mut r) = owner();
    probe.create(b"A", &mut r).unwrap();
    probe
        .configure_semantic_root_budget(1_048_576, &mut r)
        .unwrap();
    let p = probe.prepare_settings_change(&[change()], &mut r).unwrap();
    probe.commit_settings_change(p, &mut r).unwrap();
    let exact = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
    let (mut o, mut r) = owner();
    let source = o.create(b"A", &mut r).unwrap();
    o.configure_semantic_root_budget(exact, &mut r).unwrap();
    let p = o.prepare_settings_change(&[change()], &mut r).unwrap();
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: source,
                edits: vec![Replacement {
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
        o.commit_settings_change(p, &mut r),
        Err(SettingsError::Limit)
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.settings_basis(&mut r).unwrap(), basis);
    assert_eq!(o.semantic_root_usage(&mut r).unwrap(), before);
    assert!(!o.is_faulted());
}

#[test]
fn exact_boundary_noop_needs_no_unused_publication_counter() {
    let (mut o, mut r) = owner();
    let exact = o.semantic_root_usage(&mut r).unwrap().total_bytes;
    o.configure_semantic_root_budget(exact, &mut r).unwrap();
    o.generation = u64::MAX;
    let p = o.prepare_settings_change(&[], &mut r).unwrap();
    assert!(!o.commit_settings_change(p, &mut r).unwrap().changed);
    assert_eq!(o.generation(), u64::MAX);
}

#[test]
fn initial_installation_honors_an_already_configured_budget_atomically() {
    let (template, mut r) = owner();
    let mut o = SourceCoordinator::new(template.limits);
    let exact = o.semantic_root_usage(&mut r).unwrap().total_bytes;
    o.configure_semantic_root_budget(exact, &mut r).unwrap();
    assert_eq!(
        o.install_initial_settings(settings::settings_test_fixture(&mut r), &mut r),
        Err(SettingsError::Limit)
    );
    assert!(o.settings_usage().is_none());
    assert_eq!(o.generation(), 0);
    assert!(!o.is_faulted());
}

#[test]
fn issued_growth_checked_addition_cannot_wrap_under_the_largest_finite_ceiling() {
    let (mut o, _) = owner();
    let mut r = Recorder::try_with_capacity(8).unwrap();
    o.configure_semantic_root_budget(usize::MAX, &mut r)
        .unwrap();
    let generation = o.generation();
    assert_eq!(
        o.check_root_growth(usize::MAX, &mut r),
        Err(CoordinatorError::Limit)
    );
    assert_eq!(o.generation(), generation);
    assert!(!o.is_faulted());
    assert!(
        std::iter::from_fn(|| r.pop_front()).any(|e| e.boundary == Boundary::SemanticRootAdmission
            && e.disposition == Disposition::Refused)
    );
}

#[test]
fn compound_identity_at_the_exact_ceiling_needs_no_unused_publication_counter() {
    let (mut o, mut r) = owner();
    let bytes = o.semantic_root_usage(&mut r).unwrap().total_bytes;
    o.configure_semantic_root_budget(bytes, &mut r).unwrap();
    o.generation = u64::MAX;
    let p = o.prepare(&[], &[], &mut r).unwrap();
    let outcome = o.commit(p, &mut r).unwrap();
    assert_eq!(outcome.generation, u64::MAX);
    assert!(outcome.operation.is_none() && outcome.changes.is_empty());
    assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, bytes);
    assert!(!o.is_faulted());
}

#[test]
fn replacement_accounting_cannot_subtract_unowned_bytes_or_mask_the_fault_as_capacity() {
    let (mut o, _) = owner();
    let mut r = Recorder::try_with_capacity(8).unwrap();
    let bytes = o.semantic_root_usage(&mut r).unwrap().total_bytes;
    o.configure_semantic_root_budget(bytes, &mut r).unwrap();
    assert_eq!(
        o.check_root_replacement(bytes + 1, 0, &mut r),
        Err(CoordinatorError::Faulted)
    );
    assert!(
        std::iter::from_fn(|| r.pop_front()).any(|e| e.boundary == Boundary::SemanticRootAdmission
            && e.disposition == Disposition::Unknown)
    );
    assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, bytes);
}
