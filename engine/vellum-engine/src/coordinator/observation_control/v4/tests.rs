use super::*;
use crate::instrumentation::NativeTimingProfileV1;
mod responsiveness;
#[test]
fn v4_closed_control_grammar_rejects_forgery_and_mismatched_operations_before_native_service() {
    use crate::capture::InertValueV1 as D;
    use ObservationStopEnvelopeValueV4 as V;
    for revoke in [false, true] {
        for typed in [false, true] {
            let (mut o, mut setup, s, command, one, _) = source();
            let target = ObservationStopTargetV4::SourceReceipt {
                command: &command,
                context: &one,
            };
            let control = SourceReadContextControlV1 {
                scope: &s,
                owner_control_allowed: Some(true),
                retention_released: None,
                delivery_drained: None,
            };
            let authority = TrustedObservationRevocationV2 {
                original_target: target,
                revocation_allowed: Some(true),
            };
            let debt = o.retention_usage();
            let mut r = Recorder::try_with_capacity(128).unwrap();
            let operation = if revoke {
                "revoke-observation-grant"
            } else {
                "stop-observation"
            };
            for case in 0..10 {
                request(target, typed, operation, |e| {
                    let V::Object(outer) = e else { unreachable!() };
                    let mut outer = outer.to_vec();
                    let V::Object(header) = outer[1].1 else {
                        unreachable!()
                    };
                    let mut header = header.to_vec();
                    let V::Object(args) = header[3].1 else {
                        unreachable!()
                    };
                    let mut args = args.to_vec();
                    match case {
                        6 => args[0].1 = V::Data(D::Text("read-context/2")),
                        7 => args[1].1 = V::Data(D::Text("request-cancellation")),
                        8 => args[2].1 = V::Data(D::Text("forged-context")),
                        9 => args.push(("revocationAllowed", V::Data(D::Boolean(true)))),
                        _ => (),
                    }
                    header[3].1 = V::Object(&args);
                    match case {
                        3 => header[0].1 = V::Data(D::Number(999.0)),
                        4 => header[2].1 = V::Data(D::Number(2.0)),
                        5 => {
                            header[1].1 = V::Data(D::Text(if revoke {
                                "stop-observation"
                            } else {
                                "revoke-observation-grant"
                            }))
                        }
                        _ => (),
                    }
                    outer[1].1 = V::Object(&header);
                    match case {
                        0 => {
                            outer.pop();
                        }
                        1 => outer[1].0 = "route",
                        2 => outer[0].1 = V::Data(D::Text("wire")),
                        _ => (),
                    }
                    if revoke {
                        assert_eq!(
                            o.revoke_decoded_observation_context_v2(
                                &V::Object(&outer),
                                ObservationStopPortV4::SourceReceipt {
                                    command: &command,
                                    context: &one,
                                    control: &control
                                },
                                &authority,
                                &mut r
                            ),
                            Err(ObservationRevocationIngressErrorV2::InvalidRequest)
                        );
                    } else {
                        assert_eq!(
                            o.stop_decoded_observation_facade_v4(
                                DecodedObservationStopRequestV4 {
                                    envelope: &V::Object(&outer),
                                    native: ObservationStopPortV4::SourceReceipt {
                                        command: &command,
                                        context: &one,
                                        control: &control
                                    }
                                },
                                &mut r
                            ),
                            Err(ObservationStopErrorV4::InvalidRequest)
                        );
                    }
                });
            }
            while let Some(e) = r.pop_front() {
                assert_ne!(e.boundary, Boundary::SourcePublicationReceiptStop);
                if e.boundary == Boundary::ObservationStopFacadeV4 {
                    assert_eq!(e.service_time, None);
                }
            }
            assert_eq!(o.source_publication_receipt_usage_v1().unwrap().stopped, 0);
            assert!(
                o.source_publication_receipt_basis_v1(&one, &command, &s, &mut setup)
                    .is_ok()
            );
            assert_eq!(o.retention_usage(), debt);
        }
    }
}

#[test]
fn v4_existing_delegation_preserves_old_native_stream_control_and_keeps_receipts_separate() {
    let (mut o, mut r, s, command, _, _) = source();
    let stream = o
        .source_publication_command_facts_v1(&command, &s, &mut r)
        .unwrap()
        .as_observed
        .unwrap()
        .as_ref()
        .unwrap()
        .stream
        .clone();
    let target = ObservationStopTargetV4::Existing(ObservationStopTargetV3::Existing(
        ObservationStopTargetV2::SourcePublication(&stream),
    ));
    let control = SourceReadContextControlV1 {
        scope: &s,
        owner_control_allowed: Some(true),
        retention_released: None,
        delivery_drained: None,
    };
    for revoke in [false, true] {
        let port = ObservationStopPortV4::Existing(ObservationStopPortV3::Existing(
            ObservationStopPortV2::SourcePublication {
                stream: &stream,
                control: &control,
            },
        ));
        if revoke {
            request(target, false, "revoke-observation-grant", |e| {
                o.revoke_decoded_observation_context_v2(
                    e,
                    port,
                    &TrustedObservationRevocationV2 {
                        original_target: target,
                        revocation_allowed: Some(true),
                    },
                    &mut r,
                )
            })
            .unwrap();
        } else {
            request(target, true, "stop-observation", |e| {
                o.stop_decoded_observation_facade_v4(
                    DecodedObservationStopRequestV4 {
                        envelope: e,
                        native: port,
                    },
                    &mut r,
                )
            })
            .unwrap();
        }
    }
    assert_eq!(o.source_publication_usage_v1().unwrap().stopped, 1);
    assert_eq!(o.source_publication_receipt_usage_v1().unwrap().stopped, 0);
    assert!(ObservationStopErrorV4::SourceReceipt(SourcePublicationErrorV1::Faulted).faulted());
    assert!(ObservationStopErrorV4::FacetReceipt(FacetPublicationErrorV1::Faulted).faulted());
    assert!(
        ObservationStopErrorV4::CollectionReceipt(CollectionPublicationErrorV1::Faulted).faulted()
    );
    assert!(
        ObservationStopErrorV4::Existing(ObservationStopErrorV3::FacetPublication(
            FacetPublicationErrorV1::Faulted
        ))
        .faulted()
    );
    assert!(!ObservationStopErrorV4::Unavailable.faulted());
}

pub(in crate::coordinator) fn request<'h, T>(
    target: ObservationStopTargetV4<'h>,
    typed: bool,
    operation: &str,
    consume: impl FnOnce(&ObservationStopEnvelopeValueV4<'_, 'h>) -> T,
) -> T {
    use ObservationStopEnvelopeValueV4 as V;
    let args = [
        (
            "contract",
            V::Data(crate::capture::InertValueV1::Text("read-context/1")),
        ),
        (
            "operation",
            V::Data(crate::capture::InertValueV1::Text(operation)),
        ),
        ("context", V::Context(target)),
    ];
    let h = [
        if typed {
            (
                "family",
                V::Data(crate::capture::InertValueV1::Number(18.0)),
            )
        } else {
            (
                "name",
                V::Data(crate::capture::InertValueV1::Text("control-observation")),
            )
        },
        (
            "variant",
            V::Data(crate::capture::InertValueV1::Text(operation)),
        ),
        (
            "version",
            V::Data(crate::capture::InertValueV1::Number(1.0)),
        ),
        ("arguments", V::Object(&args)),
    ];
    consume(&V::Object(&[
        (
            "route",
            V::Data(crate::capture::InertValueV1::Text(if typed {
                "typed"
            } else {
                "dynamic"
            })),
        ),
        ("request", V::Object(&h)),
    ]))
}

pub(in crate::coordinator) fn source() -> (
    SourceCoordinator,
    Recorder,
    SourceReadScopeV1,
    SourcePublicationCommandIdV1,
    SourcePublicationReceiptContextIdV1,
    SourcePublicationReceiptContextIdV1,
) {
    use crate::coordinator::publication_stream::commands::*;
    let (mut o, mut r, s, command, _) =
        source_receipt_test_setup(source_receipt_test_limits(), 8388608);
    let one = source_receipt_test_open(&mut o, &command, &s, &mut r);
    let two = source_receipt_test_open(&mut o, &command, &s, &mut r);
    (o, r, s, command, one, two)
}
pub(in crate::coordinator) fn facet() -> (
    SourceCoordinator,
    Recorder,
    FacetPublicationScopeV1,
    FacetPublicationCommandIdV1,
    FacetPublicationReceiptContextIdV1,
    FacetPublicationReceiptContextIdV1,
) {
    use crate::coordinator::facet_publication::commands::*;
    let (mut o, mut r, s, command) = facet_receipt_test_setup_with_budget(8388608);
    let one = facet_receipt_test_open(&mut o, &command, &s, &mut r);
    let two = facet_receipt_test_open(&mut o, &command, &s, &mut r);
    (o, r, s, command, one, two)
}
pub(in crate::coordinator) fn collection() -> (
    SourceCoordinator,
    Recorder,
    CollectionPublicationScopeV1,
    CollectionPublicationCommandIdV1,
    CollectionPublicationReceiptContextIdV1,
    CollectionPublicationReceiptContextIdV1,
) {
    use crate::coordinator::collection_publication::commands::tests::{open_receipt, setup};
    use crate::coordinator::collection_publication::tests::{facts, packet};
    let (mut o, mut r, s, g) = setup();
    let p = packet(&mut o, &s, &g, &mut r);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let op = CollectionPublicationCommandOperationV1::Establish;
    let command = o
        .issue_collection_publication_command_v1(&op, &p, &s, &f, Some(true), &mut r)
        .unwrap();
    o.request_collection_publication_command_v1(
        &command,
        &CollectionPublicationCommandRequestV1 {
            scope: &s,
            operation: &op,
            preparation: &p,
            current: Some(&f),
            owner_control_allowed: Some(true),
            max_reply_bytes: 16384,
        },
        &mut r,
    )
    .unwrap();
    let one = open_receipt(&mut o, &command, &s, &mut r);
    let two = open_receipt(&mut o, &command, &s, &mut r);
    (o, r, s, command, one, two)
}

macro_rules! receipt_control_case {
    ($name:ident, $setup:ident, $variant:ident, $control:ident, $usage:ident, $stream_usage:ident,
        $retire_command:ident, $read:ident) => {
        #[test]
        fn $name() {
            for revoke in [false, true] {
                for typed in [false, true] {
                    for mode in 0..4 {
                        let (mut o, mut setup, s, command, one, two) = $setup();
                        let (_, _, _, foreign_command, _, _) = $setup();
                        let target = ObservationStopTargetV4::$variant {
                            command: &command,
                            context: &one,
                        };
                        let sibling = ObservationStopTargetV4::$variant {
                            command: &command,
                            context: &two,
                        };
                        let foreign_pair = ObservationStopTargetV4::$variant {
                            command: &foreign_command,
                            context: &one,
                        };
                        let debt = o.retention_usage();
                        let before = o.$stream_usage().unwrap();
                        let mut r = if mode == 0 {
                            Recorder::disabled()
                        } else {
                            Recorder::try_with_capacity(if mode == 3 { 1 } else { 256 }).unwrap()
                        };
                        if mode >= 2 {
                            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
                        }
                        let sentinel = Observation {
                            boundary: Boundary::Control,
                            disposition: Disposition::Completed,
                            service_time: None,
                        };
                        if mode == 3 {
                            r.record(sentinel);
                        }
                        let mut control = $control {
                            scope: &s,
                            owner_control_allowed: Some(true),
                            retention_released: None,
                            delivery_drained: None,
                        };
                        for denied in [None, Some(false)] {
                            if revoke {
                                let authority = TrustedObservationRevocationV2 {
                                    original_target: target,
                                    revocation_allowed: denied,
                                };
                                request(target, typed, "revoke-observation-grant", |e| {
                                    assert!(
                                        o.revoke_decoded_observation_context_v2(
                                            e,
                                            ObservationStopPortV4::$variant {
                                                command: &command,
                                                context: &one,
                                                control: &control
                                            },
                                            &authority,
                                            &mut r
                                        )
                                        .is_err()
                                    )
                                });
                            } else {
                                control.owner_control_allowed = denied;
                                request(target, typed, "stop-observation", |e| {
                                    assert!(
                                        o.stop_decoded_observation_facade_v4(
                                            DecodedObservationStopRequestV4 {
                                                envelope: e,
                                                native: ObservationStopPortV4::$variant {
                                                    command: &command,
                                                    context: &one,
                                                    control: &control
                                                }
                                            },
                                            &mut r
                                        )
                                        .is_err()
                                    )
                                });
                            }
                            assert_eq!(o.$usage().unwrap().stopped, 0);
                        }
                        control.owner_control_allowed = Some(true);
                        if revoke {
                            let wrong_authority = TrustedObservationRevocationV2 {
                                original_target: sibling,
                                revocation_allowed: Some(true),
                            };
                            request(target, typed, "revoke-observation-grant", |e| {
                                assert!(
                                    o.revoke_decoded_observation_context_v2(
                                        e,
                                        ObservationStopPortV4::$variant {
                                            command: &command,
                                            context: &one,
                                            control: &control
                                        },
                                        &wrong_authority,
                                        &mut r
                                    )
                                    .is_err()
                                )
                            });
                        }
                        // Exact original port/target binding precedes all native controls.
                        request(sibling, typed, "stop-observation", |e| {
                            assert!(
                                o.stop_decoded_observation_facade_v4(
                                    DecodedObservationStopRequestV4 {
                                        envelope: e,
                                        native: ObservationStopPortV4::$variant {
                                            command: &command,
                                            context: &one,
                                            control: &control
                                        }
                                    },
                                    &mut r
                                )
                                .is_err()
                            )
                        });
                        // Even a matching forged pair cannot bypass the actual receipt owner.
                        request(foreign_pair, typed, "stop-observation", |e| {
                            assert!(
                                o.stop_decoded_observation_facade_v4(
                                    DecodedObservationStopRequestV4 {
                                        envelope: e,
                                        native: ObservationStopPortV4::$variant {
                                            command: &foreign_command,
                                            context: &one,
                                            control: &control
                                        }
                                    },
                                    &mut r
                                )
                                .is_err()
                            )
                        });
                        assert_eq!(o.$usage().unwrap().stopped, 0);
                        for _ in 0..2 {
                            if revoke {
                                let authority = TrustedObservationRevocationV2 {
                                    original_target: target,
                                    revocation_allowed: Some(true),
                                };
                                request(target, typed, "revoke-observation-grant", |e| {
                                    o.revoke_decoded_observation_context_v2(
                                        e,
                                        ObservationStopPortV4::$variant {
                                            command: &command,
                                            context: &one,
                                            control: &control,
                                        },
                                        &authority,
                                        &mut r,
                                    )
                                })
                                .unwrap();
                            } else {
                                request(target, typed, "stop-observation", |e| {
                                    o.stop_decoded_observation_facade_v4(
                                        DecodedObservationStopRequestV4 {
                                            envelope: e,
                                            native: ObservationStopPortV4::$variant {
                                                command: &command,
                                                context: &one,
                                                control: &control,
                                            },
                                        },
                                        &mut r,
                                    )
                                })
                                .unwrap();
                            }
                        }
                        assert_eq!(o.$usage().unwrap().stopped, 1);
                        assert!(o.$read(&one, &command, &s, &mut setup).is_err());
                        assert!(o.$read(&two, &command, &s, &mut setup).is_ok());
                        assert_eq!(o.$stream_usage().unwrap(), before);
                        assert_eq!(o.retention_usage(), debt);
                        let cleanup = $control {
                            scope: &s,
                            owner_control_allowed: Some(true),
                            retention_released: Some(true),
                            delivery_drained: Some(true),
                        };
                        assert!(o.$retire_command(&command, &cleanup, &mut setup).is_err());
                        crate::coordinator::shutdown::tests::install(&mut o, &mut setup);
                        o.faulted = true;
                        o.stop_observation_facade_v4(
                            ObservationStopPortV4::$variant {
                                command: &command,
                                context: &one,
                                control: &control,
                            },
                            &mut r,
                        )
                        .unwrap();
                        assert!(o.faulted);
                        if mode == 0 {
                            assert_eq!(r.queued(), 0);
                            assert_eq!(r.dropped(), 0);
                        } else if mode == 3 {
                            assert_eq!(r.pop_front(), Some(sentinel));
                            assert!(r.dropped() > 0);
                        } else {
                            let mut completed = 0;
                            while let Some(e) = r.pop_front() {
                                if e.boundary == Boundary::ObservationStopFacadeV4
                                    && e.disposition == Disposition::Completed
                                {
                                    completed += 1;
                                    assert_eq!(e.service_time.is_some(), mode == 2);
                                }
                            }
                            assert_eq!(completed, 3);
                            assert_eq!(r.dropped(), 0);
                        }
                    }
                }
            }
        }
    };
}
receipt_control_case!(
    source_receipt_control_v4,
    source,
    SourceReceipt,
    SourceReadContextControlV1,
    source_publication_receipt_usage_v1,
    source_publication_usage_v1,
    retire_source_publication_command_v1,
    source_publication_receipt_basis_v1
);
receipt_control_case!(
    facet_receipt_control_v4,
    facet,
    FacetReceipt,
    FacetPublicationControlV1,
    facet_publication_receipt_usage_v1,
    facet_publication_usage_v1,
    retire_facet_publication_command_v1,
    facet_publication_receipt_basis_v1
);
receipt_control_case!(
    collection_receipt_control_v4,
    collection,
    CollectionReceipt,
    CollectionPublicationControlV1,
    collection_publication_receipt_usage_v1,
    collection_publication_usage_v1,
    retire_collection_publication_command_v1,
    collection_publication_receipt_basis_v1
);
