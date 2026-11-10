use super::*;
use crate::coordinator::shutdown::tests::{
    context, facts as control_facts, limits as fence_limits,
};
use crate::resource::{Acknowledgment, Protection, SaveRequirements, Synchronization};
fn limits() -> ShutdownInventoryLimitsV1 {
    ShutdownInventoryLimitsV1 {
        max_documents: 16,
        max_writes: 16,
        max_artifacts: 16,
        max_bindings: 16,
        max_reads: 16,
        max_owners: 64,
        max_attachments: 8,
        max_dependencies: 64,
        max_work: 256,
        max_capture_bytes: 65536,
    }
}
#[test]
fn complete_owner_inventory_requires_reconciled_parent_funding_not_empty_work_tables() {
    use crate::coordinator::shutdown::tests::{inspect, inspect_read, inspection_current};
    let (mut o, mut r, _, _, _) = setup();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let original = inspect(&o, &mut r);
    let read = inspect_read(&o, &original, &mut r);
    assert!(read.coverage.all_owner_obligations);
    assert_eq!(read.funding.unrepresented_parent_bytes, 0);
    let mut unregistered = o.retention.reserve(71).unwrap();
    let unknown = inspect(&o, &mut r);
    let read = inspect_read(&o, &unknown, &mut r);
    assert!(!read.coverage.all_owner_obligations);
    assert_eq!(read.funding.unrepresented_parent_bytes, 71);
    assert_eq!(
        read.funding.parent_reserved_bytes,
        o.retention_usage().reserved
    );
    assert_eq!(
        inspection_current(&o, &original, &mut r),
        Err(CoordinatorError::Stale)
    );
    o.retention.release(&mut unregistered).unwrap();
    assert_eq!(inspection_current(&o, &original, &mut r), Ok(()));
}
#[test]
fn shutdown_hold_lineage_workspace_obeys_work_and_byte_bounds_before_allocation() {
    let mut l = limits();
    l.max_work = 0;
    assert_eq!(
        Builder::new(l).unwrap().hold_bitmap(1),
        Err(CoordinatorError::Limit)
    );
    l.max_work = 8;
    l.max_capture_bytes = size_of::<ShutdownInventoryV1>() + size_of::<Cut>() + 2;
    let mut b = Builder::new(l).unwrap();
    assert_eq!(b.hold_bitmap(2).unwrap(), [0, 0]);
    assert_eq!(b.hold_bitmap(1), Err(CoordinatorError::Limit));
    assert_eq!(
        Builder::new(l).unwrap().hold_bitmap(usize::MAX),
        Err(CoordinatorError::Limit)
    );
}
fn setup() -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    crate::resource::ProviderPort,
    SaveId,
) {
    let (mut o, mut r, d, p) = resource::source_save_test_fixture();
    let id = o
        .queue_pinned_save(
            &d,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            &mut r,
        )
        .unwrap();
    (o, r, d, p, id)
}
#[test]
fn native_managed_owner_roles_are_reported_even_when_unconfigured_not_inferred_closed() {
    let mut o = SourceCoordinator::new(CoordinatorLimits {
        max_document_incarnations: 1,
        max_participants: 1,
        max_staged_bytes: 64,
        source: crate::source::Limits {
            max_source_bytes: 64,
            max_edits: 1,
            max_edit_payload_bytes: 64,
        },
    });
    let mut r = Recorder::disabled();
    o.create(b"prose", &mut r).unwrap();
    let cut = crate::coordinator::shutdown::tests::inspect(&o, &mut r);
    let read = crate::coordinator::shutdown::tests::inspect_read(&o, &cut, &mut r);
    for role in [
        ShutdownOwnerRoleV1::SourceContexts,
        ShutdownOwnerRoleV1::AnnotationPreparations,
        ShutdownOwnerRoleV1::AnnotationContexts,
        ShutdownOwnerRoleV1::VersionSetPreparations,
        ShutdownOwnerRoleV1::VersionSetContexts,
        ShutdownOwnerRoleV1::ManagedSearch,
        ShutdownOwnerRoleV1::SettingsRetention,
        ShutdownOwnerRoleV1::SettingsReader,
        ShutdownOwnerRoleV1::ResourceCatalogReads,
        ShutdownOwnerRoleV1::SerializationBases,
        ShutdownOwnerRoleV1::OperationObservers,
        ShutdownOwnerRoleV1::SourcePublication,
        ShutdownOwnerRoleV1::FacetPublication,
        ShutdownOwnerRoleV1::CollectionPublication,
        ShutdownOwnerRoleV1::RegistryObservation,
        ShutdownOwnerRoleV1::RegistrationBases,
        ShutdownOwnerRoleV1::Discovery,
        ShutdownOwnerRoleV1::HistoryInspection,
        ShutdownOwnerRoleV1::PublicationMeasurements,
        ShutdownOwnerRoleV1::ReleaseMeasurements,
        ShutdownOwnerRoleV1::InvocationMeasurements,
        ShutdownOwnerRoleV1::ShutdownControl,
        ShutdownOwnerRoleV1::History,
        ShutdownOwnerRoleV1::SearchSessions,
        ShutdownOwnerRoleV1::Annotations,
        ShutdownOwnerRoleV1::ResourceMetadata,
        ShutdownOwnerRoleV1::OwnedSaves,
        ShutdownOwnerRoleV1::OwnedSourceReads,
        ShutdownOwnerRoleV1::OwnedAnnotationReads,
        ShutdownOwnerRoleV1::SourceContinuity,
        ShutdownOwnerRoleV1::LogicalViews,
        ShutdownOwnerRoleV1::Workspaces,
        ShutdownOwnerRoleV1::BoundaryEvidence,
        ShutdownOwnerRoleV1::SettingsPreparations,
        ShutdownOwnerRoleV1::InvocationObservers,
    ] {
        let rows: Vec<_> = read
            .owners
            .iter()
            .filter(|row| row.owner.role() == role)
            .collect();
        assert_eq!(rows.len(), 1, "{role:?}");
        assert_eq!(rows[0].state, ShutdownOwnerStateV1::Unconfigured);
        assert_eq!(rows[0].reserved_bytes, None);
    }
    assert!(read.coverage.all_owner_obligations);
    let invocations = read
        .owners
        .iter()
        .find(|row| row.owner.role() == ShutdownOwnerRoleV1::Invocations)
        .unwrap();
    assert_eq!(invocations.state, ShutdownOwnerStateV1::Active);
    assert_eq!(invocations.retained_records, Some(0));
    assert_eq!(invocations.reserved_bytes, None);
}

#[test]
fn managed_owner_rows_are_charged_against_the_callers_actual_record_limit() {
    let (o, mut r, _, _, _) = setup();
    let c = context();
    let control = control_facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let before = (o.generation(), o.resource_usage(), o.retention_usage());
    let mut short = limits();
    short.max_owners = 40;
    assert!(matches!(
        o.capture_shutdown_inventory_v1(&basis, short, &f, &mut r),
        Err(CoordinatorError::Limit)
    ));
    short.max_owners = 41;
    let exact = o
        .capture_shutdown_inventory_v1(&basis, short, &f, &mut r)
        .unwrap();
    assert_eq!(
        o.read_shutdown_inventory_v1(&exact, &f, &mut r)
            .unwrap()
            .owners
            .len(),
        41
    );
    assert_eq!(
        (o.generation(), o.resource_usage(), o.retention_usage()),
        before
    );
}

#[test]
fn actual_inventory_preserves_original_documents_writers_artifacts_and_required_edges() {
    let (o, mut r, d, _, id) = setup();
    let c = context();
    let control = control_facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let before = o.resource_usage();
    let snapshot = o
        .capture_shutdown_inventory_v1(&basis, limits(), &f, &mut r)
        .unwrap();
    let read = o.read_shutdown_inventory_v1(&snapshot, &f, &mut r).unwrap();
    assert_eq!(read.documents.len(), 2);
    assert!(read.documents.iter().any(|row| row.source == d));
    assert_eq!(read.writes.len(), 1);
    assert_eq!(read.writes[0].write, id);
    assert_eq!(
        read.writes[0].baseline_capacity,
        Some(BaselineCapacityState::Pending)
    );
    assert_eq!(read.artifacts.len(), 1);
    assert_eq!(read.artifacts[0].artifact, read.writes[0].artifact);
    assert_eq!(read.dependencies.len(), 7);
    assert!(read.dependencies.iter().any(|e| e.dependent
        == ShutdownInventoryTargetV1::Write(id.clone())
        && e.required == ShutdownInventoryTargetV1::Artifact(read.artifacts[0].artifact.clone())));
    assert!(read.coverage.all_owner_obligations);
    o.validate_shutdown_inventory_current_v1(&snapshot, &f, &mut r)
        .unwrap();
    assert_eq!(o.resource_usage(), before);
    assert!(!format!("{snapshot:?}").contains("key"));
}
#[test]
fn nonpublication_progress_invalidates_currentness_but_cannot_rewrite_the_historical_cut() {
    let (mut o, mut r, _, p, id) = setup();
    let c = context();
    let control = control_facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let dispatch = o
        .dispatch_save(&id, p.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
        .unwrap();
    let basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let snapshot = o
        .capture_shutdown_inventory_v1(&basis, limits(), &f, &mut r)
        .unwrap();
    let root = o.native_root_identity().unwrap();
    let e = p
        .result(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            Some(b"v1"),
        )
        .unwrap();
    o.record_save_evidence(e, &mut r).unwrap();
    assert_eq!(o.native_root_identity().unwrap(), root);
    assert_eq!(
        o.validate_shutdown_inventory_current_v1(&snapshot, &f, &mut r),
        Err(CoordinatorError::Stale)
    );
    assert_eq!(
        o.read_shutdown_inventory_v1(&snapshot, &f, &mut r)
            .unwrap()
            .writes[0]
            .application,
        Application::Unknown
    );
    let fresh = o
        .capture_shutdown_inventory_v1(&basis, limits(), &f, &mut r)
        .unwrap();
    assert_eq!(
        o.read_shutdown_inventory_v1(&fresh, &f, &mut r)
            .unwrap()
            .writes[0]
            .application,
        Application::Written
    );
}
#[test]
fn exact_capture_bounds_and_separate_disclosure_permission_precede_any_state_change() {
    let (o, mut r, _, _, _) = setup();
    let c = context();
    let control = control_facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let bytes = size_of::<ShutdownInventoryV1>()
        + size_of::<Cut>()
        + 2 * size_of::<ShutdownDocumentRecordV1>()
        + size_of::<ShutdownWriteRecordV1>()
        + size_of::<ShutdownArtifactRecordV1>()
        + size_of::<ShutdownBindingRecordV1>()
        + 3
        // All forty-one primary native owners, including
        // unconfigured stores: each is an actual retained report row.
        + 41 * size_of::<ShutdownOwnerRecordV1>()
        + 7 * size_of::<ShutdownDependencyV1>();
    let before = o.resource_usage();
    for delta in [-1isize, 0, 1] {
        let mut l = limits();
        l.max_capture_bytes = bytes.checked_add_signed(delta).unwrap();
        assert_eq!(
            o.capture_shutdown_inventory_v1(&basis, l, &f, &mut r)
                .is_ok(),
            delta >= 0
        );
    }
    for index in 0..7 {
        let mut l = limits();
        match index {
            0 => l.max_documents = 0,
            1 => l.max_writes = 0,
            2 => l.max_artifacts = 0,
            3 => l.max_dependencies = 2,
            4 => l.max_bindings = 0,
            5 => l.max_owners = 2,
            _ => l.max_work = 5,
        };
        assert!(matches!(
            o.capture_shutdown_inventory_v1(&basis, l, &f, &mut r),
            Err(CoordinatorError::Limit)
        ));
    }
    for allowed in [None, Some(false)] {
        let denied = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: allowed,
        };
        assert!(matches!(
            o.capture_shutdown_inventory_v1(&basis, limits(), &denied, &mut r),
            Err(CoordinatorError::Unavailable)
        ));
    }
    assert_eq!(o.resource_usage(), before);
    assert!(o.shutdown.fence.is_none());
}
#[test]
fn technical_delivery_population_is_actual_and_does_not_become_effect_completion() {
    let (mut o, mut r, _, p, id) = setup();
    let c = context();
    let control = control_facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let dispatch = o
        .dispatch_save(&id, p.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
        .unwrap();
    let basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let cut = o
        .capture_shutdown_inventory_v1(&basis, limits(), &f, &mut r)
        .unwrap();
    let before = o
        .read_shutdown_inventory_v1(&cut, &f, &mut r)
        .unwrap()
        .writes[0]
        .dispatch_evidence_references;
    let delivery = p.no_future_effects(&dispatch).unwrap();
    assert_eq!(
        o.validate_shutdown_inventory_current_v1(&cut, &f, &mut r),
        Err(CoordinatorError::Stale)
    );
    let fresh = o
        .capture_shutdown_inventory_v1(&basis, limits(), &f, &mut r)
        .unwrap();
    let read = o.read_shutdown_inventory_v1(&fresh, &f, &mut r).unwrap();
    assert_eq!(read.writes[0].dispatch_evidence_references, before + 1);
    assert_eq!(read.writes[0].application, Application::Unknown);
    assert!(!read.writes[0].no_future_effects);
    assert!(!read.writes[0].quiescent);
    drop(delivery);
    o.validate_shutdown_inventory_current_v1(&cut, &f, &mut r)
        .unwrap();
}
#[test]
fn inventory_off_timed_and_saturated_capture_preserve_actual_state_and_projection() {
    for capacity in [0, 1, 128] {
        let (o, mut setup, _, _, _) = setup();
        let c = context();
        let control = control_facts(&c);
        let f = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(true),
        };
        let basis = o
            .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut setup)
            .unwrap();
        let before = o.resource_usage();
        let mut r = Recorder::try_with_capacity(capacity)
            .unwrap()
            .with_native_timing(crate::instrumentation::NativeTimingProfileV1::ProcessMonotonicV1);
        let cut = o
            .capture_shutdown_inventory_v1(&basis, limits(), &f, &mut r)
            .unwrap();
        o.validate_shutdown_inventory_current_v1(&cut, &f, &mut r)
            .unwrap();
        assert_eq!(
            o.read_shutdown_inventory_v1(&cut, &f, &mut r)
                .unwrap()
                .writes[0]
                .application,
            Application::Unknown
        );
        assert_eq!(o.resource_usage(), before);
        assert!(o.shutdown.fence.is_none());
        if capacity == 1 {
            assert!(r.dropped() > 0);
        }
        if capacity == 128 {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            for boundary in [
                Boundary::ShutdownInventoryCapture,
                Boundary::ShutdownInventoryValidation,
                Boundary::ShutdownInventoryRelease,
            ] {
                assert!(
                    events
                        .iter()
                        .any(|e| e.boundary == boundary && e.service_time.is_some())
                );
            }
        }
    }
}
