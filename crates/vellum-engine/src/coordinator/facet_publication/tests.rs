use super::*;
#[test]
fn publication_admission_fence_blocks_new_facet_pool_without_spending_funding() {
    let mut o = crate::coordinator::publication_stream::tests::bare();
    let mut r = Recorder::try_with_capacity(16).unwrap();
    o.configure_retention_budget(8388608, &mut r).unwrap();
    crate::coordinator::shutdown::tests::install(&mut o, &mut r);
    let debt = o.retention_usage();
    assert_eq!(
        o.configure_facet_publication_streams_v1("local", "engine", limits(), &ctl(), &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(o.retention_usage(), debt);
    assert!(matches!(o.facet_publication_streams, State::Unconfigured));
}
#[test]
fn shutdown_inventory_retains_native_facet_stream_and_document_dependency() {
    use crate::coordinator::shutdown::tests::inventory_owner;
    let (mut o, mut r, s) = fixture(true, true, true, true);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let grants = grants(&s);
    let views: Vec<_> = s.views.iter().map(|v| v.view.clone()).collect();
    let f = facts(&s, &n, &grants, &views);
    o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
    let before = o.facet_publication_usage_v1().unwrap();
    let row = inventory_owner(
        &o,
        &mut r,
        ShutdownOwnerRoleV1::FacetPublication,
        &[&s.source.document],
    );
    assert_eq!(row.retained_records, Some(1));
    assert_eq!(row.state, ShutdownOwnerStateV1::Active);
    assert_eq!(o.facet_publication_usage_v1().unwrap(), before);
}

#[test]
fn heterogeneous_event_named_facets_preserve_exact_native_projection_and_pressure() {
    let (mut o, mut r, s) = fixture(true, true, true, true);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let grants = grants(&s);
    let views: Vec<_> = s.views.iter().map(|v| v.view.clone()).collect();
    let mut f = facts(&s, &n, &grants, &views);
    let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
    edit(&mut o, &s.source, &mut r);
    o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
        .unwrap();
    let head = o
        .facet_publication_record(&opened.stream)
        .unwrap()
        .queue
        .front()
        .unwrap()
        .clone();
    f.max_reply_bytes = frame_bytes(&head.projection).unwrap();
    assert!(matches!(
        o.release_observation_event_v1(
            ObservationEventTargetV1::NamedFacets(&opened.stream),
            ObservationEventPortV1::NamedFacets {
                stream: &opened.stream,
                scope: &s,
                facts: &f
            },
            &mut r
        ),
        Err(ObservationEventErrorV1::NamedFacets(
            FacetPublicationErrorV1::Limit
        ))
    ));
    assert_eq!(o.facet_publication_usage_v1().unwrap().queued, 1);
    f.max_reply_bytes = limits().max_reply_bytes;
    let ObservationEventDeliveryV1::NamedFacets(FacetPublicationDeliveryV1::Projection(event)) = o
        .release_observation_event_v1(
            ObservationEventTargetV1::NamedFacets(&opened.stream),
            ObservationEventPortV1::NamedFacets {
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
}

#[test]
fn stop_v3_facet_publication_uses_original_owner_and_protected_control() {
    for typed in [false, true] {
        let (mut o, mut setup, s) = fixture(false, true, false, false);
        let n = crate::coordinator::publication_stream::tests::native(&s.source);
        let grants = grants(&s);
        let f = facts(&s, &n, &grants, &[]);
        let one = o
            .open_facet_publication_stream_v1(&s, &f, &mut setup)
            .unwrap();
        let two = o
            .open_facet_publication_stream_v1(&s, &f, &mut setup)
            .unwrap();
        let debt = o.retention_usage();
        let before = o.facet_publication_usage_v1().unwrap();
        o.faulted = true;
        let mut r = Recorder::try_with_capacity(1).unwrap();
        for (target, allowed, expected) in [
            (&two.stream, Some(true), false),
            (&one.stream, None, false),
            (&one.stream, Some(false), false),
            (&one.stream, Some(true), true),
            (&one.stream, Some(true), true),
        ] {
            let control = FacetPublicationControlV1 {
                scope: &s,
                owner_control_allowed: allowed,
                retention_released: None,
                delivery_drained: None,
            };
            let result = crate::coordinator::observation_control::request_v3(
                ObservationStopTargetV3::FacetPublication(target),
                typed,
                "stop-observation",
                "stop-observation",
                |envelope| {
                    o.stop_decoded_observation_facade_v3(
                        DecodedObservationStopRequestV3 {
                            envelope,
                            native: ObservationStopPortV3::FacetPublication {
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
        let after = o.facet_publication_usage_v1().unwrap();
        assert_eq!(after.streams, 2);
        assert_eq!(after.stopped, 1);
        assert_eq!(after.issued_lifetime, before.issued_lifetime);
        assert_eq!(o.retention_usage(), debt);
        assert!(o.faulted);
    }
}

use crate::attachment::{Affinity, AttachmentTarget};
use crate::coordinates::BytePosition;
use crate::instrumentation::NativeTimingProfileV1;
use crate::selection::{Selection, SelectionBudget};

#[test]
fn facet_stream_foreign_owner_handles_never_alias_same_numbered_slot() {
    let (mut a, mut ar, s) = fixture(false, true, false, false);
    let (mut b, mut br, bs) = fixture(false, true, false, false);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let bn = crate::coordinator::publication_stream::tests::native(&bs.source);
    let ga = grants(&s);
    let gb = grants(&bs);
    let local = a
        .open_facet_publication_stream_v1(&s, &facts(&s, &n, &ga, &[]), &mut ar)
        .unwrap();
    let foreign = b
        .open_facet_publication_stream_v1(&bs, &facts(&bs, &bn, &gb, &[]), &mut br)
        .unwrap();
    assert_ne!(local.stream, foreign.stream);
    assert_ne!(local.generation, foreign.generation);
    assert_eq!(local.stream, local.stream.clone());
}
fn limits() -> FacetPublicationLimitsV1 {
    FacetPublicationLimitsV1 {
        max_streams: 2,
        max_annotations: 4,
        max_views: 4,
        max_selections: 4,
        max_issued: 32,
        max_context_bytes: 4096,
        max_projection_bytes: 8192,
        max_reply_bytes: 16384,
        max_queue: 2,
        max_pool_bytes: 524288,
    }
}
pub(super) fn ctl() -> SourceReadPoolControlV1 {
    SourceReadPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(super) fn cleanup(s: &FacetPublicationScopeV1) -> FacetPublicationControlV1<'_> {
    FacetPublicationControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn annotation(o: &mut SourceCoordinator, s: &SourceReadScopeV1, r: &mut Recorder) -> AnnotationId {
    let version = o.read(&s.document, r).unwrap().version;
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                target: SourceTarget {
                    source: version,
                    target: AttachmentTarget::ContentRange { start: 0, end: 1 },
                    basis: TargetBasis::Before,
                },
                body: "comment".into(),
            }],
            r,
        )
        .unwrap();
    let a = p.created_annotations().next().unwrap().clone();
    o.commit(p, r).unwrap();
    a
}
fn change_body(o: &mut SourceCoordinator, a: &AnnotationId, text: &str, r: &mut Recorder) {
    let expected_content = o.annotation(a, r).unwrap().content_version;
    let p = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::UpdateBody {
                annotation: a.clone(),
                expected_content,
                body: text.into(),
            }],
            r,
        )
        .unwrap();
    o.commit(p, r).unwrap();
}
pub(super) fn fixture(
    source: bool,
    body: bool,
    target: bool,
    view: bool,
) -> (SourceCoordinator, Recorder, FacetPublicationScopeV1) {
    fixture_with_limits(source, body, target, view, limits())
}
fn fixture_with_limits(
    source: bool,
    body: bool,
    target: bool,
    view: bool,
    l: FacetPublicationLimitsV1,
) -> (SourceCoordinator, Recorder, FacetPublicationScopeV1) {
    fixture_with_limits_and_budget(source, body, target, view, l, 1048576)
}
pub(super) fn fixture_with_budget(
    source: bool,
    body: bool,
    target: bool,
    view: bool,
    maximum: usize,
) -> (SourceCoordinator, Recorder, FacetPublicationScopeV1) {
    fixture_with_limits_and_budget(source, body, target, view, limits(), maximum)
}
fn fixture_with_limits_and_budget(
    source: bool,
    body: bool,
    target: bool,
    view: bool,
    l: FacetPublicationLimitsV1,
    maximum: usize,
) -> (SourceCoordinator, Recorder, FacetPublicationScopeV1) {
    let (mut o, mut r, s) = crate::coordinator::publication_stream::tests::owner_with_budget(
        crate::coordinator::publication_stream::tests::limits(),
        maximum,
    );
    let a = annotation(&mut o, &s, &mut r);
    let mut views = Vec::new();
    if view {
        o.configure_logical_views(
            LogicalViewLimits {
                max_live_views: 4,
                max_issued_views: 8,
                max_total_selections: 16,
                normalization: SelectionBudget {
                    max_input_selections: 8,
                    max_output_selections: 8,
                    max_work: 1024,
                },
                max_mapping_steps: 1024,
            },
            &mut r,
        )
        .unwrap();
        let v = o.create_logical_view(&mut r).unwrap();
        let stamp = o.logical_view(&v, &mut r).unwrap().stamp;
        let version = o.read(&s.document, &mut r).unwrap().version;
        o.attach_logical_view(
            &v,
            stamp,
            &version,
            &[Selection {
                anchor: BytePosition(0),
                head: BytePosition(0),
                affinity: Affinity::Before,
            }],
            0,
            &mut r,
        )
        .unwrap();
        views.push(PublicationViewSelectionV1 {
            view: v,
            reference: "view-a".into(),
        });
    }
    o.configure_facet_publication_streams_v1("local", "engine", l, &ctl(), &mut r)
        .unwrap();
    (
        o,
        r,
        FacetPublicationScopeV1 {
            source: s,
            include_source: source,
            annotations: vec![PublicationAnnotationSelectionV1 {
                annotation: a,
                reference: "comment-a".into(),
                body,
                target,
            }],
            views,
        },
    )
}
pub(in crate::coordinator) fn grants(s: &FacetPublicationScopeV1) -> Vec<AnnotationFacetGrantV1> {
    s.annotations
        .iter()
        .map(|a| AnnotationFacetGrantV1 {
            annotation: a.annotation.clone(),
            visible: true,
            body: a.body,
            target: a.target,
        })
        .collect()
}
pub(in crate::coordinator) fn facts<'a>(
    s: &'a FacetPublicationScopeV1,
    n: &'a TrustedSourceReadFactsV1<'a>,
    a: &'a [AnnotationFacetGrantV1],
    v: &'a [LogicalViewId],
) -> TrustedFacetPublicationFactsV1<'a> {
    TrustedFacetPublicationFactsV1 {
        original_scope: s,
        current: n,
        projection_allowed: Some(true),
        baseline_allowed: Some(true),
        max_reply_bytes: 16384,
        annotation_grants: a,
        view_grants: v,
    }
}
pub(super) fn edit(o: &mut SourceCoordinator, s: &SourceReadScopeV1, r: &mut Recorder) {
    let d = o.read(&s.document, r).unwrap();
    let p = o
        .prepare(
            &[DocumentEdits {
                expected: d.version,
                edits: vec![Replacement {
                    start: d.source.text().len(),
                    end: d.source.text().len(),
                    expected: "".into(),
                    inserted: "!".into(),
                }],
            }],
            &[],
            r,
        )
        .unwrap();
    o.commit(p, r).unwrap();
}

#[test]
fn facet_stream_projects_only_selected_native_facets_and_hides_unselected_changes() {
    for (source, body, target, view) in [
        (false, true, false, false),
        (false, false, true, false),
        (true, true, true, true),
    ] {
        let (mut o, mut r, s) = fixture(source, body, target, view);
        let n = crate::coordinator::publication_stream::tests::native(&s.source);
        let a = grants(&s);
        let v: Vec<_> = s.views.iter().map(|v| v.view.clone()).collect();
        let f = facts(&s, &n, &a, &v);
        let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
        assert_eq!(opened.projection.source().is_some(), source);
        let row = &opened.projection.annotations()[0];
        assert_eq!(row.content.is_some(), body);
        assert_eq!(row.location.is_some(), target);
        assert_eq!(opened.projection.views().len(), usize::from(view));
        let issued = o.facet_publication_usage_v1().unwrap().issued_lifetime;
        let peer = annotation(&mut o, &s.source, &mut r);
        change_body(&mut o, &peer, "hidden peer", &mut r);
        assert_eq!(
            o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r),
            Ok(FacetPublicationSampleV1::Unchanged)
        );
        assert_eq!(
            o.facet_publication_usage_v1().unwrap().issued_lifetime,
            issued
        );
        change_body(&mut o, &s.annotations[0].annotation, "updated", &mut r);
        let sample = o
            .sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
            .unwrap();
        assert_eq!(
            sample,
            if body {
                FacetPublicationSampleV1::Queued
            } else {
                FacetPublicationSampleV1::Unchanged
            }
        );
        if body {
            let FacetPublicationDeliveryV1::Projection(event) = o
                .release_facet_publication_event_v1(&opened.stream, &s, &f, &mut r)
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(event.before, opened.generation);
            assert_eq!(
                event.projection.annotations()[0]
                    .content
                    .as_ref()
                    .unwrap()
                    .0,
                "updated"
            );
        }
    }
}

#[test]
fn facet_stream_body_only_has_no_source_revision_progress() {
    let (mut o, mut r, s) = fixture(false, true, false, false);
    let mut n = crate::coordinator::publication_stream::tests::native(&s.source);
    n.source_readable = Some(false);
    let a = grants(&s);
    let f = facts(&s, &n, &a, &[]);
    let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
    let issued = o.facet_publication_usage_v1().unwrap().issued_lifetime;
    edit(&mut o, &s.source, &mut r);
    assert_eq!(
        o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r),
        Ok(FacetPublicationSampleV1::Unchanged)
    );
    assert!(opened.projection.source().is_none());
    assert_eq!(
        o.facet_publication_usage_v1().unwrap().issued_lifetime,
        issued
    );
}

#[test]
fn facet_stream_view_reattachment_fences_queued_delivery_without_sampling() {
    let (mut o, mut r, s) = fixture(false, true, false, true);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let a = grants(&s);
    let v = [s.views[0].view.clone()];
    let f = facts(&s, &n, &a, &v);
    let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
    change_body(&mut o, &s.annotations[0].annotation, "queued", &mut r);
    o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
        .unwrap();
    let peer = o.create(b"peer", &mut r).unwrap();
    let stamp = o.logical_view(&v[0], &mut r).unwrap().stamp;
    o.detach_logical_view(&v[0], stamp, &mut r).unwrap();
    let stamp = o.logical_view(&v[0], &mut r).unwrap().stamp;
    o.attach_logical_view(
        &v[0],
        stamp,
        &peer,
        &[Selection {
            anchor: BytePosition(0),
            head: BytePosition(0),
            affinity: Affinity::Before,
        }],
        0,
        &mut r,
    )
    .unwrap();
    assert!(
        o.release_facet_publication_event_v1(&opened.stream, &s, &f, &mut r)
            .is_err()
    );
    assert_eq!(o.facet_publication_usage_v1().unwrap().stopped, 1);
}

#[test]
fn facet_stream_gap_resync_and_reply_pressure_preserve_exact_native_chain() {
    let (mut o, mut r, s) = fixture(false, true, false, false);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let a = grants(&s);
    let mut f = facts(&s, &n, &a, &[]);
    let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
    change_body(&mut o, &s.annotations[0].annotation, "one", &mut r);
    o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
        .unwrap();
    change_body(&mut o, &s.annotations[0].annotation, "two", &mut r);
    o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
        .unwrap();
    f.max_reply_bytes = 0;
    assert!(matches!(
        o.release_facet_publication_event_v1(&opened.stream, &s, &f, &mut r),
        Err(Error::Limit)
    ));
    assert_eq!(o.facet_publication_usage_v1().unwrap().queued, 2);
    f.max_reply_bytes = 16384;
    let FacetPublicationDeliveryV1::Projection(first) = o
        .release_facet_publication_event_v1(&opened.stream, &s, &f, &mut r)
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(
        first.projection.annotations()[0]
            .content
            .as_ref()
            .unwrap()
            .0,
        "one"
    );
    let FacetPublicationDeliveryV1::Projection(second) = o
        .release_facet_publication_event_v1(&opened.stream, &s, &f, &mut r)
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(first.after, second.before);
    for text in ["three", "four", "five"] {
        change_body(&mut o, &s.annotations[0].annotation, text, &mut r);
        o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
            .unwrap();
    }
    assert!(matches!(
        o.release_facet_publication_event_v1(&opened.stream, &s, &f, &mut r),
        Ok(FacetPublicationDeliveryV1::Gap)
    ));
    f.baseline_allowed = Some(false);
    assert!(
        o.resync_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
            .is_err()
    );
    f.baseline_allowed = Some(true);
    let fresh = o
        .resync_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
        .unwrap();
    assert_ne!(fresh.generation, second.after);
    assert_eq!(
        fresh.projection.annotations()[0]
            .content
            .as_ref()
            .unwrap()
            .0,
        "five"
    );
    assert!(matches!(
        o.release_facet_publication_event_v1(&opened.stream, &s, &f, &mut r),
        Ok(FacetPublicationDeliveryV1::Empty)
    ));
}

#[test]
fn facet_stream_wrong_claim_and_unknown_facets_do_not_apply_real_loss() {
    let (mut o, mut r, s) = fixture(false, true, false, false);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let mut a = grants(&s);
    let f = facts(&s, &n, &a, &[]);
    let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
    let mut unknown = facts(&s, &n, &[], &[]);
    assert!(
        o.release_facet_publication_event_v1(&opened.stream, &s, &unknown, &mut r)
            .is_err()
    );
    assert_eq!(o.facet_publication_usage_v1().unwrap().stopped, 0);
    unknown.projection_allowed = Some(false);
    let mut wrong = s.clone();
    wrong.source.context.recipient = "other".into();
    assert!(
        o.release_facet_publication_event_v1(&opened.stream, &wrong, &unknown, &mut r)
            .is_err()
    );
    assert_eq!(o.facet_publication_usage_v1().unwrap().stopped, 0);
    a[0].body = false;
    let denied = facts(&s, &n, &a, &[]);
    assert!(
        o.release_facet_publication_event_v1(&opened.stream, &s, &denied, &mut r)
            .is_err()
    );
    assert_eq!(o.facet_publication_usage_v1().unwrap().stopped, 1);
    a[0].body = true;
    assert!(
        o.resync_facet_publication_stream_v1(&opened.stream, &s, &facts(&s, &n, &a, &[]), &mut r)
            .is_err()
    );
}

#[test]
fn facet_stream_named_deletion_is_absence_not_global_collection_discovery() {
    let (mut o, mut r, s) = fixture(false, true, true, false);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let a = grants(&s);
    let f = facts(&s, &n, &a, &[]);
    let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
    let id = &s.annotations[0].annotation;
    let old = o.annotation(id, &mut r).unwrap();
    let mutation = AnnotationMutation::Delete {
        annotation: id.clone(),
        expected_content: old.content_version,
        expected_target: old.target_generation,
    };
    let p = o.prepare_compound(&[], &[], &[mutation], &mut r).unwrap();
    o.commit(p, &mut r).unwrap();
    annotation(&mut o, &s.source, &mut r);
    assert_eq!(
        o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r),
        Ok(FacetPublicationSampleV1::Queued)
    );
    let FacetPublicationDeliveryV1::Projection(event) = o
        .release_facet_publication_event_v1(&opened.stream, &s, &f, &mut r)
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(event.projection.annotations().len(), 1);
    let row = &event.projection.annotations()[0];
    assert!(!row.present);
    assert!(row.content.is_none());
    assert!(row.location.is_none());
}

#[test]
fn facet_stream_view_only_tracks_semantic_selection_without_source_authority() {
    let (mut o, mut r, mut s) = fixture(false, false, false, true);
    s.annotations.clear();
    let mut n = crate::coordinator::publication_stream::tests::native(&s.source);
    n.source_readable = None;
    let v = [s.views[0].view.clone()];
    let f = facts(&s, &n, &[], &v);
    let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
    edit(&mut o, &s.source, &mut r);
    assert_eq!(
        o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r),
        Ok(FacetPublicationSampleV1::Unchanged)
    );
    let ctx = o.logical_view(&v[0], &mut r).unwrap().context().unwrap();
    o.set_logical_selections(
        &ctx,
        &[Selection {
            anchor: BytePosition(1),
            head: BytePosition(1),
            affinity: Affinity::Before,
        }],
        0,
        &mut r,
    )
    .unwrap();
    assert_eq!(
        o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r),
        Ok(FacetPublicationSampleV1::Queued)
    );
    let FacetPublicationDeliveryV1::Projection(event) = o
        .release_facet_publication_event_v1(&opened.stream, &s, &f, &mut r)
        .unwrap()
    else {
        panic!()
    };
    assert!(event.projection.source().is_none());
    assert!(event.projection.annotations().is_empty());
    assert_eq!(
        event.projection.views()[0].selections.selections()[0].head,
        BytePosition(1)
    );
}

#[test]
fn facet_stream_capture_modes_preserve_delivery_and_retained_context_accounting() {
    for mode in 0..4 {
        let (mut o, mut setup, s) = fixture(false, true, false, false);
        let n = crate::coordinator::publication_stream::tests::native(&s.source);
        let a = grants(&s);
        let f = facts(&s, &n, &a, &[]);
        let mut r = Recorder::try_with_capacity(if mode == 0 {
            0
        } else if mode == 3 {
            1
        } else {
            64
        })
        .unwrap()
        .with_native_timing(if mode > 1 {
            NativeTimingProfileV1::ProcessMonotonicV1
        } else {
            NativeTimingProfileV1::Off
        });
        let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
        assert_eq!(
            o.document_context_pins_v1(&s.source.document, &mut setup)
                .unwrap()
                .publication_streams,
            1
        );
        change_body(&mut o, &s.annotations[0].annotation, "captured", &mut setup);
        o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
            .unwrap();
        let FacetPublicationDeliveryV1::Projection(event) = o
            .release_facet_publication_event_v1(&opened.stream, &s, &f, &mut r)
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(
            event.projection.annotations()[0]
                .content
                .as_ref()
                .unwrap()
                .0,
            "captured"
        );
        let debt = o.retention_usage();
        let generation = o.generation();
        o.faulted = true;
        o.stop_facet_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage(), debt);
        assert_eq!(o.generation(), generation);
        assert!(o.is_faulted());
        o.faulted = false;
        assert_eq!(
            o.document_context_pins_v1(&s.source.document, &mut setup)
                .unwrap()
                .publication_streams,
            1
        );
        o.retire_facet_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
            .unwrap();
        assert_eq!(
            o.document_context_pins_v1(&s.source.document, &mut setup)
                .unwrap()
                .publication_streams,
            0
        );
        let issued = o.facet_publication_usage_v1().unwrap().issued_lifetime;
        o.retire_facet_publication_pool_v1(&ctl(), &mut r).unwrap();
        assert_eq!(
            o.facet_publication_usage_v1().unwrap().issued_lifetime,
            issued
        );
        assert!(
            o.configure_facet_publication_streams_v1("local", "engine", limits(), &ctl(), &mut r)
                .is_err()
        );
        if mode == 2 {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert!(events.iter().any(
                |e| e.boundary == Boundary::FacetPublicationSample && e.service_time.is_some()
            ));
        }
    }
}
#[test]
fn facet_stream_pool_bounds_are_checked() {
    let l = FacetPublicationLimitsV1 {
        max_streams: usize::MAX,
        max_annotations: 4,
        max_views: 4,
        max_selections: 4,
        max_issued: 8,
        max_context_bytes: 4096,
        max_projection_bytes: 8192,
        max_reply_bytes: 16384,
        max_queue: 2,
        max_pool_bytes: usize::MAX,
    };
    assert_eq!(l.required_pool_bytes(), Err(Error::Limit));
}

#[test]
fn facet_stream_spent_issuance_and_unrepresentable_projection_require_explicit_recovery() {
    for exhaust in [false, true] {
        let mut l = limits();
        if exhaust {
            l.max_issued = 2;
        } else {
            l.max_projection_bytes = 512;
        }
        let (mut o, mut r, s) = fixture_with_limits(false, true, false, false, l);
        let n = crate::coordinator::publication_stream::tests::native(&s.source);
        let a = grants(&s);
        let f = facts(&s, &n, &a, &[]);
        let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
        change_body(&mut o, &s.annotations[0].annotation, "first", &mut r);
        assert_eq!(
            o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r),
            Ok(FacetPublicationSampleV1::Queued)
        );
        if exhaust {
            // An equal permitted state still needs no issuance at the lifetime ceiling.
            edit(&mut o, &s.source, &mut r);
            assert_eq!(
                o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r),
                Ok(FacetPublicationSampleV1::Unchanged)
            );
        }
        change_body(
            &mut o,
            &s.annotations[0].annotation,
            &"x".repeat(1000),
            &mut r,
        );
        assert_eq!(
            o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r),
            Ok(FacetPublicationSampleV1::Gap)
        );
        assert_eq!(o.facet_publication_usage_v1().unwrap().queued, 0);
        assert!(
            o.resync_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
                .is_err()
        );
        change_body(&mut o, &s.annotations[0].annotation, "fits", &mut r);
        assert_eq!(
            o.resync_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
                .is_ok(),
            !exhaust
        );
        let issued = o.facet_publication_usage_v1().unwrap().issued_lifetime;
        o.stop_facet_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
            .unwrap();
        o.retire_facet_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
            .unwrap();
        assert_eq!(
            o.facet_publication_usage_v1().unwrap().issued_lifetime,
            issued
        );
    }
}

#[test]
fn facet_stream_foreign_frame_refuses_without_preventing_protected_cleanup() {
    let (mut o, mut r, s) = fixture(false, true, false, false);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let a = grants(&s);
    let f = facts(&s, &n, &a, &[]);
    let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
    let mut other = s.clone();
    other.annotations[0].reference = "other-ref".into();
    let other_facts = facts(&other, &n, &a, &[]);
    let foreign = o
        .open_facet_publication_stream_v1(&other, &other_facts, &mut r)
        .unwrap();
    change_body(&mut o, &s.annotations[0].annotation, "next", &mut r);
    o.sample_facet_publication_stream_v1(&opened.stream, &s, &f, &mut r)
        .unwrap();
    o.facet_publication_record_mut(&opened.stream)
        .queue
        .front_mut()
        .unwrap()
        .projection = foreign.projection;
    assert!(matches!(
        o.release_facet_publication_event_v1(&opened.stream, &s, &f, &mut r),
        Err(Error::Faulted)
    ));
    assert!(!o.is_faulted());
    o.stop_facet_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
        .unwrap();
    o.retire_facet_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
        .unwrap();
}

#[test]
fn facet_stream_rejects_duplicate_or_oversized_scope_before_native_admission() {
    let (mut o, mut r, s) = fixture(false, true, false, false);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let debt = o.retention_usage();
    for mode in 0..3 {
        let mut bad = s.clone();
        match mode {
            0 => bad.annotations.push(bad.annotations[0].clone()),
            1 => bad.annotations[0].reference = "x".repeat(4097),
            _ => {
                bad.annotations.clear();
            }
        }
        let a = grants(&bad);
        let f = facts(&bad, &n, &a, &[]);
        assert!(
            o.open_facet_publication_stream_v1(&bad, &f, &mut r)
                .is_err()
        );
        assert_eq!(o.retention_usage(), debt);
        assert_eq!(o.facet_publication_usage_v1().unwrap().issued_lifetime, 0);
    }
}

#[test]
fn facet_stream_disposed_owner_is_not_reclaimed_until_actual_observer_retirement() {
    let (mut o, mut r, s) = fixture(false, true, false, false);
    let n = crate::coordinator::publication_stream::tests::native(&s.source);
    let a = grants(&s);
    let f = facts(&s, &n, &a, &[]);
    let opened = o.open_facet_publication_stream_v1(&s, &f, &mut r).unwrap();
    let version = o.read(&s.source.document, &mut r).unwrap().version;
    let set = o
        .annotation_set_version(&s.source.document, &mut r)
        .unwrap();
    let plan = o.prepare_retained_disposal(&version, set, &mut r).unwrap();
    let state = o.commit_retained_disposal(plan, &mut r).unwrap();
    o.release_disposed_provenance(&s.source.document, state, &mut r)
        .unwrap();
    let state = o.lifecycle_state(&s.source.document, &mut r).unwrap();
    assert!(
        o.release_facet_publication_event_v1(&opened.stream, &s, &f, &mut r)
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
    o.retire_facet_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
        .unwrap();
    o.reclaim_disposed_document_v1(&s.source.document, state, &yes, &mut r)
        .unwrap();
    o.retire_facet_publication_pool_v1(&ctl(), &mut r).unwrap();
}
