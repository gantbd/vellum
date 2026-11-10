use super::*;
fn install_classes(
    o: &mut SourceCoordinator,
    r: &mut Recorder,
    a: &SourceReadScopeV1,
    b: &SourceReadScopeV1,
) -> VersionSetAcquisitionV1 {
    o.configure_version_set_preparation("engine", "local", limits(), &pool_control(), r)
        .unwrap();
    o.configure_version_set_contexts(owner_limits(), &pool_control(), r)
        .unwrap();
    let cut = o
        .prepare_version_set(
            &[
                VersionSetParticipantV1 {
                    scope: a,
                    data: AnnotationProjectionClassesV1::Source,
                },
                VersionSetParticipantV1 {
                    scope: b,
                    data: AnnotationProjectionClassesV1::Annotations,
                },
            ],
            &[capture_facts(a, &[]), capture_facts(b, &[])],
            r,
        )
        .unwrap();
    o.install_version_set(
        &cut,
        &TrustedPreparedVersionSetFactsV1 {
            projection: &cut,
            members: &[current_member(a), current_member(b)],
        },
        r,
    )
    .unwrap()
}
#[test]
fn canonical_source_products_preserve_unicode_delimiters_and_ignore_excluded_class_facts() {
    let (mut o, mut r, a, b) = base();
    let d = o.read(&a.document, &mut r).unwrap().version;
    let text = "é\r\n\u{feff}x\r";
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: d,
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 5,
                    expected: "first".into(),
                    inserted: text.into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(edit, &mut r).unwrap();
    let acq = install_classes(&mut o, &mut r, &a, &b);
    let mut members = [current_member(&a), current_member(&b)];
    members[0].annotation_readable = Some(false);
    members[1].source_readable = Some(false);
    let f = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &members,
    };
    for (index, (body, delimiter)) in [
        ("é".as_bytes(), b"\r\n".as_slice()),
        ("\u{feff}x".as_bytes(), b"\r".as_slice()),
        (b"".as_slice(), b"".as_slice()),
    ]
    .into_iter()
    .enumerate()
    {
        let reply = o
            .query_version_set_source(
                &acq.participants[0].snapshot,
                SourceContextQueryV1::Line { index },
                &f,
                &mut r,
            )
            .unwrap();
        let SourceContextContentV1::Line(value) = reply.content else {
            panic!("wrong product")
        };
        assert_eq!(value.body(), body);
        assert_eq!(value.delimiter(), delimiter);
    }
    let reply = o
        .query_version_set_source(
            &acq.participants[0].snapshot,
            SourceContextQueryV1::Range {
                start: BytePosition(0),
                end: BytePosition(text.len()),
            },
            &f,
            &mut r,
        )
        .unwrap();
    let SourceContextContentV1::Range(value) = reply.content else {
        panic!("wrong product")
    };
    assert_eq!(value.bytes(), text.as_bytes());
    assert_eq!(o.version_set_context_usage().stopped, 0);
    assert_eq!(o.version_set_context_usage().issued_lifetime, 3);
}
#[test]
fn annotation_only_or_malformed_source_requests_cannot_poison_the_parent() {
    let (mut o, mut r, a, b) = base();
    let acq = install_classes(&mut o, &mut r, &a, &b);
    let mut members = [current_member(&a), current_member(&b)];
    members[1].authorized = Some(false);
    let f = TrustedVersionSetContextFactsV1 {
        context: &acq.context,
        members: &members,
    };
    assert!(
        o.query_version_set_source(
            &acq.participants[1].snapshot,
            SourceContextQueryV1::Line { index: 0 },
            &f,
            &mut r
        )
        .is_none()
    );
    for q in [
        SourceContextQueryV1::Line { index: 100 },
        SourceContextQueryV1::Range {
            start: BytePosition(4),
            end: BytePosition(1),
        },
        SourceContextQueryV1::Range {
            start: BytePosition(0),
            end: BytePosition(100),
        },
    ] {
        assert!(
            o.query_version_set_source(&acq.participants[0].snapshot, q, &f, &mut r)
                .is_none()
        );
    }
    assert_eq!(o.version_set_context_usage().stopped, 0);
    assert!(
        o.query_version_set_source(
            &acq.participants[0].snapshot,
            SourceContextQueryV1::Line { index: 0 },
            &TrustedVersionSetContextFactsV1 {
                context: &acq.context,
                members: &[current_member(&a), current_member(&b)]
            },
            &mut r
        )
        .is_some()
    );
}
#[test]
fn source_handoff_uses_fresh_complete_peer_facts_without_renewing_the_parent() {
    for index in 0..5 {
        for value in [None, Some(false)] {
            let (mut o, mut r, a, b, cut) = fixture(owner_limits());
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
            let mut members = [current_member(&a), current_member(&b)];
            let f = &mut members[1];
            match index {
                0 => f.authorized = value,
                1 => f.source_readable = value,
                2 => f.annotation_readable = value,
                3 => f.retained = value,
                _ => f.reply_ready = value,
            }
            let q = SourceContextQueryV1::Line { index: 0 };
            assert!(
                o.query_version_set_source(
                    &acq.participants[0].snapshot,
                    q,
                    &TrustedVersionSetContextFactsV1 {
                        context: &acq.context,
                        members: &members
                    },
                    &mut r
                )
                .is_none()
            );
            let lost = index < 4 && value == Some(false);
            assert_eq!(
                o.query_version_set_source(
                    &acq.participants[0].snapshot,
                    q,
                    &TrustedVersionSetContextFactsV1 {
                        context: &acq.context,
                        members: &[current_member(&a), current_member(&b)]
                    },
                    &mut r
                )
                .is_none(),
                lost
            );
            assert_eq!(o.version_set_context_usage().issued_lifetime, 3);
        }
    }
}
#[test]
fn source_query_instrumentation_is_optional_bounded_redacted_and_not_publication() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(64), Some(1)] {
        let (mut o, mut r, a, b) = base();
        let acq = install_classes(&mut o, &mut r, &a, &b);
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let generation = o.generation();
        let f = TrustedVersionSetContextFactsV1 {
            context: &acq.context,
            members: &[current_member(&a), current_member(&b)],
        };
        for _ in 0..2 {
            let reply = o
                .query_version_set_source(
                    &acq.participants[0].snapshot,
                    SourceContextQueryV1::Line { index: 0 },
                    &f,
                    &mut r,
                )
                .unwrap();
            assert_eq!(format!("{reply:?}"), "VersionSetSourceReplyV1(<redacted>)");
        }
        assert_eq!(o.generation(), generation);
        if capacity == Some(64) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.boundary == Boundary::VersionSetSourceQuery)
                    .count(),
                2
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
fn source_member_reads_use_the_original_cut_and_a_required_peer_can_fence_delivery() {
    let (mut o, mut r, a, b, cut) = fixture(owner_limits());
    let members = [current_member(&a), current_member(&b)];
    let acquired = o
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
    let edit = o
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
    o.commit(edit, &mut r).unwrap();
    assert_eq!(
        o.retire_version_set_preparation(&pool_control(), &mut r),
        Ok(true)
    );
    let id = &acquired.participants[0].snapshot;
    let q = SourceContextQueryV1::Range {
        start: BytePosition(0),
        end: BytePosition(5),
    };
    let f = TrustedVersionSetContextFactsV1 {
        context: &acquired.context,
        members: &members,
    };
    let reply = o.query_version_set_source(id, q, &f, &mut r).unwrap();
    let SourceContextContentV1::Range(value) = reply.content else {
        panic!("wrong product")
    };
    assert_eq!(value.bytes(), b"first");
    assert_eq!(reply.snapshot, *id);
    let mut loss = [current_member(&a), current_member(&b)];
    loss[1].annotation_readable = Some(false);
    assert!(
        o.query_version_set_source(
            id,
            q,
            &TrustedVersionSetContextFactsV1 {
                context: &acquired.context,
                members: &loss
            },
            &mut r
        )
        .is_none()
    );
    assert!(o.query_version_set_source(id, q, &f, &mut r).is_none());
    assert_eq!(o.version_set_context_usage().stopped, 1);
}
