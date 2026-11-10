use super::*;
use crate::settings::*;
fn id() -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: "target".into(),
        incarnation: 1,
    }
}
fn setup() -> (
    SourceCoordinator,
    SettingsDeclarationV1,
    ReadDependencyRequestV3,
    Recorder,
) {
    let (mut o, source, _, _, mut r) = read_dependency::configuration_read_test_fixture();
    o.configure_settings_definitions(
        SettingsDefinitionLimitsV1 {
            max_request_bytes: 16384,
            max_plan_bytes: 65536,
            max_work: 65536,
        },
        &mut r,
    )
    .unwrap();
    let d = SettingsDeclarationV1 {
        schemas: vec![SettingSchemaV1 {
            key: "flag".into(),
            value_type: SettingTypeV1::Boolean,
            default: SettingValueV1::Boolean(true),
            domain: SettingDomainV1::Analysis,
            scopes: vec![SettingScopeV1::Host],
            composition: SettingCompositionV1::Replace,
            sensitive: true,
        }],
        restrictions: vec![],
        targets: vec![SettingTargetV1 {
            identity: id(),
            layers: vec![id()],
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
    (o, d, reads, r)
}
fn expected(o: &SourceCoordinator, r: &mut Recorder) -> SettingsInterpretationEpochsV1 {
    o.settings_basis(r).unwrap().interpretation()
}
#[test]
fn original_intent_and_entire_plan_envelopes_are_bounded_without_leaked_holds() {
    let (mut o, d, reads, mut r) = setup();
    let e = expected(&o, &mut r);
    let capture = DefinitionCapture::from_state(
        &d,
        o.settings.as_ref().unwrap(),
        o.settings_definition_limits.unwrap(),
        65536,
    )
    .unwrap();
    let read_capture = read_dependency::ReadInput::V3(&reads).copy(&[]).unwrap();
    let size = std::mem::size_of::<DefinitionIntent>()
        - std::mem::size_of::<read_dependency::ReadCapture>()
        + capture.bytes()
        + read_capture.logical_bytes().unwrap();
    let ticket = o.issue_invocation(&mut r).unwrap();
    o.invocations.limits.max_intent_bytes = size - 1;
    assert_eq!(
        o.submit_settings_definition(&ticket, e, &d, &reads, &mut r),
        Err(InvocationError::Limit)
    );
    assert_eq!(
        o.invocation_state(&ticket, &mut r),
        Ok(InvocationState::Issued)
    );
    o.invocations.limits.max_intent_bytes = size;
    let p = o
        .prepare_settings_definition(e, &d, &reads, &mut r)
        .unwrap();
    let bytes = p.retained_bytes_for_test();
    drop(p);
    o.settings_definition_limits
        .as_mut()
        .unwrap()
        .max_plan_bytes = bytes - 1;
    assert!(matches!(
        o.prepare_settings_definition(e, &d, &reads, &mut r),
        Err(SettingsChangeErrorV2::Settings(SettingsError::Limit))
    ));
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    o.settings_definition_limits
        .as_mut()
        .unwrap()
        .max_plan_bytes = bytes;
    o.submit_settings_definition(&ticket, e, &d, &reads, &mut r)
        .unwrap();
    assert_eq!(o.settings_retention_usage().open_preparations, 1);
    o.run(&ticket, &mut r).unwrap();
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
}
#[test]
fn lost_reads_profile_count_or_substituted_original_plan_fault_before_installation() {
    for kind in 0..5 {
        let (mut o, mut d, reads, mut r) = setup();
        let e = expected(&o, &mut r);
        let ticket = o.issue_invocation(&mut r).unwrap();
        o.submit_settings_definition(&ticket, e, &d, &reads, &mut r)
            .unwrap();
        d.schemas[0].sensitive = false;
        let replacement = o
            .prepare_settings_definition(e, &d, &reads, &mut r)
            .unwrap();
        let slot = o.invocations.slots[ticket.slot].as_mut().unwrap();
        let Phase::SettingsDefinitionPending(p) = &mut slot.phase else {
            unreachable!()
        };
        match kind {
            0 => p.as_mut().unwrap().reads.predicates.clear(),
            1 => p.as_mut().unwrap().read_count = 0,
            2 => *p = Ok(replacement),
            3 => *p = Err(SettingsChangeErrorV2::Settings(SettingsError::InvalidValue)),
            _ => {
                p.as_mut().unwrap().reads = read_dependency::ReadInput::V1(&ReadDependencyRequest {
                    predicates: vec![],
                    budget: reads.budget,
                })
                .copy(&[])
                .unwrap()
            }
        }
        let generation = o.generation();
        assert_eq!(o.run(&ticket, &mut r), Err(InvocationError::Faulted));
        assert_eq!(o.generation(), generation);
        assert!(o.is_faulted());
        assert!(matches!(
            o.invocations.slots[ticket.slot].as_ref().unwrap().phase,
            Phase::Running
        ));
    }
}
#[test]
fn interrupted_actual_installation_is_unknown_not_a_known_refusal_or_retry() {
    let (mut o, d, reads, mut r) = setup();
    let e = expected(&o, &mut r);
    let ticket = o.issue_invocation(&mut r).unwrap();
    o.submit_settings_definition(&ticket, e, &d, &reads, &mut r)
        .unwrap();
    let generation = o.generation();
    o.settings_publication_fault = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o.run(&ticket, &mut r))).is_err()
    );
    assert_eq!(o.generation(), generation + 1);
    assert!(o.is_faulted());
    assert!(matches!(
        o.invocations.slots[ticket.slot].as_ref().unwrap().phase,
        Phase::Running
    ));
    assert_eq!(o.settings_retention_usage().open_preparations, 0);
    assert_eq!(o.run(&ticket, &mut r), Err(InvocationError::Faulted));
}
#[test]
fn declaration_retry_compares_number_bits_even_when_activation_uses_math_equality() {
    let (mut o, mut d, reads, mut r) = setup();
    let e = expected(&o, &mut r);
    d.schemas[0].value_type = SettingTypeV1::Integer { min: -1, max: 1 };
    d.schemas[0].default = SettingValueV1::Number(-0.0);
    let ticket = o.issue_invocation(&mut r).unwrap();
    o.submit_settings_definition(&ticket, e, &d, &reads, &mut r)
        .unwrap();
    d.schemas[0].default = SettingValueV1::Number(0.0);
    assert_eq!(
        o.submit_settings_definition(&ticket, e, &d, &reads, &mut r),
        Err(InvocationError::IntentMismatch)
    );
}
#[test]
fn malformed_expected_epochs_leave_issued_and_noop_needs_no_publication_headroom() {
    let (mut o, mut d, reads, mut r) = setup();
    let e = expected(&o, &mut r);
    let ticket = o.issue_invocation(&mut r).unwrap();
    for value in [0, 9_007_199_254_740_992, u64::MAX] {
        for field in 0..3 {
            let mut bad = e;
            match field {
                0 => bad.schema = value,
                1 => bad.restriction = value,
                _ => bad.association = value,
            }
            assert_eq!(
                o.submit_settings_definition(&ticket, bad, &d, &reads, &mut r),
                Err(InvocationError::MalformedIntent)
            );
            assert_eq!(
                o.invocation_state(&ticket, &mut r),
                Ok(InvocationState::Issued)
            );
        }
    }
    d.schemas[0].default = SettingValueV1::Boolean(false);
    o.generation = u64::MAX;
    o.submit_settings_definition(&ticket, e, &d, &reads, &mut r)
        .unwrap();
    o.run(&ticket, &mut r).unwrap();
    assert!(
        matches!(o.invocation_outcome(&ticket, &mut r), Ok(InvocationOutcome::SettingsDefinitionCommitted(result)) if !result.changed)
    );
    assert_eq!(o.generation(), u64::MAX);
    assert!(!o.is_faulted());
}
