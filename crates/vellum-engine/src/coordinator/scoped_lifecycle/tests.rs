use super::*;
use crate::coordinator::scoped_definition::tests::{common, context, control, setup};
mod selectors;
fn limits() -> SettingsLayerLimitsV1 {
    SettingsLayerLimitsV1 {
        max_changes: 8,
        max_created_value_bytes: 2048,
        max_request_bytes: 16384,
        max_plan_bytes: 65536,
        max_work: 65536,
    }
}
fn ready() -> (SourceCoordinator, Recorder) {
    let (mut o, mut r) = setup();
    o.configure_settings_layers(limits(), &mut r).unwrap();
    (o, r)
}
fn id(name: &str, incarnation: u64) -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: name.into(),
        incarnation,
    }
}
fn role<'a>(
    common: &'a TrustedSettingsBasisFactsV1<'a>,
) -> TrustedSettingsLifecycleBasisFactsV1<'a> {
    TrustedSettingsLifecycleBasisFactsV1 {
        common,
        lifecycle: limits(),
        basis_allowed: Some(true),
    }
}
fn gate<'a>(
    f: &'a TrustedSettingsLifecycleBasisFactsV1<'a>,
    grants: &'a [SettingIdentityV1],
) -> TrustedSettingsLifecycleMutationFactsV1<'a> {
    TrustedSettingsLifecycleMutationFactsV1 {
        basis: f,
        host_basis_allowed: Some(true),
        layer_registry_writable: Some(true),
        associations_writable: Some(true),
        writable_layers: Some(grants),
        required_reads: &[],
        selected_basis_facts: None,
    }
}
struct Proposal {
    create: Vec<ScopedSettingLayerCreateV1>,
    targets: Vec<ScopedSettingTargetV1>,
    reads: ReadDependencyRequestV3,
}
impl Proposal {
    fn new(create: bool) -> Self {
        Self {
            create: if create {
                vec![ScopedSettingLayerCreateV1 {
                    id: "new".into(),
                    scope: SettingScopeV1::Host,
                    values: vec![],
                }]
            } else {
                vec![]
            },
            targets: vec![ScopedSettingTargetV1 {
                identity: id("target", 1),
                layers: if create {
                    vec![
                        ScopedSettingLayerReferenceV1::Existing(id("target", 1)),
                        ScopedSettingLayerReferenceV1::Create { index: 0 },
                    ]
                } else {
                    vec![ScopedSettingLayerReferenceV1::Existing(id("target", 1))]
                },
            }],
            reads: ReadDependencyRequestV3 {
                predicates: vec![],
                budget: ReadDependencyBudget {
                    max_predicates: 8,
                    max_work: 4096,
                },
            },
        }
    }
    fn request<'a>(
        &'a self,
        b: &'a SettingsLifecycleBasisId,
    ) -> ScopedSettingsLifecycleRequestV1<'a> {
        ScopedSettingsLifecycleRequestV1 {
            basis: b,
            create: &self.create,
            remove: &[],
            targets: &self.targets,
            dependencies: &self.reads,
        }
    }
}
#[test]
fn source_progress_preserves_real_history_journals_across_lifecycle_publication_and_undo() {
    let (mut o, mut r) = ready();
    o.history = history::HistoryState::new(HistoryLimits {
        max_retained_groups: 16,
        max_journal_bytes: 65536,
        max_pins: 8,
    });
    let source = o
        .read(
            &DocumentId {
                namespace: o.namespace.clone(),
                slot: 0,
            },
            &mut r,
        )
        .unwrap()
        .version;
    let c = context();
    let common = common(&c);
    let f = role(&common);
    let grants = [id("new", 2)];
    let basis = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let q = Proposal::new(true);
    let p = o
        .prepare_scoped_settings_lifecycle(&q.request(&basis), &c, &gate(&f, &grants), &mut r)
        .unwrap();
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: source.clone(),
                edits: vec![Replacement {
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
    let group = o.commit(edit, &mut r).unwrap().history_group.unwrap();
    let history = o.history_usage();
    o.commit_scoped_settings_lifecycle(p, &c, &gate(&f, &grants), &mut r)
        .unwrap();
    assert_eq!(o.history_usage(), history);
    let undo = o
        .prepare_reversal(&group, HistoryDirection::Undo, &mut r)
        .unwrap();
    o.commit(undo, &mut r).unwrap();
    assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "A");
    assert_eq!(
        o.settings.as_ref().unwrap().layer_generation(&id("new", 2)),
        Ok(1)
    );
}
#[test]
fn substituted_private_capture_faults_and_protected_basis_retirement_still_works() {
    let (mut o, mut r) = ready();
    let c = context();
    let common = common(&c);
    let f = role(&common);
    let grants = [id("new", 2)];
    let basis = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let q = Proposal::new(true);
    let mut p = o
        .prepare_scoped_settings_lifecycle(&q.request(&basis), &c, &gate(&f, &grants), &mut r)
        .unwrap();
    let other = o
        .prepare_scoped_settings_lifecycle(&q.request(&basis), &c, &gate(&f, &grants), &mut r)
        .unwrap();
    p.capture = other.capture.clone();
    drop(other);
    let generation = o.generation();
    assert_eq!(
        o.commit_scoped_settings_lifecycle(p, &c, &gate(&f, &grants), &mut r),
        Err(s(SettingsError::Faulted))
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation(), generation);
    assert!(o.retire_settings_lifecycle_basis(&basis, &c, &control(&c), &mut r));
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
}
#[test]
fn wrapper_budget_preflight_is_exact_and_cannot_poison_a_low_budget_basis() {
    let (mut probe, mut r) = ready();
    let c = context();
    let common = common(&c);
    let f = role(&common);
    let grants = [id("new", 2)];
    let basis = probe
        .capture_settings_lifecycle_basis(&c, &f, &mut r)
        .unwrap();
    let q = Proposal::new(true);
    let p = probe
        .prepare_scoped_settings_lifecycle(&q.request(&basis), &c, &gate(&f, &grants), &mut r)
        .unwrap();
    let bytes = p.retained_bytes().unwrap();
    drop(p);
    for delta in [-1isize, 0, 1] {
        let (mut o, mut r) = ready();
        let mut f = role(&common);
        let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        o.set_settings_parent_limit_for_test(bytes.checked_add_signed(delta).unwrap());
        if delta < 0 {
            f.basis_allowed = Some(false);
        }
        let p = o.prepare_scoped_settings_lifecycle(&q.request(&b), &c, &gate(&f, &grants), &mut r);
        if delta < 0 {
            assert!(matches!(
                p,
                Err(SettingsChangeErrorV2::Settings(SettingsError::Limit))
            ));
            assert_eq!(o.settings_retention_usage().open_preparations, 0);
        } else {
            assert_eq!(p.unwrap().retained_bytes().unwrap(), bytes);
        }
        f.basis_allowed = Some(true);
        assert_eq!(
            o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
    }
}
#[test]
fn complete_identity_needs_no_unused_publication_or_configuration_or_allocator_counter() {
    let (mut o, mut r) = ready();
    let mut root = o.settings.as_ref().unwrap().flat_clone_for_test().unwrap();
    root.set_layer_basis_for_test(SettingsLayerBasisV1 {
        configuration_generation: 9_007_199_254_740_991,
        layer_incarnation: 9_007_199_254_740_991,
    });
    o.settings = Some(Arc::new(root));
    o.generation = u64::MAX;
    let c = context();
    let common = common(&c);
    let f = role(&common);
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let q = Proposal::new(false);
    let mut g = gate(&f, &[]);
    g.layer_registry_writable = None;
    g.associations_writable = None;
    g.writable_layers = None;
    let p = o
        .prepare_scoped_settings_lifecycle(&q.request(&b), &c, &g, &mut r)
        .unwrap();
    assert!(
        !o.commit_scoped_settings_lifecycle(p, &c, &g, &mut r)
            .unwrap()
            .changed
    );
    assert_eq!(o.generation(), u64::MAX);
}

#[test]
fn invalid_owned_issuer_state_is_an_internal_fault_not_a_request_validation_or_staleness_refusal() {
    let (mut o, mut r) = ready();
    let c = context();
    let common = common(&c);
    let f = role(&common);
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let mut root = o.settings.as_ref().unwrap().flat_clone_for_test().unwrap();
    root.set_layer_basis_for_test(SettingsLayerBasisV1 {
        configuration_generation: 0,
        layer_incarnation: 1,
    });
    o.settings = Some(Arc::new(root));
    let q = Proposal::new(true);
    let grants = [id("new", 2)];
    let generation = o.generation();
    assert!(matches!(
        o.prepare_scoped_settings_lifecycle(&q.request(&b), &c, &gate(&f, &grants), &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Faulted))
    ));
    assert!(o.is_faulted());
    assert_eq!(o.generation(), generation);
    assert!(o.retire_settings_lifecycle_basis(&b, &c, &control(&c), &mut r));
}
#[test]
fn unknown_after_actual_installation_is_not_a_fabricated_rejection_or_second_allocation() {
    let (mut o, mut r) = ready();
    let c = context();
    let common = common(&c);
    let f = role(&common);
    let grants = [id("new", 2)];
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let q = Proposal::new(true);
    let ticket = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_lifecycle(&ticket, &q.request(&b), &c, &gate(&f, &grants), &mut r)
        .unwrap();
    let generation = o.generation();
    o.settings_publication_fault = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || o.run_scoped_settings_lifecycle(&ticket, &c, &gate(&f, &grants), &mut r)
        ))
        .is_err()
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation(), generation + 1);
    assert_eq!(o.invocation_usage().faulted_executions, 1);
    assert_eq!(
        o.settings.as_ref().unwrap().layer_basis().layer_incarnation,
        2
    );
    assert_eq!(
        o.run_scoped_settings_lifecycle(&ticket, &c, &gate(&f, &grants), &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(o.retire_settings_lifecycle_basis(&b, &c, &control(&c), &mut r));
}
