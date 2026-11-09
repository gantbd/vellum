use super::*;

#[test]
fn heterogeneous_event_source_preserves_roles_queue_and_full_reply_budget() {
    for capacity in [0, 1, 64] {
        let (mut o, mut setup, s) = owner(limits());
        let n = native(&s);
        let mut f = facts(&s, &n);
        let one = o
            .open_source_publication_stream_v1(&s, &f, &mut setup)
            .unwrap();
        let two = o
            .open_source_publication_stream_v1(&s, &f, &mut setup)
            .unwrap();
        edit(&mut o, &s, "B", &mut setup);
        o.sample_source_publication_stream_v1(&one.stream, &s, &f, &mut setup)
            .unwrap();
        let head = o
            .publication_record(&one.stream)
            .unwrap()
            .queue
            .front()
            .unwrap()
            .clone();
        let mut denied = native(&s);
        denied.authorized = Some(false);
        let denied_f = facts(&s, &denied);
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let before = o.source_publication_usage_v1().unwrap();
        assert!(
            o.release_observation_event_v1(
                ObservationEventTargetV1::Source(&two.stream),
                ObservationEventPortV1::Source {
                    stream: &one.stream,
                    scope: &s,
                    facts: &denied_f
                },
                &mut r
            )
            .is_err()
        );
        assert_eq!(o.source_publication_usage_v1().unwrap(), before);
        f.max_reply_bytes = frame_bytes(&head.projection).unwrap();
        assert!(matches!(
            o.release_observation_event_v1(
                ObservationEventTargetV1::Source(&one.stream),
                ObservationEventPortV1::Source {
                    stream: &one.stream,
                    scope: &s,
                    facts: &f
                },
                &mut r
            ),
            Err(ObservationEventErrorV1::Source(
                SourcePublicationErrorV1::Limit
            ))
        ));
        assert_eq!(o.source_publication_usage_v1().unwrap(), before);
        f.max_reply_bytes = limits().max_reply_bytes;
        let debt = o.retention_usage();
        let root = o.generation();
        let ObservationEventDeliveryV1::Source(SourcePublicationDeliveryV1::Projection(event)) = o
            .release_observation_event_v1(
                ObservationEventTargetV1::Source(&one.stream),
                ObservationEventPortV1::Source {
                    stream: &one.stream,
                    scope: &s,
                    facts: &f,
                },
                &mut r,
            )
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(event.before, one.generation);
        assert_eq!(event.after, head.after);
        assert!(Arc::ptr_eq(&event.projection, &head.projection));
        assert!(matches!(
            o.release_observation_event_v1(
                ObservationEventTargetV1::Source(&one.stream),
                ObservationEventPortV1::Source {
                    stream: &one.stream,
                    scope: &s,
                    facts: &f
                },
                &mut r
            ),
            Ok(ObservationEventDeliveryV1::Source(
                SourcePublicationDeliveryV1::Empty
            ))
        ));
        assert_eq!(o.retention_usage(), debt);
        assert_eq!(o.generation(), root);
        if capacity == 64 {
            assert!(
                std::iter::from_fn(|| r.pop_front())
                    .any(|e| e.boundary == Boundary::ObservationEventDelivery
                        && e.service_time.is_some())
            );
        }
    }
}

#[test]
fn heterogeneous_event_source_parent_ceiling_cannot_be_overridden_by_caller() {
    let mut l = limits();
    l.max_reply_bytes =
        size_of::<SourcePublicationDeliveryV1>() + size_of::<SourcePublicationProjectionV1>() + 1;
    let (mut o, mut r, s) = owner(l);
    let n = native(&s);
    let f = facts(&s, &n);
    let opened = o.open_source_publication_stream_v1(&s, &f, &mut r).unwrap();
    edit(&mut o, &s, "B", &mut r);
    o.sample_source_publication_stream_v1(&opened.stream, &s, &f, &mut r)
        .unwrap();
    assert!(matches!(
        o.release_observation_event_v1(
            ObservationEventTargetV1::Source(&opened.stream),
            ObservationEventPortV1::Source {
                stream: &opened.stream,
                scope: &s,
                facts: &f
            },
            &mut r
        ),
        Err(ObservationEventErrorV1::Source(
            SourcePublicationErrorV1::Limit
        ))
    ));
    assert_eq!(o.source_publication_usage_v1().unwrap().queued, 1);
    assert!(matches!(
        o.release_source_publication_event_v1(&opened.stream, &s, &f, &mut r),
        Ok(SourcePublicationDeliveryV1::Projection(_))
    ));
}
use crate::instrumentation::NativeTimingProfileV1;
pub(in crate::coordinator) fn limits() -> SourcePublicationLimitsV1 {
    SourcePublicationLimitsV1 {
        max_streams: 2,
        max_issued: 16,
        max_context_bytes: 2048,
        max_projection_bytes: 8192,
        max_reply_bytes: 16384,
        max_queue: 2,
        max_pool_bytes: 1048576,
    }
}
fn control() -> SourceReadPoolControlV1 {
    SourceReadPoolControlV1 {
        owner_control_allowed: Some(true),
        resources_ready: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
fn cleanup(s: &SourceReadScopeV1) -> SourceReadContextControlV1<'_> {
    SourceReadContextControlV1 {
        scope: s,
        owner_control_allowed: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    }
}
pub(in crate::coordinator) fn native(s: &SourceReadScopeV1) -> TrustedSourceReadFactsV1<'_> {
    TrustedSourceReadFactsV1 {
        scope: s,
        authorized: Some(true),
        scope_allowed: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        retained: Some(true),
        source_readable: Some(true),
        resources_ready: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
    }
}
pub(in crate::coordinator) fn facts<'a>(
    s: &'a SourceReadScopeV1,
    f: &'a TrustedSourceReadFactsV1<'a>,
) -> TrustedSourcePublicationFactsV1<'a> {
    TrustedSourcePublicationFactsV1 {
        original_scope: s,
        current: f,
        projection_allowed: Some(true),
        baseline_allowed: Some(true),
        max_reply_bytes: 16384,
    }
}
pub(in crate::coordinator) fn bare() -> SourceCoordinator {
    SourceCoordinator::configured(CoordinatorConfig {
        limits: CoordinatorLimits {
            max_document_incarnations: 8,
            max_participants: 8,
            max_staged_bytes: 8192,
            source: crate::source::Limits {
                max_source_bytes: 4096,
                max_edits: 8,
                max_edit_payload_bytes: 8192,
            },
        },
        invocation: None,
        resources: None,
        history: None,
        search: None,
        annotations: Some(AnnotationLimits {
            max_issued: 8,
            max_records: 8,
            max_body_bytes: 1024,
            max_retained_body_bytes: 8192,
            max_mapping_steps: 8192,
            max_live_snapshots: 4,
            max_snapshot_bytes: 16384,
        }),
    })
}
pub(in crate::coordinator) fn owner(
    limits: SourcePublicationLimitsV1,
) -> (SourceCoordinator, Recorder, SourceReadScopeV1) {
    owner_with_budget(limits, 1048576)
}
#[test]
fn publication_admission_fence_blocks_new_source_pool_without_spending_funding() {
    let mut o = bare();
    let mut r = Recorder::try_with_capacity(16).unwrap();
    o.configure_retention_budget(8388608, &mut r).unwrap();
    crate::coordinator::shutdown::tests::install(&mut o, &mut r);
    let debt = o.retention_usage();
    assert_eq!(
        o.configure_source_publication_streams_v1("local", "engine", limits(), &control(), &mut r),
        Err(Error::Unavailable)
    );
    assert_eq!(o.retention_usage(), debt);
    assert!(matches!(o.publication_streams, State::Unconfigured));
}
pub(in crate::coordinator) fn owner_with_budget(
    limits: SourcePublicationLimitsV1,
    maximum: usize,
) -> (SourceCoordinator, Recorder, SourceReadScopeV1) {
    let mut o = bare();
    let mut r = Recorder::disabled();
    let d = o.create(b"A", &mut r).unwrap();
    let s = SourceReadScopeV1 {
        document: d.document,
        policy: "private policy".into(),
        context: SettingsExecutionContextV1 {
            authority: "local".into(),
            engine: "engine".into(),
            recipient: "recipient".into(),
            purpose: "purpose".into(),
            destination: "destination".into(),
            profile: "native-source-publication".into(),
            continuity: "continuity".into(),
        },
    };
    o.configure_retention_budget(maximum, &mut r).unwrap();
    o.configure_source_publication_streams_v1("local", "engine", limits, &control(), &mut r)
        .unwrap();
    (o, r, s)
}
fn edit(o: &mut SourceCoordinator, s: &SourceReadScopeV1, text: &str, r: &mut Recorder) {
    let old = o.read(&s.document, r).unwrap();
    let write = DocumentEdits {
        expected: old.version,
        edits: vec![Replacement {
            start: 0,
            end: old.source.text().len(),
            expected: old.source.text().into(),
            inserted: text.into(),
        }],
    };
    let p = o.prepare(&[write], &[], r).unwrap();
    o.commit(p, r).unwrap();
}
#[test]
fn source_stream_samples_actual_publications_without_hidden_progress_and_retains_exact_frames() {
    let (mut o, mut r, s) = owner(limits());
    let n = native(&s);
    let f = facts(&s, &n);
    let opened = o.open_source_publication_stream_v1(&s, &f, &mut r).unwrap();
    let version = o.read(&s.document, &mut r).unwrap().version;
    let hidden = o
        .prepare_compound(
            &[],
            &[],
            &[AnnotationMutation::Create {
                body: "private comment".into(),
                target: SourceTarget {
                    source: version,
                    basis: TargetBasis::Before,
                    target: crate::attachment::AttachmentTarget::ContentRange { start: 0, end: 1 },
                },
            }],
            &mut r,
        )
        .unwrap();
    o.commit(hidden, &mut r).unwrap();
    o.create(b"hidden peer", &mut r).unwrap();
    assert_eq!(
        o.sample_source_publication_stream_v1(&opened.stream, &s, &f, &mut r),
        Ok(SourcePublicationSampleV1::Unchanged)
    );
    assert_eq!(o.source_publication_usage_v1().unwrap().issued_lifetime, 1);
    edit(&mut o, &s, "B", &mut r);
    let publication = o.generation();
    let held = o.retention_usage();
    assert_eq!(
        o.sample_source_publication_stream_v1(&opened.stream, &s, &f, &mut r),
        Ok(SourcePublicationSampleV1::Queued)
    );
    edit(&mut o, &s, "C", &mut r);
    let SourcePublicationDeliveryV1::Projection(event) = o
        .release_source_publication_event_v1(&opened.stream, &s, &f, &mut r)
        .unwrap()
    else {
        panic!("event")
    };
    assert_eq!(event.before, opened.generation);
    assert_ne!(event.after, event.before);
    assert_eq!(event.projection.text(), "B");
    assert_eq!(opened.projection.text(), "A");
    assert!(!format!("{event:?}").contains("private comment"));
    assert_eq!(o.retention_usage(), held);
    assert!(o.generation() > publication); // Only the explicit second edit published.
    assert_eq!(o.source_publication_usage_v1().unwrap().issued_lifetime, 2);
}
#[test]
fn source_stream_overflow_is_explicit_gap_and_resync_drops_every_old_frame() {
    let (mut o, mut r, s) = owner(limits());
    let n = native(&s);
    let f = facts(&s, &n);
    let opened = o.open_source_publication_stream_v1(&s, &f, &mut r).unwrap();
    for text in ["B", "C"] {
        edit(&mut o, &s, text, &mut r);
        assert_eq!(
            o.sample_source_publication_stream_v1(&opened.stream, &s, &f, &mut r),
            Ok(SourcePublicationSampleV1::Queued)
        );
    }
    edit(&mut o, &s, "D", &mut r);
    let mut gap_capture = Recorder::try_with_capacity(16)
        .unwrap()
        .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
    assert_eq!(
        o.sample_source_publication_stream_v1(&opened.stream, &s, &f, &mut gap_capture),
        Ok(SourcePublicationSampleV1::Gap)
    );
    let marker = gap_capture.pop_front().unwrap();
    assert_eq!(marker.boundary, Boundary::SourcePublicationGap);
    assert!(marker.service_time.is_none());
    let service = gap_capture.pop_front().unwrap();
    assert_eq!(service.boundary, Boundary::SourcePublicationSample);
    assert!(service.service_time.is_some());
    o.sample_source_publication_stream_v1(&opened.stream, &s, &f, &mut gap_capture)
        .unwrap();
    assert_eq!(
        gap_capture.pop_front().unwrap().boundary,
        Boundary::SourcePublicationSample
    );
    assert!(gap_capture.pop_front().is_none());
    assert_eq!(o.source_publication_usage_v1().unwrap().queued, 0);
    assert!(matches!(
        o.release_source_publication_event_v1(&opened.stream, &s, &f, &mut r),
        Ok(SourcePublicationDeliveryV1::Gap)
    ));
    let mut denied = facts(&s, &n);
    denied.baseline_allowed = None;
    assert!(
        o.resync_source_publication_stream_v1(&opened.stream, &s, &denied, &mut r)
            .is_err()
    );
    let fresh = o
        .resync_source_publication_stream_v1(&opened.stream, &s, &f, &mut r)
        .unwrap();
    assert_eq!(fresh.projection.text(), "D");
    assert_ne!(fresh.generation, opened.generation);
    assert!(matches!(
        o.release_source_publication_event_v1(&opened.stream, &s, &f, &mut r),
        Ok(SourcePublicationDeliveryV1::Empty)
    ));
}
#[test]
fn source_stream_wrong_claim_does_not_apply_loss_and_current_loss_is_absorbing() {
    let (mut o, mut r, s) = owner(limits());
    let n = native(&s);
    let f = facts(&s, &n);
    let opened = o.open_source_publication_stream_v1(&s, &f, &mut r).unwrap();
    edit(&mut o, &s, "B", &mut r);
    o.sample_source_publication_stream_v1(&opened.stream, &s, &f, &mut r)
        .unwrap();
    let mut wrong = s.clone();
    wrong.context.recipient = "foreign".into();
    let mut lost = native(&s);
    lost.authorized = Some(false);
    let loss = facts(&s, &lost);
    assert!(
        o.release_source_publication_event_v1(&opened.stream, &wrong, &loss, &mut r)
            .is_err()
    );
    assert_eq!(o.source_publication_usage_v1().unwrap().queued, 1);
    let mut pressure = facts(&s, &n);
    pressure.max_reply_bytes = 0;
    assert_eq!(
        o.release_source_publication_event_v1(&opened.stream, &s, &pressure, &mut r)
            .err(),
        Some(Error::Limit)
    );
    assert_eq!(o.source_publication_usage_v1().unwrap().queued, 1);
    assert!(
        o.release_source_publication_event_v1(&opened.stream, &s, &loss, &mut r)
            .is_err()
    );
    assert_eq!(o.source_publication_usage_v1().unwrap().queued, 0);
    assert!(
        o.resync_source_publication_stream_v1(&opened.stream, &s, &f, &mut r)
            .is_err()
    );
    let replacement = o.open_source_publication_stream_v1(&s, &f, &mut r).unwrap();
    assert_ne!(opened.stream, replacement.stream);
    assert!(
        o.sample_source_publication_stream_v1(&opened.stream, &s, &f, &mut r)
            .is_err()
    );
}
#[test]
fn source_stream_lifetime_exhaustion_and_protected_cleanup_do_not_reset_funding_or_issuance() {
    let (mut o, mut r, s) = owner(SourcePublicationLimitsV1 {
        max_issued: 2,
        ..limits()
    });
    let n = native(&s);
    let f = facts(&s, &n);
    let opened = o.open_source_publication_stream_v1(&s, &f, &mut r).unwrap();
    edit(&mut o, &s, "B", &mut r);
    o.sample_source_publication_stream_v1(&opened.stream, &s, &f, &mut r)
        .unwrap();
    edit(&mut o, &s, "C", &mut r);
    assert_eq!(
        o.sample_source_publication_stream_v1(&opened.stream, &s, &f, &mut r),
        Ok(SourcePublicationSampleV1::Gap)
    );
    assert_eq!(
        o.resync_source_publication_stream_v1(&opened.stream, &s, &f, &mut r)
            .err(),
        Some(Error::Limit)
    );
    let held = o.retention_usage();
    o.faulted = true;
    o.stop_observation_facade_v2(
        ObservationStopPortV2::SourcePublication {
            stream: &opened.stream,
            control: &cleanup(&s),
        },
        &mut r,
    )
    .unwrap();
    o.retire_source_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage(), held);
    o.retire_source_publication_pool_v1(&control(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
    o.faulted = false;
    assert!(
        o.configure_source_publication_streams_v1("local", "engine", limits(), &control(), &mut r)
            .is_err()
    );
    assert_eq!(o.source_publication_usage_v1().unwrap().issued_lifetime, 2);
}
#[test]
fn source_stream_funding_bounds_are_real_and_counters_cannot_overflow() {
    let p = limits();
    let charge = p.required_pool_bytes().unwrap() + "local".len() + "engine".len();
    for exact in [false, true] {
        let mut o = bare();
        let mut r = Recorder::disabled();
        o.configure_retention_budget(if exact { charge } else { charge - 1 }, &mut r)
            .unwrap();
        assert_eq!(
            o.configure_source_publication_streams_v1("local", "engine", p, &control(), &mut r)
                .is_ok(),
            exact
        );
        assert_eq!(o.retention_usage().reserved, if exact { charge } else { 0 });
    }
    assert_eq!(
        SourcePublicationLimitsV1 {
            max_issued: usize::MAX,
            ..p
        }
        .required_pool_bytes(),
        Err(Error::Limit)
    );
    assert_eq!(
        SourcePublicationLimitsV1 {
            max_queue: usize::MAX,
            ..p
        }
        .required_pool_bytes(),
        Err(Error::Limit)
    );
}
#[test]
fn source_stream_disposal_and_reclamation_keep_the_context_until_actual_retirement() {
    let (mut o, mut r, s) = owner(limits());
    let n = native(&s);
    let f = facts(&s, &n);
    let opened = o.open_source_publication_stream_v1(&s, &f, &mut r).unwrap();
    let version = o.read(&s.document, &mut r).unwrap().version;
    let plan = o.prepare_retained_disposal(&version, 0, &mut r).unwrap();
    let state = o.commit_retained_disposal(plan, &mut r).unwrap();
    o.release_disposed_provenance(&s.document, state, &mut r)
        .unwrap();
    let state = o.lifecycle_state(&s.document, &mut r).unwrap();
    // A stale positive host liveness flag cannot override native disposal.
    assert!(
        o.release_source_publication_event_v1(&opened.stream, &s, &f, &mut r)
            .is_err()
    );
    let retire = TrustedDocumentReclamationV1 {
        authorized: Some(true),
        authored_disposition_ready: Some(true),
        session_references_released: Some(true),
        retention_released: Some(true),
        delivery_drained: Some(true),
    };
    assert_eq!(
        o.reclaim_disposed_document_v1(&s.document, state, &retire, &mut r),
        Err(CoordinatorError::Resource(
            crate::resource::ResourceError::Retained
        ))
    );
    assert_eq!(
        o.document_context_pins_v1(&s.document, &mut r)
            .unwrap()
            .publication_streams,
        1
    );
    o.retire_source_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
        .unwrap();
    o.reclaim_disposed_document_v1(&s.document, state, &retire, &mut r)
        .unwrap();
    o.retire_source_publication_pool_v1(&control(), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage().reserved, 0);
}
#[test]
fn source_stream_capture_off_on_and_saturated_preserve_projection_sequence_and_peer_root() {
    for timing in [
        NativeTimingProfileV1::Off,
        NativeTimingProfileV1::ProcessMonotonicV1,
    ] {
        for capacity in [0, 1, 32] {
            let (mut o, mut r, s) = owner(limits());
            let n = native(&s);
            let f = facts(&s, &n);
            let mut capture = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(timing);
            let opened = o
                .open_source_publication_stream_v1(&s, &f, &mut capture)
                .unwrap();
            edit(&mut o, &s, "B", &mut r);
            let root = o.generation();
            let held = o.retention_usage();
            o.sample_source_publication_stream_v1(&opened.stream, &s, &f, &mut capture)
                .unwrap();
            let SourcePublicationDeliveryV1::Projection(e) = o
                .release_source_publication_event_v1(&opened.stream, &s, &f, &mut capture)
                .unwrap()
            else {
                panic!("projection")
            };
            assert_eq!(e.projection.text(), "B");
            assert_eq!(e.before, opened.generation);
            assert_eq!(o.generation(), root);
            assert_eq!(o.retention_usage(), held);
            while let Some(event) = capture.pop_front() {
                assert_eq!(
                    event.service_time.is_some(),
                    timing == NativeTimingProfileV1::ProcessMonotonicV1
                );
            }
        }
    }
}
#[test]
fn source_stream_never_hands_off_a_foreign_projection_and_protected_stop_survives_local_fault() {
    let (mut o, mut r, s) = owner(limits());
    let n = native(&s);
    let f = facts(&s, &n);
    let opened = o.open_source_publication_stream_v1(&s, &f, &mut r).unwrap();
    edit(&mut o, &s, "B", &mut r);
    o.sample_source_publication_stream_v1(&opened.stream, &s, &f, &mut r)
        .unwrap();
    let peer = o.create(b"private peer", &mut r).unwrap();
    o.publication_record_mut(&opened.stream)
        .queue
        .front_mut()
        .unwrap()
        .projection = Arc::new(SourcePublicationProjectionV1 {
        version: peer.clone(),
        text: "private peer".into(),
    });
    assert_eq!(
        o.release_source_publication_event_v1(&opened.stream, &s, &f, &mut r)
            .err(),
        Some(Error::Faulted)
    );
    assert_eq!(
        o.read(&peer.document, &mut r).unwrap().source.text(),
        "private peer"
    );
    assert!(!o.is_faulted());
    o.stop_observation_facade_v2(
        ObservationStopPortV2::SourcePublication {
            stream: &opened.stream,
            control: &cleanup(&s),
        },
        &mut r,
    )
    .unwrap();
    o.retire_source_publication_stream_v1(&opened.stream, &cleanup(&s), &mut r)
        .unwrap();
}
