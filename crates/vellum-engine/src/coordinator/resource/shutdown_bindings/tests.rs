use super::*;
use crate::coordinator::{
    ShutdownFenceLimitsV1, ShutdownInventoryLimitsV1, ShutdownInventoryReadV1, ShutdownInventoryV1,
    TrustedShutdownFenceFactsV1, TrustedShutdownInventoryFactsV1,
};
fn context() -> SettingsExecutionContextV1 {
    crate::coordinator::serialization::tests::context()
}
fn facts(c: &SettingsExecutionContextV1) -> TrustedShutdownFenceFactsV1<'_> {
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
pub(in crate::coordinator::resource) fn capture(
    o: &SourceCoordinator,
    r: &mut Recorder,
) -> ShutdownInventoryV1 {
    let c = context();
    let f = facts(&c);
    let basis = o
        .capture_shutdown_fence_basis_v1(
            ShutdownFenceLimitsV1 {
                max_context_bytes: 1024,
                max_root_bytes: 65536,
                max_capture_bytes: 131072,
            },
            &f,
            r,
        )
        .unwrap();
    o.capture_shutdown_inventory_v1(
        &basis,
        ShutdownInventoryLimitsV1 {
            max_documents: 16,
            max_writes: 16,
            max_artifacts: 16,
            max_bindings: 16,
            max_reads: 16,
            max_owners: 64,
            max_attachments: 8,
            max_dependencies: 128,
            max_work: 512,
            max_capture_bytes: 65536,
        },
        &TrustedShutdownInventoryFactsV1 {
            control: &f,
            inventory_readable: Some(true),
        },
        r,
    )
    .unwrap()
}
pub(in crate::coordinator::resource) fn fence(o: &mut SourceCoordinator, r: &mut Recorder) {
    let c = context();
    let f = facts(&c);
    let basis = o
        .capture_shutdown_fence_basis_v1(
            ShutdownFenceLimitsV1 {
                max_context_bytes: 1024,
                max_root_bytes: 65536,
                max_capture_bytes: 131072,
            },
            &f,
            r,
        )
        .unwrap();
    o.install_shutdown_fence_v1(&basis, &f, r).unwrap();
}
pub(in crate::coordinator::resource) fn read<'a>(
    o: &SourceCoordinator,
    cut: &'a ShutdownInventoryV1,
    r: &mut Recorder,
) -> ShutdownInventoryReadV1<'a> {
    let c = context();
    let f = facts(&c);
    o.read_shutdown_inventory_v1(
        cut,
        &TrustedShutdownInventoryFactsV1 {
            control: &f,
            inventory_readable: Some(true),
        },
        r,
    )
    .unwrap()
}
fn written() -> (
    SourceCoordinator,
    Recorder,
    DocumentVersion,
    ProviderPort,
    SaveId,
) {
    let (mut o, mut r, d, p) = super::super::serialization_tests::fixture();
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
    let dispatch = o
        .dispatch_save(&id, p.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
        .unwrap();
    for e in [
        p.result(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            Some(b"v1"),
        )
        .unwrap(),
        p.no_future_effects(&dispatch).unwrap(),
        p.quiescence(&dispatch).unwrap(),
    ] {
        o.record_save_evidence(e, &mut r).unwrap();
    }
    drop(dispatch);
    o.publish_save_baseline(&id, &mut r).unwrap();
    (o, r, d, p, id)
}
#[test]
fn disposal_and_binding_transfer_keep_the_original_producer_obligation_visible() {
    let (mut o, mut r, d, _, id) = written();
    let initial = capture(&o, &mut r);
    let row = &read(&o, &initial, &mut r).bindings[0];
    assert_eq!(row.baseline, Origin::Write(id.clone()));
    let original = row.binding.clone();
    let p = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
    let state = o.commit_retained_disposal(p, &mut r).unwrap();
    o.release_disposed_source_binding(&d.document, state, &mut r)
        .unwrap();
    let retained = capture(&o, &mut r);
    let view = read(&o, &retained, &mut r);
    let row = view
        .bindings
        .iter()
        .find(|b| b.document == d.document)
        .unwrap();
    assert_eq!(row.binding, original);
    assert_eq!(row.location, Location::DisposedRetained);
    assert!(
        view.dependencies
            .iter()
            .any(|e| e.dependent == Target::Binding(original.clone())
                && e.required == Target::Write(id.clone()))
    );
    assert_eq!(o.retire_save(&id, &mut r), Err(ResourceError::Retained));
    assert_eq!(
        read(&o, &initial, &mut r).bindings[0].location,
        Location::Active
    );
}
#[test]
fn archived_source_binding_keeps_its_own_destination_and_writer_instead_of_becoming_current() {
    let (mut o, mut r, d, p, id) = written();
    let target = p.source_binding_target(b"next").unwrap();
    o.rebind_source_document(
        &d.document,
        target,
        super::super::source_export::tests::rebind_options(Some(0), 1),
        &super::super::source_export::tests::rebind_facts(),
        &mut r,
    )
    .unwrap();
    let cut = capture(&o, &mut r);
    let view = read(&o, &cut, &mut r);
    let rows: Vec<_> = view
        .bindings
        .iter()
        .filter(|b| b.document == d.document)
        .collect();
    assert_eq!(rows.len(), 2);
    let old = rows
        .iter()
        .find(|b| b.location == Location::AssociationHistory)
        .unwrap();
    let current = rows
        .iter()
        .find(|b| b.location == Location::Active)
        .unwrap();
    assert_eq!(old.baseline, Origin::Write(id));
    assert_eq!(current.baseline, Origin::Absent);
    assert_ne!(old.destination, current.destination);
    assert_ne!(old.binding, current.binding);
}
#[test]
fn loaded_annotation_baseline_preserves_the_original_read_even_after_disposal() {
    let (mut o, id, package, mut r) = super::super::loaded_baseline::commit_tests::ready(0);
    let source = package.capture.source().clone();
    let plan = o
        .prepare_annotation_restoration_native(package, None, &mut r)
        .unwrap();
    o.commit_annotation_restoration_native(plan, &mut r)
        .unwrap();
    o.attach_annotation_read_baselines(&mut r).unwrap();
    o.adopt_loaded_annotation_baseline(&id, None, 16384, &mut r)
        .unwrap();
    let before = capture(&o, &mut r);
    let row = read(&o, &before, &mut r)
        .bindings
        .iter()
        .find(|b| b.kind == ArtifactKind::Annotations)
        .unwrap()
        .clone();
    assert_eq!(row.baseline, Origin::AnnotationRead(id.clone()));
    let p = o.prepare_retained_disposal(&source, 0, &mut r).unwrap();
    o.commit_retained_disposal(p, &mut r).unwrap();
    let after = capture(&o, &mut r);
    assert!(
        read(&o, &after, &mut r)
            .dependencies
            .iter()
            .any(|e| e.dependent == Target::Binding(row.binding.clone())
                && e.required == Target::AnnotationRead(id.clone()))
    );
}
#[test]
fn late_source_contradiction_keeps_the_installed_baseline_visible_as_contested() {
    let (mut o, mut r, d, p) = super::super::serialization_tests::fixture();
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
    let dispatch = o
        .dispatch_save(&id, p.read_result(b"key", b"v0", b"A").unwrap(), &mut r)
        .unwrap();
    o.record_save_evidence(
        p.result(
            &dispatch,
            Application::Written,
            1,
            Synchronization::NotRequested,
            Some(b"v1"),
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    o.publish_save_baseline(&id, &mut r).unwrap();
    o.record_save_evidence(
        p.result(
            &dispatch,
            Application::NotWritten,
            0,
            Synchronization::NotRequested,
            None,
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    let cut = capture(&o, &mut r);
    let view = read(&o, &cut, &mut r);
    assert_eq!(view.bindings[0].baseline, Origin::Write(id));
    assert!(view.bindings[0].contested_destination);
    assert!(view.writes[0].contested);
    assert!(!o.is_faulted());
}
