use super::*;
pub(in crate::coordinator) fn context() -> SettingsExecutionContextV1 {
    SettingsExecutionContextV1 {
        authority: "local".into(),
        engine: "engine".into(),
        recipient: "host".into(),
        purpose: "shutdown".into(),
        destination: "host".into(),
        profile: "native-shutdown".into(),
        continuity: "local".into(),
    }
}
pub(in crate::coordinator) fn facts(
    c: &SettingsExecutionContextV1,
) -> TrustedShutdownFenceFactsV1<'_> {
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
pub(in crate::coordinator) fn limits() -> ShutdownFenceLimitsV1 {
    ShutdownFenceLimitsV1 {
        max_context_bytes: 1024,
        max_root_bytes: 65536,
        max_capture_bytes: 131072,
    }
}
pub(in crate::coordinator) fn install(o: &mut SourceCoordinator, r: &mut Recorder) {
    let c = context();
    let f = facts(&c);
    let basis = o.capture_shutdown_fence_basis_v1(limits(), &f, r).unwrap();
    o.install_shutdown_fence_v1(&basis, &f, r).unwrap();
}
pub(in crate::coordinator) fn inspect(
    o: &SourceCoordinator,
    r: &mut Recorder,
) -> ShutdownInventoryV1 {
    let c = context();
    let f = facts(&c);
    let basis = o
        .capture_shutdown_fence_basis_v1(
            ShutdownFenceLimitsV1 {
                max_context_bytes: 1024,
                max_root_bytes: 8388608,
                max_capture_bytes: 16777216,
            },
            &f,
            r,
        )
        .unwrap();
    o.capture_shutdown_inventory_v1(
        &basis,
        ShutdownInventoryLimitsV1 {
            max_documents: 64,
            max_writes: 64,
            max_artifacts: 64,
            max_bindings: 64,
            max_reads: 64,
            max_owners: 64,
            max_attachments: 64,
            max_dependencies: 1024,
            max_work: 100000,
            max_capture_bytes: 1048576,
        },
        &TrustedShutdownInventoryFactsV1 {
            control: &f,
            inventory_readable: Some(true),
        },
        r,
    )
    .unwrap()
}
pub(in crate::coordinator) fn inspect_read<'a>(
    o: &SourceCoordinator,
    snapshot: &'a ShutdownInventoryV1,
    r: &mut Recorder,
) -> ShutdownInventoryReadV1<'a> {
    let c = context();
    let f = facts(&c);
    o.read_shutdown_inventory_v1(
        snapshot,
        &TrustedShutdownInventoryFactsV1 {
            control: &f,
            inventory_readable: Some(true),
        },
        r,
    )
    .unwrap()
}
pub(in crate::coordinator) fn inventory_owner(
    o: &SourceCoordinator,
    r: &mut Recorder,
    role: ShutdownOwnerRoleV1,
    documents: &[&DocumentId],
) -> ShutdownOwnerRecordV1 {
    let before = (o.generation(), o.resource_usage(), o.retention_usage());
    let cut = inspect(o, r);
    let view = inspect_read(o, &cut, r);
    let row = view
        .owners
        .iter()
        .find(|row| row.owner.role() == role)
        .unwrap();
    for document in documents {
        assert!(view.dependencies.iter().any(|edge| edge.dependent
            == ShutdownInventoryTargetV1::Owner(row.owner.clone())
            && edge.required == ShutdownInventoryTargetV1::Document((*document).clone())));
    }
    assert_eq!(
        view.coverage.all_owner_obligations,
        view.funding.unrepresented_parent_bytes == 0
    );
    assert_eq!(
        (o.generation(), o.resource_usage(), o.retention_usage()),
        before
    );
    row.clone()
}
pub(in crate::coordinator) fn invocation_funding(
    o: &SourceCoordinator,
    id: &InvocationId,
    count: usize,
    bytes: usize,
    r: &mut Recorder,
) {
    let cut = inspect(o, r);
    let rows = inspect_read(o, &cut, r);
    let invocation = rows
        .invocations
        .iter()
        .find(|row| row.invocation == *id)
        .unwrap();
    assert_eq!(
        (invocation.retained_holds, invocation.reserved_bytes),
        (count, bytes)
    );
    let owner = rows
        .owners
        .iter()
        .find(|row| row.owner.role() == ShutdownOwnerRoleV1::Invocations)
        .unwrap();
    assert_eq!(
        owner.additional_reserved_bytes,
        Some(rows.invocations.iter().map(|row| row.reserved_bytes).sum())
    );
}
pub(in crate::coordinator) fn inspection_current(
    o: &SourceCoordinator,
    snapshot: &ShutdownInventoryV1,
    r: &mut Recorder,
) -> Result<(), CoordinatorError> {
    let c = context();
    let f = facts(&c);
    o.validate_shutdown_inventory_current_v1(
        snapshot,
        &TrustedShutdownInventoryFactsV1 {
            control: &f,
            inventory_readable: Some(true),
        },
        r,
    )
}
#[test]
fn actual_fence_blocks_birth_and_prepared_publication_without_changing_the_document() {
    let (mut o, mut r, d, _) = crate::coordinator::resource::source_save_test_fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let plan = o
        .prepare(
            &[DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "B".into(),
                }],
            }],
            &[],
            &mut r,
        )
        .unwrap();
    let c = context();
    let basis = o
        .capture_shutdown_fence_basis_v1(limits(), &facts(&c), &mut r)
        .unwrap();
    o.install_shutdown_fence_v1(&basis, &facts(&c), &mut r)
        .unwrap();
    let before = o.generation();
    assert_eq!(o.create(b"new", &mut r), Err(CoordinatorError::Unavailable));
    assert_eq!(o.commit(plan, &mut r), Err(CoordinatorError::Unavailable));
    assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "A");
    assert_eq!(o.generation(), before);
}
#[test]
fn fence_requires_original_basis_current_complete_control_and_exact_parent_headroom() {
    for delta in [-1isize, 0, 1] {
        let (mut o, mut r, _, _) = resource::source_save_test_fixture();
        let l = limits();
        let charge = l.required_capture_bytes().unwrap();
        o.configure_retention_budget(charge.checked_add_signed(delta).unwrap(), &mut r)
            .unwrap();
        let c = context();
        let basis = o
            .capture_shutdown_fence_basis_v1(l, &facts(&c), &mut r)
            .unwrap();
        let copied = o
            .capture_shutdown_fence_basis_v1(l, &facts(&c), &mut r)
            .unwrap();
        let before = o.native_root_identity().unwrap();
        for i in 0..6 {
            for value in [None, Some(false)] {
                let mut denied = facts(&c);
                *match i {
                    0 => &mut denied.authorized,
                    1 => &mut denied.scope_current,
                    2 => &mut denied.continuous,
                    3 => &mut denied.owner_control_allowed,
                    4 => &mut denied.admission_fence_allowed,
                    _ => &mut denied.resources_ready,
                } = value;
                assert_eq!(
                    o.install_shutdown_fence_v1(&basis, &denied, &mut r),
                    Err(CoordinatorError::Unavailable)
                );
                assert_eq!(o.retention_usage().reserved, 0);
                assert_eq!(o.native_root_identity().unwrap(), before);
            }
        }
        let result = o.install_shutdown_fence_v1(&basis, &facts(&c), &mut r);
        if delta < 0 {
            assert_eq!(result, Err(CoordinatorError::Limit));
            assert_eq!(o.native_root_identity().unwrap(), before);
            assert_eq!(o.retention_usage().reserved, 0);
        } else {
            let id = result.unwrap();
            assert_ne!(o.native_root_identity().unwrap(), before);
            assert_eq!(o.retention_usage().reserved, charge);
            assert_eq!(
                o.install_shutdown_fence_v1(&copied, &facts(&c), &mut r),
                Err(CoordinatorError::Unavailable)
            );
            o.faulted = true;
            assert_eq!(
                o.install_shutdown_fence_v1(&basis, &facts(&c), &mut r)
                    .unwrap(),
                id
            );
            assert_eq!(o.retention_usage().reserved, charge);
            assert!(o.faulted);
        }
    }
}
#[test]
fn stale_foreign_or_busy_basis_refuses_without_installing_or_charging_a_fence() {
    let (mut o, mut r, d, _) = resource::source_save_test_fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let c = context();
    let basis = o
        .capture_shutdown_fence_basis_v1(limits(), &facts(&c), &mut r)
        .unwrap();
    o.source_facade_busy = true;
    assert_eq!(
        o.install_shutdown_fence_v1(&basis, &facts(&c), &mut r),
        Err(CoordinatorError::Unavailable)
    );
    o.source_facade_busy = false;
    let (mut peer, mut pr, _, _) = resource::source_save_test_fixture();
    peer.configure_retention_budget(1048576, &mut pr).unwrap();
    assert_eq!(
        peer.install_shutdown_fence_v1(&basis, &facts(&c), &mut pr),
        Err(CoordinatorError::Unavailable)
    );
    o.set_read_only(&d.document, true, &mut r).unwrap();
    assert_eq!(
        o.install_shutdown_fence_v1(&basis, &facts(&c), &mut r),
        Err(CoordinatorError::Stale)
    );
    assert_eq!(o.retention_usage().reserved, 0);
    assert!(o.shutdown.fence.is_none());
    peer.create(b"unrelated", &mut pr).unwrap();
}
#[test]
fn preadmitted_save_settles_and_protected_evidence_survives_while_new_work_and_old_edits_are_fenced()
 {
    use crate::resource::{
        Acknowledgment, Application, Protection, ResourceError, SaveRequirements, Synchronization,
    };
    for capacity in [0, 1, 128] {
        let (mut o, mut setup, d, port) = resource::source_save_test_fixture();
        o.invocations = invocation::InvocationTable::new(InvocationLimits {
            max_lifetime_issued: 8,
            max_retained: 8,
            max_intent_bytes: 4096,
        });
        o.configure_retention_budget(1048576, &mut setup).unwrap();
        let pending = o.issue_invocation(&mut setup).unwrap();
        let fresh = o.issue_invocation(&mut setup).unwrap();
        let intent = SourceIntent {
            writes: vec![DocumentEdits {
                expected: d.clone(),
                edits: vec![Replacement {
                    start: 0,
                    end: 1,
                    expected: "A".into(),
                    inserted: "B".into(),
                }],
            }],
            reads: vec![],
        };
        o.submit(&pending, &intent, &mut setup).unwrap();
        let req = SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Ordinary,
        };
        let save = o.queue_pinned_save(&d, req, &mut setup).unwrap();
        let c = context();
        let basis = o
            .capture_shutdown_fence_basis_v1(limits(), &facts(&c), &mut setup)
            .unwrap();
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        let id = o
            .install_shutdown_fence_v1(&basis, &facts(&c), &mut r)
            .unwrap();
        let before = o.resource_usage();
        assert_eq!(
            o.queue_pinned_save(&d, req, &mut r),
            Err(ResourceError::Unavailable)
        );
        assert_eq!(o.resource_usage(), before);
        assert_eq!(
            o.issue_invocation(&mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            o.submit(&fresh, &intent, &mut r),
            Err(InvocationError::Unavailable)
        );
        assert_eq!(
            o.invocation_state(&fresh, &mut r).unwrap(),
            InvocationState::Issued
        );
        assert_eq!(
            o.submit(&pending, &intent, &mut r).unwrap(),
            InvocationState::Pending
        );
        o.stop_invocation(&pending, &mut r).unwrap();
        o.run(&pending, &mut r).unwrap();
        let dispatch = o
            .dispatch_save(
                &save,
                port.read_result(b"key", b"v0", b"A").unwrap(),
                &mut r,
            )
            .unwrap();
        o.stop_save(&save, &mut r).unwrap();
        assert_eq!(
            o.release_source_save_capacity_v1(&save, &mut r),
            Err(ResourceError::Retained)
        );
        for e in [
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
            o.record_save_evidence(e, &mut r).unwrap();
        }
        drop(dispatch);
        o.publish_save_baseline(&save, &mut r).unwrap();
        assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), "A");
        assert_eq!(o.resource_usage().pending_baseline_publications, 0);
        assert_eq!(
            o.install_shutdown_fence_v1(&basis, &facts(&c), &mut r)
                .unwrap(),
            id
        );
        if capacity == 1 {
            assert!(r.dropped() > 0);
        }
        if capacity == 128 {
            assert!(std::iter::from_fn(|| r.pop_front()).any(|e| e.boundary
                == Boundary::ShutdownFenceAdmission
                && e.service_time.is_some()));
        }
    }
}
#[test]
fn lost_fence_funding_is_not_a_reopened_admission_or_a_refunded_control() {
    let (mut o, mut r, _, _) = resource::source_save_test_fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let c = context();
    let basis = o
        .capture_shutdown_fence_basis_v1(limits(), &facts(&c), &mut r)
        .unwrap();
    o.install_shutdown_fence_v1(&basis, &facts(&c), &mut r)
        .unwrap();
    let charge = o.retention_usage().reserved;
    o.shutdown.fence.as_mut().unwrap().charge += 1;
    assert_eq!(o.create(b"escape", &mut r), Err(CoordinatorError::Faulted));
    assert!(o.faulted);
    assert_eq!(o.retention_usage().reserved, charge);
    assert!(o.shutdown.fence.is_some());
}
#[test]
fn source_and_annotation_promises_settle_in_either_order_under_one_actual_fence() {
    use crate::resource::{
        Acknowledgment, Application, Protection, ResourceRelation, SaveRequirements,
        Synchronization,
    };
    for reverse in [false, true] {
        let (mut o, mut r, annotation, port) = resource::annotation_save_test_fixture();
        let document = o.save_outcome(&annotation, &mut r).unwrap().captured;
        let check = o
            .ordinary_pair_check(&document.document, &mut r)
            .unwrap()
            .unwrap();
        let g = port
            .attest_ordinary_pair(&check, ResourceRelation::Distinct)
            .unwrap();
        let source = o
            .queue_pinned_save_with_pair(
                &document,
                SaveRequirements {
                    protection: Protection::BestEffort,
                    acknowledgment: Acknowledgment::Ordinary,
                },
                g,
                &mut r,
            )
            .unwrap();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let c = context();
        let basis = o
            .capture_shutdown_fence_basis_v1(limits(), &facts(&c), &mut r)
            .unwrap();
        o.install_shutdown_fence_v1(&basis, &facts(&c), &mut r)
            .unwrap();
        assert_eq!(o.resource_usage().pending_baseline_publications, 2);
        let order = if reverse {
            [&annotation, &source]
        } else {
            [&source, &annotation]
        };
        for (index, id) in order.into_iter().enumerate() {
            let is_source = id == &source;
            let check = o
                .ordinary_pair_check(&document.document, &mut r)
                .unwrap()
                .unwrap();
            let g = port
                .attest_ordinary_pair(&check, ResourceRelation::Distinct)
                .unwrap();
            let read = if is_source {
                port.read_result(b"key", b"v0", b"A").unwrap()
            } else {
                port.read_result(b"comments", b"v0", b"{}").unwrap()
            };
            let dispatch = o.dispatch_save_with_pair(id, read, g, &mut r).unwrap();
            for e in [
                port.result(
                    &dispatch,
                    Application::Written,
                    dispatch.bytes().len(),
                    Synchronization::NotRequested,
                    Some(b"v1"),
                )
                .unwrap(),
                port.no_future_effects(&dispatch).unwrap(),
                port.quiescence(&dispatch).unwrap(),
            ] {
                o.record_save_evidence(e, &mut r).unwrap();
            }
            drop(dispatch);
            let check = o
                .ordinary_pair_check(&document.document, &mut r)
                .unwrap()
                .unwrap();
            let g = port
                .attest_ordinary_pair(&check, ResourceRelation::Distinct)
                .unwrap();
            if is_source {
                o.publish_save_baseline_with_pair(id, g, &mut r).unwrap();
            } else {
                assert!(
                    o.install_annotation_save_baseline(id, Some(g), &mut r)
                        .unwrap()
                );
            }
            assert_eq!(o.resource_usage().pending_baseline_publications, 1 - index);
        }
        assert_eq!(o.resource_usage().reserved_baseline_bytes, 0);
        assert_eq!(
            o.read(&document.document, &mut r).unwrap().version,
            document
        );
    }
}
#[test]
fn lazy_reader_writer_attachment_cannot_enter_after_fencing_but_existing_attachment_is_not_renewed()
{
    for attached in [false, true] {
        let (mut o, mut r, _, _) = resource::source_save_test_fixture();
        if attached {
            o.attach_annotation_read_baselines(&mut r).unwrap();
        }
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let c = context();
        let f = facts(&c);
        let basis = o
            .capture_shutdown_fence_basis_v1(limits(), &f, &mut r)
            .unwrap();
        o.install_shutdown_fence_v1(&basis, &f, &mut r).unwrap();
        let before = o.resource_usage();
        assert_eq!(
            o.attach_annotation_read_baselines(&mut r),
            if attached {
                Ok(false)
            } else {
                Err(crate::resource::ResourceError::Unavailable)
            }
        );
        assert_eq!(o.resource_usage(), before);
    }
}
