use super::*;
#[test]
fn native_acquisition_facade_binds_the_requested_class_before_accepting_loss_or_consuming_the_cut()
{
    let (mut o, mut r, d, scope) = ready(b"old\r\nsource");
    let annotation = add(&mut o, &mut r, &d);
    let rows = grants(&annotation);
    o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), &mut r)
        .unwrap();
    o.configure_annotation_contexts(owner_limits(), &pool_control(), &mut r)
        .unwrap();
    let prepared = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &scope,
            &capture_facts(&scope, &rows),
            &mut r,
        )
        .unwrap();
    let limits = vellum_engine::capture::CaptureLimitsV1 {
        max_bytes: 8192,
        max_nodes: 256,
        max_depth: 16,
    };
    let before = o.annotation_preparation_usage();
    let reserved = o.retention_usage().reserved;
    let mut loss = current(&prepared, &scope);
    loss.authorized = Some(false);
    assert!(
        o.acquire_read_facade_v1(
            ReadAcquisitionFacadeRequestV1 {
                port: ReadAcquisitionFacadePortV1::AnnotationCurrent {
                    scope: &scope,
                    classes: AnnotationProjectionClassesV1::Annotations,
                    prepared: &prepared,
                    facts: &loss
                },
                actor: &scope.context,
                limits,
            },
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.annotation_preparation_usage(), before);
    assert!(!o.is_faulted());
    let Some(ReadAcquisitionFacadeReplyV1::Annotation(reply)) = o.acquire_read_facade_v1(
        ReadAcquisitionFacadeRequestV1 {
            port: ReadAcquisitionFacadePortV1::AnnotationCurrent {
                scope: &scope,
                classes: AnnotationProjectionClassesV1::Combined,
                prepared: &prepared,
                facts: &current(&prepared, &scope),
            },
            actor: &scope.context,
            limits,
        },
        &mut r,
    ) else {
        panic!("original prepared acquisition")
    };
    assert_eq!(reply.classes, AnnotationProjectionClassesV1::Combined);
    let result = o
        .query_annotation_context_source(
            &reply.context,
            &scope,
            SourceContextQueryV1::Line { index: 0 },
            &eligible(&reply.context, &scope),
            &mut r,
        )
        .unwrap();
    let SourceContextContentV1::Line(line) = result.content else {
        panic!("line")
    };
    assert_eq!(line.body(), b"old");
    assert_eq!(o.retention_usage().reserved, reserved);
    assert!(!o.read_facade_usage().busy);
    assert_eq!(o.read_facade_usage().workspace_bytes, 0);
}
#[path = "installed/queries.rs"]
mod queries;
#[path = "installed/retained.rs"]
mod retained;
#[path = "installed/search.rs"]
mod search;
fn owner_limits() -> AnnotationContextLimitsV1 {
    AnnotationContextLimitsV1 {
        max_selectors: 32,
        max_holds: 4,
        max_page_rows: 16,
        max_row_bytes: 4096,
        max_selector_bytes: 128,
        max_contexts: 4,
        max_issued: 8,
        max_rows: 8,
        max_capture_bytes: 8192,
        max_context_bytes: 2048,
        max_request_bytes: 8192,
        max_reply_bytes: 4096,
        max_label_bytes: 128,
        max_pool_bytes: 262144,
    }
}
fn eligible<'a>(
    id: &'a AnnotationReadContextId,
    s: &'a SourceReadScopeV1,
) -> TrustedAnnotationContextFactsV1<'a> {
    TrustedAnnotationContextFactsV1 {
        context: id,
        scope: s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        annotation_readable: Some(true),
        source_readable: Some(true),
        projection_current: Some(true),
        retained: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    }
}
#[test]
fn installation_consumes_original_cut_once_and_preparation_pool_can_retire_without_losing_owned_source()
 {
    let (mut o, mut r, d, s) = ready(b"old\r\nsource");
    let annotation = add(&mut o, &mut r, &d);
    let rows = grants(&annotation);
    o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), &mut r)
        .unwrap();
    let cut = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &capture_facts(&s, &rows),
            &mut r,
        )
        .unwrap();
    let prep_charge = o.annotation_preparation_usage().reserved_pool_bytes;
    let before = o.retention_usage().reserved;
    o.configure_annotation_contexts(owner_limits(), &pool_control(), &mut r)
        .unwrap();
    let generation = o.generation();
    let acquired = o
        .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
        .unwrap();
    assert_eq!(acquired.classes, AnnotationProjectionClassesV1::Combined);
    assert!(
        o.install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
            .is_none()
    );
    assert_eq!(o.annotation_preparation_usage().prepared, 0);
    assert_eq!(o.annotation_preparation_usage().issued_lifetime, 1);
    assert_eq!(o.annotation_context_usage().contexts, 1);
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.retire_annotation_preparation(&pool_control(), &mut r),
        Ok(true)
    );
    assert_eq!(
        o.retention_usage().reserved,
        before + o.annotation_context_usage().reserved_pool_bytes - prep_charge
    );
    let q = SourceContextQueryV1::Range {
        start: BytePosition(0),
        end: BytePosition(3),
    };
    let reply = o
        .query_annotation_context_source(
            &acquired.context,
            &s,
            q,
            &eligible(&acquired.context, &s),
            &mut r,
        )
        .unwrap();
    let SourceContextContentV1::Range(range) = reply.content else {
        panic!("wrong product")
    };
    assert_eq!(range.bytes(), b"old");
    assert_eq!(range.source, d);
}

#[test]
fn installed_capacity_refusal_does_not_consume_cut_but_bound_loss_precedes_full_occupancy() {
    let (mut o, mut r, d, s) = ready(b"source");
    let a = add(&mut o, &mut r, &d);
    let rows = grants(&a);
    o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), &mut r)
        .unwrap();
    let first = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &capture_facts(&s, &rows),
            &mut r,
        )
        .unwrap();
    let second = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &capture_facts(&s, &rows),
            &mut r,
        )
        .unwrap();
    let mut p = owner_limits();
    p.max_contexts = 1;
    p.max_issued = 1;
    o.configure_annotation_contexts(p, &pool_control(), &mut r)
        .unwrap();
    let acquired = o
        .install_annotation_context(&first, &s, &current(&first, &s), &mut r)
        .unwrap();
    assert!(
        o.install_annotation_context(&second, &s, &current(&second, &s), &mut r)
            .is_none()
    );
    assert_eq!(o.annotation_preparation_usage().prepared, 1);
    assert_eq!(o.annotation_preparation_usage().invalidated, 0);
    let mut lost = current(&second, &s);
    lost.projection_current = Some(false);
    lost.resources_ready = Some(false);
    assert!(
        o.install_annotation_context(&second, &s, &lost, &mut r)
            .is_none()
    );
    assert_eq!(o.annotation_preparation_usage().invalidated, 1);
    assert_eq!(o.annotation_context_usage().issued_lifetime, 1);
    assert!(o.stop_annotation_context(&acquired.context, &controls(&s), &mut r));
    assert!(o.retire_annotation_context(&acquired.context, &controls(&s), &mut r));
    assert_eq!(o.annotation_context_usage().issued_lifetime, 1);
}

#[test]
fn combined_source_queries_preserve_original_unicode_and_line_data_and_require_whole_projection_rights()
 {
    let (mut o, mut r, d, s) = ready("é\r\n\u{feff}x\r".as_bytes());
    let a = add(&mut o, &mut r, &d);
    let rows = grants(&a);
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
    let context = o
        .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
        .unwrap()
        .context;
    let change = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![vellum_engine::source::Replacement {
                    start: 0,
                    end: 2,
                    expected: "é".into(),
                    inserted: "NEW".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    o.commit(change, &mut r).unwrap();
    let q = SourceContextQueryV1::Line { index: 1 };
    let reply = o
        .query_annotation_context_source(&context, &s, q, &eligible(&context, &s), &mut r)
        .unwrap();
    let SourceContextContentV1::Line(line) = reply.content else {
        panic!("wrong product")
    };
    assert_eq!(line.body(), "\u{feff}x".as_bytes());
    assert_eq!(line.delimiter(), b"\r");
    assert_eq!(line.source, d);
    let mut lost = eligible(&context, &s);
    lost.annotation_readable = Some(false);
    lost.reply_ready = Some(false);
    assert!(
        o.query_annotation_context_source(
            &context,
            &s,
            SourceContextQueryV1::Range {
                start: BytePosition(1),
                end: BytePosition(2)
            },
            &lost,
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.annotation_context_usage().stopped, 0);
    assert!(
        o.query_annotation_context_source(&context, &s, q, &lost, &mut r)
            .is_none()
    );
    assert!(
        o.query_annotation_context_source(&context, &s, q, &eligible(&context, &s), &mut r)
            .is_none()
    );
    assert_eq!(o.annotation_context_usage().stopped, 1);
}

#[test]
fn annotation_only_context_never_gains_source_and_out_of_class_requests_do_not_apply_loss() {
    let (mut o, mut r, d, s) = ready(b"secret source");
    let a = add(&mut o, &mut r, &d);
    let rows = grants(&a);
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
    let acquired = o
        .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
        .unwrap();
    assert_eq!(acquired.classes, AnnotationProjectionClassesV1::Annotations);
    assert_eq!(o.annotation_context_usage().source_bytes, 0);
    let mut lost = eligible(&acquired.context, &s);
    lost.projection_current = Some(false);
    lost.source_readable = Some(false);
    assert!(
        o.query_annotation_context_source(
            &acquired.context,
            &s,
            SourceContextQueryV1::Line { index: 0 },
            &lost,
            &mut r
        )
        .is_none()
    );
    assert_eq!(o.annotation_context_usage().stopped, 0);
    assert_eq!(o.annotation_context_usage().body_bytes, 6);
}

#[test]
fn installed_source_facts_are_fresh_and_unknown_or_transient_refusal_does_not_establish_loss() {
    for index in 0..12 {
        for value in [None, Some(false)] {
            let (mut o, mut r, d, s) = ready(b"source");
            let a = add(&mut o, &mut r, &d);
            let rows = grants(&a);
            o.configure_annotation_preparation(
                "engine",
                "local",
                profile(),
                &pool_control(),
                &mut r,
            )
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
            let context = o
                .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
                .unwrap()
                .context;
            let mut f = eligible(&context, &s);
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
            let q = SourceContextQueryV1::Range {
                start: BytePosition(0),
                end: BytePosition(1),
            };
            assert!(
                o.query_annotation_context_source(&context, &s, q, &f, &mut r)
                    .is_none()
            );
            let loss = index < 9 && value == Some(false);
            assert_eq!(
                o.query_annotation_context_source(&context, &s, q, &eligible(&context, &s), &mut r)
                    .is_none(),
                loss
            );
            assert_eq!(o.annotation_context_usage().stopped, usize::from(loss));
        }
    }
}

#[test]
fn ownership_controls_and_observations_are_independent_from_content_and_never_publish_authored_changes()
 {
    use vellum_engine::instrumentation::Boundary;
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, mut r, d, s) = ready(b"source");
        let a = add(&mut o, &mut r, &d);
        let rows = grants(&a);
        o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), &mut r)
            .unwrap();
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
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
        let generation = o.generation();
        let context = o
            .install_annotation_context(&cut, &s, &current(&cut, &s), &mut r)
            .unwrap()
            .context;
        let q = SourceContextQueryV1::Line { index: 0 };
        let reply = o
            .query_annotation_context_source(&context, &s, q, &eligible(&context, &s), &mut r)
            .unwrap();
        assert_eq!(format!("{reply:?}"), "AnnotationSourceReplyV1(<redacted>)");
        assert!(!o.retire_annotation_context(&context, &controls(&s), &mut r));
        assert!(o.stop_annotation_context(&context, &controls(&s), &mut r));
        assert!(o.stop_annotation_context(&context, &controls(&s), &mut r));
        assert!(o.retire_annotation_context(&context, &controls(&s), &mut r));
        assert!(!o.retire_annotation_context(&context, &controls(&s), &mut r));
        assert_eq!(
            o.retire_annotation_contexts(&pool_control(), &mut r),
            Ok(true)
        );
        assert_eq!(
            o.configure_annotation_contexts(owner_limits(), &pool_control(), &mut r),
            Err(SourceReadContextErrorV1::Unavailable)
        );
        assert_eq!(o.annotation_context_usage().issued_lifetime, 1);
        assert_eq!(o.generation(), generation);
        if capacity == Some(128) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for b in [
                Boundary::AnnotationContextConfiguration,
                Boundary::AnnotationContextInstallation,
                Boundary::AnnotationContextSourceQuery,
                Boundary::AnnotationContextStop,
                Boundary::AnnotationContextRetirement,
                Boundary::AnnotationContextPoolRetirement,
            ] {
                assert!(events.iter().any(|e| e.boundary == b));
            }
            assert!(!events.iter().any(|e| e.boundary == Boundary::Publication));
            assert!(events.iter().all(|e| e.service_time.is_none()));
            assert!(!format!("{events:?}").contains("secret"));
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}
