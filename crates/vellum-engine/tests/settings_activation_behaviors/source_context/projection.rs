use super::*;
#[path = "projection/installed.rs"]
mod installed;
#[path = "projection/version_set.rs"]
mod version_set;
fn profile() -> AnnotationPreparationLimitsV1 {
    AnnotationPreparationLimitsV1 {
        max_prepared: 4,
        max_issued: 8,
        max_rows: 8,
        max_root_bytes: 1048576,
        max_capture_bytes: 8192,
        max_context_bytes: 2048,
        max_request_bytes: 8192,
        max_selector_bytes: 128,
        max_label_bytes: 128,
        max_pool_bytes: 262144,
        source: SourceProjectionLimitsV1 {
            max_source_bytes: 4096,
            max_request_bytes: 4096,
            max_reply_bytes: 4096,
            max_scan_bytes: 4096,
        },
    }
}
fn grants(id: &AnnotationId) -> Vec<AnnotationFacetGrantV1> {
    vec![AnnotationFacetGrantV1 {
        annotation: id.clone(),
        visible: true,
        body: true,
        target: false,
    }]
}
fn capture_facts<'a>(
    s: &'a SourceReadScopeV1,
    grants: &'a [AnnotationFacetGrantV1],
) -> TrustedAnnotationCaptureFactsV1<'a> {
    TrustedAnnotationCaptureFactsV1 {
        scope: s,
        grants,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        annotation_readable: Some(true),
        source_readable: Some(true),
        visibility_complete: Some(true),
        projection_order_attested: Some(true),
        resources_ready: Some(true),
    }
}
fn current<'a>(
    id: &'a PreparedAnnotationProjectionId,
    s: &'a SourceReadScopeV1,
) -> TrustedPreparedAnnotationFactsV1<'a> {
    TrustedPreparedAnnotationFactsV1 {
        projection: id,
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
fn add(o: &mut SourceCoordinator, r: &mut Recorder, d: &DocumentVersion) -> AnnotationId {
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                target: SourceTarget {
                    source: d.clone(),
                    basis: TargetBasis::Before,
                    target: vellum_engine::attachment::AttachmentTarget::Document,
                },
                body: "secret".into(),
            }],
            r,
        )
        .unwrap();
    o.commit(p, r).unwrap().annotation_changes[0]
        .annotation
        .clone()
}
#[test]
fn annotation_preparation_is_funded_and_does_not_require_or_retain_unrequested_source() {
    let (mut o, mut r, d, s) = ready(b"source must not be captured");
    let id = add(&mut o, &mut r, &d);
    let grants = grants(&id);
    let mut p = profile();
    p.source.max_source_bytes = 0;
    let before = o.retention_usage().reserved;
    assert_eq!(
        o.configure_annotation_preparation("engine", "local", p, &pool_control(), &mut r),
        Ok(true)
    );
    let mut f = capture_facts(&s, &grants);
    f.source_readable = Some(false);
    let generation = o.generation();
    let cut = o
        .prepare_annotation_projection(AnnotationProjectionClassesV1::Annotations, &s, &f, &mut r)
        .unwrap();
    assert_eq!(o.generation(), generation);
    let usage = o.annotation_preparation_usage();
    assert_eq!(usage.prepared, 1);
    assert_eq!(usage.source_bytes, 0);
    assert_eq!(usage.body_bytes, 6);
    assert!(o.retention_usage().reserved > before);
    let mut f = current(&cut, &s);
    f.source_readable = Some(false);
    assert!(o.validate_prepared_annotation_projection(&cut, &s, &f, &mut r));
    assert!(
        o.prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &capture_facts(&s, &grants),
            &mut r
        )
        .is_none()
    );
    assert_eq!(
        format!("{cut:?}"),
        "PreparedAnnotationProjectionId(<opaque>)"
    );
}

#[test]
fn preparation_requires_complete_unique_actual_visibility_rows_and_never_accepts_host_supplied_content()
 {
    let (mut o, mut r, d, s) = ready(b"source");
    let a = add(&mut o, &mut r, &d);
    let b = add(&mut o, &mut r, &d);
    o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), &mut r)
        .unwrap();
    let one = grants(&a)[0].clone();
    let two = grants(&b)[0].clone();
    for rows in [
        vec![],
        vec![one.clone()],
        vec![one.clone(), one.clone()],
        vec![
            AnnotationFacetGrantV1 {
                visible: false,
                body: true,
                ..one.clone()
            },
            two.clone(),
        ],
    ] {
        assert!(
            o.prepare_annotation_projection(
                AnnotationProjectionClassesV1::Annotations,
                &s,
                &capture_facts(&s, &rows),
                &mut r
            )
            .is_none()
        );
    }
    assert_eq!(o.annotation_preparation_usage().issued_lifetime, 0);
    let rows = [
        AnnotationFacetGrantV1 {
            visible: false,
            body: false,
            target: false,
            ..one
        },
        two,
    ];
    let cut = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &capture_facts(&s, &rows),
            &mut r,
        )
        .unwrap();
    assert_eq!(o.annotation_preparation_usage().projected_rows, 1);
    assert_eq!(o.annotation_preparation_usage().body_bytes, 6);
    assert!(o.validate_prepared_annotation_projection(&cut, &s, &current(&cut, &s), &mut r));
}

#[test]
fn unknown_or_false_preparation_facts_never_issue_and_explicit_retry_can_prepare() {
    for classes in [
        AnnotationProjectionClassesV1::Annotations,
        AnnotationProjectionClassesV1::Combined,
    ] {
        for index in 0..10 {
            for value in [None, Some(false)] {
                let (mut o, mut r, d, s) = ready(b"source");
                let id = add(&mut o, &mut r, &d);
                let rows = grants(&id);
                o.configure_annotation_preparation(
                    "engine",
                    "local",
                    profile(),
                    &pool_control(),
                    &mut r,
                )
                .unwrap();
                let mut f = capture_facts(&s, &rows);
                match index {
                    0 => f.authorized = value,
                    1 => f.scope_allowed = value,
                    2 => f.continuous = value,
                    3 => f.live = value,
                    4 => f.not_stopped = value,
                    5 => f.annotation_readable = value,
                    6 => f.source_readable = value,
                    7 => f.visibility_complete = value,
                    8 => f.projection_order_attested = value,
                    _ => f.resources_ready = value,
                }
                let irrelevant =
                    index == 6 && classes == AnnotationProjectionClassesV1::Annotations;
                assert_eq!(
                    o.prepare_annotation_projection(classes, &s, &f, &mut r)
                        .is_some(),
                    irrelevant
                );
                assert_eq!(
                    o.annotation_preparation_usage().issued_lifetime,
                    usize::from(irrelevant)
                );
                assert!(
                    o.prepare_annotation_projection(classes, &s, &capture_facts(&s, &rows), &mut r)
                        .is_some()
                );
            }
        }
    }
}

#[test]
fn handoff_checks_original_identity_and_scope_before_loss_and_distinguishes_loss_from_pressure() {
    for classes in [
        AnnotationProjectionClassesV1::Annotations,
        AnnotationProjectionClassesV1::Combined,
    ] {
        for index in 0..12 {
            for value in [None, Some(false)] {
                let (mut o, mut r, d, s) = ready(b"source");
                let id = add(&mut o, &mut r, &d);
                let rows = grants(&id);
                let mut p = profile();
                p.max_prepared = 1;
                p.max_issued = 1;
                o.configure_annotation_preparation("engine", "local", p, &pool_control(), &mut r)
                    .unwrap();
                let cut = o
                    .prepare_annotation_projection(classes, &s, &capture_facts(&s, &rows), &mut r)
                    .unwrap();
                let mut f = current(&cut, &s);
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
                let irrelevant =
                    index == 6 && classes == AnnotationProjectionClassesV1::Annotations;
                assert_eq!(
                    o.validate_prepared_annotation_projection(&cut, &s, &f, &mut r),
                    irrelevant
                );
                let lost = value == Some(false) && index < 9 && !irrelevant;
                assert_eq!(
                    o.validate_prepared_annotation_projection(&cut, &s, &current(&cut, &s), &mut r),
                    !lost
                );
                assert_eq!(
                    o.annotation_preparation_usage().invalidated,
                    usize::from(lost)
                );
                assert_eq!(o.annotation_preparation_usage().issued_lifetime, 1);
            }
        }
    }
    let (mut o, mut r, d, s) = ready(b"source");
    let id = add(&mut o, &mut r, &d);
    let rows = grants(&id);
    o.configure_annotation_preparation("engine", "local", profile(), &pool_control(), &mut r)
        .unwrap();
    let a = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &capture_facts(&s, &rows),
            &mut r,
        )
        .unwrap();
    let b = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &capture_facts(&s, &rows),
            &mut r,
        )
        .unwrap();
    let mut bad = current(&b, &s);
    bad.authorized = Some(false);
    assert!(!o.validate_prepared_annotation_projection(&a, &s, &bad, &mut r));
    let mut wrong = s.clone();
    wrong.policy.push('x');
    let mut bad = current(&a, &wrong);
    bad.authorized = Some(false);
    assert!(!o.validate_prepared_annotation_projection(&a, &wrong, &bad, &mut r));
    assert!(o.validate_prepared_annotation_projection(&a, &s, &current(&a, &s), &mut r));
    assert_eq!(o.annotation_preparation_usage().invalidated, 0);
}

#[test]
fn cut_discard_does_not_refill_issuance_or_refund_pool_and_empty_pool_retires_once() {
    let (mut o, mut r, d, s) = ready(b"source");
    let id = add(&mut o, &mut r, &d);
    let rows = grants(&id);
    let before = o.retention_usage().reserved;
    let mut p = profile();
    p.max_issued = 2;
    p.max_prepared = 1;
    o.configure_annotation_preparation("engine", "local", p, &pool_control(), &mut r)
        .unwrap();
    let funded = o.retention_usage().reserved;
    for issued in 1..=2 {
        let cut = o
            .prepare_annotation_projection(
                AnnotationProjectionClassesV1::Combined,
                &s,
                &capture_facts(&s, &rows),
                &mut r,
            )
            .unwrap();
        assert_eq!(
            o.retire_annotation_preparation(&pool_control(), &mut r),
            Err(SourceReadContextErrorV1::Unavailable)
        );
        let mut denied = controls(&s);
        denied.delivery_drained = None;
        assert!(!o.discard_prepared_annotation_projection(&cut, &denied, &mut r));
        assert!(o.discard_prepared_annotation_projection(&cut, &controls(&s), &mut r));
        assert!(!o.discard_prepared_annotation_projection(&cut, &controls(&s), &mut r));
        assert_eq!(o.annotation_preparation_usage().issued_lifetime, issued);
        assert_eq!(o.retention_usage().reserved, funded);
    }
    assert!(
        o.prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &capture_facts(&s, &rows),
            &mut r
        )
        .is_none()
    );
    assert_eq!(
        o.retire_annotation_preparation(&pool_control(), &mut r),
        Ok(true)
    );
    assert_eq!(o.retention_usage().reserved, before);
    assert_eq!(
        o.retire_annotation_preparation(&pool_control(), &mut r),
        Ok(false)
    );
    assert_eq!(
        o.configure_annotation_preparation("engine", "local", p, &pool_control(), &mut r),
        Err(SourceReadContextErrorV1::Unavailable)
    );
    assert_eq!(o.annotation_preparation_usage().issued_lifetime, 2);
}

#[test]
fn projection_instrumentation_is_optional_bounded_redacted_and_not_authored_publication() {
    use vellum_engine::instrumentation::{Boundary, Disposition};
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, mut r, d, s) = ready(b"source");
        let id = add(&mut o, &mut r, &d);
        let rows = grants(&id);
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        let generation = o.generation();
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
        assert!(o.validate_prepared_annotation_projection(&cut, &s, &current(&cut, &s), &mut r));
        let mut lost = current(&cut, &s);
        lost.projection_current = Some(false);
        lost.resources_ready = Some(false);
        assert!(!o.validate_prepared_annotation_projection(&cut, &s, &lost, &mut r));
        assert!(o.discard_prepared_annotation_projection(&cut, &controls(&s), &mut r));
        assert_eq!(
            o.retire_annotation_preparation(&pool_control(), &mut r),
            Ok(true)
        );
        assert_eq!(o.generation(), generation);
        if capacity == Some(128) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::AnnotationProjectionConfiguration,
                Boundary::AnnotationProjectionPreparation,
                Boundary::AnnotationProjectionValidation,
                Boundary::AnnotationProjectionInvalidation,
                Boundary::AnnotationProjectionDiscard,
                Boundary::AnnotationPreparationRetirement,
            ] {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == boundary && e.disposition == Disposition::Completed)
                );
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
