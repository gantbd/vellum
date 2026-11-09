use super::*;
use crate::coordinator::scoped_definition::tests as def;
use crate::settings::*;
pub(in crate::coordinator) fn stop_requested(o: &SourceCoordinator, id: &InvocationId) -> bool {
    o.invocations.slots[id.slot]
        .as_ref()
        .unwrap()
        .stop_requested
}
fn id(name: &str, n: u64) -> SettingIdentityV1 {
    SettingIdentityV1 {
        authority: "engine".into(),
        id: name.into(),
        incarnation: n,
    }
}
pub(in crate::coordinator) fn control(
    c: &SettingsExecutionContextV1,
) -> TrustedInvocationControlV1<'_> {
    TrustedInvocationControlV1 {
        context: c,
        owner_control_allowed: Some(true),
        control_context_current: Some(true),
        control_scope_valid: Some(true),
        protected_control_ready: Some(true),
    }
}
pub(in crate::coordinator) fn queued(
    kind: usize,
) -> (
    SourceCoordinator,
    Recorder,
    SettingsExecutionContextV1,
    InvocationId,
) {
    queued_with_retention_headroom(kind, 0)
}
pub(in crate::coordinator) fn queued_with_retention_headroom(
    kind: usize,
    headroom: usize,
) -> (
    SourceCoordinator,
    Recorder,
    SettingsExecutionContextV1,
    InvocationId,
) {
    if kind == 3 {
        use crate::coordinator::serialization::tests as ser;
        let (mut o, mut r, d) = ser::setup();
        let c = ser::context();
        let f = ser::facts(&c, &d.document);
        let reads = ser::reads();
        let ticket = o.issue_invocation(&mut r).unwrap();
        o.submit_source_serialization(&ticket, &ser::request(&d.document, &reads), &c, &f, &mut r)
            .unwrap();
        return (o, r, c, ticket);
    }
    let (mut o, mut r) = def::setup_with_retention_headroom(headroom);
    let c = def::context();
    let mut common = def::common(&c);
    let grants = [id("target", 1), id("new", 2)];
    common.basis_allowed = Some(true);
    common.writable_layers = Some(&grants);
    let ticket = o.issue_invocation(&mut r).unwrap();
    let reads = def::reads();
    match kind {
        0 => {
            let bases = [o
                .capture_settings_write_basis(&grants[0], &c, &common, &mut r)
                .unwrap()];
            let changes = [ScopedSettingChangeV1 {
                basis: 0,
                key: "flag".into(),
                operation: SettingOperationV1::Set(SettingValueV1::Boolean(true)),
            }];
            o.submit_scoped_settings_change(
                &ticket,
                &ScopedSettingsRequestV1 {
                    bases: &bases,
                    changes: &changes,
                    dependencies: &reads,
                },
                &c,
                &TrustedSettingsMutationFactsV1 {
                    basis: &common,
                    host_basis_allowed: Some(true),
                    required_reads: &[],
                },
                &mut r,
            )
            .unwrap();
        }
        1 => {
            let f = def::basis(&common);
            let basis = o.capture_settings_definition_basis(&c, &f, &mut r).unwrap();
            let declaration = def::declaration(true);
            o.submit_scoped_settings_definition(
                &ticket,
                &ScopedSettingsDefinitionRequestV1 {
                    basis: &basis,
                    declaration: &declaration,
                    dependencies: &reads,
                },
                &c,
                &def::gate(&f),
                &mut r,
            )
            .unwrap();
        }
        2 => {
            let limits = SettingsLayerLimitsV1 {
                max_changes: 8,
                max_created_value_bytes: 16384,
                max_request_bytes: 16384,
                max_plan_bytes: 65536,
                max_work: 65536,
            };
            o.configure_settings_layers(limits, &mut r).unwrap();
            let f = TrustedSettingsLifecycleBasisFactsV1 {
                common: &common,
                lifecycle: limits,
                basis_allowed: Some(true),
            };
            let basis = o.capture_settings_lifecycle_basis(&c, &f, &mut r).unwrap();
            let create = [ScopedSettingLayerCreateV1 {
                id: "new".into(),
                scope: SettingScopeV1::Host,
                values: vec![],
            }];
            let targets = [ScopedSettingTargetV1 {
                identity: id("target", 1),
                layers: vec![
                    ScopedSettingLayerReferenceV1::Existing(id("target", 1)),
                    ScopedSettingLayerReferenceV1::Create { index: 0 },
                ],
            }];
            let gates = TrustedSettingsLifecycleMutationFactsV1 {
                basis: &f,
                host_basis_allowed: Some(true),
                layer_registry_writable: Some(true),
                associations_writable: Some(true),
                writable_layers: Some(&grants),
                required_reads: &[],
                selected_basis_facts: None,
            };
            o.submit_scoped_settings_lifecycle(
                &ticket,
                &ScopedSettingsLifecycleRequestV1 {
                    basis: &basis,
                    create: &create,
                    remove: &[],
                    targets: &targets,
                    dependencies: &reads,
                },
                &c,
                &gates,
                &mut r,
            )
            .unwrap();
        }
        _ => unreachable!(),
    }
    (o, r, c, ticket)
}
#[test]
fn every_current_context_bearing_job_kind_uses_the_same_intent_then_actual_settlement() {
    for kind in 0..4 {
        for generic in [false, true] {
            let (mut o, mut r, c, ticket) = queued(kind);
            let generation = o.generation();
            let basis_before = o.settings.as_ref().map(|s| s.basis());
            o.request_invocation_stop(&ticket, &control(&c), &mut r)
                .unwrap();
            assert!(
                o.invocations.slots[ticket.slot]
                    .as_ref()
                    .unwrap()
                    .stop_requested
            );
            assert_eq!(
                o.invocation_state(&ticket, &mut r),
                Ok(InvocationState::Pending)
            );
            let settled = if generic {
                o.run(&ticket, &mut r)
            } else {
                let common = def::common(&c);
                match kind {
                    0 => o.run_scoped_settings_change(
                        &ticket,
                        &c,
                        &TrustedSettingsMutationFactsV1 {
                            basis: &common,
                            host_basis_allowed: None,
                            required_reads: &[],
                        },
                        &mut r,
                    ),
                    1 => {
                        let f = def::basis(&common);
                        o.run_scoped_settings_definition(&ticket, &c, &def::gate(&f), &mut r)
                    }
                    2 => {
                        let f = TrustedSettingsLifecycleBasisFactsV1 {
                            common: &common,
                            lifecycle: o.settings_layer_limits.unwrap(),
                            basis_allowed: None,
                        };
                        o.run_scoped_settings_lifecycle(
                            &ticket,
                            &c,
                            &TrustedSettingsLifecycleMutationFactsV1 {
                                basis: &f,
                                host_basis_allowed: None,
                                layer_registry_writable: None,
                                associations_writable: None,
                                writable_layers: None,
                                required_reads: &[],
                                selected_basis_facts: None,
                            },
                            &mut r,
                        )
                    }
                    3 => {
                        let Some(BoundIntent::Serialization(original)) =
                            &o.invocations.slots[ticket.slot].as_ref().unwrap().intent
                        else {
                            unreachable!()
                        };
                        let document = original.document.clone();
                        let f = crate::coordinator::serialization::tests::facts(&c, &document);
                        o.run_source_serialization(&ticket, &c, &f, &mut r)
                    }
                    _ => unreachable!(),
                }
            };
            assert_eq!(settled, Ok(InvocationState::Finished));
            assert_eq!(o.generation(), generation);
            assert_eq!(o.settings.as_ref().map(|s| s.basis()), basis_before);
            assert!(matches!(
                o.invocation_outcome(&ticket, &mut r),
                Ok(InvocationOutcome::StoppedBeforePublication)
            ));
            assert_eq!(o.run(&ticket, &mut r), Ok(InvocationState::Finished));
        }
    }
}

#[test]
fn accepted_stop_cannot_fabricate_settlement_across_an_owner_fault_and_running_unknown_is_never_cancelled_cleanly()
 {
    let (mut o, mut r, c, ticket) = queued(0);
    o.request_invocation_stop(&ticket, &control(&c), &mut r)
        .unwrap();
    o.faulted = true;
    let generation = o.generation();
    let usage = o.settings_retention_usage();
    assert_eq!(o.run(&ticket, &mut r), Err(InvocationError::Faulted));
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.invocation_state(&ticket, &mut r),
        Ok(InvocationState::Pending)
    );
    assert_eq!(o.settings_retention_usage(), usage);
    assert!(o.invocation_outcome(&ticket, &mut r).is_err());
    let (mut o, mut r, c, ticket) = queued(0);
    let grants = [id("target", 1)];
    let mut common = def::common(&c);
    common.basis_allowed = Some(true);
    common.writable_layers = Some(&grants);
    o.settings_publication_fault = true;
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| o
            .run_scoped_settings_change(
                &ticket,
                &c,
                &TrustedSettingsMutationFactsV1 {
                    basis: &common,
                    host_basis_allowed: Some(true),
                    required_reads: &[]
                },
                &mut r
            )))
        .is_err()
    );
    let generation = o.generation();
    assert_eq!(
        o.request_invocation_stop(&ticket, &control(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    assert_eq!(o.generation(), generation);
    assert!(
        !o.invocations.slots[ticket.slot]
            .as_ref()
            .unwrap()
            .stop_requested
    );
    assert_eq!(o.invocation_usage().faulted_executions, 1);
    assert!(o.invocation_outcome(&ticket, &mut r).is_err());
}

#[test]
fn metadata_capture_substitution_is_not_laundered_into_a_clean_stopped_outcome() {
    let (mut o, mut r, c, ticket) = queued(0);
    let (foreign_owner, _, _, foreign_ticket) = queued(0);
    let Some(BoundIntent::ScopedSettings(foreign_capture)) = &foreign_owner.invocations.slots
        [foreign_ticket.slot]
        .as_ref()
        .unwrap()
        .intent
    else {
        unreachable!()
    };
    let slot = o.invocations.slots[ticket.slot].as_mut().unwrap();
    let Phase::ScopedSettingsPending(prepared) = &mut slot.phase else {
        unreachable!()
    };
    prepared.capture = foreign_capture.clone();
    // Replace the original request's owner witness, not authored source or a caller fact.
    let mut foreign = def::context();
    foreign.recipient = "foreign".into();
    assert_eq!(
        o.request_invocation_stop(&ticket, &control(&foreign), &mut r),
        Err(InvocationError::Unavailable)
    );
    assert!(!o.faulted);
    let generation = o.generation();
    assert_eq!(
        o.request_invocation_stop(&ticket, &control(&c), &mut r),
        Err(InvocationError::Faulted)
    );
    assert!(o.faulted);
    assert_eq!(o.generation(), generation);
    assert!(
        !o.invocations.slots[ticket.slot]
            .as_ref()
            .unwrap()
            .stop_requested
    );
    assert!(o.invocation_outcome(&ticket, &mut r).is_err());
}

#[test]
fn actual_service_rechecks_the_original_metadata_witness_after_accepting_stop_intent() {
    let (mut o, mut r, c, ticket) = queued(0);
    o.request_invocation_stop(&ticket, &control(&c), &mut r)
        .unwrap();
    let (foreign, _, _, foreign_ticket) = queued(0);
    let Some(BoundIntent::ScopedSettings(capture)) = &foreign.invocations.slots
        [foreign_ticket.slot]
        .as_ref()
        .unwrap()
        .intent
    else {
        unreachable!()
    };
    let Phase::ScopedSettingsPending(prepared) =
        &mut o.invocations.slots[ticket.slot].as_mut().unwrap().phase
    else {
        unreachable!()
    };
    prepared.capture = capture.clone();
    let generation = o.generation();
    assert_eq!(o.run(&ticket, &mut r), Err(InvocationError::Faulted));
    assert!(o.faulted);
    assert_eq!(o.generation(), generation);
    assert!(
        o.invocations.slots[ticket.slot]
            .as_ref()
            .unwrap()
            .stop_requested
    );
    assert!(o.invocation_outcome(&ticket, &mut r).is_err());
}
