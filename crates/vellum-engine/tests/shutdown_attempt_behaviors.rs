use vellum_engine::coordinator::*;
use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
use vellum_engine::source::Limits;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)]
mod resource_support;

#[test]
fn returned_unknown_write_report_is_historical_and_late_evidence_keeps_its_original_owner() {
    use vellum_engine::resource::*;
    let mut f = resource_support::Fixture::new();
    let pair = f.pair().unwrap();
    let save = f
        .owner
        .queue_pinned_save_with_pair(
            &f.doc,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let pair = f.pair().unwrap();
    let dispatch = f
        .owner
        .dispatch_save_with_pair(
            &save,
            f.port.read_result(b"source", b"s0", b"text").unwrap(),
            pair,
            &mut f.recorder,
        )
        .unwrap();
    let c = context();
    let control = control(&c);
    let auth = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let id = fence(&mut f.owner, &control, &mut f.recorder);
    assert_eq!(
        f.owner
            .finish_shutdown_attempt_v1(&id, limits(), &auth, &mut f.recorder),
        Ok(ShutdownAttemptStatusV1::UnresolvedRetained)
    );
    let (_, report) = f
        .owner
        .read_shutdown_attempt_report_v1(&id, &auth, &mut f.recorder)
        .unwrap();
    assert_eq!(report.writes.len(), 1);
    assert_eq!(report.writes[0].application, Application::Unknown);
    assert!(!report.writes[0].quiescent);
    for evidence in [
        f.port
            .result(
                &dispatch,
                Application::NotWritten,
                0,
                Synchronization::Unknown,
                None,
            )
            .unwrap(),
        f.port.no_future_effects(&dispatch).unwrap(),
        f.port.quiescence(&dispatch).unwrap(),
    ] {
        f.owner
            .record_save_evidence(evidence, &mut f.recorder)
            .unwrap();
    }
    let (_, historical) = f
        .owner
        .read_shutdown_attempt_report_v1(&id, &auth, &mut f.recorder)
        .unwrap();
    assert_eq!(historical.writes[0].application, Application::Unknown);
    assert!(!historical.writes[0].quiescent);
    let current = f
        .owner
        .capture_shutdown_progress_v1(&id, limits(), &auth, &mut f.recorder)
        .unwrap();
    let current = f
        .owner
        .read_shutdown_progress_v1(&current, &auth, &mut f.recorder)
        .unwrap();
    assert!(
        current.coverage.all_owner_obligations,
        "returned report must participate in native funding census"
    );
    assert_eq!(current.writes[0].application, Application::NotWritten);
    assert!(current.writes[0].quiescent);
    f.owner
        .retire_shutdown_attempt_report_v1(&id, &control, Some(true), Some(true), &mut f.recorder)
        .unwrap();
    assert!(
        f.owner.save_outcome(&save, &mut f.recorder).is_ok(),
        "report retirement is not task retirement"
    );
    assert!(
        f.owner
            .check_shutdown_root_release_v1(&id, limits(), &auth, &mut f.recorder)
            .is_err()
    );
    f.owner.stop_save(&save, &mut f.recorder).unwrap();
    drop(dispatch);
    f.owner.retire_save(&save, &mut f.recorder).unwrap();
}

#[test]
fn incomplete_capture_refuses_without_spending_report_or_releasing_the_fence() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let c = context();
    let control = control(&c);
    let auth = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let id = fence(&mut o, &control, &mut r);
    let before = o.retention_usage();
    let mut insufficient = limits();
    insufficient.max_capture_bytes = 0;
    assert!(
        o.finish_shutdown_attempt_v1(&id, insufficient, &auth, &mut r)
            .is_err()
    );
    assert_eq!(o.retention_usage(), before);
    assert_eq!(
        o.finish_shutdown_attempt_v1(&id, limits(), &auth, &mut r),
        Ok(ShutdownAttemptStatusV1::Drained)
    );
    o.retire_shutdown_attempt_report_v1(&id, &control, Some(true), Some(true), &mut r)
        .unwrap();
    assert!(
        o.retire_shutdown_attempt_report_v1(&id, &control, Some(true), Some(true), &mut r)
            .is_err()
    );
    assert_eq!(o.retention_usage(), before);
}

fn owner() -> SourceCoordinator {
    SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 4,
        max_participants: 4,
        max_staged_bytes: 4096,
        source: Limits {
            max_source_bytes: 4096,
            max_edits: 8,
            max_edit_payload_bytes: 4096,
        },
    })
}
fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "host".into(),
        engine: "native".into(),
        recipient: "host".into(),
        purpose: "shutdown".into(),
        destination: "host".into(),
        profile: "native".into(),
        continuity: "local".into(),
    }
}
fn control(c: &SettingsExecutionContextV1) -> TrustedShutdownFenceFactsV1<'_> {
    TrustedShutdownFenceFactsV1 {
        context: c,
        authorized: Some(true),
        scope_current: Some(true),
        continuous: Some(true),
        owner_control_allowed: Some(true),
        admission_fence_allowed: Some(true),
        resources_ready: Some(true),
    }
}
fn limits() -> ShutdownInventoryLimitsV1 {
    ShutdownInventoryLimitsV1 {
        max_documents: 8,
        max_writes: 8,
        max_artifacts: 8,
        max_bindings: 8,
        max_reads: 8,
        max_owners: 64,
        max_attachments: 8,
        max_dependencies: 64,
        max_work: 1024,
        max_capture_bytes: 65536,
    }
}
fn fence(
    o: &mut SourceCoordinator,
    f: &TrustedShutdownFenceFactsV1<'_>,
    r: &mut Recorder,
) -> ShutdownFenceIdV1 {
    o.configure_retention_budget(1048576, r).unwrap();
    let basis = o
        .capture_shutdown_fence_basis_v1(
            ShutdownFenceLimitsV1 {
                max_context_bytes: 1024,
                max_root_bytes: 65536,
                max_capture_bytes: 131072,
            },
            f,
            r,
        )
        .unwrap();
    o.install_shutdown_fence_v1(&basis, f, r).unwrap()
}
#[test]
fn drained_attempt_still_requires_report_retirement_before_final_release_check() {
    for (capacity, timed) in [(0, false), (256, false), (256, true), (1, true)] {
        let mut o = owner();
        let mut r = Recorder::disabled();
        let c = context();
        let f = control(&c);
        let auth = TrustedShutdownInventoryFactsV1 {
            control: &f,
            inventory_readable: Some(true),
        };
        let id = fence(&mut o, &f, &mut r);
        let original = o.retention_usage();
        r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(if timed {
                NativeTimingProfileV1::ProcessMonotonicV1
            } else {
                NativeTimingProfileV1::Off
            });
        assert_eq!(
            o.finish_shutdown_attempt_v1(&id, limits(), &auth, &mut r),
            Ok(ShutdownAttemptStatusV1::Drained)
        );
        let held = o.retention_usage();
        assert!(held.reserved > original.reserved);
        assert_eq!(
            o.finish_shutdown_attempt_v1(&id, limits(), &auth, &mut r),
            Ok(ShutdownAttemptStatusV1::Drained)
        );
        assert_eq!(o.retention_usage(), held);
        assert!(
            o.check_shutdown_root_release_v1(&id, limits(), &auth, &mut r)
                .is_err()
        );
        assert!(o.create(b"new", &mut r).is_err());
        let (status, report) = o
            .read_shutdown_attempt_report_v1(&id, &auth, &mut r)
            .unwrap();
        assert_eq!(status, ShutdownAttemptStatusV1::Drained);
        assert!(report.documents.is_empty());
        assert!(
            o.retire_shutdown_attempt_report_v1(&id, &f, Some(true), None, &mut r)
                .is_err()
        );
        assert_eq!(o.retention_usage(), held);
        o.retire_shutdown_attempt_report_v1(&id, &f, Some(true), Some(true), &mut r)
            .unwrap();
        assert_eq!(o.retention_usage(), original);
        assert!(
            o.read_shutdown_attempt_report_v1(&id, &auth, &mut r)
                .is_err()
        );
        assert!(
            o.finish_shutdown_attempt_v1(&id, limits(), &auth, &mut r)
                .is_err()
        );
        o.check_shutdown_root_release_v1(&id, limits(), &auth, &mut r)
            .unwrap();
        assert!(o.create(b"still fenced", &mut r).is_err());
        o.retire_shutdown_control_v1(&id, limits(), &auth, &mut r)
            .unwrap();
        assert_eq!(o.retention_usage().reserved, 0);
        assert!(
            !o.is_faulted(),
            "graceful shutdown is not an integrity fault"
        );
        assert!(o.create(b"still closed", &mut r).is_err());
        assert!(
            o.capture_shutdown_fence_basis_v1(
                ShutdownFenceLimitsV1 {
                    max_context_bytes: 1024,
                    max_root_bytes: 65536,
                    max_capture_bytes: 131072
                },
                &f,
                &mut r
            )
            .is_err()
        );
        assert!(
            o.retire_shutdown_control_v1(&id, limits(), &auth, &mut r)
                .is_err()
        );
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if capacity == 256 {
            for boundary in [
                Boundary::ShutdownAttemptReturn,
                Boundary::ShutdownAttemptRelease,
                Boundary::ShutdownAttemptRetirement,
                Boundary::ShutdownRootReleaseCheck,
                Boundary::ShutdownControlRetirement,
            ] {
                let matching: Vec<_> = events.iter().filter(|e| e.boundary == boundary).collect();
                assert!(!matching.is_empty(), "{boundary:?}");
                assert!(
                    matching.iter().all(|e| e.service_time.is_some() == timed),
                    "{boundary:?}"
                );
            }
        }
    }
}

#[test]
fn unresolved_report_returns_without_discard_refresh_or_implicit_discharge() {
    let mut o = owner();
    let mut r = Recorder::disabled();
    let d = o.create(b"PRIVATE-unsaved", &mut r).unwrap();
    let c = context();
    let f = control(&c);
    let auth = TrustedShutdownInventoryFactsV1 {
        control: &f,
        inventory_readable: Some(true),
    };
    let id = fence(&mut o, &f, &mut r);
    let generation = o.generation();
    assert_eq!(
        o.finish_shutdown_attempt_v1(&id, limits(), &auth, &mut r),
        Ok(ShutdownAttemptStatusV1::UnresolvedRetained)
    );
    assert_eq!(o.generation(), generation);
    assert_eq!(
        o.read(&d.document, &mut r).unwrap().source.text(),
        "PRIVATE-unsaved"
    );
    let denied = TrustedShutdownInventoryFactsV1 {
        control: &f,
        inventory_readable: None,
    };
    assert!(
        o.read_shutdown_attempt_report_v1(&id, &denied, &mut r)
            .is_err()
    );
    let (_, report) = o
        .read_shutdown_attempt_report_v1(&id, &auth, &mut r)
        .unwrap();
    assert_eq!(report.documents.len(), 1);
    let mut changed = limits();
    changed.max_work += 1;
    assert!(
        o.finish_shutdown_attempt_v1(&id, changed, &auth, &mut r)
            .is_err()
    );
    o.retire_shutdown_attempt_report_v1(&id, &f, Some(true), Some(true), &mut r)
        .unwrap();
    assert!(
        o.check_shutdown_root_release_v1(&id, limits(), &auth, &mut r)
            .is_err()
    );
    assert!(
        o.retire_shutdown_control_v1(&id, limits(), &auth, &mut r)
            .is_err()
    );
    // The legacy discard cannot bypass shutdown's accepted-disposition path.
    // Returning/retiring this report grants no replacement discard authority.
    assert_eq!(
        o.dispose(&d.document, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    assert!(
        o.check_shutdown_root_release_v1(&id, limits(), &auth, &mut r)
            .is_err()
    );
    let mut other = owner();
    assert!(
        other
            .finish_shutdown_attempt_v1(&id, limits(), &auth, &mut r)
            .is_err()
    );
}
