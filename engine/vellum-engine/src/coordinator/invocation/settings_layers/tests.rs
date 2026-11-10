use super::*;
use crate::settings::*;
fn id(n: u64) -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "target".into(),
        incarnation: n,
    }
}
fn setup() -> (
    SourceCoordinator,
    SettingsLayerChangeV1,
    ReadDependencyRequestV3,
    Recorder,
) {
    let (mut o, source, _, _, mut r) = read_dependency::configuration_read_test_fixture();
    o.configure_settings_layers(
        SettingsLayerLimitsV1 {
            max_changes: 8,
            max_created_value_bytes: 16384,
            max_request_bytes: 16384,
            max_plan_bytes: 65536,
            max_work: 65536,
        },
        &mut r,
    )
    .unwrap();
    let q = SettingsLayerChangeV1 {
        create: vec![SettingLayerCreateV1 {
            identity: id(2),
            scope: SettingScopeV1::Host,
            values: vec![(
                "flag".into(),
                SettingContributionV1::Set(SettingValueV1::Boolean(true)),
            )],
        }],
        remove: vec![],
        targets: vec![SettingTargetV1 {
            identity: id(1),
            layers: vec![id(1)],
        }],
    };
    let reads = ReadDependencyRequestV3 {
        predicates: vec![ReadDependencyV3::Base(ReadDependencyV2::Base(
            ReadDependencyV1::Source(source),
        ))],
        budget: ReadDependencyBudget {
            max_predicates: 8,
            max_work: 256,
        },
    };
    (o, q, reads, r)
}
#[test]
fn full_lifecycle_intent_and_plan_bounds_cover_headers_rows_values_and_actual_holds() {
    let (mut o, q, reads, mut r) = setup();
    let expected = o.settings_layer_basis(&mut r).unwrap();
    let c = LayerCapture::copy(
        &q,
        o.settings.as_ref().unwrap(),
        o.settings_layer_limits.unwrap(),
        65536,
    )
    .unwrap();
    let rc = read_dependency::ReadInput::V3(&reads).copy(&[]).unwrap();
    let size = std::mem::size_of::<LayerIntent>()
        - std::mem::size_of::<read_dependency::ReadCapture>()
        + c.bytes()
        + rc.logical_bytes().unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = size - 1;
    assert_eq!(
        o.submit_settings_layers(&ticket, expected, &q, &reads, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        o.invocation_state(&ticket, &mut r),
        Ok(InvocationState::Issued)
    );
    let p = o
        .prepare_settings_layers(expected, &q, &reads, &mut r)
        .unwrap();
    let bytes = p.retained_bytes().unwrap();
    drop(p);
    o.settings_layer_limits.as_mut().unwrap().max_plan_bytes = bytes - 1;
    assert!(matches!(
        o.prepare_settings_layers(expected, &q, &reads, &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Limit))
    ));
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    o.settings_layer_limits.as_mut().unwrap().max_plan_bytes = bytes;
    o.invocations.limits.max_intent_bytes = size;
    o.submit_settings_layers(&ticket, expected, &q, &reads, &mut r)
        .unwrap();
    assert_eq!(o.settings_retention_usage().open_preparations, 1);
    o.run(&ticket, &mut r).unwrap();
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
}
#[test]
fn lost_reads_or_substituted_original_layer_plan_fault_before_issuance_is_published() {
    for kind in 0..4 {
        let (mut o, mut q, reads, mut r) = setup();
        let e = o.settings_layer_basis(&mut r).unwrap();
        let ticket = o.issue_invocation(&mut r).unwrap();
        o.submit_settings_layers(&ticket, e, &q, &reads, &mut r)
            .unwrap();
        q.create[0].values.clear();
        let replacement = o.prepare_settings_layers(e, &q, &reads, &mut r).unwrap();
        let slot = o.invocations.slots[ticket.slot].as_mut().unwrap();
        let Phase::SettingsLayersPending(p) = &mut slot.phase else {
            unreachable!()
        };
        match kind {
            0 => p.as_mut().unwrap().reads.predicates.clear(),
            1 => p.as_mut().unwrap().read_count = 0,
            2 => *p = Ok(replacement),
            _ => *p = Err(SettingsChangeErrorV2::Settings(SettingsError::InvalidValue)),
        }
        let generation = o.generation();
        assert_eq!(o.run(&ticket, &mut r), Err(InvocationError::Faulted));
        assert!(o.is_faulted());
        assert_eq!(o.generation(), generation);
        assert!(matches!(
            o.invocations.slots[ticket.slot].as_ref().unwrap().phase,
            Phase::Running
        ));
    }
}
#[test]
fn interruption_after_actual_layer_installation_fences_unknown_and_does_not_reissue() {
    let (mut o, q, reads, mut r) = setup();
    let e = o.settings_layer_basis(&mut r).unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    o.submit_settings_layers(&ticket, e, &q, &reads, &mut r)
        .unwrap();
    let generation = o.generation();
    o.settings_publication_fault = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.run(&ticket, &mut r))).is_err()
    );
    assert_eq!(o.generation(), generation + 1);
    assert!(o.is_faulted());
    assert_eq!(
        o.settings.as_ref().unwrap().layer_basis().layer_incarnation,
        2
    );
    assert!(matches!(
        o.invocations.slots[ticket.slot].as_ref().unwrap().phase,
        Phase::Running
    ));
    assert_eq!(o.run(&ticket, &mut r), Err(InvocationError::Faulted));
}
#[test]
fn malformed_versions_leave_issued_and_valid_noop_uses_no_unused_publication_counter() {
    let (mut o, mut q, reads, mut r) = setup();
    let e = o.settings_layer_basis(&mut r).unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    for value in [0, 9_007_199_254_740_992, u64::MAX] {
        assert_eq!(
            o.submit_settings_layers(
                &ticket,
                SettingsLayerBasisV1 {
                    configuration_generation: value,
                    ..e
                },
                &q,
                &reads,
                &mut r
            ),
            Err(InvocationError::MalformedIntent)
        );
        assert_eq!(
            o.invocation_state(&ticket, &mut r),
            Ok(InvocationState::Issued)
        );
    }
    q.create.clear();
    o.generation = u64::MAX;
    o.submit_settings_layers(&ticket, e, &q, &reads, &mut r)
        .unwrap();
    o.run(&ticket, &mut r).unwrap();
    assert!(
        matches!(o.invocation_outcome(&ticket, &mut r), Ok(InvocationOutcome::SettingsLayersCommitted(result)) if !result.changed)
    );
    assert_eq!(o.generation(), u64::MAX);
    assert!(!o.is_faulted());
}
