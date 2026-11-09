use super::*;
pub(super) fn limits() -> AnnotationPreparationLimitsV1 {
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
pub(super) fn control() -> SourceReadPoolControlV1 {
    SourceReadPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(super) fn cleanup(s: &SourceReadScopeV1) -> SourceReadContextControlV1<'_> {
    SourceReadContextControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(super) fn facts<'a>(
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
pub(super) fn current<'a>(
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
pub(super) fn base() -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    SourceReadScopeV1,
    Vec<AnnotationFacetGrantV1>,
) {
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 8,
        max_participants: 8,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 4096,
            max_edits: 16,
            max_edit_payload_bytes: 4096,
        },
    });
    o.annotations.limits = Some(AnnotationLimits {
        max_issued: 8,
        max_records: 8,
        max_body_bytes: 128,
        max_retained_body_bytes: 4096,
        max_mapping_steps: 4096,
        max_live_snapshots: 8,
        max_snapshot_bytes: 8192,
    });
    let mut r = Recorder::disabled();
    let d = o.create(b"ab", &mut r).unwrap();
    let s = SourceReadScopeV1 {
        document: d.document.clone(),
        context: crate::coordinator::scoped_definition::tests::context(),
        policy: "p1".into(),
    };
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                target: SourceTarget {
                    source: d.clone(),
                    basis: TargetBasis::Before,
                    target: AttachmentTarget::Document,
                },
                body: "body".into(),
            }],
            &mut r,
        )
        .unwrap();
    let id = o.commit(p, &mut r).unwrap().annotation_changes[0]
        .annotation
        .clone();
    (
        o,
        r,
        d,
        s,
        vec![AnnotationFacetGrantV1 {
            annotation: id,
            visible: true,
            body: true,
            target: true,
        }],
    )
}
pub(super) fn ready(
    p: AnnotationPreparationLimitsV1,
) -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    SourceReadScopeV1,
    Vec<AnnotationFacetGrantV1>,
) {
    let (mut o, mut r, d, s, g) = base();
    o.configure_retention_budget(usize::MAX, &mut r).unwrap();
    o.configure_annotation_preparation("engine", "local", p, &control(), &mut r)
        .unwrap();
    (o, r, d, s, g)
}
#[test]
fn complete_native_root_scope_request_capture_selector_and_source_envelopes_are_exact() {
    let (probe, mut r, _, s, g) = base();
    let root = probe.semantic_root_usage(&mut r).unwrap().total_bytes;
    let context = scope_bytes(&s).unwrap();
    let request = context
        + std::mem::size_of::<AnnotationProjectionClassesV1>()
        + std::mem::size_of::<AnnotationFacetGrantV1>();
    let capture = std::mem::size_of::<Projection>() + std::mem::size_of::<Row>() + 4 + 2;
    let selector = std::mem::size_of::<PreparedAnnotationProjectionId>();
    let label = settings::basis::labels(&s.context)
        .into_iter()
        .map(str::len)
        .max()
        .unwrap();
    drop(g);
    for (coordinate, bound) in [root, context, request, capture, selector, 2, label]
        .into_iter()
        .enumerate()
    {
        for delta in [-1isize, 0, 1] {
            let mut p = limits();
            let n = bound.checked_add_signed(delta).unwrap();
            match coordinate {
                0 => p.max_root_bytes = n,
                1 => p.max_context_bytes = n,
                2 => p.max_request_bytes = n,
                3 => p.max_capture_bytes = n,
                4 => p.max_selector_bytes = n,
                5 => p.source.max_source_bytes = n,
                _ => p.max_label_bytes = n,
            }
            let (mut o, mut r, _, s, g) = ready(p);
            let charge = o.retention_usage().reserved;
            let generation = o.generation();
            assert_eq!(
                o.prepare_annotation_projection(
                    AnnotationProjectionClassesV1::Combined,
                    &s,
                    &facts(&s, &g),
                    &mut r
                )
                .is_some(),
                delta >= 0,
                "coordinate {coordinate} delta {delta}"
            );
            assert_eq!(
                o.annotation_preparation_usage().issued_lifetime,
                usize::from(delta >= 0)
            );
            assert_eq!(o.retention_usage().reserved, charge);
            assert_eq!(o.generation(), generation);
        }
    }
}
#[test]
fn pool_and_actual_parent_funding_are_independent_and_workspace_exists_at_zero_prepared_capacity() {
    for zero in [false, true] {
        for coordinate in 0..2 {
            for delta in [-1isize, 0, 1] {
                let mut p = limits();
                if zero {
                    p.max_prepared = 0;
                }
                let charge = p.required_pool_bytes().unwrap() + "engine".len() + "local".len();
                let (mut o, mut r, _, _, _) = base();
                let n = charge.checked_add_signed(delta).unwrap();
                if coordinate == 0 {
                    p.max_pool_bytes = n;
                }
                o.configure_retention_budget(if coordinate == 1 { n } else { usize::MAX }, &mut r)
                    .unwrap();
                assert_eq!(
                    o.configure_annotation_preparation("engine", "local", p, &control(), &mut r),
                    if delta >= 0 {
                        Ok(true)
                    } else {
                        Err(Error::Limit)
                    }
                );
                assert_eq!(
                    o.retention_usage().reserved,
                    if delta >= 0 { charge } else { 0 }
                );
                if delta >= 0 {
                    assert!(charge > 0);
                    assert_eq!(
                        o.configure_annotation_preparation(
                            "engine",
                            "local",
                            p,
                            &control(),
                            &mut r
                        ),
                        Ok(false)
                    );
                }
            }
        }
    }
}
#[test]
fn cuts_copy_only_admitted_facets_and_original_coherent_values_without_refreshing_or_holding_roots()
{
    let (mut o, mut r, d, s, mut g) = ready(limits());
    let annotation = g[0].annotation.clone();
    let combined = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Combined,
            &s,
            &facts(&s, &g),
            &mut r,
        )
        .unwrap();
    g[0].target = false;
    let body_only = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &facts(&s, &g),
            &mut r,
        )
        .unwrap();
    g[0].body = false;
    let identity_only = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &facts(&s, &g),
            &mut r,
        )
        .unwrap();
    g[0].visible = false;
    let hidden = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &facts(&s, &g),
            &mut r,
        )
        .unwrap();
    let p = o
        .prepare_compound(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "a".into(),
                    inserted: "Z".into(),
                }],
            }],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation,
                expected_content: 1,
                body: "NEW".into(),
            }],
            &mut r,
        )
        .unwrap();
    o.commit(p, &mut r).unwrap();
    for id in [&combined, &body_only, &identity_only, &hidden] {
        assert!(o.validate_prepared_annotation_projection(id, &s, &current(id, &s), &mut r));
    }
    let cut = o.prepared_annotation_cut(&combined, &s).unwrap();
    assert_eq!(
        cut.value.rows[0].body.as_ref().unwrap(),
        &("body".into(), 1)
    );
    let (source, version, _) = cut.value.source.as_ref().unwrap();
    assert_eq!(source.text(), "ab");
    assert_eq!(*version, d);
    let row = &o.prepared_annotation_cut(&body_only, &s).unwrap().value;
    assert!(row.source.is_none());
    assert!(row.rows[0].location.is_none());
    let row = &o.prepared_annotation_cut(&identity_only, &s).unwrap().value;
    assert!(row.source.is_none());
    assert!(row.rows[0].body.is_none());
    assert!(row.rows[0].location.is_none());
    let row = &o.prepared_annotation_cut(&hidden, &s).unwrap().value;
    assert!(row.rows.is_empty());
    assert!(row.source.is_none());
    assert_eq!(o.source_read_context_usage().holds, 0);
    assert_eq!(o.annotation_usage().retained_snapshots, 0);
}
#[test]
fn foreign_namespace_and_actual_disposal_or_fault_cannot_be_masked_by_stale_positive_handoff_facts()
{
    for fault in [false, true] {
        let (mut o, mut r, d, s, g) = ready(limits());
        let id = o
            .prepare_annotation_projection(
                AnnotationProjectionClassesV1::Combined,
                &s,
                &facts(&s, &g),
                &mut r,
            )
            .unwrap();
        let forged = PreparedAnnotationProjectionId {
            owner: Arc::new(()),
            slot: id.slot,
        };
        assert_ne!(id, forged);
        let mut loss = current(&forged, &s);
        loss.authorized = Some(false);
        assert!(!o.validate_prepared_annotation_projection(&forged, &s, &loss, &mut r));
        assert_eq!(o.annotation_preparation_usage().invalidated, 0);
        if fault {
            o.faulted = true;
        } else {
            let set = o.document(&d.document).unwrap().annotation_generation;
            let plan = o.prepare_retained_disposal(&d, set, &mut r).unwrap();
            o.commit_retained_disposal(plan, &mut r).unwrap();
        }
        assert!(!o.validate_prepared_annotation_projection(&id, &s, &current(&id, &s), &mut r));
        assert_eq!(o.annotation_preparation_usage().invalidated, 1);
        assert!(o.discard_prepared_annotation_projection(&id, &cleanup(&s), &mut r));
        assert_eq!(
            o.retire_annotation_preparation(&control(), &mut r),
            Ok(true)
        );
        assert_eq!(o.is_faulted(), fault);
    }
}

#[test]
fn malformed_owned_location_is_faulted_not_reported_as_ordinary_missing_content() {
    let (mut o, mut r, _, s, g) = ready(limits());
    let reserved = o.retention_usage().reserved;
    o.annotations
        .records
        .iter_mut()
        .flatten()
        .next()
        .unwrap()
        .value
        .target
        .revision = None;
    assert!(
        o.prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &facts(&s, &g),
            &mut r
        )
        .is_none()
    );
    assert!(o.is_faulted());
    assert_eq!(o.annotation_preparation_usage().issued_lifetime, 0);
    assert_eq!(o.retention_usage().reserved, reserved);
    assert_eq!(
        o.retire_annotation_preparation(&control(), &mut r),
        Ok(true)
    );
    assert!(o.is_faulted());
}
