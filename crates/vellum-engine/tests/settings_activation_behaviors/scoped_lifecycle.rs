use super::scoped_basis::{context, facts};
use super::*;
#[path = "scoped_lifecycle/arguments.rs"]
pub(super) mod arguments;
#[path = "scoped_lifecycle/selectors.rs"]
mod selectors;
#[test]
fn actual_whole_root_preflight_has_exact_boundaries_and_precedes_absorbing_basis_loss() {
    let (mut probe, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let b = probe
        .capture_settings_lifecycle_basis(&c, &f, &mut r)
        .unwrap();
    let q = Proposal::new();
    let grants = [new_id(), id("left")];
    let p = probe
        .prepare_scoped_settings_lifecycle(&q.request(&b), &c, &gate(&f, &grants), &mut r)
        .unwrap();
    probe
        .commit_scoped_settings_lifecycle(p, &c, &gate(&f, &grants), &mut r)
        .unwrap();
    let exact = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
    for delta in [-1isize, 0, 1] {
        let (mut o, mut r) = ready_with_budget(None);
        let mut f = basis(&common);
        let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        o.configure_semantic_root_budget(exact.checked_add_signed(delta).unwrap(), &mut r)
            .unwrap();
        if delta < 0 {
            f.basis_allowed = Some(false);
        }
        let before = o.generation();
        let p = o.prepare_scoped_settings_lifecycle(&q.request(&b), &c, &gate(&f, &grants), &mut r);
        if delta < 0 {
            assert!(matches!(
                p,
                Err(SettingsChangeErrorV2::Settings(SettingsError::Limit))
            ));
            assert_eq!(o.generation(), before);
            f.basis_allowed = Some(true);
            assert_eq!(
                o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
                SettingsBasisAvailabilityV1::Current
            );
        } else {
            o.commit_scoped_settings_lifecycle(p.unwrap(), &c, &gate(&f, &grants), &mut r)
                .unwrap();
            assert_eq!(o.semantic_root_usage(&mut r).unwrap().total_bytes, exact);
        }
    }
}
#[test]
fn mandatory_reads_reauthorize_original_dependencies_and_count_refusal_cannot_poison_a_basis() {
    for changed_count in [false, true] {
        let (mut o, mut r) = ready();
        let c = context();
        let common = facts(&c, &[]);
        let mut f = basis(&common);
        let source = o.create(b"A", &mut r).unwrap();
        let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        let q = Proposal::new();
        let grants = [new_id(), id("left")];
        let reads = [ReadDependencyV3::Base(ReadDependencyV2::Base(
            ReadDependencyV1::Source(source.clone()),
        ))];
        let mut g = gate(&f, &grants);
        if !changed_count {
            g.required_reads = &reads;
        }
        let p = o
            .prepare_scoped_settings_lifecycle(&q.request(&b), &c, &g, &mut r)
            .unwrap();
        if changed_count {
            f.basis_allowed = Some(false);
            let invalid = [ReadDependencyV3::ConfigurationState {
                registry: o.settings_registry(&mut r).unwrap(),
                expected: 0,
                interpretation: o.settings_basis(&mut r).unwrap().interpretation(),
            }];
            let mut g = gate(&f, &grants);
            g.required_reads = &invalid;
            assert_eq!(
                o.commit_scoped_settings_lifecycle(p, &c, &g, &mut r),
                Err(SettingsChangeErrorV2::Settings(SettingsError::Stale))
            );
            f.basis_allowed = Some(true);
        } else {
            let edit = o
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
            o.commit(edit, &mut r).unwrap();
            assert!(
                o.commit_scoped_settings_lifecycle(p, &c, &g, &mut r)
                    .is_err()
            );
        }
        assert_eq!(
            o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 1);
        assert!(!o.is_faulted());
    }
}
#[test]
fn scope_context_profile_binding_happens_before_consuming_a_job_or_absorbing_loss() {
    let (mut o, _) = ready();
    let mut r = native_service_recorder();
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let q = Proposal::new();
    let grants = [new_id(), id("left")];
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_lifecycle(&id, &q.request(&b), &c, &gate(&f, &grants), &mut r)
        .unwrap();
    let mut foreign = c.clone();
    foreign.recipient = "other".into();
    assert_eq!(
        o.run_scoped_settings_lifecycle(&id, &foreign, &gate(&f, &grants), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert_eq!(
        o.invocation_state(&id, &mut r),
        Ok(InvocationState::Pending)
    );
    o.run_scoped_settings_lifecycle(&id, &c, &gate(&f, &grants), &mut r)
        .unwrap();
    o.run_scoped_settings_lifecycle(&id, &c, &gate(&f, &grants), &mut r)
        .unwrap();
    use vellum_engine::instrumentation::{Boundary, Disposition};
    assert_native_service_records(
        &mut r,
        &[
            (Boundary::InvocationExecution, Disposition::Refused),
            (Boundary::Outcome, Disposition::Completed),
            (Boundary::InvocationExecution, Disposition::Completed),
            (Boundary::InvocationReplay, Disposition::Completed),
        ],
    );
}
#[test]
fn lifecycle_lowering_preparation_and_publication_are_optional_bounded_and_content_free() {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = ready();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let c = context();
        let common = facts(&c, &[]);
        let f = basis(&common);
        let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        let q = Proposal::new();
        let grants = [new_id(), id("left")];
        let p = o
            .prepare_scoped_settings_lifecycle(&q.request(&b), &c, &gate(&f, &grants), &mut r)
            .unwrap();
        assert!(!format!("{p:?}").contains("new"));
        o.commit_scoped_settings_lifecycle(p, &c, &gate(&f, &grants), &mut r)
            .unwrap();
        assert_eq!(read(&o, "A", &mut r), 3.0);
        if capacity == Some(128) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::ScopedLifecycleLowering,
                Boundary::ScopedLifecyclePreparation,
                Boundary::ScopedLifecycleValidation,
                Boundary::ScopedLifecyclePublication,
            ] {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == boundary && e.disposition == Disposition::Completed)
                );
            }
            assert!(events.iter().all(|e| e.service_time.is_none()));
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
        if capacity.is_none() {
            assert_eq!(r.queued(), 0);
        }
    }
}
pub(super) fn limits() -> SettingsLayerLimitsV1 {
    SettingsLayerLimitsV1 {
        max_changes: 16,
        max_created_value_bytes: 65536,
        max_request_bytes: 65536,
        max_plan_bytes: 1_048_576,
        max_work: 1_048_576,
    }
}
fn ready() -> (SourceCoordinator, Recorder) {
    ready_with_budget(Some(1_048_576))
}
fn ready_with_budget(ceiling: Option<usize>) -> (SourceCoordinator, Recorder) {
    let (mut o, mut r) = super::scoped_basis::ready_with_root_budget(16, 8, ceiling);
    o.configure_settings_layers(limits(), &mut r).unwrap();
    (o, r)
}
pub(super) fn basis<'a>(
    common: &'a TrustedSettingsBasisFactsV1<'a>,
) -> TrustedSettingsLifecycleBasisFactsV1<'a> {
    TrustedSettingsLifecycleBasisFactsV1 {
        common,
        lifecycle: limits(),
        basis_allowed: Some(true),
    }
}
pub(super) fn gate<'a>(
    basis: &'a TrustedSettingsLifecycleBasisFactsV1<'a>,
    grants: &'a [SettingIdentityV1],
) -> TrustedSettingsLifecycleMutationFactsV1<'a> {
    TrustedSettingsLifecycleMutationFactsV1 {
        basis,
        host_basis_allowed: Some(true),
        layer_registry_writable: Some(true),
        associations_writable: Some(true),
        writable_layers: Some(grants),
        required_reads: &[],
        selected_basis_facts: None,
    }
}
pub(super) fn new_id() -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "new".into(),
        incarnation: 2,
    }
}
fn creates() -> Vec<ScopedSettingLayerCreateV1> {
    vec![ScopedSettingLayerCreateV1 {
        id: "new".into(),
        scope: SettingScopeV1::Workspace,
        values: vec![(
            "unit".into(),
            SettingContributionV1::Set(SettingValueV1::Number(3.0)),
        )],
    }]
}
fn targets() -> Vec<ScopedSettingTargetV1> {
    vec![
        ScopedSettingTargetV1 {
            identity: id("A"),
            layers: vec![
                ScopedSettingLayerReferenceV1::Existing(id("host")),
                ScopedSettingLayerReferenceV1::Create { index: 0 },
            ],
        },
        ScopedSettingTargetV1 {
            identity: id("B"),
            layers: vec![
                ScopedSettingLayerReferenceV1::Existing(id("host")),
                ScopedSettingLayerReferenceV1::Existing(id("right")),
            ],
        },
    ]
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
struct Proposal {
    create: Vec<ScopedSettingLayerCreateV1>,
    remove: Vec<SettingIdentityV1>,
    targets: Vec<ScopedSettingTargetV1>,
    reads: ReadDependencyRequestV3,
}
impl Proposal {
    fn new() -> Self {
        Self {
            create: creates(),
            remove: vec![id("left")],
            targets: targets(),
            reads: reads(),
        }
    }
    fn request<'a>(
        &'a self,
        basis: &'a SettingsLifecycleBasisId,
    ) -> ScopedSettingsLifecycleRequestV1<'a> {
        ScopedSettingsLifecycleRequestV1 {
            basis,
            create: &self.create,
            remove: &self.remove,
            targets: &self.targets,
            dependencies: &self.reads,
        }
    }
}
fn unchanged() -> Vec<ScopedSettingTargetV1> {
    definition::declaration()
        .targets
        .into_iter()
        .map(|t| ScopedSettingTargetV1 {
            identity: t.identity,
            layers: t
                .layers
                .into_iter()
                .map(ScopedSettingLayerReferenceV1::Existing)
                .collect(),
        })
        .collect()
}
#[test]
fn scoped_lifecycle_derives_authority_incarnation_and_removal_version_without_unrelated_layer_grants()
 {
    let (mut o, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let grants = [new_id(), id("left")];
    let creates = creates();
    let remove = [id("left")];
    let targets = targets();
    let reads = reads();
    let request = ScopedSettingsLifecycleRequestV1 {
        basis: &b,
        create: &creates,
        remove: &remove,
        targets: &targets,
        dependencies: &reads,
    };
    let p = o
        .prepare_scoped_settings_lifecycle(&request, &c, &gate(&f, &grants), &mut r)
        .unwrap();
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 1);
    let before = o.generation();
    assert!(
        o.commit_scoped_settings_lifecycle(p, &c, &gate(&f, &grants), &mut r)
            .unwrap()
            .changed
    );
    assert_eq!(o.generation(), before + 1);
    assert_eq!(o.settings_layer_generation(&new_id(), &mut r), Ok(1));
    assert_eq!(
        o.settings_layer_generation(&id("left"), &mut r),
        Err(SettingsError::Unavailable)
    );
    assert_eq!(read(&o, "A", &mut r), 3.0);
}

#[test]
fn lifecycle_service_rechecks_each_changed_facet_and_full_native_identity_grant() {
    for field in 0..6 {
        let (mut o, mut r) = ready();
        let c = context();
        let common = facts(&c, &[]);
        let f = basis(&common);
        let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        let q = Proposal::new();
        let all = [new_id(), id("left")];
        let p = o
            .prepare_scoped_settings_lifecycle(&q.request(&b), &c, &gate(&f, &all), &mut r)
            .unwrap();
        let missing = match field {
            2 => vec![id("left")],
            3 => vec![new_id()],
            4 => vec![
                SettingIdentityV1 {
                    incarnation: 3,
                    ..new_id()
                },
                id("left"),
            ],
            _ => vec![new_id(), id("left")],
        };
        let mut revoked = gate(&f, &missing);
        if field == 0 {
            revoked.layer_registry_writable = None;
        }
        if field == 1 {
            revoked.associations_writable = Some(false);
        }
        if field == 5 {
            revoked.writable_layers = None;
        }
        let generation = o.generation();
        assert_eq!(
            o.commit_scoped_settings_lifecycle(p, &c, &revoked, &mut r),
            Err(SettingsChangeErrorV2::Settings(SettingsError::Unavailable))
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(read(&o, "A", &mut r), 1.0);
        assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 1);
        assert_eq!(
            o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert!(!o.is_faulted());
    }
}

#[test]
fn malformed_or_semantically_invalid_proposals_do_not_poison_an_unobserved_basis_loss() {
    for field in 0..4 {
        let (mut o, mut r) = ready();
        let c = context();
        let common = facts(&c, &[]);
        let mut f = basis(&common);
        let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        let mut q = Proposal::new();
        match field {
            0 => q.targets[0].layers[1] = ScopedSettingLayerReferenceV1::Create { index: 99 },
            1 => q.create[0].values[0].1 = SettingContributionV1::Set(SettingValueV1::Number(99.0)),
            2 => q.targets[0].layers[1] = ScopedSettingLayerReferenceV1::Existing(id("left")),
            _ => q.create[0].id.clear(),
        }
        f.basis_allowed = Some(false);
        let grants = [new_id(), id("left")];
        let before = o.generation();
        assert!(
            o.prepare_scoped_settings_lifecycle(&q.request(&b), &c, &gate(&f, &grants), &mut r)
                .is_err()
        );
        f.basis_allowed = Some(true);
        assert_eq!(
            o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert_eq!(o.generation(), before);
        assert_eq!(o.settings_retention_usage().open_preparations, 0);
    }
}

#[test]
fn valid_preflight_observes_role_permission_loss_absorbingly_without_allocating() {
    let (mut o, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let mut f = basis(&common);
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let q = Proposal::new();
    let grants = [new_id(), id("left")];
    f.basis_allowed = Some(false);
    assert!(
        o.prepare_scoped_settings_lifecycle(&q.request(&b), &c, &gate(&f, &grants), &mut r)
            .is_err()
    );
    f.basis_allowed = Some(true);
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Unavailable
    );
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 1);
}

#[test]
fn no_op_and_reassociation_only_require_no_unrelated_layer_registry_or_layer_write_grants() {
    for changed in [false, true] {
        let (mut o, mut r) = ready();
        let c = context();
        let mut common = facts(&c, &[]);
        common.basis_allowed = None;
        common.writable_layers = None;
        let f = basis(&common);
        let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        let mut targets = unchanged();
        if changed {
            targets[0].layers = vec![ScopedSettingLayerReferenceV1::Existing(id("host"))];
        }
        let reads = reads();
        let request = ScopedSettingsLifecycleRequestV1 {
            basis: &b,
            create: &[],
            remove: &[],
            targets: &targets,
            dependencies: &reads,
        };
        let mut g = gate(&f, &[]);
        g.layer_registry_writable = None;
        g.writable_layers = None;
        g.associations_writable = changed.then_some(true);
        let generation = o.generation();
        let p = o
            .prepare_scoped_settings_lifecycle(&request, &c, &g, &mut r)
            .unwrap();
        assert_eq!(
            o.commit_scoped_settings_lifecycle(p, &c, &g, &mut r)
                .unwrap()
                .changed,
            changed
        );
        assert_eq!(o.generation(), generation + u64::from(changed));
    }
}

#[test]
fn same_labels_get_distinct_contiguous_native_incarnations_and_do_not_alias_an_existing_layer() {
    let (mut o, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let mut q = Proposal::new();
    q.remove.clear();
    q.create[0].id = "left".into();
    q.create.push(ScopedSettingLayerCreateV1 {
        id: "left".into(),
        scope: SettingScopeV1::Workspace,
        values: vec![],
    });
    let grants = [
        SettingIdentityV1 {
            incarnation: 2,
            ..id("left")
        },
        SettingIdentityV1 {
            incarnation: 3,
            ..id("left")
        },
    ];
    let p = o
        .prepare_scoped_settings_lifecycle(&q.request(&b), &c, &gate(&f, &grants), &mut r)
        .unwrap();
    o.commit_scoped_settings_lifecycle(p, &c, &gate(&f, &grants), &mut r)
        .unwrap();
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 3);
    for identity in grants.iter().chain(std::iter::once(&id("left"))) {
        assert_eq!(o.settings_layer_generation(identity, &mut r), Ok(1));
    }
    assert_eq!(read(&o, "A", &mut r), 3.0);
}

#[test]
fn retained_lifecycle_uses_original_lowering_after_source_progress_and_replay_never_allocates_again()
 {
    let (mut o, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let source = o.create(b"A", &mut r).unwrap();
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let q = Proposal::new();
    let grants = [new_id(), id("left")];
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_lifecycle(&id, &q.request(&b), &c, &gate(&f, &grants), &mut r)
        .unwrap();
    assert_eq!(o.run(&id, &mut r), Err(InvocationError::Unavailable));
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
    let generation = o.generation();
    assert_eq!(
        o.run_scoped_settings_lifecycle(&id, &c, &gate(&f, &grants), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation + 1);
    let generation = o.generation();
    assert_eq!(
        o.submit_scoped_settings_lifecycle(&id, &q.request(&b), &c, &gate(&f, &grants), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        o.run_scoped_settings_lifecycle(&id, &c, &gate(&f, &grants), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 2);
    assert_eq!(o.read(&source.document, &mut r).unwrap().source.text(), "B");
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::ScopedLifecycleCommitted(ScopedSettingsUpdateV1 { changed: true })
    ));
}

#[test]
fn replay_binds_original_reference_forms_and_basis_identity_not_just_equivalent_native_results() {
    let (mut o, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let other = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let mut q = Proposal::new();
    let grants = [new_id(), id("left")];
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_lifecycle(&id, &q.request(&b), &c, &gate(&f, &grants), &mut r)
        .unwrap();
    assert_eq!(
        o.submit_scoped_settings_lifecycle(&id, &q.request(&other), &c, &gate(&f, &grants), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    q.targets[0].layers[1] = ScopedSettingLayerReferenceV1::Existing(new_id());
    assert_eq!(
        o.submit_scoped_settings_lifecycle(&id, &q.request(&b), &c, &gate(&f, &grants), &mut r),
        Err(InvocationError::IntentMismatch)
    );
    q.targets[0].layers[1] = ScopedSettingLayerReferenceV1::Create { index: 0 };
    o.run_scoped_settings_lifecycle(&id, &c, &gate(&f, &grants), &mut r)
        .unwrap();
}

#[test]
fn stopping_or_forgetting_a_lifecycle_job_does_not_retire_its_independent_basis_or_rearm_execution()
{
    let (mut o, mut r) = ready();
    let c = context();
    let common = facts(&c, &[]);
    let f = basis(&common);
    let b = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let q = Proposal::new();
    let grants = [new_id(), id("left")];
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_scoped_settings_lifecycle(&id, &q.request(&b), &c, &gate(&f, &grants), &mut r)
        .unwrap();
    assert_eq!(
        o.retire_invocation(&id, &mut r),
        Err(InvocationError::Pending)
    );
    o.stop_invocation(&id, &mut r).unwrap();
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    assert_eq!(o.settings_retention_usage().retained, 1);
    o.forget_invocation_outcome(&id, &mut r).unwrap();
    assert_eq!(
        o.run_scoped_settings_lifecycle(&id, &c, &gate(&f, &grants), &mut r),
        Ok(InvocationState::Finished)
    );
    assert_eq!(
        o.inspect_settings_lifecycle_basis(&b, &c, &f, &mut r),
        SettingsBasisAvailabilityV1::Current
    );
    assert_eq!(o.settings_layer_basis(&mut r).unwrap().layer_incarnation, 1);
    o.retire_invocation(&id, &mut r).unwrap();
    assert_eq!(o.settings_retention_usage().retained, 1);
}
