use super::super::tests::{cleanup, facts, limits, ready};
use super::*;
#[test]
fn publication_admission_fence_collection_visibility_refuses_new_capture_but_preserves_original_packet_service()
 {
    let (mut o, mut r, _, s, g) = ready(limits());
    let ceiling = CollectionFacetCeilingV1 {
        body: true,
        target: true,
    };
    let p = o
        .prepare_collection_visibility_v1(&s, ceiling, &facts(&s, &g), &mut r)
        .unwrap();
    crate::coordinator::shutdown::tests::install(&mut o, &mut r);
    let before = (
        o.annotation_preparation_usage(),
        o.retention_usage(),
        o.generation(),
    );
    assert_eq!(
        o.prepare_collection_visibility_v1(&s, ceiling, &facts(&s, &g), &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(
        (
            o.annotation_preparation_usage(),
            o.retention_usage(),
            o.generation()
        ),
        before
    );
    let n = crate::coordinator::publication_stream::tests::native(&s);
    assert!(
        o.validate_collection_visibility_v1(&p, &s, ceiling, &current(&n), &mut r)
            .is_ok()
    );
    let capture = o
        .consume_collection_visibility_v1(&p, &s, ceiling, &current(&n), &mut r)
        .unwrap();
    assert_eq!(capture.rows().len(), 1);
    assert_eq!(o.annotation_preparation_usage().prepared, 0);
    assert_eq!(o.retention_usage(), before.1);
}
fn current<'a>(n: &'a TrustedSourceReadFactsV1<'a>) -> TrustedCollectionVisibilityFactsV1<'a> {
    TrustedCollectionVisibilityFactsV1 {
        current: n,
        annotation_readable: Some(true),
        projection_current: Some(true),
    }
}
#[test]
fn collection_visibility_packets_are_exact_root_bound_single_use_and_role_distinct() {
    let (mut o, mut r, _, s, g) = ready(limits());
    let ceiling = CollectionFacetCeilingV1 {
        body: true,
        target: false,
    };
    let packet = o
        .prepare_collection_visibility_v1(&s, ceiling, &facts(&s, &g), &mut r)
        .unwrap();
    let n = crate::coordinator::publication_stream::tests::native(&s);
    let f = current(&n);
    assert!(o.prepared_annotation_cut(&packet.inner, &s).is_err());
    assert!(
        o.validate_collection_visibility_v1(&packet, &s, ceiling, &f, &mut r)
            .is_ok()
    );
    let capture = o
        .consume_collection_visibility_v1(&packet, &s, ceiling, &f, &mut r)
        .unwrap();
    assert_eq!(capture.rows().len(), 1);
    let (_, body, target) = capture.rows().next().unwrap();
    assert_eq!(body, Some(("body", 1)));
    assert!(target.is_none());
    assert!(
        o.consume_collection_visibility_v1(&packet.clone(), &s, ceiling, &f, &mut r)
            .is_err()
    );
    assert_eq!(o.annotation_preparation_usage().prepared, 0);
}

#[test]
fn collection_visibility_stale_root_wrong_scope_and_ceiling_refuse_without_consumption() {
    let (mut o, mut r, _, s, g) = ready(limits());
    let ceiling = CollectionFacetCeilingV1 {
        body: true,
        target: true,
    };
    let packet = o
        .prepare_collection_visibility_v1(&s, ceiling, &facts(&s, &g), &mut r)
        .unwrap();
    let n = crate::coordinator::publication_stream::tests::native(&s);
    let f = current(&n);
    let mut wrong = s.clone();
    wrong.context.destination = "wrong".into();
    assert!(
        o.consume_collection_visibility_v1(&packet, &wrong, ceiling, &f, &mut r)
            .is_err()
    );
    assert!(
        o.consume_collection_visibility_v1(
            &packet,
            &s,
            CollectionFacetCeilingV1 {
                body: false,
                target: true
            },
            &f,
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.annotation_preparation_usage().prepared, 1);
    // Even a hidden peer publication makes the private preparation stale; it
    // does not allocate an observer reference or expose a recipient generation.
    o.create(b"hidden peer", &mut r).unwrap();
    assert!(
        o.consume_collection_visibility_v1(&packet, &s, ceiling, &f, &mut r)
            .is_err()
    );
    assert_eq!(o.annotation_preparation_usage().prepared, 1);
    o.faulted = true;
    let debt = o.retention_usage();
    o.retire_collection_visibility_v1(&packet, &cleanup(&s), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage(), debt);
    assert!(o.is_faulted());
    assert_eq!(o.annotation_preparation_usage().prepared, 0);
}

#[test]
fn collection_visibility_requires_bijection_not_empty_missing_duplicate_or_foreign_facts() {
    let (mut o, mut r, _, s, g) = ready(limits());
    let (_, _, _, _, foreign) = ready(limits());
    let ceiling = CollectionFacetCeilingV1 {
        body: true,
        target: true,
    };
    for bad in [vec![], vec![g[0].clone(), g[0].clone()], foreign] {
        let before = o.annotation_preparation_usage();
        assert!(
            o.prepare_collection_visibility_v1(&s, ceiling, &facts(&s, &bad), &mut r)
                .is_err()
        );
        assert_eq!(o.annotation_preparation_usage(), before);
    }
    for permission in [None, Some(false)] {
        let mut f = facts(&s, &g);
        f.visibility_complete = permission;
        assert!(
            o.prepare_collection_visibility_v1(&s, ceiling, &f, &mut r)
                .is_err()
        );
        let mut f = facts(&s, &g);
        f.projection_order_attested = permission;
        assert!(
            o.prepare_collection_visibility_v1(&s, ceiling, &f, &mut r)
                .is_err()
        );
    }
    assert_eq!(o.annotation_preparation_usage().issued_lifetime, 0);
}

#[test]
fn collection_visibility_capture_current_policy_and_transfer_are_separate_from_root_equality() {
    let (mut o, mut r, _, s, g) = ready(limits());
    let ceiling = CollectionFacetCeilingV1 {
        body: true,
        target: false,
    };
    let packet = o
        .prepare_collection_visibility_v1(&s, ceiling, &facts(&s, &g), &mut r)
        .unwrap();
    for bad in [None, Some(false)] {
        let n = crate::coordinator::publication_stream::tests::native(&s);
        let mut f = current(&n);
        f.projection_current = None;
        assert!(
            o.consume_collection_visibility_v1(&packet, &s, ceiling, &f, &mut r)
                .is_err()
        );
        let mut n = n;
        n.transfer_ready = bad;
        assert!(
            o.consume_collection_visibility_v1(&packet, &s, ceiling, &current(&n), &mut r)
                .is_err()
        );
        assert_eq!(o.annotation_preparation_usage().prepared, 1);
    }
    let n = crate::coordinator::publication_stream::tests::native(&s);
    o.consume_collection_visibility_v1(&packet, &s, ceiling, &current(&n), &mut r)
        .unwrap();
    assert_eq!(o.annotation_preparation_usage().issued_lifetime, 1);
}

#[test]
fn collection_visibility_attributed_loss_invalidates_but_foreign_claim_cannot() {
    let (mut o, mut r, _, s, g) = ready(limits());
    let c = CollectionFacetCeilingV1 {
        body: true,
        target: true,
    };
    let id = o
        .prepare_collection_visibility_v1(&s, c, &facts(&s, &g), &mut r)
        .unwrap();
    let mut wrong = s.clone();
    wrong.context.recipient = "other".into();
    let mut n = crate::coordinator::publication_stream::tests::native(&wrong);
    n.authorized = Some(false);
    assert!(
        o.validate_collection_visibility_v1(&id, &s, c, &current(&n), &mut r)
            .is_err()
    );
    assert_eq!(o.annotation_preparation_usage().invalidated, 0);
    let n = crate::coordinator::publication_stream::tests::native(&s);
    let mut revoked = current(&n);
    revoked.projection_current = Some(false);
    assert!(
        o.validate_collection_visibility_v1(&id, &s, c, &revoked, &mut r)
            .is_err()
    );
    assert_eq!(o.annotation_preparation_usage().invalidated, 1);
    assert!(
        o.consume_collection_visibility_v1(&id, &s, c, &current(&n), &mut r)
            .is_err()
    );
    o.retire_collection_visibility_v1(&id, &cleanup(&s), &mut r)
        .unwrap();
}

#[test]
fn collection_visibility_ordinary_projection_alias_is_not_a_collection_packet() {
    let (mut o, mut r, _, s, g) = ready(limits());
    let ordinary = o
        .prepare_annotation_projection(
            AnnotationProjectionClassesV1::Annotations,
            &s,
            &facts(&s, &g),
            &mut r,
        )
        .unwrap();
    // Deliberate private-role substitution, unavailable to SDK constructors.
    let fake = CollectionVisibilityPreparationIdV1 {
        inner: ordinary.clone(),
    };
    let n = crate::coordinator::publication_stream::tests::native(&s);
    let c = CollectionFacetCeilingV1 {
        body: true,
        target: true,
    };
    assert!(
        o.consume_collection_visibility_v1(&fake, &s, c, &current(&n), &mut r)
            .is_err()
    );
    assert!(
        o.retire_collection_visibility_v1(&fake, &cleanup(&s), &mut r)
            .is_err()
    );
    assert!(o.prepared_annotation_cut(&ordinary, &s).is_ok());
}

fn add(
    o: &mut SourceCoordinator,
    s: &SourceReadScopeV1,
    body: &str,
    r: &mut Recorder,
) -> AnnotationId {
    let source = o.read(&s.document, r).unwrap().version;
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                target: SourceTarget {
                    source,
                    basis: TargetBasis::Before,
                    target: AttachmentTarget::Document,
                },
                body: body.into(),
            }],
            r,
        )
        .unwrap();
    let id = p.created_annotations().next().unwrap().clone();
    o.commit(p, r).unwrap();
    id
}

#[test]
fn collection_visibility_filters_before_capture_and_honors_approved_visible_order() {
    let mut l = limits();
    l.max_capture_bytes = size_of::<Projection>() + 2 * size_of::<Row>();
    let (mut o, mut r, _, s, mut g) = ready(l);
    let b = add(&mut o, &s, "private b", &mut r);
    let c = add(&mut o, &s, "private c", &mut r);
    g[0].visible = false;
    g[0].body = false;
    g[0].target = false;
    g.insert(
        0,
        AnnotationFacetGrantV1 {
            annotation: c.clone(),
            visible: true,
            body: true,
            target: true,
        },
    );
    g.push(AnnotationFacetGrantV1 {
        annotation: b.clone(),
        visible: true,
        body: true,
        target: true,
    });
    let ceiling = CollectionFacetCeilingV1 {
        body: false,
        target: false,
    };
    let mut f = facts(&s, &g);
    f.source_readable = Some(false);
    let id = o
        .prepare_collection_visibility_v1(&s, ceiling, &f, &mut r)
        .unwrap();
    assert_eq!(o.annotation_preparation_usage().body_bytes, 0);
    let mut n = crate::coordinator::publication_stream::tests::native(&s);
    n.source_readable = None;
    let capture = o
        .consume_collection_visibility_v1(&id, &s, ceiling, &current(&n), &mut r)
        .unwrap();
    let rows: Vec<_> = capture.rows().collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, &c);
    assert_eq!(rows[1].0, &b);
    assert!(
        rows.iter()
            .all(|(_, body, target)| body.is_none() && target.is_none())
    );
}

#[test]
fn collection_visibility_hidden_only_native_changes_do_not_change_visible_projection() {
    let (mut o, mut r, _, s, mut g) = ready(limits());
    let c = CollectionFacetCeilingV1 {
        body: true,
        target: true,
    };
    let n = crate::coordinator::publication_stream::tests::native(&s);
    let id = o
        .prepare_collection_visibility_v1(&s, c, &facts(&s, &g), &mut r)
        .unwrap();
    let before = o
        .consume_collection_visibility_v1(&id, &s, c, &current(&n), &mut r)
        .unwrap();
    let hidden = add(&mut o, &s, "hidden new annotation", &mut r);
    g.insert(
        0,
        AnnotationFacetGrantV1 {
            annotation: hidden,
            visible: false,
            body: false,
            target: false,
        },
    );
    let id = o
        .prepare_collection_visibility_v1(&s, c, &facts(&s, &g), &mut r)
        .unwrap();
    let after = o
        .consume_collection_visibility_v1(&id, &s, c, &current(&n), &mut r)
        .unwrap();
    assert_eq!(
        before.rows().collect::<Vec<_>>(),
        after.rows().collect::<Vec<_>>()
    );
}

#[test]
fn collection_visibility_lifetime_bound_and_capture_modes_preserve_native_obligations() {
    use crate::instrumentation::NativeTimingProfileV1;
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 64] {
            let mut l = limits();
            l.max_prepared = 1;
            l.max_issued = 1;
            let (mut o, mut setup, _, s, g) = ready(l);
            let debt = o.retention_usage();
            let root = o.generation();
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            let c = CollectionFacetCeilingV1 {
                body: true,
                target: false,
            };
            let id = o
                .prepare_collection_visibility_v1(&s, c, &facts(&s, &g), &mut r)
                .unwrap();
            assert_eq!(
                o.document_read_pins_v1(&s.document, &mut setup)
                    .unwrap()
                    .annotation_preparations,
                1
            );
            assert!(
                o.prepare_collection_visibility_v1(&s, c, &facts(&s, &g), &mut r)
                    .is_err()
            );
            o.retire_collection_visibility_v1(&id, &cleanup(&s), &mut r)
                .unwrap();
            assert_eq!(
                o.document_read_pins_v1(&s.document, &mut setup)
                    .unwrap()
                    .annotation_preparations,
                0
            );
            assert!(
                o.prepare_collection_visibility_v1(&s, c, &facts(&s, &g), &mut r)
                    .is_err()
            );
            assert_eq!(o.retention_usage(), debt);
            assert_eq!(o.generation(), root);
            assert_eq!(o.annotation_preparation_usage().issued_lifetime, 1);
            if capacity == 64 {
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == Boundary::CollectionVisibilityPreparation
                            && e.service_time.is_some()
                                == (timing == NativeTimingProfileV1::ProcessMonotonicV1))
                );
            }
        }
    }
}

#[test]
fn collection_visibility_retained_packet_blocks_native_reclamation_until_retirement() {
    let (mut o, mut r, d, s, g) = ready(limits());
    let c = CollectionFacetCeilingV1 {
        body: true,
        target: true,
    };
    let id = o
        .prepare_collection_visibility_v1(&s, c, &facts(&s, &g), &mut r)
        .unwrap();
    let set = o.annotation_set_version(&s.document, &mut r).unwrap();
    let plan = o.prepare_retained_disposal(&d, set, &mut r).unwrap();
    let state = o.commit_retained_disposal(plan, &mut r).unwrap();
    o.release_disposed_provenance(&s.document, state, &mut r)
        .unwrap();
    let state = o.lifecycle_state(&s.document, &mut r).unwrap();
    let yes = TrustedDocumentReclamationV1 {
        authorized: Some(true),
        authored_disposition_ready: Some(true),
        session_references_released: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    assert_eq!(
        o.reclaim_disposed_document_v1(&s.document, state, &yes, &mut r),
        Err(CoordinatorError::Resource(
            crate::resource::ResourceError::Retained
        ))
    );
    o.retire_collection_visibility_v1(&id, &cleanup(&s), &mut r)
        .unwrap();
    o.reclaim_disposed_document_v1(&s.document, state, &yes, &mut r)
        .unwrap();
}
