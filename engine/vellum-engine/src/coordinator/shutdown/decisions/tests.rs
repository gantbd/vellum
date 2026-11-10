use super::*;
use crate::coordinator::shutdown::tests::{context, facts, limits as fence_limits};
fn limits() -> ShutdownInventoryLimitsV1 {
    ShutdownInventoryLimitsV1 {
        max_documents: 16,
        max_writes: 8,
        max_artifacts: 8,
        max_bindings: 16,
        max_reads: 16,
        max_owners: 64,
        max_attachments: 8,
        max_dependencies: 32,
        max_work: 128,
        max_capture_bytes: 16384,
    }
}
#[test]
fn missing_independent_choices_need_decision_without_fencing_saving_or_discarding() {
    let (mut o, mut r, d, _) = resource::source_save_test_fixture();
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let cut = o
        .capture_shutdown_inventory_v1(&basis, limits(), &f, &mut r)
        .unwrap();
    let before = o.resource_usage();
    let rows = o.read_shutdown_inventory_v1(&cut, &f, &mut r).unwrap();
    let input: Vec<_> = rows
        .documents
        .iter()
        .map(|row| ShutdownDocumentDecisionV1 {
            expected: row,
            source: Some(ShutdownDocumentChoiceV1::Save),
            annotations: if row.source == d {
                None
            } else {
                Some(ShutdownDocumentChoiceV1::Discard)
            },
        })
        .collect();
    let ShutdownDecisionCaptureV1::NeedsDecision(needs) = o
        .capture_shutdown_decisions_v1(&cut, &input, 16384, &f, &mut r)
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(
        needs,
        vec![ShutdownDecisionNeedV1 {
            document: d.document.clone(),
            source: false,
            annotations: true
        }]
    );
    assert_eq!(o.resource_usage(), before);
    assert!(o.shutdown.fence.is_none());
    assert_eq!(o.read(&d.document, &mut r).unwrap().version, d);
    o.create(b"unrelated", &mut r).unwrap();
    assert!(matches!(
        o.capture_shutdown_decisions_v1(&cut, &input, 16384, &f, &mut r),
        Err(CoordinatorError::Stale)
    ));
}
#[test]
fn captured_choices_bind_exact_original_cut_and_become_stale_after_policy_or_source_change() {
    let (mut o, mut r, d, _) = resource::source_save_test_fixture();
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let cut = o
        .capture_shutdown_inventory_v1(&basis, limits(), &f, &mut r)
        .unwrap();
    let equivalent = o
        .capture_shutdown_inventory_v1(&basis, limits(), &f, &mut r)
        .unwrap();
    let rows = o.read_shutdown_inventory_v1(&cut, &f, &mut r).unwrap();
    let input: Vec<_> = rows
        .documents
        .iter()
        .map(|row| ShutdownDocumentDecisionV1 {
            expected: row,
            source: Some(ShutdownDocumentChoiceV1::Save),
            annotations: Some(ShutdownDocumentChoiceV1::Discard),
        })
        .collect();
    let ShutdownDecisionCaptureV1::Captured(choices) = o
        .capture_shutdown_decisions_v1(&cut, &input, 16384, &f, &mut r)
        .unwrap()
    else {
        panic!()
    };
    assert!(choices.decisions.iter().any(|v| v.expected.source == d));
    assert_eq!(choices.decisions[0].source, ShutdownDocumentChoiceV1::Save);
    o.validate_shutdown_decisions_current_v1(&cut, &choices, &f, &mut r)
        .unwrap();
    assert_eq!(
        o.validate_shutdown_decisions_current_v1(&equivalent, &choices, &f, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    o.set_read_only(&d.document, true, &mut r).unwrap();
    assert_eq!(
        o.validate_shutdown_decisions_current_v1(&cut, &choices, &f, &mut r),
        Err(CoordinatorError::Stale)
    );
    let fresh_basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let fresh_cut = o
        .capture_shutdown_inventory_v1(&fresh_basis, limits(), &f, &mut r)
        .unwrap();
    assert!(matches!(
        o.capture_shutdown_decisions_v1(&fresh_cut, &input, 16384, &f, &mut r),
        Err(CoordinatorError::Stale)
    ));
    assert!(o.shutdown.fence.is_none());
    assert_eq!(o.resource_usage().issued_lifetime, 0);
}
#[test]
fn input_order_is_not_ownership_and_duplicate_foreign_or_undersized_choices_refuse() {
    let (mut o, mut r, d, _) = resource::source_save_test_fixture();
    let peer = o.create(b"peer", &mut r).unwrap();
    let (other, _, foreign, _) = resource::source_save_test_fixture();
    drop(other);
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let cut = o
        .capture_shutdown_inventory_v1(&basis, limits(), &f, &mut r)
        .unwrap();
    let rows = o.read_shutdown_inventory_v1(&cut, &f, &mut r).unwrap();
    let original = rows.documents.iter().find(|row| row.source == d).unwrap();
    let mut foreign_expectation = original.clone();
    foreign_expectation.source = foreign;
    let make = |expected| ShutdownDocumentDecisionV1 {
        expected,
        source: Some(ShutdownDocumentChoiceV1::Discard),
        annotations: Some(ShutdownDocumentChoiceV1::Save),
    };
    for input in [
        [make(original), make(original)],
        [make(original), make(&foreign_expectation)],
    ] {
        assert!(matches!(
            o.capture_shutdown_decisions_v1(&cut, &input, 16384, &f, &mut r),
            Err(CoordinatorError::Unavailable)
        ));
    }
    let rows = o.read_shutdown_inventory_v1(&cut, &f, &mut r).unwrap();
    let input: Vec<_> = rows.documents.iter().rev().map(make).collect();
    assert!(matches!(
        o.capture_shutdown_decisions_v1(&cut, &input, 0, &f, &mut r),
        Err(CoordinatorError::Limit)
    ));
    let ShutdownDecisionCaptureV1::Captured(chosen) = o
        .capture_shutdown_decisions_v1(&cut, &input, 16384, &f, &mut r)
        .unwrap()
    else {
        panic!()
    };
    assert!(chosen.decisions.iter().any(|v| v.expected.source == d));
    assert!(chosen.decisions.iter().any(|v| v.expected.source == peer));
    assert!(
        chosen
            .decisions
            .windows(2)
            .all(|w| w[0].expected.source.document.slot < w[1].expected.source.document.slot)
    );
}
#[test]
fn scratch_prose_needs_no_filesystem_or_writer_to_inspect_and_request_decisions() {
    let (template, _, _, _) = resource::source_save_test_fixture();
    let mut o = SourceCoordinator::new(template.limits);
    let mut r = Recorder::disabled();
    let original = "English notes\n\n## Deutsch\nMeine Notizen.\n";
    let d = o.create(original.as_bytes(), &mut r).unwrap();
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let basis = o
        .capture_shutdown_fence_basis_v1(fence_limits(), &control, &mut r)
        .unwrap();
    let cut = o
        .capture_shutdown_inventory_v1(&basis, limits(), &f, &mut r)
        .unwrap();
    let read = o.read_shutdown_inventory_v1(&cut, &f, &mut r).unwrap();
    assert_eq!(read.documents.len(), 1);
    assert!(read.writes.is_empty());
    assert!(read.artifacts.is_empty());
    let ShutdownDecisionCaptureV1::NeedsDecision(needs) = o
        .capture_shutdown_decisions_v1(&cut, &[], 16384, &f, &mut r)
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(
        needs,
        vec![ShutdownDecisionNeedV1 {
            document: d.document.clone(),
            source: true,
            annotations: true
        }]
    );
    assert_eq!(o.read(&d.document, &mut r).unwrap().source.text(), original);
    assert!(o.shutdown.fence.is_none());
}
