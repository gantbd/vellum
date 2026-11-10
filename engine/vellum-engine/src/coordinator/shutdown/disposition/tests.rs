use super::*;
#[test]
fn accepted_shutdown_cancellation_is_bounded_and_preserves_required_or_dispatched_saves() {
    use crate::instrumentation::NativeTimingProfileV1;
    use crate::resource::SavePhase;
    for mode in 0..4 {
        let (mut o, mut r, d, port) = resource::source_save_test_fixture();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let requirements = SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Ordinary,
        };
        let dispatched = o.queue_pinned_save(&d, requirements, &mut r).unwrap();
        let _dispatch = o
            .dispatch_save(
                &dispatched,
                port.read_result(b"key", b"v0", b"A").unwrap(),
                &mut r,
            )
            .unwrap();
        let required = o.queue_pinned_save(&d, requirements, &mut r).unwrap();
        let replaceable = o.queue_pinned_save(&d, requirements, &mut r).unwrap();
        let pending = o.issue_invocation(&mut r).unwrap();
        o.submit(
            &pending,
            &SourceIntent {
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
            },
            &mut r,
        )
        .unwrap();
        let _unsubmitted = o.issue_invocation(&mut r).unwrap();
        let c = context();
        let control = facts(&c);
        let f = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(true),
        };
        let (_, inventory, _) = capture(&o, &f, ShutdownDocumentChoiceV1::Discard, &mut r);
        let rows = o
            .read_shutdown_inventory_v1(&inventory, &f, &mut r)
            .unwrap();
        let input: Vec<_> = rows
            .documents
            .iter()
            .map(|row| ShutdownDocumentDecisionV1 {
                expected: row,
                source: Some(if row.source.document == d.document {
                    ShutdownDocumentChoiceV1::Save
                } else {
                    ShutdownDocumentChoiceV1::Discard
                }),
                annotations: Some(ShutdownDocumentChoiceV1::Discard),
            })
            .collect();
        let ShutdownDecisionCaptureV1::Captured(decisions) = o
            .capture_shutdown_decisions_v1(&inventory, &input, 65536, &f, &mut r)
            .unwrap()
        else {
            panic!()
        };
        let bindings: Vec<_> = decisions
            .decisions
            .iter()
            .map(|row| ShutdownSaveBindingsV1 {
                document: row.expected.source.document.clone(),
                source: (row.expected.source.document == d.document).then(|| required.clone()),
                annotations: None,
            })
            .collect();
        let ShutdownAcceptanceV1::Accepted(fence) = o
            .accept_shutdown_disposition_v1(&inventory, &decisions, &bindings, 262144, &f, &mut r)
            .unwrap()
        else {
            panic!()
        };
        r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        let before = (o.generation(), o.retention_usage());
        let mut scan = o
            .begin_shutdown_cancellation_v1(&fence, &control, &mut r)
            .unwrap();
        let initial = scan.progress();
        assert_eq!(
            o.advance_shutdown_cancellation_v1(&mut scan, 0, &control, &mut r),
            Err(CoordinatorError::Limit)
        );
        assert_eq!(scan.progress(), initial);
        let mut denied = facts(&c);
        denied.owner_control_allowed = None;
        assert_eq!(
            o.advance_shutdown_cancellation_v1(&mut scan, 1, &denied, &mut r),
            Err(CoordinatorError::Unavailable)
        );
        assert_eq!(scan.progress(), initial);
        for _ in 0..6 {
            let prior = scan.progress();
            let progress = o
                .advance_shutdown_cancellation_v1(&mut scan, 1, &control, &mut r)
                .unwrap();
            assert!(progress.scanned_slots <= prior.scanned_slots + 1);
            if progress.status == ShutdownCancellationStatusV1::FinishedScan {
                break;
            }
        }
        let progress = scan.progress();
        assert_eq!(progress.status, ShutdownCancellationStatusV1::FinishedScan);
        assert_eq!(
            (progress.stopped_invocations, progress.stopped_queued_saves),
            (1, 1)
        );
        assert_eq!(
            o.invocation_state(&pending, &mut r),
            Ok(InvocationState::Finished)
        );
        assert_eq!(
            o.save_outcome(&required, &mut r).unwrap().phase,
            SavePhase::Queued
        );
        assert_eq!(
            o.save_outcome(&replaceable, &mut r).unwrap().phase,
            SavePhase::Stopped
        );
        let external = o.save_outcome(&dispatched, &mut r).unwrap();
        assert_eq!(external.phase, SavePhase::Dispatched);
        assert!(!external.stop_requested && !external.no_future_effects && !external.quiescent);
        assert_eq!(
            o.advance_shutdown_cancellation_v1(&mut scan, 1, &control, &mut r)
                .unwrap(),
            progress
        );
        assert_eq!((o.generation(), o.retention_usage()), before);
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if mode == 0 {
            assert!(events.is_empty());
        }
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
        if mode == 1 || mode == 2 {
            for boundary in [
                Boundary::ShutdownCancellationStart,
                Boundary::ShutdownCancellationStep,
            ] {
                assert!(events.iter().any(|e| e.boundary == boundary
                    && e.disposition == Disposition::Completed
                    && e.service_time.is_some() == (mode == 2)));
            }
        }
    }
}
#[test]
fn shutdown_acceptance_refuses_stopped_or_conflicting_allocations_before_fencing() {
    for conflicting in [false, true] {
        let (mut o, mut r, d, _) = resource::source_save_test_fixture();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let requirements = SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Ordinary,
        };
        let old = o.queue_pinned_save(&d, requirements, &mut r).unwrap();
        let selected = if conflicting {
            let p = o
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
            let current = o.commit(p, &mut r).unwrap().changes.remove(0).after;
            o.queue_pinned_save(&current, requirements, &mut r).unwrap()
        } else {
            o.stop_save(&old, &mut r).unwrap();
            old
        };
        let c = context();
        let control = facts(&c);
        let f = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(true),
        };
        let (_, inventory, decisions) = capture(&o, &f, ShutdownDocumentChoiceV1::Save, &mut r);
        let bindings: Vec<_> = decisions
            .decisions
            .iter()
            .map(|c| ShutdownSaveBindingsV1 {
                document: c.expected.source.document.clone(),
                source: (c.expected.source.document == d.document).then(|| selected.clone()),
                annotations: None,
            })
            .collect();
        let before = (o.generation(), o.resource_usage(), o.retention_usage());
        assert!(
            matches!(o.accept_shutdown_disposition_v1(&inventory,&decisions,&bindings,262144,&f,&mut r),Ok(ShutdownAcceptanceV1::NeedsSave(n)) if n.iter().any(|n|n.document==d.document&&n.source))
        );
        assert!(o.shutdown.fence.is_none());
        assert_eq!(
            (o.generation(), o.resource_usage(), o.retention_usage()),
            before
        );
    }
}
#[test]
fn shutdown_acceptance_missing_save_keeps_ordinary_admission_open() {
    let (mut o, mut r, d, _) = resource::source_save_test_fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let (_, inventory, decisions) = capture(&o, &f, ShutdownDocumentChoiceV1::Save, &mut r);
    let bindings: Vec<_> = decisions
        .decisions
        .iter()
        .map(|c| ShutdownSaveBindingsV1 {
            document: c.expected.source.document.clone(),
            source: None,
            annotations: None,
        })
        .collect();
    let before = (o.generation(), o.resource_usage(), o.retention_usage());
    assert!(
        matches!(o.accept_shutdown_disposition_v1(&inventory,&decisions,&bindings,262144,&f,&mut r),Ok(ShutdownAcceptanceV1::NeedsSave(n)) if !n.is_empty()&&n.iter().all(|n|n.source&&!n.annotations))
    );
    assert!(o.shutdown.fence.is_none());
    assert_eq!(
        (o.generation(), o.resource_usage(), o.retention_usage()),
        before
    );
    o.queue_pinned_save(
        &d,
        SaveRequirements {
            protection: Protection::BestEffort,
            acknowledgment: Acknowledgment::Ordinary,
        },
        &mut r,
    )
    .unwrap();
}

#[test]
fn shutdown_acceptance_is_funded_once_and_keeps_the_original_plan_after_host_handles_drop() {
    use crate::instrumentation::NativeTimingProfileV1;
    for mode in 0..4 {
        let (mut o, mut r, _, _) = resource::source_save_test_fixture();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let c = context();
        let control = facts(&c);
        let f = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(true),
        };
        let (_, inventory, decisions) = capture(&o, &f, ShutdownDocumentChoiceV1::Discard, &mut r);
        let rows = o
            .read_shutdown_inventory_v1(&inventory, &f, &mut r)
            .unwrap();
        let input: Vec<_> = rows
            .documents
            .iter()
            .map(|d| ShutdownDocumentDecisionV1 {
                expected: d,
                source: Some(ShutdownDocumentChoiceV1::Discard),
                annotations: Some(ShutdownDocumentChoiceV1::Discard),
            })
            .collect();
        let ShutdownDecisionCaptureV1::Captured(other) = o
            .capture_shutdown_decisions_v1(&inventory, &input, 65536, &f, &mut r)
            .unwrap()
        else {
            panic!()
        };
        let bindings: Vec<_> = decisions
            .decisions
            .iter()
            .map(|c| ShutdownSaveBindingsV1 {
                document: c.expected.source.document.clone(),
                source: None,
                annotations: None,
            })
            .collect();
        let before = o.retention_usage();
        r = if mode == 0 {
            Recorder::disabled()
        } else {
            Recorder::try_with_capacity(if mode == 3 { 1 } else { 128 }).unwrap()
        };
        if mode >= 2 {
            r = r.with_native_timing(NativeTimingProfileV1::ProcessMonotonicV1);
        }
        assert!(matches!(
            o.accept_shutdown_disposition_v1(&inventory, &decisions, &bindings, 0, &f, &mut r),
            Err(CoordinatorError::Limit)
        ));
        assert_eq!(o.retention_usage(), before);
        assert!(o.shutdown.fence.is_none());
        let ShutdownAcceptanceV1::Accepted(fence) = o
            .accept_shutdown_disposition_v1(&inventory, &decisions, &bindings, 262144, &f, &mut r)
            .unwrap()
        else {
            panic!()
        };
        let held = o.retention_usage();
        assert!(held.reserved > before.reserved);
        assert_eq!(
            o.retire_shutdown_disposition_v1(&fence, &control, Some(true), Some(true), &mut r),
            Err(CoordinatorError::Unavailable)
        );
        assert!(
            matches!(o.accept_shutdown_disposition_v1(&inventory,&decisions,&bindings,262144,&f,&mut r),Ok(ShutdownAcceptanceV1::Accepted(id)) if id==fence)
        );
        assert!(matches!(
            o.accept_shutdown_disposition_v1(&inventory, &other, &bindings, 262144, &f, &mut r),
            Err(CoordinatorError::Unavailable)
        ));
        assert!(matches!(
            o.discard_shutdown_document_v1(
                &inventory,
                &other,
                &bindings[0].document,
                &f,
                &coverage(),
                &mut r
            ),
            Err(CoordinatorError::Unavailable)
        ));
        assert_eq!(o.retention_usage(), held);
        let documents: Vec<_> = bindings.iter().map(|b| b.document.clone()).collect();
        drop(inventory);
        drop(decisions);
        drop(other);
        drop(bindings);
        for document in &documents {
            assert_eq!(
                o.dispose_accepted_shutdown_document_v1(&fence, document, &f, &coverage(), &mut r)
                    .unwrap()
                    .phase,
                LifecyclePhase::DisposedRetained
            );
        }
        assert_eq!(o.retention_usage(), held);
        for (released, drained) in [
            (None, Some(true)),
            (Some(true), None),
            (Some(false), Some(true)),
        ] {
            assert_eq!(
                o.retire_shutdown_disposition_v1(&fence, &control, released, drained, &mut r),
                Err(CoordinatorError::Unavailable)
            );
            assert_eq!(o.retention_usage(), held);
        }
        let mut denied = facts(&c);
        denied.owner_control_allowed = None;
        assert_eq!(
            o.retire_shutdown_disposition_v1(&fence, &denied, Some(true), Some(true), &mut r),
            Err(CoordinatorError::Unavailable)
        );
        assert_eq!(o.retention_usage(), held);
        let generation = o.generation();
        o.retire_shutdown_disposition_v1(&fence, &control, Some(true), Some(true), &mut r)
            .unwrap();
        let released = o.retention_usage();
        assert_eq!(
            released.reserved,
            before.reserved + limits().required_capture_bytes().unwrap()
        );
        assert_eq!(o.generation(), generation);
        assert_eq!(
            o.retire_shutdown_disposition_v1(&fence, &control, Some(true), Some(true), &mut r),
            Err(CoordinatorError::Unavailable)
        );
        assert_eq!(
            o.dispose_accepted_shutdown_document_v1(&fence, &documents[0], &f, &coverage(), &mut r),
            Err(CoordinatorError::Unavailable)
        );
        assert_eq!(o.retention_usage(), released);
        assert!(o.create(b"still fenced", &mut r).is_err());
        assert!(!o.is_faulted());
        let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
        if mode == 0 {
            assert!(events.is_empty());
        }
        if mode == 3 {
            assert!(r.dropped() > 0);
        }
        if mode == 1 || mode == 2 {
            for boundary in [
                Boundary::ShutdownDispositionAcceptance,
                Boundary::ShutdownAcceptedDocumentDisposition,
                Boundary::ShutdownDispositionRetirement,
            ] {
                assert!(events.iter().any(|e| e.boundary == boundary
                    && e.disposition == Disposition::Completed
                    && e.service_time.is_some() == (mode == 2)));
            }
        }
    }
}
use crate::coordinator::shutdown::tests::{context, facts, limits};
use crate::resource::{Acknowledgment, Application, Protection, SaveRequirements, Synchronization};

fn coverage() -> TrustedDisposalCoverageV1 {
    TrustedDisposalCoverageV1 {
        not_stopped: Some(true),
        companions_ready: Some(true),
        versions_available: Some(true),
        reply_ready: Some(true),
        lifecycle_allowed: Some(true),
        coverage_complete: Some(true),
        owner_control_allowed: Some(true),
        source_retention_ready: None,
        source_discard_allowed: Some(true),
        annotation_retention_ready: None,
        annotation_discard_allowed: Some(true),
        retention_complete: Some(true),
        effects_retained: Some(true),
        history_retained: Some(true),
        snapshots_retained: Some(true),
        providers_retained: Some(true),
        views_detachable: Some(true),
        memberships_detachable: Some(true),
        observer_scopes_fenced: Some(true),
        binding_coverage_complete: Some(true),
    }
}
fn capture(
    o: &SourceCoordinator,
    f: &TrustedShutdownInventoryFactsV1<'_>,
    source: ShutdownDocumentChoiceV1,
    r: &mut Recorder,
) -> (
    ShutdownFenceBasisV1,
    ShutdownInventoryV1,
    ShutdownDecisionsV1,
) {
    capture_choices(o, f, (source, ShutdownDocumentChoiceV1::Discard), r)
}
fn capture_choices(
    o: &SourceCoordinator,
    f: &TrustedShutdownInventoryFactsV1<'_>,
    choices: (ShutdownDocumentChoiceV1, ShutdownDocumentChoiceV1),
    r: &mut Recorder,
) -> (
    ShutdownFenceBasisV1,
    ShutdownInventoryV1,
    ShutdownDecisionsV1,
) {
    let basis = o
        .capture_shutdown_fence_basis_v1(limits(), f.control, r)
        .unwrap();
    let inventory = o
        .capture_shutdown_inventory_v1(
            &basis,
            ShutdownInventoryLimitsV1 {
                max_documents: 16,
                max_writes: 16,
                max_artifacts: 16,
                max_bindings: 16,
                max_reads: 16,
                max_owners: 64,
                max_attachments: 16,
                max_dependencies: 64,
                max_work: 256,
                max_capture_bytes: 65536,
            },
            f,
            r,
        )
        .unwrap();
    let rows = o.read_shutdown_inventory_v1(&inventory, f, r).unwrap();
    let input: Vec<_> = rows
        .documents
        .iter()
        .map(|d| ShutdownDocumentDecisionV1 {
            expected: d,
            source: Some(choices.0),
            annotations: Some(choices.1),
        })
        .collect();
    let ShutdownDecisionCaptureV1::Captured(decisions) = o
        .capture_shutdown_decisions_v1(&inventory, &input, 65536, f, r)
        .unwrap()
    else {
        panic!("complete explicit choices")
    };
    (basis, inventory, decisions)
}

#[test]
fn original_ordinary_source_save_must_be_qualified_and_installed_before_save_disposition() {
    let (mut o, mut r, d, port) = resource::source_save_test_fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let save = o
        .queue_pinned_save(
            &d,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            &mut r,
        )
        .unwrap();
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let (basis, inventory, decisions) = capture(&o, &f, ShutdownDocumentChoiceV1::Save, &mut r);
    o.install_shutdown_fence_v1(&basis, &control, &mut r)
        .unwrap();
    let request = ShutdownDocumentDispositionRequestV1 {
        document: &d.document,
        source_save: Some(&save),
        annotation_save: None,
    };
    let mut authorized = coverage();
    authorized.source_retention_ready = Some(true);
    authorized.source_discard_allowed = None;
    assert!(
        o.dispose_shutdown_document_v1(&inventory, &decisions, &request, &f, &authorized, &mut r)
            .is_err()
    );
    let dispatch = o
        .dispatch_save(
            &save,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut r,
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
        &mut r,
    )
    .unwrap();
    assert!(
        o.dispose_shutdown_document_v1(&inventory, &decisions, &request, &f, &authorized, &mut r)
            .is_err()
    );
    o.publish_save_baseline(&save, &mut r).unwrap();
    let state = o
        .dispose_shutdown_document_v1(&inventory, &decisions, &request, &f, &authorized, &mut r)
        .unwrap();
    assert_eq!(state.phase, LifecyclePhase::DisposedRetained);
    // Save readiness does not fabricate worker/effect completion.
    let outcome = o.save_outcome(&save, &mut r).unwrap();
    assert!(!outcome.quiescent);
    assert!(!outcome.no_future_effects);
    assert_eq!(
        o.retire_save(&save, &mut r),
        Err(crate::resource::ResourceError::Retained)
    );
    assert!(!o.is_faulted());
}

#[test]
fn annotation_save_choice_uses_its_own_original_producer_and_installed_set() {
    let (mut o, mut r, save, port) = resource::annotation_save_test_fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let d = o.save_outcome(&save, &mut r).unwrap().captured;
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let (basis, inventory, decisions) = capture_choices(
        &o,
        &f,
        (
            ShutdownDocumentChoiceV1::Discard,
            ShutdownDocumentChoiceV1::Save,
        ),
        &mut r,
    );
    o.install_shutdown_fence_v1(&basis, &control, &mut r)
        .unwrap();
    let request = ShutdownDocumentDispositionRequestV1 {
        document: &d.document,
        source_save: None,
        annotation_save: Some(&save),
    };
    let mut authorized = coverage();
    authorized.annotation_retention_ready = Some(true);
    authorized.annotation_discard_allowed = None;
    let pair = o
        .ordinary_pair_check(&d.document, &mut r)
        .unwrap()
        .map(|p| {
            port.attest_ordinary_pair(&p, crate::resource::ResourceRelation::Distinct)
                .unwrap()
        });
    let dispatch = o
        .dispatch_save_with_pair(
            &save,
            port.read_result(b"comments", b"v0", b"{}").unwrap(),
            pair.expect("bound ordinary source/comment pair"),
            &mut r,
        )
        .unwrap();
    o.record_save_evidence(
        port.result(
            &dispatch,
            Application::Written,
            8,
            Synchronization::NotRequested,
            Some(b"v1"),
        )
        .unwrap(),
        &mut r,
    )
    .unwrap();
    assert!(
        o.dispose_shutdown_document_v1(&inventory, &decisions, &request, &f, &authorized, &mut r)
            .is_err()
    );
    let pair = o
        .ordinary_pair_check(&d.document, &mut r)
        .unwrap()
        .map(|p| {
            port.attest_ordinary_pair(&p, crate::resource::ResourceRelation::Distinct)
                .unwrap()
        });
    o.install_annotation_save_baseline(&save, pair, &mut r)
        .unwrap();
    assert_eq!(
        o.dispose_shutdown_document_v1(&inventory, &decisions, &request, &f, &authorized, &mut r)
            .unwrap()
            .phase,
        LifecyclePhase::DisposedRetained
    );
    assert!(!o.is_faulted());
}

#[test]
fn acknowledged_documents_dispose_sequentially_under_fence_without_reopening_or_refunding_it() {
    for capacity in [None, Some(1), Some(64)] {
        let (mut o, mut r, d, _) = resource::source_save_test_fixture();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let c = context();
        let control = facts(&c);
        let f = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(true),
        };
        let (basis, inventory, decisions) =
            capture(&o, &f, ShutdownDocumentChoiceV1::Discard, &mut r);
        let ordinary = o.prepare_retained_disposal(&d, 0, &mut r).unwrap();
        o.install_shutdown_fence_v1(&basis, &control, &mut r)
            .unwrap();
        let held = o.retention_usage();
        let resources = o.resource_usage();
        assert_eq!(
            o.commit_retained_disposal(ordinary, &mut r),
            Err(CoordinatorError::Unavailable)
        );
        r = capacity.map_or_else(Recorder::disabled, |n| {
            Recorder::try_with_capacity(n).unwrap()
        });
        for row in &decisions.decisions {
            let generation = o.generation();
            let state = o
                .discard_shutdown_document_v1(
                    &inventory,
                    &decisions,
                    &row.expected.source.document,
                    &f,
                    &coverage(),
                    &mut r,
                )
                .unwrap();
            assert_eq!(state.phase, LifecyclePhase::DisposedRetained);
            assert_eq!(state.generation, row.expected.lifecycle.generation + 1);
            assert_eq!(o.generation(), generation + 1);
            assert_eq!(o.retention_usage(), held);
            assert_eq!(o.resource_usage(), resources);
        }
        assert_eq!(o.lifecycle_usage().live, 0);
        assert_eq!(o.create(b"new", &mut r), Err(CoordinatorError::Unavailable));
        assert!(!o.is_faulted());
        if capacity == Some(64) {
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            assert_eq!(
                events
                    .iter()
                    .filter(|e| e.boundary == Boundary::ShutdownDocumentDisposition
                        && e.disposition == Disposition::Completed)
                    .count(),
                decisions.decisions.len()
            );
        } else if capacity == Some(1) {
            assert!(r.dropped() > 0);
        } else {
            assert_eq!(r.queued(), 0);
        }
    }
}

#[test]
fn save_choice_is_never_silently_converted_to_discard() {
    let (mut o, mut r, d, _) = resource::source_save_test_fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let (basis, inventory, decisions) = capture(&o, &f, ShutdownDocumentChoiceV1::Save, &mut r);
    o.install_shutdown_fence_v1(&basis, &control, &mut r)
        .unwrap();
    let before = o.native_root_identity().unwrap();
    assert_eq!(
        o.discard_shutdown_document_v1(
            &inventory,
            &decisions,
            &d.document,
            &f,
            &coverage(),
            &mut r
        ),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(o.native_root_identity().unwrap(), before);
    assert!(!o.is_faulted());
}

#[test]
fn original_cut_fence_and_current_separate_discard_authority_are_required() {
    let (mut o, mut r, d, _) = resource::source_save_test_fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let (basis, inventory, decisions) = capture(&o, &f, ShutdownDocumentChoiceV1::Discard, &mut r);
    let (other_basis, other_inventory, other_decisions) =
        capture(&o, &f, ShutdownDocumentChoiceV1::Discard, &mut r);
    assert_eq!(
        o.discard_shutdown_document_v1(
            &inventory,
            &decisions,
            &d.document,
            &f,
            &coverage(),
            &mut r
        ),
        Err(CoordinatorError::Unavailable)
    );
    o.install_shutdown_fence_v1(&basis, &control, &mut r)
        .unwrap();
    let before = o.native_root_identity().unwrap();
    for (cut, choices) in [
        (&inventory, &other_decisions),
        (&other_inventory, &decisions),
        (&other_inventory, &other_decisions),
    ] {
        assert_eq!(
            o.discard_shutdown_document_v1(cut, choices, &d.document, &f, &coverage(), &mut r),
            Err(CoordinatorError::Unavailable)
        );
        assert_eq!(o.native_root_identity().unwrap(), before);
    }
    assert_eq!(
        o.install_shutdown_fence_v1(&other_basis, &control, &mut r),
        Err(CoordinatorError::Unavailable)
    );
    for index in 0..18 {
        for value in [None, Some(false)] {
            let mut denied = coverage();
            *match index {
                0 => &mut denied.not_stopped,
                1 => &mut denied.companions_ready,
                2 => &mut denied.versions_available,
                3 => &mut denied.reply_ready,
                4 => &mut denied.lifecycle_allowed,
                5 => &mut denied.coverage_complete,
                6 => &mut denied.owner_control_allowed,
                7 => &mut denied.source_discard_allowed,
                8 => &mut denied.annotation_discard_allowed,
                9 => &mut denied.retention_complete,
                10 => &mut denied.effects_retained,
                11 => &mut denied.history_retained,
                12 => &mut denied.snapshots_retained,
                13 => &mut denied.providers_retained,
                14 => &mut denied.views_detachable,
                15 => &mut denied.memberships_detachable,
                16 => &mut denied.observer_scopes_fenced,
                _ => &mut denied.binding_coverage_complete,
            } = value;
            assert_eq!(
                o.discard_shutdown_document_v1(
                    &inventory,
                    &decisions,
                    &d.document,
                    &f,
                    &denied,
                    &mut r
                ),
                Err(CoordinatorError::Unavailable)
            );
            assert_eq!(o.native_root_identity().unwrap(), before);
        }
    }
    let mut revoked = facts(&c);
    revoked.authorized = None;
    let revoked = TrustedShutdownInventoryFactsV1 {
        control: &revoked,
        inventory_readable: Some(true),
    };
    assert_eq!(
        o.discard_shutdown_document_v1(
            &inventory,
            &decisions,
            &d.document,
            &revoked,
            &coverage(),
            &mut r
        ),
        Err(CoordinatorError::Unavailable)
    );
    assert_eq!(o.native_root_identity().unwrap(), before);
    assert!(!o.is_faulted());
}

#[test]
fn pending_original_save_blocks_disposal_until_real_protected_baseline_settlement() {
    let (mut o, mut r, d, port) = resource::source_save_test_fixture();
    o.configure_retention_budget(1048576, &mut r).unwrap();
    let save = o
        .queue_pinned_save(
            &d,
            SaveRequirements {
                protection: Protection::BestEffort,
                acknowledgment: Acknowledgment::Ordinary,
            },
            &mut r,
        )
        .unwrap();
    let c = context();
    let control = facts(&c);
    let f = TrustedShutdownInventoryFactsV1 {
        control: &control,
        inventory_readable: Some(true),
    };
    let (basis, inventory, decisions) = capture(&o, &f, ShutdownDocumentChoiceV1::Discard, &mut r);
    o.install_shutdown_fence_v1(&basis, &control, &mut r)
        .unwrap();
    assert_eq!(
        o.discard_shutdown_document_v1(
            &inventory,
            &decisions,
            &d.document,
            &f,
            &coverage(),
            &mut r
        ),
        Err(CoordinatorError::Resource(
            crate::resource::ResourceError::Retained
        ))
    );
    let dispatch = o
        .dispatch_save(
            &save,
            port.read_result(b"key", b"v0", b"A").unwrap(),
            &mut r,
        )
        .unwrap();
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
    assert_eq!(
        o.discard_shutdown_document_v1(
            &inventory,
            &decisions,
            &d.document,
            &f,
            &coverage(),
            &mut r
        )
        .unwrap()
        .phase,
        LifecyclePhase::DisposedRetained
    );
    assert!(!o.is_faulted());
}

#[test]
fn acknowledged_document_expectations_are_rechecked_at_destructive_admission() {
    for changed in 0..7 {
        let (mut o, mut r, d, _) = resource::source_save_test_fixture();
        o.configure_retention_budget(1048576, &mut r).unwrap();
        let c = context();
        let control = facts(&c);
        let f = TrustedShutdownInventoryFactsV1 {
            control: &control,
            inventory_readable: Some(true),
        };
        let (basis, inventory, decisions) =
            capture(&o, &f, ShutdownDocumentChoiceV1::Discard, &mut r);
        o.install_shutdown_fence_v1(&basis, &control, &mut r)
            .unwrap();
        // Fault-injection simulates stale native expectations independently of
        // the ordinary-edit fence; no test-only publication route is introduced.
        let current = o.documents[d.document.slot].as_mut().unwrap();
        match changed {
            0 => current.policy_generation += 1,
            1 => current.read_only = !current.read_only,
            2 => current.annotation_policy += 1,
            3 => current.annotation_read_only = !current.annotation_read_only,
            4 => current.serialization.generation += 1,
            5 => current.annotation_generation += 1,
            _ => current.lifecycle.generation += 1,
        }
        let before = o.native_root_identity().unwrap();
        assert_eq!(
            o.discard_shutdown_document_v1(
                &inventory,
                &decisions,
                &d.document,
                &f,
                &coverage(),
                &mut r
            ),
            Err(CoordinatorError::Stale)
        );
        assert_eq!(o.native_root_identity().unwrap(), before);
        assert!(!o.is_faulted());
    }
}

#[test]
fn interrupted_shutdown_disposal_keeps_both_integrity_and_admission_fences() {
    for accepted in [false, true] {
        for point in 0..5 {
            let (mut o, mut r, d, _) = resource::source_save_test_fixture();
            o.configure_retention_budget(1048576, &mut r).unwrap();
            let c = context();
            let control = facts(&c);
            let f = TrustedShutdownInventoryFactsV1 {
                control: &control,
                inventory_readable: Some(true),
            };
            let (basis, inventory, decisions) =
                capture(&o, &f, ShutdownDocumentChoiceV1::Discard, &mut r);
            let id = if accepted {
                let bindings: Vec<_> = decisions
                    .decisions
                    .iter()
                    .map(|d| ShutdownSaveBindingsV1 {
                        document: d.expected.source.document.clone(),
                        source: None,
                        annotations: None,
                    })
                    .collect();
                let ShutdownAcceptanceV1::Accepted(id) = o
                    .accept_shutdown_disposition_v1(
                        &inventory, &decisions, &bindings, 262144, &f, &mut r,
                    )
                    .unwrap()
                else {
                    panic!()
                };
                id
            } else {
                o.install_shutdown_fence_v1(&basis, &control, &mut r)
                    .unwrap()
            };
            let held = o.retention_usage();
            r = Recorder::try_with_capacity(64).unwrap();
            o.publication_fault_after = Some(point);
            let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if accepted {
                    return o.dispose_accepted_shutdown_document_v1(
                        &id,
                        &d.document,
                        &f,
                        &coverage(),
                        &mut r,
                    );
                }
                o.discard_shutdown_document_v1(
                    &inventory,
                    &decisions,
                    &d.document,
                    &f,
                    &coverage(),
                    &mut r,
                )
            }));
            assert!(interrupted.is_err());
            assert!(o.is_faulted());
            let events: Vec<_> = std::iter::from_fn(|| r.pop_front()).collect();
            if accepted {
                assert!(events.iter().any(|e| e.boundary
                    == Boundary::ShutdownAcceptedDocumentDisposition
                    && e.disposition == Disposition::Unknown));
            }
            assert!(
                events
                    .iter()
                    .any(|e| e.boundary == Boundary::ShutdownDocumentDisposition
                        && e.disposition == Disposition::Unknown)
            );
            assert_eq!(o.retention_usage(), held);
            assert_eq!(
                o.install_shutdown_fence_v1(&basis, &control, &mut r)
                    .unwrap(),
                id
            );
            assert_eq!(
                o.ensure_admission_unfenced(),
                Err(CoordinatorError::Unavailable)
            );
            assert_eq!(
                o.discard_shutdown_document_v1(
                    &inventory,
                    &decisions,
                    &d.document,
                    &f,
                    &coverage(),
                    &mut r
                ),
                Err(CoordinatorError::Faulted)
            );
        }
    }
}
