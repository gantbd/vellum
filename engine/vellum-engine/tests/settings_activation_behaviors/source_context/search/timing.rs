//! Actual retained-search service spans, not queue waits or a performance SLO.
use super::*;
use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1, Observation};

#[test]
fn retained_search_service_timing_covers_success_partial_refusal_and_protected_cleanup() {
    for partial in [false, true] {
        let (mut o, _, _, s) = ready(b"aba");
        let mut r = Recorder::try_with_capacity(128)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        let mut profile = search_profile();
        if partial {
            profile.max_matches = 1;
        }
        o.configure_source_search(profile, &pool_control(), &mut r)
            .unwrap();
        let source = o
            .capture_source_read_context(&s, &facts(&s), &mut r)
            .unwrap();
        let q = request("a");
        let generation = o.generation();
        let id = o
            .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
            .unwrap();
        let reply = o
            .release_source_search(&id, &s, &facts(&s), &qfacts(&q), &mut r)
            .unwrap();
        assert_eq!(reply.matches.len(), if partial { 1 } else { 2 });
        let occurrence = o
            .source_search_occurrence(&id, 0, &s, &facts(&s), &qfacts(&q), &mut r)
            .unwrap();
        let nav = o.navigate_source_search(
            &id,
            &s,
            SourceSearchNavigationV1 {
                direction: SearchDirection::Next,
                start: 0,
                current: Some(&occurrence),
                wrap: false,
            },
            &facts(&s),
            &qfacts(&q),
            &mut r,
        );
        assert_eq!(nav.is_some(), !partial);
        let mut unavailable = qfacts(&q);
        unavailable.capacity = None;
        assert!(
            o.query_source_search(&source, &s, &q, &facts(&s), &unavailable, &mut r)
                .is_none()
        );
        assert!(
            o.release_source_search(&id, &s, &facts(&s), &unavailable, &mut r)
                .is_none()
        );
        // A transient refusal is not absorbing loss and cannot prevent cleanup.
        assert!(
            o.release_source_search(&id, &s, &facts(&s), &qfacts(&q), &mut r)
                .is_some()
        );
        let mut denied = controls(&s);
        denied.retention_released = None;
        assert!(!o.retire_source_search_result(&id, &denied, &mut r));
        assert!(o.retire_source_search_result(&id, &controls(&s), &mut r));
        assert!(o.retire_source_search(&pool_control(), &mut r).unwrap());
        assert_eq!(o.source_read_context_usage().holds, 0);
        assert_eq!(o.generation(), generation);
        let mut seen = [false; 10];
        while let Some(record) = r.pop_front() {
            let index = match (record.boundary, record.disposition) {
                (Boundary::SourceSearchConfiguration, Disposition::Completed) => 0,
                (
                    Boundary::SourceSearchEvaluation,
                    Disposition::Completed | Disposition::Partial,
                ) => 1,
                (Boundary::SourceSearchEvaluation, Disposition::Refused) => 2,
                (Boundary::SourceSearchRelease, Disposition::Completed) => 3,
                (Boundary::SourceSearchRelease, Disposition::Refused) => 4,
                (Boundary::SearchOccurrenceResolution, Disposition::Completed) => 5,
                (
                    Boundary::SourceSearchNavigation,
                    Disposition::Completed | Disposition::Refused,
                ) => 6,
                (Boundary::SourceSearchRetirement, Disposition::Refused) => 7,
                (Boundary::SourceSearchRetirement, Disposition::Completed) => 8,
                (Boundary::SourceSearchPoolRetirement, Disposition::Completed) => 9,
                _ => continue,
            };
            assert!(
                record.service_time.is_some(),
                "missing service timing at {:?}",
                record.boundary
            );
            if record.boundary == Boundary::SourceSearchEvaluation && index == 1 {
                assert_eq!(
                    record.disposition,
                    if partial {
                        Disposition::Partial
                    } else {
                        Disposition::Completed
                    }
                );
            }
            seen[index] = true;
        }
        assert_eq!(seen, [true; 10]);
    }
}

#[test]
fn retained_search_off_available_and_saturated_capture_preserve_results_and_holds() {
    for capacity in [0, 1, 128] {
        for timed in [false, true] {
            let (mut o, _, _, s) = ready(b"a\xF0\x9F\x99\x82a");
            let mut setup = Recorder::disabled();
            o.configure_source_search(search_profile(), &pool_control(), &mut setup)
                .unwrap();
            let source = o
                .capture_source_read_context(&s, &facts(&s), &mut setup)
                .unwrap();
            let generation = o.generation();
            let reserved = o.retention_usage().reserved;
            let mut r = Recorder::try_with_capacity(capacity)
                .unwrap()
                .with_native_timing(if timed {
                    NativeTimingProfileV1::ProcessMonotonicV1
                } else {
                    NativeTimingProfileV1::Off
                });
            if capacity == 1 {
                r.record(Observation {
                    boundary: Boundary::Query,
                    disposition: Disposition::Completed,
                    service_time: None,
                });
            }
            let q = request("🙂");
            let id = o
                .query_source_search(&source, &s, &q, &facts(&s), &qfacts(&q), &mut r)
                .unwrap();
            let reply = o
                .release_source_search(&id, &s, &facts(&s), &qfacts(&q), &mut r)
                .unwrap();
            assert_eq!(reply.matches, vec![ByteRange { start: 1, end: 5 }]);
            assert_eq!(o.source_read_context_usage().holds, 1);
            let nav = o
                .navigate_source_search(
                    &id,
                    &s,
                    SourceSearchNavigationV1 {
                        direction: SearchDirection::Next,
                        start: 0,
                        current: None,
                        wrap: false,
                    },
                    &facts(&s),
                    &qfacts(&q),
                    &mut r,
                )
                .unwrap();
            assert_eq!(nav.found.unwrap().range, ByteRange { start: 1, end: 5 });
            assert!(o.retire_source_search_result(&id, &controls(&s), &mut r));
            assert_eq!(o.source_read_context_usage().holds, 0);
            assert_eq!(o.source_search_usage().issued_lifetime, 1);
            assert_eq!(o.retention_usage().reserved, reserved);
            assert_eq!(o.generation(), generation);
            if capacity == 1 {
                assert!(r.dropped() > 0);
            }
            while let Some(record) = r.pop_front() {
                if matches!(
                    record.boundary,
                    Boundary::SourceSearchEvaluation
                        | Boundary::SourceSearchRelease
                        | Boundary::SourceSearchNavigation
                        | Boundary::SourceSearchRetirement
                ) {
                    assert_eq!(record.service_time.is_some(), timed);
                }
                assert!(!format!("{record:?}").contains("🙂"));
            }
        }
    }
}
