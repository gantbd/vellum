use super::*;
use crate::capture::InertValueV1;
use crate::coordinator::publication_stream::tests::{facts, limits, native, owner};
use crate::instrumentation::NativeTimingProfileV1;

#[test]
fn versioned_stop_foreign_original_target_must_not_alias_same_numbered_native_slot() {
    let (mut a, mut r, s) = owner(limits());
    let (mut b, mut br, bs) = owner(limits());
    let n = native(&s);
    let bn = native(&bs);
    let local = a
        .open_source_publication_stream_v1(&s, &facts(&s, &n), &mut r)
        .unwrap();
    let foreign = b
        .open_source_publication_stream_v1(&bs, &facts(&bs, &bn), &mut br)
        .unwrap();
    let ctl = SourceReadContextControlV1 {
        scope: &s,
        owner_control_allowed: Some(true),
        retention_released: None,
        delivery_drained: None,
    };
    let result = request(
        ObservationStopTargetV2::SourcePublication(&foreign.stream),
        true,
        "stop-observation",
        "stop-observation",
        |envelope| {
            a.stop_decoded_observation_facade_v2(
                DecodedObservationStopRequestV2 {
                    envelope,
                    native: ObservationStopPortV2::SourcePublication {
                        stream: &local.stream,
                        control: &ctl,
                    },
                },
                &mut r,
            )
        },
    );
    assert_eq!(result, Err(ObservationStopErrorV2::Unavailable));
    assert_eq!(a.source_publication_usage_v1().unwrap().stopped, 0);
    assert_ne!(local.stream, foreign.stream);
    assert_ne!(local.generation, foreign.generation);
}

#[test]
fn versioned_stop_rejects_closed_schema_mutations_without_touching_native_owner() {
    use ObservationStopEnvelopeValueV2 as V;
    for typed in [false, true] {
        let (mut o, mut r, s) = owner(limits());
        let n = native(&s);
        let opened = o
            .open_source_publication_stream_v1(&s, &facts(&s, &n), &mut r)
            .unwrap();
        let ctl = SourceReadContextControlV1 {
            scope: &s,
            owner_control_allowed: Some(true),
            retention_released: None,
            delivery_drained: None,
        };
        request(
            ObservationStopTargetV2::SourcePublication(&opened.stream),
            typed,
            "stop-observation",
            "stop-observation",
            |env| {
                let V::Object(outer) = env else { panic!() };
                let V::Object(header) = outer[1].1 else {
                    panic!()
                };
                let V::Object(args) = header[3].1 else {
                    panic!()
                };
                // Mutate each fixed-width record independently: extra, duplicate,
                // unknown, missing, and wrong role/type. No payload is traversed.
                for depth in 0..3 {
                    for kind in 0..5 {
                        let mut changed = [*outer, header, args][depth].to_vec();
                        match kind {
                            0 => changed.push(("surprise", V::Data(InertValueV1::Null))),
                            1 => changed[1].0 = changed[0].0,
                            2 => changed[0].0 = "surprise",
                            3 => {
                                changed.pop();
                            }
                            _ => {
                                changed[0].1 = V::Context(
                                    ObservationStopTargetV2::SourcePublication(&opened.stream),
                                )
                            }
                        }
                        let mut h = header.to_vec();
                        let mut root = outer.to_vec();
                        let bad = match depth {
                            0 => V::Object(&changed),
                            1 => {
                                root[1].1 = V::Object(&changed);
                                V::Object(&root)
                            }
                            _ => {
                                h[3].1 = V::Object(&changed);
                                root[1].1 = V::Object(&h);
                                V::Object(&root)
                            }
                        };
                        let mut trace = Recorder::try_with_capacity(16).unwrap();
                        assert_eq!(
                            o.stop_decoded_observation_facade_v2(
                                DecodedObservationStopRequestV2 {
                                    envelope: &bad,
                                    native: ObservationStopPortV2::SourcePublication {
                                        stream: &opened.stream,
                                        control: &ctl
                                    }
                                },
                                &mut trace
                            ),
                            Err(ObservationStopErrorV2::InvalidRequest)
                        );
                        assert!(
                            !std::iter::from_fn(|| trace.pop_front())
                                .any(|e| e.boundary == Boundary::SourcePublicationStop)
                        );
                        assert_eq!(o.source_publication_usage_v1().unwrap().stopped, 0);
                    }
                }
            },
        );
    }
}

#[test]
fn versioned_stop_native_owner_rejects_foreign_stream_even_when_lowering_matches() {
    let (mut a, mut r, s) = owner(limits());
    let (mut b, mut br, bs) = owner(limits());
    let bn = native(&bs);
    let foreign = b
        .open_source_publication_stream_v1(&bs, &facts(&bs, &bn), &mut br)
        .unwrap();
    let ctl = SourceReadContextControlV1 {
        scope: &s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    let held = a.retention_usage();
    let result = request(
        ObservationStopTargetV2::SourcePublication(&foreign.stream),
        true,
        "stop-observation",
        "stop-observation",
        |envelope| {
            a.stop_decoded_observation_facade_v2(
                DecodedObservationStopRequestV2 {
                    envelope,
                    native: ObservationStopPortV2::SourcePublication {
                        stream: &foreign.stream,
                        control: &ctl,
                    },
                },
                &mut r,
            )
        },
    );
    assert_eq!(
        result,
        Err(ObservationStopErrorV2::SourcePublication(
            SourcePublicationErrorV1::Unavailable
        ))
    );
    assert_eq!(a.retention_usage(), held);
    assert!(!a.is_faulted());
    assert_eq!(b.source_publication_usage_v1().unwrap().stopped, 0);
}

pub(in crate::coordinator) fn request<'h, T>(
    target: ObservationStopTargetV2<'h>,
    typed: bool,
    variant: &str,
    operation: &str,
    call: impl FnOnce(&ObservationStopEnvelopeValueV2<'_, 'h>) -> T,
) -> T {
    use ObservationStopEnvelopeValueV2 as V;
    let args = [
        ("contract", V::Data(InertValueV1::Text("read-context/1"))),
        ("operation", V::Data(InertValueV1::Text(operation))),
        ("context", V::Context(target)),
    ];
    let header = [
        (
            if typed { "family" } else { "name" },
            if typed {
                V::Data(InertValueV1::Number(18.0))
            } else {
                V::Data(InertValueV1::Text("control-observation"))
            },
        ),
        ("variant", V::Data(InertValueV1::Text(variant))),
        ("version", V::Data(InertValueV1::Number(1.0))),
        ("arguments", V::Object(&args)),
    ];
    let outer = [
        (
            "route",
            V::Data(InertValueV1::Text(if typed { "typed" } else { "dynamic" })),
        ),
        ("request", V::Object(&header)),
    ];
    call(&V::Object(&outer))
}
#[test]
fn versioned_stop_reaches_source_streams_without_ordinary_grants_refunds_or_fault_healing() {
    for typed in [false, true] {
        for allowed in [None, Some(false), Some(true)] {
            for faulted in [false, true] {
                for timing in [
                    NativeTimingProfileV1::Off,
                    NativeTimingProfileV1::ProcessMonotonicV1,
                ] {
                    for capacity in [0, 1, 64] {
                        let (mut o, mut setup, s) = owner(limits());
                        let n = native(&s);
                        let f = facts(&s, &n);
                        let opened = o
                            .open_source_publication_stream_v1(&s, &f, &mut setup)
                            .unwrap();
                        let control = SourceReadContextControlV1 {
                            scope: &s,
                            owner_control_allowed: allowed,
                            retention_released: Some(false),
                            delivery_drained: None,
                        };
                        let held = o.retention_usage();
                        let generation = o.generation();
                        o.faulted = faulted;
                        let mut r = Recorder::try_with_capacity(capacity)
                            .unwrap()
                            .with_native_timing(timing);
                        let result = request(
                            ObservationStopTargetV2::SourcePublication(&opened.stream),
                            typed,
                            "stop-observation",
                            "stop-observation",
                            |envelope| {
                                o.stop_decoded_observation_facade_v2(
                                    DecodedObservationStopRequestV2 {
                                        envelope,
                                        native: ObservationStopPortV2::SourcePublication {
                                            stream: &opened.stream,
                                            control: &control,
                                        },
                                    },
                                    &mut r,
                                )
                            },
                        );
                        assert_eq!(result.is_ok(), allowed == Some(true));
                        assert_eq!(o.is_faulted(), faulted);
                        assert_eq!(o.generation(), generation);
                        assert_eq!(o.retention_usage(), held);
                        assert_eq!(
                            o.source_publication_usage_v1().unwrap().stopped,
                            usize::from(allowed == Some(true))
                        );
                        if capacity == 64 {
                            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                            assert!(
                                events.iter().any(|e| e.boundary
                                    == Boundary::ObservationStopFacadeV2
                                    && e.service_time.is_some()
                                        == (timing == NativeTimingProfileV1::ProcessMonotonicV1))
                            );
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn versioned_stop_rejects_wrong_original_target_and_revocation_spelling_before_native_control() {
    for typed in [false, true] {
        let (mut o, mut setup, s) = owner(limits());
        let n = native(&s);
        let f = facts(&s, &n);
        let a = o
            .open_source_publication_stream_v1(&s, &f, &mut setup)
            .unwrap();
        let b = o
            .open_source_publication_stream_v1(&s, &f, &mut setup)
            .unwrap();
        let control = SourceReadContextControlV1 {
            scope: &s,
            owner_control_allowed: Some(true),
            retention_released: Some(true),
            delivery_drained: Some(true),
        };
        for (target, variant, op) in [
            (&b.stream, "stop-observation", "stop-observation"),
            (&a.stream, "revoke-observation-grant", "stop-observation"),
            (&a.stream, "stop-observation", "revoke-observation-grant"),
        ] {
            let mut r = Recorder::try_with_capacity(64).unwrap();
            let result = request(
                ObservationStopTargetV2::SourcePublication(target),
                typed,
                variant,
                op,
                |envelope| {
                    o.stop_decoded_observation_facade_v2(
                        DecodedObservationStopRequestV2 {
                            envelope,
                            native: ObservationStopPortV2::SourcePublication {
                                stream: &a.stream,
                                control: &control,
                            },
                        },
                        &mut r,
                    )
                },
            );
            assert!(result.is_err());
            assert_eq!(o.source_publication_usage_v1().unwrap().stopped, 0);
            assert!(
                !std::iter::from_fn(|| r.pop_front())
                    .any(|e| e.boundary == Boundary::SourcePublicationStop)
            );
        }
    }
}
#[test]
fn versioned_stop_delegates_existing_read_owners_without_changing_v1() {
    let mut f = crate::coordinator::source_query::result_contract::tests::owner();
    let ctl = SourceReadContextControlV1 {
        scope: &f.s,
        owner_control_allowed: Some(true),
        retention_released: None,
        delivery_drained: None,
    };
    let port = ReadObservationStopPortV1::Source {
        context: &f.source,
        control: &ctl,
    };
    request(
        ObservationStopTargetV2::Read(ReadObservationStopTargetV1::Source(&f.source)),
        true,
        "stop-observation",
        "stop-observation",
        |envelope| {
            f.o.stop_decoded_observation_facade_v2(
                DecodedObservationStopRequestV2 {
                    envelope,
                    native: ObservationStopPortV2::Existing(ObservationStopPortV1::Read(port)),
                },
                &mut f.r,
            )
        },
    )
    .unwrap();
    assert!(
        f.o.query_source_read_context(
            &f.source,
            &f.s,
            SourceContextQueryV1::Line { index: 0 },
            &crate::coordinator::source_query::result_contract::tests::source_facts(&f.s),
            &mut f.r
        )
        .is_none()
    );
}
