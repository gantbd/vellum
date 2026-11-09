use super::*;
use vellum_engine::instrumentation::{Boundary, Disposition, Observation};
fn recorder(capacity: Option<usize>) -> Recorder {
    capacity.map_or_else(Recorder::disabled, |n| {
        Recorder::try_with_capacity(n).unwrap()
    })
}
fn drain(r: &mut Recorder) -> Vec<Observation> {
    std::iter::from_fn(|| r.pop_front()).collect()
}
fn conversions(events: &[Observation], boundary: Boundary) -> Vec<Disposition> {
    events
        .iter()
        .filter(|e| e.boundary == boundary)
        .map(|e| e.disposition)
        .collect()
}
#[test]
fn actual_decode_is_optional_bounded_and_preserves_signature_unicode_and_newline_bytes() {
    for capacity in [None, Some(128), Some(1)] {
        let mut o = owner();
        let mut r = recorder(capacity);
        let bytes = "\u{feff}private-λ\r\n".as_bytes();
        let d = o.create(bytes, &mut r).unwrap();
        let source = o.read(&d.document, &mut r).unwrap();
        assert_eq!(source.source.text(), "private-λ\r\n");
        assert_eq!(source.source.encode().unwrap(), bytes);
        let events = drain(&mut r);
        if capacity == Some(128) {
            assert_eq!(
                conversions(&events, Boundary::SourceDecode),
                vec![Disposition::Completed]
            );
            let decoded = events
                .iter()
                .position(|e| e.boundary == Boundary::SourceDecode)
                .unwrap();
            let admitted = events
                .iter()
                .position(|e| e.boundary == Boundary::Lifecycle)
                .unwrap();
            assert!(decoded < admitted);
        }
        // The standalone immutable value codec has no attached owner recorder.
        assert!(conversions(&events, Boundary::SourceEncode).is_empty());
        assert!(events.iter().all(|e| e.service_time.is_none()));
        if capacity.is_none() {
            assert!(events.is_empty());
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
#[test]
fn decoder_refusals_do_not_publish_and_admission_preflight_does_not_fabricate_decode_work() {
    for oversized in [false, true] {
        let mut o = owner();
        let mut r = recorder(Some(128));
        let bytes = if oversized {
            vec![b'a'; 1025]
        } else {
            vec![0xff]
        };
        let expected = if oversized {
            vellum_engine::source::SourceError::SourceLimit
        } else {
            vellum_engine::source::SourceError::InvalidUtf8
        };
        let generation = o.generation();
        assert_eq!(
            o.create(&bytes, &mut r),
            Err(CoordinatorError::Source(expected))
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(o.lifecycle_usage().issued_lifetime, 0);
        assert_eq!(
            conversions(&drain(&mut r), Boundary::SourceDecode),
            vec![Disposition::Refused]
        );
    }
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 0,
        max_participants: 1,
        max_staged_bytes: 16,
        source: Limits {
            max_source_bytes: 16,
            max_edits: 1,
            max_edit_payload_bytes: 16,
        },
    });
    let mut r = recorder(Some(128));
    assert_eq!(o.create(b"A", &mut r), Err(CoordinatorError::Limit));
    assert!(conversions(&drain(&mut r), Boundary::SourceDecode).is_empty());
}
#[test]
fn resource_open_decodes_once_and_reusing_an_existing_binding_does_not_decode_or_reload() {
    let mut o = owner();
    let mut r = recorder(Some(128));
    let port = o
        .register_provider(
            ProviderProfile {
                synchronization_supported: false,
            },
            None,
            &mut r,
        )
        .unwrap();
    drain(&mut r);
    let d = o
        .load_resource(port.read_result(b"key", b"v0", b"A\r\n").unwrap(), &mut r)
        .unwrap();
    assert_eq!(
        conversions(&drain(&mut r), Boundary::SourceDecode),
        vec![Disposition::Completed]
    );
    let generation = o.generation();
    let same = o
        .load_resource(port.read_result(b"key", b"v1", &[0xff]).unwrap(), &mut r)
        .unwrap();
    assert_eq!(same, d);
    assert_eq!(o.generation(), generation);
    assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "A\r\n");
    assert!(conversions(&drain(&mut r), Boundary::SourceDecode).is_empty());
}
#[test]
fn encoding_signals_follow_actual_capture_and_comparison_not_metadata_dispatch_or_policy_refusal() {
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = ready();
        let mut r = recorder(capacity);
        let c = context();
        let port = o
            .register_provider(
                ProviderProfile {
                    synchronization_supported: false,
                },
                None,
                &mut r,
            )
            .unwrap();
        let d = o
            .load_resource(port.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
            .unwrap();
        drain(&mut r);
        apply(&mut o, &d.document, true, &c, &mut r);
        assert!(conversions(&drain(&mut r), Boundary::SourceEncode).is_empty());
        let task = o.queue_pinned_save(&d, requirements(), &mut r).unwrap();
        let events = drain(&mut r);
        if capacity == Some(128) {
            assert_eq!(
                conversions(&events, Boundary::SourceEncode),
                vec![Disposition::Completed]
            );
        }
        let dispatch = o
            .dispatch_save(
                &task,
                port.read_result(b"key", b"v0", b"A").unwrap(),
                &mut r,
            )
            .unwrap();
        assert_eq!(dispatch.bytes(), b"\xef\xbb\xbfA");
        assert!(conversions(&drain(&mut r), Boundary::SourceEncode).is_empty());
        assert_eq!(
            o.saved_state(&d.document, &mut r).unwrap().content,
            ContentComparison::Modified
        );
        let events = drain(&mut r);
        if capacity == Some(128) {
            assert_eq!(
                conversions(&events, Boundary::SourceEncode),
                vec![Disposition::Completed]
            );
        }
        o.set_read_only(&d.document, true, &mut r).unwrap();
        drain(&mut r);
        assert_eq!(
            o.queue_pinned_save(&d, requirements(), &mut r),
            Err(ResourceError::Permission)
        );
        assert!(conversions(&drain(&mut r), Boundary::SourceEncode).is_empty());
        assert_eq!(o.read(&d.document, &mut r).unwrap().version, d);
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
#[test]
fn ambiguous_output_records_codec_refusal_before_artifact_or_save_issuance() {
    for capacity in [None, Some(128), Some(1)] {
        let (mut o, _) = ready();
        let mut r = recorder(capacity);
        let c = context();
        let port = o
            .register_provider(
                ProviderProfile {
                    synchronization_supported: false,
                },
                None,
                &mut r,
            )
            .unwrap();
        let d = o
            .load_resource(
                port.read_result(b"key", b"v0", "\u{feff}\u{feff}A".as_bytes())
                    .unwrap(),
                &mut r,
            )
            .unwrap();
        apply(&mut o, &d.document, false, &c, &mut r);
        drain(&mut r);
        let before = o.resource_usage();
        let generation = o.generation();
        assert_eq!(
            o.queue_pinned_save(&d, requirements(), &mut r),
            Err(ResourceError::Source(
                vellum_engine::source::SourceError::AmbiguousBom
            ))
        );
        assert_eq!(o.resource_usage(), before);
        assert_eq!(o.generation(), generation);
        let events = drain(&mut r);
        if capacity == Some(128) {
            assert_eq!(
                conversions(&events, Boundary::SourceEncode),
                vec![Disposition::Refused]
            );
        }
        if capacity.is_none() {
            assert!(events.is_empty());
        }
        if capacity == Some(1) {
            assert!(r.dropped() > 0);
        }
    }
}
