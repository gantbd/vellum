use super::*;
fn context() -> SettingsExecutionContextV1 {
    crate::coordinator::serialization::tests::context()
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
fn fence(
    o: &mut SourceCoordinator,
    f: &TrustedShutdownFenceFactsV1<'_>,
    r: &mut Recorder,
) -> ShutdownFenceIdV1 {
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
fn drained(
    preparation: bool,
) -> (
    SourceCoordinator,
    Option<PreparedSourceArtifactIdV1>,
    Recorder,
) {
    let (mut o, _, id, mut r) = super::super::read::tests::fixture();
    o.configure_retention_budget(4194304, &mut r).unwrap();
    let d = super::super::read::tests::original_source(&o, &id);
    o.stop_annotation_read(&id, &mut r).unwrap();
    o.retire_annotation_read(&id, &mut r).unwrap();
    let prepared = preparation.then(|| {
        let s = o.source_serialization(&d.document, &mut r).unwrap();
        o.prepare_source_artifact(&d, s, &mut r).unwrap()
    });
    let plan = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
    let state = o.commit_retained_disposal(plan, &mut r).unwrap();
    o.release_disposed_annotation_binding(&d.document, state, &mut r)
        .unwrap();
    let state = o.lifecycle_state(&d.document, &mut r).unwrap();
    o.release_disposed_provenance(&d.document, state, &mut r)
        .unwrap();
    o.stop_annotation_read_admission_v1(&mut r);
    o.retire_annotation_reader_v1(Some(true), Some(true), Some(true), &mut r)
        .unwrap();
    o.stop_source_save_admission_v1(&mut r);
    o.stop_annotation_save_admission(&mut r);
    (o, prepared, r)
}
#[test]
fn actual_writer_closes_under_its_original_fence_without_refunding_fence_or_unrelated_credit() {
    for mode in 0..4 {
        let (mut o, _, mut r) = drained(false);
        let _other = o.retention.reserve(73).unwrap();
        let c = context();
        let f = control(&c);
        let id = fence(&mut o, &f, &mut r);
        let held = o.retention_usage();
        let generation = o.generation();
        let root = o.native_root_identity().unwrap();
        r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 64 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(
                crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1,
            );
        }
        o.retire_shutdown_writer_v1(&id, &f, Some(true), Some(true), &mut r)
            .unwrap();
        assert!(o.resources.writer_closed);
        assert_ne!(o.native_root_identity().unwrap(), root);
        assert_eq!(o.retention_usage(), held);
        assert_eq!(o.generation(), generation);
        assert_eq!(o.create(b"new", &mut r), Err(CoordinatorError::Unavailable));
        assert_eq!(
            o.retire_shutdown_writer_v1(&id, &f, Some(true), Some(true), &mut r),
            Err(ResourceError::Unavailable)
        );
        if mode == 1 || mode == 2 {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            let e = events
                .iter()
                .find(|e| {
                    e.boundary == Boundary::ShutdownWriterRetirement
                        && e.disposition == Disposition::Completed
                })
                .unwrap();
            assert_eq!(e.service_time.is_some(), mode == 2);
        } else if mode == 3 {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}
#[test]
fn writer_close_requires_exact_domain_fence_current_control_and_independent_discharge_facts() {
    let (mut o, _, mut r) = drained(false);
    let (mut peer, _, mut pr) = drained(false);
    let c = context();
    let f = control(&c);
    let foreign = fence(&mut peer, &f, &mut pr);
    assert!(
        o.retire_shutdown_writer_v1(&foreign, &f, Some(true), Some(true), &mut r)
            .is_err()
    );
    let id = fence(&mut o, &f, &mut r);
    let root = o.native_root_identity().unwrap();
    for index in 0..8 {
        for denied in [None, Some(false)] {
            let mut f = control(&c);
            let mut retained = Some(true);
            let mut drained = Some(true);
            *match index {
                0 => &mut f.authorized,
                1 => &mut f.scope_current,
                2 => &mut f.continuous,
                3 => &mut f.owner_control_allowed,
                4 => &mut f.admission_fence_allowed,
                5 => &mut f.resources_ready,
                6 => &mut retained,
                _ => &mut drained,
            } = denied;
            assert!(
                o.retire_shutdown_writer_v1(&id, &f, retained, drained, &mut r)
                    .is_err()
            );
            assert_eq!(o.native_root_identity().unwrap(), root);
        }
    }
    assert!(
        o.retire_shutdown_writer_v1(&foreign, &f, Some(true), Some(true), &mut r)
            .is_err()
    );
    o.retire_shutdown_writer_v1(&id, &f, Some(true), Some(true), &mut r)
        .unwrap();
}
#[test]
fn source_preparation_must_be_explicitly_discarded_before_writer_owner_retirement() {
    let (mut o, prepared, mut r) = drained(true);
    let c = context();
    let f = control(&c);
    let id = fence(&mut o, &f, &mut r);
    let before = o.resource_usage();
    assert!(
        o.retire_shutdown_writer_v1(&id, &f, Some(true), Some(true), &mut r)
            .is_err()
    );
    assert_eq!(o.resource_usage(), before);
    o.discard_source_artifact_preparation(&prepared.unwrap(), &mut r)
        .unwrap();
    o.retire_shutdown_writer_v1(&id, &f, Some(true), Some(true), &mut r)
        .unwrap();
}
#[test]
fn empty_source_reader_is_a_real_owner_and_save_origin_storage_closes_without_regrant() {
    let (mut o, _, mut r) = drained(false);
    let operations = SaveOperationLimitsV1 {
        max_issued: 4,
        max_label_bytes: 64,
        max_capture_bytes: 8192,
        max_reply_bytes: 1024,
        max_store_bytes: 65536,
    };
    o.configure_save_operations_v1(operations, &mut r).unwrap();
    o.configure_source_reader_v1(
        SourceReaderLimitsV1 {
            max_live: 1,
            max_issued: 1,
            max_input_bytes: 64,
            max_chunks: 4,
            max_held_bytes: 8192,
            max_store_bytes: 65536,
        },
        &mut r,
    )
    .unwrap();
    let c = context();
    let f = control(&c);
    let id = fence(&mut o, &f, &mut r);
    assert_eq!(
        o.retire_shutdown_writer_v1(&id, &f, Some(true), Some(true), &mut r),
        Err(ResourceError::Retained)
    );
    o.stop_source_read_admission_v1(&mut r).unwrap();
    o.retire_source_reader_v1(Some(true), Some(true), Some(true), &mut r)
        .unwrap();
    let held = o.retention_usage();
    o.retire_shutdown_writer_v1(&id, &f, Some(true), Some(true), &mut r)
        .unwrap();
    assert!(o.resources.operations.retired_empty());
    assert_eq!(o.retention_usage(), held);
    assert_eq!(
        o.configure_save_operations_v1(operations, &mut r),
        Err(ResourceError::Unavailable)
    );
}

#[test]
fn empty_but_active_observation_pool_must_return_its_actual_parent_reservation() {
    let (mut o, _, mut r) = drained(false);
    o.configure_native_operation_observation_v1(
        NativeOperationObservationPoolLimitsV1 {
            max_contexts: 1,
            max_issued: 1,
            max_pool_bytes: 65536,
        },
        &mut r,
    )
    .unwrap();
    let c = context();
    let f = control(&c);
    let id = fence(&mut o, &f, &mut r);
    let before = o.retention_usage();
    assert!(
        o.retire_shutdown_writer_v1(&id, &f, Some(true), Some(true), &mut r)
            .is_err()
    );
    assert_eq!(o.retention_usage(), before);
    o.stop_native_operation_observation_admission_v1(&mut r)
        .unwrap();
    o.retire_native_operation_observation_pool_v1(Some(true), Some(true), Some(true), &mut r)
        .unwrap();
    let before = o.retention_usage();
    o.faulted = true;
    o.generation = u64::MAX;
    o.retire_shutdown_writer_v1(&id, &f, Some(true), Some(true), &mut r)
        .unwrap();
    assert_eq!(o.retention_usage(), before);
    assert!(o.is_faulted());
    assert_eq!(o.generation(), u64::MAX);
}
