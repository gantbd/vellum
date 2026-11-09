use super::*;
use vellum_engine::capture::CaptureLimitsV1;
use vellum_engine::capture::InertValueV1;
#[path = "facade/installed.rs"]
mod installed;
#[test]
fn given_native_timing_opt_in_when_read_refusal_and_protected_stop_execute_then_only_real_service_boundaries_gain_durations()
 {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1};
    for profile in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        let (mut o, mut r, _, scope) = ready(b"source");
        let source = o
            .capture_source_read_context(&scope, &facts(&scope), &mut r)
            .unwrap();
        r = Recorder::try_with_capacity(256)
            .unwrap()
            .with_native_timing(profile);
        let base = o.retention_usage().reserved;
        let generation = o.generation();
        let current = facts(&scope);
        let req = SourceFacadeRequestV1 {
            source: SearchReadContextV1::Source(&source),
            scope: &scope,
            actor: &scope.context,
            query: SourceContextQueryV1::Line { index: 0 },
            limits: limits(),
            facts: SearchReadFactsV1::Source(&current),
        };
        let Ok(SourceFacadeReplyV1::Source(reply)) = o.try_query_source_facade_v1(req, &mut r)
        else {
            panic!("source");
        };
        let SourceContextContentV1::Line(line) = reply.content else {
            panic!("line");
        };
        assert_eq!(line.body(), b"source");
        assert!(matches!(
            o.try_query_source_facade_v1(
                SourceFacadeRequestV1 {
                    query: SourceContextQueryV1::Line { index: 999 },
                    ..req
                },
                &mut r
            ),
            Err(SourceReadContextErrorV1::InvalidRequest)
        ));
        assert_eq!(
            o.stop_read_observation_facade_v1(
                ReadObservationStopPortV1::Source {
                    context: &source,
                    control: &controls(&scope)
                },
                &mut r
            ),
            Ok(())
        );
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        let measured: Vec<_> = events
            .iter()
            .filter(|e| e.service_time.is_some())
            .map(|e| e.boundary)
            .collect();
        assert_eq!(
            measured,
            if profile == NativeTimingProfileV1::Off {
                vec![]
            } else {
                vec![
                    Boundary::SourceFacadeRelease,
                    Boundary::SourceFacadeRelease,
                    Boundary::ReadObservationStopFacade,
                ]
            }
        );
        assert_eq!(o.retention_usage().reserved, base);
        assert_eq!(o.generation(), generation);
        assert!(!o.is_faulted());
        assert!(o.retire_source_read_context(&source, &controls(&scope), &mut r));
    }
}
#[test]
fn given_decoded_stop_envelopes_when_owner_injection_is_rejected_then_both_routes_can_still_stop_the_original_context()
 {
    use ReadObservationStopEnvelopeValueV1::{Context, Data, Object};
    for dynamic in [false, true] {
        let (mut o, mut r, _, scope) = ready(b"source");
        let source = o
            .capture_source_read_context(&scope, &facts(&scope), &mut r)
            .unwrap();
        let base = o.retention_usage().reserved;
        let c = SourceReadContextControlV1 {
            retention_released: None,
            delivery_drained: Some(false),
            ..controls(&scope)
        };
        let native = ReadObservationStopPortV1::Source {
            context: &source,
            control: &c,
        };
        let args = [
            ("contract", Data(InertValueV1::Text("read-context/1"))),
            ("operation", Data(InertValueV1::Text("stop-observation"))),
            (
                "context",
                Context(ReadObservationStopTargetV1::Source(&source)),
            ),
            ("owner", Data(InertValueV1::Text("self-issued-owner"))),
        ];
        for count in [4, 3] {
            let header = [
                (
                    if dynamic { "name" } else { "family" },
                    Data(if dynamic {
                        InertValueV1::Text("control-observation")
                    } else {
                        InertValueV1::Number(18.0)
                    }),
                ),
                ("variant", Data(InertValueV1::Text("stop-observation"))),
                ("version", Data(InertValueV1::Number(1.0))),
                ("arguments", Object(&args[..count])),
            ];
            let outer = [
                (
                    "route",
                    Data(InertValueV1::Text(if dynamic {
                        "dynamic"
                    } else {
                        "typed"
                    })),
                ),
                ("request", Object(&header)),
            ];
            assert_eq!(
                o.stop_decoded_read_observation_facade_v1(
                    DecodedReadObservationStopRequestV1 {
                        envelope: &Object(&outer),
                        native
                    },
                    &mut r
                ),
                if count == 4 {
                    Err(SourceReadContextErrorV1::InvalidRequest)
                } else {
                    Ok(())
                }
            );
            let readable = o
                .query_source_read_context(
                    &source,
                    &scope,
                    SourceContextQueryV1::Line { index: 0 },
                    &facts(&scope),
                    &mut r,
                )
                .is_some();
            assert_eq!(readable, count == 4);
            assert_eq!(o.retention_usage().reserved, base);
            assert!(!o.is_faulted());
        }
        assert!(!o.retire_source_read_context(&source, &c, &mut r));
        assert!(o.retire_source_read_context(&source, &controls(&scope), &mut r));
    }
}

#[test]
fn given_stop_observation_facade_when_ordinary_read_admission_is_unavailable_then_control_still_closes_only_its_original_context()
 {
    let (mut o, mut r, _, scope) = ready(b"source");
    let original = o
        .capture_source_read_context(&scope, &facts(&scope), &mut r)
        .unwrap();
    let copy = o
        .capture_retained_source_read_context(&original, &scope, &facts(&scope), &mut r)
        .unwrap();
    let base = o.retention_usage().reserved;
    let issued = o.source_read_context_usage().issued_lifetime;
    assert!(
        o.try_query_source_facade_v1(
            SourceFacadeRequestV1 {
                source: SearchReadContextV1::Source(&original),
                scope: &scope,
                actor: &scope.context,
                query: SourceContextQueryV1::Line { index: 0 },
                limits: CaptureLimitsV1 {
                    max_nodes: 0,
                    ..limits()
                },
                facts: SearchReadFactsV1::Source(&facts(&scope))
            },
            &mut r
        )
        .is_err()
    );
    let undrained = SourceReadContextControlV1 {
        retention_released: None,
        delivery_drained: Some(false),
        ..controls(&scope)
    };
    assert_eq!(
        o.stop_read_observation_facade_v1(
            ReadObservationStopPortV1::Source {
                context: &original,
                control: &undrained
            },
            &mut r
        ),
        Ok(())
    );
    assert!(
        o.query_source_read_context(
            &original,
            &scope,
            SourceContextQueryV1::Line { index: 0 },
            &facts(&scope),
            &mut r
        )
        .is_none()
    );
    assert!(!o.retire_source_read_context(&original, &undrained, &mut r));
    let reply = o
        .query_source_read_context(
            &copy,
            &scope,
            SourceContextQueryV1::Line { index: 0 },
            &facts(&scope),
            &mut r,
        )
        .unwrap();
    let SourceContextContentV1::Line(line) = reply.content else {
        panic!("line");
    };
    assert_eq!(line.body(), b"source");
    assert_eq!(o.retention_usage().reserved, base);
    assert_eq!(o.source_read_context_usage().issued_lifetime, issued);
    assert!(o.retire_source_read_context(&original, &controls(&scope), &mut r));
    assert_eq!(
        o.stop_read_observation_facade_v1(
            ReadObservationStopPortV1::Source {
                context: &original,
                control: &controls(&scope)
            },
            &mut r
        ),
        Err(SourceReadContextErrorV1::Unavailable)
    );
    assert_eq!(
        o.stop_read_observation_facade_v1(
            ReadObservationStopPortV1::Source {
                context: &copy,
                control: &undrained
            },
            &mut r
        ),
        Ok(())
    );
    assert!(o.retire_source_read_context(&copy, &controls(&scope), &mut r));
    assert!(!o.is_faulted());
}

#[test]
fn given_optional_release_measurements_when_reads_refuse_succeed_and_collection_stops_then_engine_behavior_and_cleanup_remain_independent()
 {
    use vellum_engine::instrumentation::measurement::{
        CounterV1, MeasureV1, PointStateV1, ValueLimitsV1, ValueV1,
    };
    let (mut o, mut r, _, scope) = ready(b"source");
    let profile = ReleaseMeasurementLimitsV1 {
        values: ValueLimitsV1 {
            max_integer: 100,
            max_seconds: 0.0,
        },
        max_pool_bytes: ReleaseMeasurementLimitsV1::required_pool_bytes(),
    };
    let base = o.retention_usage().reserved;
    assert_eq!(o.release_measurements_v1(&mut r).unwrap(), None);
    assert!(
        o.configure_release_measurements_v1(profile, Some(true), &mut r)
            .unwrap()
    );
    let current = facts(&scope);
    let Ok(ReadAcquisitionFacadeReplyV1::Source(source)) = o.try_acquire_read_facade_v1(
        ReadAcquisitionFacadeRequestV1 {
            port: ReadAcquisitionFacadePortV1::SourceCurrent {
                scope: &scope,
                facts: &current,
            },
            actor: &scope.context,
            limits: limits(),
        },
        &mut r,
    ) else {
        panic!("capture");
    };
    let req = SourceFacadeRequestV1 {
        source: SearchReadContextV1::Source(&source),
        scope: &scope,
        actor: &scope.context,
        facts: SearchReadFactsV1::Source(&current),
        query: SourceContextQueryV1::Line { index: 999 },
        limits: limits(),
    };
    assert!(matches!(
        o.try_query_source_facade_v1(req, &mut r),
        Err(SourceReadContextErrorV1::InvalidRequest)
    ));
    let valid = SourceFacadeRequestV1 {
        query: SourceContextQueryV1::Line { index: 0 },
        ..req
    };
    assert!(o.query_source_facade_v1(valid, &mut r).is_some());
    let points = o.release_measurements_v1(&mut r).unwrap().unwrap();
    assert!(
        points
            .iter()
            .all(|p| p.measure() == MeasureV1::ReleaseDispositions)
    );
    assert_eq!(
        points.map(|p| match p.state() {
            PointStateV1::Observed {
                value: ValueV1::Counter(CounterV1 { total, exact: true }),
                ..
            } => total,
            _ => panic!("complete counter"),
        }),
        [2, 0, 1, 0, 0, 0]
    );
    assert!(!o.retire_release_measurements_v1(
        &ReleaseMeasurementControlV1 {
            owner_control_allowed: Some(true),
            delivery_drained: Some(true),
            retention_released: Some(true)
        },
        &mut r
    ));
    assert!(o.stop_release_measurements_v1(Some(true), &mut r));
    assert!(o.query_source_facade_v1(valid, &mut r).is_some());
    assert!(
        o.release_measurements_v1(&mut r)
            .unwrap()
            .unwrap()
            .iter()
            .all(|p| p.state() == PointStateV1::Off)
    );
    assert!(o.retire_release_measurements_v1(
        &ReleaseMeasurementControlV1 {
            owner_control_allowed: Some(true),
            delivery_drained: Some(true),
            retention_released: Some(true)
        },
        &mut r
    ));
    assert_eq!(o.retention_usage().reserved, base);
    assert!(o.query_source_facade_v1(valid, &mut r).is_some());
    assert!(o.stop_source_read_context(&source, &controls(&scope), &mut r));
    assert!(o.retire_source_read_context(&source, &controls(&scope), &mut r));
    assert!(!o.is_faulted());
}

#[test]
fn given_typed_read_facades_when_an_invalid_request_is_retried_then_original_snapshot_and_cleanup_are_preserved()
 {
    let (mut o, mut r, version, scope) = ready(b"old");
    let current = facts(&scope);
    let request = ReadAcquisitionFacadeRequestV1 {
        port: ReadAcquisitionFacadePortV1::SourceCurrent {
            scope: &scope,
            facts: &current,
        },
        actor: &scope.context,
        limits: limits(),
    };
    assert!(matches!(
        o.try_acquire_read_facade_v1(
            ReadAcquisitionFacadeRequestV1 {
                limits: CaptureLimitsV1 {
                    max_nodes: 0,
                    ..limits()
                },
                ..request
            },
            &mut r
        ),
        Err(SourceReadContextErrorV1::InvalidRequest)
    ));
    let Ok(ReadAcquisitionFacadeReplyV1::Source(original)) =
        o.try_acquire_read_facade_v1(request, &mut r)
    else {
        panic!("capture retry");
    };
    let plan = o
        .prepare(
            &[DocumentEdits {
                expected: version.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 3,
                    expected: "old".into(),
                    inserted: "new".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(plan, &mut r).unwrap();
    let base = o.retention_usage().reserved;
    let query = SourceFacadeRequestV1 {
        source: SearchReadContextV1::Source(&original),
        scope: &scope,
        actor: &scope.context,
        query: SourceContextQueryV1::Line { index: 999 },
        facts: SearchReadFactsV1::Source(&current),
        limits: limits(),
    };
    assert!(matches!(
        o.try_query_source_facade_v1(query, &mut r),
        Err(SourceReadContextErrorV1::InvalidRequest)
    ));
    let Ok(SourceFacadeReplyV1::Source(reply)) = o.try_query_source_facade_v1(
        SourceFacadeRequestV1 {
            query: SourceContextQueryV1::Line { index: 0 },
            ..query
        },
        &mut r,
    ) else {
        panic!("read retry");
    };
    let SourceContextContentV1::Line(line) = reply.content else {
        panic!("line");
    };
    assert_eq!(line.source, version);
    assert_eq!(line.body(), b"old");
    assert_eq!(o.retention_usage().reserved, base);
    assert_eq!(o.source_read_context_usage().issued_lifetime, 1);
    assert_eq!(
        o.read_facade_usage(),
        ReadFacadeUsageV1 {
            busy: false,
            workspace_bytes: 0
        }
    );
    assert!(o.stop_source_read_context(&original, &controls(&scope), &mut r));
    assert!(o.retire_source_read_context(&original, &controls(&scope), &mut r));
    assert!(!o.is_faulted());
}

fn decoded_source_acquisition(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
    scope: &SourceReadScopeV1,
    original: Option<&SourceReadContextId>,
    dynamic: bool,
    unsupported: bool,
) -> Option<SourceReadContextId> {
    use ReadAcquisitionEnvelopeValueV1::{Classes, Data, Document, Object, Reference};
    let selection = [
        (
            "kind",
            Data(InertValueV1::Text(if original.is_some() {
                "retained"
            } else {
                "capture-current"
            })),
        ),
        (
            "reference",
            Reference(original.map_or(
                ReadAcquisitionSelectionV1::Current,
                ReadAcquisitionSelectionV1::Source,
            )),
        ),
    ];
    let args = [
        ("contract", Data(InertValueV1::Text("read-context/1"))),
        ("operation", Data(InertValueV1::Text("snapshot-handoff"))),
        ("document", Document(&scope.document)),
        ("data", Classes(AnnotationProjectionClassesV1::Source)),
        (
            "selection",
            Object(&selection[..if original.is_some() { 2 } else { 1 }]),
        ),
        (
            "profile",
            Data(InertValueV1::Text(if unsupported {
                "unsupported"
            } else {
                &scope.context.profile
            })),
        ),
    ];
    let header = [
        (
            if dynamic { "name" } else { "family" },
            Data(if dynamic {
                InertValueV1::Text("read-document")
            } else {
                InertValueV1::Number(1.0)
            }),
        ),
        ("variant", Data(InertValueV1::Text("snapshot-handoff"))),
        ("version", Data(InertValueV1::Number(1.0))),
        ("arguments", Object(&args)),
    ];
    let outer = [
        (
            "route",
            Data(InertValueV1::Text(if dynamic {
                "dynamic"
            } else {
                "typed"
            })),
        ),
        ("request", Object(&header)),
    ];
    let mut facts = facts(scope);
    if unsupported {
        facts.authorized = Some(false);
    }
    let mut actor = scope.context.clone();
    actor.recipient = "headless-agent".into();
    let port = original.map_or(
        ReadAcquisitionFacadePortV1::SourceCurrent {
            scope,
            facts: &facts,
        },
        |original| ReadAcquisitionFacadePortV1::SourceRetained {
            scope,
            original,
            facts: &facts,
        },
    );
    o.acquire_decoded_read_facade_v1(
        DecodedReadAcquisitionFacadeRequestV1 {
            envelope: &Object(&outer),
            native: ReadAcquisitionFacadeRequestV1 {
                port,
                actor: &actor,
                limits: limits(),
            },
        },
        r,
    )
    .map(|v| match v {
        ReadAcquisitionFacadeReplyV1::Source(id) => id,
        _ => panic!("source acquisition"),
    })
}
#[test]
fn given_native_acquisition_arguments_when_copying_retained_source_then_it_does_not_recapture_or_share_retirement()
 {
    for dynamic in [false, true] {
        let (mut o, mut r, version, scope) = ready(b"old");
        let original =
            decoded_source_acquisition(&mut o, &mut r, &scope, None, dynamic, false).unwrap();
        let plan = o
            .prepare(
                &[DocumentEdits {
                    expected: version.clone(),
                    edits: vec![vellum_engine::source::Replacement {
                        start: 0,
                        end: 3,
                        expected: "old".into(),
                        inserted: "new".into(),
                    }],
                }],
                &[],
                &mut r,
            )
            .unwrap();
        o.commit(plan, &mut r).unwrap();
        let copy =
            decoded_source_acquisition(&mut o, &mut r, &scope, Some(&original), !dynamic, false)
                .unwrap();
        assert_ne!(copy, original);
        assert!(o.stop_source_read_context(&original, &controls(&scope), &mut r));
        assert!(o.retire_source_read_context(&original, &controls(&scope), &mut r));
        let reply = o
            .query_source_read_context(
                &copy,
                &scope,
                SourceContextQueryV1::Line { index: 0 },
                &facts(&scope),
                &mut r,
            )
            .unwrap();
        let SourceContextContentV1::Line(line) = reply.content else {
            panic!("line")
        };
        assert_eq!(line.source, version);
        assert_eq!(line.body(), b"old");
        assert_eq!(
            o.read_facade_usage(),
            ReadFacadeUsageV1 {
                busy: false,
                workspace_bytes: 0
            }
        );
        assert!(!o.is_faulted());
    }
}
#[test]
fn given_unsupported_acquisition_arguments_when_loss_is_supplied_then_original_authority_and_issuance_are_preserved()
 {
    let (mut o, mut r, _, scope) = ready(b"source");
    let original = o
        .capture_source_read_context(&scope, &facts(&scope), &mut r)
        .unwrap();
    let issued = o.source_read_context_usage().issued_lifetime;
    assert!(
        decoded_source_acquisition(&mut o, &mut r, &scope, Some(&original), false, true).is_none()
    );
    assert_eq!(o.source_read_context_usage().issued_lifetime, issued);
    assert!(
        decoded_source_acquisition(&mut o, &mut r, &scope, Some(&original), true, false).is_some()
    );
    assert!(!o.is_faulted());
}

fn decoded_source_query(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
    scope: &SourceReadScopeV1,
    id: &SourceReadContextId,
    dynamic: bool,
    profile: &str,
    authorized: Option<bool>,
) -> Option<SourceFacadeReplyV1> {
    use SourceEnvelopeValueV1::{Data, Document, Object, Snapshot};
    let args = [
        ("contract", Data(InertValueV1::Text("source-query/1"))),
        ("operation", Data(InertValueV1::Text("line"))),
        ("snapshot", Snapshot(SearchReadContextV1::Source(id))),
        ("document", Document(&scope.document)),
        ("profile", Data(InertValueV1::Text(profile))),
        ("line", Data(InertValueV1::Number(0.0))),
    ];
    let header = [
        (
            if dynamic { "name" } else { "family" },
            Data(if dynamic {
                InertValueV1::Text("read-document")
            } else {
                InertValueV1::Number(1.0)
            }),
        ),
        ("variant", Data(InertValueV1::Text("line"))),
        ("version", Data(InertValueV1::Number(1.0))),
        ("arguments", Object(&args)),
    ];
    let outer = [
        (
            "route",
            Data(InertValueV1::Text(if dynamic {
                "dynamic"
            } else {
                "typed"
            })),
        ),
        ("request", Object(&header)),
    ];
    let mut current = facts(scope);
    current.authorized = authorized;
    let mut actor = scope.context.clone();
    actor.recipient = "headless-agent".into();
    o.query_decoded_source_facade_v1(
        DecodedSourceFacadeRequestV1 {
            envelope: &Object(&outer),
            actor: &actor,
            scope,
            limits: limits(),
            facts: SearchReadFactsV1::Source(&current),
        },
        r,
    )
}

#[test]
fn given_native_source_arguments_when_typed_or_dynamic_then_return_the_same_original_capture() {
    let (mut o, mut r, version, scope) = ready("é\r\nbody".as_bytes());
    let id = o
        .capture_source_read_context(&scope, &facts(&scope), &mut r)
        .unwrap();
    let reserved = o.retention_usage().reserved;
    for dynamic in [false, true] {
        let Some(SourceFacadeReplyV1::Source(reply)) = decoded_source_query(
            &mut o,
            &mut r,
            &scope,
            &id,
            dynamic,
            &scope.context.profile,
            Some(true),
        ) else {
            panic!("original native source reply")
        };
        assert_eq!(reply.context, id);
        let SourceContextContentV1::Line(line) = reply.content else {
            panic!("line")
        };
        assert_eq!(line.source, version);
        assert_eq!(line.body(), "é".as_bytes());
        assert_eq!(line.delimiter(), b"\r\n");
        assert_eq!(o.retention_usage().reserved, reserved);
        assert_eq!(
            o.read_facade_usage(),
            ReadFacadeUsageV1 {
                busy: false,
                workspace_bytes: 0
            }
        );
    }
}

#[test]
fn given_native_source_arguments_when_profile_is_unsupported_then_do_not_lend_loss_to_the_context()
{
    let (mut o, mut r, _, scope) = ready(b"body");
    let id = o
        .capture_source_read_context(&scope, &facts(&scope), &mut r)
        .unwrap();
    assert!(
        decoded_source_query(
            &mut o,
            &mut r,
            &scope,
            &id,
            false,
            "unsupported",
            Some(false)
        )
        .is_none()
    );
    assert!(
        decoded_source_query(
            &mut o,
            &mut r,
            &scope,
            &id,
            true,
            &scope.context.profile,
            Some(true)
        )
        .is_some()
    );
    assert!(!o.is_faulted());
}
fn limits() -> CaptureLimitsV1 {
    CaptureLimitsV1 {
        max_bytes: 8192,
        max_nodes: 256,
        max_depth: 16,
    }
}
#[test]
fn given_an_original_native_snapshot_when_facade_reads_after_publication_then_it_releases_only_the_capture()
 {
    let (mut o, mut r, version, scope) = ready("é\r\nbody".as_bytes());
    let id = o
        .capture_source_read_context(&scope, &facts(&scope), &mut r)
        .unwrap();
    let plan = o
        .prepare(
            &[DocumentEdits {
                expected: version.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 2,
                    expected: "é".into(),
                    inserted: "new".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(plan, &mut r).unwrap();
    let reserved = o.retention_usage().reserved;
    let issued = o.source_read_context_usage().issued_lifetime;
    let mut actor = scope.context.clone();
    actor.recipient = "automation-actor".into();
    for query in [
        SourceContextQueryV1::Range {
            start: BytePosition(0),
            end: BytePosition(2),
        },
        SourceContextQueryV1::Line { index: 0 },
    ] {
        let Some(SourceFacadeReplyV1::Source(reply)) = o.query_source_facade_v1(
            SourceFacadeRequestV1 {
                source: SearchReadContextV1::Source(&id),
                scope: &scope,
                actor: &actor,
                query,
                limits: limits(),
                facts: SearchReadFactsV1::Source(&facts(&scope)),
            },
            &mut r,
        ) else {
            panic!("native source result")
        };
        assert_eq!(reply.context, id);
        match reply.content {
            SourceContextContentV1::Range(v) => {
                assert_eq!(v.source, version);
                assert_eq!(v.bytes(), "é".as_bytes());
            }
            SourceContextContentV1::Line(v) => {
                assert_eq!(v.source, version);
                assert_eq!(v.body(), "é".as_bytes());
                assert_eq!(v.delimiter(), b"\r\n");
            }
        }
        assert_eq!(o.retention_usage().reserved, reserved);
        assert_eq!(o.source_read_context_usage().issued_lifetime, issued);
        assert_eq!(
            o.source_facade_usage(),
            SourceFacadeUsageV1 {
                busy: false,
                workspace_bytes: 0
            }
        );
    }
    assert!(!o.is_faulted());
}
#[test]
fn given_native_host_facts_when_unknown_then_withhold_and_when_lost_then_do_not_resurrect() {
    let (mut o, mut r, _, scope) = ready(b"body");
    let id = o
        .capture_source_read_context(&scope, &facts(&scope), &mut r)
        .unwrap();
    for (authorized, resources, expected) in [
        (None, Some(true), false),
        (Some(true), Some(false), false),
        (Some(true), Some(true), true),
        (Some(false), Some(false), false),
        (Some(true), Some(true), false),
    ] {
        let mut current = facts(&scope);
        current.authorized = authorized;
        current.resources_ready = resources;
        assert_eq!(
            o.query_source_facade_v1(
                SourceFacadeRequestV1 {
                    source: SearchReadContextV1::Source(&id),
                    scope: &scope,
                    actor: &scope.context,
                    query: SourceContextQueryV1::Line { index: 0 },
                    limits: limits(),
                    facts: SearchReadFactsV1::Source(&current),
                },
                &mut r
            )
            .is_some(),
            expected
        );
        assert!(!o.is_faulted());
        assert!(!o.source_facade_usage().busy);
        assert_eq!(o.source_facade_usage().workspace_bytes, 0);
    }
}
