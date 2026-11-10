use super::*;

#[test]
fn retained_capture_instrumentation_is_bounded_optional_and_never_authored_publication() {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(64), Some(1)] {
        let (mut o, _, _, s, c) = fixture(owner_limits());
        let mut r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let generation = o.generation();
        let reserved = o.retention_usage().reserved;
        let copy = o
            .capture_retained_annotation_context(
                &c,
                AnnotationProjectionClassesV1::Combined,
                &s,
                &eligible(&c, &s),
                Some(true),
                &mut r,
            )
            .unwrap()
            .context;
        assert!(
            o.query_annotation_context_source(
                &copy,
                &s,
                SourceContextQueryV1::Line { index: 0 },
                &eligible(&copy, &s),
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
                    .filter(|e| e.boundary == Boundary::AnnotationContextRetainedCapture)
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
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}

fn fixture(
    p: AnnotationContextLimitsV1,
) -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    SourceReadScopeV1,
    AnnotationReadContextId,
) {
    let (mut o, mut r, d, s) = ready(b"old");
    let a = add(&mut o, &mut r, &d);
    let rows = grants(&a);
    o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), &mut r)
        .unwrap();
    o.configure_annotation_contexts(p, &pool_control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &capture_facts(&s, &rows),
            &mut r,
        )
        .unwrap();
    let original = o
        .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
        .unwrap()
        .context;
    (o, r, d, s, original)
}

#[test]
fn original_whole_projection_loss_precedes_exhausted_new_admission_but_unknown_and_pressure_do_not_stop_it()
 {
    for index in 0..12 {
        for value in [None, Some(false)] {
            let mut p = owner_limits();
            p.max_contexts = 1;
            p.max_issued = 1;
            let (mut o, mut r, _, s, c) = fixture(p);
            let mut f = eligible(&c, &s);
            match index {
                0 => f.authorized = value,
                1 => f.scope_allowed = value,
                2 => f.continuous = value,
                3 => f.live = value,
                4 => f.not_stopped = value,
                5 => f.annotation_readable = value,
                6 => f.source_readable = value,
                7 => f.projection_current = value,
                8 => f.retained = value,
                9 => f.resources_ready = value,
                10 => f.reply_ready = value,
                _ => f.transfer_ready = value,
            }
            assert!(
                o.capture_retained_annotation_context(
                    &c,
                    AnnotationProjectionClassesV1::Source,
                    &s,
                    &f,
                    Some(false),
                    &mut r
                )
                .is_none()
            );
            assert_eq!(
                o.annotation_context_usage().stopped,
                usize::from(index < 9 && value == Some(false)),
                "gate {index} value {value:?}"
            );
            assert_eq!(o.annotation_context_usage().issued_lifetime, 1);
            assert_eq!(o.annotation_context_usage().holds, 0);
        }
    }
}

#[test]
fn narrowing_never_widens_and_wrong_scope_or_original_cannot_poison_a_copy() {
    let (mut o, mut r, _, s, c) = fixture(owner_limits());
    let source = o
        .capture_retained_annotation_context(
            &c,
            AnnotationProjectionClassesV1::Source,
            &s,
            &eligible(&c, &s),
            Some(true),
            &mut r,
        )
        .unwrap()
        .context;
    let mut loss = eligible(&source, &s);
    loss.authorized = Some(false);
    assert!(
        o.capture_retained_annotation_context(
            &source,
            AnnotationProjectionClassesV1::Combined,
            &s,
            &loss,
            Some(true),
            &mut r
        )
        .is_none()
    );
    assert!(
        o.capture_retained_annotation_context(
            &source,
            AnnotationProjectionClassesV1::Source,
            &s,
            &eligible(&c, &s),
            Some(true),
            &mut r
        )
        .is_none()
    );
    let mut other = s.clone();
    other.policy.push('x');
    assert!(
        o.capture_retained_annotation_context(
            &source,
            AnnotationProjectionClassesV1::Source,
            &other,
            &loss,
            Some(true),
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.annotation_context_usage().stopped, 0);
    for gate in [None, Some(false)] {
        assert!(
            o.capture_retained_annotation_context(
                &source,
                AnnotationProjectionClassesV1::Source,
                &s,
                &eligible(&source, &s),
                gate,
                &mut r
            )
            .is_none()
        );
    }
    let mut f = eligible(&source, &s);
    f.annotation_readable = Some(false);
    let copy = o
        .capture_retained_annotation_context(
            &source,
            AnnotationProjectionClassesV1::Source,
            &s,
            &f,
            Some(true),
            &mut r,
        )
        .unwrap()
        .context;
    assert!(o.stop_annotation_context(&source, &controls(&s), &mut r));
    assert!(o.retire_annotation_context(&source, &controls(&s), &mut r));
    assert!(
        o.query_annotation_context_source(
            &copy,
            &s,
            SourceContextQueryV1::Line { index: 0 },
            &eligible(&copy, &s),
            &mut r
        )
        .is_some()
    );
    assert_eq!(o.annotation_context_usage().issued_lifetime, 3);
}

#[test]
fn retained_annotation_selectors_are_fresh_and_annotation_only_copies_do_not_need_source_rights() {
    let (mut o, mut r, _, s, c) = fixture(owner_limits());
    let q = AnnotationQueryV1::Enumerate {
        limit: 1,
        continuation: None,
    };
    let reply = o
        .query_annotation_context(&c, &s, &q, &eligible(&c, &s), &mut r)
        .unwrap();
    let AnnotationQueryReplyV1::Page { records, .. } = reply else {
        panic!("wrong product")
    };
    let copy = o
        .capture_retained_annotation_context(
            &c,
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &eligible(&c, &s),
            Some(true),
            &mut r,
        )
        .unwrap()
        .context;
    let mut f = eligible(&copy, &s);
    f.source_readable = Some(false);
    assert!(
        o.query_annotation_context(
            &copy,
            &s,
            &AnnotationQueryV1::Inspect {
                annotation: records[0].reference.clone()
            },
            &f,
            &mut r
        )
        .is_none()
    );
    assert!(
        o.capture_retained_annotation_context(
            &copy,
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &f,
            Some(true),
            &mut r
        )
        .is_some()
    );
    assert_eq!(o.annotation_context_usage().stopped, 0);
    assert!(
        o.query_annotation_context(&copy, &s, &q, &f, &mut r)
            .is_some()
    );
}

#[test]
fn retained_source_cut_drives_shared_search_but_cannot_authorize_a_stale_replacement() {
    use super::super::super::search::{qfacts, request, search_profile};
    let (mut o, mut r, d, s, c) = fixture(owner_limits());
    let copy = o
        .capture_retained_annotation_context(
            &c,
            AnnotationProjectionClassesV1::Source,
            &s,
            &eligible(&c, &s),
            Some(true),
            &mut r,
        )
        .unwrap()
        .context;
    let edit = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
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
    o.commit(edit, &mut r).unwrap();
    assert!(o.stop_annotation_context(&c, &controls(&s), &mut r));
    assert!(o.retire_annotation_context(&c, &controls(&s), &mut r));
    o.configure_source_search(search_profile(), &pool_control(), &mut r)
        .unwrap();
    let q = request("old");
    let mut f = eligible(&copy, &s);
    f.annotation_readable = Some(false);
    let result = o
        .query_managed_search(
            SearchReadContextV1::Combined(&copy),
            &s,
            &q,
            SearchReadFactsV1::Combined(&f),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    let reply = o
        .release_managed_search(
            &result,
            &s,
            SearchReadFactsV1::Combined(&f),
            &qfacts(&q),
            &mut r,
        )
        .unwrap();
    assert_eq!(reply.version, d);
    assert_eq!(reply.matches, vec![ByteRange { start: 0, end: 3 }]);
    let command = SourceSearchConsumerV1::ReplaceAll {
        result,
        replacement: "Z".into(),
    };
    let cf = TrustedSourceSearchConsumerFactsV1 {
        scope: &s,
        command: &command,
        mutation_allowed: Some(true),
        resources_ready: Some(true),
    };
    let id = o.issue_invocation(&mut r).unwrap();
    assert_eq!(
        o.submit_managed_search_consumer(
            &id,
            &command,
            &s,
            SearchReadFactsV1::Combined(&f),
            &qfacts(&q),
            &cf,
            &mut r
        ),
        Ok(InvocationState::Pending)
    );
    assert_eq!(
        o.run_managed_search_consumer(
            &id,
            &s,
            SearchReadFactsV1::Combined(&f),
            &qfacts(&q),
            &cf,
            &mut r
        ),
        Ok(InvocationState::Finished)
    );
    assert!(matches!(
        o.invocation_outcome(&id, &mut r).unwrap(),
        InvocationOutcome::Rejected(CoordinatorError::Stale)
    ));
    assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "new");
}

#[test]
fn retained_combined_cut_can_narrow_without_new_root_content_or_an_original_hold() {
    let (mut o, mut r, d, s) = ready(b"old");
    let annotation = add(&mut o, &mut r, &d);
    let rows = grants(&annotation);
    o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), &mut r)
        .unwrap();
    o.configure_annotation_contexts(owner_limits(), &pool_control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &capture_facts(&s, &rows),
            &mut r,
        )
        .unwrap();
    let original = o
        .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
        .unwrap()
        .context;
    let change = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
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
    o.commit(change, &mut r).unwrap();
    let source = o
        .capture_retained_annotation_context(
            &original,
            AnnotationProjectionClassesV1::Source,
            &s,
            &eligible(&original, &s),
            Some(true),
            &mut r,
        )
        .unwrap();
    let annotations = o
        .capture_retained_annotation_context(
            &original,
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &eligible(&original, &s),
            Some(true),
            &mut r,
        )
        .unwrap();
    assert_eq!(source.classes, AnnotationProjectionClassesV1::Source);
    assert_eq!(o.annotation_context_usage().holds, 0);
    assert!(o.stop_annotation_context(&original, &controls(&s), &mut r));
    assert!(o.retire_annotation_context(&original, &controls(&s), &mut r));
    let mut f = eligible(&source.context, &s);
    f.annotation_readable = Some(false);
    assert!(
        o.query_annotation_context(
            &source.context,
            &s,
            &AnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: None
            },
            &f,
            &mut r
        )
        .is_none()
    );
    let reply = o
        .query_annotation_context_source(
            &source.context,
            &s,
            SourceContextQueryV1::Range {
                start: BytePosition(0),
                end: BytePosition(3),
            },
            &f,
            &mut r,
        )
        .unwrap();
    let SourceContextContentV1::Range(value) = reply.content else {
        panic!("wrong product")
    };
    assert_eq!(value.bytes(), b"old");
    assert_eq!(value.source, d);
    let mut f = eligible(&annotations.context, &s);
    f.source_readable = Some(false);
    assert!(
        o.query_annotation_context_source(
            &annotations.context,
            &s,
            SourceContextQueryV1::Line { index: 0 },
            &f,
            &mut r
        )
        .is_none()
    );
    let reply = o
        .query_annotation_context(
            &annotations.context,
            &s,
            &AnnotationQueryV1::Enumerate {
                limit: 1,
                continuation: None,
            },
            &f,
            &mut r,
        )
        .unwrap();
    let AnnotationQueryReplyV1::Page { records, .. } = reply else {
        panic!("wrong product")
    };
    assert_eq!(records[0].content.as_ref().unwrap().body, "secret");
    assert_eq!(o.annotation_context_usage().source_bytes, 3);
    assert_eq!(o.annotation_context_usage().projected_rows, 1);
}
