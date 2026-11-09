use super::super::super::tests::{change, facts, open};
use super::super::tests::{admission, prepare, setup};
use super::*;

fn envelope<T>(
    typed: bool,
    operation: &ResourceRegistryCommandOperationV1,
    command: &ResourceRegistryCommandIdV1,
    preparation: Option<&ResourceRegistryPreparationIdV1>,
    change: usize,
    call: impl FnOnce(&Value<'_, '_>) -> T,
) -> T {
    let resync = matches!(
        operation,
        ResourceRegistryCommandOperationV1::Resynchronize { .. }
    );
    let name = if resync {
        "authorized-resynchronization"
    } else {
        "establish-bounded-stream"
    };
    let mut args = vec![
        (
            "contract",
            Value::Data(InertValueV1::Text(RESOURCE_REGISTRY_COMMAND_PROFILE_V1)),
        ),
        ("operation", Value::Data(InertValueV1::Text(name))),
        ("command", Value::Command(command)),
        (
            "stream",
            match operation {
                ResourceRegistryCommandOperationV1::Establish => Value::Data(InertValueV1::Null),
                ResourceRegistryCommandOperationV1::Resynchronize { stream } => {
                    Value::Stream(stream)
                }
            },
        ),
        (
            "preparation",
            preparation.map_or(Value::Data(InertValueV1::Null), Value::Preparation),
        ),
    ];
    match change {
        1 => args.push(("current", Value::Data(InertValueV1::Boolean(true)))),
        2 => args[1].0 = "contract",
        3 => args[0].1 = Value::Data(InertValueV1::Text("read-context/1")),
        4 => args[1].1 = Value::Data(InertValueV1::Text("stop-observation")),
        5 => args[2].1 = Value::Data(InertValueV1::Object(&[])),
        6 => args[4].1 = Value::Command(command),
        7 if resync => args[3].1 = Value::Data(InertValueV1::Null),
        7 => args[3].1 = Value::Command(command),
        _ => {}
    }
    let header = [
        (
            if typed { "family" } else { "name" },
            if typed {
                Value::Data(InertValueV1::Number(if change == 8 { 5.0 } else { 18.0 }))
            } else {
                Value::Data(InertValueV1::Text(if change == 8 {
                    "inspect-execution"
                } else {
                    "control-observation"
                }))
            },
        ),
        (
            "variant",
            Value::Data(InertValueV1::Text(if change == 9 {
                "surprise"
            } else {
                name
            })),
        ),
        (
            "version",
            Value::Data(InertValueV1::Number(if change == 10 { 2.0 } else { 1.0 })),
        ),
        ("arguments", Value::Object(&args)),
    ];
    let outer = [
        (
            "route",
            Value::Data(InertValueV1::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", Value::Object(&header)),
    ];
    call(&Value::Object(&outer))
}

#[test]
fn registry_command_decoder_both_forms_routes_execute_once_and_replay_without_ordinary_authority() {
    use crate::instrumentation::NativeTimingProfileV1;
    for resync in [false, true] {
        for typed in [false, true] {
            for capacity in [0, 1, 64] {
                let (mut o, mut setup_r, s) = setup();
                let op = if resync {
                    ResourceRegistryCommandOperationV1::Resynchronize {
                        stream: open(&mut o, &s, None, &mut setup_r).stream,
                    }
                } else {
                    ResourceRegistryCommandOperationV1::Establish
                };
                let f = facts(&s);
                let id = o
                    .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut setup_r)
                    .unwrap();
                let p = prepare(&mut o, &id, &s, &mut setup_r);
                let mut r = Recorder::try_with_capacity(capacity)
                    .unwrap()
                    .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                let q = ResourceRegistryCommandRequestV1 {
                    operation: &op,
                    scope: &s,
                    preparation: Some(&p),
                    current: Some(&f),
                    owner_control_allowed: Some(true),
                    baseline_allowed: Some(true),
                };
                let result = envelope(typed, &op, &id, Some(&p), 0, |e| {
                    o.request_decoded_resource_registry_command_v1(
                        e,
                        &ResourceRegistryCommandPortV1 {
                            command: &id,
                            request: &q,
                        },
                        &mut r,
                    )
                })
                .unwrap();
                assert_eq!(
                    result,
                    ResourceRegistryCommandReplyV1::Settled {
                        command: id.clone()
                    }
                );
                let baseline = o
                    .resource_registry_command_facts_v1(&id, &s, &mut r)
                    .unwrap()
                    .as_observed
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .clone();
                change(&mut o, 0, false, false, &mut setup_r);
                o.faulted = true;
                let before = o.resource_registry_observation_usage_v1().unwrap();
                let replay = ResourceRegistryCommandRequestV1 {
                    operation: &op,
                    scope: &s,
                    preparation: None,
                    current: None,
                    owner_control_allowed: Some(true),
                    baseline_allowed: None,
                };
                let reply = envelope(!typed, &op, &id, None, 0, |e| {
                    o.request_decoded_resource_registry_command_v1(
                        e,
                        &ResourceRegistryCommandPortV1 {
                            command: &id,
                            request: &replay,
                        },
                        &mut r,
                    )
                })
                .unwrap();
                assert_eq!(
                    reply,
                    ResourceRegistryCommandReplyV1::ObserveExisting {
                        command: id.clone()
                    }
                );
                assert!(Arc::ptr_eq(
                    &baseline,
                    o.resource_registry_command_facts_v1(&id, &s, &mut setup_r)
                        .unwrap()
                        .as_observed
                        .unwrap()
                        .as_ref()
                        .unwrap()
                ));
                assert_eq!(o.resource_registry_observation_usage_v1().unwrap(), before);
                if capacity == 64 {
                    let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                    for boundary in [
                        Boundary::RegistryCommandArgumentNormalization,
                        Boundary::RegistryCommandFacade,
                    ] {
                        assert!(
                            events
                                .iter()
                                .any(|e| e.boundary == boundary && e.service_time.is_some())
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn registry_command_decoder_rejects_closed_grammar_mutations_before_any_native_attempt() {
    for resync in [false, true] {
        let (mut o, mut r, s) = setup();
        let op = if resync {
            ResourceRegistryCommandOperationV1::Resynchronize {
                stream: open(&mut o, &s, None, &mut r).stream,
            }
        } else {
            ResourceRegistryCommandOperationV1::Establish
        };
        let f = facts(&s);
        let id = o
            .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
            .unwrap();
        let p = prepare(&mut o, &id, &s, &mut r);
        let before = o.resource_registry_observation_usage_v1().unwrap();
        let q = ResourceRegistryCommandRequestV1 {
            operation: &op,
            scope: &s,
            preparation: Some(&p),
            current: Some(&f),
            owner_control_allowed: Some(true),
            baseline_allowed: Some(true),
        };
        for typed in [false, true] {
            for change in 1..=10 {
                assert!(matches!(
                    envelope(typed, &op, &id, Some(&p), change, |e| o
                        .request_decoded_resource_registry_command_v1(
                            e,
                            &ResourceRegistryCommandPortV1 {
                                command: &id,
                                request: &q
                            },
                            &mut r
                        )),
                    Err(ResourceError::Malformed)
                ));
            }
        }
        assert_eq!(o.resource_registry_observation_usage_v1().unwrap(), before);
        assert!(
            !o.resource_registry_command_facts_v1(&id, &s, &mut r)
                .unwrap()
                .native_started
        );
    }
}

#[test]
fn registry_command_decoder_foreign_command_and_mismatched_preparation_do_not_adopt_native_intent()
{
    let (mut o, mut r, s) = setup();
    let f = facts(&s);
    let op = ResourceRegistryCommandOperationV1::Establish;
    let id = o
        .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
        .unwrap();
    let other = o
        .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
        .unwrap();
    let p = prepare(&mut o, &id, &s, &mut r);
    let q = ResourceRegistryCommandRequestV1 {
        operation: &op,
        scope: &s,
        preparation: Some(&p),
        current: Some(&f),
        owner_control_allowed: Some(true),
        baseline_allowed: Some(true),
    };
    let port = ResourceRegistryCommandPortV1 {
        command: &id,
        request: &q,
    };
    for typed in [false, true] {
        assert!(
            envelope(typed, &op, &other, Some(&p), 0, |e| o
                .request_decoded_resource_registry_command_v1(e, &port, &mut r))
            .is_err()
        );
        assert!(
            envelope(typed, &op, &id, None, 0, |e| o
                .request_decoded_resource_registry_command_v1(e, &port, &mut r))
            .is_err()
        );
    }
    assert_eq!(
        o.resource_registry_command_facts_v1(&id, &s, &mut r)
            .unwrap()
            .phase,
        ResourceRegistryCommandPhaseV1::Issued
    );
    assert_eq!(
        o.resource_registry_command_facts_v1(&other, &s, &mut r)
            .unwrap()
            .phase,
        ResourceRegistryCommandPhaseV1::Issued
    );
    assert!(
        envelope(true, &op, &id, Some(&p), 0, |e| o
            .request_decoded_resource_registry_command_v1(e, &port, &mut r))
        .is_ok()
    );
}

#[test]
fn registry_command_decoder_preserves_closed_native_protocol_fault_and_unknown_instrumentation() {
    let (mut o, mut r, s) = setup();
    let f = facts(&s);
    let op = ResourceRegistryCommandOperationV1::Establish;
    let id = o
        .issue_resource_registry_command_v1(&op, &s, &admission(&f), &mut r)
        .unwrap();
    let p = prepare(&mut o, &id, &s, &mut r);
    let State::Active(pool) = &mut o.registry_observations else {
        panic!()
    };
    pool.command_fault = Some(FaultCut::AfterNative);
    let q = ResourceRegistryCommandRequestV1 {
        operation: &op,
        scope: &s,
        preparation: Some(&p),
        current: Some(&f),
        owner_control_allowed: Some(true),
        baseline_allowed: Some(true),
    };
    let mut r = Recorder::try_with_capacity(64).unwrap();
    let fault = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        envelope(true, &op, &id, Some(&p), 0, |e| {
            o.request_decoded_resource_registry_command_v1(
                e,
                &ResourceRegistryCommandPortV1 {
                    command: &id,
                    request: &q,
                },
                &mut r,
            )
        })
    }))
    .unwrap_err();
    assert!(fault.is::<ResourceRegistryCommandProtocolFaultV1>());
    assert_eq!(
        o.resource_registry_command_facts_v1(&id, &s, &mut r)
            .unwrap()
            .phase,
        ResourceRegistryCommandPhaseV1::Faulted
    );
    assert!(
        std::iter::from_fn(|| r.pop_front()).any(|e| e.boundary == Boundary::RegistryCommandFacade
            && e.disposition == Disposition::Unknown)
    );
}
