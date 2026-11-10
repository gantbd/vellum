use super::super::super::tests::{cleanup, facts};
use super::super::tests::{certificate, receipt_facts, setup};
use super::*;

fn envelope<T>(
    route: &str,
    command: &ResourceRegistryCommandIdV1,
    context: &ResourceRegistryReceiptContextIdV1,
    change: usize,
    run: impl FnOnce(&Value<'_, '_>) -> T,
) -> T {
    let facet = [(
        "kind",
        Value::Data(InertValueV1::Text(if change == 5 {
            "other"
        } else {
            "registry-baseline"
        })),
    )];
    let mut args = vec![
        (
            "contract",
            Value::Data(InertValueV1::Text(RESOURCE_REGISTRY_RECEIPT_PROFILE_V1)),
        ),
        (
            "operation",
            Value::Data(InertValueV1::Text("retained-outcome")),
        ),
        ("command", Value::Command(command)),
        ("context", Value::Context(context)),
        (
            "profile",
            Value::Data(InertValueV1::Text(RESOURCE_REGISTRY_OBSERVATION_PROFILE_V1)),
        ),
        ("facet", Value::Object(&facet)),
    ];
    match change {
        1 => args[0].1 = Value::Data(InertValueV1::Text("execution-receipt/1")),
        2 => args.push(("preparation", Value::Data(InertValueV1::Null))),
        3 => args[1].0 = "contract",
        4 => args[2].1 = Value::Context(context),
        6 => args[4].1 = Value::Data(InertValueV1::Text("unsupported")),
        10 => args[3].1 = Value::Data(InertValueV1::Object(&[])),
        _ => {}
    }
    let header = [
        (
            if route == "typed" { "family" } else { "name" },
            if route == "typed" {
                Value::Data(InertValueV1::Number(if change == 9 { 18.0 } else { 5.0 }))
            } else {
                Value::Data(InertValueV1::Text(if change == 9 {
                    "control-observation"
                } else {
                    "inspect-execution"
                }))
            },
        ),
        (
            "variant",
            Value::Data(InertValueV1::Text("retained-outcome")),
        ),
        (
            "version",
            Value::Data(InertValueV1::Number(if change == 7 { 2.0 } else { 1.0 })),
        ),
        ("arguments", Value::Object(&args)),
    ];
    let outer = [
        (
            "route",
            Value::Data(InertValueV1::Text(if change == 8 { "wire" } else { route })),
        ),
        ("request", Value::Object(&header)),
    ];
    run(&Value::Object(&outer))
}

#[test]
fn registry_receipt_decoder_both_routes_reach_actual_release_and_preserve_original_result() {
    use crate::instrumentation::NativeTimingProfileV1;
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 64] {
            let (mut o, _, s, cmd, result) = setup();
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            let f = facts(&s);
            let rf = receipt_facts(&f);
            let context = o
                .open_resource_registry_receipt_v1(
                    &cmd,
                    &s,
                    ResourceRegistryReceiptCeilingV1 { allowed: true },
                    &rf,
                    Some(true),
                    &mut r,
                )
                .unwrap();
            let root = o.native_root_identity().unwrap();
            for route in ["typed", "dynamic"] {
                let packet = certificate(&mut o, &context, &cmd, &s, &result, (true, true), &mut r);
                let port = ResourceRegistryReceiptPortV1 {
                    command: &cmd,
                    context: &context,
                    scope: &s,
                    preparation: &packet,
                    current: &rf,
                };
                let reply = envelope(route, &cmd, &context, 0, |e| {
                    o.release_decoded_resource_registry_receipt_v1(e, &port, &mut r)
                })
                .unwrap();
                assert!(Arc::ptr_eq(&reply.receipt, &result));
                assert_eq!(reply.status(), "operation-receipt");
                assert_eq!(reply.facet(), "registry-baseline");
                assert!(
                    envelope(route, &cmd, &context, 0, |e| o
                        .release_decoded_resource_registry_receipt_v1(e, &port, &mut r))
                    .is_err()
                );
            }
            assert_eq!(o.native_root_identity().unwrap(), root);
            assert_eq!(
                o.resource_registry_observation_usage_v1().unwrap().streams,
                1
            );
            assert_eq!(
                o.resource_registry_receipt_usage_v1().unwrap().certificates,
                0
            );
            if capacity == 64 {
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                for boundary in [
                    Boundary::RegistryReceiptArgumentNormalization,
                    Boundary::RegistryReceiptFacade,
                ] {
                    assert!(events.iter().any(|e| e.boundary == boundary
                        && e.service_time.is_some()
                            == (timing == NativeTimingProfileV1::ProcessMonotonicV1)));
                }
            }
        }
    }
}

#[test]
fn registry_receipt_decoder_rejects_unknown_extra_duplicate_and_wrong_roles_before_loss() {
    let (mut o, mut r, s, cmd, result) = setup();
    let f = facts(&s);
    let rf = receipt_facts(&f);
    let context = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let packet = certificate(&mut o, &context, &cmd, &s, &result, (true, true), &mut r);
    let mut denied = receipt_facts(&f);
    denied.receipt_grant_active = Some(false);
    let port = ResourceRegistryReceiptPortV1 {
        command: &cmd,
        context: &context,
        scope: &s,
        preparation: &packet,
        current: &denied,
    };
    let usage = o.resource_registry_observation_usage_v1().unwrap();
    for route in ["typed", "dynamic"] {
        for change in 1..=10 {
            assert!(matches!(
                envelope(route, &cmd, &context, change, |e| o
                    .release_decoded_resource_registry_receipt_v1(e, &port, &mut r)),
                Err(ResourceError::Malformed)
            ));
        }
    }
    assert_eq!(o.resource_registry_observation_usage_v1().unwrap(), usage);
    assert_eq!(o.resource_registry_receipt_usage_v1().unwrap().stopped, 0);
    let good = ResourceRegistryReceiptPortV1 {
        current: &rf,
        ..port
    };
    assert!(
        envelope("typed", &cmd, &context, 0, |e| o
            .release_decoded_resource_registry_receipt_v1(e, &good, &mut r))
        .is_ok()
    );
}

#[test]
fn registry_receipt_decoder_foreign_handles_cannot_apply_native_context_loss() {
    let (mut o, mut r, s, cmd, result) = setup();
    let (_, _, _, foreign_cmd, _) = setup();
    let f = facts(&s);
    let rf = receipt_facts(&f);
    let one = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let two = o
        .open_resource_registry_receipt_v1(
            &cmd,
            &s,
            ResourceRegistryReceiptCeilingV1 { allowed: true },
            &rf,
            Some(true),
            &mut r,
        )
        .unwrap();
    let packet = certificate(&mut o, &one, &cmd, &s, &result, (true, true), &mut r);
    let mut denied = receipt_facts(&f);
    denied.receipt_grant_active = Some(false);
    let port = ResourceRegistryReceiptPortV1 {
        command: &cmd,
        context: &one,
        scope: &s,
        preparation: &packet,
        current: &denied,
    };
    for (command, context) in [(&foreign_cmd, &one), (&cmd, &two)] {
        assert!(matches!(
            envelope("typed", command, context, 0, |e| o
                .release_decoded_resource_registry_receipt_v1(e, &port, &mut r)),
            Err(ResourceError::Unavailable)
        ));
    }
    assert_eq!(o.resource_registry_receipt_usage_v1().unwrap().stopped, 0);
    o.stop_resource_registry_receipt_v1(&one, &cleanup(&s), &mut r)
        .unwrap();
    assert!(
        envelope("dynamic", &cmd, &one, 0, |e| o
            .release_decoded_resource_registry_receipt_v1(e, &port, &mut r))
        .is_err()
    );
}
