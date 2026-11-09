use super::*;
use crate::coordinator::source_query::result_contract::tests::{Fixture, owner};
use ReadObservationStopEnvelopeValueV1::{Context, Data, Object};
#[test]
fn native_stop_timing_separates_normalization_from_native_control_without_changing_target_binding()
{
    for route in ["typed", "dynamic"] {
        let mut f = owner();
        f.r = Recorder::try_with_capacity(256)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        with_port(&mut f, 0, |o, native, alternate, r| {
            with_input(&Input::new(alternate, route), |envelope| {
                assert_eq!(
                    o.stop_decoded_read_observation_facade_v1(
                        DecodedReadObservationStopRequestV1 { envelope, native },
                        r
                    ),
                    Err(Error::Unavailable)
                );
            });
            let refused: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert_eq!(refused.len(), 2);
            assert_eq!(
                refused[0].boundary,
                Boundary::ReadObservationStopArgumentNormalization
            );
            assert!(refused[0].service_time.is_some());
            assert_eq!(refused[1].service_time, None); // no native control episode entered
            with_input(&Input::new(native.target(), route), |envelope| {
                assert_eq!(
                    o.stop_decoded_read_observation_facade_v1(
                        DecodedReadObservationStopRequestV1 { envelope, native },
                        r
                    ),
                    Ok(())
                );
            });
            let accepted: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            let measured: Vec<_> = accepted
                .iter()
                .filter(|e| e.service_time.is_some())
                .map(|e| e.boundary)
                .collect();
            assert_eq!(
                measured,
                [
                    Boundary::ReadObservationStopArgumentNormalization,
                    Boundary::ReadObservationStopFacade
                ]
            );
            assert!(!o.is_faulted());
        });
    }
}
struct Input<'a, 'h> {
    route: &'a str,
    selector: InertValueV1<'a>,
    version: f64,
    variant: &'a str,
    contract: &'a str,
    operation: &'a str,
    context: ReadObservationStopEnvelopeValueV1<'a, 'h>,
    duplicate: bool,
    extra: bool,
    reverse: bool,
}
impl<'a, 'h> Input<'a, 'h> {
    fn new(target: ReadObservationStopTargetV1<'h>, route: &'a str) -> Self {
        Self {
            route,
            selector: if route == "typed" {
                InertValueV1::Number(18.0)
            } else {
                InertValueV1::Text("control-observation")
            },
            version: 1.0,
            variant: "stop-observation",
            contract: "read-context/1",
            operation: "stop-observation",
            context: Context(target),
            duplicate: false,
            extra: false,
            reverse: false,
        }
    }
}
fn with_input<'a, 'h, T>(
    input: &Input<'a, 'h>,
    run: impl for<'v> FnOnce(&'v ReadObservationStopEnvelopeValueV1<'v, 'h>) -> T,
) -> T {
    let mut args = [
        ("contract", Data(InertValueV1::Text(input.contract))),
        ("operation", Data(InertValueV1::Text(input.operation))),
        ("context", input.context),
        ("owner", Data(InertValueV1::Null)),
    ];
    if input.duplicate {
        args[1].0 = "contract";
    }
    if input.reverse {
        args[..3].reverse();
    }
    let mut header = [
        (
            if input.route == "typed" {
                "family"
            } else {
                "name"
            },
            Data(input.selector),
        ),
        ("variant", Data(InertValueV1::Text(input.variant))),
        ("version", Data(InertValueV1::Number(input.version))),
        (
            "arguments",
            Object(&args[..if input.extra { 4 } else { 3 }]),
        ),
    ];
    if input.reverse {
        header.reverse();
    }
    let mut outer = [
        ("route", Data(InertValueV1::Text(input.route))),
        ("request", Object(&header)),
    ];
    if input.reverse {
        outer.reverse();
    }
    run(&Object(&outer))
}
fn with_port<T>(
    f: &mut Fixture,
    kind: usize,
    run: impl for<'a> FnOnce(
        &mut SourceCoordinator,
        ReadObservationStopPortV1<'a>,
        ReadObservationStopTargetV1<'a>,
        &mut Recorder,
    ) -> T,
) -> T {
    let controls = [SourceReadContextControlV1 {
        scope: &f.s,
        owner_control_allowed: Some(true),
        retention_released: None,
        delivery_drained: None,
    }];
    let port = match kind {
        0 => ReadObservationStopPortV1::Source {
            context: &f.source,
            control: &controls[0],
        },
        1 => ReadObservationStopPortV1::Annotation {
            context: &f.combined,
            control: &controls[0],
        },
        2 => ReadObservationStopPortV1::VersionSet {
            target: VersionSetControlTargetV1::Context(&f.set.context),
            controls: &controls,
        },
        _ => ReadObservationStopPortV1::VersionSet {
            target: VersionSetControlTargetV1::Member(&f.set.participants[0].snapshot),
            controls: &controls,
        },
    };
    let alternate = match kind {
        0 => ReadObservationStopTargetV1::Annotation(&f.combined),
        1 => ReadObservationStopTargetV1::Source(&f.source),
        2 => ReadObservationStopTargetV1::VersionSet(VersionSetControlTargetV1::Member(
            &f.set.participants[0].snapshot,
        )),
        _ => ReadObservationStopTargetV1::VersionSet(VersionSetControlTargetV1::Context(
            &f.set.context,
        )),
    };
    run(&mut f.o, port, alternate, &mut f.r)
}
#[test]
fn stop_envelope_routes_and_field_order_canonicalize_to_the_same_original_capability() {
    for kind in 0..4 {
        let mut f = owner();
        with_port(&mut f, kind, |_, native, _, r| {
            let input = Input::new(native.target(), "typed");
            let a = with_input(&input, |e| {
                normalize_read_observation_stop_envelope_v1(e, r)
            })
            .unwrap();
            let mut input = Input::new(native.target(), "dynamic");
            input.reverse = true;
            let b = with_input(&input, |e| {
                normalize_read_observation_stop_envelope_v1(e, r)
            })
            .unwrap();
            assert_eq!(a, b);
            assert_eq!(a.target(), native.target());
        });
    }
}
#[test]
fn stop_normalization_retains_no_ordinary_packet_borrow_or_strings_and_does_not_grant_control() {
    let mut f = owner();
    let target = ReadObservationStopTargetV1::Source(&f.source);
    let normalized = {
        let name = String::from("control-observation");
        let route = String::from("dynamic");
        let mut input = Input::new(target, &route);
        input.selector = InertValueV1::Text(&name);
        with_input(&input, |e| {
            normalize_read_observation_stop_envelope_v1(e, &mut f.r)
        })
        .unwrap()
    };
    assert_eq!(normalized.target(), target);
    assert!(
        f.o.query_source_read_context(
            &f.source,
            &f.s,
            SourceContextQueryV1::Line { index: 0 },
            &crate::coordinator::source_query::result_contract::tests::source_facts(&f.s),
            &mut f.r
        )
        .is_some()
    );
    assert_eq!(
        format!("{normalized:?}"),
        "<native-stop-arguments:redacted>"
    );
}
#[test]
fn decoded_stop_dispatches_both_routes_for_all_original_native_target_roles_without_admission() {
    for kind in 0..4 {
        for route in ["typed", "dynamic"] {
            for faulted in [false, true] {
                let mut f = owner();
                let base = f.o.retention_usage().reserved;
                let generation = f.o.generation();
                f.o.faulted = faulted;
                f.o.source_facade_workspace = 7;
                let mut pressure =
                    f.o.retention
                        .reserve(f.o.retention_usage().maximum.unwrap() - base)
                        .unwrap();
                with_port(&mut f, kind, |o, native, _, r| {
                    with_input(&Input::new(native.target(), route), |envelope| {
                        assert_eq!(
                            o.stop_decoded_read_observation_facade_v1(
                                DecodedReadObservationStopRequestV1 { envelope, native },
                                r
                            ),
                            Ok(())
                        );
                    })
                });
                assert_eq!(f.o.is_faulted(), faulted);
                assert_eq!(f.o.generation(), generation);
                assert_eq!(f.o.read_facade_usage().workspace_bytes, 7);
                f.o.retention.release(&mut pressure).unwrap();
                assert_eq!(f.o.retention_usage().reserved, base);
            }
        }
    }
}
#[test]
fn stop_envelope_rejects_all_malformed_tuples_and_exact_record_violations_before_native_control() {
    static CYCLE: [InertValueV1<'static>; 1] = [InertValueV1::Array(&CYCLE)];
    for kind in 0..4 {
        for route in ["typed", "dynamic"] {
            let mut f = owner();
            let base = f.o.retention_usage().reserved;
            f.o.source_facade_fault = Some(Fault::BeforeNative);
            with_port(&mut f, kind, |o, native, _, r| {
                for bad in 0..20 {
                    let mut input = Input::new(native.target(), route);
                    match bad {
                        0 => input.route = "unknown",
                        1 => input.selector = InertValueV1::Text("18"),
                        2 => input.selector = InertValueV1::Number(19.0),
                        3 => input.version = 2.0,
                        4 => input.version = f64::NAN,
                        5 => input.variant = "establish-bounded-stream",
                        6 => input.contract = "read-context/2",
                        7 => input.operation = "revoke-observation-grant",
                        8 => input.context = Data(InertValueV1::Null),
                        9 => input.duplicate = true,
                        10 => input.extra = true,
                        11 => input.context = Data(InertValueV1::Array(&CYCLE)),
                        _ => {}
                    }
                    with_input(&input, |envelope| {
                        let mut reject = |envelope| {
                            assert_eq!(
                                o.stop_decoded_read_observation_facade_v1(
                                    DecodedReadObservationStopRequestV1 { envelope, native },
                                    r
                                ),
                                Err(Error::InvalidRequest),
                                "{route}/{kind}/{bad}"
                            );
                        };
                        let Object(outer) = envelope else {
                            unreachable!();
                        };
                        let Object(header) = outer[1].1 else {
                            unreachable!();
                        };
                        match bad {
                            12 => reject(&Data(InertValueV1::Null)),
                            13 => reject(&Object(&outer[..1])),
                            14 => reject(&Object(&[outer[0], ("route", outer[1].1)])),
                            15 => reject(&Object(&[
                                outer[0],
                                outer[1],
                                ("owner", Data(InertValueV1::Null)),
                            ])),
                            16 => reject(&Object(&[outer[0], ("request", Object(&header[..3]))])),
                            17 => reject(&Object(&[
                                outer[0],
                                (
                                    "request",
                                    Object(&[
                                        ("version", header[0].1),
                                        header[1],
                                        header[2],
                                        header[3],
                                    ]),
                                ),
                            ])),
                            18 => reject(&Object(&[
                                outer[0],
                                (
                                    "request",
                                    Object(&[
                                        header[0],
                                        header[1],
                                        header[2],
                                        header[3],
                                        ("owner", Data(InertValueV1::Null)),
                                    ]),
                                ),
                            ])),
                            19 => reject(&Object(&[
                                outer[0],
                                ("request", Data(InertValueV1::Text("request"))),
                            ])),
                            _ => reject(envelope),
                        }
                    });
                    assert!(!o.is_faulted());
                }
                o.source_facade_fault = None;
                with_input(&Input::new(native.target(), route), |envelope| {
                    assert_eq!(
                        o.stop_decoded_read_observation_facade_v1(
                            DecodedReadObservationStopRequestV1 { envelope, native },
                            r
                        ),
                        Ok(())
                    );
                });
            });
            assert_eq!(f.o.retention_usage().reserved, base);
        }
    }
}
#[test]
fn stop_envelope_never_repairs_cross_role_or_parent_member_binding_before_native_handoff() {
    for kind in 0..4 {
        for route in ["typed", "dynamic"] {
            let mut f = owner();
            f.o.source_facade_fault = Some(Fault::BeforeNative);
            with_port(&mut f, kind, |o, native, alternate, r| {
                let input = Input::new(alternate, route);
                with_input(&input, |envelope| {
                    assert_eq!(
                        o.stop_decoded_read_observation_facade_v1(
                            DecodedReadObservationStopRequestV1 { envelope, native },
                            r
                        ),
                        Err(Error::Unavailable)
                    );
                });
                assert!(!o.is_faulted());
                o.source_facade_fault = None;
                with_input(&Input::new(native.target(), route), |envelope| {
                    assert_eq!(
                        o.stop_decoded_read_observation_facade_v1(
                            DecodedReadObservationStopRequestV1 { envelope, native },
                            r
                        ),
                        Ok(())
                    );
                });
            });
        }
    }
}
#[test]
fn decoded_stop_capture_is_optional_but_reports_normalization_and_one_control_disposition() {
    for cap in [None, Some(0), Some(1), Some(64)] {
        let mut f = owner();
        f.r = cap.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        with_port(&mut f, 0, |o, native, _, r| {
            with_input(&Input::new(native.target(), "dynamic"), |envelope| {
                assert_eq!(
                    o.stop_decoded_read_observation_facade_v1(
                        DecodedReadObservationStopRequestV1 { envelope, native },
                        r
                    ),
                    Ok(())
                );
            })
        });
        let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
        if cap == Some(64) {
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.boundary == Boundary::ReadObservationStopArgumentNormalization)
                    .count(),
                1
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.boundary == Boundary::ReadObservationStopFacade)
                    .count(),
                1
            );
            assert!(
                events
                    .iter()
                    .all(|e| e.disposition == Disposition::Completed && e.service_time.is_none())
            );
        }
        assert!(!f.o.is_faulted());
    }
}
