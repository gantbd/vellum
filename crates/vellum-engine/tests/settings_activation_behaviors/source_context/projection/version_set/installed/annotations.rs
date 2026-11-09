use super::*;
#[test]
fn native_coherent_annotation_facade_uses_the_original_member_class_and_complete_parent_facts() {
    let mut f = owner(owner_limits(), 2);
    let query = VersionSetAnnotationQueryV1::Enumerate {
        limit: 1,
        continuation: None,
    };
    let mut source = current_member(&f.a);
    source.annotation_readable = None;
    let mut annotation = current_member(&f.b);
    annotation.source_readable = None;
    let facts = TrustedVersionSetContextFactsV1 {
        context: &f.acq.context,
        members: &[source, annotation],
    };
    let before = f.o.version_set_context_usage().issued_lifetime;
    let reserved = f.o.retention_usage().reserved;
    let limits = vellum_engine::capture::CaptureLimitsV1 {
        max_bytes: 8192,
        max_nodes: 256,
        max_depth: 16,
    };
    assert!(
        f.o.query_annotation_facade_v1(
            AnnotationFacadeRequestV1 {
                port: AnnotationFacadePortV1::VersionSet {
                    member: &f.acq.participants[0].snapshot,
                    query: &query,
                    facts: &facts
                },
                scope: &f.a,
                actor: &f.a.context,
                limits,
            },
            &mut f.r
        )
        .is_none()
    );
    assert_eq!(f.o.version_set_context_usage().issued_lifetime, before);
    let Some(AnnotationFacadeReplyV1::VersionSet(VersionSetAnnotationReplyV1::Page {
        snapshot,
        records,
        continuation,
    })) = f.o.query_annotation_facade_v1(
        AnnotationFacadeRequestV1 {
            port: AnnotationFacadePortV1::VersionSet {
                member: &f.acq.participants[1].snapshot,
                query: &query,
                facts: &facts,
            },
            scope: &f.b,
            actor: &f.b.context,
            limits,
        },
        &mut f.r,
    )
    else {
        panic!("annotation member")
    };
    assert_eq!(snapshot, f.acq.participants[1].snapshot);
    assert_eq!(records.len(), 1);
    assert!(continuation.is_some());
    assert_eq!(f.o.retention_usage().reserved, reserved);
    assert!(!f.o.read_facade_usage().busy);
    assert_eq!(f.o.read_facade_usage().workspace_bytes, 0);
    assert!(!f.o.is_faulted());
}
struct Fixture {
    o: SourceCoordinator,
    r: Recorder,
    a: SourceReadScopeV1,
    b: SourceReadScopeV1,
    acq: VersionSetAcquisitionV1,
    grants: Vec<AnnotationFacetGrantV1>,
}
fn owner(p: VersionSetContextLimitsV1, count: usize) -> Fixture {
    let (mut o, mut r, a, b) = base();
    let d = o.read(&b.document, &mut r).unwrap().version;
    let mut rows = Vec::new();
    for _ in 0..count {
        rows.push(AnnotationFacetGrantV1 {
            annotation: add(&mut o, &mut r, &d),
            visible: true,
            body: true,
            target: true,
        });
    }
    o.configure_version_set_preparation("engine", "local", limits(), &pool_control(), &mut r)
        .unwrap();
    o.configure_version_set_contexts(p, &pool_control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_version_set(
            &[
                VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Source,
                },
                VersionSetParticipantV1 {
                    scope: &b,
                    data: AnnotationProjectionClassesV1::Annotations,
                },
            ],
            &[capture_facts(&a, &[]), capture_facts(&b, &rows)],
            &mut r,
        )
        .unwrap();
    let acq = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &[current_member(&a), current_member(&b)],
            },
            &mut r,
        )
        .unwrap();
    Fixture {
        o,
        r,
        a,
        b,
        acq,
        grants: rows,
    }
}
fn query(f: &mut Fixture, q: &VersionSetAnnotationQueryV1) -> Option<VersionSetAnnotationReplyV1> {
    f.o.query_version_set_annotations(
        &f.acq.participants[1].snapshot,
        q,
        &TrustedVersionSetContextFactsV1 {
            context: &f.acq.context,
            members: &[current_member(&f.a), current_member(&f.b)],
        },
        &mut f.r,
    )
}
#[test]
fn page_retry_reuses_member_bound_ids_at_full_issuance_and_retirement_removes_all_lookups() {
    let mut p = owner_limits();
    p.max_issued = 6;
    p.max_selectors = 6;
    let mut f = owner(p, 2);
    let q = VersionSetAnnotationQueryV1::Enumerate {
        limit: 1,
        continuation: None,
    };
    let VersionSetAnnotationReplyV1::Page {
        records,
        continuation,
        ..
    } = query(&mut f, &q).unwrap()
    else {
        panic!("wrong product")
    };
    let first = records[0].reference.clone();
    let cursor = continuation.unwrap();
    let next = VersionSetAnnotationQueryV1::Enumerate {
        limit: 1,
        continuation: Some(cursor.clone()),
    };
    let VersionSetAnnotationReplyV1::Page {
        records: second,
        continuation,
        ..
    } = query(&mut f, &next).unwrap()
    else {
        panic!("wrong product")
    };
    assert!(continuation.is_none());
    assert_ne!(first, second[0].reference);
    assert_eq!(f.o.version_set_context_usage().issued_lifetime, 6);
    let VersionSetAnnotationReplyV1::Page {
        records: retry,
        continuation,
        ..
    } = query(&mut f, &q).unwrap()
    else {
        panic!("wrong product")
    };
    assert_eq!(retry[0].reference, first);
    assert_eq!(continuation, Some(cursor));
    assert_eq!(f.o.version_set_context_usage().issued_lifetime, 6);
    assert!(f.o.stop_version_set_context(
        VersionSetControlTargetV1::Member(&f.acq.participants[1].snapshot),
        &[controls(&f.a), controls(&f.b)],
        &mut f.r
    ));
    assert!(f.o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&f.acq.context),
        &[controls(&f.a), controls(&f.b)],
        &mut f.r
    ));
    assert_eq!(f.o.version_set_context_usage().query_selectors, 0);
    assert_eq!(f.o.version_set_context_usage().issued_lifetime, 6);
    assert!(
        query(
            &mut f,
            &VersionSetAnnotationQueryV1::Inspect { annotation: first }
        )
        .is_none()
    );
    assert_eq!(records[0].content.as_ref().unwrap().body, "secret");
}
#[test]
fn cursor_limit_and_cross_parent_record_guards_refuse_without_observing_unrelated_loss() {
    let mut f = owner(owner_limits(), 2);
    let VersionSetAnnotationReplyV1::Page {
        records,
        continuation,
        ..
    } = query(
        &mut f,
        &VersionSetAnnotationQueryV1::Enumerate {
            limit: 1,
            continuation: None,
        },
    )
    .unwrap()
    else {
        panic!("wrong product")
    };
    let mut loss = [current_member(&f.a), current_member(&f.b)];
    loss[0].source_readable = Some(false);
    assert!(
        f.o.query_version_set_annotations(
            &f.acq.participants[1].snapshot,
            &VersionSetAnnotationQueryV1::Enumerate {
                limit: 2,
                continuation
            },
            &TrustedVersionSetContextFactsV1 {
                context: &f.acq.context,
                members: &loss
            },
            &mut f.r
        )
        .is_none()
    );
    let cut =
        f.o.prepare_version_set(
            &[
                VersionSetParticipantV1 {
                    scope: &f.a,
                    data: AnnotationProjectionClassesV1::Source,
                },
                VersionSetParticipantV1 {
                    scope: &f.b,
                    data: AnnotationProjectionClassesV1::Annotations,
                },
            ],
            &[capture_facts(&f.a, &[]), capture_facts(&f.b, &f.grants)],
            &mut f.r,
        )
        .unwrap();
    let peer =
        f.o.install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &[current_member(&f.a), current_member(&f.b)],
            },
            &mut f.r,
        )
        .unwrap();
    assert!(
        f.o.query_version_set_annotations(
            &peer.participants[1].snapshot,
            &VersionSetAnnotationQueryV1::Inspect {
                annotation: records[0].reference.clone()
            },
            &TrustedVersionSetContextFactsV1 {
                context: &peer.context,
                members: &loss
            },
            &mut f.r
        )
        .is_none()
    );
    assert!(
        f.o.query_version_set_annotations(
            &f.acq.participants[0].snapshot,
            &VersionSetAnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: None
            },
            &TrustedVersionSetContextFactsV1 {
                context: &f.acq.context,
                members: &loss
            },
            &mut f.r
        )
        .is_none()
    );
    assert_eq!(f.o.version_set_context_usage().stopped, 0);
    assert!(
        query(
            &mut f,
            &VersionSetAnnotationQueryV1::Inspect {
                annotation: records[0].reference.clone()
            }
        )
        .is_some()
    );
}
#[test]
fn original_facets_survive_newer_authoring_and_preparation_pool_retirement() {
    let mut f = owner(owner_limits(), 1);
    let p =
        f.o.prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: f.grants[0].annotation.clone(),
                expected_content: 1,
                body: "UPDATED".into(),
            }],
            &mut f.r,
        )
        .unwrap();
    f.o.commit(p, &mut f.r).unwrap();
    assert_eq!(
        f.o.retire_version_set_preparation(&pool_control(), &mut f.r),
        Ok(true)
    );
    let VersionSetAnnotationReplyV1::Page { records, .. } = query(
        &mut f,
        &VersionSetAnnotationQueryV1::Enumerate {
            limit: 1,
            continuation: None,
        },
    )
    .unwrap() else {
        panic!("wrong product")
    };
    assert_eq!(records[0].content.as_ref().unwrap().body, "secret");
    assert_eq!(records[0].content.as_ref().unwrap().version, 1);
    assert!(records[0].location.is_some());
}
#[test]
fn every_annotation_form_revalidates_required_peers_and_unknown_facts_do_not_invalidate() {
    for kind in 0..3 {
        for value in [None, Some(false)] {
            let mut f = owner(owner_limits(), 1);
            let VersionSetAnnotationReplyV1::Page { records, .. } = query(
                &mut f,
                &VersionSetAnnotationQueryV1::Enumerate {
                    limit: 1,
                    continuation: None,
                },
            )
            .unwrap() else {
                panic!("wrong product")
            };
            let q = match kind {
                0 => VersionSetAnnotationQueryV1::Enumerate {
                    limit: 1,
                    continuation: None,
                },
                1 => VersionSetAnnotationQueryV1::Inspect {
                    annotation: records[0].reference.clone(),
                },
                _ => VersionSetAnnotationQueryV1::TargetLocation {
                    annotation: records[0].reference.clone(),
                },
            };
            let mut members = [current_member(&f.a), current_member(&f.b)];
            members[0].source_readable = value;
            assert!(
                f.o.query_version_set_annotations(
                    &f.acq.participants[1].snapshot,
                    &q,
                    &TrustedVersionSetContextFactsV1 {
                        context: &f.acq.context,
                        members: &members
                    },
                    &mut f.r
                )
                .is_none()
            );
            assert_eq!(query(&mut f, &q).is_some(), value.is_none());
        }
    }
}
#[test]
fn empty_annotation_page_needs_no_new_selector_even_when_the_issuer_is_full() {
    let mut p = owner_limits();
    p.max_issued = 3;
    p.max_selectors = 3;
    let mut f = owner(p, 0);
    let VersionSetAnnotationReplyV1::Page {
        records,
        continuation,
        ..
    } = query(
        &mut f,
        &VersionSetAnnotationQueryV1::Enumerate {
            limit: 1,
            continuation: None,
        },
    )
    .unwrap()
    else {
        panic!("wrong product")
    };
    assert!(records.is_empty());
    assert!(continuation.is_none());
    assert_eq!(f.o.version_set_context_usage().query_selectors, 0);
    assert_eq!(f.o.version_set_context_usage().issued_lifetime, 3);
}
#[test]
fn annotation_query_capture_is_optional_bounded_content_free_and_not_authored_publication() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(64), Some(1)] {
        let mut f = owner(owner_limits(), 1);
        f.r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let generation = f.o.generation();
        let VersionSetAnnotationReplyV1::Page { records, .. } = query(
            &mut f,
            &VersionSetAnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: None,
            },
        )
        .unwrap() else {
            panic!("wrong product")
        };
        assert_eq!(
            format!("{:?}", records[0]),
            "VersionSetAnnotationRecordV1(<redacted>)"
        );
        assert!(
            query(
                &mut f,
                &VersionSetAnnotationQueryV1::Inspect {
                    annotation: records[0].reference.clone()
                }
            )
            .is_some()
        );
        assert!(
            query(
                &mut f,
                &VersionSetAnnotationQueryV1::TargetLocation {
                    annotation: records[0].reference.clone()
                }
            )
            .is_some()
        );
        assert_eq!(f.o.generation(), generation);
        if capacity == Some(64) {
            let events: Vec<_> = std::iter::from_fn(|| f.r.pop_front()).collect();
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.boundary == Boundary::VersionSetAnnotationQuery)
                    .count(),
                3
            );
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.boundary == Boundary::VersionSetAnnotationSelectorRegistration)
                    .count(),
                1
            );
            assert!(
                events
                    .iter()
                    .all(|e| e.service_time.is_none() && e.boundary != Boundary::Publication)
            );
            assert!(!format!("{events:?}").contains("secret"));
        } else if capacity == Some(1) {
            assert!(f.r.dropped() > 0);
        } else {
            assert_eq!(f.r.queued(), 0);
        }
    }
}
#[test]
fn annotation_member_page_inspection_and_location_use_one_original_all_peer_gated_projection() {
    let (mut o, mut r, a, b) = base();
    let d = o.read(&b.document, &mut r).unwrap().version;
    let annotation = add(&mut o, &mut r, &d);
    let rows = vec![AnnotationFacetGrantV1 {
        annotation,
        visible: true,
        body: true,
        target: true,
    }];
    o.configure_version_set_preparation("engine", "local", limits(), &pool_control(), &mut r)
        .unwrap();
    o.configure_version_set_contexts(owner_limits(), &pool_control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_version_set(
            &[
                VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Source,
                },
                VersionSetParticipantV1 {
                    scope: &b,
                    data: AnnotationProjectionClassesV1::Annotations,
                },
            ],
            &[capture_facts(&a, &[]), capture_facts(&b, &rows)],
            &mut r,
        )
        .unwrap();
    let members = [current_member(&a), current_member(&b)];
    let acq = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members,
            },
            &mut r,
        )
        .unwrap();
    let f = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &members,
    };
    let id = &acq.participants[1].snapshot;
    let reply = o
        .query_version_set_annotations(
            id,
            &VersionSetAnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: None,
            },
            &f,
            &mut r,
        )
        .unwrap();
    let VersionSetAnnotationReplyV1::Page {
        records,
        continuation,
        ..
    } = reply
    else {
        panic!("wrong product")
    };
    assert!(continuation.is_none());
    assert_eq!(records[0].content.as_ref().unwrap().body, "secret");
    let reference = records[0].reference.clone();
    assert!(matches!(
        o.query_version_set_annotations(
            id,
            &VersionSetAnnotationQueryV1::Inspect {
                annotation: reference.clone()
            },
            &f,
            &mut r
        ),
        Some(VersionSetAnnotationReplyV1::Record { .. })
    ));
    assert!(matches!(
        o.query_version_set_annotations(
            id,
            &VersionSetAnnotationQueryV1::TargetLocation {
                annotation: reference.clone()
            },
            &f,
            &mut r
        ),
        Some(VersionSetAnnotationReplyV1::Location { .. })
    ));
    let mut loss = [current_member(&a), current_member(&b)];
    loss[0].source_readable = Some(false);
    assert!(
        o.query_version_set_annotations(
            id,
            &VersionSetAnnotationQueryV1::Inspect {
                annotation: reference
            },
            &TrustedVersionSetContextFactsV1 {
                context: &acq.context,
                members: &loss
            },
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.version_set_context_usage().stopped, 1);
}
