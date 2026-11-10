use super::*;
use vellum_engine::instrumentation::{Boundary, Disposition, NativeTimingProfileV1, Observation};

fn recorder(mode: usize) -> Recorder {
    let mut r = Recorder::try_with_capacity([0, 64, 64, 1][mode])
        .unwrap()
        .with_native_timing(if mode >= 2 {
            NativeTimingProfileV1::ProcessMonotonicV1
        } else {
            NativeTimingProfileV1::Off
        });
    if mode == 3 {
        r.record(Observation {
            boundary: Boundary::Query,
            disposition: Disposition::Completed,
            service_time: None,
        });
    }
    r
}
fn check(r: &mut Recorder, mode: usize, boundary: Boundary, disposition: Disposition) {
    if mode == 1 || mode == 2 {
        let records: Vec<_> = std::iter::from_fn(|| r.pop_front())
            .filter(|e| e.boundary == boundary)
            .collect();
        assert_eq!(records.len(), 1, "{boundary:?}");
        assert_eq!(records[0].disposition, disposition);
        assert_eq!(records[0].service_time.is_some(), mode == 2, "{boundary:?}");
    }
}
#[test]
fn given_save_service_then_optional_timing_preserves_results_retention_and_refusals() {
    let mut reference = None;
    for mode in 0..4 {
        let mut o = owner();
        let mut setup = Recorder::disabled();
        let port = o
            .register_provider(
                ProviderProfile {
                    synchronization_supported: false,
                },
                None,
                &mut setup,
            )
            .unwrap();
        let source = o
            .load_resource(port.read_result(b"key", b"v0", b"A").unwrap(), &mut setup)
            .unwrap();
        let save = o
            .queue_pinned_save(&source, requirements(), &mut setup)
            .unwrap();
        let mut r = recorder(mode);
        let dispatch = o
            .dispatch_save(
                &save,
                port.read_result(b"key", b"v0", b"A").unwrap(),
                &mut r,
            )
            .unwrap();
        check(&mut r, mode, Boundary::SaveDispatch, Disposition::Completed);
        assert_eq!(
            o.dispatch_save(
                &save,
                port.read_result(b"key", b"v0", b"A").unwrap(),
                &mut r
            )
            .unwrap_err(),
            ResourceError::Unavailable
        );
        check(&mut r, mode, Boundary::SaveDispatch, Disposition::Refused);
        for evidence in [
            port.result(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(b"v1"),
            )
            .unwrap(),
            port.no_future_effects(&dispatch).unwrap(),
            port.quiescence(&dispatch).unwrap(),
        ] {
            assert_eq!(
                o.record_save_evidence(evidence, &mut r),
                Ok(EvidenceDisposition::Accepted)
            );
            check(&mut r, mode, Boundary::SaveEvidence, Disposition::Completed);
        }
        o.publish_save_baseline(&save, &mut r).unwrap();
        check(&mut r, mode, Boundary::SaveBaseline, Disposition::Completed);
        o.publish_save_baseline(&save, &mut r).unwrap();
        check(&mut r, mode, Boundary::SaveBaseline, Disposition::Completed);
        assert_eq!(
            o.saved_state(&source.document, &mut r).unwrap().content,
            ContentComparison::Unchanged
        );
        check(&mut r, mode, Boundary::Query, Disposition::Completed);
        let result = o.save_outcome(&save, &mut r).unwrap();
        check(&mut r, mode, Boundary::Outcome, Disposition::Completed);
        assert_eq!(result.application, Application::Written);
        assert!(result.quiescent && result.no_future_effects);
        drop(dispatch);
        assert_eq!(o.retire_save(&save, &mut r), Err(ResourceError::Retained));
        check(
            &mut r,
            mode,
            Boundary::SaveProducerRetirement,
            Disposition::Refused,
        );
        let stopped = o
            .queue_pinned_save(&source, requirements(), &mut setup)
            .unwrap();
        o.stop_save(&stopped, &mut r).unwrap();
        check(&mut r, mode, Boundary::Control, Disposition::Completed);
        o.retire_save(&stopped, &mut r).unwrap();
        check(
            &mut r,
            mode,
            Boundary::SaveProducerRetirement,
            Disposition::Completed,
        );
        let summary = (o.resource_usage(), o.generation(), o.retention_usage());
        if let Some(expected) = &reference {
            assert_eq!(&summary, expected);
        } else {
            reference = Some(summary);
        }
        assert_eq!(r.dropped() > 0, mode == 3);
        if mode == 0 {
            assert_eq!(r.queued(), 0);
        }
    }
}

#[test]
fn given_contradictory_evidence_then_unknown_is_timed_without_claiming_settlement() {
    for mode in 0..4 {
        let mut o = owner();
        let mut setup = Recorder::disabled();
        let port = o
            .register_provider(
                ProviderProfile {
                    synchronization_supported: false,
                },
                None,
                &mut setup,
            )
            .unwrap();
        let source = o
            .load_resource(port.read_result(b"key", b"v0", b"A").unwrap(), &mut setup)
            .unwrap();
        let save = o
            .queue_pinned_save(&source, requirements(), &mut setup)
            .unwrap();
        let dispatch = o
            .dispatch_save(
                &save,
                port.read_result(b"key", b"v0", b"A").unwrap(),
                &mut setup,
            )
            .unwrap();
        o.record_save_evidence(
            port.result(
                &dispatch,
                Application::Written,
                1,
                Synchronization::NotRequested,
                Some(b"v1"),
            )
            .unwrap(),
            &mut setup,
        )
        .unwrap();
        let mut r = recorder(mode);
        assert_eq!(
            o.record_save_evidence(
                port.result(
                    &dispatch,
                    Application::NotWritten,
                    0,
                    Synchronization::NotRequested,
                    None
                )
                .unwrap(),
                &mut r
            ),
            Ok(EvidenceDisposition::Contradiction)
        );
        check(&mut r, mode, Boundary::SaveEvidence, Disposition::Unknown);
        let outcome = o.save_outcome(&save, &mut setup).unwrap();
        assert!(outcome.contested);
        assert!(!outcome.no_future_effects && !outcome.quiescent);
        assert_eq!(
            o.retire_save(&save, &mut setup),
            Err(ResourceError::Retained)
        );
        assert_eq!(r.dropped() > 0, mode == 3);
    }
}
