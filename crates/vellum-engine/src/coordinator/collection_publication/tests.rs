use super::*;
#[test]
fn shutdown_inventory_retains_native_collection_stream_and_document_dependency() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    let (mut o, mut r, s, grants) = fixture(limits());
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let p = packet(&mut o, &s, &grants, &mut r);
    o.open_collection_publication_stream_v1(&s, &p, &f, &mut r)
        .unwrap();
    let before = o.collection_publication_usage_v1().unwrap();
    let row = inventory_owner(
        &o,
        &mut r,
        ShutdownOwnerRoleV1::CollectionPublication,
        &[&s.source.document],
    );
    assert_eq!(row.retained_records, Some(1));
    assert_eq!(o.collection_publication_usage_v1().unwrap(), before);
}

#[test]
fn heterogeneous_event_collection_preserves_ledger_payload_and_current_denial() {
    let (mut o, mut r, s, grants) = fixture(limits());
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let mut f = facts(&s, &n);
    let p = packet(&mut o, &s, &grants, &mut r);
    let opened = o
        .open_collection_publication_stream_v1(&s, &p, &f, &mut r)
        .unwrap();
    body(&mut o, &grants[0].annotation, "new body", &mut r);
    let p = packet(&mut o, &s, &grants, &mut r);
    o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    let head = o
        .collection_publication_record(&opened.stream)
        .unwrap()
        .queue
        .front()
        .unwrap()
        .clone();
    f.max_reply_bytes = frame_bytes(&head.projection).unwrap();
    let before = o.collection_publication_usage_v1().unwrap();
    assert!(matches!(
        o.release_observation_event_v1(
            ObservationEventTargetV1::Collection(&opened.stream),
            ObservationEventPortV1::Collection {
                stream: &opened.stream,
                scope: &s,
                facts: &f
            },
            &mut r
        ),
        Err(ObservationEventErrorV1::Collection(
            CollectionPublicationErrorV1::Limit
        ))
    ));
    assert_eq!(o.collection_publication_usage_v1().unwrap(), before);
    f.max_reply_bytes = limits().max_reply_bytes;
    let ObservationEventDeliveryV1::Collection(CollectionPublicationDeliveryV1::Projection(event)) =
        o.release_observation_event_v1(
            ObservationEventTargetV1::Collection(&opened.stream),
            ObservationEventPortV1::Collection {
                stream: &opened.stream,
                scope: &s,
                facts: &f,
            },
            &mut r,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(event.before, opened.generation);
    assert_eq!(event.after, head.after);
    assert!(Arc::ptr_eq(&event.projection, &head.projection));
    f.projection_allowed = Some(false);
    assert!(
        o.release_observation_event_v1(
            ObservationEventTargetV1::Collection(&opened.stream),
            ObservationEventPortV1::Collection {
                stream: &opened.stream,
                scope: &s,
                facts: &f
            },
            &mut r
        )
        .is_err()
    );
    assert_eq!(o.collection_publication_usage_v1().unwrap().stopped, 1);
}

#[test]
fn stop_v3_collection_publication_uses_original_owner_and_protected_control() {
    for typed in [false, true] {
        let (mut o, mut setup, s, grants) = fixture(limits());
        let n = crate::coordinator::publication_stream::tests::native(&s.source);
        let f = facts(&s, &n);
        let p = packet(&mut o, &s, &grants, &mut setup);
        let one = o
            .open_collection_publication_stream_v1(&s, &p, &f, &mut setup)
            .unwrap();
        let p = packet(&mut o, &s, &grants, &mut setup);
        let two = o
            .open_collection_publication_stream_v1(&s, &p, &f, &mut setup)
            .unwrap();
        let debt = o.retention_usage();
        let before = o.collection_publication_usage_v1().unwrap();
        o.faulted = true;
        let mut r = Recorder::try_with_capacity(1).unwrap();
        for (target, allowed, expected) in [
            (&two.stream, Some(true), false),
            (&one.stream, None, false),
            (&one.stream, Some(false), false),
            (&one.stream, Some(true), true),
            (&one.stream, Some(true), true),
        ] {
            let control = CollectionPublicationControlV1 {
                scope: &s,
                owner_control_allowed: allowed,
                retention_released: None,
                delivery_drained: None,
            };
            let result = crate::coordinator::observation_control::request_v3(
                ObservationStopTargetV3::CollectionPublication(target),
                typed,
                "stop-observation",
                "stop-observation",
                |envelope| {
                    o.stop_decoded_observation_facade_v3(
                        DecodedObservationStopRequestV3 {
                            envelope,
                            native: ObservationStopPortV3::CollectionPublication {
                                stream: &one.stream,
                                control: &control,
                            },
                        },
                        &mut r,
                    )
                },
            );
            assert_eq!(result.is_ok(), expected);
        }
        let after = o.collection_publication_usage_v1().unwrap();
        assert_eq!(after.streams, 2);
        assert_eq!(after.stopped, 1);
        assert_eq!(after.issued_lifetime, before.issued_lifetime);
        assert_eq!(o.retention_usage(), debt);
        assert!(o.faulted);
    }
}

use crate::attachment::AttachmentTarget;
pub(in crate::coordinator) fn limits() -> CollectionPublicationLimitsV1 {
    CollectionPublicationLimitsV1 {
        max_streams: 2,
        max_bindings: 8,
        max_binding_bytes: 4096,
        max_issued: 64,
        max_context_bytes: 4096,
        max_projection_bytes: 8192,
        max_reply_bytes: 16384,
        max_queue: 2,
        max_pool_bytes: 524288,
    }
}
pub(in crate::coordinator) fn control() -> SourceReadPoolControlV1 {
    SourceReadPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(in crate::coordinator) fn cleanup(
    s: &CollectionPublicationScopeV1,
) -> CollectionPublicationControlV1<'_> {
    CollectionPublicationControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(in crate::coordinator) fn fixture(
    l: CollectionPublicationLimitsV1,
) -> (
    SourceCoordinator,
    Recorder,
    CollectionPublicationScopeV1,
    Vec<AnnotationFacetGrantV1>,
) {
    fixture_with_budget(l, 1048576)
}
pub(in crate::coordinator) fn fixture_with_budget(
    l: CollectionPublicationLimitsV1,
    maximum: usize,
) -> (
    SourceCoordinator,
    Recorder,
    CollectionPublicationScopeV1,
    Vec<AnnotationFacetGrantV1>,
) {
    let (mut o, mut r, source) = crate::coordinator::publication_stream::tests::owner_with_budget(
        crate::coordinator::publication_stream::tests::limits(),
        maximum,
    );
    let a = add(&mut o, &source, "one", &mut r);
    let p = AnnotationPreparationLimitsV1 {
        max_prepared: 4,
        max_issued: 64,
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
    };
    o.configure_annotation_preparation("local", "engine", p, &control(), &mut r)
        .unwrap();
    o.configure_collection_publication_streams_v1("local", "engine", l, &control(), &mut r)
        .unwrap();
    (
        o,
        r,
        CollectionPublicationScopeV1 {
            source,
            ceiling: CollectionFacetCeilingV1 {
                body: true,
                target: true,
            },
        },
        vec![grant(a, true)],
    )
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
pub(super) fn body(o: &mut SourceCoordinator, a: &AnnotationId, b: &str, r: &mut Recorder) {
    let expected_content = o.annotation(a, r).unwrap().content_version;
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: a.clone(),
                expected_content,
                body: b.into(),
            }],
            r,
        )
        .unwrap();
    o.commit(p, r).unwrap();
}
fn grant(a: AnnotationId, visible: bool) -> AnnotationFacetGrantV1 {
    AnnotationFacetGrantV1 {
        annotation: a,
        visible,
        body: visible,
        target: visible,
    }
}
pub(in crate::coordinator) fn packet(
    o: &mut SourceCoordinator,
    s: &CollectionPublicationScopeV1,
    g: &[AnnotationFacetGrantV1],
    r: &mut Recorder,
) -> CollectionVisibilityPreparationIdV1 {
    o.prepare_collection_visibility_v1(
        &s.source,
        s.ceiling,
        &TrustedAnnotationCaptureFactsV1 {
            scope: &s.source,
            grants: g,
            authorized: Some(true),
            scope_allowed: Some(true),
            continuous: Some(true),
            live: Some(true),
            not_stopped: Some(true),
            annotation_readable: Some(true),
            source_readable: None,
            visibility_complete: Some(true),
            projection_order_attested: Some(true),
            resources_ready: Some(true),
        },
        r,
    )
    .unwrap()
}
pub(in crate::coordinator) fn facts<'a>(
    s: &'a CollectionPublicationScopeV1,
    n: &'a TrustedSourceReadFactsV1<'a>,
) -> TrustedCollectionPublicationFactsV1<'a> {
    TrustedCollectionPublicationFactsV1 {
        original_scope: s,
        current: n,
        projection_allowed: Some(true),
        baseline_allowed: Some(true),
        max_reply_bytes: 16384,
        annotation_readable: Some(true),
    }
}
fn event(
    o: &mut SourceCoordinator,
    id: &CollectionPublicationStreamIdV1,
    s: &CollectionPublicationScopeV1,
    f: &TrustedCollectionPublicationFactsV1<'_>,
    r: &mut Recorder,
) -> CollectionPublicationEventV1 {
    let CollectionPublicationDeliveryV1::Projection(e) = o
        .release_collection_publication_event_v1(id, s, f, r)
        .unwrap()
    else {
        panic!("expected complete projection")
    };
    e
}

#[test]
fn collection_stream_hidden_changes_spend_no_visible_progress_or_binding() {
    let (mut o, mut r, s, mut g) = fixture(limits());
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let p = packet(&mut o, &s, &g, &mut r);
    let opened = o
        .open_collection_publication_stream_v1(&s, &p, &f, &mut r)
        .unwrap();
    let reference = opened.projection.rows()[0].reference.clone();
    let usage = o.collection_publication_usage_v1().unwrap();
    let hidden = add(&mut o, &s.source, "hidden", &mut r);
    g.insert(0, grant(hidden.clone(), false));
    body(&mut o, &hidden, "hidden updated", &mut r);
    let p = packet(&mut o, &s, &g, &mut r);
    assert_eq!(
        o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r),
        Ok(CollectionPublicationSampleV1::Unchanged)
    );
    assert_eq!(o.collection_publication_usage_v1().unwrap(), usage);
    body(&mut o, &g[1].annotation, "visible changed", &mut r);
    let p = packet(&mut o, &s, &g, &mut r);
    assert_eq!(
        o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r),
        Ok(CollectionPublicationSampleV1::Queued)
    );
    let e = event(&mut o, &opened.stream, &s, &f, &mut r);
    assert_eq!(e.before, opened.generation);
    assert_eq!(e.projection.rows()[0].reference, reference);
    assert_eq!(
        e.projection.rows()[0].content.as_ref().unwrap().0,
        "visible changed"
    );
}

#[test]
fn collection_stream_visible_removal_and_return_preserve_reference_without_claiming_deletion() {
    let (mut o, mut r, s, mut g) = fixture(limits());
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let p = packet(&mut o, &s, &g, &mut r);
    let opened = o
        .open_collection_publication_stream_v1(&s, &p, &f, &mut r)
        .unwrap();
    let original = opened.projection.rows()[0].reference.clone();
    let a = g[0].annotation.clone();
    g[0] = grant(a.clone(), false);
    let p = packet(&mut o, &s, &g, &mut r);
    o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    assert!(
        event(&mut o, &opened.stream, &s, &f, &mut r)
            .projection
            .rows()
            .is_empty()
    );
    assert_eq!(o.collection_publication_usage_v1().unwrap().bindings, 1);
    g[0] = grant(a, true);
    let p = packet(&mut o, &s, &g, &mut r);
    o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    assert_eq!(
        event(&mut o, &opened.stream, &s, &f, &mut r)
            .projection
            .rows()[0]
            .reference,
        original
    );
}

#[test]
fn collection_stream_resync_installs_new_bindings_and_their_lifetime_charge_together() {
    let (mut o, mut r, s, mut g) = fixture(limits());
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let p = packet(&mut o, &s, &g, &mut r);
    let opened = o
        .open_collection_publication_stream_v1(&s, &p, &f, &mut r)
        .unwrap();
    let before = o.collection_publication_usage_v1().unwrap();
    let a = add(&mut o, &s.source, "new visible", &mut r);
    g.push(grant(a, true));
    let p = packet(&mut o, &s, &g, &mut r);
    let fresh = o
        .resync_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    let after = o.collection_publication_usage_v1().unwrap();
    assert_eq!(after.bindings, 2);
    assert_eq!(after.issued_lifetime, before.issued_lifetime + 2);
    assert_eq!(fresh.projection.rows().len(), 2);
    assert_ne!(
        fresh.projection.rows()[0].reference,
        fresh.projection.rows()[1].reference
    );
    let p = packet(&mut o, &s, &g, &mut r);
    assert_eq!(
        o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r),
        Ok(CollectionPublicationSampleV1::Unchanged)
    );
}

#[test]
fn collection_stream_new_native_incarnation_never_reuses_removed_reference() {
    let (mut o, mut r, s, mut g) = fixture(limits());
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let p = packet(&mut o, &s, &g, &mut r);
    let opened = o
        .open_collection_publication_stream_v1(&s, &p, &f, &mut r)
        .unwrap();
    let oldref = opened.projection.rows()[0].reference.clone();
    let old = o.annotation(&g[0].annotation, &mut r).unwrap();
    let mutation = AnnotationMutation::Delete {
        annotation: g[0].annotation.clone(),
        expected_content: old.content_version,
        expected_target: old.target_generation,
    };
    let p = o.prepare_compound(&[], &[], &[mutation], &mut r).unwrap();
    o.commit(p, &mut r).unwrap();
    let new = add(&mut o, &s.source, "replacement", &mut r);
    g = vec![grant(new, true)];
    let p = packet(&mut o, &s, &g, &mut r);
    o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    let e = event(&mut o, &opened.stream, &s, &f, &mut r);
    assert_ne!(e.projection.rows()[0].reference, oldref);
    assert_eq!(o.collection_publication_usage_v1().unwrap().bindings, 2);
}

#[test]
fn collection_stream_pressure_does_not_partially_install_binding_or_refill_issuance() {
    for by_bytes in [false, true] {
        let mut l = limits();
        if by_bytes {
            l.max_binding_bytes = 2 * size_of::<Binding>();
        } else {
            l.max_bindings = 2;
        }
        let (mut o, mut r, s, mut g) = fixture(l);
        let n = crate::coordinator::publication_stream::tests::native(&s.source);
        let f = facts(&s, &n);
        let p = packet(&mut o, &s, &g, &mut r);
        let opened = o
            .open_collection_publication_stream_v1(&s, &p, &f, &mut r)
            .unwrap();
        let two = add(&mut o, &s.source, "two", &mut r);
        g.push(grant(two, true));
        let p = packet(&mut o, &s, &g, &mut r);
        o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
            .unwrap();
        let accepted = event(&mut o, &opened.stream, &s, &f, &mut r);
        let before = o.collection_publication_usage_v1().unwrap();
        let three = add(&mut o, &s.source, "three", &mut r);
        g.push(grant(three.clone(), true));
        let p = packet(&mut o, &s, &g, &mut r);
        assert_eq!(
            o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r),
            Ok(CollectionPublicationSampleV1::Gap)
        );
        let failed = o.collection_publication_usage_v1().unwrap();
        assert_eq!(failed.bindings, before.bindings);
        assert_eq!(failed.issued_lifetime, before.issued_lifetime);
        assert_eq!(failed.queued, 0);
        g[2] = grant(three, false);
        let p = packet(&mut o, &s, &g, &mut r);
        let fresh = o
            .resync_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
            .unwrap();
        assert_eq!(
            fresh.projection.rows()[0].reference,
            accepted.projection.rows()[0].reference
        );
        assert_eq!(
            fresh.projection.rows()[1].reference,
            accepted.projection.rows()[1].reference
        );
        assert_eq!(o.collection_publication_usage_v1().unwrap().bindings, 2);
    }
}

#[test]
fn collection_stream_exact_old_heads_and_overflow_resync_do_not_forge_a_suffix() {
    let (mut o, mut r, s, g) = fixture(limits());
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let mut f = facts(&s, &n);
    let p = packet(&mut o, &s, &g, &mut r);
    let opened = o
        .open_collection_publication_stream_v1(&s, &p, &f, &mut r)
        .unwrap();
    for text in ["first", "second"] {
        body(&mut o, &g[0].annotation, text, &mut r);
        let p = packet(&mut o, &s, &g, &mut r);
        o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
            .unwrap();
    }
    f.max_reply_bytes = 0;
    assert!(matches!(
        o.release_collection_publication_event_v1(&opened.stream, &s, &f, &mut r),
        Err(Error::Limit)
    ));
    assert_eq!(o.collection_publication_usage_v1().unwrap().queued, 2);
    f.max_reply_bytes = 16384;
    let first = event(&mut o, &opened.stream, &s, &f, &mut r);
    let second = event(&mut o, &opened.stream, &s, &f, &mut r);
    assert_eq!(first.after, second.before);
    assert_eq!(
        first.projection.rows()[0].content.as_ref().unwrap().0,
        "first"
    );
    for text in ["three", "four", "five"] {
        body(&mut o, &g[0].annotation, text, &mut r);
        let p = packet(&mut o, &s, &g, &mut r);
        o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
            .unwrap();
    }
    assert!(matches!(
        o.release_collection_publication_event_v1(&opened.stream, &s, &f, &mut r),
        Ok(CollectionPublicationDeliveryV1::Gap)
    ));
    let p = packet(&mut o, &s, &g, &mut r);
    f.baseline_allowed = Some(false);
    assert!(
        o.resync_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
            .is_err()
    );
    f.baseline_allowed = Some(true);
    let fresh = o
        .resync_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    assert_eq!(
        fresh.projection.rows()[0].content.as_ref().unwrap().0,
        "five"
    );
    assert!(matches!(
        o.release_collection_publication_event_v1(&opened.stream, &s, &f, &mut r),
        Ok(CollectionPublicationDeliveryV1::Empty)
    ));
}

#[test]
fn collection_stream_wrong_claim_cannot_apply_loss_and_real_loss_never_rearms() {
    let (mut o, mut r, s, g) = fixture(limits());
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let mut f = facts(&s, &n);
    let p = packet(&mut o, &s, &g, &mut r);
    let opened = o
        .open_collection_publication_stream_v1(&s, &p, &f, &mut r)
        .unwrap();
    let mut wrong = s.clone();
    wrong.source.context.recipient = "other".into();
    f.annotation_readable = Some(false);
    assert!(
        o.release_collection_publication_event_v1(&opened.stream, &wrong, &f, &mut r)
            .is_err()
    );
    assert_eq!(o.collection_publication_usage_v1().unwrap().stopped, 0);
    assert!(
        o.release_collection_publication_event_v1(&opened.stream, &s, &f, &mut r)
            .is_err()
    );
    assert_eq!(o.collection_publication_usage_v1().unwrap().stopped, 1);
    f.annotation_readable = Some(true);
    let p = packet(&mut o, &s, &g, &mut r);
    assert!(
        o.resync_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
            .is_err()
    );
}

#[test]
fn collection_stream_capture_modes_and_protected_cleanup_preserve_native_retention() {
    use crate::instrumentation::NativeTimingProfileV1;
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 64] {
            let (mut o, mut setup, s, g) = fixture(limits());
            let n = crate::coordinator::publication_stream::tests::native(&s.source);
            let f = facts(&s, &n);
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            let p = packet(&mut o, &s, &g, &mut setup);
            let opened = o
                .open_collection_publication_stream_v1(&s, &p, &f, &mut r)
                .unwrap();
            body(&mut o, &g[0].annotation, "changed", &mut setup);
            let p = packet(&mut o, &s, &g, &mut setup);
            o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
                .unwrap();
            assert_eq!(
                event(&mut o, &opened.stream, &s, &f, &mut r)
                    .projection
                    .rows()[0]
                    .content
                    .as_ref()
                    .unwrap()
                    .0,
                "changed"
            );
            assert_eq!(
                o.document_context_pins_v1(&s.source.document, &mut setup)
                    .unwrap()
                    .publication_streams,
                1
            );
            let debt = o.retention_usage();
            let root = o.generation();
            let issued = o.collection_publication_usage_v1().unwrap().issued_lifetime;
            o.faulted = true;
            o.stop_collection_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
                .unwrap();
            assert!(o.is_faulted());
            assert_eq!(o.generation(), root);
            assert_eq!(o.retention_usage(), debt);
            let mut incomplete = cleanup(&s);
            incomplete.delivery_drained = Some(false);
            assert!(
                o.retire_collection_publication_stream_v1(&opened.stream, &incomplete, &mut r)
                    .is_err()
            );
            o.faulted = false;
            assert_eq!(
                o.document_context_pins_v1(&s.source.document, &mut setup)
                    .unwrap()
                    .publication_streams,
                1
            );
            o.retire_collection_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
                .unwrap();
            assert_eq!(
                o.document_context_pins_v1(&s.source.document, &mut setup)
                    .unwrap()
                    .publication_streams,
                0
            );
            o.retire_collection_publication_pool_v1(&control(), &mut r)
                .unwrap();
            assert_eq!(
                o.collection_publication_usage_v1().unwrap().issued_lifetime,
                issued
            );
            assert!(
                o.configure_collection_publication_streams_v1(
                    "local",
                    "engine",
                    limits(),
                    &control(),
                    &mut r
                )
                .is_err()
            );
            if capacity == 64 {
                let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == Boundary::CollectionPublicationSample
                            && e.service_time.is_some()
                                == (timing == NativeTimingProfileV1::ProcessMonotonicV1))
                );
            }
        }
    }
}

#[test]
fn collection_stream_foreign_owner_ids_generations_and_references_never_alias() {
    let (mut a, mut ar, s, g) = fixture(limits());
    let (mut b, mut br, bs, bg) = fixture(limits());
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let bn = crate::coordinator::publication_stream::tests::native(&bs.source);
    let f = facts(&s, &n);
    let bf = facts(&bs, &bn);
    let p = packet(&mut a, &s, &g, &mut ar);
    let bp = packet(&mut b, &bs, &bg, &mut br);
    let local = a
        .open_collection_publication_stream_v1(&s, &p, &f, &mut ar)
        .unwrap();
    let foreign = b
        .open_collection_publication_stream_v1(&bs, &bp, &bf, &mut br)
        .unwrap();
    assert_ne!(local.stream, foreign.stream);
    assert_ne!(local.generation, foreign.generation);
    assert_ne!(
        local.projection.rows()[0].reference,
        foreign.projection.rows()[0].reference
    );
    assert!(
        a.stop_collection_publication_stream_v1(&foreign.stream, &cleanup(&s), &mut ar)
            .is_err()
    );
    assert_eq!(a.collection_publication_usage_v1().unwrap().stopped, 0);
}

#[test]
fn collection_stream_native_document_reclamation_waits_for_actual_row_retirement() {
    let (mut o, mut r, s, g) = fixture(limits());
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let p = packet(&mut o, &s, &g, &mut r);
    let opened = o
        .open_collection_publication_stream_v1(&s, &p, &f, &mut r)
        .unwrap();
    let source = o.read(&s.source.document, &mut r).unwrap().version;
    let set = o
        .annotation_set_version(&s.source.document, &mut r)
        .unwrap();
    let plan = o.prepare_retained_disposal(&source, set, &mut r).unwrap();
    let state = o.commit_retained_disposal(plan, &mut r).unwrap();
    o.release_disposed_provenance(&s.source.document, state, &mut r)
        .unwrap();
    let state = o.lifecycle_state(&s.source.document, &mut r).unwrap();
    assert!(
        o.release_collection_publication_event_v1(&opened.stream, &s, &f, &mut r)
            .is_err()
    );
    let yes = TrustedDocumentReclamationV1 {
        authorized: Some(true),
        authored_disposition_ready: Some(true),
        session_references_released: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    assert_eq!(
        o.reclaim_disposed_document_v1(&s.source.document, state, &yes, &mut r),
        Err(CoordinatorError::Resource(
            crate::resource::ResourceError::Retained
        ))
    );
    o.retire_collection_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
        .unwrap();
    o.reclaim_disposed_document_v1(&s.source.document, state, &yes, &mut r)
        .unwrap();
}

#[test]
fn collection_stream_lifetime_ceiling_admits_no_partial_reference_and_hidden_equality_needs_no_credit()
 {
    let mut l = limits();
    l.max_issued = 3;
    let (mut o, mut r, s, mut g) = fixture(l);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let p = packet(&mut o, &s, &g, &mut r);
    let opened = o
        .open_collection_publication_stream_v1(&s, &p, &f, &mut r)
        .unwrap();
    let added = add(&mut o, &s.source, "new visible", &mut r);
    g.push(grant(added.clone(), true));
    let p = packet(&mut o, &s, &g, &mut r);
    assert_eq!(
        o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r),
        Ok(CollectionPublicationSampleV1::Gap)
    );
    let failed = o.collection_publication_usage_v1().unwrap();
    assert_eq!(failed.issued_lifetime, 2);
    assert_eq!(failed.bindings, 1);
    g[1] = grant(added, false);
    let p = packet(&mut o, &s, &g, &mut r);
    o.resync_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
        .unwrap();
    let full = o.collection_publication_usage_v1().unwrap();
    assert_eq!(full.issued_lifetime, 3);
    let p = packet(&mut o, &s, &g, &mut r);
    assert_eq!(
        o.sample_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r),
        Ok(CollectionPublicationSampleV1::Unchanged)
    );
    assert_eq!(o.collection_publication_usage_v1().unwrap(), full);
    let p = packet(&mut o, &s, &g, &mut r);
    assert!(
        o.resync_collection_publication_stream_v1(&opened.stream, &s, &p, &f, &mut r)
            .is_err()
    );
    o.stop_collection_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_collection_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
        .unwrap();
    assert_eq!(
        o.collection_publication_usage_v1().unwrap().issued_lifetime,
        3
    );
}

#[test]
fn collection_stream_foreign_frame_never_hands_off_and_does_not_disable_cleanup() {
    let (mut o, mut r, s, g) = fixture(limits());
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let f = facts(&s, &n);
    let p = packet(&mut o, &s, &g, &mut r);
    let local = o
        .open_collection_publication_stream_v1(&s, &p, &f, &mut r)
        .unwrap();
    let p = packet(&mut o, &s, &g, &mut r);
    let other = o
        .open_collection_publication_stream_v1(&s, &p, &f, &mut r)
        .unwrap();
    body(&mut o, &g[0].annotation, "queued", &mut r);
    let p = packet(&mut o, &s, &g, &mut r);
    o.sample_collection_publication_stream_v1(&local.stream, &s, &p, &f, &mut r)
        .unwrap();
    o.collection_publication_record_mut(&local.stream)
        .queue
        .front_mut()
        .unwrap()
        .projection = other.projection;
    assert!(matches!(
        o.release_collection_publication_event_v1(&local.stream, &s, &f, &mut r),
        Err(Error::Faulted)
    ));
    assert!(!o.is_faulted());
    o.stop_collection_publication_stream_v1(&local.stream, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_collection_publication_stream_v1(&local.stream, &cleanup(&s), &mut r)
        .unwrap();
}
#[test]
fn collection_stream_pool_arithmetic_is_checked() {
    let l = CollectionPublicationLimitsV1 {
        max_streams: usize::MAX,
        max_bindings: 4,
        max_binding_bytes: 1024,
        max_issued: 16,
        max_context_bytes: 2048,
        max_projection_bytes: 8192,
        max_reply_bytes: 16384,
        max_queue: 2,
        max_pool_bytes: usize::MAX,
    };
    assert_eq!(l.required_pool_bytes(), Err(Error::Limit));
}
