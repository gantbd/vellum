use vellum_engine::coordinator::*;
use vellum_engine::resource::*;
#[path = "support/resource_catalog_fixture.rs"]
#[allow(dead_code)]
mod catalog;
#[path = "support/annotation_read_fixture.rs"]
#[allow(dead_code)]
mod support;
fn facts() -> TrustedCatalogAnnotationReadBindingV1 {
    TrustedCatalogAnnotationReadBindingV1 {
        profile: catalog::limits(),
        expected_generation: 0,
        max_binding_bytes: 16384,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        identity_allowed: Some(true),
        read_allowed: Some(true),
        annotation_readable: Some(true),
        provider_available: Some(true),
        correspondence_current: Some(true),
        resources_ready: Some(true),
    }
}
fn setup() -> support::Fixture {
    setup_identity(true)
}
fn setup_identity(known: bool) -> support::Fixture {
    let mut f = support::Fixture::new();
    let mut configuration = catalog::configuration(ResourceCatalogTableV1::Read);
    configuration.identity_known = known;
    f.owner
        .configure_resource_catalog_v1(
            &[ResourceCatalogRowV1 {
                address: catalog::address(ResourceCatalogTableV1::Read, "annotations"),
                generation: 0,
                configuration,
            }],
            catalog::limits(),
            &catalog::facts(),
            &mut f.recorder,
        )
        .unwrap();
    f
}
fn target(f: &mut support::Fixture) -> AnnotationReadTarget {
    let raw = f.port.annotation_read_target(b"comments").unwrap();
    f.owner
        .bind_catalog_annotation_read_target_v1(
            raw,
            &catalog::address(ResourceCatalogTableV1::Read, "annotations"),
            &facts(),
            &mut f.recorder,
        )
        .unwrap()
}
fn admit(
    f: &mut support::Fixture,
    target: AnnotationReadTarget,
) -> Result<AnnotationReadId, ResourceError> {
    let pair = f.pair();
    f.owner.admit_annotation_read(
        &f.doc,
        0,
        target,
        ReadStrength::CapturedRead,
        pair,
        &mut f.recorder,
    )
}
fn remove(f: &mut support::Fixture) {
    let plan = f
        .owner
        .prepare_resource_catalog_v1(
            &[ResourceCatalogChangeV1 {
                address: catalog::address(ResourceCatalogTableV1::Read, "annotations"),
                expected: CatalogExpectationV1::Present { generation: 0 },
                action: ResourceCatalogActionV1::Remove,
            }],
            &catalog::facts(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .commit_resource_catalog_v1(plan, &catalog::facts(), &mut f.recorder)
        .unwrap();
}
#[test]
fn withdrawn_annotation_registration_refuses_admission_and_ready_dispatch() {
    let mut f = setup();
    let bound = target(&mut f);
    let waiting = target(&mut f);
    let id = admit(&mut f, bound).unwrap();
    remove(&mut f);
    assert_eq!(admit(&mut f, waiting), Err(ResourceError::Stale));
    let pair = f.pair();
    assert!(matches!(
        f.owner.submit_annotation_read(&id, pair, &mut f.recorder),
        Err(ResourceError::Stale)
    ));
}

fn complete(f: &mut support::Fixture, dispatch: &AnnotationReadDispatch) {
    for e in [
        f.port.annotation_read_start(dispatch, b"version").unwrap(),
        f.port.annotation_read_chunk(dispatch, 0, 0, b"{}").unwrap(),
        f.port
            .annotation_read_end(dispatch, b"version", 1, 2)
            .unwrap(),
        f.port.annotation_read_quiescence(dispatch).unwrap(),
    ] {
        f.record(e).unwrap();
    }
}
fn package(f: &mut support::Fixture, capture: CapturedAnnotationRead) -> AnnotationRestoration {
    let restorer = f
        .owner
        .register_annotation_restorer(&mut f.recorder)
        .unwrap();
    let witness = f
        .owner
        .capture_annotations(&f.doc, 0, &mut f.recorder)
        .unwrap();
    restorer.seal(capture, vec![witness], vec![]).unwrap()
}
#[test]
fn late_original_evidence_survives_withdrawal_and_retirement_still_requires_handle_drain() {
    let mut f = setup();
    let bound = target(&mut f);
    let id = admit(&mut f, bound).unwrap();
    let dispatch = f.submit(&id);
    let held = f.owner.resource_usage().held_read_bytes;
    remove(&mut f);
    complete(&mut f, &dispatch);
    let status = f
        .owner
        .annotation_read_status(&id, &mut f.recorder)
        .unwrap();
    assert!(status.quiescent && !status.contested);
    assert_eq!(status.received_bytes, 2);
    assert!(matches!(f.capture(&id), Err(ResourceError::Stale)));
    assert_eq!(f.owner.resource_usage().held_read_bytes, held);
    assert_eq!(
        f.owner.retire_annotation_read(&id, &mut f.recorder),
        Err(ResourceError::Retained)
    );
    drop(dispatch);
    f.owner
        .retire_annotation_read(&id, &mut f.recorder)
        .unwrap();
    assert_eq!(f.owner.resource_usage().held_read_bytes, 0);
}
#[test]
fn a_previously_captured_package_cannot_restore_after_withdrawal() {
    let mut f = setup();
    let bound = target(&mut f);
    let id = admit(&mut f, bound).unwrap();
    let dispatch = f.submit(&id);
    complete(&mut f, &dispatch);
    let capture = f.capture(&id).unwrap();
    let retained = capture.clone();
    let package = package(&mut f, capture);
    remove(&mut f);
    let generation = f.owner.generation();
    let pair = f.pair();
    assert_eq!(
        f.owner.restore_annotations(package, pair, &mut f.recorder),
        Err(ResourceError::Stale)
    );
    assert_eq!(f.owner.generation(), generation);
    assert_eq!(retained.bytes(), b"{}");
}
#[test]
fn unrelated_registration_does_not_block_restoration_and_historical_outcome_survives_withdrawal() {
    let mut f = setup();
    let bound = target(&mut f);
    let id = admit(&mut f, bound).unwrap();
    let plan = f
        .owner
        .prepare_resource_catalog_v1(
            &[catalog::install(ResourceCatalogTableV1::Write, "other")],
            &catalog::facts(),
            &mut f.recorder,
        )
        .unwrap();
    f.owner
        .commit_resource_catalog_v1(plan, &catalog::facts(), &mut f.recorder)
        .unwrap();
    let dispatch = f.submit(&id);
    complete(&mut f, &dispatch);
    let capture = f.capture(&id).unwrap();
    let package = package(&mut f, capture);
    let pair = f.pair();
    let original = f
        .owner
        .restore_annotations(package, pair, &mut f.recorder)
        .unwrap();
    remove(&mut f);
    assert_eq!(
        f.owner
            .annotation_restoration_outcome(&id, &mut f.recorder)
            .unwrap(),
        original
    );
}
#[test]
fn unknown_catalog_identity_allows_capture_but_not_restoration() {
    let mut f = setup_identity(false);
    let bound = target(&mut f);
    let id = admit(&mut f, bound).unwrap();
    let dispatch = f.submit(&id);
    complete(&mut f, &dispatch);
    let capture = f.capture(&id).unwrap();
    let package = package(&mut f, capture);
    let generation = f.owner.generation();
    let pair = f.pair();
    assert_eq!(
        f.owner.restore_annotations(package, pair, &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
    assert_eq!(f.owner.generation(), generation);
}

#[test]
fn all_annotation_binding_facts_are_strict_and_capture_is_observational_when_off_or_saturated() {
    use vellum_engine::instrumentation::{Boundary, NativeTimingProfileV1, Recorder};
    for capacity in [0, 1, 128] {
        let mut f = setup();
        f.recorder = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        for index in 0..10 {
            for value in [None, Some(false)] {
                let mut facts = facts();
                let flags = [
                    &mut facts.authorized,
                    &mut facts.scope,
                    &mut facts.continuous,
                    &mut facts.live,
                    &mut facts.identity_allowed,
                    &mut facts.read_allowed,
                    &mut facts.annotation_readable,
                    &mut facts.provider_available,
                    &mut facts.correspondence_current,
                    &mut facts.resources_ready,
                ];
                *flags.into_iter().nth(index).unwrap() = value;
                let raw = f.port.annotation_read_target(b"comments").unwrap();
                assert!(matches!(
                    f.owner.bind_catalog_annotation_read_target_v1(
                        raw,
                        &catalog::address(ResourceCatalogTableV1::Read, "annotations"),
                        &facts,
                        &mut f.recorder
                    ),
                    Err(ResourceError::Unavailable)
                ));
            }
        }
        let bound = target(&mut f);
        let id = admit(&mut f, bound).unwrap();
        remove(&mut f);
        let pair = f.pair();
        assert!(matches!(
            f.owner.submit_annotation_read(&id, pair, &mut f.recorder),
            Err(ResourceError::Stale)
        ));
        let mut bindings = 0;
        let mut checks = 0;
        while let Some(record) = f.recorder.pop_front() {
            match record.boundary {
                Boundary::CatalogAnnotationReadBinding => {
                    bindings += 1;
                    assert!(record.service_time.is_some());
                }
                Boundary::CatalogAnnotationReadCurrentness => {
                    checks += 1;
                    assert!(record.service_time.is_some());
                }
                _ => {}
            }
        }
        if capacity == 128 {
            assert_eq!(bindings, 21);
            assert_eq!(checks, 2);
        }
        if capacity == 1 {
            assert!(f.recorder.dropped() > 0);
        }
        assert_eq!(f.owner.resource_usage().issued_reads_lifetime, 1);
    }
}

#[test]
fn owned_annotation_recovery_keeps_the_original_catalog_dependency_without_relaunch_or_unbinding() {
    let mut f = setup();
    f.owner
        .configure_retention_budget(2097152, &mut f.recorder)
        .unwrap();
    f.owner
        .configure_annotation_read_operations_v1(
            AnnotationReadOperationLimitsV1 {
                max_capture_bytes: 8192,
                max_reply_bytes: 1024,
                max_label_bytes: 64,
                max_store_bytes: 1048576,
            },
            &mut f.recorder,
        )
        .unwrap();
    let target = target(&mut f);
    let annotation = f.port.source_binding_target(b"comments").unwrap();
    let source = f.port.source_binding_target(b"source").unwrap();
    let context = SettingsExecutionContextV1 {
        authority: "a".into(),
        engine: "e".into(),
        recipient: "r".into(),
        purpose: "read".into(),
        destination: "d".into(),
        profile: "native/1".into(),
        continuity: "c".into(),
    };
    let facts = TrustedAnnotationReadOperationFactsV1 {
        context: &context,
        authorized: Some(true),
        scope: Some(true),
        continuous: Some(true),
        live: Some(true),
        not_stopped: Some(true),
        read_allowed: Some(true),
        annotation_readable: Some(true),
        retained: Some(true),
        reply_ready: Some(true),
        transfer_ready: Some(true),
        resources_ready: Some(true),
        identity_allowed: Some(true),
        binding_coverage_complete: Some(true),
        provider_available: Some(true),
        launch_allowed: Some(true),
    };
    let d = f.doc.clone();
    let request = OwnedAnnotationReadRequestV1 {
        origin: AnnotationReadOperationOriginV1 {
            operation: b"load",
            effect: b"read",
            attempt: b"one",
        },
        source: &d,
        expected_set: 0,
        annotation_binding: ExpectedAnnotationReadBindingV1 {
            target: &annotation,
            generation: 1,
        },
        source_binding: Some(ExpectedAnnotationReadBindingV1 {
            target: &source,
            generation: 0,
        }),
        target: &target,
        strength: ReadStrength::CapturedRead,
    };
    let pair = f.pair();
    let first = f
        .owner
        .request_owned_annotation_read_v1(&request, &facts, pair, &mut f.recorder)
        .unwrap();
    let held = f.owner.resource_usage();
    remove(&mut f);
    let again = f
        .owner
        .request_owned_annotation_read_v1(&request, &facts, None, &mut f.recorder)
        .unwrap();
    assert_eq!(
        again.status,
        AnnotationReadOperationAdmissionStatusV1::ObserveExisting
    );
    assert_eq!(again.task, first.task);
    assert_eq!(f.owner.resource_usage(), held);
    let raw = f.port.annotation_read_target(b"comments").unwrap();
    let unbound = OwnedAnnotationReadRequestV1 {
        target: &raw,
        ..request
    };
    assert_eq!(
        f.owner
            .request_owned_annotation_read_v1(&unbound, &facts, None, &mut f.recorder),
        Err(ResourceError::Unavailable)
    );
    let pair = f.pair();
    assert!(matches!(
        f.owner
            .submit_annotation_read(&first.task, pair, &mut f.recorder),
        Err(ResourceError::Stale)
    ));
}
