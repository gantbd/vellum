use super::*;
fn decoded_ready() -> (
    SourceCoordinator,
    Recorder,
    SourceReadScopeV1,
    AnnotationReadContextId,
) {
    let (mut o, mut r, d, scope) = ready(b"source");
    let first = add(&mut o, &mut r, &d);
    let second = add(&mut o, &mut r, &d);
    let rows = [
        AnnotationFacetGrantV1 {
            annotation: first,
            visible: true,
            body: true,
            target: true,
        },
        AnnotationFacetGrantV1 {
            annotation: second,
            visible: true,
            body: false,
            target: false,
        },
    ];
    o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), &mut r)
        .unwrap();
    o.configure_annotation_contexts(owner_limits(), &pool_control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &scope,
            &capture_facts(&scope, &rows),
            &mut r,
        )
        .unwrap();
    let id = o
        .install_annotation_context(&cut, &scope, &current(&cut, &scope), &mut r)
        .unwrap()
        .context;
    (o, r, scope, id)
}
fn decoded_read(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
    scope: &SourceReadScopeV1,
    context: &AnnotationReadContextId,
    query: &AnnotationQueryV1,
    dynamic: bool,
    loss: bool,
) -> Option<AnnotationQueryReplyV1> {
    decoded_read_via(
        o,
        r,
        scope,
        context,
        query,
        dynamic,
        loss,
        |o, request, r| o.query_decoded_annotation_facade_v1(request, r),
    )
}
#[allow(clippy::too_many_arguments)]
fn decoded_read_via(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
    scope: &SourceReadScopeV1,
    context: &AnnotationReadContextId,
    query: &AnnotationQueryV1,
    dynamic: bool,
    loss: bool,
    dispatch: impl FnOnce(
        &mut SourceCoordinator,
        DecodedAnnotationFacadeRequestV1<'_, '_>,
        &mut Recorder,
    ) -> Option<AnnotationFacadeReplyV1>,
) -> Option<AnnotationQueryReplyV1> {
    use AnnotationEnvelopeValueV1::{Continuation, Data, Document, Object, Record, Snapshot};
    use vellum_engine::capture::{CaptureLimitsV1, InertValueV1 as V};
    let (operation, selector, limit) = match query {
        AnnotationQueryV1::Enumerate {
            limit,
            continuation,
        } => (
            "enumerate",
            continuation.as_ref().map_or(Data(V::Null), Continuation),
            *limit,
        ),
        AnnotationQueryV1::Inspect { annotation } => ("inspect", Record(annotation), 0),
        AnnotationQueryV1::TargetLocation { annotation } => {
            ("target-location", Record(annotation), 0)
        }
    };
    let args = [
        ("contract", Data(V::Text("annotation-query/1"))),
        ("operation", Data(V::Text(operation))),
        (
            "snapshot",
            Snapshot(AnnotationArgumentTargetV1::Single(context)),
        ),
        ("document", Document(&scope.document)),
        (
            "profile",
            Data(V::Text(if loss {
                "unsupported"
            } else {
                &scope.context.profile
            })),
        ),
        (
            if operation == "enumerate" {
                "continuation"
            } else {
                "annotation"
            },
            selector,
        ),
        ("limit", Data(V::Number(limit as f64))),
    ];
    let header = [
        (
            if dynamic { "name" } else { "family" },
            Data(if dynamic {
                V::Text("read-annotations")
            } else {
                V::Number(2.0)
            }),
        ),
        ("variant", Data(V::Text(operation))),
        ("version", Data(V::Number(1.0))),
        (
            "arguments",
            Object(&args[..if operation == "enumerate" { 7 } else { 6 }]),
        ),
    ];
    let outer = [
        (
            "route",
            Data(V::Text(if dynamic { "dynamic" } else { "typed" })),
        ),
        ("request", Object(&header)),
    ];
    let mut facts = eligible(context, scope);
    facts.source_readable = None;
    if loss {
        facts.authorized = Some(false);
    }
    let mut actor = scope.context.clone();
    actor.recipient = "headless-agent".into();
    dispatch(
        o,
        DecodedAnnotationFacadeRequestV1 {
            envelope: &Object(&outer),
            scope,
            actor: &actor,
            limits: CaptureLimitsV1 {
                max_bytes: 8192,
                max_nodes: 256,
                max_depth: 16,
            },
            facts: AnnotationArgumentFactsV1::Single(&facts),
        },
        r,
    )
    .map(|v| match v {
        AnnotationFacadeReplyV1::Single(v) => v,
        _ => panic!("single"),
    })
}
#[test]
fn given_discovered_annotation_queries_when_private_facets_are_withheld_then_paging_and_original_selectors_remain_guarded()
 {
    for dynamic in [false, true] {
        let (mut o, mut r, scope, id) = decoded_ready();
        let binding =
            crate::inspection_read::Reader::install(&mut o, &scope.context, false, &mut r);
        let debt = o.retention_usage();
        let mut read = |q: &AnnotationQueryV1, index| {
            decoded_read_via(
                &mut o,
                &mut r,
                &scope,
                &id,
                q,
                dynamic,
                false,
                |o, request, r| binding.annotations(o, index, request, r),
            )
        };
        let AnnotationQueryReplyV1::Page {
            records,
            continuation,
            ..
        } = read(
            &AnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: None,
            },
            0,
        )
        .unwrap()
        else {
            panic!("page")
        };
        assert_eq!(records[0].content.as_ref().unwrap().body, "secret");
        let first = records[0].reference.clone();
        assert!(
            read(
                &AnnotationQueryV1::Inspect {
                    annotation: first.clone()
                },
                1
            )
            .is_some()
        );
        assert!(read(&AnnotationQueryV1::TargetLocation { annotation: first }, 2).is_some());
        let AnnotationQueryReplyV1::Page { records, .. } = read(
            &AnnotationQueryV1::Enumerate {
                limit: 1,
                continuation,
            },
            0,
        )
        .unwrap() else {
            panic!("second page")
        };
        assert!(records[0].content.is_none());
        assert!(records[0].location.is_none());
        assert_eq!(o.retention_usage(), debt);
        binding.retire(&mut o, &mut r);
        assert!(o.stop_annotation_context(&id, &controls(&scope), &mut r));
        assert!(o.retire_annotation_context(&id, &controls(&scope), &mut r));
    }
}
#[test]
fn given_native_annotation_arguments_when_routes_and_selectors_round_trip_then_optional_facets_stay_withheld()
 {
    let (mut o, mut r, scope, id) = decoded_ready();
    let base = o.retention_usage().reserved;
    let first = AnnotationQueryV1::Enumerate {
        limit: 1,
        continuation: None,
    };
    let Some(AnnotationQueryReplyV1::Page {
        records,
        continuation,
        ..
    }) = decoded_read(&mut o, &mut r, &scope, &id, &first, false, false)
    else {
        panic!("first page")
    };
    assert_eq!(records[0].content.as_ref().unwrap().body, "secret");
    assert!(continuation.is_some());
    let record = records[0].reference.clone();
    assert!(
        decoded_read(
            &mut o,
            &mut r,
            &scope,
            &id,
            &AnnotationQueryV1::Inspect {
                annotation: record.clone()
            },
            true,
            false
        )
        .is_some()
    );
    assert!(
        decoded_read(
            &mut o,
            &mut r,
            &scope,
            &id,
            &AnnotationQueryV1::TargetLocation { annotation: record },
            false,
            false
        )
        .is_some()
    );
    let Some(AnnotationQueryReplyV1::Page {
        records,
        continuation,
        ..
    }) = decoded_read(
        &mut o,
        &mut r,
        &scope,
        &id,
        &AnnotationQueryV1::Enumerate {
            limit: 1,
            continuation,
        },
        true,
        false,
    )
    else {
        panic!("next page")
    };
    assert_eq!(records.len(), 1);
    assert!(records[0].content.is_none());
    assert!(records[0].location.is_none());
    assert!(continuation.is_none());
    assert_eq!(o.retention_usage().reserved, base);
    assert!(!o.is_faulted());
    assert_eq!(
        o.read_facade_usage(),
        ReadFacadeUsageV1 {
            busy: false,
            workspace_bytes: 0
        }
    );
}
#[test]
fn given_unsupported_annotation_arguments_when_loss_is_supplied_then_the_valid_context_is_not_poisoned()
 {
    let (mut o, mut r, scope, id) = decoded_ready();
    let q = AnnotationQueryV1::Enumerate {
        limit: 1,
        continuation: None,
    };
    assert!(decoded_read(&mut o, &mut r, &scope, &id, &q, false, true).is_none());
    assert!(decoded_read(&mut o, &mut r, &scope, &id, &q, true, false).is_some());
    assert!(!o.is_faulted());
}
#[test]
fn native_annotation_facade_keeps_retained_ownership_and_optional_facets_through_selector_round_trips()
 {
    fn run(
        o: &mut SourceCoordinator,
        r: &mut Recorder,
        scope: &SourceReadScopeV1,
        context: &AnnotationReadContextId,
        query: &AnnotationQueryV1,
    ) -> Option<AnnotationQueryReplyV1> {
        let mut facts = eligible(context, scope);
        facts.source_readable = None;
        o.query_annotation_facade_v1(
            AnnotationFacadeRequestV1 {
                port: AnnotationFacadePortV1::Single {
                    context,
                    query,
                    facts: &facts,
                },
                scope,
                actor: &scope.context,
                limits: vellum_engine::capture::CaptureLimitsV1 {
                    max_bytes: 8192,
                    max_nodes: 256,
                    max_depth: 16,
                },
            },
            r,
        )
        .map(|reply| match reply {
            AnnotationFacadeReplyV1::Single(v) => v,
            _ => panic!("single native port"),
        })
    }
    let (mut o, mut r, d, scope) = ready(b"source");
    let first = add(&mut o, &mut r, &d);
    let second = add(&mut o, &mut r, &d);
    let grants = [
        AnnotationFacetGrantV1 {
            annotation: first,
            visible: true,
            body: true,
            target: true,
        },
        AnnotationFacetGrantV1 {
            annotation: second,
            visible: true,
            body: false,
            target: false,
        },
    ];
    o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), &mut r)
        .unwrap();
    o.configure_annotation_contexts(owner_limits(), &pool_control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &scope,
            &capture_facts(&scope, &grants),
            &mut r,
        )
        .unwrap();
    let original = o
        .install_annotation_context(&cut, &scope, &current(&cut, &scope), &mut r)
        .unwrap()
        .context;
    let context = o
        .capture_retained_annotation_context(
            &original,
            AnnotationProjectionClassesV1::Annotations,
            &scope,
            &eligible(&original, &scope),
            Some(true),
            &mut r,
        )
        .unwrap()
        .context;
    assert!(o.stop_annotation_context(&original, &controls(&scope), &mut r));
    assert!(o.retire_annotation_context(&original, &controls(&scope), &mut r));
    let reserved = o.retention_usage().reserved;
    let page = AnnotationQueryV1::Enumerate {
        limit: 1,
        continuation: None,
    };
    let Some(AnnotationQueryReplyV1::Page {
        records,
        continuation,
        ..
    }) = run(&mut o, &mut r, &scope, &context, &page)
    else {
        panic!("page")
    };
    assert_eq!(records[0].content.as_ref().unwrap().body, "secret");
    let first = records[0].reference.clone();
    let next = AnnotationQueryV1::Enumerate {
        limit: 1,
        continuation,
    };
    let issued = o.annotation_context_usage().issued_lifetime;
    assert!(
        run(
            &mut o,
            &mut r,
            &scope,
            &context,
            &AnnotationQueryV1::Inspect {
                annotation: first.clone()
            }
        )
        .is_some()
    );
    assert!(
        run(
            &mut o,
            &mut r,
            &scope,
            &context,
            &AnnotationQueryV1::TargetLocation { annotation: first }
        )
        .is_some()
    );
    assert!(run(&mut o, &mut r, &scope, &context, &page).is_some());
    assert_eq!(o.annotation_context_usage().issued_lifetime, issued);
    let Some(AnnotationQueryReplyV1::Page {
        records,
        continuation,
        ..
    }) = run(&mut o, &mut r, &scope, &context, &next)
    else {
        panic!("next page")
    };
    assert!(records[0].content.is_none());
    assert!(records[0].location.is_none());
    assert!(continuation.is_none());
    assert_eq!(o.retention_usage().reserved, reserved);
    assert!(!o.read_facade_usage().busy);
    assert_eq!(o.read_facade_usage().workspace_bytes, 0);
    assert!(!o.is_faulted());
}
#[test]
fn f03_pages_inspection_and_location_use_original_context_bound_opaque_selectors() {
    let (mut o, mut r, d, s) = ready(b"source");
    let a = add(&mut o, &mut r, &d);
    let b = add(&mut o, &mut r, &d);
    let rows = [
        AnnotationFacetGrantV1 {
            annotation: a,
            visible: true,
            body: true,
            target: true,
        },
        AnnotationFacetGrantV1 {
            annotation: b,
            visible: true,
            body: false,
            target: false,
        },
    ];
    o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), &mut r)
        .unwrap();
    o.configure_annotation_contexts(owner_limits(), &pool_control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &capture_facts(&s, &rows),
            &mut r,
        )
        .unwrap();
    let id = o
        .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
        .unwrap()
        .context;
    let q = AnnotationQueryV1::Enumerate {
        limit: 1,
        continuation: None,
    };
    let AnnotationQueryReplyV1::Page {
        records,
        continuation,
        ..
    } = o
        .query_annotation_context(&id, &s, &q, &eligible(&id, &s), &mut r)
        .unwrap()
    else {
        panic!("not page")
    };
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].content.as_ref().unwrap().body, "secret");
    assert!(records[0].location.is_some());
    let first = records[0].reference.clone();
    let cursor = continuation.unwrap();
    let issued = o.annotation_context_usage().issued_lifetime;
    let AnnotationQueryReplyV1::Page {
        records,
        continuation,
        ..
    } = o
        .query_annotation_context(&id, &s, &q, &eligible(&id, &s), &mut r)
        .unwrap()
    else {
        panic!("not page")
    };
    assert_eq!(records[0].reference, first);
    assert_eq!(continuation, Some(cursor.clone()));
    assert_eq!(o.annotation_context_usage().issued_lifetime, issued);
    let inspect = AnnotationQueryV1::Inspect {
        annotation: first.clone(),
    };
    let AnnotationQueryReplyV1::Record { record, .. } = o
        .query_annotation_context(&id, &s, &inspect, &eligible(&id, &s), &mut r)
        .unwrap()
    else {
        panic!("not record")
    };
    assert_eq!(record.reference, first);
    let locate = AnnotationQueryV1::TargetLocation { annotation: first };
    assert!(matches!(
        o.query_annotation_context(&id, &s, &locate, &eligible(&id, &s), &mut r),
        Some(AnnotationQueryReplyV1::Location { .. })
    ));
    let next = AnnotationQueryV1::Enumerate {
        limit: 1,
        continuation: Some(cursor),
    };
    let AnnotationQueryReplyV1::Page {
        records,
        continuation,
        ..
    } = o
        .query_annotation_context(&id, &s, &next, &eligible(&id, &s), &mut r)
        .unwrap()
    else {
        panic!("not page")
    };
    assert_eq!(records.len(), 1);
    assert!(records[0].content.is_none());
    assert!(records[0].location.is_none());
    assert!(continuation.is_none());
    assert!(
        o.query_annotation_context(
            &id,
            &s,
            &AnnotationQueryV1::TargetLocation {
                annotation: records[0].reference.clone()
            },
            &eligible(&id, &s),
            &mut r
        )
        .is_none()
    );
}

struct Fixture {
    o: SourceCoordinator,
    r: Recorder,
    d: DocumentVersion,
    s: SourceReadScopeV1,
    id: AnnotationReadContextId,
    a: AnnotationId,
    rows: Vec<AnnotationFacetGrantV1>,
}
fn fixture(classes: AnnotationProjectionClassesV1, p: AnnotationContextLimitsV1) -> Fixture {
    let (mut o, mut r, d, s) = ready(b"source");
    let a = add(&mut o, &mut r, &d);
    let b = add(&mut o, &mut r, &d);
    let rows = vec![
        AnnotationFacetGrantV1 {
            annotation: a.clone(),
            visible: true,
            body: true,
            target: true,
        },
        AnnotationFacetGrantV1 {
            annotation: b,
            visible: true,
            body: false,
            target: false,
        },
    ];
    o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), &mut r)
        .unwrap();
    o.configure_annotation_contexts(p, &pool_control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_annotation_projection(classes, &s, &capture_facts(&s, &rows), &mut r)
        .unwrap();
    let id = o
        .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
        .unwrap()
        .context;
    Fixture {
        o,
        r,
        d,
        s,
        id,
        a,
        rows,
    }
}
fn page(
    f: &mut Fixture,
    limit: usize,
    continuation: Option<AnnotationContinuationId>,
) -> (Vec<AnnotationRecordV1>, Option<AnnotationContinuationId>) {
    let q = AnnotationQueryV1::Enumerate {
        limit,
        continuation,
    };
    let AnnotationQueryReplyV1::Page {
        records,
        continuation,
        ..
    } =
        f.o.query_annotation_context(&f.id, &f.s, &q, &eligible(&f.id, &f.s), &mut f.r)
            .unwrap()
    else {
        panic!("not page")
    };
    (records, continuation)
}

#[test]
fn page_reservation_is_all_or_none_and_equivalent_retry_still_works_at_full_issuance() {
    for maximum in [2, 3] {
        let mut p = owner_limits();
        p.max_issued = maximum;
        let mut f = fixture(AnnotationProjectionClassesV1::Annotations, p);
        let q = AnnotationQueryV1::Enumerate {
            limit: 1,
            continuation: None,
        };
        let out =
            f.o.query_annotation_context(&f.id, &f.s, &q, &eligible(&f.id, &f.s), &mut f.r);
        if maximum == 2 {
            assert!(out.is_none());
            assert_eq!(f.o.annotation_context_usage().issued_lifetime, 1);
            assert_eq!(f.o.annotation_context_usage().selectors, 1);
        } else {
            let AnnotationQueryReplyV1::Page {
                records,
                continuation,
                ..
            } = out.unwrap()
            else {
                panic!("not page")
            };
            assert_eq!(f.o.annotation_context_usage().issued_lifetime, 3);
            let (retry, cursor) = page(&mut f, 1, None);
            assert_eq!(retry[0].reference, records[0].reference);
            assert_eq!(cursor, continuation);
            let next = AnnotationQueryV1::Enumerate {
                limit: 1,
                continuation,
            };
            assert!(
                f.o.query_annotation_context(&f.id, &f.s, &next, &eligible(&f.id, &f.s), &mut f.r)
                    .is_none()
            );
            let inspect = AnnotationQueryV1::Inspect {
                annotation: records[0].reference.clone(),
            };
            assert!(
                f.o.query_annotation_context(
                    &f.id,
                    &f.s,
                    &inspect,
                    &eligible(&f.id, &f.s),
                    &mut f.r
                )
                .is_some()
            );
            assert_eq!(f.o.annotation_context_usage().issued_lifetime, 3);
        }
    }
}

#[test]
fn cursor_limit_and_context_binding_and_request_shape_precede_accepted_loss() {
    let mut f = fixture(AnnotationProjectionClassesV1::Annotations, owner_limits());
    let (records, cursor) = page(&mut f, 1, None);
    let cut =
        f.o.prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &f.s,
            &capture_facts(&f.s, &f.rows),
            &mut f.r,
        )
        .unwrap();
    let peer =
        f.o.install_annotation_context(&cut, &f.s, &current(&cut, &f.s), &mut f.r)
            .unwrap()
            .context;
    for q in [
        AnnotationQueryV1::Inspect {
            annotation: records[0].reference.clone(),
        },
        AnnotationQueryV1::Enumerate {
            limit: 1,
            continuation: cursor.clone(),
        },
    ] {
        let mut lost = eligible(&peer, &f.s);
        lost.authorized = Some(false);
        assert!(
            f.o.query_annotation_context(&peer, &f.s, &q, &lost, &mut f.r)
                .is_none()
        );
    }
    for q in [
        AnnotationQueryV1::Enumerate {
            limit: 0,
            continuation: None,
        },
        AnnotationQueryV1::Enumerate {
            limit: usize::MAX,
            continuation: None,
        },
        AnnotationQueryV1::Enumerate {
            limit: 2,
            continuation: cursor,
        },
    ] {
        let mut lost = eligible(&f.id, &f.s);
        lost.authorized = Some(false);
        assert!(
            f.o.query_annotation_context(&f.id, &f.s, &q, &lost, &mut f.r)
                .is_none()
        );
    }
    assert_eq!(f.o.annotation_context_usage().stopped, 0);
    assert!(
        f.o.query_annotation_context(
            &f.id,
            &f.s,
            &AnnotationQueryV1::Inspect {
                annotation: records[0].reference.clone()
            },
            &eligible(&f.id, &f.s),
            &mut f.r
        )
        .is_some()
    );
}

#[test]
fn inspection_retains_original_values_and_cannot_fill_unrequested_facets_from_newer_root() {
    let mut f = fixture(AnnotationProjectionClassesV1::Combined, owner_limits());
    let (records, _) = page(&mut f, 2, None);
    let change =
        f.o.prepare_compound(
            &[DocumentEdits {
                expected: f.d.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 1,
                    expected: "s".into(),
                    inserted: "N".into(),
                }],
            }],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: f.a.clone(),
                expected_content: 1,
                body: "NEW".into(),
            }],
            &mut f.r,
        )
        .unwrap();
    f.o.commit(change, &mut f.r).unwrap();
    for (index, reference) in records.iter().map(|r| r.reference.clone()).enumerate() {
        let q = AnnotationQueryV1::Inspect {
            annotation: reference,
        };
        let AnnotationQueryReplyV1::Record { record, .. } =
            f.o.query_annotation_context(&f.id, &f.s, &q, &eligible(&f.id, &f.s), &mut f.r)
                .unwrap()
        else {
            panic!("not record")
        };
        if index == 0 {
            assert_eq!(record.content.unwrap().body, "secret");
            assert_eq!(record.location.unwrap().source_revision, Some(0));
        } else {
            assert!(record.content.is_none());
            assert!(record.location.is_none());
        }
    }
}

#[test]
fn all_f03_operations_require_current_whole_projection_facts_without_conflating_unknown_with_loss()
{
    for classes in [
        AnnotationProjectionClassesV1::Annotations,
        AnnotationProjectionClassesV1::Combined,
    ] {
        for operation in 0..3 {
            for index in 0..12 {
                for value in [None, Some(false)] {
                    let mut f = fixture(classes, owner_limits());
                    let (records, _) = page(&mut f, 1, None);
                    let q = match operation {
                        0 => AnnotationQueryV1::Enumerate {
                            limit: 1,
                            continuation: None,
                        },
                        1 => AnnotationQueryV1::Inspect {
                            annotation: records[0].reference.clone(),
                        },
                        _ => AnnotationQueryV1::TargetLocation {
                            annotation: records[0].reference.clone(),
                        },
                    };
                    let mut facts = eligible(&f.id, &f.s);
                    match index {
                        0 => facts.authorized = value,
                        1 => facts.scope_allowed = value,
                        2 => facts.continuous = value,
                        3 => facts.live = value,
                        4 => facts.not_stopped = value,
                        5 => facts.annotation_readable = value,
                        6 => facts.source_readable = value,
                        7 => facts.projection_current = value,
                        8 => facts.retained = value,
                        9 => facts.resources_ready = value,
                        10 => facts.reply_ready = value,
                        _ => facts.transfer_ready = value,
                    }
                    let irrelevant =
                        index == 6 && classes == AnnotationProjectionClassesV1::Annotations;
                    assert_eq!(
                        f.o.query_annotation_context(&f.id, &f.s, &q, &facts, &mut f.r)
                            .is_some(),
                        irrelevant
                    );
                    let lost = index < 9 && !irrelevant && value == Some(false);
                    assert_eq!(
                        f.o.query_annotation_context(
                            &f.id,
                            &f.s,
                            &q,
                            &eligible(&f.id, &f.s),
                            &mut f.r
                        )
                        .is_none(),
                        lost
                    );
                }
            }
        }
    }
}

#[test]
fn retirement_removes_all_selector_lookup_paths_without_refunding_context_or_selector_issuance() {
    let mut f = fixture(AnnotationProjectionClassesV1::Annotations, owner_limits());
    let (records, cursor) = page(&mut f, 1, None);
    let issued = f.o.annotation_context_usage().issued_lifetime;
    let reserved = f.o.retention_usage().reserved;
    f.o.stop_annotation_context(&f.id, &controls(&f.s), &mut f.r);
    assert!(f.o.retire_annotation_context(&f.id, &controls(&f.s), &mut f.r));
    assert_eq!(f.o.annotation_context_usage().selectors, 0);
    assert_eq!(f.o.annotation_context_usage().issued_lifetime, issued);
    assert_eq!(f.o.retention_usage().reserved, reserved);
    let cut =
        f.o.prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &f.s,
            &capture_facts(&f.s, &f.rows),
            &mut f.r,
        )
        .unwrap();
    let peer =
        f.o.install_annotation_context(&cut, &f.s, &current(&cut, &f.s), &mut f.r)
            .unwrap()
            .context;
    for q in [
        AnnotationQueryV1::Inspect {
            annotation: records[0].reference.clone(),
        },
        AnnotationQueryV1::Enumerate {
            limit: 1,
            continuation: cursor,
        },
    ] {
        assert!(
            f.o.query_annotation_context(&peer, &f.s, &q, &eligible(&peer, &f.s), &mut f.r)
                .is_none()
        );
    }
    assert_eq!(records[0].content.as_ref().unwrap().body, "secret");
}

#[test]
fn f03_capture_is_optional_bounded_redacted_and_never_authored_publication() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let mut f = fixture(AnnotationProjectionClassesV1::Annotations, owner_limits());
        f.r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let generation = f.o.generation();
        let (records, _) = page(&mut f, 1, None);
        assert_eq!(
            format!("{:?}", records[0]),
            "AnnotationRecordV1(<redacted>)"
        );
        assert_eq!(
            format!("{:?}", records[0].reference),
            "AnnotationRecordId(<opaque>)"
        );
        let q = AnnotationQueryV1::Inspect {
            annotation: records[0].reference.clone(),
        };
        let reply =
            f.o.query_annotation_context(&f.id, &f.s, &q, &eligible(&f.id, &f.s), &mut f.r)
                .unwrap();
        assert_eq!(format!("{reply:?}"), "AnnotationQueryReplyV1(<redacted>)");
        assert_eq!(format!("{q:?}"), "AnnotationQueryV1(<redacted>)");
        assert_eq!(f.o.generation(), generation);
        if capacity == Some(128) {
            let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.boundary == Boundary::AnnotationSelectorRegistration)
                    .count(),
                1
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.boundary == Boundary::AnnotationContextQuery)
                    .count(),
                2
            );
            assert!(!events.iter().any(|e| e.boundary == Boundary::Publication));
            assert!(events.iter().all(|e| e.service_time.is_none()));
            assert!(!format!("{events:?}").contains("secret"));
        } else if capacity == Some(1) {
            assert!(f.r.dropped() > 0);
        } else {
            assert_eq!(f.r.queued(), 0);
        }
    }
}

#[test]
fn empty_visible_projection_returns_a_complete_empty_page_even_when_all_selector_issuance_is_spent()
{
    let (mut o, mut r, d, s) = ready(b"source");
    let a = add(&mut o, &mut r, &d);
    let rows = [AnnotationFacetGrantV1 {
        annotation: a,
        visible: false,
        body: false,
        target: false,
    }];
    o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), &mut r)
        .unwrap();
    let mut p = owner_limits();
    p.max_issued = 1;
    p.max_selectors = 1;
    p.max_row_bytes = 0;
    o.configure_annotation_contexts(p, &pool_control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &capture_facts(&s, &rows),
            &mut r,
        )
        .unwrap();
    let id = o
        .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
        .unwrap()
        .context;
    let q = AnnotationQueryV1::Enumerate {
        limit: 1,
        continuation: None,
    };
    let AnnotationQueryReplyV1::Page {
        records,
        continuation,
        ..
    } = o
        .query_annotation_context(&id, &s, &q, &eligible(&id, &s), &mut r)
        .unwrap()
    else {
        panic!("not page")
    };
    assert!(records.is_empty());
    assert!(continuation.is_none());
    assert_eq!(o.annotation_context_usage().issued_lifetime, 1);
    let mut lost = eligible(&id, &s);
    lost.retained = Some(false);
    assert!(
        o.query_annotation_context(&id, &s, &q, &lost, &mut r)
            .is_none()
    );
    assert_eq!(o.annotation_context_usage().stopped, 1);
}
