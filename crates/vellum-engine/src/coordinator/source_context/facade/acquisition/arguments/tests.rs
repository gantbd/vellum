use super::super::tests::{Fixture, counters, invoke_with, owner, params};
use super::*;
use ReadAcquisitionEnvelopeValueV1::{Classes, Data, Document, Object, Participants, Reference};
fn selection(p: ReadAcquisitionFacadePortV1<'_>) -> ReadAcquisitionSelectionV1<'_> {
    use ReadAcquisitionFacadePortV1 as P;
    match p {
        P::SourceRetained { original, .. } => ReadAcquisitionSelectionV1::Source(original),
        P::AnnotationRetained { original, .. } => ReadAcquisitionSelectionV1::Annotation(original),
        P::VersionSetRetained { original, .. } => ReadAcquisitionSelectionV1::VersionSet(original),
        _ => ReadAcquisitionSelectionV1::Current,
    }
}
pub(in crate::coordinator) fn with_envelope<'h, T>(
    request: ReadAcquisitionFacadeRequestV1<'h>,
    route: &str,
    profile: Option<&str>,
    f: impl for<'v> FnOnce(&'v ReadAcquisitionEnvelopeValueV1<'v, 'h>) -> T,
) -> T {
    let p = request.port;
    let coherent = matches!(
        p,
        ReadAcquisitionFacadePortV1::VersionSetCurrent { .. }
            | ReadAcquisitionFacadePortV1::VersionSetRetained { .. }
    );
    let rows = (0..p.count())
        .map(|j| {
            let (scope, data) = p.at(j);
            ReadAcquisitionParticipantV1 {
                document: &scope.document,
                data,
            }
        })
        .collect::<Vec<_>>();
    let selection = selection(p);
    let current = selection == ReadAcquisitionSelectionV1::Current;
    let select = [
        (
            "kind",
            Data(InertValueV1::Text(if current {
                "capture-current"
            } else {
                "retained"
            })),
        ),
        ("reference", Reference(selection)),
    ];
    let args = [
        (
            "contract",
            Data(InertValueV1::Text(if coherent {
                "read-version-set/1"
            } else {
                "read-context/1"
            })),
        ),
        ("operation", Data(InertValueV1::Text("snapshot-handoff"))),
        (
            if coherent { "authority" } else { "document" },
            if coherent {
                Data(InertValueV1::Text(&request.actor.authority))
            } else {
                Document(rows[0].document)
            },
        ),
        (
            if coherent { "participants" } else { "data" },
            if coherent {
                Participants(&rows)
            } else {
                Classes(rows[0].data)
            },
        ),
        ("selection", Object(&select[..if current { 1 } else { 2 }])),
        (
            "profile",
            Data(InertValueV1::Text(
                profile.unwrap_or(&p.at(0).0.context.profile),
            )),
        ),
    ];
    let header = [
        (
            if route == "typed" { "family" } else { "name" },
            Data(if route == "typed" {
                InertValueV1::Number(1.0)
            } else {
                InertValueV1::Text("read-document")
            }),
        ),
        ("variant", Data(InertValueV1::Text("snapshot-handoff"))),
        ("version", Data(InertValueV1::Number(1.0))),
        ("arguments", Object(&args)),
    ];
    let outer = [
        ("route", Data(InertValueV1::Text(route))),
        ("request", Object(&header)),
    ];
    f(&Object(&outer))
}
fn run(
    f: &mut Fixture,
    kind: usize,
    route: &str,
    profile: Option<&str>,
    p: super::super::tests::Params,
) -> Option<ReadAcquisitionFacadeReplyV1> {
    invoke_with(f, kind, p, |o, native, r| {
        with_envelope(native, route, profile, |e| {
            o.acquire_decoded_read_facade_v1(
                DecodedReadAcquisitionFacadeRequestV1 {
                    envelope: e,
                    native,
                },
                r,
            )
        })
    })
}
#[test]
fn decoded_acquisition_facade_preserves_normalization_errors_and_success_for_all_ports_and_routes()
{
    for kind in 0..6 {
        for route in ["typed", "dynamic"] {
            let mut f = owner();
            f.base.r = Recorder::try_with_capacity(256)
                .unwrap()
                .with_native_timing(
                    crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
                );
            crate::coordinator::release_measurements::tests::enable(&mut f.base.o, &mut f.base.r);
            invoke_with(&mut f, kind, params(), |o, native, r| {
                let malformed = ReadAcquisitionEnvelopeValueV1::Data(InertValueV1::Null);
                let before = o.retention_usage().reserved;
                assert!(matches!(
                    o.try_acquire_decoded_read_facade_v1(
                        DecodedReadAcquisitionFacadeRequestV1 {
                            envelope: &malformed,
                            native
                        },
                        r
                    ),
                    Err(Error::InvalidRequest)
                ));
                assert_eq!(o.retention_usage().reserved, before);
                with_envelope(native, route, None, |envelope| {
                    let out = o.try_acquire_decoded_read_facade_v1(
                        DecodedReadAcquisitionFacadeRequestV1 { envelope, native },
                        r,
                    );
                    assert!(out.is_ok(), "port {kind}/{route}");
                    out.ok()
                })
            });
            let events: Vec<_> = std::iter::from_fn(|| f.base.r.pop_front()).collect();
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::AcquisitionFacadeRelease
                        && e.service_time.is_some())
            );
            assert!(
                events
                    .iter()
                    .filter(|e| e.boundary != Boundary::AcquisitionFacadeRelease)
                    .all(|e| e.service_time.is_none())
            );
            assert!(!f.base.o.is_faulted());
            assert_eq!(
                crate::coordinator::release_measurements::tests::totals(
                    &mut f.base.o,
                    &mut f.base.r
                ),
                [1, 0, 1, 0, 0, 0]
            );
            assert_eq!(f.base.o.read_facade_usage().workspace_bytes, 0);
        }
    }
}
#[test]
fn native_acquisition_routes_normalize_to_equal_meaning_for_all_six_ports() {
    for kind in 0..6 {
        let mut f = owner();
        invoke_with(&mut f, kind, params(), |_, native, r| {
            let a = with_envelope(native, "typed", None, |e| {
                normalize_read_acquisition_envelope_v1(e, native.actor, native.limits, 16384, r)
            })
            .unwrap();
            let b = with_envelope(native, "dynamic", None, |e| {
                normalize_read_acquisition_envelope_v1(e, native.actor, native.limits, 16384, r)
            })
            .unwrap();
            assert_eq!(a, b);
            assert_eq!(a.selection(), selection(native.port));
            assert_eq!(a.participants().len(), native.port.count());
            assert!(std::ptr::eq(
                a.participants()[0].document,
                &native.port.at(0).0.document
            ));
            None
        });
    }
}
#[test]
fn native_acquisition_arguments_dispatch_current_and_retained_ports_through_both_routes() {
    for kind in 0..6 {
        for route in ["typed", "dynamic"] {
            let mut f = owner();
            let base = f.base.o.retention_usage().reserved;
            assert!(run(&mut f, kind, route, None, params()).is_some());
            assert!(!f.base.o.is_faulted());
            assert_eq!(
                f.base.o.read_facade_usage(),
                ReadFacadeUsageV1 {
                    busy: false,
                    workspace_bytes: 0
                }
            );
            assert!(f.base.o.retention_usage().reserved >= base); // native installed context obligations remain
            if kind == 2 || kind == 4 {
                let before = counters(&f);
                assert!(run(&mut f, kind, route, None, params()).is_none());
                assert_eq!(counters(&f), before); // prepared transfer is once-only
            }
        }
    }
}
fn changed<'h, T>(
    e: &ReadAcquisitionEnvelopeValueV1<'_, 'h>,
    mode: usize,
    f: impl for<'v> FnOnce(&'v ReadAcquisitionEnvelopeValueV1<'v, 'h>) -> T,
) -> T {
    let Object(outer) = e else { panic!("outer") };
    let mut outer = outer.to_vec();
    let Object(header) = outer[1].1 else {
        panic!("header")
    };
    let mut header = header.to_vec();
    let Object(args) = header[3].1 else {
        panic!("args")
    };
    let mut args = args.to_vec();
    let Object(select) = args[4].1 else {
        panic!("selection")
    };
    let mut select = select.to_vec();
    match mode {
        0 => header[0].1 = Data(InertValueV1::Number(2.0)),
        1 => header[2].1 = Data(InertValueV1::Number(2.0)),
        2 => header[1].1 = Data(InertValueV1::Text("line")),
        3 => args[0].1 = Data(InertValueV1::Text("unknown")),
        4 => args[1].1 = Data(InertValueV1::Text("line")),
        5 => args[5].1 = Data(InertValueV1::Text("")),
        6 => args[5].0 = "contract",
        7 => args.push(("extra", Data(InertValueV1::Null))),
        8 => select[0].1 = Data(InertValueV1::Text("unknown")),
        9 => {
            if select.len() == 1 {
                select.push(("reference", Reference(ReadAcquisitionSelectionV1::Current)));
            } else {
                select[1].1 = Reference(ReadAcquisitionSelectionV1::Current);
            }
        }
        10 => args[2].1 = Data(InertValueV1::Null),
        11 => args[3].1 = Data(InertValueV1::Null),
        12 => args[2].1 = Data(InertValueV1::Text("foreign")),
        _ => outer[0].1 = Data(InertValueV1::Text("unknown")),
    }
    args[4].1 = Object(&select);
    header[3].1 = Object(&args);
    outer[1].1 = Object(&header);
    f(&Object(&outer))
}
#[test]
fn acquisition_malformed_vocabulary_role_types_and_selection_shapes_refuse_without_coercion() {
    for kind in 0..6 {
        let mut f = owner();
        invoke_with(&mut f, kind, params(), |_, native, r| {
            for mode in 0..14 {
                with_envelope(native, "typed", None, |e| {
                    changed(e, mode, |e| {
                        assert!(
                            normalize_read_acquisition_envelope_v1(
                                e,
                                native.actor,
                                native.limits,
                                16384,
                                r
                            )
                            .is_err(),
                            "{kind}/{mode}"
                        );
                    })
                });
            }
            None
        });
    }
}
#[test]
fn acquisition_normalization_has_independent_bounds_and_inclusive_node_depth_edges() {
    for kind in 0..6 {
        let mut f = owner();
        invoke_with(&mut f, kind, params(), |_, native, r| {
            with_envelope(native, "typed", None, |e| {
                let n = normalize_read_acquisition_envelope_v1(
                    e,
                    native.actor,
                    native.limits,
                    16384,
                    r,
                )
                .unwrap();
                let nodes = n.capture_usage().iter().map(|u| u.nodes).max().unwrap();
                let depth = n.capture_usage().iter().map(|u| u.depth).max().unwrap();
                for (caps, ok) in [
                    (
                        CaptureLimitsV1 {
                            max_bytes: 1,
                            ..native.limits
                        },
                        false,
                    ),
                    (
                        CaptureLimitsV1 {
                            max_nodes: nodes,
                            ..native.limits
                        },
                        true,
                    ),
                    (
                        CaptureLimitsV1 {
                            max_nodes: nodes - 1,
                            ..native.limits
                        },
                        false,
                    ),
                    (
                        CaptureLimitsV1 {
                            max_depth: depth,
                            ..native.limits
                        },
                        true,
                    ),
                    (
                        CaptureLimitsV1 {
                            max_depth: depth - 1,
                            ..native.limits
                        },
                        false,
                    ),
                ] {
                    assert_eq!(
                        normalize_read_acquisition_envelope_v1(e, native.actor, caps, 16384, r)
                            .is_ok(),
                        ok
                    );
                }
                assert!(
                    normalize_read_acquisition_envelope_v1(e, native.actor, native.limits, 1, r)
                        .is_err()
                );
                let mut actor = native.actor.clone();
                actor.recipient.clear();
                assert!(
                    normalize_read_acquisition_envelope_v1(e, &actor, native.limits, 16384, r)
                        .is_err()
                );
                assert!(n.owned_bytes() <= native.limits.max_bytes);
                assert_eq!(n.limits(), native.limits);
            });
            None
        });
    }
}
#[test]
fn unsupported_acquisition_profile_never_consumes_a_cut_or_poisons_retained_authority() {
    for kind in 0..6 {
        let mut f = owner();
        let before = counters(&f);
        let mut p = params();
        p.authorized = Some(false);
        assert!(run(&mut f, kind, "typed", Some("unsupported"), p).is_none());
        assert_eq!(counters(&f), before);
        assert!(run(&mut f, kind, "dynamic", None, params()).is_some());
        assert!(!f.base.o.is_faulted());
    }
}
#[test]
fn acquisition_unknown_facts_withhold_but_accepted_retained_loss_is_absorbing() {
    for kind in 0..6 {
        let mut f = owner();
        let before = counters(&f);
        let mut p = params();
        p.authorized = None;
        assert!(run(&mut f, kind, "typed", None, p).is_none());
        assert_eq!(counters(&f), before);
        assert!(run(&mut f, kind, "dynamic", None, params()).is_some());
    }
    for kind in [1, 3, 5] {
        let mut f = owner();
        let mut p = params();
        p.authorized = Some(false);
        p.resources = Some(false);
        assert!(run(&mut f, kind, "typed", None, p).is_none());
        assert!(run(&mut f, kind, "dynamic", None, params()).is_none());
        assert!(!f.base.o.is_faulted());
    }
}
#[test]
fn decoded_acquisition_faults_preserve_once_only_installs_and_nonrefunding_issuance() {
    for kind in 0..6 {
        for fault in [
            Fault::Classification,
            Fault::Workspace,
            Fault::BeforeNative,
            Fault::AfterNative,
            Fault::BadResult,
            Fault::RefundDrift,
        ] {
            let mut f = owner();
            let before = counters(&f);
            let base = f.base.o.retention_usage().reserved;
            let generation = f.base.o.generation();
            f.base.o.source_facade_fault = Some(fault);
            assert!(run(&mut f, kind, "typed", None, params()).is_none());
            assert!(f.base.o.is_faulted());
            assert_eq!(f.base.o.retention_usage().reserved, base);
            assert_eq!(f.base.o.generation(), generation);
            assert_eq!(
                f.base.o.read_facade_usage(),
                ReadFacadeUsageV1 {
                    busy: false,
                    workspace_bytes: 0
                }
            );
            let mut expected = before;
            if matches!(
                fault,
                Fault::AfterNative | Fault::BadResult | Fault::RefundDrift
            ) {
                expected[if kind < 2 {
                    0
                } else if kind < 4 {
                    1
                } else {
                    2
                }] += if kind < 4 { 1 } else { 2 };
                if kind == 2 {
                    expected[3] -= 1;
                }
                if kind == 4 {
                    expected[4] -= 1;
                }
            }
            assert_eq!(counters(&f), expected);
            f.base.o.source_facade_fault = None;
            assert!(run(&mut f, kind, "dynamic", None, params()).is_none());
            assert_eq!(counters(&f), expected);
        }
    }
}
#[test]
fn decoded_acquisition_uses_exact_eight_frame_admission_and_the_shared_reentry_fence() {
    let fixed = size_of::<CaptureLimitsV1>();
    let total = fixed + 8 * params().limits.max_bytes;
    for kind in 0..6 {
        for available in [fixed - 1, fixed, total - 1, total] {
            let mut f = owner();
            let base = f.base.o.retention_usage().reserved;
            let pressure = f.base.o.retention_usage().maximum.unwrap() - base - available;
            let mut held = f.base.o.retention.reserve(pressure).unwrap();
            let before = f.base.o.retention_usage().reserved;
            assert_eq!(
                run(&mut f, kind, "typed", None, params()).is_some(),
                available >= total
            );
            assert_eq!(f.base.o.retention_usage().reserved, before);
            assert!(!f.base.o.is_faulted());
            f.base.o.retention.release(&mut held).unwrap();
        }
        let mut f = owner();
        f.base.o.source_facade_fault = Some(Fault::Reentry);
        assert!(run(&mut f, kind, "typed", None, params()).is_some());
        assert!(!f.base.o.is_faulted());
    }
}
#[test]
fn decoded_acquisition_refusals_under_pressure_do_not_spend_or_accept_loss() {
    for kind in 0..6 {
        for coordinate in 0..4 {
            let mut f = owner();
            let before = counters(&f);
            let mut p = params();
            p.authorized = Some(false);
            match coordinate {
                0 => p.limits.max_bytes = 0,
                1 => p.limits.max_nodes = 0,
                2 => p.limits.max_depth = 1,
                _ => f.base.o.set_settings_parent_limit_for_test(1),
            }
            assert!(run(&mut f, kind, "typed", None, p).is_none());
            assert_eq!(counters(&f), before);
            assert!(!f.base.o.is_faulted());
            f.base.o.set_settings_parent_limit_for_test(16384);
            assert!(run(&mut f, kind, "dynamic", None, params()).is_some());
        }
    }
}
#[test]
fn decoded_acquisition_diagnostics_are_optional_bounded_and_payload_free() {
    for kind in 0..6 {
        for capacity in [None, Some(128), Some(1)] {
            let mut f = owner();
            f.base.r = capacity.map_or_else(Recorder::disabled, |n| {
                Recorder::try_with_capacity(n).unwrap()
            });
            assert!(run(&mut f, kind, "dynamic", None, params()).is_some());
            assert!(!f.base.o.is_faulted());
            match capacity {
                None => assert_eq!(f.base.r.queued(), 0),
                Some(1) => assert!(f.base.r.dropped() > 0),
                _ => {
                    let mut seen = false;
                    while let Some(e) = f.base.r.pop_front() {
                        seen |= e.boundary == Boundary::ReadAcquisitionArgumentNormalization;
                        assert!(e.service_time.is_none());
                    }
                    assert!(seen);
                }
            }
        }
    }
}

#[test]
fn acquisition_normalized_owned_data_outlives_packet_strings_and_temporary_participant_rows() {
    let mut f = owner();
    invoke_with(&mut f, 4, params(), |_, native, r| {
        let n = {
            let profile = String::from("temporary-profile");
            let mut actor = native.actor.clone();
            actor.purpose = "temporary-purpose".into();
            with_envelope(native, "typed", Some(&profile), |e| {
                normalize_read_acquisition_envelope_v1(e, &actor, native.limits, 16384, r)
            })
            .unwrap()
        };
        assert_eq!(n.profile(), "temporary-profile");
        assert_eq!(n.actor().purpose, "temporary-purpose");
        assert_eq!(n.participants()[0].document, &native.port.at(0).0.document);
        assert_eq!(format!("{n:?}"), "<native-read-arguments:redacted>");
        None
    });
}

#[test]
fn acquisition_participants_preserve_order_and_classes_and_reject_duplicate_native_documents() {
    fn normal<'h>(
        native: ReadAcquisitionFacadeRequestV1<'h>,
        rows: &[ReadAcquisitionParticipantV1<'h>],
        r: &mut Recorder,
    ) -> Result<NormalizedReadAcquisitionArgumentsV1<'h>, Error> {
        with_envelope(native, "typed", None, |e| {
            let Object(outer) = e else { panic!("outer") };
            let mut outer = outer.to_vec();
            let Object(header) = outer[1].1 else {
                panic!("header")
            };
            let mut header = header.to_vec();
            let Object(args) = header[3].1 else {
                panic!("args")
            };
            let mut args = args.to_vec();
            args[3].1 = Participants(rows);
            header[3].1 = Object(&args);
            outer[1].1 = Object(&header);
            normalize_read_acquisition_envelope_v1(
                &Object(&outer),
                native.actor,
                native.limits,
                16384,
                r,
            )
        })
    }
    let mut f = owner();
    invoke_with(&mut f, 4, params(), |o, native, r| {
        let peer = o.create(b"peer", r).unwrap();
        let mut peer_scope = native.port.at(0).0.clone();
        peer_scope.document = peer.document;
        let a = ReadAcquisitionParticipantV1 {
            document: &native.port.at(0).0.document,
            data: AnnotationProjectionClassesV1::Source,
        };
        let b = ReadAcquisitionParticipantV1 {
            document: &peer_scope.document,
            data: AnnotationProjectionClassesV1::Annotations,
        };
        // Keep request arrays alive: normalized identities borrow document handles,
        // not array positions. Temporary packet rows are owned by with_envelope.
        let forward = [a, b];
        let reverse = [b, a];
        let duplicate = [a, a];
        let n = normal(native, &forward, r).unwrap();
        let m = normal(native, &reverse, r).unwrap();
        assert_ne!(n, m);
        assert_eq!(
            n.participants()[0].data,
            AnnotationProjectionClassesV1::Source
        );
        assert_eq!(n.participants()[1].document, &peer_scope.document);
        assert!(normal(native, &duplicate, r).is_err());
        None
    });
}

#[test]
fn acquisition_argument_class_mismatch_refuses_before_native_loss_or_preparation_consumption() {
    for kind in 0..6 {
        let mut f = owner();
        let before = counters(&f);
        let mut p = params();
        p.authorized = Some(false);
        assert!(
            invoke_with(&mut f, kind, p, |o, native, r| with_envelope(
                native,
                "typed",
                None,
                |e| {
                    let Object(outer) = e else { panic!("outer") };
                    let mut outer = outer.to_vec();
                    let Object(header) = outer[1].1 else {
                        panic!("header")
                    };
                    let mut header = header.to_vec();
                    let Object(args) = header[3].1 else {
                        panic!("args")
                    };
                    let mut args = args.to_vec();
                    let replacement =
                        if native.port.at(0).1 == AnnotationProjectionClassesV1::Combined {
                            AnnotationProjectionClassesV1::Source
                        } else {
                            AnnotationProjectionClassesV1::Combined
                        };
                    let mut rows = Vec::new();
                    if let Participants(p) = args[3].1 {
                        rows.extend_from_slice(p);
                        rows[0].data = replacement;
                        args[3].1 = Participants(&rows);
                    } else {
                        args[3].1 = Classes(replacement);
                    }
                    header[3].1 = Object(&args);
                    outer[1].1 = Object(&header);
                    o.acquire_decoded_read_facade_v1(
                        DecodedReadAcquisitionFacadeRequestV1 {
                            envelope: &Object(&outer),
                            native,
                        },
                        r,
                    )
                }
            ))
            .is_none()
        );
        assert_eq!(counters(&f), before);
        assert!(run(&mut f, kind, "dynamic", None, params()).is_some());
    }
}
