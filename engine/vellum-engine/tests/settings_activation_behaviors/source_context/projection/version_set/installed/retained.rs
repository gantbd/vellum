use super::*;
#[test]
fn omitted_peer_loss_precedes_full_pool_and_preparation_pressure_but_unknown_does_not_invalidate() {
    for value in [None, Some(false)] {
        let mut p = owner_limits();
        p.max_contexts = 1;
        let (mut o, mut r, a, b, cut) = fixture(p);
        let members = [current_member(&a), current_member(&b)];
        let original = o
            .install_version_set(
                &cut,
                &TrustedPreparedVersionSetFactsV1 {
                    projection: &cut,
                    members: &members,
                },
                &mut r,
            )
            .unwrap();
        let mut fresh = [current_member(&a), current_member(&b)];
        fresh[1].annotation_readable = value;
        assert!(
            o.capture_retained_version_set(
                &original.context,
                &[VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Source
                }],
                &TrustedVersionSetContextFactsV1 {
                    context: &original.context,
                    members: &fresh
                },
                Some(false),
                &mut r
            )
            .is_none()
        );
        assert_eq!(
            o.version_set_context_usage().stopped,
            usize::from(value == Some(false))
        );
        assert_eq!(o.version_set_context_usage().issued_lifetime, 3);
        assert_eq!(o.version_set_context_usage().contexts, 1);
    }
}
#[test]
fn duplicate_scope_unknown_member_and_widening_refuse_without_poisoning_the_original() {
    let (mut o, mut r, a, b, cut) = fixture(owner_limits());
    let members = [current_member(&a), current_member(&b)];
    let original = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members,
            },
            &mut r,
        )
        .unwrap();
    let mut lost = [current_member(&a), current_member(&b)];
    lost[1].authorized = Some(false);
    let f = TrustedVersionSetContextFactsV1 {
        context: &original.context,
        members: &lost,
    };
    assert!(
        o.capture_retained_version_set(
            &original.context,
            &[
                VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Source
                },
                VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Source
                }
            ],
            &f,
            Some(true),
            &mut r
        )
        .is_none()
    );
    assert!(
        o.capture_retained_version_set(
            &original.context,
            &[VersionSetParticipantV1 {
                scope: &a,
                data: AnnotationProjectionClassesV1::Combined
            }],
            &f,
            Some(true),
            &mut r
        )
        .is_none()
    );
    let d = o.create(b"unknown peer", &mut r).unwrap();
    let mut unknown = a.clone();
    unknown.document = d.document;
    assert!(
        o.capture_retained_version_set(
            &original.context,
            &[VersionSetParticipantV1 {
                scope: &unknown,
                data: AnnotationProjectionClassesV1::Source
            }],
            &f,
            Some(true),
            &mut r
        )
        .is_none()
    );
    let mut wrong = a.clone();
    wrong.policy.push('x');
    assert!(
        o.capture_retained_version_set(
            &original.context,
            &[VersionSetParticipantV1 {
                scope: &wrong,
                data: AnnotationProjectionClassesV1::Source
            }],
            &f,
            Some(true),
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.version_set_context_usage().stopped, 0);
    let good = TrustedVersionSetContextFactsV1 {
        context: &original.context,
        members: &members,
    };
    for ready in [None, Some(false)] {
        assert!(
            o.capture_retained_version_set(
                &original.context,
                &[VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Source
                }],
                &good,
                ready,
                &mut r
            )
            .is_none()
        );
    }
    assert_eq!(o.version_set_context_usage().contexts, 1);
}
#[test]
fn fresh_record_selectors_cannot_cross_copies_and_removed_peers_no_longer_gate_a_copy() {
    let (mut o, mut r, a, b) = base();
    let d = o.read(&b.document, &mut r).unwrap().version;
    let anno = add(&mut o, &mut r, &d);
    let rows = grants(&anno);
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
                    data: AnnotationProjectionClassesV1::Combined,
                },
            ],
            &[capture_facts(&a, &[]), capture_facts(&b, &rows)],
            &mut r,
        )
        .unwrap();
    let members = [current_member(&a), current_member(&b)];
    let original = o
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
        context: &original.context,
        members: &members,
    };
    let VersionSetAnnotationReplyV1::Page { records, .. } = o
        .query_version_set_annotations(
            &original.participants[1].snapshot,
            &VersionSetAnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: None,
            },
            &f,
            &mut r,
        )
        .unwrap()
    else {
        panic!("wrong product")
    };
    let copy = o
        .capture_retained_version_set(
            &original.context,
            &[VersionSetParticipantV1 {
                scope: &b,
                data: AnnotationProjectionClassesV1::Annotations,
            }],
            &f,
            Some(true),
            &mut r,
        )
        .unwrap();
    o.dispose(&a.document, &mut r).unwrap();
    let mut member = current_member(&b);
    member.source_readable = Some(false);
    let new_members = [member];
    let cf = TrustedVersionSetContextFactsV1 {
        context: &copy.context,
        members: &new_members,
    };
    assert!(
        o.query_version_set_annotations(
            &copy.participants[0].snapshot,
            &VersionSetAnnotationQueryV1::Inspect {
                annotation: records[0].reference.clone()
            },
            &cf,
            &mut r
        )
        .is_none()
    );
    let VersionSetAnnotationReplyV1::Page {
        records: copied, ..
    } = o
        .query_version_set_annotations(
            &copy.participants[0].snapshot,
            &VersionSetAnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: None,
            },
            &cf,
            &mut r,
        )
        .unwrap()
    else {
        panic!("wrong product")
    };
    assert_eq!(copied[0].content.as_ref().unwrap().body, "secret");
    assert_ne!(copied[0].reference, records[0].reference);
    assert!(
        o.capture_retained_version_set(
            &copy.context,
            &[VersionSetParticipantV1 {
                scope: &b,
                data: AnnotationProjectionClassesV1::Combined
            }],
            &cf,
            Some(true),
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.version_set_context_usage().holds, 0);
}
#[test]
fn a_retained_set_source_drives_original_search_but_never_authorizes_stale_replacement() {
    use crate::source_context::search::{qfacts, request, search_profile};
    let (mut o, mut r, a, b, cut) = fixture(owner_limits());
    let members = [current_member(&a), current_member(&b)];
    let original = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members,
            },
            &mut r,
        )
        .unwrap();
    let copy = o
        .capture_retained_version_set(
            &original.context,
            &[VersionSetParticipantV1 {
                scope: &a,
                data: AnnotationProjectionClassesV1::Source,
            }],
            &TrustedVersionSetContextFactsV1 {
                context: &original.context,
                members: &members,
            },
            Some(true),
            &mut r,
        )
        .unwrap();
    o.stop_version_set_context(
        VersionSetControlTargetV1::Context(&original.context),
        &[controls(&a), controls(&b)],
        &mut r,
    );
    assert!(o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&original.context),
        &[controls(&a), controls(&b)],
        &mut r
    ));
    let d = o.read(&a.document, &mut r).unwrap().version;
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: d,
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 5,
                    expected: "first".into(),
                    inserted: "new".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let parent = TrustedVersionSetContextFactsV1 {
        context: &copy.context,
        members: &[current_member(&a)],
    };
    let sf = TrustedVersionSetSearchFactsV1 {
        member: &copy.participants[0].snapshot,
        scope: &a,
        parent: &parent,
    };
    let q = request("first");
    let result = o
        .query_managed_search(
            SearchReadContextV1::VersionSet(sf.member),
            &a,
            &q,
            SearchReadFactsV1::VersionSet(&sf),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    let command = SourceSearchConsumerV1::ReplaceAll {
        result,
        replacement: "Z".into(),
    };
    let cf = TrustedSourceSearchConsumerFactsV1 {
        scope: &a,
        command: &command,
        mutation_allowed: Some(true),
        resources_ready: Some(true),
    };
    let id = o.issue_invocation(&mut r).unwrap();
    o.submit_managed_search_consumer(
        &id,
        &command,
        &a,
        SearchReadFactsV1::VersionSet(&sf),
        &qfacts(&q),
        &cf,
        &mut r,
    )
    .unwrap();
    o.run_managed_search_consumer(
        &id,
        &a,
        SearchReadFactsV1::VersionSet(&sf),
        &qfacts(&q),
        &cf,
        &mut r,
    )
    .unwrap();
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::Rejected(CoordinatorError::Stale)
    ));
    assert_eq!(o.read(&a.document, &mut r).unwrap().source.text(), "new");
    assert_eq!(o.version_set_context_usage().holds, 1);
}
#[test]
fn retained_set_capture_is_optional_bounded_redacted_and_not_authored_publication() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(64), Some(1)] {
        let (mut o, mut r, a, b, cut) = fixture(owner_limits());
        let members = [current_member(&a), current_member(&b)];
        let original = o
            .install_version_set(
                &cut,
                &TrustedPreparedVersionSetFactsV1 {
                    projection: &cut,
                    members: &members,
                },
                &mut r,
            )
            .unwrap();
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let generation = o.generation();
        let reserved = o.retention_usage().reserved;
        let copy = o
            .capture_retained_version_set(
                &original.context,
                &[VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Source,
                }],
                &TrustedVersionSetContextFactsV1 {
                    context: &original.context,
                    members: &members,
                },
                Some(true),
                &mut r,
            )
            .unwrap();
        assert!(
            o.query_version_set_source(
                &copy.participants[0].snapshot,
                SourceContextQueryV1::Line { index: 0 },
                &TrustedVersionSetContextFactsV1 {
                    context: &copy.context,
                    members: &[current_member(&a)]
                },
                &mut r
            )
            .is_some()
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.retention_usage().reserved, reserved);
        if capacity == Some(64) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.boundary == Boundary::VersionSetRetainedCapture)
                    .count(),
                1
            );
            assert!(
                events
                    .iter()
                    .all(|e| e.service_time.is_none() && e.boundary != Boundary::Publication)
            );
            assert!(!format!("{events:?}").contains("first"));
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}
#[test]
fn retained_sets_copy_ordered_old_subsets_and_remain_independent_of_original_retirement() {
    let (mut o, mut r, a, b, cut) = fixture(owner_limits());
    let members = [current_member(&a), current_member(&b)];
    let original = o
        .install_version_set(
            &cut,
            &TrustedPreparedVersionSetFactsV1 {
                projection: &cut,
                members: &members,
            },
            &mut r,
        )
        .unwrap();
    let d = o.read(&a.document, &mut r).unwrap().version;
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: d,
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 5,
                    expected: "first".into(),
                    inserted: "new".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    o.retire_version_set_preparation(&pool_control(), &mut r)
        .unwrap();
    let copy = o
        .capture_retained_version_set(
            &original.context,
            &[
                VersionSetParticipantV1 {
                    scope: &b,
                    data: AnnotationProjectionClassesV1::Annotations,
                },
                VersionSetParticipantV1 {
                    scope: &a,
                    data: AnnotationProjectionClassesV1::Source,
                },
            ],
            &TrustedVersionSetContextFactsV1 {
                context: &original.context,
                members: &members,
            },
            Some(true),
            &mut r,
        )
        .unwrap();
    assert_eq!(copy.participants[0].document, b.document);
    assert_eq!(copy.participants[1].document, a.document);
    assert_eq!(
        copy.participants[0].data,
        AnnotationProjectionClassesV1::Annotations
    );
    assert_eq!(o.version_set_context_usage().holds, 0);
    assert!(o.stop_version_set_context(
        VersionSetControlTargetV1::Context(&original.context),
        &[controls(&a), controls(&b)],
        &mut r
    ));
    assert!(o.retire_version_set_context(
        VersionSetControlTargetV1::Context(&original.context),
        &[controls(&a), controls(&b)],
        &mut r
    ));
    let f = TrustedVersionSetContextFactsV1 {
        context: &copy.context,
        members: &[current_member(&b), current_member(&a)],
    };
    let reply = o
        .query_version_set_source(
            &copy.participants[1].snapshot,
            SourceContextQueryV1::Range {
                start: BytePosition(0),
                end: BytePosition(5),
            },
            &f,
            &mut r,
        )
        .unwrap();
    let SourceContextContentV1::Range(value) = reply.content else {
        panic!("wrong product")
    };
    assert_eq!(value.bytes(), b"first");
    assert_eq!(o.version_set_context_usage().source_bytes, 5);
}
