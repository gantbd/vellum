use super::*;

fn targets() -> Vec<BasisSettingTargetV1> {
    vec![BasisSettingTargetV1 {
        identity: id("target", 1),
        layers: vec![
            BasisSettingLayerReferenceV1::Basis { index: 0 },
            BasisSettingLayerReferenceV1::Create { index: 0 },
        ],
    }]
}

#[test]
fn full_selector_wrapper_boundary_precedes_either_basis_permission_loss() {
    let c = context();
    let grant = [id("target", 1), id("new", 2)];
    let mut common = common(&c);
    common.basis_allowed = Some(true);
    common.writable_layers = Some(&grant);
    let f = role(&common);
    let q = Proposal::new(true);
    let targets = targets();
    let (mut probe, mut r) = ready();
    let lifecycle = probe
        .capture_settings_lifecycle_basis(&c, &f, &mut r)
        .unwrap();
    let bases = [probe
        .capture_settings_write_basis(&id("target", 1), &c, &common, &mut r)
        .unwrap()];
    let request = BasisSettingsLifecycleRequestV1 {
        lifecycle: &lifecycle,
        bases: &bases,
        create: &q.create,
        remove: &[],
        targets: &targets,
        dependencies: &q.reads,
    };
    let mut facts = gate(&f, &grant);
    facts.selected_basis_facts = Some(&common);
    let plan = probe
        .prepare_basis_settings_lifecycle(&request, &c, &facts, &mut r)
        .unwrap();
    let bytes = plan.retained_bytes().unwrap();
    drop(plan);
    for delta in [-1isize, 0, 1] {
        let (mut o, mut r) = ready();
        let mut f = role(&common);
        let lifecycle = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
        let bases = [o
            .capture_settings_write_basis(&id("target", 1), &c, &common, &mut r)
            .unwrap()];
        let request = BasisSettingsLifecycleRequestV1 {
            lifecycle: &lifecycle,
            bases: &bases,
            create: &q.create,
            remove: &[],
            targets: &targets,
            dependencies: &q.reads,
        };
        o.set_settings_parent_limit_for_test(bytes.checked_add_signed(delta).unwrap());
        let mut lost = super::common(&c);
        lost.basis_allowed = Some(false);
        lost.writable_layers = Some(&grant);
        if delta < 0 {
            f.basis_allowed = Some(false);
        }
        let mut facts = gate(&f, &grant);
        facts.selected_basis_facts = Some(if delta < 0 { &lost } else { &common });
        let plan = o.prepare_basis_settings_lifecycle(&request, &c, &facts, &mut r);
        if delta < 0 {
            assert!(matches!(
                plan,
                Err(SettingsChangeErrorV2::Settings(SettingsError::Limit))
            ));
            assert_eq!(o.settings_retention_usage().open_preparations, 0);
        } else {
            assert_eq!(plan.unwrap().retained_bytes().unwrap(), bytes);
        }
        f.basis_allowed = Some(true);
        assert_eq!(
            o.inspect_settings_lifecycle_basis(&lifecycle, &c, &f, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
        assert_eq!(
            o.inspect_settings_write_basis(&bases[0], &c, &common, &mut r),
            SettingsBasisAvailabilityV1::Current
        );
    }
}

#[test]
fn actual_post_install_fault_uses_the_same_unknown_fence_and_protected_individual_cleanup() {
    let (mut o, mut r) = ready();
    let c = context();
    let grant = [id("target", 1), id("new", 2)];
    let mut common = common(&c);
    common.basis_allowed = Some(true);
    common.writable_layers = Some(&grant);
    let f = role(&common);
    let lifecycle = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
    let bases = [o
        .capture_settings_write_basis(&id("target", 1), &c, &common, &mut r)
        .unwrap()];
    let q = Proposal::new(true);
    let targets = targets();
    let request = BasisSettingsLifecycleRequestV1 {
        lifecycle: &lifecycle,
        bases: &bases,
        create: &q.create,
        remove: &[],
        targets: &targets,
        dependencies: &q.reads,
    };
    let mut facts = gate(&f, &grant);
    facts.selected_basis_facts = Some(&common);
    let ticket = o.issue_invocation(&mut r).unwrap();
    o.submit_basis_settings_lifecycle(&ticket, &request, &c, &facts, &mut r)
        .unwrap();
    let generation = o.generation();
    o.settings_publication_fault = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || o.run_scoped_settings_lifecycle(&ticket, &c, &facts, &mut r)
        ))
        .is_err()
    );
    assert!(o.is_faulted());
    assert_eq!(o.generation(), generation + 1);
    assert_eq!(o.invocation_usage().faulted_executions, 1);
    assert_eq!(
        o.run_scoped_settings_lifecycle(&ticket, &c, &facts, &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(o.retire_settings_write_basis(&bases[0], &c, &control(&c), &mut r));
    assert!(o.retire_settings_lifecycle_basis(&lifecycle, &c, &control(&c), &mut r));
    assert_eq!(o.settings_retention_usage().retained, 0);
}
